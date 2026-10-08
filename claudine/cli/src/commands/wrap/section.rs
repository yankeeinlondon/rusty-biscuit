//! 9-section rendered output model + structural blank-line dedup.
//!
//! Per spec §"Section Model and Spacing Normalization": a single
//! non-interactive run renders into nine ordered sections; this writer
//! enforces "at most one blank line between any two adjacent sections
//! present in the rendered output". Trim is at the sink level — parsers
//! remain lossless.

use std::sync::{Arc, Mutex};

use super::stream_io::StreamOutput;

/// The nine ordered sections of rendered output. Only `FinalStdout` routes
/// to stdout; the other eight route to stderr.
///
/// Used directly by `LiveSemanticSink::emit_section_line` to tag lines for
/// inter-section spacing. The `FinalStdout` variant is used by
/// `SectionStream` for stdout routing and is being wired into the runtime
/// path incrementally.
#[allow(dead_code)] // FinalStdout reserved for SectionStream
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    ExecutionLine,
    EnvVariables,
    SystemPrompt,
    AgentPrompt,
    SessionAndModel,
    Thinking,
    ToolUseAndEvents,
    FinalStdout,
    TrailerMetadata,
}

/// Shared state machine for section-spacing logic.
///
/// Encapsulates the dedup rules used by both [`SectionStream`] (the
/// reference implementation) and [`super::live_semantic_sink::LiveSemanticSink`]
/// (the runtime path). Keeping the logic in one place prevents drift.
///
/// ## Rules
/// - Section transitions are separated by exactly one blank line.
/// - Consecutive blank lines inside a section collapse to one.
/// - No leading blank line is emitted before the first rendered line.
#[derive(Default)]
pub struct SectionTracker {
    last_section: Option<Section>,
    last_was_blank: bool,
}

impl SectionTracker {
    pub fn new() -> Self {
        Self::default()
    }

    /// Classifies a line for section-spacing purposes.
    ///
    /// Returns `None` if the line should be suppressed (consecutive blank
    /// dedup). Returns `Some((needs_section_separator, is_blank))` otherwise.
    pub fn classify(&mut self, section: Section, line: &str) -> Option<(bool, bool)> {
        let is_blank = line.trim().is_empty();
        let section_changed = self.last_section.is_some_and(|s| s != section);

        let needs_separator = section_changed && !self.last_was_blank && !is_blank;

        if is_blank && self.last_was_blank {
            self.last_section = Some(section);
            return None;
        }

        self.last_section = Some(section);
        self.last_was_blank = is_blank;
        Some((needs_separator, is_blank))
    }
}

/// Thin wrapper around `StreamOutput` that:
/// - tags each emit with its `Section`,
/// - dedupes consecutive blank emissions,
/// - guarantees at most one blank between adjacent sections.
///
/// Every `SectionStream` cloned from a given sink shares a single
/// [`SectionTracker`] so post-stream trailer emitters see the state left
/// behind by the live sink and vice-versa.
#[derive(Clone)]
pub struct SectionStream {
    inner: Arc<StreamOutput>,
    state: Arc<Mutex<SectionTracker>>,
    /// Terminal loss already on the process-wide counters when this stream was
    /// built, so [`output_loss`](Self::output_loss) reports this run's alone.
    loss_mark: super::output_worker::OutputLoss,
}

impl SectionStream {
    /// Build a section stream with a fresh, unshared tracker. Used by the
    /// reference-implementation unit tests below.
    #[allow(dead_code)]
    pub fn new(inner: Arc<StreamOutput>) -> Self {
        Self::with_tracker(inner, Arc::new(Mutex::new(SectionTracker::new())))
    }

    /// Build a section stream that shares an existing tracker with another
    /// writer (typically the live semantic sink). This is how trailer
    /// emitters inherit the running section state after the live stream
    /// ends.
    pub fn with_tracker(inner: Arc<StreamOutput>, state: Arc<Mutex<SectionTracker>>) -> Self {
        let loss_mark = inner.loss();
        Self {
            inner,
            state,
            loss_mark,
        }
    }

    /// Clone the shared tracker so external emitters (e.g. the flush-if-idle
    /// ticker) can participate in the same section-spacing state machine.
    pub fn tracker(&self) -> Arc<Mutex<SectionTracker>> {
        self.state.clone()
    }

    /// The stdout writer behind this stream, so the agent's final message is
    /// queued with the rest of the run's output rather than written directly.
    pub(crate) fn stdout_writer(&self) -> super::stream_io::StdoutWriter {
        self.inner.stdout_writer()
    }

    /// Wait for everything this run queued to reach the terminal; see
    /// [`StreamOutput::drain_final`].
    pub(crate) fn drain_final(&self) {
        let _ = self.inner.drain_final(super::exec::reader_join::READER_DRAIN_LIMIT);
    }

    /// Terminal output this run lost, or `None` when it lost none.
    pub(crate) fn output_loss(&self) -> Option<super::output_worker::OutputLoss> {
        let loss = self.inner.loss().since(&self.loss_mark);
        (!loss.is_empty()).then_some(loss)
    }

    pub fn emit_stderr(&self, section: Section, line: &str) {
        debug_assert!(
            section != Section::FinalStdout,
            "FinalStdout routes via emit_stdout"
        );
        self.emit(section, line, /* to_stdout = */ false);
    }

    #[allow(dead_code)] // retained for the reference impl path
    pub fn emit_stdout(&self, line: &str) {
        self.emit(Section::FinalStdout, line, /* to_stdout = */ true);
    }

    /// Classify a synthetic transition into [`Section::FinalStdout`]
    /// without writing any payload yet. Emits the section separator blank
    /// (on stderr) when needed and leaves the tracker parked in
    /// `FinalStdout` so subsequent raw stdout writes belong to that
    /// section. Idempotent: once the tracker is already in `FinalStdout`,
    /// further calls are no-ops.
    pub fn enter_final_stdout(&self) {
        let mut tracker = self.state.lock().unwrap_or_else(|e| e.into_inner());
        // Use a non-blank placeholder so the tracker records that content
        // was emitted in `FinalStdout`. The caller writes the actual
        // stdout bytes directly through `StdoutWriter`.
        let Some((needs_separator, _)) = tracker.classify(Section::FinalStdout, "x") else {
            return;
        };
        drop(tracker);
        if needs_separator {
            self.inner.emit_stderr_line("");
        }
    }

    fn emit(&self, section: Section, line: &str, to_stdout: bool) {
        let mut tracker = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let Some((needs_separator, _)) = tracker.classify(section, line) else {
            return;
        };
        drop(tracker);
        if needs_separator {
            // Section separator always goes to stderr.
            self.inner.emit_stderr_line("");
        }
        if to_stdout {
            self.inner.emit_stdout_line(line);
        } else {
            self.inner.emit_stderr_line(line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use super::super::stream_io::TestRecorder;

    fn recorder_pair() -> (SectionStream, TestRecorder) {
        let buf: TestRecorder = Arc::new(Mutex::new(Vec::new()));
        let inner = StreamOutput::test_recorder(buf.clone());
        (SectionStream::new(inner), buf)
    }

    fn lines(buf: &TestRecorder) -> Vec<String> {
        buf.lock().unwrap().iter().map(|(_, l)| l.clone()).collect()
    }

    #[test]
    fn consecutive_blank_lines_inside_a_section_are_collapsed() {
        let (s, buf) = recorder_pair();
        s.emit_stderr(Section::ToolUseAndEvents, "→ Bash");
        s.emit_stderr(Section::ToolUseAndEvents, "");
        s.emit_stderr(Section::ToolUseAndEvents, "");
        s.emit_stderr(Section::ToolUseAndEvents, "← Bash · success");
        assert_eq!(lines(&buf), vec!["→ Bash", "", "← Bash · success"]);
    }

    #[test]
    fn section_change_inserts_exactly_one_blank() {
        let (s, buf) = recorder_pair();
        s.emit_stderr(Section::SessionAndModel, "- Session id …");
        s.emit_stderr(Section::ToolUseAndEvents, "→ Bash");
        assert_eq!(lines(&buf), vec!["- Session id …", "", "→ Bash"]);
    }

    #[test]
    fn section_change_after_existing_blank_does_not_double_blank() {
        let (s, buf) = recorder_pair();
        s.emit_stderr(Section::SessionAndModel, "- Session id …");
        s.emit_stderr(Section::SessionAndModel, "");
        s.emit_stderr(Section::ToolUseAndEvents, "→ Bash");
        assert_eq!(lines(&buf), vec!["- Session id …", "", "→ Bash"]);
    }

    #[test]
    fn final_stdout_routes_to_stdout_channel() {
        let (s, buf) = recorder_pair();
        s.emit_stdout("hello");
        let routes: Vec<bool> = buf
            .lock()
            .unwrap()
            .iter()
            .map(|(stdout, _)| *stdout)
            .collect();
        assert_eq!(routes, vec![true]);
    }

    #[test]
    fn no_blank_is_emitted_before_first_line() {
        let (s, buf) = recorder_pair();
        s.emit_stderr(Section::ExecutionLine, "header");
        assert_eq!(lines(&buf), vec!["header"]);
    }

    #[test]
    fn section_change_with_blank_start_does_not_double_blank() {
        let (s, buf) = recorder_pair();
        s.emit_stderr(Section::SessionAndModel, "- Session id …");
        s.emit_stderr(Section::ToolUseAndEvents, "");
        s.emit_stderr(Section::ToolUseAndEvents, "→ Bash");
        assert_eq!(lines(&buf), vec!["- Session id …", "", "→ Bash"]);
    }

    #[test]
    fn section_change_with_double_blank_boundary_does_not_triple_blank() {
        let (s, buf) = recorder_pair();
        s.emit_stderr(Section::SessionAndModel, "- Session id …");
        s.emit_stderr(Section::SessionAndModel, "");
        s.emit_stderr(Section::ToolUseAndEvents, "");
        s.emit_stderr(Section::ToolUseAndEvents, "→ Bash");
        assert_eq!(lines(&buf), vec!["- Session id …", "", "→ Bash"]);
    }
}
