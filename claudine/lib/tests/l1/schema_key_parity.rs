//! Claudine's half of the schema-key parity fixture
//! (`tests/fixtures/schema_keys/`): its embedded bundle judges the shared
//! sequence document with exactly the findings `expected.json` lists, which
//! DMLS's registered-host test publishes from the same shipped revision
//! (`darkmatter/dmls/tests/l1/claudine_schema_key_parity.rs`).
//!
//! The document covers a root, group, leg, and step removal (one with a
//! `null` value), a closed group's misspelled key, near-miss advice on an
//! open step, unrelated step data (`owner`), the step's `agent` (never
//! advised), and a legal `group.fail_fast`. Lines and columns are one-based,
//! columns in UTF-8 bytes; the leg and step lines hold non-ASCII text before
//! the key.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::DarkmatterSchemas;
use serde_json::{Value, json};

const DOCUMENT: &str = include_str!("../fixtures/schema_keys/sequence.md");
const EXPECTED: &str = include_str!("../fixtures/schema_keys/expected.json");

/// Each finding of `text` as `{code, severity, message, hint, instance_path,
/// line, column}`, problems then advice.
fn findings(text: &str) -> Vec<Value> {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().join("sequence.md");
    std::fs::write(&path, text).unwrap();
    let markdown = Markdown::try_from(path.as_path()).unwrap();
    let schemas = claudine::composition::schema::bundle::with_claudine_registration(DarkmatterSchemas::new(
        FileResolutionContext::new(dir.path()),
    ));
    let report = schemas.validate(&markdown).unwrap();
    let record = |problem: &simplified_schema::ValidationProblem, severity: &str| {
        json!({
            "code": problem.code.diagnostic_code(),
            "severity": severity,
            "message": problem.message,
            "hint": problem.hint,
            "instance_path": problem.path,
            "line": problem.line,
            "column": problem.column,
        })
    };
    report
        .problems
        .iter()
        .map(|problem| record(problem, "error"))
        .chain(report.key_advice.iter().map(|advice| record(advice, "warning")))
        .collect()
}

#[test]
fn the_embedded_bundle_reports_the_parity_findings_for_lf_and_crlf() {
    let expected: Vec<Value> = serde_json::from_str(EXPECTED).unwrap();
    assert_eq!(findings(DOCUMENT), expected, "LF");
    assert_eq!(findings(&DOCUMENT.replace('\n', "\r\n")), expected, "CRLF");
}
