//! Typed event models for Kimi Code's `stream-json` format.

use serde::Deserialize;
use serde_json::Value;

/// Tagged enum over all Kimi stream event variants dispatched by the parser.
/// Unknown event types fail typed deserialization and are handled by the
/// parser's fallback arm.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum KimiEvent {
    #[serde(rename = "init")]
    Init(KimiInit),
    #[serde(rename = "system")]
    System(KimiInit),
    #[serde(rename = "assistant")]
    Assistant(KimiContent),
    #[serde(rename = "message")]
    Message(KimiContent),
    #[serde(rename = "content")]
    Content(KimiContent),
    #[serde(rename = "ContentPart")]
    ContentPart(KimiContent),
    #[serde(rename = "StatusUpdate")]
    StatusUpdatePascal(KimiStatusUpdate),
    #[serde(rename = "status_update")]
    StatusUpdate(KimiStatusUpdate),
    #[serde(rename = "status")]
    Status(KimiStatusUpdate),
    #[serde(rename = "error")]
    Error(KimiErrorEvent),
    #[serde(rename = "tool_use")]
    ToolUse(KimiTool),
    #[serde(rename = "tool_result")]
    ToolResult(KimiTool),
}

#[derive(Debug, Default, Deserialize)]
pub struct KimiInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Content/message payload. Text can arrive as:
/// 1. `content` array with `{text: ...}` parts
/// 2. top-level `text` field
/// 3. `content` as a plain string
#[derive(Debug, Default, Deserialize)]
pub struct KimiContent {
    #[serde(default)]
    pub content: Option<Value>,
    #[serde(default)]
    pub text: Option<String>,
}

impl KimiContent {
    /// Extract the text payload with the three-way fallback behavior of the
    /// legacy parser: array → top-level text → content as string.
    pub fn resolved_text(self) -> Option<String> {
        if let Some(content) = &self.content
            && let Some(parts) = content.as_array()
        {
            let mut collected = String::new();
            for part in parts {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    collected.push_str(text);
                }
            }
            if !collected.is_empty() {
                return Some(collected);
            }
        }
        if let Some(text) = self.text
            && !text.is_empty()
        {
            return Some(text);
        }
        if let Some(content) = self.content
            && let Some(s) = content.as_str()
            && !s.is_empty()
        {
            return Some(s.to_string());
        }
        None
    }
}

/// `StatusUpdate` payload — Kimi has no dedicated `result` event, so the
/// summary is built up from the last status update snapshot plus the child
/// exit code.
#[derive(Debug, Default, Deserialize)]
pub struct KimiStatusUpdate {
    #[serde(default)]
    pub usage: Option<KimiUsage>,
    #[serde(default)]
    pub token_usage: Option<KimiUsage>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u64>,
    #[serde(default)]
    pub context_usage: Option<KimiContextUsage>,
    #[serde(default)]
    pub context: Option<KimiContextUsage>,
}

impl KimiStatusUpdate {
    pub fn resolved_usage(&mut self) -> Option<KimiUsage> {
        self.usage.take().or_else(|| self.token_usage.take())
    }

    pub fn resolved_context(&mut self) -> Option<KimiContextUsage> {
        self.context_usage.take().or_else(|| self.context.take())
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct KimiUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct KimiContextUsage {
    #[serde(default)]
    pub used: Option<u64>,
    #[serde(default)]
    pub total: Option<u64>,
    #[serde(default)]
    pub percent: Option<f64>,
}

impl KimiContextUsage {
    /// Return the percent used, computing it from `used` / `total` when the
    /// provider doesn't pre-compute it.
    pub fn computed_percent(&self) -> Option<f64> {
        self.percent.or_else(|| match (self.used, self.total) {
            (Some(u), Some(t)) if t > 0 => Some((u as f64 / t as f64) * 100.0),
            _ => None,
        })
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct KimiErrorEvent {
    #[serde(default)]
    pub error: Option<KimiErrorDetail>,
    #[serde(default)]
    pub message: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
pub struct KimiErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

impl KimiErrorEvent {
    pub fn resolved_kind(&self) -> Option<String> {
        self.error.as_ref().and_then(|e| e.kind.clone())
    }

    pub fn resolved_message(&self) -> Option<String> {
        self.error
            .as_ref()
            .and_then(|e| e.message.clone())
            .or_else(|| self.message.clone())
    }
}

/// Tool event struct. Input field accepts five aliases (`input`,
/// `parameters`, `arguments`, `args`, `params`); output field accepts
/// three (`output`, `result`, `content`).
#[derive(Debug, Default, Deserialize)]
pub struct KimiTool {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub tool_use_id: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
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

impl KimiTool {
    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.name.as_deref().or(self.tool_name.as_deref())
    }

    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.id
            .as_deref()
            .or(self.tool_id.as_deref())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> KimiEvent {
        serde_json::from_str(line).expect("valid KimiEvent")
    }

    #[test]
    fn kimi_init_deserializes() {
        let event = parse(r#"{"type":"init","session_id":"kimi-1","model":"kimi-coder"}"#);
        let KimiEvent::Init(init) = event else {
            panic!("expected Init");
        };
        assert_eq!(init.session_id.as_deref(), Some("kimi-1"));
    }

    #[test]
    fn kimi_content_from_array() {
        let event =
            parse(r#"{"type":"assistant","content":[{"text":"Hello "},{"text":"from Kimi"}]}"#);
        let KimiEvent::Assistant(content) = event else {
            panic!("expected Assistant");
        };
        assert_eq!(content.resolved_text(), Some("Hello from Kimi".into()));
    }

    #[test]
    fn kimi_content_from_top_level_text() {
        let event = parse(r#"{"type":"message","text":"plain text"}"#);
        let KimiEvent::Message(content) = event else {
            panic!("expected Message");
        };
        assert_eq!(content.resolved_text(), Some("plain text".into()));
    }

    #[test]
    fn kimi_content_from_string_content() {
        let event = parse(r#"{"type":"content","content":"string payload"}"#);
        let KimiEvent::Content(content) = event else {
            panic!("expected Content");
        };
        assert_eq!(content.resolved_text(), Some("string payload".into()));
    }

    #[test]
    fn kimi_status_update_pascal_case() {
        let event = parse(
            r#"{"type":"StatusUpdate","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":3000}"#,
        );
        let KimiEvent::StatusUpdatePascal(mut status) = event else {
            panic!("expected StatusUpdatePascal");
        };
        assert_eq!(status.duration_ms, Some(3000));
        let usage = status.resolved_usage().expect("usage");
        assert_eq!(usage.input_tokens, Some(100));
    }

    #[test]
    fn kimi_context_usage_computed_percent() {
        let event =
            parse(r#"{"type":"StatusUpdate","context_usage":{"used":110000,"total":128000}}"#);
        let KimiEvent::StatusUpdatePascal(mut status) = event else {
            panic!("expected StatusUpdatePascal");
        };
        let ctx = status.resolved_context().expect("context");
        let pct = ctx.computed_percent().expect("percent");
        assert!(pct > 85.0 && pct < 90.0);
    }

    #[test]
    fn kimi_context_usage_provided_percent() {
        let event = parse(
            r#"{"type":"StatusUpdate","context_usage":{"used":100,"total":200,"percent":55.5}}"#,
        );
        let KimiEvent::StatusUpdatePascal(mut status) = event else {
            panic!("expected StatusUpdatePascal");
        };
        let ctx = status.resolved_context().expect("context");
        assert_eq!(ctx.computed_percent(), Some(55.5));
    }

    #[test]
    fn kimi_error_event_deserializes() {
        let event = parse(r#"{"type":"error","error":{"type":"api","message":"boom"}}"#);
        let KimiEvent::Error(err) = event else {
            panic!("expected Error");
        };
        assert_eq!(err.resolved_kind(), Some("api".into()));
        assert_eq!(err.resolved_message(), Some("boom".into()));
    }

    #[test]
    fn kimi_tool_use_with_args_alias() {
        let event =
            parse(r#"{"type":"tool_use","id":"k1","name":"bash","args":{"command":"git status"}}"#);
        let KimiEvent::ToolUse(mut tool) = event else {
            panic!("expected ToolUse");
        };
        assert_eq!(tool.resolved_tool_name(), Some("bash"));
        assert_eq!(tool.resolved_tool_id(), Some("k1"));
        let input = tool.take_input().expect("input");
        assert_eq!(
            input.get("command").and_then(Value::as_str),
            Some("git status")
        );
    }

    #[test]
    fn kimi_tool_result_with_content_output() {
        let event = parse(
            r#"{"type":"tool_result","tool_use_id":"k1","status":"success","content":"clean"}"#,
        );
        let KimiEvent::ToolResult(mut tool) = event else {
            panic!("expected ToolResult");
        };
        assert_eq!(tool.resolved_tool_id(), Some("k1"));
        assert_eq!(tool.status.as_deref(), Some("success"));
        let output = tool.take_output().expect("output");
        assert_eq!(output.as_str(), Some("clean"));
    }

    #[test]
    fn kimi_unknown_event_type_fails_typed() {
        let err = serde_json::from_str::<KimiEvent>(r#"{"type":"not_a_kimi_event"}"#);
        assert!(err.is_err());
    }
}
