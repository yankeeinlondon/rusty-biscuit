//! AsyncAPI import of a real document builds the shared-vocabulary
//! WebSocket definition identically from its YAML and JSON forms.
//!
//! The JSON fixture is the official Integration AsyncAPI converted from the
//! YAML in `specs/unfolded-circle/` without edits.

use std::path::Path;

use schematic_define::websocket::{EndpointRole, MessageDirection, WebSocketApi};
use schematic_define::{EndpointParams, RestApi};
use schematic_gen::asyncapi_import::{AsyncImportOptions, WsRole, run_import_asyncapi};
use schematic_gen::ws_codegen::plan::lower_to_plan;

fn import(input: &Path, role: WsRole) -> WebSocketApi {
    let output = tempfile::TempDir::new().unwrap();
    let options = AsyncImportOptions {
        input: input.to_string_lossy().to_string(),
        api_name: Some("ImportedIntegrationWs".to_string()),
        module_path: Some("imported_integration_ws".to_string()),
        ws_role: role,
        output: output.path().to_string_lossy().to_string(),
        dry_run: true,
        strict: false,
        verbose: false,
    };
    run_import_asyncapi(&options).expect("the official document imports").ws_api
}

fn assert_imported_shape(api: &WebSocketApi, role: EndpointRole) {
    assert_eq!(api.endpoints.len(), 1);
    let endpoint = &api.endpoints[0];
    assert_eq!(endpoint.path, "/intg");
    assert_eq!(endpoint.role, role);
    assert_eq!(endpoint.params, EndpointParams::default());
    assert!(!endpoint.messages.is_empty());
    assert!(endpoint.messages.iter().all(|m| m.responds_with.is_none() && m.selectors.is_none() && !m.fallback));
    assert_eq!(api.env_mapping, Some(RestApi::legacy_env_mapping_for(&api.auth, &api.env_auth)));
    assert!(api.headers.is_empty() && api.auth_policy.is_none());
}

#[test]
fn yaml_and_json_forms_import_the_same_definition_for_every_cli_role() {
    let yaml = biscuit_test_harness::manifest_dir!().join("../specs/unfolded-circle/UCR-integration-asyncapi.yaml");
    let json = biscuit_test_harness::manifest_dir!().join("tests/fixtures/asyncapi/UCR-integration-asyncapi.json");
    for (cli_role, role) in [
        (WsRole::Client, EndpointRole::Client),
        (WsRole::Server, EndpointRole::Host),
        (WsRole::Both, EndpointRole::Both),
    ] {
        let from_yaml = import(&yaml, cli_role);
        let from_json = import(&json, cli_role);
        assert_eq!(from_yaml, from_json, "{cli_role:?}");
        assert_imported_shape(&from_yaml, role);
    }
}

#[test]
fn cli_role_keeps_its_direction_mapping() {
    let yaml = biscuit_test_harness::manifest_dir!().join("../specs/unfolded-circle/UCR-integration-asyncapi.yaml");
    let directions = |role| -> Vec<MessageDirection> {
        import(&yaml, role).endpoints[0].messages.iter().map(|m| m.direction).collect()
    };
    let client = directions(WsRole::Client);
    let server = directions(WsRole::Server);
    assert!(client.contains(&MessageDirection::Client) && client.contains(&MessageDirection::Server));
    let inverted: Vec<MessageDirection> = client
        .iter()
        .map(|d| match d {
            MessageDirection::Client => MessageDirection::Server,
            MessageDirection::Server => MessageDirection::Client,
            other => *other,
        })
        .collect();
    assert_eq!(server, inverted, "`server` reads the document from the server's side");
    assert!(directions(WsRole::Both).iter().all(|d| *d == MessageDirection::Bidirectional));
}

#[test]
fn imported_definitions_lower_with_their_declared_role() {
    let json = biscuit_test_harness::manifest_dir!().join("tests/fixtures/asyncapi/UCR-integration-asyncapi.json");
    for (cli_role, role) in [(WsRole::Client, EndpointRole::Client), (WsRole::Both, EndpointRole::Both)] {
        let plan = lower_to_plan(&import(&json, cli_role)).expect("the import lowers");
        assert_eq!(plan.endpoints[0].role, role);
    }
}
