//! Pure parsing and classification for Codex stderr logs.
//!
//! Codex emits tracing-subscriber style records on stderr when it hits a
//! non-fatal condition (for example: `2026-04-17T14:27:47.098329Z ERROR
//! codex_core::tools::router: error=…`). These lines are valuable context
//! but their raw formatting is noisy next to Claudine's structured stderr
//! surface. The bridge defined here parses each line into a typed record,
//! re-emits it as a [`SemanticEvent::Warning`] enriched with
//! `tracing_target` / `tracing_level` extras, and suppresses the raw line
//! from passthrough so the live sink can render it with the same spacing
//! discipline as every other event.
//!
//! The parser is deliberately small and tolerant: anything that does not
//! match the expected `TIMESTAMP LEVEL target: body` header falls through
//! to [`StderrIngestOutcome::NotConsumed`] so the caller continues writing
//! the line directly to the user.

use std::sync::{Arc, Mutex};

use regex::Regex;
use serde_json::{Map, Value};
use std::sync::LazyLock;

use super::StderrIngestOutcome;
use crate::stream::semantic::{SemanticEvent, SemanticEventSink, SharedSemanticSink};

/// Severity level extracted from a Codex tracing log record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl CodexLogLevel {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "TRACE" => Some(Self::Trace),
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            _ => None,
        }
    }

    /// Lowercase label carried in `extra.tracing_level` so downstream
    /// consumers (JSONL reporting, tests) receive a stable identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            CodexLogLevel::Trace => "trace",
            CodexLogLevel::Debug => "debug",
            CodexLogLevel::Info => "info",
            CodexLogLevel::Warn => "warn",
            CodexLogLevel::Error => "error",
        }
    }
}

/// Parsed tracing record. The header is preserved via `raw` so downstream
/// logging can replay the original line when needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodexLogRecord {
    pub timestamp: String,
    pub level: CodexLogLevel,
    pub target: String,
    pub body: String,
    pub raw: String,
}

static HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    // Matches the default `tracing-subscriber` fmt layer output used by
    // Codex: ISO-8601 timestamp, space, level, space, `target:` prefix,
    // then a free-form body that runs to end-of-line. The target uses `::`
    // separators and may contain digits/underscores.
    Regex::new(
        r"^(?P<ts>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?Z)\s+(?P<level>TRACE|DEBUG|INFO|WARN|ERROR)\s+(?P<target>[A-Za-z_][A-Za-z0-9_:]*):\s?(?P<body>.*)$",
    )
    .expect("codex tracing header regex must compile")
});

/// Parse a single Codex stderr line into a typed record. Returns `None`
/// when the line does not match the tracing header so callers can fall
/// back to raw passthrough.
pub fn parse_line(line: &str) -> Option<CodexLogRecord> {
    let trimmed = line.trim_end_matches(['\r', '\n']);
    let caps = HEADER_RE.captures(trimmed)?;
    let level = CodexLogLevel::from_str(caps.name("level")?.as_str())?;
    let body_raw = caps.name("body")?.as_str().trim();
    // Codex's router surfaces the underlying cause as `error=<msg>`. Strip
    // that prefix so the BlockQuote body reads as a plain sentence rather
    // than a key=value record.
    let body = body_raw
        .strip_prefix("error=")
        .or_else(|| body_raw.strip_prefix("err="))
        .unwrap_or(body_raw)
        .trim()
        .to_string();
    Some(CodexLogRecord {
        timestamp: caps.name("ts")?.as_str().to_string(),
        level,
        target: caps.name("target")?.as_str().to_string(),
        body,
        raw: line.to_string(),
    })
}

/// Bridge that consumes Codex stderr lines, parses tracing records, and
/// re-emits them through a shared [`SemanticEventSink`] so the live
/// surface renders them with the rest of the stream.
///
/// Cloning the bridge is cheap: the only owned state is the shared sink.
pub struct CodexLogBridge<S: SemanticEventSink> {
    sink: SharedSemanticSink<S>,
    observed: Arc<Mutex<Vec<CodexLogRecord>>>,
}

impl<S: SemanticEventSink> CodexLogBridge<S> {
    pub fn new(sink: SharedSemanticSink<S>) -> Self {
        Self {
            sink,
            observed: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Expose the shared record log so tests (and the summary finalizer)
    /// can inspect what the bridge classified during a run.
    pub fn observed_records(&self) -> Arc<Mutex<Vec<CodexLogRecord>>> {
        Arc::clone(&self.observed)
    }

    /// Consume a single stderr line. Returns whether the bridge absorbed
    /// the line (so the caller suppresses raw echo).
    pub fn ingest(&mut self, line: &str) -> StderrIngestOutcome {
        let Some(record) = parse_line(line) else {
            return StderrIngestOutcome::NotConsumed;
        };

        // Capture the record first so even a failing sink lock still leaves
        // diagnostic breadcrumbs for tests and the summary finalizer.
        if let Ok(mut observed) = self.observed.lock() {
            observed.push(record.clone());
        }

        let mut extra: Map<String, Value> = Map::new();
        extra.insert("provider".into(), Value::from("codex"));
        extra.insert("tracing_target".into(), Value::from(record.target.as_str()));
        extra.insert("tracing_level".into(), Value::from(record.level.as_str()));
        extra.insert(
            "tracing_timestamp".into(),
            Value::from(record.timestamp.as_str()),
        );
        extra.insert("raw".into(), Value::from(record.raw.as_str()));

        self.sink.clone().on_semantic_event(SemanticEvent::Warning {
            message: record.body.clone(),
            extra: Value::Object(extra),
        });
        StderrIngestOutcome::Consumed
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::stream::semantic::{SemanticEvent, SemanticEventSink};

    struct Recording(Arc<Mutex<Vec<SemanticEvent>>>);
    impl SemanticEventSink for Recording {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.0.lock().unwrap().push(event);
        }
    }

    fn recorder() -> (
        Arc<Mutex<Vec<SemanticEvent>>>,
        SharedSemanticSink<Recording>,
    ) {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = SharedSemanticSink::new(Recording(events.clone()));
        (events, sink)
    }

    #[test]
    fn parse_error_with_error_prefix_strips_prefix() {
        let line = "2026-04-17T14:27:47.098329Z ERROR codex_core::tools::router: error=forked agents inherit the parent agent type";
        let record = parse_line(line).expect("must parse");
        assert_eq!(record.level, CodexLogLevel::Error);
        assert_eq!(record.target, "codex_core::tools::router");
        assert_eq!(
            record.body, "forked agents inherit the parent agent type",
            "the leading `error=` tag must be stripped from the body"
        );
    }

    #[test]
    fn parse_warn_without_prefix_preserves_body() {
        let line = "2026-04-17T14:27:47Z WARN codex_core::agent: something is odd";
        let record = parse_line(line).expect("must parse");
        assert_eq!(record.level, CodexLogLevel::Warn);
        assert_eq!(record.body, "something is odd");
    }

    #[test]
    fn unrecognized_lines_return_none() {
        assert!(parse_line("not a tracing line").is_none());
        assert!(parse_line("").is_none());
        // Missing target segment.
        assert!(parse_line("2026-04-17T14:27:47Z ERROR no colon here").is_none());
    }

    #[test]
    fn bridge_emits_warning_and_consumes_line() {
        let (events, sink) = recorder();
        let mut bridge = CodexLogBridge::new(sink);
        let outcome = bridge.ingest(
            "2026-04-17T14:27:47.098329Z ERROR codex_core::tools::router: error=forked agents inherit the parent agent type",
        );
        assert_eq!(outcome, StderrIngestOutcome::Consumed);
        let collected = events.lock().unwrap().clone();
        assert_eq!(collected.len(), 1);
        match &collected[0] {
            SemanticEvent::Warning { message, extra } => {
                assert_eq!(message, "forked agents inherit the parent agent type");
                assert_eq!(
                    extra.get("tracing_target").and_then(Value::as_str),
                    Some("codex_core::tools::router"),
                );
                assert_eq!(
                    extra.get("tracing_level").and_then(Value::as_str),
                    Some("error"),
                );
                assert_eq!(extra.get("provider").and_then(Value::as_str), Some("codex"),);
            }
            other => panic!("expected Warning, got {other:?}"),
        }
    }

    #[test]
    fn bridge_does_not_consume_unrecognized_lines() {
        let (events, sink) = recorder();
        let mut bridge = CodexLogBridge::new(sink);
        let outcome = bridge.ingest("plain text spam");
        assert_eq!(outcome, StderrIngestOutcome::NotConsumed);
        assert!(events.lock().unwrap().is_empty());
    }

    #[test]
    fn bridge_records_every_observed_record() {
        let (_events, sink) = recorder();
        let mut bridge = CodexLogBridge::new(sink);
        bridge.ingest("2026-04-17T14:27:47Z WARN a: one");
        bridge.ingest("2026-04-17T14:27:48Z ERROR b: two");
        let records = bridge.observed_records();
        let guard = records.lock().unwrap();
        assert_eq!(guard.len(), 2);
        assert_eq!(guard[0].target, "a");
        assert_eq!(guard[1].level, CodexLogLevel::Error);
    }
}
