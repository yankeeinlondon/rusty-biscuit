//! Typed event models for Qwen CLI's `stream-json` format.

use serde::Deserialize;
use serde_json::Value;

/// Tagged enum over all Qwen CLI stream event variants dispatched by the
/// parser. Unknown event types fail typed deserialization and are handled by
/// the parser's fallback arm.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
pub enum QwenEvent {
    #[serde(rename = "init")]
    Init(QwenInit),
    #[serde(rename = "system")]
    System(QwenSystem),
    #[serde(rename = "message")]
    Message(QwenMessage),
    #[serde(rename = "assistant_message")]
    AssistantMessage(QwenMessage),
    #[serde(rename = "assistant")]
    Assistant(QwenMessage),
    #[serde(rename = "error")]
    Error(QwenErrorEvent),
    #[serde(rename = "result")]
    Result(QwenResult),
    #[serde(rename = "summary")]
    Summary(QwenResult),
    #[serde(rename = "tool_use")]
    ToolUse(QwenTool),
    #[serde(rename = "tool_call")]
    ToolCall(QwenTool),
    #[serde(rename = "tool_result")]
    ToolResult(QwenTool),
    #[serde(rename = "tool_response")]
    ToolResponse(QwenTool),
}

#[derive(Debug, Default, Deserialize)]
pub struct QwenInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// `system` events only become session-start signals when `subtype` is
/// `"session_start"`. Other subtypes are ignored by the parser.
#[derive(Debug, Default, Deserialize)]
pub struct QwenSystem {
    #[serde(default)]
    pub subtype: Option<String>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

impl QwenSystem {
    pub fn is_session_start(&self) -> bool {
        self.subtype.as_deref() == Some("session_start")
    }

    pub fn into_init(self) -> QwenInit {
        QwenInit {
            session_id: self.session_id,
            model: self.model,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct QwenMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<Value>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QwenErrorEvent {
    #[serde(default)]
    pub error: Option<QwenErrorDetail>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QwenErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Qwen's `result` / `summary` event. Usage can arrive under `stats`,
/// `usage`, or `token_usage` depending on which Qwen build emits it.
#[derive(Debug, Default, Deserialize)]
pub struct QwenResult {
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub num_turns: Option<u64>,
    #[serde(default)]
    pub stop_reason: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub stats: Option<QwenUsage>,
    #[serde(default)]
    pub usage: Option<QwenUsage>,
    #[serde(default)]
    pub token_usage: Option<QwenUsage>,
}

impl QwenResult {
    pub fn resolved_usage(self) -> (Option<QwenUsage>, QwenResultMeta) {
        let meta = QwenResultMeta {
            duration_ms: self.duration_ms,
            num_turns: self.num_turns,
            stop_reason: self.stop_reason,
            cost_usd: self.cost_usd,
        };
        let usage = self.stats.or(self.usage).or(self.token_usage);
        (usage, meta)
    }
}

pub struct QwenResultMeta {
    pub duration_ms: Option<u64>,
    pub num_turns: Option<u64>,
    pub stop_reason: Option<String>,
    pub cost_usd: Option<f64>,
}

#[derive(Debug, Default, Deserialize)]
pub struct QwenUsage {
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub cache_read_input_tokens: Option<u64>,
}

/// Tool event struct covering both tool_use/tool_call and
/// tool_result/tool_response. Input field accepts five aliases:
/// `input`, `parameters`, `arguments`, `args`, `params`.
#[derive(Debug, Default, Deserialize)]
pub struct QwenTool {
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

impl QwenTool {
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

    fn parse(line: &str) -> QwenEvent {
        serde_json::from_str(line).expect("valid QwenEvent")
    }

    #[test]
    fn qwen_init_deserializes() {
        let event = parse(r#"{"type":"init","session_id":"qw-1","model":"qwen-coder-plus"}"#);
        let QwenEvent::Init(init) = event else {
            panic!("expected Init");
        };
        assert_eq!(init.session_id.as_deref(), Some("qw-1"));
    }

    #[test]
    fn qwen_system_session_start_detection() {
        let event = parse(
            r#"{"type":"system","subtype":"session_start","session_id":"qw-2","model":"qwen3-coder"}"#,
        );
        let QwenEvent::System(sys) = event else {
            panic!("expected System");
        };
        assert!(sys.is_session_start());
        let init = sys.into_init();
        assert_eq!(init.session_id.as_deref(), Some("qw-2"));
    }

    #[test]
    fn qwen_system_other_subtype_ignored() {
        let event = parse(r#"{"type":"system","subtype":"some_other"}"#);
        let QwenEvent::System(sys) = event else {
            panic!("expected System");
        };
        assert!(!sys.is_session_start());
    }

    #[test]
    fn qwen_message_array_content() {
        let event = parse(
            r#"{"type":"message","role":"assistant","content":[{"text":"Hello from Qwen"}]}"#,
        );
        let QwenEvent::Message(msg) = event else {
            panic!("expected Message");
        };
        assert_eq!(msg.role.as_deref(), Some("assistant"));
        assert!(msg.content.is_some());
    }

    #[test]
    fn qwen_message_string_content() {
        let event =
            parse(r#"{"type":"message","role":"assistant","content":"Plain string content"}"#);
        let QwenEvent::Message(msg) = event else {
            panic!("expected Message");
        };
        assert_eq!(
            msg.content.as_ref().and_then(Value::as_str),
            Some("Plain string content")
        );
    }

    #[test]
    fn qwen_assistant_event_type_maps_to_message() {
        let event =
            parse(r#"{"type":"assistant","content":[{"text":"Hook design assistant event"}]}"#);
        assert!(matches!(event, QwenEvent::Assistant(_)));
    }

    #[test]
    fn qwen_assistant_message_event_type_maps_to_message() {
        let event = parse(r#"{"type":"assistant_message","role":"assistant","content":"String"}"#);
        assert!(matches!(event, QwenEvent::AssistantMessage(_)));
    }

    #[test]
    fn qwen_result_with_usage() {
        let event = parse(
            r#"{"type":"result","duration_ms":5000,"usage":{"input_tokens":300,"output_tokens":150}}"#,
        );
        let QwenEvent::Result(result) = event else {
            panic!("expected Result");
        };
        let (usage, meta) = result.resolved_usage();
        assert_eq!(meta.duration_ms, Some(5000));
        let usage = usage.expect("usage");
        assert_eq!(usage.input_tokens, Some(300));
        assert_eq!(usage.output_tokens, Some(150));
    }

    #[test]
    fn qwen_summary_with_token_usage_alias() {
        let event = parse(
            r#"{"type":"summary","duration_ms":3000,"token_usage":{"input_tokens":100,"output_tokens":50}}"#,
        );
        let QwenEvent::Summary(summary) = event else {
            panic!("expected Summary");
        };
        let (usage, _) = summary.resolved_usage();
        let usage = usage.expect("usage");
        assert_eq!(usage.input_tokens, Some(100));
    }

    #[test]
    fn qwen_tool_call_with_args_alias() {
        let event =
            parse(r#"{"type":"tool_call","id":"q1","name":"bash","args":{"command":"ls"}}"#);
        let QwenEvent::ToolCall(mut tool) = event else {
            panic!("expected ToolCall");
        };
        assert_eq!(tool.resolved_tool_id(), Some("q1"));
        assert_eq!(tool.resolved_tool_name(), Some("bash"));
        let input = tool.take_input().expect("input");
        assert_eq!(input.get("command").and_then(Value::as_str), Some("ls"));
    }

    #[test]
    fn qwen_tool_call_with_params_alias() {
        let event = parse(r#"{"type":"tool_call","params":{"x":1}}"#);
        let QwenEvent::ToolCall(mut tool) = event else {
            panic!("expected ToolCall");
        };
        let input = tool.take_input().expect("input");
        assert_eq!(input.get("x").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn qwen_tool_response_with_content() {
        let event = parse(
            r#"{"type":"tool_response","tool_use_id":"q1","status":"success","content":"clean"}"#,
        );
        let QwenEvent::ToolResponse(mut tool) = event else {
            panic!("expected ToolResponse");
        };
        assert_eq!(tool.resolved_tool_id(), Some("q1"));
        assert_eq!(tool.status.as_deref(), Some("success"));
        let output = tool.take_output().expect("output");
        assert_eq!(output.as_str(), Some("clean"));
    }

    #[test]
    fn qwen_unknown_event_type_fails_typed() {
        let err = serde_json::from_str::<QwenEvent>(r#"{"type":"not_a_qwen_event"}"#);
        assert!(err.is_err());
    }
}
