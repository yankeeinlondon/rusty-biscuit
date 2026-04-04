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
    /// Called once when the provider emits its first event, typically an `init`
    /// or equivalent session-initialization message containing metadata such as
    /// the session ID, model name, and provider version.
    ///
    /// Use this to record session-level context (e.g. logging the model in use)
    /// or to trigger one-time setup actions like playing a start sound.
    fn on_session_start(&mut self, _meta: &EventMeta) {}

    /// Called at the beginning of a new conversational turn, before the model
    /// begins generating output for that turn.
    ///
    /// A "turn" is a single request-response cycle between the user and the
    /// model. Multi-turn sessions will fire this once per turn. Use this to
    /// track turn count or reset per-turn state.
    fn on_turn_start(&mut self, _meta: &EventMeta) {}

    /// Called when a discrete processing step begins within a turn.
    ///
    /// Providers may break a single turn into multiple steps (e.g. tool call
    /// planning, retrieval, reasoning). This fires at the start of each such
    /// granular step, allowing fine-grained progress tracking.
    fn on_step_start(&mut self, _meta: &EventMeta) {}

    /// Called when a discrete processing step completes within a turn.
    ///
    /// This is the counterpart to [`on_step_start`](Self::on_step_start) and
    /// fires when the provider signals completion of a single step, such as
    /// receiving a tool result or finishing a reasoning block.
    fn on_step_finish(&mut self, _meta: &EventMeta) {}

    /// Called when a conversational turn ends successfully, after all output
    /// (text, tool calls, etc.) for that turn has been emitted.
    ///
    /// This signals that the model has finished responding for the current
    /// turn. In a multi-turn session this fires once per completed turn.
    fn on_turn_complete(&mut self, _meta: &EventMeta) {}

    /// Called when the provider reports an error during a turn.
    ///
    /// The error may be a billing failure, authentication error, rate limit
    /// exhaustion, or any provider-reported fault. The `meta` parameter may
    /// contain `error_kind` and `error_message` entries describing the failure.
    fn on_turn_error(&mut self, _meta: &EventMeta) {}

    /// Called just before a tool invocation is executed by the provider.
    ///
    /// The `meta` parameter typically includes the tool name (under the
    /// `"tool_name"` key) so sinks can log or audibly signal which tool is
    /// about to run (e.g. playing a "tool start" sound effect).
    fn on_before_tool(&mut self, _meta: &EventMeta) {}

    /// Called just after a tool invocation completes and the provider has
    /// received the tool's result.
    ///
    /// Use this to track total tool-call counts or to signal completion of a
    /// tool sound effect.
    fn on_after_tool(&mut self, _meta: &EventMeta) {}

    /// Called when the provider is waiting for user permission to proceed with
    /// an action (e.g. a file write, shell command, or other sensitive
    /// operation).
    ///
    /// In non-interactive streaming mode this typically indicates the session
    /// will stall until permission is granted. Use this to surface a
    /// notification or play an alert sound so the user can intervene.
    fn on_permission_request(&mut self, _meta: &EventMeta) {}

    /// Called when the parser encounters a non-fatal issue, such as a
    /// malformed JSON line, a rate-limit warning from the provider, or
    /// degraded behavior that does not halt the stream.
    ///
    /// Use this to log warnings or display advisory messages without stopping
    /// stream processing.
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
