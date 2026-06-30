//! KDE backend: libei keycode injection via the RemoteDesktop portal.
//!
//! libei injects evdev keycodes; the compositor decodes them with its *active*
//! layout group. KWin won't report that group to a background portal client
//! (the `ei_keyboard.modifiers` event never arrives), so we read it from KDE's
//! D-Bus (`org.kde.keyboard`) and look up keycodes in that group. With the
//! correct group, Shift/AltGr and symbols come out right regardless of layout.
//!
//! Adapted from enigo 0.6's `linux/libei.rs` (MIT), but `Result`-based (no
//! panics, so X11 stays safe) and with correct modifier handling.

use std::collections::HashMap;
use std::os::unix::net::UnixStream;
use std::time::Instant;

use reis::{
    ei::{self, Connection},
    handshake::HandshakeResp,
    PendingRequestResult,
};
use xkbcommon::xkb;

use super::{char_to_keysym, keycode_level};

const NAME: &str = "TypeBridge";
const KEY_SHIFT_L: u32 = 0xffe1;
const KEY_ISO_LEVEL3_SHIFT: u32 = 0xfe03;

static INTERFACES: std::sync::LazyLock<HashMap<&'static str, u32>> =
    std::sync::LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("ei_button", 1);
        m.insert("ei_callback", 1);
        m.insert("ei_connection", 1);
        m.insert("ei_device", 2);
        m.insert("ei_keyboard", 1);
        m.insert("ei_pingpong", 1);
        m.insert("ei_pointer", 1);
        m.insert("ei_pointer_absolute", 1);
        m.insert("ei_scroll", 1);
        m.insert("ei_seat", 1);
        m
    });

#[derive(Debug, Default, Clone)]
struct SeatData {
    #[allow(dead_code)]
    name: Option<String>,
    capabilities: HashMap<String, u64>,
}

#[derive(Debug, Default, PartialEq, Copy, Clone)]
enum DeviceState {
    #[default]
    Paused,
    Resumed,
    Emulating,
}

#[derive(Debug, Default, Clone)]
struct DeviceData {
    #[allow(dead_code)]
    name: Option<String>,
    device_type: Option<ei::device::DeviceType>,
    interfaces: HashMap<String, reis::Object>,
    state: DeviceState,
}

impl DeviceData {
    fn interface<T: reis::Interface>(&self) -> Option<T> {
        self.interfaces.get(T::NAME)?.clone().downcast()
    }
}

/// Query KDE for the active keyboard layout index (== xkb layout group).
/// Read-only; `None` on non-KDE or any error.
async fn kde_active_group() -> Option<u32> {
    let conn = ashpd::zbus::Connection::session().await.ok()?;
    let reply = conn
        .call_method(
            Some("org.kde.keyboard"),
            "/Layouts",
            Some("org.kde.KeyboardLayouts"),
            "getLayout",
            &(),
        )
        .await
        .ok()?;
    reply.body().deserialize::<u32>().ok()
}

fn looks_absent(s: &str) -> bool {
    let s = s.to_lowercase();
    s.contains("not found")
        || s.contains("serviceunknown")
        || s.contains("no owner")
        || s.contains("not provided")
        || s.contains("not implemented")
}

/// A connected libei backend.
pub struct Backend {
    seats: HashMap<ei::Seat, SeatData>,
    devices: HashMap<ei::Device, DeviceData>,
    keyboards: HashMap<ei::Keyboard, xkb::Keymap>,
    group: u32,
    shift_kc: Option<u32>,
    altgr_kc: Option<u32>,
    sequence: u32,
    last_serial: u32,
    context: ei::Context,
    #[allow(dead_code)]
    connection: Connection,
    time_created: Instant,
}

async fn open_connection() -> Result<Option<ei::Context>, String> {
    use ashpd::desktop::remote_desktop::{DeviceType, RemoteDesktop};
    use ashpd::desktop::PersistMode;

    if let Some(context) =
        ei::Context::connect_to_env().map_err(|e| format!("connect_to_env: {e}"))?
    {
        return Ok(Some(context));
    }

    let remote_desktop = match RemoteDesktop::new().await {
        Ok(p) => p,
        Err(e) if looks_absent(&format!("{e}")) => return Ok(None),
        Err(e) => return Err(format!("RemoteDesktop portal: {e}")),
    };
    let session = remote_desktop
        .create_session()
        .await
        .map_err(|e| format!("create_session: {e}"))?;
    remote_desktop
        .select_devices(
            &session,
            DeviceType::Keyboard.into(),
            None,
            PersistMode::DoNot,
        )
        .await
        .map_err(|e| format!("select_devices: {e}"))?;
    remote_desktop
        .start(&session, None)
        .await
        .map_err(|e| format!("start: {e}"))?
        .response()
        .map_err(|e| format!("permission denied: {e}"))?;
    let fd = remote_desktop
        .connect_to_eis(&session)
        .await
        .map_err(|e| format!("connect_to_eis: {e}"))?;

    let stream = UnixStream::from(fd);
    stream
        .set_nonblocking(true)
        .map_err(|e| format!("set_nonblocking: {e}"))?;
    Ok(Some(
        ei::Context::new(stream).map_err(|e| format!("ei::Context::new: {e}"))?,
    ))
}

impl Backend {
    pub fn try_connect() -> Result<Option<Self>, String> {
        let context = match futures::executor::block_on(open_connection())? {
            Some(c) => c,
            None => return Ok(None),
        };

        let HandshakeResp {
            connection,
            serial,
            negotiated_interfaces: _,
        } = reis::handshake::ei_handshake_blocking(
            &context,
            NAME,
            ei::handshake::ContextType::Sender,
        )
        .map_err(|e| format!("handshake: {e}"))?;

        context.flush().map_err(|e| format!("flush: {e}"))?;

        let mut backend = Backend {
            seats: HashMap::new(),
            devices: HashMap::new(),
            keyboards: HashMap::new(),
            group: 0,
            shift_kc: None,
            altgr_kc: None,
            sequence: 0,
            last_serial: serial.wrapping_add(1),
            context,
            connection,
            time_created: Instant::now(),
        };

        backend.update()?;

        // Start emulating on resumed virtual devices.
        let to_start: Vec<ei::Device> = backend
            .devices
            .iter()
            .filter(|(_, d)| {
                d.device_type == Some(ei::device::DeviceType::Virtual)
                    && d.state == DeviceState::Resumed
            })
            .map(|(dev, _)| dev.clone())
            .collect();
        for device in to_start {
            if !device.is_alive() {
                return Err("ei::Device is no longer alive".into());
            }
            device.start_emulating(backend.last_serial, backend.sequence);
            backend.sequence = backend.sequence.wrapping_add(1);
            if let Some(d) = backend.devices.get_mut(&device) {
                d.state = DeviceState::Emulating;
            }
        }
        backend.update()?;

        // Active layout group + cached modifier keycodes for it.
        backend.group = futures::executor::block_on(kde_active_group()).unwrap_or(0);
        let g = backend.group;
        if let Some(keymap) = backend.keyboards.values().next() {
            backend.shift_kc = keycode_level(keymap, g, KEY_SHIFT_L).map(|(k, _)| k);
            backend.altgr_kc = keycode_level(keymap, g, KEY_ISO_LEVEL3_SHIFT).map(|(k, _)| k);
        }

        Ok(Some(backend))
    }

    pub fn send(&mut self, c: char) -> Result<(), String> {
        let group = self.group;
        let plan = {
            let keymap = self.keyboards.values().next().ok_or("no keymap received")?;
            keycode_level(keymap, group, char_to_keysym(c))
        };
        let Some((kc, level)) = plan else {
            return Ok(()); // unmapped; skip
        };

        let mut mods: Vec<u32> = Vec::new();
        if level == 1 || level == 3 {
            if let Some(s) = self.shift_kc {
                mods.push(s);
            }
        }
        if level == 2 || level == 3 {
            if let Some(a) = self.altgr_kc {
                mods.push(a);
            }
        }

        let (device, keyboard) = self
            .keyboard_handles()
            .ok_or("no keyboard device available")?;
        if !keyboard.is_alive() {
            return Err("ei::Keyboard is no longer alive".into());
        }

        for m in &mods {
            self.emit(&device, &keyboard, *m, true);
        }
        self.emit(&device, &keyboard, kc, true);
        self.emit(&device, &keyboard, kc, false);
        for m in mods.iter().rev() {
            self.emit(&device, &keyboard, *m, false);
        }
        self.update()?;
        Ok(())
    }

    fn keyboard_handles(&self) -> Option<(ei::Device, ei::Keyboard)> {
        self.devices.iter().find_map(|(dev, data)| {
            data.interface::<ei::Keyboard>().map(|kb| (dev.clone(), kb))
        })
    }

    fn emit(&mut self, device: &ei::Device, keyboard: &ei::Keyboard, keycode: u32, press: bool) {
        let st = if press {
            ei::keyboard::KeyState::Press
        } else {
            ei::keyboard::KeyState::Released
        };
        keyboard.key(keycode - 8, st);
        let elapsed = self.time_created.elapsed().as_secs();
        device.frame(self.sequence, elapsed);
        self.sequence = self.sequence.wrapping_add(1);
    }

    fn update(&mut self) -> Result<(), String> {
        let mut had_pending_events = true;
        loop {
            self.context
                .read()
                .map_err(|_| "failed to read libei context".to_string())?;

            while let Some(result) = self.context.pending_event() {
                had_pending_events = true;
                let request = match result {
                    PendingRequestResult::Request(request) => request,
                    PendingRequestResult::ParseError(msg) => {
                        return Err(format!("libei parse error: {msg}"));
                    }
                    PendingRequestResult::InvalidObject(_) => continue,
                };

                match request {
                    ei::Event::Handshake(handshake, request) => match request {
                        ei::handshake::Event::HandshakeVersion { version: _ } => {
                            handshake.handshake_version(1);
                            handshake.name(NAME);
                            handshake.context_type(ei::handshake::ContextType::Sender);
                            for (interface, version) in INTERFACES.iter() {
                                handshake.interface_version(interface, *version);
                            }
                            handshake.finish();
                        }
                        ei::handshake::Event::InterfaceVersion { .. } => {}
                        ei::handshake::Event::Connection { serial, .. } => {
                            self.last_serial = serial;
                            self.sequence = serial;
                        }
                        _ => {}
                    },
                    ei::Event::Connection(_connection, request) => match request {
                        ei::connection::Event::Disconnected {
                            last_serial,
                            reason,
                            explanation,
                        } => {
                            self.seats.clear();
                            self.devices.clear();
                            self.keyboards.clear();
                            self.sequence = 0;
                            self.last_serial = last_serial;
                            return Err(format!("disconnected: {reason:?} {explanation}"));
                        }
                        ei::connection::Event::Seat { seat } => {
                            self.seats.insert(seat, SeatData::default());
                        }
                        ei::connection::Event::InvalidObject { .. } => {}
                        ei::connection::Event::Ping { ping } => {
                            if ping.is_alive() {
                                ping.done(0);
                            }
                        }
                        _ => {}
                    },
                    ei::Event::Seat(seat, request) => {
                        let Some(data) = self.seats.get_mut(&seat) else {
                            continue;
                        };
                        match request {
                            ei::seat::Event::Destroyed { .. } => {
                                self.seats.remove(&seat);
                            }
                            ei::seat::Event::Name { name } => {
                                data.name = Some(name);
                            }
                            ei::seat::Event::Capability { mask, interface } => {
                                data.capabilities.insert(interface, mask);
                            }
                            ei::seat::Event::Done => {
                                let mut bitmask = 0u64;
                                for cap in [
                                    "ei_button",
                                    "ei_keyboard",
                                    "ei_pointer",
                                    "ei_pointer_absolute",
                                    "ei_scroll",
                                    "ei_touchscreen",
                                ] {
                                    if let Some(bits) = data.capabilities.get(cap) {
                                        bitmask |= bits;
                                    }
                                }
                                seat.bind(bitmask);
                            }
                            ei::seat::Event::Device { device } => {
                                self.devices.insert(device, DeviceData::default());
                            }
                            _ => {}
                        }
                    }
                    ei::Event::Device(device, request) => {
                        let Some(data) = self.devices.get_mut(&device) else {
                            continue;
                        };
                        match request {
                            ei::device::Event::Destroyed { .. } => {
                                self.devices.remove(&device);
                            }
                            ei::device::Event::Name { name } => {
                                data.name = Some(name);
                            }
                            ei::device::Event::DeviceType { device_type } => {
                                data.device_type = Some(device_type);
                            }
                            ei::device::Event::Interface { object } => {
                                data.interfaces
                                    .insert(object.interface().to_string(), object);
                            }
                            ei::device::Event::Resumed { serial } => {
                                self.last_serial = serial;
                                data.state = DeviceState::Resumed;
                            }
                            ei::device::Event::Paused { serial } => {
                                self.last_serial = serial;
                                data.state = DeviceState::Paused;
                            }
                            _ => {}
                        }
                    }
                    ei::Event::Keyboard(keyboard, request) => match request {
                        ei::keyboard::Event::Destroyed { .. } => {
                            self.keyboards.remove(&keyboard);
                        }
                        ei::keyboard::Event::Keymap {
                            keymap_type,
                            size,
                            keymap,
                        } => {
                            if keymap_type != ei::keyboard::KeymapType::Xkb {
                                return Err("keymap is not xkb".into());
                            }
                            use std::io::{Read, Seek, SeekFrom};
                            let mut file = std::fs::File::from(keymap);
                            file.seek(SeekFrom::Start(0))
                                .map_err(|e| format!("seek keymap fd: {e}"))?;
                            let mut buf = Vec::with_capacity(size as usize);
                            file.take(size as u64)
                                .read_to_end(&mut buf)
                                .map_err(|e| format!("read keymap fd: {e}"))?;
                            while matches!(buf.last(), Some(0)) {
                                buf.pop();
                            }
                            let text =
                                String::from_utf8(buf).map_err(|e| format!("keymap utf8: {e}"))?;
                            let ctx = xkb::Context::new(0);
                            let km = xkb::Keymap::new_from_string(
                                &ctx,
                                text,
                                xkb::KEYMAP_FORMAT_TEXT_V1,
                                xkb::KEYMAP_COMPILE_NO_FLAGS,
                            )
                            .ok_or("keymap compile failed")?;
                            self.keyboards.insert(keyboard, km);
                        }
                        _ => {}
                    },
                    _ => {}
                }
            }

            let _ = self.context.flush();
            std::thread::sleep(std::time::Duration::from_millis(10));

            if !had_pending_events {
                break;
            }
            had_pending_events = false;
        }
        Ok(())
    }
}
