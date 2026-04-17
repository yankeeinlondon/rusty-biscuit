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

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
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

    let Some(level) = caps.name("level").and_then(|m| LogLevel::from_str(m.as_str())) else {
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

    let service = record
        .tags
        .get("service")
        .map(|s| s.as_str())
        .unwrap_or("");

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
        if (bytes[cursor] == b' ' || bytes[cursor] == b'\t')
            && is_tag_boundary(bytes, cursor + 1)
        {
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

fn classify_llm_failure(record: &OpenCodeLogRecord, service: &str) -> Option<LogClassification> {
    let haystack = record.raw.as_str();

    if contains_any_ci(
        haystack,
        &["AuthenticationError", "unauthorized", "Unauthorized"],
    ) || (service == "llm" && haystack.contains("fetch failed"))
    {
        return Some(LogClassification::AuthFailure {
            message: haystack.to_string(),
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
        let provider_error = record
            .tags
            .get("error")
            .cloned()
            .unwrap_or_else(|| haystack.to_string());
        return Some(LogClassification::RateLimit {
            status_code,
            error_name,
            reset_at,
            provider_error,
        });
    }

    if haystack.contains("AI_APICallError") {
        return Some(LogClassification::ApiFailure {
            status_code: extract_status_code(haystack),
            error_name: "AI_APICallError".to_string(),
            message: haystack.to_string(),
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
                    assert_eq!(record.tags.get("service").map(String::as_str), Some("default"));
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
        assert_eq!(record.tags.get("service").map(String::as_str), Some("default"));
        assert_eq!(
            record.tags.get("providerID").map(String::as_str),
            Some("zai-coding-plan")
        );
        assert_eq!(record.tags.get("modelID").map(String::as_str), Some("glm-5.1"));
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

        for (line, expected) in [(skill_line, AssetType::Skill), (agent_line, AssetType::Agent)] {
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
                assert_eq!(reset.format("%Y-%m-%d %H:%M:%S").to_string(), "2026-04-16 04:18:56");
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
                ..
            } => {
                assert_eq!(status_code, Some(500));
                assert_eq!(error_name, "AI_APICallError");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn classifies_auth_failure() {
        let line = r#"ERROR 2026-04-15T19:26:02 +5ms service=llm error={"error":{"name":"AuthenticationError","message":"Invalid API key"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::AuthFailure { .. } => {}
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
}
