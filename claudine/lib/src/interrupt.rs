//! Process-scoped user-interrupt flag shared across the lib + CLI.
//!
//! The CLI's SIGINT guard sets this flag on the first Ctrl+C so the rest
//! of the lib (especially lifecycle side effects like messenger sends and
//! durable audio publication) can short-circuit instead of
//! running for several more seconds before the next ^C is checked.

use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

static USER_INTERRUPTED: AtomicBool = AtomicBool::new(false);

/// Nesting depth of in-flight terminal lifecycle events (`success`,
/// `blocked`, `failure`, `finalize`). A counter so nested runs stay correct.
static TERMINAL_LIFECYCLE_DEPTH: AtomicUsize = AtomicUsize::new(0);

/// Mark that a user interrupt was observed in this process.
///
/// Safe to call from a signal handler (atomic store, no allocation).
pub fn mark_interrupted() {
    USER_INTERRUPTED.store(true, Ordering::SeqCst);
}

/// Returns `true` once a Ctrl+C has been observed in this process.
pub fn interrupted() -> bool {
    USER_INTERRUPTED.load(Ordering::SeqCst)
}

/// Returns `true` while a terminal lifecycle event is running.
///
/// The CLI's SIGINT guard reads this (a single atomic load, async-signal-safe)
/// to give a repeat Ctrl+C a short grace window instead of force-exiting in
/// the middle of the run's `failure`/`finalize` side effects.
pub fn terminal_lifecycle_active() -> bool {
    TERMINAL_LIFECYCLE_DEPTH.load(Ordering::SeqCst) > 0
}

/// RAII marker held for the duration of one terminal lifecycle event.
///
/// The decrement on `Drop` fires on every exit path, so the flag cannot stick.
pub struct TerminalLifecycleScope(());

impl TerminalLifecycleScope {
    /// Mark a terminal lifecycle event as running until the scope drops.
    pub fn enter() -> Self {
        TERMINAL_LIFECYCLE_DEPTH.fetch_add(1, Ordering::SeqCst);
        Self(())
    }
}

impl Drop for TerminalLifecycleScope {
    fn drop(&mut self) {
        TERMINAL_LIFECYCLE_DEPTH.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Reset the interrupt flag. Used only by tests that need a clean
/// observable state across assertions.
#[doc(hidden)]
pub fn clear_for_tests() {
    USER_INTERRUPTED.store(false, Ordering::SeqCst);
}
