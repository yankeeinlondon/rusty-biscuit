//! API struct and variant assembly for generated code modules.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::RestApi;

use crate::codegen::{
    ModuleDocBuilder, generate_api_struct, generate_paginated_impl,
    generate_request_enum_with_suffix, generate_request_method_with_suffix,
    generate_request_struct_with_options,
};

use super::helpers::{build_definitions_import, get_module_path, get_request_suffix};
use crate::output::options::OutputOptions;

/// Threshold (in lines of formatted code) above which an API module is split
/// into a directory with submodules instead of a single file.
pub const SPLIT_THRESHOLD_LINES: usize = 2000;

/// Parts produced when splitting a large API module into submodules.
pub struct SplitApiParts {
    pub mod_rs: TokenStream,
    pub requests_rs: TokenStream,
    pub responses_rs: TokenStream,
    pub client_rs: TokenStream,
}

/// Assembles an API module with configurable options.
///
/// ## Arguments
///
/// * `api` - The API definition to generate code for
/// * `options` - Output generation options
///
/// ## Returns
///
/// A TokenStream containing the complete API module code.
pub fn assemble_api_module_with_options(api: &RestApi, options: &OutputOptions) -> TokenStream {
    let api_name_lower = get_module_path(api);
    let suffix = get_request_suffix(api);

    let request_structs: TokenStream = api
        .endpoints
        .iter()
        .map(|ep| generate_request_struct_with_options(ep, &suffix, Some(&api_name_lower)))
        .collect();

    let paginated_impls: TokenStream = api
        .endpoints
        .iter()
        .map(|ep| generate_paginated_impl(ep, &suffix))
        .collect();

    let request_enum = generate_request_enum_with_suffix(api, &suffix);
    let api_struct = generate_api_struct(api);
    let request_method = generate_request_method_with_suffix(api, &suffix);
    let module_docs = ModuleDocBuilder::new(api).build();

    let definitions_import = if options.standalone {
        quote! {
            // Re-export model types from local types module
            pub use crate::types::*;
        }
    } else {
        build_definitions_import(&api_name_lower)
    };

    let has_paginated = api
        .endpoints
        .iter()
        .any(|ep| ep.params.as_ref().is_some_and(|p| p.pagination.is_some()));

    let shared_import = if has_paginated {
        quote! { use crate::shared::{Paginated, RequestParts, SchematicError}; }
    } else {
        quote! { use crate::shared::{RequestParts, SchematicError}; }
    };

    quote! {
        #module_docs

        use serde::{Deserialize, Serialize};

        #definitions_import

        // Import shared types
        #shared_import

        #request_structs

        #paginated_impls

        #request_enum

        #api_struct

        #request_method
    }
}

/// Assembles the API module code (e.g., openai.rs).
///
/// This function generates code for a single API module, containing:
/// - Rich module documentation (intro, auth, features, example)
/// - Import statements
/// - Re-exports from definitions
/// - Per-endpoint request structs
/// - Request enum (unifying all endpoints)
/// - API client struct
/// - Request method implementation
///
/// The error type (`SchematicError`) is imported from the shared module
/// rather than being duplicated in each API module.
///
/// ## Arguments
///
/// * `api` - The REST API definition to generate code for
///
/// ## Returns
///
/// A TokenStream containing the API module code.
pub fn assemble_api_module(api: &RestApi) -> TokenStream {
    assemble_api_module_with_options(api, &OutputOptions::default())
}

/// Assembles a combined API module for multiple APIs sharing the same module path.
///
/// When two or more APIs share a `module_path` (e.g., `OllamaNative` and `OllamaOpenAI`
/// both use `"ollama"`), this function generates a single `.rs` file containing all
/// of their types.
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions sharing the same module path
///
/// ## Returns
///
/// A TokenStream containing the combined module code.
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

    let per_api_code: Vec<TokenStream> = apis
        .iter()
        .map(|api| {
            let suffix = get_request_suffix(api);

            let request_structs: TokenStream = api
                .endpoints
                .iter()
                .map(|ep| generate_request_struct_with_options(ep, &suffix, Some(&module_path)))
                .collect();

            let paginated_impls: TokenStream = api
                .endpoints
                .iter()
                .map(|ep| generate_paginated_impl(ep, &suffix))
                .collect();

            let request_enum = generate_request_enum_with_suffix(api, &suffix);
            let api_struct = generate_api_struct(api);
            let request_method = generate_request_method_with_suffix(api, &suffix);

            quote! {
                #request_structs

                #paginated_impls

                #request_enum

                #api_struct

                #request_method
            }
        })
        .collect();

    let has_paginated = apis.iter().any(|api| {
        api.endpoints
            .iter()
            .any(|ep| ep.params.as_ref().is_some_and(|p| p.pagination.is_some()))
    });

    let shared_import = if has_paginated {
        quote! { use crate::shared::{Paginated, RequestParts, SchematicError}; }
    } else {
        quote! { use crate::shared::{RequestParts, SchematicError}; }
    };

    quote! {
        #combined_docs

        use serde::{Deserialize, Serialize};

        #definitions_import

        // Import shared types
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
/// Produces four TokenStreams intended for:
/// - `mod.rs` - module docs, submodule declarations, re-exports, client struct + constructor impl
/// - `requests.rs` - all request structs, EndpointSpec impls, paginated impls, request enums
/// - `responses.rs` - definitions re-exports
/// - `client.rs` - HTTP method impl blocks
///
/// ## Arguments
///
/// * `api` - The API definition to generate code for
/// * `options` - Output generation options
pub fn assemble_split_api_module(api: &RestApi, options: &OutputOptions) -> SplitApiParts {
    let api_name_lower = get_module_path(api);
    let suffix = get_request_suffix(api);

    let request_structs: TokenStream = api
        .endpoints
        .iter()
        .map(|ep| generate_request_struct_with_options(ep, &suffix, Some(&api_name_lower)))
        .collect();

    let paginated_impls: TokenStream = api
        .endpoints
        .iter()
        .map(|ep| generate_paginated_impl(ep, &suffix))
        .collect();

    let request_enum = generate_request_enum_with_suffix(api, &suffix);
    let api_struct = generate_api_struct(api);
    let request_method = generate_request_method_with_suffix(api, &suffix);
    let module_docs = ModuleDocBuilder::new(api).build();

    let definitions_import = if options.standalone {
        quote! {
            pub use crate::types::*;
        }
    } else {
        build_definitions_import(&api_name_lower)
    };

    let has_paginated = api
        .endpoints
        .iter()
        .any(|ep| ep.params.as_ref().is_some_and(|p| p.pagination.is_some()));

    let shared_import = if has_paginated {
        quote! { use crate::shared::{Paginated, RequestParts, SchematicError}; }
    } else {
        quote! { use crate::shared::{RequestParts, SchematicError}; }
    };

    let mod_rs = quote! {
        #module_docs

        pub mod requests;
        pub mod responses;
        pub mod client;

        pub use requests::*;
        pub use responses::*;

        #api_struct
    };

    let requests_rs = quote! {
        use serde::{Deserialize, Serialize};

        #shared_import

        use super::responses::*;

        #request_structs

        #paginated_impls

        #request_enum
    };

    let responses_rs = definitions_import;

    let client_rs = quote! {
        use super::*;
        use super::requests::*;
        use crate::shared::SchematicError;

        #request_method
    };

    SplitApiParts {
        mod_rs,
        requests_rs,
        responses_rs,
        client_rs,
    }
}

/// Assembles a combined API module split into submodules for large APIs.
///
/// This is the split version of [`assemble_combined_api_module`] for when
/// multiple APIs share the same module path and the combined output exceeds
/// the threshold.
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions sharing the same module path
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

    let has_paginated = apis.iter().any(|api| {
        api.endpoints
            .iter()
            .any(|ep| ep.params.as_ref().is_some_and(|p| p.pagination.is_some()))
    });

    let shared_import = if has_paginated {
        quote! { use crate::shared::{Paginated, RequestParts, SchematicError}; }
    } else {
        quote! { use crate::shared::{RequestParts, SchematicError}; }
    };

    let mut all_structs = TokenStream::new();
    let mut all_paginated = TokenStream::new();
    let mut all_enums = TokenStream::new();
    let mut all_structs_and_builders = TokenStream::new();
    let mut all_request_methods = TokenStream::new();

    for api in apis {
        let suffix = get_request_suffix(api);

        let request_structs: TokenStream = api
            .endpoints
            .iter()
            .map(|ep| generate_request_struct_with_options(ep, &suffix, Some(&module_path)))
            .collect();

        let paginated_impls: TokenStream = api
            .endpoints
            .iter()
            .map(|ep| generate_paginated_impl(ep, &suffix))
            .collect();

        let request_enum = generate_request_enum_with_suffix(api, &suffix);
        let api_struct = generate_api_struct(api);
        let request_method = generate_request_method_with_suffix(api, &suffix);

        all_structs.extend(request_structs);
        all_paginated.extend(paginated_impls);
        all_enums.extend(request_enum);
        all_structs_and_builders.extend(api_struct);
        all_request_methods.extend(request_method);
    }

    let mod_rs = quote! {
        #combined_docs

        pub mod requests;
        pub mod responses;
        pub mod client;

        pub use requests::*;
        pub use responses::*;

        #all_structs_and_builders
    };

    let requests_rs = quote! {
        use serde::{Deserialize, Serialize};

        #shared_import

        use super::responses::*;

        #all_structs

        #all_paginated

        #all_enums
    };

    let responses_rs = definitions_import;

    let client_rs = quote! {
        use super::*;
        use super::requests::*;
        use crate::shared::SchematicError;

        #all_request_methods
    };

    SplitApiParts {
        mod_rs,
        requests_rs,
        responses_rs,
        client_rs,
    }
}

#[doc(hidden)]
pub fn assemble_api_code(api: &RestApi) -> TokenStream {
    assemble_api_module(api)
}
