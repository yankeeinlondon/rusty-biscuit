//! Per-API client generation for REST APIs.
//!
//! Each API gets an uninhabited marker type named after it, which carries the
//! API's constants and constructors, and a `{Name}Client` alias for the
//! runtime client bound to that marker:
//!
//! ```text
//! pub enum OpenAI {}
//! pub type OpenAIClient = schematic_runtime::Client<OpenAI>;
//!
//! impl schematic_runtime::Api for OpenAI {
//!     const NAME: &'static str = "OpenAI";
//! }
//!
//! impl OpenAI {
//!     pub const BASE_URL: &'static str = "https://api.openai.com/v1";
//!     pub const DOCS_URL: Option<&'static str> = Some("https://platform.openai.com/docs");
//!     pub fn definition() -> schematic_runtime::ApiDefinition { ... }
//!     pub fn builder() -> schematic_runtime::ClientBuilder<Self> { ... }
//!     pub fn new() -> Result<OpenAIClient, schematic_runtime::BuildError> { ... }
//! }
//! ```
//!
//! Sending, hooks, and connection settings belong to the runtime client and
//! its builder; nothing here forwards to them. Because every request type
//! names its API marker, the runtime client accepts only that API's requests.

mod auth;
mod helpers;
#[cfg(test)]
mod tests;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::RestApi;

/// The variables the generated client reads when no credential is supplied:
/// `(token, key, or password variables, Basic username variables)`, or
/// `None` when it reads none.
pub(crate) fn env_fallback_vars(api: &RestApi) -> Option<(Vec<String>, Vec<String>)> {
    auth::auth_config(api).env.map(|env| (env.vars, env.username_vars))
}

/// Generates the marker type, client alias, and constructors of one API.
pub fn generate_api_struct(api: &RestApi) -> TokenStream {
    let marker = format_ident!("{}", api.name);
    let client = format_ident!("{}Client", api.name);
    let name = &api.name;
    let base_url = &api.base_url;
    let docs_url = match &api.docs_url {
        Some(url) => quote! { Some(#url) },
        None => quote! { None },
    };
    let definition = helpers::generate_definition(api);
    // An API-level `Cookie` travels in each request's own headers instead;
    // see `crate::output::assemble::api`.
    let default_headers = api
        .headers
        .iter()
        .filter(|(header, _)| !header.eq_ignore_ascii_case("cookie"))
        .map(|(header, value)| quote! { .default_header(#header, #value) });

    let marker_doc = format!(" The {name} API: {}", api.description.trim());
    let client_doc = format!(" A client for the [`{name}`] API.");

    quote! {
        #[doc = #marker_doc]
        ///
        /// This type is never constructed. It names the API, so that its
        /// client sends only this API's requests, and it holds the API's
        /// constants and constructors.
        pub enum #marker {}

        #[doc = #client_doc]
        pub type #client = schematic_runtime::Client<#marker>;

        impl schematic_runtime::Api for #marker {
            const NAME: &'static str = #name;
        }

        impl #marker {
            /// The base URL a client uses unless its builder sets another.
            pub const BASE_URL: &'static str = #base_url;
            /// The API's documentation, when the definition names it.
            pub const DOCS_URL: Option<&'static str> = #docs_url;

            /// What the API tells the runtime: its base URL, how it accepts
            /// credentials, its cross-origin policy, its secret names, and its
            /// request-id header.
            pub fn definition() -> schematic_runtime::ApiDefinition {
                #definition
            }

            /// A client builder seeded from [`definition`](Self::definition)
            /// and the API's default headers.
            pub fn builder() -> schematic_runtime::ClientBuilder<Self> {
                schematic_runtime::Client::builder(Self::definition())
                    #(#default_headers)*
            }

            /// A client with every default.
            ///
            /// ## Errors
            ///
            /// Returns [`schematic_runtime::BuildError`] when the HTTP client
            /// cannot be initialized.
            pub fn new() -> Result<#client, schematic_runtime::BuildError> {
                Self::builder().build()
            }
        }
    }
}
