mod common;

use common::md_cmd;

#[test]
fn validate_refs_text_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Heading\n\n[link](https://example.com)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .assert()
        .success();
}

#[test]
fn validate_refs_json_output() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--format")
        .arg("json")
        .output()
        .unwrap();

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // JSON output should be parseable
    let _: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
        panic!("Expected valid JSON, got error: {e}\nOutput: {stdout}");
    });
}

#[test]
fn validate_refs_nonzero_exit_on_errors() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "[broken](./nonexistent.md)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .assert()
        .failure();
}

#[test]
fn validate_refs_with_fragments() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Hello\n\n[link](#hello)\n").unwrap();

    md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--fragments")
        .assert()
        .success();
}

#[test]
fn validate_refs_graph_mermaid() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n\n[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--graph")
        .arg("mermaid")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("flowchart TD"),
        "Expected mermaid flowchart output, got: {stdout}"
    );
}

#[test]
fn validate_refs_graph_dot() {
    let dir = tempfile::TempDir::new().unwrap();
    let md_path = dir.path().join("test.md");
    std::fs::write(&md_path, "# Test\n\n[link](https://example.com)\n").unwrap();

    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(&md_path)
        .arg("--graph")
        .arg("dot")
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("digraph"),
        "Expected dot graph output, got: {stdout}"
    );
}

