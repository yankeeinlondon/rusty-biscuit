//! `load_data` options at execution: an explicit `format`, a CSV
//! `delimiter`, the TSV tab rule, `null` options and fields, option failures
//! that read nothing, and memo reuse across interpretations of one source
//! (`DataRuntime::stats()` counts).
//!
//! Every fixture lives in its own temporary directory and every request is
//! anchored there. A literal options object is judged by the passive call
//! check before anything runs, so a test about what *execution* decides
//! passes its options through a frontmatter value, which the passive check
//! cannot see.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::DataFormat;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::DataError;
use darkmatter::markdown::compose::expression::{ExpressionError, FileRefFailure};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{CacheAccessMode, ComposeFailure, ComposeOptions, ComposeReport, DataRuntime};
use serde_json::{Value, json};
use tempfile::TempDir;

const INVALID_OPTIONS: &str = "dm.data.invalid_options";
const INCOMPATIBLE_CALL: &str = "dm.expression.incompatible_call";
const EVALUATION_FAILURE: &str = "dm.expression.evaluation_failure";
const ROW_SHAPE_CONFLICT: &str = "dm.data.row_shape_conflict";

// ── Fixtures ────────────────────────────────────────────────────────────────

fn write(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// The read-set spelling of `name` in `dir`.
fn entry(dir: &Path, name: &str) -> String {
    let canonical = biscuit_file::canonicalize_simplified(dir).expect("canonical dir");
    biscuit_file::to_portable_string(&canonical.join(name))
}

type Composed = Result<(Markdown, ComposeReport), Box<ComposeFailure>>;

fn compose_file(path: &Path, options: ComposeOptions) -> Composed {
    let request = crate::request_support::request(options.with_source_file(path));
    Markdown::try_from(path).expect("document loads").compose_with_warnings(&request)
}

/// Composes `content` as `name` in `dir`, sharing `runtime`.
fn compose_named(dir: &Path, name: &str, content: &str, runtime: &DataRuntime) -> Composed {
    let path = write(dir, name, content);
    compose_file(&path, ComposeOptions::new().with_data_runtime(runtime.clone()))
}

/// A document that renders `call`'s value as JSON (wrapped in a list, which
/// is all `as_json` renders), under `frontmatter` when it is not empty.
fn document(frontmatter: &str, call: &str) -> String {
    let body = format!("{{{{ as_json([{call}]) }}}}\n");
    if frontmatter.is_empty() { body } else { format!("---\n{frontmatter}\n---\n{body}") }
}

fn attempt(dir: &Path, runtime: &DataRuntime, frontmatter: &str, call: &str) -> Result<Value, Box<ComposeFailure>> {
    let (composed, _) = compose_named(dir, "doc.md", &document(frontmatter, call), runtime)?;
    let text = composed.content().trim().to_string();
    let wrapped: Value = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text:?} is not JSON: {error}"));
    Ok(wrapped[0].clone())
}

/// The value `call` returns.
fn loaded(dir: &Path, runtime: &DataRuntime, frontmatter: &str, call: &str) -> Value {
    attempt(dir, runtime, frontmatter, call).unwrap_or_else(|failure| panic!("{call} failed: {}", failure.error))
}

/// The failure of `call`.
fn failed(dir: &Path, runtime: &DataRuntime, frontmatter: &str, call: &str) -> Box<ComposeFailure> {
    match attempt(dir, runtime, frontmatter, call) {
        Ok(value) => panic!("{call} under {frontmatter:?} returned {value}"),
        Err(failure) => failure,
    }
}

fn codes(failure: &ComposeFailure) -> Vec<String> {
    error_diagnostics(&failure.error).into_iter().map(|record| record.code).collect()
}

/// Every expression error in `failure`'s cause chain, callbacks unwrapped.
fn expression_errors(failure: &ComposeFailure) -> Vec<&ExpressionError> {
    let mut found = Vec::new();
    let mut current: Option<&(dyn std::error::Error + 'static)> = Some(&failure.error);
    while let Some(error) = current {
        let expression = error
            .downcast_ref::<ExpressionError>()
            .or_else(|| error.downcast_ref::<Box<ExpressionError>>().map(AsRef::as_ref));
        if let Some(expression) = expression {
            found.push(expression.without_callbacks());
        }
        current = error.source();
    }
    found
}

fn data_error(failure: &ComposeFailure) -> Arc<DataError> {
    expression_errors(failure)
        .into_iter()
        .find_map(|error| match error {
            ExpressionError::Data(data) => Some(data.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no data error in: {}", failure.error))
}

/// The key and reason of an execution-time `InvalidOptions`.
fn invalid_option(failure: &ComposeFailure) -> (&'static str, String) {
    match &*data_error(failure) {
        DataError::InvalidOptions { key, reason } => (*key, reason.clone()),
        other => panic!("not an options error: {other}"),
    }
}

/// The format a `Malformed` failure names.
fn malformed_format(failure: &ComposeFailure) -> DataFormat {
    match &*data_error(failure) {
        DataError::Malformed { format, .. } => *format,
        other => panic!("not a malformed-data error: {other}"),
    }
}

/// `(reads, parses, binds)`.
fn work(runtime: &DataRuntime) -> (usize, usize, usize) {
    let stats = runtime.stats();
    (stats.reads, stats.parses, stats.binds)
}

/// `(source_hits, bind_hits)`.
fn hits(runtime: &DataRuntime) -> (usize, usize) {
    let stats = runtime.stats();
    (stats.source_hits, stats.bind_hits)
}

fn assert_nothing_read(runtime: &DataRuntime, label: &str) {
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.parses, stats.binds), (0, 0, 0), "{label}: {stats:?}");
}

const ROWS_CSV: &str = "name,port\nweb,8080\n";
const OBJECT_ROWS: &str = "\"{name: string, port: number}[]\"";

// ── R24: the input-robustness matrix ────────────────────────────────────────

#[test]
fn option_matrix_control_row() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let runtime = DataRuntime::new();
    let rows = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.csv\", {OBJECT_ROWS}, {{}})"));
    assert_eq!(rows, json!([{"name": "web", "port": 8080}]));
    assert_eq!(work(&runtime), (1, 1, 1));
    assert_eq!(runtime.read_set(), vec![entry(dir.path(), "rows.csv")]);
}

/// Who refuses a cell: the shared binder, or `DataOptions` for a string
/// `delimiter` the reader cannot use.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Refuser {
    Binder,
    Delimiter,
}

/// Every invalid cell, spelled once: the text is both an expression literal
/// and a YAML flow value with the same meaning.
const INVALID_CELLS: &[(&str, Refuser)] = &[
    ("5", Refuser::Binder),
    ("\"csv\"", Refuser::Binder),
    ("[]", Refuser::Binder),
    ("{quote: \",\"}", Refuser::Binder),
    ("{format: 5}", Refuser::Binder),
    ("{format: []}", Refuser::Binder),
    ("{format: {}}", Refuser::Binder),
    ("{format: \"\"}", Refuser::Binder),
    ("{format: \"CSV\"}", Refuser::Binder),
    ("{format: \" csv\"}", Refuser::Binder),
    ("{format: \"text\"}", Refuser::Binder),
    ("{delimiter: []}", Refuser::Binder),
    ("{delimiter: {}}", Refuser::Binder),
    ("{delimiter: \"\"}", Refuser::Delimiter),
    ("{delimiter: \",,\"}", Refuser::Delimiter),
    ("{delimiter: \"é\"}", Refuser::Delimiter),
    ("{delimiter: '\"'}", Refuser::Delimiter),
    ("{delimiter: \"\\r\"}", Refuser::Delimiter),
    ("{delimiter: \"\\n\"}", Refuser::Delimiter),
    ("{header: 5}", Refuser::Binder),
    ("{header: []}", Refuser::Binder),
    ("{header: {}}", Refuser::Binder),
    ("{header: \"\"}", Refuser::Binder),
];

/// Each invalid cell, written literally (refused by the passive check) and
/// through a frontmatter value (refused at execution), fails with nothing
/// read, parsed, bound, prepared, or recorded.
#[test]
fn option_matrix_invalid_cells_read_nothing() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let contract = "\"tuple([string, string])[]\"";
    for &(cell, refuser) in INVALID_CELLS {
        let literal = DataRuntime::new();
        let failure = failed(dir.path(), &literal, "", &format!("load_data(\"rows.csv\", {contract}, {cell})"));
        let expected = if refuser == Refuser::Binder { INCOMPATIBLE_CALL } else { INVALID_OPTIONS };
        assert_eq!(codes(&failure), [expected], "literal {cell}: {}", failure.error);
        assert_nothing_read(&literal, cell);
        assert_eq!(literal.stats().contract_preparations, 0, "literal {cell}");
        assert!(failure.read_set.is_empty() && literal.read_set().is_empty(), "literal {cell}");

        let computed = DataRuntime::new();
        let failure =
            failed(dir.path(), &computed, &format!("opts: {cell}"), &format!("load_data(\"rows.csv\", {contract}, opts)"));
        match refuser {
            Refuser::Binder => {
                assert_eq!(codes(&failure), [EVALUATION_FAILURE], "computed {cell}");
                assert!(failure.error.to_string().contains("options (argument 3)"), "computed {cell}: {}", failure.error);
            }
            Refuser::Delimiter => {
                assert_eq!(codes(&failure), [INVALID_OPTIONS], "computed {cell}");
                assert_eq!(invalid_option(&failure).0, "delimiter", "computed {cell}");
            }
        }
        assert_nothing_read(&computed, cell);
        assert_eq!(computed.stats().contract_preparations, 0, "computed {cell}: options are read before the contract");
        assert!(failure.read_set.is_empty() && computed.read_set().is_empty(), "computed {cell}");
    }
}

/// An omitted argument, `null`, `{}`, and each key absent or `null` are the
/// defaults, written literally or computed, and share one read, parse, and
/// binding.
#[test]
fn option_matrix_null_and_absent_cells_use_defaults() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let runtime = DataRuntime::new();
    let expected = json!([{"name": "web", "port": 8080}]);
    let omitted = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.csv\", {OBJECT_ROWS})"));
    assert_eq!(omitted, expected);
    let cells =
        ["null", "{}", "{format: null}", "{delimiter: null}", "{header: null}", "{format: null, delimiter: null, header: null}"];
    for cell in cells {
        let literal = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.csv\", {OBJECT_ROWS}, {cell})"));
        assert_eq!(literal, expected, "literal {cell}");
        let computed =
            loaded(dir.path(), &runtime, &format!("opts: {cell}"), &format!("load_data(\"rows.csv\", {OBJECT_ROWS}, opts)"));
        assert_eq!(computed, expected, "computed {cell}");
    }
    assert_eq!(work(&runtime), (1, 1, 1), "{:?}", runtime.stats());
    let reuses = cells.len() * 2;
    assert_eq!(hits(&runtime), (reuses, reuses), "{:?}", runtime.stats());
}

/// The shared coercion engine's ordinary conversions apply to options:
/// `delimiter: 5` binds to `"5"` and `header: "false"` to `false`.
#[test]
fn option_matrix_coercion_exceptions() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "fives.csv", "a5b\nx5y\n");
    write(dir.path(), "rows.csv", "a,b\n1,2\n");
    let contract = "\"tuple([string, string])[]\"";
    for frontmatter in ["", "five: 5\nno: \"false\""] {
        let (five, no) = if frontmatter.is_empty() { ("5", "\"false\"") } else { ("five", "no") };
        let runtime = DataRuntime::new();
        let split = loaded(dir.path(), &runtime, frontmatter, &format!("load_data(\"fives.csv\", {contract}, {{delimiter: {five}}})"));
        assert_eq!(split, json!([["x", "y"]]), "{frontmatter:?}");
        let headerless = loaded(dir.path(), &runtime, frontmatter, &format!("load_data(\"rows.csv\", {contract}, {{header: {no}}})"));
        assert_eq!(headerless, json!([["a", "b"], ["1", "2"]]), "{frontmatter:?}");
    }
}

/// The valid delimiter cells each split on their own character.
#[test]
fn option_matrix_valid_delimiters_read_rows() {
    let dir = TempDir::new().unwrap();
    let runtime = DataRuntime::new();
    for (index, (delimiter, written)) in [(',', ","), (';', ";"), ('|', "|"), ('\t', "\\t")].into_iter().enumerate() {
        let file = format!("rows{index}.csv");
        write(dir.path(), &file, format!("a{delimiter}b\n1{delimiter}2\n"));
        let rows = loaded(dir.path(), &runtime, "", &format!("load_data(\"{file}\", \"{{a: string, b: number}}[]\", {{delimiter: \"{written}\"}})"));
        assert_eq!(rows, json!([{"a": "1", "b": 2}]), "{written}");
    }
}

/// The two halves of the matrix reach a composed document's diagnostics:
/// a binder refusal and a `dm.data.invalid_options` finding, each located
/// at the options argument.
#[test]
fn option_wiring_binder_error_is_located_at_the_options_argument() {
    assert_wired("{quote: \",\"}", INCOMPATIBLE_CALL);
}

#[test]
fn option_wiring_invalid_options_is_located_at_the_options_argument() {
    assert_wired("{delimiter: \";;\"}", INVALID_OPTIONS);
}

fn assert_wired(options: &str, code: &str) {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let line = format!("{{{{ load_data(\"rows.csv\", \"tuple([string, string])[]\", {options}) }}}}");
    let runtime = DataRuntime::new();
    let failure = compose_named(dir.path(), "doc.md", &format!("{line}\n"), &runtime).expect_err("rejected");
    let record = failure.error.call_diagnostic().expect("a located record");
    assert_eq!(record.code, code, "{record:#?}");
    let source = record.source.as_ref().expect("located");
    let column = line.find(options).unwrap() + 1;
    assert_eq!((source.line, source.column), (Some(1), Some(column)), "{record:#?}");
    assert_eq!(codes(&failure), [code]);
    assert_nothing_read(&runtime, options);
    assert!(failure.read_set.is_empty());
}

// ── Headerless object rows ──────────────────────────────────────────────────

#[test]
fn computed_header_false_with_object_contract_fails_at_execution() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let runtime = DataRuntime::new();
    let failure = failed(dir.path(), &runtime, "hdr: false", &format!("load_data(\"rows.csv\", {OBJECT_ROWS}, {{header: hdr}})"));
    assert_eq!(codes(&failure), [ROW_SHAPE_CONFLICT]);
    assert!(matches!(&*data_error(&failure), DataError::RowShapeConflict { .. }));
    assert!(failure.error.to_string().contains("`header: false` reads rows by position"), "{}", failure.error);
    assert_nothing_read(&runtime, "header: false");
    assert_eq!(runtime.read_set(), vec![entry(dir.path(), "rows.csv")], "the file is still a dependency");
}

/// An imported contract is provisional to the passive check, so only
/// execution sees it describes object rows; it still fails before the data
/// file is read.
#[test]
fn imported_object_contract_with_header_false_fails_before_reading() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    write(dir.path(), "row.yaml", "kind: schema\ntypes:\n  Row: \"{name: string, port: number}[]\"\n");
    let runtime = DataRuntime::new();
    let failure = failed(dir.path(), &runtime, "", "load_data(\"rows.csv\", \"Row@./row.yaml\", {header: false})");
    assert_eq!(codes(&failure), [ROW_SHAPE_CONFLICT]);
    assert!(failure.error.to_string().contains("`header: false` reads rows by position"), "{}", failure.error);
    assert_eq!(runtime.stats().reads, 0, "{:?}", runtime.stats());
    assert_eq!(runtime.stats().contract_preparations, 1);
    assert_eq!(runtime.read_set(), vec![entry(dir.path(), "row.yaml"), entry(dir.path(), "rows.csv")]);
}

// ── Format and delimiter selection ──────────────────────────────────────────

#[test]
fn csv_format_with_semicolon_reads_txt() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.txt", "name;port\nweb;8080\ndb;5432\n");
    let runtime = DataRuntime::new();
    let rows = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.txt\", {OBJECT_ROWS}, {{format: \"csv\", delimiter: \";\"}})"));
    assert_eq!(rows, json!([{"name": "web", "port": 8080}, {"name": "db", "port": 5432}]));
    let without = failed(dir.path(), &runtime, "", &format!("load_data(\"rows.txt\", {OBJECT_ROWS})"));
    assert_eq!(codes(&without), ["dm.data.unsupported_format"]);
}

#[test]
fn explicit_format_overrides_a_supported_extension() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.json", ROWS_CSV);
    let runtime = DataRuntime::new();
    let rows = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.json\", {OBJECT_ROWS}, {{format: \"csv\"}})"));
    assert_eq!(rows, json!([{"name": "web", "port": 8080}]));
}

#[test]
fn tsv_format_defaults_to_tab() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.txt", "name\tport\nweb,east\t8080\n");
    let runtime = DataRuntime::new();
    let rows = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.txt\", {OBJECT_ROWS}, {{format: \"tsv\"}})"));
    assert_eq!(rows, json!([{"name": "web,east", "port": 8080}]), "a comma is data, not a separator");
}

#[test]
fn pipe_delimiter_reads_rows() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "name|port\nweb|8080\n");
    let runtime = DataRuntime::new();
    let rows = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.csv\", {OBJECT_ROWS}, {{delimiter: \"|\"}})"));
    assert_eq!(rows, json!([{"name": "web", "port": 8080}]));
}

#[test]
fn csv_with_tab_delimiter_reports_csv_in_malformed() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "bad.tsv", "a\tb\n\"x\t2\n");
    let runtime = DataRuntime::new();
    let call = "load_data(\"bad.tsv\", \"tuple([string, string])[]\", {format: \"csv\", delimiter: \"\\t\"})";
    let failure = failed(dir.path(), &runtime, "", call);
    assert_eq!(codes(&failure), ["dm.data.malformed"]);
    assert_eq!(malformed_format(&failure), DataFormat::Csv);
    assert!(failure.error.to_string().contains("is not valid CSV"), "{}", failure.error);
}

#[test]
fn tsv_extension_with_csv_format_parses_with_comma() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.tsv", "a,b\n1,2\n");
    let runtime = DataRuntime::new();
    let rows = loaded(dir.path(), &runtime, "", "load_data(\"rows.tsv\", \"tuple([string, string])[]\", {format: \"csv\"})");
    assert_eq!(rows, json!([["1", "2"]]));
}

// ── Aliases ─────────────────────────────────────────────────────────────────

/// Both spellings in one runtime: one read, one parse, one binding.
fn assert_alias_shares(file: &str, content: &str, alias: &str, canonical: &str, expected: &Value) {
    let dir = TempDir::new().unwrap();
    write(dir.path(), file, content);
    let runtime = DataRuntime::new();
    for format in [alias, canonical] {
        let value = loaded(dir.path(), &runtime, "", &format!("load_data(\"{file}\", \"string[]\", {{format: \"{format}\"}})"));
        assert_eq!(&value, expected, "{format}");
    }
    assert_eq!(work(&runtime), (1, 1, 1), "{:?}", runtime.stats());
    assert_eq!(hits(&runtime), (1, 1), "{:?}", runtime.stats());
}

#[test]
fn yml_alias_shares_parse_and_bind_with_yaml() {
    assert_alias_shares("os.txt", "- macOS\n- Linux\n", "yml", "yaml", &json!(["macOS", "Linux"]));
}

#[test]
fn ndjson_alias_shares_parse_and_bind_with_jsonl() {
    assert_alias_shares("lines.txt", "\"a\"\n\"b\"\n", "ndjson", "jsonl", &json!(["a", "b"]));
}

#[test]
fn alias_failure_names_the_canonical_format() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.txt", "- [macOS\n");
    write(dir.path(), "lines.txt", "{\n");
    let runtime = DataRuntime::new();
    for (file, alias, format, name) in
        [("os.txt", "yml", DataFormat::Yaml, "YAML"), ("lines.txt", "ndjson", DataFormat::Jsonl, "JSON Lines")]
    {
        let failure = failed(dir.path(), &runtime, "", &format!("load_data(\"{file}\", \"string[]\", {{format: \"{alias}\"}})"));
        assert_eq!(malformed_format(&failure), format, "{alias}");
        // The data error's own text; the composed failure also quotes the call.
        let message = data_error(&failure).to_string();
        assert!(message.contains(&format!("is not valid {name}")) && !message.contains(alias), "{message}");
    }
    let failure = failed(dir.path(), &runtime, "h: true", "load_data(\"os.txt\", \"string[]\", {format: \"yml\", header: h})");
    let (key, reason) = invalid_option(&failure);
    assert_eq!(key, "header");
    assert!(reason.contains("this call reads YAML") && !reason.contains("yml"), "{reason}");
}

// ── Applicability ───────────────────────────────────────────────────────────

/// An execution-time applicability failure: nothing read, the file still
/// recorded.
fn assert_inapplicable(dir: &Path, runtime: &DataRuntime, file: &str, failure: &ComposeFailure, key: &str, reason: &str) {
    assert_eq!(codes(failure), [INVALID_OPTIONS], "{}", failure.error);
    let (found_key, found_reason) = invalid_option(failure);
    assert_eq!(found_key, key);
    assert!(found_reason.contains(reason), "{found_reason}");
    assert_eq!(runtime.stats().reads, 0, "{:?}", runtime.stats());
    assert_eq!(runtime.read_set(), vec![entry(dir, file)]);
}

const TSV_REASON: &str = "TSV data is tab-separated, so only \"\\t\" is accepted";

#[test]
fn tsv_with_comma_delimiter_fails_before_reading() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let runtime = DataRuntime::new();
    let call = "load_data(\"rows.csv\", \"tuple([string, string])[]\", {format: \"tsv\", delimiter: d})";
    let failure = failed(dir.path(), &runtime, "d: \",\"", call);
    assert_inapplicable(dir.path(), &runtime, "rows.csv", &failure, "delimiter", TSV_REASON);
    // Written literally, the passive check refuses it first.
    let literal = DataRuntime::new();
    let call = "load_data(\"rows.csv\", \"tuple([string, string])[]\", {format: \"tsv\", delimiter: \",\"})";
    assert_eq!(codes(&failed(dir.path(), &literal, "", call)), [INVALID_OPTIONS]);
    assert_nothing_read(&literal, "literal");
}

#[test]
fn tsv_with_explicit_tab_delimiter_is_accepted() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.txt", "a\tb\n1\t2\n");
    let runtime = DataRuntime::new();
    for options in ["{format: \"tsv\", delimiter: \"\\t\"}", "{format: \"tsv\"}"] {
        let rows = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.txt\", \"tuple([string, string])[]\", {options})"));
        assert_eq!(rows, json!([["1", "2"]]), "{options}");
    }
    assert_eq!(work(&runtime), (1, 1, 1), "an explicit tab is the TSV default");
}

#[test]
fn tsv_extension_with_semicolon_delimiter_fails_before_reading() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.tsv", "a;b\n1;2\n");
    let runtime = DataRuntime::new();
    let failure = failed(dir.path(), &runtime, "d: \";\"", "load_data(\"rows.tsv\", \"tuple([string, string])[]\", {delimiter: d})");
    assert_inapplicable(dir.path(), &runtime, "rows.tsv", &failure, "delimiter", TSV_REASON);
}

#[test]
fn tabular_options_on_json_fail_before_reading() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.json", "[]");
    let frontmatter = "d: \";\"\nh: true";
    for (options, key) in [("{delimiter: d}", "delimiter"), ("{header: h}", "header"), ("{header: h, delimiter: d}", "delimiter")] {
        let runtime = DataRuntime::new();
        let failure = failed(dir.path(), &runtime, frontmatter, &format!("load_data(\"rows.json\", \"string[]\", {options})"));
        let reason = "applies only to CSV and TSV data, but this call reads JSON";
        assert_inapplicable(dir.path(), &runtime, "rows.json", &failure, key, reason);
    }
}

/// A delimiter the reader cannot use is refused before the contract is
/// prepared or the file is resolved.
#[test]
fn computed_invalid_delimiter_fails_at_execution() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let runtime = DataRuntime::new();
    let failure = failed(dir.path(), &runtime, "d: \";;\"", &format!("load_data(\"rows.csv\", {OBJECT_ROWS}, {{delimiter: d}})"));
    assert_eq!(codes(&failure), [INVALID_OPTIONS]);
    let (key, reason) = invalid_option(&failure);
    assert_eq!(key, "delimiter");
    assert!(reason.contains("must be one ASCII character"), "{reason}");
    assert_nothing_read(&runtime, ";;");
    assert_eq!(runtime.stats().contract_preparations, 0);
    assert!(runtime.read_set().is_empty() && failure.read_set.is_empty());
}

/// A computed `format` may override a literal file's extension, so the
/// passive check does not judge `delimiter` against `.json`.
#[test]
fn computed_format_overrides_a_literal_extension() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.json", "name;port\nweb;8080\n");
    let runtime = DataRuntime::new();
    let rows = loaded(dir.path(), &runtime, "fmt: csv", &format!("load_data(\"rows.json\", {OBJECT_ROWS}, {{format: fmt, delimiter: \";\"}})"));
    assert_eq!(rows, json!([{"name": "web", "port": 8080}]));
}

/// Resolution failures outrank format-dependent checks at execution. A
/// fully literal call is the passive check's, which judges the options from
/// the literal format and never looks for the file.
#[test]
fn missing_file_keeps_precedence_over_applicability() {
    let dir = TempDir::new().unwrap();
    let runtime = DataRuntime::new();
    let failure = failed(dir.path(), &runtime, "d: \";\"", "load_data(\"missing.json\", \"string[]\", {format: \"json\", delimiter: d})");
    assert_eq!(codes(&failure), [EVALUATION_FAILURE]);
    let kind = expression_errors(&failure).into_iter().find_map(|error| match error {
        ExpressionError::FileReference(diagnostic) => Some(diagnostic.kind),
        ExpressionError::Data(data) => match &**data {
            DataError::FileReference(diagnostic) => Some(diagnostic.kind),
            _ => None,
        },
        _ => None,
    });
    assert_eq!(kind, Some(FileRefFailure::NotFound), "{}", failure.error);
    assert_eq!(runtime.stats().reads, 0);

    let literal = DataRuntime::new();
    let failure = failed(dir.path(), &literal, "", "load_data(\"missing.json\", \"string[]\", {format: \"json\", delimiter: \";\"})");
    assert_eq!(codes(&failure), [INVALID_OPTIONS], "{}", failure.error);
}

// ── `null` ──────────────────────────────────────────────────────────────────

#[test]
fn null_options_and_null_fields_use_defaults() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "a,b\n1,2\n");
    let runtime = DataRuntime::new();
    let contract = "\"tuple([string, string])[]\"";
    let calls = [
        (String::new(), format!("load_data(\"rows.csv\", {contract}, null)")),
        (String::new(), format!("load_data(\"rows.csv\", {contract}, {{format: null, delimiter: null, header: null}})")),
        ("opts: null".to_string(), format!("load_data(\"rows.csv\", {contract}, opts)")),
        ("f: null".to_string(), format!("load_data(\"rows.csv\", {contract}, {{format: f, delimiter: f, header: f}})")),
    ];
    for (frontmatter, call) in &calls {
        assert_eq!(loaded(dir.path(), &runtime, frontmatter, call), json!([["1", "2"]]), "{call}");
    }
    assert_eq!(work(&runtime), (1, 1, 1), "{:?}", runtime.stats());
}

#[test]
fn null_file_with_options_reads_and_imports_nothing() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "row.yaml", "kind: schema\ntypes:\n  Row: \"{a: string}[]\"\n");
    let runtime = DataRuntime::new();
    let value = loaded(dir.path(), &runtime, "f: null", "load_data(f, \"Row@./row.yaml\", {format: \"csv\", header: false})");
    assert_eq!(value, Value::Null);
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.contract_preparations), (0, 0), "{stats:?}");
    assert!(runtime.read_set().is_empty(), "{:?}", runtime.read_set());
}

/// A `null` file returns before the options are interpreted, so semantic
/// option mistakes are not reported; the binder still ran, so a shape it
/// refuses is.
#[test]
fn null_file_with_invalid_computed_options_returns_null() {
    let dir = TempDir::new().unwrap();
    let runtime = DataRuntime::new();
    for options in ["{delimiter: \";;\"}", "{format: json, header: true}", "{format: tsv, delimiter: \",\"}"] {
        let value = loaded(dir.path(), &runtime, &format!("f: null\nopts: {options}"), "load_data(f, \"string[]\", opts)");
        assert_eq!(value, Value::Null, "{options}");
    }
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.contract_preparations), (0, 0), "{stats:?}");
    assert!(runtime.read_set().is_empty());
    let refused = failed(dir.path(), &runtime, "f: null\nopts: {quote: 1}", "load_data(f, \"string[]\", opts)");
    assert!(refused.error.to_string().contains("options (argument 3)"), "{}", refused.error);
}

// ── Memo reuse across interpretations ───────────────────────────────────────

#[test]
fn two_delimiters_read_once_and_parse_twice() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "a;b,c\n1;2,3\n");
    let runtime = DataRuntime::new();
    let contract = "\"tuple([string, string])[]\"";
    let comma = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.csv\", {contract})"));
    let semicolon = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.csv\", {contract}, {{delimiter: \";\"}})"));
    assert_eq!((comma, semicolon), (json!([["1;2", "3"]]), json!([["1", "2,3"]])));
    assert_eq!(work(&runtime), (1, 2, 2), "{:?}", runtime.stats());
    assert_eq!(hits(&runtime), (0, 0));
}

#[test]
fn equivalent_default_spellings_share_one_parse_and_bind() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "a,b\n1,2\n");
    write(dir.path(), "rows.tsv", "a\tb\n1\t2\n");
    let runtime = DataRuntime::new();
    let contract = "\"tuple([string, string])[]\"";
    let spellings = [
        ("rows.csv", ""),
        ("rows.csv", ", {format: \"csv\"}"),
        ("rows.csv", ", {delimiter: \",\"}"),
        ("rows.csv", ", {header: true}"),
        ("rows.csv", ", {format: \"csv\", delimiter: \",\", header: true}"),
        ("rows.tsv", ""),
        ("rows.tsv", ", {format: \"tsv\"}"),
        ("rows.tsv", ", {delimiter: \"\\t\"}"),
        ("rows.tsv", ", {format: \"tsv\", delimiter: \"\\t\", header: true}"),
    ];
    for (file, options) in spellings {
        let rows = loaded(dir.path(), &runtime, "", &format!("load_data(\"{file}\", {contract}{options})"));
        assert_eq!(rows, json!([["1", "2"]]), "{file}{options}");
    }
    assert_eq!(work(&runtime), (2, 2, 2), "{:?}", runtime.stats());
    assert_eq!(hits(&runtime), (7, 7), "{:?}", runtime.stats());
}

#[test]
fn header_only_change_parses_once_and_binds_twice() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "a,b\n1,2\n");
    let runtime = DataRuntime::new();
    let contract = "\"tuple([string, string])[]\"";
    let headed = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.csv\", {contract}, {{header: true}})"));
    let headerless = loaded(dir.path(), &runtime, "", &format!("load_data(\"rows.csv\", {contract}, {{header: false}})"));
    assert_eq!((headed, headerless), (json!([["1", "2"]]), json!([["a", "b"], ["1", "2"]])));
    assert_eq!(work(&runtime), (1, 1, 2), "{:?}", runtime.stats());
    assert_eq!(hits(&runtime), (1, 0));
}

/// Invalid UTF-8 is read once; each format turns it into its own failure,
/// and neither failure is reused for the other.
#[test]
fn distinct_formats_share_no_parse_or_failure() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.txt", b"ok\xFF\n");
    let runtime = DataRuntime::new();
    for (format, expected, name) in [("yaml", DataFormat::Yaml, "YAML"), ("json", DataFormat::Json, "JSON")] {
        let failure = failed(dir.path(), &runtime, "", &format!("load_data(\"data.txt\", \"any[]\", {{format: \"{format}\"}})"));
        match &*data_error(&failure) {
            DataError::Malformed { format, location, .. } => {
                assert_eq!((*format, location.byte_offset), (expected, Some(2)), "{name}");
            }
            other => panic!("{name}: {other}"),
        }
        assert!(failure.error.to_string().contains(&format!("is not valid {name}:")), "{}", failure.error);
    }
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.parses, stats.source_hits), (1, 0, 0), "{stats:?}");
}

const ORDERED_CALLS: [(&str, &str); 3] = [
    ("load_data(\"rows.csv\", \"{name: string, port: number}[]\")", "[{\"name\":\"web\",\"port\":8080}]"),
    ("load_data(\"rows.csv\", \"tuple([string, string])[]\", {header: false})", "[[\"name\",\"port\"],[\"web\",\"8080\"]]"),
    ("load_data(\"rows.csv\", \"tuple([string])[]\", {delimiter: \";\"})", "[[\"web,8080\"]]"),
];

#[test]
fn call_order_does_not_change_results() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let permutations = [[0, 1, 2], [0, 2, 1], [1, 0, 2], [1, 2, 0], [2, 0, 1], [2, 1, 0]];
    for order in permutations {
        let content: String = order.iter().map(|&index| format!("{{{{ as_json([{}]) }}}}\n\n", ORDERED_CALLS[index].0)).collect();
        let runtime = DataRuntime::new();
        let (composed, _) = compose_named(dir.path(), "doc.md", &content, &runtime)
            .unwrap_or_else(|failure| panic!("{order:?}: {}", failure.error));
        let lines: Vec<String> = composed.content().lines().filter(|line| !line.trim().is_empty()).map(str::to_string).collect();
        let expected: Vec<String> = order.iter().map(|&index| format!("[{}]", ORDERED_CALLS[index].1)).collect();
        assert_eq!(lines, expected, "{order:?}");
        assert_eq!(work(&runtime), (1, 2, 3), "{order:?}: {:?}", runtime.stats());
    }
}

/// A header that cannot name object fields fails the object binding alone;
/// positional bindings over the same parse still succeed, either side of it.
#[test]
fn object_header_failure_does_not_poison_positional_mode() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "dup.csv", "a,a\n1,2\n");
    let runtime = DataRuntime::new();
    let object = "load_data(\"dup.csv\", \"{a: string}[]\")";
    let failure = failed(dir.path(), &runtime, "", object);
    assert_eq!(codes(&failure), ["dm.data.malformed"]);
    assert!(failure.error.to_string().contains("repeats `a`"), "{}", failure.error);
    let positional = "\"tuple([string, string])[]\"";
    let headerless = loaded(dir.path(), &runtime, "", &format!("load_data(\"dup.csv\", {positional}, {{header: false}})"));
    assert_eq!(headerless, json!([["a", "a"], ["1", "2"]]));
    let headed = loaded(dir.path(), &runtime, "", &format!("load_data(\"dup.csv\", {positional})"));
    assert_eq!(headed, json!([["1", "2"]]));
    assert_eq!(codes(&failed(dir.path(), &runtime, "", object)), ["dm.data.malformed"]);
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.parses), (1, 1), "{stats:?}");
}

#[test]
fn reused_failure_names_each_call_site() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "bad.txt", "[1,");
    let runtime = DataRuntime::new();
    let call = "{{ load_data(\"bad.txt\", \"number[]\", {format: \"json\"}) }}";
    for (site, content, line) in [("first.md", format!("{call}\n"), 1), ("second.md", format!("intro\n\nsee {call}\n"), 3)] {
        let failure = compose_named(dir.path(), site, &content, &runtime).expect_err("malformed");
        assert_eq!(malformed_format(&failure), DataFormat::Json, "{site}");
        let records = error_diagnostics(&failure.error);
        assert_eq!(records.len(), 1, "{site}: {records:#?}");
        let source = records[0].source.as_ref().expect("located");
        assert!(source.file.as_deref().is_some_and(|file| file.ends_with(site)), "{site}: {source:?}");
        assert_eq!(source.line, Some(line), "{site}: {source:?}");
    }
    assert_eq!(runtime.stats().reads, 1, "the second site reused the memoized failure");
}

struct AllowAll;

impl ShellApprovalHandler for AllowAll {
    fn approve(
        &self,
        _request: ShellApprovalRequest,
    ) -> Result<ShellApprovalDecision, darkmatter::markdown::compose::ShellExpansionError> {
        Ok(ShellApprovalDecision::AllowOnce)
    }
}

/// A frontmatter command ends reuse at every layer, in every cache mode:
/// the read after it reads, parses, and binds again, and records nothing new.
#[test]
fn invalidation_rereads_every_layer_in_every_cache_mode() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "a\n1\n");
    let call = "load_data('rows.csv', 'tuple([string])[]', {header: false})";
    let doc = write(
        dir.path(),
        "doc.md",
        format!("---\nbefore: \"{{{{ length({call}) }}}}\"\nran: $(git --version)\n---\n{{{{ before }}}}-{{{{ length({call}) }}}}\n"),
    );
    let shell = ComposeOptions::new()
        .with_shell(ShellExpansionOptions { policy_root: Some(dir.path().to_path_buf()), ..Default::default() })
        .with_shell_working_directory(dir.path())
        .with_shell_approval_handler(Arc::new(AllowAll));
    for mode in [CacheAccessMode::Off, CacheAccessMode::ReadOnly, CacheAccessMode::ReadWrite, CacheAccessMode::Refresh] {
        let runtime = DataRuntime::new();
        let options = shell.clone().with_cache_access_mode(mode).with_data_runtime(runtime.clone());
        let (composed, report) = compose_file(&doc, options).unwrap_or_else(|failure| panic!("{mode:?}: {}", failure.error));
        assert_eq!(composed.content().trim(), "2-2", "{mode:?}");
        let stats = runtime.stats();
        assert_eq!((stats.reads, stats.parses, stats.binds), (2, 2, 2), "{mode:?}: {stats:?}");
        assert_eq!((stats.source_hits, stats.bind_hits), (0, 0), "{mode:?}: {stats:?}");
        assert!(stats.invalidations >= 1, "{mode:?}: {stats:?}");
        assert_eq!(runtime.read_set(), vec![entry(dir.path(), "rows.csv")], "{mode:?}");
        assert!(report.read_set.contains(&entry(dir.path(), "rows.csv")), "{mode:?}: {:?}", report.read_set);
    }
}

/// Four compositions on four threads over one runtime, asking for the same
/// interpretation: one read, one parse, one binding.
#[test]
fn concurrent_identical_option_calls_read_once() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.txt", "name;port\nweb;8080\n");
    let content = document("", &format!("load_data(\"rows.txt\", {OBJECT_ROWS}, {{format: \"csv\", delimiter: \";\"}})"));
    let docs: Vec<PathBuf> = (0..4).map(|index| write(dir.path(), &format!("doc{index}.md"), &content)).collect();
    let runtime = DataRuntime::new();
    let outputs: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = docs
            .iter()
            .map(|doc| {
                let options = ComposeOptions::new().with_data_runtime(runtime.clone());
                scope.spawn(move || match compose_file(doc, options) {
                    Ok((composed, _)) => composed.content().trim().to_string(),
                    Err(failure) => panic!("{}: {}", doc.display(), failure.error),
                })
            })
            .collect();
        handles.into_iter().map(|handle| handle.join().expect("thread")).collect()
    });
    assert!(outputs.iter().all(|output| output == "[[{\"name\":\"web\",\"port\":8080}]]"), "{outputs:?}");
    assert_eq!(work(&runtime), (1, 1, 1), "{:?}", runtime.stats());
}
