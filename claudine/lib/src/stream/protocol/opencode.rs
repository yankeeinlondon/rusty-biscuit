//! Typed event models for OpenCode CLI's NDJSON event stream.
//!
//! OpenCode is the most complex provider because tool fields can appear at
//! either the top level of an event OR nested inside a `part` object. The
//! typed structs capture both shapes, and the `resolve()` helpers walk the
//! top-level fields first before falling back to the `part` nested fields —
//! matching the behavior of the legacy `opencode_value` helper.

use serde::Deserialize;
use serde_json::Value;

/// Tagged enum over all OpenCode stream event variants dispatched by the
/// parser. Unknown event types fail typed deserialization and are handled by
/// the parser's fallback arm.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum OpenCodeEvent {
    #[serde(rename = "init")]
    Init(OpenCodeInit),
    #[serde(rename = "session_start")]
    SessionStart(OpenCodeInit),
    #[serde(rename = "step_start")]
    StepStart(OpenCodeStepStart),
    #[serde(rename = "text")]
    Text(OpenCodeText),
    #[serde(rename = "text_delta")]
    TextDelta(OpenCodeText),
    #[serde(rename = "assistant_text")]
    AssistantText(OpenCodeText),
    #[serde(rename = "step_finish")]
    StepFinish(OpenCodeStepFinish),
    #[serde(rename = "step_complete")]
    StepComplete(OpenCodeStepComplete),
    #[serde(rename = "turn_complete")]
    TurnComplete(OpenCodeStepComplete),
    #[serde(rename = "error")]
    Error(OpenCodeError),
    #[serde(rename = "step_error")]
    StepError(OpenCodeError),
    #[serde(rename = "tool_use")]
    ToolUse(OpenCodeTool),
    #[serde(rename = "tool_start")]
    ToolStart(OpenCodeTool),
    #[serde(rename = "tool_result")]
    ToolResult(OpenCodeTool),
    #[serde(rename = "tool_end")]
    ToolEnd(OpenCodeTool),
    #[serde(rename = "reasoning")]
    Reasoning(OpenCodeReasoning),
    #[serde(rename = "task_started")]
    TaskStarted(OpenCodeTaskEvent),
    #[serde(rename = "task_completed")]
    TaskCompleted(OpenCodeTaskEvent),
    #[serde(rename = "task_progress")]
    TaskProgress(OpenCodeTaskProgress),
}

impl OpenCodeEvent {
    /// Returns the JSON `type` discriminator for this event variant.
    pub const fn type_str(&self) -> &'static str {
        match self {
            OpenCodeEvent::Init(_) => "init",
            OpenCodeEvent::SessionStart(_) => "session_start",
            OpenCodeEvent::StepStart(_) => "step_start",
            OpenCodeEvent::Text(_) => "text",
            OpenCodeEvent::TextDelta(_) => "text_delta",
            OpenCodeEvent::AssistantText(_) => "assistant_text",
            OpenCodeEvent::StepFinish(_) => "step_finish",
            OpenCodeEvent::StepComplete(_) => "step_complete",
            OpenCodeEvent::TurnComplete(_) => "turn_complete",
            OpenCodeEvent::Error(_) => "error",
            OpenCodeEvent::StepError(_) => "step_error",
            OpenCodeEvent::ToolUse(_) => "tool_use",
            OpenCodeEvent::ToolStart(_) => "tool_start",
            OpenCodeEvent::ToolResult(_) => "tool_result",
            OpenCodeEvent::ToolEnd(_) => "tool_end",
            OpenCodeEvent::Reasoning(_) => "reasoning",
            OpenCodeEvent::TaskStarted(_) => "task_started",
            OpenCodeEvent::TaskCompleted(_) => "task_completed",
            OpenCodeEvent::TaskProgress(_) => "task_progress",
        }
    }
}

/// `init` / `session_start` payload.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// `step_start` payload. The session ID arrives as camelCase `sessionID` in
/// real OpenCode output and is the only signal of session identity before
/// the first message.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeStepStart {
    #[serde(default, rename = "sessionID")]
    pub session_id_camel: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
}

impl OpenCodeStepStart {
    pub fn resolved_session_id(self) -> Option<String> {
        self.session_id_camel.or(self.session_id)
    }
}

/// Text event. Text can arrive in three places:
/// 1. `part.text` (real OpenCode NDJSON format)
/// 2. top-level `text` (legacy format)
/// 3. top-level `content` (legacy format)
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeText {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub part: Option<OpenCodeTextPart>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeTextPart {
    #[serde(default)]
    pub text: Option<String>,
}

impl OpenCodeText {
    pub fn resolved_text(self) -> Option<String> {
        self.part
            .and_then(|p| p.text)
            .or(self.text)
            .or(self.content)
    }
}

/// Reasoning event. Text can arrive at either the top level or nested under
/// `part.text`, mirroring [`OpenCodeText`]. The `content` fallback is retained
/// for symmetry with the text shape so callers never have to care which
/// location the provider happens to use.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeReasoning {
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub part: Option<OpenCodeReasoningPart>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeReasoningPart {
    #[serde(default)]
    pub text: Option<String>,
}

impl OpenCodeReasoning {
    pub fn resolved_text(self) -> Option<String> {
        self.part
            .and_then(|p| p.text)
            .or(self.text)
            .or(self.content)
    }
}

/// Task / subagent lifecycle event. OpenCode task records are close to the
/// Claude shape: a stable task id, a human-facing task name, and an optional
/// completion status on terminal events.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeTaskEvent {
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default, rename = "taskId")]
    pub task_id_camel: Option<String>,
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub task_name: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
}

impl OpenCodeTaskEvent {
    pub fn resolved_task_id(&self) -> Option<String> {
        self.task_id
            .clone()
            .or_else(|| self.task_id_camel.clone())
            .or_else(|| self.id.clone())
    }

    pub fn resolved_name(&self) -> Option<String> {
        self.name.clone().or_else(|| self.task_name.clone())
    }
}

/// OpenCode task progress narration. Keep the payload intentionally small and
/// alias-friendly so format drift degrades to a harmless empty `Info`.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeTaskProgress {
    #[serde(default)]
    pub message: Option<String>,
}

/// `step_finish` payload with nested `part.tokens` / `part.cost` / `part.reason`.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeStepFinish {
    #[serde(default)]
    pub part: Option<OpenCodeStepFinishPart>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeStepFinishPart {
    #[serde(default)]
    pub reason: Option<String>,
    #[serde(default)]
    pub cost: Option<f64>,
    #[serde(default)]
    pub tokens: Option<OpenCodeTokens>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeTokens {
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub output: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub cache: Option<OpenCodeCache>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeCache {
    #[serde(default)]
    pub read: Option<u64>,
}

/// Legacy `step_complete` / `turn_complete` payload.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeStepComplete {
    #[serde(default)]
    pub usage: Option<OpenCodeUsage>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
}

/// Error event with flat `error_type`/`error_message` or a nested `error` object.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeError {
    #[serde(default)]
    pub error_type: Option<String>,
    #[serde(default)]
    pub error_message: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub error: Option<OpenCodeErrorDetail>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub data: Option<OpenCodeErrorData>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeErrorData {
    #[serde(default)]
    pub message: Option<String>,
}

impl OpenCodeError {
    pub fn resolved_kind(&self) -> Option<String> {
        self.error_type
            .clone()
            .or_else(|| {
                self.error
                    .as_ref()
                    .and_then(|error| error.kind.clone().or_else(|| error.name.clone()))
            })
    }

    pub fn resolved_message(&self) -> Option<String> {
        self.error_message
            .clone()
            .or_else(|| {
                self.error.as_ref().and_then(|error| {
                    error.message.clone().or_else(|| {
                        error
                            .data
                            .as_ref()
                            .and_then(|data| data.message.clone())
                    })
                })
            })
            .or_else(|| self.message.clone())
    }
}

/// Fields shared between top-level tool events and nested `part` objects.
/// OpenCode emits tool fields in both locations depending on whether the
/// event is in the legacy flat format or the new structured format.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeToolMetadata {
    #[serde(default, rename = "sessionId")]
    pub session_id: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeToolTime {
    #[serde(default)]
    pub start: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeToolFields {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default, rename = "toolUseId")]
    pub tool_use_id_camel: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default, rename = "toolName")]
    pub tool_name_camel: Option<String>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub input: Option<Value>,
    #[serde(default)]
    pub parameters: Option<Value>,
    #[serde(default)]
    pub arguments: Option<Value>,
    #[serde(default)]
    pub args: Option<Value>,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub content: Option<Value>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error: Option<Value>,
    #[serde(default)]
    pub metadata: Option<OpenCodeToolMetadata>,
    #[serde(default)]
    pub time: Option<OpenCodeToolTime>,
}

impl OpenCodeToolFields {
    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.name
            .as_deref()
            .or(self.tool_name.as_deref())
            .or(self.tool_name_camel.as_deref())
            .or(self.tool.as_deref())
    }

    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.id
            .as_deref()
            .or(self.tool_id.as_deref())
            .or(self.tool_use_id_camel.as_deref())
            .or(self.tool_use_id.as_deref())
    }

    pub fn take_input(&mut self) -> Option<Value> {
        self.input
            .take()
            .or_else(|| self.parameters.take())
            .or_else(|| self.arguments.take())
            .or_else(|| self.args.take())
            .or_else(|| self.params.take())
    }

    pub fn take_output(&mut self) -> Option<Value> {
        self.output
            .take()
            .or_else(|| self.result.take())
            .or_else(|| self.content.take())
    }
}

/// `tool_use` / `tool_start` / `tool_result` / `tool_end` event. Fields may
/// appear at the top level, nested under `part`, or (for the current OpenCode
/// `run.ts` format) nested under `part.state` — the ToolPart `state` carries
/// `status`, `input`, `output`, and `error`. Use [`OpenCodeTool::resolve`] to
/// collapse all three locations into a single resolved view.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeTool {
    #[serde(flatten)]
    pub top: OpenCodeToolFields,
    #[serde(default)]
    pub part: Option<OpenCodeToolPart>,
}

/// `ToolPart` body: mirrors `OpenCodeToolFields` but also carries the
/// OpenCode-specific `state` sub-object where recent CLI releases place
/// `status` / `input` / `output` / `error`.
#[derive(Debug, Default, Deserialize)]
pub struct OpenCodeToolPart {
    #[serde(flatten)]
    pub fields: OpenCodeToolFields,
    #[serde(default)]
    pub state: Option<OpenCodeToolFields>,
}

/// Resolved tool view with top-level fields taking priority over nested
/// `part` fields, matching the behavior of the legacy `opencode_value` helper.
#[derive(Debug, Default)]
pub struct ResolvedOpenCodeTool {
    pub id: Option<String>,
    pub name: Option<String>,
    pub input: Option<Value>,
    pub output: Option<Value>,
    pub status: Option<String>,
    pub error: Option<Value>,
    pub metadata: Option<OpenCodeToolMetadata>,
    pub time: Option<OpenCodeToolTime>,
}

impl ResolvedOpenCodeTool {
    /// Returns the subagent session id from `metadata.sessionId` when present.
    pub fn task_subagent_id(&self) -> Option<&str> {
        self.metadata.as_ref().and_then(|m| m.session_id.as_deref())
    }

    /// Returns the task start time epoch in milliseconds from `time.start` when present.
    pub fn task_started_at_epoch_ms(&self) -> Option<u64> {
        self.time.as_ref().and_then(|t| t.start)
    }
}

impl OpenCodeTool {
    pub fn resolve(self) -> ResolvedOpenCodeTool {
        let OpenCodeTool { mut top, part } = self;
        let mut resolved = ResolvedOpenCodeTool {
            id: top.resolved_tool_id().map(ToOwned::to_owned),
            name: top.resolved_tool_name().map(ToOwned::to_owned),
            input: top.take_input(),
            output: top.take_output(),
            status: top.status.take(),
            error: top.error.take(),
            metadata: top.metadata.take(),
            time: top.time.take(),
        };
        if let Some(OpenCodeToolPart {
            fields: mut part,
            state,
        }) = part
        {
            if resolved.id.is_none() {
                resolved.id = part.resolved_tool_id().map(ToOwned::to_owned);
            }
            if resolved.name.is_none() {
                resolved.name = part.resolved_tool_name().map(ToOwned::to_owned);
            }
            if resolved.input.is_none() {
                resolved.input = part.take_input();
            }
            if resolved.output.is_none() {
                resolved.output = part.take_output();
            }
            if resolved.status.is_none() {
                resolved.status = part.status.take();
            }
            if resolved.error.is_none() {
                resolved.error = part.error.take();
            }
            if resolved.metadata.is_none() {
                resolved.metadata = part.metadata.take();
            }
            if resolved.time.is_none() {
                resolved.time = part.time.take();
            }
            if let Some(mut state) = state {
                if resolved.id.is_none() {
                    resolved.id = state.resolved_tool_id().map(ToOwned::to_owned);
                }
                if resolved.name.is_none() {
                    resolved.name = state.resolved_tool_name().map(ToOwned::to_owned);
                }
                if resolved.input.is_none() {
                    resolved.input = state.take_input();
                }
                if resolved.output.is_none() {
                    resolved.output = state.take_output();
                }
                if resolved.status.is_none() {
                    resolved.status = state.status.take();
                }
                if resolved.error.is_none() {
                    resolved.error = state.error.take();
                }
                if resolved.metadata.is_none() {
                    resolved.metadata = state.metadata.take();
                }
                if resolved.time.is_none() {
                    resolved.time = state.time.take();
                }
            }
        }
        resolved
    }
}

#[cfg(test)]
mod tests;
