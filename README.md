# TypeBridge

[![CI](https://github.com/junglivre/TypeBridge/actions/workflows/ci.yml/badge.svg)](https://github.com/junglivre/TypeBridge/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/junglivre/TypeBridge?sort=semver)](https://github.com/junglivre/TypeBridge/releases/latest)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)

- **English**
- [Português](README_br.md)
- [Español](README_es.md)

A lightweight, cross-platform utility that **types arbitrary text into the
currently focused window** — one keystroke at a time. Built for VNC, Guacamole,
KVMs, remote consoles, web terminals, and BIOS/IPMI environments where
clipboard sharing isn't available.

It *simulates real keyboard input*. It does **not** paste and does **not** send
the clipboard.

- Native (Rust + [egui]/[eframe]), no Electron/Java/Python/.NET runtime
- Small native binary, fast startup, low memory
- No telemetry, no account — works fully offline (the only network call is an
  optional, silent update check)

## Download

> **Just want to use it?** No installer — download one file and open it.

**1. Get the file for your system** from the
**[latest release](https://github.com/junglivre/TypeBridge/releases/latest)**
(open the **Assets** list and click the matching file):

| Your system | File to download |
|---|---|
| **Windows** — most PCs | `typebridge-…-windows-x86_64.exe` |
| **Windows** — ARM (Snapdragon, Surface Pro X) | `typebridge-…-windows-arm64.exe` |
| **macOS** — Apple Silicon (M1–M4, 2020+) | `typebridge-…-macos-arm64` |
| **macOS** — Intel (2019 and older) | `typebridge-…-macos-x86_64` |
| **Linux** — most PCs | `typebridge-…-linux-x86_64` |
| **Linux** — ARM (Raspberry Pi 4/5, ARM servers) | `typebridge-…-linux-arm64` |

> Not sure about **x86_64 vs ARM**? Pick **x86_64** — it's almost every PC.

**2. Open it:**

- **Windows** — double-click the `.exe`. If you see *"Windows protected your
  PC"*, click **More info → Run anyway**. (The app is open-source but not
  code-signed, so Windows is just being cautious.)
- **macOS** — in Terminal: `chmod +x typebridge-*-macos-*` then
  `./typebridge-*-macos-*`. If macOS blocks it as from an *"unidentified
  developer"*, allow it in **System Settings → Privacy & Security → Open
  Anyway**, and grant **Accessibility** when asked.
- **Linux** — `chmod +x typebridge-*-linux-*` then `./typebridge-*-linux-*`.
  The binaries run on most modern distributions (glibc 2.31+).

Prefer to build it yourself? See [Building](#building).

---

## Features

- **Unicode-aware multiline editor** — tabs and newlines become real `Tab` /
  `Enter` keystrokes.
- **Configurable per-key delay** (1–2000 ms) and **initial delay** (time to
  switch to the target window).
- **Typing speed presets** (Very fast → Very slow).
- **Physical-key mode** *(default)* — types with real key presses and the right
  `Shift`/`Ctrl`/`Alt` modifiers, so `#`, capital letters and symbols arrive
  correctly in **VNC/RDP/KVM and web consoles** (noVNC, Guacamole…). Turn it off
  for Unicode injection (e.g. special characters in local apps).
- **Multi-language UI** — English, Português (BR) and Español, switchable at
  runtime.
- **Linux: X11 *and* Wayland** — works on both; the right Wayland backend
  (wlroots / GNOME / KDE) is selected automatically. See
  [how it types](#how-typebridge-types).
- **Built-in update check** — silently checks GitHub for a newer release on
  startup and, if one exists, shows a **popup with the changelog** (rendered as
  Markdown) and a download link (no telemetry; just a version ping).
- **Focus guard** *(optional)* — if the focused window changes mid-typing (a
  notification steals focus, you alt-tab by accident…), typing **pauses**, the
  window pops to the front (and flashes in the taskbar), and a prominent modal
  alert lets you **continue** (with a fresh countdown to refocus the target) or
  **restart** and reconfigure. *(Windows; no-op elsewhere.)*
- **Optional "minimize before typing"** so the app gets out of the way.
- **Cancel anytime with `Esc`** — works even while minimized (the physical key
  is watched), or via the Cancel button.
- **Paste-from-clipboard** button (fills the editor; never auto-types).
- **Live status** (`Ready` / `Waiting…` / `Typing…` / `Paused` / `Finished` /
  `Cancelled`) with a progress bar.
- **Settings persistence** (delay, initial delay, minimize, focus guard,
  keystroke mode, language, window size) with a **portable mode** fallback.
- **Background typing thread** — the UI never freezes.
- **Minimal CLI** for preloading text/parameters.

---

## Usage

1. Type or paste the text into the editor (or click **Paste Clipboard**).
2. Set the per-key **delay** and the **initial delay**.
3. (Optional) tick **Minimize window before typing**.
4. Click **Start Typing** and focus the target window during the countdown.
5. Press **`Esc`** at any time to stop immediately.

### Command line

All flags are optional:

```
typebridge --delay 25 --wait 5 --file notes.txt --minimize --autostart
```

| Flag             | Meaning                                             |
|------------------|-----------------------------------------------------|
| `--delay <ms>`   | Per-key delay (1–2000)                               |
| `--wait <s>`     | Initial delay before typing starts                  |
| `--file <path>`  | Preload the editor with a text file                 |
| `--text <str>`   | Preload the editor with a literal string            |
| `--minimize`     | Minimize before typing (`--no-minimize` to disable) |
| `--autostart`    | Begin typing automatically on launch                |

### Settings location

- **Portable mode:** if a `typebridge.toml` file exists *next to the
  executable*, it is used.
- Otherwise the per-user OS config directory is used (via [`confy`]).

---

## Building

Requires a stable Rust toolchain (`rustc`/`cargo`). Then:

```sh
cargo build --release
# binary: target/release/typebridge(.exe)
cargo test          # run the unit tests
```

### Windows toolchain note (important)

There are two Windows toolchains:

- **MSVC (recommended, simplest):** install the *Visual Studio Build Tools*
  (C++ workload), then `rustup default stable-msvc`. No extra setup — it just
  builds.

- **GNU (`x86_64-pc-windows-gnu`):** the MinGW **bundled with rustup is
  minimal** and **cannot link the full eframe/glow stack** — the resulting
  binary crashes with `STATUS_ACCESS_VIOLATION` *before `main` runs*. You need a
  **complete MinGW-w64** (e.g. [WinLibs]):

  1. Download a WinLibs GCC build and unpack it (this repo uses `D:\mingw64`).
  2. Add its `bin` directory to your `PATH` (provides `gcc`, `as`, `dlltool`).
  3. Create a **local, git-ignored** `.cargo/config.toml` pointing rustc at it:

     ```toml
     [target.x86_64-pc-windows-gnu]
     linker = 'D:\mingw64\bin\gcc.exe'
     rustflags = [
       '-Clink-self-contained=no',
       '-Cdlltool=D:\mingw64\bin\dlltool.exe',
     ]
     ```

  > `.cargo/` is git-ignored on purpose — it contains machine-specific paths and
  > must not be published.

---

## How TypeBridge types

Injecting synthetic keystrokes is trivial on Windows and X11, but a genuine maze
on Wayland: each compositor exposes a different — and incomplete — mechanism, and
none lets a background tool simply say "type this string". TypeBridge detects the
environment and picks the right backend automatically at runtime:

| Environment | Backend | Keyboard-layout handling |
|---|---|---|
| **Windows** | Win32 input (via [enigo]) | Unicode, or physical scancodes with a fixed US map for VNC |
| **macOS** | CoreGraphics events (via [enigo]) | Unicode |
| **Linux · X11** | XTEST (via enigo `x11rb`) | handled by X |
| **Linux · Wayland · wlroots** (Sway, Hyprland, river, niri…) | `zwp_virtual_keyboard` with our **own uploaded keymap** | layout-independent — we own the keymap |
| **Linux · Wayland · GNOME** | RemoteDesktop portal `NotifyKeyboardKeysym` | Mutter resolves the keysym in the active layout |
| **Linux · Wayland · KDE** | libei (RemoteDesktop portal) keycode injection | active layout group read from KDE's D-Bus |
| **Linux · Wayland · other / no portal** (e.g. Cinnamon) | falls back to X11 / XWayland | handled by X (XWayland apps only) |

### Why Wayland needs four different approaches

Wayland deliberately forbids the global input injection X11's XTEST allows, so
there is no single API. What it took to make typing work everywhere:

- **wlroots** compositors implement `zwp_virtual_keyboard`, which lets a client
  **upload its own keymap**. We upload a US keymap and type against it, so the
  output is correct *regardless of the active layout* and needs no permission
  dialog. (It briefly becomes the seat's active layout while typing, then
  reverts — a wlroots quirk, not a lasting config change.)
- **GNOME** and **KDE** don't support `zwp_virtual_keyboard`; input emulation is
  only available through the **RemoteDesktop portal** (a one-time permission
  dialog):
  - **GNOME/Mutter** resolves a *keysym* to the right key in the active layout
    itself, so the portal's `NotifyKeyboardKeysym` is layout-correct out of the
    box.
  - **KDE/KWin** injects *keycodes* and decodes them with the **currently active
    layout group** — but, as a background service, it can't tell us which group
    that is (the relevant Wayland event never reaches it). So TypeBridge reads
    the active layout from KDE's own D-Bus (`org.kde.keyboard`) and resolves the
    keycode in that group. (KWin's own keysym path had the very same blind spot,
    fixed upstream only in late 2025.)
- **Compositors without a RemoteDesktop portal** (e.g. Cinnamon's experimental
  Wayland) fall back to the X11 backend, which still reaches XWayland apps.

You never pick a backend — it's all automatic.

### Portable Linux binaries

The published Linux binaries are built with
[`cargo-zigbuild`](https://github.com/rust-cross/cargo-zigbuild) targeting
**glibc 2.31**, and use **rustls** for TLS (no OpenSSL), so a single binary runs
across a wide range of distributions (Ubuntu 20.04+, Debian 11+, Mint, Fedora…)
without `libssl`/glibc version mismatches.

## Platform notes

- **Windows** — works out of the box.
- **Linux** — both **X11 and Wayland** are supported (see
  [How TypeBridge types](#how-typebridge-types)). Build deps: X11/xcb +
  xkbcommon + Wayland dev packages (TLS is pure-Rust, so no OpenSSL needed).
- **macOS** — grant Accessibility permission under *System Settings → Privacy &
  Security → Accessibility*; the app reports a clear message if it's missing.

---

## Project layout

```
src/
  main.rs              entry point, CLI parsing, window bootstrap
  i18n.rs              compile-time translations (en / pt-br / es)
  ui/    app.rs        egui application + update loop
         widgets.rs    small UI helpers
  typing/engine.rs     char → keystroke engine (Typer; enigo + Wayland)
         wayland/      Linux Wayland backends (libei · portal keysym · vkbd)
         worker.rs     background typing thread + status channel
         cancel.rs     cancel flag + physical-Esc watcher
         window.rs     foreground-window detection (focus guard)
  settings/config.rs   load/save settings (confy + portable mode)
  clipboard/clipboard.rs  clipboard read (arboard)
```

---

## Non-goals

Macro recording, mouse automation, scripting, OCR, clipboard sync, or remote
desktop. TypeBridge does exactly one thing well: type text into the focused
window.

## License

Dual-licensed under MIT or Apache-2.0.

Made by [jung](https://jung.moe).

[egui]: https://github.com/emilk/egui
[eframe]: https://crates.io/crates/eframe
[enigo]: https://github.com/enigo-rs/enigo
[`confy`]: https://crates.io/crates/confy
[WinLibs]: https://winlibs.com/
