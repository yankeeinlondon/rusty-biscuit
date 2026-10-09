//! What a stream reader thread tells the wrapper while it holds the parser.
//!
//! The reader owns the stream parser outright, so the thread that settles the
//! run cannot ask it anything while a line is being handled. A [`RunScope`] is
//! the channel that stays open:
//!
//! - the wrapper *closes* it at cutoff, after which the reader's late output
//!   and lifecycle events are discarded instead of landing in a finished run
//!   or in the next iteration of a loop;
//! - raw bytes and identified answer prefixes are retained independently of
//!   the parser, before decoding and callbacks; settlement freezes them under
//!   the publication lock, so late readers cannot change the retained data;
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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Instant;

pub(crate) mod observation;
pub(crate) mod retention;
use observation::{Operation, OperationLane, OperationSnapshot, increment};
use retention::{Retention, RetainedData, CaptureFacts};

#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub(crate) struct CompletionObservation {
    pub(crate) retained: RetainedData,
    pub(crate) verdict_received: bool,
    pub(crate) stdout: Option<OperationSnapshot>,
    pub(crate) stderr: Option<OperationSnapshot>,
    pub(crate) settlement: Option<OperationSnapshot>,
    pub(crate) bytes_received: u64,
    pub(crate) records_processed: u64,
    pub(crate) output_worker: Option<super::output_worker::DeliveryObservation>,
}


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
    retention: Mutex<Retention>,
    frozen: Mutex<Option<CompletionObservation>>,
    stdout: OperationLane,
    stderr: OperationLane,
    settlement: OperationLane,
    bytes: AtomicU64,
    records: AtomicU64,
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
        Self::with_origin(Instant::now())
    }
}

impl RunScope {
    pub(crate) fn with_origin(origin: Instant) -> Self {
        Self(Arc::new(Inner {
            retention: Mutex::new(Retention::default()),
            frozen: Mutex::new(None),
            stdout: OperationLane::new(origin),
            stderr: OperationLane::new(origin),
            settlement: OperationLane::new(origin),
            bytes: AtomicU64::new(0),
            records: AtomicU64::new(0),
            closed: AtomicBool::new(false),
            reported: AtomicBool::new(false),
            snapshot: Mutex::new(ResultSnapshot::default()),
        }))
    }
}

thread_local! {
    static CURRENT: RefCell<Option<RunScope>> = const { RefCell::new(None) };
    static RESPONSE_OBSERVED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static STDERR_LANE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    /// Sink work a [`DeferredSink`] received while this thread is inside
    /// [`feed_line`]; `None` outside it, where the work runs at once.
    static DEFERRED: RefCell<Option<Vec<SinkWork>>> = const { RefCell::new(None) };
}

type SinkWork = Box<dyn FnOnce()>;

/// Leaves the scope the thread entered when dropped.
pub(crate) struct ScopeGuard(Option<RunScope>, bool);

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        CURRENT.with(|current| *current.borrow_mut() = self.0.take());
        STDERR_LANE.with(|lane| lane.set(self.1));
    }
}

impl RunScope {
    /// Make this the scope of the calling thread until the guard drops.
    pub(crate) fn enter(&self) -> ScopeGuard {
        self.enter_lane(false)
    }

    pub(crate) fn enter_stderr(&self) -> ScopeGuard { self.enter_lane(true) }

    fn enter_lane(&self, stderr: bool) -> ScopeGuard {
        let previous = CURRENT.with(|current| current.borrow_mut().replace(self.clone()));
        let previous_lane = STDERR_LANE.with(|lane| lane.replace(stderr));
        ScopeGuard(previous, previous_lane)
    }

    /// End the run for its reader: from now on the reader's output and events
    /// are discarded.
    pub(crate) fn close(&self) {
        let _ = self.freeze();
    }

    pub(crate) fn observe_stdout(&self, operation: Operation) { self.0.stdout.record(operation); }
    pub(crate) fn observe_stderr(&self, operation: Operation) { self.0.stderr.record(operation); }
    pub(crate) fn observe_settlement(&self, operation: Operation) { self.0.settlement.record(operation); }

    pub(crate) fn retain_raw(&self, bytes: &[u8]) {
        let mut retained = self.0.retention.lock().unwrap_or_else(PoisonError::into_inner);
        if !retained.closed {
            retained.raw(bytes);
            increment(&self.0.bytes, bytes.len() as u64);
        }
    }

    pub(crate) fn raw_eof(&self) {
        let mut retained = self.0.retention.lock().unwrap_or_else(PoisonError::into_inner);
        if !retained.closed { retained.eof(); }
    }

    pub(crate) fn retain_answer(&self, text: &str, complete: bool, replace: bool) {
        let mut retained = self.0.retention.lock().unwrap_or_else(PoisonError::into_inner);
        if !retained.closed { retained.answer(text, complete, replace); }
    }

    pub(crate) fn retain_fallback_answer(&self, text: &str, complete: bool) {
        let mut retained = self.0.retention.lock().unwrap_or_else(PoisonError::into_inner);
        if !retained.closed && !retained.has_answer() { retained.answer(text, complete, true); }
    }

    pub(crate) fn capture(&self, facts: CaptureFacts) {
        let mut retained = self.0.retention.lock().unwrap_or_else(PoisonError::into_inner);
        if !retained.closed && facts.path.len() <= 8 * 1024 { retained.capture = Some(facts); }
    }

    /// Closing and moving payloads share the publication lock. Repeated
    /// settlement returns the same frozen value, even after a reader resumes.
    pub(crate) fn freeze(&self) -> CompletionObservation { self.freeze_with_output(None) }

    pub(crate) fn freeze_with_output(&self, output_worker: Option<super::output_worker::DeliveryObservation>) -> CompletionObservation {
        let mut frozen = self.0.frozen.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(frozen) = frozen.as_ref() { return frozen.clone(); }
        let mut retained = self.0.retention.lock().unwrap_or_else(PoisonError::into_inner);
        let snapshot = self.0.snapshot.lock().unwrap_or_else(PoisonError::into_inner);
        let verdict_received = snapshot.summary.is_some() || snapshot.terminal_error.is_some();
        self.0.closed.store(true, Ordering::Release);
        let retained_data = std::mem::replace(&mut *retained, Retention::closed());
        drop(retained);
        drop(snapshot);
        let observation = CompletionObservation {
            retained: retained_data.freeze(),
            verdict_received,
            stdout: self.0.stdout.snapshot(), stderr: self.0.stderr.snapshot(),
            settlement: self.0.settlement.snapshot(),
            bytes_received: self.0.bytes.load(Ordering::Relaxed),
            records_processed: self.0.records.load(Ordering::Relaxed),
            output_worker,
        };
        *frozen = Some(observation.clone());
        observation
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
    RESPONSE_OBSERVED.with(|observed| observed.set(false));
    DEFERRED.with(|deferred| *deferred.borrow_mut() = Some(Vec::new()));
    CURRENT.with(|current| {
        if let Some(scope) = current.borrow().as_ref() { increment(&scope.0.records, 1); }
    });
    observe(Operation::ProviderParse);
    parser.feed_line(line);
    let work = DEFERRED.with(|deferred| deferred.borrow_mut().take()).unwrap_or_default();
    let publish = CURRENT.with(|current| {
        current.borrow().as_ref().is_some_and(|scope| {
            !scope.0.closed.load(Ordering::Acquire) && scope.0.reported.load(Ordering::Acquire)
        })
    });
    if publish {
        observe(Operation::SummaryConstruction);
        let summary = parser.snapshot(0);
        CURRENT.with(|current| {
            if let Some(scope) = current.borrow().as_ref()
                && (!summary.is_error || !summary.assistant_text.is_empty())
            {
                scope.retain_answer(&summary.assistant_text, !summary.is_error, true);
            }
        });
        update(|snapshot| snapshot.summary = Some(summary));
    }
    for work in work {
        if current_closed() { break; }
        observe(Operation::LifecycleCallback);
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
            let retained = scope.0.retention.lock().unwrap_or_else(PoisonError::into_inner);
            if !retained.closed {
                scope.0.reported.store(true, Ordering::Release);
                observe(Operation::VerdictPublication);
            }
        }
    });
}

fn update(change: impl FnOnce(&mut ResultSnapshot)) {
    CURRENT.with(|current| {
        if let Some(scope) = current.borrow().as_ref() {
            let mut snapshot = scope.0.snapshot.lock().unwrap_or_else(PoisonError::into_inner);
            if !scope.0.closed.load(Ordering::Acquire) { change(&mut snapshot); }
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
    fn on_response_text(&mut self, text: &str) {
        RESPONSE_OBSERVED.with(|observed| observed.set(true));
        observe(Operation::AnswerPublication);
        CURRENT.with(|current| {
            if let Some(scope) = current.borrow().as_ref() { scope.retain_answer(text, false, false); }
        });
    }

    fn on_semantic_event(&mut self, event: SemanticEvent) {
        if current_closed() { return; }
        if let SemanticEvent::OutputText { text, .. } = &event
            && !RESPONSE_OBSERVED.with(|observed| observed.replace(false))
        {
            observe(Operation::AnswerPublication);
            CURRENT.with(|current| {
                if let Some(scope) = current.borrow().as_ref() { scope.retain_answer(text, false, false); }
            });
        }
        if matches!(
            event,
            SemanticEvent::TurnComplete { .. } | SemanticEvent::Error { terminal: true, .. }
        ) {
            mark_reported();
        }
        let sink = Arc::clone(&self.0);
        let originating_scope = CURRENT.with(|current| current.borrow().clone());
        let stderr = STDERR_LANE.with(|lane| lane.get());
        let work = move || {
            if originating_scope.as_ref().is_some_and(|scope| scope.0.closed.load(Ordering::Acquire)) { return; }
            let _guard = originating_scope.as_ref().map(|scope| if stderr { scope.enter_stderr() } else { scope.enter() });
            if current_closed() { return; }
            #[cfg(feature = "test-fixtures")]
            super::exec::completion_fixture::event(&event);
            if current_closed() { return; }
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

pub(crate) fn observe(operation: Operation) {
    CURRENT.with(|current| {
        if let Some(scope) = current.borrow().as_ref() {
            if STDERR_LANE.with(|lane| lane.get()) { scope.observe_stderr(operation); }
            else { scope.observe_stdout(operation); }
        }
    });
}

#[cfg(test)]
mod tests;
