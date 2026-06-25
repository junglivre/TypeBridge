# TypeBridge

A lightweight, cross-platform utility that **types arbitrary text into the
currently focused window** — one keystroke at a time. Built for VNC, Guacamole,
KVMs, remote consoles, web terminals, and BIOS/IPMI environments where
clipboard sharing isn't available.

It *simulates real keyboard input*. It does **not** paste and does **not** send
the clipboard.

- Native (Rust + [egui]/[eframe]), no Electron/Java/Python/.NET runtime
- Tiny release binary (~3.5 MB), fast startup, low memory
- No telemetry, no account, no internet required

---

## Features

- **Unicode-aware multiline editor** — tabs and newlines become real `Tab` /
  `Enter` keystrokes.
- **Configurable per-key delay** (1–2000 ms) and **initial delay** (time to
  switch to the target window).
- **Typing speed presets** (Very fast → Very slow).
- **Multi-language UI** — English, Português (BR) and Español, switchable at
  runtime.
- **Focus guard** *(optional)* — if the focused window changes mid-typing (a
  notification steals focus, you alt-tab by accident…), typing **pauses** and a
  banner appears so you can **continue** (with a fresh countdown to refocus the
  target) or **restart** and reconfigure. *(Windows; no-op elsewhere.)*
- **Optional "minimize before typing"** so the app gets out of the way.
- **Cancel anytime with `Esc`** — works even while minimized (the physical key
  is watched), or via the Cancel button.
- **Paste-from-clipboard** button (fills the editor; never auto-types).
- **Live status** (`Ready` / `Waiting…` / `Typing…` / `Paused` / `Finished` /
  `Cancelled`) with a progress bar.
- **Settings persistence** (delay, initial delay, minimize, focus guard,
  language, window size) with a **portable mode** fallback.
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

## Platform notes

- **Windows** — works out of the box.
- **Linux** — X11 is supported; under restricted Wayland sessions key injection
  may not work (you'll get a friendly error). Build deps: an X11 dev
  environment.
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
  typing/engine.rs     char → keystroke engine (Typer)
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

[egui]: https://github.com/emilk/egui
[eframe]: https://crates.io/crates/eframe
[`confy`]: https://crates.io/crates/confy
[WinLibs]: https://winlibs.com/
