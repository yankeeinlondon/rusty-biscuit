//! AsyncAPI 3.0 export of WebSocket definitions (R25, R56–R58): structure,
//! local reference closure, determinism, document-schema validation, file
//! naming, and the definitions the exporter refuses.

use schematic_define::websocket::{MessageDirection, WebSocketApi};
use schematic_define::{AuthStrategy, DeviceCodeGrant, OAuth2ClientAuthMethod, OAuth2Config, ParamDef, QueryParamType};
use schematic_definitions::registry::{ApiRegistration, Protocol, WebSocketRegistration, registrations};
use schematic_gen::diagnostics::{Output, Severity};
use schematic_gen::errors::GeneratorError;
use schematic_gen::export::asyncapi::{
    ASYNCAPI_VERSION, AsyncApiDocument, asyncapi_documents, dangling_refs, export_asyncapi, render_asyncapi,
};
use serde_json::{Value, json};

use crate::common::asyncapi::{fixture_api, message};

fn documents() -> Vec<AsyncApiDocument> {
    asyncapi_documents(registrations())
}

/// Every WebSocket API exports one closed document whose channels, component
/// messages, and operations account for every endpoint and message, with
/// nothing of the 2.x structure left in it.
#[test]
fn every_websocket_api_exports_a_closed_document() {
    let documents = documents();
    assert_eq!(documents.len(), 5);
    for AsyncApiDocument { file_name, api } in &documents {
        let doc = export_asyncapi(api).unwrap_or_else(|e| panic!("{file_name}: {e}")).document;
        assert_eq!(doc["asyncapi"], ASYNCAPI_VERSION, "{file_name}");
        assert!(dangling_refs(&doc).is_empty(), "{file_name}: {:?}", dangling_refs(&doc));
        assert_eq!(doc["info"]["title"], json!(api.name));
        let server = &doc["servers"]["default"];
        assert!(server.get("url").is_none() && server["host"].is_string(), "{file_name}: {server}");
        assert_eq!(format!("{}://{}", server["protocol"].as_str().unwrap(), server["host"].as_str().unwrap()), api.base_url);

        let channels = doc["channels"].as_object().unwrap();
        assert_eq!(channels.len(), api.endpoints.len(), "{file_name}");
        let mut expected_operations = 0;
        for endpoint in &api.endpoints {
            let channel = &channels[&endpoint.id];
            assert_eq!(channel["address"], json!(endpoint.path));
            assert!(channel.get("publish").is_none() && channel.get("subscribe").is_none());
            assert_eq!(channel["x-schematic-role"], json!(endpoint.role.to_string()));
            for message in schematic_gen::export::asyncapi::exported_messages(endpoint) {
                assert!(channel["messages"].get(&message.name).is_some(), "{file_name}: {}", message.name);
                expected_operations += if message.direction == MessageDirection::Bidirectional { 2 } else { 1 };
            }
        }
        let operations = doc["operations"].as_object().unwrap();
        assert_eq!(operations.len(), expected_operations, "{file_name}");
        assert!(operations.values().all(|op| op["action"] == "send" || op["action"] == "receive"));
    }
}

#[test]
fn export_is_deterministic() {
    for document in documents() {
        assert_eq!(render_asyncapi(&document.api).unwrap(), render_asyncapi(&document.api).unwrap());
    }
    let fixture = fixture_api();
    let rendered = render_asyncapi(&fixture).unwrap();
    assert!(rendered.ends_with("}\n"));
    assert_eq!(rendered, render_asyncapi(&fixture).unwrap());
}

/// The vendored AsyncAPI 3.0.0 JSON Schema accepts every exported document
/// and rejects a 2.x-shaped one, so the check is load-bearing.
#[test]
fn exported_documents_validate_against_the_asyncapi_3_schema() {
    let schema: Value = serde_json::from_str(include_str!("../fixtures/asyncapi/spec-json-schemas/3.0.0.json")).unwrap();
    let validator = jsonschema::options()
        .with_draft(jsonschema::Draft::Draft7)
        .build(&schema)
        .expect("the vendored schema compiles offline");
    let mut apis: Vec<WebSocketApi> = documents().into_iter().map(|document| document.api).collect();
    apis.push(fixture_api());
    for api in &apis {
        let doc = export_asyncapi(api).unwrap().document;
        let errors: Vec<String> = validator.iter_errors(&doc).map(|e| format!("{} at {}", e, e.instance_path())).collect();
        assert!(errors.is_empty(), "{}: {errors:#?}", api.name);
    }

    let mut legacy = export_asyncapi(&fixture_api()).unwrap().document;
    legacy["channels"]["Feed"]["publish"] = json!({"message": {"payload": {}}});
    legacy["operations"]["Feed.send.Subscribe"]["action"] = json!("publish");
    assert!(!validator.is_valid(&legacy), "2.x structure under a 3.0 version must fail");
}

/// Selectors, the fallback marker, the role, and heartbeat hints (R56–R58)
/// travel as Schematic extensions on the message and channel.
#[test]
fn schematic_metadata_is_written_as_extensions() {
    let doc = export_asyncapi(&fixture_api()).unwrap().document;
    let messages = &doc["components"]["messages"];
    assert_eq!(
        messages["Feed.Update"]["x-schematic-selectors"],
        json!([{"field": "type", "value": "event"}, {"field": "msg", "value": "update"}])
    );
    assert_eq!(messages["Feed.Envelope"]["x-schematic-fallback"], json!(true));
    assert!(messages["Feed.Update"].get("x-schematic-fallback").is_none());
    assert!(messages["Host.Event"].get("x-schematic-selectors").is_none(), "no selectors declared, none written");
    assert_eq!(messages["Both.Data"]["x-schematic-selectors"], json!([]));
    // Overlapping payloads reference one schema component.
    assert_eq!(messages["Feed.Update"]["payload"], messages["Feed.Delete"]["payload"]);

    let channels = &doc["channels"];
    assert_eq!(channels["Feed"]["x-schematic-heartbeat"], json!({"interval_secs": 30, "timeout_secs": 10}));
    assert_eq!(channels["Host"]["x-schematic-heartbeat"], json!({"interval_secs": 15}));
    assert!(channels["Both"].get("x-schematic-heartbeat").is_none());
    assert_eq!(channels["Host"]["x-schematic-role"], json!("host"));
    assert_eq!(channels["Both"]["x-schematic-role"], json!("both"));
}

/// The document describes the client application for every role: client
/// messages are `send`, server messages `receive`, bidirectional both, and
/// `responds_with` is the standard `reply`.
#[test]
fn operations_take_the_client_perspective_for_every_role() {
    let doc = export_asyncapi(&fixture_api()).unwrap().document;
    let operations = &doc["operations"];
    assert_eq!(operations["Host.send.Command"]["action"], "send", "a host endpoint is still described from the client");
    assert_eq!(operations["Host.receive.Event"]["action"], "receive");
    assert_eq!(operations["Feed.send.Ping"]["action"], "send");
    assert_eq!(operations["Feed.receive.Ping"]["action"], "receive");
    assert!(operations.get("Feed.send.Update").is_none());
    assert_eq!(
        operations["Feed.send.Subscribe"]["reply"]["messages"],
        json!([{"$ref": "#/channels/Feed/messages/Subscribed"}])
    );
    assert_eq!(operations["Feed.send.Subscribe"]["messages"], json!([{"$ref": "#/channels/Feed/messages/Subscribe"}]));
}

#[test]
fn server_auth_and_parameters_use_standard_fields() {
    let doc = export_asyncapi(&fixture_api()).unwrap().document;
    assert_eq!(
        doc["servers"]["default"],
        json!({
            "host": "stream.example.com:8443",
            "pathname": "/api/v2",
            "protocol": "wss",
            "security": [{"$ref": "#/components/securitySchemes/apiKey"}],
        })
    );
    assert_eq!(
        doc["components"]["securitySchemes"]["apiKey"],
        json!({"type": "httpApiKey", "in": "header", "name": "X-Key", "x-schematic-value-prefix": "Token "})
    );
    let feed = &doc["channels"]["Feed"];
    assert_eq!(
        feed["parameters"]["room"],
        json!({"description": "Room number", "x-schematic-param": {"type": "integer"}})
    );
    // An undeclared placeholder is an empty parameter object.
    assert_eq!(doc["channels"]["Both"]["parameters"], json!({"session": {}}));
    let query = &feed["bindings"]["ws"]["query"];
    assert_eq!(query["required"], json!(["limit"]));
    assert_eq!(query["properties"]["tags"], json!({"type": "array", "items": {"type": "string"}, "x-schematic-param": {"explode": true}}));
    assert_eq!(query["properties"]["mode"], json!({"type": "string", "enum": ["live", "replay"]}));
    assert_eq!(query["properties"]["filter"], json!({}));
    assert_eq!(query["properties"]["verbose"], json!({"type": "boolean", "deprecated": true}));
    assert_eq!(
        query["properties"]["token"],
        json!({"type": "string", "x-schematic-param": {"rust_name": "auth_token", "secret": true}})
    );
    assert_eq!(
        feed["bindings"]["ws"]["headers"],
        json!({
            "type": "object",
            "properties": {"X-Client": {"type": "string", "const": "schematic"}, "X-Protocol": {"type": "string", "const": "2"}},
            "required": ["X-Protocol", "X-Client"],
        })
    );
    assert!(doc["channels"]["Host"].get("bindings").is_some(), "constant headers bind every channel");
}

/// A WebSocket API alone in its module is `{module}.json`; APIs sharing a
/// module each get `{module}.{api}.json`, so none overwrites another.
#[test]
fn documents_are_named_by_module_and_split_when_a_module_is_shared() {
    let names: Vec<String> = documents().into_iter().map(|document| document.file_name).collect();
    assert_eq!(
        names,
        [
            "elevenlabs_ws.json",
            "unfolded_circle_core_ws.json",
            "unfolded_circle_dock_ws.json",
            "unfolded_circle_integration_ws.json",
            "samsung_smart_tv_remote_ws.json",
        ]
    );

    fn registration(name: &'static str, module: &'static str) -> ApiRegistration {
        ApiRegistration {
            name,
            module,
            definitions_path: "fixture",
            protocol: Protocol::WebSocket(WebSocketRegistration { define: fixture_api, constructor: "fixture_api", summary: "fixture" }),
        }
    }
    let shared = [registration("StreamOne", "shared_ws"), registration("StreamTwo", "shared_ws"), registration("Solo", "solo_ws")];
    let names: Vec<String> = asyncapi_documents(&shared).into_iter().map(|document| document.file_name).collect();
    assert_eq!(names, ["shared_ws.stream_one.json", "shared_ws.stream_two.json", "solo_ws.json"]);
}

fn validation_error(api: &WebSocketApi) -> String {
    match export_asyncapi(api) {
        Err(GeneratorError::Validation { output, diagnostics }) => {
            assert_eq!(output, Output::AsyncApi);
            assert!(diagnostics.iter().any(|d| d.severity == Severity::Error));
            diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
        }
        other => panic!("expected a validation error, got {other:?}"),
    }
}

#[test]
fn definitions_that_cannot_become_a_valid_document_fail() {
    let mut http = fixture_api();
    http.base_url = "https://stream.example.com".into();
    assert!(validation_error(&http).contains("is not a `ws://` or `wss://` URL"));

    let mut duplicate = fixture_api();
    let copy = duplicate.endpoints[0].messages[0].clone();
    duplicate.endpoints[0].messages.push(copy);
    assert!(validation_error(&duplicate).contains("two messages share the name `Subscribe`"));

    let mut reply = fixture_api();
    reply.endpoints[0].messages[0].responds_with = Some("Missing".into());
    assert!(validation_error(&reply).contains("responds with `Missing`"));

    let mut key = fixture_api();
    key.endpoints[1].messages.push(message("Bad Name", MessageDirection::Server, "HostEvent"));
    assert!(validation_error(&key).contains("message name `Bad Name` cannot be an AsyncAPI key"));
}

/// What the subset cannot carry is reported as a warning, never dropped
/// silently.
#[test]
fn unexportable_declarations_are_reported() {
    let mut api = fixture_api();
    api.endpoints[1].params.header.push(ParamDef::new("X-Trace", QueryParamType::String));
    api.endpoints[1].params.cookie.push(ParamDef::new("session", QueryParamType::String));
    api.auth = AuthStrategy::OAuth2(OAuth2Config::DeviceCode(DeviceCodeGrant {
        device_authorization_url: "https://auth.example.com/device".into(),
        token_url: "https://auth.example.com/token".into(),
        revocation_url: None,
        default_scopes: vec![],
        client_auth: OAuth2ClientAuthMethod::None,
    }));
    let export = export_asyncapi(&api).unwrap();
    let messages: Vec<String> = export.diagnostics.iter().map(ToString::to_string).collect();
    assert!(export.diagnostics.iter().all(|d| d.severity == Severity::Warning && d.output == Output::AsyncApi));
    assert!(messages.iter().any(|m| m.contains("header parameter `X-Trace` is not exported")), "{messages:#?}");
    assert!(messages.iter().any(|m| m.contains("cookie parameter `session` is not exported")), "{messages:#?}");
    assert!(messages.iter().any(|m| m.contains("auth `oauth2` has no AsyncAPI form")), "{messages:#?}");
    assert!(export.document["components"].get("securitySchemes").is_none());
    assert!(export.document["servers"]["default"].get("security").is_none());
}
