//! Persistent configuration.
//!
//! Stored as TOML. Supports a *portable* mode: if a `typebridge.toml` file
//! exists next to the executable, it is used; otherwise the per-user OS
//! config directory is used (via `confy`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const APP_NAME: &str = "typebridge";
const CONFIG_NAME: &str = "config";
const PORTABLE_FILE: &str = "typebridge.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub delay_ms: u32,
    pub initial_delay_s: u32,
    pub minimize_before_typing: bool,
    pub window_width: f32,
    pub window_height: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            delay_ms: 20,
            initial_delay_s: 3,
            minimize_before_typing: false,
            window_width: 520.0,
            window_height: 640.0,
        }
    }
}

/// Returns the portable config path if a config file already sits next to the
/// executable.
fn portable_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    let path = dir.join(PORTABLE_FILE);
    if path.exists() {
        Some(path)
    } else {
        None
    }
}

impl Config {
    /// Load the configuration, falling back to defaults on any error.
    pub fn load() -> Self {
        match portable_path() {
            Some(path) => confy::load_path(path).unwrap_or_default(),
            None => confy::load(APP_NAME, Some(CONFIG_NAME)).unwrap_or_default(),
        }
    }

    /// Persist the configuration. Errors are returned so callers may surface
    /// them, but the app generally ignores them (settings are non-critical).
    pub fn save(&self) -> Result<(), confy::ConfyError> {
        match portable_path() {
            Some(path) => confy::store_path(path, self),
            None => confy::store(APP_NAME, Some(CONFIG_NAME), self),
        }
    }
}
