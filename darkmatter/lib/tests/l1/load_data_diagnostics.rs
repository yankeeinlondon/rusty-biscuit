//! `load_data` failures and read-side file failures as shared diagnostics
//! (`compose::report_diagnostics::error_diagnostics`).
//!
//! Each test composes a document in its own temporary directory and reads
//! the public records a failed composition reports: the `dm.data.*` code,
//! the call-site `source`, the `data_location` inside the data, the
//! structural `expected_type`/`actual_type`, and, for a file that could not be
//! read, the reference fields.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{DataLocation, Diagnostic, DiagnosticSource};
use darkmatter::markdown::compose::remote::RemoteReadConfig;
use darkmatter::markdown::compose::report_diagnostics::{CONTRACT_RECORD_CAP, error_diagnostics};
use darkmatter::markdown::compose::{ComposeFailure, ComposeOptions};
use tempfile::TempDir;

const MISMATCH: &str = "dm.data.contract_mismatch";
const EVALUATION_FAILURE: &str = "dm.expression.evaluation_failure";

// ── Fixtures ────────────────────────────────────────────────────────────────

fn write(dir: &Path, name: &str, content: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// The portable spelling `data_location.file` uses for `name` in `dir`.
fn entry(dir: &Path, name: &str) -> String {
    let canonical = biscuit_file::canonicalize_simplified(dir).expect("canonical dir");
    biscuit_file::to_portable_string(&canonical.join(name))
}

fn failure_with(path: &Path, options: ComposeOptions) -> Box<ComposeFailure> {
    let request = crate::request_support::request(options.with_source_file(path));
    match Markdown::try_from(path).expect("document loads").compose_with_warnings(&request) {
        Ok((composed, _)) => panic!("{} composed: {}", path.display(), composed.content()),
        Err(failure) => failure,
    }
}

/// The records of composing `doc` as `doc.md` in `dir`.
fn records(dir: &Path, doc: &str) -> Vec<Diagnostic> {
    let path = write(dir, "doc.md", doc);
    error_diagnostics(&failure_with(&path, ComposeOptions::new()).error)
}

/// The records of `load_data("<file>", "<contract>")` over `data`.
fn load(dir: &Path, file: &str, data: impl AsRef<[u8]>, contract: &str) -> Vec<Diagnostic> {
    write(dir, file, data);
    records(dir, &format!("x {{{{ load_data(\"{file}\", \"{contract}\") }}}}\n"))
}

/// Each record's `(pointer, line, column)`.
fn places(records: &[Diagnostic]) -> Vec<(String, Option<usize>, Option<usize>)> {
    records
        .iter()
        .map(|record| {
            let location = record.data_location.as_ref().unwrap_or_else(|| panic!("no data location: {record:?}"));
            (location.pointer.clone().expect("a contract record has a pointer"), location.line, location.column)
        })
        .collect()
}

/// Each record's `(pointer, expected_type, actual_type)`.
fn types(records: &[Diagnostic]) -> Vec<(String, String, String)> {
    records
        .iter()
        .map(|record| {
            let pointer = record.data_location.as_ref().and_then(|location| location.pointer.clone()).unwrap_or_default();
            (pointer, record.expected_type.clone().unwrap_or_default(), record.actual_type.clone().unwrap_or_default())
        })
        .collect()
}

fn place(pointer: &str, line: usize, column: usize) -> (String, Option<usize>, Option<usize>) {
    (pointer.to_string(), Some(line), Some(column))
}

fn typed(pointer: &str, expected: &str, actual: &str) -> (String, String, String) {
    (pointer.to_string(), expected.to_string(), actual.to_string())
}

fn only(records: Vec<Diagnostic>) -> Diagnostic {
    assert_eq!(records.len(), 1, "{records:#?}");
    records.into_iter().next().expect("one record")
}

fn assert_all_mismatches(records: &[Diagnostic]) {
    assert!(records.iter().all(|record| record.code == MISMATCH), "{records:#?}");
}

// ── The OS example ──────────────────────────────────────────────────────────

#[test]
fn the_os_example_reports_one_record_per_operating_system() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "os.yaml", "- macOS\n- Linux\n- Windows\n", "number[]");
    assert_all_mismatches(&found);
    assert_eq!(places(&found), vec![place("/0", 1, 3), place("/1", 2, 3), place("/2", 3, 3)]);
    let first = &found[0];
    assert_eq!(first.source, Some(DiagnosticSource::at(Some(dir.path().join("doc.md")), 1, 3)), "the call site");
    assert_eq!(
        first.data_location,
        Some(DataLocation {
            file: entry(dir.path(), "os.yaml"),
            pointer: Some("/0".into()),
            line: Some(1),
            column: Some(3),
            byte_offset: None
        })
    );
    assert_eq!((first.expected_type.as_deref(), first.actual_type.as_deref()), (Some("number"), Some("string")));
    assert!(first.message.contains("\"macOS\""), "{}", first.message);
    let json = serde_json::to_value(first).unwrap();
    assert!(json.get("reason").is_none(), "no reference fields: {json}");
}

// ── One fixture per format ──────────────────────────────────────────────────

macro_rules! locates {
    ($($name:ident: $file:literal, $data:expr, $contract:literal => $pointer:literal at $line:literal : $column:literal;)*) => {$(
        #[test]
        fn $name() {
            let dir = TempDir::new().unwrap();
            let found = load(dir.path(), $file, $data, $contract);
            assert_all_mismatches(&found);
            assert_eq!(places(&found), vec![place($pointer, $line, $column)], "{found:#?}");
            assert_eq!(found[0].data_location.as_ref().unwrap().file, entry(dir.path(), $file));
        }
    )*};
}

locates! {
    a_yaml_violation_is_located:
        "s.yaml", "servers:\n  - port: 1\n  - port: x\n", "{servers: {port: number}[]}" => "/servers/1/port" at 3:11;
    a_pretty_json_violation_is_located:
        "s.json", "{\n  \"servers\": [\n    {\"port\": 1},\n    {\"port\": \"x\"}\n  ]\n}\n", "{servers: {port: number}[]}"
        => "/servers/1/port" at 4:14;
    a_json_lines_violation_after_a_blank_line_is_located:
        "s.jsonl", "{\"port\": 1}\n\n{\"port\": \"x\"}\n", "{port: number}[]" => "/1/port" at 3:10;
    a_csv_violation_is_located:
        "s.csv", "name,port\na,1\nb,x\n", "{name: string, port: number}[]" => "/1/port" at 3:3;
    a_tsv_violation_is_located:
        "s.tsv", "name\tport\na\t1\nb\tx\n", "{name: string, port: number}[]" => "/1/port" at 3:3;
    a_toml_violation_inside_an_array_of_tables_is_located:
        "s.toml", "[[srv]]\nport = 1\n\n[[srv]]\nport = \"x\"\n", "{srv: {port: number}[]}" => "/srv/1/port" at 5:8;
    a_json5_violation_is_located:
        "s.json5", "{servers: [{port: 1}, {port: 'x'}]}\n", "{servers: {port: number}[]}" => "/servers/1/port" at 1:30;
    a_json_duplicate_key_is_located_at_its_last_occurrence:
        "d.json", "{\"port\": 1, \"port\": \"x\"}\n", "{port: number}" => "/port" at 1:21;
    a_csv_cell_after_a_quoted_delimiter_doubled_quote_and_newline_is_located_at_its_raw_start:
        "q.csv", "name,note,port\na,\"x, \"\"y\"\"\nz\",\"q\"\n", "{name: string, note: string, port: number}[]"
        => "/0/port" at 3:4;
    a_missing_csv_column_is_located_at_its_record:
        "r.csv", "name\na\n", "{name: string, port: number(required)}[]" => "/0/port" at 2:1;
    a_tsv_cell_after_a_quoted_tab_is_located_at_its_raw_start:
        "q.tsv", "a\tb\n\"x\ty\"\tz\n", "{a: string, b: number}[]" => "/0/b" at 2:7;
    a_missing_member_is_located_at_its_parent:
        "m.json", "[\n  {\"name\": \"a\"}\n]\n", "{name: string, port: number(required)}[]" => "/0/port" at 2:3;
}

// ── Pointer only ────────────────────────────────────────────────────────────

macro_rules! pointer_only {
    ($($name:ident: $data:literal, $contract:literal => [$($pointer:literal),*];)*) => {$(
        #[test]
        fn $name() {
            let dir = TempDir::new().unwrap();
            let found = load(dir.path(), "p.yaml", $data, $contract);
            assert_all_mismatches(&found);
            let expected: Vec<_> = [$($pointer),*].iter().map(|pointer| (pointer.to_string(), None, None)).collect();
            assert_eq!(places(&found), expected, "{found:#?}");
        }
    )*};
}

pointer_only! {
    a_yaml_alias_reports_pointers_only: "- &a x\n- *a\n", "number[]" => ["/0", "/1"];
    a_yaml_multi_line_flow_sequence_reports_pointers_only: "items: [\n  x,\n  1\n]\n", "{items: number[]}" => ["/items/0"];
}

#[test]
fn csv_row_records_after_a_multi_line_record_use_each_record_start() {
    let dir = TempDir::new().unwrap();
    let found =
        load(dir.path(), "r.csv", "name,note\na,\"x\ny\"\nb,z\n", "{name: string, note: string, port: number(required)}[]");
    assert_all_mismatches(&found);
    assert_eq!(places(&found), vec![place("/0/port", 2, 1), place("/1/port", 4, 1)]);
}

// ── Columns count characters ────────────────────────────────────────────────

macro_rules! same_column {
    ($($name:ident: $file:literal, $plain:literal, $other:literal, $contract:literal => $pointer:literal at $line:literal : $column:literal;)*) => {$(
        #[test]
        fn $name() {
            let plain = TempDir::new().unwrap();
            let other = TempDir::new().unwrap();
            let expected = vec![place($pointer, $line, $column)];
            assert_eq!(places(&load(plain.path(), $file, $plain, $contract)), expected);
            assert_eq!(places(&load(other.path(), $file, $other, $contract)), expected);
        }
    )*};
}

same_column! {
    a_multibyte_json_value_before_the_violation_keeps_the_ascii_column:
        "c.json", "{\"a\": \"e\", \"port\": \"x\"}", "{\"a\": \"é\", \"port\": \"x\"}", "{a: string, port: number}"
        => "/port" at 1:20;
    a_multibyte_yaml_value_before_the_violation_keeps_the_ascii_column:
        "c.yaml", "x: {a: e, b: q}\n", "x: {a: é, b: q}\n", "{x: {a: string, b: number}}" => "/x/b" at 1:14;
    a_crlf_yaml_file_keeps_the_lf_position:
        "c.yaml", "a: b\nport: x\n", "a: b\r\nport: x\r\n", "{a: string, port: number}" => "/port" at 2:7;
    a_crlf_csv_file_keeps_the_lf_position:
        "c.csv", "name,port\na,x\n", "name,port\r\na,x\r\n", "{name: string, port: number}[]" => "/0/port" at 2:3;
}

// ── Records and their types ─────────────────────────────────────────────────

#[test]
fn an_undeclared_csv_column_is_one_absent_record_per_row_and_a_converting_cell_is_none() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "s.csv", "name,port,note\na,80,x\nb,81,y\n", "{ name: string, port: number }[]");
    assert_eq!(types(&found), vec![typed("/0/note", "absent", "string"), typed("/1/note", "absent", "string")]);
    assert_eq!(places(&found), vec![place("/0/note", 2, 6), place("/1/note", 3, 6)]);
}

#[test]
fn twenty_five_bad_rows_report_the_first_twenty_and_count_the_rest() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "rows.csv", format!("port\n{}", "x\n".repeat(25)), "{port: number}[]");
    assert_all_mismatches(&found);
    assert_eq!(found.len(), CONTRACT_RECORD_CAP + 1);
    let pointers: Vec<_> = places(&found[..CONTRACT_RECORD_CAP]).into_iter().map(|(pointer, ..)| pointer).collect();
    assert_eq!(pointers, (0..20).map(|row| format!("/{row}/port")).collect::<Vec<_>>());
    let overflow = &found[CONTRACT_RECORD_CAP];
    assert_eq!((overflow.data_location.as_ref(), overflow.expected_type.as_ref(), overflow.actual_type.as_ref()), (None, None, None));
    assert_eq!(overflow.source, found[0].source, "the call site");
    assert!(overflow.message.contains("5 more"), "{}", overflow.message);
}

#[test]
fn a_nullable_list_never_names_its_null_arm() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "n.json", "[\"macOS\"]", "number[] | null");
    assert_eq!(types(&found), vec![typed("/0", "number", "string")]);
}

#[test]
fn two_same_kind_container_arms_report_one_record_at_the_root() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "u.json", "[{\"c\": 1}]", "{a: string}[] | {b: number}[]");
    assert_eq!(types(&found), vec![typed("", "{a: string}[] | {b: number}[]", "array")]);
    assert_eq!(places(&found), vec![place("", 1, 1)]);
}

#[test]
fn a_root_mismatch_names_the_contract_as_written() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "r.json", "{\"a\": 1}", "number[]");
    assert_eq!(types(&found), vec![typed("", "number[]", "object")]);
    assert_eq!(places(&found), vec![place("", 1, 1)]);
}

#[test]
fn a_nested_union_with_no_reachable_arm_reports_one_record_without_its_null_arm() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "u.yaml", "a: x\n", "{a: number[] | {b: string} | null}");
    // Each remaining arm in the engine's words; the `null` inside is `b`'s
    // own optionality, not the union's dropped null arm.
    assert_eq!(types(&found), vec![typed("/a", "number[] | { b?: null | string, no other keys }", "string")]);
    assert_eq!(places(&found), vec![place("/a", 1, 4)]);
}

#[test]
fn the_only_reachable_union_arm_is_descended_into() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "u.yaml", "a: [1]\n", "string[] | {a: string}");
    assert_eq!(types(&found), vec![typed("/a", "string", "array")]);
}

#[test]
fn a_missing_tuple_slot_is_missing_and_an_extra_slot_or_key_is_absent() {
    let dir = TempDir::new().unwrap();
    let slots = load(dir.path(), "t.json", "[[\"c\"], [\"d\", \"4\", \"x\"]]", "tuple([string, number])[]");
    assert_eq!(types(&slots), vec![typed("/0/1", "number", "missing"), typed("/1/2", "absent", "string")]);
    let keys = load(dir.path(), "k.json", "[{\"a\": \"x\", \"z\": 1}]", "{a: string}[]");
    assert_eq!(types(&keys), vec![typed("/0/z", "absent", "number")]);
    assert_eq!(places(&keys), vec![place("/0/z", 1, 18)], "an unexpected key is located at its value");
}

#[test]
fn a_missing_member_follows_the_present_members_of_its_parent() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "m.json", "[{\"b\": \"x\"}]", "{a: string(required), b: number}[]");
    assert_eq!(types(&found), vec![typed("/0/b", "number", "string"), typed("/0/a", "string", "missing")]);
}

macro_rules! authored_order {
    ($($name:ident: $file:literal, $data:literal;)*) => {$(
        #[test]
        fn $name() {
            let dir = TempDir::new().unwrap();
            let found = load(dir.path(), $file, $data, "{alpha: number, zeta: number}");
            let pointers: Vec<_> = types(&found).into_iter().map(|(pointer, ..)| pointer).collect();
            assert_eq!(pointers, ["/zeta", "/alpha"]);
        }
    )*};
}

authored_order! {
    json_fields_written_in_reverse_alphabetical_order_keep_it: "o.json", "{\"zeta\": \"x\", \"alpha\": \"y\"}";
    toml_fields_written_in_reverse_alphabetical_order_keep_it: "o.toml", "zeta = \"x\"\nalpha = \"y\"\n";
    yaml_fields_written_in_reverse_alphabetical_order_keep_it: "o.yaml", "zeta: x\nalpha: y\n";
}

macro_rules! escaped_keys {
    ($($name:ident: $file:literal, $data:literal;)*) => {$(
        #[test]
        fn $name() {
            let dir = TempDir::new().unwrap();
            let found = places(&load(dir.path(), $file, $data, "{<string>: number}"));
            let pointers: Vec<_> = found.iter().map(|(pointer, line, _)| (pointer.as_str(), *line)).collect();
            assert_eq!(pointers, [("/a~1b", Some(1)), ("/c~0d", Some(2)), ("/", Some(3))]);
            assert!(found.iter().all(|(_, _, column)| column.is_some()), "{found:?}");
        }
    )*};
}

escaped_keys! {
    json_keys_with_a_slash_a_tilde_and_no_text_round_trip: "e.json", "{\"a/b\": \"x\",\n\"c~d\": \"y\",\n\"\": \"z\"}";
    yaml_keys_with_a_slash_a_tilde_and_no_text_round_trip: "e.yaml", "\"a/b\": x\n\"c~d\": y\n\"\": z\n";
}

#[test]
fn an_escaped_key_is_located_at_its_own_value() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "e.json", "{\"a/b\": \"x\",\n\"c~d\": \"y\",\n\"\": \"z\"}", "{<string>: number}");
    assert_eq!(places(&found), vec![place("/a~1b", 1, 9), place("/c~0d", 2, 8), place("/", 3, 5)]);
}

#[test]
fn a_union_message_keeps_six_lines_of_arm_detail_and_an_ellipsis() {
    let dir = TempDir::new().unwrap();
    let found = load(dir.path(), "u.json", "[{\"z\": 1}]", "{a: number}[] | {b: number}[] | {c: number}[] | {d: number}[]");
    let record = only(found);
    let lines: Vec<&str> = record.message.lines().collect();
    assert_eq!(lines.len(), 1 + 6 + 1, "{}", record.message);
    assert_eq!(lines.last().map(|line| line.trim()), Some("…"));
}

// ── Every data code ─────────────────────────────────────────────────────────

/// A document whose contract is computed, so only execution can judge it.
fn computed(dir: &Path, file: &str, contract: &str) -> Diagnostic {
    only(records(dir, &format!("---\ncontract: \"{contract}\"\n---\nx {{{{ load_data(\"{file}\", contract) }}}}\n")))
}

#[test]
fn a_computed_scalar_contract_reports_not_a_container() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.yaml", "[]\n");
    let record = computed(dir.path(), "a.yaml", "string");
    assert_eq!(record.code, "dm.data.not_a_container");
    assert_eq!(record.data_location, None);
}

#[test]
fn a_computed_this_import_reports_this_import_rejected() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.yaml", "[]\n");
    assert_eq!(computed(dir.path(), "a.yaml", "Os@this").code, "dm.data.this_import_rejected");
}

#[test]
fn a_computed_contract_with_two_row_shapes_reports_row_shape_conflict() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.csv", "a\nx\n");
    let record = computed(dir.path(), "a.csv", "tuple([string])[] | {a: string}[]");
    assert_eq!(record.code, "dm.data.row_shape_conflict");
    assert_eq!(record.data_location, None);
}

#[test]
fn an_unknown_extension_reports_unsupported_format_at_runtime() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.txt", "x\n");
    let record = computed(dir.path(), "a.txt", "string[]");
    assert_eq!(record.code, "dm.data.unsupported_format");
    assert_eq!(record.data_location, None);
}

#[test]
fn literal_contract_problems_report_their_data_codes_before_anything_runs() {
    let dir = TempDir::new().unwrap();
    for (contract, code) in [("string", "dm.data.not_a_container"), ("Os@this", "dm.data.this_import_rejected")] {
        let record = only(records(dir.path(), &format!("x {{{{ load_data(\"a.yaml\", \"{contract}\") }}}}\n")));
        assert_eq!(record.code, code, "{contract}");
        assert_eq!(record.data_location, None);
    }
    let record = only(records(dir.path(), "x {{ load_data(\"a.csv\", \"tuple([string])[] | {a: string}[]\") }}\n"));
    assert_eq!(record.code, "dm.data.row_shape_conflict");
}

#[test]
fn malformed_json_reports_its_parser_position_without_a_pointer() {
    let dir = TempDir::new().unwrap();
    let record = only(load(dir.path(), "m.json", "[1,\n  x]\n", "number[]"));
    assert_eq!(record.code, "dm.data.malformed");
    assert_eq!(
        record.data_location,
        Some(DataLocation { file: entry(dir.path(), "m.json"), pointer: None, line: Some(2), column: Some(3), byte_offset: None })
    );
    assert_eq!((record.expected_type, record.actual_type), (None, None));
}

#[test]
fn malformed_yaml_with_an_alias_reports_its_parser_position_without_a_pointer() {
    let dir = TempDir::new().unwrap();
    let record = only(load(dir.path(), "m.yaml", "a: &x 1\nb: *x\nc: d: e\n", "{<string>: unknown}"));
    assert_eq!(record.code, "dm.data.malformed");
    assert_eq!(
        record.data_location,
        Some(DataLocation { file: entry(dir.path(), "m.yaml"), pointer: None, line: Some(3), column: Some(5), byte_offset: None })
    );
}

#[test]
fn invalid_utf8_reports_its_file_and_byte_offset_only() {
    let dir = TempDir::new().unwrap();
    let record = only(load(dir.path(), "b.json", b"[\"a\", \xff]".as_slice(), "string[]"));
    assert_eq!(record.code, "dm.data.malformed");
    assert_eq!(
        record.data_location,
        Some(DataLocation { file: entry(dir.path(), "b.json"), pointer: None, line: None, column: None, byte_offset: Some(6) })
    );
}

#[test]
fn an_invalid_contract_schema_stays_an_evaluation_failure() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "a.yaml", "[]\n");
    let record = computed(dir.path(), "a.yaml", "List@./missing-types.yaml");
    assert_eq!(record.code, EVALUATION_FAILURE);
    assert_eq!(record.data_location, None);
}

// ── Provenance through other surfaces ───────────────────────────────────────

#[test]
fn a_violation_inside_a_map_callback_keeps_its_iteration_path_and_data_location() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", "- macOS\n");
    let found = records(dir.path(), "x {{ map([1, 2], n => load_data(\"os.yaml\", \"number[]\")) }}\n");
    let record = only(found);
    assert_eq!(record.code, MISMATCH);
    assert!(!record.iteration_path.is_empty(), "{record:#?}");
    assert_eq!(places(std::slice::from_ref(&record)), vec![place("/0", 1, 3)]);
    assert_eq!(record.source.as_ref().and_then(|source| source.line), Some(1));
}

#[test]
fn a_violation_in_a_block_condition_reports_its_data_code() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", "- macOS\n");
    let found = records(dir.path(), "::block when=\"length(load_data('os.yaml', 'number[]')) > 0\"\nyes\n::end-block\n");
    assert_eq!(places(&found), vec![place("/0", 1, 3)]);
    assert_all_mismatches(&found);
}

#[test]
fn a_violation_in_frontmatter_interpolation_reports_its_data_code() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", "- macOS\n");
    let found = records(dir.path(), "---\nlist: \"{{ load_data('os.yaml', 'number[]') }}\"\n---\nbody\n");
    assert_eq!(places(&found), vec![place("/0", 1, 3)]);
    assert_all_mismatches(&found);
}

#[test]
fn a_violation_inside_nested_composition_reports_its_data_code_at_the_call() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "os.yaml", "- macOS\n");
    let found = records(dir.path(), "x {{ as_markdown(\"{{ load_data('os.yaml', 'number[]') }}\") }}\n");
    assert_all_mismatches(&found);
    assert_eq!(places(&found), vec![place("/0", 1, 3)]);
    assert_eq!(found[0].source.as_ref().and_then(|source| source.line), Some(1));
}

// ── Remote data ─────────────────────────────────────────────────────────────

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_url_loaded_violation_reports_the_url_pointer_line_and_column() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/data.json"))
        .respond_with(ResponseTemplate::new(200).set_body_string("[\n  1,\n  \"x\"\n]\n"))
        .mount(&server)
        .await;
    let url = format!("{}/data.json", server.uri());
    let dir = TempDir::new().unwrap();
    let doc = write(dir.path(), "doc.md", format!("x {{{{ load_data(\"{url}\", \"number[]\") }}}}\n"));
    let options = ComposeOptions::new()
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() });
    let failure = tokio::task::spawn_blocking(move || failure_with(&doc, options)).await.unwrap();
    let record = only(error_diagnostics(&failure.error));
    assert_eq!(record.code, MISMATCH);
    assert_eq!(
        record.data_location,
        Some(DataLocation { file: url, pointer: Some("/1".into()), line: Some(3), column: Some(3), byte_offset: None })
    );
}

// ── File-reference failures of read-side functions ──────────────────────────

/// `(reason, construct, reference, failure)` of `record`.
fn reference_fields(record: &Diagnostic) -> (Option<&str>, Option<&str>, Option<&str>, Option<&str>) {
    let details = record.reference_details.as_ref().unwrap_or_else(|| panic!("no reference fields: {record:#?}"));
    (details.reason.as_deref(), details.construct.as_deref(), details.reference.as_deref(), details.failure.as_deref())
}

macro_rules! missing_file {
    ($($name:ident: $call:literal => $construct:literal;)*) => {$(
        #[test]
        fn $name() {
            let dir = TempDir::new().unwrap();
            let record = only(records(dir.path(), concat!("x {{ ", $call, " }}\n")));
            assert_eq!(record.code, EVALUATION_FAILURE);
            assert_eq!(reference_fields(&record), (Some("file not found"), Some($construct), Some("missing.md"), Some("no-match")));
            assert!(record.reference_details.as_ref().unwrap().included_via.is_none());
            // The message is the one composition reported before these fields
            // existed, byte for byte.
            assert_eq!(record.message, concat!("interpolation of `", $call, "` failed: invalid file path: missing.md"));
            assert_eq!(record.data_location, None);
        }
    )*};
}

missing_file! {
    frontmatter_on_a_missing_file_names_frontmatter: "frontmatter(\"missing.md\")" => "frontmatter()";
    markdown_title_on_a_missing_file_names_markdown_title: "markdown_title(\"missing.md\")" => "markdown_title()";
    markdown_body_empty_on_a_missing_file_names_markdown_body_empty:
        "markdown_body_empty(\"missing.md\")" => "markdown_body_empty()";
    validate_schema_on_a_missing_file_names_validate_schema: "validate_schema(\"missing.md\")" => "validate_schema()";
}

#[test]
fn load_data_on_a_missing_file_reports_the_reference_fields() {
    let dir = TempDir::new().unwrap();
    let record = only(records(dir.path(), "x {{ load_data(\"missing.yaml\", \"string[]\") }}\n"));
    assert_eq!(record.code, EVALUATION_FAILURE);
    assert_eq!(reference_fields(&record), (Some("file not found"), Some("load_data()"), Some("missing.yaml"), Some("no-match")));
    assert_eq!(record.data_location, None);
}

/// A run that allows only `example.com`, so `127.0.0.1` is refused before any
/// request is made.
fn denied_host_options() -> ComposeOptions {
    ComposeOptions::new()
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["example.com".into()], ..Default::default() })
}

macro_rules! denied_host {
    ($($name:ident: $call:literal => $construct:literal;)*) => {$(
        #[test]
        fn $name() {
            let dir = TempDir::new().unwrap();
            let call = format!($call, url = "http://127.0.0.1:9/data.json");
            let doc = write(dir.path(), "doc.md", format!("x {{{{ {call} }}}}\n"));
            let record = only(error_diagnostics(&failure_with(&doc, denied_host_options()).error));
            assert_eq!(record.code, EVALUATION_FAILURE);
            assert_eq!(
                reference_fields(&record),
                (Some("remote check disabled"), Some($construct), Some("http://127.0.0.1:9/data.json"), Some("unsupported-remote"))
            );
            let refused = format!("{}: remote read denied for host '127.0.0.1'", $construct);
            assert_eq!(record.message, format!("interpolation of `{call}` failed: {refused}"));
            assert_eq!(record.data_location, None);
        }
    )*};
}

denied_host! {
    load_data_from_a_denied_host_reports_the_reference_fields: "load_data(\"{url}\", \"string[]\")" => "load_data()";
    frontmatter_from_a_denied_host_reports_the_reference_fields: "frontmatter(\"{url}\")" => "frontmatter()";
    file_exists_on_a_denied_host_reports_the_reference_fields: "file_exists(\"{url}\")" => "file_exists()";
}

// ── Failed reads from an allowed host ───────────────────────────────────────

/// A loopback server whose `/fail.*` routes answer HTTP 500 and whose
/// `/bad.*` routes answer 200 with a body that is not UTF-8.
async fn failing_server() -> wiremock::MockServer {
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/fail\."))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/bad\."))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![b'a', 0xff, b'b']))
        .mount(&server)
        .await;
    server
}

/// Composes `x {{ <call> }}` with `127.0.0.1` allowed, where `{url}` in
/// `call` is `route` on `server`. Returns the call as written and the
/// failure's records.
async fn allowed_host_records(server: &wiremock::MockServer, call: &str, route: &str) -> (String, Vec<Diagnostic>) {
    let call = call.replace("{url}", &format!("{}{route}", server.uri()));
    let dir = TempDir::new().unwrap();
    let doc = write(dir.path(), "doc.md", format!("x {{{{ {call} }}}}\n"));
    let options = ComposeOptions::new()
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() });
    let failure = tokio::task::spawn_blocking(move || failure_with(&doc, options)).await.unwrap();
    (call, error_diagnostics(&failure.error))
}

/// Asserts the one record of a failed allowed-host read: the evaluation
/// code, the runtime's wording, the four reference fields, and no data
/// location.
fn assert_failed_fetch(call: &str, records: Vec<Diagnostic>, construct: &str, url: &str, detail: &str) {
    let record = only(records);
    assert_eq!(record.code, EVALUATION_FAILURE, "{record:#?}");
    assert_eq!(reference_fields(&record), (Some("unreachable"), Some(construct), Some(url), Some("io")));
    assert!(record.reference_details.as_ref().unwrap().included_via.is_none());
    let detail = detail.replace("{url}", url);
    assert_eq!(record.message, format!("interpolation of `{call}` failed: {construct}: {detail}"));
    assert_eq!(record.data_location, None);
}

/// The runtime's wording of each failure; `{url}` stands for the URL read.
const HTTP_500: &str = "HTTP error status 500 from {url}";
const NOT_UTF8: &str = "invalid UTF-8 in response body: invalid utf-8 sequence of 1 bytes from index 1";

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn load_data_from_an_allowed_host_that_fails_reports_the_reference_fields() {
    let server = failing_server().await;
    for (route, detail) in [("/fail.json", HTTP_500), ("/bad.json", NOT_UTF8)] {
        let (call, records) = allowed_host_records(&server, "load_data(\"{url}\", \"object\")", route).await;
        let url = format!("{}{route}", server.uri());
        assert_failed_fetch(&call, records, "load_data()", &url, detail);
    }
}

macro_rules! failed_fetch {
    ($($name:ident: $call:literal, $route:literal, $detail:expr => $construct:literal;)*) => {$(
        #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
        async fn $name() {
            let server = failing_server().await;
            let (call, records) = allowed_host_records(&server, $call, $route).await;
            assert_failed_fetch(&call, records, $construct, &format!("{}{}", server.uri(), $route), $detail);
        }
    )*};
}

failed_fetch! {
    frontmatter_from_an_allowed_host_that_fails_reports_the_reference_fields:
        "frontmatter(\"{url}\")", "/fail.md", HTTP_500 => "frontmatter()";
    file_exists_on_an_allowed_host_with_a_non_utf8_body_reports_the_reference_fields:
        "file_exists(\"{url}\")", "/bad.md", NOT_UTF8 => "file_exists()";
    markdown_title_from_an_allowed_host_that_fails_reports_the_reference_fields:
        "markdown_title(\"{url}\")", "/fail.md", HTTP_500 => "markdown_title()";
    markdown_body_empty_on_an_allowed_host_with_a_non_utf8_body_reports_the_reference_fields:
        "markdown_body_empty(\"{url}\")", "/bad.md", NOT_UTF8 => "markdown_body_empty()";
    validate_schema_from_an_allowed_host_that_fails_reports_the_reference_fields:
        "validate_schema(\"{url}\")", "/fail.md", HTTP_500 => "validate_schema()";
}

/// `try_frontmatter` turns the same failures into its `{ ok: false }` value,
/// so the document composes and nothing is reported.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn try_frontmatter_on_an_allowed_host_that_fails_still_recovers() {
    let server = failing_server().await;
    for (route, detail) in [("/fail.md", HTTP_500), ("/bad.md", NOT_UTF8)] {
        let url = format!("{}{route}", server.uri());
        let dir = TempDir::new().unwrap();
        let doc = write(
            dir.path(),
            "doc.md",
            format!("{{{{ try_frontmatter(\"{url}\").ok }}}}|{{{{ try_frontmatter(\"{url}\").error }}}}\n"),
        );
        let options = ComposeOptions::new()
            .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() });
        let composed = tokio::task::spawn_blocking(move || {
            let request = crate::request_support::request(options.with_source_file(&doc));
            Markdown::try_from(doc.as_path()).expect("document loads").compose_with_warnings(&request)
        })
        .await
        .unwrap()
        .unwrap_or_else(|failure| panic!("{route}: {}", failure.error));
        let detail = detail.replace("{url}", &url);
        assert_eq!(composed.0.content().trim(), format!("false|try_frontmatter(): {detail}"));
        assert!(composed.1.warnings.is_empty(), "{:#?}", composed.1.warnings);
    }
}

/// The `(pointer, line, column)` of the one `dm.data.malformed` record that
/// loading `data` as `data.yaml` reports.
fn malformed_yaml_place(data: &str) -> (Option<String>, Option<usize>, Option<usize>) {
    let dir = TempDir::new().unwrap();
    let record = only(load(dir.path(), "data.yaml", data, "object"));
    assert_eq!(record.code, "dm.data.malformed", "{data:?}: {record:?}");
    let location = record.data_location.expect("a data location");
    assert_eq!(location.file, entry(dir.path(), "data.yaml"));
    (location.pointer, location.line, location.column)
}

#[test]
fn a_second_yaml_document_is_located_at_its_marker() {
    assert_eq!(malformed_yaml_place("a: ok\n---\na: next\n"), (None, Some(2), Some(1)));
    assert_eq!(malformed_yaml_place("\u{feff}a: ok\r\n---\r\na: next\r\n"), (None, Some(2), Some(1)));
}

#[test]
fn a_non_string_yaml_key_is_located_at_the_key_even_after_an_alias() {
    for (data, line, column) in [
        ("42: text\n", 1, 1),
        ("a:\n  b: ok\n  7: x\n", 3, 3),
        ("a:\n  true: x\n", 2, 3),
        ("? [a, b]\n: x\n", 1, 3),
        ("base: &b ok\na: *b\n42: text\n", 3, 1),
    ] {
        assert_eq!(malformed_yaml_place(data), (None, Some(line), Some(column)), "{data:?}");
    }
}

#[test]
fn a_yaml_tag_is_located_at_the_tag_even_after_an_alias() {
    let place = |pointer: &str, line, column| (Some(pointer.to_string()), Some(line), Some(column));
    assert_eq!(malformed_yaml_place("base: &b ok\na: *b\nb: !tag text\n"), place("/b", 3, 4));
    // Controls without an alias keep their positions.
    assert_eq!(malformed_yaml_place("a: [\n  !t x\n]\n"), place("/a/0", 2, 3));
    assert_eq!(malformed_yaml_place("!t\na: 1\n"), place("", 1, 1));
}

/// With no remote runtime at all (a standalone condition), `file_exists`
/// fails on a URL with the same typed `RemoteNotEnabled` file-reference cause
/// as the other read-side functions, so it is never an untyped failure.
#[test]
fn file_exists_without_a_remote_runtime_fails_like_its_siblings() {
    use darkmatter::markdown::compose::conditions::{ConditionError, evaluate_condition_against};
    use darkmatter::markdown::compose::expression::{ExpressionError, FileRefFailure};

    let dir = TempDir::new().unwrap();
    let context = biscuit_file::FileResolutionContext::new(dir.path());
    let url = "https://127.0.0.1:9/never-fetched.md";
    for function in ["file_exists", "frontmatter"] {
        let error = evaluate_condition_against(&format!("{function}(\"{url}\")"), &serde_json::json!({}), &context)
            .expect_err("no remote runtime");
        let ConditionError::Eval { cause, .. } = error else { panic!("{function}: {error:?}") };
        let ExpressionError::FileReference(diagnostic) = *cause else { panic!("{function}: {cause:?}") };
        assert_eq!((diagnostic.function, diagnostic.kind), (function, FileRefFailure::RemoteNotEnabled));
        assert_eq!(diagnostic.reference, url);
    }
}

/// A local Markdown file that exists but is not UTF-8 text keeps its typed
/// `io` cause through every Markdown-reading function: the reference fields
/// name the function and the file, with no reason phrase (none names an I/O
/// failure) and no data location.
#[test]
fn a_local_markdown_file_that_is_not_utf8_reports_the_io_reference_fields() {
    for (call, construct) in [
        ("frontmatter(\"bad.md\")", "frontmatter()"),
        ("markdown_title(\"bad.md\")", "markdown_title()"),
        ("markdown_body_empty(\"bad.md\")", "markdown_body_empty()"),
        ("validate_schema(\"bad.md\")", "validate_schema()"),
    ] {
        let dir = TempDir::new().unwrap();
        write(dir.path(), "bad.md", [b'a', 0xff, b'b']);
        let record = only(records(dir.path(), &format!("x {{{{ {call} }}}}\n")));
        assert_eq!(record.code, EVALUATION_FAILURE, "{call}");
        assert_eq!(reference_fields(&record), (None, Some(construct), Some("bad.md"), Some("io")), "{call}");
        assert_eq!(record.data_location, None, "{call}");
    }
}

// ── Schema files `validate_schema` cannot read ──────────────────────────────

/// The one record of composing `validate_schema("subject.md")` in `dir`,
/// where `subject.md` declares `schema` (a YAML `$schema` value).
fn validate_subject(dir: &Path, schema: &str) -> Diagnostic {
    write(dir, "subject.md", format!("---\n$schema: {schema}\nname: x\n---\nbody\n"));
    only(records(dir, "x {{ validate_schema(\"subject.md\") }}\n"))
}

/// The message `validate_schema` reported for a schema failure before the
/// reference fields existed: the `Other` wording, byte for byte.
fn validate_schema_message(cause: &str) -> String {
    format!(
        "interpolation of `validate_schema(\"subject.md\")` failed: validate_schema(): validate_schema() error for \"subject.md\": {cause}"
    )
}

#[test]
fn validate_schema_with_a_missing_schema_target_reports_the_reference_fields() {
    let dir = TempDir::new().unwrap();
    let record = validate_subject(dir.path(), "./missing-schema.yaml");
    assert_eq!(record.code, EVALUATION_FAILURE, "{record:#?}");
    assert_eq!(
        reference_fields(&record),
        (Some("file not found"), Some("validate_schema()"), Some("./missing-schema.yaml"), Some("no-match"))
    );
    assert_eq!(record.message, validate_schema_message("could not resolve $schema reference `./missing-schema.yaml`"));
    assert_eq!(record.data_location, None);
}

#[test]
fn validate_schema_with_a_missing_import_reports_the_reference_fields() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "schema.yaml", "$schema:\n  name: \"Name@./missing-types.yaml\"\n");
    let record = validate_subject(dir.path(), "./schema.yaml");
    assert_eq!(record.code, EVALUATION_FAILURE, "{record:#?}");
    assert_eq!(
        reference_fields(&record),
        (Some("file not found"), Some("validate_schema()"), Some("./missing-types.yaml"), Some("no-match"))
    );
    assert_eq!(record.message, validate_schema_message("could not resolve $schema reference `./missing-types.yaml`"));
}

/// An invalid schema that reads fine is not a read failure: it keeps the
/// `Other` record, with no reference fields.
#[test]
fn validate_schema_with_an_invalid_readable_schema_has_no_reference_fields() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "schema.yaml", "$schema:\n  name: \"not-a-type\"\n");
    let record = validate_subject(dir.path(), "./schema.yaml");
    assert_eq!(record.code, EVALUATION_FAILURE, "{record:#?}");
    assert!(record.reference_details.is_none(), "{record:#?}");
    assert!(record.message.starts_with(&validate_schema_message("referenced schema `")), "{}", record.message);
    assert!(record.message.contains("is invalid: name: unknown named type `not-a-type`"), "{}", record.message);
}

/// A remote `$schema` is never fetched, even from an allowed host: nothing is
/// read, so the refusal keeps the `Other` record with no reference fields.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn validate_schema_with_a_remote_schema_target_reads_nothing_and_has_no_reference_fields() {
    let server = failing_server().await;
    let url = format!("{}/fail.yaml", server.uri());
    let dir = TempDir::new().unwrap();
    write(dir.path(), "subject.md", format!("---\n$schema: {url}\nname: x\n---\nbody\n"));
    let doc = write(dir.path(), "doc.md", "x {{ validate_schema(\"subject.md\") }}\n");
    let options = ComposeOptions::new()
        .with_remote_read_config(RemoteReadConfig { allowed_hosts: vec!["127.0.0.1".into()], ..Default::default() });
    let failure = tokio::task::spawn_blocking(move || failure_with(&doc, options)).await.unwrap();
    let record = only(error_diagnostics(&failure.error));
    assert_eq!(record.code, EVALUATION_FAILURE, "{record:#?}");
    assert!(record.reference_details.is_none(), "{record:#?}");
    assert_eq!(
        record.message,
        validate_schema_message(&format!("remote $schema references are not supported in this version: `{url}`"))
    );
    assert!(server.received_requests().await.unwrap_or_default().is_empty());
}
