//! Schema templates through the real `md` binary, on the shipped artifacts:
//! Claudine's sandboxing schema, whose `os` enum is a `load_data` slot, and
//! the `os.yaml` list it reads.
//!
//! Each test copies both files (read with `include_str!` inside the test, so
//! a change to either runs it) into a fresh Git repository at the fixture
//! `cwd`, beside a document that names the schema, and commits them: the
//! clean control gets the automatic grant. A cell then changes one thing (a
//! dirty data file, a grant store line) and runs one command. Nothing here
//! has a terminal, so nothing ever prompts; the fixture home holds the grant
//! store, so the developer's own grants never apply.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};
use simplified_schema::SimplifiedSchema;
use std::path::{Path, PathBuf};
use std::process::Output;

const SCHEMA_FILE: &str = "claudine/docs/research/sandboxing/providers/sandboxing.yaml";
const DATA_FILE: &str = "claudine/docs/os.yaml";
const DOCUMENT_FILE: &str = "claudine/docs/research/sandboxing/providers/provider.md";
const APPROVAL_REQUIRED: &str = "dm.schema.slot_approval_required";

/// A provider document under the sandboxing schema listing `os_support`.
fn document(os_support: &[&str]) -> String {
    let items: String = os_support.iter().map(|os| format!("  - {os}\n")).collect();
    format!("---\n$schema: ./sandboxing.yaml\nschema_revision: 5\nhas_native_support: true\nos_support:\n{items}---\n\n# Provider\n")
}

/// The shipped `schema` and `data` committed in a repository at the fixture
/// `cwd`, with a valid provider document; `None` when `git` is unavailable.
fn sandbox(name: &str, schema: &str, data: &str) -> Option<CliProcessFixture> {
    let fixture = CliProcessFixture::named(name);
    if !fixture.initialize_repository() {
        return None;
    }
    fixture.write_file(Path::new("cwd").join(SCHEMA_FILE), schema);
    fixture.write_file(Path::new("cwd").join(DATA_FILE), data);
    fixture.write_file(Path::new("cwd").join(DOCUMENT_FILE), &document(&["macOS", "Linux"]));
    assert!(fixture.commit_all(fixture.cwd()), "the fixture commits");
    Some(fixture)
}

/// Appends an entry to the committed data file: tracked, but no longer
/// identical to `HEAD`, so it gets no automatic grant.
fn dirty_data(fixture: &CliProcessFixture) {
    let path = in_repo(fixture, DATA_FILE);
    let mut text = std::fs::read_to_string(&path).expect("the data file");
    text.push_str("- FreeBSD\n");
    std::fs::write(&path, text).expect("the data file is writable");
}

fn in_repo(fixture: &CliProcessFixture, relative: &str) -> PathBuf {
    relative.split('/').fold(fixture.cwd().to_path_buf(), |path, segment| path.join(segment))
}

/// `relative` as the slot gate spells it: canonical and simplified.
fn canonical(fixture: &CliProcessFixture, relative: &str) -> String {
    biscuit_file::canonicalize_simplified(&in_repo(fixture, relative)).expect("exists").display().to_string()
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Stdout as its one JSON value, having checked the exit status, that
/// stderr stayed empty (a data output never prompts or prints there), and
/// that the value satisfies `schema_text`.
fn data_output(output: &Output, status: i32, schema_text: &str) -> Value {
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(output.status.code(), Some(status), "{stdout}\n{}", stderr_of(output));
    assert!(output.stderr.is_empty(), "a data output owns stderr: {}", stderr_of(output));
    let value: Value = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"));
    let schema = SimplifiedSchema::try_from(schema_text).expect("the output schema parses");
    if let Err(failure) = schema.validate_strict(&value) {
        panic!("{failure:?}\n{value:#}");
    }
    value
}

/// The `grants_needed` of a record as `(kind, target, scope)` rows.
fn grants(record: &Value) -> Vec<(String, String, String)> {
    record["grants_needed"]
        .as_array()
        .unwrap_or_else(|| panic!("grants_needed is a list: {record:#}"))
        .iter()
        .map(|grant| {
            let field = |key: &str| grant[key].as_str().unwrap_or_default().to_string();
            (field("kind"), field("target"), field("scope"))
        })
        .collect()
}

/// The grants a dirty `os.yaml` needs, in prompt order: this file, then its
/// folder, each once and persisted (the fixture home holds a grant store).
fn data_file_grants(fixture: &CliProcessFixture) -> Vec<(String, String, String)> {
    let file = canonical(fixture, DATA_FILE);
    let folder = canonical(fixture, "claudine/docs");
    [("file", &file, "once"), ("file", &file, "persisted"), ("dir", &folder, "once"), ("dir", &folder, "persisted")]
        .into_iter()
        .map(|(kind, target, scope)| (kind.to_string(), target.clone(), scope.to_string()))
        .collect()
}

fn diagnostic_codes(records: &Value) -> Vec<&str> {
    records.as_array().expect("a list").iter().filter_map(|record| record["code"].as_str()).collect()
}

#[test]
fn compose_resolves_the_shipped_schema_and_reports_its_data_file_read() {
    let Some(fixture) = sandbox(
        "slots-compose-control",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    let output = run(&fixture, &["compose", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 0, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(report["outcome"], "success", "{report:#}");
    assert_eq!(report["diagnostics"], json!([]), "{report:#}");
    assert_eq!(report["read_set"], json!([canonical(&fixture, DATA_FILE)]), "{report:#}");
}

#[test]
fn compose_rejects_a_value_outside_the_resolved_list_naming_its_members() {
    let Some(fixture) = sandbox(
        "slots-compose-beos",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    fixture.write_file(Path::new("cwd").join(DOCUMENT_FILE), &document(&["macOS", "BeOS"]));
    let output = run(&fixture, &["compose", DOCUMENT_FILE]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    assert!(output.stdout.is_empty(), "no document is written");
    let stderr = stderr_of(&output);
    assert!(stderr.contains(r#""BeOS" is not one of "macOS", "Linux" or "Windows""#), "{stderr}");
    assert!(!stderr.contains("slot_"), "an ordinary validation problem: {stderr}");
}

#[test]
fn compose_data_output_on_a_dirty_data_file_fails_naming_the_grants_needed() {
    let Some(fixture) = sandbox(
        "slots-compose-dirty",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    dirty_data(&fixture);
    let output = run(&fixture, &["compose", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 1, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(report["outcome"], "failure", "{report:#}");
    assert_eq!(report["document"], Value::Null, "{report:#}");
    assert_eq!(diagnostic_codes(&report["diagnostics"]), [APPROVAL_REQUIRED], "{report:#}");
    let record = &report["diagnostics"][0];
    assert_eq!(record["source"]["file"], canonical(&fixture, SCHEMA_FILE), "{record:#}");
    assert_eq!(record["source"]["line"], 8, "the slot's line in the schema: {record:#}");
    assert_eq!(grants(record), data_file_grants(&fixture), "{record:#}");
    assert_eq!(report["read_set"], json!([]), "nothing unapproved was read: {report:#}");
}

/// An inline `$schema` slot is left to the slot resolver: neither the
/// reference check nor frontmatter interpolation reads its data file first.
#[test]
fn compose_reads_nothing_for_an_unapproved_inline_slot() {
    let fixture = CliProcessFixture::named("slots-compose-inline");
    fixture.write_file(Path::new("cwd").join("names.yaml"), "- Linux\n");
    fixture.write_file(
        Path::new("cwd").join("doc.md"),
        "---\n$schema:\n    os: \"enum({{ load_data('./names.yaml', 'string[]') }})\"\nos: Linux\n---\n\nBody.\n",
    );
    let output = run(&fixture, &["compose", "doc.md", "--output", "json"]);
    let report = data_output(&output, 1, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(diagnostic_codes(&report["diagnostics"]), [APPROVAL_REQUIRED], "{report:#}");
    assert_eq!(report["read_set"], json!([]), "nothing unapproved was read: {report:#}");
}

#[test]
fn compose_without_a_terminal_never_prompts_and_reports_the_approval_needed() {
    let Some(fixture) = sandbox(
        "slots-compose-no-terminal",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    dirty_data(&fixture);
    let output = fixture.command().args(["compose", DOCUMENT_FILE]).write_stdin("1\n").output().expect("md runs");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    let stderr = stderr_of(&output);
    assert!(stderr.contains("the read needs approval"), "{stderr}");
    assert!(!stderr.contains("Allow"), "no prompt was shown: {stderr}");
}

#[test]
fn compose_with_a_persisted_grant_reads_the_dirty_data_file() {
    let Some(fixture) = sandbox(
        "slots-compose-persisted",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    dirty_data(&fixture);
    let store = fixture.home().join(".config").join("darkmatter").join("schema-read-policy");
    common::fixture::write(&store, &format!("# granted by hand\nallow file {}\n", canonical(&fixture, DATA_FILE)));
    let output = run(&fixture, &["compose", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 0, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(report["read_set"], json!([canonical(&fixture, DATA_FILE)]), "{report:#}");
}

/// Matrix 2's wiring cell: an invalid grant-store line grants nothing and
/// fails the run at that line, even for a data file the automatic grant
/// would allow, since skipping the line could drop a deny.
#[test]
fn compose_with_an_invalid_grant_store_fails_at_its_line() {
    let Some(fixture) = sandbox(
        "slots-compose-store-invalid",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    let store = fixture.home().join(".config").join("darkmatter").join("schema-read-policy");
    common::fixture::write(&store, "# a comment line is valid\nallow file\n");
    let output = run(&fixture, &["compose", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 1, include_str!("../../schemas/output/compose.yaml"));
    assert_eq!(diagnostic_codes(&report["diagnostics"]), ["dm.schema.slot_grant_store_invalid"], "{report:#}");
    let at = &report["diagnostics"][0]["source"];
    assert_eq!(at["line"], 2, "{report:#}");
    assert!(at["file"].as_str().is_some_and(|file| file.ends_with("schema-read-policy")), "{report:#}");
    assert_eq!(report["read_set"], json!([]), "{report:#}");
}

/// A schema file argument is checked passively: its slot is counted, never
/// resolved, so the run succeeds even with the data file gone.
#[test]
fn schema_validate_of_the_shipped_schema_file_is_passive() {
    let Some(fixture) = sandbox(
        "slots-validate-schema-file",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    std::fs::remove_file(in_repo(&fixture, DATA_FILE)).expect("the data file is removable");
    let output = run(&fixture, &["schema", "validate", SCHEMA_FILE, "--output", "json"]);
    let report = data_output(&output, 0, include_str!("../../schemas/output/schema-validate.yaml"));
    let record = &report["files"][0];
    assert_eq!(record["valid"], true, "{report:#}");
    assert_eq!(record["slots"], 1, "{report:#}");
    assert!(record.get("error").is_none(), "{report:#}");
}

#[test]
fn schema_validate_of_a_document_resolves_with_the_automatic_grant() {
    let Some(fixture) = sandbox(
        "slots-validate-document-clean",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    let output = run(&fixture, &["schema", "validate", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 0, include_str!("../../schemas/output/schema-validate.yaml"));
    assert_eq!(report["files"][0]["valid"], true, "{report:#}");
    assert!(report["files"][0].get("slots").is_none(), "a document is not a template: {report:#}");
}

#[test]
fn schema_validate_of_a_document_with_an_unapproved_slot_is_unavailable_never_valid() {
    let Some(fixture) = sandbox(
        "slots-validate-document-dirty",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    dirty_data(&fixture);
    let output = run(&fixture, &["schema", "validate", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 1, include_str!("../../schemas/output/schema-validate.yaml"));
    assert_eq!(report["outcome"], "failure", "{report:#}");
    let record = &report["files"][0];
    assert_eq!(record["valid"], false, "{report:#}");
    assert_eq!(record["error"], "slots", "{report:#}");
    assert_eq!(record["problems"], json!([]), "{report:#}");
    assert_eq!(diagnostic_codes(&record["diagnostics"]), [APPROVAL_REQUIRED], "{report:#}");
    assert_eq!(grants(record), data_file_grants(&fixture), "{report:#}");
    assert_eq!(grants(&record["diagnostics"][0]), data_file_grants(&fixture), "{report:#}");
}

#[test]
fn clean_with_an_unapproved_slot_never_reports_the_document_clean() {
    let Some(fixture) = sandbox(
        "slots-clean-dirty",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    dirty_data(&fixture);
    let before = std::fs::read_to_string(in_repo(&fixture, DOCUMENT_FILE)).expect("the document");
    let output = run(&fixture, &["clean", DOCUMENT_FILE, "--save", "--output", "json"]);
    let report = data_output(&output, 1, include_str!("../../schemas/output/clean.yaml"));
    assert_eq!(report["outcome"], "failure", "{report:#}");
    assert_eq!(diagnostic_codes(&report["run_diagnostics"]), [APPROVAL_REQUIRED], "{report:#}");
    assert_eq!(grants(&report["run_diagnostics"][0]), data_file_grants(&fixture), "{report:#}");
    let after = std::fs::read_to_string(in_repo(&fixture, DOCUMENT_FILE)).expect("the document");
    assert_eq!(before, after, "nothing is saved");
}

#[test]
fn clean_of_a_document_valid_under_the_resolved_list_reports_no_findings() {
    let Some(fixture) = sandbox(
        "slots-clean-control",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    let output = run(&fixture, &["clean", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 0, include_str!("../../schemas/output/clean.yaml"));
    assert!(report.get("outcome").is_none(), "{report:#}");
    assert_eq!(report["diagnostics"], json!([]), "{report:#}");
}

/// A reference check never asks: an unapproved slot defers the document's
/// schema, so the graph is reported as usual.
#[test]
fn graph_defers_an_unapproved_slot() {
    let Some(fixture) = sandbox(
        "slots-graph-dirty",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    dirty_data(&fixture);
    let output = run(&fixture, &["graph", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 0, include_str!("../../schemas/output/graph.yaml"));
    assert!(report.get("outcome").is_none(), "{report:#}");
}

#[test]
fn validate_refs_defers_an_unapproved_slot() {
    let Some(fixture) = sandbox(
        "slots-validate-refs-dirty",
        include_str!("../../../../claudine/docs/research/sandboxing/providers/sandboxing.yaml"),
        include_str!("../../../../claudine/docs/os.yaml"),
    ) else {
        return;
    };
    dirty_data(&fixture);
    let output = run(&fixture, &["validate", "refs", DOCUMENT_FILE, "--output", "json"]);
    let report = data_output(&output, 0, include_str!("../../schemas/output/validate-refs.yaml"));
    assert!(report.get("outcome").is_none(), "{report:#}");
}

/// Marks the fixture `cwd` a repository without spawning `git`, so its
/// `schemas/` folder is a schema root.
fn mark_repository(fixture: &CliProcessFixture) {
    fixture.write_file("cwd/.git/HEAD", "ref: refs/heads/main\n");
    std::fs::create_dir_all(fixture.cwd().join(".git/objects")).expect("objects");
    std::fs::create_dir_all(fixture.cwd().join(".git/refs")).expect("refs");
}

/// A slot in a kind definition is refused: the YAML document claiming the
/// kind cannot be checked, and its record names the prohibition's code.
#[test]
fn schema_validate_names_the_code_of_a_slot_in_a_kind_definition() {
    let fixture = CliProcessFixture::named("slots-validate-kind");
    mark_repository(&fixture);
    fixture.write_file(
        "cwd/schemas/roster.yaml",
        "kind: yaml-schema\ntypes:\n    platform:\n        name: enum({{ load_data(\"./names.yaml\") }}; required)\n$schema:\n    kind: literal(roster; required)\n    platforms: platform[](required)\n",
    );
    fixture.write_file("cwd/roster.yaml", "kind: roster\nplatforms:\n    - name: slack\n");
    let output = run(&fixture, &["schema", "validate", "roster.yaml", "--output", "json"]);
    let report = data_output(&output, 1, include_str!("../../schemas/output/schema-validate.yaml"));
    let record = &report["files"][0];
    assert_eq!(record["error"], "schema", "{report:#}");
    assert_eq!(record["code"], "dm.schema.slot_not_allowed_in_kind", "{report:#}");
}

/// A slot in a baseline is refused before any file is validated, with its
/// own code rather than the generic baseline failure.
#[test]
fn schema_validate_names_the_code_of_a_slot_in_the_baseline() {
    let fixture = CliProcessFixture::named("slots-validate-baseline");
    fixture.write_file("cwd/base.yaml", "$schema:\n    title: enum({{ load_data(\"./titles.yaml\") }})\n");
    fixture.write_file("cwd/doc.md", "---\ntitle: x\n---\n# H\n");
    let output = run(&fixture, &["schema", "validate", "--schema", "base.yaml", "doc.md", "--output", "json"]);
    let report = data_output(&output, 1, include_str!("../../schemas/output/schema-validate.yaml"));
    assert_eq!(report["files"], json!([]), "{report:#}");
    assert_eq!(diagnostic_codes(&report["diagnostics"]), ["dm.schema.slot_not_allowed_in_baseline"], "{report:#}");
}
