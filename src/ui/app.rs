//! The egui application: state, layout and the update loop.

use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use eframe::egui::{self, Color32};

use crate::clipboard::clipboard as clip;
use crate::settings::config::Config;
use crate::typing::worker::{self, TypingJob, WorkerMsg};
use crate::CliArgs;

use super::widgets;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Waiting,
    Typing,
    Finished,
    Cancelled,
    Error,
}

pub struct TypeBridgeApp {
    // --- user inputs / settings ---
    text: String,
    delay_ms: u32,
    initial_delay_s: u32,
    minimize: bool,

    // --- runtime state ---
    phase: Phase,
    error_msg: Option<String>,
    typed: usize,
    total: usize,
    wait_remaining_ms: u64,

    job: Option<TypingJob>,
    minimized_for_job: bool,
    autostart_pending: bool,
}

impl TypeBridgeApp {
    pub fn new(cc: &eframe::CreationContext<'_>, cfg: Config, cli: CliArgs) -> Self {
        // A touch more breathing room than the default theme.
        cc.egui_ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 8.0);
            s.spacing.button_padding = egui::vec2(10.0, 6.0);
        });

        let mut app = Self {
            text: String::new(),
            delay_ms: cfg.delay_ms.clamp(1, 2000),
            initial_delay_s: cfg.initial_delay_s.min(60),
            minimize: cfg.minimize_before_typing,
            phase: Phase::Idle,
            error_msg: None,
            typed: 0,
            total: 0,
            wait_remaining_ms: 0,
            job: None,
            minimized_for_job: false,
            autostart_pending: false,
        };

        // Apply CLI overrides.
        if let Some(d) = cli.delay_ms {
            app.delay_ms = d.clamp(1, 2000);
        }
        if let Some(w) = cli.wait_s {
            app.initial_delay_s = w.min(60);
        }
        if let Some(m) = cli.minimize {
            app.minimize = m;
        }
        if let Some(text) = cli.text {
            app.text = text;
        }
        app.autostart_pending = cli.autostart && !app.text.trim().is_empty();

        app
    }

    fn running(&self) -> bool {
        self.job.is_some()
    }

    fn start_typing(&mut self, ctx: &egui::Context) {
        if self.text.trim().is_empty() {
            self.error_msg = Some("No text to type.".to_owned());
            self.phase = Phase::Error;
            return;
        }

        self.error_msg = None;
        self.typed = 0;
        self.total = self.text.chars().count();
        self.phase = Phase::Waiting;
        self.wait_remaining_ms = (self.initial_delay_s as u64) * 1000;

        // Persist current settings before we (possibly) minimize.
        self.save_config(ctx);

        self.minimized_for_job = self.minimize;
        if self.minimize {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        let ctx_repaint = ctx.clone();
        let job = worker::start(
            self.text.clone(),
            self.delay_ms,
            self.initial_delay_s,
            move || ctx_repaint.request_repaint(),
        );
        self.job = Some(job);
    }

    fn cancel_job(&self) {
        if let Some(job) = &self.job {
            job.cancel.cancel();
        }
    }

    /// Drain worker messages and update state.
    fn poll_job(&mut self, ctx: &egui::Context) {
        let Some(job) = self.job.take() else {
            return;
        };

        let mut terminal: Option<Phase> = None;
        loop {
            match job.rx.try_recv() {
                Ok(WorkerMsg::Waiting { remaining_ms }) => {
                    self.phase = Phase::Waiting;
                    self.wait_remaining_ms = remaining_ms;
                }
                Ok(WorkerMsg::Progress { typed, total }) => {
                    self.phase = Phase::Typing;
                    self.typed = typed;
                    self.total = total;
                }
                Ok(WorkerMsg::Finished) => terminal = Some(Phase::Finished),
                Ok(WorkerMsg::Cancelled) => terminal = Some(Phase::Cancelled),
                Ok(WorkerMsg::Error(e)) => {
                    self.error_msg = Some(e);
                    terminal = Some(Phase::Error);
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => {
                    if terminal.is_none() {
                        terminal = Some(Phase::Finished);
                    }
                    break;
                }
            }
        }

        match terminal {
            Some(end) => {
                self.phase = end;
                // job dropped here -> thread detached (already finishing).
                if self.minimized_for_job {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    self.minimized_for_job = false;
                }
            }
            None => {
                // Still running: put it back.
                self.job = Some(job);
            }
        }
    }

    fn save_config(&self, ctx: &egui::Context) {
        let size = ctx.input(|i| i.screen_rect().size());
        let cfg = Config {
            delay_ms: self.delay_ms,
            initial_delay_s: self.initial_delay_s,
            minimize_before_typing: self.minimize,
            window_width: size.x,
            window_height: size.y,
        };
        let _ = cfg.save();
    }

    fn status(&self) -> (String, Color32) {
        match self.phase {
            Phase::Idle => ("Ready".to_owned(), Color32::from_rgb(120, 160, 220)),
            Phase::Waiting => (
                format!("Waiting... {:.1}s", self.wait_remaining_ms as f32 / 1000.0),
                Color32::from_rgb(230, 170, 60),
            ),
            Phase::Typing => ("Typing...".to_owned(), Color32::from_rgb(90, 190, 120)),
            Phase::Finished => ("Finished".to_owned(), Color32::from_rgb(90, 200, 110)),
            Phase::Cancelled => ("Cancelled".to_owned(), Color32::from_rgb(220, 180, 70)),
            Phase::Error => ("Error".to_owned(), Color32::from_rgb(225, 90, 90)),
        }
    }

    fn body(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let running = self.running();

        ui.heading("TypeBridge");
        ui.label(
            egui::RichText::new("Types text into the focused window — no clipboard, no paste.")
                .weak(),
        );
        ui.add_space(4.0);

        // ---- Text ----
        ui.label("Text");
        ui.add_enabled(
            !running,
            egui::TextEdit::multiline(&mut self.text)
                .desired_rows(12)
                .desired_width(f32::INFINITY)
                .hint_text("Type or paste the text to send..."),
        );
        ui.label(
            egui::RichText::new(format!("{} characters", self.text.chars().count())).weak(),
        );

        ui.add_space(4.0);

        // ---- Timing ----
        ui.horizontal(|ui| {
            ui.label("Delay between keys");
            ui.add_enabled(
                !running,
                egui::DragValue::new(&mut self.delay_ms)
                    .range(1..=2000)
                    .speed(1.0)
                    .suffix(" ms"),
            );
        });
        ui.horizontal(|ui| {
            ui.label("Initial delay");
            ui.add_enabled(
                !running,
                egui::DragValue::new(&mut self.initial_delay_s)
                    .range(0..=60)
                    .speed(0.1)
                    .suffix(" s"),
            );
            ui.label(egui::RichText::new("(time to switch to the target window)").weak());
        });

        // ---- Speed presets ----
        ui.horizontal(|ui| {
            ui.label("Presets:");
            for (name, ms) in [
                ("Very fast", 2u32),
                ("Fast", 8),
                ("Normal", 20),
                ("Slow", 60),
                ("Very slow", 150),
            ] {
                let selected = self.delay_ms == ms;
                if ui
                    .add_enabled(!running, egui::SelectableLabel::new(selected, name))
                    .clicked()
                {
                    self.delay_ms = ms;
                }
            }
        });

        ui.add_enabled(
            !running,
            egui::Checkbox::new(&mut self.minimize, "Minimize window before typing"),
        );

        ui.separator();

        // ---- Actions ----
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!running, egui::Button::new("📋 Paste Clipboard"))
                .clicked()
            {
                match clip::get_text() {
                    Ok(t) => {
                        self.text = t;
                        self.error_msg = None;
                        if !running {
                            self.phase = Phase::Idle;
                        }
                    }
                    Err(e) => {
                        self.error_msg = Some(e);
                        self.phase = Phase::Error;
                    }
                }
            }
            if ui
                .add_enabled(!running && !self.text.is_empty(), egui::Button::new("🗑 Clear"))
                .clicked()
            {
                self.text.clear();
                self.phase = Phase::Idle;
                self.error_msg = None;
            }
        });

        ui.add_space(2.0);

        if running {
            if ui
                .add_sized([ui.available_width(), 38.0], egui::Button::new("■  Cancel (Esc)"))
                .clicked()
            {
                self.cancel_job();
            }
        } else if ui
            .add_sized([ui.available_width(), 38.0], egui::Button::new("▶  Start Typing"))
            .clicked()
        {
            self.start_typing(ctx);
        }

        ui.separator();

        // ---- Status ----
        let (label, color) = self.status();
        ui.horizontal(|ui| {
            ui.label("Status:");
            widgets::status_badge(ui, &label, color);
        });

        if self.phase == Phase::Typing && self.total > 0 {
            let frac = self.typed as f32 / self.total as f32;
            ui.add(
                egui::ProgressBar::new(frac)
                    .text(format!("{}/{}", self.typed, self.total))
                    .desired_width(f32::INFINITY),
            );
        }

        if let Some(err) = &self.error_msg {
            ui.colored_label(Color32::from_rgb(225, 110, 110), err);
        }
    }
}

impl eframe::App for TypeBridgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_job(ctx);

        // CLI autostart (once).
        if self.autostart_pending && !self.running() && self.phase == Phase::Idle {
            self.autostart_pending = false;
            self.start_typing(ctx);
        }

        // Esc cancels while the window is focused; the worker also watches the
        // physical Esc key for when we are minimized.
        if self.running() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.cancel_job();
        }

        // Keep the UI live while a job runs (we may be minimized).
        if self.running() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        // Persist settings (incl. window size) on close.
        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_config(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            self.body(ui, ctx);
        });
    }
}
