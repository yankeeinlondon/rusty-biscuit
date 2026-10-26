//! Export to AsyncAPI 3.0, then import: the definition comes back equal in
//! meaning within the supported subset (R27), with selectors and fallback
//! (R56), the endpoint role (R57), and heartbeat hints (R58) intact.
//!
//! "Equal in meaning" is [`subset_projection`]: each rule there is one
//! documented loss, and every shipped API's losses are asserted by name so a
//! new one cannot hide. Both the JSON and the YAML form of each document are
//! imported.

use schematic_define::websocket::{EndpointRole, MessageDirection, MessageSelector, WebSocketApi};
use schematic_define::{ApiKeyLocation, AuthStrategy, EnvList};
use schematic_definitions::registry::registrations;
use schematic_gen::asyncapi_import::AsyncApiVersion;
use schematic_gen::export::asyncapi::asyncapi_documents;
use schematic_gen::ws_codegen::plan::lower_to_plan;

use crate::common::asyncapi::{canonical, fixture_api, round_trip, subset_differences, subset_projection};

/// The shipped API registered under `module`.
fn shipped(module: &str) -> WebSocketApi {
    asyncapi_documents(registrations())
        .into_iter()
        .find(|document| document.file_name == format!("{module}.json"))
        .unwrap_or_else(|| panic!("no WebSocket API in module {module}"))
        .api
}

fn assert_round_trip(api: &WebSocketApi, module: &str) {
    let expected = subset_projection(api, module);
    for yaml in [false, true] {
        let imported = round_trip(api, module, yaml);
        assert_eq!(imported.version, AsyncApiVersion::V3);
        let warnings: Vec<_> = imported.diagnostics.iter().filter(|d| !d.is_info()).collect();
        assert!(warnings.is_empty(), "{module} (yaml: {yaml}) warns: {warnings:?}");
        assert_eq!(canonical(imported.ws_api), expected, "{module} (yaml: {yaml})");
    }
}

macro_rules! shipped_round_trips {
    ($($test:ident => $module:literal, [$($difference:literal),* $(,)?], lowers: $lowers:literal;)*) => {
        $(
            #[test]
            fn $test() {
                let api = shipped($module);
                assert_round_trip(&api, $module);
                let expected: &[&str] = &[$($difference),*];
                assert_eq!(subset_differences(&api, $module), expected, "{}", $module);
                // Without its correlation hints, a definition whose messages
                // declare `responds_with` no longer lowers to a client: the
                // documented cost of leaving correlation out of the subset.
                let lowered = lower_to_plan(&round_trip(&api, $module, false).ws_api);
                if $lowers {
                    lowered.unwrap_or_else(|e| panic!("{} re-imports into an invalid definition: {e:?}", $module));
                } else {
                    let error = format!("{:?}", lowered.expect_err($module));
                    assert!(error.contains("responds_with needs the endpoint's runtime.correlation"), "{error}");
                }
            }
        )*

        /// Every shipped WebSocket API has a round-trip test above.
        #[test]
        fn every_shipped_websocket_api_round_trips() {
            let tested = [$($module),*];
            let shipped: Vec<String> = asyncapi_documents(registrations())
                .into_iter()
                .map(|document| document.file_name.trim_end_matches(".json").to_string())
                .collect();
            assert_eq!(shipped, tested);
        }
    };
}

shipped_round_trips! {
    elevenlabs_ws_round_trips => "elevenlabs_ws",
        ["env metadata", "version defaults to 1.0.0", "lifecycle messages become ordinary messages"], lowers: true;
    unfolded_circle_core_ws_round_trips => "unfolded_circle_core_ws",
        ["env metadata", "version defaults to 1.0.0", "correlation hints", "auth flow hints"], lowers: false;
    unfolded_circle_dock_ws_round_trips => "unfolded_circle_dock_ws",
        ["env metadata", "version defaults to 1.0.0", "correlation hints", "auth flow hints"], lowers: false;
    unfolded_circle_integration_ws_round_trips => "unfolded_circle_integration_ws",
        ["env metadata", "version defaults to 1.0.0", "correlation hints", "auth flow hints"], lowers: false;
    samsung_smart_tv_remote_ws_round_trips => "samsung_smart_tv_remote_ws",
        ["version defaults to 1.0.0"], lowers: true;
}

#[test]
fn the_fixture_keeps_every_subset_field() {
    let api = fixture_api();
    assert_round_trip(&api, "fixture_stream");
    // The projection changes only what lies outside the subset; this fixture
    // declares nothing outside it but its environment variable names.
    assert_eq!(subset_differences(&api, "fixture_stream"), ["env metadata"]);
}

#[test]
fn overlapping_payloads_are_told_apart_by_their_selectors() {
    let imported = round_trip(&fixture_api(), "fixture_stream", false).ws_api;
    let feed = imported.endpoints.iter().find(|e| e.id == "Feed").unwrap();
    let by_name = |name: &str| feed.messages.iter().find(|m| m.name == name).unwrap();
    let (update, delete) = (by_name("Update"), by_name("Delete"));
    assert_eq!(update.schema, delete.schema, "one payload type");
    assert_eq!(update.selectors, Some(vec![MessageSelector::new("type", "event"), MessageSelector::new("msg", "update")]));
    assert_eq!(delete.selectors, Some(vec![MessageSelector::new("type", "event"), MessageSelector::new("msg", "delete")]));
    // An empty selector value is a value; an empty list is declared, not absent.
    assert_eq!(by_name("Ping").selectors.as_ref().unwrap()[1], MessageSelector::new("note", ""));
    let both = imported.endpoints.iter().find(|e| e.id == "Both").unwrap();
    assert_eq!(both.messages[0].selectors, Some(vec![]));
    let fallbacks: Vec<&str> = feed.messages.iter().filter(|m| m.fallback).map(|m| m.name.as_str()).collect();
    assert_eq!(fallbacks, ["Envelope"]);
}

#[test]
fn each_role_survives_and_never_changes_directions() {
    let imported = round_trip(&fixture_api(), "fixture_stream", false).ws_api;
    let endpoint = |id: &str| imported.endpoints.iter().find(|e| e.id == id).unwrap();
    assert_eq!(endpoint("Feed").role, EndpointRole::Client);
    assert_eq!(endpoint("Host").role, EndpointRole::Host);
    assert_eq!(endpoint("Both").role, EndpointRole::Both);
    // A host endpoint still describes the client: its client message is sent.
    let host = endpoint("Host");
    let command = host.messages.iter().find(|m| m.name == "Command").unwrap();
    assert_eq!(command.direction, MessageDirection::Client);
    assert_eq!(endpoint("Both").messages[0].direction, MessageDirection::Bidirectional);
}

#[test]
fn heartbeat_hints_are_read_back() {
    let imported = round_trip(&fixture_api(), "fixture_stream", true).ws_api;
    let heartbeat = |id: &str| {
        imported.endpoints.iter().find(|e| e.id == id).unwrap().runtime.as_ref().and_then(|r| r.heartbeat.clone())
    };
    let feed = heartbeat("Feed").unwrap();
    assert_eq!((feed.interval_secs, feed.timeout_secs), (30, Some(10)));
    let host = heartbeat("Host").unwrap();
    assert_eq!((host.interval_secs, host.timeout_secs), (15, None));
    assert!(heartbeat("Both").is_none());
}

#[test]
fn a_base_path_assembles_the_same_handshake_url() {
    let api = fixture_api();
    let imported = round_trip(&api, "fixture_stream", false).ws_api;
    assert_eq!(imported.base_url, "wss://stream.example.com:8443/api/v2");
    let url = |api: &WebSocketApi| {
        let feed = api.endpoints.iter().find(|e| e.id == "Feed").unwrap();
        format!("{}{}", api.base_url, feed.path)
    };
    assert_eq!(url(&imported), url(&api));
    assert_eq!(url(&imported), "wss://stream.example.com:8443/api/v2/feed/{room}");

    // A trailing slash on the base is the same URL and never doubles a `/`.
    let mut slashed = api.clone();
    slashed.base_url.push('/');
    assert_eq!(round_trip(&slashed, "fixture_stream", false).ws_api.base_url, api.base_url);
}

#[test]
fn every_auth_form_in_the_subset_round_trips() {
    let forms = [
        AuthStrategy::None,
        AuthStrategy::BearerToken { header: None },
        AuthStrategy::BearerToken { header: Some("X-Token".into()) },
        AuthStrategy::ApiKey { header: "api-key".into(), value_prefix: None },
        AuthStrategy::ApiKeyParam { name: "key".into(), location: ApiKeyLocation::Query },
        AuthStrategy::ApiKeyParam { name: "sid".into(), location: ApiKeyLocation::Cookie },
        AuthStrategy::Basic { username_env: EnvList::new(vec![]), password_env: EnvList::new(vec![]) },
    ];
    for auth in forms {
        let mut api = fixture_api();
        api.auth = auth.clone();
        assert_eq!(round_trip(&api, "fixture_stream", false).ws_api.auth, auth);
    }
}
