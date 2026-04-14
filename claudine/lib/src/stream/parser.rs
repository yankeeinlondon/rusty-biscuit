use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value};

use super::semantic::{SemanticEvent, SemanticEventSink};
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

    /// Called when the provider spawns a sub-agent (e.g. OpenCode `task`
    /// tool). This is distinct from [`on_before_tool`](Self::on_before_tool)
    /// so sinks can track sub-agent lifecycles separately from ordinary tool
    /// calls.
    fn on_subagent_start(&mut self, _meta: &EventMeta) {}

    /// Called when a sub-agent finishes execution. This is the counterpart to
    /// [`on_subagent_start`](Self::on_subagent_start).
    fn on_subagent_stop(&mut self, _meta: &EventMeta) {}

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

/// Line-by-line structured stream parser built around [`SemanticEvent`].
///
/// Unlike [`StreamParser`], each successfully-parsed line yields zero or more
/// semantic events delivered through the sink instead of a returned
/// `Option<StreamChunk>`. Malformed JSON lines emit
/// [`SemanticEvent::Warning`] and return `Ok(())` rather than propagating an
/// error.
///
/// During the migration from the legacy callback-based parsers, non-migrated
/// providers are exposed through [`LegacySemanticParser`] — a bridge that
/// wraps a [`StreamParser`] and translates its output into semantic events.
pub trait SemanticStreamParser: Send {
    /// Process one line of provider output, emitting zero or more
    /// [`SemanticEvent`]s via the parser's owned sink.
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError>;

    /// Finalize parsing and return the accumulated summary.
    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary;
}

/// Adapter: wraps a legacy [`StreamEventSink`]-based parser and presents it as
/// a [`SemanticStreamParser`] that emits [`SemanticEvent`]s to a
/// [`SemanticEventSink`].
///
/// Construction requires the caller to have built the wrapped parser with a
/// [`LegacyCallbackBridge`] that shares its queue with this adapter. Each
/// `feed_line` call drains the bridge's queued callback events and forwards
/// them to the semantic sink, in addition to translating `StreamChunk`
/// return values into [`SemanticEvent::OutputText`] / [`SemanticEvent::Reasoning`]
/// and malformed lines into [`SemanticEvent::Warning`].
///
/// This adapter is intentionally transitional. Providers migrated to native
/// [`SemanticStreamParser`] implementations (e.g. in Phase 2 of the
/// more-meta-response feature) bypass it entirely.
pub struct LegacySemanticParser<S: SemanticEventSink> {
    inner: Box<dyn StreamParser>,
    queue: Arc<Mutex<Vec<SemanticEvent>>>,
    sink: S,
}

impl<S: SemanticEventSink> LegacySemanticParser<S> {
    /// Create the adapter from a legacy parser and the queue it shares with
    /// its [`LegacyCallbackBridge`].
    pub fn new(inner: Box<dyn StreamParser>, queue: Arc<Mutex<Vec<SemanticEvent>>>, sink: S) -> Self {
        Self { inner, queue, sink }
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for LegacySemanticParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        let chunk_result = self.inner.feed_line(line);

        let drained: Vec<SemanticEvent> = {
            let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
            std::mem::take(&mut *queue)
        };
        for event in drained {
            self.sink.on_semantic_event(event);
        }

        match chunk_result {
            Ok(Some(StreamChunk::Text(text))) => {
                self.sink.on_semantic_event(SemanticEvent::OutputText {
                    text,
                    extra: Value::Object(Map::new()),
                });
                Ok(())
            }
            Ok(Some(StreamChunk::Thinking(text))) => {
                self.sink.on_semantic_event(SemanticEvent::Reasoning {
                    text,
                    extra: Value::Object(Map::new()),
                });
                Ok(())
            }
            Ok(None) => Ok(()),
            Err(StreamParseError::MalformedLine { line_num, message }) => {
                let mut extra = Map::new();
                extra.insert("line_num".into(), Value::from(line_num));
                self.sink.on_semantic_event(SemanticEvent::Warning {
                    message: format!("Malformed JSON on line {line_num}: {message}"),
                    extra: Value::Object(extra),
                });
                Ok(())
            }
            Err(e @ StreamParseError::Fatal(_)) => Err(e),
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        self.inner.finish(exit_code)
    }
}

/// Bridge: adapts the legacy [`StreamEventSink`] callback interface into
/// queued [`SemanticEvent`]s.
///
/// Instances are owned by a non-migrated legacy parser as its sink; the queue
/// is shared with a surrounding [`LegacySemanticParser`] adapter that drains
/// events after each `feed_line` call.
///
/// The translation from legacy callbacks to semantic events is conservative:
/// every callback becomes the closest-matching typed `SemanticEvent`, with
/// provider-specific metadata preserved under `extra`. This bridge is
/// transitional — native `SemanticStreamParser` implementations should emit
/// events directly without routing through it.
pub struct LegacyCallbackBridge {
    queue: Arc<Mutex<Vec<SemanticEvent>>>,
}

impl LegacyCallbackBridge {
    /// Create a new bridge that pushes translated events into `queue`.
    pub fn new(queue: Arc<Mutex<Vec<SemanticEvent>>>) -> Self {
        Self { queue }
    }

    fn push(&mut self, event: SemanticEvent) {
        let mut queue = self.queue.lock().unwrap_or_else(|e| e.into_inner());
        queue.push(event);
    }

    fn extra_from_meta(meta: &EventMeta) -> Value {
        let obj: Map<String, Value> = meta.extra.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
        Value::Object(obj)
    }

    fn string_field(meta: &EventMeta, key: &str) -> Option<String> {
        meta.extra.get(key).and_then(|v| v.as_str()).map(String::from)
    }
}

impl StreamEventSink for LegacyCallbackBridge {
    fn on_session_start(&mut self, meta: &EventMeta) {
        self.push(SemanticEvent::SessionStart {
            session_id: Self::string_field(meta, "session_id"),
            model: Self::string_field(meta, "model"),
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_turn_start(&mut self, meta: &EventMeta) {
        self.push(SemanticEvent::TurnStart {
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_step_start(&mut self, meta: &EventMeta) {
        let mut extra = Self::extra_from_meta(meta);
        if let Some(obj) = extra.as_object_mut() {
            obj.insert("step_phase".into(), Value::from("start"));
        }
        self.push(SemanticEvent::Info {
            message: "step_start".into(),
            extra,
        });
    }

    fn on_step_finish(&mut self, meta: &EventMeta) {
        let mut extra = Self::extra_from_meta(meta);
        if let Some(obj) = extra.as_object_mut() {
            obj.insert("step_phase".into(), Value::from("finish"));
        }
        self.push(SemanticEvent::Info {
            message: "step_finish".into(),
            extra,
        });
    }

    fn on_turn_complete(&mut self, meta: &EventMeta) {
        self.push(SemanticEvent::TurnComplete {
            provider_status: Self::string_field(meta, "provider_status"),
            token_usage: None,
            cost_usd: meta.extra.get("cost_usd").and_then(|v| v.as_f64()),
            duration_ms: meta.extra.get("duration_ms").and_then(|v| v.as_u64()),
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_turn_error(&mut self, meta: &EventMeta) {
        let message = Self::string_field(meta, "error_message")
            .or_else(|| Self::string_field(meta, "message"))
            .unwrap_or_default();
        self.push(SemanticEvent::Error {
            message,
            terminal: true,
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_before_tool(&mut self, meta: &EventMeta) {
        self.push(SemanticEvent::ToolCall {
            name: Self::string_field(meta, "tool_name"),
            id: Self::string_field(meta, "tool_id"),
            input: meta.extra.get("tool_input").cloned(),
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_after_tool(&mut self, meta: &EventMeta) {
        self.push(SemanticEvent::ToolResult {
            name: Self::string_field(meta, "tool_name"),
            id: Self::string_field(meta, "tool_id"),
            status: Self::string_field(meta, "status"),
            exit_code: meta.extra.get("exit_code").and_then(|v| v.as_i64()).map(|v| v as i32),
            output: meta.extra.get("tool_response").cloned(),
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_permission_request(&mut self, meta: &EventMeta) {
        self.push(SemanticEvent::PermissionRequest {
            kind: Self::string_field(meta, "kind"),
            tool_name: Self::string_field(meta, "tool_name"),
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_subagent_start(&mut self, meta: &EventMeta) {
        self.push(SemanticEvent::SubagentStart {
            name: Self::string_field(meta, "name"),
            id: Self::string_field(meta, "id"),
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_subagent_stop(&mut self, meta: &EventMeta) {
        self.push(SemanticEvent::SubagentStop {
            name: Self::string_field(meta, "name"),
            id: Self::string_field(meta, "id"),
            status: Self::string_field(meta, "status"),
            extra: Self::extra_from_meta(meta),
        });
    }

    fn on_warning(&mut self, message: &str) {
        self.push(SemanticEvent::Warning {
            message: message.to_string(),
            extra: Value::Object(Map::new()),
        });
    }
}
