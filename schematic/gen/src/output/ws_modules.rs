//! WebSocket definition module metadata and assembly.

use proc_macro2::TokenStream;
use quote::quote;

use crate::errors::GeneratorError;

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

fn assemble_ws_definition_module(module: &str) -> Option<TokenStream> {
    use crate::ws_codegen::client::generate_ws_client_module;
    use crate::ws_codegen::host::generate_ws_host_module;
    use crate::ws_codegen::plan::lower_to_plan;

    let (define_fn, reexport_path, api_fn): (
        fn() -> schematic_define::WebSocketApi,
        TokenStream,
        TokenStream,
    ) = match module {
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
    };

    let api = define_fn();
    let plan = match lower_to_plan(&api) {
        Ok(plan) => plan,
        Err(_) => return None,
    };

    let module_docs = crate::ws_codegen::docs::generate_module_docs(&plan);
    let client_code = generate_ws_client_module(&plan);
    let host_code = generate_ws_host_module(&plan);

    Some(quote! {
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

        #reexport_path

        #api_fn

        #client_code
        #host_code
    })
}

/// Generate all WebSocket definition modules.
pub fn generate_ws_definition_modules() -> Result<Vec<(String, String)>, GeneratorError> {
    WS_DEFINITION_MODULES
        .iter()
        .filter_map(|spec| assemble_ws_definition_module(spec.module).map(|tokens| (spec, tokens)))
        .map(|(spec, tokens)| {
            let file = super::format::validate_code(&tokens)?;
            let formatted = super::format::format_code(&file);
            Ok((format!("{}.rs", spec.module), formatted))
        })
        .collect()
}

/// Generate the ws_shared module.
pub fn generate_ws_shared_module() -> Result<(String, String), GeneratorError> {
    use crate::ws_codegen::shared::generate_ws_shared_module;

    let tokens = generate_ws_shared_module();
    let file = super::format::validate_code(&tokens)?;
    let formatted = super::format::format_code(&file);
    Ok(("ws_shared.rs".to_string(), formatted))
}
