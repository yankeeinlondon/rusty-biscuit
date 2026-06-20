//! Classification and error extraction for parsed OpenCode log records.
//!
//! Turns parsed [`OpenCodeLogRecord`]s into [`LogClassification`] variants
//! and extracts structured error information from inline JSON payloads.

use std::sync::LazyLock;

use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc};
use regex::Regex;
use tracing::debug;

use crate::stream::logs::opencode::events::{AssetType, LogClassification, OpenCodeLogRecord, ProviderLimitKind};
use crate::stream::summary::RateLimitInfo;

static RESET_AT_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"reset at (\d{4}-\d{2}-\d{2})[ T](\d{2}:\d{2}:\d{2})")
        .expect("opencode reset-at regex must compile")
});

static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("ansi regex must compile"));

static STATUS_CODE_RES: LazyLock<[Regex; 2]> = LazyLock::new(|| {
    [
        Regex::new(r#""statusCode":(\d{3})(?:\D|$)"#).expect("status-code regex 1 must compile"),
        Regex::new(r"statusCode=(\d{3})(?:\D|$)").expect("status-code regex 2 must compile"),
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

    if let Some(classification) = classify_lifecycle(record) {
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
        Err(err) => {
            // If it's not valid JSON, return the raw tag (truncated if huge).
            debug!(%err, "opencode error tag not valid JSON; falling back to raw");
            if error_tag.len() > 500 {
                let truncated: String = error_tag.chars().take(497).collect();
                return format!("{truncated}...");
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
        let desc = u16::try_from(code)
            .ok()
            .map(get_http_status_description)
            .filter(|d| !d.is_empty())
            .unwrap_or_default();
        if desc.is_empty() {
            parts.push(format!("{error_name} ({code})"));
        } else {
            parts.push(format!("{error_name} ({code}: {desc})"));
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
        match serde_json::from_str::<serde_json::Value>(body_str) {
            Ok(body) => {
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
            Err(err) => {
                debug!(%err, "opencode responseBody not valid JSON; falling back to raw");
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
    let is_retry_exhausted =
        haystack.contains("AI_RetryError") || haystack.contains("maxRetriesExceeded");
    // Kimi reports its billing-cycle cap as HTTP 403 / `permission_error`
    // with "reached your usage limit for this billing cycle" — a dialect the
    // ZAI-style needles above do not cover.
    let has_cap = haystack.contains("\"code\":\"1308\"")
        || haystack.contains("exceeded_current_quota_error")
        || haystack.contains("Usage limit reached")
        || haystack.contains("reached your usage limit")
        || haystack.contains("billing cycle");
    let is_overload = contains_any_ci(haystack, &["overload", "engine_overloaded_error"]);
    let has_error_context = record.tags.contains_key("error");

    // Resolution order is critical — cap-with-context wins over retries-exhausted.
    // 1. Cap signal present with error tag → terminal usage cap.
    // ProviderLimitKind is authoritative; preserve the real HTTP code when
    // available instead of stamping a 429 sentinel onto a 403 billing cap.
    if has_cap && has_error_context {
        let reset_at = extract_reset_at(haystack);
        let provider_id = record.tags.get("providerID").cloned();
        let model_id = record.tags.get("modelID").cloned();
        let provider_error = record
            .tags
            .get("error")
            .cloned()
            .unwrap_or_else(|| haystack.to_string());

        return Some(LogClassification::ProviderLimit {
            status_code,
            kind: ProviderLimitKind::UsageCap,
            reset_at,
            provider_id,
            model_id,
            provider_error,
        });
    }

    // 2. Retry exhaustion wrapping a 429 → terminal retries exhausted.
    if status_code == Some(429) && is_retry_exhausted {
        let reset_at = extract_reset_at(haystack);
        let provider_id = record.tags.get("providerID").cloned();
        let model_id = record.tags.get("modelID").cloned();
        let provider_error = record
            .tags
            .get("error")
            .cloned()
            .unwrap_or_else(|| haystack.to_string());

        return Some(LogClassification::ProviderLimit {
            status_code: Some(429),
            kind: ProviderLimitKind::RetriesExhausted,
            reset_at,
            provider_id,
            model_id,
            provider_error,
        });
    }

    // 3. Cap signal without error tag -> advisory non-fatal ApiFailure.
    if has_cap && !has_error_context {
        let mut message = None;
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&record.message) {
            message = extract_provider_message(&val);
        }
        if message.is_none() && !record.message.is_empty() {
            message = Some(record.message.clone());
        }
        let message = message.unwrap_or_default();

        return Some(LogClassification::ApiFailure {
            status_code,
            error_name: "AI_APICallError".to_string(),
            message,
            is_fatal: false,
        });
    }

    // 4. Plain 429 overload → transient overloaded.
    if status_code == Some(429) && is_overload {
        let reset_at = extract_reset_at(haystack);
        let provider_id = record.tags.get("providerID").cloned();
        let model_id = record.tags.get("modelID").cloned();
        let provider_error = record
            .tags
            .get("error")
            .cloned()
            .unwrap_or_else(|| haystack.to_string());

        return Some(LogClassification::ProviderLimit {
            status_code: Some(429),
            kind: ProviderLimitKind::Overloaded,
            reset_at,
            provider_id,
            model_id,
            provider_error,
        });
    }

    // 5. Plain 429 -> transient rate-limited.
    if status_code == Some(429) {
        let reset_at = extract_reset_at(haystack);
        let provider_id = record.tags.get("providerID").cloned();
        let model_id = record.tags.get("modelID").cloned();
        let provider_error = record
            .tags
            .get("error")
            .cloned()
            .unwrap_or_else(|| haystack.to_string());

        return Some(LogClassification::ProviderLimit {
            status_code: Some(429),
            kind: ProviderLimitKind::RateLimited,
            reset_at,
            provider_id,
            model_id,
            provider_error,
        });
    }

    // Anything else that looks like an API or retry failure.
    if haystack.contains("AI_APICallError") || is_retry_exhausted {
        let mut message = summarize_error_json(record);
        if message.is_empty() {
            // Try to find a tag that contains the error name.
            let name_to_find = if is_retry_exhausted {
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
            error_name: if is_retry_exhausted {
                "AI_RetryError"
            } else {
                "AI_APICallError"
            }
            .to_string(),
            message,
            is_fatal: is_retry_exhausted,
        });
    }

    None
}

fn classify_lifecycle(record: &OpenCodeLogRecord) -> Option<LogClassification> {
    let service = record.tags.get("service").map(|s| s.as_str()).unwrap_or("");
    let inferred_service = if service.is_empty() {
        infer_service_from_message(record)
    } else {
        service
    };
    let message = record.message.as_str();

    match inferred_service {
        "default" => classify_default_service(record, message),
        "session" => classify_session(record, message),
        "llm" => classify_llm_call(record, message),
        "session.prompt" => classify_session_prompt(record, message),
        "permission" => classify_permission(record, message),
        "snapshot" => Some(LogClassification::Snapshot {
            message: message.to_string(),
            level: record.level,
        }),
        _ => None,
    }
}

/// Infer the `service` tag value from the `message` tag and required sibling
/// tags when `service` is absent.
///
/// OpenCode's new stderr format omits `service=` for many lifecycle records.
/// The `message` tag still carries the trailing keyword that identifies the
/// lifecycle class (`loop`, `stream`, `evaluated`, `created`, `opencode`,
/// `Sent HTTP response`, `exiting loop`), and the required context tags
/// (`session.id`, `step`, `providerID`, `modelID`, `permission`, `id`,
/// `version`) are present.  This helper maps those observed shapes back to
/// the service values the dedicated classifiers expect.
fn infer_service_from_message(record: &OpenCodeLogRecord) -> &'static str {
    let msg = record
        .tags
        .get("message")
        .map(|s| s.trim_matches('"'))
        .unwrap_or("");

    match msg {
        "loop" | "exiting loop"
            if record.tags.contains_key("session.id")
                && record.tags.contains_key("step") =>
        {
            "session.prompt"
        }
        "exiting loop" if record.tags.contains_key("session.id") => "session.prompt",
        "stream"
            if record.tags.contains_key("providerID")
                && record.tags.contains_key("modelID") =>
        {
            "llm"
        }
        "evaluated" if record.tags.contains_key("permission") => "permission",
        "created" if record.tags.contains_key("id") => "session",
        "opencode" if record.tags.contains_key("version") => "default",
        "Sent HTTP response" if record.tags.contains_key("http.method") => "default",
        _ => "",
    }
}

/// True if `record.message` equals `keyword` exactly, or any tag value ends
/// with `" {keyword}"`.
///
/// The OpenCode stderr body parser greedily extracts bare values up to the
/// next `key=` boundary; when the last `key=value` pair is followed by a
/// trailing bare-word log message, those words are absorbed into the last
/// tag's value instead of becoming `record.message`. This helper papers
/// over that quirk for lifecycle classifications keyed off the trailing
/// log-message keyword.
fn has_trailing_keyword(record: &OpenCodeLogRecord, keyword: &str) -> bool {
    if record.message == keyword {
        return true;
    }
    if record
        .tags
        .get("message")
        .map(|value| value.trim_matches('"') == keyword)
        .unwrap_or(false)
    {
        return true;
    }
    let suffix = format!(" {keyword}");
    record.tags.values().any(|v| v.ends_with(&suffix))
}

/// Fetch a tag value with any trailing ` keyword` suffix stripped.
///
/// Mirror of [`has_trailing_keyword`]: when the body parser absorbed the
/// trailing log-message keyword into a tag value, downstream classifiers
/// need the clean value (e.g. `mode=primary` rather than `mode=primary stream`).
fn tag_value_stripped(record: &OpenCodeLogRecord, key: &str, keyword: &str) -> Option<String> {
    let value = record.tags.get(key)?;
    let suffix = format!(" {keyword}");
    Some(value.strip_suffix(&suffix).unwrap_or(value).to_string())
}

fn classify_default_service(
    record: &OpenCodeLogRecord,
    _message: &str,
) -> Option<LogClassification> {
    // Boot banner: trailing message is exactly "opencode" and has a version tag.
    if has_trailing_keyword(record, "opencode")
        && let Some(version) = record.tags.get("version").cloned()
    {
        return Some(LogClassification::BootBanner { version });
    }

    // HTTP response: trailing message is "Sent HTTP response" with http.* tags.
    if has_trailing_keyword(record, "Sent HTTP response") {
        let method = record.tags.get("http.method").cloned().unwrap_or_default();
        let url = record.tags.get("http.url").cloned().unwrap_or_default();
        let status = tag_value_stripped(record, "http.status", "Sent HTTP response")
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);
        // Duration is in a tag like logSpan.http.span.4=<Nms>; find the first one.
        let duration_ms = record
            .tags
            .iter()
            .find_map(|(k, v)| {
                if k.starts_with("logSpan.http.span.") {
                    let cleaned = v.strip_suffix(" Sent HTTP response").unwrap_or(v);
                    cleaned.trim_end_matches("ms").parse::<u64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        return Some(LogClassification::HttpResponse {
            method,
            url,
            status,
            duration_ms,
        });
    }

    None
}

fn classify_session(record: &OpenCodeLogRecord, _message: &str) -> Option<LogClassification> {
    if !has_trailing_keyword(record, "created") {
        return None;
    }
    let id = record.tags.get("id").cloned()?;
    let parent_id = record.tags.get("parentID").cloned();
    Some(LogClassification::SessionCreated { id, parent_id })
}

fn classify_llm_call(record: &OpenCodeLogRecord, _message: &str) -> Option<LogClassification> {
    // Only match successful stream starts, not errors ("stream error" is handled
    // by the existing classify_llm_failure path).
    if !has_trailing_keyword(record, "stream") {
        return None;
    }
    let provider_id = record.tags.get("providerID").cloned()?;
    let model_id = record.tags.get("modelID").cloned()?;
    let mode = tag_value_stripped(record, "mode", "stream").unwrap_or_default();
    Some(LogClassification::LlmCall {
        provider_id,
        model_id,
        mode,
        is_stream: true,
    })
}

fn classify_session_prompt(
    record: &OpenCodeLogRecord,
    _message: &str,
) -> Option<LogClassification> {
    let session_id = record.tags.get("session.id").cloned()?;

    // Check the longer keyword first; "exiting loop" ends with " loop"
    // and would otherwise match the shorter StepLoop branch.
    if has_trailing_keyword(record, "exiting loop") {
        return Some(LogClassification::StepExit { session_id });
    }

    if has_trailing_keyword(record, "loop") {
        let step = record
            .tags
            .get("step")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        return Some(LogClassification::StepLoop { session_id, step });
    }

    None
}

fn classify_permission(record: &OpenCodeLogRecord, _message: &str) -> Option<LogClassification> {
    if !has_trailing_keyword(record, "evaluated") {
        return None;
    }
    let permission = record.tags.get("permission").cloned().unwrap_or_default();
    let pattern = record.tags.get("pattern").cloned().unwrap_or_default();
    let action = record.tags.get("action").cloned().unwrap_or_default();
    Some(LogClassification::PermissionEvaluated {
        permission,
        pattern,
        action,
    })
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
    fn classifies_usage_cap_with_reset_time() {
        let line = r#"ERROR 2026-04-15T19:26:02 +3054ms service=llm providerID=zai-coding-plan modelID=glm-5.1 error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached. Your limit will reset at 2026-04-16 04:18:56\"}}"}]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                reset_at,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::UsageCap);
                let reset = reset_at.expect("reset_at should be parsed");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-04-16 04:18:56"
                );
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
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
                assert_eq!(
                    message,
                    "AI_APICallError (500: Internal Server Error): upstream boom"
                );
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

    fn parse_new_format_record(line: &str) -> OpenCodeLogRecord {
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured for {line}");
        };
        record
    }

    #[test]
    fn new_format_classifies_session_created() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:27.352Z level=INFO service=session id=ses_primary title=Primary session created",
        );

        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_primary");
                assert_eq!(parent_id, None);
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_session_created_subagent() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:28.000Z level=INFO service=session id=ses_child parentID=ses_parent title=Child task created",
        );

        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_child");
                assert_eq!(parent_id.as_deref(), Some("ses_parent"));
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_llm_call() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:29.000Z level=INFO service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_primary small=false agent=build mode=primary stream",
        );

        match classify(&record) {
            LogClassification::LlmCall {
                provider_id,
                model_id,
                mode,
                is_stream,
            } => {
                assert_eq!(provider_id, "kimi-for-coding");
                assert_eq!(model_id, "k2p6");
                assert_eq!(mode, "primary");
                assert!(is_stream);
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_step_loop() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:30.000Z level=INFO service=session.prompt session.id=ses_primary step=2 logSpan.http.span.4=55ms message=loop",
        );

        match classify(&record) {
            LogClassification::StepLoop { session_id, step } => {
                assert_eq!(session_id, "ses_primary");
                assert_eq!(step, 2);
            }
            other => panic!("expected StepLoop, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_step_exit() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:31.000Z level=INFO service=session.prompt session.id=ses_primary logSpan.http.span.4=7437ms message=\"exiting loop\"",
        );

        match classify(&record) {
            LogClassification::StepExit { session_id } => {
                assert_eq!(session_id, "ses_primary");
            }
            other => panic!("expected StepExit, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_permission_evaluated() {
        let record = parse_new_format_record(
            r#"timestamp=2026-06-10T16:11:32.000Z level=INFO service=permission permission=task pattern=general action={"permission":"*","action":"allow","pattern":"*"} message=evaluated"#,
        );

        match classify(&record) {
            LogClassification::PermissionEvaluated {
                permission,
                pattern,
                action,
            } => {
                assert_eq!(permission, "task");
                assert_eq!(pattern, "general");
                assert_eq!(
                    action,
                    r#"{"permission":"*","action":"allow","pattern":"*"}"#
                );
            }
            other => panic!("expected PermissionEvaluated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_step_loop() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:27.460Z level=INFO run=df5a9474 message=loop session.id=ses_14db step=1",
        );

        match classify(&record) {
            LogClassification::StepLoop { session_id, step } => {
                assert_eq!(session_id, "ses_14db");
                assert_eq!(step, 1);
            }
            other => panic!("expected StepLoop, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_llm_call() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:27.574Z level=INFO run=df5a9474 message=stream providerID=zai-coding-plan modelID=glm-5.1",
        );

        match classify(&record) {
            LogClassification::LlmCall {
                provider_id,
                model_id,
                mode,
                is_stream,
            } => {
                assert_eq!(provider_id, "zai-coding-plan");
                assert_eq!(model_id, "glm-5.1");
                assert_eq!(mode, "");
                assert!(is_stream);
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_permission_evaluated() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:31.461Z level=INFO run=df5a9474 message=evaluated permission=glob pattern=general action=allow",
        );

        match classify(&record) {
            LogClassification::PermissionEvaluated {
                permission,
                pattern,
                action,
            } => {
                assert_eq!(permission, "glob");
                assert_eq!(pattern, "general");
                assert_eq!(action, "allow");
            }
            other => panic!("expected PermissionEvaluated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_session_created() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:28.000Z level=INFO run=df5a9474 message=created id=ses_primary title=Primary session",
        );

        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_primary");
                assert_eq!(parent_id, None);
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_boot_banner() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:27.352Z level=INFO run=df5a9474 version=1.14.48 message=opencode",
        );

        match classify(&record) {
            LogClassification::BootBanner { version } => {
                assert_eq!(version, "1.14.48");
            }
            other => panic!("expected BootBanner, got {other:?}"),
        }
    }

    #[test]
    fn new_format_serviceless_classifies_step_exit() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:31.462Z level=INFO run=df5a9474 message=\"exiting loop\" session.id=ses_primary",
        );

        match classify(&record) {
            LogClassification::StepExit { session_id } => {
                assert_eq!(session_id, "ses_primary");
            }
            other => panic!("expected StepExit, got {other:?}"),
        }
    }

    #[test]
    fn new_format_classifies_tracking_as_unclassified() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:33.000Z level=INFO service=session message=tracking hash=abc123",
        );

        assert_eq!(classify(&record), LogClassification::Unclassified);
    }

    #[test]
    fn new_format_quoted_message_value() {
        let record = parse_new_format_record(
            "timestamp=2026-06-10T16:11:34.000Z level=INFO service=llm message=\"llm runtime selected\" providerID=kimi-for-coding modelID=k2p6",
        );

        assert_eq!(
            record.tags.get("message").map(String::as_str),
            Some("\"llm runtime selected\""),
        );
        assert_eq!(classify(&record), LogClassification::Unclassified);
    }

    #[test]
    fn new_format_error_with_inline_json() {
        let record = parse_new_format_record(
            r#"timestamp=2026-06-10T16:11:35.000Z level=ERROR service=llm providerID=kimi-for-coding modelID=k2p6 error={"error":{"name":"AI_APICallError","message":"upstream boom","statusCode":500}}"#,
        );

        match classify(&record) {
            LogClassification::ApiFailure {
                status_code,
                error_name,
                message,
                is_fatal,
            } => {
                assert_eq!(status_code, Some(500));
                assert_eq!(error_name, "AI_APICallError");
                assert_eq!(message, "AI_APICallError (500: Internal Server Error): upstream boom");
                assert!(!is_fatal);
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
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
    fn fixture_usage_cap_classifies() {
        let fixture = include_str!("../../../../tests/fixtures/logs/opencode-rate-limit.txt");
        let line = fixture
            .lines()
            .next()
            .expect("rate limit fixture has at least one line");
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("rate limit fixture failed to parse");
        };
        match classify(&record) {
            LogClassification::ProviderLimit { kind, reset_at, .. } => {
                assert_eq!(kind, ProviderLimitKind::UsageCap);
                let reset = reset_at.expect("reset_at should be parsed from fixture");
                assert_eq!(
                    reset.format("%Y-%m-%d %H:%M:%S").to_string(),
                    "2026-04-16 04:18:56",
                );
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn fixture_429_overload_classifies() {
        let fixture = include_str!("../../../../tests/fixtures/logs/opencode-429-overload.txt");
        let line = fixture
            .lines()
            .next()
            .expect("overload fixture has at least one line");
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("overload fixture failed to parse");
        };
        match classify(&record) {
            LogClassification::ProviderLimit { kind, status_code, .. } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::Overloaded);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_429_overload_as_overloaded() {
        let line = r#"ERROR 2026-05-15T19:26:02 +3054ms service=llm providerID=kimi-for-coding modelID=k2p6 error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"type\":\"rate_limit_error\",\"message\":\"The engine is currently overloaded, please try again later\"}}","isRetryable":true}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::Overloaded);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_429_throttled_as_rate_limited() {
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"message":"Too many requests"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::RateLimited);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_max_retries_exhausted_as_retries_exhausted() {
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429}]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::RetriesExhausted);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    /// Kimi reports its billing-cycle usage cap as HTTP 403 with
    /// `type: permission_error` and the phrase "reached your usage limit
    /// for this billing cycle" — a dialect none of the ZAI-style cap needles
    /// (`code 1308`, `exceeded_current_quota_error`, `Usage limit reached`)
    /// match. It must still classify as a terminal usage cap, not a raw
    /// `AI_APICallError` dump.
    #[test]
    fn classifies_kimi_403_billing_cycle_cap_as_usage_cap() {
        let line = r#"ERROR 2026-06-09T18:21:00 +4200ms service=llm providerID=kimi-for-coding modelID=k2p6 error={"error":{"name":"AI_APICallError","statusCode":403,"responseBody":"{\"error\":{\"type\":\"permission_error\",\"message\":\"You've reached your usage limit for this billing cycle. Your quota will be refreshed in the next cycle. Upgrade to get more: https://www.kimi.com/code/console?from=quota-upgrade\"}}","isRetryable":false}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(403));
                assert_eq!(kind, ProviderLimitKind::UsageCap);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_exceeded_quota_as_usage_cap() {
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"type\":\"exceeded_current_quota_error\",\"message\":\"Quota exceeded\"}}"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::UsageCap);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn cap_phrase_without_error_tag_is_advisory_api_failure() {
        let line = "ERROR 2026-05-15T19:26:02 +100ms service=llm dummy={} Usage limit reached for k2p6";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure {
                status_code,
                message,
                is_fatal,
                ..
            } => {
                assert_eq!(status_code, None);
                assert_eq!(message, "Usage limit reached for k2p6");
                assert!(!is_fatal);
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn cap_wins_over_retries_exhausted() {
        let line = r#"ERROR 2026-05-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_RetryError","reason":"maxRetriesExceeded","errors":[{"name":"AI_APICallError","statusCode":429,"responseBody":"{\"error\":{\"code\":\"1308\",\"message\":\"Usage limit reached\"}}"}]}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit {
                status_code,
                kind,
                ..
            } => {
                assert_eq!(status_code, Some(429));
                assert_eq!(kind, ProviderLimitKind::UsageCap);
            }
            other => panic!("expected ProviderLimit, got {other:?}"),
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
                    LogClassification::ProviderLimit { .. } => rate_limit_seen = true,
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
    fn overflow_status_code_does_not_produce_bogus_description() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","statusCode":70000,"message":"unknown error"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError (70000): unknown error");
                assert!(
                    !message.contains("Bad Request"),
                    "overflow status should not pick up a u16 description: {message}"
                );
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
                assert_eq!(
                    message,
                    "AI_APICallError: something went wrong on the server"
                );
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }

    #[test]
    fn malformed_response_body_falls_back_to_raw() {
        let line = r#"ERROR 2026-04-15T19:26:02 +100ms service=llm error={"error":{"name":"AI_APICallError","responseBody":"not json"}}"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert_eq!(message, "AI_APICallError: not json");
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
        // JSON variant: a 4-digit code must not match the first 3 digits.
        assert_eq!(extract_status_code(r#""statusCode":4291"#), None);
        assert_eq!(extract_status_code(r#""statusCode":9999"#), None);
        // Key-value variant: a 4+ digit run must not match the first 3 digits.
        assert_eq!(extract_status_code("statusCode=4299"), None);
        assert_eq!(extract_status_code("statusCode=9999"), None);
        // Valid 3-digit key-value codes still match (end-of-string or non-digit boundary).
        assert_eq!(extract_status_code("statusCode=429"), Some(429));
        assert_eq!(extract_status_code("statusCode=503 retrying"), Some(503));
        assert_eq!(extract_status_code("other=500"), None); // wrong key
    }

    // ------------------------------------------------------------------
    // Phase-2 lifecycle classifications
    // ------------------------------------------------------------------

    #[test]
    fn classifies_boot_banner() {
        let line = "INFO  2026-05-12T20:00:11 +97ms service=default version=1.14.48 args=[\"run\",\"--format\",\"json\"] process_role=main run_id=48277674-19e5-40b6-b2b5-efa7577f08ea opencode";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::BootBanner { version } => {
                assert_eq!(version, "1.14.48");
            }
            other => panic!("expected BootBanner, got {other:?}"),
        }
    }

    #[test]
    fn classifies_session_created_primary() {
        let line = "INFO  2026-05-12T20:00:12 +20ms service=session id=ses_1e23972b3ffe8QLhzuFpWS5bzd slug=happy-panda version=1.14.48 projectID=global directory=/private/tmp/oc-test path=private/tmp/oc-test title=New session created";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_1e23972b3ffe8QLhzuFpWS5bzd");
                assert_eq!(parent_id, None);
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn classifies_session_created_subagent() {
        let line = "INFO  2026-05-12T20:05:26 +1ms service=session id=ses_1e234a70dffeOCARJZRL9dhpHT slug=lucky-orchid version=1.14.48 projectID=global directory=/private/tmp/oc-test path=private/tmp/oc-test parentID=ses_1e234af48ffeViMPs5pMk6UhYk title=Count letters in 'banana' created";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::SessionCreated { id, parent_id } => {
                assert_eq!(id, "ses_1e234a70dffeOCARJZRL9dhpHT");
                assert_eq!(parent_id, Some("ses_1e234af48ffeViMPs5pMk6UhYk".into()));
            }
            other => panic!("expected SessionCreated, got {other:?}"),
        }
    }

    #[test]
    fn classifies_llm_call_primary() {
        let line = "INFO  2026-05-12T20:00:12 +0ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd small=false agent=build mode=primary stream";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::LlmCall {
                provider_id,
                model_id,
                mode,
                is_stream,
            } => {
                assert_eq!(provider_id, "kimi-for-coding");
                assert_eq!(model_id, "k2p6");
                assert_eq!(mode, "primary");
                assert!(is_stream);
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn classifies_llm_call_subagent() {
        let line = "INFO  2026-05-12T20:05:26 +1ms service=llm providerID=opencode modelID=claude-haiku-4-5 session.id=ses_1e234a70dffeOCARJZRL9dhpHT small=false agent=general mode=subagent stream";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::LlmCall { mode, .. } => {
                assert_eq!(mode, "subagent");
            }
            other => panic!("expected LlmCall, got {other:?}"),
        }
    }

    #[test]
    fn llm_stream_error_still_classifies_as_api_failure() {
        // "stream error" must NOT be classified as LlmCall; the existing
        // LLM-failure path must win.
        let line = r#"ERROR 2026-05-12T20:02:20 +1967ms service=llm providerID=kimi-for-coding modelID=k2p6 session.id=ses_1e237a304ffeqwr10bXJSRYGHJ small=false agent=build mode=primary error={"error":{"name":"AI_APICallError","statusCode":429}} stream error"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::ProviderLimit { .. } | LogClassification::ApiFailure { .. } => {}
            other => panic!("expected ApiFailure/ProviderLimit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_step_loop() {
        let line = "INFO  2026-05-12T20:00:12 +0ms service=session.prompt session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd step=0 logSpan.http.span.4=55ms loop";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::StepLoop { session_id, step } => {
                assert_eq!(session_id, "ses_1e23972b3ffe8QLhzuFpWS5bzd");
                assert_eq!(step, 0);
            }
            other => panic!("expected StepLoop, got {other:?}"),
        }
    }

    #[test]
    fn classifies_step_exit() {
        let line = "INFO  2026-05-12T20:00:19 +1ms service=session.prompt session.id=ses_1e23972b3ffe8QLhzuFpWS5bzd logSpan.http.span.4=7437ms exiting loop";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::StepExit { session_id } => {
                assert_eq!(session_id, "ses_1e23972b3ffe8QLhzuFpWS5bzd");
            }
            other => panic!("expected StepExit, got {other:?}"),
        }
    }

    #[test]
    fn classifies_permission_evaluated() {
        let line = r#"INFO  2026-05-12T20:05:26 +160ms service=permission permission=task pattern=general action={"permission":"*","action":"allow","pattern":"*"} evaluated"#;
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::PermissionEvaluated {
                permission,
                pattern,
                action,
            } => {
                assert_eq!(permission, "task");
                assert_eq!(pattern, "general");
                assert_eq!(
                    action,
                    r#"{"permission":"*","action":"allow","pattern":"*"}"#
                );
            }
            other => panic!("expected PermissionEvaluated, got {other:?}"),
        }
    }

    #[test]
    fn classifies_http_response() {
        let line = "INFO  2026-05-12T20:05:54 +0ms service=default http.method=POST http.url=/session/ses_1e2343b5cffeGOb3bcdTjvh1wZ/message http.status=500 logSpan.http.span.4=99ms Sent HTTP response";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::HttpResponse {
                method,
                url,
                status,
                duration_ms,
            } => {
                assert_eq!(method, "POST");
                assert_eq!(url, "/session/ses_1e2343b5cffeGOb3bcdTjvh1wZ/message");
                assert_eq!(status, 500);
                assert_eq!(duration_ms, 99);
            }
            other => panic!("expected HttpResponse, got {other:?}"),
        }
    }

    #[test]
    fn http_response_without_span_tag_zeroes_duration() {
        let line = "INFO  2026-05-12T20:05:54 +0ms service=default http.method=GET http.url=/health http.status=200 Sent HTTP response";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        match classify(&record) {
            LogClassification::HttpResponse { duration_ms, .. } => {
                assert_eq!(duration_ms, 0);
            }
            other => panic!("expected HttpResponse, got {other:?}"),
        }
    }

    #[test]
    fn unknown_default_service_is_unclassified() {
        // A service=default line that is neither boot banner nor HTTP response
        let line = "INFO  2026-05-12T20:00:11 +0ms service=default foo=bar some other text";
        let ParsedOpenCodeStderrLine::Structured(record) = parse_line(line) else {
            panic!("expected Structured");
        };
        assert_eq!(classify(&record), LogClassification::Unclassified);
    }

    #[test]
    fn non_json_error_tag_truncation_respects_char_boundaries() {
        use std::collections::BTreeMap;

        // 496 ASCII bytes, then a 4-byte emoji, then more ASCII. Byte 497
        // falls inside the emoji, so a byte-index slice at 497 would panic.
        let prefix = "x".repeat(496);
        let error_tag = format!("{prefix}😀AI_APICallError: something went wrong");
        assert!(error_tag.len() > 500);
        let raw = error_tag.clone();
        let record = OpenCodeLogRecord {
            level: LogLevel::Error,
            timestamp: Utc::now(),
            delta_ms: 0,
            tags: BTreeMap::from([
                ("service".into(), "llm".into()),
                ("error".into(), error_tag),
            ]),
            message: String::new(),
            raw,
        };
        match classify(&record) {
            LogClassification::ApiFailure { message, .. } => {
                assert!(message.starts_with(&prefix));
                assert!(message.contains("😀"));
                assert!(message.ends_with("..."));
            }
            other => panic!("expected ApiFailure, got {other:?}"),
        }
    }
}
