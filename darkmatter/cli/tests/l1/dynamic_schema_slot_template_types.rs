//! Template types (a `{{ … }}` slot that supplies a property's whole type)
//! through the real `md schema validate`, in both of its modes: a schema
//! file on its own, and a document naming it, with and without
//! `<prop>=<value>` assignments.
//!
//! Each test builds a Git repository at the fixture `cwd` holding
//! `schemas/schema.yaml`, whose `status` and `label` types each come from a
//! data file beside it, and `docs/doc.md`, which names the schema. Committed
//! files get the automatic grant; a cell then changes one thing (a dirty or
//! malformed data file, a grant store line) and runs one command. Nothing
//! here has a terminal, so nothing prompts; the fixture home holds the grant
//! store, so the developer's own grants never apply.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};
use simplified_schema::SimplifiedSchema;
use std::path::PathBuf;
use std::process::Output;

const SCHEMA: &str = "kind: schema\n$schema:\n    status: \"{{ load_data('./status.yaml', '{ type: string }').type }}(required)\"\n    label: \"{{ load_data('./label.yaml', '{ type: string }').type }}\"\n";
const STATUS_TYPE: &str = "type: enum(draft, published)\n";
/// `alphanumeric`, because validation never turns a number into one: an
/// assignment `label=123` passes only when the resolved type made it text.
const LABEL_TYPE: &str = "type: alphanumeric\n";
const DOCUMENT: &str = "---\n$schema: ../schemas/schema.yaml\nstatus: draft\n---\n\nBody.\n";

const SCHEMA_FILE: &str = "schemas/schema.yaml";
const STATUS_FILE: &str = "schemas/status.yaml";
const DOCUMENT_FILE: &str = "docs/doc.md";
const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";

/// The repository above, committed; `None` when `git` is unavailable.
fn repository(name: &str, label_type: &str) -> Option<CliProcessFixture> {
    let fixture = CliProcessFixture::named(name);
    if !fixture.initialize_repository() {
        return None;
    }
    fixture.write_file(format!("cwd/{SCHEMA_FILE}"), SCHEMA);
    fixture.write_file(format!("cwd/{STATUS_FILE}"), STATUS_TYPE);
    fixture.write_file("cwd/schemas/label.yaml", label_type);
    fixture.write_file(format!("cwd/{DOCUMENT_FILE}"), DOCUMENT);
    assert!(fixture.commit_all(fixture.cwd()), "the fixture commits");
    Some(fixture)
}

/// Adds a member to the committed `status` type: tracked, but no longer
/// identical to `HEAD`, so its read gets no automatic grant.
fn dirty_status(fixture: &CliProcessFixture) {
    std::fs::write(in_repo(fixture, STATUS_FILE), "type: enum(draft, published, archived)\n").expect("the status type is writable");
}

fn in_repo(fixture: &CliProcessFixture, relative: &str) -> PathBuf {
    relative.split('/').fold(fixture.cwd().to_path_buf(), |path, segment| path.join(segment))
}

/// `relative` as the slot gate spells it: canonical and simplified.
fn canonical(fixture: &CliProcessFixture, relative: &str) -> String {
    biscuit_file::canonicalize_simplified(&in_repo(fixture, relative)).expect("exists").display().to_string()
}

/// Grants reading the status type file, persisted in the fixture home.
fn grant_status(fixture: &CliProcessFixture) {
    let store = fixture.home().join(".config").join("darkmatter").join("schema-read-policy");
    common::fixture::write(&store, &format!("allow file {}\n", canonical(fixture, STATUS_FILE)));
}

fn validate(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(["schema", "validate"]).args(args).args(["--output", "json"]).output().expect("md runs")
}

/// Stdout as its one JSON value, having checked the exit status, that
/// stderr stayed empty, and that the value satisfies the command's output
/// schema.
fn report(output: &Output, status: i32) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(status), "{stdout}\n{stderr}");
    assert!(output.stderr.is_empty(), "a data output owns stderr: {stderr}");
    let value: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    let schema = SimplifiedSchema::try_from(include_str!("../../schemas/output/schema-validate.yaml")).expect("the output schema parses");
    if let Err(failure) = schema.validate_strict(&value) {
        panic!("{failure:?}\n{value:#}");
    }
    value
}

fn diagnostic_codes(record: &Value) -> Vec<&str> {
    record["diagnostics"].as_array().map_or_else(Vec::new, |records| records.iter().filter_map(|record| record["code"].as_str()).collect())
}

/// A refused or failed resolution is an unavailable check: never valid, no
/// problems judged against a substitute type, no slot count, and one record
/// with `code` at the slot of `file`'s `line`.
fn assert_unavailable(fixture: &CliProcessFixture, record: &Value, code: &str, file: &str, line: u64) {
    assert_eq!(record["valid"], false, "{record:#}");
    assert_eq!(record["error"], "slots", "{record:#}");
    assert_eq!(record["problems"], json!([]), "{record:#}");
    assert!(record.get("slots").is_none(), "{record:#}");
    assert_eq!(diagnostic_codes(record), [code], "{record:#}");
    assert_eq!(record["diagnostics"][0]["source"]["file"], json!(canonical(fixture, file)), "{record:#}");
    assert_eq!(record["diagnostics"][0]["source"]["line"], json!(line), "{record:#}");
}

#[test]
fn a_schema_file_whose_template_types_resolve_is_valid() {
    let Some(fixture) = repository("template-types-schema-file", LABEL_TYPE) else {
        return;
    };
    let report = report(&validate(&fixture, &[SCHEMA_FILE]), 0);
    let record = &report["files"][0];
    assert_eq!(record["valid"], true, "{report:#}");
    assert!(record.get("slots").is_none(), "{report:#}");
    assert!(record.get("error").is_none(), "{report:#}");
}

#[test]
fn a_schema_file_with_an_ungranted_template_type_read_is_unavailable() {
    let Some(fixture) = repository("template-types-schema-file-dirty", LABEL_TYPE) else {
        return;
    };
    dirty_status(&fixture);
    let report = report(&validate(&fixture, &[SCHEMA_FILE]), 1);
    assert_eq!(report["outcome"], "failure", "{report:#}");
    let record = &report["files"][0];
    assert_unavailable(&fixture, record, APPROVAL_REQUIRED, SCHEMA_FILE, 3);
    let targets: Vec<&str> = record["grants_needed"].as_array().expect("grants").iter().filter_map(|grant| grant["target"].as_str()).collect();
    assert!(targets.contains(&canonical(&fixture, STATUS_FILE).as_str()), "{report:#}");
}

#[test]
fn a_schema_file_with_a_persisted_grant_reads_the_dirty_type_file() {
    let Some(fixture) = repository("template-types-schema-file-granted", LABEL_TYPE) else {
        return;
    };
    dirty_status(&fixture);
    grant_status(&fixture);
    let report = report(&validate(&fixture, &[SCHEMA_FILE]), 0);
    assert_eq!(report["files"][0]["valid"], true, "{report:#}");
}

/// A returned definition that is not a type fails the schema file, where
/// the passive check used to pass it as a valid template.
#[test]
fn a_schema_file_whose_returned_definition_is_malformed_is_unavailable() {
    let Some(fixture) = repository("template-types-schema-file-malformed", "type: \"number(min(\"\n") else {
        return;
    };
    let report = report(&validate(&fixture, &[SCHEMA_FILE]), 1);
    assert_unavailable(&fixture, &report["files"][0], "dm.schema.slot_not_a_type", SCHEMA_FILE, 4);
}

#[test]
fn a_document_is_judged_by_the_resolved_template_type() {
    let Some(fixture) = repository("template-types-document", LABEL_TYPE) else {
        return;
    };
    let valid = report(&validate(&fixture, &[DOCUMENT_FILE]), 0);
    assert_eq!(valid["files"][0]["valid"], true, "{valid:#}");

    std::fs::write(in_repo(&fixture, DOCUMENT_FILE), DOCUMENT.replace("status: draft", "status: bogus")).expect("writable");
    let invalid = report(&validate(&fixture, &[DOCUMENT_FILE]), 1);
    let problems = invalid["files"][0]["problems"].as_array().expect("problems");
    assert_eq!(problems.len(), 1, "{invalid:#}");
    assert_eq!(problems[0]["path"], "/status", "{invalid:#}");
    let message = problems[0]["message"].as_str().unwrap_or_default();
    assert!(message.contains("draft") && message.contains("published"), "the resolved members are named: {invalid:#}");
}

#[test]
fn a_document_with_an_ungranted_template_type_read_is_unavailable_never_valid() {
    let Some(fixture) = repository("template-types-document-dirty", LABEL_TYPE) else {
        return;
    };
    dirty_status(&fixture);
    let report = report(&validate(&fixture, &[DOCUMENT_FILE]), 1);
    assert_unavailable(&fixture, &report["files"][0], APPROVAL_REQUIRED, SCHEMA_FILE, 3);
}

#[test]
fn a_document_with_a_persisted_grant_reads_the_dirty_type_file() {
    let Some(fixture) = repository("template-types-document-granted", LABEL_TYPE) else {
        return;
    };
    dirty_status(&fixture);
    grant_status(&fixture);
    let report = report(&validate(&fixture, &[DOCUMENT_FILE, "status=archived"]), 0);
    assert_eq!(report["files"][0]["valid"], true, "the grant read the dirty list, which allows `archived`: {report:#}");
}

/// `label=123` parses as a number; only the resolved `alphanumeric` type
/// keeps it as the text `123`, which is the one form that type accepts.
#[test]
fn an_assignment_is_converted_by_the_resolved_template_type() {
    let Some(fixture) = repository("template-types-assignment", LABEL_TYPE) else {
        return;
    };
    let report = report(&validate(&fixture, &[DOCUMENT_FILE, "label=123", "status=published"]), 0);
    let record = &report["files"][0];
    assert_eq!(record["valid"], true, "{report:#}");
    assert_eq!(record["problems"], json!([]), "{report:#}");
}

/// Without the grant the assignment is never converted against a guessed
/// type or judged: the check is unavailable.
#[test]
fn an_assignment_with_an_ungranted_template_type_read_is_unavailable() {
    let Some(fixture) = repository("template-types-assignment-dirty", LABEL_TYPE) else {
        return;
    };
    dirty_status(&fixture);
    let report = report(&validate(&fixture, &[DOCUMENT_FILE, "status=published"]), 1);
    assert_unavailable(&fixture, &report["files"][0], APPROVAL_REQUIRED, SCHEMA_FILE, 3);
}
