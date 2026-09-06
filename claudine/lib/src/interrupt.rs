//! Process-scoped user-interrupt flag shared across the lib + CLI.
//!
//! The CLI's SIGINT guard sets this flag on the first Ctrl+C so the rest
//! of the lib (especially lifecycle side effects like messenger sends and
//! durable audio publication) can short-circuit instead of
//! running for several more seconds before the next ^C is checked.

use std::sync::atomic::{AtomicBool, Ordering};

static USER_INTERRUPTED: AtomicBool = AtomicBool::new(false);

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

/// Reset the interrupt flag. Used only by tests that need a clean
/// observable state across assertions.
#[doc(hidden)]
pub fn clear_for_tests() {
    USER_INTERRUPTED.store(false, Ordering::SeqCst);
}
