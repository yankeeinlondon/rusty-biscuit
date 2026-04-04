use std::collections::HashMap;

use serde_json::Value;

use super::summary::StreamExecutionSummary;

/// Metadata accompanying coarse events discovered during stream parsing.
#[derive(Debug, Clone, Default)]
pub struct EventMeta {
    pub extra: HashMap<String, Value>,
}

/// A chunk of text produced by stream parsing.
///
/// Distinguishes assistant text (displayed on stdout) from thinking/reasoning
/// text (displayed dimmed on stderr) so the renderer can style them differently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamChunk {
    /// Regular assistant text — rendered to stdout.
    Text(String),
    /// Thinking/reasoning text — rendered dimmed to stderr.
    Thinking(String),
}

/// Callback interface for coarse events discovered during stream parsing.
///
/// Implementors receive normalized events suitable for dispatch.
/// This avoids coupling the parser to a specific dispatch mechanism.
pub trait StreamEventSink: Send {
    fn on_session_start(&mut self, _meta: &EventMeta) {}
    fn on_turn_start(&mut self, _meta: &EventMeta) {}
    fn on_step_start(&mut self, _meta: &EventMeta) {}
    fn on_step_finish(&mut self, _meta: &EventMeta) {}
    fn on_turn_complete(&mut self, _meta: &EventMeta) {}
    fn on_turn_error(&mut self, _meta: &EventMeta) {}
    fn on_before_tool(&mut self, _meta: &EventMeta) {}
    fn on_after_tool(&mut self, _meta: &EventMeta) {}
    fn on_permission_request(&mut self, _meta: &EventMeta) {}
    fn on_warning(&mut self, _message: &str) {}
}

/// A no-op sink that discards all events.
pub struct NullSink;
impl StreamEventSink for NullSink {}

/// Line-by-line structured stream parser.
///
/// Each provider implements this trait. The parser is driven by
/// `feed_line()` calls and produces a final summary on `finish()`.
pub trait StreamParser: Send {
    /// Process one line of provider output.
    ///
    /// Returns `Ok(Some(chunk))` when the line contributes text that should
    /// be displayed. Returns `Ok(None)` for metadata-only lines. Returns
    /// `Err` only for fatal parse failures.
    fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError>;

    /// Finalize parsing and return the accumulated summary.
    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary;
}

/// Errors during stream parsing.
#[derive(Debug, thiserror::Error)]
pub enum StreamParseError {
    #[error("Malformed JSON on line {line_num}: {message}")]
    MalformedLine { line_num: usize, message: String },
    #[error("Stream unusable: {0}")]
    Fatal(String),
}
