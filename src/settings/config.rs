//! Persistent configuration.
//!
//! Stored as TOML. Supports a *portable* mode: if a `typebridge.toml` file
//! exists next to the executable, it is used; otherwise the per-user OS
//! config directory is used (via `confy`).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::i18n::Lang;

const APP_NAME: &str = "typebridge";
const CONFIG_NAME: &str = "config";
const PORTABLE_FILE: &str = "typebridge.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub delay_ms: u32,
    pub initial_delay_s: u32,
    pub minimize_before_typing: bool,
    pub detect_window_change: bool,
    pub language: Lang,
    pub window_width: f32,
    pub window_height: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            delay_ms: 20,
            initial_delay_s: 3,
            minimize_before_typing: false,
            detect_window_change: false,
            language: Lang::default(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_roadmap() {
        let c = Config::default();
        assert_eq!(c.delay_ms, 20, "default per-key delay is 20 ms");
        assert_eq!(c.initial_delay_s, 3, "default initial delay is 3 s");
        assert!(!c.minimize_before_typing);
        assert!(c.window_width >= 360.0 && c.window_height >= 420.0);
    }

    #[test]
    fn round_trips_through_toml() {
        let c = Config {
            delay_ms: 42,
            initial_delay_s: 7,
            minimize_before_typing: true,
            detect_window_change: true,
            language: Lang::PtBr,
            window_width: 700.0,
            window_height: 800.0,
        };
        let toml = toml::to_string(&c).expect("serialize");
        let back: Config = toml::from_str(&toml).expect("deserialize");
        assert_eq!(back.delay_ms, 42);
        assert_eq!(back.initial_delay_s, 7);
        assert!(back.minimize_before_typing);
        assert!(back.detect_window_change);
        assert_eq!(back.language, Lang::PtBr);
    }
}
