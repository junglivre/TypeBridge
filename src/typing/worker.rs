//! Background typing worker.
//!
//! The whole flow (initial delay countdown + typing) runs on a dedicated
//! thread so the UI never freezes. It talks back to the UI through a channel.
//!
//! ```text
//! UI ──spawn──► Worker thread ──messages──► UI status updates
//! ```

use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use super::cancel::{CancelToken, EscWatcher};
use super::engine::{self, Outcome};

/// Messages sent from the worker thread to the UI.
pub enum WorkerMsg {
    /// Counting down the initial delay; `remaining_ms` until typing starts.
    Waiting { remaining_ms: u64 },
    /// `typed` of `total` characters processed.
    Progress { typed: usize, total: usize },
    Finished,
    Cancelled,
    Error(String),
}

/// Handle to a running typing job.
pub struct TypingJob {
    pub rx: Receiver<WorkerMsg>,
    pub cancel: CancelToken,
    /// Kept so the thread is owned by the job; detached on drop.
    pub _handle: JoinHandle<()>,
}

/// Spawn a typing job. `repaint` is called whenever a new message is available
/// so the (possibly minimized / unfocused) UI wakes up and drains the channel.
pub fn start(
    text: String,
    delay_ms: u32,
    initial_delay_s: u32,
    repaint: impl Fn() + Send + 'static,
) -> TypingJob {
    let (tx, rx) = mpsc::channel();
    let cancel = CancelToken::new();
    let cancel_for_thread = cancel.clone();

    let handle = thread::spawn(move || {
        run_job(text, delay_ms, initial_delay_s, tx, cancel_for_thread, repaint);
    });

    TypingJob {
        rx,
        cancel,
        _handle: handle,
    }
}

fn run_job(
    text: String,
    delay_ms: u32,
    initial_delay_s: u32,
    tx: Sender<WorkerMsg>,
    cancel: CancelToken,
    repaint: impl Fn(),
) {
    let esc = EscWatcher::new();
    let total = text.chars().count();

    // ---- Initial delay countdown -----------------------------------------
    let total_wait = Duration::from_secs(initial_delay_s as u64);
    let start = Instant::now();
    while start.elapsed() < total_wait {
        if cancel.is_cancelled() || esc.esc_pressed() {
            let _ = tx.send(WorkerMsg::Cancelled);
            repaint();
            return;
        }
        let remaining = total_wait.saturating_sub(start.elapsed());
        let _ = tx.send(WorkerMsg::Waiting {
            remaining_ms: remaining.as_millis() as u64,
        });
        repaint();
        thread::sleep(Duration::from_millis(50));
    }

    // ---- Typing -----------------------------------------------------------
    let delay = Duration::from_millis(delay_ms as u64);
    let step = (total / 200).max(1); // throttle progress messages
    let result = {
        let tx_progress = tx.clone();
        let repaint = &repaint;
        engine::run(&text, delay, &cancel, &esc, move |typed| {
            if typed % step == 0 || typed == total {
                let _ = tx_progress.send(WorkerMsg::Progress { typed, total });
                repaint();
            }
        })
    };

    match result {
        Ok(Outcome::Finished) => {
            let _ = tx.send(WorkerMsg::Finished);
        }
        Ok(Outcome::Cancelled) => {
            let _ = tx.send(WorkerMsg::Cancelled);
        }
        Err(e) => {
            let _ = tx.send(WorkerMsg::Error(e.to_string()));
        }
    }
    repaint();
}
