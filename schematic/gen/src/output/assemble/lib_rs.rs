//! lib.rs assembly for the generated schema crate.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{AuthStrategy, RestApi, RestMethod};

use super::helpers::{get_module_path, to_snake_case};
use crate::output::options::OutputOptions;
use crate::output::ws_modules::ws_definition_modules;

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
                AuthStrategy::ApiKey { header, .. } => format!("API Key (`{}`)", header),
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

    let quick_start_example = build_quick_start_example(example_api);
    let variant_example = build_variant_example(example_api);

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
                AuthStrategy::ApiKey { header, .. } => format!("API Key (`{}`)", header),
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

    let quick_start_example = build_quick_start_example(example_api);

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

fn build_quick_start_example(example_api: Option<&&RestApi>) -> String {
    if let Some(api) = example_api {
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
    }
}

fn build_variant_example(example_api: Option<&&RestApi>) -> String {
    if let Some(api) = example_api {
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
    }
}
