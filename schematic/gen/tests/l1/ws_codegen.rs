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
fn all_clients_select_one_handshake_credential_through_the_runtime() {
    for (name, plan) in lower_all_apis() {
        let code = generate_ws_client_module(&plan).to_string();
        for needle in [
            "fn auth_config () -> schematic_runtime :: AuthConfig",
            "fn credentials (mut self , credential : schematic_runtime :: Credential)",
            "fn header (mut self",
            "select_handshake (options . credential . as_ref () , self . credential . as_ref ())",
            "WsHandshake :: new",
            "fn auth_method (& self) -> & schematic_runtime :: AuthMethod",
        ] {
            assert!(code.contains(needle), "{name} client lacks `{needle}`");
        }
        assert!(!code.contains("schematic_define :: Headers"), "{name} still builds schematic_define::Headers");
    }
}

#[test]
fn the_handshake_surfaces_unparsable_headers_instead_of_dropping_them() {
    let code = generate_ws_shared_module().to_string();
    assert!(
        code.contains("WsError :: InvalidHeader"),
        "the handshake should map a header parse failure to WsError::InvalidHeader"
    );
    // The old behavior tucked both parses into an `if let (Ok(_), Ok(_))`
    // and silently skipped the header on failure.
    assert!(
        !code.contains("if let (Ok (hdr_name) , Ok (hdr_value))"),
        "the handshake must not silently drop unparsable headers"
    );
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
        code.contains("request_timeout : options . request_timeout"),
        "Expected request timeout to come from options"
    );
    assert!(
        !code.contains("request_timeout : std :: time :: Duration :: from_secs"),
        "Should not hardcode correlated request timeout"
    );
}

#[test]
fn the_handshake_uses_transport_config_and_disable_nagle() {
    let code = generate_ws_shared_module().to_string();

    assert!(
        code.contains("connect_async_with_config"),
        "Expected connect_async_with_config usage"
    );
    assert!(
        code.contains("options . websocket_config ,"),
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

// --- Typed messages in the shipped modules ---

/// The formatted source of one shipped WebSocket module, as the generator
/// writes it to `schematic/schema/src/<module>.rs`.
fn shipped_module(module: &str) -> String {
    let file = format!("{module}.rs");
    schematic_gen::output::ws_modules::generate_ws_definition_modules()
        .expect("the shipped definitions generate")
        .into_iter()
        .find(|(name, _)| *name == file)
        .unwrap_or_else(|| panic!("no generated module {file}"))
        .1
}

fn parse(source: &str) -> syn::File {
    syn::parse_file(source).expect("generated modules parse")
}

/// The variant names of `pub enum <name>`, in declaration order.
fn enum_variants(file: &syn::File, name: &str) -> Vec<String> {
    file.items
        .iter()
        .find_map(|item| match item {
            syn::Item::Enum(item) if item.ident == name => {
                Some(item.variants.iter().map(|variant| variant.ident.to_string()).collect())
            }
            _ => None,
        })
        .unwrap_or_else(|| panic!("no enum {name}"))
}

/// Every method of every inherent `impl <self_ty>` block.
fn inherent_methods<'a>(file: &'a syn::File, self_ty: &str) -> Vec<&'a syn::ImplItemFn> {
    file.items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Impl(item)
                if item.trait_.is_none() && quote::ToTokens::to_token_stream(&item.self_ty).to_string() == self_ty =>
            {
                Some(item)
            }
            _ => None,
        })
        .flat_map(|item| item.items.iter())
        .filter_map(|item| match item {
            syn::ImplItem::Fn(method) => Some(method),
            _ => None,
        })
        .collect()
}

/// Whether `impl <trait_name> for <self_ty>` exists.
fn implements(file: &syn::File, trait_name: &str, self_ty: &str) -> bool {
    file.items.iter().any(|item| match item {
        syn::Item::Impl(item) => {
            item.trait_.as_ref().is_some_and(|(_, path, _)| quote::ToTokens::to_token_stream(path).to_string() == trait_name)
                && quote::ToTokens::to_token_stream(&item.self_ty).to_string() == self_ty
        }
        _ => false,
    })
}

/// The argument types after `&self`, and the return type, of a signature.
fn signature_types(signature: &syn::Signature) -> (Vec<String>, String) {
    let arguments = signature
        .inputs
        .iter()
        .filter_map(|input| match input {
            syn::FnArg::Typed(argument) => Some(quote::ToTokens::to_token_stream(&argument.ty).to_string()),
            syn::FnArg::Receiver(_) => None,
        })
        .collect();
    let output = quote::ToTokens::to_token_stream(&signature.output).to_string();
    (arguments, output)
}

#[test]
fn core_message_enums_name_one_variant_per_declared_message() {
    let file = parse(&shipped_module("unfolded_circle_core_ws"));
    assert_eq!(
        enum_variants(&file, "CoreWsServerMessage"),
        ["ResponseEnvelope", "EventEnvelope", "AuthRequired", "Authentication", "Pong", "VersionInfo", "SystemInfo", "Result"]
    );
    assert_eq!(
        enum_variants(&file, "CoreWsClientMessage"),
        ["RequestEnvelope", "Auth", "Ping", "Version", "System"]
    );
    // A client sends client messages and receives server messages.
    assert!(implements(&file, "serde :: Serialize", "CoreWsClientMessage"));
    assert!(!implements(&file, "serde :: Serialize", "CoreWsServerMessage"));
    assert!(inherent_methods(&file, "CoreWsServerMessage").iter().any(|method| method.sig.ident == "decode"));
    assert!(inherent_methods(&file, "CoreWsClientMessage").is_empty(), "a client never decodes its own messages");
}

#[test]
fn a_bidirectional_integration_message_is_a_variant_of_both_directions() {
    let file = parse(&shipped_module("unfolded_circle_integration_ws"));
    let server = enum_variants(&file, "IntegrationServerMessage");
    let client = enum_variants(&file, "IntegrationClientMessage");
    assert!(server.contains(&"EventEnvelope".to_string()), "{server:?}");
    assert!(client.contains(&"EventEnvelope".to_string()), "{client:?}");
    // Role `Both`: each direction is sent by one side and received by the
    // other, so both enums serialize and both decode.
    for name in ["IntegrationServerMessage", "IntegrationClientMessage"] {
        assert!(implements(&file, "serde :: Serialize", name), "{name} must serialize");
        assert!(inherent_methods(&file, name).iter().any(|method| method.sig.ident == "decode"), "{name} must decode");
    }
}

#[test]
fn elevenlabs_client_enums_hold_the_declared_client_messages() {
    let file = parse(&shipped_module("elevenlabs_ws"));
    assert_eq!(enum_variants(&file, "TextToSpeechClientMessage"), ["TextChunk"]);
    assert_eq!(enum_variants(&file, "MultiContextTextToSpeechClientMessage"), ["TextChunk", "CloseContext"]);
    assert_eq!(enum_variants(&file, "TextToSpeechServerMessage"), ["AudioChunk"]);
    assert_eq!(enum_variants(&file, "MultiContextTextToSpeechServerMessage"), ["AudioChunk"]);
}

#[test]
fn the_integration_host_is_named_for_its_api_and_sends_no_protocol_envelope() {
    let module = shipped_module("unfolded_circle_integration_ws");
    assert!(module.contains("pub struct UnfoldedCircleIntegrationEventHub"), "{module}");
    assert!(module.contains("pub struct UnfoldedCircleIntegrationWsHost"), "{module}");
    assert!(!module.contains("UnfoldedCircleEventHub"), "the hub is named for the API, not the vendor");
    assert!(!module.contains("authentication_success_response"), "no hard-coded success envelope");
}

#[test]
fn the_integration_handler_takes_and_returns_typed_messages() {
    let file = parse(&shipped_module("unfolded_circle_integration_ws"));
    let handler = file
        .items
        .iter()
        .find_map(|item| match item {
            syn::Item::Trait(item) if item.ident == "WsHandler" => Some(item),
            _ => None,
        })
        .expect("the host declares WsHandler");
    let method = |name: &str| {
        handler
            .items
            .iter()
            .find_map(|item| match item {
                syn::TraitItem::Fn(method) if method.sig.ident == name => Some(signature_types(&method.sig)),
                _ => None,
            })
            .unwrap_or_else(|| panic!("WsHandler has no {name}"))
    };
    let future_of = |output: &str| format!("-> impl std :: future :: Future < Output = {output} > + Send");

    assert_eq!(
        method("handle_message"),
        (
            vec!["IntegrationClientMessage".to_string(), "WsConnectionContext".to_string()],
            future_of("Option < IntegrationServerMessage >")
        )
    );
    assert_eq!(
        method("on_authenticated"),
        (vec!["WsConnectionContext".to_string()], future_of("Option < IntegrationServerMessage >"))
    );
    assert_eq!(
        method("on_error"),
        (
            vec!["super :: ws_shared :: WsError".to_string(), "WsConnectionContext".to_string()],
            future_of("()")
        )
    );
}

#[test]
fn every_shipped_client_sends_its_typed_enum_and_values_only_through_send_value() {
    let mut clients = 0;
    for (module, source) in schematic_gen::output::ws_modules::generate_ws_definition_modules().expect("generates") {
        let file = parse(&source);
        let client_types: Vec<String> = file
            .items
            .iter()
            .filter_map(|item| match item {
                syn::Item::Struct(item) if item.ident.to_string().ends_with("Client") => Some(item.ident.to_string()),
                _ => None,
            })
            .collect();
        for client in client_types {
            clients += 1;
            let endpoint = client.strip_suffix("Client").unwrap();
            let methods = inherent_methods(&file, &client);
            let find = |name: &str| methods.iter().find(|method| method.sig.ident == name).map(|method| signature_types(&method.sig).0);
            assert!(find("send_typed").is_none(), "{module}: {client} exposes send_typed");
            assert_eq!(find("send"), Some(vec![format!("{endpoint}ClientMessage")]), "{module}: {client}::send");
            assert_eq!(find("send_value"), Some(vec!["serde_json :: Value".to_string()]), "{module}: {client}::send_value");
        }
    }
    // ElevenLabs 2, Core 4, Dock 1, Integration 1, Samsung 2.
    assert_eq!(clients, 10, "every shipped endpoint client was checked");
}

/// The order of `selected_message`'s checks: each selected message with
/// its selector count, then the fallback with `None`.
fn decode_order(module: &str, enum_name: &str) -> Vec<(String, Option<usize>)> {
    let header = format!("impl {enum_name} {{");
    let body = module
        .split(&header)
        .nth(1)
        .unwrap_or_else(|| panic!("no impl {enum_name}"))
        .split("pub fn decode(")
        .next()
        .unwrap();
    let mut order = Vec::new();
    let mut pending = None;
    for line in body.lines().map(str::trim) {
        if let Some(selectors) = line.strip_prefix("if frame.matches(&[") {
            pending = Some(selectors.matches("(\"").count());
        } else if let Some(rest) = line.strip_prefix("return Some(\"") {
            order.push((rest.split('"').next().unwrap().to_string(), pending.take()));
        } else if let Some(rest) = line.strip_prefix("Some(\"") {
            order.push((rest.split('"').next().unwrap().to_string(), None));
        }
    }
    order
}

/// The strict-subset rule as generated: `Pong` (`kind=resp, msg=pong`)
/// is tried before the generic `EventEnvelope` (`kind=event`) and the
/// `ResponseEnvelope` fallback, and every check is at least as specific as
/// the next.
#[test]
fn core_decode_tries_the_most_specific_selectors_before_generic_ones_and_the_fallback() {
    let order = decode_order(&shipped_module("unfolded_circle_core_ws"), "CoreWsServerMessage");
    let position = |variant: &str| {
        order.iter().position(|(name, _)| name == variant).unwrap_or_else(|| panic!("{variant} is not decoded: {order:?}"))
    };
    assert!(position("Pong") < position("EventEnvelope"), "{order:?}");
    assert_eq!(order.last(), Some(&("ResponseEnvelope".to_string(), None)), "the fallback is tried last: {order:?}");
    assert_eq!(order.iter().filter(|(_, count)| count.is_none()).count(), 1, "one fallback: {order:?}");
    let counts: Vec<usize> = order.iter().filter_map(|(_, count)| *count).collect();
    assert!(counts.windows(2).all(|pair| pair[0] >= pair[1]), "most specific first: {order:?}");
    assert_eq!(order.iter().find(|(name, _)| name == "Pong").unwrap().1, Some(2));
    assert_eq!(order.iter().find(|(name, _)| name == "EventEnvelope").unwrap().1, Some(1));
}

// --- Correlation (R11) ---

/// The planner pairs requests only through `responds_with`: no envelope
/// name appears in it as a string literal. Absence is the property, so the
/// source text is searched.
#[test]
fn the_planner_names_no_envelope_message() {
    let source = include_str!("../../src/ws_codegen/plan.rs");
    for literal in ["\"RequestEnvelope\"", "\"ResponseEnvelope\""] {
        assert!(!source.contains(literal), "plan.rs still compares a message name with {literal}");
    }
}

/// The client module of Core with its envelope pair renamed, with the
/// renamed spellings mapped back to the originals.
fn core_client_with_renamed_envelopes() -> String {
    fn rename(name: &str) -> String {
        match name {
            "RequestEnvelope" => "Zqcommand".to_string(),
            "ResponseEnvelope" => "Zqanswer".to_string(),
            other => other.to_string(),
        }
    }
    let mut api = define_unfolded_circle_core_ws_api();
    for endpoint in &mut api.endpoints {
        for message in &mut endpoint.messages {
            message.name = rename(&message.name);
            message.responds_with = message.responds_with.as_deref().map(rename);
        }
    }
    let plan = lower_to_plan(&api).expect("the renamed definition lowers");
    generate_ws_client_module(&plan)
        .to_string()
        .replace("Zqcommand", "RequestEnvelope")
        .replace("zqcommand", "request_envelope")
        .replace("Zqanswer", "ResponseEnvelope")
}

#[test]
fn renaming_the_envelope_messages_changes_no_correlation() {
    let original = generate_ws_client_module(&lower_to_plan(&define_unfolded_circle_core_ws_api()).unwrap()).to_string();
    let renamed = core_client_with_renamed_envelopes();
    assert!(original.contains("fn request_request_envelope"), "control: the pair is generated");
    assert_eq!(renamed, original, "renaming the pair changed generated behavior beyond the names");
}

#[test]
fn envelope_names_alone_correlate_nothing() {
    let mut api = define_unfolded_circle_core_ws_api();
    for endpoint in &mut api.endpoints {
        endpoint.runtime = None;
        for message in &mut endpoint.messages {
            message.responds_with = None;
        }
    }
    let plan = lower_to_plan(&api).expect("the uncorrelated definition lowers");
    for endpoint in &plan.endpoints {
        assert!(
            matches!(endpoint.correlation, schematic_gen::ws_codegen::plan::CorrelationStrategy::None),
            "{} correlates without hints or links",
            endpoint.id
        );
    }
    let code = generate_ws_client_module(&plan).to_string();
    assert!(!code.contains("fn request_"), "an uncorrelated endpoint has no request method");
}

/// Each declared pair is one typed method from the request payload to the
/// response payload, beside `request_value`; ids follow the declared JSON
/// type.
#[test]
fn core_requests_are_typed_per_declared_pair() {
    let file = syn::parse_file(&shipped_module("unfolded_circle_core_ws")).unwrap();
    let methods = inherent_methods(&file, "CoreWsClient");
    let signature = |name: &str| {
        let method = methods.iter().find(|method| method.sig.ident == name).unwrap_or_else(|| panic!("no {name}"));
        let syn::ReturnType::Type(_, output) = &method.sig.output else { panic!("{name} returns nothing") };
        (signature_types(&method.sig).0, quote::ToTokens::to_token_stream(output).to_string())
    };
    assert_eq!(
        signature("request_ping"),
        (
            vec!["CoreWsPingMessage".to_string()],
            "Result < CoreWsPongMessage , super :: ws_shared :: WsError >".to_string()
        )
    );
    assert_eq!(signature("request_value").0, vec!["serde_json :: Value".to_string()]);
    let names: Vec<String> = methods
        .iter()
        .map(|method| method.sig.ident.to_string())
        .filter(|name| name.starts_with("request_"))
        .collect();
    assert_eq!(
        names,
        ["request_request_envelope", "request_auth", "request_ping", "request_version", "request_system", "request_value"]
    );
}
