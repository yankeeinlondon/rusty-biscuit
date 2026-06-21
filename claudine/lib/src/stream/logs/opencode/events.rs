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

use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};
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
mod tests {
    use super::*;
    use chrono::Timelike;

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

    #[test]
    fn parses_tags_with_dots_and_hyphens() {
        let line = "ERROR 2026-04-15T21:28:30 +33ms service=llm provider-id=zai session.id=s1 model=k2p6 message";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(record.tags.get("provider-id").unwrap(), "zai");
        assert_eq!(record.tags.get("session.id").unwrap(), "s1");
        assert_eq!(record.tags.get("model").unwrap(), "k2p6 message");
        assert_eq!(record.message, "");
    }

    #[test]
    fn new_format_parses_info_level() {
        let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO service=default run=abc message=tracking";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };

        assert_eq!(record.level, LogLevel::Info);
        assert_eq!(
            record.timestamp.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
            "2026-06-10T16:11:27.352Z",
        );
        assert_eq!(record.delta_ms, 0);
        assert_eq!(
            record.tags.get("service").map(String::as_str),
            Some("default"),
        );
        assert_eq!(record.tags.get("run").map(String::as_str), Some("abc"));
        assert_eq!(
            record.tags.get("message").map(String::as_str),
            Some("tracking"),
        );
    }

    #[test]
    fn new_format_parses_all_levels() {
        for (level_str, expected) in [
            ("DEBUG", LogLevel::Debug),
            ("INFO", LogLevel::Info),
            ("WARN", LogLevel::Warn),
            ("ERROR", LogLevel::Error),
        ] {
            let line = format!(
                "timestamp=2026-06-10T16:11:27.352Z level={level_str} service=default"
            );
            let ParsedOpenCodeStderrLine::Structured(record) = parse_line(&line) else {
                panic!("expected Structured for {level_str}");
            };
            assert_eq!(record.level, expected, "{level_str}");
        }
    }

    #[test]
    fn new_format_timestamp_includes_millis() {
        let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO service=default";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };

        assert_eq!(record.timestamp.timestamp_subsec_millis(), 352);
        assert_eq!(record.timestamp.nanosecond(), 352_000_000);
    }

    #[test]
    fn new_format_preserves_raw_line() {
        let line = "timestamp=2026-06-10T16:11:27.352Z level=WARN service=default run=abc";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };

        assert_eq!(record.raw, line);
    }

    #[test]
    fn new_format_extracts_tags() {
        let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO run=abc service=session id=ses_123 parentID=ses_parent title=My task";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };

        assert_eq!(record.tags.get("run").map(String::as_str), Some("abc"));
        assert_eq!(
            record.tags.get("service").map(String::as_str),
            Some("session"),
        );
        assert_eq!(record.tags.get("id").map(String::as_str), Some("ses_123"));
        assert_eq!(
            record.tags.get("parentID").map(String::as_str),
            Some("ses_parent"),
        );
        assert_eq!(
            record.tags.get("title").map(String::as_str),
            Some("My task"),
        );
    }

    #[test]
    fn new_format_message_tag_captured() {
        let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO message=tracking hash=abc123";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };

        assert_eq!(
            record.tags.get("message").map(String::as_str),
            Some("tracking"),
        );
        assert_eq!(record.tags.get("hash").map(String::as_str), Some("abc123"));
    }

    #[test]
    fn new_format_rejects_non_matching() {
        for line in [
            "timestamp=not-a-date level=INFO service=default",
            "time=2026-06-10T16:11:27.352Z level=INFO service=default",
            "timestamp=2026-06-10T16:11:27.352Z severity=INFO service=default",
        ] {
            match parse_line(line) {
                ParsedOpenCodeStderrLine::RawText(raw) => assert_eq!(raw, line),
                other => panic!("expected RawText for {line:?}, got {other:?}"),
            }
        }
    }

    #[test]
    fn new_format_without_message_tag() {
        let line = "timestamp=2026-06-10T16:11:27.352Z level=INFO run=abc service=session id=ses_123 parentID=ses_parent";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };

        assert_eq!(
            record.tags.get("service").map(String::as_str),
            Some("session"),
        );
        assert_eq!(record.tags.get("id").map(String::as_str), Some("ses_123"));
        assert!(!record.tags.contains_key("message"));
        assert_eq!(record.message, "");
    }

    #[test]
    fn new_format_without_millis() {
        let line = "timestamp=2026-06-10T16:11:27Z level=INFO service=default";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };

        assert_eq!(
            record.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            "2026-06-10T16:11:27Z",
        );
        assert_eq!(record.timestamp.timestamp_subsec_millis(), 0);
    }

    #[test]
    fn mixed_format_stream_parses_both() {
        let lines = [
            (
                "INFO 2026-04-15T21:28:30 +5ms service=default msg=legacy",
                LogLevel::Info,
                "2026-04-15T21:28:30Z",
                5,
                "default",
            ),
            (
                "timestamp=2026-06-10T16:11:27.352Z level=WARN service=session id=ses_new",
                LogLevel::Warn,
                "2026-06-10T16:11:27Z",
                0,
                "session",
            ),
            (
                "ERROR 2026-04-15T21:28:31 +42ms service=llm providerID=legacy modelID=model",
                LogLevel::Error,
                "2026-04-15T21:28:31Z",
                42,
                "llm",
            ),
            (
                "timestamp=2026-06-10T16:11:28Z level=DEBUG service=permission permission=task",
                LogLevel::Debug,
                "2026-06-10T16:11:28Z",
                0,
                "permission",
            ),
        ];

        for (line, expected_level, expected_ts, expected_delta_ms, expected_service) in lines {
            let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
                panic!("expected Structured for {line}");
            };

            assert_eq!(record.level, expected_level, "{line}");
            assert_eq!(
                record.timestamp.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                expected_ts,
                "{line}",
            );
            assert_eq!(record.delta_ms, expected_delta_ms, "{line}");
            assert_eq!(
                record.tags.get("service").map(String::as_str),
                Some(expected_service),
                "{line}",
            );
        }
    }
}
