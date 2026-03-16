use serde_json::Value;

use super::parser::{EventMeta, StreamEventSink, StreamParseError, StreamParser};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Stream parser for Codex CLI's JSONL `exec --json` format.
///
/// Codex streams metadata/control events only. The assistant text
/// is NOT sourced from the stream — it comes from a separate
/// `--output-last-message` temp file read after child exit.
///
/// The parser tracks thread lifecycle, turn events, and token usage
/// from `turn.completed` events.
pub struct CodexStreamParser<S: StreamEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    token_usage: Option<NormalizedTokenUsage>,
    cost_usd: Option<f64>,
    duration_ms: Option<u64>,
    num_turns: u32,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
    raw_summary: Option<Value>,
    /// External assistant text set by the caller after reading the output-last-message file.
    pub assistant_text: String,
}

impl<S: StreamEventSink> CodexStreamParser<S> {
    pub fn new(sink: S) -> Self {
        Self {
            sink,
            line_num: 0,
            session_id: None,
            model: None,
            token_usage: None,
            cost_usd: None,
            duration_ms: None,
            num_turns: 0,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
            raw_summary: None,
            assistant_text: String::new(),
        }
    }

    fn handle_thread_created(&mut self, obj: &Value) {
        self.session_id = obj
            .get("thread_id")
            .or_else(|| obj.get("id"))
            .and_then(|v| v.as_str())
            .map(String::from);

        let meta = EventMeta::default();
        self.sink.on_session_start(&meta);
    }

    fn handle_turn_started(&mut self) {
        self.num_turns += 1;
        let meta = EventMeta::default();
        self.sink.on_turn_start(&meta);
    }

    fn handle_turn_completed(&mut self, obj: &Value) {
        // Extract usage from turn.completed
        if let Some(usage) = obj.get("usage") {
            let input = usage.get("input_tokens").and_then(|v| v.as_u64());
            let output = usage.get("output_tokens").and_then(|v| v.as_u64());
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => usage.get("total_tokens").and_then(|v| v.as_u64()),
            };
            let step_usage = NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read: None,
            };
            // Merge (last snapshot wins for Codex)
            match &mut self.token_usage {
                Some(existing) => existing.merge(&step_usage),
                None => self.token_usage = Some(step_usage),
            }
        }

        self.duration_ms = obj.get("duration_ms").and_then(|v| v.as_u64());
        self.cost_usd = obj.get("cost_usd").and_then(|v| v.as_f64());

        self.provider_status = obj
            .get("status")
            .or_else(|| obj.get("stop_reason"))
            .and_then(|v| v.as_str())
            .map(String::from);

        self.raw_summary = Some(obj.clone());

        let meta = EventMeta::default();
        self.sink.on_turn_complete(&meta);
    }

    fn handle_error(&mut self, obj: &Value) {
        self.is_error = true;
        self.error_kind = obj
            .get("error_type")
            .or_else(|| obj.get("error").and_then(|e| e.get("type")))
            .and_then(|t| t.as_str())
            .map(String::from);
        self.error_message = obj
            .get("error_message")
            .or_else(|| obj.get("error").and_then(|e| e.get("message")))
            .or_else(|| obj.get("message"))
            .and_then(|m| m.as_str())
            .map(String::from);

        let meta = EventMeta::default();
        self.sink.on_turn_error(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for CodexStreamParser<S> {
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
            "thread.created" => {
                self.handle_thread_created(&obj);
                Ok(None)
            }
            "turn.started" => {
                self.handle_turn_started();
                Ok(None)
            }
            "turn.completed" => {
                self.handle_turn_completed(&obj);
                Ok(None)
            }
            "error" | "turn.error" => {
                self.handle_error(&obj);
                Ok(None)
            }
            "item.tool_use" | "tool_use" => {
                self.tool_calls += 1;
                let meta = EventMeta::default();
                self.sink.on_before_tool(&meta);
                Ok(None)
            }
            "item.tool_result" | "tool_result" => {
                let meta = EventMeta::default();
                self.sink.on_after_tool(&meta);
                Ok(None)
            }
            _ => {
                // Codex stream is metadata-only; never returns text
                Ok(None)
            }
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            provider: Provider::Codex,
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
            num_turns: if self.num_turns > 0 {
                Some(self.num_turns)
            } else {
                None
            },
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

    fn make_parser() -> Box<CodexStreamParser<NullSink>> {
        Box::new(CodexStreamParser::new(NullSink))
    }

    #[test]
    fn happy_path_metadata_only() {
        let mut parser = make_parser();

        // Thread created
        let tc = r#"{"type":"thread.created","thread_id":"thrd-abc"}"#;
        assert_eq!(parser.feed_line(tc).unwrap(), None);

        // Turn started
        parser
            .feed_line(r#"{"type":"turn.started"}"#)
            .unwrap();

        // Turn completed with usage
        let tc = r#"{"type":"turn.completed","usage":{"input_tokens":200,"output_tokens":100},"duration_ms":5000,"status":"completed"}"#;
        assert_eq!(parser.feed_line(tc).unwrap(), None);

        // Set external text (simulates reading output-last-message file)
        parser.assistant_text = "Text from file".into();

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::Codex);
        assert_eq!(summary.session_id.as_deref(), Some("thrd-abc"));
        assert_eq!(summary.assistant_text, "Text from file");
        assert_eq!(summary.num_turns, Some(1));
        assert_eq!(summary.duration_ms, Some(5000));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(200));
        assert_eq!(usage.output, Some(100));
        assert_eq!(usage.total, Some(300));
    }

    #[test]
    fn stream_never_returns_text() {
        let mut parser = make_parser();
        // Even message-like events don't return text for Codex
        let result = parser
            .feed_line(r#"{"type":"item.message","content":"should not appear"}"#)
            .unwrap();
        assert_eq!(result, None);
    }

    #[test]
    fn error_handling() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"error","error_type":"rate_limit","error_message":"Too many requests"}"#)
            .unwrap();

        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("rate_limit"));
        assert_eq!(
            summary.error_message.as_deref(),
            Some("Too many requests")
        );
    }

    #[test]
    fn tool_counting() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"item.tool_use","name":"bash"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"item.tool_result","status":"ok"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"item.tool_use","name":"write"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"item.tool_result","status":"ok"}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(2));
    }
}
