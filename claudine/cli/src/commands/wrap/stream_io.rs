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

/// Shared test-only buffer of `(is_stdout, line)` tuples captured by
/// `StreamOutput`. Wrapped in `Arc<Mutex<…>>` so the writer and the test
/// observer can share a single recording surface across threads.
#[cfg(test)]
pub(crate) type TestRecorder = Arc<Mutex<Vec<(bool, String)>>>;

pub(crate) struct StreamOutput {
    inner: Mutex<StreamOutputInner>,
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
}

impl StreamOutput {
    /// Construct a new coordinator behind an `Arc` for multi-thread sharing.
    pub(crate) fn new() -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(StreamOutputInner {
                last_stdout_newline: true,
            }),
            #[cfg(test)]
            test_recorder: None,
        })
    }

    /// The process-wide coordinator.
    ///
    /// `last_stdout_newline` describes the *process's* real stdout cursor, so
    /// two independent coordinators cannot both be right about it. That was
    /// harmless while one provider session owned the terminal; a parallel group
    /// puts several writers on it at once, and the spec requires them to share
    /// one synchronized sink (spec → *Reporting Concurrency*).
    pub(crate) fn shared() -> Arc<Self> {
        static SHARED: std::sync::OnceLock<Arc<StreamOutput>> = std::sync::OnceLock::new();
        SHARED.get_or_init(Self::new).clone()
    }

    /// Emit already-rendered lines to stderr as one indivisible write.
    ///
    /// Per-line emission would let a sibling task land a line inside another's
    /// frame; holding the coordinator across the whole group is what makes
    /// attribution survive contention.
    pub(crate) fn emit_stderr_frames(&self, frames: &[String]) {
        #[cfg(test)]
        if let Some(buf) = &self.test_recorder {
            let mut buf = buf.lock().unwrap_or_else(|e| e.into_inner());
            buf.extend(frames.iter().map(|line| (false, line.clone())));
            return;
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !inner.last_stdout_newline {
            let mut stdout = io::stdout().lock();
            let _ = stdout.write_all(b"\n");
            let _ = stdout.flush();
            inner.last_stdout_newline = true;
        }
        let mut stderr = io::stderr().lock();
        for line in frames {
            let _ = writeln!(stderr, "{line}");
        }
        let _ = stderr.flush();
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
        let mut stdout = io::stdout().lock();
        if !inner.last_stdout_newline {
            let _ = stdout.write_all(b"\n");
        }
        for line in frames {
            let _ = writeln!(stdout, "{line}");
        }
        let _ = stdout.flush();
        inner.last_stdout_newline = true;
    }

    /// Construct a test-only coordinator that captures emissions into the
    /// provided buffer instead of writing to real stdout/stderr.
    #[cfg(test)]
    pub(crate) fn test_recorder(buf: TestRecorder) -> Arc<Self> {
        Arc::new(Self {
            inner: Mutex::new(StreamOutputInner {
                last_stdout_newline: true,
            }),
            test_recorder: Some(buf),
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
        let mut stdout = io::stdout().lock();
        let _ = writeln!(stdout, "{line}");
        inner.last_stdout_newline = true;
    }

    /// Obtain a `Write` adapter that routes stdout bytes through this
    /// coordinator so the renderer thread can continue to use the standard
    /// `Write` trait while this struct tracks cursor state.
    pub(crate) fn stdout_writer(self: &Arc<Self>) -> StdoutWriter {
        StdoutWriter {
            coordinator: self.clone(),
        }
    }

    /// Emit a status line to stderr, guaranteeing it lands on a fresh row.
    ///
    /// Acquires the coordinator mutex, checks whether stdout is newline-
    /// terminated, and writes `\n` to stdout first when it is not. Then
    /// writes the caller-supplied line (without any additional prefix) to
    /// stderr followed by a newline.
    pub(crate) fn emit_stderr_line(&self, line: &str) {
        #[cfg(test)]
        if let Some(buf) = &self.test_recorder {
            buf.lock()
                .unwrap_or_else(|e| e.into_inner())
                .push((false, line.to_string()));
            return;
        }
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if !inner.last_stdout_newline {
            let mut stdout = io::stdout().lock();
            let _ = stdout.write_all(b"\n");
            let _ = stdout.flush();
            inner.last_stdout_newline = true;
        }
        let mut stderr = io::stderr().lock();
        let _ = writeln!(stderr, "{line}");
    }
}

/// `Write` adapter that mirrors its bytes to `std::io::stdout` while
/// updating [`StreamOutput`]'s newline-tracking flag. Any write acquires
/// the coordinator mutex for the duration of the call, so concurrent
/// writers cannot interleave bytes inside a single `write_all`.
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
        let mut stdout = io::stdout().lock();
        let n = stdout.write(buf)?;
        if let Some(last) = buf.get(..n).and_then(|s| s.last()) {
            inner.last_stdout_newline = *last == b'\n';
        }
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        io::stdout().lock().flush()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_coordinator_treats_stdout_as_newline_terminated() {
        let coord = StreamOutput::new();
        let inner = coord.inner.lock().unwrap();
        assert!(inner.last_stdout_newline);
    }

    #[test]
    fn stdout_writer_tracks_trailing_newline() {
        let coord = StreamOutput::new();
        let mut writer = coord.stdout_writer();

        writer.write_all(b"hello").unwrap();
        assert!(!coord.inner.lock().unwrap().last_stdout_newline);

        writer.write_all(b"\n").unwrap();
        assert!(coord.inner.lock().unwrap().last_stdout_newline);
    }

    #[test]
    fn stdout_writer_tracks_no_newline_in_last_write() {
        let coord = StreamOutput::new();
        let mut writer = coord.stdout_writer();

        writer.write_all(b"first\n").unwrap();
        assert!(coord.inner.lock().unwrap().last_stdout_newline);

        writer.write_all(b"second partial").unwrap();
        assert!(!coord.inner.lock().unwrap().last_stdout_newline);
    }

    #[test]
    fn stdout_writer_flags_newline_when_buffer_contains_embedded_newlines() {
        // Only the final byte matters for cursor state.
        let coord = StreamOutput::new();
        let mut writer = coord.stdout_writer();

        writer.write_all(b"alpha\nbeta").unwrap();
        assert!(!coord.inner.lock().unwrap().last_stdout_newline);
    }

    #[test]
    fn emit_stderr_line_after_newline_terminated_stdout_leaves_flag_unchanged() {
        // When stdout is already on a fresh row, emit_stderr_line must not
        // push a spurious blank line to stdout. We cannot observe stdout
        // bytes directly in a unit test but we can assert the bookkeeping
        // remains consistent.
        let coord = StreamOutput::new();
        let mut writer = coord.stdout_writer();
        writer.write_all(b"done\n").unwrap();
        coord.emit_stderr_line("tool: bash");
        assert!(coord.inner.lock().unwrap().last_stdout_newline);
    }

    #[test]
    fn emit_stderr_line_marks_stdout_as_newline_terminated_after_injection() {
        // After stdout was left mid-line, emit_stderr_line writes `\n` to
        // stdout; the tracked flag must flip so the next stderr emission
        // does not inject another `\n`.
        let coord = StreamOutput::new();
        let mut writer = coord.stdout_writer();
        writer.write_all(b"partial text").unwrap();
        assert!(!coord.inner.lock().unwrap().last_stdout_newline);
        coord.emit_stderr_line("tool: bash");
        assert!(coord.inner.lock().unwrap().last_stdout_newline);
    }
}
