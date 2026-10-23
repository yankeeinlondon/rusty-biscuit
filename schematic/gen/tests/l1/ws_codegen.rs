//! WebSocket code generation integration tests.
//!
//! These tests exercise the full WS codegen pipeline from definition → plan → generated code,
//! verifying structural correctness and syn-parseable output for all known WS APIs.

use schematic_definitions::{
    define_elevenlabs_websocket_api, define_samsung_smart_tv_remote_ws_api,
    define_unfolded_circle_core_ws_api, define_unfolded_circle_dock_ws_api,
    define_unfolded_circle_integration_ws_api,
};
use schematic_gen::ws_codegen::client::generate_ws_client_module;
use schematic_gen::ws_codegen::docs::generate_module_docs;
use schematic_gen::ws_codegen::host::generate_ws_host_module;
use schematic_gen::ws_codegen::plan::{EndpointRole, WsRuntimePlan, lower_to_plan};
use schematic_gen::ws_codegen::shared::generate_ws_shared_module;

fn lower_all_apis() -> Vec<(&'static str, WsRuntimePlan)> {
    vec![
        (
            "ElevenLabsTTS",
            lower_to_plan(&define_elevenlabs_websocket_api()).unwrap(),
        ),
        (
            "UnfoldedCircleCoreWs",
            lower_to_plan(&define_unfolded_circle_core_ws_api()).unwrap(),
        ),
        (
            "UnfoldedCircleDockWs",
            lower_to_plan(&define_unfolded_circle_dock_ws_api()).unwrap(),
        ),
        (
            "UnfoldedCircleIntegrationWs",
            lower_to_plan(&define_unfolded_circle_integration_ws_api()).unwrap(),
        ),
        (
            "SamsungSmartTvRemote",
            lower_to_plan(&define_samsung_smart_tv_remote_ws_api()).unwrap(),
        ),
    ]
}

// --- Plan lowering tests ---

#[test]
fn all_apis_lower_without_errors() {
    for (name, plan) in lower_all_apis() {
        let errors: Vec<_> = plan
            .diagnostics
            .iter()
            .filter(|d| d.severity == schematic_gen::ws_codegen::plan::WsSeverity::Error)
            .collect();
        assert!(
            errors.is_empty(),
            "{name} lowering produced errors: {errors:?}"
        );
    }
}

#[test]
fn elevenlabs_has_two_endpoints_no_correlation() {
    let plan = lower_to_plan(&define_elevenlabs_websocket_api()).unwrap();
    assert_eq!(plan.endpoints.len(), 2);
    for ep in &plan.endpoints {
        assert!(
            matches!(
                ep.correlation,
                schematic_gen::ws_codegen::plan::CorrelationStrategy::None
            ),
            "ElevenLabs endpoint {} should NOT have correlation",
            ep.id
        );
    }
}

#[test]
fn unfolded_circle_core_has_correlated_endpoints() {
    let plan = lower_to_plan(&define_unfolded_circle_core_ws_api()).unwrap();
    assert_eq!(plan.endpoints.len(), 4);
    for ep in &plan.endpoints {
        assert!(
            matches!(
                ep.correlation,
                schematic_gen::ws_codegen::plan::CorrelationStrategy::Correlated { .. }
            ),
            "Core WS endpoint {} should have correlation",
            ep.id
        );
    }
}

#[test]
fn integration_ws_has_both_role() {
    let plan = lower_to_plan(&define_unfolded_circle_integration_ws_api()).unwrap();
    assert_eq!(plan.endpoints.len(), 1);
    assert_eq!(plan.endpoints[0].role, EndpointRole::Both);
    // The role is declared, not inferred: the messages are one-way except
    // the shared event envelope, and the host is still generated.
    assert!(!plan.endpoints[0].client_messages.is_empty());
    assert!(!plan.endpoints[0].server_messages.is_empty());
    assert!(!generate_ws_host_module(&plan).is_empty());
}

// --- Shared module generation tests ---

#[test]
fn ws_shared_module_parses_as_valid_rust() {
    let tokens = generate_ws_shared_module();
    let result = syn::parse2::<syn::File>(tokens);
    assert!(result.is_ok(), "ws_shared must parse: {:?}", result.err());
}

// --- Client generation tests ---

#[test]
fn all_client_modules_parse_as_valid_rust() {
    for (name, plan) in lower_all_apis() {
        let tokens = generate_ws_client_module(&plan);
        let result = syn::parse2::<syn::File>(tokens.clone());
        assert!(
            result.is_ok(),
            "{name} client module must parse as valid Rust: {:?}\n\nGenerated code:\n{}",
            result.err(),
            tokens
        );
    }
}

#[test]
fn client_struct_name_avoids_ws_duplication() {
    for (name, plan) in lower_all_apis() {
        let tokens = generate_ws_client_module(&plan);
        let code = tokens.to_string();
        assert!(
            !code.contains("WsWs"),
            "{name} client has duplicated WsWs in struct name"
        );
    }
}

#[test]
fn correlated_clients_have_request_method() {
    let plan = lower_to_plan(&define_unfolded_circle_core_ws_api()).unwrap();
    let tokens = generate_ws_client_module(&plan);
    let code = tokens.to_string();
    assert!(
        code.contains("pub async fn request"),
        "Correlated API client should have request method"
    );
}

#[test]
fn non_correlated_clients_lack_request_method() {
    let plan = lower_to_plan(&define_elevenlabs_websocket_api()).unwrap();
    let tokens = generate_ws_client_module(&plan);
    let code = tokens.to_string();
    assert!(
        !code.contains("pub async fn request"),
        "Non-correlated API client should NOT have request method"
    );
}

#[test]
fn all_clients_have_events_stream() {
    for (name, plan) in lower_all_apis() {
        let tokens = generate_ws_client_module(&plan);
        let code = tokens.to_string();
        assert!(
            code.contains("fn events"),
            "{name} client should have events() stream adapter"
        );
    }
}

#[test]
fn all_clients_have_auth_header_support() {
    for (name, plan) in lower_all_apis() {
        let tokens = generate_ws_client_module(&plan);
        let code = tokens.to_string();
        assert!(
            code.contains("headers"),
            "{name} client should have headers field"
        );
        assert!(
            code.contains("header_pairs"),
            "{name} connect should pass header_pairs"
        );
    }
}

#[test]
fn dial_surfaces_unparsable_headers_instead_of_dropping_them() {
    for (name, plan) in lower_all_apis() {
        let code = generate_ws_client_module(&plan).to_string();
        assert!(
            code.contains("InvalidHeader"),
            "{name} dial should map a header parse failure to WsError::InvalidHeader"
        );
        // The old behavior tucked both parses into an `if let (Ok(_), Ok(_))`
        // and silently skipped the header on failure.
        assert!(
            !code.contains("if let (Ok (hdr_name) , Ok (hdr_value))"),
            "{name} dial must not silently drop unparsable headers"
        );
    }
}

#[test]
fn elevenlabs_client_has_api_key_env_mapping() {
    let plan = lower_to_plan(&define_elevenlabs_websocket_api()).unwrap();
    let tokens = generate_ws_client_module(&plan);
    let code = tokens.to_string();
    assert!(
        code.contains("api_key"),
        "ElevenLabs should have api_key env mapping"
    );
    assert!(
        code.contains("xi-api-key"),
        "ElevenLabs should reference xi-api-key header"
    );
}

#[test]
fn elevenlabs_connect_signature_uses_typed_path_and_query_params() {
    let plan = lower_to_plan(&define_elevenlabs_websocket_api()).unwrap();
    let tokens = generate_ws_client_module(&plan);
    let code = tokens.to_string();

    assert!(
        code.contains("TextToSpeechConnectionParams"),
        "Expected endpoint query params struct for TextToSpeech"
    );
    assert!(
        code.contains("connect_text_to_speech"),
        "Expected connect_text_to_speech method"
    );
    assert!(
        code.contains("voice_id : impl Into < String >"),
        "Expected typed voice_id path argument"
    );
    assert!(
        code.contains("params : TextToSpeechConnectionParams"),
        "Expected typed connection params argument"
    );
    assert!(
        code.contains("replace")
            && code.contains("{voice_id}")
            && code.contains("urlencoding :: encode"),
        "Expected path placeholder substitution"
    );
}

#[test]
fn correlated_clients_use_options_request_timeout() {
    let plan = lower_to_plan(&define_unfolded_circle_core_ws_api()).unwrap();
    let tokens = generate_ws_client_module(&plan);
    let code = tokens.to_string();

    assert!(
        code.contains("request_timeout : _options . request_timeout"),
        "Expected request timeout to come from options"
    );
    assert!(
        !code.contains("request_timeout : std :: time :: Duration :: from_secs"),
        "Should not hardcode correlated request timeout"
    );
}

#[test]
fn client_connect_uses_transport_config_and_disable_nagle() {
    let plan = lower_to_plan(&define_unfolded_circle_core_ws_api()).unwrap();
    let tokens = generate_ws_client_module(&plan);
    let code = tokens.to_string();

    assert!(
        code.contains("connect_async_with_config"),
        "Expected connect_async_with_config usage"
    );
    assert!(
        code.contains("options . websocket_config . clone"),
        "Expected websocket config to be passed into connect"
    );
    assert!(
        code.contains("options . disable_nagle"),
        "Expected disable_nagle to be passed into connect"
    );
}

#[test]
fn ws_shared_exposes_options_builder() {
    let tokens = generate_ws_shared_module();
    let code = tokens.to_string();
    assert!(code.contains("WsClientOptionsBuilder"));
    assert!(code.contains("fn builder"));
    assert!(code.contains("fn build"));
}

// --- Host generation tests ---

#[test]
fn host_generated_only_for_host_or_both_roles() {
    for (name, plan) in lower_all_apis() {
        let tokens = generate_ws_host_module(&plan);
        let has_host_endpoints = plan
            .endpoints
            .iter()
            .any(|ep| matches!(ep.role, EndpointRole::Host | EndpointRole::Both));

        if has_host_endpoints {
            assert!(
                !tokens.is_empty(),
                "{name} should generate host code for its host/both endpoints"
            );
        } else {
            assert!(
                tokens.is_empty(),
                "{name} should NOT generate host code (no host/both endpoints)"
            );
        }
    }
}

#[test]
fn host_module_parses_as_valid_rust() {
    let plan = lower_to_plan(&define_unfolded_circle_integration_ws_api()).unwrap();
    let tokens = generate_ws_host_module(&plan);
    assert!(!tokens.is_empty());
    let result = syn::parse2::<syn::File>(tokens.clone());
    assert!(
        result.is_ok(),
        "Integration WS host module must parse: {:?}\n\nGenerated code:\n{}",
        result.err(),
        tokens
    );
}

#[test]
fn host_struct_name_avoids_ws_duplication() {
    let plan = lower_to_plan(&define_unfolded_circle_integration_ws_api()).unwrap();
    let tokens = generate_ws_host_module(&plan);
    let code = tokens.to_string();
    assert!(
        !code.contains("WsWsHost"),
        "Host struct has duplicated WsWs in name"
    );
    assert!(
        code.contains("WsHost"),
        "Host struct should end with WsHost"
    );
}

// --- Module docs tests ---

#[test]
fn module_docs_include_api_info() {
    for (name, plan) in lower_all_apis() {
        let tokens = generate_module_docs(&plan);
        let code = tokens.to_string();
        assert!(
            code.contains(&plan.base_url),
            "{name} module docs should include base URL"
        );
        for ep in &plan.endpoints {
            assert!(
                code.contains(&ep.id),
                "{name} module docs should list endpoint {}",
                ep.id
            );
        }
    }
}

// --- Full assembly pipeline test ---

#[test]
fn full_assembly_produces_valid_rust_for_all_apis() {
    for (name, plan) in lower_all_apis() {
        let module_docs = generate_module_docs(&plan);
        let client_code = generate_ws_client_module(&plan);
        let host_code = generate_ws_host_module(&plan);

        let combined = quote::quote! {
            #module_docs
            #client_code
            #host_code
        };

        let result = syn::parse2::<syn::File>(combined.clone());
        assert!(
            result.is_ok(),
            "{name} full assembly must parse as valid Rust: {:?}",
            result.err()
        );
    }
}

// --- Samsung-specific tests ---

#[test]
fn samsung_remote_has_no_correlation() {
    let plan = lower_to_plan(&define_samsung_smart_tv_remote_ws_api()).unwrap();
    assert_eq!(plan.endpoints.len(), 2);
    for ep in &plan.endpoints {
        assert!(
            matches!(
                ep.correlation,
                schematic_gen::ws_codegen::plan::CorrelationStrategy::None
            ),
            "Samsung remote endpoint {} should NOT have correlation",
            ep.id
        );
    }
}

#[test]
fn samsung_remote_connect_has_connection_params() {
    let plan = lower_to_plan(&define_samsung_smart_tv_remote_ws_api()).unwrap();
    let tokens = generate_ws_client_module(&plan);
    let code = tokens.to_string();

    assert!(
        code.contains("connect_remote_control"),
        "Expected connect_remote_control method"
    );
    assert!(
        code.contains("RemoteControlConnectionParams"),
        "Expected endpoint connection params struct for RemoteControl"
    );
}

// --- Fixture definitions: declared role and heartbeat metadata ---

fn one_way_fixture(role: EndpointRole) -> schematic_define::WebSocketApi {
    use schematic_define::websocket::{MessageDirection, MessageSchema, WebSocketEndpoint};
    use schematic_define::{AuthStrategy, Schema};
    schematic_define::WebSocketApi {
        name: "RelayWs".to_string(),
        description: "Relay fixture".to_string(),
        base_url: "ws://127.0.0.1".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_mapping: None,
        headers: vec![],
        version: None,
        endpoints: vec![WebSocketEndpoint {
            id: "Relay".to_string(),
            path: "/relay".to_string(),
            description: "Relay channel".to_string(),
            role,
            messages: vec![
                MessageSchema::new("Command", MessageDirection::Client, Schema::new("serde_json::Value")),
                MessageSchema::new("Update", MessageDirection::Server, Schema::new("serde_json::Value")),
            ],
            ..WebSocketEndpoint::default()
        }],
        runtime: None,
    }
}

#[test]
fn a_both_role_with_one_way_messages_keeps_its_host_and_client() {
    use schematic_gen::output::ws_modules::render_ws_api_module;
    let both = render_ws_api_module(&one_way_fixture(EndpointRole::Both)).expect("renders");
    assert!(both.contains("pub struct RelayWsHost"), "{both}");
    assert!(both.contains("pub struct RelayClient"), "{both}");

    let client = render_ws_api_module(&one_way_fixture(EndpointRole::Client)).expect("renders");
    assert!(!client.contains("RelayWsHost"), "a client role generates no host");
    assert!(client.contains("pub struct RelayClient"));
}

#[test]
fn a_declared_heartbeat_is_documented_on_the_client_without_a_warning() {
    use schematic_define::Schema;
    use schematic_define::websocket::{HeartbeatHints, MessageDirection, MessageSchema, WebSocketEndpointHints};
    use schematic_gen::output::ws_modules::{render_ws_api_module, validate_ws_apis};

    let mut api = one_way_fixture(EndpointRole::Client);
    api.endpoints[0].lifecycle.keepalive = Some(MessageSchema::new(
        "Heartbeat",
        MessageDirection::Client,
        Schema::new("serde_json::Value"),
    ));
    api.endpoints[0].runtime = Some(WebSocketEndpointHints {
        heartbeat: Some(HeartbeatHints { interval_secs: 20, timeout_secs: Some(5) }),
        ..WebSocketEndpointHints::default()
    });

    assert!(validate_ws_apis(std::slice::from_ref(&api)).diagnostics.is_empty());
    let module = render_ws_api_module(&api).expect("renders");
    let doc = "/// The server expects the `Heartbeat` keepalive message (`send_heartbeat`) every \
               20 s, and expects the reply within 5 s. This client does not send it: schedule \
               it from the caller.";
    assert!(module.contains("/// ## Heartbeat"), "{module}");
    assert!(module.contains(doc), "{module}");

    // Without a keepalive message the interval is still stated.
    api.endpoints[0].lifecycle.keepalive = None;
    api.endpoints[0].runtime.as_mut().unwrap().heartbeat.as_mut().unwrap().timeout_secs = None;
    let module = render_ws_api_module(&api).expect("renders");
    assert!(
        module.contains("/// The server expects an application heartbeat message every 20 s. This"),
        "{module}"
    );
}
