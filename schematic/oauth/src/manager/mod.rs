//! OAuth2 lifecycle manager.
//!
//! [`OAuth2Manager`] wraps the `oauth2` crate's `BasicClient` and provides
//! high-level methods for authorization code, client credentials, token
//! refresh, and revocation flows.

mod authorization_code;
mod client_credentials;
mod refresh;
mod revocation;

use oauth2::basic::{
    BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
    BasicTokenResponse,
};
use oauth2::{
    AuthUrl, ClientId, ClientSecret, EndpointMaybeSet, EndpointNotSet, EndpointSet, RedirectUrl,
    RevocationUrl, TokenResponse, TokenUrl,
};
use tokio::sync::Mutex;

use crate::error::OAuthError;
use crate::store::TokenStore;
use crate::types::{OAuth2RuntimeConfig, StoredTokens};

/// Concrete client type with all optional endpoints using `EndpointMaybeSet`.
///
/// The token URL is always set (`EndpointSet`), while authorization and
/// revocation URLs may or may not be configured at runtime.
type OAuthClient = oauth2::Client<
    BasicErrorResponse,
    BasicTokenResponse,
    BasicTokenIntrospectionResponse,
    oauth2::StandardRevocableToken,
    BasicRevocationErrorResponse,
    EndpointMaybeSet, // auth URL
    EndpointNotSet,   // device auth URL
    EndpointNotSet,   // introspection URL
    EndpointMaybeSet, // revocation URL
    EndpointSet,      // token URL
>;

/// Manages the OAuth2 token lifecycle for a single API provider.
///
/// Supports authorization code (with PKCE), client credentials, refresh, and
/// revocation flows. Token refreshes are serialized through `refresh_lock` so
/// concurrent callers of [`OAuth2Manager::get_valid_token`] perform a single
/// refresh rather than stampeding the token endpoint.
///
/// ## Examples
///
/// ```no_run
/// use schematic_oauth::{OAuth2Manager, OAuth2RuntimeConfig, MemoryTokenStore};
/// use schematic_define::{OAuth2Config, OAuth2GrantType, PkceRequirement, OAuth2ClientAuthMethod};
///
/// let config = OAuth2RuntimeConfig {
///     provider: OAuth2Config {
///         grant_type: OAuth2GrantType::AuthorizationCodePkce,
///         authorization_url: Some("https://example.com/authorize".into()),
///         token_url: "https://example.com/token".into(),
///         revocation_url: None,
///         device_authorization_url: None,
///         default_scopes: vec!["read".into()],
///         pkce: PkceRequirement::Required,
///         client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
///     },
///     client_id: "my_client_id".into(),
///     client_secret: Some("my_secret".into()),
///     redirect_uri: Some("http://localhost:8080/callback".into()),
///     scopes: vec!["read".into(), "write".into()],
/// };
///
/// let manager = OAuth2Manager::new(config, Box::new(MemoryTokenStore::new())).unwrap();
/// ```
pub struct OAuth2Manager {
    pub(super) client: OAuthClient,
    pub(super) config: OAuth2RuntimeConfig,
    /// `TokenStore` is `Send + Sync` with `&self` methods, so the store needs no
    /// outer lock for access; single-flight refresh is guarded separately.
    pub(super) store: Box<dyn TokenStore>,
    /// Shared HTTP client reused across token operations.
    pub(super) http_client: reqwest::Client,
    /// Serializes the load -> check-expiry -> refresh -> save critical section.
    pub(super) refresh_lock: Mutex<()>,
}

impl OAuth2Manager {
    /// Creates a new OAuth2 manager from the given runtime configuration and token store.
    ///
    /// ## Errors
    ///
    /// Returns `OAuthError::Configuration` if any of the provider URLs are malformed.
    pub fn new(
        config: OAuth2RuntimeConfig,
        store: Box<dyn TokenStore>,
    ) -> Result<Self, OAuthError> {
        let token_url = TokenUrl::new(config.provider.token_url.clone())
            .map_err(|e| OAuthError::Configuration(format!("Invalid token URL: {e}")))?;

        let auth_url = config
            .provider
            .authorization_url
            .as_ref()
            .map(|u| AuthUrl::new(u.clone()))
            .transpose()
            .map_err(|e| OAuthError::Configuration(format!("Invalid authorization URL: {e}")))?;

        let revocation_url = config
            .provider
            .revocation_url
            .as_ref()
            .map(|u| RevocationUrl::new(u.clone()))
            .transpose()
            .map_err(|e| OAuthError::Configuration(format!("Invalid revocation URL: {e}")))?;

        let redirect_url = config
            .redirect_uri
            .as_ref()
            .map(|u| RedirectUrl::new(u.clone()))
            .transpose()
            .map_err(|e| OAuthError::Configuration(format!("Invalid redirect URI: {e}")))?;

        let mut client = oauth2::Client::new(ClientId::new(config.client_id.clone()))
            .set_auth_uri_option(auth_url)
            .set_token_uri(token_url)
            .set_revocation_url_option(revocation_url);

        if let Some(ref secret) = config.client_secret {
            client = client.set_client_secret(ClientSecret::new(secret.clone()));
        }

        if let Some(redirect) = redirect_url {
            client = client.set_redirect_uri(redirect);
        }

        Ok(Self {
            client,
            config,
            store,
            http_client: build_http_client()?,
            refresh_lock: Mutex::new(()),
        })
    }
}

/// Builds a reqwest HTTP client configured for OAuth2 flows.
///
/// Disables redirect following to prevent SSRF vulnerabilities, as
/// recommended by the `oauth2` crate documentation.
pub(super) fn build_http_client() -> Result<reqwest::Client, OAuthError> {
    reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(OAuthError::Http)
}

/// Extracts token data from an OAuth2 token response into a `StoredTokens`.
pub(super) fn extract_tokens<EF, TT>(
    response: &oauth2::StandardTokenResponse<EF, TT>,
    default_scopes: &[String],
) -> StoredTokens
where
    EF: oauth2::ExtraTokenFields,
    TT: oauth2::TokenType,
{
    let expires_at = response
        .expires_in()
        .map(|duration| chrono::Utc::now() + chrono::Duration::seconds(duration.as_secs() as i64));

    let scopes: Vec<String> = response
        .scopes()
        .map(|s| s.iter().map(|scope| scope.to_string()).collect())
        .unwrap_or_else(|| default_scopes.to_vec());

    StoredTokens {
        access_token: response.access_token().secret().to_string(),
        refresh_token: response.refresh_token().map(|rt| rt.secret().to_string()),
        expires_at,
        scopes,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::MemoryTokenStore;
    use schematic_define::{
        OAuth2ClientAuthMethod, OAuth2Config, OAuth2GrantType, PkceRequirement,
    };

    pub(super) fn auth_code_config() -> OAuth2RuntimeConfig {
        OAuth2RuntimeConfig {
            provider: OAuth2Config {
                grant_type: OAuth2GrantType::AuthorizationCodePkce,
                authorization_url: Some("https://example.com/authorize".into()),
                token_url: "https://example.com/token".into(),
                revocation_url: None,
                device_authorization_url: None,
                default_scopes: vec!["read".into()],
                pkce: PkceRequirement::Required,
                client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
            },
            client_id: "test_client".into(),
            client_secret: Some("test_secret".into()),
            redirect_uri: Some("http://localhost:8080/callback".into()),
            scopes: vec!["read".into(), "write".into()],
        }
    }

    pub(super) fn client_credentials_config() -> OAuth2RuntimeConfig {
        OAuth2RuntimeConfig {
            provider: OAuth2Config {
                grant_type: OAuth2GrantType::ClientCredentials,
                authorization_url: None,
                token_url: "https://example.com/token".into(),
                revocation_url: None,
                device_authorization_url: None,
                default_scopes: vec!["api".into()],
                pkce: PkceRequirement::NotUsed,
                client_auth: OAuth2ClientAuthMethod::ClientSecretBasic,
            },
            client_id: "service_client".into(),
            client_secret: Some("service_secret".into()),
            redirect_uri: None,
            scopes: vec!["api".into()],
        }
    }

    #[test]
    fn manager_creation_succeeds_with_valid_config() {
        let result = OAuth2Manager::new(auth_code_config(), Box::new(MemoryTokenStore::new()));
        assert!(result.is_ok());
    }

    #[test]
    fn manager_creation_fails_with_invalid_token_url() {
        let mut config = auth_code_config();
        config.provider.token_url = "not a url".into();
        let result = OAuth2Manager::new(config, Box::new(MemoryTokenStore::new()));
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("Invalid token URL"), "got: {err_msg}");
    }

    #[test]
    fn begin_authorization_returns_url_and_state() {
        let manager =
            OAuth2Manager::new(auth_code_config(), Box::new(MemoryTokenStore::new())).unwrap();

        let session = manager.begin_authorization().unwrap();

        assert!(session
            .authorization_url
            .starts_with("https://example.com/authorize"));
        assert!(session.authorization_url.contains("client_id=test_client"));
        assert!(!session.csrf_state.is_empty());
        assert!(session.pkce_verifier.is_some());
    }

    #[test]
    fn begin_authorization_includes_pkce_challenge() {
        let manager =
            OAuth2Manager::new(auth_code_config(), Box::new(MemoryTokenStore::new())).unwrap();

        let session = manager.begin_authorization().unwrap();
        assert!(session.authorization_url.contains("code_challenge="));
        assert!(session
            .authorization_url
            .contains("code_challenge_method=S256"));
    }

    #[test]
    fn begin_authorization_without_pkce_when_not_used() {
        let mut config = auth_code_config();
        config.provider.pkce = PkceRequirement::NotUsed;
        let manager = OAuth2Manager::new(config, Box::new(MemoryTokenStore::new())).unwrap();

        let session = manager.begin_authorization().unwrap();
        assert!(session.pkce_verifier.is_none());
        assert!(!session.authorization_url.contains("code_challenge"));
    }

    #[test]
    fn begin_authorization_fails_without_auth_url() {
        let manager = OAuth2Manager::new(
            client_credentials_config(),
            Box::new(MemoryTokenStore::new()),
        )
        .unwrap();

        let result = manager.begin_authorization();
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn exchange_code_rejects_csrf_mismatch() {
        let manager =
            OAuth2Manager::new(auth_code_config(), Box::new(MemoryTokenStore::new())).unwrap();

        let session = manager.begin_authorization().unwrap();
        let result = manager
            .exchange_code("some_code", "wrong_state", &session)
            .await;

        assert!(result.is_err());
        let err = result.err().unwrap();
        match err {
            OAuthError::StateMismatch { expected, actual } => {
                assert_eq!(expected, session.csrf_state);
                assert_eq!(actual, "wrong_state");
            }
            other => panic!("Expected StateMismatch, got: {other}"),
        }
    }

    #[tokio::test]
    async fn get_valid_token_returns_auth_required_when_no_token() {
        let manager =
            OAuth2Manager::new(auth_code_config(), Box::new(MemoryTokenStore::new())).unwrap();

        let result = manager.get_valid_token().await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(
            err_msg.contains("authentication required"),
            "got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn get_valid_token_returns_stored_token() {
        let store = MemoryTokenStore::new();
        store
            .save(&StoredTokens {
                access_token: "valid_token".into(),
                refresh_token: None,
                expires_at: Some(chrono::Utc::now() + chrono::Duration::hours(1)),
                scopes: vec!["read".into()],
            })
            .unwrap();

        let manager = OAuth2Manager::new(auth_code_config(), Box::new(store)).unwrap();

        let token = manager.get_valid_token().await.unwrap();
        assert_eq!(token, "valid_token");
    }

    #[tokio::test]
    async fn get_valid_token_returns_auth_required_for_expired_without_refresh() {
        let store = MemoryTokenStore::new();
        store
            .save(&StoredTokens {
                access_token: "expired_token".into(),
                refresh_token: None,
                expires_at: Some(chrono::Utc::now() - chrono::Duration::hours(1)),
                scopes: vec![],
            })
            .unwrap();

        let manager = OAuth2Manager::new(auth_code_config(), Box::new(store)).unwrap();

        let result = manager.get_valid_token().await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(
            err_msg.contains("authentication required"),
            "got: {err_msg}"
        );
    }

    #[tokio::test]
    async fn revoke_token_fails_without_revocation_url() {
        let manager =
            OAuth2Manager::new(auth_code_config(), Box::new(MemoryTokenStore::new())).unwrap();

        let result = manager.revoke_token().await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(err_msg.contains("revocation URL"), "got: {err_msg}");
    }

    #[tokio::test]
    async fn revoke_token_fails_without_stored_token() {
        let mut config = auth_code_config();
        config.provider.revocation_url = Some("https://example.com/revoke".into());
        let manager = OAuth2Manager::new(config, Box::new(MemoryTokenStore::new())).unwrap();

        let result = manager.revoke_token().await;
        assert!(result.is_err());
        let err_msg = format!("{}", result.err().unwrap());
        assert!(
            err_msg.contains("authentication required"),
            "got: {err_msg}"
        );
    }
}
