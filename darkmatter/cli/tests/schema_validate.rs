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
fn schema_validate_pretty_reports_line_for_type_mismatch() {
    let tmp = TempDir::new().unwrap();
    // `rating` lands on line 3 of the canonical re-serialised frontmatter.
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\nrating: nope\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("at line"))
        .stdout(predicate::str::contains("of frontmatter"));
}

#[test]
fn schema_validate_json_reports_arm_index_for_root_union() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  - title: 'string(required)'\n  - name: 'string(required)'\nother: value\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"arm_index\":0"));
}

#[test]
fn schema_validate_unresolved_document_schema_exits_2() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "post.md",
        "---\n$schema: ./missing.yaml\ntitle: hi\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_unresolved_document_schema_outranks_validation_failure() {
    let tmp = TempDir::new().unwrap();
    // One file has a missing `$schema` reference (schema-load error → 2);
    // the other has a normal validation failure (→ 1). Schema-load errors
    // outrank validation failures so the overall exit code must be 2.
    let bad_schema = write_file(
        &tmp,
        "bad_schema.md",
        "---\n$schema: ./missing.yaml\ntitle: hi\n---\nBody\n",
    );
    let bad_value = write_file(
        &tmp,
        "bad_value.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&bad_schema)
        .arg(&bad_value)
        .assert()
        .code(2);
}

#[test]
fn schema_validate_pretty_prefixes_root_union_problems_with_arm_index() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  - title: 'string(required)'\n  - name: 'string(required)'\nother: value\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("arm["));
}

#[test]
fn schema_validate_pretty_does_not_render_root_label_as_markup() {
    let tmp = TempDir::new().unwrap();
    // Missing-required at the root produced `<root>` markup which the
    // Prose renderer interpreted as a tag. The rendered output must not
    // leak a closing `</root>` (or any other angle-bracketed artifact).
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  title: 'string(required)'\nother: stuff\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("</root>").not())
        .stdout(predicate::str::contains("<root>").not());
}

#[test]
fn schema_validate_pretty_reports_source_line_for_problem() {
    let tmp = TempDir::new().unwrap();
    // The opening `---` is line 1. `$schema:` is line 2, the inline
    // mapping spans lines 3-4, the blank comment is line 5, and the
    // invalid `rating: nope` value is on line 6 of the source. The
    // position must be reported against the original source, not against
    // a re-serialised view.
    let doc = write_file(
        &tmp,
        "draft.md",
        "---\n$schema:\n  rating: number\n# important: do not reorder\n\nrating: nope\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "validate", "--format", "json"])
        .arg(&doc)
        .assert()
        .code(1)
        .stdout(predicate::str::contains("\"line\":6"));
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
