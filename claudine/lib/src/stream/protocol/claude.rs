//! Typed event models for Claude Code's `stream-json` format.
//!
//! Each variant of [`ClaudeEvent`] corresponds to a `type` string emitted by
//! Claude Code. Every struct uses `#[serde(default)]` on every field so the
//! parser silently tolerates absent or extra fields, matching the existing
//! `serde_json::Value`-based extraction behavior.

use chrono::{DateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Tagged enum over all Claude Code stream event variants that the parser
/// dispatches on. Unknown event types fail to deserialize and are handled by
/// the parser's fallback arm.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum ClaudeEvent {
    #[serde(rename = "init")]
    Init(ClaudeInit),
    #[serde(rename = "system")]
    System(ClaudeInit),
    #[serde(rename = "assistant")]
    Assistant(ClaudeAssistant),
    #[serde(rename = "content_block_start")]
    ContentBlockStart(ClaudeContentBlockStart),
    #[serde(rename = "content_block_delta")]
    ContentBlockDelta(ClaudeContentBlockDelta),
    #[serde(rename = "error")]
    Error(ClaudeErrorEvent),
    #[serde(rename = "assistant.error")]
    AssistantError(ClaudeErrorEvent),
    #[serde(rename = "result")]
    Result(ClaudeResult),
    #[serde(rename = "rate_limit_event")]
    RateLimit(ClaudeRateLimit),
    #[serde(rename = "tool_use")]
    ToolUse(ClaudeToolUse),
    #[serde(rename = "tool_result")]
    ToolResult(ClaudeToolResult),
    #[serde(rename = "user")]
    User(ClaudeUser),
    #[serde(rename = "task_started")]
    TaskStarted(ClaudeTaskEvent),
    #[serde(rename = "task_progress")]
    TaskProgress(ClaudeTaskEvent),
    #[serde(rename = "task_notification")]
    TaskNotification(ClaudeTaskEvent),
    #[serde(rename = "task_completed")]
    TaskCompleted(ClaudeTaskEvent),
    #[serde(rename = "system/api_retry")]
    SystemApiRetry(ClaudeApiRetry),
}

impl ClaudeEvent {
    /// Returns the JSON `type` discriminator for this event variant.
    pub const fn type_str(&self) -> &'static str {
        match self {
            ClaudeEvent::Init(_) => "init",
            ClaudeEvent::System(_) => "system",
            ClaudeEvent::Assistant(_) => "assistant",
            ClaudeEvent::ContentBlockStart(_) => "content_block_start",
            ClaudeEvent::ContentBlockDelta(_) => "content_block_delta",
            ClaudeEvent::Error(_) => "error",
            ClaudeEvent::AssistantError(_) => "assistant.error",
            ClaudeEvent::Result(_) => "result",
            ClaudeEvent::RateLimit(_) => "rate_limit_event",
            ClaudeEvent::ToolUse(_) => "tool_use",
            ClaudeEvent::ToolResult(_) => "tool_result",
            ClaudeEvent::User(_) => "user",
            ClaudeEvent::TaskStarted(_) => "task_started",
            ClaudeEvent::TaskProgress(_) => "task_progress",
            ClaudeEvent::TaskNotification(_) => "task_notification",
            ClaudeEvent::TaskCompleted(_) => "task_completed",
            ClaudeEvent::SystemApiRetry(_) => "system/api_retry",
        }
    }
}

/// Session metadata emitted by Claude Code's `init` and `system` events.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default, rename = "apiKeySource")]
    pub api_key_source: Option<String>,
    /// `subtype` is present on `system` events (e.g. `"init"`). Unused today
    /// but captured for future diagnostics.
    #[serde(default)]
    pub subtype: Option<String>,
    /// Dynamic fallback for unknown fields so the raw payload can be
    /// reconstructed without a second parse.
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

/// Full assistant message event. Claude Code nests the message under a
/// `message` key, while the simplified test format puts `content` at the top
/// level. Both forms are accepted.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeAssistant {
    #[serde(default)]
    pub message: Option<ClaudeAssistantMessage>,
    #[serde(default)]
    pub content: Option<Vec<ClaudeContentPart>>,
    /// Newer Claude Code releases attach a top-level `error` discriminator to
    /// `assistant` envelopes when the model's reply is a synthetic failure
    /// (e.g. `"billing_error"`). The nested `message.content[0].text` then
    /// carries the human-readable message.
    #[serde(default)]
    pub error: Option<String>,
}

/// `user` event carrying a replay of the user's previous turn or a nested
/// `tool_result` block. The parser extracts tool_result blocks into
/// [`SemanticEvent::ToolResult`](crate::stream::semantic::SemanticEvent::ToolResult);
/// text blocks are silently dropped because they only echo input.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeUser {
    #[serde(default)]
    pub message: Option<ClaudeUserMessage>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeUserMessage {
    #[serde(default)]
    pub content: Option<Vec<Value>>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeAssistantMessage {
    #[serde(default)]
    pub content: Option<Vec<ClaudeContentPart>>,
    #[serde(default)]
    pub role: Option<String>,
}

/// A single `content` array entry. The parser only looks at entries where
/// `kind == "text"` for plain text extraction; other kinds (images, tool_use)
/// are dispatched through dedicated event variants.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeContentPart {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<Value>,
}

impl ClaudeContentPart {
    pub fn into_tool_use(self) -> Option<ClaudeToolUse> {
        if self.kind.as_deref() != Some("tool_use") {
            return None;
        }
        Some(ClaudeToolUse {
            id: self.id,
            tool_use_id: self.tool_use_id,
            name: self.name,
            tool_name: self.tool_name,
            input: self.input,
        })
    }
}

/// `content_block_start` event — when `content_block.kind == "tool_use"` the
/// parser dispatches this through the tool-use pipeline.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeContentBlockStart {
    #[serde(default)]
    pub content_block: Option<ClaudeContentBlock>,
    #[serde(default)]
    pub index: Option<usize>,
}

/// Nested `content_block` payload on `content_block_start` events. Shares its
/// tool-related fields with [`ClaudeToolUse`] so the parser can forward it
/// into the same handler.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeContentBlock {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<Value>,
}

impl ClaudeContentBlock {
    /// Convert a `content_block_start` tool_use block into a [`ClaudeToolUse`]
    /// so the parser can reuse the common tool-use handler.
    pub fn into_tool_use(self) -> ClaudeToolUse {
        ClaudeToolUse {
            id: self.id,
            tool_use_id: None,
            name: self.name,
            tool_name: self.tool_name,
            input: self.input,
        }
    }
}

/// `content_block_delta` event. The nested `delta` is a flat struct with a
/// `kind` discriminator because Claude emits multiple delta variants
/// (`text_delta`, `thinking_delta`, `input_json_delta`) and we want each one
/// to tolerate missing fields without failing deserialization.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeContentBlockDelta {
    #[serde(default)]
    pub delta: Option<ClaudeDelta>,
    #[serde(default)]
    pub index: Option<usize>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeDelta {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub thinking: Option<String>,
    #[serde(default)]
    pub partial_json: Option<String>,
}

/// `error` / `assistant.error` event. The nested `error` object carries the
/// `kind` and `message` that the parser surfaces in the execution summary.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeErrorEvent {
    #[serde(default)]
    pub error: Option<ClaudeErrorDetail>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Terminal `result` event. All fields are optional because Claude omits many
/// of them in practice, and the parser merges whatever is present into the
/// `StreamExecutionSummary`.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeResult {
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub duration_api_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u64>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// Newer Claude Code releases use `total_cost_usd`.
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
    /// Older Claude Code releases used `cost_usd`; kept as a fallback.
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub usage: Option<ClaudeUsage>,
    /// Newer `result` envelopes flag overall failure independent of
    /// `stop_reason` (e.g. `is_error=true` with `stop_reason="stop_sequence"`).
    #[serde(default)]
    pub is_error: Option<bool>,
    /// Human-readable result text — carries the error message when
    /// `is_error=true`.
    #[serde(default)]
    pub result: Option<String>,
    /// List of permission-denial records; shape varies by provider release.
    #[serde(default)]
    pub permission_denials: Option<Vec<Value>>,
    /// Terminal lifecycle reason (e.g. `"completed"`, `"cancelled"`).
    #[serde(default)]
    pub terminal_reason: Option<String>,
    /// Presence of fast-mode state in the terminal envelope.
    #[serde(default)]
    pub fast_mode_state: Option<String>,
    /// Per-model usage map; shape is provider-defined.
    #[serde(default, rename = "modelUsage")]
    pub model_usage: Option<Value>,
    /// Dynamic fallback for unknown fields so the raw payload can be
    /// reconstructed without a second parse.
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

impl ClaudeResult {
    /// Returns the cost in USD, preferring `total_cost_usd` over the legacy
    /// `cost_usd` field.
    pub fn effective_cost_usd(&self) -> Option<f64> {
        self.total_cost_usd.or(self.cost_usd)
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

/// `rate_limit_event` — surfaces throttling notifications.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeRateLimit {
    #[serde(default)]
    pub is_throttled: Option<bool>,
    #[serde(default)]
    pub retry_after_ms: Option<u64>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default, rename = "rate_limit_info")]
    pub rate_limit_info: Option<ClaudeRateLimitInfo>,
    #[serde(default, rename = "resetsAt")]
    pub resets_at: Option<i64>,
    #[serde(default, rename = "reset_at")]
    pub reset_at_seconds: Option<i64>,
    /// Dynamic fallback for unknown fields so the raw payload can be
    /// reconstructed without a second parse.
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeRateLimitInfo {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, rename = "resetsAt")]
    pub resets_at: Option<i64>,
    #[serde(default, rename = "rateLimitType")]
    pub rate_limit_type: Option<String>,
    #[serde(default, rename = "overageStatus")]
    pub overage_status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

impl ClaudeRateLimit {
    pub fn resolved_message(&self) -> Option<&str> {
        self.message
            .as_deref()
            .or_else(|| self.rate_limit_info.as_ref()?.message.as_deref())
    }

    pub fn resolved_status(&self) -> Option<&str> {
        self.rate_limit_info.as_ref()?.status.as_deref()
    }

    pub fn resolved_rate_limit_type(&self) -> Option<&str> {
        self.rate_limit_info.as_ref()?.rate_limit_type.as_deref()
    }

    pub fn resolved_overage_status(&self) -> Option<&str> {
        self.rate_limit_info.as_ref()?.overage_status.as_deref()
    }

    pub fn resolved_is_throttled(&self) -> Option<bool> {
        self.is_throttled
            .or_else(|| {
                let status = self.resolved_status()?;
                Some(matches!(status, "limited" | "blocked"))
            })
            .or_else(|| {
                let overage = self.resolved_overage_status()?;
                Some(matches!(overage, "blocked"))
            })
    }

    pub fn resolved_reset_at(&self) -> Option<DateTime<Utc>> {
        let seconds = self
            .rate_limit_info
            .as_ref()
            .and_then(|info| info.resets_at)
            .or(self.resets_at)
            .or(self.reset_at_seconds)?;
        Utc.timestamp_opt(seconds, 0).single()
    }
}

/// Top-level `tool_use` event. For `content_block_start` events that wrap a
/// tool_use block, the parser constructs this struct via
/// [`ClaudeContentBlock::into_tool_use`].
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeToolUse {
    #[serde(default)]
    pub id: Option<String>,
    /// Some tool_use payloads place the id under `tool_use_id` instead.
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    /// Some providers emit `tool_name` instead of `name`.
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub input: Option<Value>,
}

impl ClaudeToolUse {
    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.id.as_deref().or(self.tool_use_id.as_deref())
    }

    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.name.as_deref().or(self.tool_name.as_deref())
    }

    pub fn take_input(&mut self) -> Option<Value> {
        self.input.take()
    }
}

/// `tool_result` event. The result payload can arrive under three different
/// keys (`content`, `output`, `result`) depending on the tool.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeToolResult {
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub content: Option<Value>,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
}

impl ClaudeToolResult {
    /// Resolve the tool id, preferring `tool_use_id` over the legacy `id`
    /// field, matching the existing parser behavior.
    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.tool_use_id.as_deref().or(self.id.as_deref())
    }

    /// Returns the first populated response payload.
    pub fn response(self) -> Option<Value> {
        self.content.or(self.output).or(self.result)
    }
}

/// Task lifecycle events (`task_started`, `task_progress`, `task_notification`,
/// `task_completed`) emitted by Claude Code's sub-agent orchestration.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeTaskEvent {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default, alias = "task_name")]
    pub task_name: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// `system/api_retry` event emitted by Claude Code when retrying an API call.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ClaudeApiRetry {
    #[serde(default)]
    pub message: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> ClaudeEvent {
        serde_json::from_str(line).expect("valid ClaudeEvent")
    }

    #[test]
    fn claude_init_deserializes() {
        let event = parse(
            r#"{"type":"init","session_id":"sess-1","model":"claude-sonnet-4-20250514","apiKeySource":"ANTHROPIC_API_KEY"}"#,
        );
        let ClaudeEvent::Init(init) = event else {
            panic!("expected Init");
        };
        assert_eq!(init.session_id.as_deref(), Some("sess-1"));
        assert_eq!(init.model.as_deref(), Some("claude-sonnet-4-20250514"));
        assert_eq!(init.api_key_source.as_deref(), Some("ANTHROPIC_API_KEY"));
    }

    #[test]
    fn claude_system_maps_to_init() {
        let event = parse(r#"{"type":"system","subtype":"init","session_id":"sess-2"}"#);
        let ClaudeEvent::System(init) = event else {
            panic!("expected System");
        };
        assert_eq!(init.session_id.as_deref(), Some("sess-2"));
        assert_eq!(init.subtype.as_deref(), Some("init"));
    }

    #[test]
    fn claude_assistant_deserializes_flat_content() {
        let event = parse(r#"{"type":"assistant","content":[{"type":"text","text":"Hello"}]}"#);
        let ClaudeEvent::Assistant(assistant) = event else {
            panic!("expected Assistant");
        };
        assert!(assistant.message.is_none());
        let parts = assistant.content.expect("content");
        assert_eq!(parts.len(), 1);
        assert_eq!(parts[0].kind.as_deref(), Some("text"));
        assert_eq!(parts[0].text.as_deref(), Some("Hello"));
    }

    #[test]
    fn claude_assistant_deserializes_with_message_key() {
        let event = parse(
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"text","text":"Nested"}]}}"#,
        );
        let ClaudeEvent::Assistant(assistant) = event else {
            panic!("expected Assistant");
        };
        let message = assistant.message.expect("message");
        assert_eq!(message.role.as_deref(), Some("assistant"));
        let parts = message.content.expect("content");
        assert_eq!(parts[0].text.as_deref(), Some("Nested"));
    }

    #[test]
    fn claude_assistant_deserializes_nested_tool_use_content() {
        let event = parse(
            r#"{"type":"assistant","message":{"role":"assistant","content":[{"type":"tool_use","id":"tu_1","name":"Bash","input":{"command":"ls -la"}}]}}"#,
        );
        let ClaudeEvent::Assistant(assistant) = event else {
            panic!("expected Assistant");
        };
        let message = assistant.message.expect("message");
        let part = message
            .content
            .expect("content")
            .into_iter()
            .next()
            .unwrap();
        let tool = part.into_tool_use().expect("tool_use");
        assert_eq!(tool.resolved_tool_id(), Some("tu_1"));
        assert_eq!(tool.resolved_tool_name(), Some("Bash"));
        assert_eq!(
            tool.input
                .as_ref()
                .and_then(|v| v.get("command"))
                .and_then(Value::as_str),
            Some("ls -la")
        );
    }

    #[test]
    fn claude_content_block_delta_text() {
        let event =
            parse(r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"chunk"}}"#);
        let ClaudeEvent::ContentBlockDelta(d) = event else {
            panic!("expected ContentBlockDelta");
        };
        let delta = d.delta.expect("delta");
        assert_eq!(delta.kind.as_deref(), Some("text_delta"));
        assert_eq!(delta.text.as_deref(), Some("chunk"));
    }

    #[test]
    fn claude_content_block_delta_thinking() {
        let event = parse(
            r#"{"type":"content_block_delta","delta":{"type":"thinking_delta","thinking":"reasoning"}}"#,
        );
        let ClaudeEvent::ContentBlockDelta(d) = event else {
            panic!("expected ContentBlockDelta");
        };
        let delta = d.delta.expect("delta");
        assert_eq!(delta.kind.as_deref(), Some("thinking_delta"));
        assert_eq!(delta.thinking.as_deref(), Some("reasoning"));
    }

    #[test]
    fn claude_error_event_deserializes() {
        let event =
            parse(r#"{"type":"error","error":{"type":"billing_error","message":"no credits"}}"#);
        let ClaudeEvent::Error(err) = event else {
            panic!("expected Error");
        };
        let detail = err.error.expect("detail");
        assert_eq!(detail.kind.as_deref(), Some("billing_error"));
        assert_eq!(detail.message.as_deref(), Some("no credits"));
    }

    #[test]
    fn claude_assistant_error_event_deserializes() {
        let event = parse(
            r#"{"type":"assistant.error","error":{"type":"rate_limit","message":"slow down"}}"#,
        );
        let ClaudeEvent::AssistantError(err) = event else {
            panic!("expected AssistantError");
        };
        let detail = err.error.expect("detail");
        assert_eq!(detail.kind.as_deref(), Some("rate_limit"));
    }

    #[test]
    fn claude_result_deserializes() {
        let event = parse(
            r#"{"type":"result","duration_ms":12345,"duration_api_ms":11000,"num_turns":1,"stop_reason":"end_turn","cost_usd":0.0042,"usage":{"input_tokens":1000,"output_tokens":500,"cache_read_input_tokens":200}}"#,
        );
        let ClaudeEvent::Result(result) = event else {
            panic!("expected Result");
        };
        assert_eq!(result.duration_ms, Some(12345));
        assert_eq!(result.duration_api_ms, Some(11000));
        assert_eq!(result.num_turns, Some(1));
        assert_eq!(result.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(result.effective_cost_usd(), Some(0.0042));
        let usage = result.usage.expect("usage");
        assert_eq!(usage.input_tokens, Some(1000));
        assert_eq!(usage.output_tokens, Some(500));
        assert_eq!(usage.cache_read_input_tokens, Some(200));
        assert!(
            result.extra.is_empty(),
            "known result fields must not land in extra; extra={:?}",
            result.extra
        );
    }

    #[test]
    fn claude_result_round_trips_through_json() {
        let line = r#"{"type":"result","duration_ms":12345,"duration_api_ms":11000,"num_turns":1,"stop_reason":"end_turn","total_cost_usd":0.0042,"usage":{"input_tokens":1000,"output_tokens":500,"cache_read_input_tokens":200}}"#;
        let event = parse(line);
        let serialized = serde_json::to_string(&event).unwrap();
        let reparsed: ClaudeEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            serde_json::to_string(&reparsed).unwrap(),
            serialized,
            "parse -> serialize -> parse should be stable for a known event"
        );
    }

    #[test]
    fn claude_result_total_cost_usd_preferred() {
        let event =
            parse(r#"{"type":"result","duration_ms":0,"total_cost_usd":0.185,"cost_usd":0.5}"#);
        let ClaudeEvent::Result(result) = event else {
            panic!("expected Result");
        };
        // total_cost_usd wins over cost_usd
        assert_eq!(result.effective_cost_usd(), Some(0.185));
    }

    #[test]
    fn claude_rate_limit_deserializes() {
        let event = parse(
            r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"slow"}"#,
        );
        let ClaudeEvent::RateLimit(rl) = event else {
            panic!("expected RateLimit");
        };
        assert_eq!(rl.is_throttled, Some(true));
        assert_eq!(rl.retry_after_ms, Some(5000));
        assert_eq!(rl.message.as_deref(), Some("slow"));
        assert!(
            rl.extra.is_empty(),
            "known rate-limit fields must not land in extra; extra={:?}",
            rl.extra
        );
    }

    #[test]
    fn claude_rate_limit_nested_metadata_deserializes() {
        let event = parse(
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"approaching_limit","resetsAt":1712000000,"rateLimitType":"usage","overageStatus":"allowed"}}"#,
        );
        let ClaudeEvent::RateLimit(rl) = event else {
            panic!("expected RateLimit");
        };
        assert_eq!(rl.resolved_status(), Some("approaching_limit"));
        assert_eq!(rl.resolved_rate_limit_type(), Some("usage"));
        assert_eq!(rl.resolved_overage_status(), Some("allowed"));
        assert_eq!(rl.resolved_is_throttled(), Some(false));
        assert_eq!(
            rl.resolved_reset_at().map(|dt| dt.timestamp()),
            Some(1712000000)
        );
    }

    #[test]
    fn claude_tool_use_deserializes() {
        let event =
            parse(r#"{"type":"tool_use","id":"tu-1","name":"bash","input":{"command":"ls"}}"#);
        let ClaudeEvent::ToolUse(tu) = event else {
            panic!("expected ToolUse");
        };
        assert_eq!(tu.id.as_deref(), Some("tu-1"));
        assert_eq!(tu.name.as_deref(), Some("bash"));
        assert_eq!(
            tu.input
                .as_ref()
                .and_then(|v| v.get("command"))
                .and_then(Value::as_str),
            Some("ls")
        );
    }

    #[test]
    fn claude_tool_result_deserializes() {
        let event =
            parse(r#"{"type":"tool_result","tool_use_id":"tu-1","content":"file contents"}"#);
        let ClaudeEvent::ToolResult(tr) = event else {
            panic!("expected ToolResult");
        };
        assert_eq!(tr.resolved_tool_id(), Some("tu-1"));
        assert_eq!(
            tr.response().and_then(|v| v.as_str().map(String::from)),
            Some("file contents".into())
        );
    }

    #[test]
    fn claude_content_block_start_tool_use() {
        let event = parse(
            r#"{"type":"content_block_start","content_block":{"type":"tool_use","id":"tu-2","name":"bash","input":{"command":"ls -la"}}}"#,
        );
        let ClaudeEvent::ContentBlockStart(cbs) = event else {
            panic!("expected ContentBlockStart");
        };
        let block = cbs.content_block.expect("content_block");
        assert_eq!(block.kind.as_deref(), Some("tool_use"));
        let tu = block.into_tool_use();
        assert_eq!(tu.id.as_deref(), Some("tu-2"));
        assert_eq!(tu.name.as_deref(), Some("bash"));
    }

    #[test]
    fn claude_init_tolerates_unknown_fields() {
        let event = parse(
            r#"{"type":"init","session_id":"sess","model":"m","tools":[{"name":"a"}],"mcp_servers":["x"]}"#,
        );
        let ClaudeEvent::Init(init) = event else {
            panic!("expected Init");
        };
        assert_eq!(init.session_id.as_deref(), Some("sess"));
    }

    #[test]
    fn claude_unknown_event_type_fails_typed_deserialization() {
        let err = serde_json::from_str::<ClaudeEvent>(
            r#"{"type":"some_unknown_event","text":"ignored"}"#,
        );
        assert!(err.is_err());
    }

    #[test]
    fn claude_missing_type_fails_typed_deserialization() {
        let err = serde_json::from_str::<ClaudeEvent>(r#"{"session_id":"sess"}"#);
        assert!(err.is_err());
    }

    #[test]
    fn claude_user_event_deserializes_with_tool_result_content() {
        let line = r#"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"tu_1","content":"hello","is_error":false}]},"session_id":"s1"}"#;
        let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
        let ClaudeEvent::User(user) = event else {
            panic!("expected ClaudeEvent::User, got {event:?}");
        };
        let content = user.message.and_then(|m| m.content).expect("content");
        assert!(
            content
                .iter()
                .any(|c: &Value| c.get("type").and_then(Value::as_str) == Some("tool_result"))
        );
    }

    #[test]
    fn claude_system_hook_subtypes_deserialize() {
        for subtype in ["hook_started", "hook_response"] {
            let line = format!(
                r#"{{"type":"system","subtype":"{subtype}","session_id":"s1","hook_name":"SessionStart"}}"#
            );
            let event: ClaudeEvent = serde_json::from_str(&line).expect("valid event");
            assert!(
                matches!(event, ClaudeEvent::System(_)),
                "subtype {subtype} must parse as System event"
            );
        }
    }

    #[test]
    fn claude_assistant_error_field_preserved() {
        let line = r#"{"type":"assistant","message":{"model":"<synthetic>","content":[{"type":"text","text":"Credit balance is too low"}]},"session_id":"s1","error":"billing_error"}"#;
        let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
        let ClaudeEvent::Assistant(a) = event else {
            panic!("expected Assistant")
        };
        assert_eq!(a.error.as_deref(), Some("billing_error"));
    }

    #[test]
    fn claude_result_fields_billing_error_surface() {
        let line = r#"{"type":"result","subtype":"success","is_error":true,"result":"Credit balance is too low","session_id":"s1","permission_denials":[],"terminal_reason":"completed","fast_mode_state":"off","total_cost_usd":0,"usage":{"input_tokens":0,"output_tokens":0,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"service_tier":"standard"},"modelUsage":{}}"#;
        let event: ClaudeEvent = serde_json::from_str(line).expect("valid event");
        let ClaudeEvent::Result(r) = event else {
            panic!("expected Result")
        };
        assert_eq!(r.is_error, Some(true));
        assert_eq!(r.result.as_deref(), Some("Credit balance is too low"));
    }
}
