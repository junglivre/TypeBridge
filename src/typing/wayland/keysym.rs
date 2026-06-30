//! GNOME (and generic portal) backend: `NotifyKeyboardKeysym`.
//!
//! We send the keysym and let the compositor pick the keycode + modifiers in
//! its active layout. Mutter does this correctly, so it's layout-independent
//! with no keymap/group bookkeeping. (KWin's implementation is buggy on older
//! versions — KDE uses the libei path instead.)

use ashpd::desktop::remote_desktop::{DeviceType, KeyState, RemoteDesktop};
use ashpd::desktop::{PersistMode, Session};

use super::char_to_keysym;

type Rd = RemoteDesktop<'static>;

/// A connected portal keysym backend.
pub struct Backend {
    proxy: Rd,
    session: Session<'static, Rd>,
}

/// Heuristic: does this error mean the RemoteDesktop portal isn't available
/// (so we should fall back to enigo) rather than a genuine failure?
fn looks_absent(e: &ashpd::Error) -> bool {
    let s = format!("{e}").to_lowercase();
    s.contains("not found")
        || s.contains("serviceunknown")
        || s.contains("no owner")
        || s.contains("not provided")
        || s.contains("not implemented")
}

impl Backend {
    /// Open a RemoteDesktop session. `Ok(None)` if the portal is absent (caller
    /// falls back to enigo); `Err` if present but the session failed/was denied.
    pub fn try_connect() -> Result<Option<Self>, String> {
        futures::executor::block_on(async {
            let proxy: Rd = match RemoteDesktop::new().await {
                Ok(p) => p,
                Err(e) if looks_absent(&e) => return Ok(None),
                Err(e) => return Err(format!("RemoteDesktop portal: {e}")),
            };
            let session = match proxy.create_session().await {
                Ok(s) => s,
                Err(e) if looks_absent(&e) => return Ok(None),
                Err(e) => return Err(format!("create_session: {e}")),
            };
            proxy
                .select_devices(
                    &session,
                    DeviceType::Keyboard.into(),
                    None,
                    PersistMode::DoNot,
                )
                .await
                .map_err(|e| format!("select_devices: {e}"))?;
            proxy
                .start(&session, None)
                .await
                .map_err(|e| format!("start: {e}"))?
                .response()
                .map_err(|e| format!("permission denied: {e}"))?;
            Ok(Some(Backend { proxy, session }))
        })
    }

    pub fn send(&mut self, c: char) -> Result<(), String> {
        let keysym = char_to_keysym(c) as i32;
        futures::executor::block_on(async {
            self.proxy
                .notify_keyboard_keysym(&self.session, keysym, KeyState::Pressed)
                .await
                .map_err(|e| format!("press {c:?}: {e}"))?;
            self.proxy
                .notify_keyboard_keysym(&self.session, keysym, KeyState::Released)
                .await
                .map_err(|e| format!("release {c:?}: {e}"))?;
            Ok(())
        })
    }
}
