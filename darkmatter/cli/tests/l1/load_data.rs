//! `load_data(file, as)` through the real `md` binary: one success and one
//! malformed-input route per loader family, a contract failure that stops the
//! document before a later action runs, YAML alias and multi-line flow
//! locations, the Markdown-mode report, absent
//! optional diagnostic fields, lifecycle reads, the `md graph` and
//! `md validate refs` envelopes, the file-reference fields of a missing
//! data file, and the options argument (a headerless read and a refused
//! option).
//!
//! Broader read-set and contract-mismatch conformance lives in
//! `output_envelope.rs`; nothing here repeats it. Each test spawns `md` once
//! and spells the schema file it validates against, because CI maps an
//! `include_str!` to the test that names it.

use crate::common;

use common::CliProcessFixture;
use serde_json::{Value, json};
use simplified_schema::SimplifiedSchema;
use std::path::Path;
use std::process::Output;
use test_toolkit::argv_recorder::ARGV_RECORDER_FILE;

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The one JSON value on stdout, after requiring an empty stderr and `status`.
#[track_caller]
fn json_envelope(output: &Output, status: i32) -> Value {
    assert!(output.stderr.is_empty(), "{}", stderr_of(output));
    let text = String::from_utf8_lossy(&output.stdout);
    let mut stream = serde_json::Deserializer::from_str(&text).into_iter::<Value>();
    let envelope = stream.next().expect("a value").unwrap_or_else(|error| panic!("{error}: {text}"));
    assert!(stream.next().is_none(), "one value only: {text}");
    assert_eq!(output.status.code(), Some(status), "{envelope:#}");
    envelope
}

#[track_caller]
fn json_run(fixture: &CliProcessFixture, args: &[&str], status: i32) -> Value {
    let output = fixture.command().args(args).args(["--output", "json"]).output().expect("md runs");
    json_envelope(&output, status)
}

#[track_caller]
fn validate_strictly(schema_text: &str, value: &Value) {
    let schema = SimplifiedSchema::try_from(schema_text).expect("the schema file parses");
    if let Err(failure) = schema.validate_strict(value) {
        panic!("{failure:?}\n{value:#}");
    }
}

/// How the read set and `data_location.file` name a file that exists.
fn read_entry(path: &Path) -> String {
    biscuit_file::to_portable_string(&biscuit_file::canonicalize_simplified(path).expect("the file exists"))
}

/// The only diagnostic, after requiring there is exactly one.
#[track_caller]
fn only_record(envelope: &Value) -> &Value {
    let records = envelope["diagnostics"].as_array().expect("diagnostics");
    assert_eq!(records.len(), 1, "{envelope:#}");
    &records[0]
}

#[track_caller]
fn assert_failed(envelope: &Value) {
    assert_eq!(envelope["outcome"], "failure", "{envelope:#}");
    assert_eq!(envelope["document"], Value::Null, "{envelope:#}");
}

// ── 1. One success and one malformed input per loader family ───────────────

/// Composes `{{ <expression> }}` beside `data` written as `file`, and requires
/// the document `expected` and a read set naming only that file.
#[track_caller]
fn assert_loads(schema_text: &str, name: &str, file: &str, data: &str, expression: &str, expected: &str) {
    let fixture = CliProcessFixture::named(name);
    let data_path = fixture.write_file(format!("cwd/{file}"), data);
    fixture.write_file("cwd/doc.md", &format!("{{{{ {expression} }}}}\n"));

    let envelope = json_run(&fixture, &["compose", "doc.md"], 0);
    validate_strictly(schema_text, &envelope);
    assert_eq!(envelope["outcome"], "success", "{envelope:#}");
    assert_eq!(envelope["document"].as_str().map(str::trim_end), Some(expected), "{envelope:#}");
    assert_eq!(envelope["diagnostics"], json!([]), "{envelope:#}");
    assert_eq!(envelope["read_set"], json!([read_entry(&data_path)]), "{envelope:#}");
}

/// Composes `load_data(file, contract)` over malformed `data` and requires one
/// `dm.data.malformed` record at the parser-proven `line` and `column`, with
/// no pointer, and the file in the read set.
#[track_caller]
fn assert_malformed(schema_text: &str, name: &str, file: &str, data: &str, contract: &str, line: u64, column: u64) {
    let fixture = CliProcessFixture::named(name);
    let data_path = fixture.write_file(format!("cwd/{file}"), data);
    fixture.write_file("cwd/doc.md", &format!("{{{{ load_data(\"{file}\", \"{contract}\") }}}}\n"));

    let envelope = json_run(&fixture, &["compose", "doc.md"], 1);
    validate_strictly(schema_text, &envelope);
    assert_failed(&envelope);
    let record = only_record(&envelope);
    assert_eq!(record["code"], "dm.data.malformed", "{record:#}");
    assert_eq!(record["data_location"], json!({ "file": read_entry(&data_path), "line": line, "column": column }), "{record:#}");
    assert_eq!(envelope["read_set"], json!([read_entry(&data_path)]), "{envelope:#}");
}

#[test]
fn yaml_loads() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    assert_loads(schema, "load_data_yaml_ok", "data.yaml", "- a\n- b\n", "load_data(\"data.yaml\", \"string[]\")[1]", "b");
}

/// The flow sequence opened on line 2 is still open at the end of the file.
#[test]
fn yaml_malformed() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    assert_malformed(schema, "load_data_yaml_bad", "data.yaml", "a: 1\nb: [1\n", "unknown[]", 3, 1);
}

#[test]
fn json_loads() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    let expression = "load_data(\"data.json\", \"{ n: string }[]\")[1].n";
    assert_loads(schema, "load_data_json_ok", "data.json", "[{\"n\": \"a\"}, {\"n\": \"b\"}]\n", expression, "b");
}

/// The value missing after `"b":` is the `}` at line 2, column 8.
#[test]
fn json_malformed() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    assert_malformed(schema, "load_data_json_bad", "data.json", "{\"a\": 1,\n  \"b\": }\n", "unknown[]", 2, 8);
}

#[test]
fn jsonl_loads() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    let expression = "load_data(\"data.jsonl\", \"{ n: string }[]\")[1].n";
    assert_loads(schema, "load_data_jsonl_ok", "data.jsonl", "{\"n\": \"a\"}\n{\"n\": \"b\"}\n", expression, "b");
}

/// The unquoted key on line 2 starts at column 2.
#[test]
fn jsonl_malformed() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    assert_malformed(schema, "load_data_jsonl_bad", "data.jsonl", "{\"a\": 1}\n{bad}\n", "unknown[]", 2, 2);
}

#[test]
fn csv_loads() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    let expression = "load_data(\"data.csv\", \"{ <string>: string }[]\")[0].port";
    assert_loads(schema, "load_data_csv_ok", "data.csv", "name,port\nweb,80\n", expression, "80");
}

/// CSV and TSV share one loader; TSV proves the malformed route. The quote
/// that never closes opens at line 2, column 3.
#[test]
fn tsv_malformed() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    assert_malformed(schema, "load_data_tsv_bad", "data.tsv", "a\tb\n1\t\"x\n", "{ <string>: string }[]", 2, 3);
}

#[test]
fn toml_loads() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    let expression = "load_data(\"data.toml\", \"{ server: { name: string } }\").server.name";
    assert_loads(schema, "load_data_toml_ok", "data.toml", "[server]\nname = \"web\"\n", expression, "web");
}

/// `b = ` has no value; the parser stops at the line end, column 5.
#[test]
fn toml_malformed() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    assert_malformed(schema, "load_data_toml_bad", "data.toml", "a = 1\nb = \n", "{ a: number }", 2, 5);
}

#[test]
fn json5_loads() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    let expression = "load_data(\"data.json5\", \"{ n: string }\").n";
    assert_loads(schema, "load_data_json5_ok", "data.json5", "{n: 'a', // a comment\n}\n", expression, "a");
}

/// The `}` where a value belongs is at line 2, column 6.
#[test]
fn json5_malformed() {
    let schema = include_str!("../../schemas/output/compose.yaml");
    assert_malformed(schema, "load_data_json5_bad", "data.json5", "{a: 1,\n  b: }\n", "{ a: number }", 2, 6);
}

// ── 2. A contract failure stops the document before a later action ─────────

/// A document whose `load_data` call reads `data` and whose next block is an
/// approved `::shell` command recording to `sentinel` when it runs.
fn sentinel_fixture(name: &str, data: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    std::fs::copy(
        biscuit_test_harness::bin_exe!("darkmatter-argv-recorder"),
        fixture.bin_dir().join(format!("argv-recorder{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the recorder fixture binary is built with `test-fixtures`");
    fixture.write_file("cwd/.darkmatter-shell-whitelist", "exact argv-recorder sentinel\n");
    fixture.write_file("cwd/ports.json", data);
    fixture.write_file("cwd/doc.md", "Ports: {{ load_data(\"ports.json\", \"number[]\") }}\n\n::shell argv-recorder sentinel\n");
    fixture
}

fn run_sentinel(fixture: &CliProcessFixture, status: i32) -> Value {
    let output = fixture
        .command()
        .env(ARGV_RECORDER_FILE, fixture.cwd().join("sentinel"))
        .args(["compose", "doc.md", "--output", "json"])
        .output()
        .expect("md runs");
    json_envelope(&output, status)
}

/// The control for [`a_contract_mismatch_runs_no_later_action`]: with
/// conforming data the same document's command runs and leaves its file.
#[test]
fn conforming_data_lets_the_later_action_run() {
    let fixture = sentinel_fixture("load_data_sentinel_control", "[1, 2]\n");
    let envelope = run_sentinel(&fixture, 0);
    assert_eq!(envelope["outcome"], "success", "{envelope:#}");
    assert!(fixture.cwd().join("sentinel").exists(), "the command ran: {envelope:#}");
}

/// `[1, true, 3]` against `number[]`: the record points at `/1`, line 1,
/// column 5, and the `::shell` after the call never runs.
#[test]
fn a_contract_mismatch_runs_no_later_action() {
    let fixture = sentinel_fixture("load_data_sentinel", "[1, true, 3]\n");
    let envelope = run_sentinel(&fixture, 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_failed(&envelope);
    let record = only_record(&envelope);
    assert_eq!(record["code"], "dm.data.contract_mismatch", "{record:#}");
    let data = fixture.cwd().join("ports.json");
    assert_eq!(
        record["data_location"],
        json!({ "file": read_entry(&data), "pointer": "/1", "line": 1, "column": 5 }),
        "{record:#}"
    );
    assert_eq!(envelope["read_set"], json!([read_entry(&data)]), "{envelope:#}");
    assert!(!fixture.cwd().join("sentinel").exists(), "the later command must not run: {envelope:#}");
}

// ── 2a. YAML aliases and multi-line flow collections are located ─────────

const ALIAS_CONTRACT: &str = "{ base: { port: number }, one: { port: number } }";

/// A document loading `data.yaml` against `contract`, then an approved
/// `::shell` command recording to `sentinel` when it runs.
fn yaml_sentinel_fixture(name: &str, data: &str, contract: &str) -> CliProcessFixture {
    let fixture = sentinel_fixture(name, "[]\n");
    fixture.write_file("cwd/data.yaml", data);
    fixture.write_file(
        "cwd/doc.md",
        &format!("Data: {{{{ load_data(\"data.yaml\", \"{contract}\") }}}}\n\n::shell argv-recorder sentinel\n"),
    );
    fixture
}

#[test]
fn conforming_aliased_yaml_lets_the_later_action_run() {
    let fixture = yaml_sentinel_fixture("load_data_alias_control", "base: &b {port: 1}\none: *b\n", ALIAS_CONTRACT);
    let envelope = run_sentinel(&fixture, 0);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_eq!(envelope["outcome"], "success", "{envelope:#}");
    assert!(fixture.cwd().join("sentinel").exists(), "the command ran: {envelope:#}");
}

/// Each use of a failing anchor is its own record at its `*name` token, the
/// call-site `source` is the one a plain file reports, and nothing later runs.
#[test]
fn an_aliased_yaml_mismatch_is_located_at_each_alias_and_runs_no_later_action() {
    let fixture = yaml_sentinel_fixture("load_data_alias", "base: &b {port: x}\none: *b\n", ALIAS_CONTRACT);
    let envelope = run_sentinel(&fixture, 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_failed(&envelope);
    let data = read_entry(&fixture.cwd().join("data.yaml"));
    let records = envelope["diagnostics"].as_array().expect("diagnostics");
    assert!(records.iter().all(|record| record["code"] == "dm.data.contract_mismatch"), "{envelope:#}");
    let locations: Vec<&Value> = records.iter().map(|record| &record["data_location"]).collect();
    assert_eq!(
        locations,
        [
            &json!({ "file": data, "pointer": "/base/port", "line": 1, "column": 17 }),
            &json!({ "file": data, "pointer": "/one/port", "line": 2, "column": 6 }),
        ],
        "{envelope:#}"
    );
    assert_eq!(envelope["read_set"], json!([data]), "{envelope:#}");
    assert!(!fixture.cwd().join("sentinel").exists(), "the later command must not run: {envelope:#}");

    fixture.write_file("cwd/data.yaml", "base: {port: x}\none: {port: 1}\n");
    let plain = run_sentinel(&fixture, 1);
    assert_eq!(only_record(&plain)["source"], records[0]["source"], "{plain:#}");
}

#[test]
fn a_multi_line_flow_sequence_is_located() {
    let fixture = yaml_sentinel_fixture("load_data_multi_line_flow", "items: [\n  1,\n  x\n]\n", "{ items: number[] }");
    let envelope = run_sentinel(&fixture, 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_failed(&envelope);
    let data = read_entry(&fixture.cwd().join("data.yaml"));
    assert_eq!(
        only_record(&envelope)["data_location"],
        json!({ "file": data, "pointer": "/items/1", "line": 3, "column": 3 }),
        "{envelope:#}"
    );
}

// ── 3. Markdown mode ───────────────────────────────────────────────────────

/// The default output reports the failure on stderr and prints no document.
/// The human report is the expression-failure block, which names the data
/// file and the offending element but not the diagnostic code.
#[test]
fn a_contract_mismatch_in_markdown_mode_is_reported_on_stderr() {
    let fixture = CliProcessFixture::named("load_data_markdown_mode");
    let data = fixture.write_file("cwd/ports.json", "[1, \"x\"]\n");
    fixture.write_file("cwd/doc.md", "Ports: {{ load_data(\"ports.json\", \"number[]\") }}\n");

    let output = fixture.command().args(["compose", "doc.md"]).output().expect("md runs");
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty(), "no document: {}", String::from_utf8_lossy(&output.stdout));
    // The data source is named by its portable spelling on every OS.
    for expected in [
        "interpolation failed".to_owned(),
        format!("`{}` does not match `number[]`", read_entry(&data)),
        "[1]: expected number".to_owned(),
        "Expression at line: 1, column: 8".to_owned(),
    ] {
        assert!(stderr.contains(&expected), "{expected:?} in: {stderr}");
    }
}

// ── 4. Optional diagnostic fields ──────────────────────────────────────────

/// Neither a pointer nor (for invalid UTF-8) a line and column is invented:
/// those `data_location` fields are absent, not `null`, and the envelope still
/// conforms. Invalid UTF-8 carries the byte offset alone.
#[test]
fn invalid_utf8_reports_only_a_byte_offset() {
    let fixture = CliProcessFixture::named("load_data_invalid_utf8");
    let data = fixture.cwd().join("data.json");
    std::fs::write(&data, b"[\xff]\n").expect("write the data file");
    fixture.write_file("cwd/doc.md", "{{ load_data(\"data.json\", \"unknown[]\") }}\n");

    let envelope = json_run(&fixture, &["compose", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_failed(&envelope);
    let record = only_record(&envelope);
    assert_eq!(record["code"], "dm.data.malformed", "{record:#}");
    assert_eq!(record["data_location"], json!({ "file": read_entry(&data), "byte_offset": 1 }), "{record:#}");
    for absent in ["reason", "construct", "reference", "failure", "included_via"] {
        assert!(record.get(absent).is_none(), "{absent} is omitted: {record:#}");
    }
    assert_eq!((&record["expected_type"], &record["actual_type"]), (&Value::Null, &Value::Null), "{record:#}");
}

// ── 5. Lifecycle reads ─────────────────────────────────────────────────────

/// A `composed` stage whose condition probes `flag.txt` and whose message
/// loads `data_file`.
fn lifecycle_document(data_file: &str) -> String {
    format!(
        "---\ncomposed:\n  - when: \"file_exists('flag.txt')\"\n    then:\n      - info: flagged\n  - info: \"second {{{{ load_data('{data_file}', 'string[]')[1] }}}}\"\n---\nthe-document\n"
    )
}

#[test]
fn lifecycle_reads_are_in_the_read_set_on_success() {
    let fixture = CliProcessFixture::named("load_data_lifecycle_ok");
    let data = fixture.write_file("cwd/ok.yaml", "- a\n- b\n");
    fixture.write_file("cwd/doc.md", &lifecycle_document("ok.yaml"));

    let envelope = json_run(&fixture, &["compose", "doc.md"], 0);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_eq!(envelope["document"], "the-document", "{envelope:#}");
    assert_eq!(envelope["lifecycle_output"], json!([{ "stage": "composed", "step": "info", "text": "second b" }]), "{envelope:#}");
    // `flag.txt` does not exist: the read set names the probed candidate.
    let probed = format!("{}/flag.txt", read_entry(fixture.cwd()));
    assert_eq!(envelope["read_set"], json!([probed, read_entry(&data)]), "{envelope:#}");
}

/// A lifecycle expression has no authored position, so `source.line` and
/// `source.column` are `null` while `data_location` still points into the data.
#[test]
fn lifecycle_reads_are_in_the_read_set_on_failure() {
    let fixture = CliProcessFixture::named("load_data_lifecycle_bad");
    let data = fixture.write_file("cwd/bad.json", "{\"a\":\n");
    fixture.write_file("cwd/doc.md", &lifecycle_document("bad.json"));

    let envelope = json_run(&fixture, &["compose", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_failed(&envelope);
    let record = only_record(&envelope);
    assert_eq!(record["code"], "dm.data.malformed", "{record:#}");
    assert_eq!((&record["source"]["line"], &record["source"]["column"]), (&Value::Null, &Value::Null), "{record:#}");
    assert_eq!(record["data_location"], json!({ "file": read_entry(&data), "line": 2, "column": 1 }), "{record:#}");
    let probed = format!("{}/flag.txt", read_entry(fixture.cwd()));
    assert_eq!(envelope["read_set"], json!([read_entry(&data), probed]), "{envelope:#}");
}

// ── 6. `md graph` and `md validate refs` ───────────────────────────────────

/// A document loading a JSON file that ends inside an object.
fn malformed_fixture(name: &str) -> (CliProcessFixture, std::path::PathBuf) {
    let fixture = CliProcessFixture::named(name);
    let data = fixture.write_file("cwd/bad.json", "{\"a\": 1,\n");
    fixture.write_file("cwd/doc.md", "{{ load_data(\"bad.json\", \"unknown[]\") }}\n");
    (fixture, data)
}

#[track_caller]
fn assert_malformed_record(envelope: &Value, data: &Path) {
    assert_eq!(envelope["outcome"], "failure", "{envelope:#}");
    let record = only_record(envelope);
    assert_eq!(record["code"], "dm.data.malformed", "{record:#}");
    assert_eq!(record["data_location"], json!({ "file": read_entry(data), "line": 2, "column": 1 }), "{record:#}");
}

#[test]
fn graph_reports_a_malformed_data_file_in_its_schema() {
    let (fixture, data) = malformed_fixture("load_data_graph_malformed");
    let envelope = json_run(&fixture, &["graph", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/graph.yaml"), &envelope);
    assert_malformed_record(&envelope, &data);
}

#[test]
fn validate_refs_reports_a_malformed_data_file_in_its_schema() {
    let (fixture, data) = malformed_fixture("load_data_validate_refs_malformed");
    let envelope = json_run(&fixture, &["validate", "refs", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/validate-refs.yaml"), &envelope);
    assert_malformed_record(&envelope, &data);
}

// ── 7. File-reference fields of a missing data file ────────────────────────

/// A missing data file is an expression failure carrying the four reference
/// fields, and no `data_location`.
#[test]
fn a_missing_data_file_reports_its_reference_details() {
    let fixture = CliProcessFixture::named("load_data_missing_file");
    fixture.write_file("cwd/doc.md", "{{ load_data(\"gone.json\", \"unknown[]\") }}\n");

    let envelope = json_run(&fixture, &["compose", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_failed(&envelope);
    let record = only_record(&envelope);
    assert_eq!(record["code"], "dm.expression.evaluation_failure", "{record:#}");
    assert_eq!(record["reason"], "file not found", "{record:#}");
    assert_eq!(record["construct"], "load_data()", "{record:#}");
    assert_eq!(record["reference"], "gone.json", "{record:#}");
    assert_eq!(record["failure"], "no-match", "{record:#}");
    assert!(record.get("data_location").is_none(), "{record:#}");
    assert!(envelope["read_set"].is_array(), "{envelope:#}");
}

// ── 8. A transcluded child's malformed data ────────────────────────────────

/// The reference check prepares the transcluded child and fails on its data
/// before composition starts; the record names the child as its source.
#[test]
fn a_transcluded_childs_malformed_data_fails_the_reference_check() {
    let fixture = CliProcessFixture::named("load_data_transcluded_child");
    let data = fixture.write_file("cwd/bad.json", "{\"a\": 1,\n");
    let child = fixture.write_file("cwd/child.md", "{{ load_data(\"bad.json\", \"unknown[]\") }}\n");
    fixture.write_file("cwd/doc.md", "# Root\n\n::file ./child.md\n");

    let envelope = json_run(&fixture, &["compose", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_failed(&envelope);
    assert_eq!(envelope["summary"], Value::Null, "the check stopped: {envelope:#}");
    let record = only_record(&envelope);
    assert_eq!(record["code"], "dm.data.malformed", "{record:#}");
    // `source.file` is a native path, unlike the portable `data_location.file`.
    let child_path = biscuit_file::canonicalize_simplified(&child).expect("the file exists");
    assert_eq!(record["source"]["file"], child_path.to_string_lossy().as_ref(), "{record:#}");
    assert_eq!(record["data_location"], json!({ "file": read_entry(&data), "line": 2, "column": 1 }), "{record:#}");
    assert_eq!(envelope["read_set"], json!([read_entry(&data)]), "{envelope:#}");
}

// ── 9. The options argument ────────────────────────────────────────────────

/// A headerless CSV read through `{header: false}` yields every record as a
/// positional row, and the file is in the read set.
#[test]
fn compose_headerless_rows_json() {
    let fixture = CliProcessFixture::named("load_data_headerless_rows");
    let data = fixture.write_file("cwd/rows.csv", "web,8080\ndb,5432\n");
    fixture.write_file("cwd/doc.md", "Ports: {{ load_data(\"rows.csv\", \"tuple([string, number])[]\", {header: false}) }}\n");

    let envelope = json_run(&fixture, &["compose", "doc.md"], 0);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_eq!(envelope["outcome"], "success", "{envelope:#}");
    assert_eq!(envelope["document"].as_str().map(str::trim_end), Some("Ports: [[\"web\",8080],[\"db\",5432]]"), "{envelope:#}");
    assert_eq!(envelope["diagnostics"], json!([]), "{envelope:#}");
    assert_eq!(envelope["read_set"], json!([read_entry(&data)]), "{envelope:#}");
}

/// A literal invalid delimiter is refused by the call check before anything
/// runs: one `dm.data.invalid_options` record located at the `{` of the
/// options argument (column 55), no `data_location`, and an empty read set.
/// `docs/cli/data-contracts.md` ("Option errors") shows this envelope.
#[test]
fn compose_invalid_option_json_envelope() {
    let fixture = CliProcessFixture::named("load_data_invalid_option");
    fixture.write_file("cwd/rows.csv", "web,8080\ndb,5432\n");
    fixture.write_file("cwd/doc.md", "{{ load_data(\"rows.csv\", \"tuple([string, number])[]\", {delimiter: \",,\"}) }}\n");

    let envelope = json_run(&fixture, &["compose", "doc.md"], 1);
    validate_strictly(include_str!("../../schemas/output/compose.yaml"), &envelope);
    assert_failed(&envelope);
    let record = only_record(&envelope);
    assert_eq!(record["code"], "dm.data.invalid_options", "{record:#}");
    assert_eq!(record["severity"], "error", "{record:#}");
    assert_eq!(
        record["message"],
        "load_data(): option `delimiter` is invalid: must be one ASCII character other than a double quote, CR, or LF (write a tab as \"\\t\")",
        "{record:#}"
    );
    assert_eq!((&record["source"]["line"], &record["source"]["column"]), (&json!(1), &json!(55)), "{record:#}");
    assert_eq!(record["source"]["inserted"], false, "{record:#}");
    assert_eq!(
        record["expected_type"],
        "{format: enum(yaml,yml,json,jsonl,ndjson,csv,tsv,toml,json5),delimiter: string,header: boolean} | null",
        "{record:#}"
    );
    assert_eq!(record["actual_type"], "literal {\"delimiter\":\",,\"}", "{record:#}");
    assert!(record.get("data_location").is_none(), "no data file was read: {record:#}");
    assert_eq!(envelope["summary"], Value::Null, "{envelope:#}");
    assert_eq!(envelope["read_set"], json!([]), "the call check ran nothing: {envelope:#}");
}
