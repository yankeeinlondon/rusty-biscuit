//! WebSocket definition module metadata and assembly.

use proc_macro2::TokenStream;
use quote::quote;
use schematic_define::WebSocketApi;

use crate::diagnostics::{Feature, Output, ValidationDiagnostic};
use crate::errors::GeneratorError;
use crate::ws_codegen::client::generate_ws_client_module;
use crate::ws_codegen::host::generate_ws_host_module;
use crate::ws_codegen::plan::{WsRuntimePlan, lower_to_plan};

/// Metadata for generated WebSocket definition helper modules.
pub struct WsDefinitionModule {
    pub module: &'static str,
    pub helper: &'static str,
    pub display_name: &'static str,
    pub description: &'static str,
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
    WsDefinitionModule {
        module: "samsung_smart_tv_remote_ws",
        helper: "define_samsung_smart_tv_remote_ws_api_definition",
        display_name: "SamsungSmartTvRemote",
        description: "Samsung Smart TV Remote Control WebSocket API definition",
    },
];

/// Returns the list of WS definition module specs for use in prelude/lib.rs generation.
pub fn ws_definition_modules() -> &'static [WsDefinitionModule] {
    WS_DEFINITION_MODULES
}

/// The definition function, re-export, and `define_api` tokens of one
/// shipped WebSocket module; `None` for a module name not in
/// [`WS_DEFINITION_MODULES`].
type ShippedWsModule = (fn() -> WebSocketApi, TokenStream, TokenStream);

fn shipped_ws_module(module: &str) -> Option<ShippedWsModule> {
    Some(match module {
        "elevenlabs_ws" => (
            || schematic_definitions::define_elevenlabs_websocket_api(),
            quote! { pub use schematic_definitions::elevenlabs::*; },
            quote! {
                /// Builds the ElevenLabs Text-to-Speech WebSocket API definition.
                #[must_use]
                pub fn define_api() -> schematic_define::websocket::WebSocketApi {
                    schematic_definitions::define_elevenlabs_websocket_api()
                }
            },
        ),
        "unfolded_circle_core_ws" => (
            || schematic_definitions::define_unfolded_circle_core_ws_api(),
            quote! { pub use schematic_definitions::unfolded_circle::core_ws::*; },
            quote! {
                /// Builds the Unfolded Circle Core WebSocket API definition.
                #[must_use]
                pub fn define_api() -> schematic_define::websocket::WebSocketApi {
                    schematic_definitions::define_unfolded_circle_core_ws_api()
                }
            },
        ),
        "unfolded_circle_dock_ws" => (
            || schematic_definitions::define_unfolded_circle_dock_ws_api(),
            quote! { pub use schematic_definitions::unfolded_circle::dock_ws::*; },
            quote! {
                /// Builds the Unfolded Circle Dock WebSocket API definition.
                #[must_use]
                pub fn define_api() -> schematic_define::websocket::WebSocketApi {
                    schematic_definitions::define_unfolded_circle_dock_ws_api()
                }
            },
        ),
        "unfolded_circle_integration_ws" => (
            || schematic_definitions::define_unfolded_circle_integration_ws_api(),
            quote! { pub use schematic_definitions::unfolded_circle::integration_ws::*; },
            quote! {
                /// Builds the Unfolded Circle Integration WebSocket API definition.
                #[must_use]
                pub fn define_api() -> schematic_define::websocket::WebSocketApi {
                    schematic_definitions::define_unfolded_circle_integration_ws_api()
                }
            },
        ),
        "samsung_smart_tv_remote_ws" => (
            || schematic_definitions::define_samsung_smart_tv_remote_ws_api(),
            quote! { pub use schematic_definitions::samsung_smart_tv::remote_ws::*; },
            quote! {
                /// Builds the Samsung Smart TV Remote Control WebSocket API definition.
                #[must_use]
                pub fn define_api() -> schematic_define::websocket::WebSocketApi {
                    schematic_definitions::define_samsung_smart_tv_remote_ws_api()
                }
            },
        ),
        _ => return None,
    })
}

/// The shipped WebSocket API definitions, in [`WS_DEFINITION_MODULES`] order.
pub fn shipped_ws_apis() -> Vec<WebSocketApi> {
    WS_DEFINITION_MODULES
        .iter()
        .filter_map(|spec| shipped_ws_module(spec.module))
        .map(|(define_fn, _, _)| define_fn())
        .collect()
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

/// [`validate_ws_apis`] over the shipped WebSocket definitions, which every
/// Rust generation run writes.
pub fn validate_ws_definition_modules() -> WsValidation {
    validate_ws_apis(&shipped_ws_apis())
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

/// Generate all WebSocket definition modules.
///
/// ## Errors
///
/// [`GeneratorError::Validation`] for [`Output::Rust`] naming every shipped
/// definition's warnings and errors when any of them fails to lower; no
/// module is dropped silently.
pub fn generate_ws_definition_modules() -> Result<Vec<(String, String)>, GeneratorError> {
    let mut modules = Vec::new();
    let mut diagnostics = Vec::new();
    for spec in WS_DEFINITION_MODULES {
        let (define_fn, reexport_path, api_fn) = shipped_ws_module(spec.module)
            .ok_or_else(|| {
                GeneratorError::CodeGenError(format!(
                    "WebSocket module `{}` has no definition function",
                    spec.module
                ))
            })?;
        let plan = match lower_for_rust(&define_fn()) {
            Ok(plan) => plan,
            Err(GeneratorError::Validation { diagnostics: found, .. }) => {
                diagnostics.extend(found);
                continue;
            }
            Err(other) => return Err(other),
        };
        let tokens = ws_module_tokens(
            &plan,
            quote! {
                #reexport_path

                #api_fn
            },
        );
        let file = super::format::validate_code(&tokens)?;
        modules.push((format!("{}.rs", spec.module), super::format::format_code(&file)));
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
