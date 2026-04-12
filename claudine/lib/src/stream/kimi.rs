use std::collections::HashMap;

use serde_json::Value;

use super::parser::{EventMeta, StreamChunk, StreamEventSink, StreamParseError, StreamParser};
use super::protocol::kimi::{
    KimiContent, KimiErrorEvent, KimiEvent, KimiInit, KimiStatusUpdate, KimiTool,
};
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
    tool_uses: HashMap<String, (Option<String>, Option<Value>)>,
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
            tool_uses: HashMap::new(),
        }
    }

    fn handle_init(&mut self, init: KimiInit) {
        self.session_id = init.session_id;
        self.model = init.model;
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

    fn handle_content(&mut self, event: KimiContent) -> Option<StreamChunk> {
        let text = event.resolved_text()?;
        if text.is_empty() {
            return None;
        }
        self.assistant_text.push_str(&text);
        Some(StreamChunk::Text(super::ensure_message_newline(text)))
    }

    fn handle_status_update(&mut self, mut event: KimiStatusUpdate) {
        // Token usage from latest status update (last snapshot wins)
        if let Some(usage) = event.resolved_usage() {
            let input = usage.input_tokens;
            let output = usage.output_tokens;
            let total = match (input, output) {
                (Some(i), Some(o)) => Some(i + o),
                _ => usage.total_tokens,
            };
            let cache_read = usage.cache_read_input_tokens;
            self.token_usage = Some(NormalizedTokenUsage {
                input,
                output,
                total,
                cache_read,
            });
        }

        self.cost_usd = event.cost_usd.or(self.cost_usd);
        self.duration_ms = event.duration_ms.or(self.duration_ms);
        self.num_turns = event.num_turns.map(|v| v as u32).or(self.num_turns);

        // Context window pressure
        if let Some(ctx) = event.resolved_context() {
            let used = ctx.used;
            let total = ctx.total;
            let percent = ctx.computed_percent();

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

    fn handle_error(&mut self, event: KimiErrorEvent) {
        self.is_error = true;
        self.error_kind = event.resolved_kind();
        self.error_message = event.resolved_message();

        let meta = EventMeta::default();
        self.sink.on_turn_error(&meta);
    }

    fn handle_tool_use(&mut self, mut tool: KimiTool) {
        self.tool_calls += 1;
        let tool_id = tool.resolved_tool_id().map(ToOwned::to_owned);
        let tool_name = tool.resolved_tool_name().map(ToOwned::to_owned);
        let tool_input = tool.take_input();
        super::trace_tool_event(Provider::KimiCode, self.tool_calls, tool_name.as_deref());
        let mut meta = EventMeta::default();
        if let Some(tool_id) = &tool_id {
            meta.extra
                .insert("tool_id".into(), Value::String(tool_id.clone()));
        }
        if let Some(tool_name) = &tool_name {
            meta.extra
                .insert("tool_name".into(), Value::String(tool_name.clone()));
        }
        if let Some(tool_input) = &tool_input {
            meta.extra.insert("tool_input".into(), tool_input.clone());
        }
        if let Some(tool_id) = tool_id {
            self.tool_uses.insert(tool_id, (tool_name, tool_input));
        }
        self.sink.on_before_tool(&meta);
    }

    fn handle_tool_result(&mut self, mut tool: KimiTool) {
        let tool_id = tool.resolved_tool_id().map(ToOwned::to_owned);
        let (tool_name, tool_input) = tool_id
            .as_ref()
            .and_then(|id| self.tool_uses.remove(id))
            .unwrap_or((None, None));
        let tool_output = tool.take_output();
        let status = tool.status.take();
        let error = tool.error.take();

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
        if let Some(tool_output) = tool_output {
            meta.extra.insert("tool_response".into(), tool_output);
        }
        if let Some(status) = status {
            meta.extra.insert("status".into(), Value::String(status));
        }
        if let Some(error) = error {
            meta.extra.insert("error".into(), error);
        }
        self.sink.on_after_tool(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for KimiStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let raw: Value = serde_json::from_str(line).map_err(|e| {
            self.sink
                .on_warning(&format!("Malformed JSON on line {}: {e}", self.line_num));
            super::trace_malformed_line(Provider::KimiCode, self.line_num, &e.to_string());
            StreamParseError::MalformedLine {
                line_num: self.line_num,
                message: e.to_string(),
            }
        })?;

        let event_type = raw.get("type").and_then(|t| t.as_str()).unwrap_or("");
        super::trace_parser_event(Provider::KimiCode, event_type, self.line_num);

        match serde_json::from_value::<KimiEvent>(raw) {
            Ok(KimiEvent::Init(init) | KimiEvent::System(init)) => {
                self.handle_init(init);
                Ok(None)
            }
            Ok(
                KimiEvent::Assistant(content)
                | KimiEvent::Message(content)
                | KimiEvent::Content(content)
                | KimiEvent::ContentPart(content),
            ) => Ok(self.handle_content(content)),
            Ok(
                KimiEvent::StatusUpdatePascal(status)
                | KimiEvent::StatusUpdate(status)
                | KimiEvent::Status(status),
            ) => {
                self.handle_status_update(status);
                Ok(None)
            }
            Ok(KimiEvent::Error(err)) => {
                self.handle_error(err);
                Ok(None)
            }
            Ok(KimiEvent::ToolUse(tool)) => {
                self.handle_tool_use(tool);
                Ok(None)
            }
            Ok(KimiEvent::ToolResult(tool)) => {
                self.handle_tool_result(tool);
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        let mut summary = StreamExecutionSummary {
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
            badges: Vec::new(),
            raw_summary: None,
            stderr_text: None,
        };
        summary.badges = crate::stream::badges::derive_badges(&summary, Provider::KimiCode);
        summary
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;
    use crate::stream::parser::NullSink;
    use crate::stream::test_support::{ToolContractExpectation, assert_tool_event_contract};

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

    #[derive(Default)]
    struct ToolRecordingSink {
        before_tool: Vec<EventMeta>,
        after_tool: Vec<EventMeta>,
    }

    impl StreamEventSink for ToolRecordingSink {
        fn on_before_tool(&mut self, meta: &EventMeta) {
            self.before_tool.push(meta.clone());
        }

        fn on_after_tool(&mut self, meta: &EventMeta) {
            self.after_tool.push(meta.clone());
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

    #[test]
    fn tool_events_preserve_parameters_and_results() {
        let mut parser = Box::new(KimiStreamParser::new(ToolRecordingSink::default()));

        parser
            .feed_line(
                r#"{"type":"tool_use","id":"k1","name":"bash","input":{"command":"git status"}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_result","tool_use_id":"k1","status":"success","content":"clean"}"#,
            )
            .unwrap();

        let sink = parser.sink;
        assert_eq!(sink.before_tool.len(), 1);
        assert_eq!(sink.after_tool.len(), 1);
        assert_tool_event_contract(
            &sink.before_tool[0],
            Some(&sink.after_tool[0]),
            ToolContractExpectation {
                name: "bash",
                id: Some("k1"),
                input_field: Some(("command", "git status")),
                status: Some("success"),
                response: Some(Value::String("clean".into())),
            },
        );
    }
}
