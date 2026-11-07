use crate::common;

use common::{CliProcessFixture, md_file};
use predicates::prelude::*;
use std::io::Write;

const FM_DOC: &str = "---\n\
title: Hello World\n\
author: Alice\n\
count: 42\n\
tags:\n\
  - rust\n\
  - cli\n\
---\n\
# Document\n";

const FM_DOC_TAB_INDENT: &str = "---\n\
\tlast_updated: 2026-02-27\n\
---\n\
# Document\n";

#[test]
fn test_get_single_property_string() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-single-property-string");
    fixture
        .command()
        .args(["get", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Hello World\""));
}

#[test]
fn test_get_single_property_number() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-single-property-number");
    fixture
        .command()
        .args(["get", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_single_property_array() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-single-property-array");
    fixture
        .command()
        .args(["get", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_get_missing_property_returns_empty_string() {
    let fixture =
        CliProcessFixture::named("get-set-rm-test-get-missing-property-returns-empty-string");
    fixture
        .command()
        .args(["get", "-", "nonexistent"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_multiple_properties_returns_object() {
    let fixture =
        CliProcessFixture::named("get-set-rm-test-get-multiple-properties-returns-object");
    fixture
        .command()
        .args(["get", "-", "title", "author"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"author\""))
        .stdout(predicate::str::contains("\"Alice\""));
}

#[test]
fn test_get_multiple_with_missing_includes_empty_string() {
    let fixture =
        CliProcessFixture::named("get-set-rm-test-get-multiple-with-missing-includes-empty-string");
    fixture
        .command()
        .args(["get", "-", "title", "missing"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"missing\""))
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_json5_output() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-json5-output");
    fixture
        .command()
        .args(["get", "--output", "json5", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        // JSON5 uses unquoted keys for valid identifiers
        .stdout(predicate::str::contains("title:"))
        .stdout(predicate::str::contains("count:"));
}

#[test]
fn test_get_yaml_output() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-yaml-output");
    fixture
        .command()
        .args(["get", "--output", "yaml", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello World"));
}

#[test]
fn test_get_toml_output() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-toml-output");
    fixture
        .command()
        .args(["get", "--output", "toml", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("Hello World"))
        .stdout(predicate::str::contains("count"))
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_from_file() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-from-file");
    let tmp = md_file("---\nversion: 2\n---\n# Doc\n");

    fixture
        .command()
        .args(["get"])
        .arg(tmp.path())
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_get_no_frontmatter_returns_empty_string() {
    let fixture =
        CliProcessFixture::named("get-set-rm-test-get-no-frontmatter-returns-empty-string");
    fixture
        .command()
        .args(["get", "-", "title"])
        .write_stdin("# No frontmatter")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

/// Regression: malformed frontmatter (a quoted scalar followed by trailing
/// unquoted text) used to be silently treated as "no frontmatter", so
/// `md get phases` returned `""` even when the file clearly defined `phases`.
/// The fix surfaces a `MarkdownError::FrontmatterParse` with the offending
/// YAML line in the rendered StatusBlock.
#[test]
fn test_get_malformed_frontmatter_renders_status_block_with_offending_line() {
    let fixture = CliProcessFixture::named(
        "get-set-rm-test-get-malformed-frontmatter-renders-status-block-with-offending-line",
    );
    use darkmatter::testing::strip_ansi_codes;

    let yaml = "---\nphases: 5\nfindings:\n  - id: '@' magic lookup emits results\n---\n# Doc\n";

    let output = fixture
        .command()
        .args(["get", "--output", "plain", "-", "phases"])
        .write_stdin(yaml)
        .output()
        .expect("md get should run");

    assert!(!output.status.success(), "expected a failure exit status");

    // The offending YAML line is syntax-highlighted, so its characters are
    // interleaved with SGR escapes in the raw stderr; strip ANSI before
    // asserting on the visible text.
    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("MarkdownError"), "stderr: {stderr}");
    assert!(
        stderr.contains("frontmatter parse failed"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("'@' magic lookup emits results"),
        "offending line must be shown. stderr: {stderr}"
    );
}

#[test]
fn test_get_rejects_duplicate_keys_through_the_normal_cli_path() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-rejects-duplicate-keys");
    let yaml = "---\nstatus: first\nstatus: second\n---\n# Doc\n";

    let output = fixture
        .command()
        .args(["get", "-", "status"])
        .write_stdin(yaml)
        .output()
        .expect("md get should run");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "failure", "{report:#}");
    assert_eq!(report["value"], serde_json::Value::Null, "{report:#}");
    assert!(report["diagnostics"].to_string().contains("duplicate entry with key"), "{report:#}");
}

#[test]
fn test_get_malformed_frontmatter_reports_the_failure_in_the_default_json_payload() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-malformed-frontmatter-json");
    let yaml = "---\nphases: 5\nfindings:\n  - id: '@' magic lookup emits results\n---\n# Doc\n";

    let output = fixture
        .command()
        .args(["get", "-", "phases"])
        .write_stdin(yaml)
        .output()
        .expect("md get should run");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty(), "{output:?}");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).expect("one JSON report");
    assert_eq!(report["outcome"], "failure", "{report:#}");
    assert_eq!(report["value"], serde_json::Value::Null, "{report:#}");
    assert_eq!(report["diagnostics"][0]["code"], "dm.frontmatter.yaml_parse", "{report:#}");
    assert_eq!(report["diagnostics"][0]["source"]["line"], 4, "{report:#}");
}

#[test]
fn test_get_tab_indented_frontmatter_property_is_populated() {
    let fixture = CliProcessFixture::named(
        "get-set-rm-test-get-tab-indented-frontmatter-property-is-populated",
    );
    fixture
        .command()
        .args(["get", "-", "last_updated"])
        .write_stdin(FM_DOC_TAB_INDENT)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"2026-02-27\""));
}

// -- raw flag tests --

#[test]
fn test_get_raw_string_unquoted() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-raw-string-unquoted");
    fixture
        .command()
        .args(["get", "--output", "plain", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("Hello World\n");
}

#[test]
fn test_get_raw_number() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-raw-number");
    fixture
        .command()
        .args(["get", "--output", "plain", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("42\n");
}

#[test]
fn test_get_raw_null_returns_empty() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-raw-null-returns-empty");
    fixture
        .command()
        .args(["get", "--output", "plain", "-", "nonexistent"])
        .write_stdin("---\nnonexistent: null\n---\n# Doc")
        .assert()
        .success()
        .stdout("\n");
}

#[test]
fn test_get_raw_array_one_per_line() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-raw-array-one-per-line");
    fixture
        .command()
        .args(["get", "--output", "plain", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("rust\ncli\n");
}

#[test]
fn test_get_raw_object_key_value_lines() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-raw-object-key-value-lines");
    fixture
        .command()
        .args(["get", "--output", "plain", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Hello World"))
        .stdout(predicate::str::contains("count: 42"));
}

// -- compact flag tests --

#[test]
fn test_get_compact_array() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-compact-array");
    fixture
        .command()
        .args(["get", "--compact", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("[\"rust\",\"cli\"]\n");
}

#[test]
fn test_get_compact_object() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-compact-object");
    fixture
        .command()
        .args(["get", "--compact", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"Hello World\""))
        .stdout(predicate::str::contains("\"count\":42"));
}

#[test]
fn test_get_compact_scalar_unchanged() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-compact-scalar-unchanged");
    fixture
        .command()
        .args(["get", "--compact", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("\"Hello World\"\n");
}

// =============================================================================
//                          SET SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_set_string_value_via_stdin() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-string-value-via-stdin");
    fixture
        .command()
        .args(["set", "-", "title", "New Title"])
        .write_stdin("---\ntitle: Old Title\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: New Title"))
        .stdout(predicate::str::contains("# Content"));
}

#[test]
fn test_set_adds_new_property_via_stdin() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-adds-new-property-via-stdin");
    fixture
        .command()
        .args(["set", "-", "author", "Alice"])
        .write_stdin("---\ntitle: Hello\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("author: Alice"))
        .stdout(predicate::str::contains("title: Hello"));
}

#[test]
fn test_set_numeric_value() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-numeric-value");
    fixture
        .command()
        .args(["set", "-", "count", "42"])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("count: 42"));
}

#[test]
fn test_set_boolean_value() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-boolean-value");
    fixture
        .command()
        .args(["set", "-", "draft", "true"])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("draft: true"));
}

#[test]
fn test_set_json_array_value() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-json-array-value");
    fixture
        .command()
        .args(["set", "-", "tags", r#"["rust","cli"]"#])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_set_creates_frontmatter_when_none_exists() {
    let fixture =
        CliProcessFixture::named("get-set-rm-test-set-creates-frontmatter-when-none-exists");
    fixture
        .command()
        .args(["set", "-", "title", "Brand New"])
        .write_stdin("# No Frontmatter\n\nJust content.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Brand New"))
        .stdout(predicate::str::contains("# No Frontmatter"));
}

#[test]
fn test_set_updates_file_in_place() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-updates-file-in-place");
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\ntitle: Original\n---\n# Content\n").unwrap();

    fixture
        .command()
        .arg("set")
        .arg(tmp.path())
        .args(["title", "Updated", "--save"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("title: Updated"));
    assert!(updated.contains("# Content"));
    assert!(!updated.contains("Original"));
}

#[test]
fn test_set_without_save_does_not_mutate_file() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-without-save-does-not-mutate-file");
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\ntitle: Original\n---\n# Content\n").unwrap();

    fixture
        .command()
        .arg("set")
        .arg(tmp.path())
        .args(["title", "Updated"])
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Updated"))
        .stdout(predicate::str::contains("# Content"));

    // File should be unchanged
    let on_disk = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(on_disk.contains("title: Original"));
}

#[test]
fn test_set_preserves_body_content() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-preserves-body-content");
    let input =
        "---\ntitle: Test\n---\n# Heading\n\nParagraph with **bold** text.\n\n- list item\n";
    fixture
        .command()
        .args(["set", "-", "version", "2"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("# Heading"))
        .stdout(predicate::str::contains("**bold**"))
        .stdout(predicate::str::contains("- list item"));
}

#[test]
fn test_get_requires_at_least_one_prop() {
    let fixture = CliProcessFixture::named("get-set-rm-test-get-requires-at-least-one-prop");
    fixture
        .command()
        .args(["get", "-"])
        .write_stdin(FM_DOC)
        .assert()
        .failure();
}

/// Hand-authored frontmatter whose formatting a re-serialization would change:
/// four-space list indentation, a comment, quoting, and a block scalar.
const AUTHORED_DOC: &str = "---\ntotal_phases: 7\nphase: 1  # first phase\ntitle: 'Quoted Title'\nfiles:\n    - a.rs\n    - b.rs\nnote: |-\n  line one\n  line two\n---\n# Body\n";

fn authored_file() -> tempfile::NamedTempFile {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    tmp.write_all(AUTHORED_DOC.as_bytes()).unwrap();
    tmp
}

#[test]
fn test_set_save_new_property_keeps_every_other_byte() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-save-new-property-keeps-bytes");
    let tmp = authored_file();

    fixture
        .command()
        .arg("set")
        .arg(tmp.path())
        .args(["start_phase", "6", "--save"])
        .assert()
        .success();

    let expected = AUTHORED_DOC.replace("---\n# Body", "start_phase: 6\n---\n# Body");
    assert_eq!(std::fs::read_to_string(tmp.path()).unwrap(), expected);
}

#[test]
fn test_set_save_existing_property_keeps_position_and_other_bytes() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-save-existing-property-keeps-bytes");
    let tmp = authored_file();

    fixture
        .command()
        .arg("set")
        .arg(tmp.path())
        .args(["total_phases", "8", "--save"])
        .assert()
        .success();

    let expected = AUTHORED_DOC.replace("total_phases: 7", "total_phases: 8");
    assert_eq!(std::fs::read_to_string(tmp.path()).unwrap(), expected);
}

#[test]
fn test_set_stdout_keeps_every_other_byte() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-stdout-keeps-bytes");
    let expected = AUTHORED_DOC.replace("---\n# Body", "start_phase: 6\n---\n# Body");

    fixture
        .command()
        .args(["set", "-", "start_phase", "6"])
        .write_stdin(AUTHORED_DOC)
        .assert()
        .success()
        .stdout(predicate::eq(expected));
}

#[test]
fn test_rm_keeps_every_other_byte() {
    let fixture = CliProcessFixture::named("get-set-rm-test-rm-keeps-bytes");
    let tmp = authored_file();

    fixture
        .command()
        .arg("rm")
        .arg(tmp.path())
        .args(["title", "note"])
        .assert()
        .success();

    let expected = AUTHORED_DOC
        .replace("title: 'Quoted Title'\n", "")
        .replace("note: |-\n  line one\n  line two\n", "");
    assert_eq!(std::fs::read_to_string(tmp.path()).unwrap(), expected);
}

#[test]
fn test_set_then_rm_round_trip_restores_original_bytes() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-rm-round-trip");
    let tmp = authored_file();

    for _ in 0..2 {
        fixture
            .command()
            .arg("set")
            .arg(tmp.path())
            .args(["start_phase", "6", "--save"])
            .assert()
            .success();
        fixture
            .command()
            .args(["get", "--output", "plain"])
            .arg(tmp.path())
            .arg("start_phase")
            .assert()
            .success()
            .stdout("6\n");
        fixture
            .command()
            .arg("rm")
            .arg(tmp.path())
            .arg("start_phase")
            .assert()
            .success();
        assert_eq!(std::fs::read_to_string(tmp.path()).unwrap(), AUTHORED_DOC);
    }
}

#[test]
fn test_set_save_indented_root_falls_back_to_reserializing() {
    let fixture = CliProcessFixture::named("get-set-rm-test-set-save-indented-root-falls-back");
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\n  title: Indented\n---\n# Content\n").unwrap();

    fixture
        .command()
        .arg("set")
        .arg(tmp.path())
        .args(["count", "3", "--save"])
        .assert()
        .success();

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("title: Indented"));
    assert!(updated.contains("count: 3"));
    assert!(updated.contains("# Content"));
}

// =============================================================================
//                              PROPERTY PATHS
// =============================================================================

const NESTED_DOC: &str = "---\ntitle: Notes\nstyle:\n  page:\n    max-width: 96 # wide\n    alignment: left\nauthor: Ken\nnothing: null\nitems:\n  - name: a\n  - name: b\nletters: [a, b, c, d]\n\"0\": zero\n\"00\": double-zero\nnumbers:\n  \"0\": mapped\n\"dotted.key\": literal\n'C:\\': { name: drive }\n'C:\\dir': windows\n\"\": empty\nénoncé: été\n---\n# Body\n";

fn nested_document(name: &str) -> CliProcessFixture {
    with_document(name, NESTED_DOC)
}

fn with_document(name: &str, content: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/doc.md", content);
    fixture
}

fn read_document(fixture: &CliProcessFixture) -> String {
    std::fs::read_to_string(fixture.cwd().join("doc.md")).expect("the document")
}

/// Runs `md` with `args` and returns its status and the text it wrote.
fn run_md(fixture: &CliProcessFixture, args: &[&str]) -> (Option<i32>, String, String) {
    let output = fixture.command().args(args).output().expect("md runs");
    (output.status.code(), String::from_utf8_lossy(&output.stdout).into_owned(), String::from_utf8_lossy(&output.stderr).into_owned())
}

/// `md get`'s JSON value at `path`.
fn get_json(fixture: &CliProcessFixture, path: &str) -> serde_json::Value {
    let (status, stdout, stderr) = run_md(fixture, &["get", "doc.md", path, "--output", "json"]);
    assert_eq!(status, Some(0), "{stdout}{stderr}");
    serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: {stdout}"))
}

/// Runs a mutation that must fail with `status` and leave the source's bytes
/// alone, returning stderr.
fn assert_refused(fixture: &CliProcessFixture, args: &[&str], status: i32) -> String {
    let before = read_document(fixture);
    let (code, stdout, stderr) = run_md(fixture, args);
    assert_eq!(code, Some(status), "{args:?}\n{stdout}\n{stderr}");
    assert_eq!(read_document(fixture), before, "{args:?} must write nothing");
    stderr
}

/// `md set --save`, which must succeed, returning the file it left.
fn set_saved(fixture: &CliProcessFixture, path: &str, value: &str) -> String {
    let (code, stdout, stderr) = run_md(fixture, &["set", "doc.md", path, value, "--save"]);
    assert_eq!(code, Some(0), "{path}={value}\n{stdout}\n{stderr}");
    read_document(fixture)
}

mod get_paths {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_nested_value_is_read_and_an_ordinary_key_is_unchanged() {
        let fixture = nested_document("paths-get-nested");
        assert_eq!(get_json(&fixture, "style.page.max-width"), json!(96));
        assert_eq!(get_json(&fixture, "title"), json!("Notes"));
        assert_eq!(get_json(&fixture, "style.page"), json!({ "max-width": 96, "alignment": "left" }));
    }

    #[test]
    fn a_dot_always_separates_and_an_escaped_dot_names_a_literal_key() {
        let fixture = nested_document("paths-get-dotted");
        assert_eq!(get_json(&fixture, "dotted.key"), json!(""), "one syntactic reading, whatever the document holds");
        assert_eq!(get_json(&fixture, r"dotted\.key"), json!("literal"));
    }

    #[test]
    fn backslashes_follow_the_two_escapes() {
        let fixture = nested_document("paths-get-backslash");
        assert_eq!(get_json(&fixture, r"C:\\.name"), json!("drive"), "`C:\\\\` is the key ending in a backslash");
        assert_eq!(get_json(&fixture, r"C:\dir"), json!("windows"), "a backslash before anything else is itself");
    }

    #[test]
    fn the_empty_argument_special_keys_and_unicode_are_ordinary_keys() {
        let fixture = with_document("paths-get-special", "---\n$schema:\n  title: string\n\"\": empty\nénoncé: été\n---\n# Body\n");
        assert_eq!(get_json(&fixture, ""), json!("empty"));
        assert_eq!(get_json(&fixture, "$schema"), json!({ "title": "string" }));
        assert_eq!(get_json(&fixture, "énoncé"), json!("été"));
    }

    #[test]
    fn a_number_is_an_index_on_an_array_and_a_key_on_a_mapping() {
        let fixture = nested_document("paths-get-numbers");
        assert_eq!(get_json(&fixture, "items.1.name"), json!("b"));
        assert_eq!(get_json(&fixture, "numbers.0"), json!("mapped"));
        assert_eq!(get_json(&fixture, "0"), json!("zero"));
        assert_eq!(get_json(&fixture, "00"), json!("double-zero"), "`0` and `00` are different mapping keys");
    }

    #[test]
    fn an_unreachable_path_is_missing_and_prints_empty() {
        let fixture = nested_document("paths-get-unreachable");
        for path in [
            "items.9",
            "items.01",
            "items.-1",
            "items.-",
            "items.1234567890123456789012345678901234567890",
            "author.name",
            "nothing.deep",
            "style.absent",
            "absent.deep",
        ] {
            assert_eq!(get_json(&fixture, path), json!(""), "{path}");
        }
        let (code, stdout, _) = run_md(&fixture, &["get", "doc.md", "items.9", "--output", "plain"]);
        assert_eq!((code, stdout.as_str()), (Some(0), "\n"));
    }

    #[test]
    fn an_explicit_null_stays_null_in_data_output() {
        let fixture = nested_document("paths-get-null");
        assert_eq!(get_json(&fixture, "nothing"), json!(null));
    }

    #[test]
    fn several_paths_give_a_flat_object_keyed_as_written() {
        let fixture = nested_document("paths-get-several");
        let (code, stdout, stderr) =
            run_md(&fixture, &["get", "doc.md", "style.page.alignment", "title", r"dotted\.key", "items.9", "--compact", "--output", "json"]);
        assert_eq!(code, Some(0), "{stderr}");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&stdout).unwrap(),
            json!({ "style.page.alignment": "left", "title": "Notes", r"dotted\.key": "literal", "items.9": "" }),
        );
    }
}

mod malformed_paths {
    use super::*;

    /// A missing file would fail with status 1, so status 2 proves the path was
    /// judged before anything was opened.
    #[test]
    fn each_command_fails_as_usage_before_reading_anything() {
        let fixture = CliProcessFixture::named("paths-malformed");
        for path in ["a.", ".a", "a..b"] {
            for args in [vec!["get", "missing.md", path], vec!["set", "missing.md", path, "1"], vec!["rm", "missing.md", path]] {
                let (code, stdout, stderr) = run_md(&fixture, &args);
                assert_eq!(code, Some(2), "{args:?}\n{stdout}\n{stderr}");
                assert!(stdout.is_empty(), "{args:?}: {stdout}");
                assert!(stderr.contains("empty segment"), "{args:?}: {stderr}");
            }
        }
    }

    #[test]
    fn one_malformed_path_among_good_ones_stops_get_and_rm() {
        let fixture = nested_document("paths-malformed-among-good");
        let (code, stdout, _) = run_md(&fixture, &["get", "doc.md", "title", "a..b", "--output", "json"]);
        assert_eq!((code, stdout.as_str()), (Some(2), ""));
        assert_refused(&fixture, &["rm", "doc.md", "title", "a."], 2);
    }

    #[test]
    fn stdin_is_not_consumed_for_a_malformed_path() {
        let fixture = CliProcessFixture::named("paths-malformed-stdin");
        fixture.command().args(["get", "-", "a."]).write_stdin(FM_DOC).assert().code(2).stdout(predicate::str::is_empty());
    }
}

mod set_paths {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_nested_edit_keeps_comments_and_spelling_in_the_shared_parent() {
        let source = "---\nstyle:\n  # about the page\n  page: 'quoted'   # kept\n  width: 0x10\n---\n# Body\n\nkept\n";
        let fixture = with_document("paths-set-comments", source);
        let saved = set_saved(&fixture, "style.width", "20");
        assert_eq!(saved, source.replace("width: 0x10", "width: 20"));
    }

    #[test]
    fn a_dotted_path_never_writes_a_literal_key_and_an_escaped_one_does() {
        let fixture = with_document("paths-set-literal", "---\ntitle: Notes\n---\n# Body\n");
        let saved = set_saved(&fixture, "a.b", "1");
        assert_eq!(saved, "---\ntitle: Notes\na:\n  b: 1\n---\n# Body\n");

        let saved = set_saved(&fixture, r"a\.b", "2");
        assert!(saved.contains("a.b: 2\n"), "{saved}");
        assert_eq!(get_json(&fixture, r"a\.b"), json!(2));
        assert_eq!(get_json(&fixture, "a.b"), json!(1));

        let (code, _, stderr) = run_md(&fixture, &["rm", "doc.md", r"a\.b"]);
        assert_eq!(code, Some(0), "{stderr}");
        assert_eq!(get_json(&fixture, r"a\.b"), json!(""));
        assert_eq!(get_json(&fixture, "a.b"), json!(1), "the nested value was not the one named");
    }

    #[test]
    fn a_missing_parent_is_created() {
        let fixture = with_document("paths-set-parent", "---\ntitle: Notes\n---\n# Body\n");
        set_saved(&fixture, "author.name", "Ken");
        assert_eq!(get_json(&fixture, "author"), json!({ "name": "Ken" }));
    }

    #[test]
    fn a_number_under_a_missing_parent_is_refused() {
        let fixture = with_document("paths-set-number", "---\ntitle: Notes\n---\n# Body\n");
        let stderr = assert_refused(&fixture, &["set", "doc.md", "items.0.name", "x", "--save"], 1);
        assert!(stderr.contains("items.0"), "{stderr}");
    }

    #[test]
    fn a_parent_that_cannot_hold_the_key_is_named_with_its_type() {
        let fixture = nested_document("paths-set-scalar-parent");
        let stderr = assert_refused(&fixture, &["set", "doc.md", "author.name", "x", "--save"], 1);
        assert!(stderr.contains("author") && stderr.contains("text"), "{stderr}");
        let stderr = assert_refused(&fixture, &["set", "doc.md", "nothing.deep", "x", "--save"], 1);
        assert!(stderr.contains("nothing") && stderr.contains("null"), "{stderr}");
    }

    #[test]
    fn a_whole_scalar_or_null_can_be_replaced_by_naming_it() {
        let fixture = nested_document("paths-set-replace");
        set_saved(&fixture, "nothing", "{deep:1}");
        assert_eq!(get_json(&fixture, "nothing.deep"), json!(1));
        set_saved(&fixture, "author", r#"{name:"Bob"}"#);
        assert_eq!(get_json(&fixture, "author.name"), json!("Bob"));
    }

    #[test]
    fn an_array_element_is_edited_and_an_index_out_of_range_names_the_length() {
        let fixture = nested_document("paths-set-array");
        set_saved(&fixture, "items.1.name", "z");
        assert_eq!(get_json(&fixture, "items"), json!([{ "name": "a" }, { "name": "z" }]));

        let stderr = assert_refused(&fixture, &["set", "doc.md", "items.2.name", "x", "--save"], 1);
        assert!(stderr.contains("items.2") && stderr.contains("2 element"), "{stderr}");
        assert_refused(&fixture, &["set", "doc.md", "items.01.name", "x", "--save"], 1);
        assert_refused(&fixture, &["set", "doc.md", "items.1234567890123456789012345678901234567890", "x", "--save"], 1);
    }

    #[test]
    fn a_number_on_a_mapping_is_a_key() {
        let fixture = nested_document("paths-set-number-key");
        set_saved(&fixture, "numbers.00", "new");
        assert_eq!(get_json(&fixture, "numbers"), json!({ "0": "mapped", "00": "new" }));
    }

    #[test]
    fn the_empty_top_level_key_can_be_set() {
        let fixture = nested_document("paths-set-empty-key");
        set_saved(&fixture, "", "changed");
        assert_eq!(get_json(&fixture, ""), json!("changed"));
    }

    #[test]
    fn repeated_edits_round_trip_through_the_file() {
        let source = "---\n# top\nnotes:\n  page: { a: 1 }\n  # inner\n  ul:\n    padding: 2 # two\ntitle: Notes\n---\n# Body\n";
        let fixture = with_document("paths-set-round-trip", source);
        for padding in ["3", "4"] {
            set_saved(&fixture, "notes.ul.padding", padding);
            assert_eq!(get_json(&fixture, "notes.ul.padding").to_string(), padding);
        }
        assert_eq!(read_document(&fixture), source.replace("padding: 2 # two", "padding: 4 # two"));
    }

    #[test]
    fn a_flow_collection_falls_back_and_still_edits_the_value() {
        let fixture = with_document("paths-set-flow", "---\nauthor: { name: Ann, email: a@x }\nother: 1\n---\n# Body\n");
        let saved = set_saved(&fixture, "author.name", "Bob");
        assert!(saved.ends_with("---\n# Body\n"), "the body keeps its bytes: {saved}");
        assert_eq!(get_json(&fixture, "author"), json!({ "name": "Bob", "email": "a@x" }));
        assert_eq!(get_json(&fixture, "other"), json!(1));
    }

    #[test]
    fn an_edit_an_alias_reads_is_refused_with_no_fallback() {
        let fixture = with_document("paths-set-alias", "---\nbase: &shared\n  name: Ann\ncopy: *shared\n---\n# Body\n");
        let stderr = assert_refused(&fixture, &["set", "doc.md", "base.name", "Bob", "--save"], 1);
        assert!(stderr.contains("copy"), "names the value that would change: {stderr}");
    }

    #[test]
    fn stdin_without_save_prints_the_edited_document() {
        let fixture = CliProcessFixture::named("paths-set-stdin");
        fixture
            .command()
            .args(["set", "-", "a.b", "1"])
            .write_stdin("---\na:\n  c: 2 # kept\n---\n# Body\n")
            .assert()
            .success()
            .stdout("---\na:\n  c: 2 # kept\n  b: 1\n---\n# Body\n");
    }

    #[test]
    fn the_result_carries_the_path_as_written_and_the_value_as_bound() {
        let fixture = with_document("paths-set-result", "---\n$schema:\n  a:\n    v: string\n---\n# Body\n");
        let (code, stdout, stderr) = run_md(&fixture, &["set", "doc.md", "a.v", "1.10", "--output", "json"]);
        assert_eq!(code, Some(0), "{stderr}");
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["property"], "a.v");
        assert_eq!(report["value"], "1.10");
        assert_eq!(report["saved"], false);
    }
}

mod set_values {
    use super::*;
    use serde_json::json;

    const TYPED: &str = "---\n$schema:\n  version: string\n  count: number\n  zip: numberlike\n  flag: boolish\n  either: string | number\n  nums: number[]\n  names: string[]\n  maybe: string\n  author:\n    name: string\n    email: string\n---\n# Body\n";

    #[test]
    fn a_declared_type_reads_the_text() {
        let fixture = with_document("values-declared", TYPED);
        let saved = set_saved(&fixture, "version", "1.10");
        assert!(saved.contains("version: '1.10'") || saved.contains("version: \"1.10\""), "{saved}");
        assert_eq!(get_json(&fixture, "version"), json!("1.10"));
        set_saved(&fixture, "count", "5");
        assert_eq!(get_json(&fixture, "count"), json!(5));
    }

    #[test]
    fn numberlike_boolish_and_unions_follow_the_shared_rules() {
        let fixture = with_document("values-rules", TYPED);
        set_saved(&fixture, "zip", "02134");
        assert_eq!(get_json(&fixture, "zip"), json!("02134"));
        set_saved(&fixture, "flag", "On");
        assert_eq!(get_json(&fixture, "flag"), json!("On"));
        set_saved(&fixture, "flag", "true");
        assert_eq!(get_json(&fixture, "flag"), json!(true));
        set_saved(&fixture, "either", "1.10");
        assert_eq!(get_json(&fixture, "either"), json!("1.10"));
    }

    #[test]
    fn an_array_element_reads_through_the_element_type() {
        let fixture = with_document("values-element", &TYPED.replace("---\n# Body", "nums: [1, 2]\nnames: [a, b]\n---\n# Body"));
        set_saved(&fixture, "nums.1", "5");
        assert_eq!(get_json(&fixture, "nums"), json!([1, 5]));
        set_saved(&fixture, "names.0", "1.10");
        assert_eq!(get_json(&fixture, "names"), json!(["1.10", "b"]));
    }

    #[test]
    fn an_undeclared_key_keeps_the_guess() {
        let fixture = with_document("values-undeclared", "---\ntitle: Notes\n---\n# Body\n");
        set_saved(&fixture, "count", "5");
        assert_eq!(get_json(&fixture, "count"), json!(5));
        set_saved(&fixture, "ratio", "1.10");
        assert_eq!(get_json(&fixture, "ratio"), json!(1.1));
        set_saved(&fixture, "text", "plain words");
        assert_eq!(get_json(&fixture, "text"), json!("plain words"));
    }

    #[test]
    fn containers_read_the_same_in_json5_and_yaml_flow() {
        let fixture = with_document("values-containers", "---\ntitle: Notes\n---\n# Body\n");
        set_saved(&fixture, "a", "{a:1}");
        assert_eq!(get_json(&fixture, "a"), json!({ "a": 1 }));
        set_saved(&fixture, "b", "{ max-width: 50 }");
        assert_eq!(get_json(&fixture, "b"), json!({ "max-width": 50 }));
        set_saved(&fixture, "c", "[1,'two']");
        assert_eq!(get_json(&fixture, "c"), json!([1, "two"]));
    }

    #[test]
    fn a_quoted_string_decodes_once_and_null_is_null() {
        let fixture = with_document("values-quoted", "---\ntitle: Notes\n---\n# Body\n");
        set_saved(&fixture, "who", "\"Alice\"");
        assert_eq!(get_json(&fixture, "who"), json!("Alice"));
        set_saved(&fixture, "gone", "null");
        assert_eq!(get_json(&fixture, "gone"), json!(null));
        set_saved(&fixture, "empty", "");
        assert_eq!(get_json(&fixture, "empty"), json!(""));
    }

    #[test]
    fn an_inadmissible_container_fails_and_never_becomes_text() {
        let fixture = with_document("values-inadmissible", "---\ntitle: Notes\n---\n# Body\n");
        for value in ["{a:1,a:2}", "{a:NaN}", "{a:9007199254740993}", "{ a: 1, a: 2, max-width: 3 }"] {
            let stderr = assert_refused(&fixture, &["set", "doc.md", "meta", value, "--save"], 1);
            assert!(stderr.contains("meta"), "{value}: {stderr}");
        }
    }

    #[test]
    fn a_failed_value_reports_through_the_data_output_with_no_document() {
        let fixture = with_document("values-envelope", TYPED);
        let (code, stdout, stderr) = run_md(&fixture, &["set", "doc.md", "count", "abc", "--save", "--output", "json"]);
        assert_eq!(code, Some(1), "{stdout}");
        assert!(stderr.is_empty(), "{stderr}");
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["outcome"], "failure");
        assert_eq!(report["document"], serde_json::Value::Null);
        assert_eq!(report["saved"], false);
        assert_eq!(report["property"], "count");
        assert!(report["diagnostics"][0]["code"].as_str().unwrap().starts_with("dm.schema."), "{report:#}");
        assert_eq!(report["diagnostics"][0]["source"]["line"], serde_json::Value::Null, "never the old on-disk scalar: {report:#}");
        assert_eq!(read_document(&fixture), TYPED);
    }

    #[test]
    fn a_pending_value_is_written_unevaluated() {
        let fixture = with_document("values-pending", TYPED);
        let saved = set_saved(&fixture, "count", "{{ fm.other }}");
        assert!(saved.contains("{{ fm.other }}"), "{saved}");
        let saved = set_saved(&fixture, "maybe", "$(touch sentinel)");
        assert!(saved.contains("$(touch sentinel)"), "{saved}");
        assert!(!fixture.cwd().join("sentinel").exists(), "nothing runs");
    }
}

mod set_validation {
    use super::*;
    use serde_json::json;

    /// `count: number` and a missing required `title`.
    const REQUIRED: &str = "---\n$schema:\n  title: string(required)\n  count: number\n---\n# Body\n";

    #[test]
    fn a_valid_value_writes_despite_a_problem_elsewhere() {
        let fixture = with_document("validation-elsewhere", REQUIRED);
        set_saved(&fixture, "count", "5");
        assert_eq!(get_json(&fixture, "count"), json!(5));
    }

    #[test]
    fn a_value_the_type_rejects_writes_nothing() {
        let fixture = with_document("validation-rejected", REQUIRED);
        let stderr = assert_refused(&fixture, &["set", "doc.md", "count", "abc", "--save"], 1);
        assert!(stderr.contains("count"), "{stderr}");
    }

    #[test]
    fn setting_the_missing_key_repairs_the_document() {
        let fixture = with_document("validation-repair", REQUIRED);
        set_saved(&fixture, "title", "Notes");
        let (code, stdout, stderr) = run_md(&fixture, &["schema", "validate", "doc.md"]);
        assert_eq!(code, Some(0), "{stdout}{stderr}");
    }

    #[test]
    fn a_broken_schema_fails_with_an_explanation_and_no_write() {
        let missing = with_document("validation-missing-schema", "---\n$schema: ./absent.yaml\ncount: 1\n---\n# Body\n");
        let stderr = assert_refused(&missing, &["set", "doc.md", "count", "2", "--save"], 1);
        assert!(stderr.contains("absent.yaml"), "{stderr}");

        let malformed = with_document("validation-malformed-schema", "---\n$schema:\n  count: not-a-real-type(\ncount: 1\n---\n# Body\n");
        assert_refused(&malformed, &["set", "doc.md", "count", "2", "--save"], 1);
    }

    #[test]
    fn a_schema_in_a_separate_file_applies() {
        let fixture = with_document("validation-file-schema", "---\n$schema: ./s.yaml\ncount: 1\n---\n# Body\n");
        fixture.write_file("cwd/s.yaml", "$schema:\n  count: number\n");
        assert_refused(&fixture, &["set", "doc.md", "count", "abc", "--save"], 1);
        set_saved(&fixture, "count", "2");
        assert_eq!(get_json(&fixture, "count"), json!(2));
    }

    #[test]
    fn setting_schema_checks_only_the_reference() {
        let fixture = with_document("validation-set-schema", "---\ncount: abc\n---\n# Body\n");
        fixture.write_file("cwd/s.yaml", "$schema:\n  count: number\n");
        let saved = set_saved(&fixture, "$schema", "./s.yaml");
        assert!(saved.contains("$schema: ./s.yaml"), "the document fails it, and that does not matter: {saved}");
        // A reference that resolves to nothing is still a reference.
        set_saved(&fixture, "$schema", "./absent.yaml");
        assert_eq!(get_json(&fixture, "$schema"), json!("./absent.yaml"));
        assert_refused(&fixture, &["set", "doc.md", "$schema", "https://example.invalid/s.yaml", "--save"], 1);
        // Text is always a name, so only a value that cannot be one is refused.
        assert_refused(&fixture, &["set", "doc.md", "$schema", "null", "--save"], 1);
    }

    #[test]
    fn a_bare_schema_name_stays_a_string_and_an_inline_schema_is_read() {
        let fixture = with_document("validation-schema-forms", "---\ncount: abc\n---\n# Body\n");
        set_saved(&fixture, "$schema", "2");
        assert_eq!(get_json(&fixture, "$schema"), json!("2"));
        set_saved(&fixture, "$schema", "{count: number}");
        assert_eq!(get_json(&fixture, "$schema"), json!({ "count": "number" }));
        assert_refused(&fixture, &["set", "doc.md", "$schema", "{count: not-a-type(}", "--save"], 1);
    }

    #[test]
    fn a_nested_key_a_closed_object_does_not_declare_is_refused() {
        let fixture = with_document("validation-closed", "---\n$schema:\n  author:\n    name: string\nauthor:\n  name: Ann\n---\n# Body\n");
        assert_refused(&fixture, &["set", "doc.md", "author.mistake", "1", "--save"], 1);
    }

    #[test]
    fn a_second_undeclared_key_is_refused_though_the_object_already_has_one() {
        let fixture = with_document(
            "validation-second-unknown",
            "---\n$schema:\n  author:\n    name: string\nauthor:\n  name: Ann\n  old: 1\n---\n# Body\n",
        );
        assert_refused(&fixture, &["set", "doc.md", "author.mistake", "1", "--save"], 1);
        assert_refused(&fixture, &["set", "doc.md", "author.old", "2", "--save"], 1);
        // A problem elsewhere never blocks an edit.
        set_saved(&fixture, "author.name", "Bob");
    }

    #[test]
    fn a_pattern_key_opens_the_object() {
        let fixture = with_document(
            "validation-open",
            "---\n$schema:\n  author:\n    name: string\n    \"<string>\": unknown\nauthor:\n  name: Ann\n---\n# Body\n",
        );
        set_saved(&fixture, "author.mistake", "1");
        assert_eq!(get_json(&fixture, "author.mistake"), json!(1));
    }

    #[test]
    fn a_required_child_of_a_new_or_replaced_object_blocks() {
        let fixture = with_document(
            "validation-ancestor",
            "---\n$schema:\n  author:\n    name: string(required)\n    email: string(required)\n---\n# Body\n",
        );
        assert_refused(&fixture, &["set", "doc.md", "author.name", "Ann", "--save"], 1);
        assert_refused(&fixture, &["set", "doc.md", "author", "{name:\"Ann\"}", "--save"], 1);
        set_saved(&fixture, "author", "{name:\"Ann\",email:\"a@x\"}");
        set_saved(&fixture, "author.name", "Bob");
        assert_eq!(get_json(&fixture, "author"), json!({ "name": "Bob", "email": "a@x" }));
    }

    #[test]
    fn a_literal_tag_narrows_the_declared_type() {
        let fixture = with_document("validation-union", "---\n$schema: ./s.yaml\nthing:\n  kind: rust\n---\n# Body\n");
        fixture.write_file(
            "cwd/s.yaml",
            "$schema:\n  thing: Rust@this | Go@this\ntypes:\n  Rust:\n    kind: literal(rust; required)\n    edition: string\n  Go:\n    kind: literal(go; required)\n    edition: number\n",
        );
        // Under `kind: rust` the edition is text, and under `kind: go` a number.
        set_saved(&fixture, "thing.edition", "1.10");
        assert_eq!(get_json(&fixture, "thing.edition"), json!("1.10"));
        let (code, _, stderr) = run_md(&fixture, &["rm", "doc.md", "thing.edition"]);
        assert_eq!(code, Some(0), "{stderr}");
        set_saved(&fixture, "thing.kind", "go");
        set_saved(&fixture, "thing.edition", "1.10");
        assert_eq!(get_json(&fixture, "thing.edition"), json!(1.1));
    }

    #[test]
    fn a_trigger_selected_by_the_edit_reads_the_value_and_a_broken_trigger_is_only_a_warning() {
        let fixture = with_document("validation-trigger", "---\nedition: 1.5\nlang: rust\n---\n# Body\n");
        fixture.write_file("cwd/.git/HEAD", "ref: refs/heads/main\n");
        fixture.write_file("cwd/.git/config", "[core]\n\trepositoryformatversion = 0\n");
        std::fs::create_dir_all(fixture.cwd().join(".git/objects")).unwrap();
        std::fs::create_dir_all(fixture.cwd().join(".git/refs")).unwrap();
        fixture.write_file(
            "cwd/schemas/lang.trigger.yaml",
            "kind: schema-trigger\nmatch:\n  lang: enum(rust; required)\n$schema:\n  edition: string\n",
        );
        set_saved(&fixture, "edition", "1.10");
        assert_eq!(get_json(&fixture, "edition"), json!("1.10"));

        // Leaving the trigger's match leaves its schema: the key is undeclared.
        set_saved(&fixture, "lang", "go");
        set_saved(&fixture, "edition", "1.10");
        assert_eq!(get_json(&fixture, "edition"), json!(1.1));

        fixture.write_file("cwd/schemas/broken.trigger.yaml", "kind: schema-trigger\n");
        let (code, stdout, stderr) = run_md(&fixture, &["set", "doc.md", "other", "1", "--output", "json"]);
        assert_eq!(code, Some(0), "{stdout}{stderr}");
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["diagnostics"][0]["code"], "dm.schema.invalid_trigger", "{report:#}");
    }
}

mod rm_paths {
    use super::*;
    use serde_json::json;

    #[test]
    fn removing_a_nested_key_keeps_its_siblings_and_their_comments() {
        let source = "---\nstyle:\n  # about\n  a: 1 # kept\n  b: 2\n---\n# Body\n";
        let fixture = with_document("paths-rm-nested", source);
        fixture.command().args(["rm", "doc.md", "style.b"]).assert().success();
        assert_eq!(read_document(&fixture), source.replace("  b: 2\n", ""));
    }

    #[test]
    fn removing_the_last_child_leaves_the_parent() {
        let fixture = with_document("paths-rm-last-child", "---\nstyle:\n  page:\n    max-width: 96\nitems: [only]\n---\n# Body\n");
        let (code, stdout, stderr) = run_md(&fixture, &["rm", "doc.md", "style.page.max-width", "items.0", "--output", "json"]);
        assert_eq!(code, Some(0), "{stderr}");
        let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
        assert_eq!(report["removed"], json!(["style.page.max-width", "items.0"]));
        assert_eq!(report["remaining"], json!(["style", "items"]), "top-level keys only");
        assert_eq!(get_json(&fixture, "style"), json!({ "page": {} }));
        assert_eq!(get_json(&fixture, "items"), json!([]));
    }

    #[test]
    fn array_removals_use_original_indexes_in_either_order() {
        for (name, order) in [("paths-rm-order-a", ["items.0", "items.2"]), ("paths-rm-order-b", ["items.2", "items.0"])] {
            let fixture = with_document(name, "---\nitems:\n  - a\n  - b\n  - c\n  - d\n---\n# Body\n");
            let (code, stdout, stderr) = run_md(&fixture, &["rm", "doc.md", order[0], order[1], "--output", "json"]);
            assert_eq!(code, Some(0), "{stderr}");
            let report: serde_json::Value = serde_json::from_str(&stdout).unwrap();
            assert_eq!(report["removed"], json!(order), "argument order");
            assert_eq!(get_json(&fixture, "items"), json!(["b", "d"]));
        }
    }

    #[test]
    fn a_duplicate_or_an_ancestor_with_its_descendant_cancels_the_command() {
        let fixture = nested_document("paths-rm-conflict");
        for args in [
            ["style.page", "style.page"],
            ["style", "style.page.alignment"],
            ["style.page.alignment", "style"],
            ["items.0", "items.0.name"],
        ] {
            let stderr = assert_refused(&fixture, &["rm", "doc.md", args[0], args[1]], 1);
            assert!(stderr.contains(args[0]) && stderr.contains(args[1]), "{args:?}: {stderr}");
        }
    }

    #[test]
    fn a_missing_target_cancels_the_whole_command() {
        let fixture = nested_document("paths-rm-missing");
        let stderr = assert_refused(&fixture, &["rm", "doc.md", "title", "style.absent"], 1);
        assert!(stderr.contains("style.absent"), "{stderr}");
        // The literal key is not reached by the dotted spelling.
        assert_refused(&fixture, &["rm", "doc.md", "dotted.key"], 1);
    }

    #[test]
    fn an_index_out_of_range_or_a_scalar_parent_is_an_error() {
        let fixture = nested_document("paths-rm-range");
        let stderr = assert_refused(&fixture, &["rm", "doc.md", "items.9"], 1);
        assert!(stderr.contains("items.9") && stderr.contains("2 element"), "{stderr}");
        assert_refused(&fixture, &["rm", "doc.md", "items.01"], 1);
        let stderr = assert_refused(&fixture, &["rm", "doc.md", "author.name"], 1);
        assert!(stderr.contains("author"), "{stderr}");
        assert_refused(&fixture, &["rm", "doc.md", "nothing.deep"], 1);
    }

    #[test]
    fn removing_an_anchor_an_alias_needs_is_refused() {
        let fixture = with_document("paths-rm-alias", "---\nbase: &shared\n  name: Ann\ncopy: *shared\n---\n# Body\n");
        assert_refused(&fixture, &["rm", "doc.md", "base"], 1);
    }

    #[test]
    fn the_empty_key_and_a_required_key_can_be_removed() {
        let fixture = with_document(
            "paths-rm-structural",
            "---\n$schema:\n  title: string(required)\ntitle: Notes\n\"\": empty\n---\n# Body\n",
        );
        fixture.command().args(["rm", "doc.md", "", "title"]).assert().success();
        assert_eq!(get_json(&fixture, "title"), json!(""));
        assert_eq!(get_json(&fixture, ""), json!(""));
    }
}
