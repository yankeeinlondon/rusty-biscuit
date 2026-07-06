mod common;

use common::md_cmd;
use predicates::prelude::*;
use std::path::PathBuf;

fn write_file(dir: &tempfile::TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    std::fs::write(&path, content).unwrap();
    path
}

#[test]
fn compose_uses_darkmatter_base_schema_by_default() {
    let dir = tempfile::TempDir::new().unwrap();
    let doc = write_file(&dir, "bad-draft.md", "---\ndraft: maybe\n---\nBody\n");

    md_cmd()
        .arg("compose")
        .arg(&doc)
        .env_remove("DARKMATTER_NO_BASELINE_SCHEMA")
        .assert()
        .failure()
        .stderr(predicate::str::contains("draft"));
}

#[test]
fn compose_generated_ctx_reference_succeeds_without_authored_ctx() {
    let dir = tempfile::TempDir::new().unwrap();
    let doc = write_file(&dir, "ctx.md", "---\ntitle: Context\n---\n{{ ctx.today }}\n");

    md_cmd()
        .arg("compose")
        .arg(&doc)
        .env_remove("DARKMATTER_NO_BASELINE_SCHEMA")
        .assert()
        .success()
        .stdout(predicate::str::contains("-"));
}

#[test]
fn compose_default_baseline_rejects_custom_ctx_keys() {
    let dir = tempfile::TempDir::new().unwrap();
    let doc = write_file(
        &dir,
        "custom-ctx.md",
        "---\nctx:\n  project_slug: biscuit\n---\n{{ ctx.project_slug }}\n",
    );

    md_cmd()
        .arg("compose")
        .arg(&doc)
        .env_remove("DARKMATTER_NO_BASELINE_SCHEMA")
        .assert()
        .failure()
        .stderr(predicate::str::contains("ctx"));
}

#[test]
fn compose_document_schema_precedence_is_preserved() {
    let dir = tempfile::TempDir::new().unwrap();
    let doc = write_file(
        &dir,
        "override.md",
        "---\n$schema:\n  title: number\ntitle: still text\n---\nBody\n",
    );

    md_cmd()
        .arg("compose")
        .arg(&doc)
        .env_remove("DARKMATTER_NO_BASELINE_SCHEMA")
        .assert()
        .failure()
        .stderr(predicate::str::contains("title"));
}

#[test]
fn compose_unknown_frontmatter_keys_remain_allowed() {
    let dir = tempfile::TempDir::new().unwrap();
    let doc = write_file(&dir, "custom.md", "---\ncustom_key: 42\n---\nBody\n");

    md_cmd()
        .arg("compose")
        .arg(&doc)
        .env_remove("DARKMATTER_NO_BASELINE_SCHEMA")
        .assert()
        .success()
        .stdout(predicate::str::contains("Body"));
}

#[test]
fn compose_no_baseline_schema_disables_default_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let doc = write_file(&dir, "raw.md", "---\ndraft: maybe\n---\nBody\n");

    md_cmd()
        .args(["compose", "--no-baseline-schema"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("Body"));
}

#[test]
fn compose_no_baseline_schema_env_disables_default_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let doc = write_file(&dir, "raw-env.md", "---\ndraft: maybe\n---\nBody\n");

    md_cmd()
        .arg("compose")
        .arg(&doc)
        .env("DARKMATTER_NO_BASELINE_SCHEMA", "1")
        .assert()
        .success()
        .stdout(predicate::str::contains("Body"));
}

#[test]
fn compose_custom_baseline_schema_overrides_default_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let schema = write_file(&dir, "schema.yaml", "$schema:\n  custom: string\n");
    let doc = write_file(&dir, "custom-baseline.md", "---\ndraft: maybe\n---\nBody\n");

    md_cmd()
        .arg("compose")
        .arg("--baseline-schema")
        .arg(&schema)
        .arg(&doc)
        .env_remove("DARKMATTER_NO_BASELINE_SCHEMA")
        .assert()
        .success()
        .stdout(predicate::str::contains("Body"));
}

#[test]
fn compose_explicit_baseline_schema_wins_over_no_baseline_schema_env() {
    let dir = tempfile::TempDir::new().unwrap();
    let schema = write_file(
        &dir,
        "schema.yaml",
        "$schema:\n  custom: string(pattern(^[a-z]+$))\n",
    );
    let doc = write_file(&dir, "custom-env.md", "---\ncustom: 42\n---\nBody\n");

    md_cmd()
        .arg("compose")
        .arg("--baseline-schema")
        .arg(&schema)
        .arg(&doc)
        .env("DARKMATTER_NO_BASELINE_SCHEMA", "1")
        .assert()
        .failure()
        .stderr(predicate::str::contains("custom"));
}
