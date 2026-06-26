//! The egui application: state, layout and the update loop.

use std::sync::atomic::Ordering;
use std::sync::mpsc::TryRecvError;
use std::time::Duration;

use eframe::egui::{self, Color32};

use crate::clipboard::clipboard as clip;
use crate::i18n::Lang;
use crate::settings::config::Config;
use crate::typing::engine::KeyMode;
use crate::typing::window;
use crate::typing::worker::{self, JobConfig, TypingJob, WorkerMsg};
use crate::CliArgs;

use super::widgets;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Waiting,
    Typing,
    Paused,
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
    detect_window_change: bool,
    key_mode: KeyMode,
    lang: Lang,

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
        cc.egui_ctx.style_mut(|s| {
            s.spacing.item_spacing = egui::vec2(8.0, 8.0);
            s.spacing.button_padding = egui::vec2(10.0, 6.0);
        });

        let mut app = Self {
            text: String::new(),
            delay_ms: cfg.delay_ms.clamp(1, 2000),
            initial_delay_s: cfg.initial_delay_s.min(60),
            minimize: cfg.minimize_before_typing,
            detect_window_change: cfg.detect_window_change,
            key_mode: cfg.key_mode,
            lang: cfg.language,
            phase: Phase::Idle,
            error_msg: None,
            typed: 0,
            total: 0,
            wait_remaining_ms: 0,
            job: None,
            minimized_for_job: false,
            autostart_pending: false,
        };

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
            self.error_msg = Some(self.lang.s().no_text.to_owned());
            self.phase = Phase::Error;
            return;
        }

        self.error_msg = None;
        self.typed = 0;
        self.total = self.text.chars().count();
        self.phase = Phase::Waiting;
        self.wait_remaining_ms = (self.initial_delay_s as u64) * 1000;

        self.save_config(ctx);

        self.minimized_for_job = self.minimize;
        if self.minimize {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }

        let ctx_repaint = ctx.clone();
        let job = worker::start(
            JobConfig {
                text: self.text.clone(),
                delay_ms: self.delay_ms,
                initial_delay_s: self.initial_delay_s,
                detect_window_change: self.detect_window_change,
                key_mode: self.key_mode,
            },
            move || ctx_repaint.request_repaint(),
        );
        self.job = Some(job);
    }

    /// Resume after a focus-change pause (re-runs the initial countdown so the
    /// user can refocus the target window).
    fn resume_job(&mut self, ctx: &egui::Context) {
        let minimize = self.minimize;
        if let Some(job) = &self.job {
            if minimize {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            job.resume.store(true, Ordering::SeqCst);
        }
        self.phase = Phase::Waiting;
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
        let mut just_paused = false;
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
                Ok(WorkerMsg::WindowChanged { typed, total }) => {
                    self.phase = Phase::Paused;
                    self.typed = typed;
                    self.total = total;
                    just_paused = true;
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
                if self.minimized_for_job {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    self.minimized_for_job = false;
                }
            }
            None => {
                // Bring our window forward so the user sees the pause banner.
                if just_paused {
                    ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
                    ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(
                        egui::UserAttentionType::Critical,
                    ));
                }
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
            detect_window_change: self.detect_window_change,
            key_mode: self.key_mode,
            language: self.lang,
            window_width: size.x,
            window_height: size.y,
        };
        let _ = cfg.save();
    }

    fn status(&self) -> (String, Color32) {
        let s = self.lang.s();
        match self.phase {
            Phase::Idle => (s.ready.to_owned(), Color32::from_rgb(120, 160, 220)),
            Phase::Waiting => (
                format!("{} {:.1}s", s.waiting, self.wait_remaining_ms as f32 / 1000.0),
                Color32::from_rgb(230, 170, 60),
            ),
            Phase::Typing => (s.typing.to_owned(), Color32::from_rgb(90, 190, 120)),
            Phase::Paused => (s.paused.to_owned(), Color32::from_rgb(230, 170, 60)),
            Phase::Finished => (s.finished.to_owned(), Color32::from_rgb(90, 200, 110)),
            Phase::Cancelled => (s.cancelled.to_owned(), Color32::from_rgb(220, 180, 70)),
            Phase::Error => (s.error.to_owned(), Color32::from_rgb(225, 90, 90)),
        }
    }

    fn body(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let s = self.lang.s();
        let running = self.running();

        // ---- Header + language selector ----
        ui.horizontal(|ui| {
            ui.heading("TypeBridge");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let prev = self.lang;
                egui::ComboBox::from_id_salt("lang_combo")
                    .selected_text(self.lang.label())
                    .show_ui(ui, |ui| {
                        for l in Lang::ALL {
                            ui.selectable_value(&mut self.lang, l, l.label());
                        }
                    });
                ui.label(format!("{}:", s.language));
                if self.lang != prev {
                    self.save_config(ctx);
                }
            });
        });
        ui.label(egui::RichText::new(s.subtitle).weak());
        ui.add_space(4.0);

        // ---- Text ----
        ui.label(s.text);
        ui.add_enabled(
            !running,
            egui::TextEdit::multiline(&mut self.text)
                .desired_rows(11)
                .desired_width(f32::INFINITY)
                .hint_text(s.hint),
        );

        // ---- Clipboard / Clear (right below the text field) ----
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!running, egui::Button::new(s.paste_clipboard))
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
                    Err(detail) => {
                        self.error_msg = Some(format!("{} ({detail})", s.clipboard_error));
                        self.phase = Phase::Error;
                    }
                }
            }
            if ui
                .add_enabled(!running && !self.text.is_empty(), egui::Button::new(s.clear))
                .clicked()
            {
                self.text.clear();
                self.phase = Phase::Idle;
                self.error_msg = None;
            }
        });
        ui.label(
            egui::RichText::new(format!("{} {}", self.text.chars().count(), s.characters)).weak(),
        );

        ui.add_space(4.0);

        // ---- Timing ----
        ui.horizontal(|ui| {
            ui.label(s.delay_between_keys);
            ui.add_enabled(
                !running,
                egui::DragValue::new(&mut self.delay_ms)
                    .range(1..=2000)
                    .speed(1.0)
                    .suffix(" ms"),
            );
        });
        ui.horizontal(|ui| {
            ui.label(s.initial_delay);
            ui.add_enabled(
                !running,
                egui::DragValue::new(&mut self.initial_delay_s)
                    .range(0..=60)
                    .speed(0.1)
                    .suffix(" s"),
            );
            ui.label(egui::RichText::new(s.initial_delay_help).weak());
        });

        // ---- Speed presets ----
        ui.horizontal(|ui| {
            ui.label(s.presets);
            for (name, ms) in [
                (s.very_fast, 2u32),
                (s.fast, 8),
                (s.normal, 20),
                (s.slow, 60),
                (s.very_slow, 150),
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

        // ---- Options ----
        if ui
            .add_enabled(!running, egui::Checkbox::new(&mut self.minimize, s.minimize_before))
            .changed()
        {
            self.save_config(ctx);
        }
        ui.horizontal(|ui| {
            let resp = ui.add_enabled(
                !running && window::SUPPORTED,
                egui::Checkbox::new(&mut self.detect_window_change, s.detect_window_change),
            );
            if resp.changed() {
                self.save_config(ctx);
            }
            ui.label(egui::RichText::new(s.detect_window_change_help).weak());
        });
        ui.add_enabled_ui(!running, |ui| {
            ui.horizontal(|ui| {
                ui.label(s.key_method);
                let prev = self.key_mode;
                egui::ComboBox::from_id_salt("key_mode_combo")
                    .selected_text(match self.key_mode {
                        KeyMode::Unicode => s.km_unicode,
                        KeyMode::PhysicalAuto => s.km_physical_auto,
                        KeyMode::PhysicalUs => s.km_physical_us,
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.key_mode, KeyMode::Unicode, s.km_unicode);
                        ui.selectable_value(
                            &mut self.key_mode,
                            KeyMode::PhysicalAuto,
                            s.km_physical_auto,
                        );
                        ui.selectable_value(
                            &mut self.key_mode,
                            KeyMode::PhysicalUs,
                            s.km_physical_us,
                        );
                    });
                if self.key_mode != prev {
                    self.save_config(ctx);
                }
            });
        });
        ui.label(egui::RichText::new(s.key_method_help).weak());

        ui.separator();

        // ---- Start / Cancel ----
        if running {
            if ui
                .add_sized([ui.available_width(), 38.0], egui::Button::new(s.cancel))
                .clicked()
            {
                self.cancel_job();
            }
        } else if ui
            .add_sized([ui.available_width(), 38.0], egui::Button::new(s.start_typing))
            .clicked()
        {
            self.start_typing(ctx);
        }

        ui.separator();

        // ---- Status ----
        let (label, color) = self.status();
        ui.horizontal(|ui| {
            ui.label(s.status);
            widgets::status_badge(ui, &label, color);
        });

        if (self.phase == Phase::Typing || self.phase == Phase::Paused) && self.total > 0 {
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

    /// A prominent modal shown when typing is paused by a focus change.
    fn focus_modal(&mut self, ctx: &egui::Context) {
        let s = self.lang.s();
        let (typed, total) = (self.typed, self.total);
        let mut do_resume = false;
        let mut do_restart = false;

        egui::Modal::new(egui::Id::new("focus_modal")).show(ctx, |ui| {
            ui.set_width(380.0);
            ui.vertical_centered(|ui| {
                widgets::warning_icon(ui, 56.0);
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new(s.window_changed_title)
                        .size(19.0)
                        .strong()
                        .color(Color32::from_rgb(230, 80, 80)),
                );
                ui.add_space(8.0);
                ui.label(s.window_changed_msg);
                if total > 0 {
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new(format!("{typed}/{total}")).weak());
                }
                ui.add_space(16.0);
            });
            ui.horizontal(|ui| {
                if ui
                    .add_sized(
                        [176.0, 36.0],
                        egui::Button::new(egui::RichText::new(s.continue_btn).strong()),
                    )
                    .clicked()
                {
                    do_resume = true;
                }
                if ui
                    .add_sized([176.0, 36.0], egui::Button::new(s.restart_btn))
                    .clicked()
                {
                    do_restart = true;
                }
            });
        });

        if do_resume {
            self.resume_job(ctx);
        }
        if do_restart {
            self.cancel_job();
        }
    }
}

impl eframe::App for TypeBridgeApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_job(ctx);

        if self.autostart_pending && !self.running() && self.phase == Phase::Idle {
            self.autostart_pending = false;
            self.start_typing(ctx);
        }

        // Esc cancels while focused; the worker also watches the physical key.
        if self.running() && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.cancel_job();
        }

        if self.running() {
            ctx.request_repaint_after(Duration::from_millis(50));
        }

        if ctx.input(|i| i.viewport().close_requested()) {
            self.save_config(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.body(ui, ctx);
                });
        });

        // A focus change while typing surfaces a prominent modal.
        if self.phase == Phase::Paused {
            self.focus_modal(ctx);
        }
    }
}
