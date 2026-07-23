//! Native [`SemanticStreamParser`] implementation for Antigravity's
//! `agy --print --output-format json` output.
//!
//! Antigravity is bespoke and, uniquely, NOT a line-delimited stream: `agy`
//! print mode emits ONE buffered JSON envelope after the run completes. So this
//! parser accumulates every fed line into a buffer, parses it as a single
//! [`AntigravityEnvelope`] as soon as the buffer is valid JSON, and derives the
//! terminal events from that one object:
//!
//! - `response` → [`SemanticEvent::OutputText`] (also the summary's
//!   `assistant_text`).
//! - a non-`SUCCESS` `status` and/or a top-level `error` →
//!   [`SemanticEvent::Error`] (terminal — the envelope IS the run end),
//!   text-classified into a [`SemanticErrorKind`] (agy exposes no structured
//!   error category).
//! - `usage` / `duration_seconds` / `num_turns` → [`SemanticEvent::TurnComplete`]
//!   (the terminal record carrying token accounting).
//!
//! `conversation_id` is captured into the summary's `session_id` so the wrapper
//! can resume with `agy --conversation <id>`. If the run produced no parseable
//! envelope (e.g. an auth failure prints a plain-text error and exits non-zero),
//! `finish` records the raw stdout as a classified error.

use serde_json::{Map, Value};

use super::parser::SemanticStreamParser;
use super::protocol::antigravity::AntigravityEnvelope;
use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use super::token_usage::NormalizedTokenUsage;
use crate::provider_id::Provider;

pub struct AntigravitySemanticStreamParser<S: SemanticEventSink> {
    sink: S,
    model: Option<String>,
    /// Accumulated stdout; parsed as one JSON envelope once complete.
    buffer: String,
    emitted: bool,
    session_id: Option<String>,
    assistant_text: String,
    token_usage: NormalizedTokenUsage,
    has_usage: bool,
    cost_usd: f64,
    duration_ms: Option<u64>,
    num_turns: u32,
    provider_status: Option<String>,
    is_error: bool,
    error_kind: Option<String>,
    error_message: Option<String>,
}

impl<S: SemanticEventSink> AntigravitySemanticStreamParser<S> {
    pub fn new(sink: S, model: Option<String>) -> Self {
        Self {
            sink,
            model,
            buffer: String::new(),
            emitted: false,
            session_id: None,
            assistant_text: String::new(),
            token_usage: NormalizedTokenUsage::default(),
            has_usage: false,
            cost_usd: 0.0,
            duration_ms: None,
            num_turns: 0,
            provider_status: None,
            is_error: false,
            error_kind: None,
            error_message: None,
        }
    }

    fn base_extra(&self) -> Map<String, Value> {
        let mut m = Map::new();
        m.insert("provider".into(), Value::from("antigravity"));
        m
    }

    /// Attempt to parse the accumulated buffer as the single result envelope.
    /// Silent on incomplete JSON (agy may pretty-print across lines); the real
    /// diagnosis happens once in [`finish`].
    fn try_emit(&mut self) {
        if self.emitted {
            return;
        }
        let trimmed = self.buffer.trim();
        if trimmed.is_empty() {
            return;
        }
        if let Some(envelope) = parse_envelope(trimmed) {
            self.process(envelope);
        }
    }

    fn process(&mut self, envelope: AntigravityEnvelope) {
        self.emitted = true;
        self.session_id = envelope.conversation_id;
        super::trace_session_metadata(
            Provider::Antigravity,
            self.session_id.as_deref(),
            self.model.as_deref(),
        );

        if let Some(usage) = envelope.usage {
            self.token_usage = NormalizedTokenUsage {
                input: usage.input_tokens,
                output: usage.output_tokens,
                total: usage.total_tokens,
                cache_read: None,
            };
            self.has_usage = usage.input_tokens.is_some() || usage.output_tokens.is_some();
        }
        self.duration_ms = envelope
            .duration_seconds
            .map(|s| (s * 1000.0).round() as u64);
        self.num_turns = envelope.num_turns.unwrap_or(0);
        self.provider_status = envelope.status.clone();

        if let Some(text) = envelope.response
            && !text.is_empty()
        {
            self.assistant_text.push_str(&text);
            self.sink.on_semantic_event(SemanticEvent::OutputText {
                text,
                extra: Value::Object(self.base_extra()),
            });
        }

        // The envelope is the whole run: a non-SUCCESS status or a top-level
        // error means the (terminal) run failed.
        let status_ok = envelope
            .status
            .as_deref()
            .map(|s| s.eq_ignore_ascii_case("success"))
            .unwrap_or(true);
        let error_text = envelope.error.filter(|e| !e.is_empty());
        if !status_ok || error_text.is_some() {
            let message = error_text.unwrap_or_else(|| {
                format!(
                    "agy run ended with status {}",
                    self.provider_status.as_deref().unwrap_or("<none>")
                )
            });
            self.record_error(&message);
        }

        super::trace_summary_update(
            Provider::Antigravity,
            self.provider_status.as_deref(),
            self.duration_ms,
            Some(self.cost_usd),
        );
        self.num_turns = self.num_turns.max(1);
        self.sink.on_semantic_event(SemanticEvent::TurnComplete {
            provider_status: self.provider_status.clone(),
            token_usage: self.has_usage.then(|| self.token_usage.clone()),
            cost_usd: None,
            duration_ms: self.duration_ms,
            extra: Value::Object(self.base_extra()),
        });
    }

    fn record_error(&mut self, message: &str) {
        self.is_error = true;
        self.error_message = Some(message.to_string());
        let kind = classify_error(message);
        self.error_kind = Some(kind.as_str().to_string());
        let mut extra = self.base_extra();
        extra.insert("error_kind".into(), Value::from(kind.as_str()));
        self.sink.on_semantic_event(SemanticEvent::Error {
            message: message.to_string(),
            // agy's `--output-format json` envelope is the entire run, so an
            // error state here IS the terminal outcome.
            terminal: true,
            kind,
            extra: Value::Object(extra),
        });
    }
}

impl<S: SemanticEventSink> SemanticStreamParser for AntigravitySemanticStreamParser<S> {
    fn feed_line(&mut self, line: &str) {
        // agy buffers the whole response into one JSON object; accumulate until
        // it parses (handles both single-line and pretty-printed output).
        self.buffer.push_str(line);
        self.buffer.push('\n');
        self.try_emit();
    }

    fn finish(self: Box<Self>, exit_code: i32) -> StreamExecutionSummary {
        let mut this = *self;
        // Last chance to parse a complete envelope, then diagnose a run that
        // produced no JSON at all (e.g. a plain-text auth failure).
        this.try_emit();
        if !this.emitted {
            let raw = this.buffer.trim();
            if !raw.is_empty() {
                let message = raw.lines().last().unwrap_or(raw).to_string();
                this.record_error(&message);
            } else if exit_code != 0 {
                this.record_error(&format!("agy exited with code {exit_code}"));
            }
        }
        super::trace_parser_finish(
            Provider::Antigravity,
            exit_code,
            0,
            this.num_turns,
            this.provider_status.as_deref(),
        );
        super::common::finish_summary(
            Provider::Antigravity,
            StreamExecutionSummary {
                session_id: this.session_id,
                model: this.model,
                assistant_text: this.assistant_text,
                provider_status: this.provider_status,
                exit_code,
                is_error: this.is_error,
                error_kind: this.error_kind,
                error_message: this.error_message,
                duration_ms: this.duration_ms,
                num_turns: (this.num_turns > 0).then_some(this.num_turns),
                token_usage: this.has_usage.then_some(this.token_usage),
                ..Default::default()
            },
        )
    }
}

/// Parse the buffered result envelope, tolerating the unescaped control
/// characters agy occasionally emits inside the `response` string. Strict JSON
/// is tried first — the normal case, since agy emits compact, correctly-escaped
/// single-line JSON — and only on failure are bare control characters escaped
/// and the parse retried. agy's output has no structural whitespace (it is one
/// compact line), so escaping strays cannot corrupt a genuinely-valid document
/// (a valid one already parsed on the first attempt).
fn parse_envelope(raw: &str) -> Option<AntigravityEnvelope> {
    if let Ok(envelope) = serde_json::from_str::<AntigravityEnvelope>(raw) {
        return Some(envelope);
    }
    let escaped = raw
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    serde_json::from_str::<AntigravityEnvelope>(&escaped).ok()
}

/// Classify an agy error string into a typed [`SemanticErrorKind`]. agy exposes
/// no structured error category (print mode returns free text / a `status`
/// state), so classification is text-based — mirroring the Pi/OpenCode path,
/// with an auth branch for the keyring/OAuth failure modes agy surfaces.
fn classify_error(message: &str) -> SemanticErrorKind {
    super::common::classify_error_by_keywords(
        super::vocabulary::error_keywords(Provider::Antigravity),
        None,
        None,
        Some(message),
    )
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
        Box<AntigravitySemanticStreamParser<Recording>>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = Recording {
            events: events.clone(),
        };
        (
            events,
            Box::new(AntigravitySemanticStreamParser::new(
                sink,
                Some("Gemini 3.5 Flash (Low)".to_string()),
            )),
        )
    }

    const SUCCESS_ENVELOPE: &str = r#"{"conversation_id":"46be74f2-14d5-4f30-8ff4-94aca94420d4","status":"SUCCESS","response":"OK\n","duration_seconds":1.785492,"num_turns":1,"usage":{"input_tokens":23791,"output_tokens":6,"thinking_tokens":0,"total_tokens":23797}}"#;

    #[test]
    fn success_envelope_emits_text_and_turn_complete() {
        let (events, mut parser) = new_parser();
        parser.feed_line(SUCCESS_ENVELOPE);
        let summary = parser.finish(0);

        let events = events.lock().unwrap();
        assert!(matches!(events[0], SemanticEvent::OutputText { ref text, .. } if text == "OK\n"));
        assert!(matches!(
            events.last().unwrap(),
            SemanticEvent::TurnComplete { .. }
        ));
        assert!(!summary.is_error);
        assert_eq!(summary.assistant_text, "OK\n");
        assert_eq!(summary.session_id.as_deref(), Some("46be74f2-14d5-4f30-8ff4-94aca94420d4"));
        assert_eq!(summary.duration_ms, Some(1785));
        let usage = summary.token_usage.unwrap();
        assert_eq!(usage.input, Some(23791));
        assert_eq!(usage.output, Some(6));
        assert_eq!(usage.total, Some(23797));
    }

    #[test]
    fn pretty_printed_envelope_parses_across_lines() {
        let (events, mut parser) = new_parser();
        for line in ["{", "  \"status\": \"SUCCESS\",", "  \"response\": \"hi\"", "}"] {
            parser.feed_line(line);
        }
        let summary = parser.finish(0);
        assert!(!summary.is_error);
        assert_eq!(summary.assistant_text, "hi");
        let events = events.lock().unwrap();
        assert!(events.iter().any(|e| matches!(e, SemanticEvent::OutputText { .. })));
    }

    #[test]
    fn unescaped_control_char_in_response_is_tolerated() {
        // agy has been observed to occasionally emit a raw (unescaped) newline
        // inside the response string — strict JSON, and thus serde, rejects it.
        // The lenient reparse recovers the text instead of surfacing an error.
        let (_events, mut parser) = new_parser();
        parser
            .feed_line("{\"status\":\"SUCCESS\",\"response\":\"line one\nline two\"}");
        let summary = parser.finish(0);
        assert!(!summary.is_error, "raw control char should be tolerated");
        assert_eq!(summary.assistant_text, "line one\nline two");
    }

    #[test]
    fn error_status_records_terminal_error() {
        let (events, mut parser) = new_parser();
        parser
            .feed_line(r#"{"status":"ERROR","error":"quota exhausted for this model"}"#);
        let summary = parser.finish(1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("api_remote"));
        let events = events.lock().unwrap();
        assert!(events.iter().any(|e| matches!(
            e,
            SemanticEvent::Error { terminal: true, kind: SemanticErrorKind::ApiRemote, .. }
        )));
    }

    #[test]
    fn non_json_stdout_is_classified_on_finish() {
        let (_events, mut parser) = new_parser();
        parser
            .feed_line("Error: authentication failed or timed out");
        let summary = parser.finish(1);
        assert!(summary.is_error);
        // Auth failures classify as Configuration (keyring/OAuth, not remote).
        assert_eq!(summary.error_kind.as_deref(), Some("configuration"));
    }

    #[test]
    fn empty_stdout_nonzero_exit_is_error() {
        let (_events, parser) = new_parser();
        let summary = parser.finish(2);
        assert!(summary.is_error);
    }
}
