//! Pure parsing and classification for OpenCode stderr logs.
//!
//! OpenCode emits structured log records on stderr when launched with
//! `--print-logs --log-level ERROR`. Each record has a fixed header
//! (`LEVEL TIMESTAMP +DELTAms ...`) followed by a free-form body of
//! `key=value` tags and an optional trailing message. Inline JSON is
//! permitted as a value, and the special keys `error=` and `err=` are
//! terminal-to-end-of-line.
//!
//! The parser is deliberately small and resilient: unknown tags are
//! preserved in an open-ended [`BTreeMap`], missing tags are accepted,
//! and any line that does not match the header falls through to
//! [`ParsedOpenCodeStderrLine::RawText`]. Classification is a pure
//! function over the parsed record plus a small raw-text fallback for
//! fatal exceptions.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use regex::Regex;

/// Log severity, matching the levels OpenCode emits in the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    fn from_str(value: &str) -> Option<Self> {
        match value {
            "DEBUG" => Some(Self::Debug),
            "INFO" => Some(Self::Info),
            "WARN" => Some(Self::Warn),
            "ERROR" => Some(Self::Error),
            _ => None,
        }
    }
}

/// Malformed asset kind discovered from "failed to load X" tails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetType {
    Skill,
    Command,
    Agent,
    Config,
    Unknown,
}

/// Structured OpenCode log record parsed from a single stderr line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenCodeLogRecord {
    pub level: LogLevel,
    pub timestamp: DateTime<Utc>,
    pub delta_ms: u64,
    pub tags: BTreeMap<String, String>,
    pub message: String,
    pub raw: String,
}

/// Outcome of running the parser over a single stderr line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedOpenCodeStderrLine {
    Structured(OpenCodeLogRecord),
    RawText(String),
}

/// Action category derived from a parsed record or raw stderr line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogClassification {
    RateLimit {
        status_code: u16,
        error_name: String,
        reset_at: Option<DateTime<Utc>>,
        provider_id: Option<String>,
        model_id: Option<String>,
        provider_error: String,
    },
    MalformedAsset {
        asset_type: AssetType,
        path: Option<String>,
        error: String,
    },
    ApiFailure {
        status_code: Option<u16>,
        error_name: String,
        message: String,
    },
    AuthFailure {
        message: String,
    },
    UncaughtError {
        raw_text: String,
    },
    Unclassified,
}

static HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<level>DEBUG|INFO|WARN|ERROR)\s+(?P<ts>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\s+\+(?P<delta>\d+)ms(?:\s+(?P<body>.*))?$",
    )
    .expect("opencode log header regex must compile")
});

static RESET_AT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"reset at (\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2}:\d{2})")
        .expect("opencode reset-at regex must compile")
});

static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("ansi regex must compile"));

/// Parse a single OpenCode stderr line into either a structured record
/// or a raw passthrough string.
pub fn parse_line(line: &str) -> ParsedOpenCodeStderrLine {
    let trimmed_right = line.trim_end_matches(['\r', '\n']);
    let Some(caps) = HEADER_RE.captures(trimmed_right) else {
        return ParsedOpenCodeStderrLine::RawText(line.to_string());
    };

    let Some(level) = caps
        .name("level")
        .and_then(|m| LogLevel::from_str(m.as_str()))
    else {
        return ParsedOpenCodeStderrLine::RawText(line.to_string());
    };

    let ts_str = caps.name("ts").map(|m| m.as_str()).unwrap_or("");
    let timestamp = match parse_timestamp(ts_str) {
        Some(t) => t,
        None => return ParsedOpenCodeStderrLine::RawText(line.to_string()),
    };

    let delta_ms = caps
        .name("delta")
        .and_then(|m| m.as_str().parse::<u64>().ok())
        .unwrap_or(0);

    let body = caps.name("body").map(|m| m.as_str()).unwrap_or("");
    let (tags, message) = parse_body(body);

    ParsedOpenCodeStderrLine::Structured(OpenCodeLogRecord {
        level,
        timestamp,
        delta_ms,
        tags,
        message,
        raw: line.to_string(),
    })
}

/// Classify a parsed structured log record.
pub fn classify(record: &OpenCodeLogRecord) -> LogClassification {
    if let Some(classification) = classify_malformed_asset(record) {
        return classification;
    }

    let service = record.tags.get("service").map(|s| s.as_str()).unwrap_or("");

    if (service == "llm" || service == "provider")
        && let Some(classification) = classify_llm_failure(record, service)
    {
        return classification;
    }

    if looks_like_uncaught_error(record) {
        return LogClassification::UncaughtError {
            raw_text: record.raw.clone(),
        };
    }

    LogClassification::Unclassified
}

/// Classify a raw stderr line that did not match the structured header.
///
/// Fatal error prefixes (`Error:` with or without ANSI colors) map to
/// [`LogClassification::UncaughtError`]; anything else is left as
/// [`LogClassification::Unclassified`] so the caller can pass the line
/// through to operators.
pub fn classify_raw(line: &str) -> LogClassification {
    let stripped = strip_ansi(line);
    let trimmed = stripped.trim_start();
    if trimmed.starts_with("Error:") || trimmed.starts_with("error:") {
        return LogClassification::UncaughtError {
            raw_text: line.to_string(),
        };
    }
    LogClassification::Unclassified
}

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .ok()
        .map(|naive| Utc.from_utc_datetime(&naive))
}

fn parse_body(body: &str) -> (BTreeMap<String, String>, String) {
    let mut tags = BTreeMap::new();
    let bytes = body.as_bytes();
    let len = bytes.len();
    let mut pos = 0;

    loop {
        while pos < len && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
            pos += 1;
        }
        if pos >= len {
            break;
        }

        let key_start = pos;
        let mut key_end = pos;
        while key_end < len && is_ident_byte(bytes[key_end]) {
            key_end += 1;
        }
        if key_end == key_start || key_end >= len || bytes[key_end] != b'=' {
            break;
        }

        let key = &body[key_start..key_end];
        let value_start = key_end + 1;

        // `err` and `error` swallow the remainder of the line.
        if key == "error" || key == "err" {
            let value = body[value_start..].trim_end().to_string();
            tags.insert(key.to_string(), value);
            pos = len;
            break;
        }

        let (value, new_pos) = extract_value(body, value_start);
        tags.insert(key.to_string(), value);
        pos = new_pos;
    }

    let message = body[pos..].trim().to_string();
    (tags, message)
}

fn extract_value(body: &str, start: usize) -> (String, usize) {
    let bytes = body.as_bytes();
    let len = bytes.len();
    if start >= len {
        return (String::new(), len);
    }

    if (bytes[start] == b'{' || bytes[start] == b'[')
        && let Some(end) = find_json_end(&body[start..])
    {
        let value = body[start..start + end].to_string();
        return (value, start + end);
    }

    // Bare value: read until next whitespace-prefixed `ident=` boundary
    // or the end of the line.
    let mut cursor = start;
    while cursor < len {
        if (bytes[cursor] == b' ' || bytes[cursor] == b'\t') && is_tag_boundary(bytes, cursor + 1) {
            break;
        }
        cursor += 1;
    }

    let value = body[start..cursor].trim_end().to_string();
    (value, cursor)
}

fn is_tag_boundary(bytes: &[u8], mut idx: usize) -> bool {
    while idx < bytes.len() && (bytes[idx] == b' ' || bytes[idx] == b'\t') {
        idx += 1;
    }
    let start = idx;
    while idx < bytes.len() && is_ident_byte(bytes[idx]) {
        idx += 1;
    }
    idx > start && idx < bytes.len() && bytes[idx] == b'='
}

fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn find_json_end(s: &str) -> Option<usize> {
    let mut stream = serde_json::Deserializer::from_str(s).into_iter::<serde_json::Value>();
    stream.next()?.ok()?;
    Some(stream.byte_offset())
}

fn classify_malformed_asset(record: &OpenCodeLogRecord) -> Option<LogClassification> {
    let err = record
        .tags
        .get("err")
        .or_else(|| record.tags.get("error"))
        .map(String::as_str)
        .unwrap_or("");

    let (asset_type, error_text) = if let Some(kind) = detect_asset_suffix(err) {
        (kind, err.to_string())
    } else if let Some(kind) = detect_asset_suffix(&record.message) {
        (kind, record.message.clone())
    } else {
        return None;
    };

    let path = match asset_type {
        AssetType::Skill => record.tags.get("skill").cloned(),
        AssetType::Command => record.tags.get("command").cloned(),
        AssetType::Agent => record.tags.get("agent").cloned(),
        AssetType::Config => record
            .tags
            .get("path")
            .or_else(|| record.tags.get("file"))
            .cloned(),
        AssetType::Unknown => None,
    };

    Some(LogClassification::MalformedAsset {
        asset_type,
        path,
        error: error_text,
    })
}

fn detect_asset_suffix(value: &str) -> Option<AssetType> {
    let lowered = value.to_lowercase();
    if lowered.contains("failed to load skill") {
        Some(AssetType::Skill)
    } else if lowered.contains("failed to load command") {
        Some(AssetType::Command)
    } else if lowered.contains("failed to load agent") {
        Some(AssetType::Agent)
    } else if lowered.contains("failed to load config") {
        Some(AssetType::Config)
    } else {
        None
    }
}

fn summarize_error_json(record: &OpenCodeLogRecord) -> String {
    let error_tag = match record.tags.get("error") {
        Some(tag) => tag,
        None => return String::new(),
    };

    let root: serde_json::Value = match serde_json::from_str(error_tag) {
        Ok(v) => v,
        Err(_) => return String::new(),
    };

    let envelope = root.get("error").unwrap_or(&root);

    let error_name = envelope
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let status_code = envelope
        .get("statusCode")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            envelope
                .get("errors")
                .and_then(|errors| errors.get(0))
                .and_then(|e| e.get("statusCode"))
                .and_then(|v| v.as_u64())
        })
        .or_else(|| {
            envelope
                .get("lastError")
                .and_then(|e| e.get("statusCode"))
                .and_then(|v| v.as_u64())
        });

    let provider_message = extract_provider_message(envelope);

    let mut parts = Vec::new();

    if let Some(code) = status_code {
        parts.push(format!("{error_name} ({code})"));
    } else {
        parts.push(error_name.to_string());
    }

    if let Some(msg) = &provider_message {
        parts.push(msg.clone());
    }

    parts.join(": ")
}

fn extract_provider_message(envelope: &serde_json::Value) -> Option<String> {
    if let Some(msg) = envelope.get("message").and_then(|v| v.as_str())
        && !msg.is_empty()
    {
        return Some(msg.to_string());
    }

    if let Some(body_str) = envelope.get("responseBody").and_then(|v| v.as_str()) {
        if let Ok(body) = serde_json::from_str::<serde_json::Value>(body_str)
            && let Some(msg) = body
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
            && !msg.is_empty()
        {
            return Some(msg.to_string());
        }
        if !body_str.is_empty() {
            return Some(body_str.to_string());
        }
    }

    for source in ["errors", "lastError"] {
        let entries = match source {
            "errors" => envelope.get("errors").and_then(|v| v.as_array()).cloned(),
            _ => envelope
                .get(source)
                .map(|v| vec![v.clone()]),
        };
        if let Some(arr) = entries {
            for entry in &arr {
                if let Some(msg) = extract_provider_message(entry) {
                    return Some(msg);
                }
            }
        }
    }

    None
}

fn classify_llm_failure(record: &OpenCodeLogRecord, service: &str) -> Option<LogClassification> {
    let haystack = record.raw.as_str();

    if contains_any_ci(
        haystack,
        &["AuthenticationError", "unauthorized", "Unauthorized"],
    ) || (service == "llm" && haystack.contains("fetch failed"))
    {
        return Some(LogClassification::AuthFailure {
            message: summarize_error_json(record),
        });
    }

    if is_rate_limit(haystack) {
        let status_code = extract_status_code(haystack).unwrap_or(429);
        let error_name = if haystack.contains("AI_RetryError") {
            "AI_RetryError"
        } else {
            "AI_APICallError"
        }
        .to_string();
        let reset_at = extract_reset_at(haystack);
        let provider_id = record.tags.get("providerID").cloned();
        let model_id = record.tags.get("modelID").cloned();
        let provider_error = record
            .tags
            .get("error")
            .cloned()
            .unwrap_or_else(|| haystack.to_string());
        return Some(LogClassification::RateLimit {
            status_code,
            error_name,
            reset_at,
            provider_id,
            model_id,
            provider_error,
        });
    }

    if haystack.contains("AI_APICallError") {
        return Some(LogClassification::ApiFailure {
            status_code: extract_status_code(haystack),
            error_name: "AI_APICallError".to_string(),
            message: summarize_error_json(record),
        });
    }

    None
}

fn is_rate_limit(haystack: &str) -> bool {
    contains_any(
        haystack,
        &[
            "AI_RetryError",
            "maxRetriesExceeded",
            "\"statusCode\":429",
            "statusCode=429",
            "\"code\":\"1308\"",
        ],
    )
}

fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

fn contains_any_ci(haystack: &str, needles: &[&str]) -> bool {
    let lowered = haystack.to_lowercase();
    needles.iter().any(|n| lowered.contains(&n.to_lowercase()))
}

fn extract_status_code(haystack: &str) -> Option<u16> {
    let patterns = [r#""statusCode":(\d{3})"#, r"statusCode=(\d{3})"];
    for pattern in patterns {
        if let Ok(re) = Regex::new(pattern)
            && let Some(caps) = re.captures(haystack)
            && let Some(m) = caps.get(1)
            && let Ok(code) = m.as_str().parse::<u16>()
        {
            return Some(code);
        }
    }
    None
}

fn extract_reset_at(haystack: &str) -> Option<DateTime<Utc>> {
    let caps = RESET_AT_RE.captures(haystack)?;
    let date = NaiveDate::parse_from_str(caps.get(1)?.as_str(), "%Y-%m-%d").ok()?;
    let time = NaiveTime::parse_from_str(caps.get(2)?.as_str(), "%H:%M:%S").ok()?;
    Some(Utc.from_utc_datetime(&NaiveDateTime::new(date, time)))
}

fn looks_like_uncaught_error(record: &OpenCodeLogRecord) -> bool {
    if let Some(name) = record.tags.get("name") {
        let head = name.split_whitespace().next().unwrap_or("");
        if matches!(
            head,
            "TypeError" | "ReferenceError" | "SyntaxError" | "RangeError" | "Error"
        ) {
            return true;
        }
    }
    if record.message.split_whitespace().any(|tok| tok == "fatal") {
        return true;
    }
    record
        .tags
        .values()
        .any(|value| value.split_whitespace().any(|tok| tok == "fatal"))
}

fn strip_ansi(line: &str) -> String {
    ANSI_RE.replace_all(line, "").into_owned()
}

// ---------------------------------------------------------------------------
// Bridge: feed classified stderr log lines into the shared semantic sink,
// maintain stderr-side summary counters, and signal early termination when a
// pre-stream usage-cap error is detected.
// ---------------------------------------------------------------------------

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

use serde_json::{Map, Value, json};
use tracing::{debug, warn};

use crate::stream::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use crate::stream::summary::{RateLimitInfo, StderrDiagnostics};

/// Whether an incoming stderr log line was converted into a semantic event
/// and should therefore be suppressed from raw stderr passthrough.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StderrIngestOutcome {
    /// The bridge classified and emitted a [`SemanticEvent`] for this line.
    /// Callers should not also echo the raw line to the user.
    Consumed,
    /// The bridge did not recognize the line as a meaningful diagnostic.
    /// Callers should keep the existing raw-passthrough behavior.
    NotConsumed,
}

/// Reason the bridge wants `run_child_stream_semantic(...)` to terminate
/// the child process early.
///
/// Today this fires for pre-stream usage-cap failures plus wrapper-driven
/// post-stop hang recovery in OpenCode's structured non-interactive path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EarlyTermination {
    /// The provider reported a rate-limit failure before any stdout
    /// semantic event was observed.
    RateLimit {
        message: String,
        reset_at: Option<DateTime<Utc>>,
    },
    /// OpenCode reported a terminal stop condition (`reason = "stop"`) but
    /// the process never exited. The wrapper terminates the hung process and
    /// treats the run as successful because the semantic stream had already
    /// finished.
    CompletedButHung { message: String },
    /// The harness-configured step-silence budget elapsed with no stream
    /// event observed. The wrapper terminates the child process and maps
    /// the outcome to [`crate::harness::ProcessTermination::TimedOut`] so
    /// the standard `handle_timeout` failure handler runs.
    StepTimeout { message: String },
}

/// Shared stderr-side state accumulated by the bridge as it parses lines.
///
/// Held behind a `Mutex` so the bridge can be cloned cheaply across threads
/// and the main wait loop can read the accumulated diagnostics at the end
/// of the run. The bridge never mutates [`crate::stream::summary::StreamExecutionSummary`]
/// directly; merging is the wrapper layer's responsibility.
#[derive(Debug, Default)]
pub struct SharedStderrState {
    pub diagnostics: StderrDiagnostics,
    pub rate_limit: Option<RateLimitInfo>,
}

impl SharedStderrState {
    /// Convenience accessor: does the shared state contain any parsed
    /// structured log records?
    pub fn any_records(&self) -> bool {
        self.diagnostics.log_records_parsed > 0
    }
}

/// Stderr-side integration object for the OpenCode structured wrapper path.
///
/// Responsibilities:
/// - parse and classify one stderr line at a time via [`parse_line`] +
///   [`classify`] / [`classify_raw`]
/// - emit [`SemanticEvent`]s through a shared sink so the live renderer and
///   JSONL reporting surface stderr diagnostics alongside stdout events
/// - update shared stderr summary counters so the wrapper layer can merge
///   them into the final [`crate::stream::summary::StreamExecutionSummary`]
/// - optionally send an [`EarlyTermination`] signal when a rate limit is
///   observed before any stdout activity
pub struct OpenCodeLogBridge<S: SemanticEventSink> {
    sink: S,
    state: Arc<Mutex<SharedStderrState>>,
    stdout_event_seen: Arc<AtomicBool>,
    early_terminate: Option<Sender<EarlyTermination>>,
    early_terminate_fired: bool,
}

impl<S: SemanticEventSink> OpenCodeLogBridge<S> {
    /// Build a new bridge wired to a shared sink, an observation gate, and
    /// an optional early-termination channel.
    pub fn new(
        sink: S,
        stdout_event_seen: Arc<AtomicBool>,
        early_terminate: Option<Sender<EarlyTermination>>,
    ) -> Self {
        Self {
            sink,
            state: Arc::new(Mutex::new(SharedStderrState::default())),
            stdout_event_seen,
            early_terminate,
            early_terminate_fired: false,
        }
    }

    /// Create a new early-termination channel. Returns the sender for the
    /// bridge plus the receiver the wait loop should poll.
    pub fn new_early_terminate_channel() -> (Sender<EarlyTermination>, Receiver<EarlyTermination>) {
        mpsc::channel()
    }

    /// Clone handle into the shared stderr state for post-run merging.
    pub fn shared_state(&self) -> Arc<Mutex<SharedStderrState>> {
        Arc::clone(&self.state)
    }

    /// Consume one stderr line and return whether the bridge absorbed it.
    ///
    /// A `Consumed` result means the bridge emitted a [`SemanticEvent`];
    /// the caller should suppress raw passthrough for that line.
    pub fn ingest(&mut self, line: &str) -> StderrIngestOutcome {
        match parse_line(line) {
            ParsedOpenCodeStderrLine::Structured(record) => self.handle_structured(record),
            ParsedOpenCodeStderrLine::RawText(raw) => self.handle_raw(&raw),
        }
    }

    fn handle_structured(&mut self, record: OpenCodeLogRecord) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.log_records_parsed =
                state.diagnostics.log_records_parsed.saturating_add(1);
        }

        let classification = classify(&record);
        match classification {
            LogClassification::RateLimit {
                status_code,
                ref error_name,
                reset_at,
                ref provider_id,
                ref model_id,
                ref provider_error,
            } => self.on_rate_limit(
                &record,
                status_code,
                error_name.clone(),
                reset_at,
                provider_id.clone(),
                model_id.clone(),
                provider_error.clone(),
            ),
            LogClassification::MalformedAsset {
                asset_type,
                ref path,
                ref error,
            } => self.on_malformed_asset(&record, asset_type, path.clone(), error.clone()),
            LogClassification::ApiFailure {
                status_code,
                ref error_name,
                ref message,
            } => self.on_api_failure(&record, status_code, error_name.clone(), message.clone()),
            LogClassification::AuthFailure { ref message } => {
                self.on_auth_failure(&record, message.clone())
            }
            LogClassification::UncaughtError { ref raw_text } => {
                self.on_uncaught_error(raw_text.clone(), Some(&record))
            }
            LogClassification::Unclassified => StderrIngestOutcome::NotConsumed,
        }
    }

    fn handle_raw(&mut self, line: &str) -> StderrIngestOutcome {
        match classify_raw(line) {
            LogClassification::UncaughtError { raw_text } => self.on_uncaught_error(raw_text, None),
            _ => StderrIngestOutcome::NotConsumed,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn on_rate_limit(
        &mut self,
        record: &OpenCodeLogRecord,
        status_code: u16,
        error_name: String,
        reset_at: Option<DateTime<Utc>>,
        provider_id: Option<String>,
        model_id: Option<String>,
        provider_error: String,
    ) -> StderrIngestOutcome {
        let stdout_seen = self.stdout_event_seen.load(Ordering::SeqCst);
        let rendered_message = render_rate_limit_message(provider_id, model_id, reset_at);

        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.rate_limit_events =
                state.diagnostics.rate_limit_events.saturating_add(1);
            state.diagnostics.rate_limit_reset_at =
                max_reset_at(state.diagnostics.rate_limit_reset_at, reset_at);
            state.rate_limit = Some(merge_rate_limit(
                state.rate_limit.take(),
                RateLimitInfo {
                    is_throttled: Some(true),
                    retry_after_ms: None,
                    message: Some(rendered_message.clone()),
                    reset_at,
                },
            ));
        }

        let mut extra_map = base_extra(record, "rate_limit");
        extra_map.insert("status_code".into(), json!(status_code));
        extra_map.insert("error_name".into(), Value::String(error_name.clone()));
        if let Some(reset) = reset_at {
            extra_map.insert(
                "reset_at".into(),
                Value::String(reset.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
            );
        }
        if !provider_error.is_empty() {
            extra_map.insert("provider_error".into(), Value::String(provider_error));
        }

        if stdout_seen {
            debug!(
                status_code,
                error_name = %error_name,
                reset_at = ?reset_at,
                "opencode rate-limit classified after stdout activity; emitting warning",
            );
            self.sink.on_semantic_event(SemanticEvent::Warning {
                message: rendered_message,
                extra: Value::Object(extra_map),
            });
        } else {
            debug!(
                status_code,
                error_name = %error_name,
                reset_at = ?reset_at,
                "opencode rate-limit classified before any stdout activity; requesting early termination",
            );
            self.sink.on_semantic_event(SemanticEvent::Error {
                message: rendered_message.clone(),
                terminal: true,
                kind: SemanticErrorKind::ApiRemote,
                extra: Value::Object(extra_map),
            });
            self.fire_early_termination(EarlyTermination::RateLimit {
                message: rendered_message,
                reset_at,
            });
        }

        StderrIngestOutcome::Consumed
    }

    fn on_malformed_asset(
        &mut self,
        record: &OpenCodeLogRecord,
        asset_type: AssetType,
        path: Option<String>,
        error: String,
    ) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.malformed_asset_events =
                state.diagnostics.malformed_asset_events.saturating_add(1);
        }

        let mut extra_map = base_extra(record, "malformed_asset");
        extra_map.insert(
            "asset_type".into(),
            Value::String(asset_type_as_str(asset_type).into()),
        );
        if let Some(ref path) = path {
            extra_map.insert("path".into(), Value::String(path.clone()));
        }
        if !error.is_empty() {
            extra_map.insert("error".into(), Value::String(error.clone()));
        }

        let message = render_malformed_asset_message(asset_type, path.as_deref());
        self.sink.on_semantic_event(SemanticEvent::Warning {
            message,
            extra: Value::Object(extra_map),
        });

        StderrIngestOutcome::Consumed
    }

    fn on_api_failure(
        &mut self,
        record: &OpenCodeLogRecord,
        status_code: Option<u16>,
        error_name: String,
        message: String,
    ) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.api_failures = state.diagnostics.api_failures.saturating_add(1);
        }

        let mut extra_map = base_extra(record, "api_failure");
        extra_map.insert("error_name".into(), Value::String(error_name.clone()));
        if let Some(code) = status_code {
            extra_map.insert("status_code".into(), json!(code));
        }
        if let Some(raw_error) = record.tags.get("error") {
            extra_map.insert("raw_error".into(), Value::String(raw_error.clone()));
        }

        let rendered = if !message.is_empty() {
            message
        } else {
            format!("OpenCode API failure ({error_name})")
        };

        self.sink.on_semantic_event(SemanticEvent::Error {
            message: rendered,
            terminal: true,
            kind: SemanticErrorKind::ApiRemote,
            extra: Value::Object(extra_map),
        });

        StderrIngestOutcome::Consumed
    }

    fn on_auth_failure(
        &mut self,
        record: &OpenCodeLogRecord,
        message: String,
    ) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.auth_failures = state.diagnostics.auth_failures.saturating_add(1);
        }

        let mut extra_map = base_extra(record, "auth_failure");
        if !message.is_empty() {
            extra_map.insert("detail".into(), Value::String(message.clone()));
        }

        let rendered = if message.is_empty() {
            "OpenCode authentication failed".to_string()
        } else {
            message
        };

        self.sink.on_semantic_event(SemanticEvent::Error {
            message: rendered,
            terminal: true,
            kind: SemanticErrorKind::ApiRemote,
            extra: Value::Object(extra_map),
        });

        StderrIngestOutcome::Consumed
    }

    fn on_uncaught_error(
        &mut self,
        raw_text: String,
        record: Option<&OpenCodeLogRecord>,
    ) -> StderrIngestOutcome {
        {
            let mut state = self.state.lock().expect("stderr state poisoned");
            state.diagnostics.uncaught_errors = state.diagnostics.uncaught_errors.saturating_add(1);
        }

        let mut extra_map = Map::new();
        extra_map.insert("provider".into(), Value::String("opencode".into()));
        extra_map.insert("source".into(), Value::String("stderr_log".into()));
        extra_map.insert(
            "classification".into(),
            Value::String("uncaught_error".into()),
        );
        extra_map.insert("raw".into(), Value::String(raw_text.clone()));

        if let Some(record) = record {
            if let Some(service) = record.tags.get("service") {
                extra_map.insert("service".into(), Value::String(service.clone()));
            }
            if let Some(name) = record.tags.get("name") {
                extra_map.insert("error_name".into(), Value::String(name.clone()));
            }
        }

        let rendered = strip_ansi(&raw_text).trim().to_string();
        let rendered = if rendered.is_empty() {
            "OpenCode produced an uncaught error".to_string()
        } else {
            rendered
        };

        self.sink.on_semantic_event(SemanticEvent::Error {
            message: rendered,
            terminal: true,
            kind: SemanticErrorKind::Unknown,
            extra: Value::Object(extra_map),
        });

        StderrIngestOutcome::Consumed
    }

    fn fire_early_termination(&mut self, termination: EarlyTermination) {
        if self.early_terminate_fired {
            return;
        }
        self.early_terminate_fired = true;
        let Some(sender) = self.early_terminate.as_ref() else {
            return;
        };
        if let Err(err) = sender.send(termination) {
            warn!(
                error = %err,
                "failed to deliver OpenCode early-termination signal; receiver dropped",
            );
        }
    }
}

fn base_extra(record: &OpenCodeLogRecord, classification: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("provider".into(), Value::String("opencode".into()));
    map.insert("source".into(), Value::String("stderr_log".into()));
    map.insert(
        "classification".into(),
        Value::String(classification.into()),
    );
    map.insert("raw".into(), Value::String(record.raw.clone()));
    if let Some(service) = record.tags.get("service") {
        map.insert("service".into(), Value::String(service.clone()));
    }
    map
}

fn asset_type_as_str(asset_type: AssetType) -> &'static str {
    match asset_type {
        AssetType::Skill => "skill",
        AssetType::Command => "command",
        AssetType::Agent => "agent",
        AssetType::Config => "config",
        AssetType::Unknown => "unknown",
    }
}

fn render_rate_limit_message(
    provider_id: Option<String>,
    model_id: Option<String>,
    reset_at: Option<DateTime<Utc>>,
) -> String {
    let target = match (provider_id, model_id) {
        (Some(p), Some(m)) => format!(" for {m} ({p})"),
        (None, Some(m)) => format!(" for {m}"),
        (Some(p), None) => format!(" for {p}"),
        (None, None) => "".to_string(),
    };

    match reset_at {
        Some(reset) => {
            let local_time = reset.with_timezone(&Local);
            format!(
                "Usage limit reached{target}; resets at {}",
                local_time.format("%Y-%m-%d %H:%M:%S")
            )
        }
        None => format!("Usage limit reached{target}"),
    }
}

fn render_malformed_asset_message(asset_type: AssetType, path: Option<&str>) -> String {
    let noun = asset_type_as_str(asset_type);
    match path {
        Some(p) => format!("Skipped malformed OpenCode {noun}: {p}"),
        None => format!("Skipped malformed OpenCode {noun}"),
    }
}

fn max_reset_at(
    current: Option<DateTime<Utc>>,
    incoming: Option<DateTime<Utc>>,
) -> Option<DateTime<Utc>> {
    match (current, incoming) {
        (Some(a), Some(b)) => Some(a.max(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// Merge an incoming [`RateLimitInfo`] into an existing one field-by-field.
///
/// Used by the wrapper layer after a structured OpenCode run to combine the
/// stdout parser's rate-limit signals with the ones the stderr log bridge
/// accumulated during streaming. The merge is additive: `is_throttled = true`
/// wins, `retry_after_ms` takes the maximum, incoming `message` replaces when
/// present, and `reset_at` takes the later of the two timestamps.
pub fn merge_rate_limit(existing: Option<RateLimitInfo>, incoming: RateLimitInfo) -> RateLimitInfo {
    let Some(mut base) = existing else {
        return incoming;
    };
    if let Some(true) = incoming.is_throttled {
        base.is_throttled = Some(true);
    } else if base.is_throttled.is_none() {
        base.is_throttled = incoming.is_throttled;
    }
    match (base.retry_after_ms, incoming.retry_after_ms) {
        (Some(a), Some(b)) => base.retry_after_ms = Some(a.max(b)),
        (None, Some(b)) => base.retry_after_ms = Some(b),
        _ => {}
    }
    if let Some(message) = incoming.message {
        base.message = Some(message);
    }
    base.reset_at = max_reset_at(base.reset_at, incoming.reset_at);
    base
}

/// Merge the bridge's accumulated [`SharedStderrState`] into a summary.
///
/// Called once by the wrapper layer after the stderr thread has joined.
/// Always sets `summary.stderr_diagnostics` when the bridge parsed at least
/// one structured log record. Always merges `summary.rate_limit` when the
/// bridge accumulated stderr-side rate-limit state. Always recomputes
/// `summary.badges` via [`crate::stream::badges::derive_badges`] so the
/// stderr-derived badge categories (rate-limit resets, malformed-asset
/// warnings) appear in the final output.
pub fn merge_stderr_state_into_summary(
    state: &std::sync::Arc<std::sync::Mutex<SharedStderrState>>,
    summary: &mut crate::stream::summary::StreamExecutionSummary,
) {
    let Ok(state) = state.lock() else {
        return;
    };
    if state.any_records() {
        summary.stderr_diagnostics = Some(state.diagnostics.clone());
    }
    if let Some(stderr_rl) = state.rate_limit.clone() {
        summary.rate_limit = Some(merge_rate_limit(summary.rate_limit.clone(), stderr_rl));
    }
    drop(state);
    summary.badges = crate::stream::badges::derive_badges(summary, summary.provider);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_accepts_all_levels() {
        for (level_str, expected) in [
            ("DEBUG", LogLevel::Debug),
            ("INFO", LogLevel::Info),
            ("WARN", LogLevel::Warn),
            ("ERROR", LogLevel::Error),
        ] {
            let line = format!("{level_str} 2026-04-15T21:28:30 +5ms service=default msg=ok");
            match parse_line(&line) {
                ParsedOpenCodeStderrLine::Structured(record) => {
                    assert_eq!(record.level, expected, "{level_str}");
                    assert_eq!(record.delta_ms, 5);
                    assert_eq!(
                        record.tags.get("service").map(String::as_str),
                        Some("default")
                    );
                    assert_eq!(record.tags.get("msg").map(String::as_str), Some("ok"));
                }
                other => panic!("expected Structured for {level_str}, got {other:?}"),
            }
        }
    }

    #[test]
    fn header_rejects_non_matching_lines() {
        for line in [
            "not a log line at all",
            "    at processTicksAndRejections (native:7:39) fatal",
            "\u{1b}[91m\u{1b}[1mError: \u{1b}[0mUnexpected error",
            "",
        ] {
            match parse_line(line) {
                ParsedOpenCodeStderrLine::RawText(raw) => assert_eq!(raw, line),
                other => panic!("expected RawText for {line:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn header_rejects_unknown_level() {
        match parse_line("TRACE 2026-04-15T21:28:30 +5ms service=default") {
            ParsedOpenCodeStderrLine::RawText(_) => {}
            other => panic!("expected RawText, got {other:?}"),
        }
    }

    #[test]
    fn parses_simple_key_value_tags() {
        let line = "INFO 2026-04-15T21:28:30 +0ms service=default providerID=zai-coding-plan modelID=glm-5.1";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(
            record.tags.get("service").map(String::as_str),
            Some("default")
        );
        assert_eq!(
            record.tags.get("providerID").map(String::as_str),
            Some("zai-coding-plan")
        );
        assert_eq!(
            record.tags.get("modelID").map(String::as_str),
            Some("glm-5.1")
        );
        assert_eq!(record.message, "");
    }

    #[test]
    fn parses_inline_json_tag() {
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429}]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        let error_value = record
            .tags
            .get("error")
            .expect("error tag should be captured");
        assert!(error_value.contains("AI_RetryError"), "{error_value}");
        assert!(error_value.contains("AI_APICallError"), "{error_value}");
    }

    #[test]
    fn error_equals_is_terminal_to_end_of_line() {
        let line = "ERROR 2026-04-15T21:28:30 +1ms service=config error=some raw failure text here";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(
            record.tags.get("error").map(String::as_str),
            Some("some raw failure text here"),
        );
    }

    #[test]
    fn err_captures_trailing_failed_to_load_command() {
        let line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/Users/ken/.config/opencode/commands/catalog.md err=ENOENT: no such file or directory, open '/Users/ken/.config/opencode/commands/catalog.md' failed to load command";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        let err = record.tags.get("err").expect("err tag should be captured");
        assert!(err.ends_with("failed to load command"), "{err}");
        assert_eq!(
            record.tags.get("command").map(String::as_str),
            Some("/Users/ken/.config/opencode/commands/catalog.md"),
        );
    }

    #[test]
    fn classifies_malformed_command() {
        let line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/tmp/foo.md err=ENOENT: no such file or directory failed to load command";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::MalformedAsset {
                asset_type,
                path,
                error,
            } => {
                assert_eq!(asset_type, AssetType::Command);
                assert_eq!(path.as_deref(), Some("/tmp/foo.md"));
                assert!(error.contains("failed to load command"));
            }
            other => panic!("expected MalformedAsset, got {other:?}"),
        }
    }

    #[test]
    fn classifies_malformed_skill_and_agent() {
        let skill_line = "ERROR 2026-04-15T21:28:30 +0ms service=config skill=/tmp/s.md err=ENOENT failed to load skill";
        let agent_line = "ERROR 2026-04-15T21:28:30 +0ms service=config agent=/tmp/a.md err=ENOENT failed to load agent";

        for (line, expected) in [
            (skill_line, AssetType::Skill),
            (agent_line, AssetType::Agent),
        ] {
            let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
                panic!("expected Structured for {line}");
            };
            match classify(&record) {
                LogClassification::MalformedAsset { asset_type, .. } => {
                    assert_eq!(asset_type, expected);
                }
                other => panic!("expected MalformedAsset, got {other:?}"),
            }
        }
    }

    #[test]
    fn classifies_rate_limit_with_reset_time() {
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::RateLimit {
                status_code,
                error_name,
                reset_at,
                ..
            } => {
                assert_eq!(status_code, 429);
                assert_eq!(error_name, "AI_RetryError");
                let reset = reset_at.expect("reset_at should be parsed");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-04-16 04:18:56"
                );
            }
            other => panic!("expected RateLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_api_failure_when_not_rate_limited() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","message":"upstream boom","statusCode":500}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure {
                status_code,
                error_name,
                message,
            } => {
                assert_eq!(status_code, Some(500));
                assert_eq!(error_name, "AI_APICallError");
                assert_eq!(message, "AI_APICallError (500): upstream boom");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn api_failure_message_strips_request_body() {
        let line = r#"ERROR 2026-04-21T19:30:26 +49275ms service=llm providerID=zai-coding-plan modelID=glm-5.1 session.id=ses_24e7a9448ffeyo7E2zcOHvsiOn small=false agent=explore mode=subagent error={"error":{"name":"AI_APICallError","url":"https://api.z.ai/api/coding/paas/v4/chat/completions","requestBodyValues":{"model":"glm-5.1","max_tokens":32000,"thinking":{"type":"enabled","clear_thinking":false},"messages":[{"role":"system","content":"You are a file search specialist with a very long system prompt that goes on and on"}]},"statusCode":400,"responseBody":"{\"error\":{\"code\":\"invalid_request\",\"message\":\"model does not support thinking\"}}"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure {
                status_code,
                error_name,
                message,
            } => {
                assert_eq!(status_code, Some(400));
                assert_eq!(error_name, "AI_APICallError");
                assert!(
                    !message.contains("system prompt"),
                    "message should not contain the request body: {message}"
                );
                assert!(
                    !message.contains("requestBodyValues"),
                    "message should not contain requestBodyValues: {message}"
                );
                assert!(
                    message.contains("model does not support thinking"),
                    "message should contain provider error: {message}"
                );
                assert_eq!(message, "AI_APICallError (400): model does not support thinking");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn auth_failure_message_is_concise() {
        let line = r#"ERROR 2026-04-15T19:26:02 +5ms service=llm error={"error":{"name":"AuthenticationError","message":"Invalid API key"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::AuthFailure { message } => {
                assert_eq!(message, "AuthenticationError: Invalid API key");
            }
            other => panic!("expected AuthFailure, got {other:?}"),
        }
    }

    #[test]
    fn classifies_uncaught_from_type_error_record() {
        let line = r#"ERROR 2026-04-15T21:28:30 +33ms service=default name=TypeError message=U.split is not a function stack=TypeError: U.split is not a function fatal"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::UncaughtError { .. } => {}
            other => panic!("expected UncaughtError, got {other:?}"),
        }
    }

    #[test]
    fn classifies_unknown_records_as_unclassified() {
        let line = "INFO 2026-04-15T21:28:30 +0ms service=default msg=hello";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(classify(&record), LogClassification::Unclassified);
    }

    #[test]
    fn classify_raw_matches_ansi_error_prefix() {
        let ansi_line = "\u{1b}[91m\u{1b}[1mError: \u{1b}[0mUnexpected error, check log file";
        match classify_raw(ansi_line) {
            LogClassification::UncaughtError { raw_text } => assert_eq!(raw_text, ansi_line),
            other => panic!("expected UncaughtError, got {other:?}"),
        }
    }

    #[test]
    fn classify_raw_ignores_plain_text() {
        assert_eq!(
            classify_raw("just some chatter"),
            LogClassification::Unclassified,
        );
    }

    #[test]
    fn tolerates_unknown_tags() {
        let line = "ERROR 2026-04-15T21:28:30 +0ms service=default brand_new_tag=hello other=world";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(
            record.tags.get("brand_new_tag").map(String::as_str),
            Some("hello"),
        );
        assert_eq!(record.tags.get("other").map(String::as_str), Some("world"));
    }

    #[test]
    fn preserves_raw_line() {
        let line = "ERROR 2026-04-15T21:28:30 +0ms service=default msg=ok";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(record.raw, line);
    }

    #[test]
    fn parses_timestamp_as_utc() {
        let line = "ERROR 2026-04-15T21:28:30 +0ms service=default msg=ok";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(
            record.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2026-04-15T21:28:30Z",
        );
    }

    #[test]
    fn header_body_is_optional() {
        let line = "INFO 2026-04-15T21:28:30 +0ms";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert!(record.tags.is_empty());
        assert_eq!(record.message, "");
    }

    /// Trailing bare tokens after the last tag become part of that tag's
    /// value rather than a separate `message`. The classifier still picks
    /// up the `fatal` keyword either way.
    #[test]
    fn trailing_bare_token_stays_with_last_tag_value() {
        let line = "ERROR 2026-04-15T21:28:30 +0ms service=default name=TypeError fatal";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(
            record.tags.get("name").map(String::as_str),
            Some("TypeError fatal"),
        );
        assert_eq!(record.message, "");
        assert!(matches!(
            classify(&record),
            LogClassification::UncaughtError { .. }
        ));
    }

    #[test]
    fn rate_limit_without_reset_still_classifies() {
        let line = r#"ERROR 2026-04-15T19:26:02 +10ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::RateLimit { reset_at, .. } => {
                assert!(reset_at.is_none());
            }
            other => panic!("expected RateLimit, got {other:?}"),
        }
    }

    /// Integration smoke test over the bundled fixtures - guards against
    /// the `err=... failed to load command` nuance regressing silently.
    #[test]
    fn fixture_malformed_assets_each_line_classifies() {
        let fixture = include_str!("../../../tests/fixtures/logs/opencode-malformed-assets.txt");
        let mut counts = (0usize, 0usize, 0usize);
        for line in fixture.lines() {
            let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
                panic!("fixture line did not parse: {line}");
            };
            match classify(&record) {
                LogClassification::MalformedAsset {
                    asset_type: AssetType::Command,
                    ..
                } => counts.0 += 1,
                LogClassification::MalformedAsset {
                    asset_type: AssetType::Skill,
                    ..
                } => counts.1 += 1,
                LogClassification::MalformedAsset {
                    asset_type: AssetType::Agent,
                    ..
                } => counts.2 += 1,
                other => panic!("expected MalformedAsset, got {other:?}"),
            }
        }
        assert_eq!(counts, (4, 1, 1));
    }

    #[test]
    fn fixture_rate_limit_classifies() {
        let fixture = include_str!("../../../tests/fixtures/logs/opencode-rate-limit.txt");
        let line = fixture
            .lines()
            .next()
            .expect("rate limit fixture has at least one line");
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("rate limit fixture failed to parse");
        };
        match classify(&record) {
            LogClassification::RateLimit { reset_at, .. } => {
                let reset = reset_at.expect("reset_at should be parsed from fixture");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-04-16 04:18:56",
                );
            }
            other => panic!("expected RateLimit, got {other:?}"),
        }
    }

    #[test]
    fn fixture_uncaught_error_classifies_first_line() {
        let fixture = include_str!("../../../tests/fixtures/logs/opencode-uncaught-error.txt");
        let first = fixture
            .lines()
            .next()
            .expect("uncaught error fixture has at least one line");
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(first) else {
            panic!("first line did not parse: {first}");
        };
        match classify(&record) {
            LogClassification::UncaughtError { .. } => {}
            other => panic!("expected UncaughtError, got {other:?}"),
        }
    }

    #[test]
    fn fixture_mixed_handles_all_shapes() {
        let fixture = include_str!("../../../tests/fixtures/logs/opencode-mixed.txt");
        let mut rate_limit_seen = false;
        let mut malformed_seen = false;
        let mut raw_error_seen = false;
        let mut raw_passthrough_seen = false;

        for line in fixture.lines() {
            match parse_line(line) {
                ParsedOpenCodeStderrLine::Structured(record) => match classify(&record) {
                    LogClassification::RateLimit { .. } => rate_limit_seen = true,
                    LogClassification::MalformedAsset { .. } => malformed_seen = true,
                    _ => {}
                },
                ParsedOpenCodeStderrLine::RawText(raw) => match classify_raw(&raw) {
                    LogClassification::UncaughtError { .. } => raw_error_seen = true,
                    LogClassification::Unclassified => raw_passthrough_seen = true,
                    _ => {}
                },
            }
        }

        assert!(rate_limit_seen, "rate limit line should classify");
        assert!(malformed_seen, "malformed asset line should classify");
        assert!(raw_error_seen, "raw ANSI error line should classify");
        assert!(
            raw_passthrough_seen,
            "unstructured chatter should be unclassified raw text",
        );
    }

    // -----------------------------------------------------------------
    // OpenCodeLogBridge tests
    // -----------------------------------------------------------------

    #[derive(Default)]
    struct RecordingSink {
        events: Vec<SemanticEvent>,
    }

    impl SemanticEventSink for RecordingSink {
        fn on_semantic_event(&mut self, event: SemanticEvent) {
            self.events.push(event);
        }
    }

    fn stdout_seen() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(true))
    }

    fn stdout_unseen() -> Arc<AtomicBool> {
        Arc::new(AtomicBool::new(false))
    }

    fn assert_string(extra: &Value, key: &str, expected: &str) {
        let actual = extra
            .get(key)
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("missing {key} in extra: {extra}"));
        assert_eq!(actual, expected, "extra.{key} mismatch: {extra}");
    }

    #[test]
    fn unclassified_structured_line_returns_not_consumed() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let outcome = bridge.ingest("INFO 2026-04-15T21:28:30 +0ms service=default msg=hello");
        assert_eq!(outcome, StderrIngestOutcome::NotConsumed);
        assert_eq!(bridge.sink.events.len(), 0);
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.log_records_parsed, 1);
    }

    #[test]
    fn unstructured_noise_returns_not_consumed_and_is_not_counted() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let outcome = bridge.ingest("just some chatter");
        assert_eq!(outcome, StderrIngestOutcome::NotConsumed);
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.log_records_parsed, 0);
        assert_eq!(state.diagnostics.uncaught_errors, 0);
    }

    #[test]
    fn malformed_command_emits_warning_and_consumes() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "ERROR 2026-04-15T21:28:30 +315ms service=config command=/tmp/foo.md err=ENOENT failed to load command";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert!(message.contains("command"), "{message}");
                assert!(message.contains("/tmp/foo.md"), "{message}");
                assert_string(extra, "provider", "opencode");
                assert_string(extra, "source", "stderr_log");
                assert_string(extra, "classification", "malformed_asset");
                assert_string(extra, "asset_type", "command");
                assert_string(extra, "path", "/tmp/foo.md");
                assert_string(extra, "service", "config");
                assert!(
                    extra.get("raw").and_then(Value::as_str).is_some(),
                    "raw must be present in extra: {extra}",
                );
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.malformed_asset_events, 1);
    }

    #[test]
    fn rate_limit_after_stdout_emits_warning_no_early_terminate() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.sink.events.len(), 1);
        match &bridge.sink.events[0] {
            SemanticEvent::Warning { message, extra } => {
                assert!(
                    message.to_lowercase().contains("usage limit"),
                    "{message}"
                );
                // We no longer assert the exact timestamp because it's converted to local time
                assert!(message.contains("2026-04-"), "{message}");
                assert_string(extra, "classification", "rate_limit");
                assert_string(extra, "error_name", "AI_RetryError");
            }
            other => panic!("expected Warning, got {other:?}"),
        }
        assert!(
            rx.try_recv().is_err(),
            "no early-termination signal expected when stdout already seen",
        );
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.rate_limit_events, 1);
        assert!(state.diagnostics.rate_limit_reset_at.is_some());
        assert_eq!(state.rate_limit.as_ref().unwrap().is_throttled, Some(true));
    }

    #[test]
    fn rate_limit_before_stdout_emits_terminal_error_and_early_terminate() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert!(message.to_lowercase().contains("usage limit"));
                assert_string(extra, "classification", "rate_limit");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        match rx.try_recv() {
            Ok(EarlyTermination::RateLimit { message, reset_at }) => {
                assert!(message.to_lowercase().contains("usage limit"));
                let reset = reset_at.expect("reset_at should be forwarded");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-04-16 04:18:56",
                );
            }
            other => panic!("expected EarlyTermination::RateLimit, got {other:?}"),
        }
    }

    #[test]
    fn rate_limit_fires_early_termination_only_once() {
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +10ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        assert!(rx.try_recv().is_ok(), "first rate-limit fires channel");
        assert!(
            rx.try_recv().is_err(),
            "second rate-limit must not re-fire the channel",
        );
    }

    #[test]
    fn auth_failure_emits_terminal_error() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = r#"ERROR 2026-04-15T19:26:02 +5ms service=llm error={"error":{"name":"AuthenticationError","message":"Invalid API key"}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert_string(extra, "classification", "auth_failure");
                assert_eq!(message, "AuthenticationError: Invalid API key");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.auth_failures, 1);
    }

    #[test]
    fn api_failure_emits_terminal_error_with_status_code() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","message":"upstream boom","statusCode":500}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::ApiRemote);
                assert_string(extra, "classification", "api_failure");
                assert_string(extra, "error_name", "AI_APICallError");
                assert_eq!(extra.get("status_code"), Some(&json!(500)));
                assert_eq!(message, "AI_APICallError (500): upstream boom");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.api_failures, 1);
    }

    #[test]
    fn uncaught_structured_error_emits_unknown_error_event() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "ERROR 2026-04-15T21:28:30 +33ms service=default name=TypeError message=U.split is not a function fatal";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                extra,
                ..
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::Unknown);
                assert_string(extra, "classification", "uncaught_error");
                assert_string(extra, "error_name", "TypeError");
            }
            other => panic!("expected Error, got {other:?}"),
        }
        let state = bridge.state.lock().unwrap();
        assert_eq!(state.diagnostics.uncaught_errors, 1);
    }

    #[test]
    fn raw_ansi_error_line_emits_unknown_error_event() {
        let mut bridge = OpenCodeLogBridge::new(RecordingSink::default(), stdout_seen(), None);
        let line = "\u{1b}[91m\u{1b}[1mError: \u{1b}[0mUnexpected error, check log file";
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
        match &bridge.sink.events[0] {
            SemanticEvent::Error {
                terminal,
                kind,
                message,
                extra,
            } => {
                assert!(*terminal);
                assert_eq!(*kind, SemanticErrorKind::Unknown);
                assert!(
                    !message.contains('\u{1b}'),
                    "ANSI must be stripped: {message}"
                );
                assert_string(extra, "classification", "uncaught_error");
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[test]
    fn early_termination_channel_works_without_receiver_dropped_panic() {
        // Receiver is dropped before the bridge ingests; bridge should not
        // panic, just log a warning and continue.
        let (tx, rx) = OpenCodeLogBridge::<RecordingSink>::new_early_terminate_channel();
        drop(rx);
        let mut bridge =
            OpenCodeLogBridge::new(RecordingSink::default(), stdout_unseen(), Some(tx));
        let line = r#"ERROR 2026-04-15T19:26:02 +10ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[]}}"#;
        assert_eq!(bridge.ingest(line), StderrIngestOutcome::Consumed);
    }

    #[test]
    fn merge_rate_limit_prefers_throttled_and_latest_reset() {
        use chrono::TimeZone;
        let older = Utc.with_ymd_and_hms(2026, 4, 16, 1, 0, 0).unwrap();
        let newer = Utc.with_ymd_and_hms(2026, 4, 16, 4, 0, 0).unwrap();
        let existing = RateLimitInfo {
            is_throttled: Some(false),
            retry_after_ms: Some(1000),
            message: Some("old".into()),
            reset_at: Some(older),
        };
        let incoming = RateLimitInfo {
            is_throttled: Some(true),
            retry_after_ms: Some(5000),
            message: Some("new".into()),
            reset_at: Some(newer),
        };
        let merged = merge_rate_limit(Some(existing), incoming);
        assert_eq!(merged.is_throttled, Some(true));
        assert_eq!(merged.retry_after_ms, Some(5000));
        assert_eq!(merged.message.as_deref(), Some("new"));
        assert_eq!(merged.reset_at, Some(newer));
    }

    #[test]
    fn merge_stderr_state_applies_diagnostics_without_emitting_config_badge() {
        use crate::events::Provider;
        use crate::stream::badges::BadgeCategory;
        use crate::stream::summary::StreamExecutionSummary;

        let state = Arc::new(Mutex::new(SharedStderrState {
            diagnostics: StderrDiagnostics {
                log_records_parsed: 3,
                malformed_asset_events: 2,
                ..Default::default()
            },
            rate_limit: None,
        }));
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
            ..Default::default()
        };

        merge_stderr_state_into_summary(&state, &mut summary);

        // Per the 2026-04-18 OpenCode reporting contract, the malformed
        // asset counter is preserved on the summary (so JSONL/dashboards
        // can observe it) but the trailer Config badge is removed —
        // each malformed asset is already surfaced as a per-line Warning.
        let diagnostics = summary
            .stderr_diagnostics
            .as_ref()
            .expect("diagnostics should be attached");
        assert_eq!(diagnostics.log_records_parsed, 3);
        assert_eq!(diagnostics.malformed_asset_events, 2);
        assert!(
            !summary
                .badges
                .iter()
                .any(|b| b.category == BadgeCategory::Config),
            "Config trailer badge must NOT be emitted for malformed assets: {:?}",
            summary.badges,
        );
    }

    #[test]
    fn merge_stderr_state_merges_rate_limit_and_yields_rate_limit_badge() {
        use crate::events::Provider;
        use crate::stream::badges::BadgeCategory;
        use crate::stream::summary::StreamExecutionSummary;
        use chrono::TimeZone;

        let reset = Utc.with_ymd_and_hms(2026, 4, 16, 4, 18, 56).unwrap();
        let state = Arc::new(Mutex::new(SharedStderrState {
            diagnostics: StderrDiagnostics {
                log_records_parsed: 1,
                rate_limit_events: 1,
                rate_limit_reset_at: Some(reset),
                ..Default::default()
            },
            rate_limit: Some(RateLimitInfo {
                is_throttled: Some(true),
                retry_after_ms: None,
                message: Some("Usage limit reached".into()),
                reset_at: Some(reset),
            }),
        }));
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
            ..Default::default()
        };

        merge_stderr_state_into_summary(&state, &mut summary);

        let rate_limit = summary
            .rate_limit
            .as_ref()
            .expect("rate_limit should be merged onto summary");
        assert_eq!(rate_limit.is_throttled, Some(true));
        assert_eq!(rate_limit.reset_at, Some(reset));
        assert!(
            summary
                .badges
                .iter()
                .any(|b| b.category == BadgeCategory::RateLimit),
            "RateLimit badge should be recomputed after merge",
        );
    }

    #[test]
    fn merge_stderr_state_without_records_does_not_attach_diagnostics() {
        use crate::events::Provider;
        use crate::stream::summary::StreamExecutionSummary;

        let state = Arc::new(Mutex::new(SharedStderrState::default()));
        let mut summary = StreamExecutionSummary {
            provider: Provider::OpenCode,
            ..Default::default()
        };

        merge_stderr_state_into_summary(&state, &mut summary);
        assert!(summary.stderr_diagnostics.is_none());
        assert!(summary.rate_limit.is_none());
        assert!(summary.badges.is_empty());
    }
}
