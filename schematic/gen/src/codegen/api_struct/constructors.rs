//! Constructor generation for API client structs.

use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generates the constants and constructor methods for the API struct impl block.
///
/// Produces:
/// - `BASE_URL` constant
/// - `DOCS_URL` constant
/// - `new()` constructor
/// - `with_base_url()` constructor
/// - `with_client()` constructor
/// - `with_client_and_base_url()` constructor
pub(super) fn generate_constructors(
    _struct_name: &Ident,
    base_url: &str,
    env_auth: &[String],
    auth_strategy_init: &TokenStream,
    auth_policy_init: &TokenStream,
    env_username_init: &TokenStream,
    headers_init: &TokenStream,
    docs_url_init: &TokenStream,
) -> TokenStream {
    quote! {
        /// Base URL for the API.
        pub const BASE_URL: &'static str = #base_url;

        /// Official API documentation URL, if available.
        pub const DOCS_URL: Option<&'static str> = #docs_url_init;

        /// Creates a new API client with the default base URL.
        pub fn new() -> Self {
            Self {
                client: reqwest::Client::new(),
                base_url: Self::BASE_URL.to_string(),
                env_auth: vec![#(#env_auth.to_string()),*],
                auth_strategy: #auth_strategy_init,
                auth_policy: #auth_policy_init,
                env_username: #env_username_init,
                headers: #headers_init,
                variant_hooks: crate::shared::VariantHooks::default(),
            }
        }

        /// Creates a new API client with a custom base URL.
        ///
        /// ## Examples
        ///
        /// ```text
        /// let client = Api::with_base_url("http://localhost:8080/v1");
        /// ```
        pub fn with_base_url(base_url: impl Into<String>) -> Self {
            Self {
                client: reqwest::Client::new(),
                base_url: base_url.into(),
                env_auth: vec![#(#env_auth.to_string()),*],
                auth_strategy: #auth_strategy_init,
                auth_policy: #auth_policy_init,
                env_username: #env_username_init,
                headers: #headers_init,
                variant_hooks: crate::shared::VariantHooks::default(),
            }
        }

        /// Creates a new API client with a pre-configured reqwest client.
        ///
        /// Use this when you need custom timeouts, connection pools, or middleware.
        ///
        /// ## Examples
        ///
        /// ```text
        /// let custom_client = reqwest::Client::builder()
        ///     .timeout(std::time::Duration::from_secs(60))
        ///     .build()
        ///     .unwrap();
        /// let api = Api::with_client(custom_client);
        /// ```
        pub fn with_client(client: reqwest::Client) -> Self {
            Self {
                client,
                base_url: Self::BASE_URL.to_string(),
                env_auth: vec![#(#env_auth.to_string()),*],
                auth_strategy: #auth_strategy_init,
                auth_policy: #auth_policy_init,
                env_username: #env_username_init,
                headers: #headers_init,
                variant_hooks: crate::shared::VariantHooks::default(),
            }
        }

        /// Creates a new API client with a pre-configured reqwest client and custom base URL.
        ///
        /// ## Examples
        ///
        /// ```text
        /// let custom_client = reqwest::Client::builder()
        ///     .timeout(std::time::Duration::from_secs(60))
        ///     .build()
        ///     .unwrap();
        /// let api = Api::with_client_and_base_url(custom_client, "http://localhost:8080");
        /// ```
        pub fn with_client_and_base_url(client: reqwest::Client, base_url: impl Into<String>) -> Self {
            Self {
                client,
                base_url: base_url.into(),
                env_auth: vec![#(#env_auth.to_string()),*],
                auth_strategy: #auth_strategy_init,
                auth_policy: #auth_policy_init,
                env_username: #env_username_init,
                headers: #headers_init,
                variant_hooks: crate::shared::VariantHooks::default(),
            }
        }
    }
}

/// Generates the `Default` trait implementation for the API struct.
pub(super) fn generate_default_impl(struct_name: &Ident) -> TokenStream {
    quote! {
        impl Default for #struct_name {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}
