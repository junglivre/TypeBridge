//! Thin wrapper over `arboard` for reading text from the system clipboard.

use arboard::Clipboard;

/// Read the current clipboard text.
///
/// Returns a friendly error string when the clipboard is unavailable or holds
/// no text.
pub fn get_text() -> Result<String, String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Clipboard unavailable: {e}"))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Could not read clipboard text: {e}"))
}
