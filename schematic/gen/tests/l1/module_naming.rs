//! One module-name resolver names every artifact (R36): the generated Rust
//! module, the OpenAPI document, the Postman collection, the module grouping,
//! and the definitions module `import` writes. Names that differ only by
//! letter case are refused, since a case-insensitive file system would write
//! both to one file.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use schematic_define::{DefinitionContext, DefinitionError, Endpoint, RestApi};
use schematic_definitions::naming::resolve_module_name;
use schematic_definitions::registry::{assemble_module_group, registrations};
use schematic_gen::diagnostics::{Feature, ValidationDiagnostic};
use schematic_gen::import_pipeline::{ImportOptions, run_import};
use schematic_gen::pipeline::{BatchScope, GenerateOpts, OpenApiFormat, generate_batch};
use schematic_gen::validation::{validate_openapi, validate_postman, validate_rust_client};

fn api(ctx: &mut DefinitionContext, name: &str, module_path: Option<&str>) -> Result<RestApi, DefinitionError> {
    let ping = Endpoint::get(format!("{name}Ping"), format!("/{}", name.to_lowercase())).describes("Ping").returns_empty().build()?;
    let builder = RestApi::builder(ctx, name, "https://fixture.test").describes("Fixture").no_auth().endpoints([ping]);
    match module_path {
        Some(path) => builder.module_path(path).build(),
        None => builder.build(),
    }
}

/// Builds one module group's APIs, `(name, explicit module path)` each.
fn group(members: &[(&str, Option<&str>)]) -> Vec<RestApi> {
    assemble_module_group("::fixture", |ctx| members.iter().map(|(name, path)| api(ctx, name, *path)).collect())
        .expect("fixture group builds")
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

fn file_names(dir: &Path) -> BTreeSet<String> {
    fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().into_string().unwrap()).collect()
}

fn stem_errors(diagnostics: &[ValidationDiagnostic]) -> Vec<&ValidationDiagnostic> {
    diagnostics.iter().filter(|d| d.is_error() && d.feature == Feature::InvalidOutputFilename).collect()
}

#[test]
fn every_writer_files_an_api_under_the_one_resolved_stem() {
    // Inferred (`PagerDutyApi`, `PagerDutyClient`: one shared group with no
    // explicit path), fallback (`Widgets`), and explicit (`StatusService`).
    let mut apis = group(&[("PagerDutyApi", None), ("PagerDutyClient", None)]);
    apis.extend(group(&[("Widgets", None)]));
    apis.extend(group(&[("StatusService", Some("status_board"))]));
    let stems: Vec<String> = apis.iter().map(resolve_module_name).collect();
    assert_eq!(stems, ["pagerduty", "pagerduty", "widgets", "status_board"]);

    let temp = tempfile::tempdir().unwrap();
    let path = |relative: &str| temp.path().join(relative).to_str().unwrap().to_owned();
    let (rust, openapi, postman) = (path("schema/src"), path("openapi"), path("postman"));
    generate_batch(&apis, &BatchScope::Complete, &opts(&rust, &openapi, &postman))
        .into_result()
        .expect("fixture batch generates");

    let expected = ["pagerduty", "status_board", "widgets"];
    assert_eq!(file_names(Path::new(&openapi)), expected.iter().map(|stem| format!("{stem}.json")).collect());
    assert_eq!(
        file_names(Path::new(&postman)),
        expected.iter().map(|stem| format!("{stem}.postman_collection.json")).collect()
    );
    let rust_files = file_names(Path::new(&rust));
    for stem in expected {
        assert!(rust_files.contains(&format!("{stem}.rs")), "{stem}.rs missing from {rust_files:?}");
    }
    assert!(
        !rust_files.iter().any(|name| name.starts_with("pagerdutyapi") || name.starts_with("statusservice")),
        "{rust_files:?}"
    );

    // The shared group is one module in every output.
    let module = fs::read_to_string(Path::new(&rust).join("pagerduty.rs")).unwrap();
    assert!(module.contains("pub struct PagerDutyApi") && module.contains("pub struct PagerDutyClient"));
    let lib = fs::read_to_string(Path::new(&rust).join("lib.rs")).unwrap();
    assert!(lib.contains("pub mod pagerduty;") && !lib.contains("pagerdutyapi"), "{lib}");
    let document = fs::read_to_string(Path::new(&openapi).join("pagerduty.json")).unwrap();
    assert!(document.contains("\"/pagerdutyapi\"") && document.contains("\"/pagerdutyclient\""), "{document}");
    let collection = fs::read_to_string(Path::new(&postman).join("pagerduty.postman_collection.json")).unwrap();
    assert!(collection.contains("PagerDutyApiPing") && collection.contains("PagerDutyClientPing"));
}

#[test]
fn module_names_differing_only_by_case_are_refused_by_every_writer() {
    let mut apis = group(&[("Widgets", Some("Widgets"))]);
    apis.extend(group(&[("WidgetsMirror", Some("widgets"))]));
    let refs: Vec<&RestApi> = apis.iter().collect();

    for (output, diagnostics) in [
        ("rust", validate_rust_client(&refs)),
        ("openapi", validate_openapi(&refs)),
        ("postman", validate_postman(&refs)),
    ] {
        let errors = stem_errors(&diagnostics);
        assert_eq!(errors.len(), 1, "{output}: {diagnostics:?}");
        assert_eq!(errors[0].api, ["WidgetsMirror"], "{output}");
        assert!(errors[0].message.contains("`widgets` differs from `Widgets` only by letter case"), "{output}: {}", errors[0].message);
    }

    let temp = tempfile::tempdir().unwrap();
    let path = |relative: &str| temp.path().join(relative).to_str().unwrap().to_owned();
    let (rust, openapi, postman) = (path("schema/src"), path("openapi"), path("postman"));
    assert!(generate_batch(&apis, &BatchScope::Complete, &opts(&rust, &openapi, &postman)).into_result().is_err());
    for root in [&rust, &openapi, &postman] {
        assert!(!Path::new(root).exists() || file_names(Path::new(root)).is_empty(), "{root} was written");
    }

    // Control: distinct names in one case pass.
    let distinct = group(&[("Widgets", Some("widgets"))]);
    let distinct: Vec<&RestApi> = distinct.iter().collect();
    assert!(stem_errors(&validate_openapi(&distinct)).is_empty());
}

#[test]
fn a_module_name_cannot_take_a_generated_file_in_another_case() {
    for taken in ["Lib", "Shared", "ElevenLabs_WS"] {
        let apis = group(&[("Widgets", Some(taken))]);
        let refs: Vec<&RestApi> = apis.iter().collect();
        let diagnostics = validate_rust_client(&refs);
        let errors = stem_errors(&diagnostics);
        assert_eq!(errors.len(), 1, "{taken}: {diagnostics:?}");
        assert!(errors[0].message.contains("would overwrite the generated"), "{taken}: {}", errors[0].message);
    }
}

#[test]
fn import_names_the_definitions_module_with_the_shared_resolver() {
    let temp = tempfile::tempdir().unwrap();
    let spec = temp.path().join("pagerduty.yaml");
    fs::write(
        &spec,
        "openapi: '3.0.3'\ninfo: {title: PagerDuty, version: '1'}\npaths:\n  /incidents:\n    get:\n      operationId: listIncidents\n      responses:\n        '204': {description: none}\n",
    )
    .unwrap();
    let definitions = temp.path().join("definitions");
    let import = |api_name: &str, module_path: Option<&str>| {
        run_import(&ImportOptions {
            input: spec.display().to_string(),
            api_name: Some(api_name.to_owned()),
            module_path: module_path.map(str::to_owned),
            output: temp.path().join("out").display().to_string(),
            definitions_out: Some(definitions.display().to_string()),
            overlay: None,
            exclude_paths: vec![],
            request_id_header: None,
            dry_run: false,
            strict: false,
            verbose: false,
        })
        .expect("import succeeds")
    };

    import("PagerDutyApi", None);
    let module = fs::read_to_string(definitions.join("mod.rs")).expect("mod.rs");
    assert!(module.contains("fn define_pagerduty_api"), "{module}");
    assert!(module.contains("::schematic_definitions::pagerduty") && !module.contains("pagerdutyapi"), "{module}");

    import("PagerDutyApi", Some("pager_duty"));
    let module = fs::read_to_string(definitions.join("mod.rs")).expect("mod.rs");
    assert!(module.contains("fn define_pager_duty_api"), "explicit module path wins: {module}");
}

#[test]
fn every_shipped_artifact_is_named_by_the_resolver() {
    let mut rest_modules = BTreeSet::new();
    for (module, apis) in schematic_definitions::apis_by_module() {
        for api in &apis {
            assert_eq!(resolve_module_name(api), module, "{}", api.name);
        }
        rest_modules.insert(module);
    }
    assert_eq!(rest_modules.len(), 15, "the 18 shipped REST APIs form 15 modules");

    // Committed export names are unchanged by unifying the resolver: each is
    // exactly one resolved stem.
    let openapi = file_names(&biscuit_test_harness::manifest_dir!().join("../openapi"));
    assert_eq!(openapi, rest_modules.iter().map(|stem| format!("{stem}.json")).collect());
    let postman = file_names(&biscuit_test_harness::manifest_dir!().join("../postman"));
    let collections_and_environments = rest_modules
        .iter()
        .flat_map(|stem| [format!("{stem}.postman_collection.json"), format!("{stem}.postman_environment.json")])
        .collect();
    assert_eq!(postman, collections_and_environments);

    let ws_modules = registrations().iter().filter(|entry| entry.websocket().is_some()).map(|entry| entry.module.to_owned());
    let mut modules: BTreeSet<String> = rest_modules.iter().cloned().chain(ws_modules).collect();
    modules.extend(["lib", "prelude", "shared", "ws_shared"].map(str::to_owned));
    let rust: BTreeSet<String> = file_names(&biscuit_test_harness::manifest_dir!().join("../schema/src"))
        .into_iter()
        .map(|name| name.strip_suffix(".rs").map_or(name.clone(), str::to_owned))
        .collect();
    assert_eq!(rust, modules);
}
