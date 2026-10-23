//! lib.rs assembly for the generated schema crate.

use std::collections::HashSet;

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::{AuthStrategy, RestApi, RestMethod};

use schematic_definitions::naming::resolve_module_name;
use crate::output::options::OutputOptions;
use schematic_definitions::registry::ApiRegistration;

use crate::output::ws_modules::ws_registrations;

/// Assembles the lib.rs content for the schema crate, declaring and listing
/// every WebSocket entry of `registrations` beside the REST `apis`.
///
/// Each module declaration, and its row in the crate docs, is behind the
/// Cargo feature named for the module (see [`crate::cargo_gen::ModuleFeatures`]);
/// `ws_shared` compiles when any WebSocket module does.
pub fn assemble_lib_rs(apis: &[&RestApi], registrations: &[ApiRegistration]) -> TokenStream {
    let mut seen_modules = HashSet::new();
    let mut module_decls: Vec<_> = apis
        .iter()
        .filter_map(|api| {
            let path = resolve_module_name(api);
            if seen_modules.insert(path.clone()) {
                let module_name = format_ident!("{}", path);
                Some(quote! {
                    #[cfg(feature = #path)]
                    pub mod #module_name;
                })
            } else {
                None
            }
        })
        .collect();
    let ws_modules: Vec<&str> = ws_registrations(registrations)
        .map(|(entry, _)| entry.module)
        .filter(|module| !seen_modules.contains(*module))
        .collect();
    if seen_modules.insert("ws_shared".to_string()) {
        // Some helpers serve only some WebSocket modules, so a build without
        // every one of them may leave those unused; the default build checks all.
        module_decls.push(quote! {
            #[cfg(any(#(feature = #ws_modules),*))]
            #[cfg_attr(not(all(#(feature = #ws_modules),*)), allow(dead_code))]
            pub mod ws_shared;
        });
    }
    for module in &ws_modules {
        if seen_modules.insert(module.to_string()) {
            let module_name = format_ident!("{}", module);
            module_decls.push(quote! {
                #[cfg(feature = #module)]
                pub mod #module_name;
            });
        }
    }

    let api_table_tokens: TokenStream = apis
        .iter()
        .map(|api| {
            let module = resolve_module_name(api);
            let name = &api.name;
            let desc = &api.description;
            let auth_desc = match &api.auth {
                AuthStrategy::None => "None".to_string(),
                AuthStrategy::BearerToken { .. } => "Bearer".to_string(),
                AuthStrategy::ApiKey { header, .. } => format!("API Key (`{}`)", header),
                AuthStrategy::Basic { .. } => "Basic".to_string(),
                _ => "Custom".to_string(),
            };
            feature_doc(
                &module,
                &format!(" | [`{module}`] | [`{name}`]({module}::{name}) | {desc} | {auth_desc} |"),
            )
        })
        .collect();
    let ws_table_tokens: TokenStream = ws_registrations(registrations)
        .map(|(entry, ws)| {
            feature_doc(
                entry.module,
                &format!(
                    " | [`{}`] | [`{}`]({}::define_api) | {} |",
                    entry.module, entry.name, entry.module, ws.summary
                ),
            )
        })
        .collect();

    let example_api = apis
        .iter()
        .find(|api| api.endpoints.iter().any(|ep| ep.method == RestMethod::Get))
        .or_else(|| apis.first());

    let quick_start_example = build_quick_start_example(example_api);

    let quick_start_tokens: TokenStream = quick_start_example.parse().unwrap_or_default();
    let runtime_compatibility = runtime_compatibility();

    quote! {
        //! Generated REST API clients and WebSocket API definition helpers.
        //!
        //! Each REST API module holds the API's client struct, named after the
        //! API and wrapping a [`schematic_runtime::Client`], with one method per
        //! endpoint; the call type each method returns; its request types; and
        //! its response types re-exported from the definitions. Sending,
        //! credentials, redirects, deadlines, and the response envelope belong
        //! to [`schematic_runtime`].
        //!
        //! ## Features
        //!
        //! Every module is behind a Cargo feature named for the module, and
        //! the default features enable all of them. Build with
        //! `default-features = false` and list the modules you use to compile
        //! only those; APIs that share a module share its feature.
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
        //! Awaiting an endpoint call returns `Ok` with the endpoint's data for a
        //! 2xx reply that decoded, and otherwise a
        //! [`schematic_runtime::CallError`]. After `.full()`, it returns `Ok`
        //! with a [`schematic_runtime::Response`] whenever the server answered,
        //! whatever the status, and
        //! `Err(`[`schematic_runtime::TransportError`]`)` only when no response
        //! arrived.

        #runtime_compatibility

        // Shared types and utilities
        pub mod shared;

        pub mod prelude;

        #(#module_decls)*
    }
}

/// A crate-level doc line shown only when `feature` is enabled.
pub(crate) fn feature_doc(feature: &str, line: &str) -> TokenStream {
    quote! {
        #![cfg_attr(feature = #feature, doc = #line)]
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
            let path = resolve_module_name(api);
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
            let module = resolve_module_name(api);
            let name = &api.name;
            let desc = &api.description;
            let auth_desc = match &api.auth {
                AuthStrategy::None => "None".to_string(),
                AuthStrategy::BearerToken { .. } => "Bearer".to_string(),
                AuthStrategy::ApiKey { header, .. } => format!("API Key (`{}`)", header),
                AuthStrategy::Basic { .. } => "Basic".to_string(),
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
    let Some(endpoint) = api.endpoints.iter().find(|ep| ep.method == RestMethod::Get) else {
        return format!(
            r#"//! ```text
//! use schematic_schema::prelude::*;
//!
//! let client = {api_name}::new();
//! ```"#
        );
    };
    let call = crate::codegen::endpoint_methods::example_invocation(api, endpoint, "client");
    format!(
        r#"//! ```text
//! use schematic_schema::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {{
//!     let client = {api_name}::new();
//!     let data = {call}.await?;
//!     Ok(())
//! }}
//! ```"#
    )
}
