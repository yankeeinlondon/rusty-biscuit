//! API struct field generation.

use proc_macro2::TokenStream;
use quote::quote;

/// Generates the struct field declarations for the API client struct.
///
/// All API client structs share the same field layout:
/// - `client` - reqwest HTTP client
/// - `base_url` - API base URL
/// - `env_auth` - environment variable names for credentials
/// - `auth_strategy` - authentication strategy enum
/// - `auth_policy` - effective authentication policy
/// - `env_username` - environment variable for Basic auth username
/// - `headers` - Headers builder with env mapping
/// - `variant_hooks` - variant-specific hooks for response customization
pub(super) fn generate_field_list() -> TokenStream {
    quote! {
        client: reqwest::Client,
        base_url: String,
        /// Environment variable names for authentication credentials.
        env_auth: Vec<String>,
        /// Authentication strategy for this API client.
        auth_strategy: schematic_define::AuthStrategy,
        /// Effective authentication policy for this API client.
        auth_policy: schematic_define::AuthPolicy,
        /// Environment variable for Basic auth username.
        env_username: Option<String>,
        /// Headers builder with environment variable support for credentials.
        headers: schematic_define::Headers,
        /// Variant-specific hooks for response customization.
        variant_hooks: crate::shared::VariantHooks,
    }
}
