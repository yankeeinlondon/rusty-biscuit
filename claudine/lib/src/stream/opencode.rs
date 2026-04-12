use std::collections::HashMap;

use serde_json::Value;

use super::parser::{EventMeta, StreamChunk, StreamEventSink, StreamParseError, StreamParser};
use super::protocol::opencode::{
    OpenCodeError, OpenCodeEvent, OpenCodeInit, OpenCodeStepComplete, OpenCodeStepFinish,
    OpenCodeStepStart, OpenCodeText,
};
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
    tool_uses: HashMap<String, (Option<String>, Option<Value>)>,
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
            tool_uses: HashMap::new(),
        }
    }

    fn handle_init(&mut self, init: OpenCodeInit) {
        self.session_id = init.session_id;
        // Override model if stream provides it
        if let Some(model) = init.model {
            self.model = Some(model);
        }
        super::trace_session_metadata(
            Provider::OpenCode,
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

    fn handle_text(&mut self, event: OpenCodeText) -> Option<StreamChunk> {
        let text = event.resolved_text()?;
        if text.is_empty() {
            return None;
        }
        self.assistant_text.push_str(&text);
        Some(StreamChunk::Text(text))
    }

    fn handle_step_start(&mut self, step: OpenCodeStepStart) {
        // Capture session ID from first step_start and emit session_start.
        // OpenCode doesn't send a dedicated init/session_start event; the
        // session ID arrives in the first step_start payload instead.
        let first_step = self.session_id.is_none();
        if first_step {
            self.session_id = step.resolved_session_id();

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

        self.num_turns += 1;
        let meta = EventMeta::default();
        self.sink.on_step_start(&meta);
    }

    fn handle_step_finish(&mut self, event: OpenCodeStepFinish) {
        if let Some(part) = event.part {
            if let Some(tokens) = part.tokens {
                let step = NormalizedTokenUsage {
                    input: tokens.input,
                    output: tokens.output,
                    total: tokens.total,
                    cache_read: tokens.cache.and_then(|c| c.read),
                };
                self.token_usage.accumulate(&step);
            }

            if let Some(cost) = part.cost {
                self.cost_usd += cost;
            }

            if let Some(reason) = part.reason {
                self.provider_status = Some(reason);
            }
        }

        super::trace_summary_update(
            Provider::OpenCode,
            self.provider_status.as_deref(),
            self.duration_ms,
            Some(self.cost_usd),
        );

        let meta = EventMeta::default();
        self.sink.on_step_finish(&meta);
    }

    fn handle_step_complete(&mut self, event: OpenCodeStepComplete) {
        self.num_turns += 1;

        // Legacy format: {"type":"step_complete","usage":{...},"cost_usd":...}
        if let Some(usage) = event.usage {
            let step = NormalizedTokenUsage {
                input: usage.input_tokens,
                output: usage.output_tokens,
                total: usage.total_tokens,
                cache_read: None,
            };
            self.token_usage.accumulate(&step);
        }

        if let Some(cost) = event.cost_usd {
            self.cost_usd += cost;
        }

        if let Some(duration) = event.duration_ms {
            self.duration_ms = Some(duration);
        }

        super::trace_summary_update(
            Provider::OpenCode,
            self.provider_status.as_deref(),
            self.duration_ms,
            Some(self.cost_usd),
        );

        let meta = EventMeta::default();
        self.sink.on_turn_complete(&meta);
    }

    fn handle_error(&mut self, event: OpenCodeError) {
        self.is_error = true;
        self.error_kind = event.resolved_kind();
        self.error_message = event.resolved_message();

        self.sink
            .on_warning(self.error_message.as_deref().unwrap_or("Step failure"));

        let meta = EventMeta::default();
        self.sink.on_turn_error(&meta);
    }
}

impl<S: StreamEventSink + Send> StreamParser for OpenCodeStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<Option<StreamChunk>, StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(None);
        }

        let raw: Value = serde_json::from_str(line).map_err(|e| {
            self.sink
                .on_warning(&format!("Malformed JSON on line {}: {e}", self.line_num));
            super::trace_malformed_line(Provider::OpenCode, self.line_num, &e.to_string());
            StreamParseError::MalformedLine {
                line_num: self.line_num,
                message: e.to_string(),
            }
        })?;

        let event_type = raw.get("type").and_then(|t| t.as_str()).unwrap_or("");
        super::trace_parser_event(Provider::OpenCode, event_type, self.line_num);

        match serde_json::from_value::<OpenCodeEvent>(raw) {
            Ok(OpenCodeEvent::Init(init) | OpenCodeEvent::SessionStart(init)) => {
                self.handle_init(init);
                Ok(None)
            }
            Ok(OpenCodeEvent::StepStart(step)) => {
                self.handle_step_start(step);
                Ok(None)
            }
            Ok(
                OpenCodeEvent::Text(text)
                | OpenCodeEvent::TextDelta(text)
                | OpenCodeEvent::AssistantText(text),
            ) => Ok(self.handle_text(text)),
            Ok(OpenCodeEvent::StepFinish(sf)) => {
                self.handle_step_finish(sf);
                Ok(None)
            }
            Ok(OpenCodeEvent::StepComplete(sc) | OpenCodeEvent::TurnComplete(sc)) => {
                self.handle_step_complete(sc);
                Ok(None)
            }
            Ok(OpenCodeEvent::Error(err) | OpenCodeEvent::StepError(err)) => {
                self.handle_error(err);
                Ok(None)
            }
            Ok(OpenCodeEvent::ToolUse(tool) | OpenCodeEvent::ToolStart(tool)) => {
                self.tool_calls += 1;
                let resolved = tool.resolve();
                super::trace_tool_event(
                    Provider::OpenCode,
                    self.tool_calls,
                    resolved.name.as_deref(),
                );
                let mut meta = EventMeta::default();
                if let Some(tool_id) = &resolved.id {
                    meta.extra
                        .insert("tool_id".into(), Value::String(tool_id.clone()));
                }
                if let Some(tool_name) = &resolved.name {
                    meta.extra
                        .insert("tool_name".into(), Value::String(tool_name.clone()));
                }
                if let Some(tool_input) = &resolved.input {
                    meta.extra.insert("tool_input".into(), tool_input.clone());
                }
                if let Some(tool_id) = resolved.id {
                    self.tool_uses
                        .insert(tool_id, (resolved.name, resolved.input));
                }
                self.sink.on_before_tool(&meta);
                Ok(None)
            }
            Ok(OpenCodeEvent::ToolResult(tool) | OpenCodeEvent::ToolEnd(tool)) => {
                let resolved = tool.resolve();
                let (tool_name, tool_input) = resolved
                    .id
                    .as_ref()
                    .and_then(|id| self.tool_uses.remove(id))
                    .unwrap_or((resolved.name.clone(), None));
                let mut meta = EventMeta::default();
                if let Some(tool_id) = resolved.id {
                    meta.extra.insert("tool_id".into(), Value::String(tool_id));
                }
                if let Some(tool_name) = tool_name {
                    meta.extra
                        .insert("tool_name".into(), Value::String(tool_name));
                }
                if let Some(tool_input) = tool_input {
                    meta.extra.insert("tool_input".into(), tool_input);
                }
                if let Some(tool_output) = resolved.output {
                    meta.extra.insert("tool_response".into(), tool_output);
                }
                if let Some(status) = resolved.status {
                    meta.extra.insert("status".into(), Value::String(status));
                }
                if let Some(error) = resolved.error {
                    meta.extra.insert("error".into(), error);
                }
                self.sink.on_after_tool(&meta);
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        super::trace_parser_finish(
            Provider::OpenCode,
            exit_code,
            self.tool_calls,
            self.num_turns,
            self.provider_status.as_deref(),
        );
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
            stderr_text: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::parser::{EventMeta, NullSink, StreamEventSink};
    use crate::stream::test_support::{ToolContractExpectation, assert_tool_event_contract};

    #[derive(Default)]
    struct RecordingSink {
        session_start: usize,
        turn_start: usize,
        step_start: usize,
        step_finish: usize,
        turn_complete: usize,
        before_tool: Vec<EventMeta>,
        after_tool: Vec<EventMeta>,
    }

    impl StreamEventSink for RecordingSink {
        fn on_session_start(&mut self, _meta: &EventMeta) {
            self.session_start += 1;
        }

        fn on_turn_start(&mut self, _meta: &EventMeta) {
            self.turn_start += 1;
        }

        fn on_step_start(&mut self, _meta: &EventMeta) {
            self.step_start += 1;
        }

        fn on_step_finish(&mut self, _meta: &EventMeta) {
            self.step_finish += 1;
        }

        fn on_turn_complete(&mut self, _meta: &EventMeta) {
            self.turn_complete += 1;
        }

        fn on_before_tool(&mut self, meta: &EventMeta) {
            self.before_tool.push(meta.clone());
        }

        fn on_after_tool(&mut self, meta: &EventMeta) {
            self.after_tool.push(meta.clone());
        }
    }

    fn make_parser() -> Box<OpenCodeStreamParser<NullSink>> {
        Box::new(OpenCodeStreamParser::new(NullSink, Some("gpt-4o".into())))
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
    fn real_opencode_ndjson_format() {
        let mut parser = make_parser();

        let step_start = r#"{"type":"step_start","timestamp":1773725437967,"sessionID":"ses_abc123","part":{"id":"prt_1","sessionID":"ses_abc123","messageID":"msg_1","type":"step-start","snapshot":"abc"}}"#;
        parser.feed_line(step_start).unwrap();

        let text = r#"{"type":"text","timestamp":1773725438532,"sessionID":"ses_abc123","part":{"id":"prt_2","sessionID":"ses_abc123","messageID":"msg_1","type":"text","text":"hello"}}"#;
        let result = parser.feed_line(text).unwrap();
        assert_eq!(result, Some(StreamChunk::Text("hello".into())));

        let step_finish = r#"{"type":"step_finish","timestamp":1773725438789,"sessionID":"ses_abc123","part":{"id":"prt_3","sessionID":"ses_abc123","messageID":"msg_1","type":"step-finish","reason":"stop","cost":0.0205797,"tokens":{"total":54665,"input":150,"output":23,"reasoning":0,"cache":{"read":0,"write":54492}}}}"#;
        parser.feed_line(step_finish).unwrap();

        let summary = parser.finish(0);
        assert_eq!(summary.session_id.as_deref(), Some("ses_abc123"));
        assert_eq!(summary.assistant_text, "hello");
        assert_eq!(summary.cost_usd, Some(0.0205797));
        assert_eq!(summary.provider_status.as_deref(), Some("stop"));

        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(150));
        assert_eq!(usage.output, Some(23));
        assert_eq!(usage.total, Some(54665));
    }

    #[test]
    fn step_failure_warning() {
        let mut parser = make_parser();
        parser
            .feed_line(r#"{"type":"error","error_message":"API timeout"}"#)
            .unwrap();

        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_message.as_deref(), Some("API timeout"));
    }

    #[test]
    fn tool_name_extraction_supports_nested_part_fields() {
        let mut parser = OpenCodeStreamParser::new(RecordingSink::default(), None);

        parser
            .feed_line(r#"{"type":"tool_use","part":{"name":"search"}}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"tool_start","part":{"tool_name":"write_file"}}"#)
            .unwrap();

        let sink = parser.sink;
        assert_eq!(sink.before_tool.len(), 2);
        assert_eq!(sink.before_tool[0].extra["tool_name"], "search");
        assert_eq!(sink.before_tool[1].extra["tool_name"], "write_file");
    }

    #[test]
    fn tool_events_preserve_opencode_parameters_and_results() {
        let mut parser = OpenCodeStreamParser::new(RecordingSink::default(), None);

        parser
            .feed_line(
                r#"{"type":"tool_start","part":{"id":"tool-1","tool_name":"bash","input":{"command":"git status"}}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_end","part":{"tool_use_id":"tool-1","status":"success","content":"working tree clean"}}"#,
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
                id: Some("tool-1"),
                input_field: Some(("command", "git status")),
                status: Some("success"),
                response: Some(Value::String("working tree clean".into())),
            },
        );
    }

    #[test]
    fn step_boundaries_do_not_emit_high_level_turn_lifecycle_events() {
        let mut parser = OpenCodeStreamParser::new(RecordingSink::default(), None);

        parser
            .feed_line(r#"{"type":"step_start","sessionID":"ses_1"}"#)
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"step_finish","part":{"reason":"stop","cost":0.01,"tokens":{"input":1,"output":2,"total":3,"cache":{"read":0}}}}"#,
            )
            .unwrap();
        parser
            .feed_line(r#"{"type":"step_complete","usage":{"input_tokens":1,"output_tokens":2,"total_tokens":3},"cost_usd":0.01}"#)
            .unwrap();

        let sink = parser.sink;
        assert_eq!(sink.session_start, 1);
        assert_eq!(sink.turn_start, 0);
        assert_eq!(sink.step_start, 1);
        assert_eq!(sink.step_finish, 1);
        assert_eq!(sink.turn_complete, 1);
    }
}
