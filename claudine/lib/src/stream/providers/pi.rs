//! Native [`SemanticStreamParser`] implementation for Pi's `--mode json`
//! NDJSON output.
//!
//! Pi is a bespoke (non-fork) provider, so this parser is authored from scratch
//! rather than reusing another provider's. The wire format is one JSON object
//! per line with a top-level `type` discriminator; `message_update` carries a
//! nested `assistantMessageEvent.type` for streaming text/thinking deltas.
//!
//! Routing:
//!
//! - `session` → [`SemanticEvent::SessionStart`] (session id + cwd).
//! - `message_update` → [`SemanticEvent::OutputText`] (`text_delta`) or
//!   [`SemanticEvent::Reasoning`] (`thinking_delta`); a nested `error` delta
//!   becomes [`SemanticEvent::Error`]. Block-boundary deltas
//!   (`*_start`/`*_end`/`done`) and model-side `toolcall_*` deltas are dropped —
//!   tool execution is reported by the `tool_execution_*` lifecycle instead.
//! - `message_end` → accumulates per-message usage/cost; a `stopReason` of
//!   `error`/`aborted` becomes a terminal-classified [`SemanticEvent::Error`].
//! - `tool_execution_start` → [`SemanticEvent::ToolCall`].
//! - `tool_execution_end` → [`SemanticEvent::ToolResult`] (status from
//!   `isError`; Pi does not normalize a top-level exit code).
//! - `tool_execution_update` → dropped (accumulated progress, not a delta).
//! - `agent_end` → [`SemanticEvent::TurnComplete`] (terminal record).
//! - `auto_retry_start` → [`SemanticEvent::Info`]; `auto_retry_end` with
//!   `success: false` → [`SemanticEvent::Error`].
//! - `compaction_end` with an `errorMessage` → [`SemanticEvent::Warning`].
//! - Other lifecycle events are dropped; unknown types →
//!   [`SemanticEvent::ProviderExtension`].

use serde_json::{Map, Value};

use super::parser::{SemanticStreamParser, StreamParseError};
use super::protocol::pi::{
    PiAssistantMessageEvent, PiAutoRetryEnd, PiAutoRetryStart, PiCompactionEnd, PiEvent,
    PiMessageEnvelope, PiSession, PiToolEnd, PiToolStart,
};
use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::provider_id::Provider;

pub struct PiSemanticStreamParser<S: SemanticEventSink> {
    sink: S,
    line_num: usize,
    session_id: Option<String>,
    model: Option<String>,
    assistant_text: String,
    token_usage: NormalizedTokenUsage,
    cost_usd: f64,
    num_turns: u32,
    tool_calls: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
}

impl<S: SemanticEventSink> PiSemanticStreamParser<S> {
    pub fn new(sink: S, model: Option<String>) -> Self {
        Self {
            sink,
            line_num: 0,
            session_id: None,
            model,
            assistant_text: String::new(),
            token_usage: NormalizedTokenUsage::default(),
            cost_usd: 0.0,
            num_turns: 0,
            tool_calls: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
        }
    }

    fn base_extra(&self, raw_kind: &str) -> Map<String, Value> {
        super::common::base_extra(Provider::Pi, self.line_num, raw_kind)
    }

    fn handle_session(&mut self, session: PiSession, raw_kind: &str) {
        self.session_id = session.id;
        super::trace_session_metadata(
            Provider::Pi,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );
        let mut extra = self.base_extra(raw_kind);
        if let Some(cwd) = &session.cwd {
            extra.insert("cwd".into(), Value::from(cwd.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::SessionStart {
            session_id: self.session_id.clone(),
            model: self.model.clone(),
            extra: Value::Object(extra),
        });
    }

    fn handle_message_update(&mut self, update: PiAssistantMessageEvent, raw_kind: &str) {
        let event_type = update.event_type.as_deref().unwrap_or("");
        match event_type {
            "text_delta" => {
                let Some(text) = update.delta else { return };
                if text.is_empty() {
                    return;
                }
                self.assistant_text.push_str(&text);
                self.sink.on_semantic_event(SemanticEvent::OutputText {
                    text,
                    extra: Value::Object(self.base_extra(raw_kind)),
                });
            }
            "thinking_delta" => {
                let Some(text) = update.delta else { return };
                if text.is_empty() {
                    return;
                }
                self.sink.on_semantic_event(SemanticEvent::Reasoning {
                    text,
                    extra: Value::Object(self.base_extra(raw_kind)),
                });
            }
            "error" => {
                let message = update
                    .error_message
                    .unwrap_or_else(|| "assistant stream error".to_string());
                self.record_error(&message, raw_kind);
            }
            // start/text_start/text_end/thinking_start/thinking_end/done and the
            // model-side toolcall_* deltas carry no user-visible content; tool
            // execution is reported by the tool_execution_* lifecycle.
            _ => {}
        }
    }

    fn handle_message_end(&mut self, envelope: PiMessageEnvelope, raw_kind: &str) {
        let Some(message) = envelope.message else {
            return;
        };
        if let Some(model) = message.model {
            self.model = Some(model);
        }
        if let Some(usage) = message.usage {
            let step = NormalizedTokenUsage {
                input: usage.input,
                output: usage.output,
                total: usage.total_tokens,
                cache_read: usage.cache_read,
            };
            self.token_usage.accumulate(&step);
            if let Some(cost) = usage.cost.and_then(|c| c.total) {
                self.cost_usd += cost;
            }
        }
        if let Some(reason) = message.stop_reason {
            let is_failure = reason == "error" || reason == "aborted";
            self.provider_status = Some(reason);
            if is_failure {
                let message = message
                    .error_message
                    .unwrap_or_else(|| "assistant message ended in error".to_string());
                self.record_error(&message, raw_kind);
            }
        }
    }

    fn record_error(&mut self, message: &str, raw_kind: &str) {
        self.is_error = true;
        self.error_message = Some(message.to_string());
        let kind = classify_error(message);
        self.error_kind = Some(semantic_kind_label(kind).to_string());
        let mut extra = self.base_extra(raw_kind);
        if let Some(k) = &self.error_kind {
            extra.insert("error_kind".into(), Value::from(k.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::Error {
            message: message.to_string(),
            // `agent_end` is Pi's terminal record; an assistant-message error is
            // not itself terminal (a retry may follow), matching the summary's
            // is_error flag without claiming the stream ended.
            terminal: false,
            kind,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_start(&mut self, tool: PiToolStart, raw_kind: &str) {
        self.tool_calls += 1;
        super::trace_tool_event(Provider::Pi, self.tool_calls, tool.tool_name.as_deref());
        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &tool.tool_call_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &tool.tool_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        self.sink.on_semantic_event(SemanticEvent::ToolCall {
            name: tool.tool_name,
            id: tool.tool_call_id,
            input: tool.args,
            extra: Value::Object(extra),
        });
    }

    fn handle_tool_end(&mut self, tool: PiToolEnd, raw_kind: &str) {
        let status = match tool.is_error {
            Some(true) => "error",
            _ => "success",
        };
        let mut extra = self.base_extra(raw_kind);
        if let Some(id) = &tool.tool_call_id {
            extra.insert("tool_id".into(), Value::from(id.as_str()));
        }
        if let Some(name) = &tool.tool_name {
            extra.insert("tool_name".into(), Value::from(name.as_str()));
        }
        extra.insert("status".into(), Value::from(status));
        self.sink.on_semantic_event(SemanticEvent::ToolResult {
            name: tool.tool_name,
            id: tool.tool_call_id,
            status: Some(status.to_string()),
            exit_code: None,
            output: tool.result,
            extra: Value::Object(extra),
        });
    }

    fn handle_agent_end(&mut self, raw_kind: &str) {
        self.num_turns += 1;
        super::trace_summary_update(
            Provider::Pi,
            self.provider_status.as_deref(),
            None,
            Some(self.cost_usd),
        );
        let has_usage = self.token_usage.input.is_some() || self.token_usage.output.is_some();
        self.sink.on_semantic_event(SemanticEvent::TurnComplete {
            provider_status: self.provider_status.clone(),
            token_usage: has_usage.then(|| self.token_usage.clone()),
            cost_usd: (self.cost_usd > 0.0).then_some(self.cost_usd),
            duration_ms: None,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_auto_retry_start(&mut self, retry: PiAutoRetryStart, raw_kind: &str) {
        let attempt = retry.attempt.unwrap_or(0);
        let max = retry.max_attempts.unwrap_or(0);
        let reason = retry.error_message.unwrap_or_default();
        let message = if reason.is_empty() {
            format!("auto-retry (attempt {attempt}/{max})")
        } else {
            format!("auto-retry (attempt {attempt}/{max}): {reason}")
        };
        self.sink.on_semantic_event(SemanticEvent::Info {
            message,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn handle_auto_retry_end(&mut self, retry: PiAutoRetryEnd, raw_kind: &str) {
        if retry.success == Some(false) {
            let message = retry
                .final_error
                .unwrap_or_else(|| "auto-retry exhausted".to_string());
            self.record_error(&message, raw_kind);
        }
    }

    fn handle_compaction_end(&mut self, compaction: PiCompactionEnd, raw_kind: &str) {
        if let Some(message) = compaction.error_message {
            self.sink.on_semantic_event(SemanticEvent::Warning {
                message: format!("compaction failed: {message}"),
                extra: Value::Object(self.base_extra(raw_kind)),
            });
        }
    }

    fn emit_provider_extension(&mut self, kind: &str, payload: Value) {
        super::common::emit_provider_extension(&mut self.sink, Provider::Pi, kind, payload);
    }

    fn emit_malformed_warning(&mut self, err: &str) {
        super::common::emit_malformed_warning(&mut self.sink, Provider::Pi, self.line_num, err);
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for PiSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        // Pi's RPC docs warn clients to split on LF only; feed_line already
        // receives a single LF-delimited record, so trimming trailing CR/space
        // is all that remains.
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        match serde_json::from_str::<PiEvent>(line) {
            Ok(event) => {
                let raw_kind = event.type_str();
                super::trace_parser_event(Provider::Pi, raw_kind, self.line_num);
                match event {
                    PiEvent::Session(session) => self.handle_session(session, raw_kind),
                    PiEvent::MessageUpdate(update) => {
                        if let Some(delta) = update.assistant_message_event {
                            self.handle_message_update(delta, raw_kind);
                        }
                    }
                    PiEvent::MessageEnd(envelope) => self.handle_message_end(envelope, raw_kind),
                    PiEvent::ToolExecutionStart(tool) => self.handle_tool_start(tool, raw_kind),
                    PiEvent::ToolExecutionEnd(tool) => self.handle_tool_end(tool, raw_kind),
                    PiEvent::AgentEnd(_) => self.handle_agent_end(raw_kind),
                    PiEvent::AutoRetryStart(retry) => self.handle_auto_retry_start(retry, raw_kind),
                    PiEvent::AutoRetryEnd(retry) => self.handle_auto_retry_end(retry, raw_kind),
                    PiEvent::CompactionEnd(compaction) => {
                        self.handle_compaction_end(compaction, raw_kind)
                    }
                    // Recognized-but-silent lifecycle events.
                    PiEvent::AgentStart(_)
                    | PiEvent::TurnStart(_)
                    | PiEvent::MessageStart(_)
                    | PiEvent::ToolExecutionUpdate(_)
                    | PiEvent::TurnEnd(_)
                    | PiEvent::CompactionStart(_)
                    | PiEvent::QueueUpdate(_)
                    | PiEvent::EntryAppended(_)
                    | PiEvent::SessionInfoChanged(_)
                    | PiEvent::ThinkingLevelChanged(_) => {}
                }
            }
            Err(_) => {
                let raw: Map<String, Value> = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        super::trace_malformed_line(Provider::Pi, self.line_num, &e.to_string());
                        self.emit_malformed_warning(&e.to_string());
                        return Ok(());
                    }
                };
                let raw_kind = raw
                    .get("type")
                    .and_then(Value::as_str)
                    .unwrap_or("")
                    .to_string();
                super::trace_parser_event(Provider::Pi, &raw_kind, self.line_num);
                self.emit_provider_extension(&raw_kind, Value::Object(raw));
            }
        }
        Ok(())
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        super::trace_parser_finish(
            Provider::Pi,
            exit_code,
            self.tool_calls,
            self.num_turns,
            self.provider_status.as_deref(),
        );
        let has_usage = self.token_usage.input.is_some() || self.token_usage.output.is_some();
        super::common::finish_summary(
            Provider::Pi,
            StreamExecutionSummary {
                session_id: self.session_id,
                model: self.model,
                assistant_text: self.assistant_text,
                provider_status: self.provider_status,
                exit_code,
                is_error: self.is_error,
                error_kind: self.error_kind,
                error_message: self.error_message,
                num_turns: (self.num_turns > 0).then_some(self.num_turns),
                token_usage: has_usage.then_some(self.token_usage),
                cost_usd: (self.cost_usd > 0.0).then_some(self.cost_usd),
                tool_calls: (self.tool_calls > 0).then_some(self.tool_calls),
                ..Default::default()
            },
        )
    }
}

/// Classify a Pi assistant/error message string into a typed
/// [`SemanticErrorKind`].
///
/// Pi normalizes provider failures into free-text `errorMessage` strings with no
/// structured category, so classification is text-based (mirroring OpenCode's
/// message-fallback path).
fn classify_error(message: &str) -> SemanticErrorKind {
    super::common::classify_error_by_keywords(
        super::vocabulary::error_keywords(Provider::Pi),
        None,
        None,
        Some(message),
    )
}

/// Stable snake_case label for a [`SemanticErrorKind`], carried in the summary's
/// `error_kind` field.
fn semantic_kind_label(kind: SemanticErrorKind) -> &'static str {
    match kind {
        SemanticErrorKind::Configuration => "configuration",
        SemanticErrorKind::AgentNative => "agent_native",
        SemanticErrorKind::ApiRemote => "api_remote",
        SemanticErrorKind::Interrupted => "interrupted",
        SemanticErrorKind::Unknown => "unknown",
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
        Box<PiSemanticStreamParser<Recording>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Recording {
            events: events.clone(),
        };
        (
            events,
            Box::new(PiSemanticStreamParser::new(sink, Some("claude-opus-4-8".into()))),
        )
    }

    fn kinds(events: &[SemanticEvent]) -> Vec<&'static str> {
        events.iter().map(|e| e.kind_str()).collect()
    }

    #[test]
    fn session_header_emits_session_start_with_cwd() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"session","version":3,"id":"s-1","cwd":"/work"}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::SessionStart {
                session_id,
                model,
                extra,
            } => {
                assert_eq!(session_id.as_deref(), Some("s-1"));
                assert_eq!(model.as_deref(), Some("claude-opus-4-8"));
                assert_eq!(extra.get("cwd"), Some(&Value::from("/work")));
            }
            other => panic!("expected SessionStart, got {other:?}"),
        }
    }

    #[test]
    fn text_delta_emits_output_text_and_accumulates() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"Hello "}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"world"}}"#,
            )
            .unwrap();
        assert_eq!(
            kinds(&events.lock().unwrap()),
            vec!["output_text", "output_text"]
        );
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "Hello world");
    }

    #[test]
    fn thinking_delta_emits_reasoning() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_delta","delta":"weighing options"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::Reasoning { text, .. } => assert_eq!(text, "weighing options"),
            other => panic!("expected Reasoning, got {other:?}"),
        }
    }

    #[test]
    fn block_boundary_and_toolcall_deltas_are_silent() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"type":"message_update","assistantMessageEvent":{"type":"start"}}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"text_start"}}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"text_end"}}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"toolcall_delta","delta":"{\"a\":1}"}}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"done"}}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        assert!(
            events.lock().unwrap().is_empty(),
            "boundary/toolcall deltas must not emit; got {:?}",
            events.lock().unwrap()
        );
    }

    #[test]
    fn tool_execution_lifecycle_emits_call_then_result() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_execution_start","toolCallId":"t1","toolName":"bash","args":{"command":"ls"}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"tool_execution_end","toolCallId":"t1","toolName":"bash","result":"file.txt","isError":false}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(kinds(&collected), vec!["tool_call", "tool_result"]);
        match &collected[0] {
            SemanticEvent::ToolCall { name, id, input, .. } => {
                assert_eq!(name.as_deref(), Some("bash"));
                assert_eq!(id.as_deref(), Some("t1"));
                assert_eq!(
                    input.as_ref().and_then(|v| v.get("command")).and_then(Value::as_str),
                    Some("ls")
                );
            }
            other => panic!("expected ToolCall, got {other:?}"),
        }
        match &collected[1] {
            SemanticEvent::ToolResult { status, output, .. } => {
                assert_eq!(status.as_deref(), Some("success"));
                assert_eq!(output.as_ref().and_then(Value::as_str), Some("file.txt"));
            }
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn tool_error_marks_result_error_status() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_execution_end","toolCallId":"t9","toolName":"bash","result":"boom","isError":true}"#,
            )
            .unwrap();
        match &events.lock().unwrap()[0] {
            SemanticEvent::ToolResult { status, .. } => assert_eq!(status.as_deref(), Some("error")),
            other => panic!("expected ToolResult, got {other:?}"),
        }
    }

    #[test]
    fn tool_execution_update_is_silent() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"tool_execution_update","toolCallId":"t1","partialResult":"partial output so far"}"#,
            )
            .unwrap();
        assert!(
            events.lock().unwrap().is_empty(),
            "accumulated progress must not emit a delta"
        );
    }

    #[test]
    fn message_end_accumulates_usage_and_cost() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":1200,"output":150,"cacheRead":300,"totalTokens":1650,"cost":{"total":0.00594}}}}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":100,"output":50,"totalTokens":150,"cost":{"total":0.001}}}}"#,
            )
            .unwrap();
        // message_end alone emits nothing user-visible.
        assert!(events.lock().unwrap().is_empty());
        let summary = parser.finish(0);
        let usage = summary.token_usage.expect("usage accumulated");
        assert_eq!(usage.input, Some(1300));
        assert_eq!(usage.output, Some(200));
        assert_eq!(usage.total, Some(1800));
        assert_eq!(usage.cache_read, Some(300));
        assert!((summary.cost_usd.unwrap() - 0.00694).abs() < 1e-9);
    }

    #[test]
    fn message_end_error_stop_reason_emits_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"message_end","message":{"stopReason":"error","errorMessage":"Provider returned error: 503 service unavailable"}}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        match &collected[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                ..
            } => {
                assert!(!terminal, "an assistant-message error is not the terminal record");
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert!(message.contains("503"));
            }
            other => panic!("expected Error, got {other:?}"),
        }
        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("api_remote"));
    }

    #[test]
    fn agent_end_emits_terminal_turn_complete() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":10,"output":5,"totalTokens":15}}}"#,
            )
            .unwrap();
        parser
            .feed_line(r#"{"type":"agent_end","willRetry":false}"#)
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(kinds(&collected), vec!["turn_complete"]);
        match &collected[0] {
            SemanticEvent::TurnComplete {
                provider_status,
                token_usage,
                ..
            } => {
                assert_eq!(provider_status.as_deref(), Some("stop"));
                assert_eq!(token_usage.as_ref().and_then(|u| u.input), Some(10));
            }
            other => panic!("expected TurnComplete, got {other:?}"),
        }
        let summary = parser.finish(0);
        assert_eq!(summary.num_turns, Some(1));
    }

    #[test]
    fn auto_retry_start_emits_info_and_end_failure_emits_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"auto_retry_start","attempt":3,"maxAttempts":3,"errorMessage":"503 service unavailable"}"#,
            )
            .unwrap();
        parser
            .feed_line(
                r#"{"type":"auto_retry_end","success":false,"attempt":3,"finalError":"provider overloaded"}"#,
            )
            .unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(kinds(&collected), vec!["info", "error"]);
        match &collected[0] {
            SemanticEvent::Info { message, .. } => assert!(message.contains("attempt 3/3")),
            other => panic!("expected Info, got {other:?}"),
        }
        assert!(parser.finish(1).is_error);
    }

    #[test]
    fn successful_auto_retry_end_is_silent() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"auto_retry_end","success":true,"attempt":2}"#)
            .unwrap();
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn failed_compaction_emits_warning() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"compaction_end","result":null,"aborted":false,"errorMessage":"compaction model unavailable"}"#,
            )
            .unwrap();
        match &events.lock().unwrap()[0] {
            SemanticEvent::Warning { message, .. } => assert!(message.contains("compaction failed")),
            other => panic!("expected Warning, got {other:?}"),
        }
    }

    #[test]
    fn silent_lifecycle_events_emit_nothing() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"type":"agent_start"}"#,
            r#"{"type":"turn_start"}"#,
            r#"{"type":"message_start"}"#,
            r#"{"type":"turn_end","message":{}}"#,
            r#"{"type":"compaction_start"}"#,
            r#"{"type":"queue_update"}"#,
            r#"{"type":"entry_appended"}"#,
            r#"{"type":"session_info_changed"}"#,
            r#"{"type":"thinking_level_changed","level":"high"}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        assert!(
            events.lock().unwrap().is_empty(),
            "recognized lifecycle events must be silent; got {:?}",
            events.lock().unwrap()
        );
    }

    #[test]
    fn unknown_event_becomes_provider_extension() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"some_future_event","x":1}"#)
            .unwrap();
        match &events.lock().unwrap()[0] {
            SemanticEvent::ProviderExtension { provider, kind, .. } => {
                assert_eq!(*provider, Provider::Pi);
                assert_eq!(kind, "some_future_event");
            }
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_emits_warning() {
        let (events, mut parser) = new_parser();
        assert!(parser.feed_line("not json").is_ok());
        assert!(matches!(
            events.lock().unwrap()[0],
            SemanticEvent::Warning { .. }
        ));
    }

    #[test]
    fn blank_lines_are_skipped() {
        let (events, mut parser) = new_parser();
        parser.feed_line("").unwrap();
        parser.feed_line("   ").unwrap();
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn round_trip_serialization_fidelity() {
        let (events, mut parser) = new_parser();
        for line in [
            r#"{"type":"session","id":"s","cwd":"/w"}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"thinking_delta","delta":"hmm"}}"#,
            r#"{"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"hi"}}"#,
            r#"{"type":"tool_execution_start","toolCallId":"t","toolName":"read","args":{"path":"a"}}"#,
            r#"{"type":"tool_execution_end","toolCallId":"t","toolName":"read","result":"x","isError":false}"#,
            r#"{"type":"message_end","message":{"stopReason":"stop","usage":{"input":1,"output":2,"totalTokens":3,"cost":{"total":0.01}}}}"#,
            r#"{"type":"agent_end","willRetry":false}"#,
            r#"{"type":"future.kind","x":1}"#,
        ] {
            parser.feed_line(line).unwrap();
        }
        for event in events.lock().unwrap().iter() {
            let v = serde_json::to_value(event).unwrap();
            let decoded: SemanticEvent = serde_json::from_value(v.clone()).unwrap();
            assert_eq!(v, serde_json::to_value(&decoded).unwrap());
        }
        let summary = parser.finish(0);
        assert_eq!(summary.assistant_text, "hi");
        assert_eq!(summary.tool_calls, Some(1));
        assert_eq!(summary.num_turns, Some(1));
    }

    #[test]
    fn classify_error_covers_categories() {
        assert_eq!(classify_error("rate limit exceeded"), SemanticErrorKind::ApiRemote);
        assert_eq!(classify_error("no API key found for anthropic"), SemanticErrorKind::Configuration);
        assert_eq!(classify_error("run aborted by user"), SemanticErrorKind::Interrupted);
        assert_eq!(classify_error("something odd"), SemanticErrorKind::AgentNative);
    }
}
