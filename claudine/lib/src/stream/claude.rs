use serde_json::Value;

use super::parser::{EventMeta, StreamEventSink, StreamParseError, StreamParser};
use super::summary::{RateLimitInfo, StreamExecutionSummary};
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Stream parser for Claude Code's `stream-json` format.
///
/// Handles the following event types from Claude's structured output:
/// - `init` - session metadata (session_id, model, auth, version)
/// - `assistant` messages with text content
/// - `assistant.error` - structured error reports
/// - `result` - final session summary with usage/cost/duration
/// - `rate_limit_event` - throttling notifications
/// - `tool_use` / `tool_result` - tool activity tracking
pub struct ClaudeStreamParser<S: StreamEventSink> {
    sink: S,
    line_num: usize,
    // Accumulated from init
    session_id: Option<String>,
    model: Option<String>,
    // Accumulated from assistant message content
    assistant_text: String,
    // Accumulated from result
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    duration_api_ms: Option<u64>,
    num_turns: Option<u32>,
    tool_calls: u32,
    provider_status: Option<String>,
    // Error state
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    // Rate limit
    rate_limit: Option<RateLimitInfo>,
    // Raw result for provider-specific extra
    raw_summary: Option<Value>,
}

impl<S: StreamEventSink> ClaudeStreamParser<S> {
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
            duration_api_ms: None,
            num_turns: None,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            rate_limit: None,
            raw_summary: None,
        }
    }

    fn handle_init(&mut self, obj: &Value) {
        self.session_id = obj.get("session_id").and_then(|v| v.as_str()).map(String::from);
        self.model = obj.get("model").and_then(|v| v.as_str()).map(String::from);

        let mut meta = EventMeta::default();
        if let Some(sid) = &self.session_id {
            meta.extra
                .insert("session_id".into(), Value::String(sid.clone()));
        }
        if let Some(model) = &self.model {
            meta.extra
                .insert("model".into(), Value::String(model.clone()));
        }
        self.sink.on_session_start(&meta);
    }

    fn handle_assistant_message(&mut self, obj: &Value) -> Option<String> {
        // Extract text from content array.
        // Claude Code wraps it as {"message":{"content":[...]}} while the
        // simplified test format uses {"content":[...]} at the top level.
        let content = obj
            .get("message")
            .and_then(|m| m.get("content"))
            .or_else(|| obj.get("content"))
            .and_then(|c| c.as_array())?;
        let mut text_parts = String::new();
        for part in content {
            if part.get("type").and_then(|t| t.as_str()) == Some("text")
                && let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    text_parts.push_str(text);
                }
        }
        if text_parts.is_empty() {
            return None;
        }
        self.assistant_text.push_str(&text_parts);
        Some(text_parts)
    }

    fn handle_content_block_delta(&mut self, obj: &Value) -> Option<String> {
        let delta = obj.get("delta")?;
        if delta.get("type").and_then(|t| t.as_str()) == Some("text_delta") {
            let text = delta.get("text").and_then(|t| t.as_str())?;
            self.assistant_text.push_str(text);
            return Some(text.to_string());
        }
        None
    }

    fn handle_error(&mut self, obj: &Value) {
        self.is_error = true;
        self.error_kind = obj
            .get("error")
            .and_then(|e| e.get("type"))
            .and_then(|t| t.as_str())
            .map(String::from);
        self.error_message = obj
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .map(String::from);

        let meta = EventMeta::default();
        self.sink.on_turn_error(&meta);
    }

    fn handle_result(&mut self, obj: &Value) {
        // Duration
        self.duration_ms = obj
            .get("duration_ms")
            .and_then(|v| v.as_u64());
        self.duration_api_ms = obj
            .get("duration_api_ms")
            .and_then(|v| v.as_u64());

        // Turns
        self.num_turns = obj
            .get("num_turns")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);

        // Stop reason / status
        self.provider_status = obj
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(String::from);

        // Cost (Claude Code uses "total_cost_usd", older versions may use "cost_usd")
        self.cost_usd = obj
            .get("total_cost_usd")
            .or_else(|| obj.get("cost_usd"))
            .and_then(|v| v.as_f64());

        // Token usage
        if let Some(usage) = obj.get("usage") {
            let input = usage.get("input_tokens").and_then(|v| v.as_u64());
            let output = usage.get("output_tokens").and_then(|v| v.as_u64());
            let cache_read = usage
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64());
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => None,
            };
            self.token_usage = Some(NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read,
            });
        }

        // Store compact raw summary (exclude large arrays)
        let mut raw = obj.clone();
        if let Some(map) = raw.as_object_mut() {
            map.remove("tools");
            map.remove("skills");
            map.remove("agents");
            map.remove("mcp_servers");
        }
        self.raw_summary = Some(raw);
    }

    fn handle_rate_limit(&mut self, obj: &Value) {
        let info = RateLimitInfo {
            is_throttled: obj.get("is_throttled").and_then(|v| v.as_bool()),
            retry_after_ms: obj.get("retry_after_ms").and_then(|v| v.as_u64()),
            message: obj.get("message").and_then(|v| v.as_str()).map(String::from),
        };
        if let Some(msg) = &info.message {
            self.sink.on_warning(msg);
        }
        self.rate_limit = Some(info);
    }

    fn handle_tool_use(&mut self) {
        self.tool_calls += 1;
        let meta = EventMeta::default();
        self.sink.on_before_tool(&meta);
    }

    fn handle_tool_result(&mut self) {
        let meta = EventMeta::default();
        self.sink.on_after_tool(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for ClaudeStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<Option<String>, StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let obj: Value = serde_json::from_str(line).map_err(|e| {
            self.sink
                .on_warning(&format!("Malformed JSON on line {}: {e}", self.line_num));
            StreamParseError::MalformedLine {
                line_num: self.line_num,
                message: e.to_string(),
            }
        })?;

        let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match event_type {
            "init" | "system" => {
                self.handle_init(&obj);
                Ok(None)
            }
            "assistant" => {
                // Full assistant message with content array
                let text = self.handle_assistant_message(&obj);
                Ok(text)
            }
            "content_block_delta" => {
                let text = self.handle_content_block_delta(&obj);
                Ok(text)
            }
            "error" | "assistant.error" => {
                self.handle_error(&obj);
                Ok(None)
            }
            "result" => {
                self.handle_result(&obj);
                Ok(None)
            }
            "rate_limit_event" => {
                self.handle_rate_limit(&obj);
                Ok(None)
            }
            "tool_use" | "content_block_start"
                if obj
                    .get("content_block")
                    .and_then(|b| b.get("type"))
                    .and_then(|t| t.as_str())
                    == Some("tool_use") =>
            {
                self.handle_tool_use();
                Ok(None)
            }
            "tool_use" => {
                self.handle_tool_use();
                Ok(None)
            }
            "tool_result" => {
                self.handle_tool_result();
                Ok(None)
            }
            _ => {
                // Unknown event types are silently skipped
                Ok(None)
            }
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider: Provider::Claude,
            session_id: self.session_id,
            model: self.model,
            assistant_text: self.assistant_text,
            provider_status: self.provider_status,
            exit_code,
            is_error: self.is_error,
            error_kind: self.error_kind,
            error_message: self.error_message,
            duration_ms: self.duration_ms,
            duration_api_ms: self.duration_api_ms,
            num_turns: self.num_turns,
            token_usage: self.token_usage,
            cost_usd: self.cost_usd,
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            rate_limit: self.rate_limit,
            context_usage: None,
            raw_summary: self.raw_summary,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::stream::parser::NullSink;

    /// A recording sink that captures event calls for verification.
    struct RecordingSink {
        events: Mutex<Vec<String>>,
        warnings: Mutex<Vec<String>>,
    }

    impl RecordingSink {
        fn new() -> Self {
            Self {
                events: Mutex::new(Vec::new()),
                warnings: Mutex::new(Vec::new()),
            }
        }
        fn event_names(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
        fn warning_messages(&self) -> Vec<String> {
            self.warnings.lock().unwrap().clone()
        }
    }

    impl StreamEventSink for RecordingSink {
        fn on_session_start(&mut self, _meta: &EventMeta) {
            self.events.lock().unwrap().push("session_start".into());
        }
        fn on_turn_error(&mut self, _meta: &EventMeta) {
            self.events.lock().unwrap().push("turn_error".into());
        }
        fn on_before_tool(&mut self, _meta: &EventMeta) {
            self.events.lock().unwrap().push("before_tool".into());
        }
        fn on_after_tool(&mut self, _meta: &EventMeta) {
            self.events.lock().unwrap().push("after_tool".into());
        }
        fn on_warning(&mut self, message: &str) {
            self.warnings.lock().unwrap().push(message.into());
        }
    }

    fn make_parser() -> Box<ClaudeStreamParser<NullSink>> {
        Box::new(ClaudeStreamParser::new(NullSink))
    }

    fn make_recording_parser() -> Box<ClaudeStreamParser<RecordingSink>> {
        Box::new(ClaudeStreamParser::new(RecordingSink::new()))
    }

    #[test]
    fn happy_path_init_assistant_result() {
        let mut parser = make_parser();

        // Init
        let init = r#"{"type":"init","session_id":"sess-abc","model":"claude-sonnet-4-20250514"}"#;
        assert_eq!(parser.feed_line(init).unwrap(), None);

        // Assistant message
        let msg = r#"{"type":"assistant","content":[{"type":"text","text":"Hello, world!"}]}"#;
        assert_eq!(
            parser.feed_line(msg).unwrap(),
            Some("Hello, world!".into())
        );

        // Result
        let result = r#"{"type":"result","duration_ms":12345,"duration_api_ms":11000,"num_turns":1,"stop_reason":"end_turn","cost_usd":0.0042,"usage":{"input_tokens":1000,"output_tokens":500,"cache_read_input_tokens":200}}"#;
        assert_eq!(parser.feed_line(result).unwrap(), None);

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::Claude);
        assert_eq!(summary.session_id.as_deref(), Some("sess-abc"));
        assert_eq!(summary.model.as_deref(), Some("claude-sonnet-4-20250514"));
        assert_eq!(summary.assistant_text, "Hello, world!");
        assert_eq!(summary.exit_code, 0);
        assert!(!summary.is_error);
        assert_eq!(summary.duration_ms, Some(12345));
        assert_eq!(summary.duration_api_ms, Some(11000));
        assert_eq!(summary.num_turns, Some(1));
        assert_eq!(summary.cost_usd, Some(0.0042));
        assert_eq!(summary.provider_status.as_deref(), Some("end_turn"));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(1000));
        assert_eq!(usage.output, Some(500));
        assert_eq!(usage.total, Some(1500));
        assert_eq!(usage.cache_read, Some(200));
    }

    #[test]
    fn error_path_assistant_error() {
        let mut parser = make_parser();

        let init = r#"{"type":"init","session_id":"sess-err","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();

        let error = r#"{"type":"error","error":{"type":"billing_error","message":"Insufficient credits"}}"#;
        parser.feed_line(error).unwrap();

        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("billing_error"));
        assert_eq!(
            summary.error_message.as_deref(),
            Some("Insufficient credits")
        );
        assert_eq!(summary.exit_code, 1);
    }

    #[test]
    fn rate_limit_event() {
        let mut parser = make_recording_parser();

        let init = r#"{"type":"init","session_id":"sess-rl","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();

        let rl = r#"{"type":"rate_limit_event","is_throttled":true,"retry_after_ms":5000,"message":"Rate limit exceeded"}"#;
        parser.feed_line(rl).unwrap();

        let result = r#"{"type":"result","duration_ms":5000,"usage":{"input_tokens":100,"output_tokens":50}}"#;
        parser.feed_line(result).unwrap();

        assert_eq!(
            parser.sink.warning_messages(),
            vec!["Rate limit exceeded".to_string()]
        );

        let summary = parser.finish(0);
        let rl_info = summary.rate_limit.unwrap();
        assert_eq!(rl_info.is_throttled, Some(true));
        assert_eq!(rl_info.retry_after_ms, Some(5000));
        assert_eq!(rl_info.message.as_deref(), Some("Rate limit exceeded"));
    }

    #[test]
    fn malformed_line_recovery() {
        let mut parser = make_parser();

        let init = r#"{"type":"init","session_id":"sess-bad","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();

        // Malformed line - should error but not fatal
        let result = parser.feed_line("this is not json {{{");
        assert!(matches!(
            result,
            Err(StreamParseError::MalformedLine { .. })
        ));

        // Normal line after malformed - should still work
        let msg = r#"{"type":"assistant","content":[{"type":"text","text":"After recovery"}]}"#;
        assert_eq!(
            parser.feed_line(msg).unwrap(),
            Some("After recovery".into())
        );

        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "After recovery");
    }

    #[test]
    fn multi_turn_concatenation() {
        let mut parser = make_parser();

        let init = r#"{"type":"init","session_id":"sess-mt","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();

        let msg1 = r#"{"type":"assistant","content":[{"type":"text","text":"First turn. "}]}"#;
        parser.feed_line(msg1).unwrap();

        let msg2 = r#"{"type":"assistant","content":[{"type":"text","text":"Second turn."}]}"#;
        parser.feed_line(msg2).unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "First turn. Second turn.");
    }

    #[test]
    fn tool_use_events_counted_and_dispatched() {
        let mut parser = make_recording_parser();

        let init = r#"{"type":"init","session_id":"sess-tu","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();

        let tool = r#"{"type":"tool_use","name":"read_file","input":{"path":"test.rs"}}"#;
        parser.feed_line(tool).unwrap();

        let tool_result = r#"{"type":"tool_result","tool_use_id":"tu-1","content":"file contents"}"#;
        parser.feed_line(tool_result).unwrap();

        let tool2 = r#"{"type":"tool_use","name":"edit_file","input":{"path":"test.rs"}}"#;
        parser.feed_line(tool2).unwrap();

        let tool_result2 = r#"{"type":"tool_result","tool_use_id":"tu-2","content":"ok"}"#;
        parser.feed_line(tool_result2).unwrap();

        let events = parser.sink.event_names();
        assert_eq!(
            events,
            vec![
                "session_start",
                "before_tool",
                "after_tool",
                "before_tool",
                "after_tool"
            ]
        );

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(2));
    }

    #[test]
    fn large_init_arrays_not_stored_in_summary() {
        let mut parser = make_parser();

        // Init with large tools array
        let init = r#"{"type":"init","session_id":"sess-lg","model":"claude-sonnet-4-20250514","tools":[{"name":"a"},{"name":"b"},{"name":"c"}]}"#;
        parser.feed_line(init).unwrap();

        let result = r#"{"type":"result","duration_ms":100,"tools":["a","b","c"],"skills":["x"],"agents":["y"],"mcp_servers":["z"],"usage":{"input_tokens":10,"output_tokens":5}}"#;
        parser.feed_line(result).unwrap();

        let summary = parser.finish(0);
        let raw = summary.raw_summary.unwrap();
        assert!(raw.get("tools").is_none());
        assert!(raw.get("skills").is_none());
        assert!(raw.get("agents").is_none());
        assert!(raw.get("mcp_servers").is_none());
        // But other fields are preserved
        assert!(raw.get("duration_ms").is_some());
    }

    #[test]
    fn content_block_delta() {
        let mut parser = make_parser();

        let init = r#"{"type":"init","session_id":"sess-delta","model":"claude-sonnet-4-20250514"}"#;
        parser.feed_line(init).unwrap();

        let delta1 = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":"Hello"}}"#;
        assert_eq!(parser.feed_line(delta1).unwrap(), Some("Hello".into()));

        let delta2 = r#"{"type":"content_block_delta","delta":{"type":"text_delta","text":" world"}}"#;
        assert_eq!(parser.feed_line(delta2).unwrap(), Some(" world".into()));

        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "Hello world");
    }

    #[test]
    fn empty_lines_skipped() {
        let mut parser = make_parser();
        assert_eq!(parser.feed_line("").unwrap(), None);
        assert_eq!(parser.feed_line("  ").unwrap(), None);
        assert_eq!(parser.feed_line("\t").unwrap(), None);
    }

    #[test]
    fn unknown_event_types_skipped() {
        let mut parser = make_parser();
        let unknown = r#"{"type":"reasoning_delta","text":"thinking..."}"#;
        assert_eq!(parser.feed_line(unknown).unwrap(), None);
    }

    #[test]
    fn finish_with_nonzero_exit_code() {
        let parser = make_parser();
        let summary = parser.finish(42);
        assert_eq!(summary.exit_code, 42);
    }

    #[test]
    fn tool_calls_none_when_zero() {
        let parser = make_parser();
        let summary = parser.finish(0);
        assert!(summary.tool_calls.is_none());
    }

    #[test]
    fn total_cost_usd_field_name() {
        let mut parser = make_parser();

        let init = r#"{"type":"init","session_id":"sess-cost","model":"claude-opus-4-6"}"#;
        parser.feed_line(init).unwrap();

        // Claude Code uses "total_cost_usd" in the result event
        let result = r#"{"type":"result","duration_ms":5396,"total_cost_usd":0.185,"usage":{"input_tokens":3,"output_tokens":4}}"#;
        parser.feed_line(result).unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.cost_usd, Some(0.185));
        assert_eq!(summary.duration_ms, Some(5396));
        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(3));
        assert_eq!(usage.output, Some(4));
    }

    #[test]
    fn assistant_message_nested_under_message_key() {
        let mut parser = make_parser();

        let init = r#"{"type":"system","subtype":"init","session_id":"sess-nested","model":"claude-opus-4-6"}"#;
        parser.feed_line(init).unwrap();

        // Real Claude Code format: content is nested under "message"
        let msg = r#"{"type":"assistant","message":{"content":[{"type":"text","text":"The sky is blue because of Rayleigh scattering."}],"role":"assistant"}}"#;
        let result = parser.feed_line(msg).unwrap();
        assert_eq!(
            result.as_deref(),
            Some("The sky is blue because of Rayleigh scattering.")
        );

        let summary = parser.finish(0);
        assert_eq!(
            summary.assistant_text,
            "The sky is blue because of Rayleigh scattering."
        );
    }
}
