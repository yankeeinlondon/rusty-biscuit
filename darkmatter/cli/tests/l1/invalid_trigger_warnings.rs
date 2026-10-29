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
    assert_eq!(rows[0]["source"], Value::Null, "unsupported keys have no position to report: {report:#}");
    assert!(report["document"].as_str().unwrap().contains("Prompt body"), "{report:#}");
}

/// A malformed envelope's YAML error position is the record's `source`, on
/// the trigger file; the ordinary run's stderr row still spells it out.
#[test]
fn compose_report_json_places_a_malformed_envelope_at_its_yaml_error() {
    let process = CliProcessFixture::new();
    initialize_repository(process.cwd());
    let trigger = write(process.cwd(), "schemas/broken.yaml", "kind: schema-trigger\nmatch: [unclosed\n");
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n");

    let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).expect("one JSON report");
    let rows: Vec<&Value> =
        report["diagnostics"].as_array().unwrap().iter().filter(|row| row["code"] == CODE).collect();
    let [row] = rows[..] else { panic!("one trigger row: {report:#}") };
    let file = PathBuf::from(row["source"]["file"].as_str().expect("the trigger file: {report:#}"));
    assert_eq!(
        biscuit_file::canonicalize_simplified(&file).unwrap(),
        biscuit_file::canonicalize_simplified(&trigger).unwrap(),
        "{report:#}"
    );
    assert_eq!(row["source"]["line"], 3, "{report:#}");
    assert_eq!(row["source"]["column"], 1, "{report:#}");

    let ordinary = run(&process, &["compose", document.to_str().unwrap()]);
    assert!(ordinary.status.success(), "{ordinary:?}");
    let stderr = stderr_of(&ordinary);
    assert_eq!(stderr.matches(WARNING).count(), 1, "{stderr}");
    assert!(stderr.contains(&compact("broken.yaml: not valid YAML at line 3, column 1")), "{stderr}");
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

/// The `dm.schema.invalid_trigger` rows of a `--output report-json` run,
/// after checking that stderr carries no warning row.
fn report_trigger_rows(output: &Output) -> (Value, usize) {
    assert!(!stderr_of(output).contains(WARNING), "the report carries the warning: {output:?}");
    let report: Value = serde_json::from_slice(&output.stdout).expect("one JSON report");
    let rows = report["diagnostics"].as_array().unwrap().iter().filter(|row| row["code"] == CODE).count();
    (report, rows)
}

/// `--shell` inspects without composing, and still reports the trigger its
/// walk skipped, beside an unchanged shell report.
#[test]
fn compose_shell_report_keeps_the_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n");

    let output = run(&process, &["compose", "--shell", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = stdout_of(&output);
    assert!(stdout.contains("No shell commands discovered."), "{stdout}");
    assert!(!compact(&stdout).contains(WARNING), "{stdout}");
    assert_incident_warning(&stderr_of(&output));
}

/// A `--shell` walk that fails keeps its error and the warning found first.
#[test]
fn compose_shell_report_failure_keeps_the_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\n$schema: ./missing.yaml\n---\nPrompt body\n");

    let output = run(&process, &["compose", "--shell", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = stderr_of(&output);
    assert_incident_warning(&stderr);
    assert!(stderr.contains(&compact("missing.yaml")), "{stderr}");
}

/// A lifecycle whose `initialize` skips the document never composes it; the
/// warning its frontmatter bootstrap found is still delivered, on stderr or
/// in the report.
#[test]
fn compose_lifecycle_skipped_by_initialize_keeps_the_bootstrap_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\ninitialize:\n  - skip\n---\nPrompt body\n");

    let output = run(&process, &["compose", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    assert!(!stdout_of(&output).contains("Prompt body"), "{output:?}");
    assert_incident_warning(&stderr_of(&output));

    let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let (report, rows) = report_trigger_rows(&output);
    assert_eq!(rows, 1, "{report:#}");
    assert_eq!(report["outcome"], "success", "{report:#}");
    assert!(report["document"].is_null(), "{report:#}");
}

/// A lifecycle that composes finds the trigger in its call check, its
/// bootstrap, and its composition, and reports it once, on success and on
/// an authored-schema failure, in both output modes.
#[test]
fn compose_lifecycle_past_initialize_warns_once() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    for (frontmatter, succeeds) in [("", true), ("$schema: ./missing.yaml\n", false)] {
        let document = write(
            process.cwd(),
            "docs/prompt.md",
            &format!("---\n{frontmatter}initialize:\n  - info: start\n---\nPrompt body\n"),
        );

        let output = run(&process, &["compose", document.to_str().unwrap()]);
        assert_eq!(output.status.success(), succeeds, "{output:?}");
        assert_eq!(stdout_of(&output).contains("Prompt body"), succeeds, "{output:?}");
        assert_incident_warning(&stderr_of(&output));

        let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
        assert_eq!(output.status.success(), succeeds, "{output:?}");
        let (report, rows) = report_trigger_rows(&output);
        assert_eq!(rows, 1, "{report:#}");
        assert_eq!(report["outcome"], if succeeds { "success" } else { "failure" }, "{report:#}");
    }
}

/// A lifecycle call the passive check rejects stops the run before any
/// composition; the trigger that check's schema discovery skipped is
/// delivered with the rejection.
#[test]
fn compose_lifecycle_call_rejection_keeps_the_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(
        process.cwd(),
        "docs/prompt.md",
        "---\ncomposed:\n  - stdout: '{{ min(true, 5) }}'\n---\nPrompt body\n",
    );

    let output = run(&process, &["compose", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = stderr_of(&output);
    assert_incident_warning(&stderr);
    assert!(stderr.contains(&compact("can never bind")), "{stderr}");

    let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
    let (report, rows) = report_trigger_rows(&output);
    assert_eq!(rows, 1, "{report:#}");
    assert_eq!(report["outcome"], "failure", "{report:#}");
}

/// A lifecycle call warning `--deny` blocks stops the run before the
/// bootstrap; the trigger the call check found is delivered with it.
#[test]
fn compose_lifecycle_blocked_call_warning_keeps_the_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(
        process.cwd(),
        "docs/prompt.md",
        "---\ncomposed:\n  - stdout: '{{ file_exists(\"\") }}'\n---\nPrompt body\n",
    );
    let deny = ["compose", "--deny", "dm.expression.always_false_call"];

    let output = run(&process, &[&deny[..], &[document.to_str().unwrap()]].concat());
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert_incident_warning(&stderr_of(&output));

    let output = run(&process, &[&deny[..], &["--output", "report-json", document.to_str().unwrap()]].concat());
    let (report, rows) = report_trigger_rows(&output);
    assert_eq!(rows, 1, "{report:#}");
    assert_eq!(report["outcome"], "failure", "{report:#}");
}

/// Unresolved references refuse the document before it composes, with or
/// without a lifecycle; the refusal (exit 2) still carries the warning.
#[test]
fn compose_reference_refusal_keeps_the_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    for lifecycle in ["", "initialize:\n  - info: start\n"] {
        let document = write(
            process.cwd(),
            "docs/prompt.md",
            &format!("---\ntitle: prompt\n{lifecycle}---\nPrompt body\n\n::file ./nope.md\n"),
        );

        let output = run(&process, &["compose", document.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(2), "{lifecycle:?}: {output:?}");
        let stderr = stderr_of(&output);
        assert_incident_warning(&stderr);
        assert!(stderr.contains(&compact("nope.md")), "{lifecycle:?}: {stderr}");

        let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(2), "{lifecycle:?}: {output:?}");
        let (report, rows) = report_trigger_rows(&output);
        assert_eq!(rows, 1, "{lifecycle:?}: {report:#}");
    }
}

/// A repository whose only invalid trigger is in package `alpha`'s own
/// `schemas/` root, beside `alpha/docs/child.md`: a document outside `alpha`
/// discovers it only through that transcluded child's context.
fn child_only_trigger_repository(process: &CliProcessFixture) -> PathBuf {
    let root = process.cwd();
    initialize_repository(root);
    write(root, "Cargo.toml", "[workspace]\nmembers = [\"alpha\"]\n");
    write(root, "alpha/Cargo.toml", "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\nedition = \"2024\"\n");
    write(root, "alpha/src/lib.rs", "");
    write(root, "alpha/docs/child.md", "---\ntitle: child\n---\nChild body\n");
    write(root, "alpha/schemas/memory.yaml", INCIDENT)
}

/// Reference validation prepares each transcluded child in its own context,
/// so a refusal (exit 2) carries the trigger only the child's schema roots
/// hold, once, though the root never composes.
#[test]
fn compose_reference_refusal_keeps_a_child_only_warning() {
    let process = CliProcessFixture::new();
    let trigger = child_only_trigger_repository(&process);
    let control = write(process.cwd(), "docs/control.md", "---\ntitle: control\n---\nControl body\n");
    let output = run(&process, &["compose", control.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    assert!(!stderr_of(&output).contains(WARNING), "the root's context holds no invalid trigger: {output:?}");

    for lifecycle in ["", "initialize:\n  - info: start\n"] {
        let document = write(
            process.cwd(),
            "docs/prompt.md",
            &format!("---\ntitle: prompt\n{lifecycle}---\nPrompt body\n\n::file ../alpha/docs/child.md\n\n::file ./nope.md\n"),
        );

        let output = run(&process, &["compose", document.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(2), "{lifecycle:?}: {output:?}");
        let stderr = stderr_of(&output);
        assert_incident_warning(&stderr);
        assert!(stderr.contains(&compact("nope.md")), "{lifecycle:?}: {stderr}");

        let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(2), "{lifecycle:?}: {output:?}");
        let (report, rows) = report_trigger_rows(&output);
        assert_eq!(rows, 1, "{lifecycle:?}: {report:#}");
        let row = report["diagnostics"].as_array().unwrap().iter().find(|row| row["code"] == CODE).unwrap();
        assert_eq!(row["severity"], "warning", "{report:#}");
        let message = row["message"].as_str().unwrap();
        assert!(compact(message).contains(&compact(&trigger.display().to_string())), "names alpha's trigger: {report:#}");
    }
}

/// Reported (recovered) reference issues let the document compose; the
/// child-only trigger reference validation found and the composition found
/// again is reported once.
#[test]
fn compose_reference_report_and_composition_share_a_child_only_warning() {
    let process = CliProcessFixture::new();
    child_only_trigger_repository(&process);
    for lifecycle in ["", "initialize:\n  - info: start\n"] {
        let document = write(
            process.cwd(),
            "docs/prompt.md",
            &format!("---\ntitle: prompt\n{lifecycle}---\nPrompt body [gone](./nope.md)\n\n::file ../alpha/docs/child.md\n"),
        );
        let args = ["compose", "--allow", "missing-hyperlinks"];

        let output = run(&process, &[&args[..], &[document.to_str().unwrap()]].concat());
        assert!(output.status.success(), "{lifecycle:?}: {output:?}");
        assert!(stdout_of(&output).contains("Child body"), "{lifecycle:?}: {output:?}");
        assert_incident_warning(&stderr_of(&output));

        let output = run(&process, &[&args[..], &["--output", "report-json", document.to_str().unwrap()]].concat());
        assert!(output.status.success(), "{lifecycle:?}: {output:?}");
        let (report, rows) = report_trigger_rows(&output);
        assert_eq!(rows, 1, "{lifecycle:?}: {report:#}");
    }
}

/// An `error` in `initialize` ends the run before composition (`failed` and
/// `finalize` run, exit 1); the trigger the bootstrap found is delivered once
/// with that failure.
#[test]
fn compose_lifecycle_failing_in_initialize_keeps_the_bootstrap_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(
        process.cwd(),
        "docs/prompt.md",
        "---\ninitialize:\n  - error: \"initialize broke\"\nfailed:\n  - warn: \"cleanup ran\"\n---\nPrompt body\n",
    );

    let output = run(&process, &["compose", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(!stdout_of(&output).contains("Prompt body"), "{output:?}");
    let stderr = stderr_of(&output);
    assert_incident_warning(&stderr);
    assert!(stderr.contains(&compact("initialize broke")), "{stderr}");
    assert!(stderr.contains(&compact("cleanup ran")), "{stderr}");

    let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let (report, rows) = report_trigger_rows(&output);
    assert_eq!(rows, 1, "{report:#}");
    assert_eq!(report["outcome"], "failure", "{report:#}");
    assert!(report.to_string().contains("initialize broke"), "{report:#}");
}

/// A lifecycle `shell` that stops preparation before composition, because
/// its placeholder reads a missing value or because the preflight refuses it
/// (no approval is possible on a non-interactive run), never runs; the
/// bootstrap's trigger is delivered once with that failure.
#[test]
fn compose_lifecycle_shell_failing_in_preparation_keeps_the_bootstrap_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    for (command, failure) in [
        ("echo {{ nope }}", "reads `nope`, which is missing"),
        ("echo lifecycle-shell-ran", "Approval required"),
    ] {
        let document = write(
            process.cwd(),
            "docs/prompt.md",
            &format!("---\ncomposed:\n  - shell: \"{command}\"\n---\nPrompt body\n"),
        );

        let output = run(&process, &["compose", document.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(1), "{command}: {output:?}");
        assert!(!stdout_of(&output).contains("lifecycle-shell-ran"), "{command}: {output:?}");
        let stderr = stderr_of(&output);
        assert_incident_warning(&stderr);
        assert!(stderr.contains(&compact(failure)), "{command}: {stderr}");

        let output = run(&process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(1), "{command}: {output:?}");
        let (report, rows) = report_trigger_rows(&output);
        assert_eq!(rows, 1, "{command}: {report:#}");
        assert_eq!(report["outcome"], "failure", "{command}: {report:#}");
    }
}

/// A composed document that cannot be rendered fails after composition,
/// before the warnings that follow the document; the warning still reaches
/// stderr, with or without a lifecycle.
#[test]
fn compose_render_failure_keeps_the_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    for lifecycle in ["", "initialize:\n  - info: start\n"] {
        let document = write(
            process.cwd(),
            "docs/prompt.md",
            &format!("---\n{lifecycle}style:\n  page:\n    max-width: banana\n---\n# Prompt\n"),
        );

        let output = run(&process, &["compose", "--output", "html", document.to_str().unwrap()]);
        assert_eq!(output.status.code(), Some(1), "{lifecycle:?}: {output:?}");
        let stderr = stderr_of(&output);
        assert_incident_warning(&stderr);
        assert!(stderr.contains(&compact("Failed to parse `style:` frontmatter")), "{lifecycle:?}: {stderr}");
    }
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

/// Triggers are told apart by their full path, never by file name: two
/// packages each holding an invalid `schemas/memory.yaml` give one warning
/// each, each on the record of the file that discovered it.
#[test]
fn schema_validate_json_keeps_same_named_triggers_of_two_packages_distinct() {
    let process = CliProcessFixture::new();
    let root = process.cwd();
    initialize_repository(root);
    write(root, "Cargo.toml", "[workspace]\nmembers = [\"alpha\", \"beta\"]\n");
    let mut triggers = Vec::new();
    let mut documents = Vec::new();
    for package in ["alpha", "beta"] {
        write(root, &format!("{package}/Cargo.toml"), &format!("[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n"));
        write(root, &format!("{package}/src/lib.rs"), "");
        triggers.push(write(root, &format!("{package}/schemas/memory.yaml"), INCIDENT));
        documents.push(write(root, &format!("{package}/docs/notes.md"), "---\ntitle: notes\n---\n"));
    }

    let output = run(
        &process,
        &["schema", "validate", "--format", "json", documents[0].to_str().unwrap(), documents[1].to_str().unwrap()],
    );
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(!stderr_of(&output).contains(WARNING), "{output:?}");
    let records: Vec<Value> =
        stdout_of(&output).lines().map(|line| serde_json::from_str(line).expect("one JSON record per line")).collect();
    assert_eq!(records.len(), 2, "{records:#?}");
    for (record, trigger) in records.iter().zip(&triggers) {
        let warnings = record["warnings"].as_array().expect("each file carries its own package's warning");
        assert_eq!(warnings.len(), 1, "{records:#?}");
        assert_incident_row(&warnings[0], trigger);
    }
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

/// The warning, with its reason, goes to stderr once; the trace on stdout
/// names the skipped trigger without repeating the warning, and the valid
/// neighbor is still traced and matched.
#[test]
fn schema_triggers_warns_on_stderr_and_lists_the_skipped_trigger_on_stdout() {
    let process = CliProcessFixture::new();
    mixed_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\nkind: prompt\nowner: me\n---\n");

    let output = run(&process, &["schema", "triggers", document.to_str().unwrap()]);
    assert!(output.status.success(), "{output:?}");
    let stdout = stdout_of(&output);
    let skipped = stdout.find("Skipped invalid triggers:").expect("a skipped-triggers section");
    let triggers = stdout.find("Triggers:").expect("the triggers section");
    let section = &stdout[skipped..triggers];
    assert!(section.contains("memory.yaml") && section.contains(CODE), "{stdout}");
    assert!(stdout[triggers..].contains("prompt.trigger.yaml — matched"), "{stdout}");
    assert!(!compact(&stdout).contains(WARNING), "the warning is not repeated on stdout: {stdout}");
    assert!(!stdout.contains("found unknown keys"), "the warning is not repeated on stdout: {stdout}");
    assert_incident_warning(&stderr_of(&output));
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

/// An ordinary clean that fails on its authored `$schema` still writes the
/// warning discovered before the failure to stderr, once, beside the error.
#[test]
fn clean_keeps_the_warning_when_the_authored_schema_fails_to_load() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\n$schema: ./missing.yaml\n---\nPrompt body\n");

    let output = run(&process, &["clean", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    assert!(output.stdout.is_empty(), "{output:?}");
    let stderr = stderr_of(&output);
    assert_incident_warning(&stderr);
    assert!(stderr.contains(&compact("missing.yaml")), "{stderr}");
}

/// `--json` failing on its authored `$schema` still writes the version-1
/// envelope with the retained warning in `warnings`, keeps the schema error
/// on stderr and exit 1, and writes no warning row to stderr.
#[test]
fn clean_json_schema_failure_carries_the_warning_in_the_envelope() {
    let process = CliProcessFixture::new();
    let trigger = incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\n$schema: ./missing.yaml\n---\nPrompt body\n");

    let output = run(&process, &["clean", "--json", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = stderr_of(&output);
    assert!(!stderr.contains(WARNING), "report mode never repeats the warning: {stderr}");
    assert!(stderr.contains(&compact("missing.yaml")), "the schema error stays on stderr: {stderr}");
    let report: Value = serde_json::from_slice(&output.stdout).expect("the JSON envelope");
    assert_eq!(report["version"], 1, "{report:#}");
    assert_eq!(report["frontmatter"]["present"], true, "{report:#}");
    assert_eq!(report["diagnostics"], Value::Array(Vec::new()), "{report:#}");
    assert_eq!(report["applied"], Value::Array(Vec::new()), "{report:#}");
    assert_eq!(report["changed"], false, "{report:#}");
    let warnings = report["warnings"].as_array().unwrap();
    assert_eq!(warnings.len(), 1, "{report:#}");
    assert_incident_row(&warnings[0], &trigger);
}

// ── shell approval and preparation ─────────────────────────────────────────

/// Runs `document` ordinarily and as `--output report-json`, asserting the
/// exit status, one trigger warning in each mode, and `fragment` on stderr.
fn assert_compose_warns_once(process: &CliProcessFixture, document: &Path, code: i32, fragment: &str, case: &str) {
    let output = run(process, &["compose", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(code), "{case}: {output:?}");
    let stderr = stderr_of(&output);
    assert_incident_warning(&stderr);
    assert!(stderr.contains(&compact(fragment)), "{case}: {stderr}");

    let output = run(process, &["compose", "--output", "report-json", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(code), "{case}: {output:?}");
    let (report, rows) = report_trigger_rows(&output);
    assert_eq!(rows, 1, "{case}: {report:#}");
    assert_eq!(report["outcome"], if code == 0 { "success" } else { "failure" }, "{case}: {report:#}");
}

/// A shell no policy approves stops a non-interactive run in preparation;
/// the trigger the preparation walk skipped is delivered with the refusal.
#[test]
fn compose_approval_failure_keeps_the_root_warning() {
    let process = CliProcessFixture::new();
    incident_repository(&process);
    let document = write(process.cwd(), "docs/prompt.md", "---\ntitle: prompt\n---\nPrompt body\n\n::shell echo review-probe\n");

    assert_compose_warns_once(&process, &document, 1, "Approval required", "root");
    assert!(!stdout_of(&run(&process, &["compose", document.to_str().unwrap()])).contains("review-probe"));
}

/// Writes `docs/prompt.md` with `lifecycle` frontmatter lines and `body`.
fn prompt_with(process: &CliProcessFixture, lifecycle: &str, body: &str) -> PathBuf {
    write(process.cwd(), "docs/prompt.md", &format!("---\ntitle: prompt\n{lifecycle}---\nParent body\n\n{body}"))
}

const LIFECYCLES: [&str; 2] = ["", "initialize:\n  - info: start\n"];
const CHILD_AND_SHELL: &str = "::file ../alpha/docs/child.md\n\n::shell echo review-probe\n";

/// A trigger only a child composition skips holds is found by the
/// preparation walk alone; ordinary and lifecycle runs deliver it once. The
/// root's bootstrap finds no trigger, so it cannot stand in for the walk.
#[test]
fn compose_skipping_a_child_keeps_its_child_only_warning() {
    let process = CliProcessFixture::new();
    child_only_trigger_repository(&process);
    for lifecycle in LIFECYCLES {
        let document = prompt_with(&process, lifecycle, "::file ../alpha/docs/child.md when=false\n");
        assert_compose_warns_once(&process, &document, 0, "", &format!("{lifecycle:?} skipped child"));
    }
}

/// Preparation that refuses an unapproved shell delivers the child-only
/// trigger its walk found, in ordinary and lifecycle runs.
#[test]
fn compose_approval_failure_keeps_a_child_only_warning() {
    let process = CliProcessFixture::new();
    child_only_trigger_repository(&process);
    for lifecycle in LIFECYCLES {
        let document = prompt_with(&process, lifecycle, CHILD_AND_SHELL);
        assert_compose_warns_once(&process, &document, 1, "Approval required", &format!("{lifecycle:?} unapproved"));
    }
}

/// A user blacklist refusing the shell keeps the walk's child-only trigger.
#[test]
fn compose_blacklist_refusal_keeps_a_child_only_warning() {
    let process = CliProcessFixture::new();
    child_only_trigger_repository(&process);
    // The policy files sit beside the document.
    write(process.cwd(), "docs/.darkmatter-shell-blacklist", "exact echo review-probe\n");
    for lifecycle in LIFECYCLES {
        let document = prompt_with(&process, lifecycle, CHILD_AND_SHELL);
        assert_compose_warns_once(&process, &document, 1, "blacklist", &format!("{lifecycle:?} blacklisted"));
    }
}

/// The prompt whose earlier child holds the only trigger and whose later
/// sibling names a schema that does not exist.
fn sibling_failure_prompt(process: &CliProcessFixture, lifecycle: &str) -> PathBuf {
    child_only_trigger_repository(process);
    write(process.cwd(), "docs/bad.md", "---\ntitle: bad\n$schema: ./missing.yaml\n---\nBad body\n");
    prompt_with(process, lifecycle, "::file ../alpha/docs/child.md\n\n::file ./bad.md\n")
}

/// A later sibling's failure keeps the trigger an earlier sibling's walk
/// found, through inspection and ordinary composition.
#[test]
fn a_later_siblings_failure_keeps_an_earlier_child_only_warning() {
    let process = CliProcessFixture::new();
    let document = sibling_failure_prompt(&process, "");

    let output = run(&process, &["compose", "--shell", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(1), "{output:?}");
    let stderr = stderr_of(&output);
    assert_incident_warning(&stderr);
    assert!(stderr.contains(&compact("missing.yaml")), "{stderr}");

    assert_compose_warns_once(&process, &document, 1, "missing.yaml", "ordinary sibling failure");
}

/// The same sibling failure through a lifecycle run.
#[test]
fn a_later_siblings_failure_keeps_an_earlier_child_only_warning_in_a_lifecycle() {
    let process = CliProcessFixture::new();
    let document = sibling_failure_prompt(&process, LIFECYCLES[1]);

    assert_compose_warns_once(&process, &document, 1, "missing.yaml", "lifecycle sibling failure");
}

// ── explicit denial before effects ─────────────────────────────────────────

/// [`run`] with the host's `PATH`, for a document whose approved shell is `git`.
fn run_with_git(process: &CliProcessFixture, args: &[&str]) -> Output {
    process.command_builder().plain_terminal(1000, 50).host_path().build().args(args).output().expect("md runs")
}

/// Writes a prompt whose approved `git init` of `docs/{marker}` comes before
/// `child`, and returns it with the marker the shell would create.
fn approved_effect_prompt(process: &CliProcessFixture, lifecycle: &str, marker: &str, child: &str) -> (PathBuf, PathBuf) {
    write(process.cwd(), "docs/.darkmatter-shell-whitelist", "prefix git\n");
    let document = prompt_with(process, lifecycle, &format!("::shell git init --quiet {marker}\n\n{child}\n"));
    (document, process.cwd().join("docs").join(marker))
}

/// A denied warning stops the run as a policy stop before the approved
/// shell runs, in ordinary and `--output report-json` runs: the warning keeps
/// its severity and names the trigger, never a schema-load error.
fn assert_denied_before_the_shell(process: &CliProcessFixture, trigger: &Path, lifecycle: &str, child: &str, deny: &str) {
    let case = format!("{lifecycle:?} {child:?} --deny {deny}");
    let (document, marker) = approved_effect_prompt(process, lifecycle, "ran-marker", child);
    let document = document.to_str().unwrap();

    let output = run_with_git(process, &["compose", "--deny", deny, document]);
    assert_eq!(output.status.code(), Some(1), "{case}: {output:?}");
    let stderr = stderr_of(&output);
    assert!(stderr.contains(&compact("memory.yaml")) && stderr.contains(&compact("found unknown keys")), "{case}: {stderr}");
    assert!(stderr.contains(&compact("blocking")), "{case}: the policy stop: {stderr}");
    assert_eq!(stderr.matches(WARNING).count(), 1, "{case}: shown once: {stderr}");
    assert!(!stderr.contains(&compact("schema load failed")), "{case}: {stderr}");
    assert!(!marker.exists(), "{case}: the approved shell ran before the stop");

    let output = run_with_git(process, &["compose", "--output", "report-json", "--deny", deny, document]);
    assert_eq!(output.status.code(), Some(1), "{case}: {output:?}");
    let (report, rows) = report_trigger_rows(&output);
    assert_eq!(rows, 1, "{case}: {report:#}");
    assert_eq!(report["outcome"], "failure", "{case}: {report:#}");
    assert!(report["document"].is_null(), "{case}: {report:#}");
    let row = report["diagnostics"].as_array().unwrap().iter().find(|row| row["code"] == CODE).unwrap();
    assert_eq!(row["severity"], "warning", "{case}: {report:#}");
    let message = row["message"].as_str().unwrap();
    let names_trigger = [trigger.to_path_buf(), biscuit_file::canonicalize_simplified(trigger).unwrap()]
        .iter()
        .any(|path| message.contains(&path.display().to_string()));
    assert!(names_trigger && message.contains("found unknown keys"), "{case}: {report:#}");
    assert!(!marker.exists(), "{case}: the approved shell ran before the stop");
}

const SKIPPED_ALPHA_CHILD: &str = "::file ../alpha/docs/child.md when=false";

/// Only the shell-approval walk finds a trigger a skipped child holds; a
/// denial of the code or of everything stops before the approved shell,
/// while the default policy runs it and warns.
#[test]
fn compose_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell() {
    let process = CliProcessFixture::new();
    let trigger = child_only_trigger_repository(&process);
    for deny in [CODE, "all"] {
        assert_denied_before_the_shell(&process, &trigger, LIFECYCLES[0], SKIPPED_ALPHA_CHILD, deny);
    }

    let (document, marker) = approved_effect_prompt(&process, LIFECYCLES[0], "allowed-marker", SKIPPED_ALPHA_CHILD);
    let output = run_with_git(&process, &["compose", document.to_str().unwrap()]);
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert_incident_warning(&stderr_of(&output));
    assert!(marker.exists(), "the default policy runs the approved shell");
}

/// The same denial in a lifecycle run whose `initialize` continues.
#[test]
fn compose_lifecycle_deny_of_a_skipped_childs_warning_stops_before_an_approved_shell() {
    let process = CliProcessFixture::new();
    let trigger = child_only_trigger_repository(&process);
    for deny in [CODE, "all"] {
        assert_denied_before_the_shell(&process, &trigger, LIFECYCLES[1], SKIPPED_ALPHA_CHILD, deny);
    }
}

/// An active child's trigger denied stops before the root's approved shell,
/// in ordinary and lifecycle runs.
#[test]
fn compose_deny_of_an_active_childs_warning_stops_before_an_approved_shell() {
    let process = CliProcessFixture::new();
    let trigger = child_only_trigger_repository(&process);
    for lifecycle in LIFECYCLES {
        assert_denied_before_the_shell(&process, &trigger, lifecycle, "::file ../alpha/docs/child.md", CODE);
    }
}

/// `--shell` inspection runs nothing, so a denial does not change it: it
/// still lists the shell and warns about the skipped child's trigger.
#[test]
fn compose_shell_inspection_under_deny_warns_without_running_anything() {
    let process = CliProcessFixture::new();
    child_only_trigger_repository(&process);
    let (document, marker) = approved_effect_prompt(&process, LIFECYCLES[0], "ran-marker", SKIPPED_ALPHA_CHILD);

    let output = run_with_git(&process, &["compose", "--shell", "--deny", CODE, document.to_str().unwrap()]);

    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(stdout_of(&output).contains("git init --quiet ran-marker"), "{output:?}");
    assert_incident_warning(&stderr_of(&output));
    assert!(!marker.exists(), "inspection runs nothing");
}
