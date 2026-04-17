//! Native [`SemanticStreamParser`] implementation for Qwen Code's
//! `stream-json` format.
//!
//! Structurally similar to Gemini but tolerates Qwen-specific event names
//! (`tool_call`/`tool_response`, `assistant_message`, `summary` in place of
//! `result`, `system/session_start` in place of `init`, etc.).

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::qwen::{
    QwenErrorEvent, QwenEvent, QwenInit, QwenMessage, QwenResult, QwenTool,
};
use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

pub struct QwenSemanticStreamParser<S: SemanticEventSink> {
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

impl<S: SemanticEventSink> QwenSemanticStreamParser<S> {
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

    fn base_extra(&self, raw_kind: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("provider".into(), Value::from("qwen"));
        m.insert("line_num".into(), Value::from(self.line_num));
        m.insert("raw_kind".into(), Value::from(raw_kind));
        m
    }

    fn handle_init(&mut self, init: QwenInit, raw_kind: &str) {
        self.session_id = init.session_id;
        self.model = init.model;
        super::trace_session_metadata(
            Provider::QwenCode,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_message(
        &mut self,
        msg: QwenMessage,
        from_assistant_type: bool,
        raw_kind: &str,
        raw: Value,
    ) {
        if !from_assistant_type && msg.role.as_deref() != Some("assistant") {
            self.sink
                .on_semantic_event(SemanticEvent::ProviderExtension {
                    provider: Provider::QwenCode,
                    kind: "message.non_assistant".into(),
                    payload: raw,
                });
            return;
        }
        let Some(text) = msg.resolved_text() else {
            return;
        };
        self.assistant_text.push_str(&text);
        self.sink.on_semantic_event(SemanticEvent::OutputText {
            text: super::ensure_message_newline(text),
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_result(&mut self, result: QwenResult, raw: Value, raw_kind: &str) {
        let (usage, meta) = result.resolved_usage();
        self.duration_ms = meta.duration_ms;
        self.num_turns = meta.num_turns.map(|v| v as u32);
        self.provider_status = meta.stop_reason.clone();
        self.cost_usd = meta.cost_usd;

        let step_usage = usage.map(|usage| {
            let total = match (usage.input_tokens, usage.output_tokens) {
                (Some(i), Some(o)) => Some(i + o),
                _ => usage.total_tokens,
            };
            NormalizedTokenUsage {
                input: usage.input_tokens,
                output: usage.output_tokens,
                total,
                cache_read: usage.cache_read_input_tokens,
            }
        });
        if let Some(tu) = &step_usage {
            self.token_usage = Some(tu.clone());
        }

        self.raw_summary = Some(raw);
        super::trace_summary_update(
            Provider::QwenCode,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );

        self.sink.on_semantic_event(SemanticEvent::TurnComplete {
            provider_status: self.provider_status.clone(),
            token_usage: step_usage,
            cost_usd: self.cost_usd,
            duration_ms: self.duration_ms,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_error(&mut self, event: QwenErrorEvent, raw_kind: &str) {
        self.is_error = true;
        let detail = event.error;
        self.error_kind = detail.as_ref().and_then(|d| d.kind.clone());
        self.error_message = detail.and_then(|d| d.message);

        let mut extra = self.base_extra(raw_kind);
        if let Some(kind) = &self.error_kind {
            extra.insert("error_kind".into(), Value::from(kind.as_str()));
        }
        let semantic_kind =
            classify_error(self.error_kind.as_deref(), self.error_message.as_deref());
        self.sink.on_semantic_event(SemanticEvent::Error {
            message: self.error_message.clone().unwrap_or_default(),
            terminal: true,
            kind: semantic_kind,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_use(&mut self, mut tool: QwenTool, raw_kind: &str) {
        self.tool_calls += 1;
        let tool_id = tool.resolved_tool_id().map(String::from);
        let tool_name = tool.resolved_tool_name().map(String::from);
        let tool_input = tool.take_input();
        super::trace_tool_event(Provider::QwenCode, self.tool_calls, tool_name.as_deref());

        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &tool_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &tool_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        if let Some(id) = &tool_id {
            self.tool_uses
                .insert(id.clone(), (tool_name.clone(), tool_input.clone()));
        }
        self.sink.on_semantic_event(SemanticEvent::ToolCall {
            name: tool_name,
            id: tool_id,
            input: tool_input,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_result(&mut self, mut tool: QwenTool, raw_kind: &str) {
        let tool_id = tool.resolved_tool_id().map(String::from);
        let (tool_name, _tool_input) = tool_id
            .as_ref()
            .and_then(|id| self.tool_uses.remove(id))
            .unwrap_or((None, None));
        let tool_output = tool.take_output();
        let status = tool.status.take();
        let error = tool.error.take();

        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &tool_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &tool_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        if let Some(s) = &status {
            extra.insert("status".into(), Value::from(s.as_str()));
        }
        if let Some(err) = &error {
            extra.insert("error".into(), err.clone());
        }

        self.sink.on_semantic_event(SemanticEvent::ToolResult {
            name: tool_name,
            id: tool_id,
            status,
            exit_code: None,
            output: tool_output,
            extra: Value::Object(extra),
        });
    }

    fn emit_provider_extension(&mut self, kind: &str, payload: Value) {
        self.sink
            .on_semantic_event(SemanticEvent::ProviderExtension {
                provider: Provider::QwenCode,
                kind: kind.to_string(),
                payload,
            });
    }

    fn emit_malformed_warning(&mut self, err: &str) {
        let mut extra = self.base_extra("malformed_json");
        extra.insert("line_num".into(), Value::from(self.line_num));
        self.sink.on_semantic_event(SemanticEvent::Warning {
            message: format!("Malformed JSON on line {}: {err}", self.line_num),
            extra: Value::Object(extra),
        });
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for QwenSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }
        let raw: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                super::trace_malformed_line(Provider::QwenCode, self.line_num, &e.to_string());
                self.emit_malformed_warning(&e.to_string());
                return Ok(());
            }
        };
        let raw_kind = raw
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        super::trace_parser_event(Provider::QwenCode, &raw_kind, self.line_num);

        match serde_json::from_value::<QwenEvent>(raw.clone()) {
            Ok(QwenEvent::Init(init)) => {
                self.handle_init(init, &raw_kind);
            }
            Ok(QwenEvent::System(sys)) => {
                if sys.is_session_start() {
                    self.handle_init(sys.into_init(), &raw_kind);
                } else {
                    self.emit_provider_extension(&raw_kind, raw);
                }
            }
            Ok(QwenEvent::Message(msg) | QwenEvent::AssistantMessage(msg)) => {
                self.handle_message(msg, false, &raw_kind, raw);
            }
            Ok(QwenEvent::Assistant(msg)) => {
                self.handle_message(msg, true, &raw_kind, raw);
            }
            Ok(QwenEvent::Error(err)) => {
                self.handle_error(err, &raw_kind);
            }
            Ok(QwenEvent::Result(result) | QwenEvent::Summary(result)) => {
                self.handle_result(result, raw, &raw_kind);
            }
            Ok(QwenEvent::ToolUse(tool) | QwenEvent::ToolCall(tool)) => {
                self.handle_tool_use(tool, &raw_kind);
            }
            Ok(QwenEvent::ToolResult(tool) | QwenEvent::ToolResponse(tool)) => {
                self.handle_tool_result(tool, &raw_kind);
            }
            Err(_) => {
                self.emit_provider_extension(&raw_kind, raw);
            }
        }
        Ok(())
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        let mut summary = StreamExecutionSummary {
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
            permission_prompts: None,
            user_input_prompts: None,
            rate_limit: None,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: self.raw_summary,
            stderr_text: None,
            stderr_diagnostics: None,
        };
        summary.badges = crate::stream::badges::derive_badges(&summary, Provider::QwenCode);
        summary
    }
}

/// Map a Qwen Code error envelope onto a typed [`SemanticErrorKind`].
fn classify_error(error_kind: Option<&str>, message: Option<&str>) -> SemanticErrorKind {
    if let Some(kind) = error_kind {
        let lower = kind.to_ascii_lowercase();
        if lower.contains("rate") || lower.contains("quota") || lower.contains("billing") {
            return SemanticErrorKind::ApiRemote;
        }
        if lower.contains("auth") || lower.contains("config") || lower.contains("permission") {
            return SemanticErrorKind::Configuration;
        }
        if lower.contains("interrupt") || lower.contains("cancel") || lower.contains("abort") {
            return SemanticErrorKind::Interrupted;
        }
        if lower.contains("api") || lower.contains("upstream") || lower.contains("server") {
            return SemanticErrorKind::ApiRemote;
        }
    }
    if let Some(msg) = message {
        let lower = msg.to_ascii_lowercase();
        if lower.contains("rate limit")
            || lower.contains("quota")
            || lower.contains("billing")
            || lower.contains("api error")
        {
            return SemanticErrorKind::ApiRemote;
        }
        if lower.contains("api key")
            || lower.contains("authentication")
            || lower.contains("not authorized")
            || lower.contains("permission denied")
        {
            return SemanticErrorKind::Configuration;
        }
        if lower.contains("interrupt") || lower.contains("cancel") || lower.contains("aborted") {
            return SemanticErrorKind::Interrupted;
        }
    }
    SemanticErrorKind::AgentNative
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    struct Recording {
        events: Arc<Mutex<Vec<SemanticEvent>>>,
    }
    impl SemanticEventSink for Recording {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.events.lock().unwrap().push(event);
        }
    }

    fn new_parser() -> (
        Arc<Mutex<Vec<SemanticEvent>>>,
        Box<QwenSemanticStreamParser<Recording>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Recording {
            events: events.clone(),
        };
        (events, Box::new(QwenSemanticStreamParser::new(sink)))
    }

    fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
        events.iter().map(|e| e.kind_str()).collect()
    }

    #[test]
    fn init_emits_session_start() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"q1","model":"qwen-coder"}"#)
            .unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::SessionStart { .. }
        ));
    }

    #[test]
    fn assistant_message_emits_output_text() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"message","role":"assistant","content":[{"text":"Hello from Qwen"}]}"#,
            )
            .unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::OutputText { ref text, .. } if text == "Hello from Qwen\n"
        ));
    }

    #[test]
    fn qwen_tool_call_and_response_emit_typed_events() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_call","id":"q1","name":"bash","input":{"command":"git status"}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_response","tool_use_id":"q1","status":"success","content":"clean"}"#,
            )
            .unwrap();
        assert_eq!(
            kinds(&events.lock().unwrap()),
            vec!["tool_call", "tool_result"]
        );
    }

    #[test]
    fn summary_alias_routes_to_turn_complete() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"summary","duration_ms":3000,"token_usage":{"input_tokens":100,"output_tokens":50}}"#,
            )
            .unwrap();
        match &events.lock().unwrap()[0] {
            SemanticEvent::TurnComplete {
                duration_ms,
                token_usage,
                ..
            } => {
                assert_eq!(*duration_ms, Some(3000));
                assert_eq!(token_usage.as_ref().unwrap().input, Some(100));
            }
            other => panic!("expected TurnComplete, got {other:?}"),
        }
    }

    #[test]
    fn system_session_start_maps_to_init() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"system","subtype":"session_start","session_id":"q2","model":"qwen3-coder"}"#,
            )
            .unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::SessionStart { .. }
        ));
        let summary = parser.finish(0);
        assert_eq!(summary.session_id.as_deref(), Some("q2"));
    }

    #[test]
    fn error_event_emits_terminal_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"error","error":{"type":"rate_limit","message":"slow down"}}"#)
            .unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::Error { terminal: true, .. }
        ));
    }

    #[test]
    fn unknown_event_becomes_provider_extension() {
        let (events, mut parser) = new_parser();
        parser.feed_line(r#"{"type":"something.new"}"#).unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::ProviderExtension { ref kind, .. } if kind == "something.new"
        ));
    }

    #[test]
    fn malformed_json_emits_warning() {
        let (events, mut parser) = new_parser();
        assert!(parser.feed_line("x").is_ok());
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::Warning { .. }
        ));
    }

    #[test]
    fn round_trip_fidelity_mixed_fixture() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"type":"init","session_id":"q","model":"m"}"#,
            r#"{"type":"message","role":"assistant","content":"hi"}"#,
            r#"{"type":"tool_call","id":"t","name":"b","input":{}}"#,
            r#"{"type":"tool_response","tool_use_id":"t","status":"success","content":"ok"}"#,
            r#"{"type":"summary","duration_ms":1,"usage":{"input_tokens":1,"output_tokens":2}}"#,
            r#"{"type":"future.unknown"}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        for event in events.lock().unwrap().iter() {
            let v = serde_json::to_value(event).unwrap();
            let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(v, serde_json::to_value(&decoded).unwrap());
        }
    }
}
