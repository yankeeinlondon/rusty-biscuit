mod common;

use common::{baseline, md_cmd};

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

// ── JSON baseline fixtures (byte-for-byte compatibility) ────────────────
//
// `md validate refs --json` serializes the library
// `ReferenceValidationReport` directly via `Serialize` (the same shape
// `md graph --validate --json` uses for its `validation` block). These
// tests pin that public shape against the baseline fixtures under
// `darkmatter/features/2026-06-17-cli-atheist/baseline/json/`. Each
// test reproduces the source content used to capture the baseline,
// runs the serde-backed JSON path, normalizes temp paths and any
// remaining FNV-1a reference-id hash prefix, then compares the full
// JSON value.

/// Asserts that `md validate refs --json <input>` produces output that,
/// after environment normalization, equals the named baseline fixture.
///
/// Note: `md validate refs` exits with a non-zero status when validation
/// finds errors, even in `--format json` mode (the JSON payload is still
/// printed to stdout before the process exits). The baseline fixtures
/// capture both the success and the error shape, so we must not require
/// `status.success()` here.
fn assert_json_matches_baseline(input: &std::path::Path, baseline_name: &str) {
    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(input)
        .arg("--format")
        .arg("json")
        .output()
        .expect("md command failed to spawn");

    // The CLI always prints the JSON report to stdout even when the
    // subsequent validation summary errors out and sets a non-zero exit.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
            panic!(
                "md validate refs --json did not produce valid JSON: {e}\n\
                 status: {:?}\nstderr: {}\nstdout: {stdout}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr),
            )
        });

    let parent = input.parent().expect("input has a parent dir");
    let redact = baseline::paths_to_redact(parent);
    let redact_refs: Vec<&str> = redact.iter().map(|s| s.as_str()).collect();
    let actual_norm = baseline::normalize(actual, &redact_refs);

    let expected = baseline::load_json(baseline_name);
    let expected_norm = baseline::normalize(expected, &redact_refs);

    assert_eq!(
        actual_norm, expected_norm,
        "md validate refs --json output did not match baseline {baseline_name}\n\
         status: {:?}\nraw output:\n{stdout}",
        output.status.code(),
    );
}

#[test]
fn validate_refs_json_local_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("local.md"), "# Local Test\n\n[local link](./other.md)\n![local image](./img.png)\n::file other.md\n").unwrap();
    // `other.md` exists so the hyperlink and the `::file` transclusion are
    // both valid; only the image reference at `./img.png` is broken.
    std::fs::write(root.join("other.md"), "# Other\n").unwrap();
    assert_json_matches_baseline(&root.join("local.md"), "validate_refs_local.json");
}

#[test]
fn validate_refs_json_remote_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("remote.md"), "# Remote\n\n[remote](https://example.com)\n").unwrap();
    assert_json_matches_baseline(&root.join("remote.md"), "validate_refs_remote.json");
}

#[test]
fn validate_refs_json_fragment_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("fragment.md"), "# Fragment\n\n[fragment](#fragment)\n").unwrap();

    // Fragment validation requires `--fragments`, which the baseline
    // capture also used; run the CLI directly to mirror that flag.
    let output = md_cmd()
        .arg("validate")
        .arg("refs")
        .arg(root.join("fragment.md"))
        .arg("--fragments")
        .arg("--format")
        .arg("json")
        .output()
        .expect("md command failed to spawn");

    // The CLI always prints the JSON report to stdout even when the
    // subsequent validation summary errors out and sets a non-zero exit.
    let stdout = String::from_utf8_lossy(&output.stdout);
    let actual: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or_else(|e| {
            panic!(
                "md validate refs --json --fragments did not produce valid JSON: {e}\n\
                 status: {:?}\nstderr: {}\nstdout: {stdout}",
                output.status.code(),
                String::from_utf8_lossy(&output.stderr),
            )
        });
    let redact = baseline::paths_to_redact(root);
    let redact_refs: Vec<&str> = redact.iter().map(|s| s.as_str()).collect();
    let actual_norm = baseline::normalize(actual, &redact_refs);
    let expected_norm = baseline::normalize(
        baseline::load_json("validate_refs_fragment.json"),
        &redact_refs,
    );
    assert_eq!(
        actual_norm, expected_norm,
        "md validate refs --json --fragments output did not match baseline\n\
         raw output:\n{stdout}",
    );
}

#[test]
fn validate_refs_json_datauri_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("datauri.md"),
        "# Data\n\n![data](data:image/png;base64,abc)\n",
    )
    .unwrap();
    assert_json_matches_baseline(&root.join("datauri.md"), "validate_refs_datauri.json");
}

#[test]
fn validate_refs_json_inline_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(
        root.join("inline.md"),
        "# Inline\n\n<style>.x{color:red}</style>\n\n<script src=\"app.js\"></script>\n",
    )
    .unwrap();
    // The baseline capture treats the `<script src>` as a valid local
    // reference because `app.js` exists next to the document.
    std::fs::write(root.join("app.js"), "// inline script target\n").unwrap();
    assert_json_matches_baseline(&root.join("inline.md"), "validate_refs_inline.json");
}

#[test]
fn validate_refs_json_errors_baseline() {
    let dir = tempfile::TempDir::new().unwrap();
    let root = dir.path();
    std::fs::write(root.join("errors.md"), "# Errors\n\n[missing](./missing.md)\n").unwrap();
    // Note: no `missing.md` is written — the test asserts the missing
    // target produces exactly the baseline error shape.
    assert_json_matches_baseline(&root.join("errors.md"), "validate_refs_errors.json");
}
