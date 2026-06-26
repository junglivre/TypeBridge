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
        use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetKeyboardLayout, VkKeyScanExW};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            GetForegroundWindow, GetWindowThreadProcessId,
        };

        // Characters outside the BMP can't be expressed as a single VK.
        if (ch as u32) > 0xFFFF {
            return self.send_unicode(ch);
        }

        // Map the character to a virtual key + modifier state on the target
        // window's keyboard layout.
        let scan = unsafe {
            let hwnd = GetForegroundWindow();
            let tid = GetWindowThreadProcessId(hwnd, std::ptr::null_mut());
            let layout = GetKeyboardLayout(tid);
            VkKeyScanExW(ch as u16, layout)
        };
        if scan == -1 {
            // Not reachable on this layout (e.g. a dead-key combo): fall back.
            return self.send_unicode(ch);
        }

        let hi = ((scan as u16) >> 8) & 0xFF;
        let mods = [
            (hi & 1 != 0, Key::Shift),
            (hi & 2 != 0, Key::Control),
            (hi & 4 != 0, Key::Alt),
        ];

        for (needed, key) in mods {
            if needed {
                self.hold(key, Direction::Press)?;
            }
        }
        // The base key: enigo emits the layout's scancode for `ch` (without the
        // modifiers), which combined with the ones we hold yields `ch`.
        let result = self
            .enigo
            .key(Key::Unicode(ch), Direction::Click)
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
}
