# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project follows
[Semantic Versioning](https://semver.org/).

## [1.0.2] - 2026-06-26

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

[1.0.2]: https://github.com/junglivre/TypeBridge/releases/tag/1.0.2
[1.0.1]: https://github.com/junglivre/TypeBridge/releases/tag/1.0.1
[1.0.0]: https://github.com/junglivre/TypeBridge/releases/tag/1.0.0
