//! Top-level API error type.

use super::{AuthError, ClientError, ConfigError, ValidationError};
use thiserror::Error;

/// Top-level error type for all API operations.
///
/// This enum aggregates all error categories, enabling unified error handling
/// while preserving the ability to match on specific error types when needed.
///
/// ## Examples
///
/// ```rust,ignore
/// use api::error::ApiError;
///
/// fn handle_error(err: ApiError) {
///     match err {
///         ApiError::Client(e) => eprintln!("Network error: {e}"),
///         ApiError::Validation(e) => eprintln!("Invalid response: {e}"),
///         ApiError::Auth(e) => eprintln!("Auth failed: {e}"),
///         ApiError::Config(e) => eprintln!("Configuration error: {e}"),
///     }
/// }
/// ```
#[derive(Debug, Error)]
pub enum ApiError {
    /// HTTP client errors (network, timeout, connection failures).
    #[error(transparent)]
    Client(#[from] ClientError),

    /// Response validation errors (parse failures, schema violations).
    #[error(transparent)]
    Validation(#[from] ValidationError),

    /// Authentication and authorization errors.
    #[error(transparent)]
    Auth(#[from] AuthError),

    /// Endpoint configuration errors.
    #[error(transparent)]
    Config(#[from] ConfigError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_client_error() {
        let client_err = ClientError::Timeout { duration_ms: 5000 };
        let api_err: ApiError = client_err.into();
        assert!(matches!(api_err, ApiError::Client(_)));
    }

    #[test]
    fn test_from_auth_error() {
        let auth_err = AuthError::MissingApiKey {
            provider: "test".to_string(),
        };
        let api_err: ApiError = auth_err.into();
        assert!(matches!(api_err, ApiError::Auth(_)));
    }

    #[test]
    fn test_error_display() {
        let err = ApiError::Auth(AuthError::InvalidKeyFormat);
        let display = err.to_string();
        assert!(display.contains("Invalid API key format"));
    }
}
