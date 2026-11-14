//! Host registration through `md`: a user registration in the fixture home's
//! `~/.darkmatter.yaml` is loaded once per invocation, the host that
//! identifies a document layers its triggers into `md schema validate`,
//! `md clean`, `md compose`, and `md schema triggers`, and a registration
//! that does not load is one `dm.host.registration` warning per run (stderr
//! for a formatted output, `diagnostics` for a data output, whose stderr
//! stays empty).

use crate::common;
use crate::host_fixtures::{GADGET_TRIGGER, GLOBALS, IDENTIFY, LIBRARY, MANIFEST, WIDGET_TRIGGER, lifecycle_artifact_for};

use common::CliProcessFixture;
use serde_json::Value;
use simplified_schema::SimplifiedSchema;
use std::process::Output;

/// Identified by `widget`; `size: big` breaks the host's `size: number`.
const WIDGET_DOCUMENT: &str = "---\nwidget:\n  size: big\n---\n# Widget\n\nBody.\n";
/// No host identifies it.
const NOTES: &str = "---\ntitle: Notes\n---\n# Notes\n\nBody.\n";
const REGISTRATION: &str = "dm.host.registration";

/// A fixture whose home registers the acme host (installed under
/// `home/.acme/`, with no function catalogs) and, when `missing`, a second
/// entry naming no manifest.
fn registered(name: &str, missing: bool) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    let manifest = MANIFEST.replace(
        "functions:\n    - catalog: ./schemas/functions.yaml\n      facts: ./schemas/functions.facts.json\n",
        "functions: []\n",
    );
    for (file, text) in [
        ("darkmatter-host.yaml", manifest.as_str()),
        ("schemas/identify.yaml", IDENTIFY),
        ("schemas/widget.yaml", WIDGET_TRIGGER),
        ("schemas/gadget.yaml", GADGET_TRIGGER),
        ("schemas/library.yaml", LIBRARY),
        ("schemas/globals.yaml", GLOBALS),
        ("schemas/globals.lifecycle.json", &lifecycle_artifact_for("acme", GLOBALS)),
    ] {
        fixture.write_file(format!("home/.acme/{file}"), text);
    }
    let missing = if missing { "  - ./.missing/darkmatter-host.yaml\n" } else { "" };
    fixture.write_file("home/.darkmatter.yaml", &format!("register:\n  - ./.acme/darkmatter-host.yaml\n{missing}"));
    fixture.write_file("cwd/widget.md", WIDGET_DOCUMENT);
    fixture.write_file("cwd/notes.md", NOTES);
    fixture
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
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

fn registration_records(diagnostics: &Value) -> usize {
    diagnostics.as_array().expect("diagnostics").iter().filter(|record| record["code"] == REGISTRATION).count()
}

#[test]
fn schema_validate_applies_the_registered_schema_of_an_identified_document() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-validate", false);

    let output = run(&fixture, &["schema", "validate", "--output", "json", "widget.md", "notes.md"]);

    assert_eq!(output.status.code(), Some(1), "the widget is invalid: {}", String::from_utf8_lossy(&output.stdout));
    let value = data_value(&output, include_str!("../../schemas/output/schema-validate.yaml"));
    let files = value["files"].as_array().expect("one record per file");
    assert_eq!(files[0]["valid"], false, "{value:#}");
    let problems: Vec<&str> = files[0]["problems"].as_array().unwrap().iter().filter_map(|problem| problem["path"].as_str()).collect();
    assert_eq!(problems, ["/widget/size"], "{value:#}");
    assert_eq!(files[1]["valid"], true, "no host judges the notes: {value:#}");
}

#[test]
fn compose_applies_the_registered_schema_and_composes_unrelated_documents() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-compose", false);

    let widget = run(&fixture, &["compose", "widget.md"]);
    let notes = run(&fixture, &["compose", "notes.md"]);

    assert_eq!(widget.status.code(), Some(1), "{}", stderr_of(&widget));
    assert!(stderr_of(&widget).contains("size"), "{}", stderr_of(&widget));
    assert!(notes.status.success(), "{}", stderr_of(&notes));
    assert!(notes.stderr.is_empty(), "{}", stderr_of(&notes));
}

#[test]
fn clean_judges_an_identified_document_against_the_registered_schema() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-clean", false);

    let output = run(&fixture, &["clean", "widget.md", "--output", "json"]);

    let value = data_value(&output, include_str!("../../schemas/output/clean.yaml"));
    assert!(value.to_string().contains("size"), "the host's `widget.size` is reported: {value:#}");
}

#[test]
fn schema_triggers_reports_the_registered_origin() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-triggers", false);
    if !fixture.initialize_repository_at(fixture.cwd()) {
        return;
    }

    let data = run(&fixture, &["schema", "triggers", "widget.md", "--output", "json"]);
    let formatted = run(&fixture, &["schema", "triggers", "widget.md"]);

    let value = data_value(&data, include_str!("../../schemas/output/schema-triggers.yaml"));
    let registered = &value["registered"];
    assert_eq!(registered["host"], "acme", "{value:#}");
    assert_eq!(registered["version"], "1.2.0");
    assert!(registered["manifest"].as_str().is_some_and(|path| path.ends_with("darkmatter-host.yaml")), "{value:#}");
    assert!(registered["revision"].as_str().is_some_and(|revision| !revision.is_empty()));
    let matched: Vec<bool> =
        registered["triggers"].as_array().unwrap().iter().map(|trigger| trigger["matched"].as_bool().unwrap()).collect();
    assert_eq!(matched, [true, false], "widget matched, gadget did not: {value:#}");
    let stdout = String::from_utf8_lossy(&formatted.stdout);
    assert!(formatted.status.success(), "{}", stderr_of(&formatted));
    assert!(stdout.contains("Registered host") && stdout.contains("acme"), "{stdout}");
}

#[test]
fn a_registration_that_does_not_load_warns_once_in_a_formatted_run() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-warning-formatted", true);

    let output = run(&fixture, &["schema", "validate", "notes.md", "widget.md"]);

    let stderr = stderr_of(&output);
    assert_eq!(stderr.matches("is not loaded").count(), 1, "one warning: {stderr}");
    assert!(stderr.contains(".missing/darkmatter-host.yaml"), "{stderr}");
    let text = format!("{}{stderr}", String::from_utf8_lossy(&output.stdout));
    assert!(text.contains("size"), "the loaded host still applies: {text}");
}

#[test]
fn a_registration_that_does_not_load_is_one_record_in_a_compose_data_output() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-warning-compose-json", true);

    let output = run(&fixture, &["compose", "notes.md", "--output", "json"]);

    assert!(output.status.success());
    let value = data_value(&output, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(registration_records(&value["diagnostics"]), 1, "{value:#}");
}

#[test]
fn a_registration_that_does_not_load_is_one_record_in_a_validate_data_output() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-warning-validate-json", true);

    let output = run(&fixture, &["schema", "validate", "notes.md", "--output", "json"]);

    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stdout));
    let value = data_value(&output, include_str!("../../schemas/output/schema-validate.yaml"));
    assert_eq!(registration_records(&value["diagnostics"]), 1, "{value:#}");
}

#[test]
fn selecting_a_host_never_migrates_a_repository_legacy_policy_file() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-passive-repository", false);
    let legacy = fixture.write_file("cwd/.darkmatter-shell-whitelist", "prefix ls\n");
    if !fixture.initialize_repository_at(fixture.cwd()) || !fixture.commit_all(fixture.cwd()) {
        return;
    }

    let validate = run(&fixture, &["schema", "validate", "widget.md"]);
    let triggers = run(&fixture, &["schema", "triggers", "widget.md"]);

    assert_eq!(validate.status.code(), Some(1), "the host still applies: {}", stderr_of(&validate));
    assert!(triggers.status.success(), "{}", stderr_of(&triggers));
    assert!(legacy.exists(), "the legacy file is left in place");
    assert!(!fixture.cwd().join(".darkmatter.yaml").exists(), "no repository configuration is written");
}
