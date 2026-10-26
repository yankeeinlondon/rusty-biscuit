//! The AsyncAPI reader's input robustness matrix, through the public import.
//!
//! Every cell edits one field of the control document (the exported fixture,
//! which imports cleanly) and checks the outcome at the field's pointer.
//!
//! Default outcomes:
//! - a malformed **standard** field is an ERROR at its pointer and the import
//!   fails, so nothing is ever written;
//! - a malformed **Schematic extension** is a WARN and reads as absent.
//!
//! Each row below that does not say ERROR (for extensions: WARN) is one of
//! the listed exceptions.

use schematic_define::websocket::EndpointRole;
use schematic_define::AuthStrategy;
use schematic_gen::asyncapi_import::{AsyncImportResult, WsRole, import_asyncapi_text};
use schematic_gen::export::asyncapi::export_asyncapi;
use serde_json::{Value, json};

use crate::common::asyncapi::{fixture_api, options};

/// The endpoint `id` of an import (endpoints come back in channel-key order).
fn endpoint<'a>(result: &'a AsyncImportResult, id: &str) -> &'a schematic_define::websocket::WebSocketEndpoint {
    result.ws_api.endpoints.iter().find(|e| e.id == id).unwrap_or_else(|| panic!("no endpoint {id}"))
}

fn control() -> Value {
    export_asyncapi(&fixture_api()).unwrap().document
}

fn import_with(text: &str, role: WsRole, api_name: Option<&str>) -> Result<AsyncImportResult, String> {
    let mut options = options("fixture_stream", role);
    options.api_name = api_name.map(String::from);
    import_asyncapi_text(text, &options).map_err(|error| error.to_string())
}

fn import(doc: &Value) -> Result<AsyncImportResult, String> {
    import_with(&serde_json::to_string(doc).unwrap(), WsRole::Client, None)
}

/// `pointer` set to `replacement`, or removed when it is `None`.
fn edited(pointer: &str, replacement: Option<Value>) -> Value {
    let mut doc = control();
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let key = key.replace("~1", "/").replace("~0", "~");
    match (doc.pointer_mut(parent).unwrap_or_else(|| panic!("no {parent}")), replacement) {
        (Value::Object(map), Some(value)) => {
            map.insert(key, value);
        }
        (Value::Object(map), None) => {
            assert!(map.remove(&key).is_some(), "{pointer} was not present");
        }
        (Value::Array(items), Some(value)) => items[key.parse::<usize>().unwrap()] = value,
        _ => panic!("{pointer}: cannot edit"),
    }
    doc
}

#[track_caller]
fn assert_error(pointer: &str, replacement: Option<Value>, location: &str) {
    match import(&edited(pointer, replacement.clone())) {
        Err(message) => assert!(
            message.contains(&format!("[ERROR] {location}:")),
            "{pointer} = {replacement:?}: expected an error at {location}, got:\n{message}"
        ),
        Ok(_) => panic!("{pointer} = {replacement:?} must fail at {location}"),
    }
}

#[track_caller]
fn imported(pointer: &str, replacement: Option<Value>) -> AsyncImportResult {
    import(&edited(pointer, replacement.clone())).unwrap_or_else(|e| panic!("{pointer} = {replacement:?} must import: {e}"))
}

#[track_caller]
fn assert_diagnostic(result: &AsyncImportResult, warn: bool, location: &str) {
    let found = result.diagnostics.iter().any(|d| d.location == location && if warn { d.is_warn() } else { d.is_info() });
    assert!(found, "expected a {} at {location}: {:#?}", if warn { "WARN" } else { "INFO" }, result.diagnostics);
}

#[track_caller]
fn assert_warned(pointer: &str, replacement: Option<Value>, location: &str) -> AsyncImportResult {
    let result = imported(pointer, replacement);
    assert_diagnostic(&result, true, location);
    result
}

#[test]
fn the_control_document_imports_without_warnings() {
    let result = import(&control()).unwrap();
    assert!(result.diagnostics.iter().all(|d| d.is_info()), "{:#?}", result.diagnostics);
    assert_eq!(result.endpoint_count, 3);
}

// ---------------------------------------------------------------------------
// Document shapes (one loader: JSON when the text starts with `{`, else YAML)
// ---------------------------------------------------------------------------

#[test]
fn duplicate_keys_trailing_content_and_extra_documents_are_rejected() {
    let json = serde_json::to_string(&control()).unwrap();
    let duplicate = json.replacen('{', r#"{"asyncapi":"3.0.0","#, 1);
    assert!(import_with(&duplicate, WsRole::Client, None).unwrap_err().contains("Duplicate key asyncapi"));
    assert!(import_with(&format!("{json} trailing"), WsRole::Client, None).unwrap_err().contains("Failed to parse JSON"));

    let yaml = biscuit_file::serde_yaml_ng::to_string(&control()).unwrap();
    let duplicate = format!("info:\n  title: Twice\n  version: '1'\n{yaml}");
    assert!(import_with(&duplicate, WsRole::Client, None).unwrap_err().contains("Duplicate key info"));
    let two_documents = format!("{yaml}---\nasyncapi: 3.0.0\n");
    assert!(import_with(&two_documents, WsRole::Client, None).unwrap_err().contains("Failed to parse YAML"));
    assert!(import_with("[]", WsRole::Client, None).unwrap_err().contains("[ERROR] #:"));
}

#[test]
fn yaml_bare_key_is_null_not_absent() {
    let yaml = biscuit_file::serde_yaml_ng::to_string(&control()).unwrap();
    let null_role = yaml.replacen("x-schematic-role: host", "x-schematic-role:", 1);
    assert_ne!(null_role, yaml);
    let result = import_with(&null_role, WsRole::Client, None).unwrap();
    assert_diagnostic(&result, true, "#/channels/Host/x-schematic-role");
    // Absent would read silently as the default.
    let absent = yaml.replacen("    x-schematic-role: host\n", "", 1);
    assert_ne!(absent, yaml);
    let quiet = import_with(&absent, WsRole::Client, None).unwrap();
    assert!(quiet.diagnostics.iter().all(|d| d.location != "#/channels/Host/x-schematic-role"));
}

// ---------------------------------------------------------------------------
// Standard fields: ERROR unless listed
// ---------------------------------------------------------------------------

#[test]
fn version_field() {
    for replacement in [None, Some(Value::Null), Some(json!(3)), Some(json!("")), Some(json!("3.1.0")), Some(json!("1.2.0"))] {
        assert_error("/asyncapi", replacement, "#/asyncapi");
    }
}

#[test]
fn info_fields() {
    assert_error("/info", None, "#/info");
    assert_error("/info", Some(json!([])), "#/info");
    for replacement in [None, Some(Value::Null), Some(json!("")), Some(json!(5))] {
        assert_error("/info/title", replacement.clone(), "#/info/title");
        assert_error("/info/version", replacement, "#/info/version");
    }
    // Exception: `--api-name` stands in for an absent title.
    let doc = edited("/info/title", None);
    let named = import_with(&doc.to_string(), WsRole::Client, Some("Named")).unwrap();
    assert_eq!(named.ws_api.name, "Named");
    // Exception: an absent description is empty.
    assert_eq!(imported("/info/description", None).ws_api.description, "");
    assert_error("/info/description", Some(json!(5)), "#/info/description");
    assert_eq!(imported("/info/externalDocs", None).ws_api.docs_url, None);
    assert_error("/info/externalDocs", Some(json!("x")), "#/info/externalDocs");
    assert_error("/info/externalDocs/url", Some(json!("")), "#/info/externalDocs/url");
    assert_error("/info/externalDocs/url", None, "#/info/externalDocs/url");
}

#[test]
fn server_fields() {
    // Exceptions: no server is a WARN and the localhost default.
    let none = assert_warned("/servers", None, "#/servers");
    assert_eq!(none.ws_api.base_url, "ws://localhost");
    assert_warned("/servers", Some(json!({})), "#/servers");
    assert_error("/servers", Some(json!([])), "#/servers");
    assert_error("/servers/default", Some(json!("x")), "#/servers/default");
    for replacement in [None, Some(json!("")), Some(json!(5))] {
        assert_error("/servers/default/host", replacement, "#/servers/default/host");
    }
    for replacement in [None, Some(json!("http")), Some(json!(""))] {
        assert_error("/servers/default/protocol", replacement, "#/servers/default/protocol");
    }
    assert_error("/servers/default/pathname", Some(json!("api")), "#/servers/default/pathname");
    assert_error("/servers/default/pathname", Some(json!(5)), "#/servers/default/pathname");
    assert_eq!(imported("/servers/default/pathname", None).ws_api.base_url, "wss://stream.example.com:8443");
}

#[test]
fn applied_security_decides_auth() {
    // Exceptions: no applied security, or an empty list, is anonymous even
    // though a component scheme is declared.
    let unapplied = imported("/servers/default/security", None);
    assert_eq!(unapplied.ws_api.auth, AuthStrategy::None);
    assert_diagnostic(&unapplied, false, "#/servers/default/security");
    assert_eq!(imported("/servers/default/security", Some(json!([]))).ws_api.auth, AuthStrategy::None);
    // An inline scheme object applies as well as a reference.
    let inline = imported("/servers/default/security", Some(json!([{"type": "http", "scheme": "bearer"}])));
    assert_eq!(inline.ws_api.auth, AuthStrategy::BearerToken { header: None });

    assert_error("/servers/default/security", Some(json!("apiKey")), "#/servers/default/security");
    assert_error("/servers/default/security/0", Some(json!({"$ref": "#/components/securitySchemes/missing"})), "#/servers/default/security/0/$ref");
    assert_error("/servers/default/security/0", Some(json!({"$ref": "https://example.com/s.json"})), "#/servers/default/security/0/$ref");
    let scheme = "/components/securitySchemes/apiKey";
    assert_error(&format!("{scheme}/type"), None, "#/components/securitySchemes/apiKey/type");
    assert_error(&format!("{scheme}/name"), None, "#/components/securitySchemes/apiKey/name");
    assert_error(&format!("{scheme}/name"), Some(json!("")), "#/components/securitySchemes/apiKey/name");
    assert_error(&format!("{scheme}/in"), Some(json!("body")), "#/components/securitySchemes/apiKey/in");
    assert_error(&format!("{scheme}/in"), None, "#/components/securitySchemes/apiKey/in");
    // Exception: a scheme outside the subset is a WARN and anonymous.
    let oauth = assert_warned(scheme, Some(json!({"type": "oauth2", "flows": {}})), "#/components/securitySchemes/apiKey");
    assert_eq!(oauth.ws_api.auth, AuthStrategy::None);
    // Extension: a malformed prefix is ignored.
    let prefix = assert_warned(&format!("{scheme}/x-schematic-value-prefix"), Some(json!(5)), "#/components/securitySchemes/apiKey/x-schematic-value-prefix");
    assert_eq!(prefix.ws_api.auth, AuthStrategy::ApiKey { header: "X-Key".into(), value_prefix: None });
}

#[test]
fn channel_fields() {
    assert_error("/channels", None, "#/channels");
    assert_error("/channels", Some(json!([])), "#/channels");
    // Exception: no channels is a WARN.
    let mut empty = edited("/channels", Some(json!({})));
    empty.as_object_mut().unwrap().remove("operations");
    assert_diagnostic(&import(&empty).unwrap(), true, "#/channels");
    assert_error("/channels/Host", Some(json!("x")), "#/channels/Host");
    for replacement in [None, Some(Value::Null), Some(json!("")), Some(json!(5))] {
        assert_error("/channels/Host/address", replacement, "#/channels/Host/address");
    }
    assert_error("/channels/Host/description", Some(json!(5)), "#/channels/Host/description");
    assert_error("/channels/Host/messages", Some(json!(5)), "#/channels/Host/messages");
    assert_error("/channels/Host/messages/Event", Some(json!({"$ref": "#/components/messages/Missing"})), "#/channels/Host/messages/Event/$ref");
    assert_error("/channels/Host/messages/Event", Some(json!({"$ref": "other.json#/Event"})), "#/channels/Host/messages/Event/$ref");
}

#[test]
fn message_fields() {
    let at = "/components/messages/Host.Event";
    for replacement in [Some(json!("")), Some(json!(5))] {
        assert_error(&format!("{at}/name"), replacement, "#/components/messages/Host.Event/name");
    }
    // Exception: without `name`, the channel's message key names it.
    let unnamed = imported(&format!("{at}/name"), None);
    assert!(endpoint(&unnamed, "Host").messages.iter().any(|m| m.name == "Event"));
    assert_error(&format!("{at}/description"), Some(json!(5)), "#/components/messages/Host.Event/description");
    // Exception: a missing payload is a WARN and `JsonValue`; `{}` is any value.
    let missing = assert_warned(&format!("{at}/payload"), None, "#/components/messages/Host.Event/payload");
    assert!(endpoint(&missing, "Host").messages.iter().any(|m| m.schema.type_name == "JsonValue"));
    let any = imported(&format!("{at}/payload"), Some(json!({})));
    assert!(endpoint(&any, "Host").messages.iter().any(|m| m.schema.type_name == "JsonValue"));
    assert_error(&format!("{at}/payload"), Some(json!("x")), "#/components/messages/Host.Event/payload");
    assert_error(&format!("{at}/payload"), Some(json!({"$ref": "#/components/schemas/Missing"})), "#/components/messages/Host.Event/payload/$ref");
}

#[test]
fn parameter_fields() {
    let feed = "/channels/Feed";
    assert_error(&format!("{feed}/parameters"), Some(json!([])), "#/channels/Feed/parameters");
    assert_error(&format!("{feed}/parameters/room"), Some(json!("x")), "#/channels/Feed/parameters/room");
    assert_error(&format!("{feed}/parameters/room"), Some(json!({"$ref": "#/components/parameters/room"})), "#/channels/Feed/parameters/room");
    // Exception: a parameter without a placeholder is a WARN and dropped.
    assert_warned(&format!("{feed}/parameters/other"), Some(json!({"description": "x"})), "#/channels/Feed/parameters/other");

    let query = format!("{feed}/bindings/ws/query");
    let query_at = "#/channels/Feed/bindings/ws/query";
    assert_error(&format!("{feed}/bindings"), Some(json!("x")), "#/channels/Feed/bindings");
    assert_error(&format!("{feed}/bindings/ws"), Some(json!(5)), "#/channels/Feed/bindings/ws");
    assert_error(&format!("{query}/type"), Some(json!("string")), &format!("{query_at}/type"));
    assert_error(&format!("{query}/properties"), Some(json!([])), &format!("{query_at}/properties"));
    assert_error(&format!("{query}/required"), Some(json!("limit")), &format!("{query_at}/required"));
    assert_error(&format!("{query}/required"), Some(json!([5])), &format!("{query_at}/required/0"));
    let property = format!("{query}/properties/limit");
    let property_at = format!("{query_at}/properties/limit");
    assert_error(&property, Some(json!(5)), &property_at);
    assert_error(&format!("{property}/type"), Some(json!(5)), &format!("{property_at}/type"));
    assert_error(&format!("{property}/type"), Some(json!("date")), &format!("{property_at}/type"));
    assert_error(&format!("{property}/type"), Some(json!("array")), &format!("{property_at}/items"));
    assert_error(&format!("{property}/enum"), Some(json!("x")), &format!("{property_at}/enum"));
    assert_error(&format!("{property}/enum"), Some(json!([1])), &format!("{property_at}/enum/0"));
    assert_error(&format!("{property}/deprecated"), Some(json!("yes")), &format!("{property_at}/deprecated"));
    assert_error(&format!("{property}/description"), Some(json!(5)), &format!("{property_at}/description"));
    // Exception: an object type is any JSON value, with an INFO.
    let object = imported(&format!("{property}/type"), Some(json!("object")));
    assert_diagnostic(&object, false, &format!("{property_at}/type"));

    // Exception: a header that is not a constant cannot be sent: WARN, dropped.
    let headers = format!("{feed}/bindings/ws/headers/properties/X-Client");
    let dropped = assert_warned(&headers, Some(json!({"type": "string"})), "#/channels/Feed/bindings/ws/headers/properties/X-Client");
    assert!(dropped.ws_api.headers.iter().all(|(name, _)| name != "X-Client") || dropped.ws_api.headers.len() == 2);
}

#[test]
fn operation_fields() {
    let op = "/operations/Host.receive.Event";
    let op_at = "#/operations/Host.receive.Event";
    assert_error("/operations", Some(json!([])), "#/operations");
    assert_error(op, Some(json!("x")), op_at);
    for replacement in [None, Some(json!("publish")), Some(json!(""))] {
        assert_error(&format!("{op}/action"), replacement, &format!("{op_at}/action"));
    }
    assert_error(&format!("{op}/channel"), None, &format!("{op_at}/channel"));
    assert_error(&format!("{op}/channel"), Some(json!({"$ref": "#/channels/Missing"})), &format!("{op_at}/channel/$ref"));
    assert_error(&format!("{op}/messages"), Some(json!("x")), &format!("{op_at}/messages"));
    assert_error(&format!("{op}/messages/0"), Some(json!({"$ref": "#/channels/Feed/messages/Update"})), &format!("{op_at}/messages/0/$ref"));
    // Exceptions: an operation without `messages` covers its channel (INFO);
    // an empty list carries none (WARN), so the message is not imported.
    let covering = imported(&format!("{op}/messages"), None);
    assert_diagnostic(&covering, false, &format!("{op_at}/messages"));
    let empty = assert_warned(&format!("{op}/messages"), Some(json!([])), &format!("{op_at}/messages"));
    assert_diagnostic(&empty, true, "#/channels/Host/messages/Event");
    // Exception: no operations leaves every message unused (WARN each).
    let unused = imported("/operations", None);
    assert_eq!(unused.message_count, 0);
    assert_diagnostic(&unused, true, "#/channels/Host/messages/Event");
    // Operation security is not imported (WARN); a reply to a received
    // message is ignored (INFO); a reply naming nothing is a WARN.
    assert_warned(&format!("{op}/security"), Some(json!([])), &format!("{op_at}/security"));
    let reply = imported(&format!("{op}/reply"), Some(json!({"messages": [{"$ref": "#/channels/Host/messages/Command"}]})));
    assert_diagnostic(&reply, false, &format!("{op_at}/reply"));
    assert_warned("/operations/Feed.send.Subscribe/reply/messages", Some(json!([])), "#/operations/Feed.send.Subscribe/reply");
}

// ---------------------------------------------------------------------------
// Schematic extensions: WARN and read as absent unless listed
// ---------------------------------------------------------------------------

#[test]
fn selectors_extension() {
    let at = "/components/messages/Feed.Update/x-schematic-selectors";
    let pointer = "#/components/messages/Feed.Update/x-schematic-selectors";
    let update = |result: &AsyncImportResult| {
        endpoint(result, "Feed").messages.iter().find(|m| m.name == "Update").unwrap().selectors.clone()
    };
    for replacement in [Value::Null, json!("x"), json!(5)] {
        assert_eq!(update(&assert_warned(at, Some(replacement), pointer)), None);
    }
    for (element, location) in [
        (json!(5), format!("{pointer}/0")),
        (json!({"field": "", "value": "a"}), format!("{pointer}/0/field")),
        (json!({"field": 5, "value": "a"}), format!("{pointer}/0/field")),
        (json!({"value": "a"}), format!("{pointer}/0/field")),
        (json!({"field": "type"}), format!("{pointer}/0/value")),
        (json!({"field": "type", "value": null}), format!("{pointer}/0/value")),
        (json!({"field": "type", "value": 5}), format!("{pointer}/0/value")),
    ] {
        // One bad element rejects the whole list.
        assert_eq!(update(&assert_warned(&format!("{at}/0"), Some(element), &location)), None);
    }
    // Exceptions: an empty value is a value, an empty list is declared, an
    // unknown member is INFO, and absence infers from payload `const`s.
    let empty_value = imported(&format!("{at}/0"), Some(json!({"field": "type", "value": ""})));
    assert_eq!(update(&empty_value).unwrap()[0].value, "");
    assert_eq!(update(&imported(at, Some(json!([])))), Some(vec![]));
    let unknown = imported(&format!("{at}/0"), Some(json!({"field": "type", "value": "event", "note": 1})));
    assert_diagnostic(&unknown, false, &format!("{pointer}/0/note"));
    assert_eq!(update(&imported(at, None)), None);
}

#[test]
fn fallback_extension() {
    let at = "/components/messages/Feed.Envelope/x-schematic-fallback";
    let pointer = "#/components/messages/Feed.Envelope/x-schematic-fallback";
    let envelope = |result: &AsyncImportResult| endpoint(result, "Feed").messages.iter().find(|m| m.name == "Envelope").unwrap().fallback;
    for replacement in [Value::Null, json!(""), json!(1)] {
        assert!(!envelope(&assert_warned(at, Some(replacement), pointer)));
    }
    assert!(!envelope(&imported(at, Some(json!(false)))));
    assert!(!envelope(&imported(at, None)));

    // A fallback with selectors is not the fallback (WARN).
    let selected = assert_warned("/components/messages/Feed.Update/x-schematic-fallback", Some(json!(true)), "#/channels/Feed/messages/Update");
    assert!(endpoint(&selected, "Feed").messages.iter().all(|m| m.name != "Update" || !m.fallback));

    // Two fallbacks in one direction: a WARN, and neither is the fallback.
    let mut doc = control();
    doc["components"]["messages"]["Feed.Delete"]["x-schematic-fallback"] = json!(true);
    doc["components"]["messages"]["Feed.Delete"].as_object_mut().unwrap().remove("x-schematic-selectors");
    let result = import(&doc).unwrap();
    assert_diagnostic(&result, true, "#/channels/Feed");
    let feed = endpoint(&result, "Feed");
    assert!(feed.messages.iter().all(|m| !m.fallback || m.direction == schematic_define::websocket::MessageDirection::Client));
}

#[test]
fn role_extension() {
    let at = "/channels/Host/x-schematic-role";
    let pointer = "#/channels/Host/x-schematic-role";
    for replacement in [Value::Null, json!("Host"), json!("server"), json!(5), json!("")] {
        assert_eq!(endpoint(&assert_warned(at, Some(replacement), pointer), "Host").role, EndpointRole::Client);
    }
    // Exception: absent takes `--ws-role`, which never changes directions.
    let doc = edited(at, None);
    let hosted = import_with(&doc.to_string(), WsRole::Server, None).unwrap();
    let host = endpoint(&hosted, "Host");
    assert_eq!(host.role, EndpointRole::Host);
    assert_eq!(host.messages, endpoint(&import(&control()).unwrap(), "Host").messages);
}

#[test]
fn heartbeat_extension() {
    let at = "/channels/Feed/x-schematic-heartbeat";
    let pointer = "#/channels/Feed/x-schematic-heartbeat";
    let heartbeat = |result: &AsyncImportResult| endpoint(result, "Feed").runtime.clone();
    for (replacement, location) in [
        (Value::Null, pointer.to_string()),
        (json!("x"), pointer.to_string()),
        (json!({}), format!("{pointer}/interval_secs")),
        (json!({"interval_secs": 0}), format!("{pointer}/interval_secs")),
        (json!({"interval_secs": -1}), format!("{pointer}/interval_secs")),
        (json!({"interval_secs": 1.5}), format!("{pointer}/interval_secs")),
        (json!({"interval_secs": "30"}), format!("{pointer}/interval_secs")),
        (json!({"interval_secs": 30, "timeout_secs": null}), format!("{pointer}/timeout_secs")),
        (json!({"interval_secs": 30, "timeout_secs": 0}), format!("{pointer}/timeout_secs")),
    ] {
        assert_eq!(heartbeat(&assert_warned(at, Some(replacement), &location)), None);
    }
    // Exception: an unknown member is INFO and the hint is kept.
    let unknown = imported(at, Some(json!({"interval_secs": 5, "jitter": 1})));
    assert_diagnostic(&unknown, false, &format!("{pointer}/jitter"));
    assert_eq!(heartbeat(&unknown).unwrap().heartbeat.unwrap().interval_secs, 5);
    assert_eq!(heartbeat(&imported(at, None)), None);
}

#[test]
fn parameter_extension() {
    let at = "/channels/Feed/bindings/ws/query/properties/token/x-schematic-param";
    let pointer = "#/channels/Feed/bindings/ws/query/properties/token/x-schematic-param";
    let token = |result: &AsyncImportResult| {
        endpoint(result, "Feed").params.query.iter().find(|p| p.name == "token").unwrap().clone()
    };
    for replacement in [Value::Null, json!("x")] {
        assert!(!token(&assert_warned(at, Some(replacement), pointer)).secret);
    }
    // One bad member is dropped alone.
    for (member, value) in [("secret", json!("yes")), ("explode", json!(1)), ("rust_name", json!("")), ("style", json!("Weird"))] {
        let result = assert_warned(&format!("{at}/{member}"), Some(value), &format!("{pointer}/{member}"));
        assert_eq!(token(&result).rust_name.is_some(), member != "rust_name");
    }
    let unknown = imported(&format!("{at}/note"), Some(json!(1)));
    assert_diagnostic(&unknown, false, &format!("{pointer}/note"));
    let path = "/channels/Feed/parameters/room/x-schematic-param/type";
    assert_warned(path, Some(json!("date")), "#/channels/Feed/parameters/room/x-schematic-param/type");
}

// ---------------------------------------------------------------------------
// The 2.x reader shares the field readers; its own fields
// ---------------------------------------------------------------------------

fn v2_control() -> Value {
    json!({
        "asyncapi": "2.2.0",
        "info": {"title": "legacy-ws", "version": "1.0.0"},
        "servers": {"local": {"url": "localhost:8001", "protocol": "wss", "security": [{"token": []}]}},
        "channels": {
            "/rooms/{room}": {
                "parameters": {"room": {"description": "Room", "schema": {"type": "integer"}}},
                "publish": {"message": {"$ref": "#/components/messages/hello"}},
                "subscribe": {"message": {"oneOf": [{"$ref": "#/components/messages/event"}]}},
            },
        },
        "components": {
            "securitySchemes": {"token": {"type": "httpApiKey", "in": "header", "name": "auth-token"}},
            "messages": {
                "hello": {"payload": {"properties": {"kind": {"const": "req"}}}},
                "event": {"payload": {"properties": {"kind": {"const": "event"}, "code": {"const": 200}}}},
            },
        },
    })
}

fn v2_edited(pointer: &str, replacement: Option<Value>) -> Result<AsyncImportResult, String> {
    let mut doc = v2_control();
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let parent = doc.pointer_mut(parent).unwrap().as_object_mut().unwrap();
    match replacement {
        Some(value) => parent.insert(key.replace("~1", "/"), value),
        None => parent.remove(&key.replace("~1", "/")),
    };
    import(&doc)
}

#[test]
fn legacy_reader_fields() {
    let control = import(&v2_control()).unwrap();
    assert_eq!(control.ws_api.base_url, "wss://localhost:8001");
    assert_eq!(control.ws_api.auth, AuthStrategy::ApiKey { header: "auth-token".into(), value_prefix: None });
    let endpoint = &control.ws_api.endpoints[0];
    assert_eq!(endpoint.params.path[0].param_type, schematic_define::QueryParamType::Integer);
    // A non-string `const` selects nothing, and says so.
    assert_diagnostic(&control, false, "#/components/messages/event/payload");
    assert_eq!(endpoint.messages.len(), 2);

    let room = "/channels/~1rooms~1{room}";
    let error = |pointer: &str, replacement: Option<Value>, location: &str| match v2_edited(pointer, replacement) {
        Err(message) => assert!(message.contains(&format!("[ERROR] {location}:")), "{pointer}: {message}"),
        Ok(_) => panic!("{pointer} must fail at {location}"),
    };
    error("/servers/local/url", None, "#/servers/local/url");
    error("/servers/local/url", Some(json!(5)), "#/servers/local/url");
    error("/components/securitySchemes/token/name", None, "#/components/securitySchemes/token/name");
    error("/servers/local/security", Some(json!([{"missing": []}])), "#/servers/local/security/0");
    error(&format!("{room}/publish"), Some(json!(5)), "#/channels/~1rooms~1{room}/publish");
    error(&format!("{room}/subscribe/message/oneOf"), Some(json!({})), "#/channels/~1rooms~1{room}/subscribe/message/oneOf");
    error(&format!("{room}/parameters/room/schema"), Some(json!("integer")), "#/channels/~1rooms~1{room}/parameters/room/schema");
    error("/components/messages/hello/payload", Some(json!({"$ref": "#/components/schemas/Missing"})), "#/components/messages/hello/payload/$ref");
    error("/components/messages", Some(json!([])), "#/components/messages");
    // Exception: 2.x keeps its INFO for a message without a payload.
    let unpayloaded = v2_edited("/components/messages/hello/payload", None).unwrap();
    assert_diagnostic(&unpayloaded, false, "#/components/messages/hello/payload");
}
