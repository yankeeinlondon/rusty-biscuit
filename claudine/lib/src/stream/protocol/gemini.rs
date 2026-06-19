//! Typed event models for Gemini CLI's `stream-json` NDJSON format.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Tagged enum over all Gemini CLI stream event variants dispatched by the
/// parser. Unknown event types fail typed deserialization and are handled by
/// the parser's fallback arm.
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum GeminiEvent {
    #[serde(rename = "init")]
    Init(GeminiInit),
    #[serde(rename = "system")]
    System(GeminiInit),
    #[serde(rename = "message")]
    Message(GeminiMessage),
    #[serde(rename = "error")]
    Error(GeminiErrorEvent),
    #[serde(rename = "result")]
    Result(GeminiResult),
    #[serde(rename = "tool_use")]
    ToolUse(GeminiToolUse),
    #[serde(rename = "tool_result")]
    ToolResult(GeminiToolResult),
}

impl GeminiEvent {
    /// Returns the JSON `type` discriminator for this event variant.
    pub const fn type_str(&self) -> &'static str {
        match self {
            GeminiEvent::Init(_) => "init",
            GeminiEvent::System(_) => "system",
            GeminiEvent::Message(_) => "message",
            GeminiEvent::Error(_) => "error",
            GeminiEvent::Result(_) => "result",
            GeminiEvent::ToolUse(_) => "tool_use",
            GeminiEvent::ToolResult(_) => "tool_result",
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Assistant/user message. The parser filters on `role == "assistant"` before
/// extracting text. `content` is typed as [`GeminiMessageContent`] which
/// accepts both Gemini's real plain-string format and the defensive array
/// fallback shape. `delta` is captured from streaming-mode messages even
/// though arrival of the event itself is what drives streaming.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<GeminiMessageContent>,
    #[serde(default)]
    pub delta: Option<bool>,
}

impl GeminiMessage {
    /// Extract the text content with the parser's three-way fallback:
    /// plain string → array of `{text: ...}` parts → `None`.
    pub fn resolved_text(self) -> Option<String> {
        let content = self.content?;
        match content {
            GeminiMessageContent::Text(text) if !text.is_empty() => Some(text),
            GeminiMessageContent::Text(_) => None,
            GeminiMessageContent::Parts(parts) => {
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
        }
    }
}

/// Gemini's `message.content` arrives as a plain string in the canonical
/// format, but historical and defensive paths emit an array of
/// [`GeminiContentPart`] entries. This untagged enum accepts both shapes so
/// the parser doesn't have to walk a `serde_json::Value`.
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum GeminiMessageContent {
    Text(String),
    Parts(Vec<GeminiContentPart>),
}

/// Single entry in a `message.content` array. Only `text` is consumed today;
/// other fields are tolerated but ignored.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiContentPart {
    #[serde(default)]
    pub text: Option<String>,
}

/// `error` event. Gemini's error events carry a `severity` that determines
/// whether they surface as a warning or a turn error.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiErrorEvent {
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Terminal `result` event. `status` is either `"success"` or `"error"`;
/// in the error case the nested `error` struct carries the kind/message.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiResult {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error: Option<GeminiResultError>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub stats: Option<GeminiStats>,
    /// Dynamic fallback for unknown fields so the raw payload can be
    /// reconstructed without a second parse.
    #[serde(flatten, default)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiResultError {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Stats block reported by `result.stats`. Gemini reports `cached` as
/// cache-read tokens and `input` as the pre-cache input count.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiStats {
    #[serde(default)]
    pub total_tokens: Option<u64>,
    #[serde(default)]
    pub input_tokens: Option<u64>,
    #[serde(default)]
    pub output_tokens: Option<u64>,
    #[serde(default)]
    pub cached: Option<u64>,
    #[serde(default)]
    pub input: Option<u64>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub tool_calls: Option<u64>,
}

/// `tool_use` event. Gemini places input under either `parameters` or
/// `input`, with `tool_name` and `name` as aliases for the tool name.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiToolUse {
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub parameters: Option<Value>,
    #[serde(default)]
    pub input: Option<Value>,
}

impl GeminiToolUse {
    pub fn resolved_tool_name(&self) -> Option<&str> {
        self.tool_name.as_deref().or(self.name.as_deref())
    }

    pub fn resolved_tool_id(&self) -> Option<&str> {
        self.tool_id.as_deref()
    }

    pub fn take_input(&mut self) -> Option<Value> {
        self.parameters.take().or_else(|| self.input.take())
    }
}

/// `tool_result` event. Response is in `output` or `result`; errors may
/// surface via the `error` field and `status` reports success/failure.
#[derive(Debug, Default, Deserialize, Serialize)]
pub struct GeminiToolResult {
    #[serde(default)]
    pub tool_id: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub output: Option<Value>,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<Value>,
}

impl GeminiToolResult {
    pub fn response(self) -> (Option<Value>, Option<Value>, Option<String>) {
        (self.output.or(self.result), self.error, self.status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(line: &str) -> GeminiEvent {
        serde_json::from_str(line).expect("valid GeminiEvent")
    }

    #[test]
    fn gemini_init_deserializes() {
        let event = parse(r#"{"type":"init","session_id":"gem-1","model":"gemini-2.5-pro"}"#);
        let GeminiEvent::Init(init) = event else {
            panic!("expected Init");
        };
        assert_eq!(init.session_id.as_deref(), Some("gem-1"));
        assert_eq!(init.model.as_deref(), Some("gemini-2.5-pro"));
    }

    #[test]
    fn gemini_message_string_content() {
        let event = parse(r#"{"type":"message","role":"assistant","content":"Hello"}"#);
        let GeminiEvent::Message(msg) = event else {
            panic!("expected Message");
        };
        assert_eq!(msg.role.as_deref(), Some("assistant"));
        assert_eq!(msg.resolved_text(), Some("Hello".into()));
    }

    #[test]
    fn gemini_message_array_content() {
        let event = parse(
            r#"{"type":"message","role":"assistant","content":[{"text":"Hi"},{"text":" there"}]}"#,
        );
        let GeminiEvent::Message(msg) = event else {
            panic!("expected Message");
        };
        assert_eq!(msg.resolved_text(), Some("Hi there".into()));
    }

    #[test]
    fn gemini_message_delta_field_captured() {
        let event =
            parse(r#"{"type":"message","role":"assistant","content":"chunk","delta":true}"#);
        let GeminiEvent::Message(msg) = event else {
            panic!("expected Message");
        };
        assert_eq!(msg.delta, Some(true));
        assert_eq!(msg.resolved_text(), Some("chunk".into()));
    }

    #[test]
    fn gemini_user_message_preserved_for_filtering() {
        let event = parse(r#"{"type":"message","role":"user","content":"hey"}"#);
        let GeminiEvent::Message(msg) = event else {
            panic!("expected Message");
        };
        assert_eq!(msg.role.as_deref(), Some("user"));
    }

    #[test]
    fn gemini_error_warning_severity() {
        let event = parse(r#"{"type":"error","severity":"warning","message":"Loop detected"}"#);
        let GeminiEvent::Error(err) = event else {
            panic!("expected Error");
        };
        assert_eq!(err.severity.as_deref(), Some("warning"));
        assert_eq!(err.message.as_deref(), Some("Loop detected"));
    }

    #[test]
    fn gemini_result_success_with_stats() {
        let event = parse(
            r#"{"type":"result","status":"success","stats":{"total_tokens":750,"input_tokens":500,"output_tokens":250,"cached":100,"input":400,"duration_ms":8000,"tool_calls":0}}"#,
        );
        let GeminiEvent::Result(result) = event else {
            panic!("expected Result");
        };
        assert_eq!(result.status.as_deref(), Some("success"));
        let stats = result.stats.expect("stats");
        assert_eq!(stats.total_tokens, Some(750));
        assert_eq!(stats.input_tokens, Some(500));
        assert_eq!(stats.output_tokens, Some(250));
        assert_eq!(stats.cached, Some(100));
        assert_eq!(stats.input, Some(400));
        assert_eq!(stats.duration_ms, Some(8000));
        assert_eq!(stats.tool_calls, Some(0));
        assert!(
            result.extra.is_empty(),
            "known result fields must not land in extra; extra={:?}",
            result.extra
        );
    }

    #[test]
    fn gemini_result_round_trips_through_json() {
        let line = r#"{"type":"result","status":"success","stats":{"total_tokens":10,"input_tokens":6,"output_tokens":4},"cost_usd":0.001}"#;
        let event = parse(line);
        let serialized = serde_json::to_string(&event).unwrap();
        let reparsed: GeminiEvent = serde_json::from_str(&serialized).unwrap();
        assert_eq!(
            serde_json::to_string(&reparsed).unwrap(),
            serialized,
            "parse -> serialize -> parse should be stable for a known event"
        );
    }

    #[test]
    fn gemini_result_error_status() {
        let event = parse(
            r#"{"type":"result","status":"error","error":{"type":"FatalTurnLimitedError","message":"Reached max turns"}}"#,
        );
        let GeminiEvent::Result(result) = event else {
            panic!("expected Result");
        };
        assert_eq!(result.status.as_deref(), Some("error"));
        let detail = result.error.expect("error");
        assert_eq!(detail.kind.as_deref(), Some("FatalTurnLimitedError"));
        assert_eq!(detail.message.as_deref(), Some("Reached max turns"));
    }

    #[test]
    fn gemini_tool_use_parameters() {
        let event = parse(
            r#"{"type":"tool_use","tool_name":"search","tool_id":"t1","parameters":{"query":"rust"}}"#,
        );
        let GeminiEvent::ToolUse(mut tu) = event else {
            panic!("expected ToolUse");
        };
        assert_eq!(tu.resolved_tool_name(), Some("search"));
        assert_eq!(tu.resolved_tool_id(), Some("t1"));
        let input = tu.take_input().expect("input");
        assert_eq!(input.get("query").and_then(Value::as_str), Some("rust"));
    }

    #[test]
    fn gemini_tool_use_input_alias() {
        let event = parse(r#"{"type":"tool_use","name":"search","input":{"query":"fallback"}}"#);
        let GeminiEvent::ToolUse(mut tu) = event else {
            panic!("expected ToolUse");
        };
        assert_eq!(tu.resolved_tool_name(), Some("search"));
        let input = tu.take_input().expect("input");
        assert_eq!(input.get("query").and_then(Value::as_str), Some("fallback"));
    }

    #[test]
    fn gemini_tool_result_output_and_status() {
        let event = parse(
            r#"{"type":"tool_result","tool_id":"t1","status":"success","output":{"hits":3}}"#,
        );
        let GeminiEvent::ToolResult(tr) = event else {
            panic!("expected ToolResult");
        };
        let (response, error, status) = tr.response();
        assert!(error.is_none());
        assert_eq!(status.as_deref(), Some("success"));
        let response = response.expect("response");
        assert_eq!(response.get("hits").and_then(Value::as_u64), Some(3));
    }

    #[test]
    fn gemini_unknown_event_type_fails_typed() {
        let err = serde_json::from_str::<GeminiEvent>(r#"{"type":"session.not_a_real_event"}"#);
        assert!(err.is_err());
    }
}
