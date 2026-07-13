//! Client credentials flow.

use oauth2::Scope;

use crate::error::OAuthError;
use crate::manager::{extract_tokens, OAuth2Manager};
use crate::types::StoredTokens;

impl OAuth2Manager {
    /// Acquires a token using the client credentials grant.
    ///
    /// Stores the resulting token for subsequent use via [`Self::get_valid_token`].
    ///
    /// ## Errors
    ///
    /// Returns `OAuthError::TokenExchange` if the token endpoint returns an error.
    pub async fn acquire_client_credentials_token(&self) -> Result<StoredTokens, OAuthError> {
        let mut request = self.client.exchange_client_credentials();

        for scope in &self.config.scopes {
            request = request.add_scope(Scope::new(scope.clone()));
        }

        let token_response = request
            .request_async(&self.http_client)
            .await
            .map_err(|e| OAuthError::TokenExchange(e.to_string()))?;

        let tokens = extract_tokens(&token_response, &self.config.scopes);

        self.store.save(&tokens)?;

        Ok(tokens)
    }
}
