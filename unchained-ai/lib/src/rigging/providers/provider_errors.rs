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

    #[error("No runnable model found for capability {capability}")]
    NoRunnableModel { capability: String },

    #[error("LLM execution failed for {provider}: {reason}")]
    ExecutionFailed { provider: String, reason: String },

    /// The requested operation is not supported by the resolved model or
    /// provider at all (as opposed to a transient credential/availability
    /// problem). Distinct from [`ExecutionFailed`] so callers can surface a
    /// stable "unsupported" category instead of a generic provider failure.
    #[error("Operation not supported by {provider}: {reason}")]
    Unsupported { provider: String, reason: String },

    /// A structured-output response could not be parsed into a single JSON
    /// value. Carried as a distinct variant so the contract layer can map it to
    /// an invalid-response category rather than a generic provider failure.
    #[error("Structured response was not a single valid JSON value: {reason}")]
    StructuredParse { reason: String },
}
