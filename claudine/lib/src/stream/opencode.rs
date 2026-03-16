use serde_json::Value;

use super::parser::{EventMeta, StreamEventSink, StreamParseError, StreamParser};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Stream parser for OpenCode CLI's NDJSON `json` output.
///
/// Accumulates text fragments from text events and per-step usage/cost
/// across the run. Model identity is sourced externally (not from stream)
/// and accepted as a constructor parameter.
pub struct OpenCodeStreamParser<S: StreamEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    assistant_text: String,
    token_usage: NormalizedTokenUsage,
    cost_usd: f64,
    duration_ms: Option<u64>,
    num_turns: u32,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
}

impl<S: StreamEventSink> OpenCodeStreamParser<S> {
    /// Create a new OpenCode parser.
    ///
    /// `model` is sourced externally (wrapper config/env) since the
    /// stream itself does not always report it.
    pub fn new(sink: S, model: Option<String>) -> Self {
        Self {
            sink,
            line_num: 0,
            session_id: None,
            model,
            assistant_text: String::new(),
            token_usage: NormalizedTokenUsage::default(),
            cost_usd: 0.0,
            duration_ms: None,
            num_turns: 0,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
        }
    }

    fn handle_init(&mut self, obj: &Value) {
        self.session_id = obj
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        // Override model if stream provides it
        if let Some(model) = obj.get("model").and_then(|v| v.as_str()) {
            self.model = Some(model.to_string());
        }

        let meta = EventMeta::default();
        self.sink.on_session_start(&meta);
    }

    fn handle_text(&mut self, obj: &Value) -> Option<String> {
        let text = obj
            .get("text")
            .or_else(|| obj.get("content"))
            .and_then(|t| t.as_str())?;
        if text.is_empty() {
            return None;
        }
        self.assistant_text.push_str(text);
        Some(text.to_string())
    }

    fn handle_step_complete(&mut self, obj: &Value) {
        self.num_turns += 1;

        // Accumulate per-step usage
        if let Some(usage) = obj.get("usage") {
            let step = NormalizedTokenUsage {
                input: usage.get("input_tokens").and_then(|v| v.as_u64()),
                output: usage.get("output_tokens").and_then(|v| v.as_u64()),
                total: usage.get("total_tokens").and_then(|v| v.as_u64()),
                cache_read: None,
            };
            self.token_usage.accumulate(&step);
        }

        // Accumulate cost
        if let Some(cost) = obj.get("cost_usd").and_then(|v| v.as_f64()) {
            self.cost_usd += cost;
        }

        self.duration_ms = obj
            .get("duration_ms")
            .and_then(|v| v.as_u64())
            .or(self.duration_ms);

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

        self.sink.on_warning(
            self.error_message
                .as_deref()
                .unwrap_or("Step failure"),
        );

        let meta = EventMeta::default();
        self.sink.on_turn_error(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for OpenCodeStreamParser<S> {
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
            "init" | "session_start" => {
                self.handle_init(&obj);
                Ok(None)
            }
            "text" | "text_delta" | "assistant_text" => Ok(self.handle_text(&obj)),
            "step_complete" | "turn_complete" => {
                self.handle_step_complete(&obj);
                Ok(None)
            }
            "error" | "step_error" => {
                self.handle_error(&obj);
                Ok(None)
            }
            "tool_use" | "tool_start" => {
                self.tool_calls += 1;
                let meta = EventMeta::default();
                self.sink.on_before_tool(&meta);
                Ok(None)
            }
            "tool_result" | "tool_end" => {
                let meta = EventMeta::default();
                self.sink.on_after_tool(&meta);
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        let has_usage = self.token_usage.input.is_some() || self.token_usage.output.is_some();
        StreamExecutionSummary {
            provider: Provider::OpenCode,
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
            token_usage: if has_usage {
                Some(self.token_usage)
            } else {
                None
            },
            cost_usd: if self.cost_usd > 0.0 {
                Some(self.cost_usd)
            } else {
                None
            },
            tool_calls: if self.tool_calls > 0 {
                Some(self.tool_calls)
            } else {
                None
            },
            rate_limit: None,
            context_usage: None,
            raw_summary: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::parser::NullSink;

    fn make_parser() -> Box<OpenCodeStreamParser<NullSink>> {
        Box::new(OpenCodeStreamParser::new(
            NullSink,
            Some("gpt-4o".into()),
        ))
    }

    #[test]
    fn accumulates_usage_across_steps() {
        let mut parser = make_parser();

        parser
            .feed_line(r#"{"type":"text","text":"Hello "}"#)
            .unwrap();

        parser
            .feed_line(
                r#"{"type":"step_complete","usage":{"input_tokens":100,"output_tokens":50},"cost_usd":0.001}"#,
            )
            .unwrap();

        parser
            .feed_line(r#"{"type":"text","text":"world"}"#)
            .unwrap();

        parser
            .feed_line(
                r#"{"type":"step_complete","usage":{"input_tokens":150,"output_tokens":75},"cost_usd":0.002}"#,
            )
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::OpenCode);
        assert_eq!(summary.model.as_deref(), Some("gpt-4o"));
        assert_eq!(summary.assistant_text, "Hello world");
        assert_eq!(summary.num_turns, Some(2));
        assert_eq!(summary.cost_usd, Some(0.003));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(250)); // 100 + 150
        assert_eq!(usage.output, Some(125)); // 50 + 75
    }

    #[test]
    fn model_from_constructor() {
        let parser = make_parser();
        let summary = parser.finish(0);
        assert_eq!(summary.model.as_deref(), Some("gpt-4o"));
    }

    #[test]
    fn model_overridden_by_stream() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"init","model":"gpt-4-turbo"}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.model.as_deref(), Some("gpt-4-turbo"));
    }

    #[test]
    fn no_usage_when_no_steps() {
        let parser = make_parser();
        let summary = parser.finish(0);
        assert!(summary.token_usage.is_none());
        assert!(summary.cost_usd.is_none());
    }

    #[test]
    fn step_failure_warning() {
        let mut parser = make_parser();
        parser
            .feed_line(
                r#"{"type":"error","error_message":"API timeout"}"#,
            )
            .unwrap();

        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_message.as_deref(), Some("API timeout"));
    }
}
