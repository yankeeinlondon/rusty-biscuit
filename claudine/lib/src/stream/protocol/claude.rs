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
mod tests;
