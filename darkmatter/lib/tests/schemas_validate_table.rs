//! Table-driven validation tests for the schemas subsystem.
//!
//! Each subdirectory of `tests/fixtures/validate/` is a self-contained case
//! with at least:
//!
//! - `doc.md` — the Markdown document with frontmatter under test
//! - `expected.json` — either `{ "valid": true, "problem_count": N }`,
//!   `{ "valid": false, "problem_count_at_least": N }`, or
//!   `{ "valid": false, "problem_count": N }`
//!
//! Optional inputs include a sibling `schema.yaml` / `schema.json` referenced
//! from `doc.md`'s `$schema` field.

use std::{fs, path::Path};

use darkmatter::markdown::{Markdown, compose::ComposeSource, schemas::DarkmatterSchemas};
use serde_json::Value;

fn fixtures_root() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/validate")
}

fn list_cases() -> Vec<std::path::PathBuf> {
    let mut cases: Vec<_> = fs::read_dir(fixtures_root())
        .expect("read fixtures directory")
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.path())
        .collect();
    cases.sort();
    cases
}

fn load_case(dir: &Path) -> (Markdown, Value) {
    let doc_path = dir.join("doc.md");
    let content = fs::read_to_string(&doc_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", doc_path.display()));
    let md: Markdown = content.as_str().into();
    let md = md.with_source(ComposeSource::File(doc_path));

    let expected_path = dir.join("expected.json");
    let expected_raw = fs::read_to_string(&expected_path)
        .unwrap_or_else(|err| panic!("read {}: {err}", expected_path.display()));
    let expected: Value = serde_json::from_str(&expected_raw)
        .unwrap_or_else(|err| panic!("parse {}: {err}", expected_path.display()));
    (md, expected)
}

#[test]
fn validate_table() {
    let api = DarkmatterSchemas::new();
    let cases = list_cases();
    assert!(!cases.is_empty(), "no validate fixtures found");

    for dir in cases {
        let case_name = dir.file_name().unwrap().to_string_lossy().into_owned();
        let (md, expected) = load_case(&dir);
        let report = api
            .validate(&md)
            .unwrap_or_else(|err| panic!("[{case_name}] validate failed: {err}"));

        let expected_valid = expected["valid"]
            .as_bool()
            .expect("expected.json must declare `valid`");
        assert_eq!(
            report.valid, expected_valid,
            "[{case_name}] valid mismatch: problems={:?}",
            report.problems
        );

        if let Some(exact) = expected.get("problem_count").and_then(Value::as_u64) {
            assert_eq!(
                report.problems.len() as u64,
                exact,
                "[{case_name}] problem count mismatch: got {:?}",
                report.problems
            );
        }
        if let Some(at_least) = expected
            .get("problem_count_at_least")
            .and_then(Value::as_u64)
        {
            assert!(
                (report.problems.len() as u64) >= at_least,
                "[{case_name}] expected at least {at_least} problems, got {:?}",
                report.problems
            );
        }
    }
}
