//! `md` delivery of the schema triggers discovery skips as invalid
//! (`dm.schema.invalid_trigger`) through each command's normal invocation
//! path: `md compose`, `md schema validate`, `md schema triggers`, and
//! `md clean`, in ordinary and report modes.
//!
//! Every fixture is an isolated repository whose `schemas/` root holds the
//! incident trigger: a `memory.yaml` with unsupported envelope keys and a
//! null `match:`. The library proves the classification matrix in-process;
//! these tests prove each command shows, carries, or withholds the warning.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Output;

const CODE: &str = "dm.schema.invalid_trigger";
/// The warning's opening words, without whitespace (see [`stderr_of`]).
const WARNING: &str = "ignoringinvalidschematrigger";
const INCIDENT: &str =
    "kind: schema-trigger\nname: memory\ndescription: Memory notes\nmatch:\n$schema: memory-payload.yaml\n";
const VALID_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  kind: enum(prompt; required)\n$schema: prompt.yaml\n";
const VALID_PAYLOAD: &str = "$schema:\n  owner: string(required)\n";

fn write(root: &Path, relative: &str, content: &str) -> PathBuf {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
    path
}

fn initialize_repository(root: &Path) {
    let git_dir = root.join(".git");
    std::fs::create_dir_all(git_dir.join("objects")).unwrap();
    std::fs::create_dir_all(git_dir.join("refs/heads")).unwrap();
    std::fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
    std::fs::write(git_dir.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").unwrap();
}

/// A repository with the incident trigger at `schemas/memory.yaml`.
fn incident_repository(process: &CliProcessFixture) -> PathBuf {
    let root = process.cwd();
    initialize_repository(root);
    write(root, "schemas/memory.yaml", INCIDENT)
}

/// The incident trigger plus a valid `prompt` trigger requiring `owner`.
fn mixed_repository(process: &CliProcessFixture) {
    incident_repository(process);
    write(process.cwd(), "schemas/prompt.trigger.yaml", VALID_TRIGGER);
    write(process.cwd(), "schemas/prompt.yaml", VALID_PAYLOAD);
}

/// Runs `md` with `args` on a wide, colorless terminal, so a warning is one
/// unwrapped row whatever the temporary directory's length.
fn run(process: &CliProcessFixture, args: &[&str]) -> Output {
    process.command_builder().plain_terminal(1000, 50).build().args(args).output().expect("md runs")
}

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Stderr without styling, frames, whitespace, or hyphens: a warning row
/// wraps at its own width, hyphenating a long path where it breaks, and the
/// break moves with the temporary directory's length on each OS. Compare
/// against [`compact`] fragments.
fn stderr_of(output: &Output) -> String {
    compact(&common::unwrapped(&String::from_utf8_lossy(&output.stderr)))
}

fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace() && *c != '-').collect()
}

/// The stderr warning names the trigger, says it was ignored, and gives the
/// envelope's reason, never blaming a document's `$schema`.
fn assert_incident_warning(stderr: &str) {
    assert_eq!(stderr.matches(WARNING).count(), 1, "exactly one warning: {stderr}");
    for fragment in ["memory.yaml", "found unknown keys: name, description"] {
        assert!(stderr.contains(&compact(fragment)), "`{fragment}` not in {stderr}");
    }
    assert!(!stderr.contains("CompositionError"), "{stderr}");
}

/// The `{source, code, path, message}` row for the incident trigger.
fn assert_incident_row(row: &Value, trigger: &Path) {
    let mut keys: Vec<&str> = row.as_object().expect("a row object").keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, ["code", "message", "path", "source"], "{row:#}");
    assert_eq!(row["code"], CODE, "{row:#}");
    assert_eq!(row["source"], "darkmatter.schema", "{row:#}");
    let path = PathBuf::from(row["path"].as_str().expect("a disk trigger has a path"));
    assert_eq!(
        biscuit_file::canonicalize_simplified(&path).unwrap(),
        biscuit_file::canonicalize_simplified(trigger).unwrap(),
        "{row:#}"
    );
    let message = row["message"].as_str().unwrap();
    assert!(
        message.starts_with("ignoring invalid schema trigger") && message.contains("found unknown keys: name, description"),
        "{row:#}"
    );
}

// ── md compose ──────────────────────────────────────────────────────────────

/// The incident end to end: a `$schema`-less prompt beside the invalid
/// trigger composes, its transcluded child shares the one warning.
#[test]
fn compose_beside_the_incident_trigger_succeeds_with_one_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    write(process.cwd(), "docs/child.md", "---\ntitle: child\n---\nChild body\n");
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n\n::file ./child.md\n");

    let output = run(&process, &["compose", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = stdout_of(&output);
    assert!(stdout.contains("Prompt body") && stdout.contains("Child body"), "{stdout}");
    assert!(!stdout.contains(WARNING), "the warning never reaches stdout: {stdout}");
    assert_incident_warning(&stderr_of(&output));
}

#[test]
fn compose_report_json_carries_the_warning_as_a_diagnostic_and_not_on_stderr() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n");

    let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    assert!(!stderr_of(&output).contains(WARNING), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "success", "{report:#}");
    let rows: Vec<&Value> =
        report["diagnostics"].as_array().unwrap().iter().filter(|row| row["code"] == CODE).collect();
    assert_eq!(rows.len(), 1, "{report:#}");
    assert_eq!(rows[0]["severity"], "warning", "{report:#}");
    assert!(rows[0]["message"].as_str().unwrap().contains("memory.yaml"), "{report:#}");
    assert!(report["document"].as_str().unwrap().contains("Prompt body"), "{report:#}");
}

/// A validation failure under a valid neighboring trigger still fails, and
/// the warning discovered before it is delivered with it.
#[test]
fn compose_keeps_the_warning_when_a_valid_trigger_rejects_the_document() {
    let process = CliProcessFixture::new();
    mixed_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\nkind: prompt\n---\nPrompt body\n");

    let output = run(&process, &["compose", document.to_str().unwrap()]);
    assert!(!output.status.success(), "the valid trigger still enforces `owner`: {output:?}");
    assert!(!stdout_of(&output).contains("Prompt body"), "{output:?}");
    let stderr = stderr_of(&output);
    assert_incident_warning(&stderr);
    assert!(stderr.contains("owner"), "{stderr}");
}

/// A missing authored `$schema` keeps its existing error and the warning.
#[test]
fn compose_keeps_the_warning_when_the_authored_schema_fails_to_load() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\n$schema: ./missing.yaml\n---\nPrompt body\n");

    let output = run(&process, &["compose", document.to_str().unwrap()]);
    assert!(!output.status.success(), "{output:?}");
    let stderr = stderr_of(&output);
    assert_incident_warning(&stderr);
    assert!(stderr.contains(&compact("missing.yaml")), "{stderr}");
}

/// `--deny` of the code, or of everything, makes the warning blocking: no
/// document is written and the warning, not a schema-load error, is shown.
#[test]
fn compose_deny_of_the_code_or_all_stops_before_writing_the_document() {
    for deny in [CODE, "all"] {
        let process = CliProcessFixture::new();
        incident_repository(&process);
        let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n");

        let output = run(&process, &["compose", "--deny", deny, document.to_str().unwrap()]);
        assert!(!output.status.success(), "--deny {deny}: {output:?}");
        assert!(!stdout_of(&output).contains("Prompt body"), "--deny {deny}: {output:?}");
        let stderr = stderr_of(&output);
        assert!(
            stderr.contains(&compact("memory.yaml")) && stderr.contains(&compact("found unknown keys")),
            "--deny {deny}: {stderr}"
        );
        assert!(!stderr.contains(&compact("schema load failed")), "--deny {deny}: {stderr}");

        let report = run(&process, &["compose", "--output", "report-json", "--deny", deny, document.to_str().unwrap()]);
        assert!(!report.status.success(), "--deny {deny}: {report:?}");
        let report: Value = serde_json::from_slice(&report.stdout).expect("one JSON report");
        assert!(report["document"].is_null(), "--deny {deny}: {report:#}");
        let row = report["diagnostics"].as_array().unwrap().iter().find(|row| row["code"] == CODE);
        assert_eq!(row.map(|row| &row["severity"]), Some(&Value::from("warning")), "--deny {deny}: {report:#}");
    }
}

#[test]
fn compose_with_schema_triggers_disabled_reports_nothing() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n");

    let output = run(&process, &["compose", "--disable", "schema-triggers", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    assert!(stdout_of(&output).contains("Prompt body"), "{output:?}");
    assert!(!stderr_of(&output).contains(WARNING), "{output:?}");
}

/// Discovery skipping a trigger never makes it referenceable: an authored
/// `$schema` naming the ignored file fails with the existing error.
#[test]
fn compose_of_a_document_referencing_the_ignored_trigger_still_fails() {
    let process = CliProcessFixture::new();
    initialize_repository(process.cwd());
    write(process.cwd(), "schemas/memory.trigger.yaml", INCIDENT);
    let document =
        write(process.cwd(), "docs/prompt.md", "---\n$schema: ../schemas/memory.trigger.yaml\n---\nPrompt body\n");

    let output = run(&process, &["compose", document.to_str().unwrap()]);
    assert!(!output.status.success(), "{output:?}");
    assert!(!stdout_of(&output).contains("Prompt body"), "{output:?}");
    assert!(stderr_of(&output).contains(&compact("schema-trigger file")), "{output:?}");
}

// ── md schema validate ─────────────────────────────────────────────────────

/// Pretty mode: one stderr warning for the invocation, however many files
/// discover the trigger, and the exit status stays success.
#[test]
fn schema_validate_warns_once_on_stderr_for_several_files() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let first = write(process.cwd(), "docs/a.md", "---\ntitle: a\n---\n");
    let second = write(process.cwd(), "docs/b.md", "---\ntitle: b\n---\n");

    let output = run(&process, &["schema", "validate", first.to_str().unwrap(), second.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stdout_of(&output).matches("is valid").count(), 2, "{output:?}");
    assert!(!stdout_of(&output).contains(WARNING), "{output:?}");
    assert_incident_warning(&stderr_of(&output));
}

/// JSON mode: the row rides the first file's `warnings`; the second file,
/// which discovered the same trigger, has none.
#[test]
fn schema_validate_json_attaches_a_shared_warning_to_the_first_file() {
    let process = CliProcessFixture::new();
    let trigger = incident_repository(&process);
    let first = write(process.cwd(), "docs/a.md", "---\ntitle: a\n---\n");
    let second = write(process.cwd(), "docs/b.md", "---\ntitle: b\n---\n");

    let output =
        run(&process, &["schema", "validate", "--format", "json", first.to_str().unwrap(), second.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!stderr_of(&output).contains(WARNING), "{output:?}");
    let records: Vec<Value> =
        stdout_of(&output).lines().map(|line| serde_json::from_str(line).expect("one JSON record per line")).collect();
    assert_eq!(records.len(), 2, "{records:#?}");
    assert_eq!(records[0]["valid"], true, "{records:#?}");
    let warnings = records[0]["warnings"].as_array().expect("the first file carries the warning");
    assert_eq!(warnings.len(), 1, "{records:#?}");
    assert_incident_row(&warnings[0], &trigger);
    assert!(records[1].get("warnings").is_none(), "{records:#?}");
}

/// A validation failure keeps its exit status and fields, and the record
/// retains the discovery warning found before it.
#[test]
fn schema_validate_json_failure_keeps_the_discovery_warning() {
    let process = CliProcessFixture::new();
    mixed_repository(&process);
    let trigger = process.cwd().join("schemas/memory.yaml");
    let document = write(process.cwd(), "docs/prompt.md", "---\nkind: prompt\n---\n");

    let output = run(&process, &["schema", "validate", "--format", "json", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let record: Value = serde_json::from_str(stdout_of(&output).trim()).unwrap();
    assert_eq!(record["valid"], false, "{record:#}");
    assert!(record["problems"].as_array().unwrap().iter().any(|problem| problem["property"] == "owner"), "{record:#}");
    let warnings = record["warnings"].as_array().expect("the failure keeps its warning");
    assert_eq!(warnings.len(), 1, "{record:#}");
    assert_incident_row(&warnings[0], &trigger);
}

/// A schema failure after discovery keeps exit 2 and the warning.
#[test]
fn schema_validate_json_schema_error_keeps_the_discovery_warning() {
    let process = CliProcessFixture::new();
    let trigger = incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\n$schema: ./missing.yaml\n---\n");

    let output = run(&process, &["schema", "validate", "--format", "json", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(2), "{output:?}");
    let record: Value = serde_json::from_str(stdout_of(&output).trim()).unwrap();
    assert_eq!(record["error"], "schema", "{record:#}");
    assert_incident_row(&record["warnings"][0], &trigger);
}

/// Assignments coerce and re-resolve against the recovered registry: the
/// valid trigger still applies beside an invalid one in either filename
/// order.
#[test]
fn schema_validate_assignments_use_the_valid_trigger_beside_an_invalid_one() {
    for invalid in ["schemas/a-memory.yaml", "schemas/z-memory.yaml"] {
        let process = CliProcessFixture::new();
        initialize_repository(process.cwd());
        write(process.cwd(), invalid, INCIDENT);
        write(process.cwd(), "schemas/prompt.trigger.yaml", VALID_TRIGGER);
        write(process.cwd(), "schemas/prompt.yaml", VALID_PAYLOAD);
        let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: x\n---\n");
        let document = document.to_str().unwrap();

        let assigned = run(&process, &["schema", "validate", document, "kind=prompt", "owner=me"]);
        assert_eq!(assigned.status.code(), Some(0), "{invalid}: {assigned:?}");
        let stderr = stderr_of(&assigned);
        assert_eq!(stderr.matches(WARNING).count(), 1, "{invalid}: {stderr}");
        let file_name = Path::new(invalid).file_name().unwrap().to_str().unwrap();
        assert!(stderr.contains(&compact(file_name)), "{invalid}: {stderr}");

        let missing = run(&process, &["schema", "validate", document, "kind=prompt"]);
        assert_eq!(missing.status.code(), Some(1), "{invalid}: the trigger requires owner: {missing:?}");
        assert!(stdout_of(&missing).contains("owner"), "{invalid}: {missing:?}");
    }
}

#[test]
fn schema_validate_with_schema_triggers_disabled_reports_nothing() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/a.md", "---\ntitle: a\n---\n");

    let output = run(&process, &["schema", "validate", "--disable", "schema-triggers", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!stderr_of(&output).contains(WARNING), "{output:?}");
}

// ── md schema triggers ─────────────────────────────────────────────────────

/// The trace lists the skipped trigger inline with its reason; the valid
/// neighbor is still traced and matched.
#[test]
fn schema_triggers_lists_the_skipped_trigger_and_still_matches_the_valid_one() {
    let process = CliProcessFixture::new();
    mixed_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\nkind: prompt\nowner: me\n---\n");

    let output = run(&process, &["schema", "triggers", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = stdout_of(&output);
    let skipped = stdout.find("Skipped invalid triggers:").expect("a skipped-triggers section");
    let triggers = stdout.find("Triggers:").expect("the triggers section");
    let section = &stdout[skipped..triggers];
    assert!(section.contains("memory.yaml") && section.contains("found unknown keys: name, description"), "{stdout}");
    assert!(section.contains(CODE), "{stdout}");
    assert!(stdout[triggers..].contains("prompt.trigger.yaml — matched"), "{stdout}");
    assert!(!stderr_of(&output).contains(WARNING), "inspection output stays on stdout: {output:?}");
}

#[test]
fn schema_triggers_reports_none_skipped_for_a_valid_root() {
    let process = CliProcessFixture::new();
    initialize_repository(process.cwd());
    write(process.cwd(), "schemas/prompt.trigger.yaml", VALID_TRIGGER);
    write(process.cwd(), "schemas/prompt.yaml", VALID_PAYLOAD);
    let document = write(process.cwd(), "docs/prompt.md", "---\nkind: prompt\n---\n");

    let output = run(&process, &["schema", "triggers", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = stdout_of(&output);
    let skipped = stdout.find("Skipped invalid triggers:").expect("a skipped-triggers section");
    assert!(stdout[skipped..].lines().nth(1).is_some_and(|line| line.contains("none")), "{stdout}");
}

// ── md clean ────────────────────────────────────────────────────────────────

#[test]
fn clean_beside_the_incident_trigger_succeeds_with_one_stderr_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n");

    let output = run(&process, &["clean", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_eq!(stdout_of(&output), "---\ntitle: prompt\n---\nPrompt body\n", "{output:?}");
    assert_incident_warning(&stderr_of(&output));
}

/// `--json` keeps the version-1 envelope, adds the top-level `warnings`
/// array beside an unchanged `diagnostics`, and keeps stderr clean.
#[test]
fn clean_json_carries_the_warning_in_the_top_level_warnings_array() {
    let process = CliProcessFixture::new();
    let trigger = incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n");

    let output = run(&process, &["clean", "--json", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "report mode never repeats the warning: {output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).expect("the JSON envelope");
    assert_eq!(report["version"], 1, "{report:#}");
    assert_eq!(report["diagnostics"], Value::Array(Vec::new()), "{report:#}");
    assert_eq!(report["applied"], Value::Array(Vec::new()), "{report:#}");
    assert_eq!(report["changed"], false, "{report:#}");
    let warnings = report["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{report:#}");
    assert_incident_row(&warnings[0], &trigger);
}

#[test]
fn clean_with_schema_triggers_disabled_reports_nothing() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n");

    let output = run(&process, &["clean", "--disable", "schema-triggers", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");

    let output = run(&process, &["clean", "--json", "--disable", "schema-triggers", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["warnings"], Value::Array(Vec::new()), "{report:#}");
}

/// A clean that fails on its authored `$schema` still delivers the warning
/// discovered before the failure, in either mode.
#[test]
fn clean_keeps_the_warning_when_the_authored_schema_fails_to_load() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\n$schema: ./missing.yaml\n---\nPrompt body\n");

    for json in [false, true] {
        let mut args = vec!["clean", document.to_str().unwrap()];
        if json {
            args.push("--json");
        }
        let output = run(&process, &args);
        assert!(!output.status.success(), "--json {json}: {output:?}");
        let stderr = stderr_of(&output);
        assert_incident_warning(&stderr);
        assert!(stderr.contains(&compact("missing.yaml")), "--json {json}: {stderr}");
    }
}
