//! Per-API client generation for REST APIs.
//!
//! Each API gets one client struct named after it, the only name a caller
//! writes, and a tag type that its request types name as their
//! `EndpointSpec::Api`, so the client sends only that API's requests:
//!
//! ```text
//! /// The OpenAI API: ...
//! #[derive(Clone, Debug)]
//! pub struct OpenAI(schematic_runtime::Client<OpenAIApi>);
//!
//! pub enum OpenAIApi {}
//!
//! impl schematic_runtime::Api for OpenAIApi {
//!     const NAME: &'static str = "OpenAI";
//! }
//!
//! schematic_runtime::api_client!(OpenAI, OpenAIApi);
//!
//! impl OpenAI {
//!     pub const BASE_URL: &'static str = "https://api.openai.com/v1";
//!     pub const DOCS_URL: Option<&'static str> = Some("https://platform.openai.com/docs");
//!     pub fn definition() -> schematic_runtime::ApiDefinition { ... }
//!     pub fn new() -> Self { ... }
//!     // only when the API accepts HTTP Basic credentials:
//!     pub fn basic_auth(self, username: impl Into<String>, password: impl Into<String>) -> Self { ... }
//! }
//! ```
//!
//! The macro forwards every client-wide setting and send to the runtime
//! client the struct wraps, so that surface is written once, in the runtime.
//! Generation emits only what is specific to the API.

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

/// The name of `api`'s tag type, which its request types name as their
/// `EndpointSpec::Api`.
pub(crate) fn api_tag_name(api: &RestApi) -> String {
    format!("{}Api", api.name)
}

/// Generates the client struct, tag type, and API-specific items of one API.
pub fn generate_api_struct(api: &RestApi) -> TokenStream {
    let client = format_ident!("{}", api.name);
    let tag = format_ident!("{}", api_tag_name(api));
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
    let basic_auth = auth::auth_config(api)
        .accepted
        .contains(&auth::Placement::Basic)
        .then(|| {
            quote! {
                /// Sends HTTP Basic `username` and `password` when a call
                /// supplies no credential, replacing any credential set
                /// before.
                #[must_use]
                pub fn basic_auth(self, username: impl Into<String>, password: impl Into<String>) -> Self {
                    self.credentials(schematic_runtime::Credential::basic(username, password))
                }
            }
        });

    let client_doc = format!(" The {name} API: {}", api.description.trim());

    let tag_doc = format!(
        " The tag of the [`{name}`] API: each of its request types names it as \
         `EndpointSpec::Api`, so that [`{name}`] sends only this API's requests. \
         Callers never write it."
    );

    quote! {
        #[doc = #client_doc]
        ///
        /// Create a client with [`new`](Self::new), which never fails, and
        /// chain settings onto it. Settings are validated by
        /// [`build`](Self::build), or else by the first send, which reports
        /// an invalid one as `TransportErrorKind::InvalidClient`.
        #[derive(Clone, Debug)]
        pub struct #client(schematic_runtime::Client<#tag>);

        #[doc = #tag_doc]
        pub enum #tag {}

        impl schematic_runtime::Api for #tag {
            const NAME: &'static str = #name;
        }

        schematic_runtime::api_client!(#client, #tag);

        impl #client {
            /// The base URL a client uses unless `base_url` sets another.
            pub const BASE_URL: &'static str = #base_url;
            /// The API's documentation, when the definition names it.
            pub const DOCS_URL: Option<&'static str> = #docs_url;

            /// What the API tells the runtime: its base URL, how it accepts
            /// credentials, its cross-origin policy, its secret names, and its
            /// request-id header.
            pub fn definition() -> schematic_runtime::ApiDefinition {
                #definition
            }

            /// A client with the API's defaults and default headers. With no
            /// credential set, a send reads the API's environment variables,
            /// and with none present it is sent anonymously.
            pub fn new() -> Self {
                Self(schematic_runtime::Client::new(Self::definition()) #(#default_headers)*)
            }

            #basic_auth
        }

        impl Default for #client {
            fn default() -> Self {
                Self::new()
            }
        }
    }
}
