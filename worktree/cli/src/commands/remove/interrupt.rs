//! Ctrl-C and termination requests while the removal lifecycle runs.
//!
//! While a [`Window`] is open, an interrupt (`SIGINT`, `SIGTERM`, or `SIGHUP`
//! on Unix; any console control event on Windows) does not end the process:
//! it sets a flag that the lifecycle polls at every deletion entry and in
//! every wait, so the removal stops there and keeps its compact record and
//! whatever is left, which the next `wt remove` of that name finishes
//! without asking.
//!
//! The process-wide handler is installed when the first window opens, since
//! `ctrlc` allows one per process. Before that, an interrupt keeps the
//! platform default; after a window closes, the handler exits with
//! [`INTERRUPTED_EXIT`] instead.

use std::io::Write as _;
use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, Ordering};

static ARMED: AtomicBool = AtomicBool::new(false);
static INTERRUPTED: AtomicBool = AtomicBool::new(false);
static INSTALLED: OnceLock<bool> = OnceLock::new();

/// The exit code of an interrupt outside a window: what a shell reports for
/// a process killed by `SIGINT`.
const INTERRUPTED_EXIT: i32 = 130;

/// The span in which an interrupt stops the removal at its next safe point
/// instead of ending the process. Closed on drop.
pub struct Window(());

impl Window {
    /// Opens a window with no interrupt yet received. When the handler
    /// cannot be installed (another library installed one first), interrupts
    /// keep their default effect and the removal relies on restart recovery.
    pub fn open() -> Self {
        INSTALLED.get_or_init(|| ctrlc::try_set_handler(on_interrupt).is_ok());
        INTERRUPTED.store(false, Ordering::SeqCst);
        ARMED.store(true, Ordering::SeqCst);
        Self(())
    }

    /// Whether an interrupt arrived since the window opened; the lifecycle's
    /// [`Env::cancelled`](worktree::remove::confirmed::Env::cancelled).
    pub fn interrupted(&self) -> bool {
        INTERRUPTED.load(Ordering::SeqCst)
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        ARMED.store(false, Ordering::SeqCst);
    }
}

/// The handler `ctrlc` runs on its own thread for every interrupt. The notice
/// is written once, and a failed write (a closed terminal) is ignored.
fn on_interrupt() {
    if !ARMED.load(Ordering::SeqCst) {
        std::process::exit(INTERRUPTED_EXIT);
    }
    if !INTERRUPTED.swap(true, Ordering::SeqCst) {
        let _ = writeln!(
            std::io::stderr(),
            "\nInterrupted: stopping at the next safe point; files not yet deleted are kept."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each window starts clear, and an interrupt received while it is open
    /// is what the lifecycle's predicate reports. The handler body is called
    /// directly: delivering a real signal is the binary test's job.
    #[test]
    fn an_interrupt_in_an_open_window_sets_the_lifecycle_predicate() {
        let window = Window::open();
        assert!(!window.interrupted());

        on_interrupt();

        assert!(window.interrupted());
        on_interrupt();
        assert!(window.interrupted(), "a second interrupt does not clear the first");
        drop(window);

        let window = Window::open();
        assert!(!window.interrupted(), "a new window does not inherit an earlier interrupt");
    }
}
