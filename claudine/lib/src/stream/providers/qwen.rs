//! Native [`SemanticStreamParser`] implementation for Qwen Code's
//! `stream-json` format.
//!
//! Structurally similar to Gemini but tolerates Qwen-specific event names
//! (`tool_call`/`tool_response`, `assistant_message`, `summary` in place of
//! `result`, `system/session_start` or `system/init` in place of `init`,
//! etc.).

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::parser::SemanticStreamParser;
use super::protocol::qwen::{
    QwenErrorEvent, QwenEvent, QwenInit, QwenMessage, QwenResult, QwenTool,
};
use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::provider_id::Provider;
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
        super::common::base_extra(Provider::QwenCode, self.line_num, raw_kind)
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

    fn handle_message(&mut self, msg: QwenMessage, raw_kind: &str) {
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
        super::common::emit_provider_extension(&mut self.sink, Provider::QwenCode, kind, payload);
    }

    fn emit_malformed_warning(&mut self, err: &str) {
        super::common::emit_malformed_warning(&mut self.sink, Provider::QwenCode, self.line_num, err);
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for QwenSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return;
        }

        // Try typed deserialization first to avoid `serde_json::Value` DOM
        // allocation on the hot path. Fall back to `Value` only for unknown
        // event types, non-assistant messages, or `result`/`summary` events
        // that need the raw payload for `raw_summary`.
        match serde_json::from_str::<QwenEvent>(line) {
            Ok(event) => {
                let raw_kind = event.type_str().to_string();
                super::trace_parser_event(Provider::QwenCode, &raw_kind, self.line_num);
                match event {
                    QwenEvent::Init(init) => {
                        self.handle_init(init, &raw_kind);
                    }
                    QwenEvent::System(sys) => {
                        if sys.is_session_start() {
                            self.handle_init(sys.into_init(), &raw_kind);
                        } else {
                            let raw = serde_json::to_value(&sys).expect("QwenSystem serializes");
                            self.emit_provider_extension(&raw_kind, raw);
                        }
                    }
                    QwenEvent::Message(msg) | QwenEvent::AssistantMessage(msg) => {
                        if msg.role.as_deref() != Some("assistant") {
                            let raw = serde_json::to_value(&msg).expect("QwenMessage serializes");
                            self.emit_provider_extension(&raw_kind, raw);
                        } else {
                            self.handle_message(msg, &raw_kind);
                        }
                    }
                    QwenEvent::Assistant(msg) => {
                        self.handle_message(msg, &raw_kind);
                    }
                    QwenEvent::Error(err) => {
                        self.handle_error(err, &raw_kind);
                    }
                    QwenEvent::Result(result) | QwenEvent::Summary(result) => {
                        let raw = serde_json::to_value(&result).expect("QwenResult serializes");
                        self.handle_result(result, raw, &raw_kind);
                    }
                    QwenEvent::ToolUse(tool) | QwenEvent::ToolCall(tool) => {
                        self.handle_tool_use(tool, &raw_kind);
                    }
                    QwenEvent::ToolResult(tool) | QwenEvent::ToolResponse(tool) => {
                        self.handle_tool_result(tool, &raw_kind);
                    }
                }
            }
            Err(_) => {
                let raw: Map<String, Value> = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        super::trace_malformed_line(
                            Provider::QwenCode,
                            self.line_num,
                            &e.to_string(),
                        );
                        self.emit_malformed_warning(&e.to_string());
                        return;
                    }
                };
                let raw_kind = raw
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                super::trace_parser_event(Provider::QwenCode, &raw_kind, self.line_num);
                self.emit_provider_extension(&raw_kind, Value::Object(raw));
            }
        }
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        super::common::finish_summary(
            Provider::QwenCode,
            StreamExecutionSummary {
                session_id: self.session_id,
                model: self.model,
                assistant_text: self.assistant_text,
                provider_status: self.provider_status,
                exit_code,
                is_error: self.is_error,
                error_kind: self.error_kind,
                error_message: self.error_message,
                duration_ms: self.duration_ms,
                num_turns: self.num_turns,
                token_usage: self.token_usage,
                cost_usd: self.cost_usd,
                tool_calls: (self.tool_calls > 0).then_some(self.tool_calls),
                raw_summary: self.raw_summary,
                ..Default::default()
            },
        )
    }
}

/// Map a Qwen Code error envelope onto a typed [`SemanticErrorKind`].
fn classify_error(error_kind: Option<&str>, message: Option<&str>) -> SemanticErrorKind {
    super::common::classify_error_by_keywords(
        super::vocabulary::error_keywords(Provider::QwenCode),
        None,
        error_kind,
        message,
    )
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use serde_json::json;

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
            .feed_line(r#"{"type":"init","session_id":"q1","model":"qwen-coder"}"#);
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
            );
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
            );
        parser
            .feed_line(
                r#"{"type":"tool_response","tool_use_id":"q1","status":"success","content":"clean"}"#,
            );
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
            );
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
            );
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::SessionStart { .. }
        ));
        let summary = parser.finish(0);
        assert_eq!(summary.session_id.as_deref(), Some("q2"));
    }

    #[test]
    fn system_init_maps_to_session_start() {
        // Serializer-faithful wire sample from the signals corpus
        // (qwen.md record `stream-model_resolved-system-init`, since 0.19.6).
        const QWEN_SYSTEM_INIT: &str = include_str!(
            "../../../../docs/research/signals/fixtures/qwen/system-init-model-version.jsonl"
        );
        let (events, mut parser) = new_parser();
        parser.feed_line(QWEN_SYSTEM_INIT.trim());
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::SessionStart { .. }
        ));
        let summary = parser.finish(0);
        assert_eq!(summary.session_id.as_deref(), Some("qw-1"));
        assert_eq!(summary.model.as_deref(), Some("qwen3-coder-plus"));
    }

    #[test]
    fn system_unknown_subtype_stays_provider_extension() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"system","subtype":"telemetry","session_id":"qx"}"#);
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::ProviderExtension { .. }
        ));
    }

    #[test]
    fn error_event_emits_terminal_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"error","error":{"type":"rate_limit","message":"slow down"}}"#);
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::Error { terminal: true, .. }
        ));
    }

    #[test]
    fn unknown_event_becomes_provider_extension() {
        let (events, mut parser) = new_parser();
        parser.feed_line(r#"{"type":"something.new"}"#);
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::ProviderExtension { ref kind, .. } if kind == "something.new"
        ));
    }

    #[test]
    fn malformed_json_emits_warning() {
        let (events, mut parser) = new_parser();
        parser.feed_line("x");
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::Warning { .. }
        ));
    }

    #[test]
    fn tool_input_string_fallback_parses_without_panic() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"tool_call","id":"t","name":"b","input":"ls -la"}"#);
        let collected = events.lock().unwrap().clone();
        assert_eq!(kinds(&collected), vec!["tool_call"]);
        match &collected[0] {
            SemanticEvent::ToolCall { input, .. } => {
                assert_eq!(input.as_ref().and_then(Value::as_str), Some("ls -la"));
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
    }

    #[test]
    fn missing_discriminator_falls_through_to_provider_extension() {
        let (events, mut parser) = new_parser();
        parser.feed_line(r#"{"payload":{"k":1}}"#);
        let collected = events.lock().unwrap().clone();
        assert_eq!(collected.len(), 1);
        match &collected[0] {
            SemanticEvent::ProviderExtension {
                provider,
                kind,
                payload,
            } => {
                assert_eq!(*provider, Provider::QwenCode);
                assert_eq!(kind, "");
                assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
            }
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
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
            parser.feed_line(line);
        }
        for event in events.lock().unwrap().iter() {
            let v = serde_json::to_value(event).unwrap();
            let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(v, serde_json::to_value(&decoded).unwrap());
        }
    }
}
