//! Typed event models for Gemini CLI's `stream-json` NDJSON format.

use serde::Deserialize;
use serde_json::Value;

/// Tagged enum over all Gemini CLI stream event variants dispatched by the
/// parser. Unknown event types fail typed deserialization and are handled by
/// the parser's fallback arm.
#[derive(Debug, Deserialize)]
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

#[derive(Debug, Default, Deserialize)]
pub struct GeminiInit {
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
}

/// Assistant/user message. The parser filters on `role == "assistant"` before
/// extracting text. `content` can arrive as either a plain string or an array
/// of `{text: ...}` parts.
#[derive(Debug, Default, Deserialize)]
pub struct GeminiMessage {
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub content: Option<Value>,
}

/// `error` event. Gemini's error events carry a `severity` that determines
/// whether they surface as a warning or a turn error.
#[derive(Debug, Default, Deserialize)]
pub struct GeminiErrorEvent {
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Terminal `result` event. `status` is either `"success"` or `"error"`;
/// in the error case the nested `error` struct carries the kind/message.
#[derive(Debug, Default, Deserialize)]
pub struct GeminiResult {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub error: Option<GeminiResultError>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub stats: Option<GeminiStats>,
}

#[derive(Debug, Default, Deserialize)]
pub struct GeminiResultError {
    #[serde(rename = "type", default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
}

/// Stats block reported by `result.stats`. Gemini reports `cached` as
/// cache-read tokens and `input` as the pre-cache input count.
#[derive(Debug, Default, Deserialize)]
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
#[derive(Debug, Default, Deserialize)]
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

    pub fn resolved_input(self) -> Option<Value> {
        self.parameters.or(self.input)
    }
}

/// `tool_result` event. Response is in `output` or `result`; errors may
/// surface via the `error` field and `status` reports success/failure.
#[derive(Debug, Default, Deserialize)]
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
        assert_eq!(msg.content.as_ref().and_then(Value::as_str), Some("Hello"));
    }

    #[test]
    fn gemini_message_array_content() {
        let event = parse(
            r#"{"type":"message","role":"assistant","content":[{"text":"Hi"},{"text":" there"}]}"#,
        );
        let GeminiEvent::Message(msg) = event else {
            panic!("expected Message");
        };
        let arr = msg.content.as_ref().and_then(Value::as_array).unwrap();
        assert_eq!(arr.len(), 2);
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
        assert_eq!(stats.cached, Some(100));
        assert_eq!(stats.tool_calls, Some(0));
        assert_eq!(stats.duration_ms, Some(8000));
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
        let GeminiEvent::ToolUse(tu) = event else {
            panic!("expected ToolUse");
        };
        assert_eq!(tu.resolved_tool_name(), Some("search"));
        assert_eq!(tu.tool_id.as_deref(), Some("t1"));
        let input = tu.resolved_input().expect("input");
        assert_eq!(input.get("query").and_then(Value::as_str), Some("rust"));
    }

    #[test]
    fn gemini_tool_use_input_alias() {
        let event = parse(r#"{"type":"tool_use","name":"search","input":{"query":"fallback"}}"#);
        let GeminiEvent::ToolUse(tu) = event else {
            panic!("expected ToolUse");
        };
        assert_eq!(tu.resolved_tool_name(), Some("search"));
        let input = tu.resolved_input().expect("input");
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
