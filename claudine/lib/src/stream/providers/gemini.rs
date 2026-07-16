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
use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::provider_id::Provider;
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
    /// Accumulates streaming `delta: true` assistant text. Flushed on
    /// paragraph boundaries (`\n\n`), on any non-text event, and at
    /// `finish`. See Task 0c investigation for the flush-rule contract.
    pending_text: String,
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
            pending_text: String::new(),
        }
    }

    fn base_extra(&self, raw_kind: &str) -> Map<String, Value> {
        super::common::base_extra(Provider::Gemini, self.line_num, raw_kind)
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

    fn handle_message(&mut self, msg: GeminiMessage, raw_kind: &str) {
        if msg.role.as_deref() != Some("assistant") {
            // Gemini replays the operator's own prompt (role=user) and
            // occasional system messages back into the stream. These are
            // always noise on stderr, so drop silently here. Fidelity is
            // preserved via the raw JSONL log path, which writes each raw
            // line independently of the semantic event surface.
            return;
        }
        let is_delta = msg.delta.unwrap_or(false);
        let Some(text) = msg.resolved_text() else {
            return;
        };

        if is_delta {
            // Streaming delta: append to the pending buffer and flush on
            // paragraph boundaries or list-item boundaries. Per Task 0c,
            // code-fence handling is deferred to the renderer.
            self.pending_text.push_str(&text);
            while let Some(idx) = self.pending_text.find("\n\n") {
                let flush_upto = idx + 2;
                let chunk: String = self.pending_text.drain(..flush_upto).collect();
                self.emit_output_text_chunk(chunk, raw_kind);
            }
            // Flush on list-item boundaries to improve perceived
            // responsiveness during long lists (e.g. "- Item 1\n- Item 2").
            if let Some(pos) = find_list_flush_position(&self.pending_text) {
                let chunk: String = self.pending_text.drain(..pos).collect();
                self.emit_output_text_chunk(chunk, raw_kind);
            }
        } else {
            // Non-delta (single-shot) message: flush any buffered content
            // first, then emit this message immediately with the usual
            // trailing-newline normalization. Track the raw (unnormalized)
            // text in `assistant_text` for summary fidelity.
            self.flush_pending_text(raw_kind);
            self.assistant_text.push_str(&text);
            let normalized = super::ensure_message_newline(text);
            self.sink.on_semantic_event(SemanticEvent::OutputText {
                text: normalized,
                extra: Value::Object(self.base_extra(raw_kind)),
            });
        }
    }

    fn emit_output_text_chunk(&mut self, chunk: String, raw_kind: &str) {
        if chunk.is_empty() {
            return;
        }
        self.assistant_text.push_str(&chunk);
        self.sink.on_semantic_event(SemanticEvent::OutputText {
            text: chunk,
            extra: Value::Object(self.base_extra(raw_kind)),
        });
    }

    fn flush_pending_text(&mut self, raw_kind: &str) {
        if !self.pending_text.is_empty() {
            let drained = std::mem::take(&mut self.pending_text);
            self.emit_output_text_chunk(drained, raw_kind);
        }
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
            let semantic_kind =
                classify_error(self.error_kind.as_deref(), self.error_message.as_deref());
            self.sink.on_semantic_event(SemanticEvent::Error {
                message: self.error_message.clone().unwrap_or_default(),
                terminal: true,
                kind: semantic_kind,
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
        let semantic_kind = classify_error(self.error_kind.as_deref(), Some(&message));
        self.sink.on_semantic_event(SemanticEvent::Error {
            message,
            terminal: true,
            kind: semantic_kind,
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
        super::common::emit_provider_extension(&mut self.sink, Provider::Gemini, kind, payload);
    }

    fn emit_malformed_warning(&mut self, err: &str) {
        super::common::emit_malformed_warning(&mut self.sink, Provider::Gemini, self.line_num, err);
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for GeminiSemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) -> Result<(), StreamParseError> {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return Ok(());
        }

        // Try typed deserialization first to avoid `serde_json::Value` DOM
        // allocation on the hot path. Fall back to `Value` only for unknown
        // event types that must be preserved as `ProviderExtension`, or for
        // `result` events that need the raw payload for `raw_summary`.
        match serde_json::from_str::<GeminiEvent>(line) {
            Ok(event) => {
                let raw_kind = event.type_str().to_string();
                super::trace_parser_event(Provider::Gemini, &raw_kind, self.line_num);

                // Any event other than a streaming assistant `message` is a
                // logical break in the text stream: flush the delta buffer so
                // buffered prose is rendered before the next semantic event.
                let is_streaming_message = matches!(
                    &event,
                    GeminiEvent::Message(m)
                        if m.role.as_deref() == Some("assistant") && m.delta.unwrap_or(false)
                );
                if !is_streaming_message {
                    self.flush_pending_text(&raw_kind);
                }

                match event {
                    GeminiEvent::Init(init) | GeminiEvent::System(init) => {
                        self.handle_init(init, &raw_kind);
                    }
                    GeminiEvent::Message(msg) => {
                        self.handle_message(msg, &raw_kind);
                    }
                    GeminiEvent::Error(err) => {
                        self.handle_error(err, &raw_kind);
                    }
                    GeminiEvent::Result(result) => {
                        // Reconstruct the raw payload from the typed struct
                        // without a second parse.
                        let raw = serde_json::to_value(&result).expect("GeminiResult serializes");
                        self.handle_result(result, raw, &raw_kind);
                    }
                    GeminiEvent::ToolUse(tu) => {
                        self.handle_tool_use(tu, &raw_kind);
                    }
                    GeminiEvent::ToolResult(tr) => {
                        self.handle_tool_result(tr, &raw_kind);
                    }
                }
            }
            Err(_) => {
                let raw: Map<String, Value> = match serde_json::from_str(line) {
                    Ok(v) => v,
                    Err(e) => {
                        super::trace_malformed_line(
                            Provider::Gemini,
                            self.line_num,
                            &e.to_string(),
                        );
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
                self.flush_pending_text(&raw_kind);
                self.emit_provider_extension(&raw_kind, Value::Object(raw));
            }
        }
        Ok(())
    }

    fn finish(mut self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        self.flush_pending_text("gemini_finish");
        super::common::finish_summary(
            Provider::Gemini,
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

/// Finds the flush position for list-item boundaries in streaming text.
///
/// Scans for newlines followed by Markdown list markers (`- `, `* `, `+ `,
/// or numbered like `1. `) and returns the byte offset just after the last
/// such newline. This allows the caller to drain completed list lines while
/// keeping the newest (potentially incomplete) list item buffered.
fn find_list_flush_position(text: &str) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut last_boundary = None;

    for i in 1..bytes.len() {
        if bytes[i - 1] != b'\n' {
            continue;
        }
        let rest = &text[i..];
        if rest.starts_with("- ")
            || rest.starts_with("* ")
            || rest.starts_with("+ ")
            || is_numbered_list_start(rest)
        {
            last_boundary = Some(i);
        }
    }

    last_boundary
}

fn is_numbered_list_start(s: &str) -> bool {
    let digits = s.bytes().take_while(|b| b.is_ascii_digit()).count();
    digits > 0 && s[digits..].starts_with(". ")
}

/// Map a Gemini error envelope onto a typed [`SemanticErrorKind`].
///
/// Gemini errors carry a free-form `severity` field plus a message. This
/// helper inspects both so the live error renderer and the end-of-run
/// report can pick a consistent label and color.
fn classify_error(error_kind: Option<&str>, message: Option<&str>) -> SemanticErrorKind {
    super::common::classify_error_by_keywords(
        super::vocabulary::error_keywords(Provider::Gemini),
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
    fn gemini_non_assistant_message_emits_no_provider_extension() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(
                r#"{"type":"message","content":"Hi how are you?","role":"user","timestamp":"2026-04-14T00:00:00Z"}"#,
            )
            .unwrap();

        let captured = events.lock().unwrap().clone();
        assert!(
            !captured.iter().any(|e| matches!(
                e,
                SemanticEvent::ProviderExtension { kind, .. } if kind == "message.non_assistant"
            )),
            "non-assistant messages must be dropped silently, got {captured:?}"
        );
        assert!(
            captured.is_empty(),
            "no semantic events should be emitted for user-role messages, got {captured:?}"
        );
    }

    #[test]
    fn gemini_assistant_message_still_routes_to_output_text() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"message","content":"response text","role":"assistant"}"#)
            .unwrap();

        let captured = events.lock().unwrap().clone();
        assert!(
            captured
                .iter()
                .any(|e| matches!(e, SemanticEvent::OutputText { .. })),
            "assistant message must still route to OutputText"
        );
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
            SemanticEvent::ToolResult {
                name,
                id,
                status,
                output,
                ..
            } => {
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
        assert!(matches!(
            collected[0],
            SemanticEvent::Error { terminal: true, .. }
        ));
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
    fn tool_input_string_fallback_parses_without_panic() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"tool_use","tool_id":"t","tool_name":"bash","input":"ls -la"}"#)
            .unwrap();
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
        parser.feed_line(r#"{"payload":{"k":1}}"#).unwrap();
        let collected = events.lock().unwrap().clone();
        assert_eq!(collected.len(), 1);
        match &collected[0] {
            SemanticEvent::ProviderExtension {
                provider,
                kind,
                payload,
            } => {
                assert_eq!(*provider, Provider::Gemini);
                assert_eq!(kind, "");
                assert_eq!(payload.get("payload"), Some(&json!({"k": 1})));
            }
            other => panic!("expected ProviderExtension, got {other:?}"),
        }
    }

    #[test]
    fn streamed_markdown_list_emits_contiguous_items() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/providers/gemini-markdown-list.ndjson");
        let raw = std::fs::read_to_string(&path).expect("fixture exists");
        let (events, mut parser) = new_parser();
        for line in raw.lines() {
            parser.feed_line(line).unwrap();
        }
        let text: String = events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                SemanticEvent::OutputText { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        // No bullet item should appear split mid-content (i.e. emitted twice
        // across two OutputText events — joined, no awkward internal split).
        let bullet_lines: Vec<&str> = text
            .lines()
            .filter(|l| l.trim_start().starts_with("- ") || l.trim_start().starts_with("* "))
            .collect();
        assert!(
            !bullet_lines.is_empty(),
            "fixture must include bullet items"
        );
        for line in &bullet_lines {
            assert!(
                line.len() > 5,
                "bullet item appears truncated or split: {line:?}\nfull text:\n{text}"
            );
        }
        // No three-or-more consecutive newlines (would indicate stray blank
        // lines from per-chunk emission).
        assert!(
            !text.contains("\n\n\n"),
            "unexpected triple-newline in:\n{text}"
        );
    }

    #[test]
    fn delta_false_message_bypasses_buffer() {
        // Non-delta messages must emit immediately, not be held back.
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"g1","model":"gemini-2.5"}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"message","role":"assistant","content":"one-shot answer"}"#)
            .unwrap();
        let kinds: Vec<&'static str> = events
            .lock()
            .unwrap()
            .iter()
            .map(|e| e.kind_str())
            .collect();
        assert!(
            kinds.contains(&"output_text"),
            "non-delta message must emit output_text immediately; got {kinds:?}"
        );
    }

    #[test]
    fn pending_delta_flushed_on_non_text_event() {
        // Buffered text from a delta must be flushed when a non-text event
        // (e.g. turn completion) arrives, even without an explicit blank line.
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"type":"init","session_id":"g1","model":"gemini-2.5"}"#)
            .unwrap();
        // Partial delta: no trailing \n\n
        parser
            .feed_line(r#"{"type":"message","role":"assistant","delta":true,"content":"partial "}"#)
            .unwrap();
        parser
            .feed_line(r#"{"type":"message","role":"assistant","delta":true,"content":"more"}"#)
            .unwrap();
        // Turn completes — buffer must flush.
        parser
            .feed_line(r#"{"type":"result","usage":{"input_tokens":10,"output_tokens":20}}"#)
            .unwrap();
        let text: String = events
            .lock()
            .unwrap()
            .iter()
            .filter_map(|e| match e {
                SemanticEvent::OutputText { text, .. } => Some(text.clone()),
                _ => None,
            })
            .collect();
        assert!(
            text.contains("partial more"),
            "buffered delta content must be flushed on terminal event; got {text:?}"
        );
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
