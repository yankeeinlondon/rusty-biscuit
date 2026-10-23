//! The registration inventory is the one list of APIs: the registry's
//! derived lists agree with it, its entries agree with the APIs they build,
//! and a local fixture inventory reaches every derived list without editing
//! the shipped one.

use std::collections::HashSet;

use schematic_define::{DefinitionError, Endpoint, RestApi, WebSocketApi};
use schematic_definitions::registry::{
    ApiRegistration, Protocol, RestRegistration, WebSocketRegistration, assemble_module_group,
    find_rest, group_by_module, registrations, registry_key_for, rest_apis, websocket_apis,
};
use schematic_definitions::{all_apis, apis_by_module};

fn fixture_api(ctx: &mut schematic_define::DefinitionContext, name: &str) -> Result<RestApi, DefinitionError> {
    let ping = Endpoint::get("Ping", "/ping").describes("Ping").returns_empty().build()?;
    RestApi::builder(ctx, name, "https://fixture.test")
        .describes("Fixture")
        .no_auth()
        .module_path("fixture_shared")
        .endpoints([ping])
        .build()
}

/// Two REST APIs sharing one module, the shape of `ollama` and `emqx`.
fn fixture_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::fixture", |ctx| Ok(vec![fixture_api(ctx, "FixtureOne")?, fixture_api(ctx, "FixtureTwo")?]))
}

fn fixture_websocket() -> WebSocketApi {
    let mut api = schematic_definitions::define_elevenlabs_websocket_api();
    api.name = "FixtureWs".into();
    api
}

fn rest_entry(name: &'static str, key: &'static str, member: usize) -> ApiRegistration {
    ApiRegistration {
        name,
        module: "fixture_shared",
        definitions_path: "fixture",
        protocol: Protocol::Rest(RestRegistration { key, group: fixture_group, member }),
    }
}

fn fixture_inventory() -> Vec<ApiRegistration> {
    vec![
        rest_entry("FixtureOne", "fixture-one", 0),
        rest_entry("FixtureTwo", "fixture-two", 1),
        ApiRegistration {
            name: "FixtureWs",
            module: "fixture_ws",
            definitions_path: "fixture::ws",
            protocol: Protocol::WebSocket(WebSocketRegistration {
                define: fixture_websocket,
                constructor: "define_fixture_ws_api",
                summary: "Fixture WebSocket API definition",
            }),
        },
    ]
}

#[test]
fn a_fixture_registration_reaches_every_derived_list() {
    let inventory = fixture_inventory();

    let names: Vec<String> = rest_apis(&inventory).into_iter().map(|api| api.name).collect();
    assert_eq!(names, ["FixtureOne", "FixtureTwo"]);

    let grouped = group_by_module(&inventory);
    assert_eq!(grouped.keys().collect::<Vec<_>>(), ["fixture_shared"]);
    assert_eq!(grouped["fixture_shared"].iter().map(|api| api.name.as_str()).collect::<Vec<_>>(), ["FixtureOne", "FixtureTwo"]);

    assert_eq!(find_rest(&inventory, "fixture-two").map(|entry| entry.name), Some("FixtureTwo"));
    assert!(find_rest(&inventory, "fixture_ws").is_none(), "a WebSocket entry has no selection key");
    assert!(find_rest(&inventory, "FixtureOne").is_none(), "selection is by key, not by name");

    let websocket: Vec<String> = websocket_apis(&inventory).into_iter().map(|api| api.name).collect();
    assert_eq!(websocket, ["FixtureWs"]);
}

#[test]
#[should_panic(expected = "module group `fixture_shared` builds `FixtureTwo`, which has no registration")]
fn an_unregistered_group_member_is_refused() {
    let _ = rest_apis(&[rest_entry("FixtureOne", "fixture-one", 0)]);
}

#[test]
#[should_panic(expected = "member 1 of module group `fixture_shared`")]
fn a_registration_naming_the_wrong_member_is_refused() {
    let _ = rest_apis(&[rest_entry("FixtureOne", "fixture-one", 1), rest_entry("FixtureTwo", "fixture-two", 0)]);
}

#[test]
#[should_panic(expected = "names member 2 of module group `fixture_shared`")]
fn a_registration_past_the_group_is_refused() {
    let _ = rest_apis(&[rest_entry("FixtureOne", "fixture-one", 0), rest_entry("FixtureTwo", "fixture-two", 1), rest_entry("FixtureThree", "fixture-three", 2)]);
}

#[test]
fn the_registry_lists_derive_from_the_shipped_inventory_in_its_order() {
    let registered: Vec<&str> = registrations().iter().filter(|entry| entry.rest().is_some()).map(|entry| entry.name).collect();
    let all: Vec<String> = all_apis().into_iter().map(|api| api.name).collect();
    assert_eq!(all, registered);

    let by_module: Vec<String> = apis_by_module().into_values().flatten().map(|api| api.name).collect();
    assert_eq!(by_module, registered, "apis_by_module keeps catalog order within and across modules");

    let websocket: Vec<String> = websocket_apis(registrations()).into_iter().map(|api| api.name).collect();
    let registered_ws: Vec<&str> = registrations().iter().filter(|entry| entry.websocket().is_some()).map(|entry| entry.name).collect();
    assert_eq!(websocket, registered_ws);
}

#[test]
fn every_shipped_registration_agrees_with_the_api_it_builds() {
    let mut keys = HashSet::new();
    let mut names = HashSet::new();
    let rest_modules: HashSet<&str> = registrations().iter().filter(|entry| entry.rest().is_some()).map(|entry| entry.module).collect();
    let apis = all_apis();
    for entry in registrations() {
        assert!(names.insert(entry.name), "{} is registered twice", entry.name);
        assert!(
            entry.definitions_path.split("::").all(syn_like_ident),
            "{}: definitions path {:?}",
            entry.name,
            entry.definitions_path
        );
        match &entry.protocol {
            Protocol::Rest(rest) => {
                assert!(keys.insert(rest.key), "{}: key {:?} is used twice", entry.name, rest.key);
                assert!(rest.key.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-'), "{}: key {:?} is not kebab-case", entry.name, rest.key);
                assert_eq!(registry_key_for(entry.name), rest.key);
                let api = apis.iter().find(|api| api.name == entry.name).expect("built");
                let resolved = api.module_path.clone().unwrap_or_else(|| api.name.to_lowercase());
                assert_eq!(entry.module, resolved, "{}: registered module differs from the API's module", entry.name);
            }
            Protocol::WebSocket(ws) => {
                assert_eq!((ws.define)().name, entry.name);
                assert!(!rest_modules.contains(entry.module), "{}: WebSocket module {:?} collides with a REST module", entry.name, entry.module);
                assert!(ws.summary.ends_with("WebSocket API definition"), "{}: summary {:?}", entry.name, ws.summary);
            }
        }
    }
}

fn syn_like_ident(segment: &str) -> bool {
    let mut chars = segment.chars();
    chars.next().is_some_and(|c| c.is_ascii_lowercase() || c == '_') && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}
