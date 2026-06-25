//! The typing engine: turns a string into simulated keystrokes.
//!
//! ```text
//! for char in text:
//!     if cancelled: break
//!     send_key(char)
//!     sleep(delay)   // in small slices, so cancel is responsive
//! ```

use std::time::Duration;

use enigo::{Direction, Enigo, Key, Keyboard, Settings};

use super::cancel::{CancelToken, EscWatcher};

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

/// Result of a typing run.
pub enum Outcome {
    Finished,
    Cancelled,
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

/// Type `text` character by character, sleeping `delay` between keys.
///
/// `on_progress` is called with the number of characters processed so far.
/// Cancellation is checked before every key and during every sleep.
pub fn run<F: FnMut(usize)>(
    text: &str,
    delay: Duration,
    cancel: &CancelToken,
    esc: &EscWatcher,
    mut on_progress: F,
) -> Result<Outcome, TypeError> {
    let mut enigo =
        Enigo::new(&Settings::default()).map_err(|e| TypeError::Init(e.to_string()))?;

    for (i, c) in text.chars().enumerate() {
        if cancel.is_cancelled() || esc.esc_pressed() {
            return Ok(Outcome::Cancelled);
        }

        match classify(c) {
            CharAction::Skip => {}
            CharAction::Enter => press(&mut enigo, Key::Return)?,
            CharAction::Tab => press(&mut enigo, Key::Tab)?,
            CharAction::Backspace => press(&mut enigo, Key::Backspace)?,
            CharAction::Char(ch) => enigo
                .text(ch.encode_utf8(&mut [0u8; 4]))
                .map_err(|e| TypeError::Inject(e.to_string()))?,
        }

        on_progress(i + 1);

        if !sleep_cancellable(delay, cancel, esc) {
            return Ok(Outcome::Cancelled);
        }
    }

    Ok(Outcome::Finished)
}

fn press(enigo: &mut Enigo, key: Key) -> Result<(), TypeError> {
    enigo
        .key(key, Direction::Click)
        .map_err(|e| TypeError::Inject(e.to_string()))
}

/// Sleep for `dur`, split into small slices so cancellation is responsive even
/// with long per-key delays. Returns `false` if cancelled during the wait.
fn sleep_cancellable(dur: Duration, cancel: &CancelToken, esc: &EscWatcher) -> bool {
    const SLICE: Duration = Duration::from_millis(10);
    let mut remaining = dur;
    while remaining > Duration::ZERO {
        if cancel.is_cancelled() || esc.esc_pressed() {
            return false;
        }
        let step = remaining.min(SLICE);
        std::thread::sleep(step);
        remaining = remaining.saturating_sub(step);
    }
    !(cancel.is_cancelled() || esc.esc_pressed())
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
