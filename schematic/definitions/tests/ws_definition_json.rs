//! The JSON form of a WebSocket definition: every decision-bearing field has
//! one outcome for each shape it can take.
//!
//! Each cell starts from the shipped Dock definition (with a heartbeat and
//! API runtime hints added so every field is present) and makes one edit.
//! Default outcome: an invalid shape fails to load. The cells listed as
//! loading are the exceptions; empty strings and zero durations load here
//! and are rejected by generation-time validation in `schematic-gen`
//! (`ws_codegen::plan` tests), which is the one place that decides them.

use schematic_define::websocket::{
    EndpointRole, FrameFormat, MessageSelector, RequestIdType, WebSocketApi,
    WebSocketRuntimeHints,
};
use schematic_define::{AuthPolicy, EnvMapping};
use schematic_definitions::define_unfolded_circle_dock_ws_api;
use serde_json::{Value, json};

/// The control document: the Dock definition plus a heartbeat and explicit
/// API runtime hints.
fn control() -> Value {
    let mut value = serde_json::to_value(define_unfolded_circle_dock_ws_api()).unwrap();
    value["endpoints"][0]["runtime"]["heartbeat"] = json!({"interval_secs": 30, "timeout_secs": 10});
    value["runtime"] =
        json!({"frame_format": "json_text", "supports_reconnect": true, "request_id_type": "u64"});
    value
}

fn load(text: &str) -> Result<WebSocketApi, serde_json::Error> {
    serde_json::from_str(text)
}

/// `pointer` replaced by `replacement`, or removed when it is `None`.
fn edited(pointer: &str, replacement: Option<Value>) -> String {
    let mut value = control();
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let parent = value.pointer_mut(parent).unwrap_or_else(|| panic!("no {parent}"));
    match (parent, replacement) {
        (Value::Object(map), Some(new)) => {
            map.insert(key.to_string(), new);
        }
        (Value::Object(map), None) => {
            assert!(map.remove(key).is_some(), "{pointer} was not present");
        }
        (Value::Array(items), Some(new)) => items[key.parse::<usize>().unwrap()] = new,
        (_, None) => panic!("{pointer}: only object keys are removed"),
        _ => panic!("{pointer}: not a container"),
    }
    serde_json::to_string(&value).unwrap()
}

const ENDPOINT: &str = "/endpoints/0";
const REQUEST: &str = "/endpoints/0/messages/0"; // RequestEnvelope: selectors + responds_with
const RESPONSE: &str = "/endpoints/0/messages/1"; // ResponseEnvelope: the fallback
const HINTS: &str = "/endpoints/0/runtime";

fn assert_rejected(pointer: &str, replacement: Option<Value>) {
    let text = edited(pointer, replacement.clone());
    assert!(load(&text).is_err(), "{pointer} = {replacement:?} must not load");
}

fn loaded(pointer: &str, replacement: Option<Value>) -> WebSocketApi {
    let text = edited(pointer, replacement.clone());
    load(&text).unwrap_or_else(|error| panic!("{pointer} = {replacement:?}: {error}"))
}

#[test]
fn control_loads_with_every_decision_bearing_field() {
    let api = load(&serde_json::to_string(&control()).unwrap()).unwrap();
    let endpoint = &api.endpoints[0];
    assert_eq!(endpoint.role, EndpointRole::Client);
    assert_eq!(endpoint.messages[0].responds_with.as_deref(), Some("ResponseEnvelope"));
    assert_eq!(
        endpoint.messages[0].selectors.as_deref(),
        Some(&[MessageSelector::new("type", "dock")][..])
    );
    assert!(endpoint.messages[1].fallback && endpoint.messages[1].selectors.is_none());
    let hints = endpoint.runtime.as_ref().unwrap();
    assert_eq!(hints.correlation.as_ref().unwrap().response_id_field, "req_id");
    assert_eq!(hints.auth_flow.as_ref().unwrap().challenge_message, "auth_required");
    assert_eq!(hints.heartbeat.as_ref().unwrap().timeout_secs, Some(10));
    assert_eq!(api.runtime, Some(WebSocketRuntimeHints::default()));
    assert_eq!(api.env_mapping, Some(EnvMapping::default()));
}

#[test]
fn absent_optional_fields_take_their_documented_meaning() {
    let role = loaded(&format!("{ENDPOINT}/role"), None);
    assert_eq!(role.endpoints[0].role, EndpointRole::Client);

    let link = loaded(&format!("{REQUEST}/responds_with"), None);
    assert_eq!(link.endpoints[0].messages[0].responds_with, None);

    let selectors = loaded(&format!("{REQUEST}/selectors"), None);
    assert_eq!(selectors.endpoints[0].messages[0].selectors, None);

    let fallback = loaded(&format!("{RESPONSE}/fallback"), None);
    assert!(!fallback.endpoints[0].messages[1].fallback);

    for hint in ["correlation", "auth_flow", "heartbeat"] {
        let api = loaded(&format!("{HINTS}/{hint}"), None);
        let hints = api.endpoints[0].runtime.as_ref().unwrap();
        let present = match hint {
            "correlation" => hints.correlation.is_some(),
            "auth_flow" => hints.auth_flow.is_some(),
            _ => hints.heartbeat.is_some(),
        };
        assert!(!present, "absent {hint} is no declaration");
    }

    let timeout = loaded(&format!("{HINTS}/correlation/default_timeout_secs"), None);
    let correlation = timeout.endpoints[0].runtime.as_ref().unwrap().correlation.clone().unwrap();
    assert_eq!(correlation.default_timeout_secs, 30, "the documented default");

    // The Dock declares no failure indicator, so only success is removable.
    let indicator = loaded(&format!("{HINTS}/auth_flow/success_indicator"), None);
    let flow = indicator.endpoints[0].runtime.as_ref().unwrap().auth_flow.clone().unwrap();
    assert_eq!((flow.success_indicator, flow.failure_indicator), (None, None));

    let heartbeat = loaded(&format!("{HINTS}/heartbeat/timeout_secs"), None);
    let hints = heartbeat.endpoints[0].runtime.as_ref().unwrap();
    assert_eq!(hints.heartbeat.as_ref().unwrap().timeout_secs, None);

    assert_eq!(loaded(HINTS, None).endpoints[0].runtime, None);
    assert_eq!(loaded("/runtime", None).runtime, None);
    for leaf in ["frame_format", "supports_reconnect", "request_id_type"] {
        let api = loaded(&format!("/runtime/{leaf}"), None);
        assert_eq!(api.runtime, Some(WebSocketRuntimeHints::default()), "{leaf}");
    }

    assert_eq!(loaded("/env_mapping", None).env_mapping, None);
    assert_eq!(loaded("/auth_policy", Some(json!({}))).auth_policy, Some(AuthPolicy::default()));
}

#[test]
fn explicit_empty_values_are_kept_as_declared() {
    let selectors = loaded(&format!("{REQUEST}/selectors"), Some(json!([])));
    assert_eq!(selectors.endpoints[0].messages[0].selectors, Some(vec![]));

    let hints = loaded(HINTS, Some(json!({})));
    assert_eq!(hints.endpoints[0].runtime, Some(Default::default()));

    let runtime = loaded("/runtime", Some(json!({})));
    assert_eq!(runtime.runtime, Some(WebSocketRuntimeHints::default()));

    let mapping = loaded("/env_mapping", Some(json!({})));
    assert_eq!(mapping.env_mapping, Some(EnvMapping::default()));

    let headers = loaded("/headers", Some(json!([])));
    assert!(headers.headers.is_empty());

    // Empty strings and zero durations load; generation rejects them.
    let link = loaded(&format!("{REQUEST}/responds_with"), Some(json!("")));
    assert_eq!(link.endpoints[0].messages[0].responds_with.as_deref(), Some(""));
    let field = loaded(&format!("{REQUEST}/selectors/0/field"), Some(json!("")));
    assert_eq!(field.endpoints[0].messages[0].selectors.as_ref().unwrap()[0].field, "");
    let value = loaded(&format!("{REQUEST}/selectors/0/value"), Some(json!("")));
    assert_eq!(value.endpoints[0].messages[0].selectors.as_ref().unwrap()[0].value, "");
    let zero = loaded(&format!("{HINTS}/heartbeat/interval_secs"), Some(json!(0)));
    assert_eq!(zero.endpoints[0].runtime.as_ref().unwrap().heartbeat.as_ref().unwrap().interval_secs, 0);
}

#[test]
fn declared_values_are_read_exactly() {
    let host = loaded(&format!("{ENDPOINT}/role"), Some(json!("both")));
    assert_eq!(host.endpoints[0].role, EndpointRole::Both);
    let runtime = loaded(
        "/runtime",
        Some(json!({"frame_format": "json_binary", "supports_reconnect": false, "request_id_type": "string"})),
    );
    let hints = runtime.runtime.unwrap();
    assert_eq!(
        (hints.frame_format, hints.supports_reconnect, hints.request_id_type),
        (FrameFormat::JsonBinary, false, RequestIdType::String)
    );
    let headers = loaded("/headers", Some(json!([["X-Client", "dock"]])));
    assert_eq!(headers.headers, [("X-Client".to_string(), "dock".to_string())]);
}

#[test]
fn explicit_null_is_rejected_and_never_read_as_absent() {
    for pointer in [
        format!("{ENDPOINT}/role"),
        format!("{ENDPOINT}/params"),
        format!("{ENDPOINT}/runtime"),
        format!("{REQUEST}/responds_with"),
        format!("{REQUEST}/selectors"),
        format!("{REQUEST}/selectors/0/field"),
        format!("{REQUEST}/selectors/0/value"),
        format!("{RESPONSE}/fallback"),
        format!("{HINTS}/correlation"),
        format!("{HINTS}/correlation/request_id_field"),
        format!("{HINTS}/correlation/response_id_field"),
        format!("{HINTS}/correlation/default_timeout_secs"),
        format!("{HINTS}/auth_flow"),
        format!("{HINTS}/auth_flow/challenge_message"),
        format!("{HINTS}/auth_flow/auth_request_schema"),
        format!("{HINTS}/auth_flow/success_indicator"),
        format!("{HINTS}/auth_flow/failure_indicator"),
        format!("{HINTS}/heartbeat"),
        format!("{HINTS}/heartbeat/interval_secs"),
        format!("{HINTS}/heartbeat/timeout_secs"),
        "/runtime".to_string(),
        "/runtime/frame_format".to_string(),
        "/runtime/supports_reconnect".to_string(),
        "/runtime/request_id_type".to_string(),
        "/env_mapping".to_string(),
        "/auth_policy".to_string(),
        "/headers".to_string(),
    ] {
        assert_rejected(&pointer, Some(Value::Null));
    }
}

#[test]
fn wrong_types_and_malformed_elements_are_rejected() {
    let cells: Vec<(String, Value)> = vec![
        (format!("{ENDPOINT}/role"), json!(1)),
        (format!("{ENDPOINT}/role"), json!("")),
        (format!("{ENDPOINT}/role"), json!("server")),
        (format!("{ENDPOINT}/params"), json!(1)),
        (format!("{ENDPOINT}/params"), json!({})),
        (format!("{ENDPOINT}/params"), json!([[], [], []])),
        (format!("{ENDPOINT}/runtime"), json!([])),
        (format!("{REQUEST}/responds_with"), json!(1)),
        (format!("{REQUEST}/responds_with"), json!(["ResponseEnvelope"])),
        (format!("{REQUEST}/selectors"), json!({"field": "type", "value": "req"})),
        (format!("{REQUEST}/selectors"), json!([1])),
        (format!("{REQUEST}/selectors"), json!([{"field": "type", "value": "req"}, 1])),
        (format!("{REQUEST}/selectors"), json!([{}])),
        (format!("{REQUEST}/selectors"), json!([["type", "req"]])),
        (format!("{REQUEST}/selectors/0/field"), json!(1)),
        (format!("{REQUEST}/selectors/0/value"), json!(1)),
        (format!("{REQUEST}/selectors/0/value"), json!(true)),
        (format!("{REQUEST}/selectors/0/value"), json!({})),
        (format!("{RESPONSE}/fallback"), json!("true")),
        (format!("{RESPONSE}/fallback"), json!({})),
        (format!("{RESPONSE}/fallback"), json!([])),
        (format!("{HINTS}/correlation"), json!(1)),
        (format!("{HINTS}/correlation"), json!({})),
        (format!("{HINTS}/correlation"), json!(["id", "req_id"])),
        (format!("{HINTS}/correlation/request_id_field"), json!(1)),
        (format!("{HINTS}/correlation/default_timeout_secs"), json!("30")),
        (format!("{HINTS}/correlation/default_timeout_secs"), json!(-1)),
        (format!("{HINTS}/auth_flow"), json!("auth_required")),
        (format!("{HINTS}/auth_flow"), json!({})),
        (format!("{HINTS}/auth_flow/challenge_message"), json!(1)),
        (format!("{HINTS}/auth_flow/success_indicator"), json!(true)),
        (format!("{HINTS}/heartbeat"), json!(30)),
        (format!("{HINTS}/heartbeat"), json!({})),
        (format!("{HINTS}/heartbeat"), json!([30])),
        (format!("{HINTS}/heartbeat/interval_secs"), json!("30")),
        (format!("{HINTS}/heartbeat/timeout_secs"), json!([10])),
        ("/runtime".to_string(), json!(1)),
        ("/runtime".to_string(), json!([])),
        ("/env_mapping".to_string(), json!([])),
        ("/runtime/frame_format".to_string(), json!("")),
        ("/runtime/frame_format".to_string(), json!(1)),
        ("/runtime/supports_reconnect".to_string(), json!("true")),
        ("/runtime/supports_reconnect".to_string(), json!({})),
        ("/runtime/request_id_type".to_string(), json!("")),
        ("/runtime/request_id_type".to_string(), json!(64)),
        ("/env_mapping".to_string(), json!(1)),
        ("/auth_policy".to_string(), json!([])),
        ("/headers".to_string(), json!(1)),
        ("/headers".to_string(), json!([1])),
        ("/headers".to_string(), json!([["X-Client", "dock"], 1])),
    ];
    for (pointer, replacement) in cells {
        assert_rejected(&pointer, Some(replacement));
    }
}

#[test]
fn required_fields_cannot_be_omitted() {
    for pointer in [
        format!("{ENDPOINT}/params"),
        format!("{REQUEST}/selectors/0/field"),
        format!("{REQUEST}/selectors/0/value"),
        format!("{HINTS}/correlation/request_id_field"),
        format!("{HINTS}/correlation/response_id_field"),
        format!("{HINTS}/auth_flow/challenge_message"),
        format!("{HINTS}/auth_flow/auth_request_schema"),
        format!("{HINTS}/heartbeat/interval_secs"),
        "/headers".to_string(),
        "/env_auth".to_string(),
    ] {
        assert_rejected(&pointer, None);
    }
}

#[test]
fn duplicate_keys_and_trailing_content_are_rejected() {
    let text = serde_json::to_string(&control()).unwrap();
    let duplicates = [
        (r#""role":"client""#, r#""role":"client","role":"host""#),
        (r#""responds_with":"ResponseEnvelope""#, r#""responds_with":"ResponseEnvelope","responds_with":"Other""#),
        (r#""fallback":true"#, r#""fallback":true,"fallback":false"#),
        (r#""selectors":["#, r#""selectors":[],"selectors":["#),
        (r#""field":"type""#, r#""field":"type","field":"msg""#),
        (r#""correlation":{"#, r#""correlation":null,"correlation":{"#),
        (r#""heartbeat":{"#, r#""heartbeat":{},"heartbeat":{"#),
        (r#""interval_secs":30"#, r#""interval_secs":30,"interval_secs":5"#),
        (r#""frame_format":"json_text""#, r#""frame_format":"json_text","frame_format":"mixed""#),
        (r#""headers":[]"#, r#""headers":[],"headers":[]"#),
    ];
    for (original, duplicated) in duplicates {
        assert!(text.contains(original), "control lacks {original}");
        let edited = text.replacen(original, duplicated, 1);
        assert!(load(&edited).is_err(), "duplicate accepted: {duplicated}");
    }
    assert!(load(&format!("{text} garbage")).is_err());
    assert!(load(&format!("{text}{text}")).is_err());
    assert!(load(&text[..text.len() - 1]).is_err());
}

#[test]
fn every_shipped_definition_round_trips_twice_without_loss() {
    use schematic_definitions::{
        define_elevenlabs_websocket_api, define_samsung_smart_tv_remote_ws_api,
        define_unfolded_circle_core_ws_api, define_unfolded_circle_integration_ws_api,
    };
    for api in [
        define_elevenlabs_websocket_api(),
        define_unfolded_circle_core_ws_api(),
        define_unfolded_circle_dock_ws_api(),
        define_unfolded_circle_integration_ws_api(),
        define_samsung_smart_tv_remote_ws_api(),
    ] {
        let first = serde_json::to_string(&api).unwrap();
        let restored: WebSocketApi = serde_json::from_str(&first).unwrap();
        let second = serde_json::to_string(&restored).unwrap();
        let reread: WebSocketApi = serde_json::from_str(&second).unwrap();
        assert_eq!(reread, api, "{}", api.name);
        assert_eq!(first, second, "{}", api.name);
    }
}

/// `lifecycle.open_credential_field`, the opening payload field bound to the
/// handshake credential. Its own control adds an open message with the
/// binding to the Dock document; each cell edits that one field.
#[test]
fn the_open_credential_field_has_one_outcome_per_shape() {
    const FIELD: &str = "/endpoints/0/lifecycle/open_credential_field";
    let with_binding = |replacement: Option<Value>| {
        let mut value = control();
        let lifecycle = &mut value["endpoints"][0]["lifecycle"];
        lifecycle["open"] = json!({
            "name": "Init",
            "direction": "client",
            "schema": serde_json::to_value(schematic_define::Schema::new("InitMessage")).unwrap(),
            "description": null,
        });
        let (parent, key) = FIELD.rsplit_once('/').unwrap();
        let map = value.pointer_mut(parent).unwrap().as_object_mut().unwrap();
        if let Some(replacement) = replacement {
            map.insert(key.to_string(), replacement);
        }
        load(&serde_json::to_string(&value).unwrap())
    };
    let field = |api: WebSocketApi| api.endpoints[0].lifecycle.open_credential_field.clone();

    // Control: the declared field is read exactly and survives a round trip.
    let control_api = with_binding(Some(json!("xi-api-key"))).expect("control loads");
    assert_eq!(field(control_api.clone()).as_deref(), Some("xi-api-key"));
    let again: WebSocketApi = load(&serde_json::to_string(&control_api).unwrap()).unwrap();
    assert_eq!(again, control_api);

    // Absent binds nothing and is not written back.
    let absent = with_binding(None).expect("absent loads");
    assert_eq!(field(absent.clone()), None);
    let written = serde_json::to_value(&absent).unwrap();
    assert!(written.pointer(FIELD).is_none(), "{written}");

    // Explicit null and wrong types never read as absent.
    for shape in [Value::Null, json!(1), json!(true), json!([]), json!({}), json!(["xi-api-key"])] {
        assert!(with_binding(Some(shape.clone())).is_err(), "{shape} must not load");
    }

    // Empty loads as declared; generation rejects it (`ws_codegen::plan` tests).
    assert_eq!(field(with_binding(Some(json!(""))).expect("empty loads")).as_deref(), Some(""));
}
