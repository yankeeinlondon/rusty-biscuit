use serde_json::Value;

use super::parser::{EventMeta, StreamEventSink, StreamParseError, StreamParser};
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
        }
    }

    fn handle_init(&mut self, obj: &Value) {
        self.session_id = obj
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        self.model = obj.get("model").and_then(|v| v.as_str()).map(String::from);

        let meta = EventMeta::default();
        self.sink.on_session_start(&meta);
    }

    fn handle_message(&mut self, obj: &Value) -> Option<String> {
        let role = obj.get("role").and_then(|r| r.as_str()).unwrap_or("");
        if role != "assistant" {
            return None;
        }

        let content = obj.get("content").and_then(|c| c.as_array())?;
        let mut text_parts = String::new();
        for part in content {
            if let Some(text) = part.get("text").and_then(|t| t.as_str()) {
                text_parts.push_str(text);
            }
        }
        if text_parts.is_empty() {
            return None;
        }
        self.assistant_text.push_str(&text_parts);
        Some(text_parts)
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

        // Gemini uses stats or usage for token info
        let stats = obj.get("stats").or_else(|| obj.get("usage"));
        if let Some(stats) = stats {
            let input = stats
                .get("input_tokens")
                .or_else(|| stats.get("total_input"))
                .and_then(|v| v.as_u64());
            let output = stats
                .get("output_tokens")
                .and_then(|v| v.as_u64());
            let cache_read = stats
                .get("cache_read_input_tokens")
                .or_else(|| stats.get("cached_input"))
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
        let meta = EventMeta::default();
        self.sink.on_turn_error(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for GeminiStreamParser<S> {
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
                self.tool_calls += 1;
                let meta = EventMeta::default();
                self.sink.on_before_tool(&meta);
                Ok(None)
            }
            "tool_result" => {
                let meta = EventMeta::default();
                self.sink.on_after_tool(&meta);
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::parser::NullSink;

    fn make_parser() -> Box<GeminiStreamParser<NullSink>> {
        Box::new(GeminiStreamParser::new(NullSink))
    }

    #[test]
    fn happy_path() {
        let mut parser = make_parser();

        parser
            .feed_line(r#"{"type":"init","session_id":"gem-1","model":"gemini-2.5-pro"}"#)
            .unwrap();

        let text = parser
            .feed_line(
                r#"{"type":"message","role":"assistant","content":[{"text":"Hello from Gemini"}]}"#,
            )
            .unwrap();
        assert_eq!(text, Some("Hello from Gemini".into()));

        parser
            .feed_line(r#"{"type":"result","duration_ms":8000,"stats":{"input_tokens":500,"output_tokens":250},"cost_usd":0.002}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::Gemini);
        assert_eq!(summary.session_id.as_deref(), Some("gem-1"));
        assert_eq!(summary.assistant_text, "Hello from Gemini");

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(500));
        assert_eq!(usage.output, Some(250));
        assert_eq!(usage.total, Some(750));
    }

    #[test]
    fn non_assistant_messages_ignored() {
        let mut parser = make_parser();
        let text = parser
            .feed_line(
                r#"{"type":"message","role":"user","content":[{"text":"User message"}]}"#,
            )
            .unwrap();
        assert_eq!(text, None);
    }

    #[test]
    fn error_handling() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"error","error":{"type":"quota_exceeded","message":"Quota exceeded"}}"#)
            .unwrap();

        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("quota_exceeded"));
    }

    #[test]
    fn tool_counting() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"tool_use","name":"search","tool_id":"t1"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_result","tool_id":"t1","content":"result"}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(1));
    }
}
