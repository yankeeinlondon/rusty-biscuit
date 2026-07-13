//! Token refresh flow.

use oauth2::RefreshToken;

use crate::error::OAuthError;
use crate::manager::{extract_tokens, OAuth2Manager};
use crate::types::StoredTokens;

impl OAuth2Manager {
    /// Returns a valid access token string, refreshing if necessary.
    ///
    /// Concurrent callers are serialized through the single-flight refresh guard
    /// and re-check expiry under it, so only the first caller drives a refresh.
    ///
    /// ## Errors
    ///
    /// - `OAuthError::AuthenticationRequired` if no token is stored.
    /// - `OAuthError::TokenRefresh` if refresh fails.
    pub async fn get_valid_token(&self) -> Result<String, OAuthError> {
        // Fast path: return a still-valid token without contending on the guard.
        if let Some(tokens) = self.store.load()? {
            if !tokens.is_expired() {
                return Ok(tokens.access_token);
            }
        }

        // Serialize refresh so concurrent callers don't stampede the token
        // endpoint or race refresh-token rotation.
        let _guard = self.refresh_lock.lock().await;

        // Re-check under the guard: an earlier caller may have already refreshed.
        let tokens = self.store.load()?.ok_or(OAuthError::AuthenticationRequired)?;
        if !tokens.is_expired() {
            return Ok(tokens.access_token);
        }
        if tokens.refresh_token.is_some() {
            self.refresh_token(&tokens).await
        } else {
            Err(OAuthError::AuthenticationRequired)
        }
    }

    /// Refreshes the token using the stored refresh token.
    ///
    /// Callers must already hold `refresh_lock`.
    pub(super) async fn refresh_token(&self, tokens: &StoredTokens) -> Result<String, OAuthError> {
        let refresh_token = tokens
            .refresh_token
            .as_ref()
            .ok_or(OAuthError::TokenRefresh(
                "No refresh token available".into(),
            ))?;

        let token_response = self
            .client
            .exchange_refresh_token(&RefreshToken::new(refresh_token.clone()))
            .request_async(&self.http_client)
            .await
            .map_err(|e| OAuthError::TokenRefresh(e.to_string()))?;

        let mut new_tokens = extract_tokens(&token_response, &self.config.scopes);

        // RFC 6749 §6: a refresh response MAY omit `refresh_token`, in which case
        // the previously issued refresh token remains valid and must be retained.
        if new_tokens.refresh_token.is_none() {
            new_tokens.refresh_token = tokens.refresh_token.clone();
        }

        self.store.save(&new_tokens)?;

        Ok(new_tokens.access_token)
    }
}
