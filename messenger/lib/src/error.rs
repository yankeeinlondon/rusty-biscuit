use crate::receipt::ProviderKind;

/// All errors that can occur during message dispatch.
#[derive(Debug, thiserror::Error)]
pub enum MessengerError {
    #[error("invalid message: {0}")]
    InvalidMessage(String),

    #[error("{provider} does not support {feature}")]
    UnsupportedFeature {
        provider: ProviderKind,
        feature: &'static str,
    },

    #[error("missing {provider} configuration: {field}")]
    MissingConfiguration {
        provider: ProviderKind,
        field: &'static str,
    },

    #[error("{provider} authentication failed: {message}")]
    Authentication {
        provider: ProviderKind,
        message: String,
    },

    #[error("{provider} rate limited{}", retry_after_ms.map(|ms| format!(", retry after {ms}ms")).unwrap_or_default())]
    RateLimited {
        provider: ProviderKind,
        retry_after_ms: Option<u64>,
    },

    #[error("{provider} transport error: {message}")]
    Transport {
        provider: ProviderKind,
        message: String,
    },

    #[error("{provider} error{}: {message}", code.as_ref().map(|c| format!(" ({c})")).unwrap_or_default())]
    Provider {
        provider: ProviderKind,
        code: Option<String>,
        message: String,
    },
}
