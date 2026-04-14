//! Native [`SemanticStreamParser`] implementation for Kimi Code's
//! `stream-json` format.
//!
//! Kimi emits no aggregate final `result` event; its summary is derived from
//! `StatusUpdate` snapshots plus the child exit code. Context-window pressure
//! warnings are surfaced when usage crosses [`CONTEXT_PRESSURE_WARN_PERCENT`].

use std::collections::HashMap;

use serde_json::{Map, Value};

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::kimi::{
    KimiContent, KimiErrorEvent, KimiEvent, KimiInit, KimiStatusUpdate, KimiTool,
};
use super::semantic::{SemanticEvent, SemanticEventSink};
use super::summary::{ContextUsage, StreamExecutionSummary};
use super::token_usage::NormalizedTokenUsage;
use crate::events::Provider;

const CONTEXT_PRESSURE_WARN_PERCENT: f64 = 80.0;

pub struct KimiSemanticStreamParser<S: SemanticEventSink> {
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

impl<S: SemanticEventSink> KimiSemanticStreamParser<S> {
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

    fn base_extra(&self, raw_kind: &str) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("provider".into(), Value::from("kimi"));
        m.insert("line_num".into(), Value::from(self.line_num));
        m.insert("raw_kind".into(), Value::from(raw_kind));
        m
    }

    fn handle_init(&mut self, init: KimiInit, raw_kind: &str) {
        self.session_id = init.session_id;
        self.model = init.model;
        super::trace_session_metadata(
            Provider::KimiCode,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_content(&mut self, event: KimiContent, raw_kind: &str) {
        let Some(text) = event.resolved_text() else {
            return;
        };
        if text.is_empty() {
            return;
        }
        self.assistant_text.push_str(&text);
        self.sink.on_semantic_event(SemanticEvent::OutputText {
            text: super::ensure_message_newline(text),
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_status_update(&mut self, event: KimiStatusUpdate, raw_kind: &str) {
        let mut event = event;
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
                let mut extra = self.base_extra(raw_kind);
                extra.insert("percent".into(), Value::from(pct));
                extra.insert("used".into(), Value::from(used.unwrap_or(0)));
                extra.insert("total".into(), Value::from(total.unwrap_or(0)));
                self.sink.on_semantic_event(SemanticEvent::Warning {
                    message: format!(
                        "Context window pressure: {pct:.0}% used ({}/{} tokens)",
                        used.unwrap_or(0),
                        total.unwrap_or(0)
                    ),
                    extra: Value::Object(extra),
                });
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

    fn handle_error(&mut self, event: KimiErrorEvent, raw_kind: &str) {
        self.is_error = true;
        self.error_kind = event.resolved_kind();
        self.error_message = event.resolved_message();

        let mut extra = self.base_extra(raw_kind);
        if let Some(kind) = &self.error_kind {
            extra.insert("error_kind".into(), Value::from(kind.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::Error {
            message: self.error_message.clone().unwrap_or_default(),
            terminal: true,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_use(&mut self, mut tool: KimiTool, raw_kind: &str) {
        self.tool_calls += 1;
        let tool_id = tool.resolved_tool_id().map(String::from);
        let tool_name = tool.resolved_tool_name().map(String::from);
        let tool_input = tool.take_input();
        super::trace_tool_event(Provider::KimiCode, self.tool_calls, tool_name.as_deref());

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

    fn handle_tool_result(&mut self, mut tool: KimiTool, raw_kind: &str) {
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
        self.sink.on_semantic_event(SemanticEvent::ProviderExtension {
            provider: Provider::KimiCode,
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

impl<S: SemanticEventSink> SemanticStreamParser for KimiSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }
        let raw: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => {
                super::trace_malformed_line(Provider::KimiCode, self.line_num, &e.to_string());
                self.emit_malformed_warning(&e.to_string());
                return Ok(());
            }
        };
        let raw_kind = raw
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        super::trace_parser_event(Provider::KimiCode, &raw_kind, self.line_num);

        match serde_json::from_value::<KimiEvent>(raw.clone()) {
            Ok(KimiEvent::Init(init) | KimiEvent::System(init)) => {
                self.handle_init(init, &raw_kind);
            }
            Ok(
                KimiEvent::Assistant(content)
                | KimiEvent::Message(content)
                | KimiEvent::Content(content)
                | KimiEvent::ContentPart(content),
            ) => {
                self.handle_content(content, &raw_kind);
            }
            Ok(
                KimiEvent::StatusUpdatePascal(status)
                | KimiEvent::StatusUpdate(status)
                | KimiEvent::Status(status),
            ) => {
                self.handle_status_update(status, &raw_kind);
            }
            Ok(KimiEvent::Error(err)) => {
                self.handle_error(err, &raw_kind);
            }
            Ok(KimiEvent::ToolUse(tool)) => {
                self.handle_tool_use(tool, &raw_kind);
            }
            Ok(KimiEvent::ToolResult(tool)) => {
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
            permission_prompts: None,
            user_input_prompts: None,
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
        Box<KimiSemanticStreamParser<Recording>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Recording {
            events: events.clone(),
        };
        (events, Box::new(KimiSemanticStreamParser::new(sink)))
    }

    fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
        events.iter().map(|e| e.kind_str()).collect()
    }

    #[test]
    fn init_emits_session_start() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"k1","model":"kimi-coder"}"#)
            .unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::SessionStart { .. }
        ));
    }

    #[test]
    fn assistant_content_emits_output_text() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"assistant","content":[{"text":"Hello from Kimi"}]}"#)
            .unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::OutputText { ref text, .. } if text == "Hello from Kimi\n"
        ));
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "Hello from Kimi");
    }

    #[test]
    fn context_pressure_emits_warning() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"StatusUpdate","context_usage":{"used":110000,"total":128000}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::Warning { message, extra } => {
                assert!(message.contains("Context window pressure"));
                assert_eq!(extra.get("used"), Some(&Value::from(110000u64)));
            }
            other => panic!("expected Warning, got {other:?}"),
        }
    }

    #[test]
    fn context_below_threshold_emits_nothing() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"StatusUpdate","context_usage":{"used":50000,"total":128000}}"#,
            )
            .unwrap();
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn tool_use_and_result_emit_typed_events() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"tool_use","id":"k1","name":"bash","input":{"cmd":"ls"}}"#)
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_result","tool_use_id":"k1","status":"success","content":"clean"}"#,
            )
            .unwrap();
        assert_eq!(kinds(&events.lock().unwrap()), vec!["tool_call", "tool_result"]);
    }

    #[test]
    fn error_emits_terminal_error() {
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
        parser.feed_line(r#"{"type":"future.unknown"}"#).unwrap();
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::ProviderExtension { .. }
        ));
    }

    #[test]
    fn malformed_json_emits_warning() {
        let (events, mut parser) = new_parser();
        assert!(parser.feed_line("x").is_ok());
        assert!(matches!(events.lock().unwrap()[0], SemanticEvent::Warning { .. }));
    }

    #[test]
    fn round_trip_fidelity_mixed_fixture() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"type":"init","session_id":"k","model":"m"}"#,
            r#"{"type":"assistant","content":[{"text":"hi"}]}"#,
            r#"{"type":"tool_use","id":"t","name":"b","input":{}}"#,
            r#"{"type":"tool_result","tool_use_id":"t","status":"success","content":"ok"}"#,
            r#"{"type":"StatusUpdate","usage":{"input_tokens":1,"output_tokens":2}}"#,
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
