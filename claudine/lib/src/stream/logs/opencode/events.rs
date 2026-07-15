//! JSONL→event translation for OpenCode stderr logs.
//!
//! Parses both OpenCode stderr header formats: the legacy
//! `LEVEL TIMESTAMP +DELTAms ...` envelope and the newer
//! `timestamp=... level=...` envelope. The free-form `key=value` body is
//! normalized into structured [`OpenCodeLogRecord`]s. Unknown tags are
//! preserved, missing tags are accepted, and any line that does not match
//! either header falls through to [`ParsedOpenCodeStderrLine::RawText`].

use std::collections::BTreeMap;
use std::sync::LazyLock;

use chrono::{DateTime, NaiveDateTime, SecondsFormat, TimeZone, Utc};
use regex::{Captures, Regex};

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

/// Classification of an OpenCode 429 / retry-exhaustion signal.
///
/// Server overload and rate-limiting are distinct conditions on unrelated
/// axes (provider capacity vs. account consumption).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderLimitKind {
    /// The provider's servers are busy. Transient, retryable, not a cap.
    Overloaded,
    /// This account sent requests too fast. Transient, retryable.
    RateLimited,
    /// The account's usage allowance is exhausted. Terminal.
    UsageCap,
    /// A 429 wrapped in `AI_RetryError` / `maxRetriesExceeded` — the call
    /// failed after exhausting retries. Terminal.
    RetriesExhausted,
}

/// Action category derived from a parsed record or raw stderr line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogClassification {
    ProviderLimit {
        status_code: Option<u16>,
        kind: ProviderLimitKind,
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
        is_fatal: bool,
    },
    AuthFailure {
        message: String,
    },
    UncaughtError {
        raw_text: String,
    },
    /// OpenCode boot banner line (e.g. `service=default version=... opencode`).
    BootBanner {
        version: String,
    },
    /// A session was created, optionally with a parentID for subagent child sessions.
    SessionCreated {
        id: String,
        parent_id: Option<String>,
    },
    /// An LLM call was initiated.
    LlmCall {
        provider_id: String,
        model_id: String,
        mode: String,
        is_stream: bool,
    },
    /// A step loop started for a session.
    StepLoop {
        session_id: String,
        step: u32,
    },
    /// A step loop exited for a session.
    StepExit {
        session_id: String,
    },
    /// A permission evaluation was completed.
    PermissionEvaluated {
        permission: String,
        pattern: String,
        action: String,
    },
    /// An HTTP response was sent.
    HttpResponse {
        method: String,
        url: String,
        status: u16,
        duration_ms: u64,
    },
    /// A `service=snapshot` log line. OpenCode emits these from the git-based
    /// file-snapshot subsystem to record success and failure of per-step
    /// snapshot attempts. Surfaced to operators because the message is the
    /// only context (e.g. "failed to add snapshot files"); tag values carry
    /// the file path / reason.
    Snapshot {
        /// The trailing log message — e.g. `"failed to add snapshot files"`.
        /// Empty when the message slot was absorbed into a tag value.
        message: String,
        /// Severity from the parsed record header. The bridge maps `WARN` /
        /// `ERROR` to [`crate::stream::semantic::SemanticEvent::Warning`] and
        /// every other level to [`crate::stream::semantic::SemanticEvent::Info`].
        level: LogLevel,
    },
    Unclassified,
}

impl ProviderLimitKind {
    /// Wire name in the serialized signal payload's `kind` field. Must stay
    /// in lockstep with the `match_value` terms of the opencode
    /// `stderr_promoted` detection records in the signals research corpus.
    fn payload_name(self) -> &'static str {
        match self {
            Self::Overloaded => "Overloaded",
            Self::RateLimited => "RateLimited",
            Self::UsageCap => "UsageCap",
            Self::RetriesExhausted => "RetriesExhausted",
        }
    }
}

impl LogClassification {
    /// The glue-mode signal payload (ratified 2026-07-06): the JSON object
    /// the `stderr_promoted` signal-detection tables match against.
    ///
    /// `ProviderLimit` is discriminated by `kind` (the four
    /// [`ProviderLimitKind`] names); every other variant by `classification`
    /// (the variant name). Struct fields keep their snake_case names;
    /// absent optional fields are omitted, not serialized as `null`, so an
    /// `exists` match never fires on a missing value. `reset_at` serializes
    /// as RFC 3339 UTC because the corpus extraction declares
    /// `unit: iso8601 / zone: utc`.
    ///
    /// `claudine signals check` replays `.txt` fixtures through this
    /// serialization, and the E5 runtime shim feeds the identical payload
    /// from live promoted stderr — this method IS that shim's core.
    pub fn to_signal_payload(&self) -> serde_json::Value {
        use serde_json::{Map, Value, json};
        fn payload(
            classification: &str,
            fields: Vec<(&str, Option<Value>)>,
        ) -> Value {
            let mut map = Map::new();
            map.insert("classification".into(), json!(classification));
            for (key, value) in fields {
                if let Some(value) = value {
                    map.insert(key.into(), value);
                }
            }
            Value::Object(map)
        }
        match self {
            Self::ProviderLimit {
                status_code,
                kind,
                reset_at,
                provider_id,
                model_id,
                provider_error,
            } => payload(
                "ProviderLimit",
                vec![
                    ("kind", Some(json!(kind.payload_name()))),
                    ("status_code", status_code.map(|code| json!(code))),
                    (
                        "reset_at",
                        reset_at.map(|at| {
                            json!(at.to_rfc3339_opts(SecondsFormat::Secs, true))
                        }),
                    ),
                    ("provider_id", provider_id.as_ref().map(|id| json!(id))),
                    ("model_id", model_id.as_ref().map(|id| json!(id))),
                    ("provider_error", Some(json!(provider_error))),
                ],
            ),
            Self::MalformedAsset {
                asset_type,
                path,
                error,
            } => payload(
                "MalformedAsset",
                vec![
                    (
                        "asset_type",
                        Some(json!(super::classify::asset_type_as_str(*asset_type))),
                    ),
                    ("path", path.as_ref().map(|p| json!(p))),
                    ("error", Some(json!(error))),
                ],
            ),
            Self::ApiFailure {
                status_code,
                error_name,
                message,
                is_fatal,
            } => payload(
                "ApiFailure",
                vec![
                    ("status_code", status_code.map(|code| json!(code))),
                    ("error_name", Some(json!(error_name))),
                    ("message", Some(json!(message))),
                    ("is_fatal", Some(json!(is_fatal))),
                ],
            ),
            Self::AuthFailure { message } => {
                payload("AuthFailure", vec![("message", Some(json!(message)))])
            }
            Self::UncaughtError { raw_text } => {
                payload("UncaughtError", vec![("raw_text", Some(json!(raw_text)))])
            }
            Self::BootBanner { version } => {
                payload("BootBanner", vec![("version", Some(json!(version)))])
            }
            Self::SessionCreated { id, parent_id } => payload(
                "SessionCreated",
                vec![
                    ("id", Some(json!(id))),
                    ("parent_id", parent_id.as_ref().map(|id| json!(id))),
                ],
            ),
            Self::LlmCall {
                provider_id,
                model_id,
                mode,
                is_stream,
            } => payload(
                "LlmCall",
                vec![
                    ("provider_id", Some(json!(provider_id))),
                    ("model_id", Some(json!(model_id))),
                    ("mode", Some(json!(mode))),
                    ("is_stream", Some(json!(is_stream))),
                ],
            ),
            Self::StepLoop { session_id, step } => payload(
                "StepLoop",
                vec![
                    ("session_id", Some(json!(session_id))),
                    ("step", Some(json!(step))),
                ],
            ),
            Self::StepExit { session_id } => payload(
                "StepExit",
                vec![("session_id", Some(json!(session_id)))],
            ),
            Self::PermissionEvaluated {
                permission,
                pattern,
                action,
            } => payload(
                "PermissionEvaluated",
                vec![
                    ("permission", Some(json!(permission))),
                    ("pattern", Some(json!(pattern))),
                    ("action", Some(json!(action))),
                ],
            ),
            Self::HttpResponse {
                method,
                url,
                status,
                duration_ms,
            } => payload(
                "HttpResponse",
                vec![
                    ("method", Some(json!(method))),
                    ("url", Some(json!(url))),
                    ("status", Some(json!(status))),
                    ("duration_ms", Some(json!(duration_ms))),
                ],
            ),
            Self::Snapshot { message, level } => payload(
                "Snapshot",
                vec![
                    ("message", Some(json!(message))),
                    (
                        "level",
                        Some(json!(match level {
                            LogLevel::Debug => "debug",
                            LogLevel::Info => "info",
                            LogLevel::Warn => "warn",
                            LogLevel::Error => "error",
                        })),
                    ),
                ],
            ),
            Self::Unclassified => payload("Unclassified", Vec::new()),
        }
    }
}

static HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^(?P<level>DEBUG|INFO|WARN|ERROR)\s+(?P<ts>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2})\s+\+(?P<delta>\d+)ms(?:\s+(?P<body>.*))?$",
    )
    .expect("opencode log header regex must compile")
});

static NEW_HEADER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^timestamp=(?P<ts>\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d{3})?Z)\s+level=(?P<level>DEBUG|INFO|WARN|ERROR)(?:\s+(?P<body>.*))?$",
    )
    .expect("opencode new log header regex must compile")
});

/// Parse a single OpenCode stderr line into either a structured record
/// or a raw passthrough string.
pub fn parse_line(line: &str) -> ParsedOpenCodeStderrLine {
    let trimmed_right = line.trim_end_matches(['\r', '\n']);
    if let Some(caps) = HEADER_RE.captures(trimmed_right) {
        let delta_ms = caps
            .name("delta")
            .and_then(|m| m.as_str().parse::<u64>().ok())
            .unwrap_or(0);

        return parse_captures(line, &caps, delta_ms);
    }

    if let Some(caps) = NEW_HEADER_RE.captures(trimmed_right) {
        return parse_captures(line, &caps, 0);
    }

    ParsedOpenCodeStderrLine::RawText(line.to_string())
}

fn parse_captures(
    line: &str,
    caps: &Captures<'_>,
    delta_ms: u64,
) -> ParsedOpenCodeStderrLine {
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

fn parse_timestamp(value: &str) -> Option<DateTime<Utc>> {
    [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%dT%H:%M:%S%.3fZ",
        "%Y-%m-%dT%H:%M:%SZ",
    ]
    .iter()
    .find_map(|format| {
        NaiveDateTime::parse_from_str(value, format)
            .ok()
            .map(|naive| Utc.from_utc_datetime(&naive))
    })
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
    b.is_ascii_alphanumeric() || b == b'_' || b == b'.' || b == b'-'
}

fn find_json_end(s: &str) -> Option<usize> {
    let mut stream = serde_json::Deserializer::from_str(s).into_iter::<serde_json::Value>();
    stream.next()?.ok()?;
    Some(stream.byte_offset())
}

#[cfg(test)]
mod tests;
