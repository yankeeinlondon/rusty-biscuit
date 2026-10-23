//! Definition bundles of every shipped REST API, and the `bundle` and
//! `export-bundle` commands. Loading reads only the bundle text; the registry
//! builds the expected values.

use std::path::Path;
use std::process::Command;

use biscuit_test_harness::bin_exe;
use schematic_define::openapi::{ExportFormat, ExportOptions};
use schematic_define::{DefinitionBundle, RestApi};
use schematic_definitions::naming::resolve_module_name;
use schematic_gen::export::openapi::{build_openapi_grouped, render_openapi_document};
use schematic_gen::export::postman::render_postman_module;

fn exports(api: &RestApi) -> (String, String) {
    let module = resolve_module_name(api);
    let options = ExportOptions::new().with_version(api.version.as_deref().unwrap_or("0.1.0"));
    let document = build_openapi_grouped(&[api], &module, &options).expect("OpenAPI");
    (render_openapi_document(&document, ExportFormat::Json).expect("render"), render_postman_module(&[api], &module).expect("Postman"))
}

#[test]
fn every_shipped_bundle_round_trips_through_a_file_and_exports_like_the_registry() {
    let apis = schematic_definitions::all_apis();
    assert!(!apis.is_empty());
    let dir = tempfile::tempdir().unwrap();
    for api in &apis {
        let path = dir.path().join(format!("{}.json", api.name));
        let json = DefinitionBundle::new(api.clone()).unwrap_or_else(|e| panic!("{}: {e}", api.name)).to_json();
        std::fs::write(&path, &json).unwrap();
        let loaded = DefinitionBundle::from_json(&std::fs::read_to_string(&path).unwrap())
            .unwrap_or_else(|e| panic!("{}: {e}", api.name));
        assert!(loaded.api() == api, "{}: loaded bundle differs from its definition", api.name);
        assert!(loaded.to_json() == json, "{}: canonical JSON is not byte-stable", api.name);
        assert!(exports(loaded.api()) == exports(api), "{}: isolated export differs from the registry export", api.name);
    }
}

fn run_cli(args: &[&str]) -> std::process::Output {
    Command::new(bin_exe!("schematic-gen")).args(args).env("NO_COLOR", "1").output().expect("run schematic-gen")
}

fn path(path: &Path) -> &str { path.to_str().unwrap() }

#[test]
fn bundle_then_export_bundle_reproduces_the_committed_artifacts() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("github.json");
    let (openapi, postman) = (dir.path().join("openapi"), dir.path().join("postman"));
    let written = run_cli(&["bundle", "--api", "github", "--out", path(&bundle)]);
    assert!(written.status.success(), "{}", String::from_utf8_lossy(&written.stderr));
    let exported = run_cli(&["export-bundle", "--input", path(&bundle), "--openapi-out", path(&openapi), "--postman-out", path(&postman)]);
    assert!(exported.status.success(), "{}", String::from_utf8_lossy(&exported.stderr));
    assert_eq!(std::fs::read_to_string(openapi.join("github.json")).unwrap(), include_str!("../../../openapi/github.json"));
    assert_eq!(std::fs::read_to_string(postman.join("github.postman_collection.json")).unwrap(),
        include_str!("../../../postman/github.postman_collection.json"));
}

#[test]
fn export_bundle_reports_a_located_error_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let bundle = dir.path().join("bundle.json");
    let openapi = dir.path().join("openapi");
    let written = run_cli(&["bundle", "--api", "eversolo", "--out", path(&bundle)]);
    assert!(written.status.success(), "{}", String::from_utf8_lossy(&written.stderr));
    let text = std::fs::read_to_string(&bundle).unwrap();
    let cases = [
        (text.replacen("\"format_version\": 1", "\"format_version\": 1,\n  \"format_version\": 1", 1), "duplicate key \"format_version\" at line"),
        (text.replacen("\"cross_origin\": \"SameOrigin\"", "\"cross_origin\": \"SameOrigin\",\n    \"request_id_header\": null", 1), "bundle field /api/request_id_header: explicit null"),
    ];
    for (edited, expected) in cases {
        std::fs::write(&bundle, edited).unwrap();
        let output = run_cli(&["export-bundle", "--input", path(&bundle), "--openapi-out", path(&openapi)]);
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!output.status.success() && stderr.contains("definition bundle") && stderr.contains(expected), "{stderr}");
        assert!(!openapi.exists(), "a bundle that does not load must write nothing");
    }
}
