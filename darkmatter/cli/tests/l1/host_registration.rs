//! Host registration through `md`: a user registration in the fixture home's
//! `~/.darkmatter.yaml` names a format 2 `darkmatter-host` bundle, loaded
//! once per invocation; the host one of whose triggers matches a document
//! layers its triggers into `md schema validate`, `md clean`, `md compose`,
//! and `md schema triggers`, and a registration that does not load (missing,
//! format 1, or a member that fails its digest) is one located
//! `dm.host.registration` warning per run (stderr for a formatted output,
//! `diagnostics` for a data output, whose stderr stays empty) and no host
//! applies. A whole YAML document whose `kind` a registered host defines is
//! judged by that definition, and two matched triggers of one host that
//! disagree on a property are one `dm.host.trigger_conflict` record naming
//! both trigger files.

use crate::common;
use crate::host_fixtures::{Bundle, MANIFEST_NAME};

use common::CliProcessFixture;
use serde_json::Value;
use simplified_schema::SimplifiedSchema;
use std::process::Output;

/// Matched by the `widget` trigger; `size: big` breaks the host's `size: number`.
const WIDGET_DOCUMENT: &str = "---\nwidget:\n  size: big\n---\n# Widget\n\nBody.\n";
/// No host's trigger matches it.
const NOTES: &str = "---\ntitle: Notes\n---\n# Notes\n\nBody.\n";
const REGISTRATION: &str = "dm.host.registration";

/// The `acme` bundle without its function pair: `md` has no handler for a
/// host function, and none of these tests calls one.
fn acme() -> Bundle {
    let mut bundle = Bundle::acme();
    for (catalog, facts) in std::mem::take(&mut bundle.functions) {
        bundle.members.remove(&catalog);
        bundle.members.remove(&facts);
    }
    bundle
}

/// A fixture whose home registers `bundle` (installed under `home/.acme/`)
/// and, when `missing`, a second entry naming no manifest.
fn registered_bundle(name: &str, bundle: &Bundle, missing: bool) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    for (file, text) in bundle.files_with_manifest() {
        fixture.write_file(format!("home/.acme/{file}"), &text);
    }
    let missing = if missing { "  - ./.missing/darkmatter-host.yaml\n" } else { "" };
    fixture.write_file("home/.darkmatter.yaml", &format!("register:\n  - ./.acme/{MANIFEST_NAME}\n{missing}"));
    fixture.write_file("cwd/widget.md", WIDGET_DOCUMENT);
    fixture.write_file("cwd/notes.md", NOTES);
    fixture
}

/// A fixture whose home registers the acme host and, when `missing`, a
/// second entry naming no manifest.
fn registered(name: &str, missing: bool) -> CliProcessFixture {
    registered_bundle(name, &acme(), missing)
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
    // Bundle-path order: `schemas/gadget.yaml` before `schemas/widget.yaml`.
    let matched: Vec<bool> =
        registered["triggers"].as_array().unwrap().iter().map(|trigger| trigger["matched"].as_bool().unwrap()).collect();
    assert_eq!(matched, [false, true], "widget matched, gadget did not: {value:#}");
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

// ── loader wiring: one valid format 2 bundle, two located refusals ──────────

/// A format 1 manifest over the same members: `identify`, `triggers`, and
/// `globals` instead of `files` and `digests`.
const FORMAT_1_MANIFEST: &str = "\
kind: darkmatter-host
format_version: 1
host: acme
version: 1.0.0
identify: ./schemas/widget.yaml
triggers:
    - ./schemas/widget.yaml
globals: ./schemas/run.yaml
";

/// `md compose --output json` of `widget.md`: one spawn.
fn compose_widget(fixture: &CliProcessFixture) -> (Option<i32>, Value) {
    let output = run(fixture, &["compose", "widget.md", "--output", "json"]);
    (output.status.code(), data_value(&output, include_str!("../../schemas/output/compose.yaml")))
}

fn records<'v>(value: &'v Value, code: &str) -> Vec<&'v Value> {
    value["diagnostics"].as_array().expect("diagnostics").iter().filter(|record| record["code"] == code).collect()
}

/// The refused registration's one warning, located in the acme manifest at
/// `line`:`column`, and no host applied: the widget composes as an `md`
/// document (its `size: big` is untyped), with no foreign-document notice
/// and nothing run.
fn assert_refused_at(code: Option<i32>, value: &Value, line: u64, column: u64) -> String {
    assert_eq!(code, Some(0), "no host types the widget: {value:#}");
    assert_eq!(value["document"], "# Widget\n\nBody.", "{value:#}");
    assert!(records(value, "dm.host.foreign_document").is_empty(), "no host applies: {value:#}");
    assert!(records(value, "dm.schema.type_mismatch").is_empty(), "no host schema judges it: {value:#}");
    assert_eq!(value["lifecycle_output"], serde_json::json!([]), "nothing ran: {value:#}");
    let refusals = records(value, REGISTRATION);
    assert_eq!(refusals.len(), 1, "{value:#}");
    let source = &refusals[0]["source"];
    let file = source["file"].as_str().expect("the refusal names a file");
    assert!(std::path::Path::new(file).ends_with(format!(".acme/{MANIFEST_NAME}")), "located at the manifest: {value:#}");
    assert_eq!((source["line"].as_u64(), source["column"].as_u64()), (Some(line), Some(column)), "{value:#}");
    refusals[0]["message"].as_str().expect("a message").to_string()
}

#[test]
fn a_format_2_bundle_types_the_document_its_trigger_matches() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-wiring-format-2", false);

    let (code, value) = compose_widget(&fixture);

    assert_eq!(code, Some(1), "the host's `size: number` rejects `big`: {value:#}");
    assert!(value["document"].is_null(), "nothing is written: {value:#}");
    assert!(records(&value, REGISTRATION).is_empty(), "the bundle loads: {value:#}");
    assert_eq!(records(&value, "dm.host.foreign_document").len(), 1, "the acme host applies: {value:#}");
    let mismatches: Vec<(u64, u64)> = records(&value, "dm.schema.type_mismatch")
        .iter()
        .map(|record| (record["source"]["line"].as_u64().unwrap(), record["source"]["column"].as_u64().unwrap()))
        .collect();
    assert_eq!(mismatches, [(3, 9)], "located at `size: big`: {value:#}");
    assert_eq!(value["lifecycle_output"], serde_json::json!([]), "nothing ran: {value:#}");
}

#[test]
fn a_format_1_manifest_is_refused_with_an_upgrade_warning_at_its_format_version() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-wiring-format-1", false);
    fixture.write_file(format!("home/.acme/{MANIFEST_NAME}"), FORMAT_1_MANIFEST);

    let (code, value) = compose_widget(&fixture);

    // `format_version: 1` is line 2; its value starts at column 17.
    let message = assert_refused_at(code, &value, 2, 17);
    assert!(message.contains("format 1") && message.contains("run `acme setup`"), "the upgrade warning: {message}");
}

#[test]
fn a_member_that_fails_its_digest_is_refused_at_its_files_entry() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered("host-wiring-digest", false);
    // Still a valid trigger matching the widget, but not the bytes the
    // manifest addresses.
    fixture.write_file("home/.acme/schemas/widget.yaml", "kind: schema-trigger\nmatch:\n    widget: object(required)\n");

    let (code, value) = compose_widget(&fixture);

    // `files[0]` (`- ./schemas/widget.yaml`) is line 6; its value starts at
    // column 7.
    let message = assert_refused_at(code, &value, 6, 7);
    assert!(
        message.contains("at `files[0]`") && message.contains("`./schemas/widget.yaml` does not match its digest"),
        "names the member field and file: {message}"
    );
}

// ── registered YAML kinds ────────────────────────────────────────────────────

/// A registered definition of `kind: roster`.
const ROSTER: &str = "kind: yaml-schema\n$schema:\n    kind: literal(roster; required)\n    platforms: string[](required)\n";

#[test]
fn schema_validate_judges_a_yaml_document_by_a_registered_hosts_kind() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered_bundle("host-yaml-kind", &acme().with_file("schemas/roster.yaml", ROSTER), false);
    fixture.write_file("cwd/team.yaml", "kind: roster\nplatforms: 3\n");
    fixture.write_file("cwd/crew.yaml", "kind: roster\nplatforms: [linux]\n");

    let output = run(&fixture, &["schema", "validate", "--output", "json", "team.yaml", "crew.yaml"]);

    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
    let value = data_value(&output, include_str!("../../schemas/output/schema-validate.yaml"));
    let files = value["files"].as_array().expect("one record per file");
    let problems: Vec<&str> = files[0]["problems"].as_array().unwrap().iter().filter_map(|problem| problem["path"].as_str()).collect();
    assert_eq!(problems, ["/platforms"], "{value:#}");
    assert_eq!(files[1]["valid"], true, "{value:#}");
}

#[test]
fn schema_validate_refuses_a_kind_two_registered_hosts_define_differently() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let fixture = registered_bundle("host-yaml-duplicate", &acme().with_file("schemas/roster.yaml", ROSTER), false);
    let mut beta = acme().with_file("schemas/roster.yaml", &ROSTER.replace("string[]", "number[]"));
    beta.host = "beta".into();
    beta.members.insert("schemas/globals.lifecycle.json".into(), crate::host_fixtures::lifecycle_artifact_for("beta", &[("schemas/run.yaml", crate::host_fixtures::RUN_NAMESPACE)]));
    for (file, text) in beta.files_with_manifest() {
        fixture.write_file(format!("home/.beta/{file}"), &text);
    }
    fixture.write_file("home/.darkmatter.yaml", &format!("register:\n  - ./.acme/{MANIFEST_NAME}\n  - ./.beta/{MANIFEST_NAME}\n"));
    fixture.write_file("cwd/team.yaml", "kind: roster\nplatforms: []\n");

    let output = run(&fixture, &["schema", "validate", "--output", "json", "team.yaml"]);

    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
    let value = data_value(&output, include_str!("../../schemas/output/schema-validate.yaml"));
    let text = value["files"][0].to_string();
    assert!(text.contains("roster"), "{value:#}");
    assert!(text.contains(".acme/schemas/roster.yaml") && text.contains(".beta/schemas/roster.yaml"), "both definitions are named: {value:#}");
}

/// A trigger matching every document with a `team` string, declaring `agent`
/// as `type`.
fn team_trigger(agent: &str) -> String {
    format!("kind: schema-trigger\nmatch:\n    team: string(required)\n$schema:\n    agent: {agent}\n")
}

#[test]
fn two_matched_triggers_that_disagree_are_one_coded_conflict_naming_both() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let bundle = acme().with_file("schemas/a.yaml", &team_trigger("number")).with_file("schemas/b.yaml", &team_trigger("boolean"));
    let fixture = registered_bundle("host-trigger-conflict", &bundle, false);
    fixture.write_file("cwd/team.md", "---\nteam: core\n---\n# Team\n");

    let validate = run(&fixture, &["schema", "validate", "--output", "json", "team.md"]);
    let value = data_value(&validate, include_str!("../../schemas/output/schema-validate.yaml"));
    let record = &value["files"][0];
    assert_eq!(record["code"], "dm.host.trigger_conflict", "{value:#}");
    let related: Vec<&str> = record["related"].as_array().expect("related triggers").iter().filter_map(Value::as_str).collect();
    assert!(related.len() == 2 && related[0].ends_with("schemas/a.yaml") && related[1].ends_with("schemas/b.yaml"), "{value:#}");
    assert_ne!(validate.status.code(), Some(0), "{value:#}");

    let compose = run(&fixture, &["compose", "team.md", "--output", "json"]);
    let value: Value = serde_json::from_slice(&compose.stdout).expect("one JSON value");
    let codes: Vec<&str> = value["diagnostics"].as_array().into_iter().flatten().filter_map(|record| record["code"].as_str()).collect();
    assert!(codes.contains(&"dm.host.trigger_conflict"), "{value:#}");
    assert!(!compose.status.success(), "{value:#}");
}
