//! wlroots backend: `zwp_virtual_keyboard` with our own uploaded keymap.
//!
//! Because the compositor decodes this virtual device's keycodes with the
//! keymap WE upload (US), the output is independent of the user's active layout
//! and needs no portal/permission dialog. Same idea as `wtype`.

use std::io::{Seek, SeekFrom, Write};
use std::os::fd::{AsFd, FromRawFd, OwnedFd};

use wayland_client::protocol::{wl_registry, wl_seat};
use wayland_client::{delegate_noop, Connection, Dispatch, EventQueue, QueueHandle};
use wayland_protocols_misc::zwp_virtual_keyboard_v1::client::{
    zwp_virtual_keyboard_manager_v1::ZwpVirtualKeyboardManagerV1,
    zwp_virtual_keyboard_v1::ZwpVirtualKeyboardV1,
};
use xkbcommon::xkb;

use super::{char_to_keysym, keycode_level};

const KEYMAP_FORMAT_XKB_V1: u32 = 1;
const MOD_INVALID: u32 = 0xffff_ffff;

#[derive(Default)]
struct State {
    seat: Option<wl_seat::WlSeat>,
    mgr: Option<ZwpVirtualKeyboardManagerV1>,
}

impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let wl_registry::Event::Global {
            name,
            interface,
            version,
        } = event
        {
            match interface.as_str() {
                "wl_seat" => {
                    state.seat =
                        Some(registry.bind::<wl_seat::WlSeat, _, _>(name, version.min(7), qh, ()));
                }
                "zwp_virtual_keyboard_manager_v1" => {
                    state.mgr =
                        Some(registry.bind::<ZwpVirtualKeyboardManagerV1, _, _>(name, 1, qh, ()));
                }
                _ => {}
            }
        }
    }
}

delegate_noop!(State: ignore wl_seat::WlSeat);
delegate_noop!(State: ZwpVirtualKeyboardManagerV1);
delegate_noop!(State: ZwpVirtualKeyboardV1);

/// A connected virtual-keyboard backend.
pub struct Backend {
    conn: Connection,
    queue: EventQueue<State>,
    state: State,
    vkbd: ZwpVirtualKeyboardV1,
    keymap: xkb::Keymap,
    shift_mask: u32,
    altgr_mask: u32,
    time: u32,
}

fn mod_mask(keymap: &xkb::Keymap, name: &str) -> u32 {
    let idx = keymap.mod_get_index(&name);
    if idx == MOD_INVALID {
        0
    } else {
        1u32 << idx
    }
}

/// Anonymous in-memory fd holding the keymap text (NUL-terminated).
fn keymap_fd(bytes: &[u8]) -> Result<OwnedFd, String> {
    let name = std::ffi::CString::new("typebridge-keymap").unwrap();
    let raw = unsafe { libc::memfd_create(name.as_ptr(), 0) };
    if raw < 0 {
        return Err("memfd_create failed".into());
    }
    let mut f = unsafe { std::fs::File::from_raw_fd(raw) };
    f.write_all(bytes).map_err(|e| format!("write keymap: {e}"))?;
    f.write_all(&[0u8]).map_err(|e| format!("write nul: {e}"))?;
    f.seek(SeekFrom::Start(0)).ok();
    Ok(OwnedFd::from(f))
}

impl Backend {
    /// Connect if the compositor advertises `zwp_virtual_keyboard_manager_v1`
    /// (wlroots). Returns `Ok(None)` otherwise so the caller tries the portal.
    pub fn try_connect() -> Result<Option<Self>, String> {
        let Ok(conn) = Connection::connect_to_env() else {
            return Ok(None);
        };
        let mut queue = conn.new_event_queue();
        let qh = queue.handle();
        conn.display().get_registry(&qh, ());

        let mut state = State::default();
        queue
            .roundtrip(&mut state)
            .map_err(|e| format!("wayland roundtrip: {e}"))?;

        let (Some(seat), Some(mgr)) = (state.seat.clone(), state.mgr.clone()) else {
            // No virtual-keyboard protocol here → not wlroots.
            return Ok(None);
        };
        let vkbd = mgr.create_virtual_keyboard(&seat, &qh, ());

        // Our own US keymap; the compositor decodes this device with it.
        let ctx = xkb::Context::new(0);
        let keymap = xkb::Keymap::new_from_names(
            &ctx,
            &"",
            &"",
            &"us",
            &"",
            None,
            xkb::KEYMAP_COMPILE_NO_FLAGS,
        )
        .ok_or("failed to build the US keymap")?;
        let kstr = keymap.get_as_string(xkb::KEYMAP_FORMAT_TEXT_V1);
        let size = kstr.as_bytes().len() + 1;
        let fd = keymap_fd(kstr.as_bytes())?;
        vkbd.keymap(KEYMAP_FORMAT_XKB_V1, fd.as_fd(), size as u32);
        queue
            .roundtrip(&mut state)
            .map_err(|e| format!("wayland roundtrip (keymap): {e}"))?;

        let shift_mask = mod_mask(&keymap, "Shift");
        let altgr_mask = mod_mask(&keymap, "Mod5");

        Ok(Some(Backend {
            conn,
            queue,
            state,
            vkbd,
            keymap,
            shift_mask,
            altgr_mask,
            time: 0,
        }))
    }

    pub fn send(&mut self, c: char) -> Result<(), String> {
        let Some((kc, level)) = keycode_level(&self.keymap, 0, char_to_keysym(c)) else {
            return Ok(()); // not on a US keymap (rare for our use); skip
        };

        let depressed = match level {
            1 => self.shift_mask,
            2 => self.altgr_mask,
            3 => self.shift_mask | self.altgr_mask,
            _ => 0,
        };

        // On a virtual keyboard the compositor does NOT derive modifiers from
        // key presses; we must set the state explicitly.
        self.vkbd.modifiers(depressed, 0, 0, 0);
        self.vkbd.key(self.time, kc - 8, 1);
        self.time = self.time.wrapping_add(10);
        self.vkbd.key(self.time, kc - 8, 0);
        self.time = self.time.wrapping_add(10);
        self.vkbd.modifiers(0, 0, 0, 0);

        self.conn.flush().map_err(|e| format!("flush: {e}"))?;
        self.queue
            .roundtrip(&mut self.state)
            .map_err(|e| format!("roundtrip: {e}"))?;
        Ok(())
    }
}
