//! Optional update checker and repository links.
//!
//! Contacts the GitHub Releases API to see whether a newer version is
//! published, and exposes the latest release notes. Entirely opt-in and
//! non-blocking; failures are silent so the app works fully offline.

use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use serde::Deserialize;

/// The current app version (from `Cargo.toml`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The project repository URL (from `Cargo.toml`).
pub const REPO_URL: &str = env!("CARGO_PKG_REPOSITORY");

/// Details of the latest published release.
#[derive(Clone)]
pub struct Release {
    pub tag: String,
    pub version: String,
    pub notes: String,
    pub url: String,
}

/// Outcome of an update check.
#[derive(Clone)]
pub enum UpdateState {
    Idle,
    Checking,
    UpToDate,
    Available(Release),
    Failed,
}

pub type SharedState = Arc<Mutex<UpdateState>>;

pub fn shared() -> SharedState {
    Arc::new(Mutex::new(UpdateState::Idle))
}

/// `owner/repo` parsed from the repository URL.
fn owner_repo() -> Option<String> {
    REPO_URL
        .trim_end_matches('/')
        .trim_end_matches(".git")
        .strip_prefix("https://github.com/")
        .map(str::to_string)
}

fn api_latest_url() -> Option<String> {
    owner_repo().map(|or| format!("https://api.github.com/repos/{or}/releases/latest"))
}

/// Spawn a background update check, storing the outcome in `state`.
pub fn spawn_check<R: Fn() + Send + 'static>(state: SharedState, repaint: R) {
    {
        let mut s = state.lock().unwrap();
        if matches!(*s, UpdateState::Checking) {
            return;
        }
        *s = UpdateState::Checking;
    }
    repaint();

    thread::spawn(move || {
        let outcome = match fetch_latest() {
            Ok(rel) if is_newer(&rel.version, VERSION) => UpdateState::Available(rel),
            Ok(_) => UpdateState::UpToDate,
            Err(()) => UpdateState::Failed,
        };
        *state.lock().unwrap() = outcome;
        repaint();
    });
}

fn fetch_latest() -> Result<Release, ()> {
    #[derive(Deserialize)]
    struct ApiRelease {
        tag_name: String,
        #[serde(default)]
        body: Option<String>,
        html_url: String,
    }

    let url = api_latest_url().ok_or(())?;
    // ureq's `tls` feature uses rustls with bundled roots — pure Rust, so no
    // OpenSSL/libssl dependency (keeps the Linux binary portable) and it
    // registers itself as the backend automatically.
    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(8))
        .timeout_read(Duration::from_secs(8))
        .build();

    let resp = agent
        .get(&url)
        .set("User-Agent", concat!("TypeBridge/", env!("CARGO_PKG_VERSION")))
        .set("Accept", "application/vnd.github+json")
        .call()
        .map_err(|_| ())?;

    let api: ApiRelease = resp.into_json().map_err(|_| ())?;
    let version = api.tag_name.trim_start_matches('v').to_string();
    Ok(Release {
        tag: api.tag_name,
        version,
        notes: api.body.unwrap_or_default(),
        url: api.html_url,
    })
}

/// Compare dotted versions, ignoring a leading `v` and any pre-release suffix.
pub fn is_newer(latest: &str, current: &str) -> bool {
    parse(latest) > parse(current)
}

fn parse(v: &str) -> (u64, u64, u64) {
    let core = v.trim().trim_start_matches('v');
    let core = core.split(|c| c == '-' || c == '+').next().unwrap_or(core);
    let mut parts = core.split('.');
    let major = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let minor = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    let patch = parts.next().and_then(|p| p.parse().ok()).unwrap_or(0);
    (major, minor, patch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_comparison() {
        assert!(is_newer("1.0.1", "1.0.0"));
        assert!(is_newer("v1.1.0", "1.0.9"));
        assert!(is_newer("2.0.0", "1.9.9"));
        assert!(!is_newer("1.0.0", "1.0.0"));
        assert!(!is_newer("1.0.0", "1.0.1"));
        assert!(!is_newer("1.0.0-rc1", "1.0.0"));
    }

    #[test]
    fn repo_is_parsed() {
        assert_eq!(owner_repo().as_deref(), Some("junglivre/TypeBridge"));
    }
}
