//! Token revocation flow.

use oauth2::AccessToken;

use crate::error::OAuthError;
use crate::manager::OAuth2Manager;

impl OAuth2Manager {
    /// Revokes the current token if a revocation URL is configured.
    ///
    /// Clears the token store after successful revocation.
    ///
    /// ## Errors
    ///
    /// - `OAuthError::Configuration` if no revocation URL is configured.
    /// - `OAuthError::TokenRefresh` if the revocation request fails.
    /// - `OAuthError::AuthenticationRequired` if no token is stored.
    pub async fn revoke_token(&self) -> Result<(), OAuthError> {
        if self.config.provider.revocation_url.is_none() {
            return Err(OAuthError::Configuration(
                "No revocation URL configured for this provider".into(),
            ));
        }

        let tokens = self.store.load()?.ok_or(OAuthError::AuthenticationRequired)?;

        let token_to_revoke = AccessToken::new(tokens.access_token);

        self.client
            .revoke_token(token_to_revoke.into())
            .map_err(|e| OAuthError::Configuration(e.to_string()))?
            .request_async(&self.http_client)
            .await
            .map_err(|e| OAuthError::TokenRefresh(format!("Revocation failed: {e}")))?;

        self.store.clear()?;

        Ok(())
    }
}
