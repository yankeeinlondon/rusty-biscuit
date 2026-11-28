//! `load_data` without a contract, root normalization, and the three CSV/TSV
//! header modes, observed through composition's public result: the value a
//! call returns, the failure's typed error and diagnostic records, and the
//! request runtime's read/parse/bind counts (`DataRuntime::stats()`).
//!
//! Defaults the cells below rely on: every invalid shape is a located error
//! that publishes no composed value; options and row-shape refusals read
//! nothing; malformed content and a shape conflict are never recovered by
//! `try`. A literal options object is judged by the passive call check
//! before anything runs, so a cell about what *execution* decides passes its
//! options through a frontmatter value.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::DataError;
use darkmatter::markdown::compose::expression::{DataLocation, ExpressionError};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeFailure, ComposeOptions, ComposeReport, DataRuntime};
use serde_json::{Value, json};
use tempfile::TempDir;

const MALFORMED: &str = "dm.data.malformed";
const MISMATCH: &str = "dm.data.contract_mismatch";
const ROW_SHAPE_CONFLICT: &str = "dm.data.row_shape_conflict";
const INVALID_OPTIONS: &str = "dm.data.invalid_options";

// ── Fixtures ────────────────────────────────────────────────────────────────

fn write(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// The portable spelling `data_location.file` uses for `name` in `dir`.
fn entry(dir: &Path, name: &str) -> String {
    let canonical = biscuit_file::canonicalize_simplified(dir).expect("canonical dir");
    biscuit_file::to_forward_slash_string(&canonical.join(name))
}

type Composed = Result<(Markdown, ComposeReport), Box<ComposeFailure>>;

/// Composes a document rendering `call`'s value as JSON under `frontmatter`
/// (when not empty), in `dir`, sharing `runtime`.
fn compose(dir: &Path, runtime: &DataRuntime, frontmatter: &str, call: &str) -> Composed {
    let body = format!("{{{{ as_json([{call}]) }}}}\n");
    let content = if frontmatter.is_empty() { body } else { format!("---\n{frontmatter}\n---\n{body}") };
    let path = write(dir, "doc.md", content);
    let options = ComposeOptions::new().with_data_runtime(runtime.clone()).with_source_file(&path);
    Markdown::try_from(path.as_path()).expect("document loads").compose_with_warnings(&crate::request_support::request(options))
}

fn attempt(dir: &Path, runtime: &DataRuntime, frontmatter: &str, call: &str) -> Result<Value, Box<ComposeFailure>> {
    let (composed, _) = compose(dir, runtime, frontmatter, call)?;
    let text = composed.content().trim().to_string();
    let wrapped: Value = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text:?} is not JSON: {error}"));
    Ok(wrapped[0].clone())
}

/// The value `call` returns, in a fresh runtime.
fn value(dir: &Path, call: &str) -> Value {
    loaded(dir, &DataRuntime::new(), "", call)
}

fn loaded(dir: &Path, runtime: &DataRuntime, frontmatter: &str, call: &str) -> Value {
    attempt(dir, runtime, frontmatter, call).unwrap_or_else(|failure| panic!("{call} failed: {}", failure.error))
}

fn failed(dir: &Path, runtime: &DataRuntime, frontmatter: &str, call: &str) -> Box<ComposeFailure> {
    match attempt(dir, runtime, frontmatter, call) {
        Ok(value) => panic!("{call} under {frontmatter:?} returned {value}"),
        Err(failure) => failure,
    }
}

fn codes(failure: &ComposeFailure) -> Vec<String> {
    error_diagnostics(&failure.error).into_iter().map(|record| record.code).collect()
}

fn data_locations(failure: &ComposeFailure) -> Vec<Option<DataLocation>> {
    error_diagnostics(&failure.error).into_iter().map(|record| record.data_location).collect()
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

/// A `Malformed` failure's message and `(line, column)`.
fn malformed(failure: &ComposeFailure) -> (String, (Option<usize>, Option<usize>)) {
    match &*data_error(failure) {
        DataError::Malformed { message, location, .. } => (message.clone(), (location.line, location.column)),
        other => panic!("not malformed: {other}"),
    }
}

/// `(reads, parses, binds)`.
fn work(runtime: &DataRuntime) -> (usize, usize, usize) {
    let stats = runtime.stats();
    (stats.reads, stats.parses, stats.binds)
}

fn located(file: String, pointer: &str, line: usize, column: usize) -> Option<DataLocation> {
    Some(DataLocation { file, pointer: Some(pointer.to_string()), line: Some(line), column: Some(column), byte_offset: None })
}

const ROWS_CSV: &str = "name,port\nweb,8080\n";

// ── File and contract ──────────────────────────────────────────────────────

/// An omitted, a literal `null`, and a computed `null` contract are one
/// call: one read, one parse, one bind; CSV cells stay text.
#[test]
fn omitted_and_null_contracts_share_one_read_parse_and_bind() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    let runtime = DataRuntime::new();
    let rows = json!([{"name": "web", "port": "8080"}]);
    assert_eq!(loaded(dir.path(), &runtime, "", "load_data(\"rows.csv\")"), rows);
    assert_eq!(loaded(dir.path(), &runtime, "", "load_data(\"rows.csv\", null)"), rows);
    assert_eq!(loaded(dir.path(), &runtime, "c: null", "load_data(\"rows.csv\", c)"), rows);
    assert_eq!(loaded(dir.path(), &runtime, "", "load_data(\"rows.csv\", null, {})"), rows);
    assert_eq!(work(&runtime), (1, 1, 1), "{:?}", runtime.stats());
    assert_eq!(runtime.stats().contract_preparations, 0);
    // A contract is its own binding over the same parse.
    let typed = loaded(dir.path(), &runtime, "", "load_data(\"rows.csv\", \"{name: string, port: number}[]\")");
    assert_eq!(typed, json!([{"name": "web", "port": 8080}]));
    assert_eq!(work(&runtime), (1, 1, 2), "{:?}", runtime.stats());
}

/// A literal or computed `null` file returns `null` before options, the
/// contract (here an import of a missing file), or any read.
#[test]
fn a_null_file_returns_null_without_reads_or_imports() {
    let dir = TempDir::new().unwrap();
    let runtime = DataRuntime::new();
    for (frontmatter, call) in [
        ("", "load_data(null)"),
        ("", "load_data(null, \"Row@./missing.yaml\")"),
        ("f: null", "load_data(f)"),
        ("f: null", "load_data(f, \"Row@./missing.yaml\", {header: [\"a\"]})"),
    ] {
        assert_eq!(loaded(dir.path(), &runtime, frontmatter, call), Value::Null, "{call}");
    }
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.contract_preparations), (0, 0), "{stats:?}");
    assert!(runtime.read_set().is_empty());
}

/// A scalar root is wrapped in a one-element array before any contract;
/// a tuple needing two elements fails at the document's root.
#[test]
fn a_scalar_root_is_wrapped_before_the_contract() {
    let dir = TempDir::new().unwrap();
    for (name, text, expected) in [
        ("n.yaml", "5\n", json!([5])),
        ("b.yaml", "true\n", json!([true])),
        ("s.json", "\"\"", json!([""])),
        ("s.yaml", "''\n", json!([""])),
    ] {
        write(dir.path(), name, text);
        assert_eq!(value(dir.path(), &format!("load_data(\"{name}\")")), expected, "{name}");
    }
    assert_eq!(value(dir.path(), "load_data(\"n.yaml\", \"number[]\")"), json!([5]));
    let failure = failed(dir.path(), &DataRuntime::new(), "", "load_data(\"n.yaml\", \"tuple([number, number])\")");
    assert_eq!(codes(&failure), [MISMATCH]);
    assert_eq!(data_locations(&failure), [located(entry(dir.path(), "n.yaml"), "", 1, 1)]);
}

/// A `null` root is `[]`; empty or malformed input follows its parser.
#[test]
fn a_null_root_is_an_empty_list() {
    let dir = TempDir::new().unwrap();
    for (name, text) in [("e.yaml", ""), ("t.yaml", "~\n"), ("n.json", "null"), ("e.jsonl", "")] {
        write(dir.path(), name, text);
        assert_eq!(value(dir.path(), &format!("load_data(\"{name}\")")), json!([]), "{name}");
    }
    assert_eq!(value(dir.path(), "load_data(\"n.json\", \"string[]\")"), json!([]), "the contract judges `[]`");
    write(dir.path(), "e.json", "");
    let failure = failed(dir.path(), &DataRuntime::new(), "", "load_data(\"e.json\")");
    assert_eq!(codes(&failure), [MALFORMED]);
    // A mapping root is kept, and its members stay unconverted.
    write(dir.path(), "m.toml", "port = \"8080\"\n");
    assert_eq!(value(dir.path(), "load_data(\"m.toml\")"), json!({"port": "8080"}));
}

// ── Header modes ───────────────────────────────────────────────────────────

const EXPORT_CSV: &str = "First Name,Age\nAda,36\n";

/// The specification's example: each header mode without a contract.
#[test]
fn each_header_mode_chooses_the_row_shape_without_a_contract() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "export.csv", EXPORT_CSV);
    write(dir.path(), "export.tsv", "First Name\tAge\nAda\t36\n");
    for call in ["load_data(\"export.csv\")", "load_data(\"export.csv\", null, {header: true})", "load_data(\"export.tsv\")"] {
        assert_eq!(value(dir.path(), call), json!([{"firstName": "Ada", "age": "36"}]), "{call}");
    }
    assert_eq!(
        value(dir.path(), "load_data(\"export.csv\", null, {header: false})"),
        json!([["First Name", "Age"], ["Ada", "36"]])
    );
    assert_eq!(
        value(dir.path(), "load_data(\"export.csv\", null, {header: [\"label\", \"years\"]})"),
        json!([{"label": "First Name", "years": "Age"}, {"label": "Ada", "years": "36"}])
    );
}

/// With a contract, the contract chooses the row shape and the header mode
/// must allow it.
#[test]
fn each_header_mode_with_a_compatible_contract() {
    let dir = TempDir::new().unwrap();
    // Names a header record could not key objects with: positional reading
    // skips the record without inspecting them.
    write(dir.path(), "dup.csv", "a,a\nweb,8080\n");
    assert_eq!(value(dir.path(), "load_data(\"dup.csv\", \"tuple([string, number])[]\")"), json!([["web", 8080]]));
    assert_eq!(
        value(dir.path(), "load_data(\"dup.csv\", \"tuple([string, string])[]\", {header: false})"),
        json!([["a", "a"], ["web", "8080"]])
    );
    assert_eq!(
        value(dir.path(), "load_data(\"dup.csv\", \"{label: string, port: string}[]\", {header: [\"label\", \"port\"]})"),
        json!([{"label": "a", "port": "a"}, {"label": "web", "port": "8080"}])
    );
    write(dir.path(), "export.csv", EXPORT_CSV);
    assert_eq!(
        value(dir.path(), "load_data(\"export.csv\", \"{firstName: string, age: number}[]\")"),
        json!([{"firstName": "Ada", "age": 36}])
    );
}

/// Explicit names are used verbatim after ordinary binding (a number binds
/// as its text); two lists over one file are two bindings of one parse.
#[test]
fn explicit_names_are_verbatim_and_bind_separately() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "plain.csv", "web,8080\n");
    let runtime = DataRuntime::new();
    let call = |names: &str| format!("load_data(\"plain.csv\", null, {{header: {names}}})");
    assert_eq!(loaded(dir.path(), &runtime, "", &call("[\" Label \", \"port\"]")), json!([{" Label ": "web", "port": "8080"}]));
    assert_eq!(loaded(dir.path(), &runtime, "", &call("[\"port\", \" Label \"]")), json!([{"port": "web", " Label ": "8080"}]));
    assert_eq!(loaded(dir.path(), &runtime, "", &call("[\" Label \", \"port\"]")), json!([{" Label ": "web", "port": "8080"}]));
    assert_eq!(work(&runtime), (1, 1, 2), "{:?}", runtime.stats());
    assert_eq!(runtime.stats().bind_hits, 1);
    assert_eq!(value(dir.path(), &call("[1, 2]")), json!([{"1": "web", "2": "8080"}]));
}

/// Header names that cannot key objects fail on the header record, naming
/// the original text, even with no data rows; a contract does not change it.
#[test]
fn unusable_header_names_are_malformed_at_the_header() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "symbols.csv", "id,!!!\n1,2\n");
    let failure = failed(dir.path(), &DataRuntime::new(), "", "load_data(\"symbols.csv\")");
    let (message, position) = malformed(&failure);
    assert!(message.contains("column 2 (`!!!`) has no letters or digits"), "{message}");
    assert_eq!(position, (Some(1), Some(4)));

    write(dir.path(), "collide.csv", "First Name,first_name\n");
    for call in ["load_data(\"collide.csv\")", "load_data(\"collide.csv\", \"{ <string>: string }[]\")"] {
        let failure = failed(dir.path(), &DataRuntime::new(), "", call);
        assert_eq!(codes(&failure), [MALFORMED], "{call}");
        let (message, position) = malformed(&failure);
        assert!(message.contains("columns 1 (`First Name`) and 2 (`first_name`) both name the field `firstName`"), "{message}");
        assert_eq!(position, (Some(1), Some(12)), "{call}");
    }
}

/// An explicit list must be as wide as the first record, which is checked
/// before any row is built; later records keep the parser's width check.
#[test]
fn explicit_names_must_match_the_first_record_width() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "wide.csv", "\n\na,b,c\n1,2,3\n");
    let failure = failed(dir.path(), &DataRuntime::new(), "", "load_data(\"wide.csv\", null, {header: [\"x\", \"y\"]})");
    assert_eq!(codes(&failure), [MALFORMED]);
    let (message, position) = malformed(&failure);
    assert!(message.contains("names 2 columns, but the first record on line 3 has 3 fields"), "{message}");
    assert_eq!(position, (Some(3), Some(1)));

    write(dir.path(), "ragged.csv", "a,b\n1\n");
    let failure = failed(dir.path(), &DataRuntime::new(), "", "load_data(\"ragged.csv\", null, {header: [\"x\", \"y\"]})");
    assert_eq!(codes(&failure), [MALFORMED]);
    assert_eq!(malformed(&failure).1 .0, Some(2), "the ragged record's own line");
}

/// Input with no records has no rows in every mode, and a supplied contract
/// judges that `[]`.
#[test]
fn no_records_is_an_empty_list_judged_by_the_contract() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "empty.csv", "");
    for options in ["{}", "{header: false}", "{header: [\"label\"]}"] {
        assert_eq!(value(dir.path(), &format!("load_data(\"empty.csv\", null, {options})")), json!([]), "{options}");
    }
    let failure =
        failed(dir.path(), &DataRuntime::new(), "", "load_data(\"empty.csv\", \"tuple([{label: string}])\", {header: [\"label\"]})");
    assert_eq!(codes(&failure), [MISMATCH]);
}

/// A computed header that conflicts with the contract, or that is invalid,
/// fails before the read and is not recovered by `try`.
#[test]
fn computed_header_refusals_read_nothing_and_are_not_recovered() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", ROWS_CSV);
    write(dir.path(), "rows.yaml", "- a\n");
    for (frontmatter, call, code, key) in [
        ("opts: {header: [a]}", "load_data(\"rows.csv\", \"tuple([string, string])[]\", opts)", ROW_SHAPE_CONFLICT, None),
        ("opts: {header: false}", "load_data(\"rows.csv\", \"{name: string}[]\", opts)", ROW_SHAPE_CONFLICT, None),
        ("opts: {header: [a, a]}", "load_data(\"rows.csv\", null, opts)", INVALID_OPTIONS, Some("header")),
        ("opts: {header: []}", "load_data(\"rows.csv\", null, opts)", INVALID_OPTIONS, Some("header")),
        ("opts: {header: [a]}", "load_data(\"rows.yaml\", null, opts)", INVALID_OPTIONS, Some("header")),
    ] {
        for call in [call.to_string(), format!("try({call}, [])")] {
            let runtime = DataRuntime::new();
            let failure = failed(dir.path(), &runtime, frontmatter, &call);
            assert_eq!(codes(&failure), [code], "{call}");
            if let Some(key) = key {
                match &*data_error(&failure) {
                    DataError::InvalidOptions { key: found, .. } => assert_eq!(*found, key, "{call}"),
                    other => panic!("{call}: {other}"),
                }
            }
            assert_eq!(work(&runtime), (0, 0, 0), "{call}: {:?}", runtime.stats());
        }
    }
    // `null` is "unspecified" on any format.
    assert_eq!(value(dir.path(), "load_data(\"rows.yaml\", null, {header: null})"), json!(["a"]));
}

/// Header names that cannot key objects, and an explicit list of the wrong
/// width, are malformed data: like any present-but-bad file, `try` recovers
/// them, while the same call alone fails.
#[test]
fn malformed_names_are_recovered_like_other_malformed_data() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "dup.csv", "a,a\n1,2\n");
    for call in ["load_data(\"dup.csv\")", "load_data(\"dup.csv\", null, {header: [\"x\"]})"] {
        assert_eq!(codes(&failed(dir.path(), &DataRuntime::new(), "", call)), [MALFORMED], "{call}");
        assert_eq!(value(dir.path(), &format!("try({call}, [])")), json!([]), "{call}");
    }
}

/// A contract violation names the original cell through a header-derived
/// key and through an explicit name.
#[test]
fn contract_failures_locate_cells_by_their_keys() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "export.csv", "First Name,Age\nAda,old\n");
    let failure = failed(dir.path(), &DataRuntime::new(), "", "load_data(\"export.csv\", \"{firstName: string, age: number}[]\")");
    assert_eq!(data_locations(&failure), [located(entry(dir.path(), "export.csv"), "/0/age", 2, 5)]);

    write(dir.path(), "plain.csv", "Ada,old\n");
    let failure = failed(
        dir.path(),
        &DataRuntime::new(),
        "",
        "load_data(\"plain.csv\", \"{label: string, years: number}[]\", {header: [\"label\", \"years\"]})",
    );
    assert_eq!(data_locations(&failure), [located(entry(dir.path(), "plain.csv"), "/0/years", 1, 5)]);
}
