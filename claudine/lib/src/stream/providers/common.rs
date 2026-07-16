//! Shared skeleton for the per-provider semantic stream parsers.
//!
//! Every line-oriented parser re-implemented the same `extra`-map base, the
//! provider-extension fallback, and the malformed-line warning with only the
//! provider constant changing (seven near-identical copies of each). These
//! free functions are that skeleton; each parser keeps its own state struct
//! and delegates through thin methods, so provider-specific layers (Codex's
//! `session_id` key, Claude's `extra_with`, Kimi's `info_extra_with_kind`)
//! stay local.

use serde_json::{Map, Value};
use tracing::debug;

use super::semantic::{SemanticErrorKind, SemanticEvent, SemanticEventSink};
use super::summary::StreamExecutionSummary;
use crate::provider_id::Provider;

/// Ordered keyword buckets driving [`classify_error_by_keywords`].
///
/// Bucket *order* is the behavior contract — it encodes each provider's
/// precedence quirks (Gemini checks Configuration before ApiRemote in the
/// kind branch; several providers run a second ApiRemote pass after
/// Interrupted; Antigravity checks auth keywords first in the message
/// branch) — so the shared cascade never reorders, it just walks the table.
pub(crate) struct ErrorKeywords {
    /// Buckets checked against a structured error-kind discriminator.
    pub(crate) kind_buckets: &'static [(SemanticErrorKind, &'static [&'static str])],
    /// Buckets checked against the free-form error message.
    pub(crate) msg_buckets: &'static [(SemanticErrorKind, &'static [&'static str])],
    /// Exact numeric wire-code → kind mappings (Kimi JSON-RPC codes),
    /// checked before the kind/message branches when a code is supplied.
    /// Empty for every provider whose wire protocol carries no numeric
    /// error code.
    pub(crate) code_buckets: &'static [(i32, SemanticErrorKind)],
}

/// The shared keyword cascade every parser's `classify_error` ran by hand:
/// an exact numeric-code match first (when a code is supplied), then
/// lowercase the kind and the message and walk the provider's ordered
/// buckets, first substring hit wins, `AgentNative` as the fallthrough.
///
/// `code` is threaded through for the numeric protocols (Kimi); providers
/// with no numeric error code pass `None` and their empty `code_buckets`
/// make the code branch a no-op.
pub(crate) fn classify_error_by_keywords(
    kw: &ErrorKeywords,
    code: Option<i32>,
    error_kind: Option<&str>,
    message: Option<&str>,
) -> SemanticErrorKind {
    if let Some(code) = code {
        for (needle, result) in kw.code_buckets {
            if *needle == code {
                return *result;
            }
        }
    }
    for (input, buckets) in [
        (error_kind, kw.kind_buckets),
        (message, kw.msg_buckets),
    ] {
        if let Some(text) = input {
            let lower = text.to_ascii_lowercase();
            for (result, needles) in buckets {
                if needles.iter().any(|needle| lower.contains(needle)) {
                    return *result;
                }
            }
        }
    }
    SemanticErrorKind::AgentNative
}

/// The `{provider, line_num}` base every parser `extra` payload starts from.
pub(crate) fn base_extra_parts(provider: Provider, line_num: usize) -> Map<String, Value> {
    let mut m = Map::new();
    m.insert("provider".into(), Value::from(provider.as_slug()));
    m.insert("line_num".into(), Value::from(line_num));
    m
}

/// [`base_extra_parts`] plus the `raw_kind` discriminator.
pub(crate) fn base_extra(
    provider: Provider,
    line_num: usize,
    raw_kind: &str,
) -> Map<String, Value> {
    let mut m = base_extra_parts(provider, line_num);
    m.insert("raw_kind".into(), Value::from(raw_kind));
    m
}

/// Preserve a successfully-parsed but unrecognized event as
/// [`SemanticEvent::ProviderExtension`] rather than dropping it.
pub(crate) fn emit_provider_extension(
    sink: &mut impl SemanticEventSink,
    provider: Provider,
    kind: &str,
    payload: Value,
) {
    debug!(
        provider = provider.as_slug(),
        event_type = %kind,
        "parser falling back to provider extension for unknown event type"
    );
    sink.on_semantic_event(SemanticEvent::ProviderExtension {
        provider,
        kind: kind.to_string(),
        payload,
    });
}

/// Stamp the shared `finish()` tail onto a parser-built summary: the owning
/// provider and its derived badges.
///
/// Callers construct the [`StreamExecutionSummary`] with only the fields
/// they populate (plus `..Default::default()`); stamping `provider` here
/// keeps it from ever drifting from the badge derivation.
pub(crate) fn finish_summary(
    provider: Provider,
    mut summary: StreamExecutionSummary,
) -> StreamExecutionSummary {
    summary.provider = provider;
    summary.badges = crate::stream::badges::derive_badges(&summary, provider);
    summary
}

/// Surface a hard-malformed input line as a [`SemanticEvent::Warning`].
pub(crate) fn emit_malformed_warning(
    sink: &mut impl SemanticEventSink,
    provider: Provider,
    line_num: usize,
    err: &str,
) {
    sink.on_semantic_event(SemanticEvent::Warning {
        message: format!("Malformed JSON on line {line_num}: {err}"),
        extra: Value::Object(base_extra(provider, line_num, "malformed_json")),
    });
}
