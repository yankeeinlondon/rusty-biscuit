use std::collections::HashMap;

use serde_json::Value;

use super::parser::{EventMeta, StreamChunk, StreamEventSink, StreamParseError, StreamParser};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Stream parser for Gemini CLI's `stream-json` format.
///
/// Handles `init`, assistant `message` (role=assistant), `tool_use`,
/// `tool_result`, `error`, and `result` events. Normalizes `result.stats`
/// into `NormalizedTokenUsage`.
pub struct GeminiStreamParser<S: StreamEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    assistant_text: String,
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    num_turns: Option<u32>,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    raw_summary: Option<Value>,
    tool_uses: HashMap<String, (Option<String>, Option<Value>)>,
}

impl<S: StreamEventSink> GeminiStreamParser<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            line_num: 0,
            session_id: None,
            model: None,
            assistant_text: String::new(),
            token_usage: None,
            cost_usd: None,
            duration_ms: None,
            num_turns: None,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            raw_summary: None,
            tool_uses: HashMap::new(),
        }
    }

    fn handle_init(&mut self, obj: &Value) {
        self.session_id = obj
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        self.model = obj.get("model").and_then(|v| v.as_str()).map(String::from);
        super::trace_session_metadata(
            Provider::Gemini,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );

        let mut meta = EventMeta::default();
        if let Some(session_id) = &self.session_id {
            meta.extra
                .insert("session_id".into(), Value::String(session_id.clone()));
        }
        if let Some(model) = &self.model {
            meta.extra
                .insert("model".into(), Value::String(model.clone()));
        }
        self.sink.on_session_start(&meta);
    }

    fn handle_message(&mut self, obj: &Value) -> Option<StreamChunk> {
        let role = obj.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role != "assistant" {
            return None;
        }

        let content_val = obj.get("content")?;

        // Gemini emits content as a plain string (confirmed from types.d.ts),
        // but handle array format defensively in case future versions change.
        let text = if let Some(s) = content_val.as_str() {
            s.to_string()
        } else if let Some(arr) = content_val.as_array() {
            let mut parts = String::new();
            for part in arr {
                if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                    parts.push_str(t);
                }
            }
            parts
        } else {
            return None;
        };

        if text.is_empty() {
            return None;
        }
        self.assistant_text.push_str(&text);
        Some(StreamChunk::Text(super::ensure_message_newline(text)))
    }

    fn handle_result(&mut self, obj: &Value) {
        // Result status: "success" or "error"
        self.provider_status = obj.get("status").and_then(|v| v.as_str()).map(String::from);

        // Handle result-level errors
        if obj.get("status").and_then(|v| v.as_str()) == Some("error") {
            self.is_error = true;
            if let Some(err) = obj.get("error") {
                self.error_kind = err.get("type").and_then(|v| v.as_str()).map(String::from);
                self.error_message = err
                    .get("message")
                    .and_then(|v| v.as_str())
                    .map(String::from);
            }
        }

        self.cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64());

        // Gemini stats: {total_tokens, input_tokens, output_tokens, cached, input, duration_ms, tool_calls}
        let stats = obj.get("stats");
        if let Some(stats) = stats {
            self.duration_ms = stats.get("duration_ms").and_then(|v| v.as_u64());

            // Tool calls from stats
            if let Some(tc) = stats.get("tool_calls").and_then(|v| v.as_u64()) {
                self.tool_calls = tc as u32;
            }

            let input = stats.get("input_tokens").and_then(|v| v.as_u64());
            let output = stats.get("output_tokens").and_then(|v| v.as_u64());
            // "cached" = cached prompt tokens (maps to cache_read)
            let cache_read = stats.get("cached").and_then(|v| v.as_u64());
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => stats.get("total_tokens").and_then(|v| v.as_u64()),
            };
            self.token_usage = Some(NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read,
            });
        }

        self.raw_summary = Some(obj.clone());
        super::trace_summary_update(
            Provider::Gemini,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );
    }

    fn handle_error(&mut self, obj: &Value) {
        // Real format: {"type":"error","severity":"warning","message":"Loop detected"}
        self.error_kind = obj
            .get("severity")
            .and_then(|v| v.as_str())
            .map(String::from);
        self.error_message = obj
            .get("message")
            .and_then(|v| v.as_str())
            .map(String::from);
        let mut meta = EventMeta::default();
        if let Some(message) = &self.error_message {
            meta.extra
                .insert("error_message".into(), Value::String(message.clone()));
        }
        if let Some(kind) = &self.error_kind {
            meta.extra
                .insert("error_kind".into(), Value::String(kind.clone()));
        }
        if self.error_kind.as_deref() == Some("warning") {
            if let Some(message) = &self.error_message {
                self.sink.on_warning(message);
            }
            return;
        }

        self.is_error = true;
        self.sink.on_turn_error(&meta);
    }

    fn handle_tool_use(&mut self, obj: &Value) {
        self.tool_calls += 1;
        super::trace_tool_event(
            Provider::Gemini,
            self.tool_calls,
            obj.get("tool_name")
                .or_else(|| obj.get("name"))
                .and_then(|value| value.as_str()),
        );

        let tool_id = obj
            .get("tool_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        let tool_name = obj
            .get("tool_name")
            .or_else(|| obj.get("name"))
            .and_then(|v| v.as_str())
            .map(String::from);
        let parameters = obj.get("parameters").or_else(|| obj.get("input")).cloned();

        if let Some(tool_id) = &tool_id {
            self.tool_uses
                .insert(tool_id.clone(), (tool_name.clone(), parameters.clone()));
        }

        let mut meta = EventMeta::default();
        if let Some(tool_id) = tool_id {
            meta.extra.insert("tool_id".into(), Value::String(tool_id));
        }
        if let Some(tool_name) = tool_name {
            meta.extra
                .insert("tool_name".into(), Value::String(tool_name));
        }
        if let Some(parameters) = parameters {
            meta.extra.insert("tool_input".into(), parameters);
        }
        self.sink.on_before_tool(&meta);
    }

    fn handle_tool_result(&mut self, obj: &Value) {
        let tool_id = obj
            .get("tool_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        let (tool_name, tool_input) = tool_id
            .as_ref()
            .and_then(|id| self.tool_uses.get(id).cloned())
            .unwrap_or((None, None));

        let mut meta = EventMeta::default();
        if let Some(tool_id) = tool_id {
            meta.extra.insert("tool_id".into(), Value::String(tool_id));
        }
        if let Some(tool_name) = tool_name {
            meta.extra
                .insert("tool_name".into(), Value::String(tool_name));
        }
        if let Some(tool_input) = tool_input {
            meta.extra.insert("tool_input".into(), tool_input);
        }
        if let Some(output) = obj.get("output").or_else(|| obj.get("result")) {
            meta.extra.insert("tool_response".into(), output.clone());
        }
        if let Some(error) = obj.get("error") {
            meta.extra.insert("error".into(), error.clone());
        }
        if let Some(status) = obj.get("status").and_then(|v| v.as_str()) {
            meta.extra
                .insert("status".into(), Value::String(status.to_string()));
        }
        self.sink.on_after_tool(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for GeminiStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let obj: Value = serde_json::from_str(line).map_err(|e| {
            self.sink
                .on_warning(&format!("Malformed JSON on line {}: {e}", self.line_num));
            super::trace_malformed_line(Provider::Gemini, self.line_num, &e.to_string());
            StreamParseError::MalformedLine {
                line_num: self.line_num,
                message: e.to_string(),
            }
        })?;

        let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
        super::trace_parser_event(Provider::Gemini, event_type, self.line_num);

        match event_type {
            "init" | "system" => {
                self.handle_init(&obj);
                Ok(None)
            }
            "message" => Ok(self.handle_message(&obj)),
            "error" => {
                self.handle_error(&obj);
                Ok(None)
            }
            "result" => {
                self.handle_result(&obj);
                Ok(None)
            }
            "tool_use" => {
                self.handle_tool_use(&obj);
                Ok(None)
            }
            "tool_result" => {
                self.handle_tool_result(&obj);
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider: Provider::Gemini,
            session_id: self.session_id,
            model: self.model,
            assistant_text: self.assistant_text,
            provider_status: self.provider_status,
            exit_code,
            is_error: self.is_error,
            error_kind: self.error_kind,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
            duration_api_ms: None,
            num_turns: self.num_turns,
            token_usage: self.token_usage,
            cost_usd: self.cost_usd,
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            rate_limit: None,
            context_usage: None,
            raw_summary: self.raw_summary,
            stderr_text: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::parser::NullSink;
    use crate::stream::test_support::{ToolContractExpectation, assert_tool_event_contract};

    fn make_parser() -> Box<GeminiStreamParser<NullSink>> {
        Box::new(GeminiStreamParser::new(NullSink))
    }

    #[test]
    fn happy_path_real_gemini_format() {
        let mut parser = make_parser();

        parser
            .feed_line(r#"{"type":"init","timestamp":"2026-03-16T12:00:00Z","session_id":"gem-1","model":"gemini-2.5-pro"}"#)
            .unwrap();

        // Gemini emits content as a plain string, with delta: true for streaming
        let text = parser
            .feed_line(
                r#"{"type":"message","timestamp":"2026-03-16T12:00:01Z","role":"assistant","content":"Hello from ","delta":true}"#,
            )
            .unwrap();
        assert_eq!(text, Some(StreamChunk::Text("Hello from \n".into())));

        let text2 = parser
            .feed_line(
                r#"{"type":"message","timestamp":"2026-03-16T12:00:01Z","role":"assistant","content":"Gemini","delta":true}"#,
            )
            .unwrap();
        assert_eq!(text2, Some(StreamChunk::Text("Gemini\n".into())));

        // Real result with stats object
        parser
            .feed_line(r#"{"type":"result","timestamp":"2026-03-16T12:00:02Z","status":"success","stats":{"total_tokens":750,"input_tokens":500,"output_tokens":250,"cached":100,"input":400,"duration_ms":8000,"tool_calls":0}}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::Gemini);
        assert_eq!(summary.session_id.as_deref(), Some("gem-1"));
        assert_eq!(summary.assistant_text, "Hello from Gemini");
        assert_eq!(summary.provider_status.as_deref(), Some("success"));
        assert_eq!(summary.duration_ms, Some(8000));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(500));
        assert_eq!(usage.output, Some(250));
        assert_eq!(usage.total, Some(750));
        assert_eq!(usage.cache_read, Some(100));
    }

    #[test]
    fn user_messages_ignored() {
        let mut parser = make_parser();
        let text = parser
            .feed_line(r#"{"type":"message","role":"user","content":"User message"}"#)
            .unwrap();
        assert_eq!(text, None);
    }

    #[test]
    fn error_event() {
        let mut parser = make_parser();
        // Non-fatal error event (severity: warning/error)
        parser
            .feed_line(r#"{"type":"error","timestamp":"2026-03-16T12:00:00Z","severity":"warning","message":"Loop detected"}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert!(!summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("warning"));
        assert_eq!(summary.error_message.as_deref(), Some("Loop detected"));
    }

    #[test]
    fn result_error_status() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"result","timestamp":"2026-03-16T12:00:00Z","status":"error","error":{"type":"FatalTurnLimitedError","message":"Reached max turns"}}"#)
            .unwrap();

        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("FatalTurnLimitedError"));
        assert_eq!(summary.error_message.as_deref(), Some("Reached max turns"));
    }

    #[test]
    fn tool_counting_from_events() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"tool_use","timestamp":"2026-03-16T12:00:00Z","tool_name":"search","tool_id":"t1","parameters":{}}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_result","timestamp":"2026-03-16T12:00:01Z","tool_id":"t1","status":"success","output":"found it"}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(1));
    }

    #[test]
    fn tool_result_correlates_back_to_tool_use() {
        use std::sync::Mutex;

        struct RecordingSink {
            tools: Mutex<Vec<EventMeta>>,
        }

        impl StreamEventSink for RecordingSink {
            fn on_after_tool(&mut self, meta: &EventMeta) {
                self.tools.lock().unwrap().push(meta.clone());
            }
        }

        let mut parser = Box::new(GeminiStreamParser::new(RecordingSink {
            tools: Mutex::new(Vec::new()),
        }));

        parser
            .feed_line(r#"{"type":"tool_use","tool_name":"search","tool_id":"tool-1","parameters":{"query":"rust"}}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_result","tool_id":"tool-1","status":"success","output":{"hits":3}}"#)
            .unwrap();

        let after_tool = parser.sink.tools.lock().unwrap().clone();
        let before_tool = EventMeta {
            extra: [
                ("tool_name".to_string(), Value::String("search".into())),
                ("tool_id".to_string(), Value::String("tool-1".into())),
                (
                    "tool_input".to_string(),
                    serde_json::json!({"query":"rust"}),
                ),
            ]
            .into_iter()
            .collect(),
        };
        assert_tool_event_contract(
            &before_tool,
            Some(&after_tool[0]),
            ToolContractExpectation {
                name: "search",
                id: Some("tool-1"),
                input_field: Some(("query", "rust")),
                status: Some("success"),
                response: Some(serde_json::json!({"hits": 3})),
            },
        );
    }

    #[test]
    fn tool_count_from_stats_overrides_events() {
        let mut parser = make_parser();
        // Even without tool_use events, stats.tool_calls is authoritative
        parser
            .feed_line(r#"{"type":"result","timestamp":"2026-03-16T12:00:00Z","status":"success","stats":{"total_tokens":100,"input_tokens":80,"output_tokens":20,"cached":0,"input":80,"duration_ms":1000,"tool_calls":5}}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(5));
    }

    #[test]
    fn content_as_array_fallback() {
        // Defensive: handle content as array in case future Gemini versions change
        let mut parser = make_parser();
        let text = parser
            .feed_line(
                r#"{"type":"message","role":"assistant","content":[{"text":"Array format"}]}"#,
            )
            .unwrap();
        assert_eq!(text, Some(StreamChunk::Text("Array format\n".into())));
    }
}
