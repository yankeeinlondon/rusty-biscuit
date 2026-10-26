//! One local fixture registration reaches every generator consumer of the
//! registration inventory: selection names and inputs, the REST outputs
//! (Rust, OpenAPI, Postman), and the WebSocket packaging (module files,
//! `lib.rs`, `prelude.rs`, reserved module names, and the partial-run
//! inventory). The shipped inventory is never edited.

use std::fs;
use std::path::Path;

use schematic_define::{DefinitionError, Endpoint, RestApi, WebSocketApi};
use schematic_definitions::registry::{
    ApiRegistration, Protocol, RestRegistration, WebSocketRegistration, assemble_module_group,
    registrations, rest_apis,
};
use schematic_gen::diagnostics::Feature;
use schematic_gen::output::ws_modules::{generate_ws_definition_modules, validate_ws_definition_modules};
use schematic_gen::output::{RustOutputRequest, Selection, prepare_rust_output};
use schematic_gen::pipeline::{
    BatchScope, GenerateOpts, OpenApiFormat, available_apis, available_apis_in, generate_batch,
    resolve_api_in,
};
use schematic_gen::validation::validate_rust_client_in;

fn fixture_api(ctx: &mut schematic_define::DefinitionContext, name: &str, path: &str) -> Result<RestApi, DefinitionError> {
    let ping = Endpoint::get(format!("{name}Ping"), path).describes("Ping").returns_empty().build()?;
    RestApi::builder(ctx, name, "https://fixture.test")
        .describes("Fixture")
        .no_auth()
        .module_path("fixture_shared")
        .endpoints([ping])
        .build()
}

/// Two REST APIs sharing one module, the shape of `ollama` and `emqx`.
fn fixture_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::fixture", |ctx| {
        Ok(vec![fixture_api(ctx, "FixtureOne", "/one")?, fixture_api(ctx, "FixtureTwo", "/two")?])
    })
}

fn fixture_websocket() -> WebSocketApi {
    let mut api = schematic_definitions::define_elevenlabs_websocket_api();
    api.name = "FixtureWs".into();
    api
}

/// A local inventory: one shared REST module and one WebSocket module.
pub(crate) fn fixture_inventory() -> Vec<ApiRegistration> {
    let rest = |name, key, member| ApiRegistration {
        name,
        module: "fixture_shared",
        definitions_path: "fixture",
        protocol: Protocol::Rest(RestRegistration { key, group: fixture_group, member }),
    };
    vec![
        rest("FixtureOne", "fixture-one", 0),
        rest("FixtureTwo", "fixture-two", 1),
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

fn opts<'a>(rust: &'a str, openapi: &'a str, postman: &'a str) -> GenerateOpts<'a> {
    GenerateOpts {
        output: rust,
        dry_run: false,
        verbose: 0,
        openapi_out: Some(openapi),
        openapi_format: OpenApiFormat::Json,
        openapi_version: None,
        postman_out: Some(postman),
        no_openapi: false,
        no_postman: false,
        postman_environment: false,
    }
}

#[test]
fn selection_names_and_inputs_come_from_the_inventory() {
    let inventory = fixture_inventory();
    assert_eq!(available_apis_in(&inventory), "fixture-one, fixture-two, all");
    assert_eq!(resolve_api_in(&inventory, "fixture-two").unwrap().name, "FixtureTwo");

    let unknown = resolve_api_in(&inventory, "fixture_ws").unwrap_err().to_string();
    assert!(unknown.contains("Unknown API: 'fixture_ws'"), "{unknown}");
    assert!(unknown.contains("Available APIs: fixture-one, fixture-two, all"), "{unknown}");
    assert!(resolve_api_in(&inventory, "all").is_err());

    let shipped: Vec<&str> = registrations().iter().filter_map(|entry| entry.rest()).map(|rest| rest.key).collect();
    assert_eq!(available_apis(), format!("{}, all", shipped.join(", ")), "shipped help lists every key in catalog order");
}

#[test]
fn a_fixture_rest_registration_reaches_every_rest_output() {
    let temp = tempfile::tempdir().unwrap();
    let path = |relative: &str| temp.path().join(relative).to_str().unwrap().to_owned();
    let (rust, openapi, postman) = (path("schema/src"), path("openapi"), path("postman"));

    let apis = rest_apis(&fixture_inventory());
    let report = generate_batch(&apis, &BatchScope::Complete, &opts(&rust, &openapi, &postman));
    report.into_result().expect("fixture batch generates");

    let rust_module = fs::read_to_string(Path::new(&rust).join("fixture_shared.rs")).unwrap();
    assert!(rust_module.contains("pub struct FixtureOne") && rust_module.contains("pub struct FixtureTwo"));
    let document = fs::read_to_string(Path::new(&openapi).join("fixture_shared.json")).unwrap();
    assert!(document.contains("\"/one\"") && document.contains("\"/two\""), "{document}");
    let collection = fs::read_to_string(Path::new(&postman).join("fixture_shared.postman_collection.json")).unwrap();
    assert!(collection.contains("FixtureOnePing") && collection.contains("FixtureTwoPing"));
}

#[test]
fn a_fixture_websocket_registration_reaches_every_websocket_consumer() {
    let inventory = fixture_inventory();
    let apis = rest_apis(&inventory);
    let registry: Vec<&RestApi> = apis.iter().collect();

    assert!(validate_ws_definition_modules(&inventory).diagnostics.is_empty());
    let modules: Vec<String> = generate_ws_definition_modules(&inventory).unwrap().into_iter().map(|(file, _)| file).collect();
    assert_eq!(modules, ["fixture_ws.rs"]);

    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("src");
    let request = |selection| RustOutputRequest {
        registry: &registry,
        registrations: &inventory,
        selection,
        output_dir: &root,
        manifest_dir: None,
    };
    prepare_rust_output(&request(Selection::All)).unwrap().plan.publish().unwrap();

    let module = fs::read_to_string(root.join("fixture_ws.rs")).unwrap();
    assert!(module.contains("pub use schematic_definitions::fixture::ws::*;"), "{module}");
    assert!(module.contains("/// Builds the Fixture WebSocket API definition."), "{module}");
    assert!(module.contains("schematic_definitions::define_fixture_ws_api()"), "{module}");
    assert!(!root.join("elevenlabs_ws.rs").exists(), "only the given inventory's WebSocket modules are packaged");

    let lib = fs::read_to_string(root.join("lib.rs")).unwrap();
    assert!(lib.contains("pub mod fixture_ws;"), "{lib}");
    assert!(lib.contains("[`FixtureWs`](fixture_ws::define_api) | Fixture WebSocket API definition |"), "{lib}");
    let prelude = fs::read_to_string(root.join("prelude.rs")).unwrap();
    assert!(prelude.contains("pub use crate::fixture_ws::define_api as define_fixture_ws_api_definition;"), "{prelude}");

    // The partial-run inventory recognizes the fixture module as WebSocket
    // output, not as a REST module group to keep.
    let selected = [registry[0]];
    let partial = prepare_rust_output(&request(Selection::Apis(&selected))).expect("partial run reads the fixture inventory");
    assert_eq!(partial.regenerated, ["fixture_shared"]);
    assert!(partial.retained.is_empty(), "{:?}", partial.retained);

    // A REST module may not take the fixture WebSocket module's name.
    let mut clash = apis[0].clone();
    clash.module_path = Some("fixture_ws".into());
    let reserved = |inventory: &[ApiRegistration]| {
        validate_rust_client_in(&[&clash], inventory).iter().any(|d| d.is_error() && d.feature == Feature::InvalidOutputFilename)
    };
    assert!(reserved(&inventory));
    assert!(!reserved(registrations()), "the shipped inventory has no `fixture_ws` module");
}
