//! Level-1 CLI coverage for `md clean --json` (acceptance row D-5).
//!
//! The envelope is a machine contract, so these tests pin field names, value
//! shapes, enum spellings, and the STDOUT/STDERR channel split rather than
//! asserting that output merely "looks right".
//!
//! See `darkmatter/features/2026-07-14-invalid-frontmatter/spec.md`.

mod common;

use common::md_cmd;
use serde_json::{Value, json};
use std::fs;
use std::path::{Path, PathBuf};

fn doc(content: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("doc.md");
    fs::write(&path, content).unwrap();
    (dir, path)
}

/// Runs `md clean --json` and parses the envelope, asserting STDOUT is valid
/// JSON and nothing else.
fn envelope(path: &Path, extra: &[&str]) -> Value {
    let assert = md_cmd()
        .arg("clean")
        .arg(path)
        .arg("--json")
        .args(extra)
        .assert()
        .success();
    assert!(
        assert.get_output().stderr.is_empty(),
        "JSON mode must not emit human diagnostics on stderr"
    );
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout was not a JSON envelope ({e}):\n{stdout}"))
}

/// Golden contract for the flagship repair, including every v1 field.
#[test]
fn test_json_envelope_has_exactly_the_documented_top_level_fields() {
    let (_dir, path) = doc("---\ntitle: @daily-report\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    let object = report.as_object().unwrap();
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "applied",
            "changed",
            "diagnostics",
            "frontmatter",
            "source",
            "version"
        ],
        "envelope field set is a wire contract"
    );

    assert_eq!(
        report,
        json!({
            "version": 1,
            "source": {
                "kind": "file",
                "path": path,
            },
            "frontmatter": {
                "present": true,
                "span": { "start": 4, "end": 25 },
            },
            "diagnostics": [{
                "code": "yaml.reserved-indicator",
                "classification": "deterministic",
                "message": "plain scalar begins with the reserved YAML indicator `@` and does not parse",
                "span": {
                    "start": 11,
                    "end": 24,
                    "start_line": 2,
                    "start_column": 8,
                    "end_line": 2,
                    "end_column": 21,
                },
                "repairs": [{
                    "span": { "start": 11, "end": 24 },
                    "replacement": "\"@daily-report\"",
                    "explanation": "quote the scalar so the indicator is treated as string content",
                }],
            }],
            "applied": [{
                "span": { "start": 11, "end": 24 },
                "replacement": "\"@daily-report\"",
                "explanation": "quote the scalar so the indicator is treated as string content",
            }],
            "changed": true,
        })
    );
}

/// Each diagnostic carries the spec's shape: a stable code, a byte span, a
/// certainty classification, a message, and zero or more repairs.
#[test]
fn test_json_diagnostic_shape_is_fully_pinned() {
    let (_dir, path) = doc("---\ntitle: @daily-report\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 1, "one finding for the flagship input");

    let diagnostic = &diagnostics[0];
    let mut keys: Vec<&str> = diagnostic.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["classification", "code", "message", "repairs", "span"],
        "diagnostic field set is a wire contract"
    );

    assert_eq!(
        diagnostic["code"],
        Value::String("yaml.reserved-indicator".into())
    );
    assert_eq!(
        diagnostic["classification"],
        Value::String("deterministic".into())
    );
    assert!(diagnostic["message"].is_string());

    assert!(diagnostic["span"]["start"].is_u64());
    assert!(diagnostic["span"]["end"].is_u64());
    assert!(diagnostic["span"]["start_line"].is_u64());
    assert!(diagnostic["span"]["start_column"].is_u64());
    assert!(diagnostic["span"]["end_line"].is_u64());
    assert!(diagnostic["span"]["end_column"].is_u64());

    let repairs = diagnostic["repairs"].as_array().unwrap();
    assert_eq!(repairs.len(), 1);
    let repair = &repairs[0];
    let mut repair_keys: Vec<&str> =
        repair.as_object().unwrap().keys().map(String::as_str).collect();
    repair_keys.sort_unstable();
    assert_eq!(repair_keys, ["explanation", "replacement", "span"]);
    assert_eq!(
        repair["replacement"],
        Value::String("\"@daily-report\"".into())
    );
    assert!(repair["span"]["start"].is_u64());
    assert!(repair["span"]["end"].is_u64());
    assert!(repair["explanation"].is_string());
}

/// Diagnostic and repair spans directly index the whole authored document.
#[test]
fn test_json_spans_are_projected_into_document_coordinates() {
    let source = "---\ntitle: @daily-report\n---\n\n# Body\n";
    let (_dir, path) = doc(source);
    let report = envelope(&path, &[]);

    let diagnostic = &report["diagnostics"][0];
    let start = diagnostic["span"]["start"].as_u64().unwrap() as usize;
    let end = diagnostic["span"]["end"].as_u64().unwrap() as usize;

    assert_eq!(
        &source[start..end],
        "@daily-report",
        "the diagnostic span must index the authored lexeme"
    );
    assert_eq!(
        diagnostic["repairs"][0]["span"],
        json!({ "start": 11, "end": 24 })
    );
    assert_eq!(report["applied"][0]["span"], json!({ "start": 11, "end": 24 }));
}

/// A stream BOM participates in the applied audit with document coordinates.
#[test]
fn test_json_bom_repair_is_audited_in_document_coordinates() {
    let (_dir, path) = doc("\u{feff}---\ntitle: @daily-report\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    assert_eq!(
        report["frontmatter"],
        json!({ "present": true, "span": { "start": 7, "end": 28 } })
    );
    assert_eq!(report["diagnostics"][0]["span"]["start"], json!(14));
    assert_eq!(
        report["applied"],
        json!([
            {
                "span": { "start": 0, "end": 3 },
                "replacement": "",
                "explanation": "remove the UTF-8 byte-order mark at document start",
            },
            {
                "span": { "start": 14, "end": 27 },
                "replacement": "\"@daily-report\"",
                "explanation": "quote the scalar so the indicator is treated as string content",
            }
        ])
    );
}

/// Columns are 1-indexed byte columns, not Unicode scalar columns.
#[test]
fn test_json_span_columns_are_byte_indexed() {
    let (_dir, path) = doc("---\n\"\u{1f4a1}\": @daily-report\n---\n\n# Body\n");
    let report = envelope(&path, &[]);
    let diagnostic = &report["diagnostics"][0];

    assert_eq!(diagnostic["span"]["start_line"], json!(2));
    assert_eq!(diagnostic["span"]["start_column"], json!(9));
    assert_eq!(diagnostic["span"]["end_column"], json!(22));
}

/// A report-only finding serializes with its own classification and an empty
/// `repairs` array — not a null, and not an omitted field.
#[test]
fn test_json_report_only_diagnostic_has_empty_repairs_array() {
    let (_dir, path) = doc("---\ntitle:\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    let diagnostic = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["code"] == Value::String("yaml.suspicious-empty-value".into()))
        .expect("expected the empty-value finding");

    assert_eq!(
        diagnostic["classification"],
        Value::String("non_deterministic_find".into()),
        "certainty-tier spelling is a wire contract"
    );
    assert_eq!(diagnostic["repairs"], Value::Array(vec![]));
    assert_eq!(
        report["changed"],
        Value::Bool(true),
        "document cleanup may change independently of the report-only finding"
    );
    assert_eq!(report["applied"], Value::Array(vec![]));
}

/// Both tiers share one v1 diagnostic shape and retain their code ownership.
#[test]
fn test_json_combines_syntax_and_schema_diagnostics() {
    // `ctx` is a Darkmatter baseline key, so the undeclared child raises a
    // schema-tier finding; the empty value raises a syntax-tier one.
    let (_dir, path) = doc("---\nempty:\nctx:\n  nope: 1\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    let diagnostics = report["diagnostics"].as_array().unwrap();
    let codes: Vec<&str> = diagnostics
        .iter()
        .map(|d| d["code"].as_str().unwrap())
        .collect();

    assert!(codes.iter().any(|code| code.starts_with("yaml.")), "{codes:?}");
    assert!(codes.iter().any(|code| code.starts_with("schema.")), "{codes:?}");
}

/// Multi-diagnostic ordering is deterministic and repeatable.
#[test]
fn test_json_multi_diagnostic_ordering_is_deterministic() {
    let source = "---\ntitle: @daily-report\nempty:\ntags:  [a,  b]\n---\n\n# Body\n";
    let (_dir, path) = doc(source);

    let first: Vec<String> = envelope(&path, &[])["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap().to_string())
        .collect();

    assert!(first.len() > 1, "expected several diagnostics, got {first:?}");

    let (_dir2, path2) = doc(source);
    let second: Vec<String> = envelope(&path2, &[])["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["code"].as_str().unwrap().to_string())
        .collect();

    assert_eq!(first, second, "diagnostic ordering must be stable");
}

/// D-5 + D-8: a document with no frontmatter still emits a well-formed
/// envelope whose structured source/frontmatter fields prove the bypass.
#[test]
fn test_json_no_frontmatter_envelope_is_null_and_empty() {
    let (_dir, path) = doc("# Just A Body\n\nNo frontmatter here.\n");
    let report = envelope(&path, &[]);

    assert_eq!(report["version"], json!(1));
    assert_eq!(report["source"]["kind"], json!("file"));
    assert_eq!(report["source"]["path"], json!(path));
    assert_eq!(report["frontmatter"], json!({ "present": false }));
    assert_eq!(report["diagnostics"], Value::Array(vec![]));
    assert_eq!(report["applied"], Value::Array(vec![]));
    assert_eq!(report["changed"], Value::Bool(false));
}

/// D-8: an empty frontmatter block remains present even though analysis is bypassed.
#[test]
fn test_json_empty_frontmatter_is_present_with_empty_span() {
    let (_dir, path) = doc("---\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    assert_eq!(
        report["frontmatter"],
        json!({ "present": true, "span": { "start": 4, "end": 4 } })
    );
    assert_eq!(report["diagnostics"], Value::Array(vec![]));
    assert_eq!(report["applied"], Value::Array(vec![]));
}

/// The envelope is the *sole* stdout payload: no document, no delta report,
/// and no human suggestion rendering.
#[test]
fn test_json_suppresses_document_and_human_rendering_on_stdout() {
    let (_dir, path) = doc("---\ntitle: @daily-report\nempty:\n---\n\n# Unique Heading\n");

    let assert = md_cmd()
        .arg("clean")
        .arg(&path)
        .arg("--json")
        .assert()
        .success();
    let output = assert.get_output();
    let stdout = String::from_utf8(output.stdout.clone()).unwrap();
    let stderr = String::from_utf8(output.stderr.clone()).unwrap();

    assert!(!stdout.contains("# Unique Heading"), "document leaked:\n{stdout}");
    assert!(
        !stdout.contains("frontmatter suggestions"),
        "human suggestion rendering leaked:\n{stdout}"
    );
    assert!(
        stderr.is_empty(),
        "JSON mode suppresses the stderr renderer, got:\n{stderr}"
    );

    // Whole-stdout parse is the real proof there is no trailing prose.
    let _: Value = serde_json::from_str(&stdout).unwrap();
}

/// `--save --json` performs the write *and* prints the envelope.
#[test]
fn test_json_with_save_writes_file_and_prints_envelope() {
    let (_dir, path) = doc("---\ntitle: @daily-report\n---\n\n# Body\n");

    let report = envelope(&path, &["--save"]);
    assert_eq!(report["changed"], Value::Bool(true));
    assert_eq!(report["applied"].as_array().unwrap().len(), 1);

    let saved = fs::read_to_string(&path).unwrap();
    assert!(
        saved.contains("title: \"@daily-report\""),
        "the file must be repaired on disk, got:\n{saved}"
    );
}

/// `changed` covers the whole document, not only frontmatter repairs.
#[test]
fn test_json_changed_reports_body_only_cleanup() {
    let (_dir, path) = doc("---\ntitle: Fine\n---\n\n# Body  \n");
    let report = envelope(&path, &[]);

    assert_eq!(report["applied"], Value::Array(vec![]));
    assert_eq!(report["changed"], Value::Bool(true));
}

/// `--save --json` prints the envelope instead of the delta report.
#[test]
fn test_json_with_save_suppresses_delta_report() {
    let (_dir, path) = doc("---\ntitle: Fine\n---\n\n# Body  \n");

    let assert = md_cmd()
        .arg("clean")
        .arg(&path)
        .args(["--save", "--json"])
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();

    assert!(!stdout.contains("Frontmatter:"), "delta report leaked:\n{stdout}");
    let report: Value = serde_json::from_str(&stdout).unwrap();
    assert_eq!(report["changed"], Value::Bool(true));
}

/// Stdin input has a structured source with a null path.
#[test]
fn test_json_stdin_reports_null_path() {
    let assert = md_cmd()
        .args(["clean", "-", "--json"])
        .write_stdin("---\ntitle: @daily-report\n---\n\n# Body\n")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let report: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(report["source"], json!({ "kind": "stdin", "path": null }));
    assert_eq!(report["changed"], Value::Bool(true));
}

/// Unrepairable YAML still returns the v1 envelope as the sole machine payload.
#[test]
fn test_json_unrepairable_frontmatter_exits_one_with_envelope() {
    let (_dir, path) = doc("---\ntitle: [unclosed\n---\n\n# Body\n");

    let assert = md_cmd()
        .arg("clean")
        .arg(&path)
        .arg("--json")
        .assert()
        .failure();

    let output = assert.get_output();
    assert!(output.stderr.is_empty(), "human error leaked to stderr");
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["version"], json!(1));
    assert_eq!(report["source"], json!({ "kind": "file", "path": path }));
    assert_eq!(report["frontmatter"], json!({
        "present": true,
        "span": { "start": 4, "end": 21 },
    }));
    assert_eq!(report["applied"], Value::Array(vec![]));
    assert_eq!(report["changed"], Value::Bool(false));
    let diagnostics = report["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics.iter().any(|diagnostic| diagnostic["code"] == "yaml.parse"),
        "parse failure missing from envelope: {report}"
    );
    for diagnostic in diagnostics {
        let keys: std::collections::BTreeSet<&str> = diagnostic
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            ["classification", "code", "message", "repairs", "span"]
                .into_iter()
                .collect()
        );
    }
}
