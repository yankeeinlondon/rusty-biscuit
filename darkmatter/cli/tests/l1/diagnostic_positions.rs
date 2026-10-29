//! Source positions in `md schema validate` (JSON and human output) and in
//! `md compose`'s schema-validation error block.
//!
//! The documents and their expected starts come from the shared
//! `position_fixtures` module, which the DMLS suite checks against its own
//! diagnostics; each test spells that file with `include_str!` so a fixture
//! edit runs it.

use crate::common;
use crate::position_fixtures::{self, ExpectedStart, PositionCase};

use common::CliProcessFixture;
use std::path::PathBuf;

fn write_document(process: &CliProcessFixture, name: &str, text: &str) -> PathBuf {
    let path = process.cwd().join(format!("{name}.md"));
    std::fs::write(&path, text).unwrap();
    path
}

fn validate_json(process: &CliProcessFixture, doc: &PathBuf, assignments: &[&str]) -> (i32, serde_json::Value) {
    let output = process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(doc)
        .args(assignments)
        .output()
        .unwrap();
    let report = serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|err| panic!("JSON report: {err}\n{}", String::from_utf8_lossy(&output.stdout)));
    (output.status.code().expect("exit code"), report)
}

fn problem<'a>(report: &'a serde_json::Value, path: &str, kind: &str) -> &'a serde_json::Value {
    report["problems"]
        .as_array()
        .expect("problems array")
        .iter()
        .find(|p| p["path"] == path && p["kind"] == kind)
        .unwrap_or_else(|| panic!("no {kind} problem at {path:?}: {report:#}"))
}

/// Validates `case` (with `\n` replaced by `line_end`) and asserts every
/// expected start, the problem count, and the failure exit code.
fn assert_case(case: &PositionCase, line_end: &str) {
    let process = CliProcessFixture::new();
    let doc = write_document(&process, case.name, &case.document.replace('\n', line_end));
    let (code, report) = validate_json(&process, &doc, &[]);

    assert_eq!(code, 1, "{report:#}");
    assert_eq!(report["valid"], false);
    assert_eq!(
        report["problems"].as_array().unwrap().len(),
        case.expected.len(),
        "{report:#}"
    );
    for &ExpectedStart { path, kind, line, column, .. } in case.expected {
        let found = problem(&report, path, kind);
        assert_eq!(
            (&found["line"], &found["column"]),
            (&serde_json::json!(line), &serde_json::json!(column)),
            "{} {path:?} with {line_end:?} line ends: {found:#}",
            case.name
        );
    }
}

#[test]
fn json_reports_the_acceptance_key_token() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    assert_case(&position_fixtures::ACCEPTANCE, "\n");
}

#[test]
fn json_reports_the_acceptance_key_token_with_crlf_line_ends() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    assert_case(&position_fixtures::ACCEPTANCE, "\r\n");
}

#[test]
fn json_reports_block_list_items_and_mappings_without_the_dash() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    assert_case(&position_fixtures::BLOCK_LIST, "\n");
}

#[test]
fn json_distinguishes_same_line_flow_keys_and_values() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    assert_case(&position_fixtures::FLOW, "\n");
}

#[test]
fn json_columns_count_utf8_bytes_after_a_non_ascii_prefix() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    assert_case(&position_fixtures::NON_ASCII_PREFIX, "\n");
}

#[test]
fn json_reports_the_root_mapping_for_a_root_missing_field() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    assert_case(&position_fixtures::ROOT_MISSING, "\n");
}

/// Guards the per-case tests above: a case added to `CASES` needs its own test.
#[test]
fn every_shared_case_has_a_json_test() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    let tested = ["acceptance", "block_list", "flow", "non_ascii_prefix", "root_missing"];
    let cases: Vec<_> = position_fixtures::CASES.iter().map(|case| case.name).collect();
    assert_eq!(cases, tested);
}

#[test]
fn human_output_shows_line_and_column() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let doc = write_document(&process, "acceptance", position_fixtures::ACCEPTANCE.document);

    let output = process
        .command()
        .args(["schema", "validate"])
        .arg(&doc)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert_eq!(output.status.code(), Some(1), "{stdout}");
    assert!(
        stdout.contains("'sya' was unexpected) (at line 5, column 9)"),
        "{stdout}"
    );
}

#[test]
fn a_valid_document_exits_zero_with_no_problems() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let text = position_fixtures::ACCEPTANCE.document.replace("sya:", "say:");
    let doc = write_document(&process, "valid", &text);

    let (code, report) = validate_json(&process, &doc, &[]);

    assert_eq!(code, 0, "{report:#}");
    assert_eq!(report["valid"], true);
    assert_eq!(report["problems"], serde_json::json!([]));
}

/// Assignments keep authored node starts, fall back to an authored ancestor
/// for new nested paths, and report `null` for new top-level paths.
#[test]
fn assigned_values_use_the_original_node_or_report_null() {
    let process = CliProcessFixture::new();
    let doc = write_document(
        &process,
        "assigned",
        "---\ncount: 1\nm:\n  a: 1\n$schema:\n  count: number\n  m: '{ a: number, b: number }'\n  extra: number\n---\n",
    );

    let (code, report) = validate_json(
        &process,
        &doc,
        &["count=abc", "m.a=abc", "m.b=abc", "extra=abc"],
    );

    assert_eq!(code, 1, "{report:#}");
    assert_eq!(report["problems"].as_array().unwrap().len(), 4, "{report:#}");
    let replaced = problem(&report, "/count", "type");
    assert_eq!((&replaced["line"], &replaced["column"]), (&serde_json::json!(2), &serde_json::json!(8)));
    let nested_replaced = problem(&report, "/m/a", "type");
    assert_eq!(
        (&nested_replaced["line"], &nested_replaced["column"]),
        (&serde_json::json!(4), &serde_json::json!(6)),
    );
    let nested_added = problem(&report, "/m/b", "type");
    assert_eq!(
        (&nested_added["line"], &nested_added["column"]),
        (&serde_json::json!(4), &serde_json::json!(3)),
    );
    let added = problem(&report, "/extra", "type");
    assert!(added["line"].is_null() && added["column"].is_null(), "{added:#}");
}

/// `md compose` renders the same start in its schema-validation error block.
#[test]
fn compose_error_block_shows_line_and_column() {
    let _ = include_str!("../../../lib/tests/position_fixtures/mod.rs");
    let process = CliProcessFixture::new();
    let doc = write_document(&process, "acceptance", position_fixtures::ACCEPTANCE.document);

    let output = process.command().arg("compose").arg(&doc).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The block wraps at its own width, so compare without whitespace.
    let compact: String = common::unwrapped(&stderr).split_whitespace().collect();

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(compact.contains("schemavalidationfailed"), "{stderr}");
    assert!(compact.contains("'sya'wasunexpected)at5:9"), "{stderr}");
}
