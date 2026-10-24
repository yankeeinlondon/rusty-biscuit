//! `md schema validate -vv` stage timings: the report goes to stderr only,
//! after the document's existing output, and one `-v` changes nothing. Only
//! names are asserted, never durations.

use std::path::{Path, PathBuf};
use std::process::Output;

use crate::common;

use common::CliProcessFixture;

const STAGE_LABELS: [&str; 7] = ["discover", "match", "resolve", "compile", "validate", "unassigned", "total"];

/// Writes `content` at `relative` under the fixture's repository (its cwd).
fn write(fixture: &CliProcessFixture, relative: &str, content: &str) -> PathBuf {
    let path = fixture.cwd().join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// A repository fixture with one trigger that matches `doc.md`.
fn triggered_repository() -> (CliProcessFixture, PathBuf) {
    let fixture = CliProcessFixture::named("schema-validate-timings");
    assert!(fixture.initialize_repository());
    write(
        &fixture,
        "schemas/timed-trigger.yaml",
        "kind: schema-trigger\nmatch:\n  timed_prompt: string(required)\n$schema: timed-payload.yaml\n",
    );
    write(&fixture, "schemas/timed-payload.yaml", "$schema:\n  timed_model: string(required)\n");
    let document = write(&fixture, "doc.md", "---\ntimed_prompt: hello\ntimed_model: m\n---\nbody\n");
    (fixture, document)
}

fn validate(fixture: &CliProcessFixture, args: &[&str], document: &Path) -> Output {
    fixture
        .command()
        .args(["schema", "validate"])
        .args(args)
        .arg(document)
        .output()
        .expect("run md schema validate")
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_stage_report(stderr: &str) {
    assert!(stderr.contains("Validation timings"), "no timing report:\n{stderr}");
    for label in STAGE_LABELS.iter().chain(&["compile cache:", "matched triggers:"]) {
        assert!(stderr.contains(label), "`{label}` missing from:\n{stderr}");
    }
}

#[test]
fn double_verbose_adds_a_stage_report_to_stderr_only() {
    let (fixture, document) = triggered_repository();

    let quiet = validate(&fixture, &[], &document);
    let verbose = validate(&fixture, &["-vv"], &document);

    assert_eq!(verbose.status.code(), quiet.status.code());
    assert_eq!(verbose.stdout, quiet.stdout, "stdout is unchanged");
    assert!(!stderr(&quiet).contains("Validation timings"), "without -v nothing is added");
    let report = stderr(&verbose);
    assert!(report.starts_with(&stderr(&quiet)), "the report follows the existing output");
    assert_stage_report(&report);
    assert!(report.contains("matched triggers: timed-trigger.yaml"), "{report}");
}

#[test]
fn single_verbose_output_is_unchanged() {
    let (fixture, document) = triggered_repository();

    let quiet = validate(&fixture, &[], &document);
    let verbose = validate(&fixture, &["-v"], &document);

    assert_eq!(verbose.status.code(), quiet.status.code());
    assert_eq!(verbose.stdout, quiet.stdout);
    assert_eq!(verbose.stderr, quiet.stderr);
    for label in ["Validation timings", "compile cache:", "unassigned"] {
        assert!(!stderr(&verbose).contains(label), "`{label}` under one -v");
    }
}

#[test]
fn json_stdout_is_unchanged_under_double_verbose() {
    let (fixture, document) = triggered_repository();

    let quiet = validate(&fixture, &["--format", "json"], &document);
    let verbose = validate(&fixture, &["--format", "json", "-vv"], &document);

    assert_eq!(verbose.status.code(), quiet.status.code());
    assert_eq!(verbose.stdout, quiet.stdout, "the JSON report is unchanged");
    assert_stage_report(&stderr(&verbose));
}

#[test]
fn unresolvable_schema_keeps_its_error_and_reports_partial_timings() {
    let fixture = CliProcessFixture::named("schema-validate-timings-failure");
    assert!(fixture.initialize_repository());
    let document = write(&fixture, "broken.md", "---\n$schema: ./missing-schema.yaml\ntitle: x\n---\n");

    let quiet = validate(&fixture, &[], &document);
    let verbose = validate(&fixture, &["-vv"], &document);

    assert_eq!(quiet.status.code(), Some(2), "a schema that does not resolve exits 2");
    assert_eq!(verbose.status.code(), Some(2));
    assert_eq!(verbose.stdout, quiet.stdout);
    let report = stderr(&verbose);
    assert!(report.starts_with(&stderr(&quiet)), "the existing error is printed unchanged first");
    assert_stage_report(&report);
    assert!(report.contains("compile cache: not requested"), "{report}");
}
