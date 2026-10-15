//! Pagination validation through the generator's public validators and its
//! batch pipeline.
//!
//! The define-side rules arrive as `pagination-*` features on the Rust
//! client only; item types and item locations are checked against an API's
//! schema registry where it can describe them.

use std::path::Path;

use schematic_define::pagination::{
    Collection, CursorSource, LinkSource, MoreSignal, PageSize, PaginationException, Partition, Session, Strategy, ValueSource,
};
use schematic_define::{ApiResponse, AuthStrategy, CrossOrigin, Endpoint, EndpointParams, QueryParamType, RestApi, RestMethod};
use schematic_definitions::registry::SchemaRegistry;
use schematic_gen::diagnostics::{Feature, Output, Severity};
use schematic_gen::pipeline::{BatchScope, GenerateOpts, OpenApiFormat, generate_batch, resolve_all_apis};
use schematic_gen::validation::{validate_openapi, validate_postman, validate_rust_client};

fn endpoint(id: &str, path: &str, response: &str) -> Endpoint {
    Endpoint {
        id: id.to_string(),
        method: RestMethod::Get,
        path: path.to_string(),
        description: String::new(),
        request: None,
        response: ApiResponse::json_type(response),
        headers: vec![],
        params: None,
        oauth_scopes: None, cross_origin: None, pagination: None,
    }
}

fn api(name: &str, endpoints: Vec<Endpoint>) -> RestApi {
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
        endpoints,
        module_path: Some(name.to_lowercase()),
        request_suffix: None,
        version: None,
        cross_origin: CrossOrigin::default(),
        request_id_header: None,
        secret_names: vec![],
        default_pagination: None,
        pagination_exceptions: vec![],
    }
}

fn page_default() -> Strategy {
    Strategy::page_number("page", MoreSignal::link_rel("next"))
        .page_size(PageSize::new("per_page").max(100))
        .build()
}

/// The rendered pagination diagnostics of the Rust client.
fn pagination_lines(api: &RestApi) -> Vec<String> {
    validate_rust_client(&[api])
        .into_iter()
        .filter(|d| {
            matches!(
                d.feature,
                Feature::PaginationStrategy | Feature::PaginationException | Feature::PaginationParameter
            )
        })
        .map(|d| d.to_string())
        .collect()
}

#[test]
fn each_rule_group_renders_as_its_feature_on_the_rust_client_only() {
    let mut forge = api("Forge", vec![endpoint("ListReleases", "/releases", "Vec<Release>")]);
    forge.default_pagination = Some(Strategy::cursor("after", CursorSource::last_item("id")).build());
    forge.pagination_exceptions = vec![PaginationException::new("ListReleaze", Collection::root().unpaginated())];

    assert_eq!(
        pagination_lines(&forge),
        vec![
            "severity=error api=Forge endpoint=<none> feature=pagination-strategy output=rust: \
             cursor-without-end-signal: a cursor read only from items has no end signal; add \
             `more_when(..)`"
                .to_string(),
            "severity=error api=Forge endpoint=ListReleaze feature=pagination-exception \
             output=rust: unknown-exception-id: exception names `ListReleaze`, which is not an \
             endpoint of this API; did you mean `ListReleases`?"
                .to_string(),
        ]
    );
    assert!(validate_rust_client(&[&forge]).iter().all(|d| d.severity == Severity::Error && d.output == Output::Rust));
    assert_eq!(validate_openapi(&[&forge]), vec![]);
    assert_eq!(validate_postman(&[&forge]), vec![]);

    let mut files = endpoint("ListFiles", "/files", "Vec<File>");
    files.params = Some(EndpointParams::default().with_query_param("after", QueryParamType::String, false, None));
    let mut adopting = api("Files", vec![files]);
    adopting.default_pagination = Some(Strategy::cursor("after", CursorSource::body("next")).build());
    assert_eq!(
        pagination_lines(&adopting),
        vec![
            "severity=error api=Files endpoint=ListFiles feature=pagination-parameter output=rust: \
             adoption-without-knob: strategy would adopt declared parameter `after` without \
             authoring the knob that replaces it; author the knob or leave the parameter out of \
             the strategy"
                .to_string()
        ]
    );
}

#[test]
fn item_type_must_be_rust_type_syntax() {
    let mut forge = api("Forge", vec![endpoint("ListRuns", "/runs", "RunsResponse")]);
    forge.default_pagination = Some(page_default());
    forge.pagination_exceptions = vec![PaginationException::new("ListRuns", Collection::at("runs").item_type("Vec<Run"))];
    assert_eq!(
        pagination_lines(&forge),
        vec![
            "severity=error api=Forge endpoint=ListRuns feature=pagination-exception output=rust: \
             invalid-item-type: item type `Vec<Run` is not valid Rust type syntax"
                .to_string()
        ]
    );
    // Without a registry, a well-formed name is left to the compiler.
    forge.pagination_exceptions = vec![PaginationException::new("ListRuns", Collection::at("runs").item_type("Run"))];
    assert_eq!(pagination_lines(&forge), Vec::<String>::new());
}

/// GitHub's real registry: `WorkflowRunsResponse` is registered top-level and
/// `WorkflowRun` only as a nested schema.
fn github_runs(collection: Collection) -> RestApi {
    let mut github = api(
        "GitHub",
        vec![endpoint("ListWorkflowRuns", "/repos/{owner}/{repo}/actions/runs", "WorkflowRunsResponse")],
    );
    github.default_pagination = Some(page_default());
    github.pagination_exceptions = vec![PaginationException::new("ListWorkflowRuns", collection)];
    github
}

#[test]
fn item_types_resolve_through_the_registry_including_nested_schemas() {
    for accepted in ["WorkflowRun", "serde_json::Value", "String", "u64"] {
        let github = github_runs(Collection::at("workflow_runs").item_type(accepted));
        assert_eq!(pagination_lines(&github), Vec::<String>::new(), "{accepted}");
    }
    let github = github_runs(Collection::at("workflow_runs").item_type("WorkflowRunz"));
    assert_eq!(
        pagination_lines(&github),
        vec![
            "severity=error api=GitHub endpoint=ListWorkflowRuns feature=pagination-exception \
             output=rust: unknown-item-type: item type `WorkflowRunz` is not a schema of this \
             API's registry; use a registered type, a primitive, or a qualified path such as \
             `serde_json::Value`"
                .to_string()
        ]
    );
}

#[test]
fn item_location_is_checked_against_the_registered_response_schema() {
    let github = github_runs(Collection::at("total_count").item_type("WorkflowRun"));
    assert_eq!(
        pagination_lines(&github),
        vec![
            "severity=error api=GitHub endpoint=ListWorkflowRuns feature=pagination-exception \
             output=rust: not-an-array-location: `total_count[*]` is not an array in the \
             registered schema of `WorkflowRunsResponse`"
                .to_string()
        ]
    );
    // A key the schema does not describe cannot be judged; the runtime checks it.
    let unknown = github_runs(Collection::at("runs").item_type("WorkflowRun"));
    assert_eq!(pagination_lines(&unknown), Vec::<String>::new());
}

#[test]
fn live_definitions_report_no_pagination_diagnostics() {
    let apis = resolve_all_apis();
    let refs: Vec<&RestApi> = apis.iter().collect();
    assert_eq!(refs.len(), 18, "shipped REST APIs");
    let pagination: Vec<String> = validate_rust_client(&refs)
        .into_iter()
        .filter(|d| {
            matches!(
                d.feature,
                Feature::PaginationStrategy | Feature::PaginationException | Feature::PaginationParameter
            )
        })
        .map(|d| d.to_string())
        .collect();
    assert_eq!(pagination, Vec::<String>::new());
}

#[test]
fn a_pagination_error_blocks_only_the_rust_client() {
    let temp = tempfile::tempdir().expect("temp dir");
    let path = |relative: &str| temp.path().join(relative).to_str().unwrap().to_string();
    let (rust, openapi, postman) = (path("schema/src"), path("openapi"), path("postman"));
    let opts = GenerateOpts {
        output: &rust,
        dry_run: false,
        verbose: 0,
        openapi_out: Some(&openapi),
        openapi_format: OpenApiFormat::Json,
        openapi_version: None,
        postman_out: Some(&postman),
        no_openapi: false,
        no_postman: false,
    };
    let mut forge = api("Forge", vec![endpoint("ListReleases", "/releases", "Vec<Release>")]);
    forge.default_pagination = Some(Strategy::next_link(LinkSource::link_header("")).build());
    let registry_for = |_: &str| Some(SchemaRegistry::new().register::<serde_json::Value>("Vec<Release>"));

    let report = generate_batch(&[forge], &BatchScope::Complete, &opts, &registry_for);

    assert_eq!(report.failed(), vec![Output::Rust]);
    assert_eq!(report.succeeded(), vec![Output::OpenApi, Output::Postman]);
    let rust_report = report.get(Output::Rust).unwrap();
    assert_eq!(rust_report.diagnostics.len(), 1);
    assert_eq!(rust_report.diagnostics[0].feature, Feature::PaginationStrategy);
    assert!(rust_report.diagnostics[0].message.starts_with("empty-value: link relation is empty"));
    assert!(!Path::new(&rust).join("lib.rs").exists());
    assert!(Path::new(&openapi).join("forge.json").is_file());
    assert!(Path::new(&postman).join("forge.postman_collection.json").is_file());
}

#[test]
fn partitioned_first_page_sessions_report_a_strategy_error() {
    let mut search = api("Search", vec![endpoint("Search", "/search", "Vec<Hit>")]);
    search.default_pagination = Some(
        Strategy::cursor("after", CursorSource::body("next"))
            .session(Session::opened_by_first_page()
                .handle_from_page(ValueSource::body("session"))
                .send_handle_as("session"))
            .partition(Partition::new("slice", "slices").max(4)),
    );
    assert_eq!(pagination_lines(&search), vec![
        "severity=error api=Search endpoint=<none> feature=pagination-strategy output=rust: \
         partitioned-first-page-session: partitions require a session opened by a separate request; \
         first-page opening creates independent sessions without shared cleanup".to_string(),
    ]);

    let temp = tempfile::tempdir().expect("temp dir");
    let output = temp.path().join("schema");
    let output = output.to_str().unwrap();
    let opts = GenerateOpts {
        output,
        dry_run: false,
        verbose: 0,
        openapi_out: None,
        openapi_format: OpenApiFormat::Json,
        openapi_version: None,
        postman_out: None,
        no_openapi: true,
        no_postman: true,
    };
    let registry_for = |_: &str| Some(SchemaRegistry::new().register::<serde_json::Value>("Vec<Hit>"));
    let report = generate_batch(&[search], &BatchScope::Complete, &opts, &registry_for);
    assert_eq!(report.failed(), vec![Output::Rust]);
    assert!(!Path::new(output).join("lib.rs").exists());
}
