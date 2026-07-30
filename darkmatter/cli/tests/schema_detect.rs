//! Integration tests for `md schema detect`.

use predicates::prelude::*;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

fn md_cmd() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("md").unwrap()
}

fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn schema_detect_yaml_emits_simplified_schema() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(&tmp, "doc.md", "---\ntitle: Hello\ncount: 42\n---\nBody\n");

    md_cmd()
        .args(["schema", "detect"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("$schema:"))
        .stdout(predicate::str::contains("title: string"))
        .stdout(predicate::str::contains("count: number(integer)"));
}

#[test]
fn schema_detect_json_emits_json_schema() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(&tmp, "doc.md", "---\ntitle: Hello\n---\nBody\n");

    md_cmd()
        .args(["schema", "detect", "--format", "json"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"$schema\""))
        .stdout(predicate::str::contains("\"type\": \"object\""))
        .stdout(predicate::str::contains("\"title\""));
}

#[test]
fn schema_detect_merge_promotes_required() {
    let tmp = TempDir::new().unwrap();
    let a = write_file(&tmp, "a.md", "---\ntitle: A\n---\n");
    let b = write_file(&tmp, "b.md", "---\ntitle: B\n---\n");

    md_cmd()
        .args(["schema", "detect", "--merge"])
        .arg(&a)
        .arg(&b)
        .assert()
        .success()
        .stdout(predicate::str::contains("required"));
}

#[test]
fn schema_detect_no_merge_emits_per_file_headers() {
    let tmp = TempDir::new().unwrap();
    let a = write_file(&tmp, "a.md", "---\ntitle: A\n---\n");
    let b = write_file(&tmp, "b.md", "---\nname: B\n---\n");

    md_cmd()
        .args(["schema", "detect"])
        .arg(&a)
        .arg(&b)
        .assert()
        .success()
        .stdout(predicate::str::contains("a.md"))
        .stdout(predicate::str::contains("b.md"));
}

#[test]
fn schema_detect_unparseable_frontmatter_exits_3() {
    let tmp = TempDir::new().unwrap();
    let bad = write_file(
        &tmp,
        "bad.md",
        "---\n: : : not valid yaml ::\n  - [unbalanced\n---\nBody\n",
    );

    md_cmd()
        .args(["schema", "detect"])
        .arg(&bad)
        .assert()
        .code(3);
}
