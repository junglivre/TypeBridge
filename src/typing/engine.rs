//! The typing engine: maps characters to keystrokes.
//!
//! [`Typer`] wraps the platform keyboard backend and sends one character at a
//! time. The loop (timing, cancellation, focus detection) lives in
//! [`super::worker`].
//!
//! ## Keystroke methods ([`KeyMode`])
//!
//! * **Unicode** (`enigo.text`) — injects the character via `KEYEVENTF_UNICODE`.
//!   Universal for local apps, but remote clients (noVNC, RDP, KVM-over-IP) read
//!   *physical* scancodes + modifier state and ignore these synthetic events.
//! * **Physical — system layout** — presses the real base key (by scancode) plus
//!   the Shift/Ctrl/Alt modifiers required to produce the character *on the
//!   active Windows layout* (`VkKeyScanExW`). Correct when your local layout
//!   matches the remote one.
//! * **Physical — US layout** — same, but the base key + modifiers are resolved
//!   against a fixed **US-QWERTY** map, regardless of your local layout. This is
//!   what a US remote console (noVNC/QEMU in raw-scancode mode) expects, and it
//!   needs no changes to your system keyboard settings. Characters that don't
//!   exist on a US keyboard (e.g. `ç`, `á`) fall back to Unicode injection.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use serde::{Deserialize, Serialize};

/// Which method to use when injecting printable characters.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyMode {
    /// Unicode injection (best for local apps / any character).
    Unicode,
    /// Physical keys resolved against the active system layout.
    PhysicalAuto,
    /// Physical keys resolved against a fixed US-QWERTY layout (best for VNC).
    PhysicalUs,
}

impl Default for KeyMode {
    fn default() -> Self {
        KeyMode::PhysicalAuto
    }
}

/// How an input character maps to a keystroke.
#[derive(Debug, PartialEq)]
enum CharAction {
    /// A printable character.
    Char(char),
    Enter,
    Tab,
    Backspace,
    /// Ignored (e.g. carriage return in CRLF line endings).
    Skip,
}

fn classify(c: char) -> CharAction {
    match c {
        '\n' => CharAction::Enter,
        '\t' => CharAction::Tab,
        '\r' => CharAction::Skip,
        '\u{8}' | '\u{7f}' => CharAction::Backspace,
        _ => CharAction::Char(c),
    }
}

/// Errors raised while injecting keystrokes.
#[derive(Debug)]
pub enum TypeError {
    /// Could not create the keyboard backend (e.g. missing macOS Accessibility
    /// permission, or unsupported Wayland session).
    Init(String),
    /// A keystroke failed to send.
    Inject(String),
}

impl std::fmt::Display for TypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TypeError::Init(e) => write!(
                f,
                "Could not start keyboard input ({e}).\n\
                 On macOS: enable Accessibility for this app under \
                 System Settings → Privacy & Security → Accessibility.\n\
                 On Linux: an X11 session is required (Wayland is restricted)."
            ),
            TypeError::Inject(e) => write!(f, "Keyboard injection failed: {e}"),
        }
    }
}

/// Wraps the keyboard backend; sends one character at a time.
pub struct Typer {
    enigo: Enigo,
    mode: KeyMode,
}

impl Typer {
    pub fn new(mode: KeyMode) -> Result<Self, TypeError> {
        Enigo::new(&Settings::default())
            .map(|enigo| Self { enigo, mode })
            .map_err(|e| TypeError::Init(e.to_string()))
    }

    /// Inject the keystroke(s) for a single character.
    pub fn send(&mut self, c: char) -> Result<(), TypeError> {
        match classify(c) {
            CharAction::Skip => Ok(()),
            CharAction::Enter => self.tap(Key::Return),
            CharAction::Tab => self.tap(Key::Tab),
            CharAction::Backspace => self.tap(Key::Backspace),
            CharAction::Char(ch) => match self.mode {
                KeyMode::Unicode => self.send_unicode(ch),
                KeyMode::PhysicalAuto => self.send_physical_auto(ch),
                KeyMode::PhysicalUs => self.send_physical_us(ch),
            },
        }
    }

    fn tap(&mut self, key: Key) -> Result<(), TypeError> {
        self.enigo
            .key(key, Direction::Click)
            .map_err(|e| TypeError::Inject(e.to_string()))
    }

    fn send_unicode(&mut self, ch: char) -> Result<(), TypeError> {
        self.enigo
            .text(ch.encode_utf8(&mut [0u8; 4]))
            .map_err(|e| TypeError::Inject(e.to_string()))
    }

    // ---- Physical: system layout -----------------------------------------

    #[cfg(windows)]
    fn send_physical_auto(&mut self, ch: char) -> Result<(), TypeError> {
        match resolve_key(ch) {
            Some(p) => self.send_with_mods(p.scancode, p.shift, p.ctrl, p.alt),
            None => self.send_unicode(ch),
        }
    }

    #[cfg(not(windows))]
    fn send_physical_auto(&mut self, ch: char) -> Result<(), TypeError> {
        self.send_unicode(ch)
    }

    // ---- Physical: fixed US-QWERTY layout --------------------------------

    #[cfg(windows)]
    fn send_physical_us(&mut self, ch: char) -> Result<(), TypeError> {
        match us_scancode(ch) {
            Some((scancode, shift)) => self.send_with_mods(scancode, shift, false, false),
            None => self.send_unicode(ch), // not present on a US keyboard
        }
    }

    #[cfg(not(windows))]
    fn send_physical_us(&mut self, ch: char) -> Result<(), TypeError> {
        self.send_unicode(ch)
    }

    /// Hold the given modifiers, tap the base scancode, release the modifiers.
    #[cfg(windows)]
    fn send_with_mods(
        &mut self,
        scancode: u16,
        shift: bool,
        ctrl: bool,
        alt: bool,
    ) -> Result<(), TypeError> {
        let mods = [
            (shift, Key::Shift),
            (ctrl, Key::Control),
            (alt, Key::Alt),
        ];
        for (needed, key) in mods {
            if needed {
                self.hold(key, Direction::Press)?;
            }
        }
        let result = self
            .enigo
            .raw(scancode, Direction::Click)
            .map_err(|e| TypeError::Inject(e.to_string()));
        for (needed, key) in mods.into_iter().rev() {
            if needed {
                let _ = self.hold(key, Direction::Release);
            }
        }
        result
    }

    #[cfg(windows)]
    fn hold(&mut self, key: Key, dir: Direction) -> Result<(), TypeError> {
        self.enigo
            .key(key, dir)
            .map_err(|e| TypeError::Inject(e.to_string()))
    }
}

/// A resolved physical-key plan for a character on the active layout.
#[cfg(windows)]
struct KeyPlan {
    scancode: u16,
    shift: bool,
    ctrl: bool,
    alt: bool,
}

/// Resolve a character to a base scancode + modifiers on the *active* keyboard
/// layout. Returns `None` when the character isn't reachable with a single
/// physical key (the caller falls back to Unicode).
#[cfg(windows)]
fn resolve_key(ch: char) -> Option<KeyPlan> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardLayout, MapVirtualKeyExW, VkKeyScanExW, MAPVK_VK_TO_VSC,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    if (ch as u32) > 0xFFFF {
        return None;
    }

    let (vk_scan, layout) = unsafe {
        let hwnd = GetForegroundWindow();
        let tid = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
        let layout = GetKeyboardLayout(tid);
        (VkKeyScanExW(ch as u16, layout), layout)
    };
    if vk_scan == -1 {
        return None;
    }

    let vk = (vk_scan as u16) & 0x00FF; // mask off the shift state
    let shift_state = ((vk_scan as u16) >> 8) & 0xFF;

    // Translate the *masked* virtual key to its scancode (the unmasked value
    // would yield scancode 0 -> a dropped keystroke).
    let scancode = unsafe { MapVirtualKeyExW(vk as u32, MAPVK_VK_TO_VSC, layout) } as u16;
    if scancode == 0 {
        return None;
    }

    Some(KeyPlan {
        scancode,
        shift: shift_state & 1 != 0,
        ctrl: shift_state & 2 != 0,
        alt: shift_state & 4 != 0,
    })
}

/// Map a character to a US-QWERTY (scancode, shift) pair, using PS/2 set-1
/// scancodes. Returns `None` for characters that aren't on a US keyboard.
#[cfg(windows)]
fn us_scancode(ch: char) -> Option<(u16, bool)> {
    if ch.is_ascii_lowercase() {
        return Some((letter_scancode(ch), false));
    }
    if ch.is_ascii_uppercase() {
        return Some((letter_scancode(ch.to_ascii_lowercase()), true));
    }
    let entry = match ch {
        '1' => (0x02, false),
        '!' => (0x02, true),
        '2' => (0x03, false),
        '@' => (0x03, true),
        '3' => (0x04, false),
        '#' => (0x04, true),
        '4' => (0x05, false),
        '$' => (0x05, true),
        '5' => (0x06, false),
        '%' => (0x06, true),
        '6' => (0x07, false),
        '^' => (0x07, true),
        '7' => (0x08, false),
        '&' => (0x08, true),
        '8' => (0x09, false),
        '*' => (0x09, true),
        '9' => (0x0A, false),
        '(' => (0x0A, true),
        '0' => (0x0B, false),
        ')' => (0x0B, true),
        '-' => (0x0C, false),
        '_' => (0x0C, true),
        '=' => (0x0D, false),
        '+' => (0x0D, true),
        '[' => (0x1A, false),
        '{' => (0x1A, true),
        ']' => (0x1B, false),
        '}' => (0x1B, true),
        ';' => (0x27, false),
        ':' => (0x27, true),
        '\'' => (0x28, false),
        '"' => (0x28, true),
        '`' => (0x29, false),
        '~' => (0x29, true),
        '\\' => (0x2B, false),
        '|' => (0x2B, true),
        ',' => (0x33, false),
        '<' => (0x33, true),
        '.' => (0x34, false),
        '>' => (0x34, true),
        '/' => (0x35, false),
        '?' => (0x35, true),
        ' ' => (0x39, false),
        _ => return None,
    };
    Some(entry)
}

/// US-QWERTY PS/2 set-1 scancode for a lowercase letter.
#[cfg(windows)]
fn letter_scancode(c: char) -> u16 {
    match c {
        'q' => 0x10,
        'w' => 0x11,
        'e' => 0x12,
        'r' => 0x13,
        't' => 0x14,
        'y' => 0x15,
        'u' => 0x16,
        'i' => 0x17,
        'o' => 0x18,
        'p' => 0x19,
        'a' => 0x1E,
        's' => 0x1F,
        'd' => 0x20,
        'f' => 0x21,
        'g' => 0x22,
        'h' => 0x23,
        'j' => 0x24,
        'k' => 0x25,
        'l' => 0x26,
        'z' => 0x2C,
        'x' => 0x2D,
        'c' => 0x2E,
        'v' => 0x2F,
        'b' => 0x30,
        'n' => 0x31,
        'm' => 0x32,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn control_chars_map_to_special_keys() {
        assert_eq!(classify('\n'), CharAction::Enter);
        assert_eq!(classify('\t'), CharAction::Tab);
        assert_eq!(classify('\r'), CharAction::Skip);
        assert_eq!(classify('\u{8}'), CharAction::Backspace);
        assert_eq!(classify('\u{7f}'), CharAction::Backspace);
    }

    #[test]
    fn printable_and_unicode_pass_through() {
        assert_eq!(classify('a'), CharAction::Char('a'));
        assert_eq!(classify(' '), CharAction::Char(' '));
        assert_eq!(classify('é'), CharAction::Char('é'));
        assert_eq!(classify('🚀'), CharAction::Char('🚀'));
    }

    /// Regression: enigo's `Key::Unicode` path produced scancode 0 for shifted
    /// characters, so `#`, capitals and `!` were dropped by VNC clients.
    #[cfg(windows)]
    #[test]
    fn shifted_chars_get_a_real_scancode() {
        let upper = resolve_key('A').expect("'A' should map to a key");
        assert!(upper.shift, "'A' requires Shift");
        assert_ne!(upper.scancode, 0, "'A' must have a real scancode");

        let lower = resolve_key('a').expect("'a' should map to a key");
        assert!(!lower.shift, "'a' needs no Shift");
        assert_ne!(lower.scancode, 0);

        assert_eq!(upper.scancode, lower.scancode);
    }

    /// The fixed US map must produce the right scancode + shift regardless of
    /// the host layout, and reject characters absent from a US keyboard.
    #[cfg(windows)]
    #[test]
    fn us_layout_table_is_correct() {
        assert_eq!(us_scancode('a'), Some((0x1E, false)));
        assert_eq!(us_scancode('A'), Some((0x1E, true)));
        assert_eq!(us_scancode('3'), Some((0x04, false)));
        assert_eq!(us_scancode('#'), Some((0x04, true)));
        assert_eq!(us_scancode('/'), Some((0x35, false)));
        assert_eq!(us_scancode(';'), Some((0x27, false)));
        assert_eq!(us_scancode('!'), Some((0x02, true)));
        assert_eq!(us_scancode(' '), Some((0x39, false)));
        assert_eq!(us_scancode('ç'), None);
        assert_eq!(us_scancode('á'), None);
    }
}
