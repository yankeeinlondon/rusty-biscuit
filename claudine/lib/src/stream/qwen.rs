use serde_json::Value;

use super::parser::{EventMeta, StreamChunk, StreamEventSink, StreamParseError, StreamParser};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Stream parser for Qwen CLI's `stream-json` format.
///
/// Shares Gemini-style parsing logic where event shapes match,
/// but tolerates Qwen-specific event names and result envelopes.
pub struct QwenStreamParser<S: StreamEventSink> {
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
}

impl<S: StreamEventSink> QwenStreamParser<S> {
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
        }
    }

    fn handle_init(&mut self, obj: &Value) {
        self.session_id = obj
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        self.model = obj.get("model").and_then(|v| v.as_str()).map(String::from);

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
        let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
        if role != "assistant" && event_type != "assistant" {
            return None;
        }

        // Try content array (Gemini-style)
        if let Some(content) = obj.get("content").and_then(|c| c.as_array()) {
            let mut text_parts = String::new();
            for part in content {
                if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                    text_parts.push_str(text);
                }
            }
            if !text_parts.is_empty() {
                self.assistant_text.push_str(&text_parts);
                return Some(StreamChunk::Text(super::ensure_message_newline(text_parts)));
            }
        }

        // Try content as string (Qwen-specific)
        if let Some(text) = obj.get("content").and_then(|c| c.as_str())
            && !text.is_empty()
        {
            self.assistant_text.push_str(text);
            return Some(StreamChunk::Text(super::ensure_message_newline(text.to_string())));
        }

        None
    }

    fn handle_result(&mut self, obj: &Value) {
        self.duration_ms = obj.get("duration_ms").and_then(|v| v.as_u64());
        self.num_turns = obj
            .get("num_turns")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32);
        self.provider_status = obj
            .get("stop_reason")
            .and_then(|v| v.as_str())
            .map(String::from);
        self.cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64());

        let stats = obj
            .get("stats")
            .or_else(|| obj.get("usage"))
            .or_else(|| obj.get("token_usage"));
        if let Some(stats) = stats {
            let input = stats.get("input_tokens").and_then(|v| v.as_u64());
            let output = stats.get("output_tokens").and_then(|v| v.as_u64());
            let cache_read = stats
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64());
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
        let mut meta = EventMeta::default();
        if let Some(message) = &self.error_message {
            meta.extra
                .insert("error_message".into(), Value::String(message.clone()));
        }
        if let Some(kind) = &self.error_kind {
            meta.extra
                .insert("error_kind".into(), Value::String(kind.clone()));
        }
        self.sink.on_turn_error(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for QwenStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError> {
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
        let subtype = obj.get("subtype").and_then(|t| t.as_str()).unwrap_or("");

        match event_type {
            "init" => {
                self.handle_init(&obj);
                Ok(None)
            }
            "system" if subtype == "session_start" => {
                self.handle_init(&obj);
                Ok(None)
            }
            "message" | "assistant_message" | "assistant" => Ok(self.handle_message(&obj)),
            "error" => {
                self.handle_error(&obj);
                Ok(None)
            }
            "result" | "summary" => {
                self.handle_result(&obj);
                Ok(None)
            }
            "tool_use" | "tool_call" => {
                self.tool_calls += 1;
                let mut meta = EventMeta::default();
                if let Some(tool_name) = obj
                    .get("name")
                    .or_else(|| obj.get("tool_name"))
                    .and_then(|v| v.as_str())
                {
                    meta.extra
                        .insert("tool_name".into(), Value::String(tool_name.to_string()));
                }
                self.sink.on_before_tool(&meta);
                Ok(None)
            }
            "tool_result" | "tool_response" => {
                let meta = EventMeta::default();
                self.sink.on_after_tool(&meta);
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider: Provider::QwenCode,
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

    fn make_parser() -> Box<QwenStreamParser<NullSink>> {
        Box::new(QwenStreamParser::new(NullSink))
    }

    #[test]
    fn happy_path() {
        let mut parser = make_parser();

        parser
            .feed_line(r#"{"type":"init","session_id":"qw-1","model":"qwen-coder-plus"}"#)
            .unwrap();

        let text = parser
            .feed_line(
                r#"{"type":"message","role":"assistant","content":[{"text":"Hello from Qwen"}]}"#,
            )
            .unwrap();
        assert_eq!(text, Some(StreamChunk::Text("Hello from Qwen\n".into())));

        parser
            .feed_line(r#"{"type":"result","duration_ms":5000,"usage":{"input_tokens":300,"output_tokens":150}}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::QwenCode);
        assert_eq!(summary.assistant_text, "Hello from Qwen");

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(300));
        assert_eq!(usage.output, Some(150));
        assert_eq!(usage.total, Some(450));
    }

    #[test]
    fn qwen_specific_event_names() {
        let mut parser = make_parser();

        // Qwen-specific message type
        let text = parser
            .feed_line(
                r#"{"type":"assistant_message","role":"assistant","content":"String content"}"#,
            )
            .unwrap();
        assert_eq!(text, Some(StreamChunk::Text("String content\n".into())));

        // Qwen-specific tool event names
        parser
            .feed_line(r#"{"type":"tool_call","name":"search"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_response","content":"result"}"#)
            .unwrap();

        // Qwen-specific result name
        parser
            .feed_line(
                r#"{"type":"summary","duration_ms":3000,"token_usage":{"input_tokens":100,"output_tokens":50}}"#,
            )
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(1));
        assert_eq!(summary.duration_ms, Some(3000));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(100));
    }

    #[test]
    fn qwen_hook_design_session_and_assistant_events_are_supported() {
        let mut parser = make_parser();

        parser
            .feed_line(r#"{"type":"system","subtype":"session_start","session_id":"qw-2","model":"qwen3-coder"}"#)
            .unwrap();
        let text = parser
            .feed_line(r#"{"type":"assistant","content":[{"text":"Hook design assistant event"}]}"#)
            .unwrap();

        assert_eq!(text, Some(StreamChunk::Text("Hook design assistant event\n".into())));

        let summary = parser.finish(0);
        assert_eq!(summary.session_id.as_deref(), Some("qw-2"));
        assert_eq!(summary.model.as_deref(), Some("qwen3-coder"));
    }

    #[test]
    fn content_as_string() {
        let mut parser = make_parser();
        let text = parser
            .feed_line(r#"{"type":"message","role":"assistant","content":"Plain string content"}"#)
            .unwrap();
        assert_eq!(text, Some(StreamChunk::Text("Plain string content\n".into())));
    }
}
