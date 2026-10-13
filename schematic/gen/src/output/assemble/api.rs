//! API module assembly for generated code modules.
//!
//! An API module holds the API's client struct and tag type, one request
//! struct per endpoint with its `schematic_runtime::EndpointSpec`
//! implementation, one call type per endpoint with the endpoint method that
//! returns it, and the re-exported definition types. Its sends are the
//! runtime client's, forwarded by `schematic_runtime::api_client!` and by
//! the call types.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{Endpoint, RestApi};

use crate::codegen::api_struct::api_tag_name;
use crate::codegen::{
    ModuleDocBuilder, generate_api_struct, generate_endpoint_methods, generate_paginated_impl,
    generate_request_struct_with_options,
};

use super::helpers::{build_definitions_import, get_module_path, get_request_suffix};
use crate::output::options::OutputOptions;

/// Threshold (in lines of formatted code) above which an API module is split
/// into a directory with submodules instead of a single file.
pub const SPLIT_THRESHOLD_LINES: usize = 2000;

/// Parts produced when splitting a large API module into submodules.
pub struct SplitApiParts {
    /// Module docs, submodule declarations, re-exports, and each API's
    /// client struct and tag type.
    pub mod_rs: TokenStream,
    /// Request structs, their `EndpointSpec` and `Paginated` impls, form
    /// bodies, call types, and the endpoint methods.
    pub requests_rs: TokenStream,
    /// The re-exported definition types.
    pub responses_rs: TokenStream,
}

/// The request structs, `Paginated` impls, call types, and endpoint methods
/// of one API.
///
/// An API-level `Cookie` header cannot be a client default header: the
/// runtime refuses a default header that can carry a credential. So the API's
/// `Cookie` headers are folded into every endpoint's static headers here,
/// ahead of the endpoint's own, where they merge with its cookies and cookie
/// parameters into one `Cookie` header. `generate_api_struct` leaves them out
/// of the default headers.
fn api_requests(api: &RestApi, module_path: &str) -> TokenStream {
    let suffix = get_request_suffix(api);
    let api_cookies: Vec<(String, String)> = api
        .headers
        .iter()
        .filter(|(name, _)| name.eq_ignore_ascii_case("cookie"))
        .cloned()
        .collect();

    let mut tokens = TokenStream::new();
    for endpoint in &api.endpoints {
        let endpoint = with_api_cookies(endpoint, &api_cookies);
        tokens.extend(generate_request_struct_with_options(
            &endpoint,
            &suffix,
            Some(module_path),
            &api_tag_name(api),
        ));
    }
    for endpoint in &api.endpoints {
        tokens.extend(generate_paginated_impl(endpoint, &suffix));
    }
    tokens.extend(generate_endpoint_methods(api));
    tokens
}

fn with_api_cookies(endpoint: &Endpoint, api_cookies: &[(String, String)]) -> Endpoint {
    let mut endpoint = endpoint.clone();
    if !api_cookies.is_empty() {
        let mut headers = api_cookies.to_vec();
        headers.append(&mut endpoint.headers);
        endpoint.headers = headers;
    }
    endpoint
}

fn has_paginated(apis: &[&RestApi]) -> bool {
    apis.iter().any(|api| {
        api.endpoints
            .iter()
            .any(|ep| ep.params.as_ref().is_some_and(|p| p.pagination.is_some()))
    })
}

fn shared_import(apis: &[&RestApi]) -> TokenStream {
    if has_paginated(apis) {
        quote! { use crate::shared::Paginated; }
    } else {
        TokenStream::new()
    }
}

/// Assembles an API module with configurable options.
pub fn assemble_api_module_with_options(api: &RestApi, options: &OutputOptions) -> TokenStream {
    let module_path = get_module_path(api);
    let requests = api_requests(api, &module_path);
    let api_struct = generate_api_struct(api);
    let module_docs = ModuleDocBuilder::new(api).build();
    let shared_import = shared_import(&[api]);

    let definitions_import = if options.standalone {
        quote! {
            // Re-export model types from local types module
            pub use crate::types::*;
        }
    } else {
        build_definitions_import(&module_path)
    };

    quote! {
        #module_docs

        use serde::{Deserialize, Serialize};

        #definitions_import

        #shared_import

        #api_struct

        #requests
    }
}

/// Assembles the API module code (e.g., anthropic.rs): module documentation,
/// the re-exported definition types, the API's client struct and tag type,
/// and its request structs.
pub fn assemble_api_module(api: &RestApi) -> TokenStream {
    assemble_api_module_with_options(api, &OutputOptions::default())
}

/// Assembles a combined API module for multiple APIs sharing the same module path.
///
/// When two or more APIs share a `module_path` (e.g., `OllamaNative` and `OllamaOpenAI`
/// both use `"ollama"`), this function generates a single `.rs` file containing all
/// of their types.
///
/// ## Panics
///
/// Panics if `apis` is empty.
pub fn assemble_combined_api_module(apis: &[&RestApi]) -> TokenStream {
    assert!(
        !apis.is_empty(),
        "assemble_combined_api_module requires at least one API"
    );

    let module_path = get_module_path(apis[0]);
    let combined_docs = build_combined_module_docs(apis);
    let definitions_import = build_definitions_import(&module_path);
    let shared_import = shared_import(apis);

    let per_api_code = apis.iter().map(|api| {
        let api_struct = generate_api_struct(api);
        let requests = api_requests(api, &module_path);
        quote! {
            #api_struct

            #requests
        }
    });

    quote! {
        #combined_docs

        use serde::{Deserialize, Serialize};

        #definitions_import

        #shared_import

        #(#per_api_code)*
    }
}

/// Builds combined module documentation for multiple APIs sharing a module.
fn build_combined_module_docs(apis: &[&RestApi]) -> TokenStream {
    let mut doc_parts: Vec<String> = Vec::new();

    for (i, api) in apis.iter().enumerate() {
        if i > 0 {
            doc_parts.push("---".to_string());
            doc_parts.push(String::new());
        }

        let name = &api.name;
        let desc = &api.description;
        let intro = if let Some(docs_url) = &api.docs_url {
            format!(
                "Generated API client for [{}]({}).\n\n{}",
                name, docs_url, desc
            )
        } else {
            format!("Generated API client for {}.\n\n{}", name, desc)
        };

        doc_parts.push(intro);
    }

    let full_doc = doc_parts.join("\n\n");
    crate::codegen::lines_to_doc_comments(&full_doc)
}

/// Assembles a single API module split into submodules for large APIs.
///
/// Produces three TokenStreams intended for:
/// - `mod.rs` - module docs, submodule declarations, re-exports, and the
///   API's client struct and tag type
/// - `requests.rs` - all request structs with their `EndpointSpec` and
///   `Paginated` impls, the call types, and the endpoint methods
/// - `responses.rs` - definitions re-exports
pub fn assemble_split_api_module(api: &RestApi, options: &OutputOptions) -> SplitApiParts {
    let module_path = get_module_path(api);
    let module_docs = ModuleDocBuilder::new(api).build();

    let definitions_import = if options.standalone {
        quote! {
            pub use crate::types::*;
        }
    } else {
        build_definitions_import(&module_path)
    };

    split_parts(&[api], module_docs, definitions_import, &module_path)
}

/// Assembles a combined API module split into submodules for large APIs.
///
/// This is the split version of [`assemble_combined_api_module`] for when
/// multiple APIs share the same module path and the combined output exceeds
/// the threshold.
///
/// ## Panics
///
/// Panics if `apis` is empty.
pub fn assemble_split_combined_api_module(apis: &[&RestApi]) -> SplitApiParts {
    assert!(
        !apis.is_empty(),
        "assemble_split_combined_api_module requires at least one API"
    );

    let module_path = get_module_path(apis[0]);
    let combined_docs = build_combined_module_docs(apis);
    let definitions_import = build_definitions_import(&module_path);
    split_parts(apis, combined_docs, definitions_import, &module_path)
}

fn split_parts(
    apis: &[&RestApi],
    docs: TokenStream,
    definitions_import: TokenStream,
    module_path: &str,
) -> SplitApiParts {
    let shared_import = shared_import(apis);
    // The endpoint methods in `requests.rs` extend the API structs.
    let api_items: Vec<_> = apis
        .iter()
        .flat_map(|api| [format_ident!("{}", api.name), format_ident!("{}", api_tag_name(api))])
        .collect();
    let api_structs = apis.iter().map(|api| generate_api_struct(api));
    let requests = apis.iter().map(|api| api_requests(api, module_path));

    let mod_rs = quote! {
        #docs

        pub mod requests;
        pub mod responses;

        pub use requests::*;
        pub use responses::*;

        #(#api_structs)*
    };

    let requests_rs = quote! {
        use serde::{Deserialize, Serialize};

        #shared_import

        use super::responses::*;
        use super::{#(#api_items),*};

        #(#requests)*
    };

    SplitApiParts {
        mod_rs,
        requests_rs,
        responses_rs: definitions_import,
    }
}

#[doc(hidden)]
pub fn assemble_api_code(api: &RestApi) -> TokenStream {
    assemble_api_module(api)
}
