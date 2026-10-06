//! What a stream reader thread tells the wrapper while it holds the parser.
//!
//! The reader owns the stream parser outright, so the thread that settles the
//! run cannot ask it anything while a line is being handled. A [`RunScope`] is
//! the channel that stays open:
//!
//! - the wrapper *closes* it at cutoff, after which the reader's late output
//!   and lifecycle events are discarded instead of landing in a finished run
//!   or in the next iteration of a loop;
//! - the reader's event sink *publishes* the run's result into it as soon as
//!   the provider reports one, before it renders anything, so a reader that
//!   stalls afterwards still leaves the wrapper the outcome.
//!
//! The reader enters the scope for its own thread, so the sink and the output
//! layer reach it without a handle threaded through the parser builder.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// The result a provider reported, as far as the reader got.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ResultSnapshot {
    /// The provider reported a completed turn.
    pub(crate) turn_complete: Option<TurnResult>,
    /// The provider reported a terminal error: `(error kind, message)`.
    pub(crate) terminal_error: Option<(String, String)>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct TurnResult {
    pub(crate) provider_status: Option<String>,
    pub(crate) duration_ms: Option<u64>,
}

struct Inner {
    closed: AtomicBool,
    // Held only to copy a small value in or out, never across terminal I/O.
    snapshot: Mutex<ResultSnapshot>,
}

/// One run's channel between its reader thread and the wrapper.
#[derive(Clone)]
pub(crate) struct RunScope(Arc<Inner>);

impl Default for RunScope {
    fn default() -> Self {
        Self(Arc::new(Inner {
            closed: AtomicBool::new(false),
            snapshot: Mutex::new(ResultSnapshot::default()),
        }))
    }
}

thread_local! {
    static CURRENT: RefCell<Option<RunScope>> = const { RefCell::new(None) };
}

/// Leaves the scope the thread entered when dropped.
pub(crate) struct ScopeGuard(());

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        CURRENT.with(|current| *current.borrow_mut() = None);
    }
}

impl RunScope {
    /// Make this the scope of the calling thread until the guard drops.
    pub(crate) fn enter(&self) -> ScopeGuard {
        CURRENT.with(|current| *current.borrow_mut() = Some(self.clone()));
        ScopeGuard(())
    }

    /// End the run for its reader: from now on the reader's output and events
    /// are discarded.
    pub(crate) fn close(&self) {
        self.0.closed.store(true, Ordering::Release);
    }

    pub(crate) fn snapshot(&self) -> ResultSnapshot {
        self.0
            .snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// Whether the calling thread's run has ended. False on a thread with no scope.
pub(crate) fn current_closed() -> bool {
    CURRENT.with(|current| {
        current
            .borrow()
            .as_ref()
            .is_some_and(|scope| scope.0.closed.load(Ordering::Acquire))
    })
}

/// Record a result fact in the calling thread's scope, if it has one.
pub(crate) fn publish(update: impl FnOnce(&mut ResultSnapshot)) {
    CURRENT.with(|current| {
        if let Some(scope) = current.borrow().as_ref() {
            update(
                &mut scope
                    .0
                    .snapshot
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner()),
            );
        }
    });
}

#[cfg(test)]
mod tests;
