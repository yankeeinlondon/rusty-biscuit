//! Per-output validation through the public generation entry points.
//!
//! Each output (Rust client, OpenAPI, Postman) validates the whole batch and
//! is written only when its own validator reports no error. These tests drive
//! `pipeline::generate_batch` and the `schematic-gen` binary into temporary
//! directories and assert what each output root holds afterward.

use std::collections::BTreeSet;
use std::path::Path;
use std::process::Command;

use biscuit_test_harness::bin_exe;
use schematic_define::auth::ApiKeyLocation;
use schematic_define::params::{EndpointParams, ParamDef, ParamStyle, QueryParamType};
use schematic_define::{SuccessShape, AuthStrategy, Endpoint, RestApi, SchemaComponent};
use schematic_gen::codegen::endpoint_methods::{endpoint_method_renames, resolve_names};
use schematic_gen::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use schematic_gen::errors::GeneratorError;
use schematic_gen::export::postman::{write_postman, write_postman_grouped};
use schematic_gen::pipeline::{
    BatchScope, GenerateOpts, OpenApiFormat, generate_batch, resolve_all_apis,
};
use schematic_gen::validation::{
    validate_openapi, validate_postman, validate_rust_client,
};

use crate::common::fresh_output::{TreeSnapshot, snapshot_tree};

/// A one-endpoint API in `module` whose response is free-form JSON.
fn fixture_api(name: &str, module: &str) -> RestApi {
    let mut context = schematic_define::DefinitionContext::types_in("::fixture");
    let endpoint = Endpoint::get("ListThings", "/things").describes("List things")
        .returns_json::<serde_json::Value>(&mut context).build().unwrap();
    RestApi::builder(&context, name, "https://fixture.example/v1")
        .describes(format!("{name} fixture")).no_auth().module_path(module)
        .endpoints([endpoint]).build().unwrap()
}

/// `api` with its response naming a `Thing` component spelled `rust_path`.
/// The stored Rust path may be malformed while the component's wire schema
/// is valid, as in a deserialized definition.
fn with_thing_component(mut api: RestApi, rust_path: &str) -> RestApi {
    api.schemas.push(SchemaComponent {
        name: "Thing".into(),
        rust_path: Some(rust_path.into()),
        schema: serde_json::json!({"type": "object"}),
    });
    api.endpoints[0].response = vec![SuccessShape::json_type("Thing")];
    api
}

/// Output roots for one batch run, under one temporary directory.
struct Roots {
    _temp: tempfile::TempDir,
    rust: String,
    openapi: String,
    postman: String,
}

impl Roots {
    fn new() -> Self {
        let temp = tempfile::tempdir().expect("temp dir");
        let path = |relative: &str| temp.path().join(relative).to_str().unwrap().to_string();
        Self {
            rust: path("schema/src"),
            openapi: path("openapi"),
            postman: path("postman"),
            _temp: temp,
        }
    }

    fn opts(&self) -> GenerateOpts<'_> {
        GenerateOpts {
            output: &self.rust,
            dry_run: false,
            verbose: 0,
            openapi_out: Some(&self.openapi),
            openapi_format: OpenApiFormat::Json,
            openapi_version: None,
            postman_out: Some(&self.postman),
            no_openapi: false,
            no_postman: false,
        }
    }

    fn schema_dir(&self) -> &Path {
        Path::new(&self.rust).parent().unwrap()
    }
}

fn snapshot_or_absent(root: &Path) -> Option<TreeSnapshot> {
    root.exists().then(|| snapshot_tree(root))
}

#[test]
fn malformed_rust_type_blocks_only_the_rust_client() {
    let roots = Roots::new();
    // A previous Rust run left these behind; the failed run must not touch them.
    std::fs::create_dir_all(&roots.rust).unwrap();
    std::fs::write(Path::new(&roots.rust).join("lib.rs"), "// earlier output\n").unwrap();
    std::fs::write(Path::new(&roots.rust).join("old.rs"), "// earlier module\n").unwrap();
    let rust_before = snapshot_tree(roots.schema_dir());

    let valid = fixture_api("Inventory", "inventory");
    // The component's Rust path is Rust-only data; exports name the component.
    let broken = with_thing_component(fixture_api("Broken", "broken"), "Vec<Thing");

    let report = generate_batch(
        &[valid, broken],
        &BatchScope::Complete,
        &roots.opts());

    // Every requested output was attempted; only Rust failed.
    assert_eq!(
        report.outputs.iter().map(|o| o.output).collect::<Vec<_>>(),
        vec![Output::Rust, Output::OpenApi, Output::Postman]
    );
    assert_eq!(report.failed(), vec![Output::Rust]);
    assert_eq!(report.succeeded(), vec![Output::OpenApi, Output::Postman]);

    let rust = report.get(Output::Rust).unwrap();
    assert_eq!(
        rust.diagnostics,
        vec![ValidationDiagnostic::new(
            Severity::Error,
            "Broken",
            Some("ListThings"),
            Feature::InvalidTypeSyntax,
            Output::Rust,
            "response type `Vec<Thing` is not valid Rust type syntax",
        )]
    );
    assert!(matches!(
        &rust.result,
        Err(GeneratorError::Validation { output: Output::Rust, diagnostics }) if diagnostics.len() == 1
    ));

    // The failing output's root is byte-identical, and no manifest appeared.
    assert_eq!(snapshot_tree(roots.schema_dir()), rust_before);
    assert!(!roots.schema_dir().join("Cargo.toml").exists());

    // Both exports were written for both APIs; OpenAPI names the component.
    let openapi_broken =
        std::fs::read_to_string(Path::new(&roots.openapi).join("broken.json")).unwrap();
    assert!(openapi_broken.contains("#/components/schemas/Thing"), "{openapi_broken}");
    assert!(!openapi_broken.contains("Vec<Thing"), "{openapi_broken}");
    assert!(Path::new(&roots.openapi).join("inventory.json").is_file());
    for module in ["inventory", "broken"] {
        assert!(
            Path::new(&roots.postman)
                .join(format!("{module}.postman_collection.json"))
                .is_file(),
            "{module}"
        );
    }

    // The batch result is a failure that names both sides.
    match report.into_result() {
        Err(GeneratorError::OutputsFailed { succeeded, failed }) => {
            assert_eq!(failed, vec![Output::Rust]);
            assert_eq!(succeeded, vec![Output::OpenApi, Output::Postman]);
        }
        other => panic!("expected OutputsFailed, got {other:?}"),
    }
}

#[test]
fn dangling_reference_blocks_rust_and_openapi_but_not_postman() {
    let roots = Roots::new();
    let inventory = fixture_api("Inventory", "inventory");
    let mut catalog = fixture_api("Catalog", "catalog");
    // `catalog` names a component it does not carry, so neither the Rust
    // client nor any OpenAPI document may be written, not even the valid
    // `inventory` ones. Postman never reads a schema.
    catalog.endpoints[0].response = vec![SuccessShape::json_vec_type("CatalogPage")];

    let report = generate_batch(&[inventory, catalog], &BatchScope::Complete, &roots.opts());

    assert_eq!(report.failed(), vec![Output::Rust, Output::OpenApi]);
    assert_eq!(report.succeeded(), vec![Output::Postman]);
    for output in [Output::Rust, Output::OpenApi] {
        let diagnostics = &report.get(output).unwrap().diagnostics;
        assert_eq!(diagnostics.len(), 1, "{output}: {diagnostics:#?}");
        let diagnostic = &diagnostics[0];
        assert_eq!(diagnostic.feature, Feature::InvalidSchemaReference);
        assert_eq!(diagnostic.api, vec!["Catalog"]);
        assert_eq!(diagnostic.endpoint.as_deref(), Some("ListThings"));
        assert!(diagnostic.message.contains("`CatalogPage`"), "{diagnostic}");
    }

    assert_eq!(snapshot_or_absent(Path::new(&roots.openapi)), None);
    assert_eq!(snapshot_or_absent(roots.schema_dir()), None);
    assert!(
        Path::new(&roots.postman)
            .join("catalog.postman_collection.json")
            .is_file()
    );
    assert!(report.into_result().is_err());
}

/// Two members of module `paired`. `PairedA` declares `ListThings` at
/// `GET /things`; `PairedB` declares the given id, path, and request suffix.
fn paired_group(second_id: &str, second_path: &str, second_suffix: Option<&str>) -> [RestApi; 2] {
    let first = fixture_api("PairedA", "paired");
    let mut second = fixture_api("PairedB", "paired");
    second.endpoints[0].id = second_id.to_string();
    second.endpoints[0].path = second_path.to_string();
    second.request_suffix = second_suffix.map(str::to_string);
    [first, second]
}

#[test]
fn incompatible_group_route_blocks_only_openapi_and_writes_no_document() {
    let roots = Roots::new();
    // `inventory` is valid and comes first, so a writer that published module
    // by module would already have written it when `paired` fails.
    let inventory = fixture_api("Inventory", "inventory");
    let [paired_a, paired_b] = paired_group("FetchThings", "/things", None);

    let report = generate_batch(
        &[inventory, paired_a, paired_b],
        &BatchScope::Complete,
        &roots.opts());

    assert_eq!(report.failed(), vec![Output::OpenApi]);
    assert_eq!(report.succeeded(), vec![Output::Rust, Output::Postman]);
    let Some(Err(GeneratorError::Validation {
        output: Output::OpenApi,
        diagnostics,
    })) = report.get(Output::OpenApi).map(|o| &o.result)
    else {
        panic!("expected an OpenAPI validation failure");
    };
    let rendered: Vec<String> = diagnostics.iter().map(ToString::to_string).collect();
    assert_eq!(
        rendered,
        ["severity=error api=PairedA,PairedB endpoint=<none> feature=duplicate-method-path \
          output=openapi: module 'paired': GET /things is declared by both APIs with \
          different operations; an OpenAPI document holds one operation per method and path"]
    );

    // No OpenAPI document was written, not even the valid `inventory` one.
    assert_eq!(snapshot_or_absent(Path::new(&roots.openapi)), None);
    // Cross-API route overlap is not a Rust or Postman error.
    assert!(Path::new(&roots.rust).join("paired.rs").is_file());
    assert!(
        Path::new(&roots.postman)
            .join("paired.postman_collection.json")
            .is_file()
    );
}

#[test]
fn same_api_duplicate_route_blocks_every_output() {
    let roots = Roots::new();
    let mut routes = fixture_api("Routes", "routes");
    routes.endpoints[0].path = "/things/{id}".to_string();
    // Only the path-parameter name differs, so the route is the same.
    let mut twin = routes.endpoints[0].clone();
    twin.id = "FetchThing".to_string();
    twin.path = "/things/{name}".to_string();
    routes.endpoints.push(twin);

    let report = generate_batch(
        &[routes],
        &BatchScope::Complete,
        &roots.opts());

    assert_eq!(
        report.failed(),
        vec![Output::Rust, Output::OpenApi, Output::Postman]
    );
    for output in [Output::Rust, Output::OpenApi, Output::Postman] {
        let diagnostics = &report.get(output).unwrap().diagnostics;
        let found: Vec<_> = diagnostics
            .iter()
            .map(|d| (d.severity, d.feature, d.endpoint.as_deref()))
            .collect();
        assert_eq!(
            found,
            vec![(
                Severity::Error,
                Feature::DuplicateMethodPath,
                Some("FetchThing")
            )],
            "{output}"
        );
    }
    // Nothing of any output was published.
    assert_eq!(snapshot_or_absent(roots.schema_dir()), None);
    assert_eq!(snapshot_or_absent(Path::new(&roots.openapi)), None);
    assert_eq!(snapshot_or_absent(Path::new(&roots.postman)), None);
    assert!(report.into_result().is_err());
}

fn located_param(name: &str, required: bool) -> ParamDef {
    ParamDef { deprecated: false, rust_name: None, secret: false, pager_managed: false,
        name: name.to_string(),
        required,
        description: Some(format!("{name} input")),
        param_type: QueryParamType::String,
        explode: false,
        style: ParamStyle::Form,
    }
}

/// An API whose one endpoint declares a required and an optional parameter
/// in both the header and the cookie location.
fn header_and_cookie_api() -> RestApi {
    let mut api = fixture_api("Located", "located");
    api.endpoints[0].params = Some(EndpointParams {
        header: vec![
            located_param("x-required", true),
            located_param("x-optional", false),
        ],
        cookie: vec![
            located_param("session", true),
            located_param("theme", false),
        ],
        ..EndpointParams::default()
    });
    api
}

/// `(key, value, disabled)` for every header of the collection's only request.
fn request_headers(collection: &serde_json::Value) -> Vec<(String, String, bool)> {
    crate::postman_schema::validate_postman_json(collection).expect("valid Postman collection");
    // Requests sit inside folders; descend to the only one.
    let mut item = collection;
    while item.get("request").is_none() {
        let children = item["item"].as_array().expect("items");
        assert_eq!(children.len(), 1, "{collection:#}");
        item = &children[0];
    }
    item["request"]["header"]
        .as_array()
        .expect("header array")
        .iter()
        .map(|header| {
            (
                header["key"].as_str().unwrap().to_string(),
                header["value"].as_str().unwrap().to_string(),
                header["disabled"].as_bool().unwrap_or(false),
            )
        })
        .collect()
}

fn read_json(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

#[test]
fn header_and_cookie_parameters_reach_every_postman_writer() {
    let expected = vec![
        ("x-required".to_string(), "<x-required>".to_string(), false),
        ("x-optional".to_string(), "<x-optional>".to_string(), true),
        ("Cookie".to_string(), "session=<session>".to_string(), false),
        ("Cookie".to_string(), "theme=<theme>".to_string(), true),
    ];
    let api = header_and_cookie_api();

    // Batch generator: every output succeeds with no diagnostic at all.
    let roots = Roots::new();
    let report = generate_batch(
        std::slice::from_ref(&api),
        &BatchScope::Complete,
        &roots.opts(),
    );
    assert!(report.failed().is_empty(), "{:#?}", report.outputs);
    for output in [Output::Rust, Output::OpenApi, Output::Postman] {
        assert!(
            report.get(output).unwrap().diagnostics.is_empty(),
            "{output}: {:#?}",
            report.get(output).unwrap().diagnostics
        );
    }
    let batch = read_json(&Path::new(&roots.postman).join("located.postman_collection.json"));
    assert_eq!(request_headers(&batch), expected);

    // OpenAPI declares all four in their own locations.
    let openapi = std::fs::read_to_string(Path::new(&roots.openapi).join("located.json")).unwrap();
    let openapi: serde_json::Value = serde_json::from_str(&openapi).unwrap();
    let declared: Vec<(&str, &str, bool)> = openapi["paths"]["/things"]["get"]["parameters"]
        .as_array()
        .expect("parameters")
        .iter()
        .map(|p| {
            (
                p["in"].as_str().unwrap(),
                p["name"].as_str().unwrap(),
                p["required"].as_bool().unwrap_or(false),
            )
        })
        .collect();
    assert_eq!(
        declared,
        vec![
            ("header", "x-required", true),
            ("header", "x-optional", false),
            ("cookie", "session", true),
            ("cookie", "theme", false),
        ]
    );

    // The single-API and grouped writers produce the same request headers.
    let dir = tempfile::tempdir().unwrap();
    let single = write_postman(&api, dir.path(), false).expect("single-API writer");
    assert_eq!(request_headers(&read_json(&single)), expected);
    let grouped =
        write_postman_grouped(&[&api], "grouped", dir.path(), false).expect("grouped writer");
    assert_eq!(request_headers(&read_json(&grouped)), expected);
}

#[test]
fn cookie_api_key_rides_a_cookie_header_in_postman() {
    let mut api = fixture_api("CookieKey", "cookie_key");
    api.auth = AuthStrategy::ApiKeyParam {
        name: "sid".to_string(),
        location: ApiKeyLocation::Cookie,
    };
    api.env_auth = vec!["COOKIE_KEY".to_string()];
    let dir = tempfile::tempdir().unwrap();

    let collection = read_json(&write_postman(&api, dir.path(), false).unwrap());

    crate::postman_schema::validate_postman_json(&collection).expect("valid Postman collection");
    // Postman's API-key auth places a key only in a header or the query.
    assert_eq!(
        collection["auth"],
        serde_json::json!({
            "type": "apikey",
            "apikey": [
                { "key": "key", "value": "Cookie" },
                { "key": "value", "value": "sid={{apiKey}}" },
                { "key": "in", "value": "header" },
            ],
        })
    );
}

fn array_of(inner: QueryParamType) -> QueryParamType {
    QueryParamType::Array(Box::new(inner))
}

/// An API whose one endpoint declares `param_type` as its only parameter, at
/// the location `place` puts it.
fn api_with_located_type(
    place: fn(ParamDef) -> EndpointParams,
    param_type: QueryParamType,
) -> RestApi {
    let mut api = fixture_api("Located", "located");
    api.endpoints[0].params = Some(place(ParamDef { rust_name: None, secret: false, pager_managed: false,
        param_type,
        ..located_param("flags", false)
    }));
    api
}

type Placement = (&'static str, Feature, fn(ParamDef) -> EndpointParams);

const PLACEMENTS: [Placement; 3] = [
    ("query", Feature::QueryParameter, |param| EndpointParams {
        query: vec![param],
        ..EndpointParams::default()
    }),
    ("header", Feature::HeaderParameter, |param| EndpointParams {
        header: vec![param],
        ..EndpointParams::default()
    }),
    ("cookie", Feature::CookieParameter, |param| EndpointParams {
        cookie: vec![param],
        ..EndpointParams::default()
    }),
];

#[test]
fn nested_arrays_fail_the_rust_client_at_every_parameter_location() {
    for (location, feature, place) in PLACEMENTS {
        let flat = api_with_located_type(place, array_of(QueryParamType::Boolean));
        assert!(
            validate_rust_client(&[&flat]).is_empty(),
            "{location}: flat array"
        );

        let nested = api_with_located_type(place, array_of(array_of(QueryParamType::Boolean)));
        let diagnostics = validate_rust_client(&[&nested]);
        // An optional query parameter the client cannot serialize is left
        // out of the request type (a warning); header and cookie parameters
        // have no such fallback.
        let (severity, message) = if feature == Feature::QueryParameter {
            (
                Severity::Warning,
                "query parameter `flags` is an array of arrays in `form` style with explode false, \
                 which the Rust client cannot serialize; it is left out of the request type"
                    .to_string(),
            )
        } else {
            (
                Severity::Error,
                format!(
                    "{location} parameter `flags` is an array of arrays, which has no \
                     {location} serialization"
                ),
            )
        };
        assert_eq!(
            diagnostics,
            vec![ValidationDiagnostic::new(severity, "Located", Some("ListThings"), feature, Output::Rust, message)],
            "{location}"
        );
        // OpenAPI states the nested shape as a schema; Postman carries no
        // parameter type. Neither export has anything to reject.
        assert!(validate_openapi(&[&nested]).is_empty(), "{location}");
        assert!(validate_postman(&[&nested]).is_empty(), "{location}");
    }
}

#[test]
fn nested_query_array_leaves_earlier_rust_output_untouched() {
    let roots = Roots::new();
    std::fs::create_dir_all(&roots.rust).unwrap();
    std::fs::write(Path::new(&roots.rust).join("lib.rs"), "// earlier output\n").unwrap();
    let rust_before = snapshot_tree(roots.schema_dir());
    let (_, _, query) = PLACEMENTS[0];
    let mut api = api_with_located_type(query, array_of(array_of(QueryParamType::Boolean)));
    // Required, so the client cannot leave it out: the Rust output fails.
    api.endpoints[0].params.as_mut().unwrap().query[0].required = true;
    assert!(
        validate_rust_client(&[&api]).iter().any(|d| d.is_error() && d.feature == Feature::QueryParameter),
        "the required nested array is a Rust validation error"
    );

    let report = generate_batch(&[api], &BatchScope::Complete, &roots.opts());

    assert_eq!(report.failed(), vec![Output::Rust]);
    assert_eq!(report.succeeded(), vec![Output::OpenApi, Output::Postman]);
    assert_eq!(snapshot_tree(roots.schema_dir()), rust_before);
    assert!(!roots.schema_dir().join("Cargo.toml").exists());
}

#[test]
fn live_definitions_declare_no_nested_array_parameters() {
    let apis = resolve_all_apis();
    let refs: Vec<&RestApi> = apis.iter().collect();

    let nested: Vec<ValidationDiagnostic> = validate_rust_client(&refs)
        .into_iter()
        .filter(|d| d.message.contains("array of arrays"))
        .collect();
    assert!(nested.is_empty(), "{nested:#?}");
}

#[test]
fn live_definitions_declare_no_duplicate_routes() {
    let apis = resolve_all_apis();
    let refs: Vec<&RestApi> = apis.iter().collect();

    let duplicates: Vec<ValidationDiagnostic> = validate_rust_client(&refs)
        .into_iter()
        .chain(validate_openapi(&refs))
        .chain(validate_postman(&refs))
        .filter(|d| d.feature == Feature::DuplicateMethodPath)
        .collect();
    assert!(duplicates.is_empty(), "{duplicates:#?}");
}

/// The endpoint-method renames of the shipped definitions, one rendered line
/// each. A definition change that adds a collision changes this list, so the
/// rename shows up in review instead of silently in generated names.
const SHIPPED_ENDPOINT_METHOD_RENAMES: &[&str] = &[];

#[test]
fn live_definitions_endpoint_method_renames_are_pinned() {
    let apis = resolve_all_apis();
    let refs: Vec<&RestApi> = apis.iter().collect();

    // Control: every shipped API resolves a name for each of its endpoints, so
    // an empty rename list is a result, not a skipped input.
    assert_eq!(refs.len(), 18, "shipped REST APIs");
    for api in &refs {
        let names = resolve_names(api);
        assert_eq!(names.endpoints.len(), api.endpoints.len(), "{}", api.name);
        assert!(!names.endpoints.is_empty(), "{} has no endpoints", api.name);
        assert!(names.conflicts.is_empty(), "{}: {:#?}", api.name, names.conflicts);
    }

    let renames: Vec<String> = endpoint_method_renames(&refs)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(renames, SHIPPED_ENDPOINT_METHOD_RENAMES);

    // The validator reports exactly the same renames, and no name collision.
    let diagnostics = validate_rust_client(&refs);
    let reported = diagnostics
        .iter()
        .filter(|d| d.feature == Feature::EndpointMethodRename)
        .count();
    assert_eq!(reported, SHIPPED_ENDPOINT_METHOD_RENAMES.len());
    let collisions: Vec<&ValidationDiagnostic> = diagnostics
        .iter()
        .filter(|d| d.feature == Feature::GeneratedNameCollision)
        .collect();
    assert!(collisions.is_empty(), "{collisions:#?}");
}

#[test]
fn grouped_document_renames_a_shared_raw_id() {
    let roots = Roots::new();
    // The Ollama shape: one raw id in two group members, distinct routes,
    // distinct request suffixes.
    let [paired_a, paired_b] = paired_group("ListThings", "/v2/things", Some("Compat"));

    let report = generate_batch(
        &[paired_a, paired_b],
        &BatchScope::Complete,
        &roots.opts());
    assert!(report.failed().is_empty(), "{:?}", report.failed());

    let written: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(Path::new(&roots.openapi).join("paired.json")).unwrap(),
    )
    .unwrap();
    let ids: Vec<(&str, &str)> = written["paths"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(path, item)| (path.as_str(), item["get"]["operationId"].as_str().unwrap()))
        .collect();
    assert_eq!(
        ids,
        [("/things", "ListThings"), ("/v2/things", "PairedBListThings")]
    );
}

/// All nine original query-bearing routes have been normalized or merged.
const EMBEDDED_QUERY_ENDPOINTS: [(&str, &str); 0] = [];

/// The query parameters of the live definitions that the Rust client cannot
/// serialize, so it leaves them out of their request types: JSON values
/// (OpenAI's import maps some `$ref`'d enums and objects to JSON) and one
/// `deepObject` filter.
const OMITTED_RUST_QUERY_PARAMS: [(&str, &str, &str); 15] = [
    ("OpenAI", "CreateConversationItems", "include"),
    ("OpenAI", "GetConversationItem", "include"),
    ("OpenAI", "GetResponse", "include"),
    ("OpenAI", "ListAuditLogs", "effective_at"),
    ("OpenAI", "ListAuditLogs", "event_types[]"),
    ("OpenAI", "ListChatCompletions", "metadata"),
    ("OpenAI", "ListConversationItems", "include"),
    ("OpenAI", "ListInputItems", "include"),
    ("OpenAI", "ListPaginatedFineTuningJobs", "metadata"),
    ("OpenAI", "ListSkills", "order"),
    ("OpenAI", "ListSkillVersions", "order"),
    ("OpenAI", "ListThreadItemsMethod", "order"),
    ("OpenAI", "ListThreadsMethod", "order"),
    ("OpenAI", "ListVideos", "order"),
    ("OpenAI", "RetrieveVideoContent", "variant"),
];

#[test]
fn live_definitions_have_no_embedded_queries_or_validation_gaps() {
    let apis = resolve_all_apis();
    let refs: Vec<&RestApi> = apis.iter().collect();

    let by_output = [
        (Output::Rust, validate_rust_client(&refs)),
        (Output::OpenApi, validate_openapi(&refs)),
        (Output::Postman, validate_postman(&refs)),
    ];

    let expected: BTreeSet<(String, String)> = EMBEDDED_QUERY_ENDPOINTS
        .iter()
        .map(|(api, endpoint)| (api.to_string(), endpoint.to_string()))
        .collect();
    let mut all_lines = 0;
    let mut unique = BTreeSet::new();
    for (output, diagnostics) in &by_output {
        let errors: Vec<_> = diagnostics.iter().filter(|d| d.is_error()).collect();
        assert!(errors.is_empty(), "{output}: {errors:#?}");
        // The Rust client leaves out exactly the listed query parameters.
        let omitted: BTreeSet<(String, String, String)> = diagnostics
            .iter()
            .filter(|d| d.feature == Feature::QueryParameter)
            .map(|d| {
                assert_eq!((d.severity, d.output), (Severity::Warning, Output::Rust), "{d}");
                assert!(d.message.ends_with("it is left out of the request type"), "{d}");
                let param = d.message.split('`').nth(1).unwrap().to_string();
                (d.api.join(","), d.endpoint.clone().unwrap(), param)
            })
            .collect();
        let expected_omitted: BTreeSet<(String, String, String)> = if *output == Output::Rust {
            OMITTED_RUST_QUERY_PARAMS
                .iter()
                .map(|(api, endpoint, param)| (api.to_string(), endpoint.to_string(), param.to_string()))
                .collect()
        } else {
            BTreeSet::new()
        };
        assert_eq!(omitted, expected_omitted, "{output}");
        // Embedded queries are the only other accepted gap in any output.
        let other: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.feature != Feature::QueryInPath && d.feature != Feature::QueryParameter)
            .collect();
        assert!(other.is_empty(), "{output}: {other:#?}");

        let warned: Vec<(String, String)> = diagnostics
            .iter()
            .filter(|d| d.feature == Feature::QueryInPath)
            .map(|d| {
                assert_eq!(d.severity, Severity::Warning);
                assert_eq!(d.output, *output);
                (d.api.join(","), d.endpoint.clone().unwrap())
            })
            .collect();
        assert_eq!(warned.len(), expected.len(), "{output}: {warned:#?}");
        let warned: BTreeSet<_> = warned.into_iter().collect();
        assert_eq!(warned, expected, "{output}");
        all_lines += warned.len();
        unique.extend(warned);
    }
    assert_eq!(all_lines, expected.len() * by_output.len());
    assert_eq!(unique.len(), expected.len());
}

#[test]
fn downstream_code_can_build_and_render_both_severities() {
    let warning = ValidationDiagnostic::new(
        Severity::Warning,
        "Feed",
        Some("Subscribe"),
        Feature::QueryInPath,
        Output::Postman,
        "embedded query",
    );
    let error = ValidationDiagnostic::for_group(
        Severity::Error,
        ["Feed", "FeedCompat"],
        Feature::GeneratedNameCollision,
        Output::Rust,
        "`SubscribeRequest` is generated twice",
    );

    assert!(!warning.is_error());
    assert!(error.is_error());
    assert_eq!(
        warning.to_string(),
        "severity=warning api=Feed endpoint=Subscribe feature=query-in-path output=postman: \
         embedded query"
    );
    assert_eq!(
        error.to_string(),
        "severity=error api=Feed,FeedCompat endpoint=<none> feature=generated-name-collision \
         output=rust: `SubscribeRequest` is generated twice"
    );
    assert_eq!(serde_json::to_value(&error).unwrap()["severity"], "error");
    assert_eq!(serde_json::to_value(&warning).unwrap()["severity"], "warning");
}

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(bin_exe!("schematic-gen"))
        .args(args)
        .env("NO_COLOR", "1")
        .output()
        .expect("run schematic-gen")
}

#[test]
fn cli_migrated_provider_generates_all_outputs_without_warnings() {
    let roots = Roots::new();
    let output = run_cli(&[
        "generate",
        "--api",
        "gitea",
        "--output",
        &roots.rust,
        "--openapi-out",
        &roots.openapi,
        "--postman-out",
        &roots.postman,
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(output.status.success(), "stdout:\n{stdout}\nstderr:\n{stderr}");
    for target in ["rust", "openapi", "postman"] {
        assert!(stdout.contains(&format!("[OK] {target} output")), "{stdout}");
    }
    assert!(!stderr.contains("severity="), "{stderr}");
    assert!(Path::new(&roots.openapi).join("gitea.json").is_file());
}

#[test]
fn cli_reports_each_output_and_exits_nonzero_when_one_fails() {
    let roots = Roots::new();
    // A regular file where the OpenAPI directory should be makes that
    // output's write fail on every platform.
    std::fs::write(&roots.openapi, "not a directory").unwrap();

    let output = run_cli(&[
        "generate",
        "--api",
        "gitea",
        "--output",
        &roots.rust,
        "--openapi-out",
        &roots.openapi,
        "--postman-out",
        &roots.postman,
    ]);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(output.status.code(), Some(1), "stdout:\n{stdout}\nstderr:\n{stderr}");
    assert!(stdout.contains("[OK] rust output"), "{stdout}");
    assert!(stdout.contains("[FAILED] openapi output"), "{stdout}");
    // Postman ran after the OpenAPI failure.
    assert!(stdout.contains("[OK] postman output"), "{stdout}");
    assert!(
        stderr.contains("generation failed for output(s) openapi (succeeded: rust, postman)"),
        "{stderr}"
    );
    assert!(Path::new(&roots.rust).join("lib.rs").is_file());
    assert!(roots.schema_dir().join("Cargo.toml").is_file());
    assert!(
        Path::new(&roots.postman)
            .join("gitea.postman_collection.json")
            .is_file()
    );
    assert_eq!(
        std::fs::read_to_string(&roots.openapi).unwrap(),
        "not a directory"
    );
}

/// `fixture_api`'s endpoint with `query` as its only parameters.
fn with_query(query: Vec<ParamDef>) -> RestApi {
    let mut api = fixture_api("Located", "located");
    api.endpoints[0].params = Some(EndpointParams { query, ..EndpointParams::default() });
    api
}

fn query_param(name: &str, required: bool, param_type: QueryParamType, style: ParamStyle, explode: bool) -> ParamDef {
    let mut param = ParamDef::new(name, param_type);
    param.required = required;
    param.style = style;
    param.explode = explode;
    param
}

fn rust_findings(api: &RestApi) -> Vec<(Severity, Feature, String)> {
    validate_rust_client(&[api]).into_iter().map(|d| (d.severity, d.feature, d.message)).collect()
}

#[test]
fn query_shapes_are_checked_for_the_rust_client_only() {
    let array = || QueryParamType::Array(Box::new(QueryParamType::String));
    // Control: every supported combination validates cleanly.
    let supported = with_query(vec![
        query_param("q", true, QueryParamType::String, ParamStyle::Form, false),
        query_param("n", false, QueryParamType::Number, ParamStyle::Form, true),
        query_param("form", false, array(), ParamStyle::Form, false),
        query_param("exploded", false, array(), ParamStyle::Form, true),
        query_param("spaces", false, array(), ParamStyle::SpaceDelimited, false),
        query_param("pipes", false, array(), ParamStyle::PipeDelimited, false),
        query_param("kind", false, QueryParamType::Enum(vec!["a".into()]), ParamStyle::Form, false),
    ]);
    assert!(rust_findings(&supported).is_empty(), "{:?}", rust_findings(&supported));

    let cases = [
        (QueryParamType::Json, ParamStyle::Form, false, "a JSON value in `form` style with explode false"),
        (QueryParamType::Json, ParamStyle::DeepObject, true, "a JSON value in `deepObject` style with explode true"),
        (QueryParamType::String, ParamStyle::Simple, false, "a single value in `simple` style with explode false"),
        (QueryParamType::String, ParamStyle::PipeDelimited, false, "a single value in `pipeDelimited` style with explode false"),
        (array(), ParamStyle::SpaceDelimited, true, "an array in `spaceDelimited` style with explode true"),
        (array(), ParamStyle::PipeDelimited, true, "an array in `pipeDelimited` style with explode true"),
        (array(), ParamStyle::Simple, false, "an array in `simple` style with explode false"),
        (
            QueryParamType::Array(Box::new(QueryParamType::Json)),
            ParamStyle::Form,
            true,
            "an array of JSON values in `form` style with explode true",
        ),
    ];
    for (param_type, style, explode, declared) in cases {
        let optional = with_query(vec![query_param("p", false, param_type.clone(), style, explode)]);
        assert_eq!(
            rust_findings(&optional),
            [(
                Severity::Warning,
                Feature::QueryParameter,
                format!(
                    "query parameter `p` is {declared}, which the Rust client cannot serialize; it is left \
                     out of the request type"
                ),
            )],
            "optional {declared}"
        );
        let required = with_query(vec![query_param("p", true, param_type, style, explode)]);
        assert_eq!(
            rust_findings(&required),
            [(
                Severity::Error,
                Feature::QueryParameter,
                format!("required query parameter `p` is {declared}, which the Rust client cannot serialize"),
            )],
            "required {declared}"
        );
        // The exports state any style; they have nothing to report.
        assert!(validate_openapi(&[&required]).is_empty(), "{declared}");
        assert!(validate_postman(&[&required]).is_empty(), "{declared}");
    }
}

#[test]
fn query_enums_need_values_distinct_variants_and_a_free_type_name() {
    let enum_param = |values: &[&str]| {
        query_param(
            "sort",
            false,
            QueryParamType::Enum(values.iter().map(|value| value.to_string()).collect()),
            ParamStyle::Form,
            false,
        )
    };
    assert_eq!(
        rust_findings(&with_query(vec![enum_param(&[])])),
        [(Severity::Error, Feature::QueryParameter, "query parameter `sort` declares an enum with no values".to_string())]
    );
    assert_eq!(
        rust_findings(&with_query(vec![enum_param(&["a-b", "a_b"])])),
        [(
            Severity::Error,
            Feature::GeneratedNameCollision,
            "query parameter `sort` enum values `a-b`, `a_b` all generate the variant `AB`".to_string()
        )]
    );
    // An array of enums is checked the same way.
    let mut array = enum_param(&["x", "x"]);
    array.param_type = QueryParamType::Array(Box::new(array.param_type));
    array.explode = true;
    assert_eq!(rust_findings(&with_query(vec![array]))[0].1, Feature::GeneratedNameCollision);

    // A grouped API in the same module whose request suffix spells its
    // `ListThings` request type `ListThingsRequestSort`.
    let api = with_query(vec![enum_param(&["asc", "desc"])]);
    let mut other = fixture_api("Other", "located");
    other.request_suffix = Some("RequestSort".to_string());
    let collisions: Vec<_> = validate_rust_client(&[&api, &other])
        .into_iter()
        .filter(|d| {
            d.is_error()
                && d.feature == Feature::GeneratedNameCollision
                && d.message.contains("`ListThingsRequestSort`")
                && d.message.contains("query-parameter enum")
        })
        .collect();
    assert_eq!(collisions.len(), 1, "{:?}", validate_rust_client(&[&api, &other]));
}
