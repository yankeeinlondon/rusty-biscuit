//! Integration tests for `md schema about`.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;

fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

#[test]
fn schema_about_prints_simplified_schema_reference() {
    md_cmd()
        .args(["schema", "about"])
        .assert()
        .success()
        .stdout(predicate::str::contains("SimplifiedSchema"))
        .stdout(predicate::str::contains("Schema Shapes"))
        .stdout(predicate::str::contains("Type Vocabulary"))
        .stdout(predicate::str::contains("Constraint Vocabulary"))
        .stdout(predicate::str::contains("Inline Object Rules"))
        .stdout(predicate::str::contains("Compose-time Coercion"))
        .stdout(predicate::str::contains("Validation Behaviour"));
}

#[test]
fn schema_about_lists_every_supported_type_keyword() {
    let mut cmd = md_cmd();
    let output = cmd.args(["schema", "about"]).output().expect("run md schema about");
    let stdout = String::from_utf8(output.stdout).expect("utf-8 stdout");
    for keyword in [
        "string",
        "date",
        "datetime",
        "time",
        "number",
        "numberlike",
        "boolean",
        "boolish",
        "object",
        "file",
        "enum",
        "url",
        "email",
        "any",
    ] {
        assert!(
            stdout.contains(keyword),
            "schema about report missing type keyword `{keyword}`"
        );
    }
}

#[test]
fn schema_about_mentions_inline_object_rules() {
    md_cmd()
        .args(["schema", "about"])
        .assert()
        .success()
        .stdout(predicate::str::contains("additionalProperties: false"))
        .stdout(predicate::str::contains("32"))
        .stdout(predicate::str::contains("Brace delimiters"));
}

#[test]
fn schema_about_exits_zero() {
    md_cmd().args(["schema", "about"]).assert().success();
}

#[test]
fn schema_about_is_documentation_only() {
    // Running `md schema about` must not require or read any markdown file.
    // We assert that:
    //   1. Running from an empty / non-existent working directory produces
    //      the same key sections as running from the project root.
    //   2. Both invocations exit with status `0`.
    // (Strict byte-for-byte equality is not portable: terminal capability
    // detection in the prose renderer can change the exact escape sequences
    // emitted between sessions, but the textual content is stable.)
    let tmp = tempfile::TempDir::new().unwrap();
    let output_a = md_cmd()
        .args(["schema", "about"])
        .current_dir(tmp.path())
        .output()
        .expect("run md schema about from temp dir");
    let output_b = md_cmd()
        .args(["schema", "about"])
        .output()
        .expect("run md schema about from project root");

    assert!(output_a.status.success(), "first invocation should succeed");
    assert!(output_b.status.success(), "second invocation should succeed");

    let a = String::from_utf8_lossy(&output_a.stdout);
    let b = String::from_utf8_lossy(&output_b.stdout);
    for needle in [
        "SimplifiedSchema",
        "Type Vocabulary",
        "Constraint Vocabulary",
        "Inline Object Rules",
        "Compose-time Coercion",
        "Validation Behaviour",
    ] {
        assert!(
            a.contains(needle) && b.contains(needle),
            "schema about is missing `{needle}` from one of the two runs"
        );
    }
}
