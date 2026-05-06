//! Classification and error extraction for parsed OpenCode log records.
//!
//! Turns parsed [`OpenCodeLogRecord`]s into [`LogClassification`] variants
//! and extracts structured error information from inline JSON payloads.

use std::sync::LazyLock;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use regex::Regex;

use crate::stream::logs::opencode::events::{AssetType, LogClassification, OpenCodeLogRecord};
use crate::stream::summary::RateLimitInfo;

static RESET_AT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"reset at (\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2}:\d{2})")
        .expect("opencode reset-at regex must compile")
});

static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("ansi regex must compile"));

static STATUS_CODE_RES: LazyLock<[Regex; 2]> = LazyLock::new(|| {
    [
        Regex::new(r#""statusCode":(\d{3})"#).expect("status-code regex 1 must compile"),
        Regex::new(r"statusCode=(\d{3})").expect("status-code regex 2 must compile"),
    ]
});

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
        Err(_) => {
            // If it's not valid JSON, return the raw tag (truncated if huge).
            if error_tag.len() > 500 {
                return format!("{}...", &error_tag[..497]);
            }
            return error_tag.to_string();
        }
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
        let desc = get_http_status_description(code as u16);
        if !desc.is_empty() {
            parts.push(format!("{error_name} ({code}: {desc})"));
        } else {
            parts.push(format!("{error_name} ({code})"));
        }
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
        if let Ok(body) = serde_json::from_str::<serde_json::Value>(body_str) {
            if let Some(msg) = body
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|v| v.as_str())
                && !msg.is_empty()
            {
                return Some(msg.to_string());
            }

            if let Some(msg) = body.get("message").and_then(|v| v.as_str())
                && !msg.is_empty()
            {
                return Some(msg.to_string());
            }

            let code = body
                .get("code")
                .and_then(|v| v.as_str().map(|s| s.to_string()))
                .or_else(|| {
                    body.get("code")
                        .and_then(|v| v.as_u64())
                        .map(|v| v.to_string())
                });

            if let Some(code_str) = code {
                let desc = get_provider_code_description(&code_str);
                if !desc.is_empty() {
                    return Some(format!("{code_str}: {desc}"));
                }
            }
        }
        if !body_str.is_empty() {
            return Some(body_str.to_string());
        }
    }

    for source in ["errors", "lastError", "data"] {
        let entries = match source {
            "errors" => envelope.get("errors").and_then(|v| v.as_array()).cloned(),
            _ => envelope.get(source).map(|v| vec![v.clone()]),
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

pub(super) fn get_http_status_description(code: u16) -> &'static str {
    match code {
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Payload Too Large",
        422 => "Unprocessable Entity",
        429 => "Too Many Requests",
        500 => "Internal Server Error",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        _ => "",
    }
}

pub(super) fn get_provider_code_description(code: &str) -> &'static str {
    match code {
        "1210" | "1211" | "1212" => "Invalid or expired API Key",
        "1301" => "System error",
        "1302" => "Input error",
        "1303" => "Service overloaded",
        "1305" => "Request timeout",
        "1308" => "Usage limit reached",
        _ => "",
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
            message: summarize_error_json(record),
        });
    }

    let status_code = extract_status_code(haystack);
    let is_fatal = haystack.contains("AI_RetryError") || haystack.contains("maxRetriesExceeded");

    // It's a rate limit if it has explicit 429/1308 or known substrings,
    // OR if it's a fatal retry failure specifically for a 429.
    let is_rate_limit = status_code == Some(429)
        || haystack.contains("\"code\":\"1308\"")
        || haystack.contains("Usage limit reached")
        || (is_fatal && status_code == Some(429));

    if is_rate_limit {
        let status_code = status_code.unwrap_or(429);
        let error_name = if is_fatal {
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
            is_fatal,
        });
    }

    if haystack.contains("AI_APICallError") || is_fatal {
        let mut message = summarize_error_json(record);
        if message.is_empty() {
            // Try to find a tag that contains the error name.
            let name_to_find = if is_fatal {
                "AI_RetryError"
            } else {
                "AI_APICallError"
            };
            for (key, value) in &record.tags {
                if key != "service"
                    && key != "providerID"
                    && key != "modelID"
                    && value.contains(name_to_find)
                {
                    message = value.clone();
                    break;
                }
            }
        }
        if message.is_empty() && !record.message.is_empty() {
            message = record.message.clone();
        }

        return Some(LogClassification::ApiFailure {
            status_code,
            error_name: if is_fatal {
                "AI_RetryError"
            } else {
                "AI_APICallError"
            }
            .to_string(),
            message,
            is_fatal,
        });
    }

    None
}

fn contains_any_ci(haystack: &str, needles: &[&str]) -> bool {
    let lowered = haystack.to_lowercase();
    needles.iter().any(|n| lowered.contains(&n.to_lowercase()))
}

fn extract_status_code(haystack: &str) -> Option<u16> {
    for re in STATUS_CODE_RES.iter() {
        if let Some(caps) = re.captures(haystack)
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

pub(super) fn strip_ansi(line: &str) -> String {
    ANSI_RE.replace_all(line, "").into_owned()
}

pub(super) fn asset_type_as_str(asset_type: AssetType) -> &'static str {
    match asset_type {
        AssetType::Skill => "skill",
        AssetType::Command => "command",
        AssetType::Agent => "agent",
        AssetType::Config => "config",
        AssetType::Unknown => "unknown",
    }
}

pub(super) fn render_rate_limit_message(
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
            let local_time = reset.with_timezone(&chrono::Local);
            format!(
                "Usage limit reached{target}; resets at {}",
                local_time.format("%Y-%m-%d %H:%M:%S")
            )
        }
        None => format!("Usage limit reached{target}"),
    }
}

pub(super) fn render_malformed_asset_message(asset_type: AssetType, path: Option<&str>) -> String {
    let noun = asset_type_as_str(asset_type);
    match path {
        Some(p) => format!("Skipped malformed OpenCode {noun}: {p}"),
        None => format!("Skipped malformed OpenCode {noun}"),
    }
}

pub(super) fn max_reset_at(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stream::logs::opencode::events::*;

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
                ..
            } => {
                assert_eq!(status_code, Some(500));
                assert_eq!(error_name, "AI_APICallError");
                assert_eq!(message, "AI_APICallError (500: Internal Server Error): upstream boom");
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
                ..
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
                assert_eq!(
                    message,
                    "AI_APICallError (400: Bad Request): model does not support thinking"
                );
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
            LogClassification::ApiFailure { is_fatal, .. } => {
                assert!(is_fatal);
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    /// Integration smoke test over the bundled fixtures - guards against
    /// the `err=... failed to load command` nuance regressing silently.
    #[test]
    fn fixture_malformed_assets_each_line_classifies() {
        let fixture = include_str!("../../../../tests/fixtures/logs/opencode-malformed-assets.txt");
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
        let fixture = include_str!("../../../../tests/fixtures/logs/opencode-rate-limit.txt");
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
        let fixture = include_str!("../../../../tests/fixtures/logs/opencode-uncaught-error.txt");
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
        let fixture = include_str!("../../../../tests/fixtures/logs/opencode-mixed.txt");
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
    fn extract_status_code_finds_json_variant() {
        assert_eq!(extract_status_code(r#""statusCode":429"#), Some(429));
        assert_eq!(extract_status_code(r#""statusCode":500"#), Some(500));
    }

    #[test]
    fn extract_status_code_finds_key_value_variant() {
        assert_eq!(extract_status_code("statusCode=429"), Some(429));
        assert_eq!(extract_status_code("statusCode=503"), Some(503));
    }

    #[test]
    fn extract_status_code_prefers_first_match() {
        // When both patterns appear, the first (JSON) match wins
        let haystack = r#""statusCode":200 statusCode=500"#;
        assert_eq!(extract_status_code(haystack), Some(200));
    }

    #[test]
    fn classifies_api_failure_with_status_description() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":502}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError (502: Bad Gateway)");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn extracts_zai_code_from_response_body() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","responseBody":"{\"code\":1301,\"message\":\"internal server error\"}"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError: internal server error");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn extracts_zai_description_when_message_missing() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","responseBody":"{\"code\":1305}"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError: 1305: Request timeout");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn preserves_non_json_error_tag() {
        let line = "ERROR 2026-04-15T19:26:02 +100ms service=llm error=AI_APICallError: something went wrong on the server";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError: something went wrong on the server");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn api_failure_falls_back_to_record_message() {
        let line = "ERROR 2026-04-15T19:26:02 +100ms service=llm dummy=tag AI_APICallError: connection reset";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "tag AI_APICallError: connection reset");
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn extract_status_code_returns_none_for_missing_code() {
        assert_eq!(extract_status_code("no status here"), None);
        assert_eq!(extract_status_code(""), None);
        assert_eq!(extract_status_code("statusCode=99"), None); // too short
        assert_eq!(extract_status_code("statusCode=9999"), Some(999)); // matches first 3 digits
        assert_eq!(extract_status_code("other=500"), None); // wrong key
    }
}
