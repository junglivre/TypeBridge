# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project follows
[Semantic Versioning](https://semver.org/).

**Downloads:** get the binary for your system from the
[latest release](https://github.com/junglivre/TypeBridge/releases/latest) — the
[README](https://github.com/junglivre/TypeBridge#download) shows which file to
pick and how to run it. Each version below links to its release.

## [2.2.0] - 2026-07-01

### Fixed
- **Rendering backend now works without graphics acceleration.** Switched from
  OpenGL (glow) to wgpu with a software fallback (Direct3D WARP on Windows), so
  TypeBridge runs in RDP sessions and on VMs/older machines that don't provide
  OpenGL 2.0+ (previously it failed to start with an OpenGL error). The dark
  theme is now always used, so it no longer appears in light mode on remote
  sessions.

## [2.1.0] - 2026-06-30

### Added
- **Application icon** — shown in the window title bar, the taskbar, and (on
  Windows) the executable itself in Explorer.

### Changed
- Much clearer **[Download](https://github.com/junglivre/TypeBridge#download)**
  guide in the README: a "which file do I get?" table per system (Windows /
  macOS / Linux, x86_64 / ARM), plus how to open it — including the Windows
  SmartScreen and macOS Gatekeeper prompts that non-technical users hit.
- Every GitHub release now leads with the same plain-language download guide.

## [2.0.0] - 2026-06-30

### Added
- **Wayland keyboard support.** Typing now works on native Wayland sessions, not
  just X11. The right mechanism is selected automatically per compositor:
  - **wlroots** (Sway, Hyprland, river, niri…) — a `zwp_virtual_keyboard` with
    our own uploaded keymap. Layout-independent and needs no permission dialog.
  - **GNOME / Mutter** — the RemoteDesktop portal's `NotifyKeyboardKeysym`; the
    compositor resolves the keysym in the active layout itself.
  - **KDE / KWin** — libei (RemoteDesktop portal) injecting keycodes, with the
    active layout group read from KDE's D-Bus so symbols/Shift come out right.
  - Anything else (or no portal, e.g. Cinnamon's Wayland) falls back to the X11
    backend, which still reaches XWayland apps.
- Update notifications now appear as a **startup popup** with the release notes
  rendered as Markdown, plus Download / Dismiss actions.

### Changed
- **TLS now uses rustls** instead of native-tls, so the binary no longer depends
  on OpenSSL/`libssl` — smaller surface and more portable Linux builds.
- **Linux release binaries are built with `cargo-zigbuild` targeting an old
  glibc (2.31)**, so they run on a wide range of distributions (Ubuntu 20.04+,
  Debian 11+, Mint, Fedora, …) instead of only on very recent ones.

### Fixed
- Linux/Wayland keystrokes now reach native Wayland apps (the 1.0.2 limitation).



### Added
- The UI language now defaults to the system locale on first run (pt-BR / es /
  en); the user's later choice is remembered.
- Linux: a startup notice that keyboard injection can be unstable depending on
  the desktop (most reliable on X11).
- Linux: a warning under "Minimize window before typing" — shown only when it is
  enabled — that the window may not come back, due to desktop restrictions.

### Changed
- Linux: type via enigo's pure-Rust `x11rb` backend instead of `libxdo` (no
  `libxdo` dependency at build or runtime).

### Known limitations
- Linux/Wayland: keystrokes don't reach apps yet — use an X11 session for now.
  Native Wayland support is in progress.

## [1.0.1] - 2026-06-26

### Fixed
- Update checker could never reach GitHub ("Couldn't check for updates"): the
  `native-tls` backend was compiled in but not registered with the HTTP client,
  so every HTTPS request failed with "no TLS backend is configured". The TLS
  connector is now wired in explicitly (SChannel on Windows, OpenSSL on Linux,
  Secure Transport on macOS).

## [1.0.0] - 2026-06-26

### Added
- Type arbitrary text into the focused window, one real keystroke at a time
  (for VNC, Guacamole, KVMs, web terminals, BIOS/IPMI).
- Unicode multiline editor; tabs and newlines become `Tab` / `Enter`.
- Configurable per-key delay (1–2000 ms), initial delay, and speed presets.
- Three keystroke methods: Unicode, physical (system layout), and physical with
  a fixed US-QWERTY layout for raw-scancode remote consoles (noVNC/QEMU).
- Focus guard: pauses with a modal alert if the focused window changes mid-typing
  (continue or restart).
- Cancel with `Esc` even while minimized; optional minimize-before-typing.
- Paste-from-clipboard, progress bar, friendly errors.
- Multi-language UI (English, Português-BR, Español) with persisted settings and
  a portable-config mode.
- Background update check + repository/author links in the footer.
- Minimal CLI (`--delay/--wait/--file/--text/--minimize/--autostart`).
- GitHub Actions: CI (build/test) and multi-platform release builds
  (Windows/Linux/macOS, x86_64 and ARM64).

[2.2.0]: https://github.com/junglivre/TypeBridge/releases/tag/2.2.0
[2.1.0]: https://github.com/junglivre/TypeBridge/releases/tag/2.1.0
[2.0.0]: https://github.com/junglivre/TypeBridge/releases/tag/2.0.0
[1.0.2]: https://github.com/junglivre/TypeBridge/releases/tag/1.0.2
[1.0.1]: https://github.com/junglivre/TypeBridge/releases/tag/1.0.1
[1.0.0]: https://github.com/junglivre/TypeBridge/releases/tag/1.0.0
