//! WebSocket definition module metadata and assembly.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use schematic_define::WebSocketApi;
use schematic_definitions::registry::{ApiRegistration, WebSocketRegistration, websocket_apis};

use crate::diagnostics::{Feature, Output, ValidationDiagnostic};
use crate::errors::GeneratorError;
use crate::ws_codegen::client::generate_ws_client_module;
use crate::ws_codegen::host::generate_ws_host_module;
use crate::ws_codegen::plan::{WsRuntimePlan, lower_to_plan};

/// The WebSocket entries of `registrations`, in order.
pub fn ws_registrations(
    registrations: &[ApiRegistration],
) -> impl Iterator<Item = (&ApiRegistration, &WebSocketRegistration)> {
    registrations.iter().filter_map(|entry| entry.websocket().map(|ws| (entry, ws)))
}

/// The prelude name of a WebSocket module's `define_api` helper.
pub fn ws_helper_name(module: &str) -> String {
    format!("define_{module}_api_definition")
}

/// The definition re-export and `define_api` tokens of one registered
/// WebSocket module.
fn ws_module_items(entry: &ApiRegistration, ws: &WebSocketRegistration) -> TokenStream {
    let segments = entry.definitions_path.split("::").map(|segment| format_ident!("{}", segment));
    let constructor = format_ident!("{}", ws.constructor);
    let doc = format!(" Builds the {}.", ws.summary);
    quote! {
        pub use schematic_definitions::#(#segments)::*::*;

        #[doc = #doc]
        #[must_use]
        pub fn define_api() -> schematic_define::websocket::WebSocketApi {
            schematic_definitions::#constructor()
        }
    }
}

/// The findings of lowering WebSocket definitions for the Rust output.
#[derive(Debug, Default)]
pub struct WsValidation {
    /// Warnings and errors in the shared format ([`Output::Rust`]); an error
    /// blocks the Rust output like any other Rust validation error.
    pub diagnostics: Vec<ValidationDiagnostic>,
    /// Info notes, which the shared format has no level for. The pipeline
    /// prints them only in a verbose run.
    pub notes: Vec<String>,
}

/// Lowers every API in `apis` and collects all of their findings.
pub fn validate_ws_apis(apis: &[WebSocketApi]) -> WsValidation {
    let mut validation = WsValidation::default();
    for api in apis {
        let findings = match lower_to_plan(api) {
            Ok(plan) => plan.diagnostics,
            Err(failure) => failure.diagnostics,
        };
        for finding in findings {
            match finding.to_validation_diagnostic(&api.name) {
                Some(diagnostic) => validation.diagnostics.push(diagnostic),
                None => validation.notes.push(format!(
                    "severity=info api={} feature={} output={}: {}: {}",
                    api.name,
                    Feature::WebSocketDefinition,
                    Output::Rust,
                    finding.path,
                    finding.message
                )),
            }
        }
    }
    validation
}

/// [`validate_ws_apis`] over the WebSocket definitions of `registrations`,
/// which every Rust generation run writes.
pub fn validate_ws_definition_modules(registrations: &[ApiRegistration]) -> WsValidation {
    validate_ws_apis(&websocket_apis(registrations))
}

/// Lowers `api`, turning a failure into the Rust output's validation error.
fn lower_for_rust(api: &WebSocketApi) -> Result<WsRuntimePlan, GeneratorError> {
    lower_to_plan(api).map_err(|failure| GeneratorError::Validation {
        output: Output::Rust,
        diagnostics: failure.validation_diagnostics(),
    })
}

/// The module body shared by shipped and fixture WebSocket modules: module
/// docs, `prelude` items, client, and host.
fn ws_module_tokens(plan: &WsRuntimePlan, prelude: TokenStream) -> TokenStream {
    let module_docs = crate::ws_codegen::docs::generate_module_docs(plan);
    let client_code = generate_ws_client_module(plan);
    let host_code = generate_ws_host_module(plan);

    quote! {
        #module_docs

        // Generated WebSocket modules intentionally use slightly verbose
        // patterns (explicit clones, separate `if let` blocks, single-arm
        // `match`, large `Result::Err` variants from the runtime
        // transport). These are stable codegen shapes shared by every WS
        // module; suppressing the corresponding clippy lints here keeps
        // the templates simple without changing emitted behavior.
        #![allow(
            clippy::clone_on_copy,
            clippy::collapsible_if,
            clippy::possible_missing_else,
            clippy::result_large_err,
            clippy::single_match,
        )]

        #prelude

        #client_code
        #host_code
    }
}

/// Renders the WebSocket module for any `api`, formatted, as it would be
/// written beside `ws_shared.rs` (it refers to `super::ws_shared`).
///
/// Unlike a shipped module it has no definition re-export or `define_api`,
/// so a fixture definition can be compiled into a crate that does not
/// register it.
///
/// ## Errors
///
/// [`GeneratorError::Validation`] for [`Output::Rust`] when lowering reports
/// an error, carrying every warning and error of the run.
pub fn render_ws_api_module(api: &WebSocketApi) -> Result<String, GeneratorError> {
    let plan = lower_for_rust(api)?;
    let tokens = ws_module_tokens(&plan, TokenStream::new());
    let file = super::format::validate_code(&tokens)?;
    Ok(super::format::format_code(&file))
}

/// Generate the WebSocket definition module of every WebSocket entry in
/// `registrations`.
///
/// ## Errors
///
/// [`GeneratorError::Validation`] for [`Output::Rust`] naming every
/// definition's warnings and errors when any of them fails to lower; no
/// module is dropped silently.
pub fn generate_ws_definition_modules(
    registrations: &[ApiRegistration],
) -> Result<Vec<(String, String)>, GeneratorError> {
    let mut modules = Vec::new();
    let mut diagnostics = Vec::new();
    for (entry, ws) in ws_registrations(registrations) {
        let plan = match lower_for_rust(&(ws.define)()) {
            Ok(plan) => plan,
            Err(GeneratorError::Validation { diagnostics: found, .. }) => {
                diagnostics.extend(found);
                continue;
            }
            Err(other) => return Err(other),
        };
        let tokens = ws_module_tokens(&plan, ws_module_items(entry, ws));
        let file = super::format::validate_code(&tokens)?;
        modules.push((format!("{}.rs", entry.module), super::format::format_code(&file)));
    }
    if !diagnostics.is_empty() {
        return Err(GeneratorError::Validation {
            output: Output::Rust,
            diagnostics,
        });
    }
    Ok(modules)
}

/// Generate the ws_shared module.
pub fn generate_ws_shared_module() -> Result<(String, String), GeneratorError> {
    use crate::ws_codegen::shared::generate_ws_shared_module;

    let tokens = generate_ws_shared_module();
    let file = super::format::validate_code(&tokens)?;
    let formatted = super::format::format_code(&file);
    Ok(("ws_shared.rs".to_string(), formatted))
}
