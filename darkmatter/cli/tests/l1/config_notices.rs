//! The invocation's configuration on `md`'s result channels: a missing user
//! file is one `dm.config.missing` warning per run (stderr for a formatted
//! output, the envelope's `diagnostics` for a data output), never for help or a
//! usage error; a repository without `.darkmatter.yaml` is normal; a user file
//! that does not load stops a command that reads it with a located error.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use simplified_schema::SimplifiedSchema;
use std::process::Output;

const DOCUMENT: &str = "---\ntitle: T\n---\n# Head\n\nbody\n";
const MISSING: &str = "dm.config.missing";

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A fixture with `doc.md` in its launch directory and no user configuration.
fn without_user_config(name: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name).without_user_config();
    fixture.write_file("cwd/doc.md", DOCUMENT);
    fixture
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

/// Stdout's one JSON value, judged against `schema_text`, with stderr empty.
fn data_value(output: &Output, schema_text: &str) -> Value {
    assert!(output.stderr.is_empty(), "a data output owns stderr too: {}", stderr_of(output));
    let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON value");
    let schema = SimplifiedSchema::try_from(schema_text).expect("the schema file parses");
    if let Err(failure) = schema.validate_strict(&value) {
        panic!("{failure:?}\n{value:#}");
    }
    value
}

fn missing_notices(value: &Value) -> Vec<&Value> {
    value["diagnostics"].as_array().expect("diagnostics").iter().filter(|record| record["code"] == MISSING).collect()
}

/// The formatted warning names the file and the command that creates it.
fn assert_one_missing_warning(fixture: &CliProcessFixture, output: &Output) {
    let stderr = stderr_of(output);
    let path = fixture.home().join(".darkmatter.yaml").display().to_string();
    assert_eq!(stderr.matches("does not exist").count(), 1, "{stderr}");
    assert!(stderr.contains(&path) && stderr.contains("md init"), "{stderr}");
}

#[test]
fn a_composing_run_warns_once_on_stderr() {
    let fixture = without_user_config("config-missing-compose");

    let output = run(&fixture, &["compose", "doc.md"]);

    assert!(output.status.success(), "{}", stderr_of(&output));
    assert_one_missing_warning(&fixture, &output);
}

#[test]
fn a_composing_data_output_carries_the_warning_in_its_diagnostics() {
    let fixture = without_user_config("config-missing-compose-json");

    let output = run(&fixture, &["compose", "doc.md", "--output", "json"]);

    assert!(output.status.success());
    let value = data_value(&output, include_str!("../../schemas/output/compose.yaml"));
    let notices = missing_notices(&value);
    assert_eq!(notices.len(), 1, "{value:#}");
    assert_eq!(notices[0]["severity"], "warning");
    assert_eq!(
        notices[0]["source"]["file"].as_str(),
        Some(fixture.home().join(".darkmatter.yaml").to_str().unwrap())
    );
}

#[test]
fn a_command_that_composes_nothing_warns_too() {
    let fixture = without_user_config("config-missing-hash");

    let formatted = run(&fixture, &["hash", "doc.md"]);
    let data = run(&fixture, &["hash", "doc.md", "--output", "json"]);

    assert!(formatted.status.success(), "{}", stderr_of(&formatted));
    assert_one_missing_warning(&fixture, &formatted);
    let value = data_value(&data, include_str!("../../schemas/output/hash.yaml"));
    assert_eq!(missing_notices(&value).len(), 1, "{value:#}");
}

#[test]
fn help_and_usage_errors_report_no_configuration() {
    let fixture = without_user_config("config-missing-help");

    for args in [&["--help"][..], &["compose", "--help"], &["compose", "doc.md", "--no-such-flag"]] {
        let output = run(&fixture, args);
        assert!(!stderr_of(&output).contains("does not exist"), "{args:?}: {}", stderr_of(&output));
        assert!(!String::from_utf8_lossy(&output.stdout).contains(MISSING), "{args:?}");
    }
}

#[test]
fn a_repository_without_configuration_is_normal() {
    let fixture = CliProcessFixture::named("config-repository-absent");
    fixture.write_file("cwd/doc.md", DOCUMENT);
    if !fixture.initialize_repository_at(fixture.cwd()) {
        return;
    }

    let output = run(&fixture, &["compose", "doc.md"]);

    assert!(output.status.success(), "{}", stderr_of(&output));
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
}

#[test]
fn a_user_file_that_does_not_load_stops_a_command_that_reads_it() {
    let fixture = CliProcessFixture::named("config-user-broken");
    fixture.write_file("cwd/doc.md", DOCUMENT);
    let config = fixture.home().join(".darkmatter.yaml");
    // A key the closed schema does not declare fails the whole file.
    std::fs::write(&config, "register: []\nnot-a-setting: 1\n").unwrap();

    let formatted = run(&fixture, &["compose", "doc.md"]);
    let data = run(&fixture, &["compose", "doc.md", "--output", "json"]);
    let unread = run(&fixture, &["hash", "doc.md"]);

    assert_eq!(formatted.status.code(), Some(1));
    let stderr = stderr_of(&formatted);
    assert!(stderr.contains(&format!("{}:2:", config.display())), "file and line: {stderr}");
    assert!(formatted.stdout.is_empty(), "nothing was composed");
    assert_eq!(data.status.code(), Some(1));
    let value = data_value(&data, include_str!("../../schemas/output/compose.yaml"));
    let error = &value["diagnostics"][0];
    assert_eq!(error["code"], "dm.config.invalid", "{value:#}");
    assert_eq!(error["severity"], "error");
    assert_eq!(error["source"]["file"].as_str(), Some(config.to_str().unwrap()));
    assert_eq!(error["source"]["line"], 2, "{value:#}");
    assert!(error["source"]["column"].is_u64(), "{value:#}");
    assert!(unread.status.success(), "a command that reads no configuration runs: {}", stderr_of(&unread));
}
