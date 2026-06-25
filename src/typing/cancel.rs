//! Cancellation primitives.
//!
//! Two independent ways to stop typing:
//! * [`CancelToken`] — a shared flag the UI sets (Cancel button / focused Esc).
//! * [`EscWatcher`] — polls the *physical* keyboard for the Esc key so the user
//!   can cancel even while the window is minimized and another app is focused.
//!   It only reads key state; it never grabs the key.

use device_query::{DeviceQuery, DeviceState, Keycode};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

#[derive(Clone, Default)]
pub struct CancelToken {
    flag: Arc<AtomicBool>,
}

impl CancelToken {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }
}

/// Polls the physical keyboard for the cancel key (Esc).
pub struct EscWatcher {
    device: DeviceState,
}

impl EscWatcher {
    pub fn new() -> Self {
        Self {
            device: DeviceState::new(),
        }
    }

    /// `true` if the Esc key is physically held right now.
    pub fn esc_pressed(&self) -> bool {
        self.device.get_keys().contains(&Keycode::Escape)
    }
}
