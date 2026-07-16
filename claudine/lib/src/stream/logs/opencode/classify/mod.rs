//! Classification and error extraction for parsed OpenCode log records.
//!
//! Turns parsed [`OpenCodeLogRecord`]s into [`LogClassification`] variants
//! and extracts structured error information from inline JSON payloads.

mod asset;
mod llm;
mod session;
mod text_util;

use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use regex::Regex;

use crate::stream::logs::opencode::events::{AssetType, LogClassification, OpenCodeLogRecord};
use crate::stream::summary::RateLimitInfo;

use asset::classify_malformed_asset;
use llm::{classify_lifecycle, classify_llm_failure, infer_service_from_message};
use session::looks_like_uncaught_error;

static ANSI_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*[A-Za-z]").expect("ansi regex must compile"));

/// Classify a parsed structured log record.
pub fn classify(record: &OpenCodeLogRecord) -> LogClassification {
    if let Some(classification) = classify_malformed_asset(record) {
        return classification;
    }

    let service = record.tags.get("service").map(|s| s.as_str()).unwrap_or("");
    // OpenCode 1.17.8 drops `service=` from `message="stream error"` failure
    // records, so fall back to the same message-inference used for lifecycle
    // events. Without this the usage-cap path never runs and a terminal cap is
    // mistaken for an ongoing call (see fixes/2026-06-21-opencode-log-fix).
    let effective_service = if service.is_empty() {
        infer_service_from_message(record)
    } else {
        service
    };

    if (effective_service == "llm" || effective_service == "provider")
        && let Some(classification) = classify_llm_failure(record, effective_service)
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

/// The tag carrying provider error context, normalized across log formats.
///
/// Pre-1.17.8 OpenCode emitted a flat `error={JSON}` tag. 1.17.8's
/// `message="stream error"` records nest it as `error.error="<string>"` (the
/// body parser keeps `error.error` as one dotted key). Accept either, plus the
/// short `err` alias. Surrounding quotes — present on the new flat-string form
/// but absent on the old JSON form — are stripped so callers see the bare value.
pub(super) fn error_context(record: &OpenCodeLogRecord) -> Option<String> {
    let raw = record
        .tags
        .get("error")
        .or_else(|| record.tags.get("error.error"))
        .or_else(|| record.tags.get("err"))?;
    let trimmed = raw.trim();
    let value = if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
        &trimmed[1..trimmed.len() - 1]
    } else {
        trimmed
    };
    Some(value.to_string())
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
mod tests;
