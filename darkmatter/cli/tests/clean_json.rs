//! Level-1 CLI coverage for `md clean --json` (acceptance row D-5).
//!
//! The envelope is a machine contract, so these tests pin field names, value
//! shapes, enum spellings, and the STDOUT/STDERR channel split rather than
//! asserting that output merely "looks right".
//!
//! See `darkmatter/features/2026-07-14-invalid-frontmatter/spec.md`.

mod common;

use common::md_cmd;
use serde_json::Value;
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
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    serde_json::from_str(&stdout)
        .unwrap_or_else(|e| panic!("stdout was not a JSON envelope ({e}):\n{stdout}"))
}

/// The envelope's four documented top-level fields, and nothing else.
#[test]
fn test_json_envelope_has_exactly_the_documented_top_level_fields() {
    let (_dir, path) = doc("---\ntitle: @daily-report\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    let object = report.as_object().unwrap();
    let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        ["diagnostics", "frontmatter_offset", "path", "repaired"],
        "envelope field set is a wire contract"
    );

    assert_eq!(report["path"], Value::String(path.display().to_string()));
    assert_eq!(report["repaired"], Value::Bool(true));
    assert!(report["frontmatter_offset"].is_number());
    assert!(report["diagnostics"].is_array());
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
        ["classification", "code", "message", "repairs", "span", "stage"],
        "diagnostic field set is a wire contract"
    );

    assert_eq!(diagnostic["stage"], Value::String("syntax".into()));
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

/// `frontmatter_offset` is documented as the projection term that turns a
/// `syntax` span into document coordinates. Prove it actually does.
#[test]
fn test_json_frontmatter_offset_projects_spans_into_document_coordinates() {
    let source = "---\ntitle: @daily-report\n---\n\n# Body\n";
    let (_dir, path) = doc(source);
    let report = envelope(&path, &[]);

    let offset = report["frontmatter_offset"].as_u64().unwrap() as usize;
    let diagnostic = &report["diagnostics"][0];
    let start = diagnostic["span"]["start"].as_u64().unwrap() as usize;
    let end = diagnostic["span"]["end"].as_u64().unwrap() as usize;

    assert_eq!(
        &source[offset + start..offset + end],
        "@daily-report",
        "offset + syntax span must index the authored lexeme"
    );
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
    assert_eq!(report["repaired"], Value::Bool(false));
}

/// Both tiers appear in one envelope, discriminated by `stage`.
#[test]
fn test_json_stage_discriminates_syntax_from_schema_diagnostics() {
    // `ctx` is a Darkmatter baseline key, so the undeclared child raises a
    // schema-tier finding; the empty value raises a syntax-tier one.
    let (_dir, path) = doc("---\nempty:\nctx:\n  nope: 1\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    let diagnostics = report["diagnostics"].as_array().unwrap();
    let stages: Vec<&str> = diagnostics
        .iter()
        .map(|d| d["stage"].as_str().unwrap())
        .collect();

    assert!(stages.contains(&"syntax"), "expected a syntax finding: {stages:?}");
    assert!(stages.contains(&"schema"), "expected a schema finding: {stages:?}");

    for diagnostic in diagnostics {
        assert!(
            matches!(diagnostic["stage"].as_str(), Some("syntax" | "schema")),
            "stage is a closed two-variant enum, got {}",
            diagnostic["stage"]
        );
    }
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
        .map(|d| format!("{}:{}", d["stage"], d["code"]))
        .collect();

    assert!(first.len() > 1, "expected several diagnostics, got {first:?}");

    let (_dir2, path2) = doc(source);
    let second: Vec<String> = envelope(&path2, &[])["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| format!("{}:{}", d["stage"], d["code"]))
        .collect();

    assert_eq!(first, second, "diagnostic ordering must be stable");
}

/// D-5 + D-8: a document with no frontmatter still emits a well-formed
/// envelope, with the nulls that prove the analysis was bypassed.
#[test]
fn test_json_no_frontmatter_envelope_is_null_and_empty() {
    let (_dir, path) = doc("# Just A Body\n\nNo frontmatter here.\n");
    let report = envelope(&path, &[]);

    assert_eq!(report["frontmatter_offset"], Value::Null);
    assert_eq!(report["repaired"], Value::Bool(false));
    assert_eq!(report["diagnostics"], Value::Array(vec![]));
}

/// D-8: an *empty* frontmatter block is the sharper case — the block exists,
/// so a null `frontmatter_offset` can only mean the bypass fired before any
/// YAML analysis ran.
#[test]
fn test_json_empty_frontmatter_offset_is_null_proving_bypass() {
    let (_dir, path) = doc("---\n---\n\n# Body\n");
    let report = envelope(&path, &[]);

    assert_eq!(
        report["frontmatter_offset"],
        Value::Null,
        "an analyzed block would have reported its offset"
    );
    assert_eq!(report["diagnostics"], Value::Array(vec![]));
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
    assert_eq!(report["repaired"], Value::Bool(true));

    let saved = fs::read_to_string(&path).unwrap();
    assert!(
        saved.contains("title: \"@daily-report\""),
        "the file must be repaired on disk, got:\n{saved}"
    );
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

    assert!(!stdout.contains("changed"), "delta report leaked:\n{stdout}");
    let _: Value = serde_json::from_str(&stdout).unwrap();
}

/// Stdin input has no path, so `path` is null rather than absent or `"-"`.
#[test]
fn test_json_stdin_reports_null_path() {
    let assert = md_cmd()
        .args(["clean", "-", "--json"])
        .write_stdin("---\ntitle: @daily-report\n---\n\n# Body\n")
        .assert()
        .success();
    let stdout = String::from_utf8(assert.get_output().stdout.clone()).unwrap();
    let report: Value = serde_json::from_str(&stdout).unwrap();

    assert_eq!(report["path"], Value::Null);
    assert_eq!(report["repaired"], Value::Bool(true));
}

/// D-5 + D-7: unrepairable frontmatter fails before any envelope is printed,
/// so `--json` never emits a half-truthful success payload.
#[test]
fn test_json_unrepairable_frontmatter_exits_one_without_envelope() {
    let (_dir, path) = doc("---\ntitle: [unclosed\n---\n\n# Body\n");

    let assert = md_cmd()
        .arg("clean")
        .arg(&path)
        .arg("--json")
        .assert()
        .failure();

    assert!(
        assert.get_output().stdout.is_empty(),
        "no envelope on the failure path"
    );
}
