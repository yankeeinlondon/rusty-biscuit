//! Authorization code flow with PKCE support.

use oauth2::{AuthorizationCode, CsrfToken, PkceCodeChallenge, PkceCodeVerifier, Scope};
use schematic_define::PkceRequirement;

use crate::error::OAuthError;
use crate::manager::{OAuth2Manager, build_http_client, extract_tokens};
use crate::types::{AuthorizationSession, StoredTokens};

impl OAuth2Manager {
    /// Begins an authorization code flow by building the authorization URL.
    ///
    /// The caller should direct the user to the returned URL. After the user
    /// authorizes, pass the callback `code` and `state` to [`Self::exchange_code`].
    ///
    /// ## Errors
    ///
    /// Returns `OAuthError::Configuration` if the authorization URL is not set
    /// on the provider config.
    pub fn begin_authorization(&self) -> Result<AuthorizationSession, OAuthError> {
        let mut auth_request = self
            .client
            .authorize_url(CsrfToken::new_random)
            .map_err(|e| OAuthError::Configuration(e.to_string()))?;

        for scope in &self.config.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }

        let pkce_verifier = match self.config.provider.pkce {
            PkceRequirement::Required | PkceRequirement::Supported => {
                let (challenge, verifier) = PkceCodeChallenge::new_random_sha256();
                auth_request = auth_request.set_pkce_challenge(challenge);
                Some(verifier.secret().to_string())
            }
            PkceRequirement::NotUsed | _ => None,
        };

        let (url, csrf_state) = auth_request.url();

        Ok(AuthorizationSession {
            authorization_url: url.to_string(),
            csrf_state: csrf_state.secret().to_string(),
            pkce_verifier,
        })
    }

    /// Exchanges an authorization code for tokens and stores them.
    ///
    /// Validates the CSRF state parameter before performing the exchange.
    ///
    /// ## Errors
    ///
    /// - `OAuthError::StateMismatch` if the CSRF state does not match.
    /// - `OAuthError::TokenExchange` if the token endpoint returns an error.
    pub async fn exchange_code(
        &self,
        code: &str,
        state: &str,
        session: &AuthorizationSession,
    ) -> Result<StoredTokens, OAuthError> {
        if state != session.csrf_state {
            return Err(OAuthError::StateMismatch {
                expected: session.csrf_state.clone(),
                actual: state.to_string(),
            });
        }

        let mut token_request = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()));

        if let Some(ref verifier) = session.pkce_verifier {
            token_request =
                token_request.set_pkce_verifier(PkceCodeVerifier::new(verifier.clone()));
        }

        let http_client = build_http_client()?;
        let token_response = token_request
            .request_async(&http_client)
            .await
            .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

        let tokens = extract_tokens(&token_response, &self.config.scopes);

        let store = self.store.write().await;
        store
            .save(&tokens)
            .map_err(|e| OAuthError::TokenStore(e.to_string()))?;

        Ok(tokens)
    }
}
