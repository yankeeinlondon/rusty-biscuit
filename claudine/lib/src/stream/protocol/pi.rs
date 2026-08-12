//! Typed event models for Pi's `--mode json` NDJSON stream.
//!
//! Pi is a bespoke (non-fork) provider: it emits one JSON object per line with a
//! top-level `type` discriminator and a nested `assistantMessageEvent.type`
//! discriminator on `message_update`. Unlike a typed error enum, Pi normalizes
//! provider failures into assistant messages (`stopReason: "error"` plus a
//! free-text `errorMessage`), so the parser classifies from message text rather
//! than a structured category. The terminal event is `agent_end`.
//!
//! Lifecycle events that carry no user-visible payload (`turn_start`,
//! `turn_end`, `message_start`, `agent_start`, `tool_execution_update`, the
//! queue/session-mutation family) are modeled as [`PiIgnore`] so they are
//! recognized and dropped rather than surfaced as `ProviderExtension` noise;
//! genuinely-unknown future types still fall through to the parser's
//! `ProviderExtension` fallback.

use serde::Deserialize;
use serde_json::Value;

/// Tagged enum over the Pi `--mode json` stream events the parser handles.
///
/// Unknown types fail typed deserialization and are preserved as
/// `ProviderExtension` by the parser's fallback arm.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum PiEvent {
    #[serde(rename = "session")]
    Session(PiSession),
    #[serde(rename = "agent_start")]
    AgentStart(PiIgnore),
    #[serde(rename = "turn_start")]
    TurnStart(PiIgnore),
    #[serde(rename = "message_start")]
    MessageStart(PiIgnore),
    #[serde(rename = "message_update")]
    MessageUpdate(PiMessageUpdate),
    #[serde(rename = "message_end")]
    MessageEnd(PiMessageEnvelope),
    #[serde(rename = "tool_execution_start")]
    ToolExecutionStart(PiToolStart),
    #[serde(rename = "tool_execution_update")]
    ToolExecutionUpdate(PiIgnore),
    #[serde(rename = "tool_execution_end")]
    ToolExecutionEnd(PiToolEnd),
    #[serde(rename = "turn_end")]
    TurnEnd(PiIgnore),
    #[serde(rename = "agent_end")]
    AgentEnd(PiAgentEnd),
    #[serde(rename = "auto_retry_start")]
    AutoRetryStart(PiAutoRetryStart),
    #[serde(rename = "auto_retry_end")]
    AutoRetryEnd(PiAutoRetryEnd),
    #[serde(rename = "compaction_start")]
    CompactionStart(PiIgnore),
    #[serde(rename = "compaction_end")]
    CompactionEnd(PiCompactionEnd),
    #[serde(rename = "queue_update")]
    QueueUpdate(PiIgnore),
    #[serde(rename = "entry_appended")]
    EntryAppended(PiIgnore),
    #[serde(rename = "session_info_changed")]
    SessionInfoChanged(PiIgnore),
    #[serde(rename = "thinking_level_changed")]
    ThinkingLevelChanged(PiIgnore),
}

impl PiEvent {
    /// Returns the JSON `type` discriminator for this event variant.
    pub const fn type_str(&self) -> &'static str {
        match self {
            PiEvent::Session(_) => "session",
            PiEvent::AgentStart(_) => "agent_start",
            PiEvent::TurnStart(_) => "turn_start",
            PiEvent::MessageStart(_) => "message_start",
            PiEvent::MessageUpdate(_) => "message_update",
            PiEvent::MessageEnd(_) => "message_end",
            PiEvent::ToolExecutionStart(_) => "tool_execution_start",
            PiEvent::ToolExecutionUpdate(_) => "tool_execution_update",
            PiEvent::ToolExecutionEnd(_) => "tool_execution_end",
            PiEvent::TurnEnd(_) => "turn_end",
            PiEvent::AgentEnd(_) => "agent_end",
            PiEvent::AutoRetryStart(_) => "auto_retry_start",
            PiEvent::AutoRetryEnd(_) => "auto_retry_end",
            PiEvent::CompactionStart(_) => "compaction_start",
            PiEvent::CompactionEnd(_) => "compaction_end",
            PiEvent::QueueUpdate(_) => "queue_update",
            PiEvent::EntryAppended(_) => "entry_appended",
            PiEvent::SessionInfoChanged(_) => "session_info_changed",
            PiEvent::ThinkingLevelChanged(_) => "thinking_level_changed",
        }
    }
}

/// `session` header (first line in JSON mode).
#[derive(Debug, Default, Deserialize)]
pub struct PiSession {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub version: Option<u64>,
}

/// `message_update` envelope. Streaming assistant deltas nest under
/// `assistantMessageEvent`, whose own `type` selects text vs thinking vs error.
#[derive(Debug, Default, Deserialize)]
pub struct PiMessageUpdate {
    #[serde(default, rename = "assistantMessageEvent")]
    pub assistant_message_event: Option<PiAssistantMessageEvent>,
}

/// Nested assistant delta. `type` is one of `text_delta`, `thinking_delta`,
/// `text_start`/`text_end`, `thinking_start`/`thinking_end`,
/// `toolcall_start`/`toolcall_delta`/`toolcall_end`, `start`, `done`, `error`.
/// Only `*_delta` carry a `delta` payload the parser renders.
#[derive(Debug, Default, Deserialize)]
pub struct PiAssistantMessageEvent {
    #[serde(default, rename = "type")]
    pub event_type: Option<String>,
    #[serde(default)]
    pub delta: Option<String>,
    #[serde(default, rename = "errorMessage")]
    pub error_message: Option<String>,
}

/// `message_end` envelope carrying the completed assistant message.
#[derive(Debug, Default, Deserialize)]
pub struct PiMessageEnvelope {
    #[serde(default)]
    pub message: Option<PiAssistantMessage>,
}

/// Completed assistant message. Usage/cost are per-message; `stopReason`
/// distinguishes success (`stop`/`tool_use`) from failure (`error`/`aborted`).
#[derive(Debug, Default, Deserialize)]
pub struct PiAssistantMessage {
    #[serde(default)]
    pub usage: Option<PiUsage>,
    #[serde(default, rename = "stopReason")]
    pub stop_reason: Option<String>,
    #[serde(default, rename = "errorMessage")]
    pub error_message: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
}

/// Per-message token usage. `totalTokens` maps to the normalized `total`;
/// `cost.total` is the monetary total Pi already computed.
#[derive(Debug, Default, Deserialize)]
pub struct PiUsage {
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
    #[serde(default, rename = "cacheRead")]
    pub cache_read: Option<u64>,
    #[serde(default, rename = "totalTokens")]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub cost: Option<PiCost>,
}

/// Monetary cost breakdown; only the `total` is consumed.
#[derive(Debug, Default, Deserialize)]
pub struct PiCost {
    #[serde(default)]
    pub total: Option<f64>,
}

/// `tool_execution_start`: tool call requested by the model.
#[derive(Debug, Default, Deserialize)]
pub struct PiToolStart {
    #[serde(default, rename = "toolCallId")]
    pub tool_call_id: Option<String>,
    #[serde(default, rename = "toolName")]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub args: Option<Value>,
}

/// `tool_execution_end`: tool result. Pi does not normalize a top-level exit
/// code; `isError` is the success signal and `result` carries the output.
#[derive(Debug, Default, Deserialize)]
pub struct PiToolEnd {
    #[serde(default, rename = "toolCallId")]
    pub tool_call_id: Option<String>,
    #[serde(default, rename = "toolName")]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default, rename = "isError")]
    pub is_error: Option<bool>,
}

/// `agent_end`: the terminal stream event for a run.
#[derive(Debug, Default, Deserialize)]
pub struct PiAgentEnd {
    #[serde(default, rename = "willRetry")]
    pub will_retry: Option<bool>,
}

/// `auto_retry_start`: transient provider retry attempt.
#[derive(Debug, Default, Deserialize)]
pub struct PiAutoRetryStart {
    #[serde(default)]
    pub attempt: Option<u64>,
    #[serde(default, rename = "maxAttempts")]
    pub max_attempts: Option<u64>,
    #[serde(default, rename = "errorMessage")]
    pub error_message: Option<String>,
}

/// `auto_retry_end`: final retry outcome. `success: false` + `finalError` is a
/// terminal failure.
#[derive(Debug, Default, Deserialize)]
pub struct PiAutoRetryEnd {
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub attempt: Option<u64>,
    #[serde(default, rename = "finalError")]
    pub final_error: Option<String>,
}

/// `compaction_end`: overflow-recovery outcome; a non-null `errorMessage`
/// indicates a failed compaction.
#[derive(Debug, Default, Deserialize)]
pub struct PiCompactionEnd {
    #[serde(default, rename = "errorMessage")]
    pub error_message: Option<String>,
}

/// A recognized-but-ignored lifecycle event. Deserializes from any JSON object
/// (serde ignores unknown fields by default), so listing an event as
/// [`PiIgnore`] silences it without a `ProviderExtension` fallthrough.
#[derive(Debug, Default, Deserialize)]
pub struct PiIgnore {}
