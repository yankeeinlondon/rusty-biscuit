//! Authentication and authorization errors.

use thiserror::Error;

/// Errors related to API authentication.
///
/// These errors occur during authentication setup or when the server
/// rejects credentials.
#[derive(Debug, Error)]
pub enum AuthError {
    /// API key is missing for the specified provider.
    #[error("Missing API key for {provider}")]
    MissingApiKey {
        /// The provider name that requires the API key.
        provider: String,
    },

    /// API key has an invalid format.
    #[error("Invalid API key format")]
    InvalidKeyFormat,

    /// Server rejected the authentication credentials.
    #[error("Authentication failed: {message}")]
    AuthenticationFailed {
        /// Error message from the server.
        message: String,
    },

    /// Token has expired and needs to be refreshed.
    #[error("Token expired")]
    TokenExpired,

    /// Insufficient permissions for the requested operation.
    #[error("Insufficient permissions: {operation}")]
    InsufficientPermissions {
        /// The operation that was denied.
        operation: String,
    },
}

impl AuthError {
    /// Returns `true` if this error could potentially be resolved by
    /// refreshing credentials.
    pub fn is_refreshable(&self) -> bool {
        matches!(self, Self::TokenExpired | Self::AuthenticationFailed { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_api_key_display() {
        let err = AuthError::MissingApiKey {
            provider: "OpenAI".to_string(),
        };
        assert_eq!(err.to_string(), "Missing API key for OpenAI");
    }

    #[test]
    fn test_token_expired_is_refreshable() {
        let err = AuthError::TokenExpired;
        assert!(err.is_refreshable());
    }

    #[test]
    fn test_invalid_format_not_refreshable() {
        let err = AuthError::InvalidKeyFormat;
        assert!(!err.is_refreshable());
    }

    #[test]
    fn test_insufficient_permissions() {
        let err = AuthError::InsufficientPermissions {
            operation: "delete_user".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "Insufficient permissions: delete_user"
        );
        assert!(!err.is_refreshable());
    }
}
