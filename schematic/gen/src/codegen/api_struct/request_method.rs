//! Accessor and introspection method generation for API client structs.

use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generates accessor methods that expose internal state of the API client.
///
/// Produces:
/// - `http_client()` - Returns a reference to the underlying HTTP client
/// - `api_base_url()` - Returns the base URL
/// - `auth_policy()` - Returns the effective authentication policy
/// - `oauth_config()` - Returns OAuth2 provider metadata, if any
/// - `api_key_header()` - Returns the API key header name and value
pub(super) fn generate_accessors(_struct_name: &Ident) -> TokenStream {
    quote! {
        /// Returns a reference to the underlying HTTP client.
        ///
        /// Use this for custom requests that aren't covered by the generated methods,
        /// such as paginated endpoints that require query parameters.
        pub fn http_client(&self) -> &reqwest::Client {
            &self.client
        }

        /// Returns the base URL for this API client.
        pub fn api_base_url(&self) -> &str {
            &self.base_url
        }

        /// Returns the effective authentication policy for this client.
        pub fn auth_policy(&self) -> &schematic_define::AuthPolicy {
            &self.auth_policy
        }

        /// Returns OAuth2 provider metadata, if this client accepts OAuth tokens.
        pub fn oauth_config(&self) -> Option<&schematic_define::OAuth2Config> {
            self.auth_policy.oauth2()
        }

        /// Returns the API key header name and value for authentication.
        ///
        /// Returns `None` if the authentication strategy is not `ApiKey`
        /// or if the API key environment variable is not set.
        pub fn api_key_header(&self) -> Option<(String, String)> {
            let (header, value_prefix) = self
                .auth_policy
                .explicit
                .iter()
                .find_map(|method| match method {
                    schematic_define::AuthMethod::ApiKey { header, value_prefix } => {
                        Some((header.clone(), value_prefix.clone()))
                    }
                    _ => None,
                })
                .or_else(|| {
                    self.headers
                        .env_mapping()
                        .api_key
                        .as_ref()
                        .map(|api_key| (api_key.header.clone(), None))
                })?;

            self.headers
                .env_mapping()
                .api_key
                .as_ref()
                .and_then(|api_key| {
                    api_key
                        .names
                        .names()
                        .iter()
                        .find_map(|env_name| std::env::var(env_name).ok())
                })
                .map(|value| {
                    let value = format!("{}{}", value_prefix.clone().unwrap_or_default(), value);
                    (header, value)
                })
        }
    }
}
