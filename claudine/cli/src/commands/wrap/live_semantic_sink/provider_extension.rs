//! Provider-extension classification and payload summarization helpers.
//!
//! These free functions decide which `ProviderExtension` / Warning events are
//! suppressed on stderr and derive the terse one-line summaries used by
//! [`LiveSemanticSink::provider_extension_description`](super::LiveSemanticSink).

use claudine::provider::{Provider, provider_info};
use claudine::stream::semantic::SemanticEvent;
use serde_json::Value;

/// Return `true` when `event` is a Claude `task_progress` Info line —
/// the narration Claude emits just before the matching tool call. Only
/// `Provider::Claude` emits this shape today; the gate is explicit so
/// unrelated Info events do not get delayed one tick.
pub(crate) fn is_claude_task_progress(provider: Provider, event: &SemanticEvent) -> bool {
    if provider != Provider::Claude {
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

/// Produce a terse one-line human summary of a [`SemanticEvent::ProviderExtension`]
/// payload.
///
/// Returns `None` when no summary can be derived from known nested shapes —
/// callers must render `provider/kind` WITHOUT a trailing ` · <payload>` in
/// that case rather than falling back to raw JSON. This is a deliberate UX
/// trade-off: a bare `provider/kind` is less informative but still readable,
/// whereas a truncated raw JSON blob is actively harmful noise on stderr.
pub(crate) fn summarize_provider_payload(payload: &Value) -> Option<String> {
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

pub(crate) fn provider_short(p: Provider) -> &'static str {
    provider_info(p).short_name
}

/// Kinds that are known to be high-volume or entirely redundant on stderr.
/// Listed explicitly rather than relying on summary heuristics so the
/// suppression is visible, reviewable, and reversible. Events in this set
/// still flow through dispatch and the JSONL log; only the stderr status
/// line is suppressed.
const SILENT_PROVIDER_EXTENSION_KINDS: &[(Provider, &str)] = &[
    // Claude: partial assistant token deltas — redundant with OutputText.
    (Provider::Claude, "stream_event"),
    // Claude: hook lifecycle events. Claude parser (Task 2a.2) emits
    // these as ProviderExtension with kind `system/<subtype>` after
    // buffering them to trail SessionStart.
    (Provider::Claude, "system/hook_started"),
    (Provider::Claude, "system/hook_response"),
    (Provider::Claude, "system/hook_progress"),
    // Codex: unknown/unmodeled item lifecycle markers. When the inner
    // `item.type` is something Claudine does not classify (new Codex
    // builds, experimental item types), the parser falls back to a
    // ProviderExtension. Leaking `codex/item.started · {...}` onto stderr
    // is noise — the underlying detail is what callers care about, and
    // the raw event is still in the JSONL log.
    (Provider::Codex, "item.started"),
    (Provider::Codex, "item.completed"),
    // Kimi: high-volume wire envelope kinds that the Kimi semantic parser
    // already maps to first-class semantic events. If a future Kimi
    // protocol revision changes the payload shape so typed
    // deserialization fails, the parser falls back to a
    // `ProviderExtension` with a `event:<inner_type>` raw_kind. These
    // entries keep the stderr surface quiet on drift; the raw envelopes
    // still flow through dispatch and the JSONL log.
    //
    // ContentPart (assistant text/think deltas) and ToolCallPart
    // (streamed tool argument fragments) can fire many times per turn,
    // so suppressing fallback rendering here matches the "high-volume
    // wire fallback kinds" contract from Phase 5 of the fix-kimi plan.
    (Provider::KimiCode, "event:ContentPart"),
    (Provider::KimiCode, "event:ToolCallPart"),
    // StatusUpdate fires on every step boundary and carries token /
    // context-percent telemetry. The parser emits a Warning only when
    // the context-pressure threshold is crossed; routine StatusUpdate
    // payload-shape drift should not surface as a fallback line.
    (Provider::KimiCode, "event:StatusUpdate"),
    // Legacy stream-json payload names that Kimi is unlikely to emit on
    // the wire transport but are kept here as defensive entries so an
    // accidental cross-mode payload (or replay of a legacy fixture) does
    // not flood stderr.
    (Provider::KimiCode, "event:MessageStart"),
    (Provider::KimiCode, "event:MessageDelta"),
    (Provider::KimiCode, "event:MessageEnd"),
    (Provider::KimiCode, "event:Thinking"),
];

pub(crate) fn is_silent_extension_kind(provider: Provider, kind: &str) -> bool {
    SILENT_PROVIDER_EXTENSION_KINDS
        .iter()
        .any(|(p, k)| *p == provider && *k == kind)
}

/// Suppress only the legacy generic Claude `rate limit` Warning on stderr
/// when the session metadata shows subscription auth. Explicit Claude
/// metadata text must still render because it can include cap-window timing.
pub(crate) fn is_suppressed_claude_rate_limit(
    provider: Provider,
    message: &str,
    extra: &Value,
    api_key_source: Option<&str>,
) -> bool {
    if provider != Provider::Claude {
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
