//! The AsyncAPI import persists its definition, model catalog, and report
//! through the staged publication every output uses (R26), and the CLI
//! commands are wired to the library.

use std::fs;
use std::path::Path;
use std::process::Command;

use schematic_define::websocket::WebSocketApi;
use schematic_gen::asyncapi_import::{
    AsyncImportOptions, WsRole, definition_file_name, models_file_name, read_import_output, report_file_name,
    run_import_asyncapi,
};
use schematic_gen::errors::GeneratorError;
use schematic_gen::export::asyncapi::render_asyncapi;

use crate::common::asyncapi::{canonical, fixture_api, subset_projection};

const MODULE: &str = "fixture_stream";

/// Writes the fixture's exported document into `dir`.
fn document(dir: &Path, api: &WebSocketApi) -> String {
    let path = dir.join("fixture.asyncapi.json");
    fs::write(&path, render_asyncapi(api).unwrap()).unwrap();
    path.to_string_lossy().into_owned()
}

fn options(input: String, output: &Path, dry_run: bool) -> AsyncImportOptions {
    AsyncImportOptions {
        input,
        api_name: None,
        module_path: Some(MODULE.to_string()),
        ws_role: WsRole::Client,
        output: output.to_string_lossy().into_owned(),
        dry_run,
        strict: false,
        verbose: false,
    }
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = match fs::read_dir(dir) {
        Ok(entries) => entries.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

#[test]
fn saved_output_reads_back_as_the_imported_definition_and_catalog() {
    let temp = tempfile::tempdir().unwrap();
    let input = document(temp.path(), &fixture_api());
    let output = temp.path().join("out/nested");
    let result = run_import_asyncapi(&options(input, &output, false)).unwrap();

    let written: Vec<String> = result.outputs.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect();
    assert_eq!(written, [definition_file_name(MODULE), models_file_name(MODULE), report_file_name(MODULE)]);
    let (api, models) = read_import_output(&output, MODULE).unwrap();
    assert_eq!(api, result.ws_api);
    assert_eq!(models, result.models);
    assert_eq!(canonical(api), subset_projection(&fixture_api(), MODULE));
    assert!(models.types.iter().any(|model| format!("{model:?}").contains("FeedEvent")));

    let report = fs::read_to_string(output.join(report_file_name(MODULE))).unwrap();
    assert!(report.starts_with("api_name=FixtureStream\nendpoint_count=3\n"), "{report}");
    // INFO diagnostics of the import are kept in the report.
    for diagnostic in &result.diagnostics {
        assert!(report.contains(&diagnostic.location), "{report}");
    }
}

/// Write, read, export, and import again: the persisted definition is
/// reusable, and a second pass changes nothing.
#[test]
fn persisted_definitions_survive_a_second_round_trip() {
    let temp = tempfile::tempdir().unwrap();
    let first = temp.path().join("first");
    run_import_asyncapi(&options(document(temp.path(), &fixture_api()), &first, false)).unwrap();
    let (saved, _) = read_import_output(&first, MODULE).unwrap();

    let second = temp.path().join("second");
    let again = temp.path().join("again");
    fs::create_dir(&again).unwrap();
    run_import_asyncapi(&options(document(&again, &saved), &second, false)).unwrap();
    let (reread, _) = read_import_output(&second, MODULE).unwrap();
    assert_eq!(canonical(reread), canonical(saved));
    // Regenerating into the same root overwrites its own files.
    run_import_asyncapi(&options(document(&again, &fixture_api()), &second, false)).unwrap();
    assert_eq!(entries(&second).len(), 3);
}

#[test]
fn a_dry_run_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let input = document(temp.path(), &fixture_api());
    let output = temp.path().join("never");
    let result = run_import_asyncapi(&options(input, &output, true)).unwrap();
    assert!(result.outputs.is_empty());
    assert_eq!(result.endpoint_count, 3);
    assert!(!output.exists());
}

/// A planned path the generator cannot own fails before any write.
#[test]
fn a_conflicting_path_fails_before_anything_is_written() {
    let temp = tempfile::tempdir().unwrap();
    let input = document(temp.path(), &fixture_api());
    let output = temp.path().join("out");
    fs::create_dir_all(output.join(models_file_name(MODULE))).unwrap();
    let error = run_import_asyncapi(&options(input, &output, false)).unwrap_err();
    assert!(matches!(error, GeneratorError::UnsafeOutput { .. }), "{error}");
    assert!(error.to_string().contains("is a directory where a file is planned"), "{error}");
    assert_eq!(entries(&output), [models_file_name(MODULE)]);
}

#[test]
fn a_failed_import_writes_nothing() {
    let temp = tempfile::tempdir().unwrap();
    let input = temp.path().join("bad.yaml");
    fs::write(&input, "asyncapi: 3.1.0\ninfo: {title: X, version: '1'}\nchannels: {}\n").unwrap();
    let output = temp.path().join("out");
    let error = run_import_asyncapi(&options(input.to_string_lossy().into_owned(), &output, false)).unwrap_err();
    assert!(error.to_string().contains("[ERROR] #/asyncapi: Unsupported AsyncAPI version '3.1.0'"), "{error}");
    assert!(!output.exists());

    // A module path is a file-name component, never a path.
    let input = document(temp.path(), &fixture_api());
    let mut escaping = options(input, &output, false);
    escaping.module_path = Some("../escape".into());
    assert!(matches!(run_import_asyncapi(&escaping), Err(GeneratorError::ConfigError(_))));
    assert!(!output.exists());
}

fn schematic_gen() -> Command {
    Command::new(env!("CARGO_BIN_EXE_schematic-gen"))
}

/// The CLI wiring: `export-asyncapi` writes one document per WebSocket API,
/// and `import-asyncapi` persists one, or reports the failing location and
/// writes nothing.
#[test]
fn cli_commands_export_and_import() {
    let temp = tempfile::tempdir().unwrap();
    let exported = temp.path().join("asyncapi");
    let status = schematic_gen().args(["export-asyncapi", "--output"]).arg(&exported).output().unwrap();
    assert!(status.status.success(), "{}", String::from_utf8_lossy(&status.stderr));
    assert_eq!(entries(&exported).len(), 5);

    let imported = temp.path().join("imported");
    let output = schematic_gen()
        .args(["import-asyncapi", "--module-path", "dock", "--input"])
        .arg(exported.join("unfolded_circle_dock_ws.json"))
        .arg("--output")
        .arg(&imported)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let (api, _) = read_import_output(&imported, "dock").unwrap();
    assert_eq!(api.name, "UnfoldedCircleDockWs");

    let invalid = temp.path().join("invalid.json");
    let text = fs::read_to_string(exported.join("elevenlabs_ws.json")).unwrap().replacen("\"wss\"", "\"https\"", 1);
    fs::write(&invalid, text).unwrap();
    let rejected = temp.path().join("rejected");
    let output = schematic_gen()
        .args(["import-asyncapi", "--input"])
        .arg(&invalid)
        .arg("--output")
        .arg(&rejected)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("[ERROR] #/servers/default/protocol: Protocol 'https' is not `ws` or `wss`"), "{stderr}");
    assert!(!rejected.exists());
}
