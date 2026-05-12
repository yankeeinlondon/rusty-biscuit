//! Integration tests for `md schema validate`.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn schema_validate_valid_inline_succeeds() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"));
}

#[test]
fn schema_validate_failing_returns_exit_1() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1);
}

#[test]
fn schema_validate_json_format_emits_ndjson() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"valid\":true"))
        .stdout(predicate::str::contains("\"problems\":[]"));
}

#[test]
fn schema_validate_json_format_reports_problems() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"valid\":false"))
        .stdout(predicate::str::contains("\"problems\""));
}

#[test]
fn schema_validate_no_schema_no_baseline_is_vacuous_success() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(&tmp, "no-schema.md", "---\nname: alice\n---\nBody\n");

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .success();
}

#[test]
fn schema_validate_quiet_suppresses_success_lines() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: Hello\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--quiet"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

#[test]
fn schema_validate_baseline_from_flag() {
    let tmp = TempDir::new().unwrap();
    let baseline = write_file(
        &tmp,
        "baseline.yaml",
        "$schema:\n  owner: 'string(required)'\n",
    );
    let doc = write_file(&tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    md_cmd()
        .args(["schema", "validate", "--schema"])
        .arg(&baseline)
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn schema_validate_baseline_from_env_var() {
    let tmp = TempDir::new().unwrap();
    let baseline = write_file(
        &tmp,
        "baseline.yaml",
        "$schema:\n  owner: 'string(required)'\n",
    );
    let doc = write_file(&tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .env("BASELINE_SCHEMA", &baseline)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("owner"));
}

#[test]
fn schema_validate_bad_baseline_exits_2() {
    let tmp = TempDir::new().unwrap();
    let bad = write_file(&tmp, "baseline.yaml", "not: a-schema\n");
    let doc = write_file(&tmp, "doc.md", "---\ntitle: hi\n---\nBody\n");

    md_cmd()
        .args(["schema", "validate", "--schema"])
        .arg(&bad)
        .arg(&doc)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_unparseable_frontmatter_exits_3() {
    let tmp = TempDir::new().unwrap();
    // Intentionally malformed YAML inside frontmatter delimiters.
    let doc = write_file(
        &tmp,
        "bad.md",
        "---\n: : : not valid yaml ::\n  - [unbalanced\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(3);
}

#[test]
fn schema_validate_multiple_files_aggregates_failure() {
    let tmp = TempDir::new().unwrap();
    let good = write_file(
        &tmp,
        "good.md",
        "---\n$schema:\n  title: 'string(required)'\ntitle: ok\n---\n",
    );
    let bad = write_file(
        &tmp,
        "bad.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&good)
        .arg(&bad)
        .assert()
        .code(1);
}
