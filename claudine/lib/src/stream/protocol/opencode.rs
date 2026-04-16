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

/// Error event with flat `error_type`/`error_message` OR nested `error` object.
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
    pub message: Option<String>,
}

impl OpenCodeError {
    pub fn resolved_kind(&self) -> Option<String> {
        self.error_type
            .clone()
            .or_else(|| self.error.as_ref().and_then(|e| e.kind.clone()))
    }

    pub fn resolved_message(&self) -> Option<String> {
        self.error_message
            .clone()
            .or_else(|| self.error.as_ref().and_then(|e| e.message.clone()))
            .or_else(|| self.message.clone())
    }
}

/// Fields shared between top-level tool events and nested `part` objects.
/// OpenCode emits tool fields in both locations depending on whether the
/// event is in the legacy flat format or the new structured format.
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
            }
        }
        resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> OpenCodeEvent {
        serde_json::from_str(line).expect("valid OpenCodeEvent")
    }

    #[test]
    fn opencode_init_deserializes() {
        let event = parse(r#"{"type":"init","model":"gpt-4-turbo"}"#);
        let OpenCodeEvent::Init(init) = event else {
            panic!("expected Init");
        };
        assert_eq!(init.model.as_deref(), Some("gpt-4-turbo"));
    }

    #[test]
    fn opencode_step_start_camel_case_session_id() {
        let event = parse(r#"{"type":"step_start","sessionID":"ses_abc","part":{"id":"prt_1"}}"#);
        let OpenCodeEvent::StepStart(step) = event else {
            panic!("expected StepStart");
        };
        assert_eq!(step.resolved_session_id(), Some("ses_abc".into()));
    }

    #[test]
    fn opencode_text_from_part() {
        let event = parse(r#"{"type":"text","part":{"id":"prt_2","type":"text","text":"hello"}}"#);
        let OpenCodeEvent::Text(text) = event else {
            panic!("expected Text");
        };
        assert_eq!(text.resolved_text(), Some("hello".into()));
    }

    #[test]
    fn opencode_text_from_top_level() {
        let event = parse(r#"{"type":"text","text":"legacy"}"#);
        let OpenCodeEvent::Text(text) = event else {
            panic!("expected Text");
        };
        assert_eq!(text.resolved_text(), Some("legacy".into()));
    }

    #[test]
    fn opencode_text_from_content_fallback() {
        let event = parse(r#"{"type":"text","content":"from content"}"#);
        let OpenCodeEvent::Text(text) = event else {
            panic!("expected Text");
        };
        assert_eq!(text.resolved_text(), Some("from content".into()));
    }

    #[test]
    fn opencode_step_finish_with_tokens() {
        let event = parse(
            r#"{"type":"step_finish","part":{"reason":"stop","cost":0.0205797,"tokens":{"total":54665,"input":150,"output":23,"cache":{"read":100}}}}"#,
        );
        let OpenCodeEvent::StepFinish(sf) = event else {
            panic!("expected StepFinish");
        };
        let part = sf.part.expect("part");
        assert_eq!(part.reason.as_deref(), Some("stop"));
        assert_eq!(part.cost, Some(0.0205797));
        let tokens = part.tokens.expect("tokens");
        assert_eq!(tokens.input, Some(150));
        assert_eq!(tokens.output, Some(23));
        assert_eq!(tokens.total, Some(54665));
        assert_eq!(tokens.cache.expect("cache").read, Some(100));
    }

    #[test]
    fn opencode_step_complete_legacy_usage() {
        let event = parse(
            r#"{"type":"step_complete","usage":{"input_tokens":100,"output_tokens":50},"cost_usd":0.001}"#,
        );
        let OpenCodeEvent::StepComplete(sc) = event else {
            panic!("expected StepComplete");
        };
        let usage = sc.usage.expect("usage");
        assert_eq!(usage.input_tokens, Some(100));
        assert_eq!(usage.output_tokens, Some(50));
        assert_eq!(sc.cost_usd, Some(0.001));
    }

    #[test]
    fn opencode_error_flat_fields() {
        let event = parse(r#"{"type":"error","error_message":"API timeout"}"#);
        let OpenCodeEvent::Error(err) = event else {
            panic!("expected Error");
        };
        assert_eq!(err.resolved_message(), Some("API timeout".into()));
    }

    #[test]
    fn opencode_tool_name_from_top_level_name() {
        let event = parse(r#"{"type":"tool_use","name":"search"}"#);
        let OpenCodeEvent::ToolUse(tool) = event else {
            panic!("expected ToolUse");
        };
        let resolved = tool.resolve();
        assert_eq!(resolved.name.as_deref(), Some("search"));
    }

    #[test]
    fn opencode_tool_name_from_nested_part() {
        let event = parse(r#"{"type":"tool_use","part":{"name":"search"}}"#);
        let OpenCodeEvent::ToolUse(tool) = event else {
            panic!("expected ToolUse");
        };
        let resolved = tool.resolve();
        assert_eq!(resolved.name.as_deref(), Some("search"));
    }

    #[test]
    fn opencode_tool_name_camel_case_nested() {
        let event = parse(r#"{"type":"tool_start","part":{"tool_name":"write_file"}}"#);
        let OpenCodeEvent::ToolStart(tool) = event else {
            panic!("expected ToolStart");
        };
        let resolved = tool.resolve();
        assert_eq!(resolved.name.as_deref(), Some("write_file"));
    }

    #[test]
    fn opencode_tool_all_fields_from_part() {
        let event = parse(
            r#"{"type":"tool_start","part":{"id":"tool-1","tool_name":"bash","input":{"command":"git status"}}}"#,
        );
        let OpenCodeEvent::ToolStart(tool) = event else {
            panic!("expected ToolStart");
        };
        let resolved = tool.resolve();
        assert_eq!(resolved.id.as_deref(), Some("tool-1"));
        assert_eq!(resolved.name.as_deref(), Some("bash"));
        let input = resolved.input.expect("input");
        assert_eq!(
            input.get("command").and_then(Value::as_str),
            Some("git status")
        );
    }

    #[test]
    fn opencode_tool_result_from_part_content_and_status() {
        let event = parse(
            r#"{"type":"tool_end","part":{"tool_use_id":"tool-1","status":"success","content":"working tree clean"}}"#,
        );
        let OpenCodeEvent::ToolEnd(tool) = event else {
            panic!("expected ToolEnd");
        };
        let resolved = tool.resolve();
        assert_eq!(resolved.id.as_deref(), Some("tool-1"));
        assert_eq!(resolved.status.as_deref(), Some("success"));
        let response = resolved.output.expect("output");
        assert_eq!(response.as_str(), Some("working tree clean"));
    }

    #[test]
    fn opencode_tool_state_fills_when_part_missing_fields() {
        // When tool info lives under `part.state.*` (current OpenCode wire format
        // per message-v2.ts) and is absent at top-level and `part.*`, resolve()
        // must pick it up.
        let event = parse(
            r#"{"type":"tool_use","part":{"id":"t1","tool":"bash",
                 "state":{"status":"completed","input":{"command":"ls -la"},"output":"file.txt"}}}"#,
        );
        let OpenCodeEvent::ToolUse(tool) = event else {
            panic!("expected ToolUse");
        };
        let resolved = tool.resolve();
        assert_eq!(resolved.id.as_deref(), Some("t1"));
        assert_eq!(resolved.name.as_deref(), Some("bash"));
        assert_eq!(resolved.status.as_deref(), Some("completed"));
        let input = resolved.input.expect("input from part.state");
        assert_eq!(
            input.get("command").and_then(Value::as_str),
            Some("ls -la")
        );
        let output = resolved.output.expect("output from part.state");
        assert_eq!(output.as_str(), Some("file.txt"));
    }

    #[test]
    fn opencode_tool_part_fields_beat_state_when_both_present() {
        // Priority: top-level > part.fields > part.state. When part.fields and
        // part.state both carry `status`, part.fields must win.
        let event = parse(
            r#"{"type":"tool_use","part":{"id":"t1","tool":"bash","status":"from_part",
                 "state":{"status":"from_state"}}}"#,
        );
        let OpenCodeEvent::ToolUse(tool) = event else {
            panic!("expected ToolUse");
        };
        let resolved = tool.resolve();
        assert_eq!(resolved.status.as_deref(), Some("from_part"));
    }

    #[test]
    fn opencode_tool_args_params_aliases() {
        let event = parse(r#"{"type":"tool_use","args":{"x":1}}"#);
        let OpenCodeEvent::ToolUse(tool) = event else {
            panic!("expected ToolUse");
        };
        let resolved = tool.resolve();
        let input = resolved.input.expect("input");
        assert_eq!(input.get("x").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn opencode_unknown_event_type_fails_typed() {
        let err = serde_json::from_str::<OpenCodeEvent>(r#"{"type":"not_a_real_event"}"#);
        assert!(err.is_err());
    }

    #[test]
    fn opencode_reasoning_top_level_text_deserializes() {
        let event = parse(r#"{"type":"reasoning","text":"Thinking it over"}"#);
        let OpenCodeEvent::Reasoning(reasoning) = event else {
            panic!("expected Reasoning");
        };
        assert_eq!(reasoning.resolved_text(), Some("Thinking it over".into()));
    }

    #[test]
    fn opencode_reasoning_nested_part_text_resolves() {
        let event = parse(
            r#"{"type":"reasoning","part":{"id":"prt_1","type":"reasoning","text":"nested prose"}}"#,
        );
        let OpenCodeEvent::Reasoning(reasoning) = event else {
            panic!("expected Reasoning");
        };
        assert_eq!(reasoning.resolved_text(), Some("nested prose".into()));
    }
}
