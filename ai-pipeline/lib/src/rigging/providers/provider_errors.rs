use thiserror::Error;

use super::Provider;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("Rate limit exceeded for provider {provider}")]
    RateLimitExceeded { provider: String },

    #[error("API authentication failed for {provider}")]
    AuthenticationFailed { provider: String },

    #[error("JSON serialization failed: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Timeout waiting for {provider}")]
    Timeout { provider: String },

    #[error("Response too large from {provider}: {size} bytes")]
    ResponseTooLarge { provider: String, size: usize },

    #[error("Invalid model name for URL generation: {model}")]
    InvalidUrl { model: String },

    #[error("Unknown model '{model}' for provider {provider:?}")]
    UnknownModel { provider: Provider, model: String },

    #[error("Invalid model string format: '{input}' (expected 'provider/model-id')")]
    InvalidModelString { input: String },

    #[error("Validation timeout for provider {provider:?}")]
    ValidationTimeout { provider: Provider },

    #[error("No providers available (no API keys configured)")]
    NoProvidersAvailable,

    #[error("Code generation failed: {details}")]
    CodegenFailed { details: String },

    #[error("Missing API key for {provider}. Set one of: {}", env_vars.join(", "))]
    MissingApiKey {
        provider: String,
        env_vars: Vec<String>,
    },

    #[error("Failed to build client for {provider}: {reason}")]
    ClientBuildFailed { provider: String, reason: String },
}
