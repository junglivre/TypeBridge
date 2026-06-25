//! The typing engine: maps characters to keystrokes.
//!
//! [`Typer`] wraps the platform keyboard backend and sends one character at a
//! time. The actual loop (timing, cancellation, focus-change detection) lives
//! in [`super::worker`].

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

/// How an input character maps to a keystroke.
#[derive(Debug, PartialEq)]
enum CharAction {
    /// A printable character, typed via Unicode injection.
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
}

impl Typer {
    pub fn new() -> Result<Self, TypeError> {
        Enigo::new(&Settings::default())
            .map(|enigo| Self { enigo })
            .map_err(|e| TypeError::Init(e.to_string()))
    }

    /// Inject the keystroke(s) for a single character.
    pub fn send(&mut self, c: char) -> Result<(), TypeError> {
        match classify(c) {
            CharAction::Skip => Ok(()),
            CharAction::Enter => self.press(Key::Return),
            CharAction::Tab => self.press(Key::Tab),
            CharAction::Backspace => self.press(Key::Backspace),
            CharAction::Char(ch) => self
                .enigo
                .text(ch.encode_utf8(&mut [0u8; 4]))
                .map_err(|e| TypeError::Inject(e.to_string())),
        }
    }

    fn press(&mut self, key: Key) -> Result<(), TypeError> {
        self.enigo
            .key(key, Direction::Click)
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
