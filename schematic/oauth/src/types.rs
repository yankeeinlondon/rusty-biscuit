//! Core types for the OAuth2 runtime.

use serde::{Deserialize, Serialize};

/// Runtime configuration for OAuth2, combining provider config with client credentials.
///
/// This bridges the declarative `OAuth2Config` from `schematic-define` with the
/// consumer-specific credentials needed at runtime (client ID, secret, redirect URI).
#[derive(Debug)]
pub struct OAuth2RuntimeConfig {
    /// Provider-level OAuth2 metadata (endpoints, grant type, PKCE requirements).
    pub provider: schematic_define::OAuth2Config,
    /// OAuth2 client ID for this application.
    pub client_id: String,
    /// OAuth2 client secret (not needed for public clients).
    pub client_secret: Option<String>,
    /// Redirect URI for authorization code flows.
    pub redirect_uri: Option<String>,
    /// Scopes to request during authorization.
    pub scopes: Vec<String>,
}

/// Represents an in-progress authorization session.
///
/// Returned by [`crate::OAuth2Manager::begin_authorization`] and consumed by
/// [`crate::OAuth2Manager::exchange_code`] to complete the authorization code flow.
#[derive(Debug)]
pub struct AuthorizationSession {
    /// The URL the user should visit to authorize the application.
    pub authorization_url: String,
    /// CSRF state token for validating the callback.
    pub csrf_state: String,
    /// PKCE code verifier (present when PKCE is used).
    pub pkce_verifier: Option<String>,
}

/// Stored OAuth2 tokens with metadata.
///
/// ## Examples
///
/// ```
/// use schematic_oauth::StoredTokens;
///
/// let tokens = StoredTokens {
///     access_token: "access_123".into(),
///     refresh_token: Some("refresh_456".into()),
///     expires_at: None,
///     scopes: vec!["read".into()],
/// };
///
/// assert!(!tokens.is_expired());
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredTokens {
    /// The OAuth2 access token.
    pub access_token: String,
    /// The OAuth2 refresh token (if provided by the authorization server).
    pub refresh_token: Option<String>,
    /// When the access token expires (if known).
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
    /// Scopes granted by the authorization server.
    pub scopes: Vec<String>,
}

impl StoredTokens {
    /// Returns true if the token is expired or will expire within the next 30 seconds.
    ///
    /// Tokens without an expiry time are treated as never expiring.
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expires) => chrono::Utc::now() + chrono::Duration::seconds(30) >= expires,
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_without_expiry_is_not_expired() {
        let tokens = StoredTokens {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: None,
            scopes: vec![],
        };
        assert!(!tokens.is_expired());
    }

    #[test]
    fn token_with_future_expiry_is_not_expired() {
        let tokens = StoredTokens {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
            scopes: vec![],
        };
        assert!(!tokens.is_expired());
    }

    #[test]
    fn token_with_past_expiry_is_expired() {
        let tokens = StoredTokens {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
            scopes: vec![],
        };
        assert!(tokens.is_expired());
    }

    #[test]
    fn token_expiring_within_30_seconds_is_expired() {
        let tokens = StoredTokens {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(10)),
            scopes: vec![],
        };
        assert!(tokens.is_expired());
    }

    #[test]
    fn token_expiring_in_more_than_30_seconds_is_not_expired() {
        let tokens = StoredTokens {
            access_token: "test".into(),
            refresh_token: None,
            expires_at: Some(chrono::Utc::now() + chrono::Duration::seconds(60)),
            scopes: vec![],
        };
        assert!(!tokens.is_expired());
    }
}
