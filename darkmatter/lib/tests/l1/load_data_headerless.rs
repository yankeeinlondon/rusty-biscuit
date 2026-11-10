//! `load_data(..., {header: false})`: every CSV or TSV record is a row by
//! position, and a failure is located at the raw offset of the record and
//! cell it names (`/0/1` is the second cell of the first input record).
//!
//! Every fixture lives in its own temporary directory and every request is
//! anchored there. Positions are asserted as line and character column, so a
//! row view shifted by one record fails.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::{DataError, DataSourceLocation};
use darkmatter::markdown::compose::expression::{Diagnostic, ExpressionError};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeFailure, ComposeOptions, ComposeReport, DataRuntime};
use serde_json::{Value, json};
use tempfile::TempDir;

const MISMATCH: &str = "dm.data.contract_mismatch";
const MALFORMED: &str = "dm.data.malformed";
const PAIRS: &str = "tuple([string, string])[]";
const NAMED_PORTS: &str = "tuple([string, number])[]";

// ── Fixtures ────────────────────────────────────────────────────────────────

fn write(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write fixture");
    path
}

type Composed = Result<(Markdown, ComposeReport), Box<ComposeFailure>>;

/// Composes `content` as `doc.md` in `dir`, sharing `runtime`.
fn compose_with(dir: &Path, content: &str, runtime: &DataRuntime) -> Composed {
    let path = write(dir, "doc.md", content);
    let options = ComposeOptions::new().with_data_runtime(runtime.clone()).with_source_file(&path);
    let request = crate::request_support::request(options);
    Markdown::try_from(path.as_path()).expect("document loads").compose_with_warnings(&request)
}

fn rendered_with(dir: &Path, content: &str, runtime: &DataRuntime) -> String {
    match compose_with(dir, content, runtime) {
        Ok((composed, _)) => composed.content().trim().to_string(),
        Err(failure) => panic!("{content:?} failed: {}", failure.error),
    }
}

/// The expression `load_data("<file>", "<contract>", {header: false})`.
fn headerless_call(file: &str, contract: &str) -> String {
    format!("load_data(\"{file}\", \"{contract}\", {{header: false}})")
}

/// The JSON a document renders for `[call]`; `as_json` renders only a list.
fn value_of(dir: &Path, call: &str, runtime: &DataRuntime) -> Value {
    let text = rendered_with(dir, &format!("{{{{ as_json([{call}]) }}}}\n"), runtime);
    let wrapped: Value = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text:?} is not JSON: {error}"));
    wrapped[0].clone()
}

/// The value `load_data(file, contract, {header: false})` returns over `data`.
fn headerless(dir: &Path, file: &str, data: impl AsRef<[u8]>, contract: &str) -> Value {
    write(dir, file, data);
    value_of(dir, &headerless_call(file, contract), &DataRuntime::new())
}

fn failure_of(dir: &Path, file: &str, data: impl AsRef<[u8]>, contract: &str) -> Box<ComposeFailure> {
    write(dir, file, data);
    let content = format!("x {{{{ {} }}}}\n", headerless_call(file, contract));
    match compose_with(dir, &content, &DataRuntime::new()) {
        Ok((composed, _)) => panic!("{content:?} composed: {}", composed.content()),
        Err(failure) => failure,
    }
}

/// The typed data error in `failure`'s cause chain.
fn data_error(failure: &ComposeFailure) -> Arc<DataError> {
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&failure.error);
    while let Some(error) = current {
        let expression = error
            .downcast_ref::<ExpressionError>()
            .or_else(|| error.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref));
        if let Some(ExpressionError::Data(data)) = expression.map(ExpressionError::without_callbacks) {
            return data.clone();
        }
        current = error.source();
    }
    panic!("no data error in: {}", failure.error)
}

/// The `(line, column)` of a headerless read that fails as `dm.data.malformed`.
fn malformed_at(dir: &Path, file: &str, data: impl AsRef<[u8]>, contract: &str) -> (Option<usize>, Option<usize>) {
    let error = data_error(&failure_of(dir, file, data, contract));
    assert_eq!(error.code(), Some(MALFORMED), "{error}");
    let location: &DataSourceLocation = match &*error {
        DataError::Malformed { location, .. } => location,
        other => panic!("expected malformed, got {other}"),
    };
    (location.line, location.column)
}

/// The shared diagnostic records of a failed headerless read.
fn records(dir: &Path, file: &str, data: impl AsRef<[u8]>, contract: &str) -> Vec<Diagnostic> {
    error_diagnostics(&failure_of(dir, file, data, contract).error)
}

/// Each contract record's `(pointer, line, column)`, after checking its code.
fn places(records: &[Diagnostic]) -> Vec<(String, Option<usize>, Option<usize>)> {
    records
        .iter()
        .map(|record| {
            assert_eq!(record.code, MISMATCH, "{record:#?}");
            let location = record.data_location.as_ref().unwrap_or_else(|| panic!("no data location: {record:?}"));
            (location.pointer.clone().expect("a contract record has a pointer"), location.line, location.column)
        })
        .collect()
}

fn place(pointer: &str, line: usize, column: usize) -> (String, Option<usize>, Option<usize>) {
    (pointer.to_string(), Some(line), Some(column))
}

// ── Successful rows ─────────────────────────────────────────────────────────

#[test]
fn headerless_csv_returns_both_records() {
    let dir = TempDir::new().unwrap();
    let rows = headerless(dir.path(), "rows.csv", "web,8080\ndb,5432\n", NAMED_PORTS);
    assert_eq!(rows, json!([["web", 8080], ["db", 5432]]));
}

#[test]
fn headerless_tsv_returns_both_records() {
    let dir = TempDir::new().unwrap();
    let rows = headerless(dir.path(), "rows.tsv", "web\t8080\ndb\t5432\n", NAMED_PORTS);
    assert_eq!(rows, json!([["web", 8080], ["db", 5432]]));
}

#[test]
fn headerless_single_record_is_one_row() {
    let dir = TempDir::new().unwrap();
    assert_eq!(headerless(dir.path(), "one.csv", "web,8080\n", NAMED_PORTS), json!([["web", 8080]]));
    // No trailing newline reads the same record.
    assert_eq!(headerless(dir.path(), "bare.csv", "web,8080", NAMED_PORTS), json!([["web", 8080]]));
}

#[test]
fn headerless_empty_and_blank_only_inputs_are_empty() {
    let dir = TempDir::new().unwrap();
    for (file, data) in [("empty.csv", ""), ("blank.csv", "\n\r\n\n"), ("empty.tsv", ""), ("blank.tsv", "\n\n")] {
        assert_eq!(headerless(dir.path(), file, data, PAIRS), json!([]), "{file}: {data:?}");
    }
    // `[]` is still judged against `as`.
    let error = data_error(&failure_of(dir.path(), "empty.csv", "", "tuple([string, string])[](min(1))"));
    assert_eq!(error.code(), Some(MISMATCH), "{error}");
}

#[test]
fn headerless_one_record_keeps_empty_and_repeated_cells() {
    let dir = TempDir::new().unwrap();
    let contract = "tuple([string, string, string, string])[]";
    let rows = headerless(dir.path(), "cells.csv", "a,,a,\n", contract);
    assert_eq!(rows, json!([["a", "", "a", ""]]));
}

#[test]
fn headerless_blank_lines_are_skipped_with_original_positions() {
    let dir = TempDir::new().unwrap();
    let data = "\nweb,8080\n\n\ndb,x\n";
    assert_eq!(
        headerless(dir.path(), "ok.csv", "\nweb,8080\n\n\ndb,5432\n", NAMED_PORTS),
        json!([["web", 8080], ["db", 5432]])
    );
    let found = records(dir.path(), "blank.csv", data, NAMED_PORTS);
    assert_eq!(places(&found), vec![place("/1/1", 5, 4)], "{found:#?}");
}

#[test]
fn header_true_skips_the_first_record_for_positional_rows() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "web,8080\ndb,5432\n");
    let runtime = DataRuntime::new();
    let explicit = value_of(dir.path(), &format!("load_data(\"rows.csv\", \"{NAMED_PORTS}\", {{header: true}})"), &runtime);
    let default = value_of(dir.path(), &format!("load_data(\"rows.csv\", \"{NAMED_PORTS}\")"), &runtime);
    assert_eq!((explicit, default), (json!([["db", 5432]]), json!([["db", 5432]])));
}

// ── Strict parsing ──────────────────────────────────────────────────────────

#[test]
fn headerless_ragged_record_is_malformed_at_that_record() {
    let dir = TempDir::new().unwrap();
    // The first record sets the width; the short and the long record each
    // fail at their own line.
    assert_eq!(malformed_at(dir.path(), "short.csv", "a,b\n1,2\n3\n", PAIRS), (Some(3), Some(1)));
    assert_eq!(malformed_at(dir.path(), "long.csv", "a,b\n1,2,3\n", PAIRS), (Some(2), Some(1)));
    assert_eq!(malformed_at(dir.path(), "short.tsv", "a\tb\n\n3\n", PAIRS), (Some(3), Some(1)));
}

#[test]
fn headerless_quoted_delimiter_newline_and_doubled_quote_parse_strictly() {
    let dir = TempDir::new().unwrap();
    let rows = headerless(dir.path(), "q.csv", "\"a,b\",\"x\ny\"\n\"say \"\"hi\"\"\",z\n", PAIRS);
    assert_eq!(rows, json!([["a,b", "x\ny"], ["say \"hi\"", "z"]]));
    let tabs = headerless(dir.path(), "q.tsv", "\"a\tb\"\tc\n", PAIRS);
    assert_eq!(tabs, json!([["a\tb", "c"]]));
}

#[test]
fn headerless_invalid_quoting_is_malformed_without_a_result() {
    let dir = TempDir::new().unwrap();
    assert_eq!(malformed_at(dir.path(), "open.csv", "a,b\n\"x,y\n", PAIRS), (Some(2), Some(1)));
    assert_eq!(malformed_at(dir.path(), "inner.csv", "a,b\nx\"y,z\n", PAIRS), (Some(2), Some(2)));
    assert_eq!(malformed_at(dir.path(), "after.csv", "\"x\"y,z\n", PAIRS), (Some(1), Some(4)));
    // No partial text is composed: the document fails as a whole.
    let failure = failure_of(dir.path(), "open.csv", "a,b\n\"x,y\n", PAIRS);
    assert_eq!(data_error(&failure).code(), Some(MALFORMED));
}

// ── Located contract failures ───────────────────────────────────────────────

#[test]
fn headerless_first_record_failure_reports_pointer_0_1() {
    let dir = TempDir::new().unwrap();
    let found = records(dir.path(), "rows.csv", "web,x\ndb,5432\n", NAMED_PORTS);
    assert_eq!(places(&found), vec![place("/0/1", 1, 5)], "{found:#?}");
    let record = &found[0];
    assert_eq!((record.expected_type.as_deref(), record.actual_type.as_deref()), (Some("number"), Some("string")));
}

#[test]
fn headerless_later_row_failure_is_located_at_its_own_record() {
    let dir = TempDir::new().unwrap();
    let found = records(dir.path(), "rows.csv", "web,8080\ndb,5432\ncache,x\n", NAMED_PORTS);
    assert_eq!(places(&found), vec![place("/2/1", 3, 7)], "{found:#?}");
}

#[test]
fn headerless_later_row_after_a_quoted_multiline_field_is_located() {
    let dir = TempDir::new().unwrap();
    let contract = "tuple([string, string, number])[]";
    // The second record's first field spans lines 2 and 3; its failing cell
    // starts after the closing quote on line 3.
    let found = records(dir.path(), "rows.csv", "a,b,1\n\"x\ny\",z,q\n", contract);
    assert_eq!(places(&found), vec![place("/1/2", 3, 6)], "{found:#?}");
}

#[test]
fn headerless_multibyte_text_before_the_failing_cell_counts_characters() {
    let dir = TempDir::new().unwrap();
    let contract = "tuple([string, string, number])[]";
    let found = records(dir.path(), "rows.csv", "a,b,1\né,日本,x\n", contract);
    assert_eq!(places(&found), vec![place("/1/2", 2, 6)], "{found:#?}");
}

#[test]
fn headerless_crlf_positions_match_lf_positions() {
    let dir = TempDir::new().unwrap();
    let contract = "tuple([string, string, number])[]";
    let lf = records(dir.path(), "lf.csv", "a,b,1\n\"x\ny\",日本,q\n".as_bytes(), contract);
    let crlf = records(dir.path(), "crlf.csv", b"a,b,1\r\n\"x\r\ny\",\xe6\x97\xa5\xe6\x9c\xac,q\r\n", contract);
    assert_eq!(places(&lf), vec![place("/1/2", 3, 7)], "{lf:#?}");
    assert_eq!(places(&crlf), places(&lf), "{crlf:#?}");
}

#[test]
fn headerless_bom_is_not_counted_in_the_first_record() {
    let dir = TempDir::new().unwrap();
    let found = records(dir.path(), "bom.csv", "\u{feff}web,x\n", NAMED_PORTS);
    assert_eq!(places(&found), vec![place("/0/1", 1, 5)], "{found:#?}");
}

#[test]
fn headerless_missing_cell_uses_nearest_parent() {
    let dir = TempDir::new().unwrap();
    // A one-cell file is one column wide, so the second tuple slot is missing
    // in every row and is located at the start of that row's record.
    let found = records(dir.path(), "rows.csv", "web\ndb\n", NAMED_PORTS);
    assert_eq!(places(&found), vec![place("/0/1", 1, 1), place("/1/1", 2, 1)], "{found:#?}");
    assert!(found.iter().all(|record| record.actual_type.as_deref() == Some("missing")), "{found:#?}");
}

// ── Reuse across header choices ─────────────────────────────────────────────

#[test]
fn header_false_then_true_share_one_parse() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "web,8080\ndb,5432\n");
    let without = format!("load_data(\"rows.csv\", \"{NAMED_PORTS}\", {{header: false}})");
    let with = format!("load_data(\"rows.csv\", \"{NAMED_PORTS}\", {{header: true}})");
    let headerless_rows = json!([["web", 8080], ["db", 5432]]);
    let headed_rows = json!([["db", 5432]]);
    for (first, second, first_rows, second_rows) in [
        (&without, &with, &headerless_rows, &headed_rows),
        (&with, &without, &headed_rows, &headerless_rows),
    ] {
        let runtime = DataRuntime::new();
        let both = value_of(dir.path(), &format!("[{first}, {second}]"), &runtime);
        assert_eq!(both, json!([first_rows, second_rows]), "{first} then {second}");
        let stats = runtime.stats();
        assert_eq!((stats.reads, stats.parses, stats.binds), (1, 1, 2), "{first} then {second}: {stats:?}");
    }
}
