//! Typed event models for Qwen CLI's `stream-json` format.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Tagged enum over all Qwen CLI stream event variants dispatched by the
/// parser. Unknown event types fail typed deserialization and are handled by
/// the parser's fallback arm.
#[derive(Debug, Deserialize, Serialize)]
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

impl QwenEvent {
    /// Returns the JSON `type` discriminator for this event variant.
    pub const fn type_str(&self) -> &'static str {
        match self {
            QwenEvent::Init(_) => "init",
            QwenEvent::System(_) => "system",
            QwenEvent::Message(_) => "message",
            QwenEvent::AssistantMessage(_) => "assistant_message",
            QwenEvent::Assistant(_) => "assistant",
            QwenEvent::Error(_) => "error",
            QwenEvent::Result(_) => "result",
            QwenEvent::Summary(_) => "summary",
            QwenEvent::ToolUse(_) => "tool_use",
            QwenEvent::ToolCall(_) => "tool_call",
            QwenEvent::ToolResult(_) => "tool_result",
            QwenEvent::ToolResponse(_) => "tool_response",
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct QwenInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// `system` events only become session-start signals when `subtype` is
/// `"session_start"` or `"init"` (the latter emitted since qwen 0.19.6 with
/// model/session metadata). Other subtypes are ignored by the parser.
#[derive(Debug, Default, Deserialize, Serialize)]
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
        matches!(self.subtype.as_deref(), Some("session_start") | Some("init"))
    }

    pub fn into_init(self) -> QwenInit {
        QwenInit {
            session_id: self.session_id,
            model: self.model,
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct QwenMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<QwenMessageContent>,
}

impl QwenMessage {
    /// Extract the text content with the parser's two-way fallback:
    /// array of `{text: ...}` parts → plain string → `None`.
    pub fn resolved_text(self) -> Option<String> {
        let content = self.content?;
        match content {
            QwenMessageContent::Parts(parts) => {
                let mut collected = String::new();
                for part in parts {
                    if let Some(text) = part.text {
                        collected.push_str(&text);
                    }
                }
                if collected.is_empty() {
                    None
                } else {
                    Some(collected)
                }
            }
            QwenMessageContent::Text(text) if !text.is_empty() => Some(text),
            QwenMessageContent::Text(_) => None,
        }
    }
}

/// Qwen's `message.content` accepts either a Gemini-style array of
/// [`QwenContentPart`] entries or a plain string. Order matters: the array
/// variant must come first so a JSON array doesn't fall through to `Text`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum QwenMessageContent {
    Parts(Vec<QwenContentPart>),
    Text(String),
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct QwenContentPart {
    #[serde(default)]
    pub text: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct QwenErrorEvent {
    #[serde(default)]
    pub error: Option<QwenErrorDetail>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct QwenErrorDetail {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Qwen's `result` / `summary` event. Usage can arrive under `stats`,
/// `usage`, or `token_usage` depending on which Qwen build emits it.
#[derive(Debug, Default, Deserialize, Serialize)]
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
    /// Dynamic fallback for unknown fields so the raw payload can be
    /// reconstructed without a second parse.
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
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

#[derive(Debug, Default, Deserialize, Serialize)]
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
#[derive(Debug, Default, Deserialize, Serialize)]
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
    fn qwen_system_init_detection() {
        // Wire sample from the signals corpus (qwen.md record
        // `stream-model_resolved-system-init`, since 0.19.6).
        const QWEN_SYSTEM_INIT: &str = include_str!(
            "../../../../docs/research/signals/fixtures/qwen/system-init-model-version.jsonl"
        );
        let event = parse(QWEN_SYSTEM_INIT.trim());
        let QwenEvent::System(sys) = event else {
            panic!("expected System");
        };
        assert!(sys.is_session_start());
        let init = sys.into_init();
        assert_eq!(init.session_id.as_deref(), Some("qw-1"));
        assert_eq!(init.model.as_deref(), Some("qwen3-coder-plus"));
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
        assert_eq!(msg.resolved_text(), Some("Hello from Qwen".into()));
    }

    #[test]
    fn qwen_message_string_content() {
        let event =
            parse(r#"{"type":"message","role":"assistant","content":"Plain string content"}"#);
        let QwenEvent::Message(msg) = event else {
            panic!("expected Message");
        };
        assert_eq!(msg.resolved_text(), Some("Plain string content".into()));
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
        assert!(
            result.extra.is_empty(),
            "known result fields must not land in extra; extra={:?}",
            result.extra
        );
        let (usage, meta) = result.resolved_usage();
        assert_eq!(meta.duration_ms, Some(5000));
        let usage = usage.expect("usage");
        assert_eq!(usage.input_tokens, Some(300));
        assert_eq!(usage.output_tokens, Some(150));
    }

    #[test]
    fn qwen_result_round_trips_through_json() {
        let line = r#"{"type":"result","duration_ms":1200,"usage":{"input_tokens":10,"output_tokens":5},"cost_usd":0.0001}"#;
        let event = parse(line);
        let serialized = serde_json::to_string(&event).unwrap();
        let reparsed: QwenEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            serde_json::to_string(&reparsed).unwrap(),
            serialized,
            "parse -> serialize -> parse should be stable for a known event"
        );
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
