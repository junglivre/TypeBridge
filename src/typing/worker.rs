//! Background typing worker.
//!
//! The whole flow (initial delay countdown, typing, optional focus-change
//! pause/resume) runs on a dedicated thread so the UI never freezes. It talks
//! back to the UI through a channel.
//!
//! ```text
//! UI ──spawn──► Worker thread ──messages──► UI status updates
//!    ◄──cancel / resume signals──
//! ```

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::cancel::{CancelToken, EscWatcher};
use super::engine::Typer;
use super::window;

/// Messages sent from the worker thread to the UI.
pub enum WorkerMsg {
    /// Counting down the initial delay; `remaining_ms` until typing starts.
    Waiting { remaining_ms: u64 },
    /// `typed` of `total` characters processed.
    Progress { typed: usize, total: usize },
    /// The focused window changed: typing is paused, awaiting resume/cancel.
    WindowChanged { typed: usize, total: usize },
    Finished,
    Cancelled,
    Error(String),
}

/// Parameters for a typing job.
pub struct JobConfig {
    pub text: String,
    pub delay_ms: u32,
    pub initial_delay_s: u32,
    pub detect_window_change: bool,
    pub physical_keys: bool,
}

/// Handle to a running typing job.
pub struct TypingJob {
    pub rx: Receiver<WorkerMsg>,
    pub cancel: CancelToken,
    /// Set to `true` by the UI to resume after a focus-change pause.
    pub resume: Arc<AtomicBool>,
    /// Kept so the thread is owned by the job; detached on drop.
    pub _handle: JoinHandle<()>,
}

/// Spawn a typing job. `repaint` is called whenever a new message is available
/// so the (possibly minimized / unfocused) UI wakes up and drains the channel.
pub fn start(cfg: JobConfig, repaint: impl Fn() + Send + 'static) -> TypingJob {
    let (tx, rx) = mpsc::channel();
    let cancel = CancelToken::new();
    let resume = Arc::new(AtomicBool::new(false));

    let cancel_t = cancel.clone();
    let resume_t = resume.clone();
    let handle = thread::spawn(move || run_job(cfg, tx, cancel_t, resume_t, repaint));

    TypingJob {
        rx,
        cancel,
        resume,
        _handle: handle,
    }
}

fn run_job(
    cfg: JobConfig,
    tx: Sender<WorkerMsg>,
    cancel: CancelToken,
    resume: Arc<AtomicBool>,
    repaint: impl Fn(),
) {
    let esc = EscWatcher::new();
    let chars: Vec<char> = cfg.text.chars().collect();
    let total = chars.len();
    let delay = Duration::from_millis(cfg.delay_ms as u64);
    let initial = Duration::from_secs(cfg.initial_delay_s as u64);
    let step = (total / 200).max(1);

    // ---- Initial delay countdown -----------------------------------------
    if !countdown(initial, &cancel, &esc, &tx, &repaint) {
        let _ = tx.send(WorkerMsg::Cancelled);
        repaint();
        return;
    }

    let mut typer = match Typer::new(cfg.physical_keys) {
        Ok(t) => t,
        Err(e) => {
            let _ = tx.send(WorkerMsg::Error(e.to_string()));
            repaint();
            return;
        }
    };

    // Remember the window we are typing into (if detection is on).
    let mut target = if cfg.detect_window_change {
        window::foreground_window()
    } else {
        None
    };

    // ---- Typing loop ------------------------------------------------------
    let mut i = 0;
    while i < total {
        if cancel.is_cancelled() || esc.esc_pressed() {
            let _ = tx.send(WorkerMsg::Cancelled);
            repaint();
            return;
        }

        // Pause if focus left the target window.
        if cfg.detect_window_change {
            if let (Some(t), Some(cur)) = (target, window::foreground_window()) {
                if t != cur {
                    let _ = tx.send(WorkerMsg::WindowChanged { typed: i, total });
                    repaint();

                    // Wait for the UI to resume or cancel.
                    resume.store(false, Ordering::SeqCst);
                    loop {
                        if cancel.is_cancelled() || esc.esc_pressed() {
                            let _ = tx.send(WorkerMsg::Cancelled);
                            repaint();
                            return;
                        }
                        if resume.swap(false, Ordering::SeqCst) {
                            break;
                        }
                        thread::sleep(Duration::from_millis(50));
                    }

                    // Fresh countdown so the user can refocus the target.
                    if !countdown(initial, &cancel, &esc, &tx, &repaint) {
                        let _ = tx.send(WorkerMsg::Cancelled);
                        repaint();
                        return;
                    }
                    target = window::foreground_window();
                    continue; // re-check before typing the same character
                }
            }
        }

        if let Err(e) = typer.send(chars[i]) {
            let _ = tx.send(WorkerMsg::Error(e.to_string()));
            repaint();
            return;
        }
        i += 1;

        if i % step == 0 || i == total {
            let _ = tx.send(WorkerMsg::Progress { typed: i, total });
            repaint();
        }

        if !sleep_cancellable(delay, &cancel, &esc) {
            let _ = tx.send(WorkerMsg::Cancelled);
            repaint();
            return;
        }
    }

    let _ = tx.send(WorkerMsg::Finished);
    repaint();
}

/// Count down for `dur`, emitting `Waiting` messages. Returns `false` if
/// cancelled during the wait.
fn countdown<R: Fn()>(
    dur: Duration,
    cancel: &CancelToken,
    esc: &EscWatcher,
    tx: &Sender<WorkerMsg>,
    repaint: &R,
) -> bool {
    let start = Instant::now();
    while start.elapsed() < dur {
        if cancel.is_cancelled() || esc.esc_pressed() {
            return false;
        }
        let remaining = dur.saturating_sub(start.elapsed());
        let _ = tx.send(WorkerMsg::Waiting {
            remaining_ms: remaining.as_millis() as u64,
        });
        repaint();
        thread::sleep(Duration::from_millis(50));
    }
    !(cancel.is_cancelled() || esc.esc_pressed())
}

/// Sleep for `dur`, split into small slices so cancellation is responsive even
/// with long per-key delays. Returns `false` if cancelled during the wait.
fn sleep_cancellable(dur: Duration, cancel: &CancelToken, esc: &EscWatcher) -> bool {
    const SLICE: Duration = Duration::from_millis(10);
    let mut remaining = dur;
    while remaining > Duration::ZERO {
        if cancel.is_cancelled() || esc.esc_pressed() {
            return false;
        }
        let stepped = remaining.min(SLICE);
        thread::sleep(stepped);
        remaining = remaining.saturating_sub(stepped);
    }
    !(cancel.is_cancelled() || esc.esc_pressed())
}
