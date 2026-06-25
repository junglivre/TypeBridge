//! Foreground-window detection.
//!
//! Used by the optional "stop if the focused window changes" feature: we
//! remember which window was focused when typing started, and pause if focus
//! moves elsewhere (e.g. a notification steals it).

/// An opaque identifier for the current foreground window, or `None` if it
/// cannot be determined (unsupported platform, or no window focused).
#[cfg(windows)]
pub fn foreground_window() -> Option<u64> {
    use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    // SAFETY: GetForegroundWindow has no preconditions and returns a handle or
    // null. We only use the handle as an opaque comparison key.
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_null() {
        None
    } else {
        Some(hwnd as usize as u64)
    }
}

#[cfg(not(windows))]
pub fn foreground_window() -> Option<u64> {
    None
}

/// Whether focus-change detection actually works on this platform.
#[cfg(windows)]
pub const SUPPORTED: bool = true;
#[cfg(not(windows))]
pub const SUPPORTED: bool = false;
