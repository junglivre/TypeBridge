//! Wayland keyboard backends.
//!
//! Wayland forbids the X11-style global input injection enigo uses, so each
//! compositor family needs a different mechanism — selected at runtime:
//!
//! * **wlroots** (Sway, Hyprland, river, niri…) — [`vkbd`]: a `zwp_virtual_keyboard`
//!   with our OWN uploaded keymap. Layout-independent, no permission dialog.
//! * **KDE/KWin** — [`libei`]: inject evdev keycodes over libei (RemoteDesktop
//!   portal). KWin decodes them with the *active* layout group, which it won't
//!   report, so we read it from KDE's D-Bus and look up keycodes in that group.
//! * **GNOME/Mutter** (and other portals) — [`keysym`]: the portal's
//!   `NotifyKeyboardKeysym`; Mutter resolves the keysym against the active
//!   layout itself, so this is layout-correct with no bookkeeping.
//!
//! If none apply (e.g. Cinnamon's Wayland has no RemoteDesktop portal),
//! [`WaylandTyper::try_connect`] returns `Ok(None)` and the caller falls back to
//! enigo (which still reaches XWayland apps).

use xkbcommon::xkb;

mod keysym;
mod libei;
mod vkbd;

/// A connected Wayland keyboard backend.
pub enum WaylandTyper {
    Libei(libei::Backend),
    Keysym(keysym::Backend),
    Vkbd(vkbd::Backend),
}

impl WaylandTyper {
    /// Select and connect a Wayland backend for the current session.
    ///
    /// * `Ok(Some(_))` — a backend connected; use it.
    /// * `Ok(None)` — no Wayland backend applies here; fall back to enigo.
    /// * `Err(_)` — a backend was attempted but failed (e.g. the user denied the
    ///   portal); surface the error.
    pub fn try_connect() -> Result<Option<Self>, String> {
        // Not a Wayland session → let enigo handle X11.
        if std::env::var_os("WAYLAND_DISPLAY").is_none() {
            return Ok(None);
        }

        // wlroots: a virtual keyboard with our own keymap is the cleanest path.
        if let Some(v) = vkbd::Backend::try_connect()? {
            return Ok(Some(WaylandTyper::Vkbd(v)));
        }

        // Otherwise the portal. KDE's keysym path is buggy on older versions, so
        // we use libei + D-Bus there; everyone else gets the keysym path.
        let desktop = std::env::var("XDG_CURRENT_DESKTOP")
            .unwrap_or_default()
            .to_uppercase();
        if desktop.contains("KDE") {
            return Ok(libei::Backend::try_connect()?.map(WaylandTyper::Libei));
        }
        Ok(keysym::Backend::try_connect()?.map(WaylandTyper::Keysym))
    }

    /// Inject the keystroke(s) for one character.
    pub fn send(&mut self, c: char) -> Result<(), String> {
        // CRLF: the '\n' already produces Enter, so drop the '\r'.
        if c == '\r' {
            return Ok(());
        }
        match self {
            WaylandTyper::Libei(b) => b.send(c),
            WaylandTyper::Keysym(b) => b.send(c),
            WaylandTyper::Vkbd(b) => b.send(c),
        }
    }
}

/// Map a character to its X11 keysym (printable ranges + a few controls),
/// without depending on `xkb_utf32_to_keysym`.
pub(super) fn char_to_keysym(c: char) -> u32 {
    match c {
        '\n' => 0xff0d,    // Return
        '\t' => 0xff09,    // Tab
        '\u{8}' => 0xff08, // BackSpace
        '\u{7f}' => 0xff08,
        '\u{1b}' => 0xff1b, // Escape
        _ => {
            let cp = c as u32;
            if (0x20..=0x7e).contains(&cp) || (0xa0..=0xff).contains(&cp) {
                cp
            } else {
                0x0100_0000 + cp
            }
        }
    }
}

/// Find a keycode + shift level producing `keysym` in the given layout group.
///
/// Levels: 0 = base, 1 = Shift, 2 = AltGr, 3 = Shift+AltGr. Searched by level
/// first so the simplest key wins (e.g. the dedicated `/` key rather than an
/// `AltGr` combo on a lower keycode).
pub(super) fn keycode_level(keymap: &xkb::Keymap, group: u32, keysym: u32) -> Option<(u32, usize)> {
    for level in 0..=3u32 {
        for kc in keymap.min_keycode()..=keymap.max_keycode() {
            if keymap.key_get_syms_by_level(kc, group, level).contains(&keysym) {
                return Some((kc, level as usize));
            }
        }
    }
    None
}
