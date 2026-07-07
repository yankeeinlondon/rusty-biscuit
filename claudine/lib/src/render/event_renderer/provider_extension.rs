//! Provider-extension classification and payload summarization helpers.
//!
//! These free functions decide which `ProviderExtension` / Warning events are
//! suppressed on stderr and derive the terse one-line summaries used for the
//! provider-extension status line. Provider-conditional decisions read the
//! resolved [`DisplayPolicy`], never a matched `Provider`; the `Provider`
//! parameters here are the *event's own* provider field, used only for data
//! lookups (short name, that provider's silent-kind set).

use serde_json::Value;

use crate::provider::{DisplayPolicy, Provider, provider_info};
use crate::stream::semantic::SemanticEvent;

/// Return `true` when `event` is a `task_progress` Info line — the
/// narration Claude emits just before the matching tool call. The gate is
/// the catalog's `collapse_task_progress` policy (set only for providers
/// that emit this shape) so unrelated Info events do not get delayed one
/// tick.
pub(super) fn is_claude_task_progress(policy: &DisplayPolicy, event: &SemanticEvent) -> bool {
    if !policy.collapse_task_progress {
        return false;
    }
    let SemanticEvent::Info { extra, .. } = event else {
        return false;
    };
    extra
        .get("type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind == "task_progress")
}

/// Format a `ProviderExtension` status description as `provider_short/kind`
/// with an optional ` · summary` suffix. `provider` is the event's own
/// provider field — a data lookup for the short name, not dispatch.
pub(super) fn provider_extension_description(
    provider: Provider,
    kind: &str,
    payload: &Value,
) -> String {
    match summarize_provider_payload(payload) {
        Some(s) => format!("{}/{kind} \u{00b7} {s}", provider_short(provider)),
        None => format!("{}/{kind}", provider_short(provider)),
    }
}

/// Produce a terse one-line human summary of a
/// [`SemanticEvent::ProviderExtension`] payload.
///
/// Returns `None` when no summary can be derived from known nested shapes —
/// callers must render `provider/kind` WITHOUT a trailing ` · <payload>` in
/// that case rather than falling back to raw JSON. This is a deliberate UX
/// trade-off: a bare `provider/kind` is less informative but still readable,
/// whereas a truncated raw JSON blob is actively harmful noise on stderr.
pub(super) fn summarize_provider_payload(payload: &Value) -> Option<String> {
    // Known single-string text locations, in descending specificity. Each
    // entry is a path of object keys from the root of the payload.
    let known_paths: &[&[&str]] = &[
        &["message"],
        &["status"],
        &["name"],
        &["path"],
        &["text"],
        &["content"],
        &["error", "message"],
        &["error_message"],
        &["title"],
        &["description"],
    ];

    payload.as_object()?;

    for path in known_paths {
        let mut cursor: &Value = payload;
        let mut ok = true;
        for segment in path.iter() {
            match cursor.get(*segment) {
                Some(next) => cursor = next,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if ok && let Some(s) = cursor.as_str().filter(|s| !s.is_empty()) {
            return Some(s.to_string());
        }
    }

    // Nested content arrays: message.content[*].text, item.content.parts[*].text, etc.
    let nested_array_paths: &[&[&str]] = &[
        &["message", "content"],
        &["item", "content", "parts"],
        &["content", "parts"],
        &["parts"],
    ];
    for nested_path in nested_array_paths {
        let mut cursor: &Value = payload;
        let mut ok = true;
        for seg in nested_path.iter() {
            match cursor.get(*seg) {
                Some(next) => cursor = next,
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        if let Some(array) = cursor.as_array() {
            for elem in array {
                if let Some(text) = elem
                    .get("text")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    return Some(text.to_string());
                }
                if let Some(text) = elem
                    .get("content")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    return Some(text.to_string());
                }
            }
        }
    }

    // Last resort: the first non-empty top-level string value. This recovers
    // summary text for shapes we haven't explicitly enumerated while still
    // avoiding a raw-JSON dump.
    if let Some(obj) = payload.as_object() {
        for (_, v) in obj.iter() {
            if let Some(s) = v.as_str().filter(|s| !s.is_empty()) {
                return Some(s.to_string());
            }
        }
    }

    None
}

pub(super) fn provider_short(p: Provider) -> &'static str {
    provider_info(p).short_name
}

/// Return `true` when `kind` is one of the provider's catalog-curated
/// `silent_extension_kinds` — kinds known to be high-volume or entirely
/// redundant on stderr, listed explicitly (per-provider in the facts
/// files, with the per-kind rationale) rather than relying on summary
/// heuristics so the suppression is visible, reviewable, and reversible.
/// Events in this set still flow through dispatch and the JSONL log; only
/// the stderr status line is suppressed.
///
/// `provider` is the extension event's own provider field — the silent-kind
/// set is keyed off the emitting provider, a data lookup.
pub(super) fn is_silent_extension_kind(provider: Provider, kind: &str) -> bool {
    provider_info(provider)
        .display_policy
        .silent_extension_kinds
        .contains(&kind)
}

/// Suppress only the legacy generic `rate limit` Warning on stderr when
/// the session metadata shows subscription auth (Claude's shape today,
/// gated by the catalog's `suppress_subscription_rate_limit` policy).
/// Explicit metadata text must still render because it can include
/// cap-window timing.
pub(super) fn is_suppressed_claude_rate_limit(
    policy: &DisplayPolicy,
    message: &str,
    extra: &Value,
    api_key_source: Option<&str>,
) -> bool {
    if !policy.suppress_subscription_rate_limit {
        return false;
    }
    let raw_kind = extra.get("raw_kind").and_then(Value::as_str).unwrap_or("");
    if raw_kind != "rate_limit_event" || message.trim() != "rate limit" {
        return false;
    }
    if let Some(api_key_source) = api_key_source {
        return api_key_source != "ANTHROPIC_API_KEY";
    }
    std::env::var("ANTHROPIC_API_KEY")
        .map(|v| v.trim().is_empty())
        .unwrap_or(true)
}
