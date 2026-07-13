//! API struct generation for REST APIs.
//!
//! Generates the main API struct that serves as the client entry point,
//! with constructors and the base URL constant.

mod auth;
mod constructors;
mod fields;
mod helpers;
mod request_method;
#[cfg(test)]
mod tests;
mod variant;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

pub(crate) use auth::{generate_auth_policy_init, generate_auth_strategy_init};
pub(crate) use helpers::{generate_explicit_auth_helpers, generate_headers_init_from_mapping};

/// Generates the API struct for the given API definition.
///
/// Creates a struct with:
/// - `BASE_URL` constant containing the API's base URL
/// - `DOCS_URL` constant containing the API's documentation URL (or `None`)
/// - `new()` constructor using the default base URL
/// - `with_base_url()` constructor for custom base URLs
/// - `with_client()` constructor for custom reqwest clients
/// - `with_client_and_base_url()` constructor for both custom client and URL
/// - `Default` trait implementation
///
/// ## Examples
///
/// For an API named "OpenAi" with base URL `https://api.openai.com/v1`:
/// ```text
/// // Generated code:
/// /// OpenAI API client.
/// pub struct OpenAi {
///     client: reqwest::Client,
///     base_url: String,
/// }
///
/// impl OpenAi {
///     pub const BASE_URL: &'static str = "https://api.openai.com/v1";
///
///     pub fn new() -> Self {
///         Self {
///             client: reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build().unwrap_or_else(|_| reqwest::Client::new()),
///             base_url: Self::BASE_URL.to_string(),
///         }
///     }
///
///     pub fn with_base_url(base_url: impl Into<String>) -> Self {
///         Self {
///             client: reqwest::Client::builder().timeout(std::time::Duration::from_secs(30)).build().unwrap_or_else(|_| reqwest::Client::new()),
///             base_url: base_url.into(),
///         }
///     }
/// }
///
/// impl Default for OpenAi {
///     fn default() -> Self {
///         Self::new()
///     }
/// }
/// ```
pub fn generate_api_struct(api: &RestApi) -> TokenStream {
    let struct_name = format_ident!("{}", api.name);
    let base_url = &api.base_url;
    // Leading space for proper /// formatting
    let description = format!(" {} client.", api.description);
    let env_auth = &api.env_auth;

    // Generate auth strategy initialization
    let auth_strategy_init = generate_auth_strategy_init(&api.auth);
    let auth_policy_init = generate_auth_policy_init(&api.effective_auth_policy());

    // Generate env_username initialization
    let env_username_init = match &api.env_username {
        Some(name) => quote! { Some(#name.to_string()) },
        None => quote! { None },
    };

    // Generate headers initialization from env_mapping
    let headers_init = generate_headers_init_from_mapping(api);

    // Generate DOCS_URL constant
    let docs_url_init = match &api.docs_url {
        Some(url) => quote! { Some(#url) },
        None => quote! { None },
    };

    // Generate builder struct name
    let builder_name = format_ident!("{}VariantBuilder", api.name);
    let explicit_auth_helpers = generate_explicit_auth_helpers(api);

    // Generate struct fields
    let field_list = fields::generate_field_list();

    // Generate constructors and constants
    let constructors = constructors::generate_constructors(
        &struct_name,
        &constructors::ConstructorInit {
            base_url,
            env_auth,
            auth_strategy_init: &auth_strategy_init,
            auth_policy_init: &auth_policy_init,
            env_username_init: &env_username_init,
            headers_init: &headers_init,
            docs_url_init: &docs_url_init,
        },
    );

    // Generate accessor methods
    let accessors = request_method::generate_accessors(&struct_name);

    // Generate variant methods and builder
    let variant_methods = variant::generate_variant_methods(&builder_name);
    let variant_builder = variant::generate_variant_builder(&struct_name, &builder_name);

    // Generate Default impl
    let default_impl = constructors::generate_default_impl(&struct_name);

    quote! {
        #[doc = #description]
        pub struct #struct_name {
            #field_list
        }

        impl #struct_name {
            #constructors
            #accessors
            #variant_methods
            #explicit_auth_helpers
        }

        #default_impl

        #variant_builder
    }
}
