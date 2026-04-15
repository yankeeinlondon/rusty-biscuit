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

        let needs_separator = section_changed && !self.last_was_blank;

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
/// Reference implementation of the section-spacing invariants. Uses
/// [`SectionTracker`] internally so the logic stays in sync with
/// `LiveSemanticSink`.
#[allow(dead_code)] // reference implementation; see LiveSemanticSink for runtime path
#[derive(Clone)]
pub struct SectionStream {
    inner: Arc<StreamOutput>,
    state: Arc<Mutex<SectionTracker>>,
}

#[allow(dead_code)] // reference impl; exercised by unit tests below
impl SectionStream {
    pub fn new(inner: Arc<StreamOutput>) -> Self {
        Self {
            inner,
            state: Arc::new(Mutex::new(SectionTracker::new())),
        }
    }

    pub fn emit_stderr(&self, section: Section, line: &str) {
        debug_assert!(
            section != Section::FinalStdout,
            "FinalStdout routes via emit_stdout"
        );
        self.emit(section, line, /* to_stdout = */ false);
    }

    pub fn emit_stdout(&self, line: &str) {
        self.emit(Section::FinalStdout, line, /* to_stdout = */ true);
    }

    fn emit(&self, section: Section, line: &str, to_stdout: bool) {
        let mut tracker = self.state.lock().unwrap();
        let Some((needs_separator, _)) = tracker.classify(section, line) else {
            return;
        };
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

    fn recorder_pair() -> (SectionStream, Arc<Mutex<Vec<(bool, String)>>>) {
        let buf: Arc<Mutex<Vec<(bool, String)>>> = Arc::new(Mutex::new(Vec::new()));
        let inner = StreamOutput::test_recorder(buf.clone());
        (SectionStream::new(inner), buf)
    }

    fn lines(buf: &Arc<Mutex<Vec<(bool, String)>>>) -> Vec<String> {
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
}
