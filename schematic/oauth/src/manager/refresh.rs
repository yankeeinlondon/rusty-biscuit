//! Token refresh flow.

use oauth2::RefreshToken;

use crate::error::OAuthError;
use crate::manager::{build_http_client, extract_tokens, OAuth2Manager};
use crate::types::StoredTokens;

impl OAuth2Manager {
    /// Returns a valid access token string, refreshing if necessary.
    ///
    /// ## Errors
    ///
    /// - `OAuthError::AuthenticationRequired` if no token is stored.
    /// - `OAuthError::TokenRefresh` if refresh fails.
    pub async fn get_valid_token(&self) -> Result<String, OAuthError> {
        let store = self.store.read().await;
        let tokens = store.load()?;
        drop(store);

        match tokens {
            Some(tokens) if !tokens.is_expired() => Ok(tokens.access_token),
            Some(tokens) if tokens.refresh_token.is_some() => self.refresh_token(&tokens).await,
            Some(_) | None => Err(OAuthError::AuthenticationRequired),
        }
    }

    /// Refreshes the token using the stored refresh token.
    pub(super) async fn refresh_token(&self, tokens: &StoredTokens) -> Result<String, OAuthError> {
        let refresh_token = tokens
            .refresh_token
            .as_ref()
            .ok_or(OAuthError::TokenRefresh(
                "No refresh token available".into(),
            ))?;

        let http_client = build_http_client()?;
        let token_response = self
            .client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.clone()))
            .request_async(&http_client)
            .await
            .map_err(|e| OAuthError::TokenRefresh(e.to_string()))?;

        let new_tokens = extract_tokens(&token_response, &self.config.scopes);

        let store = self.store.write().await;
        store.save(&new_tokens)?;

        Ok(new_tokens.access_token)
    }
}
