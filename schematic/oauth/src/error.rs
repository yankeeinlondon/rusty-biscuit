//! OAuth2 error types for the schematic OAuth2 runtime.

use thiserror::Error;

/// Errors that can occur during OAuth2 operations.
#[derive(Debug, Error)]
pub enum OAuthError {
    /// OAuth2 configuration is invalid or incomplete.
    #[error("OAuth2 configuration error: {0}")]
    Configuration(String),

    /// No valid token is available; the user must complete authorization first.
    #[error("OAuth2 authentication required: complete the authorization flow first")]
    AuthenticationRequired,

    /// CSRF state parameter mismatch between request and callback.
    ///
    /// The state values are intentionally omitted from the `Display` message to
    /// avoid leaking CSRF tokens into logs; inspect the fields directly if needed.
    #[error("OAuth2 CSRF state mismatch: the CSRF state parameter did not match the expected value")]
    StateMismatch {
        /// The expected CSRF state value.
        expected: String,
        /// The actual CSRF state value received.
        actual: String,
    },

    /// Token exchange (authorization code -> tokens) failed.
    #[error("OAuth2 token exchange failed: {0}")]
    TokenExchange(String),

    /// Token refresh failed.
    #[error("OAuth2 token refresh failed: {0}")]
    TokenRefresh(String),

    /// Token store read/write error.
    #[error("OAuth2 token store error: {0}")]
    TokenStore(String),

    /// HTTP transport error during an OAuth2 flow.
    #[error("HTTP error during OAuth2 flow: {0}")]
    Http(#[from] reqwest::Error),
}
