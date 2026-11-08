//! A failed render style keeps every warning the run had already found.
//!
//! The document's own `style:` has an unknown key (`magin`) and a deprecated
//! one (`page.max_width`), and its repository holds an invalid schema trigger.
//! Whatever then fails (a `--style` file, a setter, the schema selection, or a
//! selective `--deny`), the run exits 1 with those records beside its failure,
//! at their authored positions.

use std::process::Output;

use crate::common::CliProcessFixture;
use serde_json::Value;

const DOCUMENT: &str = "---\nstyle:\n  magin: 4\n  page:\n    max_width: 50\n---\n# Notes\n";

const TRIGGER: &str = "dm.schema.invalid_trigger";
const UNKNOWN: &str = "dm.style.unknown_key";
const DEPRECATED: &str = "dm.style.deprecated";

/// A repository holding `doc.md` (`document`) and the invalid trigger.
fn fixture(name: &str, document: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/.git/HEAD", "ref: refs/heads/main\n");
    fixture.write_file("cwd/.git/config", "[core]\n\trepositoryformatversion = 0\n");
    std::fs::create_dir_all(fixture.cwd().join(".git/objects")).expect("objects");
    std::fs::create_dir_all(fixture.cwd().join(".git/refs")).expect("refs");
    fixture.write_file("cwd/schemas/broken.trigger.yaml", "kind: schema-trigger\n");
    fixture.write_file("cwd/doc.md", document);
    fixture
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command_builder().application_input("MD_DRY_RUN", "fail").build().args(args).output().expect("md runs")
}

/// The one failure value of `md doc.md <args> --output json`.
fn failure(fixture: &CliProcessFixture, args: &[&str]) -> Value {
    let output = run(fixture, &[&["doc.md"][..], args, &["--output", "json"][..]].concat());
    assert!(output.stderr.is_empty(), "a data output owns stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(output.status.code(), Some(1), "{}", String::from_utf8_lossy(&output.stdout));
    let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON value");
    assert_eq!(value["outcome"], "failure", "{value:#}");
    value
}

/// The one record with `code` and `severity`.
fn record<'v>(value: &'v Value, code: &str, severity: &str) -> &'v Value {
    let records = value["diagnostics"].as_array().expect("diagnostics");
    let found: Vec<&Value> = records.iter().filter(|record| record["code"] == code && record["severity"] == severity).collect();
    assert_eq!(found.len(), 1, "one {severity} {code}: {value:#}");
    found[0]
}

/// The document's two style warnings, at their authored lines, and the
/// trigger warning.
fn assert_document_and_trigger_warnings(value: &Value) {
    record(value, TRIGGER, "warning");
    assert_at(record(value, UNKNOWN, "warning"), 3, 3);
    assert_at(record(value, DEPRECATED, "warning"), 5, 5);
}

fn assert_at(record: &Value, line: u64, column: u64) {
    assert_eq!(record["source"]["line"], line, "{record:#}");
    assert_eq!(record["source"]["column"], column, "{record:#}");
    assert!(record["source"]["file"].as_str().is_some_and(|file| file.ends_with("doc.md")), "{record:#}");
}

#[test]
fn a_missing_style_file_keeps_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-missing", DOCUMENT);
    let value = failure(&fixture, &["--style", "missing.yaml"]);
    record(&value, "dm.input.read_failure", "error");
    assert_document_and_trigger_warnings(&value);
}

#[test]
fn a_malformed_style_file_keeps_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-malformed", DOCUMENT);
    fixture.write_file("cwd/bad.yaml", "margin: [\n");
    let value = failure(&fixture, &["--style", "bad.yaml"]);
    record(&value, "dm.data.malformed", "error");
    assert_document_and_trigger_warnings(&value);
}

#[test]
fn an_offset_that_is_not_a_mapping_keeps_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-offset", DOCUMENT);
    fixture.write_file("cwd/offset.yaml", "theme: 2\n");
    let value = failure(&fixture, &["--style", "theme@offset.yaml"]);
    record(&value, "dm.style.invalid_root", "error");
    assert_document_and_trigger_warnings(&value);
}

#[test]
fn a_remote_style_reference_keeps_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-remote", DOCUMENT);
    // No `--allow host=`: a fetch would be refused, never attempted.
    let value = failure(&fixture, &["--style", "https://example.com/s.yaml"]);
    record(&value, "dm.input.read_failure", "error");
    assert_document_and_trigger_warnings(&value);
}

#[test]
fn an_unsupported_style_extension_keeps_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-css", DOCUMENT);
    fixture.write_file("cwd/s.css", "body {}\n");
    let value = failure(&fixture, &["--style", "s.css"]);
    record(&value, "dm.input.read_failure", "error");
    assert_document_and_trigger_warnings(&value);
}

#[test]
fn a_setter_that_is_not_a_property_path_keeps_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-setter-path", DOCUMENT);
    let value = failure(&fixture, &["style..x=1"]);
    record(&value, "dm.style.invalid_setter", "error");
    assert_document_and_trigger_warnings(&value);
}

#[test]
fn a_later_failing_file_keeps_an_earlier_files_warning() {
    let fixture = fixture("carried-earlier-file", DOCUMENT);
    fixture.write_file("cwd/good.yaml", "margin: 2\nbogus: 1\n");
    fixture.write_file("cwd/bad.yaml", "margin: [\n");
    let value = failure(&fixture, &["--style", "good.yaml", "--style", "bad.yaml"]);
    record(&value, "dm.data.malformed", "error");
    record(&value, TRIGGER, "warning");
    let records = value["diagnostics"].as_array().expect("diagnostics");
    let unknown: Vec<&Value> = records.iter().filter(|record| record["code"] == UNKNOWN && record["severity"] == "warning").collect();
    assert_eq!(unknown.len(), 2, "the document's and good.yaml's: {value:#}");
    assert!(
        unknown.iter().any(|record| record["source"]["file"].as_str().is_some_and(|file| file.ends_with("good.yaml"))),
        "{value:#}"
    );
    assert_at(record(&value, DEPRECATED, "warning"), 5, 5);
}

#[test]
fn denying_the_unknown_key_code_keeps_the_other_warnings() {
    let fixture = fixture("carried-deny-unknown", DOCUMENT);
    let value = failure(&fixture, &["--deny", UNKNOWN]);
    assert_at(record(&value, UNKNOWN, "error"), 3, 3);
    assert_at(record(&value, DEPRECATED, "warning"), 5, 5);
    record(&value, TRIGGER, "warning");
}

#[test]
fn denying_the_deprecation_code_keeps_the_other_warnings() {
    let fixture = fixture("carried-deny-deprecated", DOCUMENT);
    let value = failure(&fixture, &["--deny", DEPRECATED]);
    assert_at(record(&value, DEPRECATED, "error"), 5, 5);
    assert_at(record(&value, UNKNOWN, "warning"), 3, 3);
    record(&value, TRIGGER, "warning");
}

#[test]
fn a_setter_type_error_keeps_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-setter-type", DOCUMENT);
    let value = failure(&fixture, &["style.margin=abc"]);
    record(&value, "dm.schema.type_mismatch", "error");
    assert_document_and_trigger_warnings(&value);
}

#[test]
fn a_duplicate_key_setter_value_keeps_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-setter-duplicate", DOCUMENT);
    let value = failure(&fixture, &["style={margin:2,margin:3}"]);
    record(&value, "dm.style.invalid_setter", "error");
    assert_document_and_trigger_warnings(&value);
}

#[test]
fn denying_style_makes_both_document_warnings_errors() {
    let fixture = fixture("carried-deny-style", DOCUMENT);
    let value = failure(&fixture, &["--deny", "style"]);
    assert_at(record(&value, UNKNOWN, "error"), 3, 3);
    assert_at(record(&value, DEPRECATED, "error"), 5, 5);
    record(&value, TRIGGER, "warning");
}

#[test]
fn a_successful_render_reports_the_document_and_trigger_warnings() {
    let fixture = fixture("carried-success", DOCUMENT);
    for args in [&["doc.md", "--output", "json"][..], &["doc.md", "style.margin=2", "--output", "json"][..]] {
        let output = run(&fixture, args);
        assert_eq!(output.status.code(), Some(0), "{args:?}: {}", String::from_utf8_lossy(&output.stdout));
        assert!(output.stderr.is_empty(), "{args:?}");
        let value: Value = serde_json::from_slice(&output.stdout).expect("one JSON value");
        assert_eq!(value["outcome"], "success", "{args:?}: {value:#}");
        assert_document_and_trigger_warnings(&value);
    }
}

#[test]
fn a_formatted_failure_prints_the_warnings_and_no_page() {
    let fixture = fixture("carried-formatted", DOCUMENT);
    for args in [&["doc.md", "--style", "missing.yaml", "--output", "plain"][..], &["doc.md", "--deny", UNKNOWN, "--output", "plain"][..]]
    {
        let output = run(&fixture, args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stdout.is_empty(), "{args:?}: no page: {}", String::from_utf8_lossy(&output.stdout));
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("broken.trigger.yaml"), "{args:?}: {stderr}");
        assert!(stderr.contains("style.page.max_width"), "{args:?}: {stderr}");
        assert!(stderr.contains("style.magin"), "{args:?}: {stderr}");
    }
}

#[test]
fn a_document_schema_that_fails_keeps_the_document_and_trigger_warnings() {
    let document = "---\n$schema: ./missing-schema.yaml\nstyle:\n  magin: 4\n  page:\n    max_width: 50\n---\n# Notes\n";
    let fixture = fixture("carried-schema", document);
    let value = failure(&fixture, &["style.margin=2"]);
    record(&value, TRIGGER, "warning");
    assert_at(record(&value, UNKNOWN, "warning"), 4, 3);
    assert_at(record(&value, DEPRECATED, "warning"), 6, 5);
    let errors = value["diagnostics"].as_array().expect("diagnostics").iter().filter(|record| record["severity"] == "error").count();
    assert_eq!(errors, 1, "{value:#}");
}
