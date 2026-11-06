//! Integration tests for `md schema detect`.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;
use std::io::Write;
use std::path::{Path, PathBuf};

fn write_file(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn schema_detect_yaml_emits_simplified_schema() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(tmp, "doc.md", "---\ntitle: Hello\ncount: 42\n---\nBody\n");

    process
        .command()
        .args(["schema", "detect"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("$schema:"))
        .stdout(predicate::str::contains("title: string"))
        .stdout(predicate::str::contains("count: number(integer)"));
}

#[test]
fn schema_detect_json_emits_the_simplified_schema_as_data() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(tmp, "doc.md", "---\ntitle: Hello\n---\nBody\n");

    let output = process.command().args(["schema", "detect", "--output", "json"]).arg(&doc).output().unwrap();

    assert!(output.status.success(), "{output:?}");
    assert!(output.stderr.is_empty(), "{output:?}");
    let schema: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(schema, serde_json::json!({ "$schema": { "title": "string" } }));
}

#[test]
fn schema_detect_json_schema_output_emits_draft_2020_12() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let doc = write_file(tmp, "doc.md", "---\ntitle: Hello\n---\nBody\n");

    process
        .command()
        .args(["schema", "detect", "--output", "json-schema"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("2020-12"))
        .stdout(predicate::str::contains("\"type\": \"object\""))
        .stdout(predicate::str::contains("\"title\""));
}

#[test]
fn schema_detect_merge_promotes_required() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let a = write_file(tmp, "a.md", "---\ntitle: A\n---\n");
    let b = write_file(tmp, "b.md", "---\ntitle: B\n---\n");

    process
        .command()
        .args(["schema", "detect", "--merge"])
        .arg(&a)
        .arg(&b)
        .assert()
        .success()
        .stdout(predicate::str::contains("required"));
}

#[test]
fn schema_detect_no_merge_emits_per_file_headers() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let a = write_file(tmp, "a.md", "---\ntitle: A\n---\n");
    let b = write_file(tmp, "b.md", "---\nname: B\n---\n");

    process
        .command()
        .args(["schema", "detect"])
        .arg(&a)
        .arg(&b)
        .assert()
        .success()
        .stdout(predicate::str::contains("a.md"))
        .stdout(predicate::str::contains("b.md"));
}

#[test]
fn schema_detect_unparseable_frontmatter_exits_1() {
    let process = CliProcessFixture::new();
    let tmp = process.cwd();
    let bad = write_file(
        tmp,
        "bad.md",
        "---\n: : : not valid yaml ::\n  - [unbalanced\n---\nBody\n",
    );

    process
        .command()
        .args(["schema", "detect"])
        .arg(&bad)
        .assert()
        .code(1);
}
