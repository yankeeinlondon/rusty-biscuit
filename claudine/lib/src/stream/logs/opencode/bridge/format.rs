//! Message-formatting helpers for the OpenCode stderr bridge.
//!
//! These free functions render the inline message strings and base `extra`
//! maps that the bridge emits alongside [`SemanticEvent`]s. Keeping them in
//! one place makes the formatting vocabulary easy to audit and keeps the
//! bridge's impl blocks focused on dispatch logic.

use serde_json::{Map, Value};

use crate::stream::logs::opencode::events::OpenCodeLogRecord;

/// Render the inline message string for a `service=llm ... stream` event.
/// Example: `llm_call_start anthropic/claude-opus-4-7 (mode=primary, agent=build)`.
pub(super) fn format_llm_call_message(
    provider_id: &str,
    model_id: &str,
    mode: &str,
    agent: Option<&str>,
) -> String {
    let identity = match (provider_id, model_id) {
        ("", "") => "(unknown provider/model)".to_string(),
        ("", model) => model.to_string(),
        (provider, "") => provider.to_string(),
        (provider, model) => format!("{provider}/{model}"),
    };
    let mut parts: Vec<String> = Vec::with_capacity(2);
    if !mode.is_empty() {
        parts.push(format!("mode={mode}"));
    }
    if let Some(agent) = agent.filter(|a| !a.is_empty()) {
        parts.push(format!("agent={agent}"));
    }
    if parts.is_empty() {
        format!("llm_call_start {identity}")
    } else {
        format!("llm_call_start {identity} ({})", parts.join(", "))
    }
}

/// Render the inline message string for a permission evaluation. OpenCode
/// emits the resolved `action` either as a bare value or as a JSON object
/// of the shape `{"permission": "...", "pattern": "...", "action": "..."}`;
/// when JSON, we extract the inner `action` ("allow" / "deny") so the
/// rendered text stays scannable.
pub(super) fn format_permission_message(permission: &str, pattern: &str, action: &str) -> String {
    let action_short = summarize_permission_action(action);
    let subject = match (permission, pattern) {
        ("", "") => String::new(),
        ("", pat) => pat.to_string(),
        (perm, "") => perm.to_string(),
        (perm, pat) => format!("{perm}:{pat}"),
    };
    match (subject.is_empty(), action_short.is_empty()) {
        (true, true) => "permission_evaluated".to_string(),
        (true, false) => format!("permission_evaluated → {action_short}"),
        (false, true) => format!("permission_evaluated {subject}"),
        (false, false) => format!("permission_evaluated {subject} → {action_short}"),
    }
}

pub(super) fn summarize_permission_action(action: &str) -> String {
    let trimmed = action.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed.starts_with('{')
        && let Ok(Value::Object(map)) = serde_json::from_str::<Value>(trimmed)
        && let Some(Value::String(inner)) = map.get("action")
    {
        return inner.clone();
    }
    trimmed.to_string()
}

/// Render the inline message string for a `Sent HTTP response` event.
pub(super) fn format_http_response_message(
    method: &str,
    url: &str,
    status: u16,
    duration_ms: u64,
) -> String {
    let duration = if duration_ms == 0 {
        String::new()
    } else {
        format!(" ({duration_ms}ms)")
    };
    let target = match (method.is_empty(), url.is_empty()) {
        (true, true) => String::new(),
        (true, false) => url.to_string(),
        (false, true) => method.to_string(),
        (false, false) => format!("{method} {url}"),
    };
    let status_part = if status == 0 {
        String::new()
    } else {
        format!(" {status}")
    };
    if target.is_empty() && status == 0 {
        "http_response".to_string()
    } else if target.is_empty() {
        format!("http_response{status_part}{duration}")
    } else {
        format!("http_response {target}{status_part}{duration}")
    }
}

/// Render the inline message string for a snapshot subsystem log line.
/// `tag_summary` is a one-line, comma-joined view of the most informative
/// non-control tags on the record.
pub(super) fn format_snapshot_message(message: &str, tag_summary: &str) -> String {
    let base = if message.is_empty() {
        "snapshot".to_string()
    } else {
        format!("snapshot: {message}")
    };
    if tag_summary.is_empty() {
        base
    } else {
        format!("{base} ({tag_summary})")
    }
}

/// Copy snapshot-relevant tags into `extra` and return a comma-joined
/// `key=value` summary suitable for inline rendering. Falls back to all
/// non-`service` tags when the well-known keys are absent.
pub(super) fn summarize_snapshot_tags(
    record: &OpenCodeLogRecord,
    extra: &mut Map<String, Value>,
) -> String {
    const PREFERRED: &[&str] = &["file", "files", "path", "id", "session.id", "err", "error"];
    let mut parts: Vec<String> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();

    for key in PREFERRED {
        if let Some(value) = record.tags.get(*key) {
            extra.insert((*key).into(), Value::String(value.clone()));
            parts.push(format!("{key}={}", truncate_for_inline(value)));
            seen.insert((*key).to_string());
        }
    }

    if parts.is_empty() {
        for (key, value) in &record.tags {
            if key == "service" || seen.contains(key) {
                continue;
            }
            extra.insert(key.clone(), Value::String(value.clone()));
            parts.push(format!("{key}={}", truncate_for_inline(value)));
        }
    }

    parts.join(", ")
}

/// Clip a tag value for inline display so a single multi-kilobyte
/// `error=` payload cannot blow up the rendered status line.
fn truncate_for_inline(value: &str) -> String {
    const MAX: usize = 80;
    if value.chars().count() <= MAX {
        return value.to_string();
    }
    let head: String = value.chars().take(MAX).collect();
    format!("{head}…")
}

/// Build the base `extra` map shared by every bridge-emitted
/// [`SemanticEvent`]. Stamps the provider, source, classification label,
/// and raw line so downstream consumers (JSONL reporting, live renderer)
/// have consistent context.
pub(super) fn base_extra(record: &OpenCodeLogRecord, classification: &str) -> Map<String, Value> {
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

/// Map an owned tag value to `None` when empty so absent OpenCode tags don't
/// surface as blank context fields.
pub(super) fn non_empty(value: String) -> Option<String> {
    if value.is_empty() { None } else { Some(value) }
}

/// Saturating `Duration` → milliseconds as `u64`. `Duration::as_millis`
/// returns `u128`; clamp to `u64::MAX` rather than truncate so a pathological
/// duration cannot wrap into a small value in the JSONL `extra` map.
pub(super) fn duration_as_millis_u64(duration: std::time::Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}
