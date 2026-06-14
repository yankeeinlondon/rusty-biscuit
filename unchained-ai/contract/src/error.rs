//! Mapping from `unchained-ai` provider outcomes onto the contract's stable
//! [`InferenceErrorKind`] categories.
//!
//! Provider detail (raw HTTP bodies, auth headers, API keys) is treated as
//! potentially secret-bearing: it is logged through `tracing` only and never
//! placed in [`InferenceError::message`].

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
    let haystack = error.to_string().to_ascii_lowercase();

    if haystack.contains("timeout") || haystack.contains("timed out") {
        return inference_error(InferenceErrorKind::Timeout, "provider request timed out");
    }

    if haystack.contains("connect")
        || haystack.contains("dns")
        || haystack.contains("unreachable")
    {
        tracing::warn!(error = %error, "provider network request failed");
        return inference_error(
            InferenceErrorKind::Unavailable,
            "provider is temporarily unavailable",
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
        return inference_error(InferenceErrorKind::RateLimited, "provider rate-limited the request");
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

    if haystack.contains("overload")
        || haystack.contains("unavailable")
        || haystack.contains("502")
        || haystack.contains("503")
        || haystack.contains("504")
        || haystack.contains("internal server error")
    {
        return inference_error(
            InferenceErrorKind::Unavailable,
            "provider is temporarily unavailable",
        );
    }

    tracing::warn!(provider, reason, "provider execution failed");
    inference_error(InferenceErrorKind::Provider, "provider request failed")
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
}
