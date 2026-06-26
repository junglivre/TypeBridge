//! The typing engine: maps characters to keystrokes.
//!
//! [`Typer`] wraps the platform keyboard backend and sends one character at a
//! time. The loop (timing, cancellation, focus detection) lives in
//! [`super::worker`].
//!
//! ## Unicode vs. physical keys
//!
//! Two ways to inject a printable character on Windows:
//!
//! * **Unicode** (`enigo.text`) — injects the character via `KEYEVENTF_UNICODE`.
//!   Works for any character in local apps, but many remote clients (noVNC,
//!   RDP, KVM-over-IP, some games) ignore these synthetic Unicode events: they
//!   read *physical* scancodes + modifier state, so `#` arrives as `3` and
//!   uppercase letters arrive lowercase (the Shift modifier is never pressed).
//! * **Physical keys** — presses the real keys a human would: the base key by
//!   scancode plus the required `Shift`/`Ctrl`/`Alt` modifiers (computed with
//!   `VkKeyScanExW`). This is what remote consoles expect. Characters that
//!   aren't reachable on the active keyboard layout fall back to Unicode.

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

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
    /// When true, type printable characters as physical key presses (with real
    /// modifiers) instead of Unicode injection. Recommended for VNC/remote.
    physical_keys: bool,
}

impl Typer {
    pub fn new(physical_keys: bool) -> Result<Self, TypeError> {
        Enigo::new(&Settings::default())
            .map(|enigo| Self {
                enigo,
                physical_keys,
            })
            .map_err(|e| TypeError::Init(e.to_string()))
    }

    /// Inject the keystroke(s) for a single character.
    pub fn send(&mut self, c: char) -> Result<(), TypeError> {
        match classify(c) {
            CharAction::Skip => Ok(()),
            CharAction::Enter => self.tap(Key::Return),
            CharAction::Tab => self.tap(Key::Tab),
            CharAction::Backspace => self.tap(Key::Backspace),
            CharAction::Char(ch) => {
                if self.physical_keys {
                    self.send_physical(ch)
                } else {
                    self.send_unicode(ch)
                }
            }
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

    #[cfg(not(windows))]
    fn send_physical(&mut self, ch: char) -> Result<(), TypeError> {
        // Physical-key shift computation is implemented for Windows only; other
        // platforms fall back to Unicode injection.
        self.send_unicode(ch)
    }

    /// Press the base key (by scancode) together with the Shift/Ctrl/Alt
    /// modifiers a real keyboard would use, so remote clients see proper input.
    #[cfg(windows)]
    fn send_physical(&mut self, ch: char) -> Result<(), TypeError> {
        // Fall back to Unicode injection for characters we can't reach with a
        // single physical key on the target's layout (dead-key combos, etc.).
        let Some(plan) = resolve_key(ch) else {
            return self.send_unicode(ch);
        };

        let mods = [
            (plan.shift, Key::Shift),
            (plan.ctrl, Key::Control),
            (plan.alt, Key::Alt),
        ];

        for (needed, key) in mods {
            if needed {
                self.hold(key, Direction::Press)?;
            }
        }
        // Send the base key by scancode (this avoids enigo's Key::Unicode path,
        // which mis-handles shifted characters). Combined with the modifiers we
        // hold, this yields `ch`.
        let result = self
            .enigo
            .raw(plan.scancode, Direction::Click)
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

/// Resolve a character to a base scancode + the modifiers needed to produce it
/// on the target window's keyboard layout. Returns `None` when the character
/// isn't reachable with a single physical key (the caller falls back to Unicode).
#[cfg(windows)]
fn resolve_key(ch: char) -> Option<KeyPlan> {
    use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
        GetKeyboardLayout, MapVirtualKeyExW, VkKeyScanExW, MAPVK_VK_TO_VSC,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetForegroundWindow, GetWindowThreadProcessId,
    };

    // Characters outside the BMP can't be expressed as a single VK.
    if (ch as u32) > 0xFFFF {
        return None;
    }

    // Map the character to a virtual key + shift state on the target window's
    // layout: low byte = virtual key, high byte = shift state.
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

    // Translate the *masked* virtual key to its scancode. Passing the unmasked
    // value (with the shift bits) yields scancode 0 — a dropped keystroke.
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
    /// `resolve_key` must return a real scancode and flag Shift.
    #[cfg(windows)]
    #[test]
    fn shifted_chars_get_a_real_scancode() {
        let upper = resolve_key('A').expect("'A' should map to a key");
        assert!(upper.shift, "'A' requires Shift");
        assert_ne!(upper.scancode, 0, "'A' must have a real scancode");

        let lower = resolve_key('a').expect("'a' should map to a key");
        assert!(!lower.shift, "'a' needs no Shift");
        assert_ne!(lower.scancode, 0);

        // Upper- and lower-case of the same letter share the physical key.
        assert_eq!(upper.scancode, lower.scancode);
    }
}
