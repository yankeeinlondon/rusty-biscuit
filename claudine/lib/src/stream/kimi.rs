use serde_json::Value;

use super::parser::{EventMeta, StreamChunk, StreamEventSink, StreamParseError, StreamParser};
use super::summary::{ContextUsage, StreamExecutionSummary};
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

/// Context pressure warning threshold (percentage).
const CONTEXT_PRESSURE_WARN_PERCENT: f64 = 80.0;

/// Stream parser for Kimi Code's `stream-json` format.
///
/// Kimi has no aggregate final result event. The summary comes from
/// the latest `StatusUpdate` snapshot plus the child exit code.
/// Context pressure warnings are surfaced when usage exceeds threshold.
pub struct KimiStreamParser<S: StreamEventSink> {
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
    context_usage: Option<ContextUsage>,
}

impl<S: StreamEventSink> KimiStreamParser<S> {
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
            context_usage: None,
        }
    }

    fn handle_init(&mut self, obj: &Value) {
        self.session_id = obj
            .get("session_id")
            .and_then(|v| v.as_str())
            .map(String::from);
        self.model = obj.get("model").and_then(|v| v.as_str()).map(String::from);
        super::trace_session_metadata(
            Provider::KimiCode,
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

    fn handle_content(&mut self, obj: &Value) -> Option<StreamChunk> {
        // Try content array with text parts
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

        // Try direct text field
        if let Some(text) = obj.get("text").and_then(|t| t.as_str())
            && !text.is_empty()
        {
            self.assistant_text.push_str(text);
            return Some(StreamChunk::Text(super::ensure_message_newline(
                text.to_string(),
            )));
        }

        // Try content as string
        if let Some(text) = obj.get("content").and_then(|c| c.as_str())
            && !text.is_empty()
        {
            self.assistant_text.push_str(text);
            return Some(StreamChunk::Text(super::ensure_message_newline(
                text.to_string(),
            )));
        }

        None
    }

    fn handle_status_update(&mut self, obj: &Value) {
        // Token usage from latest status update (last snapshot wins)
        if let Some(usage) = obj.get("usage").or_else(|| obj.get("token_usage")) {
            let input = usage.get("input_tokens").and_then(|v| v.as_u64());
            let output = usage.get("output_tokens").and_then(|v| v.as_u64());
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => usage.get("total_tokens").and_then(|v| v.as_u64()),
            };
            let cache_read = usage
                .get("cache_read_input_tokens")
                .and_then(|v| v.as_u64());
            self.token_usage = Some(NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read,
            });
        }

        self.cost_usd = obj
            .get("cost_usd")
            .and_then(|v| v.as_f64())
            .or(self.cost_usd);
        self.duration_ms = obj
            .get("duration_ms")
            .and_then(|v| v.as_u64())
            .or(self.duration_ms);
        self.num_turns = obj
            .get("num_turns")
            .and_then(|v| v.as_u64())
            .map(|v| v as u32)
            .or(self.num_turns);

        // Context window pressure
        if let Some(ctx) = obj.get("context_usage").or_else(|| obj.get("context")) {
            let used = ctx.get("used").and_then(|v| v.as_u64());
            let total = ctx.get("total").and_then(|v| v.as_u64());
            let percent =
                ctx.get("percent")
                    .and_then(|v| v.as_f64())
                    .or_else(|| match (used, total) {
                        (Some(u), Some(t)) if t > 0 => Some((u as f64 / t as f64) * 100.0),
                        _ => None,
                    });

            let context = ContextUsage {
                used,
                total,
                percent,
            };

            if let Some(pct) = percent
                && pct >= CONTEXT_PRESSURE_WARN_PERCENT
            {
                self.sink.on_warning(&format!(
                    "Context window pressure: {pct:.0}% used ({}/{} tokens)",
                    used.unwrap_or(0),
                    total.unwrap_or(0)
                ));
            }

            self.context_usage = Some(context);
        }
        super::trace_summary_update(
            Provider::KimiCode,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );
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
            .or_else(|| obj.get("message").and_then(|m| m.as_str()))
            .map(String::from);

        let meta = EventMeta::default();
        self.sink.on_turn_error(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for KimiStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let obj: Value = serde_json::from_str(line).map_err(|e| {
            self.sink
                .on_warning(&format!("Malformed JSON on line {}: {e}", self.line_num));
            super::trace_malformed_line(Provider::KimiCode, self.line_num, &e.to_string());
            StreamParseError::MalformedLine {
                line_num: self.line_num,
                message: e.to_string(),
            }
        })?;

        let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");
        super::trace_parser_event(Provider::KimiCode, event_type, self.line_num);

        match event_type {
            "init" | "system" => {
                self.handle_init(&obj);
                Ok(None)
            }
            "assistant" | "message" | "content" | "ContentPart" => Ok(self.handle_content(&obj)),
            "StatusUpdate" | "status_update" | "status" => {
                self.handle_status_update(&obj);
                Ok(None)
            }
            "error" => {
                self.handle_error(&obj);
                Ok(None)
            }
            "tool_use" => {
                self.tool_calls += 1;
                super::trace_tool_event(
                    Provider::KimiCode,
                    self.tool_calls,
                    obj.get("name")
                        .or_else(|| obj.get("tool_name"))
                        .and_then(|value| value.as_str()),
                );
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
            provider: Provider::KimiCode,
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
            context_usage: self.context_usage,
            raw_summary: None,
            stderr_text: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::stream::parser::NullSink;

    struct WarningSink {
        warnings: Mutex<Vec<String>>,
    }

    impl WarningSink {
        fn new() -> Self {
            Self {
                warnings: Mutex::new(Vec::new()),
            }
        }
    }

    impl StreamEventSink for WarningSink {
        fn on_warning(&mut self, message: &str) {
            self.warnings.lock().unwrap().push(message.into());
        }
    }

    fn make_parser() -> Box<KimiStreamParser<NullSink>> {
        Box::new(KimiStreamParser::new(NullSink))
    }

    #[test]
    fn summary_from_last_status_update() {
        let mut parser = make_parser();

        parser
            .feed_line(r#"{"type":"init","session_id":"kimi-1","model":"kimi-coder"}"#)
            .unwrap();

        parser
            .feed_line(r#"{"type":"assistant","content":[{"text":"Hello from Kimi"}]}"#)
            .unwrap();

        // First status update
        parser
            .feed_line(
                r#"{"type":"StatusUpdate","usage":{"input_tokens":100,"output_tokens":50},"duration_ms":3000}"#,
            )
            .unwrap();

        // Second (later) status update overwrites
        parser
            .feed_line(
                r#"{"type":"StatusUpdate","usage":{"input_tokens":200,"output_tokens":100},"duration_ms":6000,"num_turns":2}"#,
            )
            .unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.provider, Provider::KimiCode);
        assert_eq!(summary.assistant_text, "Hello from Kimi");
        assert_eq!(summary.duration_ms, Some(6000));
        assert_eq!(summary.num_turns, Some(2));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(200));
        assert_eq!(usage.output, Some(100));
    }

    #[test]
    fn context_pressure_warning() {
        let sink = WarningSink::new();
        let mut parser = Box::new(KimiStreamParser::new(sink));

        parser
            .feed_line(r#"{"type":"StatusUpdate","context_usage":{"used":110000,"total":128000}}"#)
            .unwrap();

        let warnings: Vec<String> = parser.sink.warnings.lock().unwrap().clone();
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].contains("Context window pressure"));

        let summary = parser.finish(0);
        let ctx = summary.context_usage.unwrap();
        assert_eq!(ctx.used, Some(110000));
        assert_eq!(ctx.total, Some(128000));
        assert!(ctx.percent.unwrap() > 85.0);
    }

    #[test]
    fn no_warning_below_threshold() {
        let sink = WarningSink::new();
        let mut parser = Box::new(KimiStreamParser::new(sink));

        parser
            .feed_line(r#"{"type":"StatusUpdate","context_usage":{"used":50000,"total":128000}}"#)
            .unwrap();

        let warnings: Vec<String> = parser.sink.warnings.lock().unwrap().clone();
        assert!(warnings.is_empty());
    }

    #[test]
    fn missing_model_and_cost_tolerated() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"kimi-2"}"#)
            .unwrap();

        let summary = parser.finish(0);
        assert!(summary.model.is_none());
        assert!(summary.cost_usd.is_none());
    }
}
