//! Core types for the OAuth2 runtime.

use serde::{Deserialize, Serialize};

/// Runtime configuration for OAuth2, combining provider config with client credentials.
///
/// This bridges the declarative `OAuth2Config` from `schematic-define` with the
/// consumer-specific credentials needed at runtime (client ID, secret, redirect URI).
///
/// `Debug` is hand-written to redact `client_secret`; deriving it would leak the
/// live secret into any `{:?}`, `dbg!`, or panic message.
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

impl std::fmt::Debug for OAuth2RuntimeConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OAuth2RuntimeConfig")
            .field("provider", &self.provider)
            .field("client_id", &self.client_id)
            .field("client_secret", &redact_option(&self.client_secret))
            .field("redirect_uri", &self.redirect_uri)
            .field("scopes", &self.scopes)
            .finish()
    }
}

/// Represents an in-progress authorization session.
///
/// Returned by [`crate::OAuth2Manager::begin_authorization`] and consumed by
/// [`crate::OAuth2Manager::exchange_code`] to complete the authorization code flow.
///
/// `Debug` is hand-written to redact `pkce_verifier`; deriving it would leak the
/// PKCE secret into any `{:?}`, `dbg!`, or panic message.
pub struct AuthorizationSession {
    /// The URL the user should visit to authorize the application.
    pub authorization_url: String,
    /// CSRF state token for validating the callback.
    pub csrf_state: String,
    /// PKCE code verifier (present when PKCE is used).
    pub pkce_verifier: Option<String>,
}

impl std::fmt::Debug for AuthorizationSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AuthorizationSession")
            .field("authorization_url", &self.authorization_url)
            .field("csrf_state", &self.csrf_state)
            .field("pkce_verifier", &redact_option(&self.pkce_verifier))
            .finish()
    }
}

/// Renders a secret `Option<String>` for `Debug`: `Some(_)` becomes the literal
/// `[redacted]`, `None` stays `None`, so presence is visible but content is not.
fn redact_option(value: &Option<String>) -> Option<&'static str> {
    value.as_ref().map(|_| "[redacted]")
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
///
/// `Debug` is hand-written to redact `access_token` and `refresh_token`; deriving
/// it would leak live credentials into any `{:?}`, `dbg!`, or panic message.
#[derive(Clone, Serialize, Deserialize)]
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

impl std::fmt::Debug for StoredTokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoredTokens")
            .field("access_token", &"[redacted]")
            .field("refresh_token", &redact_option(&self.refresh_token))
            .field("expires_at", &self.expires_at)
            .field("scopes", &self.scopes)
            .finish()
    }
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
    fn debug_redacts_token_material() {
        let tokens = StoredTokens {
            access_token: "super_secret_access".into(),
            refresh_token: Some("super_secret_refresh".into()),
            expires_at: None,
            scopes: vec!["read".into()],
        };

        let rendered = format!("{tokens:?}");
        assert!(!rendered.contains("super_secret_access"), "got: {rendered}");
        assert!(!rendered.contains("super_secret_refresh"), "got: {rendered}");
        assert!(rendered.contains("[redacted]"), "got: {rendered}");
        // Non-secret metadata is still visible.
        assert!(rendered.contains("read"), "got: {rendered}");
    }

    #[test]
    fn debug_shows_none_refresh_token_as_none() {
        let tokens = StoredTokens {
            access_token: "secret".into(),
            refresh_token: None,
            expires_at: None,
            scopes: vec![],
        };

        let rendered = format!("{tokens:?}");
        assert!(rendered.contains("refresh_token: None"), "got: {rendered}");
    }

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
