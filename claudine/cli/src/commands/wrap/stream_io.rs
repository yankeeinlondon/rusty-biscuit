//! Coordinated stdout/stderr writer for wrapped provider sessions.
//!
//! The wrapper emits assistant text on stdout and lifecycle status lines
//! (tool announcements, progress heartbeats) on stderr. When those two
//! streams land in the same terminal, a mid-line stdout write followed by
//! a stderr write produces visually corrupt output — the stderr line lands
//! at the stdout cursor position instead of a fresh row.
//!
//! `StreamOutput` serializes writes to both streams behind a shared mutex
//! and tracks whether the last stdout byte was a newline. Stderr emission
//! injects a synthetic newline on stdout first when needed, guaranteeing
//! stderr status lines always start on a fresh row without producing
//! gratuitous blank lines when stdout was already newline-terminated.
use std::io::{self, Write};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use super::output_worker::{
    Drained, FrameSink, OutputLoss, OutputWorker, Stream, Submitted, TerminalSink,
};
use super::run_scope;

static SHARED: std::sync::OnceLock<Arc<StreamOutput>> = std::sync::OnceLock::new();

/// Shared test-only buffer of `(is_stdout, line)` tuples captured by
/// `StreamOutput`. Wrapped in `Arc<Mutex<…>>` so the writer and the test
/// observer can share a single recording surface across threads.
#[cfg(test)]
pub(crate) type TestRecorder = Arc<Mutex<Vec<(bool, String)>>>;

/// Terminal output of a wrapped run.
///
/// Nothing here writes to the terminal on the calling thread. Every emission
/// is queued on the [`OutputWorker`] as a whole frame, so a terminal that has
/// stopped accepting output can stall the worker but never the parser or the
/// wrapper. The cursor bookkeeping below describes the stream as the worker
/// will leave it once the queue drains.
pub(crate) struct StreamOutput {
    /// Shared rather than owned so a [`decorated`](StreamOutput::decorated)
    /// handle serializes against the same cursor: per-task decoration must not
    /// buy a second opinion about where the real stdout cursor is.
    inner: Arc<Mutex<StreamOutputInner>>,
    /// The one writer behind every handle derived from the same coordinator.
    worker: Arc<OutputWorker>,
    /// Prefix applied to every status line this handle emits, so a sequence
    /// task's status and reasoning carry the same bar as its data (spec →
    /// *Reporting Concurrency*: "every subsequent status/output line").
    ///
    /// Status only. Task data is decorated by `TaskFrameWriter` on its way to
    /// the data channel; prefixing here too would draw the bar twice.
    status_gutter: Option<String>,
    /// Test-only recording buffer. When `Some`, `emit_stdout_line` and
    /// `emit_stderr_line` push `(is_stdout, line)` tuples into the shared
    /// buffer and skip writing to real stdout/stderr. This lets unit tests
    /// for higher-level writers (e.g. `SectionStream`) observe emitted
    /// lines without touching the process's real streams.
    #[cfg(test)]
    test_recorder: Option<TestRecorder>,
}

struct StreamOutputInner {
    /// True when the most recent byte written to stdout was `\n`. Initial
    /// value is `true` because stdout starts at column zero.
    last_stdout_newline: bool,
    /// When the wrapper's wait for the terminal ends, set by the run that
    /// started the clock and consumed by the next [`StreamOutput::drain_final`].
    drain_deadline: Option<Instant>,
}

impl StreamOutput {
    /// Construct a new coordinator behind an `Arc` for multi-thread sharing.
    pub(crate) fn new() -> Arc<Self> {
        Self::with_sink(Box::new(TerminalSink))
    }

    /// A coordinator whose worker writes to `sink`.
    pub(crate) fn with_sink(sink: Box<dyn FrameSink>) -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::new(Mutex::new(StreamOutputInner {
                last_stdout_newline: true,
                drain_deadline: None,
            })),
            worker: Arc::new(OutputWorker::new(sink)),
            status_gutter: None,
            #[cfg(test)]
            test_recorder: None,
        })
    }

    /// A handle whose status lines carry `gutter`, sharing this coordinator's
    /// cursor state and worker.
    ///
    /// Derived rather than constructed so the "one synchronized render sink"
    /// rule survives: N tasks each hold their own decorated handle, but every
    /// write still serializes on one mutex.
    pub(crate) fn decorated(self: &Arc<Self>, gutter: String) -> Arc<Self> {
        Arc::new(Self {
            inner: Arc::clone(&self.inner),
            worker: Arc::clone(&self.worker),
            status_gutter: Some(gutter),
            #[cfg(test)]
            test_recorder: self.test_recorder.clone(),
        })
    }

    /// Apply this handle's status gutter to an already-rendered block.
    ///
    /// Rendered status is frequently multi-line (a `Status` block, a wrapped
    /// `Prose` paragraph), and an unprefixed continuation line would read as
    /// belonging to whichever task wrote last.
    fn decorate_status(&self, line: &str) -> String {
        match &self.status_gutter {
            None => line.to_string(),
            Some(gutter) => line
                .split('\n')
                .map(|part| format!("{gutter}{part}"))
                .collect::<Vec<_>>()
                .join("\n"),
        }
    }

    /// The process-wide coordinator.
    ///
    /// `last_stdout_newline` describes the *process's* real stdout cursor, so
    /// two independent coordinators cannot both be right about it. That was
    /// harmless while one provider session owned the terminal; a parallel group
    /// puts several writers on it at once, and the spec requires them to share
    /// one synchronized sink (spec → *Reporting Concurrency*).
    ///
    /// It is also the one worker for the process: a terminal found stalled in
    /// one iteration of a loop stays disabled for the rest, and no iteration
    /// starts a writer of its own.
    ///
    /// Creating it is what starts a wrapped run's ownership of the terminal:
    /// from then until the process exits, Claudine's own diagnostics are
    /// frames on this worker too ([`crate::terminal_gate::route_to`]). Only
    /// wrapped-run code asks for it, so every other command keeps writing
    /// directly.
    pub(crate) fn shared() -> Arc<Self> {
        SHARED
            .get_or_init(|| {
                let output = Self::new();
                crate::terminal_gate::route_to(Arc::clone(&output));
                output
            })
            .clone()
    }

    /// The process-wide coordinator, if a wrapped run created it.
    pub(crate) fn shared_if_created() -> Option<Arc<Self>> {
        SHARED.get().cloned()
    }

    /// Queue one frame, unless the calling thread's run has already ended.
    /// Returns whether the frame was queued.
    ///
    /// Callers hold `inner` so frames reach the queue in the order the cursor
    /// bookkeeping assumed; queueing never blocks. A dropped or refused frame
    /// is counted by the worker and reported through `loss`, and must not move
    /// the cursor: it never reached the terminal.
    fn submit(&self, stream: Stream, bytes: Vec<u8>) -> bool {
        if run_scope::current_closed() {
            self.worker.reject();
            return false;
        }
        run_scope::observe(run_scope::observation::Operation::OutputSubmission);
        self.worker.submit(stream, bytes) == Submitted::Queued
    }

    /// Queue the newline that puts a status line on a fresh row, when stdout
    /// was left mid-line.
    fn separate_from_stdout(&self, inner: &mut StreamOutputInner) {
        if !inner.last_stdout_newline
            && self.submit(Stream::Stdout, b"\n".to_vec())
        {
            inner.last_stdout_newline = true;
        }
    }

    /// Emit already-rendered lines to stderr as one indivisible write.
    ///
    /// Per-line emission would let a sibling task land a line inside another's
    /// frame; holding the coordinator across the whole group is what makes
    /// attribution survive contention.
    pub(crate) fn emit_stderr_frames(&self, frames: &[String]) {
        let frames: Vec<String> = frames
            .iter()
            .map(|line| self.decorate_status(line))
            .collect();
        #[cfg(test)]
        if let Some(buf) = &self.test_recorder {
            let mut buf = buf.lock().unwrap_or_else(|e| e.into_inner());
            buf.extend(frames.iter().map(|line| (false, line.clone())));
            return;
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.separate_from_stdout(&mut inner);
        let mut bytes = Vec::new();
        for line in &frames {
            let _ = writeln!(bytes, "{line}");
        }
        self.submit(Stream::Stderr, bytes);
    }

    /// Emit already-rendered lines to stdout as one indivisible write.
    ///
    /// The stdout twin of [`emit_stderr_frames`](Self::emit_stderr_frames), for
    /// attributed *task data*. Same atomicity contract: a sibling task must not
    /// be able to land a line inside another's frame group.
    pub(crate) fn emit_stdout_frames(&self, frames: &[String]) {
        #[cfg(test)]
        if let Some(buf) = &self.test_recorder {
            let mut buf = buf.lock().unwrap_or_else(|e| e.into_inner());
            buf.extend(frames.iter().map(|line| (true, line.clone())));
            return;
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        let mut bytes = Vec::new();
        if !inner.last_stdout_newline {
            bytes.push(b'\n');
        }
        for line in frames {
            let _ = writeln!(bytes, "{line}");
        }
        if self.submit(Stream::Stdout, bytes) {
            inner.last_stdout_newline = true;
        }
    }

    /// Construct a test-only coordinator that captures emissions into the
    /// provided buffer instead of writing to real stdout/stderr.
    #[cfg(test)]
    pub(crate) fn test_recorder(buf: TestRecorder) -> Arc<Self> {
        Arc::new(Self {
            test_recorder: Some(buf),
            ..Arc::into_inner(Self::new()).expect("a fresh coordinator has one owner")
        })
    }

    /// Emit a line to stdout followed by a newline. In test-recorder mode,
    /// appends `(true, line)` to the buffer and returns without touching
    /// real stdout.
    #[allow(dead_code)] // used by SectionStream reference impl and its tests
    pub(crate) fn emit_stdout_line(&self, line: &str) {
        #[cfg(test)]
        if let Some(buf) = &self.test_recorder {
            buf.lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((true, line.to_string()));
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.last_stdout_newline = true;
            return;
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if self.submit(Stream::Stdout, format!("{line}\n").into_bytes()) {
            inner.last_stdout_newline = true;
        }
    }

    /// Obtain a `Write` adapter that routes stdout bytes through this
    /// coordinator so the renderer thread can continue to use the standard
    /// `Write` trait while this struct tracks cursor state.
    pub(crate) fn stdout_writer(self: &Arc<Self>) -> StdoutWriter {
        StdoutWriter {
            coordinator: self.clone(),
        }
    }

    /// The stderr twin of [`stdout_writer`](Self::stdout_writer), for the
    /// agent's own stderr forwarded line by line: no status gutter, and each
    /// `write` lands on a fresh row.
    pub(crate) fn stderr_writer(self: &Arc<Self>) -> StderrWriter {
        StderrWriter {
            coordinator: self.clone(),
        }
    }

    /// Emit a status line to stderr, guaranteeing it lands on a fresh row.
    ///
    /// Queues a `\n` for stdout first when stdout is not newline-terminated,
    /// then the caller-supplied line (without any additional prefix) for
    /// stderr followed by a newline.
    pub(crate) fn emit_stderr_line(&self, line: &str) {
        let line = &self.decorate_status(line);
        self.emit_stderr_undecorated(line);
    }

    /// Like [`emit_stderr_line`](Self::emit_stderr_line), without the status
    /// gutter: for text that is the agent's own stderr, not Claudine's status.
    pub(crate) fn emit_stderr_undecorated(&self, line: &str) {
        #[cfg(test)]
        if let Some(buf) = &self.test_recorder {
            buf.lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((false, line.to_string()));
            return;
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.separate_from_stdout(&mut inner);
        self.submit(Stream::Stderr, format!("{line}\n").into_bytes());
    }

    /// Queue a diagnostic's bytes unchanged ([`crate::terminal_gate::write`]);
    /// returns whether they were queued.
    ///
    /// A stderr diagnostic gets no fresh-row separation, as a direct write
    /// would not, and takes no cursor lock, so a panic report from a thread
    /// that panicked inside this type cannot wait on itself. A stdout
    /// diagnostic is data, and moves the cursor like any stdout frame.
    pub(crate) fn submit_diagnostic(&self, stream: Stream, bytes: Vec<u8>) -> bool {
        #[cfg(test)]
        if let Some(buf) = &self.test_recorder {
            buf.lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((stream == Stream::Stdout, String::from_utf8_lossy(&bytes).into_owned()));
            return true;
        }
        match stream {
            Stream::Stderr => self.submit(stream, bytes),
            Stream::Stdout => {
                let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
                let ends_line = bytes.last().map(|last| *last == b'\n');
                let queued = self.submit(stream, bytes);
                if let (true, Some(ends_line)) = (queued, ends_line) {
                    inner.last_stdout_newline = ends_line;
                }
                queued
            }
        }
    }

    /// Wait for everything queued so far to reach the terminal, up to
    /// `deadline`.
    ///
    /// Expiry disables delivery for the rest of the process and abandons the
    /// writer; see [`OutputWorker::drain`].
    pub(crate) fn drain(&self, deadline: Instant) -> Drained {
        self.worker.drain(deadline)
    }

    /// Start the clock the trailer and every later write wait against.
    pub(crate) fn set_drain_deadline(&self, deadline: Instant) {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain_deadline = Some(deadline);
    }

    /// Wait for the queue against the deadline of the run that just ended, or
    /// `fallback` after it when no run set one. Used after the trailer and
    /// before the process exits.
    pub(crate) fn drain_final(&self, fallback: std::time::Duration) -> Drained {
        let deadline = self
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .drain_deadline
            .take()
            .unwrap_or_else(|| Instant::now() + fallback);
        self.drain(deadline)
    }

    /// Frames queued behind the one the worker is writing.
    #[cfg(test)]
    pub(crate) fn queued_frames(&self) -> usize {
        self.worker.queued_frames()
    }

    /// Terminal output lost so far in this process; see [`OutputLoss::since`].
    pub(crate) fn observation_since(&self, origin: Instant) -> super::output_worker::DeliveryObservation {
        self.worker.observation_since(origin)
    }

    pub(crate) fn loss(&self) -> OutputLoss {
        self.worker.loss()
    }
}

/// `Write` adapter that queues its bytes for stdout while updating
/// [`StreamOutput`]'s newline-tracking flag. Each `write` is one frame, so
/// concurrent writers cannot interleave bytes inside a single `write_all`.
pub(crate) struct StdoutWriter {
    coordinator: Arc<StreamOutput>,
}

impl Write for StdoutWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut inner = self
            .coordinator
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        // A frame refused because delivery is disabled still reports success:
        // the loss is recorded, and the parser must not fail over a terminal.
        if let Some(last) = buf.last()
            && self.coordinator.submit(Stream::Stdout, buf.to_vec())
        {
            inner.last_stdout_newline = *last == b'\n';
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// `Write` adapter that queues its bytes for stderr. Each `write` is one frame
/// placed on a fresh row, so a caller writes a whole line per call.
pub(crate) struct StderrWriter {
    coordinator: Arc<StreamOutput>,
}

impl Write for StderrWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if buf.is_empty() {
            return Ok(0);
        }
        let mut inner = self
            .coordinator
            .inner
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        self.coordinator.separate_from_stdout(&mut inner);
        // Refused frames still report success, as for `StdoutWriter`.
        self.coordinator.submit(Stream::Stderr, buf.to_vec());
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
