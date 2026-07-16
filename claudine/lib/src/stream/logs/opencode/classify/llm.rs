//! LLM-failure classification and lifecycle-service routing.
//!
//! [`classify_llm_failure`] handles the `service=llm` / `service=provider`
//! error envelope (auth failures, rate limits, API failures) and
//! [`classify_lifecycle`] routes records to their service-specific
//! classifier based on the inferred `service` tag.

use crate::stream::logs::opencode::events::{
    LogClassification, OpenCodeLogRecord, ProviderLimitKind,
};

use super::asset::{extract_provider_message, summarize_error_json};
use super::session::{
    classify_llm_call, classify_permission, classify_session, classify_session_prompt,
};
use super::text_util::{contains_any_ci, extract_reset_at, extract_status_code};
use super::error_context;

pub(super) fn classify_llm_failure(
    record: &OpenCodeLogRecord,
    service: &str,
) -> Option<LogClassification> {
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
    let provider_error_context = error_context(record);
    let has_error_context = provider_error_context.is_some();

    // Resolution order is critical — cap-with-context wins over retries-exhausted.
    // 1. Cap signal present with error tag → terminal usage cap.
    // ProviderLimitKind is authoritative; preserve the real HTTP code when
    // available instead of stamping a 429 sentinel onto a 403 billing cap.
    if has_cap && has_error_context {
        let reset_at = extract_reset_at(haystack);
        let provider_id = record.tags.get("providerID").cloned();
        let model_id = record.tags.get("modelID").cloned();
        let provider_error = provider_error_context
            .clone()
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
        let provider_error = provider_error_context
            .clone()
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
        let provider_error = provider_error_context
            .clone()
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
        let provider_error = provider_error_context
            .clone()
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

pub(super) fn classify_lifecycle(record: &OpenCodeLogRecord) -> Option<LogClassification> {
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
pub(super) fn infer_service_from_message(record: &OpenCodeLogRecord) -> &'static str {
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
        // OpenCode 1.17.8 reuses `message="stream"` for the call start and
        // `message="stream error"` for the failure (fixes/2026-06-21-opencode-log-fix).
        // Both route to `llm` so the failure reaches `classify_llm_failure`; the
        // call start is still distinguished downstream by `classify_llm_call`.
        "stream" | "stream error"
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
pub(super) fn has_trailing_keyword(record: &OpenCodeLogRecord, keyword: &str) -> bool {
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
pub(super) fn tag_value_stripped(
    record: &OpenCodeLogRecord,
    key: &str,
    keyword: &str,
) -> Option<String> {
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
