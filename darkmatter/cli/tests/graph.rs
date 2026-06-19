mod common;

use common::{md_cmd, md_file};
use predicates::prelude::*;
use std::io::Write;

#[test]
fn test_graph_basic() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    writeln!(
        tmp,
        "# Test\n\n[link](https://example.com)\n\n![img](./logo.png)"
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(tmp.path())
        .assert()
        .success()
        .stdout(predicate::str::contains("example.com"))
        .stdout(predicate::str::contains("logo.png"));
}

#[test]
fn test_graph_follow() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("parent.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Parent\n\n::file child.md").unwrap();
    std::fs::write(&child, "# Child\n\n[link](https://child.example.com)").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("parent.md"))
        .stdout(predicate::str::contains("child.md"));
}

#[test]
fn test_graph_validate_valid() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("valid.md");
    let linked = dir.path().join("linked.md");
    std::fs::write(&md_path, "# Valid\n\n[link](./linked.md)").unwrap();
    std::fs::write(&linked, "# Linked").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&md_path)
        .arg("--validate")
        .assert()
        .success()
        .stdout(predicate::str::contains("valid"))
        .stdout(predicate::str::contains("0 issues"));
}

#[test]
fn test_graph_validate_invalid() {
    let tmp = md_file("# Test\n\n[broken](./nonexistent.md)\n");

    let output = md_cmd()
        .arg("graph")
        .arg(tmp.path())
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 for validation errors"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[missing]"),
        "expected [missing] suffix in output"
    );
    assert!(
        stdout.contains("1 issues"),
        "expected issue count in summary"
    );
}

#[test]
fn test_graph_follow_toc_linking() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("root.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Root\n\n::toc-linking child.md").unwrap();
    std::fs::write(
        &child,
        "# Child\n\n## Section A\n\n## Section B\n\n[link](https://child.example.com)",
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("root.md"))
        .stdout(predicate::str::contains("child.md"))
        .stdout(predicate::str::contains("child.example.com"));
}

#[test]
fn test_graph_follow_validate_child_broken_link() {
    let dir = tempfile::TempDir::new().unwrap();
    let parent = dir.path().join("root.md");
    let child = dir.path().join("child.md");
    std::fs::write(&parent, "# Root\n\n::toc-linking child.md").unwrap();
    std::fs::write(&child, "# Child\n\n[broken](./missing.md)").unwrap();

    let output = md_cmd()
        .arg("graph")
        .arg(&parent)
        .arg("--follow")
        .arg("--validate")
        .output()
        .unwrap();

    assert_eq!(
        output.status.code(),
        Some(2),
        "expected exit code 2 when followed child has a broken link"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[missing]"),
        "expected [missing] suffix for broken link in child"
    );
}

#[test]
fn test_graph_follow_multiple_prologues() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().join("root.md");
    let a = dir.path().join("a.md");
    let b = dir.path().join("b.md");
    std::fs::write(&root, "---\nprologue:\n  - a.md\n  - b.md\n---\n\n# Root").unwrap();
    std::fs::write(&a, "# A\n\n[a-link](https://a.example.com)").unwrap();
    std::fs::write(&b, "# B\n\n[b-link](https://b.example.com)").unwrap();

    md_cmd()
        .arg("graph")
        .arg(&root)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("a.md"))
        .stdout(predicate::str::contains("b.md"))
        .stdout(predicate::str::contains("a.example.com"))
        .stdout(predicate::str::contains("b.example.com"));
}

#[test]
fn test_graph_follow_epilogue() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path().join("root.md");
    let epilogue = dir.path().join("epilogue.md");
    std::fs::write(
        &root,
        "---\nepilogue: epilogue.md\n---\n\n# Root\n\n[main](https://main.example.com)",
    )
    .unwrap();
    std::fs::write(
        &epilogue,
        "# Epilogue\n\n[epi-link](https://epilogue.example.com)",
    )
    .unwrap();

    md_cmd()
        .arg("graph")
        .arg(&root)
        .arg("--follow")
        .assert()
        .success()
        .stdout(predicate::str::contains("root.md"))
        .stdout(predicate::str::contains("epilogue.md"))
        .stdout(predicate::str::contains("epilogue.example.com"));
}

#[test]
fn test_graph_file_not_found() {
    md_cmd()
        .arg("graph")
        .arg("/nonexistent/file.md")
        .assert()
        .failure();
}

#[test]
fn test_graph_help() {
    md_cmd()
        .arg("graph")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("--follow"))
        .stdout(predicate::str::contains("--validate"));
}


#[test]
fn test_graph_json_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("test.md");
    std::fs::write(&file, "# Test\n\n[link](https://example.com)").unwrap();

    md_cmd()
        .args(["graph", "--json"])
        .arg(&file)
        .assert()
        .success()
        .stdout(predicate::str::contains("{"))
        .stdout(predicate::str::contains("\"references\""))
        .stdout(predicate::str::contains("example.com"));
}

// ── JSON baseline fixtures (byte-for-byte compatibility) ────────────────
//
// The Phase 1 leak extraction (CLI Atheist / Leak 3) replaced the
// hand-rolled CLI JSON serializers with `#[derive(serde::Serialize)]`
// on the library reference types. These tests pin the public JSON shape
// of `md graph --json` against the captured baseline fixtures under
// `darkmatter/features/2026-06-17-cli-atheist/baseline/json/`.

use common::baseline;

/// Runs `md graph` with the supplied args, parses stdout as JSON,
/// normalizes temp paths / hash prefixes, and compares the full value
/// against the named baseline fixture.
fn assert_graph_json_matches_baseline(
    args: &[&str],
    temp_dir: &std::path::Path,
    baseline_name: &str,
) {
    let output = md_cmd()
        .args(args)
        .output()
        .expect("md command failed to spawn");

    assert!(
        output.status.success(),
        "md graph --json failed with status {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr),
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("md graph output must be valid JSON");

    let redact = baseline::paths_to_redact(temp_dir);
    let redact_refs: Vec<&str> = redact.iter().map(|s| s.as_str()).collect();
    let actual_norm = baseline::normalize(actual, &redact_refs);
    let expected_norm = baseline::normalize(baseline::load_json(baseline_name), &redact_refs);

    assert_eq!(
        actual_norm, expected_norm,
        "md graph --json output did not match baseline {baseline_name}\n\
         raw output:\n{stdout}",
    );
}

#[test]
fn graph_json_local_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("local.md"), "# Local Test\n\n[local link](./other.md)\n![local image](./img.png)\n::file other.md\n").unwrap();
    std::fs::write(root.join("other.md"), "# Other\n").unwrap();
    let local = root.join("local.md");
    assert_graph_json_matches_baseline(
        &["graph", "--json", local.to_str().unwrap()],
        root,
        "graph_local.json",
    );
}

#[test]
fn graph_json_follow_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("prologue.md"),
        "---\nprologue: other.md\n---\n\n# Prologue\n",
    )
    .unwrap();
    std::fs::write(root.join("other.md"), "# Other\n").unwrap();
    let prologue = root.join("prologue.md");
    assert_graph_json_matches_baseline(
        &[
            "graph",
            "--json",
            "--follow",
            prologue.to_str().unwrap(),
        ],
        root,
        "graph_follow.json",
    );
}

#[test]
fn graph_json_validate_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("errors.md"), "# Errors\n\n[missing](./missing.md)\n").unwrap();
    let errors = root.join("errors.md");
    assert_graph_json_matches_baseline(
        &[
            "graph",
            "--json",
            "--validate",
            errors.to_str().unwrap(),
        ],
        root,
        "graph_validate.json",
    );
}

