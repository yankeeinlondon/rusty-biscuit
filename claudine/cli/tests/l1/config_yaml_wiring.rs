//! The `claudine` binary reads `~/.claudine/config.yaml` through the typed
//! loader: a valid file runs the command, and a file the loader refuses stops
//! it with the file and line on stderr before any provider is launched. The
//! in-process matrix (`claudine::l1 config_schema_matrix`) owns every other
//! cell.

use crate::common;
use common::{CliProcessFixture, launch_recorder, write};

const DOCUMENT: &str = "Say hello.\n";
const RESULT: &str =
    "printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false,\"result\":\"Done\"}'";

/// Runs `claudine compose doc.md --claude` with `config` as the user file;
/// returns `(success, stderr, launches)`.
fn compose_with(name: &str, config: &str) -> (bool, String, Vec<Vec<String>>, CliProcessFixture) {
    let fixture = CliProcessFixture::named(name);
    write(&fixture.home().join(".claudine/config.yaml"), config);
    write(&fixture.cwd().join("doc.md"), DOCUMENT);
    let records = launch_recorder::install(&fixture, "claude", RESULT);
    let output = fixture.command().args(["compose", "doc.md", "--claude", "--silent"]).output().expect("claudine runs");
    let stderr = common::strip_ansi(&String::from_utf8_lossy(&output.stderr));
    (output.status.success(), stderr, launch_recorder::launches(&records), fixture)
}

#[test]
fn a_valid_user_file_runs_the_command() {
    let (success, stderr, launches, _fixture) = compose_with("config-yaml-control", "logging: false\nprotect: false\n");
    assert!(success, "{stderr}");
    assert_eq!(launches.len(), 1, "{stderr}");
}

#[test]
fn a_field_of_the_wrong_type_stops_before_any_launch() {
    let (success, stderr, launches, _fixture) =
        compose_with("config-yaml-field", "protect: false\nlogging: \"yes\"\n");
    assert!(!success);
    assert!(stderr.contains("config.yaml:2:"), "the file and line are reported: {stderr}");
    assert!(stderr.contains("/logging"), "{stderr}");
    assert!(launches.is_empty(), "nothing was launched: {launches:?}");
}

#[test]
fn a_syntax_error_stops_before_any_launch() {
    let (success, stderr, launches, _fixture) = compose_with("config-yaml-syntax", "protect: false\nlogging: [\n");
    assert!(!success);
    assert!(stderr.contains("config.yaml:"), "the file and position are reported: {stderr}");
    assert!(launches.is_empty(), "nothing was launched: {launches:?}");
}

#[test]
fn a_legacy_json_file_is_converted_once_with_a_notice() {
    let fixture = CliProcessFixture::named("config-yaml-legacy");
    let dir = fixture.home().join(".claudine");
    write(&dir.join("config.json"), "{\n  // mine\n  logging: false,\n  protect: false,\n}\n");
    write(&fixture.cwd().join("doc.md"), DOCUMENT);
    let records = launch_recorder::install(&fixture, "claude", RESULT);

    let output = fixture.command().args(["compose", "doc.md", "--claude", "--silent"]).output().expect("claudine runs");
    let stderr = common::strip_ansi(&String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success(), "{stderr}");
    assert!(stderr.contains("JSON5 comments were not carried over"), "{stderr}");
    assert!(stderr.contains("dm.config.migration"), "{stderr}");
    assert_eq!(launch_recorder::launches(&records).len(), 1);
    assert!(dir.join("config.yaml").is_file());
    assert!(dir.join("config.json.bak").is_file());
    assert!(!dir.join("config.json").exists());

    let again = fixture.command().args(["compose", "doc.md", "--claude", "--silent"]).output().expect("claudine runs");
    let stderr = common::strip_ansi(&String::from_utf8_lossy(&again.stderr));
    assert!(again.status.success(), "{stderr}");
    assert!(!stderr.contains("dm.config.migration"), "the notice is emitted once: {stderr}");
}
