//! `load_data(file, as)` executed through public composition.
//!
//! Every fixture lives in its own temporary directory and every request is
//! anchored there, so no test reads this repository or the process
//! directory. Each test asserts a public result: the composed text, the
//! typed `DataError` inside a `ComposeFailure`, the read set, or a
//! caller-attached runtime's counters.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::contract::ROW_SHAPE_SUGGESTION;
use darkmatter::markdown::compose::expression::data::{DataError, DataSourceLocation, RowMode};
use darkmatter::markdown::compose::expression::{ExpressionError, FileRefFailure, FileReferenceDiagnostic};
use darkmatter::markdown::compose::shell_expansion::{
    ShellApprovalDecision, ShellApprovalHandler, ShellApprovalRequest, ShellExpansionOptions,
};
use darkmatter::markdown::compose::{CacheAccessMode, ComposeFailure, ComposeOptions, ComposeReport, DataRuntime};
use serde_json::{Value, json};
use simplified_schema::coercion::ActualKind;
use tempfile::TempDir;

// ── Fixtures ────────────────────────────────────────────────────────────────

fn write(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
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

/// Composes `content` as `doc.md` in `dir`, sharing `runtime`.
fn compose_with(dir: &Path, content: &str, runtime: &DataRuntime) -> Composed {
    let path = write(dir, "doc.md", content);
    compose_file(&path, ComposeOptions::new().with_data_runtime(runtime.clone()))
}

fn compose(dir: &Path, content: &str) -> Composed {
    compose_with(dir, content, &DataRuntime::new())
}

fn rendered(dir: &Path, content: &str) -> String {
    match compose(dir, content) {
        Ok((composed, _)) => composed.content().trim().to_string(),
        Err(failure) => panic!("{content:?} failed: {}", failure.error),
    }
}

/// The value `load_data(file, contract)` returns. `as_json` renders only a
/// list, so the value is wrapped in one.
fn loaded(dir: &Path, file: &str, contract: &str) -> Value {
    let text = rendered(dir, &format!("{{{{ as_json([load_data(\"{file}\", \"{contract}\")]) }}}}\n"));
    let wrapped: Value = serde_json::from_str(&text).unwrap_or_else(|error| panic!("{text:?} is not JSON: {error}"));
    wrapped[0].clone()
}

fn failure_of(dir: &Path, content: &str) -> Box<ComposeFailure> {
    match compose(dir, content) {
        Ok((composed, _)) => panic!("{content:?} composed: {}", composed.content()),
        Err(failure) => failure,
    }
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

fn file_reference(failure: &ComposeFailure) -> FileReferenceDiagnostic {
    expression_errors(failure)
        .into_iter()
        .find_map(|error| match error {
            ExpressionError::FileReference(diagnostic) => Some(diagnostic.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("no file-reference error in: {}", failure.error))
}

/// The typed failure of `load_data(file, contract)`.
fn load_failure(dir: &Path, file: &str, contract: &str) -> Arc<DataError> {
    data_error(&failure_of(dir, &format!("{{{{ load_data(\"{file}\", \"{contract}\") }}}}\n")))
}

fn malformed(dir: &Path, file: &str, contract: &str) -> DataSourceLocation {
    match &*load_failure(dir, file, contract) {
        DataError::Malformed { location, .. } => location.clone(),
        other => panic!("{file}: expected malformed, got {other}"),
    }
}

/// The `(pointer, actual kind)` of every located contract violation.
fn mismatches(dir: &Path, file: &str, contract: &str) -> Vec<(String, ActualKind)> {
    match &*load_failure(dir, file, contract) {
        DataError::ContractMismatch { failure, written, .. } => {
            assert_eq!(written, contract);
            failure.located_failures().into_iter().map(|located| (located.pointer, located.actual)).collect()
        }
        other => panic!("{file}: expected a contract mismatch, got {other}"),
    }
}

fn line_column(location: &DataSourceLocation) -> (Option<usize>, Option<usize>) {
    (location.line, location.column)
}

const OS_YAML: &str = "- macOS\n- Linux\n- Windows\n";

// ── R1: signature, null, imports, computed contracts ────────────────────────

#[test]
fn the_os_example_renders_as_csv() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    let text = rendered(dir.path(), "Supported: {{ as_csv(load_data(\"os.yaml\", \"string[]\")) }}\n");
    assert_eq!(text, "Supported: macOS, Linux, Windows");
}

#[test]
fn a_null_file_returns_null_and_reads_nothing() {
    let dir = TempDir::new().unwrap();
    let runtime = DataRuntime::new();
    let (composed, report) =
        compose_with(dir.path(), "{{ is_null(load_data(null, \"x@./missing.yaml\")) }}\n", &runtime).unwrap();
    assert_eq!(composed.content().trim(), "true");
    assert!(report.read_set.is_empty(), "{:?}", report.read_set);
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.contract_preparations), (0, 0), "{stats:?}");
}

#[test]
fn a_named_file_import_executes() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    write(dir.path(), "schemas/os.yaml", "kind: schema\ntypes:\n  os-list: string[]\n");
    assert_eq!(loaded(dir.path(), "os.yaml", "os-list@./schemas/os.yaml"), json!(["macOS", "Linux", "Windows"]));
}

#[test]
fn a_computed_scalar_contract_is_not_a_container() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    let failure = failure_of(dir.path(), "---\nt: string\n---\n{{ load_data(\"os.yaml\", t) }}\n");
    let error = data_error(&failure);
    assert!(matches!(&*error, DataError::NotAContainer { written } if written == "string"), "{error}");
    assert_eq!(error.code(), Some("dm.data.not_a_container"));
}

#[test]
fn a_computed_this_import_is_rejected() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    let failure = failure_of(dir.path(), "---\nt: Os@this\n---\n{{ load_data(\"os.yaml\", t) }}\n");
    assert_eq!(data_error(&failure).code(), Some("dm.data.this_import_rejected"));
}

// ── R3: one fixture per extension ───────────────────────────────────────────

macro_rules! loads {
    ($($name:ident: $file:literal, $content:expr, $contract:literal => $expected:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let dir = TempDir::new().unwrap();
                write(dir.path(), $file, $content);
                assert_eq!(loaded(dir.path(), $file, $contract), $expected);
            }
        )+
    };
}

loads! {
    yaml_loads: "d.yaml", "a: [1, 2]\n", "{ a: number[] }" => json!({"a": [1, 2]});
    yml_loads: "d.yml", "a: [1, 2]\n", "{ a: number[] }" => json!({"a": [1, 2]});
    json_loads: "d.json", "{\"a\": [1, 2]}", "{ a: number[] }" => json!({"a": [1, 2]});
    jsonl_loads: "d.jsonl", "{\"a\": 1}\n{\"a\": 2}\n", "{ a: number }[]" => json!([{"a": 1}, {"a": 2}]);
    ndjson_loads: "d.ndjson", "{\"a\": 1}\n{\"a\": 2}\n", "{ a: number }[]" => json!([{"a": 1}, {"a": 2}]);
    csv_loads: "d.csv", "a,b\n1,x\n", "{ a: number, b: string }[]" => json!([{"a": 1, "b": "x"}]);
    tsv_loads: "d.tsv", "a\tb\n1\tx\n", "{ a: number, b: string }[]" => json!([{"a": 1, "b": "x"}]);
    toml_loads: "d.toml", "[a]\nb = 1\n", "{ a: { b: number } }" => json!({"a": {"b": 1}});
    json5_loads: "d.json5", "{a: [1, 2]}", "{ a: number[] }" => json!({"a": [1, 2]});
    an_upper_case_extension_loads: "DATA.JSON", "[1, 2]", "number[]" => json!([1, 2]);

    // Document-loader cases that load.
    json_duplicate_keys_keep_the_last: "d.json", "{\"a\": 1, \"a\": 2}", "{ a: number }" => json!({"a": 2});
    yaml_aliases_are_expanded: "d.yaml", "base: &b [x, y]\ncopy: *b\n", "{ <string>: string[] }"
        => json!({"base": ["x", "y"], "copy": ["x", "y"]});
    json5_comments_and_trailing_commas_load: "d.json5", "// note\n{a: [1, 2,],}\n", "{ a: number[] }"
        => json!({"a": [1, 2]});
    json5_duplicate_keys_keep_the_last: "d.json5", "{a: 1, a: 2}", "{ a: number }" => json!({"a": 2});
    jsonl_blank_lines_are_skipped: "d.jsonl", "1\n\n  \n2\n", "number[]" => json!([1, 2]);
    csv_quoted_delimiters_and_newlines_load: "d.csv", "a,b\n\"1,2\",\"x\ny\"\n", "{ a: string, b: string }[]"
        => json!([{"a": "1,2", "b": "x\ny"}]);
    tsv_quoted_tabs_load: "d.tsv", "a\tb\n\"1\t2\"\tz\n", "{ a: string, b: string }[]"
        => json!([{"a": "1\t2", "b": "z"}]);
    // `<<` is an ordinary key: the shared loader expands no merge keys.
    yaml_merge_keys_stay_ordinary_keys: "d.yaml", "base: &b {x: 1}\nother:\n  <<: *b\n", "{ <string>: { <string>: unknown } }"
        => json!({"base": {"x": 1}, "other": {"<<": {"x": 1}}});
    csv_blank_lines_outside_quotes_are_skipped: "d.csv", "\na\n\nx\n\ny\n", "{ a: string }[]"
        => json!([{"a": "x"}, {"a": "y"}]);
    csv_a_delimiter_only_line_is_a_row_of_empty_cells: "d.csv", "a,b\n,\n", "{ a: string, b: string }[]"
        => json!([{"a": "", "b": ""}]);
    csv_headers_and_cells_are_not_trimmed: "d.csv", "a , b\n x ,y\n", "{ <string>: string }[]"
        => json!([{"a ": " x ", " b": "y"}]);
    csv_a_quoted_newline_in_a_list_row_loads: "d.csv", "a,b\n\"x\ny\",z\n", "tuple([string, string])[]"
        => json!([["x\ny", "z"]]);

    // A byte-order mark is stripped for every format family.
    yaml_bom_is_stripped: "d.yaml", "\u{feff}- a\r\n- b\r\n", "string[]" => json!(["a", "b"]);
    json_bom_is_stripped: "d.json", "\u{feff}[\"a\"]", "string[]" => json!(["a"]);
    jsonl_bom_is_stripped: "d.jsonl", "\u{feff}\"a\"\n", "string[]" => json!(["a"]);
    csv_bom_is_stripped: "d.csv", "\u{feff}a\nx\n", "{ a: string }[]" => json!([{"a": "x"}]);
    tsv_bom_is_stripped: "d.tsv", "\u{feff}a\nx\n", "{ a: string }[]" => json!([{"a": "x"}]);
    toml_bom_is_stripped: "d.toml", "\u{feff}a = \"x\"\n", "{ a: string }" => json!({"a": "x"});
    json5_bom_is_stripped: "d.json5", "\u{feff}['a']", "string[]" => json!(["a"]);

    // CRLF line endings are accepted for every format family.
    yaml_crlf_loads: "d.yaml", "a: x\r\nb: y\r\n", "{ <string>: string }" => json!({"a": "x", "b": "y"});
    json_crlf_loads: "d.json", "[\r\n\"a\"\r\n]\r\n", "string[]" => json!(["a"]);
    jsonl_crlf_loads: "d.jsonl", "\"a\"\r\n\"b\"\r\n", "string[]" => json!(["a", "b"]);
    csv_crlf_loads: "d.csv", "a,b\r\n1,2\r\n", "{ a: string, b: string }[]" => json!([{"a": "1", "b": "2"}]);
    tsv_crlf_loads: "d.tsv", "a\tb\r\n1\t2\r\n", "{ a: string, b: string }[]" => json!([{"a": "1", "b": "2"}]);
    toml_crlf_loads: "d.toml", "a = 1\r\nb = 2\r\n", "{ <string>: number }" => json!({"a": 1, "b": 2});
    json5_crlf_loads: "d.json5", "[\r\n'a',\r\n]\r\n", "string[]" => json!(["a"]);

    // Empty input: YAML null, JSON Lines/CSV/TSV no rows, TOML an empty table.
    empty_yaml_is_null: "d.yaml", "", "string[] | null" => Value::Null;
    empty_jsonl_is_an_empty_list: "d.jsonl", "", "string[]" => json!([]);
    empty_csv_is_an_empty_list: "d.csv", "", "{ a: string }[]" => json!([]);
    empty_tsv_is_an_empty_list: "d.tsv", "", "{ a: string }[]" => json!([]);
    blank_lines_only_csv_is_an_empty_list: "d.csv", "\n\r\n\n", "{ a: string }[]" => json!([]);
    toml_floats_load: "d.toml", "a = 1.5\nb = -2.25\n", "{ <string>: number }" => json!({"a": 1.5, "b": -2.25});
    empty_toml_is_an_empty_table: "d.toml", "", "{ <string>: string }" => json!({});
}

#[test]
fn an_unknown_extension_is_unsupported() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "notes.txt", "- a\n");
    match &*load_failure(dir.path(), "notes.txt", "string[]") {
        DataError::UnsupportedFormat { extension, .. } => assert_eq!(extension.as_deref(), Some("txt")),
        other => panic!("{other}"),
    }
}

/// A malformed document's location: `(line, column)` when the parser proved
/// one, else `None`, which asserts that neither is reported.
macro_rules! rejects {
    ($($name:ident: $file:literal, $content:expr, $contract:literal => $at:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let dir = TempDir::new().unwrap();
                write(dir.path(), $file, $content);
                let location = malformed(dir.path(), $file, $contract);
                let expected: Option<(usize, usize)> = $at;
                assert_eq!(line_column(&location), expected.unzip(), "{location:?}");
                assert_eq!(location.byte_offset, None, "{location:?}");
            }
        )+
    };
}

rejects! {
    json_trailing_garbage_is_malformed: "d.json", "[1] x", "number[]" => Some((1, 5));
    json_reports_the_character_column_after_a_multibyte_character: "d.json", "{\"a\": \"é\", x}", "{ a: string }"
        => Some((1, 12));
    json_inexact_integer_is_malformed: "d.json", "[9007199254740993]", "number[]" => Some((1, 2));
    empty_json_is_malformed: "d.json", "", "string[]" => Some((1, 1));

    // The YAML parser locates a duplicate key at the start of the mapping
    // that holds it.
    yaml_duplicate_key_is_malformed: "d.yaml", "a: 1\na: 2\n", "{ <string>: number }" => Some((1, 1));
    yaml_nested_duplicate_key_is_located_at_its_mapping: "d.yaml", "x: 0\nm:\n  a: 1\n  a: 2\n", "{ <string>: unknown }"
        => Some((3, 3));
    // The YAML parser reports a second document without a position, and no
    // other source proves one.
    yaml_multiple_documents_are_malformed: "d.yaml", "- a\n---\n- b\n", "string[]" => None;
    yaml_custom_tag_is_malformed: "d.yaml", "- !foo a\n", "string[]" => Some((1, 3));
    yaml_invalid_syntax_is_malformed: "d.yaml", "a: [1\n", "{ a: number[] }" => Some((2, 1));
    yaml_inexact_integer_is_malformed: "d.yaml", "- 9007199254740993\n", "number[]" => Some((1, 3));
    // The parser accepts a non-string key; the shared loader refuses it
    // afterwards, and its position map cannot index such a mapping.
    yaml_non_string_key_is_malformed: "d.yaml", "a:\n  1: x\n", "{ <string>: unknown }" => None;
    // Aliases only make contract violations pointer-only; a parser error in
    // a file that holds one keeps the parser's coordinates.
    yaml_invalid_syntax_after_an_alias_keeps_its_parser_position: "d.yaml", "a: &x 1\nb: *x\nc: d: e\n",
        "{ <string>: unknown }" => Some((3, 5));
    yaml_unknown_alias_is_malformed: "d.yaml", "- a\n- *nope\n", "string[]" => Some((2, 3));

    toml_duplicate_key_is_malformed: "d.toml", "a = 1\na = 2\n", "{ a: number }" => Some((2, 1));
    toml_trailing_garbage_is_malformed: "d.toml", "a = 1 x\n", "{ a: number }" => Some((1, 5));
    toml_nan_is_malformed: "d.toml", "a = nan\n", "{ a: number }" => Some((1, 5));
    toml_infinity_is_malformed: "d.toml", "a = inf\n", "{ a: number }" => Some((1, 5));
    toml_inexact_integer_is_malformed: "d.toml", "a = 9007199254740993\n", "{ a: number }" => Some((1, 5));

    json5_invalid_text_is_malformed: "d.json5", "{a: ]}", "{ a: number }" => Some((1, 5));
    json5_trailing_text_is_malformed: "d.json5", "[1] x", "number[]" => Some((1, 5));
    json5_non_finite_is_malformed: "d.json5", "[Infinity]", "number[]" => Some((1, 2));
    json5_inexact_integer_is_malformed: "d.json5", "[9007199254740993]", "number[]" => Some((1, 2));
    empty_json5_is_malformed: "d.json5", "", "string[]" => Some((1, 1));
    json5_nan_is_malformed: "d.json5", "[1, NaN]", "number[]" => Some((1, 5));

    jsonl_malformed_line_reports_its_number: "d.jsonl", "1\n\n{x\n", "number[]" => Some((3, 2));
    jsonl_two_values_on_one_line_are_malformed: "d.jsonl", "1 2\n", "number[]" => Some((1, 3));
    jsonl_inexact_integer_is_malformed: "d.jsonl", "1\n9007199254740993\n", "number[]" => Some((2, 1));

    csv_unterminated_quote_is_malformed: "d.csv", "a\n\"x\n", "{ a: string }[]" => Some((2, 1));
    csv_quote_in_an_unquoted_field_is_malformed: "d.csv", "a\nx\"y\n", "{ a: string }[]" => Some((2, 2));
    csv_text_after_a_closing_quote_is_malformed: "d.csv", "a\n\"x\"y\n", "{ a: string }[]" => Some((2, 4));
    csv_ragged_row_reports_its_line: "d.csv", "a,b\n1,2\n3\n", "{ a: string, b: string }[]" => Some((3, 1));
    csv_long_row_reports_its_line: "d.csv", "a,b\n1,2\n3,4,5\n", "{ a: string, b: string }[]" => Some((3, 1));
    csv_ragged_record_with_a_quoted_newline_reports_its_start: "d.csv", "a,b\n1,2\n\"x\ny\"\n",
        "{ a: string, b: string }[]" => Some((3, 1));
    csv_ragged_list_row_reports_its_line: "d.csv", "a,b\n1\n", "tuple([string, string])[]" => Some((2, 1));
    tsv_unterminated_quote_is_malformed: "d.tsv", "a\tb\n1\t\"x\n", "{ a: string, b: string }[]" => Some((2, 3));
    tsv_text_after_a_closing_quote_is_malformed: "d.tsv", "a\tb\n\"x\"y\tz\n", "{ a: string, b: string }[]"
        => Some((2, 4));
}

#[test]
fn json_lines_report_the_failing_line() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.jsonl", "1\n\n{x\n");
    match &*load_failure(dir.path(), "d.jsonl", "number[]") {
        DataError::Malformed { message, .. } => assert!(message.contains("line 3"), "{message}"),
        other => panic!("{other}"),
    }
}

/// Local text that is not UTF-8 is located by its byte offset in the
/// original bytes (the byte-order mark counted), with no line or column.
macro_rules! rejects_invalid_utf8 {
    ($($name:ident: $file:literal, $contract:literal;)+) => {
        $(
            #[test]
            fn $name() {
                let dir = TempDir::new().unwrap();
                write(dir.path(), $file, b"\xEF\xBB\xBFab\xFFc\n");
                let location = malformed(dir.path(), $file, $contract);
                assert_eq!(location, DataSourceLocation { byte_offset: Some(5), ..DataSourceLocation::default() });
            }
        )+
    };
}

rejects_invalid_utf8! {
    yaml_invalid_utf8_reports_its_byte: "d.yaml", "string[]";
    json_invalid_utf8_reports_its_byte: "d.json", "string[]";
    jsonl_invalid_utf8_reports_its_byte: "d.jsonl", "string[]";
    csv_invalid_utf8_reports_its_byte: "d.csv", "{ a: string }[]";
    tsv_invalid_utf8_reports_its_byte: "d.tsv", "{ a: string }[]";
    toml_invalid_utf8_reports_its_byte: "d.toml", "{ a: string }";
    json5_invalid_utf8_reports_its_byte: "d.json5", "string[]";
}

#[test]
fn loaded_text_stays_data() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[\"{{ x }}\", \"$(cmd)\"]");
    assert_eq!(loaded(dir.path(), "d.json", "string[]"), json!(["{{ x }}", "$(cmd)"]));
}

#[test]
fn a_kind_mapping_selects_no_schema_and_reads_nothing_else() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.yaml", "kind: schema\n$schema: ./other.yaml\n");
    let (composed, report) =
        compose(dir.path(), "{{ as_json([load_data(\"d.yaml\", \"{ <string>: string }\")]) }}\n").unwrap();
    let value: Value = serde_json::from_str(composed.content().trim()).unwrap();
    assert_eq!(value, json!([{"kind": "schema", "$schema": "./other.yaml"}]));
    assert_eq!(report.read_set, vec![entry(dir.path(), "d.yaml")]);
}

// ── R4: tabular rows follow `as` ────────────────────────────────────────────

const HOSTS_CSV: &str = "name,port\nweb,8080\ndb,5432\n";

#[test]
fn object_rows_convert_a_numeric_port() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "hosts.csv", HOSTS_CSV);
    assert_eq!(
        loaded(dir.path(), "hosts.csv", "{ name: string, port: number }[]"),
        json!([{"name": "web", "port": 8080}, {"name": "db", "port": 5432}])
    );
}

#[test]
fn tuple_rows_skip_the_header() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "hosts.csv", HOSTS_CSV);
    assert_eq!(loaded(dir.path(), "hosts.csv", "tuple([string, number])[]"), json!([["web", 8080], ["db", 5432]]));
}

#[test]
fn an_object_header_must_not_repeat_a_name() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "\n\nname,port,name\nweb,1,x\n");
    match &*load_failure(dir.path(), "d.csv", "{ <string>: string }[]") {
        DataError::Malformed { message, location, .. } => {
            assert!(message.contains("column 3 repeats `name` from column 1"), "{message}");
            assert_eq!(line_column(location), (Some(3), Some(11)), "located on the header's own line");
        }
        other => panic!("{other}"),
    }
}

#[test]
fn an_object_header_must_name_every_column() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "a,,b\n1,2,3\n");
    match &*load_failure(dir.path(), "d.csv", "{ <string>: string }[]") {
        DataError::Malformed { message, location, .. } => {
            assert!(message.contains("column 2 has no name"), "{message}");
            assert_eq!(line_column(location), (Some(1), Some(3)));
        }
        other => panic!("{other}"),
    }
}

#[test]
fn list_rows_allow_repeated_and_empty_headers() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "a,a,\n1,2,3\n");
    assert_eq!(loaded(dir.path(), "d.csv", "tuple([string, string, string])[]"), json!([["1", "2", "3"]]));
}

#[test]
fn a_header_only_file_has_no_rows() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "name,port\n");
    assert_eq!(loaded(dir.path(), "d.csv", "{ name: string, port: number }[]"), json!([]));
}

/// The file is computed, so the passive check cannot see its extension and
/// the rule is applied at execution.
#[test]
fn a_scalar_row_element_is_a_row_shape_conflict_before_reading() {
    let dir = TempDir::new().unwrap();
    let runtime = DataRuntime::new();
    write(dir.path(), "d.csv", "a\n1\n");
    let failure =
        compose_with(dir.path(), "---\nf: d.csv\n---\n{{ load_data(f, \"string[]\") }}\n", &runtime).unwrap_err();
    assert_eq!(data_error(&failure).code(), Some("dm.data.row_shape_conflict"));
    assert_eq!(runtime.stats().reads, 0, "the row mode is chosen before the file is read");
}

/// A computed contract is judged at execution with the same code and
/// suggestion the passive check gives a literal one.
#[test]
fn a_computed_non_row_contract_is_a_row_shape_conflict_with_the_suggestion() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "a\n1\n");
    for contract in ["unknown[]", "object", "{ <string>: string }"] {
        let content = format!("---\nt: \"{contract}\"\n---\n{{{{ load_data(\"d.csv\", t) }}}}\n");
        let error = data_error(&failure_of(dir.path(), &content));
        assert_eq!(error.code(), Some("dm.data.row_shape_conflict"), "{contract}: {error}");
        assert!(error.to_string().contains(ROW_SHAPE_SUGGESTION), "{contract}: {error}");
    }
}

// The cell-conversion fixture: one row of the R4 table per assertion.

#[test]
fn number_cells_trim_whitespace_and_drop_separators() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "k,n\na,\" 42 \"\nb,1_000\n");
    assert_eq!(loaded(dir.path(), "d.csv", "{ k: string, n: number }[]"), json!([{"k": "a", "n": 42}, {"k": "b", "n": 1000}]));
}

#[test]
fn boolean_cells_read_yes_on_and_one_as_true() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "k,b\na,yes\nb,on\nc,1\n");
    assert_eq!(
        loaded(dir.path(), "d.csv", "{ k: string, b: boolean }[]"),
        json!([{"k": "a", "b": true}, {"k": "b", "b": true}, {"k": "c", "b": true}])
    );
}

#[test]
fn string_cells_are_kept_verbatim() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "k,s\na,\" 42 \"\nb,1_000\nc,yes\nd,\ne,\"1,000\"\nf,NaN\ng,0x10\nh,9007199254740993\n");
    let cells: Vec<Value> = loaded(dir.path(), "d.csv", "{ k: string, s: string }[]")
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["s"].clone())
        .collect();
    assert_eq!(cells, vec![json!(" 42 "), json!("1_000"), json!("yes"), json!(""), json!("1,000"), json!("NaN"), json!("0x10"), json!("9007199254740993")]);
}

#[test]
fn number_cells_reject_empty_grouped_nan_hex_and_inexact_text() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "k,n\na,\nb,\"1,000\"\nc,NaN\nd,0x10\ne,9007199254740993\n");
    let pointers: Vec<String> =
        mismatches(dir.path(), "d.csv", "{ k: string, n: number }[]").into_iter().map(|(pointer, _)| pointer).collect();
    assert_eq!(pointers, ["/0/n", "/1/n", "/2/n", "/3/n", "/4/n"]);
}

#[test]
fn an_empty_boolean_cell_is_a_mismatch_with_its_table() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.csv", "k,b\na,\n");
    match &*load_failure(dir.path(), "d.csv", "{ k: string, b: boolean }[]") {
        DataError::ContractMismatch { failure, table: Some(table), .. } => {
            let pointers: Vec<_> = failure.located_failures().into_iter().map(|located| located.pointer).collect();
            assert_eq!(pointers, ["/0/b"]);
            assert_eq!(table.mode, RowMode::Object);
            assert_eq!(table.data.rows[0].fields[1].offset, 6, "the cell keeps its raw source offset");
        }
        other => panic!("{other}"),
    }
}

// ── R5: the field validators, once through composition ─────────────────────

#[test]
fn a_required_member_that_is_absent_is_missing() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[{}]");
    assert_eq!(mismatches(dir.path(), "d.json", "{ name: string(required) }[]"), [("/0/name".to_string(), ActualKind::Missing)]);
}

#[test]
fn a_required_member_that_is_null_fails() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[{\"name\": null}]");
    assert_eq!(mismatches(dir.path(), "d.json", "{ name: string(required) }[]"), [("/0/name".to_string(), ActualKind::Null)]);
}

#[test]
fn an_optional_member_stays_absent_or_null() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[{}, {\"name\": null}]");
    assert_eq!(loaded(dir.path(), "d.json", "{ name: string }[]"), json!([{}, {"name": null}]));
}

#[test]
fn compatible_scalar_text_converts() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[\"8080\"]");
    assert_eq!(loaded(dir.path(), "d.json", "number[]"), json!([8080]));
}

#[test]
fn a_mixed_array_fails_at_the_bad_element_only() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[1, \"x\", 3]");
    assert_eq!(mismatches(dir.path(), "d.json", "number[]"), [("/1".to_string(), ActualKind::String)]);
}

#[test]
fn an_all_invalid_array_fails_at_every_element() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[\"x\", true]");
    assert_eq!(
        mismatches(dir.path(), "d.json", "number[]"),
        [("/0".to_string(), ActualKind::String), ("/1".to_string(), ActualKind::Boolean)]
    );
}

#[test]
fn a_whole_field_of_the_wrong_type_fails() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "{\"tags\": \"x\"}");
    assert_eq!(mismatches(dir.path(), "d.json", "{ tags: number[] }"), [("/tags".to_string(), ActualKind::String)]);
}

#[test]
fn a_missing_tuple_slot_is_missing() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[[\"a\"]]");
    assert_eq!(mismatches(dir.path(), "d.json", "tuple([string, number])[]"), [("/0/1".to_string(), ActualKind::Missing)]);
}

#[test]
fn a_null_array_element_fails() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[1, null]");
    assert_eq!(mismatches(dir.path(), "d.json", "number[]"), [("/1".to_string(), ActualKind::Null)]);
}

#[test]
fn an_optional_collection_field_admits_null() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "{\"tags\": null}");
    assert_eq!(loaded(dir.path(), "d.json", "{ tags: number[] }"), json!({"tags": null}));
}

#[test]
fn tuple_slots_fail_for_null_and_the_wrong_type_and_convert_text() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "good.json", "[[\"a\", \"3\"]]");
    write(dir.path(), "bad.json", "[[\"a\", null], [\"b\", \"x\"], [\"c\", \"3\"], [{}, []]]");
    assert_eq!(loaded(dir.path(), "good.json", "tuple([string, number])[]"), json!([["a", 3]]));
    assert_eq!(
        mismatches(dir.path(), "bad.json", "tuple([string, number])[]"),
        [
            ("/0/1".to_string(), ActualKind::Null),
            ("/1/1".to_string(), ActualKind::String),
            ("/3/0".to_string(), ActualKind::Object),
            ("/3/1".to_string(), ActualKind::Array),
        ]
    );
}

#[test]
fn a_tuple_row_of_the_wrong_type_fails_as_a_whole() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[\"a\"]");
    assert_eq!(mismatches(dir.path(), "d.json", "tuple([string])[]"), [("/0".to_string(), ActualKind::String)]);
}

#[test]
fn a_null_dictionary_value_fails() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "{\"a\": 1, \"b\": null}");
    assert_eq!(mismatches(dir.path(), "d.json", "{ <string>: number }"), [("/b".to_string(), ActualKind::Null)]);
}

#[test]
fn a_dictionary_value_converts_or_fails_by_key() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "good.json", "{\"a\": \"1\"}");
    write(dir.path(), "bad.json", "{\"a\": \"1\", \"b\": \"x\"}");
    assert_eq!(loaded(dir.path(), "good.json", "{ <string>: number }"), json!({"a": 1}));
    assert_eq!(mismatches(dir.path(), "bad.json", "{ <string>: number }"), [("/b".to_string(), ActualKind::String)]);
}

#[test]
fn empty_values_remain_explicit() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "{\"list\": [], \"map\": {}, \"text\": \"\"}");
    assert_eq!(
        loaded(dir.path(), "d.json", "{ list: string[], map: { <string>: string }, text: string }"),
        json!({"list": [], "map": {}, "text": ""})
    );
}

#[test]
fn a_contract_mismatch_publishes_no_value() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[1, \"x\"]");
    let failure = failure_of(dir.path(), "before {{ as_json(load_data(\"d.json\", \"number[]\")) }} after\n");
    assert_eq!(data_error(&failure).code(), Some("dm.data.contract_mismatch"));
}

// ── R6: remote reads ────────────────────────────────────────────────────────

#[test]
fn a_url_without_a_remote_runtime_is_not_fetched() {
    let dir = TempDir::new().unwrap();
    let failure = failure_of(dir.path(), "{{ load_data(\"https://example.invalid/data.json\", \"string[]\") }}\n");
    let diagnostic = file_reference(&failure);
    assert_eq!(diagnostic.kind, FileRefFailure::RemoteNotEnabled, "{}", failure.error);
    assert_eq!(failure.read_set, vec!["https://example.invalid/data.json".to_string()]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_allowed_url_with_a_query_and_fragment_loads_as_json_once() {
    use darkmatter::markdown::compose::remote::RemoteReadConfig;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/data.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[\"macOS\", \"Linux\"]"))
        .mount(&server)
        .await;
    let url = format!("{}/data.json?v=2#top", server.uri());
    let dir = TempDir::new().unwrap();
    let doc = write(
        dir.path(),
        "doc.md",
        format!("{{{{ as_csv(load_data(\"{url}\", \"string[]\")) }}}} / {{{{ length(load_data(\"{url}\", \"string[]\")) }}}}\n"),
    );
    let options = ComposeOptions::new()
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() });
    let (composed, report) = tokio::task::spawn_blocking(move || compose_file(&doc, options))
        .await
        .unwrap()
        .unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "macOS, Linux / 2");
    assert_eq!(report.read_set, vec![url.clone()]);
    let fetched = server.received_requests().await.unwrap_or_default();
    assert_eq!(fetched.len(), 1, "one fetch for two calls");
}

// ── R7: repeated reads within one command ───────────────────────────────────

const MAP_OVER_THREE: &str = "---\n$schema:\n  items: string[]\nitems: [a, b, c]\n---\n\
    ::map items \"i => i\"\n- {{ as_csv(load_data(\"os.yaml\", \"string[]\")) }}\n::end-map\n";

fn reads_after(dir: &Path, content: &str, options: ComposeOptions) -> darkmatter::markdown::compose::DataRuntimeStats {
    let runtime = DataRuntime::new();
    let path = write(dir, "doc.md", content);
    compose_file(&path, options.with_data_runtime(runtime.clone()))
        .unwrap_or_else(|failure| panic!("{content:?} failed: {}", failure.error));
    runtime.stats()
}

#[test]
fn a_map_over_three_items_reads_its_data_once() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    let stats = reads_after(dir.path(), MAP_OVER_THREE, ComposeOptions::new());
    assert_eq!((stats.reads, stats.parses, stats.binds), (1, 1, 1), "{stats:?}");
}

#[test]
fn a_map_callback_and_the_body_share_one_read() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    let content = "---\n$schema:\n  items: string[]\nitems: [a, b]\n---\n\
        ::map items \"i => length(load_data('os.yaml', 'string[]'))\"\n- item\n::end-map\n\n\
        {{ as_csv(load_data(\"os.yaml\", \"string[]\")) }}\n";
    let stats = reads_after(dir.path(), content, ComposeOptions::new());
    assert_eq!((stats.reads, stats.binds), (1, 1), "{stats:?}");
}

#[test]
fn two_contracts_read_once_and_bind_twice() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[\"1\", \"2\"]");
    let content = "{{ as_json(load_data(\"d.json\", \"string[]\")) }} {{ as_json(load_data(\"d.json\", \"number[]\")) }}\n";
    let runtime = DataRuntime::new();
    let (composed, _) = compose_with(dir.path(), content, &runtime).unwrap();
    assert_eq!(composed.content().trim(), "[\"1\",\"2\"] [1,2]");
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.binds), (1, 2), "{stats:?}");
}

#[test]
fn one_relative_import_in_two_folders_validates_separately() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "d.json", "[\"1\", \"2\"]");
    write(dir.path(), "a/types.yaml", "kind: schema\ntypes:\n  List: string[]\n");
    write(dir.path(), "b/types.yaml", "kind: schema\ntypes:\n  List: number[]\n");
    let content = "{{ as_json(load_data(\"../d.json\", \"List@./types.yaml\")) }}\n";
    let runtime = DataRuntime::new();
    let mut outputs = Vec::new();
    for folder in ["a", "b"] {
        let path = write(dir.path(), &format!("{folder}/doc.md"), content);
        let (composed, _) = compose_file(&path, ComposeOptions::new().with_data_runtime(runtime.clone()))
            .unwrap_or_else(|failure| panic!("{folder}: {}", failure.error));
        outputs.push(composed.content().trim().to_string());
    }
    assert_eq!(outputs, ["[\"1\",\"2\"]", "[1,2]"]);
    let stats = runtime.stats();
    assert_eq!((stats.reads, stats.binds), (1, 2), "{stats:?}");
    assert!(runtime.read_set().contains(&entry(dir.path(), "a/types.yaml")));
    assert!(runtime.read_set().contains(&entry(dir.path(), "b/types.yaml")));
}

macro_rules! reads_once_under {
    ($($name:ident: $mode:expr;)+) => {
        $(
            #[test]
            fn $name() {
                let dir = TempDir::new().unwrap();
                write(dir.path(), "os.yaml", OS_YAML);
                let stats = reads_after(dir.path(), MAP_OVER_THREE, ComposeOptions::new().with_cache_access_mode($mode));
                assert_eq!((stats.reads, stats.binds), (1, 1), "{stats:?}");
            }
        )+
    };
}

reads_once_under! {
    read_write_mode_reads_once: CacheAccessMode::ReadWrite;
    read_only_mode_reads_once: CacheAccessMode::ReadOnly;
    refresh_mode_reads_once: CacheAccessMode::Refresh;
    off_mode_reads_once: CacheAccessMode::Off;
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

/// `git config --file` rewrites one key of an INI-style file in place; the
/// result is valid TOML, and `git` is on every host these tests run on.
#[test]
fn a_frontmatter_command_that_rewrites_the_data_is_read_again() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "hosts.toml", "[a]\nb = 1\n");
    let content = "---\nbefore: \"{{ load_data('hosts.toml', '{ a: { b: number } }').a.b }}\"\n\
        made: $(git config --file hosts.toml a.b 2)\n---\n\
        {{ before }}-{{ load_data(\"hosts.toml\", \"{ a: { b: number } }\").a.b }}\n";
    let path = write(dir.path(), "doc.md", content);
    let runtime = DataRuntime::new();
    let options = ComposeOptions::new()
        .with_shell(ShellExpansionOptions { policy_root: Some(dir.path().to_path_buf()), ..Default::default() })
        .with_shell_working_directory(dir.path())
        .with_shell_approval_handler(Arc::new(AllowAll))
        .with_data_runtime(runtime.clone());
    let (composed, _) = compose_file(&path, options).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(composed.content().trim(), "1-2");
    assert_eq!(runtime.stats().reads, 2, "{:?}", runtime.stats());
}

#[test]
fn a_repeated_failure_fails_at_each_site() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "bad.json", "[1,");
    let runtime = DataRuntime::new();
    for site in ["first.md", "second.md"] {
        let path = write(dir.path(), site, "{{ load_data(\"bad.json\", \"number[]\") }}\n");
        let failure = compose_file(&path, ComposeOptions::new().with_data_runtime(runtime.clone())).unwrap_err();
        assert_eq!(data_error(&failure).code(), Some("dm.data.malformed"), "{site}");
    }
    assert_eq!(runtime.stats().reads, 1, "the second site reused the memoized failure");
}

// ── R8: the read set ────────────────────────────────────────────────────────

#[test]
fn the_read_set_holds_the_data_file_and_its_contract_import() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", OS_YAML);
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  List: string[]\n");
    let (_, report) = compose(dir.path(), "{{ as_csv(load_data(\"os.yaml\", \"List@./types.yaml\")) }}\n").unwrap();
    assert_eq!(report.read_set, vec![entry(dir.path(), "os.yaml"), entry(dir.path(), "types.yaml")]);
}

#[test]
fn a_load_that_fails_after_resolving_is_recorded() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "bad.yaml", "a: [1\n");
    let failure = failure_of(dir.path(), "{{ load_data(\"bad.yaml\", \"{ a: number[] }\") }}\n");
    assert_eq!(failure.read_set, vec![entry(dir.path(), "bad.yaml")]);
}

// ── R9: optional data files ─────────────────────────────────────────────────

const GUARDED: &str = "{{ as_json(file_exists(\"opt.yaml\") ? load_data(\"opt.yaml\", \"string[]\") : []) }}\n";

#[test]
fn the_guard_yields_an_empty_list_for_a_missing_file() {
    let dir = TempDir::new().unwrap();
    assert_eq!(rendered(dir.path(), GUARDED), "[]");
}

#[test]
fn the_guard_yields_the_data_of_a_present_file() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "opt.yaml", "- a\n");
    assert_eq!(rendered(dir.path(), GUARDED), "[\"a\"]");
}

#[test]
fn the_guard_does_not_hide_a_malformed_file() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "opt.yaml", "- [a\n");
    assert_eq!(data_error(&failure_of(dir.path(), GUARDED)).code(), Some("dm.data.malformed"));
}

#[test]
fn or_does_not_absorb_a_load_failure() {
    let dir = TempDir::new().unwrap();
    let failure = failure_of(dir.path(), "{{ as_json(load_data(\"missing.yaml\", \"string[]\") || []) }}\n");
    assert_eq!(file_reference(&failure).kind, FileRefFailure::NotFound);
}
