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

use super::parser::SemanticStreamParser;
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
    fn feed_line(&mut self, line: &str) {
        self.line_num += 1;
        let line = line.trim();
        if line.is_empty() {
            return;
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
                        return;
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
mod tests;
