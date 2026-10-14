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
use schematic_define::{ApiResponse, AuthStrategy, CrossOrigin, Endpoint, RestApi, RestMethod};
use schematic_definitions::registry::{SchemaRegistry, get_registries_for_module};
use schematic_gen::codegen::endpoint_methods::{endpoint_method_renames, resolve_names};
use schematic_gen::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use schematic_gen::errors::GeneratorError;
use schematic_gen::export::postman::{write_postman, write_postman_grouped};
use schematic_gen::pipeline::{
    BatchScope, GenerateOpts, OpenApiFormat, generate_batch, resolve_all_apis,
};
use schematic_gen::validation::{
    validate_openapi, validate_openapi_registry, validate_postman, validate_rust_client,
};

use crate::common::fresh_output::{TreeSnapshot, snapshot_tree};

fn fixture_api(name: &str, module: &str, response_type: &str) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{name} fixture"),
        base_url: "https://fixture.example/v1".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        auth_policy: None,
        env_auth: vec![],
        env_username: None,
        env_mapping: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "ListThings".to_string(),
            method: RestMethod::Get,
            path: "/things".to_string(),
            description: "List things".to_string(),
            request: None,
            response: ApiResponse::json_type(response_type),
            headers: vec![],
            params: None,
            oauth_scopes: None,
        }],
        module_path: Some(module.to_string()),
        request_suffix: None,
        version: None,
        cross_origin: CrossOrigin::default(),
        request_id_header: None,
        secret_names: vec![],
        default_pagination: None,
        pagination_exceptions: vec![],
    }
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

    let valid = fixture_api("Inventory", "inventory", "serde_json::Value");
    let broken = fixture_api("Broken", "broken", "Vec<Thing");
    let registry_for = |module: &str| match module {
        // The export carries the malformed name as plain data.
        "broken" => Some(SchemaRegistry::new().register::<serde_json::Value>("Vec<Thing")),
        "inventory" => Some(SchemaRegistry::new()),
        _ => None,
    };

    let report = generate_batch(
        &[valid, broken],
        &BatchScope::Complete,
        &roots.opts(),
        &registry_for,
    );

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

    // Both exports were written for both APIs, the malformed name included.
    let openapi_broken =
        std::fs::read_to_string(Path::new(&roots.openapi).join("broken.json")).unwrap();
    assert!(openapi_broken.contains("Vec<Thing"), "{openapi_broken}");
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
fn incomplete_registry_blocks_only_openapi() {
    let roots = Roots::new();
    let inventory = fixture_api("Inventory", "inventory", "serde_json::Value");
    let catalog = fixture_api("Catalog", "catalog", "CatalogPage");
    // `catalog` has no registry, so no OpenAPI document may be written,
    // not even the valid `inventory` one.
    let registry_for = |module: &str| (module == "inventory").then(SchemaRegistry::new);

    let report = generate_batch(
        &[inventory, catalog],
        &BatchScope::Complete,
        &roots.opts(),
        &registry_for,
    );

    assert_eq!(report.failed(), vec![Output::OpenApi]);
    assert_eq!(report.succeeded(), vec![Output::Rust, Output::Postman]);
    let openapi = &report.get(Output::OpenApi).unwrap().diagnostics;
    assert_eq!(openapi.len(), 1, "{openapi:#?}");
    assert_eq!(openapi[0].feature, Feature::IncompleteSchemaRegistry);
    assert_eq!(openapi[0].api, vec!["Catalog"]);
    assert!(openapi[0].message.contains("catalog"), "{}", openapi[0]);

    assert_eq!(snapshot_or_absent(Path::new(&roots.openapi)), None);
    assert!(Path::new(&roots.rust).join("catalog.rs").is_file());
    assert!(Path::new(&roots.rust).join("inventory.rs").is_file());
    assert!(roots.schema_dir().join("Cargo.toml").is_file());
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
    let first = fixture_api("PairedA", "paired", "serde_json::Value");
    let mut second = fixture_api("PairedB", "paired", "serde_json::Value");
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
    let inventory = fixture_api("Inventory", "inventory", "serde_json::Value");
    let [paired_a, paired_b] = paired_group("FetchThings", "/things", None);
    let registry_for = |_: &str| Some(SchemaRegistry::new());

    let report = generate_batch(
        &[inventory, paired_a, paired_b],
        &BatchScope::Complete,
        &roots.opts(),
        &registry_for,
    );

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
    let mut routes = fixture_api("Routes", "routes", "serde_json::Value");
    routes.endpoints[0].path = "/things/{id}".to_string();
    // Only the path-parameter name differs, so the route is the same.
    let mut twin = routes.endpoints[0].clone();
    twin.id = "FetchThing".to_string();
    twin.path = "/things/{name}".to_string();
    routes.endpoints.push(twin);
    let registry_for = |_: &str| Some(SchemaRegistry::new());

    let report = generate_batch(
        &[routes],
        &BatchScope::Complete,
        &roots.opts(),
        &registry_for,
    );

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
    ParamDef {
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
    let mut api = fixture_api("Located", "located", "serde_json::Value");
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
        &|_: &str| Some(SchemaRegistry::new()),
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
    let mut api = fixture_api("CookieKey", "cookie_key", "serde_json::Value");
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
    let mut api = fixture_api("Located", "located", "serde_json::Value");
    api.endpoints[0].params = Some(place(ParamDef {
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
        assert_eq!(
            diagnostics,
            vec![ValidationDiagnostic::new(
                Severity::Error,
                "Located",
                Some("ListThings"),
                feature,
                Output::Rust,
                format!(
                    "{location} parameter `flags` is an array of arrays, which has no \
                     {location} serialization"
                ),
            )],
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
    let api = api_with_located_type(query, array_of(array_of(QueryParamType::Boolean)));
    let registry_for = |module: &str| (module == "located").then(SchemaRegistry::new);

    let report = generate_batch(&[api], &BatchScope::Complete, &roots.opts(), &registry_for);

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
    let registry_for = |_: &str| Some(SchemaRegistry::new());

    let report = generate_batch(
        &[paired_a, paired_b],
        &BatchScope::Complete,
        &roots.opts(),
        &registry_for,
    );
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

/// The nine shipped endpoints whose path template embeds a query string.
const EMBEDDED_QUERY_ENDPOINTS: [(&str, &str); 9] = [
    ("Gitea", "GetGitTreeRecursive"),
    ("GitLab", "GetRepositoryFile"),
    ("OpenAI", "BetaCancelResponse"),
    ("OpenAI", "BetaCompactconversation"),
    ("OpenAI", "BetaCreateResponse"),
    ("OpenAI", "BetaDeleteResponse"),
    ("OpenAI", "BetaGetResponse"),
    ("OpenAI", "BetaGetinputtokencounts"),
    ("OpenAI", "BetaListInputItems"),
];

#[test]
fn live_definitions_warn_once_per_output_for_each_embedded_query() {
    let apis = resolve_all_apis();
    let refs: Vec<&RestApi> = apis.iter().collect();

    let mut openapi = validate_openapi(&refs);
    let mut modules: Vec<String> = Vec::new();
    for api in &refs {
        let module = schematic_gen::export::resolve_module_name(api);
        if !modules.contains(&module) {
            modules.push(module);
        }
    }
    for module in &modules {
        let members: Vec<&RestApi> = refs
            .iter()
            .copied()
            .filter(|api| &schematic_gen::export::resolve_module_name(api) == module)
            .collect();
        openapi.extend(validate_openapi_registry(
            module,
            &members,
            get_registries_for_module(module).as_ref(),
        ));
    }
    let by_output = [
        (Output::Rust, validate_rust_client(&refs)),
        (Output::OpenApi, openapi),
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
        // Embedded queries are the only accepted gap in any output.
        let other: Vec<_> = diagnostics
            .iter()
            .filter(|d| d.feature != Feature::QueryInPath)
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
        assert_eq!(warned.len(), 9, "{output}: {warned:#?}");
        let warned: BTreeSet<_> = warned.into_iter().collect();
        assert_eq!(warned, expected, "{output}");
        all_lines += 9;
        unique.extend(warned);
    }
    // Nine distinct endpoints, reported once for each of the three outputs.
    assert_eq!(all_lines, 27);
    assert_eq!(unique.len(), 9);
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
fn cli_warnings_alone_exit_successfully() {
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
        let line = format!(
            "severity=warning api=Gitea endpoint=GetGitTreeRecursive feature=query-in-path \
             output={target}: "
        );
        assert_eq!(stderr.matches(&line).count(), 1, "{target}:\n{stderr}");
        assert!(stdout.contains(&format!("[OK] {target} output")), "{stdout}");
    }
    assert!(!stderr.contains("severity=error"), "{stderr}");
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
