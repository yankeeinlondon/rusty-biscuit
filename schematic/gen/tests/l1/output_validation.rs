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
use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};
use schematic_definitions::registry::{SchemaRegistry, get_registries_for_module};
use schematic_gen::diagnostics::{Feature, Output, Severity, ValidationDiagnostic};
use schematic_gen::errors::GeneratorError;
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
