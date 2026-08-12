//! Mapping from session outcomes onto the contract's stable
//! [`InferenceErrorKind`] categories.
//!
//! Provider/session detail (stderr, raw error text, exit codes) is treated as
//! potentially secret-bearing: it is logged through `tracing` only and never
//! placed in [`InferenceError::message`].

use std::time::Duration;

use biscuit_contract::inference::{InferenceError, InferenceErrorKind};
use claudine::stream::summary::StreamExecutionSummary;

use crate::session::RawSession;

/// Build an [`InferenceError`] with a fixed, secret-free message.
pub(crate) fn inference_error(
    kind: InferenceErrorKind,
    message: impl Into<String>,
) -> InferenceError {
    InferenceError::new(kind, message)
}

/// Map a provider process spawn failure onto the contract's stable error kinds.
pub(crate) fn map_spawn_error(err: std::io::Error, program: &str) -> InferenceError {
    match err.kind() {
        std::io::ErrorKind::NotFound => inference_error(
            InferenceErrorKind::Unavailable,
            format!("provider binary `{}` not found on PATH", program),
        ),
        std::io::ErrorKind::PermissionDenied => inference_error(
            InferenceErrorKind::Unavailable,
            format!("provider binary `{}` is not executable", program),
        ),
        _ => inference_error(InferenceErrorKind::Provider, "failed to spawn provider process"),
    }
}

/// Reject a session that escaped the tool-free contract.
///
/// A successful-looking summary that nonetheless recorded tool calls,
/// permission prompts, or interactive user-input prompts means the agentic CLI
/// tried to act as an agent. The contract authorizes none of that, so the
/// request fails rather than returning whatever text leaked out.
pub(crate) fn check_security(summary: &StreamExecutionSummary) -> Result<(), InferenceError> {
    let tool_calls = summary.tool_calls.unwrap_or(0);
    let permission_prompts = summary.permission_prompts.unwrap_or(0);
    let user_input_prompts = summary.user_input_prompts.unwrap_or(0);

    if tool_calls > 0 || permission_prompts > 0 || user_input_prompts > 0 {
        tracing::warn!(
            tool_calls,
            permission_prompts,
            user_input_prompts,
            "rejecting inference: session attempted tool use or interactive prompts"
        );
        return Err(inference_error(
            InferenceErrorKind::InvalidResponse,
            "session attempted tool use or an interactive prompt under a tool-free contract",
        ));
    }
    Ok(())
}

/// Classify a failed session into an [`InferenceError`], or `None` when the
/// session is a clean success.
///
/// Failure is either an explicit stream error (`summary.is_error`) or a
/// non-zero exit with no usable assistant text. Detail is read from the
/// summary first; `raw.stderr` is consulted as a fallback for auth/rate
/// signals that some providers only emit on stderr.
pub(crate) fn detect_failure(
    summary: &StreamExecutionSummary,
    raw: &RawSession,
) -> Option<InferenceError> {
    let exit_failed = raw.exit_code != 0;
    if !summary.is_error && !exit_failed {
        return None;
    }
    // A non-zero exit alongside usable text is tolerated: some providers exit
    // non-zero for non-fatal reasons while still producing a final answer.
    if !summary.is_error && exit_failed && !summary.assistant_text.trim().is_empty() {
        return None;
    }

    // Rate limiting carries a retry hint when the provider supplies one.
    if let Some(rate) = &summary.rate_limit
        && (rate.is_throttled.unwrap_or(false) || rate.retry_after_ms.is_some())
    {
        let mut error =
            inference_error(InferenceErrorKind::RateLimited, "provider rate-limited the request");
        if let Some(ms) = rate.retry_after_ms {
            error = error.with_retry_after(Duration::from_millis(ms));
        }
        tracing::warn!(retry_after_ms = rate.retry_after_ms, "inference rate-limited");
        return Some(error);
    }

    if let Some(diag) = &summary.stderr_diagnostics
        && diag.auth_failures > 0
    {
        return Some(inference_error(
            InferenceErrorKind::Unauthorized,
            "provider authentication failed",
        ));
    }

    let haystack = format!(
        "{} {} {}",
        summary.error_kind.as_deref().unwrap_or(""),
        summary.error_message.as_deref().unwrap_or(""),
        raw.stderr.as_deref().unwrap_or(""),
    )
    .to_ascii_lowercase();

    let kind = classify_text(&haystack);
    tracing::warn!(
        ?kind,
        exit_code = raw.exit_code,
        error_kind = summary.error_kind.as_deref().unwrap_or(""),
        "inference session failed"
    );
    Some(inference_error(kind, message_for(kind)))
}

/// Keyword classification over lowercased provider/stderr text.
fn classify_text(haystack: &str) -> InferenceErrorKind {
    let has = |needle: &str| haystack.contains(needle);

    if has("rate limit") || has("ratelimit") || has("429") || has("throttl") || has("quota") {
        InferenceErrorKind::RateLimited
    } else if has("unauthor")
        || has("401")
        || has("403")
        || has("forbidden")
        || has("api key")
        || has("not logged in")
        || has("authenticat")
        || has("credential")
    {
        InferenceErrorKind::Unauthorized
    } else if has("timeout") || has("timed out") {
        InferenceErrorKind::Timeout
    } else if has("overload")
        || has("unavailable")
        || has("502")
        || has("503")
        || has("500")
        || has("internal server error")
    {
        InferenceErrorKind::Unavailable
    } else {
        InferenceErrorKind::Provider
    }
}

/// Fixed, secret-free message for a classified failure.
fn message_for(kind: InferenceErrorKind) -> &'static str {
    match kind {
        InferenceErrorKind::RateLimited => "provider rate-limited the request",
        InferenceErrorKind::Unauthorized => "provider rejected authentication",
        InferenceErrorKind::Timeout => "provider session timed out",
        InferenceErrorKind::Unavailable => "provider is temporarily unavailable",
        InferenceErrorKind::Provider
        | InferenceErrorKind::InvalidRequest
        | InferenceErrorKind::Unsupported
        | InferenceErrorKind::InvalidResponse => "provider session failed",
    }
}
