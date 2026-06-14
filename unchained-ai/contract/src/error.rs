//! Mapping from `unchained-ai` provider outcomes onto the contract's stable
//! [`InferenceErrorKind`] categories.
//!
//! Provider detail (raw HTTP bodies, auth headers, API keys) is treated as
//! potentially secret-bearing: it is logged through `tracing` only and never
//! placed in [`InferenceError::message`].

use std::time::Duration;

use biscuit_contract::inference::{InferenceError, InferenceErrorKind};
use unchained_ai::rigging::providers::provider_errors::ProviderError;

/// Build an [`InferenceError`] with a fixed, secret-free message.
pub(crate) fn inference_error(
    kind: InferenceErrorKind,
    message: impl Into<String>,
) -> InferenceError {
    InferenceError::new(kind, message)
}

/// Classify a [`ProviderError`] into an [`InferenceError`].
///
/// The `explicit_model` flag distinguishes an explicit `with_model` pin from
/// profile-driven selection: missing credentials on an explicit pin are
/// `Unauthorized`, while an empty capability stack is `Unavailable`.
pub(crate) fn classify_provider_error(
    error: ProviderError,
    explicit_model: bool,
) -> InferenceError {
    match error {
        ProviderError::MissingApiKey { provider, .. } => {
            if explicit_model {
                inference_error(
                    InferenceErrorKind::Unauthorized,
                    format!("missing API key for pinned provider {provider}"),
                )
            } else {
                inference_error(
                    InferenceErrorKind::Unavailable,
                    "no runnable model found for the requested profile",
                )
            }
        }
        ProviderError::NoRunnableModel { .. } => inference_error(
            InferenceErrorKind::Unavailable,
            "no runnable model found for the requested profile",
        ),
        ProviderError::AuthenticationFailed { provider } => {
            tracing::warn!(provider, "provider authentication failed");
            inference_error(
                InferenceErrorKind::Unauthorized,
                format!("provider {provider} rejected authentication"),
            )
        }
        ProviderError::RateLimitExceeded { provider } => {
            tracing::warn!(provider, "provider rate limit exceeded");
            inference_error(InferenceErrorKind::RateLimited, "provider rate-limited the request")
        }
        ProviderError::Unsupported { provider, ref reason } => {
            tracing::warn!(provider, reason, "provider reported the operation as unsupported");
            inference_error(
                InferenceErrorKind::Unsupported,
                format!("provider {provider} does not support the requested operation"),
            )
        }
        ProviderError::StructuredParse { ref reason } => {
            tracing::debug!(reason, "structured response could not be parsed");
            inference_error(
                InferenceErrorKind::InvalidResponse,
                "provider did not return a single JSON value for a structured request",
            )
        }
        ProviderError::Timeout { provider } => inference_error(
            InferenceErrorKind::Timeout,
            format!("provider {provider} did not respond in time"),
        ),
        ProviderError::ValidationTimeout { provider } => inference_error(
            InferenceErrorKind::Timeout,
            format!("provider {provider:?} validation timed out"),
        ),
        ProviderError::NoProvidersAvailable => inference_error(
            InferenceErrorKind::Unavailable,
            "no providers are configured",
        ),
        ProviderError::HttpError(ref e) => classify_http_error(e),
        ProviderError::ExecutionFailed { provider, ref reason } => {
            classify_execution_failed(provider, reason)
        }
        ProviderError::ClientBuildFailed { provider, ref reason } => {
            tracing::warn!(provider, reason, "failed to build provider client");
            if reason.to_ascii_lowercase().contains("auth")
                || reason.to_ascii_lowercase().contains("key")
                || reason.to_ascii_lowercase().contains("credential")
            {
                inference_error(
                    InferenceErrorKind::Unauthorized,
                    format!("provider {provider} rejected authentication"),
                )
            } else {
                inference_error(
                    InferenceErrorKind::Unavailable,
                    format!("provider {provider} is temporarily unavailable"),
                )
            }
        }
        ProviderError::SerializationError(ref e) => {
            tracing::warn!(error = %e, "serialization failed");
            inference_error(
                InferenceErrorKind::InvalidResponse,
                "provider response could not be serialized",
            )
        }
        ProviderError::ResponseTooLarge { provider, size } => {
            tracing::warn!(provider, size, "provider response too large");
            inference_error(
                InferenceErrorKind::Provider,
                format!("provider {provider} returned a response that was too large"),
            )
        }
        ProviderError::InvalidUrl { model } => inference_error(
            InferenceErrorKind::InvalidRequest,
            format!("invalid model identifier: {model}"),
        ),
        ProviderError::UnknownModel { provider, model } => inference_error(
            InferenceErrorKind::InvalidRequest,
            format!("unknown model '{model}' for provider {provider:?}"),
        ),
        ProviderError::InvalidModelString { input } => inference_error(
            InferenceErrorKind::InvalidRequest,
            format!("invalid model string: '{input}'"),
        ),
        ProviderError::CodegenFailed { details } => {
            tracing::warn!(details, "code generation failed");
            inference_error(InferenceErrorKind::Provider, "provider request failed")
        }
    }
}

fn classify_http_error(error: &impl std::fmt::Display) -> InferenceError {
    let rendered = error.to_string();
    let haystack = rendered.to_ascii_lowercase();

    if haystack.contains("timeout") || haystack.contains("timed out") {
        return inference_error(InferenceErrorKind::Timeout, "provider request timed out");
    }

    if haystack.contains("connect")
        || haystack.contains("dns")
        || haystack.contains("unreachable")
    {
        tracing::warn!(error = %error, "provider network request failed");
        return with_optional_retry_after(
            inference_error(
                InferenceErrorKind::Unavailable,
                "provider is temporarily unavailable",
            ),
            &haystack,
        );
    }

    tracing::warn!(error = %error, "unclassified HTTP error");
    inference_error(InferenceErrorKind::Provider, "provider request failed")
}

fn classify_execution_failed(provider: String, reason: &str) -> InferenceError {
    let haystack = reason.to_ascii_lowercase();

    if haystack.contains("rate limit")
        || haystack.contains("ratelimit")
        || haystack.contains("429")
        || haystack.contains("throttl")
        || haystack.contains("quota")
    {
        return with_optional_retry_after(
            inference_error(InferenceErrorKind::RateLimited, "provider rate-limited the request"),
            &haystack,
        );
    }

    if haystack.contains("unauthor")
        || haystack.contains("401")
        || haystack.contains("403")
        || haystack.contains("forbidden")
        || haystack.contains("api key")
        || haystack.contains("authenticat")
        || haystack.contains("credential")
    {
        return inference_error(
            InferenceErrorKind::Unauthorized,
            format!("provider {provider} rejected authentication"),
        );
    }

    if haystack.contains("timeout") || haystack.contains("timed out") {
        return inference_error(InferenceErrorKind::Timeout, "provider request timed out");
    }

    // Local providers (e.g. Ollama) resolve without credentials, so a profile
    // request can target a provider with nothing listening. The rig backend
    // flattens that transport failure into this opaque reason string, so the
    // network wording must classify here as well as in `classify_http_error`.
    if haystack.contains("overload")
        || haystack.contains("unavailable")
        || haystack.contains("502")
        || haystack.contains("503")
        || haystack.contains("504")
        || haystack.contains("internal server error")
        || haystack.contains("connect")
        || haystack.contains("dns")
        || haystack.contains("network")
        || haystack.contains("unreachable")
    {
        return with_optional_retry_after(
            inference_error(
                InferenceErrorKind::Unavailable,
                "provider is temporarily unavailable",
            ),
            &haystack,
        );
    }

    tracing::warn!(provider, reason, "provider execution failed");
    inference_error(InferenceErrorKind::Provider, "provider request failed")
}

/// Attaches a `retry_after` hint to `error` when `haystack` (already
/// lowercased) carries a provider-supplied retry delay in whole seconds.
///
/// The execution layer flattens provider responses to opaque strings, so the
/// only place a `Retry-After` header survives is the rendered error text.
/// Recognized forms: `retry-after: 30`, `retry after 30`, and `try again in 30
/// seconds`. A missing or unparseable hint leaves the error unchanged.
fn with_optional_retry_after(error: InferenceError, haystack: &str) -> InferenceError {
    match parse_retry_after_seconds(haystack) {
        Some(seconds) => error.with_retry_after(Duration::from_secs(seconds)),
        None => error,
    }
}

fn parse_retry_after_seconds(haystack: &str) -> Option<u64> {
    const MARKERS: &[&str] = &["retry-after", "retry after", "try again in"];

    for marker in MARKERS {
        if let Some(index) = haystack.find(marker) {
            let tail = &haystack[index + marker.len()..];
            // Skip separators (`:`, whitespace) and take the leading run of
            // digits as the delay in seconds.
            let digits: String = tail
                .trim_start_matches([':', ' ', '\t'])
                .chars()
                .take_while(char::is_ascii_digit)
                .collect();
            if let Ok(seconds) = digits.parse::<u64>() {
                return Some(seconds);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use unchained_ai::rigging::providers::Provider;

    #[test]
    fn missing_api_key_on_explicit_model_is_unauthorized() {
        let err = classify_provider_error(
            ProviderError::MissingApiKey {
                provider: "openai".to_string(),
                env_vars: vec!["OPENAI_API_KEY".to_string()],
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unauthorized);
    }

    #[test]
    fn missing_api_key_in_profile_selection_is_unavailable() {
        let err = classify_provider_error(
            ProviderError::MissingApiKey {
                provider: "openai".to_string(),
                env_vars: vec!["OPENAI_API_KEY".to_string()],
            },
            false,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unavailable);
    }

    #[test]
    fn no_runnable_model_is_unavailable() {
        let err = classify_provider_error(
            ProviderError::NoRunnableModel {
                capability: "Normal".to_string(),
            },
            false,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unavailable);
    }

    #[test]
    fn authentication_failed_is_unauthorized() {
        let err = classify_provider_error(
            ProviderError::AuthenticationFailed {
                provider: "anthropic".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unauthorized);
    }

    #[test]
    fn rate_limit_exceeded_is_rate_limited() {
        let err = classify_provider_error(
            ProviderError::RateLimitExceeded {
                provider: "openai".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::RateLimited);
    }

    #[test]
    fn timeout_is_timeout() {
        let err = classify_provider_error(
            ProviderError::Timeout {
                provider: "openai".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Timeout);
    }

    #[test]
    fn execution_failed_with_rate_limit_text_is_rate_limited() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "openai".to_string(),
                reason: "rate limit exceeded, retry after 429".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::RateLimited);
    }

    #[test]
    fn execution_failed_with_auth_text_is_unauthorized() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "openai".to_string(),
                reason: "401 Unauthorized: invalid API key".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unauthorized);
    }

    #[test]
    fn execution_failed_with_timeout_text_is_timeout() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "openai".to_string(),
                reason: "request timed out".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Timeout);
    }

    #[test]
    fn execution_failed_with_unavailable_text_is_unavailable() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "openai".to_string(),
                reason: "503 Service Unavailable".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unavailable);
    }

    #[test]
    fn execution_failed_with_connection_refused_text_is_unavailable() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "LLM".to_string(),
                reason: "error sending request: tcp connect error: connection refused (os error 61)"
                    .to_string(),
            },
            false,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unavailable);
    }

    #[test]
    fn execution_failed_with_dns_failure_text_is_unavailable() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "LLM".to_string(),
                reason: "dns error: failed to lookup address information: nodename nor servname provided"
                    .to_string(),
            },
            false,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unavailable);
    }

    #[test]
    fn client_build_failed_with_auth_text_is_unauthorized() {
        let err = classify_provider_error(
            ProviderError::ClientBuildFailed {
                provider: "openai".to_string(),
                reason: "authentication failed: missing credentials".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unauthorized);
    }

    #[test]
    fn invalid_model_string_is_invalid_request() {
        let err = classify_provider_error(
            ProviderError::InvalidModelString {
                input: "not-a-model".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::InvalidRequest);
    }

    #[test]
    fn unknown_model_is_invalid_request() {
        let err = classify_provider_error(
            ProviderError::UnknownModel {
                provider: Provider::OpenAi,
                model: "unknown".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::InvalidRequest);
    }

    #[test]
    fn unsupported_operation_is_unsupported() {
        let err = classify_provider_error(
            ProviderError::Unsupported {
                provider: "HuggingFace".to_string(),
                reason: "text completion is not supported".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unsupported);
    }

    #[test]
    fn structured_parse_failure_is_invalid_response() {
        let err = classify_provider_error(
            ProviderError::StructuredParse {
                reason: "output was not a single JSON value".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::InvalidResponse);
    }

    #[test]
    fn rate_limit_text_with_retry_after_populates_duration() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "openai".to_string(),
                reason: "429 Too Many Requests; Retry-After: 30".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::RateLimited);
        assert_eq!(err.retry_after, Some(Duration::from_secs(30)));
    }

    #[test]
    fn unavailable_text_with_try_again_populates_retry_after() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "anthropic".to_string(),
                reason: "503 overloaded, try again in 12 seconds".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::Unavailable);
        assert_eq!(err.retry_after, Some(Duration::from_secs(12)));
    }

    #[test]
    fn rate_limit_without_retry_hint_leaves_retry_after_unset() {
        let err = classify_provider_error(
            ProviderError::ExecutionFailed {
                provider: "openai".to_string(),
                reason: "quota exceeded for this billing period".to_string(),
            },
            true,
        );
        assert_eq!(err.kind, InferenceErrorKind::RateLimited);
        assert_eq!(err.retry_after, None);
    }
}
