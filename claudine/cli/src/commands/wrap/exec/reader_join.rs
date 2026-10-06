//! Joining a child's stream reader threads once the child has exited.
//!
//! A reader thread ends only when it has read its pipe to EOF and finished
//! handling every line, so the wait for it must be bounded: a process the
//! provider started can inherit the pipe and hold it open indefinitely (see
//! `kill_process_group`). But a reader that is not waiting on its pipe is
//! still working through output it already has, typically writing it to a
//! terminal that is slow to drain, and cutting that off turns a successful run
//! into a failure. [`join_reader`] therefore applies the short cap only while
//! the reader is blocked reading its pipe.

use std::any::Any;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use claudine::stream::parser::SemanticStreamParser;
use claudine::stream::summary::StreamExecutionSummary;

/// How long a reader may stay blocked on its pipe after the child exited.
pub(crate) const READER_PIPE_CAP: Duration = Duration::from_secs(5);

/// How long a reader that is not blocked on its pipe may keep working after
/// the child exited.
//
// WHY 120 s: the stalls recorded in the JSONL logs between 2026-09-25 and
// 2026-10-06 left the reader busy for up to about a minute after the child
// exited, while it wrote a large final message to a terminal that was not
// draining. Twice that covers the worst case seen. No `TimeoutConfig` knob
// fits: `kill_grace` is the SIGTERM-to-SIGKILL interval and `step_timeout`
// measures the child's silence, not the wrapper's own output. The wait must
// still end, because a terminal that never drains would otherwise hang the
// wrapper forever.
pub(crate) const READER_DRAIN_LIMIT: Duration = Duration::from_secs(120);

/// How long a reader must have been blocked on its pipe, without a line
/// arriving, before the pipe cap may end the wait.
//
// WHY: a pull from a pipe that still holds data returns in microseconds, so a
// reader between two buffered lines is briefly "waiting" too. Without this a
// reader that had just finished a slow line would be cut off at the first
// poll that caught it between lines. A quarter second of continuous blocking
// after the child exited means nothing is left to read.
pub(crate) const READER_PIPE_SETTLE: Duration = Duration::from_millis(250);

const POLL: Duration = Duration::from_millis(10);

/// The bounds [`join_reader`] applies; tests inject short ones.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReaderBudget {
    /// Applies while the reader is blocked reading its pipe.
    pub(crate) pipe_cap: Duration,
    /// How long the reader must have been blocked for the pipe cap to apply.
    pub(crate) pipe_settle: Duration,
    /// Applies while the reader is handling lines it has already read, or
    /// has reached EOF.
    pub(crate) drain_limit: Duration,
}

impl Default for ReaderBudget {
    fn default() -> Self {
        Self {
            pipe_cap: READER_PIPE_CAP,
            pipe_settle: READER_PIPE_SETTLE,
            drain_limit: READER_DRAIN_LIMIT,
        }
    }
}

/// Why a reader was still running when its bound ran out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ReaderStall {
    /// Blocked reading a pipe that has not reached EOF.
    PipeOpen,
    /// Handling output it had already read, or past EOF.
    Processing,
    /// The caller did not track the reader's progress.
    Unknown,
}

/// How waiting for a reader thread ended.
#[derive(Debug)]
pub(crate) enum JoinOutcome<T> {
    Joined(T),
    /// The thread panicked; carries the panic message.
    Panicked(String),
    /// The bound ran out. The thread is detached and keeps running.
    TimedOut(ReaderStall),
}

// `ReaderProgress::state` encoding: these two sentinels, or else the reader
// is blocked on its pipe since `base + (state - 1)` nanoseconds.
const PROCESSING: u64 = 0;
const REACHED_EOF: u64 = u64::MAX;

/// Where a reader thread is in its line loop, shared with the joining thread.
#[derive(Clone)]
pub(crate) struct ReaderProgress(Arc<ProgressState>);

struct ProgressState {
    base: Instant,
    state: AtomicU64,
}

impl Default for ReaderProgress {
    /// A reader that has not pulled its first line counts as waiting on its
    /// pipe from now.
    fn default() -> Self {
        Self(Arc::new(ProgressState {
            base: Instant::now(),
            state: AtomicU64::new(1),
        }))
    }
}

impl ReaderProgress {
    /// Wrap a reader's line source so every pull records whether the reader
    /// is blocked on the pipe, handling a line, or done reading.
    pub(crate) fn track<I: Iterator>(&self, lines: I) -> TrackedLines<I> {
        TrackedLines {
            lines,
            progress: self.clone(),
        }
    }

    fn mark_waiting(&self) {
        let since = u64::try_from(self.0.base.elapsed().as_nanos()).unwrap_or(REACHED_EOF - 2);
        self.0.state.store(since + 1, Ordering::Release);
    }

    fn store(&self, state: u64) {
        self.0.state.store(state, Ordering::Release);
    }

    /// How long the reader has been blocked on its pipe, if it is.
    fn waiting_for(&self) -> Option<Duration> {
        match self.0.state.load(Ordering::Acquire) {
            PROCESSING | REACHED_EOF => None,
            since => {
                let since = self.0.base + Duration::from_nanos(since - 1);
                Some(Instant::now().saturating_duration_since(since))
            }
        }
    }

    #[cfg(test)]
    fn reached_eof(&self) -> bool {
        self.0.state.load(Ordering::Acquire) == REACHED_EOF
    }
}

/// A line source whose pulls are recorded in a [`ReaderProgress`].
pub(crate) struct TrackedLines<I> {
    lines: I,
    progress: ReaderProgress,
}

impl<I: Iterator> Iterator for TrackedLines<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.progress.mark_waiting();
        let next = self.lines.next();
        self.progress.store(if next.is_some() {
            PROCESSING
        } else {
            REACHED_EOF
        });
        next
    }
}

/// Wait for a reader thread under `budget`, measured from `since`.
///
/// The pipe cap applies once the reader has been blocked on its pipe for the
/// settle time, so a reader that finishes a slow line and then blocks on a
/// pipe a descendant still holds open times out soon after. Readers joined
/// one after another share `since`, so together they wait at most one drain
/// limit.
pub(crate) fn join_reader<T>(
    handle: thread::JoinHandle<T>,
    progress: &ReaderProgress,
    budget: ReaderBudget,
    since: Instant,
) -> JoinOutcome<T> {
    loop {
        if handle.is_finished() {
            return joined(handle);
        }
        let elapsed = since.elapsed();
        let blocked_on_pipe = progress
            .waiting_for()
            .is_some_and(|waiting| waiting >= budget.pipe_settle);
        let stall = if blocked_on_pipe && elapsed >= budget.pipe_cap {
            Some(ReaderStall::PipeOpen)
        } else if elapsed >= budget.drain_limit {
            Some(ReaderStall::Processing)
        } else {
            None
        };
        if let Some(stall) = stall {
            // A reader blocked in `read` cannot be interrupted from outside,
            // so it is detached rather than joined.
            std::mem::forget(handle);
            return JoinOutcome::TimedOut(stall);
        }
        thread::sleep(POLL);
    }
}

/// Wait at most `timeout` for a thread whose progress is not tracked.
pub(crate) fn join_within<T>(handle: thread::JoinHandle<T>, timeout: Duration) -> JoinOutcome<T> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if handle.is_finished() {
            return joined(handle);
        }
        thread::sleep(POLL);
    }
    std::mem::forget(handle);
    JoinOutcome::TimedOut(ReaderStall::Unknown)
}

fn joined<T>(handle: thread::JoinHandle<T>) -> JoinOutcome<T> {
    match handle.join() {
        Ok(value) => JoinOutcome::Joined(value),
        Err(payload) => JoinOutcome::Panicked(panic_message(payload.as_ref())),
    }
}

/// The message of a `panic!` payload, which is a `&str` or a `String` for
/// every panic raised with a message.
pub(crate) fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "the panic payload was not a string".to_string()
    }
}

/// The user-facing explanation of a reader that outlived its bound.
pub(crate) fn timeout_message(stream: &str, stall: ReaderStall, budget: ReaderBudget) -> String {
    match stall {
        ReaderStall::PipeOpen => format!(
            "Claudine stopped waiting for the agent's {stream} {}s after the agent exited: \
             the pipe is still open, most likely held by a process the agent started",
            budget.pipe_cap.as_secs()
        ),
        ReaderStall::Processing | ReaderStall::Unknown => format!(
            "Claudine stopped waiting for the agent's {stream} {}s after the agent exited: \
             it was still being processed, most likely because the terminal was not \
             accepting output",
            budget.drain_limit.as_secs()
        ),
    }
}

/// The slot a stdout reader hands its parser back through once every line has
/// been fed, before its final render.
pub(crate) type ParserSlot = Arc<Mutex<Option<Box<dyn SemanticStreamParser>>>>;

/// Summary source used when the stdout reader did not hand its parser back.
pub(crate) struct FallbackParser {
    exit_code: i32,
    error_kind: &'static str,
    message: String,
}

impl SemanticStreamParser for FallbackParser {
    fn feed_line(&mut self, _line: &str) {}

    fn finish(self: Box<Self>, _exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            is_error: true,
            error_kind: Some(self.error_kind.into()),
            error_message: Some(self.message),
            exit_code: self.exit_code,
            ..Default::default()
        }
    }
}

/// The parser whose summary the run reports, and a warning to show on stderr.
///
/// A reader that timed out after feeding its parser every line, and so was
/// stuck only in its final render, still yields the real summary: the child's
/// outcome is known, and only the terminal output may be incomplete.
pub(crate) fn settle_parser(
    outcome: JoinOutcome<()>,
    slot: &ParserSlot,
    exit_code: i32,
    budget: ReaderBudget,
) -> (Box<dyn SemanticStreamParser>, Option<String>) {
    let parsed = || {
        slot.lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take()
    };
    match outcome {
        JoinOutcome::Joined(()) => match parsed() {
            Some(parser) => (parser, None),
            None => (
                fallback(
                    exit_code,
                    "parse_failure",
                    "The stream parser thread ended without its parser".into(),
                ),
                None,
            ),
        },
        JoinOutcome::Panicked(message) => (
            fallback(
                exit_code,
                "parse_failure",
                format!("Stream parser thread panicked: {message}"),
            ),
            None,
        ),
        JoinOutcome::TimedOut(stall) => {
            let message = timeout_message("output", stall, budget);
            match parsed() {
                Some(parser) => (
                    parser,
                    Some(format!(
                        "{message}. The run's result is kept; its output may be incomplete"
                    )),
                ),
                None => (
                    fallback(exit_code, "stream_reader_timeout", message.clone()),
                    Some(message),
                ),
            }
        }
    }
}

fn fallback(
    exit_code: i32,
    error_kind: &'static str,
    message: String,
) -> Box<dyn SemanticStreamParser> {
    Box::new(FallbackParser {
        exit_code,
        error_kind,
        message,
    })
}

#[cfg(test)]
mod tests;
