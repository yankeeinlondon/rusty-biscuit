//! Output assembly and file writing for generated code.
//!
//! This module handles the final phase of code generation: assembling all generated
//! pieces into a complete Rust file, validating the output, formatting it, and
//! writing it to disk atomically.
//!
//! ## Output Structure
//!
//! The generator produces per-API module files:
//! ```text
//! schema/src/
//! ├── lib.rs         # Module declarations and re-exports
//! ├── openai.rs      # OpenAI API client code
//! └── prelude.rs     # Common re-exports for consumers
//! ```
//!
//! ## Safety Guarantees
//!
//! - **Validation**: All generated code is validated with `syn` before writing
//! - **Formatting**: Output is formatted with `prettyplease` for consistent style
//! - **Atomic writes**: Uses temp file + rename pattern to prevent partial writes

use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::Path;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{AuthStrategy, RestApi, RestMethod};

use crate::codegen::{
    ModuleDocBuilder, generate_api_struct, generate_error_type, generate_paginated_impl,
    generate_paginated_trait, generate_request_enum_with_suffix,
    generate_request_method_with_suffix, generate_request_parts_type,
    generate_request_struct_with_options, generate_variant_types, lines_to_doc_comments,
};
use crate::errors::GeneratorError;
use crate::inference::infer_module_path;

/// Options for controlling code generation output.
#[derive(Debug, Clone, Default)]
pub struct OutputOptions {
    /// If true, don't include `pub use schematic_definitions::...` in generated modules.
    ///
    /// This is used for imported APIs where types are generated locally in `types.rs`
    /// instead of being imported from `schematic-definitions`.
    pub standalone: bool,
    /// If true, include a `pub mod types;` declaration in lib.rs.
    ///
    /// This is used when types are generated from imported OpenAPI specs.
    pub include_types_module: bool,
}

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

/// Metadata for generated WebSocket definition helper modules.
struct WsDefinitionModule {
    module: &'static str,
    helper: &'static str,
    display_name: &'static str,
    description: &'static str,
}

const WS_DEFINITION_MODULES: &[WsDefinitionModule] = &[
    WsDefinitionModule {
        module: "elevenlabs_ws",
        helper: "define_elevenlabs_ws_api_definition",
        display_name: "ElevenLabsTTS",
        description: "ElevenLabs Text-to-Speech WebSocket API definition",
    },
    WsDefinitionModule {
        module: "unfolded_circle_core_ws",
        helper: "define_unfolded_circle_core_ws_api_definition",
        display_name: "UnfoldedCircleCoreWs",
        description: "Unfolded Circle Core WebSocket API definition",
    },
    WsDefinitionModule {
        module: "unfolded_circle_dock_ws",
        helper: "define_unfolded_circle_dock_ws_api_definition",
        display_name: "UnfoldedCircleDockWs",
        description: "Unfolded Circle Dock WebSocket API definition",
    },
    WsDefinitionModule {
        module: "unfolded_circle_integration_ws",
        helper: "define_unfolded_circle_integration_ws_api_definition",
        display_name: "UnfoldedCircleIntegrationWs",
        description: "Unfolded Circle Integration WebSocket API definition",
    },
];

fn assemble_ws_definition_module(module: &str) -> Option<TokenStream> {
    match module {
        "elevenlabs_ws" => Some(quote! {
            //! Generated WebSocket API definition helper for ElevenLabs TTS.
            //!
            //! This module currently exposes the typed `WebSocketApi` definition
            //! and related model types from `schematic-definitions`.
            //!
            //! WebSocket runtime client code generation is not yet implemented in `schematic-gen`.

            pub use schematic_definitions::elevenlabs::*;
            use schematic_define::websocket::WebSocketApi;

            /// Builds the ElevenLabs Text-to-Speech WebSocket API definition.
            #[must_use]
            pub fn define_api() -> WebSocketApi {
                schematic_definitions::define_elevenlabs_websocket_api()
            }
        }),
        "unfolded_circle_core_ws" => Some(quote! {
            //! Generated WebSocket API definition helper for Unfolded Circle Core WS.
            //!
            //! This module currently exposes the typed `WebSocketApi` definition
            //! and related model types from `schematic-definitions`.
            //!
            //! WebSocket runtime client code generation is not yet implemented in `schematic-gen`.

            pub use schematic_definitions::unfolded_circle::core_ws::*;
            use schematic_define::websocket::WebSocketApi;

            /// Builds the Unfolded Circle Core WebSocket API definition.
            #[must_use]
            pub fn define_api() -> WebSocketApi {
                schematic_definitions::define_unfolded_circle_core_ws_api()
            }
        }),
        "unfolded_circle_dock_ws" => Some(quote! {
            //! Generated WebSocket API definition helper for Unfolded Circle Dock WS.
            //!
            //! This module currently exposes the typed `WebSocketApi` definition
            //! and related model types from `schematic-definitions`.
            //!
            //! WebSocket runtime client code generation is not yet implemented in `schematic-gen`.

            pub use schematic_definitions::unfolded_circle::dock_ws::*;
            use schematic_define::websocket::WebSocketApi;

            /// Builds the Unfolded Circle Dock WebSocket API definition.
            #[must_use]
            pub fn define_api() -> WebSocketApi {
                schematic_definitions::define_unfolded_circle_dock_ws_api()
            }
        }),
        "unfolded_circle_integration_ws" => Some(quote! {
            //! Generated WebSocket API definition helper for Unfolded Circle Integration WS.
            //!
            //! This module currently exposes the typed `WebSocketApi` definition
            //! and related model types from `schematic-definitions`.
            //!
            //! WebSocket runtime client code generation is not yet implemented in `schematic-gen`.

            pub use schematic_definitions::unfolded_circle::integration_ws::*;
            use schematic_define::websocket::WebSocketApi;

            /// Builds the Unfolded Circle Integration WebSocket API definition.
            #[must_use]
            pub fn define_api() -> WebSocketApi {
                schematic_definitions::define_unfolded_circle_integration_ws_api()
            }
        }),
        _ => None,
    }
}

fn generate_ws_definition_modules() -> Result<Vec<(String, String)>, GeneratorError> {
    WS_DEFINITION_MODULES
        .iter()
        .filter_map(|spec| assemble_ws_definition_module(spec.module).map(|tokens| (spec, tokens)))
        .map(|(spec, tokens)| {
            let file = validate_code(&tokens)?;
            let formatted = format_code(&file);
            Ok((format!("{}.rs", spec.module), formatted))
        })
        .collect()
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
///
/// ## Examples
///
/// ```ignore
/// // Explicit module_path takes precedence
/// let api = RestApi { name: "HuggingFaceHub".to_string(), module_path: Some("huggingface".to_string()), ... };
/// assert_eq!(get_module_path(&api), "huggingface");
///
/// // Inferred from CamelCase
/// let api = RestApi { name: "OllamaNative".to_string(), module_path: None, ... };
/// assert_eq!(get_module_path(&api), "ollama");
///
/// // Fallback to lowercase
/// let api = RestApi { name: "OpenAI".to_string(), module_path: None, ... };
/// // Inference returns "open", fallback returns "openai"
/// ```
fn get_module_path(api: &RestApi) -> String {
    api.module_path
        .clone()
        .unwrap_or_else(|| infer_module_path(&api.name).unwrap_or_else(|| api.name.to_lowercase()))
}

/// Returns the request suffix for the given API.
///
/// Uses `api.request_suffix` if set, otherwise defaults to `"Request"`.
///
/// ## Examples
///
/// ```ignore
/// let api = RestApi { request_suffix: None, ... };
/// assert_eq!(get_request_suffix(&api), "Request");
///
/// let api = RestApi { request_suffix: Some("Params".to_string()), ... };
/// assert_eq!(get_request_suffix(&api), "Params");
/// ```
fn get_request_suffix(api: &RestApi) -> String {
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
    // Generate shared types
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

    // Generate request structs for each endpoint
    let request_structs: TokenStream = api
        .endpoints
        .iter()
        .map(|ep| generate_request_struct_with_options(ep, &suffix, Some(&api_name_lower)))
        .collect();

    // Generate Paginated trait implementations for paginated endpoints
    let paginated_impls: TokenStream = api
        .endpoints
        .iter()
        .map(|ep| generate_paginated_impl(ep, &suffix))
        .collect();

    // Generate request enum
    let request_enum = generate_request_enum_with_suffix(api, &suffix);

    // Generate API struct
    let api_struct = generate_api_struct(api);

    // Generate request method with the appropriate suffix
    let request_method = generate_request_method_with_suffix(api, &suffix);

    // Generate rich module documentation
    let module_docs = ModuleDocBuilder::new(api).build();

    // Build the re-export line based on options
    let definitions_import = if options.standalone {
        // For standalone/imported APIs, import from local types module
        quote! {
            // Re-export model types from local types module
            pub use crate::types::*;
        }
    } else {
        // For native APIs, import from schematic_definitions
        let definitions_module = format_ident!("{}", api_name_lower);
        quote! {
            // Re-export response types from definitions so consumers can import from one place
            pub use schematic_definitions::#definitions_module::*;
        }
    };

    // Conditionally import Paginated only when endpoints use pagination
    let has_paginated = api
        .endpoints
        .iter()
        .any(|ep| ep.params.as_ref().is_some_and(|p| p.pagination.is_some()));

    let shared_import = if has_paginated {
        quote! { use crate::shared::{Paginated, RequestParts, SchematicError}; }
    } else {
        quote! { use crate::shared::{RequestParts, SchematicError}; }
    };

    // Combine all pieces with necessary imports
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
/// of their types. This avoids duplicate `pub mod` declarations and file overwrites.
///
/// The generated module contains:
/// - Combined documentation (one section per API, separated by horizontal rules)
/// - Shared imports emitted once
/// - Per-API: request structs, request enum, API client struct, request method
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

    // Build combined module documentation
    let combined_docs = build_combined_module_docs(apis);

    // Build the definitions re-export (shared once — all APIs share the module)
    let definitions_module = format_ident!("{}", module_path);
    let definitions_import = quote! {
        // Re-export response types from definitions so consumers can import from one place
        pub use schematic_definitions::#definitions_module::*;
    };

    // Generate per-API code blocks
    let per_api_code: Vec<TokenStream> = apis
        .iter()
        .map(|api| {
            let suffix = get_request_suffix(api);

            let request_structs: TokenStream = api
                .endpoints
                .iter()
                .map(|ep| generate_request_struct_with_options(ep, &suffix, Some(&module_path)))
                .collect();

            // Generate Paginated trait implementations for paginated endpoints
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

    // Conditionally import Paginated only when any API has paginated endpoints
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
///
/// Each API gets its own documentation section. Sections are separated by
/// horizontal rules (`---`) for visual clarity.
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
/// This generates the main library file that:
/// - Declares the shared module (containing common types like `SchematicError`)
/// - Declares all API modules
/// - Re-exports modules at crate root
/// - Provides a prelude module
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to include
///
/// ## Returns
///
/// A TokenStream containing the lib.rs code.
pub fn assemble_lib_rs(apis: &[&RestApi]) -> TokenStream {
    // Generate module declarations, deduplicating shared module paths
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
    for ws in WS_DEFINITION_MODULES {
        if seen_modules.insert(ws.module.to_string()) {
            let module_name = format_ident!("{}", ws.module);
            module_decls.push(quote! {
                pub mod #module_name;
            });
        }
    }

    // Build the API table rows for documentation
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
    let ws_rows: Vec<String> = WS_DEFINITION_MODULES
        .iter()
        .map(|ws| {
            format!(
                "//! | [`{}`] | [`{}`]({}::define_api) | {} |",
                ws.module, ws.display_name, ws.module, ws.description
            )
        })
        .collect();
    let ws_table = ws_rows.join("\n");

    // Choose the first API with a GET endpoint for the quick start example
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
                r#"//! ```ignore
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
                r#"//! ```ignore
//! use schematic_schema::prelude::*;
//! ```"#,
            )
        }
    } else {
        String::from(
            r#"//! ```ignore
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
//! ```ignore
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

    // Parse the dynamic sections into token streams
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
/// The prelude provides convenient re-exports for consumers:
/// - All API client structs
/// - All request enums
/// - Common error type (from shared module)
///
/// Note: Response types from definitions are NOT re-exported in the prelude
/// to avoid ambiguous glob re-exports (e.g., `ModelInfo` exists in multiple APIs).
/// Import response types from specific API modules instead:
/// `use schematic_schema::openai::ChatCompletionResponse;`
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
    // Generate re-exports for each API (client and request enum only, not error)
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
        WS_DEFINITION_MODULES
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

    // Build a list of re-exported items for documentation
    // These names are re-exported in prelude, so rustdoc resolves them directly
    let client_list: Vec<String> = apis
        .iter()
        .map(|api| format!("//! - [`{name}`] + [`{name}Request`]", name = api.name))
        .collect();
    let client_list_str = client_list.join("\n");
    let client_list_tokens: TokenStream = client_list_str.parse().unwrap_or_default();
    let ws_helper_list_tokens: TokenStream = if include_ws_helpers {
        WS_DEFINITION_MODULES
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
        //! ```ignore
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
        //! ```ignore
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

// Keep the old function for backwards compatibility in tests
#[doc(hidden)]
pub fn assemble_api_code(api: &RestApi) -> TokenStream {
    assemble_api_module(api)
}

/// Validates generated code using syn.
///
/// Parses the token stream as a complete Rust file to ensure it's syntactically
/// valid before writing to disk.
///
/// ## Arguments
///
/// * `tokens` - The generated code to validate
///
/// ## Returns
///
/// The parsed `syn::File` on success, or an error if the code is invalid.
///
/// ## Errors
///
/// Returns `GeneratorError::CodeGenError` if the code fails to parse.
pub fn validate_code(tokens: &TokenStream) -> Result<syn::File, GeneratorError> {
    syn::parse2(tokens.clone())
        .map_err(|e| GeneratorError::CodeGenError(format!("Generated code is invalid: {}", e)))
}

/// Formats generated code using prettyplease.
///
/// Converts a parsed syn::File back to a nicely formatted string,
/// prepending an auto-generated notice as a regular comment.
///
/// ## Arguments
///
/// * `file` - The parsed Rust file to format
///
/// ## Returns
///
/// A formatted string representation of the code with auto-generated notice.
pub fn format_code(file: &syn::File) -> String {
    let formatted = prettyplease::unparse(file);
    // Prepend auto-generated notice as regular comment
    format!(
        "// This code was automatically generated by schematic-gen. Do not edit manually.\n\n{}",
        formatted
    )
}

/// Writes content to a file atomically using temp file + rename.
///
/// This pattern ensures that:
/// - The file is never left in a partially-written state
/// - Other processes see either the old or new content, never a mix
/// - Power failures or crashes don't corrupt the file
///
/// ## Arguments
///
/// * `path` - The target file path
/// * `content` - The content to write
///
/// ## Returns
///
/// `Ok(())` on success.
///
/// ## Errors
///
/// Returns `GeneratorError::WriteError` if:
/// - Parent directories cannot be created
/// - The temp file cannot be written
/// - The rename operation fails
pub fn write_atomic(path: &Path, content: &str) -> Result<(), GeneratorError> {
    // Create parent directories if needed
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| GeneratorError::WriteError {
            path: parent.display().to_string(),
            source: e,
        })?;
    }

    // Write to temp file first
    let temp_path = path.with_extension("tmp");
    fs::write(&temp_path, content).map_err(|e| GeneratorError::WriteError {
        path: temp_path.display().to_string(),
        source: e,
    })?;

    // Atomically rename to final path
    fs::rename(&temp_path, path).map_err(|e| GeneratorError::WriteError {
        path: path.display().to_string(),
        source: e,
    })?;

    Ok(())
}

/// Generates and writes all API code to the output directory.
///
/// This is the main entry point for code generation. It produces:
/// - `lib.rs` - Module declarations and crate documentation
/// - `prelude.rs` - Convenient re-exports
/// - `{api_name}.rs` - Per-API module files
///
/// ## Arguments
///
/// * `api` - The API definition to generate code for
/// * `output_dir` - Directory to write generated files to
/// * `dry_run` - If true, print code instead of writing files
///
/// ## Returns
///
/// The formatted API module code (useful for dry-run mode or testing).
///
/// ## Errors
///
/// Returns an error if:
/// - Code generation produces invalid Rust
/// - File writing fails
pub fn generate_and_write(
    api: &RestApi,
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    let apis = [api];
    generate_and_write_all(&apis, output_dir, dry_run)
}

/// Generates and writes code for multiple APIs to the output directory.
///
/// This function produces a complete schema crate with:
/// - `lib.rs` - Module declarations for all APIs
/// - `shared.rs` - Shared types (error type, etc.)
/// - `prelude.rs` - Re-exports from all APIs
/// - `{api_name}.rs` - One module file per API
///
/// ## Arguments
///
/// * `apis` - Slice of API definitions to generate code for
/// * `output_dir` - Directory to write generated files to
/// * `dry_run` - If true, print code instead of writing files
///
/// ## Returns
///
/// The formatted code for the first API module (for backwards compatibility).
///
/// ## Errors
///
/// Returns an error if:
/// - Code generation produces invalid Rust
/// - File writing fails
pub fn generate_and_write_all(
    apis: &[&RestApi],
    output_dir: &Path,
    dry_run: bool,
) -> Result<String, GeneratorError> {
    // Generate and validate lib.rs
    let lib_tokens = assemble_lib_rs(apis);
    let lib_file = validate_code(&lib_tokens)?;
    let lib_formatted = format_code(&lib_file);

    // Generate and validate shared.rs
    let shared_tokens = assemble_shared_module();
    let shared_file = validate_code(&shared_tokens)?;
    let shared_formatted = format_code(&shared_file);

    // Generate and validate prelude.rs
    let prelude_tokens = assemble_prelude(apis);
    let prelude_file = validate_code(&prelude_tokens)?;
    let prelude_formatted = format_code(&prelude_file);

    // Group APIs by module path so shared-module APIs get a single file
    let mut module_groups: BTreeMap<String, Vec<&RestApi>> = BTreeMap::new();
    for api in apis {
        let path = get_module_path(api);
        module_groups.entry(path).or_default().push(api);
    }

    // Generate and validate each REST API module (single or combined)
    let mut api_modules: Vec<(String, String)> = Vec::new();
    for (module_path, group) in &module_groups {
        let tokens = if group.len() == 1 {
            assemble_api_module(group[0])
        } else {
            assemble_combined_api_module(group)
        };
        let file = validate_code(&tokens)?;
        let formatted = format_code(&file);
        let filename = format!("{}.rs", module_path);
        api_modules.push((filename, formatted));
    }
    // Generate WebSocket definition helper modules.
    api_modules.extend(generate_ws_definition_modules()?);

    if dry_run {
        println!("=== lib.rs ===\n{}\n", lib_formatted);
        println!("=== shared.rs ===\n{}\n", shared_formatted);
        println!("=== prelude.rs ===\n{}\n", prelude_formatted);
        for (filename, content) in &api_modules {
            println!("=== {} ===\n{}\n", filename, content);
        }
    } else {
        // Write lib.rs
        write_atomic(&output_dir.join("lib.rs"), &lib_formatted)?;

        // Write shared.rs
        write_atomic(&output_dir.join("shared.rs"), &shared_formatted)?;

        // Write prelude.rs
        write_atomic(&output_dir.join("prelude.rs"), &prelude_formatted)?;

        // Write each API module
        for (filename, content) in &api_modules {
            write_atomic(&output_dir.join(filename), content)?;
        }

        // Clean up stale .rs files that are no longer generated
        let expected_files: HashSet<String> = {
            let mut files = HashSet::new();
            files.insert("lib.rs".to_string());
            files.insert("shared.rs".to_string());
            files.insert("prelude.rs".to_string());
            for (filename, _) in &api_modules {
                files.insert(filename.clone());
            }
            files
        };

        if let Ok(entries) = fs::read_dir(output_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().is_some_and(|ext| ext == "rs")
                    && let Some(name) = path.file_name().and_then(|n| n.to_str())
                    && !expected_files.contains(name)
                {
                    let _ = fs::remove_file(&path);
                }
            }
        }
    }

    // Return the first API module content for backwards compatibility
    Ok(api_modules
        .into_iter()
        .next()
        .map(|(_, content)| content)
        .unwrap_or_default())
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
    // Generate module declarations, deduplicating shared module paths
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

    // Add types module if requested (for imported APIs)
    let types_module = if options.include_types_module {
        quote! {
            pub mod types;
        }
    } else {
        TokenStream::new()
    };

    // Build the API table rows for documentation
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

    // Choose the first API with a GET endpoint for the quick start example
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
                r#"//! ```ignore
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
                r#"//! ```ignore
//! use schematic_schema::prelude::*;
//! ```"#,
            )
        }
    } else {
        String::from(
            r#"//! ```ignore
//! use schematic_schema::prelude::*;
//! ```"#,
        )
    };

    // Parse the dynamic sections into token streams
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

/// Generates and writes standalone API code (for imported OpenAPI specs).
///
/// Unlike `generate_and_write`, this function:
/// - Does NOT include `pub use schematic_definitions::...` imports
/// - Uses `pub use crate::types::*` to reference locally generated types
/// - Adds `pub mod types;` to lib.rs if types were generated
///
/// ## Arguments
///
/// * `api` - The API definition to generate code for
/// * `output_dir` - Directory to write generated files to
/// * `dry_run` - If true, print code instead of writing files
/// * `has_types` - If true, include `pub mod types;` in lib.rs
///
/// ## Returns
///
/// The formatted API module code.
///
/// ## Errors
///
/// Returns an error if:
/// - Code generation produces invalid Rust
/// - File writing fails
pub fn generate_and_write_standalone(
    api: &RestApi,
    output_dir: &Path,
    dry_run: bool,
    has_types: bool,
) -> Result<String, GeneratorError> {
    let apis = [api];
    let options = OutputOptions {
        standalone: true,
        include_types_module: has_types,
    };

    // Generate and validate lib.rs with options
    let lib_tokens = assemble_lib_rs_with_options(&apis, &options);
    let lib_file = validate_code(&lib_tokens)?;
    let lib_formatted = format_code(&lib_file);

    // Generate and validate shared.rs
    let shared_tokens = assemble_shared_module();
    let shared_file = validate_code(&shared_tokens)?;
    let shared_formatted = format_code(&shared_file);

    // Generate and validate prelude.rs
    let prelude_tokens = assemble_prelude_with_options(&apis, false);
    let prelude_file = validate_code(&prelude_tokens)?;
    let prelude_formatted = format_code(&prelude_file);

    // Generate and validate the API module with standalone options
    let api_tokens = assemble_api_module_with_options(api, &options);
    let api_file = validate_code(&api_tokens)?;
    let api_formatted = format_code(&api_file);
    let api_filename = format!("{}.rs", get_module_path(api));

    if dry_run {
        println!("=== lib.rs ===\n{}\n", lib_formatted);
        println!("=== shared.rs ===\n{}\n", shared_formatted);
        println!("=== prelude.rs ===\n{}\n", prelude_formatted);
        println!("=== {} ===\n{}\n", api_filename, api_formatted);
    } else {
        // Write lib.rs
        write_atomic(&output_dir.join("lib.rs"), &lib_formatted)?;

        // Write shared.rs
        write_atomic(&output_dir.join("shared.rs"), &shared_formatted)?;

        // Write prelude.rs
        write_atomic(&output_dir.join("prelude.rs"), &prelude_formatted)?;

        // Write API module
        write_atomic(&output_dir.join(&api_filename), &api_formatted)?;
    }

    Ok(api_formatted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{ApiRequest, ApiResponse, AuthStrategy, Endpoint, RestMethod};
    use std::fs;
    use tempfile::TempDir;

    fn make_simple_api() -> RestApi {
        RestApi {
            name: "TestApi".to_string(),
            description: "Test API".to_string(),
            base_url: "https://api.test.com/v1".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/items".to_string(),
                description: "List all items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
                params: None,
            }],
            module_path: None,
            request_suffix: None,
        }
    }

    fn make_complex_api() -> RestApi {
        RestApi {
            name: "OpenAI".to_string(),
            description: "OpenAI REST API".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            docs_url: Some("https://platform.openai.com/docs".to_string()),
            auth: AuthStrategy::BearerToken { header: None },
            env_auth: vec!["OPENAI_API_KEY".to_string()],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![
                Endpoint {
                    id: "ListModels".to_string(),
                    method: RestMethod::Get,
                    path: "/models".to_string(),
                    description: "Lists available models".to_string(),
                    request: None,
                    response: ApiResponse::json_type("ListModelsResponse"),
                    headers: vec![],
                    params: None,
                },
                Endpoint {
                    id: "RetrieveModel".to_string(),
                    method: RestMethod::Get,
                    path: "/models/{model}".to_string(),
                    description: "Retrieves a model".to_string(),
                    request: None,
                    response: ApiResponse::json_type("Model"),
                    headers: vec![],
                    params: None,
                },
                Endpoint {
                    id: "CreateCompletion".to_string(),
                    method: RestMethod::Post,
                    path: "/completions".to_string(),
                    description: "Creates a completion".to_string(),
                    request: Some(ApiRequest::json_type("CreateCompletionRequest")),
                    response: ApiResponse::json_type("Completion"),
                    headers: vec![],
                    params: None,
                },
            ],
            module_path: None,
            request_suffix: None,
        }
    }

    // === assemble_api_code tests ===

    #[test]
    fn assemble_api_code_produces_valid_tokenstream() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);

        // Should produce non-empty output
        assert!(!tokens.is_empty());
    }

    #[test]
    fn assemble_api_code_includes_all_components() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();

        // Should include error type
        assert!(code.contains("SchematicError"));

        // Should include request structs
        assert!(code.contains("ListModelsRequest"));
        assert!(code.contains("RetrieveModelRequest"));
        assert!(code.contains("CreateCompletionRequest"));

        // Should include request enum
        assert!(code.contains("OpenAIRequest"));

        // Should include API struct
        assert!(code.contains("struct OpenAI"));

        // Should include request method
        assert!(code.contains("async fn request"));
    }

    #[test]
    fn assemble_api_code_includes_imports() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();

        assert!(code.contains("serde"));
    }

    #[test]
    fn assemble_api_code_has_no_unnecessary_lint_allows() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let code = tokens.to_string();

        // Generated code should not need blanket lint suppressions
        // All public items are used, all imports are used
        assert!(!code.contains("dead_code"));
        assert!(!code.contains("unused_imports"));
    }

    // === validate_code tests ===

    #[test]
    fn validate_code_accepts_valid_code() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);

        let result = validate_code(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_code_accepts_complex_api() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);

        let result = validate_code(&tokens);
        assert!(result.is_ok());
    }

    #[test]
    fn validate_code_rejects_invalid_code() {
        // Create a token stream that is valid tokens but not a valid Rust file
        // "let x =" is an incomplete statement that won't parse as a file
        let invalid_tokens = quote! {
            let x =
        };

        let result = validate_code(&invalid_tokens);
        assert!(result.is_err());

        match result {
            Err(GeneratorError::CodeGenError(_)) => {} // Expected
            Err(other) => panic!("Unexpected error type: {:?}", other),
            Ok(_) => panic!("Expected error but got success"),
        }
    }

    // === format_code tests ===

    #[test]
    fn format_code_produces_string() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();

        let formatted = format_code(&file);
        assert!(!formatted.is_empty());
    }

    #[test]
    fn format_code_produces_readable_output() {
        let api = make_simple_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();

        let formatted = format_code(&file);

        // Should have proper indentation and newlines
        assert!(formatted.contains('\n'));
        // Should have doc comments
        assert!(formatted.contains("///") || formatted.contains("//!"));
    }

    #[test]
    fn format_code_preserves_structure() {
        let api = make_complex_api();
        let tokens = assemble_api_code(&api);
        let file = validate_code(&tokens).unwrap();

        let formatted = format_code(&file);

        // All major elements should be present
        assert!(formatted.contains("use crate::shared::{RequestParts, SchematicError}"));
        assert!(formatted.contains("pub struct OpenAI"));
        assert!(formatted.contains("pub enum OpenAIRequest"));
    }

    // === write_atomic tests ===

    #[test]
    fn write_atomic_creates_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("test.rs");

        let content = "// Test content";
        let result = write_atomic(&file_path, content);

        assert!(result.is_ok());
        assert!(file_path.exists());

        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, content);
    }

    #[test]
    fn write_atomic_creates_parent_directories() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("nested/deep/test.rs");

        let content = "// Nested content";
        let result = write_atomic(&file_path, content);

        assert!(result.is_ok());
        assert!(file_path.exists());
    }

    #[test]
    fn write_atomic_overwrites_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("existing.rs");

        // Write initial content
        fs::write(&file_path, "// Old content").unwrap();

        // Overwrite with atomic write
        let new_content = "// New content";
        let result = write_atomic(&file_path, new_content);

        assert!(result.is_ok());
        let read_content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(read_content, new_content);
    }

    #[test]
    fn write_atomic_no_temp_file_left_behind() {
        let temp_dir = TempDir::new().unwrap();
        let file_path = temp_dir.path().join("clean.rs");

        write_atomic(&file_path, "// Content").unwrap();

        // Check no .tmp file exists
        let temp_path = file_path.with_extension("tmp");
        assert!(!temp_path.exists());
    }

    // === generate_and_write tests ===

    #[test]
    fn generate_and_write_dry_run_returns_code() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();

        let result = generate_and_write(&api, temp_dir.path(), true);

        assert!(result.is_ok());
        let code = result.unwrap();
        assert!(code.contains("pub struct TestApi"));
    }

    #[test]
    fn generate_and_write_dry_run_no_file_created() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();

        generate_and_write(&api, temp_dir.path(), true).unwrap();

        // No file should be created in dry run
        let output_path = temp_dir.path().join("lib.rs");
        assert!(!output_path.exists());
    }

    #[test]
    fn generate_and_write_creates_lib_rs() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();

        let result = generate_and_write(&api, temp_dir.path(), false);

        assert!(result.is_ok());

        let output_path = temp_dir.path().join("lib.rs");
        assert!(output_path.exists());
    }

    #[test]
    fn generate_and_write_file_contains_formatted_code() {
        let api = make_complex_api();
        let temp_dir = TempDir::new().unwrap();

        generate_and_write(&api, temp_dir.path(), false).unwrap();

        // Check the API module file (openai.rs)
        let api_module_path = temp_dir.path().join("openai.rs");
        let content = fs::read_to_string(api_module_path).unwrap();

        // Should be properly formatted (has indentation)
        assert!(content.contains("    ")); // 4-space indent
        // Should have all components
        assert!(content.contains("pub struct OpenAI"));
        assert!(content.contains("pub enum OpenAIRequest"));
        assert!(content.contains("use crate::shared::{RequestParts, SchematicError}"));

        // Check shared.rs exists and contains SchematicError and RequestParts
        let shared_path = temp_dir.path().join("shared.rs");
        let shared_content = fs::read_to_string(shared_path).unwrap();
        assert!(shared_content.contains("pub type RequestParts"));
        assert!(shared_content.contains("pub enum SchematicError"));

        // Check lib.rs exists and has module declarations
        let lib_path = temp_dir.path().join("lib.rs");
        let lib_content = fs::read_to_string(lib_path).unwrap();
        assert!(lib_content.contains("pub mod shared;"));
        assert!(lib_content.contains("pub mod openai;"));
        assert!(lib_content.contains("pub mod prelude;"));
    }

    #[test]
    fn generate_and_write_returns_same_as_file_content() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();

        let returned = generate_and_write(&api, temp_dir.path(), false).unwrap();

        // The returned content should match the API module file, not lib.rs
        // Note: "TestApi" has "Api" suffix, so it infers to "test"
        let api_module_path = temp_dir.path().join("test.rs");
        let file_content = fs::read_to_string(api_module_path).unwrap();

        assert_eq!(returned, file_content);
    }

    #[test]
    fn generate_and_write_creates_nested_output_dir() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();
        let nested_dir = temp_dir.path().join("src/generated");

        let result = generate_and_write(&api, &nested_dir, false);

        assert!(result.is_ok());
        let output_path = nested_dir.join("lib.rs");
        assert!(output_path.exists());
    }

    // === Integration tests ===

    #[test]
    fn full_pipeline_with_all_auth_strategies() {
        // Test configurations: (auth, env_auth, env_username)
        // For BasicAuth, password comes from env_auth[0]
        let test_cases: Vec<(AuthStrategy, Vec<String>, Option<String>)> = vec![
            (AuthStrategy::None, vec![], None),
            (
                AuthStrategy::BearerToken { header: None },
                vec!["TOKEN".to_string()],
                None,
            ),
            (
                AuthStrategy::ApiKey {
                    header: "X-API-Key".to_string(),
                },
                vec!["KEY".to_string()],
                None,
            ),
            (
                AuthStrategy::Basic,
                vec!["PASS".to_string()], // Password from env_auth[0]
                Some("USER".to_string()),
            ),
        ];

        for (auth, env_auth, env_username) in test_cases {
            let api = RestApi {
                name: "TestApi".to_string(),
                description: "Test".to_string(),
                base_url: "https://test.com".to_string(),
                docs_url: None,
                auth: auth.clone(),
                env_auth,
                env_username,
                env_mapping: None,
                headers: vec![],
                endpoints: vec![Endpoint {
                    id: "Test".to_string(),
                    method: RestMethod::Get,
                    path: "/test".to_string(),
                    description: "Test endpoint".to_string(),
                    request: None,
                    response: ApiResponse::json_type("TestResponse"),
                    headers: vec![],
                    params: None,
                }],
                module_path: None,
                request_suffix: None,
            };

            let temp_dir = TempDir::new().unwrap();
            let result = generate_and_write(&api, temp_dir.path(), false);
            assert!(result.is_ok(), "Failed for auth strategy: {:?}", auth);
        }
    }

    #[test]
    fn full_pipeline_with_all_http_methods() {
        let methods = [
            RestMethod::Get,
            RestMethod::Post,
            RestMethod::Put,
            RestMethod::Patch,
            RestMethod::Delete,
            RestMethod::Head,
            RestMethod::Options,
        ];

        let endpoints: Vec<Endpoint> = methods
            .iter()
            .enumerate()
            .map(|(i, method)| Endpoint {
                id: format!("Endpoint{}", i),
                method: *method,
                path: format!("/path{}", i),
                description: format!("{:?} endpoint", method),
                request: None,
                response: ApiResponse::json_type("Response"),
                headers: vec![],
                params: None,
            })
            .collect();

        let api = RestApi {
            name: "AllMethods".to_string(),
            description: "API with all HTTP methods".to_string(),
            base_url: "https://test.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints,
            module_path: None,
            request_suffix: None,
        };

        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);
        assert!(result.is_ok());

        let content = result.unwrap();
        for method in methods {
            let method_str = format!("{:?}", method).to_uppercase();
            assert!(
                content.contains(&format!("\"{}\"", method_str)),
                "Missing method: {}",
                method_str
            );
        }
    }

    #[test]
    fn full_pipeline_empty_api_produces_valid_code() {
        let api = RestApi {
            name: "EmptyApi".to_string(),
            description: "API with no endpoints".to_string(),
            base_url: "https://empty.com".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![],
            module_path: None,
            request_suffix: None,
        };

        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write(&api, temp_dir.path(), false);

        // Even empty API should produce valid code
        assert!(result.is_ok());
    }

    #[test]
    fn generated_code_has_module_documentation() {
        let api = make_simple_api();
        let temp_dir = TempDir::new().unwrap();

        let code = generate_and_write(&api, temp_dir.path(), true).unwrap();

        // Should have module-level doc comments
        assert!(code.contains("//!"));
        assert!(code.contains("Generated API client"));
        // Auto-generated notice should be a regular comment (not doc comment)
        assert!(code.starts_with("// This code was automatically generated"));
        assert!(code.contains("Do not edit manually"));
    }

    // === Shared module path helpers ===

    /// Creates two APIs that share the same module_path, like Ollama Native/OpenAI.
    fn make_shared_module_apis() -> (RestApi, RestApi) {
        let api_a = RestApi {
            name: "FooNative".to_string(),
            description: "Foo native API".to_string(),
            base_url: "http://localhost:8080".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/api/items".to_string(),
                description: "List items".to_string(),
                request: None,
                response: ApiResponse::json_type("ListItemsResponse"),
                headers: vec![],
                params: None,
            }],
            module_path: Some("foo".to_string()),
            request_suffix: Some("NativeRequest".to_string()),
        };

        let api_b = RestApi {
            name: "FooCompat".to_string(),
            description: "Foo compat API".to_string(),
            base_url: "http://localhost:8080".to_string(),
            docs_url: None,
            auth: AuthStrategy::None,
            env_auth: vec![],
            env_username: None,
            env_mapping: None,
            headers: vec![],
            endpoints: vec![Endpoint {
                id: "ListItems".to_string(),
                method: RestMethod::Get,
                path: "/v1/items".to_string(),
                description: "List items (compat)".to_string(),
                request: None,
                response: ApiResponse::json_type("CompatListItemsResponse"),
                headers: vec![],
                params: None,
            }],
            module_path: Some("foo".to_string()),
            request_suffix: Some("CompatRequest".to_string()),
        };

        (api_a, api_b)
    }

    // === Combined module tests ===

    #[test]
    fn assemble_lib_rs_deduplicates_modules() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];

        let tokens = assemble_lib_rs(&apis);
        let code = tokens.to_string();

        // "pub mod foo" should appear exactly once
        let count = code.matches("pub mod foo").count();
        assert_eq!(
            count, 1,
            "Expected exactly 1 'pub mod foo', found {}",
            count
        );
    }

    #[test]
    fn assemble_combined_module_includes_both_apis() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];

        let tokens = assemble_combined_api_module(&apis);
        let file = validate_code(&tokens).unwrap();
        let code = format_code(&file);

        // Should include both client structs
        assert!(
            code.contains("pub struct FooNative"),
            "Missing FooNative struct"
        );
        assert!(
            code.contains("pub struct FooCompat"),
            "Missing FooCompat struct"
        );

        // Should include both request enums
        assert!(
            code.contains("pub enum FooNativeRequest"),
            "Missing FooNativeRequest enum"
        );
        assert!(
            code.contains("pub enum FooCompatRequest"),
            "Missing FooCompatRequest enum"
        );

        // Should have definitions import only once
        let import_count = code
            .matches("pub use schematic_definitions::foo::*")
            .count();
        assert_eq!(
            import_count, 1,
            "Expected exactly 1 definitions import, found {}",
            import_count
        );

        // Should have shared import only once
        let shared_count = code
            .matches("use crate::shared::{RequestParts, SchematicError}")
            .count();
        assert_eq!(
            shared_count, 1,
            "Expected exactly 1 shared import, found {}",
            shared_count
        );
    }

    #[test]
    fn generate_and_write_all_groups_shared_modules() {
        let (api_a, api_b) = make_shared_module_apis();
        let standalone = make_simple_api();
        let apis: Vec<&RestApi> = vec![&standalone, &api_a, &api_b];

        let temp_dir = TempDir::new().unwrap();
        let result = generate_and_write_all(&apis, temp_dir.path(), false);
        assert!(result.is_ok());

        // Should write a single foo.rs (not foonative.rs + foocompat.rs)
        assert!(temp_dir.path().join("foo.rs").exists(), "Missing foo.rs");
        assert!(
            !temp_dir.path().join("foonative.rs").exists(),
            "Unexpected foonative.rs"
        );
        assert!(
            !temp_dir.path().join("foocompat.rs").exists(),
            "Unexpected foocompat.rs"
        );

        // The combined file should contain both structs
        let foo_content = fs::read_to_string(temp_dir.path().join("foo.rs")).unwrap();
        assert!(foo_content.contains("pub struct FooNative"));
        assert!(foo_content.contains("pub struct FooCompat"));

        // The standalone API should still get its own file
        assert!(temp_dir.path().join("test.rs").exists(), "Missing test.rs");
    }

    #[test]
    fn generate_and_write_all_cleans_stale_files() {
        let api = make_simple_api();
        let apis: Vec<&RestApi> = vec![&api];

        let temp_dir = TempDir::new().unwrap();

        // Create a stale file that should be cleaned up
        fs::write(temp_dir.path().join("ollamaopenai.rs"), "// stale").unwrap();
        fs::write(temp_dir.path().join("old_api.rs"), "// stale").unwrap();
        // Non-.rs files should NOT be cleaned
        fs::write(temp_dir.path().join("Cargo.toml"), "# keep").unwrap();

        generate_and_write_all(&apis, temp_dir.path(), false).unwrap();

        // Stale .rs files should be removed
        assert!(
            !temp_dir.path().join("ollamaopenai.rs").exists(),
            "ollamaopenai.rs should be deleted"
        );
        assert!(
            !temp_dir.path().join("old_api.rs").exists(),
            "old_api.rs should be deleted"
        );

        // Non-.rs files should be preserved
        assert!(
            temp_dir.path().join("Cargo.toml").exists(),
            "Cargo.toml should be preserved"
        );

        // Generated files should exist
        assert!(temp_dir.path().join("lib.rs").exists());
        assert!(temp_dir.path().join("shared.rs").exists());
        assert!(temp_dir.path().join("prelude.rs").exists());
        assert!(temp_dir.path().join("test.rs").exists());
    }

    #[test]
    fn prelude_exports_all_apis_in_shared_module() {
        let (api_a, api_b) = make_shared_module_apis();
        let apis: Vec<&RestApi> = vec![&api_a, &api_b];

        let tokens = assemble_prelude(&apis);
        let file = validate_code(&tokens).unwrap();
        let code = format_code(&file);

        // Both API clients should be re-exported from the shared module
        assert!(code.contains("FooNative"), "Missing FooNative in prelude");
        assert!(code.contains("FooCompat"), "Missing FooCompat in prelude");
        assert!(
            code.contains("FooNativeRequest"),
            "Missing FooNativeRequest in prelude"
        );
        assert!(
            code.contains("FooCompatRequest"),
            "Missing FooCompatRequest in prelude"
        );
    }
}
