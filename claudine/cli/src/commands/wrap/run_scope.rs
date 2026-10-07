//! What a stream reader thread tells the wrapper while it holds the parser.
//!
//! The reader owns the stream parser outright, so the thread that settles the
//! run cannot ask it anything while a line is being handled. A [`RunScope`] is
//! the channel that stays open:
//!
//! - the wrapper *closes* it at cutoff, after which the reader's late output
//!   and lifecycle events are discarded instead of landing in a finished run
//!   or in the next iteration of a loop;
//! - once the provider has reported a result or a terminal error, the reader
//!   publishes the parser's own finalized summary into it after every line
//!   ([`feed_line`]): the answer, session, usage, and the provider's verdict;
//! - the sink work a line causes (rendering, logging, lifecycle hooks) runs
//!   only *after* that publication ([`DeferredSink`]), so a reader that stalls
//!   in a completion callback still leaves the wrapper the complete outcome.
//!
//! The reader enters the scope for its own thread, so the sink and the output
//! layer reach it without a handle threaded through the parser builder.

use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use claudine::stream::parser::SemanticStreamParser;
use claudine::stream::semantic::{SemanticEvent, SemanticEventSink};
use claudine::stream::summary::StreamExecutionSummary;

/// The result a provider reported, as far as the reader got.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ResultSnapshot {
    /// A terminal error reported by either reader (a stderr bridge reports
    /// through the same sink): `(error kind, message)`.
    pub(crate) terminal_error: Option<(String, String)>,
    /// The parser's [`SemanticStreamParser::snapshot`] at exit 0, taken after
    /// every line the reader parsed once a result or terminal error was
    /// reported. `None` until the stdout reader has parsed such a line.
    pub(crate) summary: Option<StreamExecutionSummary>,
}

struct Inner {
    closed: AtomicBool,
    /// A result or terminal error has been reported, so [`feed_line`]
    /// publishes the parser's summary.
    reported: AtomicBool,
    // Held only to copy a value in or out, never across terminal I/O or while
    // the parser computes a snapshot.
    snapshot: Mutex<ResultSnapshot>,
}

/// One run's channel between its reader thread and the wrapper.
#[derive(Clone)]
pub(crate) struct RunScope(Arc<Inner>);

impl Default for RunScope {
    fn default() -> Self {
        Self(Arc::new(Inner {
            closed: AtomicBool::new(false),
            reported: AtomicBool::new(false),
            snapshot: Mutex::new(ResultSnapshot::default()),
        }))
    }
}

thread_local! {
    static CURRENT: RefCell<Option<RunScope>> = const { RefCell::new(None) };
    /// Sink work a [`DeferredSink`] received while this thread is inside
    /// [`feed_line`]; `None` outside it, where the work runs at once.
    static DEFERRED: RefCell<Option<Vec<SinkWork>>> = const { RefCell::new(None) };
}

type SinkWork = Box<dyn FnOnce()>;

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

/// Feed `line` to the reader's parser; once the provider has reported a result
/// or a terminal error, publish the parser's finalized view of the run, and
/// only then run the sink work the line caused.
///
/// The child's exit code is not known yet, so the view is finalized at exit 0:
/// success is preserved only for a child that exits 0, and settlement keeps
/// the real exit code otherwise.
pub(crate) fn feed_line(parser: &mut dyn SemanticStreamParser, line: &str) {
    DEFERRED.with(|deferred| *deferred.borrow_mut() = Some(Vec::new()));
    parser.feed_line(line);
    let work = DEFERRED.with(|deferred| deferred.borrow_mut().take()).unwrap_or_default();
    let publish = CURRENT.with(|current| {
        current.borrow().as_ref().is_some_and(|scope| {
            !scope.0.closed.load(Ordering::Acquire) && scope.0.reported.load(Ordering::Acquire)
        })
    });
    if publish {
        let summary = parser.snapshot(0);
        update(|snapshot| snapshot.summary = Some(summary));
    }
    for work in work {
        work();
    }
}

/// Record a terminal error in the calling thread's scope, if it has one.
pub(crate) fn report_terminal_error(kind: String, message: String) {
    mark_reported();
    update(|snapshot| snapshot.terminal_error = Some((kind, message)));
}

fn mark_reported() {
    CURRENT.with(|current| {
        if let Some(scope) = current.borrow().as_ref() {
            scope.0.reported.store(true, Ordering::Release);
        }
    });
}

fn update(change: impl FnOnce(&mut ResultSnapshot)) {
    CURRENT.with(|current| {
        if let Some(scope) = current.borrow().as_ref() {
            change(&mut scope.0.snapshot.lock().unwrap_or_else(PoisonError::into_inner));
        }
    });
}

/// The sink a reader's parser emits into: inside [`feed_line`] it holds each
/// event until the line's summary is published, then hands it to the wrapped
/// sink; outside it (a parser fed directly, `finish`) it hands events on at
/// once.
///
/// A result or terminal error marks the scope as reported the moment the
/// parser emits it, before any sink work runs.
pub(crate) struct DeferredSink<S>(Arc<Mutex<S>>);

impl<S: SemanticEventSink + 'static> DeferredSink<S> {
    pub(crate) fn new(sink: S) -> Self {
        Self(Arc::new(Mutex::new(sink)))
    }
}

impl<S: SemanticEventSink + 'static> SemanticEventSink for DeferredSink<S> {
    fn on_semantic_event(&mut self, event: SemanticEvent) {
        if matches!(
            event,
            SemanticEvent::TurnComplete { .. } | SemanticEvent::Error { terminal: true, .. }
        ) {
            mark_reported();
        }
        let sink = Arc::clone(&self.0);
        let work = move || {
            sink.lock()
                .unwrap_or_else(PoisonError::into_inner)
                .on_semantic_event(event);
        };
        let work = DEFERRED.with(|deferred| match deferred.borrow_mut().as_mut() {
            Some(queue) => {
                queue.push(Box::new(work));
                None
            }
            None => Some(work),
        });
        if let Some(work) = work {
            work();
        }
    }
}

#[cfg(test)]
mod tests;
