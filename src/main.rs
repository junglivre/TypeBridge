//! TypeBridge — types arbitrary text into the currently focused window.
//!
//! Designed for VNC, Guacamole, KVMs, web terminals and BIOS/IPMI consoles
//! where clipboard sharing is unavailable. It actually *simulates* keystrokes;
//! it does not paste.

// Hide the console window for the GUI in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod i18n;
mod settings;
mod typing;
mod ui;
mod update;

use eframe::egui;

use settings::config::Config;
use ui::app::TypeBridgeApp;

/// Command-line overrides (all optional). Example:
/// `typebridge --delay 25 --wait 5 --file text.txt --minimize --autostart`
pub struct CliArgs {
    pub delay_ms: Option<u32>,
    pub wait_s: Option<u32>,
    pub minimize: Option<bool>,
    pub text: Option<String>,
    pub autostart: bool,
}

fn parse_cli() -> CliArgs {
    parse_cli_args(std::env::args().skip(1))
}

fn parse_cli_args<I: Iterator<Item = String>>(args: I) -> CliArgs {
    let mut cli = CliArgs {
        delay_ms: None,
        wait_s: None,
        minimize: None,
        text: None,
        autostart: false,
    };

    let mut args = args;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--delay" => cli.delay_ms = args.next().and_then(|v| v.parse().ok()),
            "--wait" => cli.wait_s = args.next().and_then(|v| v.parse().ok()),
            "--minimize" => cli.minimize = Some(true),
            "--no-minimize" => cli.minimize = Some(false),
            "--autostart" => cli.autostart = true,
            "--text" => cli.text = args.next(),
            "--file" => {
                if let Some(path) = args.next() {
                    match std::fs::read_to_string(&path) {
                        Ok(t) => cli.text = Some(t),
                        Err(e) => eprintln!("Could not read '{path}': {e}"),
                    }
                }
            }
            _ => {}
        }
    }

    cli
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> std::vec::IntoIter<String> {
        items
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
            .into_iter()
    }

    #[test]
    fn parses_all_flags() {
        let c = parse_cli_args(args(&[
            "--delay", "25", "--wait", "5", "--minimize", "--text", "hi", "--autostart",
        ]));
        assert_eq!(c.delay_ms, Some(25));
        assert_eq!(c.wait_s, Some(5));
        assert_eq!(c.minimize, Some(true));
        assert_eq!(c.text.as_deref(), Some("hi"));
        assert!(c.autostart);
    }

    #[test]
    fn no_minimize_overrides() {
        let c = parse_cli_args(args(&["--no-minimize"]));
        assert_eq!(c.minimize, Some(false));
    }

    #[test]
    fn empty_args_are_all_none() {
        let c = parse_cli_args(std::iter::empty());
        assert!(c.delay_ms.is_none());
        assert!(c.wait_s.is_none());
        assert!(c.minimize.is_none());
        assert!(c.text.is_none());
        assert!(!c.autostart);
    }

    #[test]
    fn bad_numbers_are_ignored() {
        let c = parse_cli_args(args(&["--delay", "abc"]));
        assert!(c.delay_ms.is_none());
    }
}

/// Decode the embedded window/taskbar icon (a transparent rounded-square PNG).
fn load_icon() -> egui::IconData {
    let png = include_bytes!("../assets/icon.png");
    let image = image::load_from_memory(png)
        .expect("embedded icon should decode")
        .into_rgba8();
    let (width, height) = image.dimensions();
    egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

/// A wgpu setup that prefers real hardware but falls back to a software adapter
/// (Direct3D WARP on Windows, llvmpipe on Linux) when none is present — so the
/// app runs over RDP and on VMs without GPU acceleration. `request_adapter`
/// alone never returns the software adapter, so we pick it from the enumerated
/// list ourselves.
fn wgpu_config() -> eframe::egui_wgpu::WgpuConfiguration {
    use eframe::egui_wgpu::{wgpu, NativeAdapterSelectorMethod, WgpuSetup};

    let selector: NativeAdapterSelectorMethod = std::sync::Arc::new(
        |adapters: &[wgpu::Adapter], _surface: Option<&wgpu::Surface<'_>>| {
            // Real hardware if present; otherwise the software adapter (WARP).
            adapters
                .iter()
                .find(|a| a.get_info().device_type != wgpu::DeviceType::Cpu)
                .or_else(|| adapters.first())
                .cloned()
                .ok_or_else(|| "no wgpu adapter found (including software fallback)".to_owned())
        },
    );

    let mut cfg = eframe::egui_wgpu::WgpuConfiguration::default();
    if let WgpuSetup::CreateNew(setup) = &mut cfg.wgpu_setup {
        setup.native_adapter_selector = Some(selector);
    }
    cfg
}

fn main() -> eframe::Result<()> {
    let cfg = Config::load();
    let cli = parse_cli();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("TypeBridge")
            .with_app_id("typebridge")
            .with_icon(std::sync::Arc::new(load_icon()))
            .with_inner_size([cfg.window_width.max(360.0), cfg.window_height.max(420.0)])
            .with_min_inner_size([360.0, 420.0]),
        wgpu_options: wgpu_config(),
        ..Default::default()
    };

    eframe::run_native(
        "TypeBridge",
        options,
        Box::new(move |cc| Ok(Box::new(TypeBridgeApp::new(cc, cfg, cli)))),
    )
}
