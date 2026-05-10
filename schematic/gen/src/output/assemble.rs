//! Token assembly for generated code modules.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{AuthStrategy, RestApi, RestMethod};

use crate::codegen::{
    ModuleDocBuilder, generate_api_struct, generate_error_type, generate_paginated_impl,
    generate_paginated_trait, generate_request_enum_with_suffix,
    generate_request_method_with_suffix, generate_request_parts_type,
    generate_request_struct_with_options, generate_variant_types, lines_to_doc_comments,
};

use super::options::OutputOptions;
use super::ws_modules::ws_definition_modules;

/// Converts a PascalCase string to snake_case.
fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}

/// Returns the module path for the given API.
///
/// Uses `api.module_path` if set, otherwise attempts to infer from the API name.
/// Falls back to `api.name.to_lowercase()` if inference returns None.
///
/// ## Resolution Order
///
/// 1. Explicit `module_path` (highest priority)
/// 2. Inferred from CamelCase name (e.g., "OllamaNative" -> "ollama")
/// 3. Lowercase API name (fallback)
pub fn get_module_path(api: &RestApi) -> String {
    api.module_path.clone().unwrap_or_else(|| {
        crate::inference::infer_module_path(&api.name).unwrap_or_else(|| api.name.to_lowercase())
    })
}

/// Returns the request suffix for the given API.
///
/// Uses `api.request_suffix` if set, otherwise defaults to `"Request"`.
pub fn get_request_suffix(api: &RestApi) -> String {
    api.request_suffix
        .clone()
        .unwrap_or_else(|| "Request".to_string())
}

/// Assembles the shared module code (shared.rs).
///
/// This function generates code for the shared module, containing:
/// - Module documentation
/// - Common error type used by all API clients
/// - Common type aliases (e.g., `RequestParts`)
/// - Re-export of reqwest for downstream crates
/// - `Paginated` marker trait for paginated request types
///
/// ## Returns
///
/// A TokenStream containing the shared module code.
pub fn assemble_shared_module() -> TokenStream {
    let request_parts_type = generate_request_parts_type();
    let error_type = generate_error_type();
    let variant_types = generate_variant_types();
    let paginated_trait = generate_paginated_trait();

    quote! {
        //! Shared types and utilities for generated API clients.

        /// Re-export reqwest for downstream crates that need to make custom requests.
        ///
        /// This allows consumers to use the same HTTP client types without adding
        /// reqwest as a direct dependency.
        pub use reqwest;

        #request_parts_type

        #error_type

        #variant_types

        #paginated_trait
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
        let definitions_module = format_ident!("{}", api_name_lower);
        quote! {
            // Re-export response types from definitions so consumers can import from one place
            pub use schematic_definitions::#definitions_module::*;
        }
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

    let definitions_module = format_ident!("{}", module_path);
    let definitions_import = quote! {
        // Re-export response types from definitions so consumers can import from one place
        pub use schematic_definitions::#definitions_module::*;
    };

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
    lines_to_doc_comments(&full_doc)
}

/// Assembles the lib.rs content for the schema crate.
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to include
///
/// ## Returns
///
/// A TokenStream containing the lib.rs code.
pub fn assemble_lib_rs(apis: &[&RestApi]) -> TokenStream {
    let mut seen_modules = HashSet::new();
    let mut module_decls: Vec<_> = apis
        .iter()
        .filter_map(|api| {
            let path = get_module_path(api);
            if seen_modules.insert(path.clone()) {
                let module_name = format_ident!("{}", path);
                Some(quote! {
                    pub mod #module_name;
                })
            } else {
                None
            }
        })
        .collect();
    if seen_modules.insert("ws_shared".to_string()) {
        module_decls.push(quote! {
            pub mod ws_shared;
        });
    }
    for ws in ws_definition_modules() {
        if seen_modules.insert(ws.module.to_string()) {
            let module_name = format_ident!("{}", ws.module);
            module_decls.push(quote! {
                pub mod #module_name;
            });
        }
    }

    let api_rows: Vec<String> = apis
        .iter()
        .map(|api| {
            let module = get_module_path(api);
            let name = &api.name;
            let desc = &api.description;
            let auth_desc = match &api.auth {
                AuthStrategy::None => "None".to_string(),
                AuthStrategy::BearerToken { .. } => "Bearer".to_string(),
                AuthStrategy::ApiKey { header } => format!("API Key (`{}`)", header),
                AuthStrategy::Basic => "Basic".to_string(),
                _ => "Custom".to_string(),
            };
            format!("//! | [`{module}`] | [`{name}`]({module}::{name}) | {desc} | {auth_desc} |")
        })
        .collect();
    let api_table = api_rows.join("\n");
    let ws_rows: Vec<String> = ws_definition_modules()
        .iter()
        .map(|ws| {
            format!(
                "//! | [`{}`] | [`{}`]({}::define_api) | {} |",
                ws.module, ws.display_name, ws.module, ws.description
            )
        })
        .collect();
    let ws_table = ws_rows.join("\n");

    let example_api = apis
        .iter()
        .find(|api| api.endpoints.iter().any(|ep| ep.method == RestMethod::Get))
        .or_else(|| apis.first());

    let quick_start_example = if let Some(api) = example_api {
        let api_name = &api.name;
        let first_get = api.endpoints.iter().find(|ep| ep.method == RestMethod::Get);
        if let Some(ep) = first_get {
            let method_name = to_snake_case(&ep.id);
            let has_path_params = ep.path.contains('{');
            let call = if has_path_params {
                format!("let response = client.{}(\"example\").await?;", method_name)
            } else {
                format!("let response = client.{}().await?;", method_name)
            };
            format!(
                r#"//! ```text
//! use schematic_schema::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), SchematicError> {{
//!     let client = {api_name}::new();
//!     {call}
//!     println!("{{:?}}", response);
//!     Ok(())
//! }}
//! ```"#
            )
        } else {
            String::from(
                r#"//! ```text
//! use schematic_schema::prelude::*;
//! ```"#,
            )
        }
    } else {
        String::from(
            r#"//! ```text
//! use schematic_schema::prelude::*;
//! ```"#,
        )
    };

    let variant_example = if let Some(api) = example_api {
        let api_name = &api.name;
        let env_var = api
            .env_auth
            .first()
            .map(|s| s.as_str())
            .unwrap_or("API_KEY");
        let staging_var = format!("STAGING_{}", env_var);
        format!(
            r#"//! ## Variants
//!
//! Create alternate client configurations for staging, testing, or different
//! environments using the [`variant()`]({module}::{api_name}::variant) builder
//! or [`variant_with()`]({module}::{api_name}::variant_with) convenience method:
//!
//! ```text
//! use schematic_schema::prelude::*;
//! use schematic_define::UpdateStrategy;
//!
//! let client = {api_name}::new();
//!
//! // Simple environment switch with variant_with()
//! let staging = client.variant_with(
//!     "https://staging.example.com/v1",
//!     vec!["{staging_var}".to_string()],
//!     UpdateStrategy::NoChange,
//! );
//!
//! // Full builder with response hooks
//! let custom = client.variant()
//!     .base_url("http://localhost:8080/v1")
//!     .env_auth(vec!["LOCAL_KEY".to_string()])
//!     .auth_update(UpdateStrategy::ChangeTo(schematic_define::AuthStrategy::ApiKey {{
//!         header: "X-API-Key".to_string(),
//!     }}))
//!     .pre_response_json(|_ctx, json| Ok(json))
//!     .build();
//! ```"#,
            module = get_module_path(api),
        )
    } else {
        String::new()
    };

    let api_table_tokens: TokenStream = api_table.parse().unwrap_or_default();
    let ws_table_tokens: TokenStream = ws_table.parse().unwrap_or_default();
    let quick_start_tokens: TokenStream = quick_start_example.parse().unwrap_or_default();
    let variant_tokens: TokenStream = variant_example.parse().unwrap_or_default();

    quote! {
        //! Generated REST API clients and WebSocket API definition helpers.
        //!
        //! Each API is available as a separate module with its client struct,
        //! request types, and response types re-exported from definitions.
        //!
        //! ## Available REST APIs
        //!
        //! | Module | Client | Description | Auth |
        //! |--------|--------|-------------|------|
        #api_table_tokens
        //!
        //! ## Available WebSocket Definitions
        //!
        //! | Module | API | Description |
        //! |--------|-----|-------------|
        #ws_table_tokens
        //!
        //! ## Quick Start
        //!
        //! Use the prelude for convenient imports:
        //!
        #quick_start_tokens
        //!
        //! ## Ergonomic Features
        //!
        //! Generated clients include several convenience features:
        //!
        //! - **`From<&str>` / `From<String>`** on single-param request structs
        //! - **`From<BodyType>`** on body-only request structs
        //! - **`#[must_use]`** on all async methods to catch missing `.await`
        //! - **`DOCS_URL`** constant on each client for programmatic doc access
        //! - **`variant()`** method for creating alternate configurations
        //!
        #variant_tokens
        //!
        //! ## Error Handling
        //!
        //! All request methods return `Result<T, SchematicError>`. See
        //! [`shared::SchematicError`] for the full error enum and handling examples.

        // Shared types and utilities
        pub mod shared;

        pub mod prelude;

        #(#module_decls)*
    }
}

/// Assembles the prelude.rs content for the schema crate.
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to include
///
/// ## Returns
///
/// A TokenStream containing the prelude.rs code.
pub fn assemble_prelude(apis: &[&RestApi]) -> TokenStream {
    assemble_prelude_with_options(apis, true)
}

/// Assembles prelude.rs with optional WebSocket definition helper exports.
pub fn assemble_prelude_with_options(apis: &[&RestApi], include_ws_helpers: bool) -> TokenStream {
    let api_reexports: Vec<_> = apis
        .iter()
        .map(|api| {
            let module_name = format_ident!("{}", get_module_path(api));
            let client_name = format_ident!("{}", api.name);
            let request_enum = format_ident!("{}Request", api.name);

            quote! {
                pub use crate::#module_name::{#client_name, #request_enum};
            }
        })
        .collect();
    let ws_reexports: Vec<_> = if include_ws_helpers {
        ws_definition_modules()
            .iter()
            .map(|ws| {
                let module_name = format_ident!("{}", ws.module);
                let helper_name = format_ident!("{}", ws.helper);
                quote! {
                    pub use crate::#module_name::define_api as #helper_name;
                }
            })
            .collect()
    } else {
        Vec::new()
    };

    let client_list: Vec<String> = apis
        .iter()
        .map(|api| format!("//! - [`{name}`] + [`{name}Request`]", name = api.name))
        .collect();
    let client_list_str = client_list.join("\n");
    let client_list_tokens: TokenStream = client_list_str.parse().unwrap_or_default();
    let ws_helper_list_tokens: TokenStream = if include_ws_helpers {
        ws_definition_modules()
            .iter()
            .map(|ws| format!("//! - [`{}`] ({})", ws.helper, ws.display_name))
            .collect::<Vec<_>>()
            .join("\n")
            .parse()
            .unwrap_or_default()
    } else {
        TokenStream::new()
    };

    quote! {
        //! Convenient re-exports for working with generated API clients.
        //!
        //! This prelude exports the client structs, request enums, and shared error
        //! types. Response types are **not** re-exported here to avoid naming conflicts.
        //! Import them from specific API modules instead:
        //!
        //! ```text
        //! use schematic_schema::openai::Model;
        //! use schematic_schema::anthropic::CreateMessageResponse;
        //! ```
        //!
        //! ## Re-exports
        //!
        //! **Clients and request enums:**
        //!
        #client_list_tokens
        //!
        //! **WebSocket definition helpers:**
        //!
        #ws_helper_list_tokens
        //!
        //! **Shared types:**
        //!
        //! - [`SchematicError`] - Error type for all API operations
        //! - [`RequestParts`] - Low-level request decomposition tuple
        //!
        //! ## Examples
        //!
        //! ```text
        //! use schematic_schema::prelude::*;
        //!
        //! #[tokio::main]
        //! async fn main() -> Result<(), SchematicError> {
        //!     let client = OpenAI::new();
        //!
        //!     // Typed request with new()
        //!     let models = client.list_models().await?;
        //!
        //!     // Ergonomic From<&str> for single-param endpoints
        //!     use schematic_schema::openai::{RetrieveModelRequest, Model};
        //!     let model: Model = client
        //!         .request(RetrieveModelRequest::from("gpt-4"))
        //!         .await?;
        //!
        //!     Ok(())
        //! }
        //! ```

        // Shared types
        pub use crate::shared::{RequestParts, SchematicError};

        // API clients and request types
        #(#api_reexports)*

        // WebSocket definition helpers
        #(#ws_reexports)*
    }
}

#[doc(hidden)]
pub fn assemble_api_code(api: &RestApi) -> TokenStream {
    assemble_api_module(api)
}

/// Assembles lib.rs content with configurable options.
///
/// This is similar to `assemble_lib_rs` but supports options like including
/// a types module for imported APIs.
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to include
/// * `options` - Output generation options
///
/// ## Returns
///
/// A TokenStream containing the lib.rs code.
pub fn assemble_lib_rs_with_options(apis: &[&RestApi], options: &OutputOptions) -> TokenStream {
    let mut seen_modules = HashSet::new();
    let module_decls: Vec<_> = apis
        .iter()
        .filter_map(|api| {
            let path = get_module_path(api);
            if seen_modules.insert(path.clone()) {
                let module_name = format_ident!("{}", path);
                Some(quote! {
                    pub mod #module_name;
                })
            } else {
                None
            }
        })
        .collect();

    let types_module = if options.include_types_module {
        quote! {
            pub mod types;
        }
    } else {
        TokenStream::new()
    };

    let api_rows: Vec<String> = apis
        .iter()
        .map(|api| {
            let module = get_module_path(api);
            let name = &api.name;
            let desc = &api.description;
            let auth_desc = match &api.auth {
                AuthStrategy::None => "None".to_string(),
                AuthStrategy::BearerToken { .. } => "Bearer".to_string(),
                AuthStrategy::ApiKey { header } => format!("API Key (`{}`)", header),
                AuthStrategy::Basic => "Basic".to_string(),
                _ => "Custom".to_string(),
            };
            format!("//! | [`{module}`] | [`{name}`]({module}::{name}) | {desc} | {auth_desc} |")
        })
        .collect();
    let api_table = api_rows.join("\n");

    let example_api = apis
        .iter()
        .find(|api| api.endpoints.iter().any(|ep| ep.method == RestMethod::Get))
        .or_else(|| apis.first());

    let quick_start_example = if let Some(api) = example_api {
        let api_name = &api.name;
        let first_get = api.endpoints.iter().find(|ep| ep.method == RestMethod::Get);
        if let Some(ep) = first_get {
            let method_name = to_snake_case(&ep.id);
            let has_path_params = ep.path.contains('{');
            let call = if has_path_params {
                format!("let response = client.{}(\"example\").await?;", method_name)
            } else {
                format!("let response = client.{}().await?;", method_name)
            };
            format!(
                r#"//! ```text
//! use schematic_schema::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), SchematicError> {{
//!     let client = {api_name}::new();
//!     {call}
//!     println!("{{:?}}", response);
//!     Ok(())
//! }}
//! ```"#
            )
        } else {
            String::from(
                r#"//! ```text
//! use schematic_schema::prelude::*;
//! ```"#,
            )
        }
    } else {
        String::from(
            r#"//! ```text
//! use schematic_schema::prelude::*;
//! ```"#,
        )
    };

    let api_table_tokens: TokenStream = api_table.parse().unwrap_or_default();
    let quick_start_tokens: TokenStream = quick_start_example.parse().unwrap_or_default();

    quote! {
        //! Generated REST API clients.
        //!
        //! Each API is available as a separate module with its client struct,
        //! request types, and response types.
        //!
        //! ## Available APIs
        //!
        //! | Module | Client | Description | Auth |
        //! |--------|--------|-------------|------|
        #api_table_tokens
        //!
        //! ## Quick Start
        //!
        //! Use the prelude for convenient imports:
        //!
        #quick_start_tokens
        //!
        //! ## Error Handling
        //!
        //! All request methods return `Result<T, SchematicError>`. See
        //! [`shared::SchematicError`] for the full error enum and handling examples.

        // Shared types and utilities
        pub mod shared;

        pub mod prelude;

        #types_module

        #(#module_decls)*
    }
}
