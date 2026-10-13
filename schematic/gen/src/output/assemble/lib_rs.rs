//! lib.rs assembly for the generated schema crate.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{AuthStrategy, RestApi, RestMethod};

use super::helpers::{get_module_path, get_request_suffix};
use crate::output::options::OutputOptions;
use crate::output::ws_modules::ws_definition_modules;

/// Assembles the lib.rs content for the schema crate.
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

    let api_table_tokens: TokenStream = api_table.parse().unwrap_or_default();
    let ws_table_tokens: TokenStream = ws_table.parse().unwrap_or_default();
    let quick_start_tokens: TokenStream = quick_start_example.parse().unwrap_or_default();
    let runtime_compatibility = runtime_compatibility();

    quote! {
        //! Generated REST API clients and WebSocket API definition helpers.
        //!
        //! Each REST API module holds the API's client struct, named after the
        //! API and wrapping a [`schematic_runtime::Client`], its request types,
        //! and its response types re-exported from the definitions. Sending, credentials,
        //! redirects, deadlines, and the response envelope belong to
        //! [`schematic_runtime`].
        //!
        //! ## Available REST APIs
        //!
        //! | Module | API | Description | Auth |
        //! |--------|-----|-------------|------|
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
        #quick_start_tokens
        //!
        //! ## Results
        //!
        //! A send returns `Ok` with a [`schematic_runtime::Response`] whenever
        //! the server answered, whatever the status, and
        //! `Err(`[`schematic_runtime::TransportError`]`)` only when no response
        //! arrived.

        #runtime_compatibility

        // Shared types and utilities
        pub mod shared;

        pub mod prelude;

        #(#module_decls)*
    }
}

/// The ABI marker of generated `lib.rs`, the compile-time check that the
/// `schematic-runtime` being built against has that ABI, and the runtime
/// re-export.
///
/// Partial generation reads the marker back with [`syn`] (see
/// `crate::output::inventory`), so its form is part of the contract: one
/// `pub const SCHEMATIC_RUNTIME_ABI: u32 = <integer literal>;` item.
fn runtime_compatibility() -> TokenStream {
    let abi = proc_macro2::Literal::u32_unsuffixed(crate::RUNTIME_ABI);
    let message = format!(
        "schematic-schema was generated for schematic-runtime ABI {}, but the \
         schematic-runtime it is built against has another ABI; regenerate the \
         crate with the schematic-gen of that runtime (`just generate` in schematic/)",
        crate::RUNTIME_ABI
    );
    quote! {
        /// The `schematic-runtime` ABI this crate was generated against.
        pub const SCHEMATIC_RUNTIME_ABI: u32 = #abi;

        const _: () = assert!(schematic_runtime::ABI == SCHEMATIC_RUNTIME_ABI, #message);

        /// The runtime that sends this crate's requests.
        pub use schematic_runtime;
    }
}

/// Assembles lib.rs content with configurable options.
///
/// This is similar to `assemble_lib_rs` but supports options like including
/// a types module for imported APIs.
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

    let runtime_compatibility = runtime_compatibility();

    quote! {
        //! Generated REST API clients.
        //!
        //! Each API is available as a separate module with its client struct,
        //! request types, and response types.
        //!
        //! ## Available APIs
        //!
        //! | Module | API | Description | Auth |
        //! |--------|-----|-------------|------|
        #api_table_tokens
        //!
        //! ## Quick Start
        //!
        #quick_start_tokens

        #runtime_compatibility

        // Shared types and utilities
        pub mod shared;

        pub mod prelude;

        #types_module

        #(#module_decls)*
    }
}

fn build_quick_start_example(example_api: Option<&&RestApi>) -> String {
    let Some(api) = example_api else {
        return String::from(
            r#"//! ```text
//! use schematic_schema::prelude::*;
//! ```"#,
        );
    };
    let api_name = &api.name;
    let module = get_module_path(api);
    let Some(endpoint) = api.endpoints.iter().find(|ep| ep.method == RestMethod::Get) else {
        return format!(
            r#"//! ```text
//! use schematic_schema::prelude::*;
//!
//! let client = {api_name}::new();
//! ```"#
        );
    };
    let request = format!("{}{}", endpoint.id, get_request_suffix(api));
    let construct = if endpoint.path.contains('{') {
        format!("{request}::from(\"example\")")
    } else {
        format!("{request}::default()")
    };
    format!(
        r#"//! ```text
//! use schematic_schema::prelude::*;
//! use schematic_schema::{module}::{request};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {{
//!     let client = {api_name}::new();
//!     let response = client.send({construct}, &CallOptions::new()).await?;
//!     println!("{{}}", response.meta);
//!     let data = response.into_result()?;
//!     Ok(())
//! }}
//! ```"#
    )
}
