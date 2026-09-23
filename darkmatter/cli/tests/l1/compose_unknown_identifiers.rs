//! End-to-end `md compose` coverage for Requirement 4 of the
//! dasherized-identifiers spec
//! (`darkmatter/features/2026-09-15-dasherized-identifiers`): an unknown
//! identifier warns once on stderr with its file and line, the command still
//! succeeds with the value rendered empty, and stdout carries only the
//! document.

use crate::common;

use common::CliProcessFixture;

struct Run {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

fn compose(name: &str, relative: &str, content: &str, extra: &[&str]) -> Run {
    let fixture = CliProcessFixture::named(name);
    let document = fixture.write_file(format!("cwd/{relative}"), content);

    let output = fixture
        .command()
        .arg("compose")
        .arg(&document)
        .args(extra)
        .output()
        .unwrap();

    Run {
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

const UNDECLARED: &str = "---\ntitle: T\n---\n[{{ iteration-1 }}]\n\n[{{ iteration-1 }}]\n";

#[test]
fn compose_warns_once_for_an_unknown_identifier_and_still_succeeds() {
    let run = compose(
        "compose_warns_once_for_an_unknown_identifier_and_still_succeeds",
        "doc.md",
        UNDECLARED,
        &[],
    );

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert_eq!(run.stdout.trim_end(), "[]\n\n[]", "the value renders empty");
    assert_eq!(run.stderr.matches("unknown identifier").count(), 1, "{}", run.stderr);
    assert!(run.stderr.contains("unknown identifier 'iteration-1'"), "{}", run.stderr);
    assert!(run.stderr.contains("doc.md:4"), "the first read's file and line: {}", run.stderr);
    assert!(!run.stdout.contains("unknown identifier"), "warnings stay on stderr: {}", run.stdout);
}

#[test]
fn compose_is_silent_for_a_root_supplied_with_set_even_when_null() {
    let run = compose(
        "compose_is_silent_for_a_root_supplied_with_set_even_when_null",
        "doc.md",
        UNDECLARED,
        &["--set", r#"{"iteration-1": null}"#],
    );

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(!run.stderr.contains("unknown identifier"), "{}", run.stderr);
}

#[test]
fn compose_reports_only_the_schema_failure_for_an_unset_required_root() {
    let run = compose(
        "compose_reports_only_the_schema_failure_for_an_unset_required_root",
        "doc.md",
        "---\n$schema:\n  iteration-1: number(required)\n---\n[{{ iteration-1 }}]\n",
        &[],
    );

    assert_eq!(run.code, Some(1), "stderr: {}", run.stderr);
    assert!(run.stdout.is_empty(), "{}", run.stdout);
    assert!(run.stderr.contains("iteration-1"), "{}", run.stderr);
    assert!(!run.stderr.contains("unknown identifier"), "one issue, one message: {}", run.stderr);
}

/// `--output json` stays document-only; the warning is still on stderr.
#[test]
fn compose_json_output_carries_no_warning() {
    let run = compose(
        "compose_json_output_carries_no_warning",
        "doc.md",
        UNDECLARED,
        &["--output", "json"],
    );

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    serde_json::from_str::<serde_json::Value>(&run.stdout).expect("stdout is one JSON document");
    assert!(!run.stdout.contains("unknown identifier"), "{}", run.stdout);
    assert_eq!(run.stderr.matches("unknown identifier").count(), 1, "{}", run.stderr);
}

/// The message is rendered as Prose; the underscores of a partial's path are
/// literal, not emphasis.
#[test]
fn compose_renders_an_underscored_path_literally() {
    let run = compose(
        "compose_renders_an_underscored_path_literally",
        "_partials/_part.md",
        "{{ some_root }}\n",
        &[],
    );

    assert_eq!(run.code, Some(0), "stderr: {}", run.stderr);
    assert!(run.stderr.contains("unknown identifier 'some_root'"), "{}", run.stderr);
    assert!(run.stderr.contains("_part.md:1"), "{}", run.stderr);
}
