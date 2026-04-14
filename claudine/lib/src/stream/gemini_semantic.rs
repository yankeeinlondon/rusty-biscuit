//! Native [`SemanticStreamParser`] implementation for Gemini CLI's
//! `stream-json` format.
//!
//! Gemini's event set is smaller than Claude's / Codex's. This parser routes
//! the known event types to typed semantic variants and preserves anything
//! else as [`SemanticEvent::ProviderExtension`]:
//!
//! - `init` / `system` → [`SemanticEvent::SessionStart`]
//! - `message` (role = `assistant`) → [`SemanticEvent::OutputText`]; other
//!   roles are preserved via `ProviderExtension`.
//! - `tool_use` → [`SemanticEvent::ToolCall`]; `tool_result` → [`SemanticEvent::ToolResult`].
//! - `error` with `severity = "warning"` → [`SemanticEvent::Warning`]; other
//!   severities → [`SemanticEvent::Error`].
//! - `result` → [`SemanticEvent::TurnComplete`] and summary rollup.

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::gemini::{
    GeminiErrorEvent, GeminiEvent, GeminiInit, GeminiMessage, GeminiResult, GeminiToolResult,
    GeminiToolUse,
};
use super::semantic::{SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

pub struct GeminiSemanticStreamParser<S: SemanticEventSink> {
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

impl<S: SemanticEventSink> GeminiSemanticStreamParser<S> {
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
        m.insert("provider".into(), Value::from("gemini"));
        m.insert("line_num".into(), Value::from(self.line_num));
        m.insert("raw_kind".into(), Value::from(raw_kind));
        m
    }

    fn handle_init(&mut self, init: GeminiInit, raw_kind: &str) {
        self.session_id = init.session_id;
        self.model = init.model;
        super::trace_session_metadata(
            Provider::Gemini,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_message(&mut self, msg: GeminiMessage, raw_kind: &str, raw: Value) {
        if msg.role.as_deref() != Some("assistant") {
            // Non-assistant messages (user, system) are preserved but do not
            // flow through OutputText.
            self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
                provider: Provider::Gemini,
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

    fn handle_result(&mut self, result: GeminiResult, raw: Value, raw_kind: &str) {
        self.provider_status = result.status.clone();

        if result.status.as_deref() == Some("error") {
            self.is_error = true;
            if let Some(err) = &result.error {
                self.error_kind = err.kind.clone();
                self.error_message = err.message.clone();
            }
        }

        self.cost_usd = result.cost_usd;

        let step_usage = result.stats.as_ref().map(|stats| {
            self.duration_ms = stats.duration_ms;
            if let Some(tc) = stats.tool_calls {
                self.tool_calls = tc as u32;
            }
            let total = match (stats.input_tokens, stats.output_tokens) {
                (Some(i), Some(o)) => Some(i + o),
                _ => stats.total_tokens,
            };
            NormalizedTokenUsage {
                input: stats.input_tokens,
                output: stats.output_tokens,
                total,
                cache_read: stats.cached,
            }
        });
        if let Some(tu) = &step_usage {
            self.token_usage = Some(tu.clone());
        }

        self.raw_summary = Some(raw);
        super::trace_summary_update(
            Provider::Gemini,
            self.provider_status.as_deref(),
            self.duration_ms,
            self.cost_usd,
        );

        if self.is_error {
            let mut extra = self.base_extra(raw_kind);
            if let Some(kind) = &self.error_kind {
                extra.insert("error_kind".into(), Value::from(kind.as_str()));
            }
            self.sink.on_semantic_event(SemanticEvent::Error {
                message: self.error_message.clone().unwrap_or_default(),
                terminal: true,
                extra: Value::Object(extra),
            });
        } else {
            self.sink.on_semantic_event(SemanticEvent::TurnComplete {
                provider_status: self.provider_status.clone(),
                token_usage: step_usage,
                cost_usd: self.cost_usd,
                duration_ms: self.duration_ms,
                extra: Value::Object(self.base_extra(raw_kind)),
            });
        }
    }

    fn handle_error(&mut self, event: GeminiErrorEvent, raw_kind: &str) {
        self.error_kind = event.severity.clone();
        self.error_message = event.message.clone();

        let mut extra = self.base_extra(raw_kind);
        if let Some(kind) = &self.error_kind {
            extra.insert("severity".into(), Value::from(kind.as_str()));
        }
        let message = self.error_message.clone().unwrap_or_default();

        // Gemini's "error" event carries a `severity` field. Only non-warning
        // severities are terminal per the legacy parser's behavior.
        if self.error_kind.as_deref() == Some("warning") {
            self.sink.on_semantic_event(SemanticEvent::Warning {
                message,
                extra: Value::Object(extra),
            });
            return;
        }
        self.is_error = true;
        self.sink.on_semantic_event(SemanticEvent::Error {
            message,
            terminal: true,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_use(&mut self, mut tu: GeminiToolUse, raw_kind: &str) {
        self.tool_calls += 1;
        super::trace_tool_event(Provider::Gemini, self.tool_calls, tu.resolved_tool_name());

        let tool_id = tu.resolved_tool_id().map(String::from);
        let tool_name = tu.resolved_tool_name().map(String::from);
        let parameters = tu.take_input();

        if let Some(id) = &tool_id {
            self.tool_uses
                .insert(id.clone(), (tool_name.clone(), parameters.clone()));
        }

        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &tool_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &tool_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }

        self.sink.on_semantic_event(SemanticEvent::ToolCall {
            name: tool_name,
            id: tool_id,
            input: parameters,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_result(&mut self, tr: GeminiToolResult, raw_kind: &str) {
        let tool_id = tr.tool_id.clone();
        let (tool_name, _tool_input) = tool_id
            .as_ref()
            .and_then(|id| self.tool_uses.get(id).cloned())
            .unwrap_or((None, None));
        let (output, error, status) = tr.response();

        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &tool_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &tool_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        if let Some(err) = &error {
            extra.insert("error".into(), err.clone());
        }
        if let Some(s) = &status {
            extra.insert("status".into(), Value::from(s.as_str()));
        }

        self.sink.on_semantic_event(SemanticEvent::ToolResult {
            name: tool_name,
            id: tool_id,
            status,
            exit_code: None,
            output,
            extra: Value::Object(extra),
        });
    }

    fn emit_provider_extension(&mut self, kind: &str, payload: Value) {
        self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::Gemini,
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

impl<S: SemanticEventSink> SemanticStreamParser for GeminiSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }
        let raw: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                super::trace_malformed_line(Provider::Gemini, self.line_num, &e.to_string());
                self.emit_malformed_warning(&e.to_string());
                return Ok(());
            }
        };

        let raw_kind = raw
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        super::trace_parser_event(Provider::Gemini, &raw_kind, self.line_num);

        match serde_json::from_value::<GeminiEvent>(raw.clone()) {
            Ok(GeminiEvent::Init(init) | GeminiEvent::System(init)) => {
                self.handle_init(init, &raw_kind);
            }
            Ok(GeminiEvent::Message(msg)) => {
                self.handle_message(msg, &raw_kind, raw);
            }
            Ok(GeminiEvent::Error(err)) => {
                self.handle_error(err, &raw_kind);
            }
            Ok(GeminiEvent::Result(result)) => {
                self.handle_result(result, raw, &raw_kind);
            }
            Ok(GeminiEvent::ToolUse(tu)) => {
                self.handle_tool_use(tu, &raw_kind);
            }
            Ok(GeminiEvent::ToolResult(tr)) => {
                self.handle_tool_result(tr, &raw_kind);
            }
            Err(_) => {
                self.emit_provider_extension(&raw_kind, raw);
            }
        }
        Ok(())
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        let mut summary = StreamExecutionSummary {
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
            permission_prompts: None,
            user_input_prompts: None,
            rate_limit: None,
            context_usage: None,
            badges: Vec::new(),
            raw_summary: self.raw_summary,
            stderr_text: None,
        };
        summary.badges = crate::stream::badges::derive_badges(&summary, Provider::Gemini);
        summary
    }
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
        Box<GeminiSemanticStreamParser<Recording>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Recording {
            events: events.clone(),
        };
        (events, Box::new(GeminiSemanticStreamParser::new(sink)))
    }

    fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
        events.iter().map(|e| e.kind_str()).collect()
    }

    #[test]
    fn init_emits_session_start() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"gem-1","model":"gemini-2.5-pro"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::SessionStart { ref session_id, .. }
                if session_id.as_deref() == Some("gem-1")
        ));
    }

    #[test]
    fn assistant_message_emits_output_text() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"message","role":"assistant","content":"Hello"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::OutputText { ref text, .. } if text == "Hello\n"
        ));
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "Hello");
    }

    #[test]
    fn user_message_becomes_provider_extension() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"message","role":"user","content":"question"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::ProviderExtension { ref kind, .. } if kind == "message.non_assistant"
        ));
    }

    #[test]
    fn tool_use_and_result_emit_typed_events() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_use","tool_id":"t1","tool_name":"search","parameters":{"q":"rust"}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_result","tool_id":"t1","status":"success","output":{"hits":3}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(kinds(&collected), vec!["tool_call", "tool_result"]);
        match &collected[1] {
            SemanticEvent::ToolResult { name, id, status, output, .. } => {
                assert_eq!(name.as_deref(), Some("search"));
                assert_eq!(id.as_deref(), Some("t1"));
                assert_eq!(status.as_deref(), Some("success"));
                assert_eq!(*output, Some(json!({"hits": 3})));
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn error_severity_warning_emits_warning() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"error","severity":"warning","message":"Loop detected"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::Warning { ref message, .. } if message == "Loop detected"
        ));
        let summary = parser.finish(0);
        assert!(!summary.is_error);
    }

    #[test]
    fn error_fatal_severity_emits_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"error","severity":"fatal","message":"Catastrophe"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::Error { terminal: true, .. }
        ));
    }

    #[test]
    fn result_status_success_emits_turn_complete() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"result","status":"success","stats":{"input_tokens":500,"output_tokens":250,"cached":100,"duration_ms":8000,"tool_calls":2}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::TurnComplete {
                provider_status,
                duration_ms,
                token_usage,
                ..
            } => {
                assert_eq!(provider_status.as_deref(), Some("success"));
                assert_eq!(*duration_ms, Some(8000));
                let tu = token_usage.as_ref().unwrap();
                assert_eq!(tu.input, Some(500));
                assert_eq!(tu.output, Some(250));
                assert_eq!(tu.total, Some(750));
                assert_eq!(tu.cache_read, Some(100));
            }
            other => panic!("expected TurnComplete, got {other:?}"),
        }
        let summary = parser.finish(0);
        assert_eq!(summary.tool_calls, Some(2));
    }

    #[test]
    fn result_status_error_emits_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"result","status":"error","error":{"type":"FatalTurnLimited","message":"max turns"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(collected[0], SemanticEvent::Error { terminal: true, .. }));
        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("FatalTurnLimited"));
    }

    #[test]
    fn unknown_event_type_becomes_provider_extension() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"some_unknown","data":"x"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert!(matches!(
            collected[0],
            SemanticEvent::ProviderExtension { ref kind, .. } if kind == "some_unknown"
        ));
    }

    #[test]
    fn malformed_json_emits_warning() {
        let (events, mut parser) = new_parser();
        assert!(parser.feed_line("garbage").is_ok());
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::Warning { .. }
        ));
    }

    #[test]
    fn round_trip_fidelity_mixed_fixture() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"type":"init","session_id":"g","model":"m"}"#,
            r#"{"type":"message","role":"assistant","content":"hi"}"#,
            r#"{"type":"tool_use","tool_id":"t","tool_name":"s","parameters":{}}"#,
            r#"{"type":"tool_result","tool_id":"t","status":"success","output":"ok"}"#,
            r#"{"type":"error","severity":"warning","message":"loop"}"#,
            r#"{"type":"future.unknown","x":1}"#,
            r#"{"type":"result","status":"success","stats":{"duration_ms":1}}"#,
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
