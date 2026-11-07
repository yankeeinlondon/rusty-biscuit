//! The output contract of every command except `md compose` and `md render`,
//! through the real binary: each command's data output against its schema file
//! (success, failure, warnings), then its streams, exit statuses, defaults,
//! `--width`, and `--output` spellings. The clap definitions, the leaf table,
//! the schema files, and the conformance tests are held together by
//! [`every_leaf_has_a_schema_file_and_a_conformance_test`].
//!
//! `md edit` runs a recorder as `EDITOR`; nothing here launches an application
//! or takes focus.

use crate::common;

use clap::CommandFactory as _;
use common::CliProcessFixture;
use darkmatter_cli::args::output::{COMMAND_LEAVES, CommandLeaf};
use serde_json::{Value, json};
use simplified_schema::SimplifiedSchema;
use std::process::Output;

const GOOD: &str = "---\ntitle: T\ntags:\n  - a\n  - b\n---\n# Head\n\none two\nthree four\n\n## Sub\n\nbody\n";
const BROKEN: &str = "---\ntitle: T\n---\n# Head\n\nsee [gone](./nope.md)\n";
const LEGACY_WIDE: &str = "---\ntitle: T\n---\n# Head\n\none two three four five six seven eight nine ten\n";

fn stdout_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// A fixture holding `content` as `doc.md`, in the launch directory.
fn with_document(name: &str, content: &str) -> CliProcessFixture {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/doc.md", content);
    fixture
}

/// Marks the launch directory a repository without spawning `git`.
fn init_repository(fixture: &CliProcessFixture) {
    fixture.write_file("cwd/.git/HEAD", "ref: refs/heads/main\n");
    fixture.write_file("cwd/.git/config", "[core]\n\trepositoryformatversion = 0\n");
    std::fs::create_dir_all(fixture.cwd().join(".git/objects")).expect("objects");
    std::fs::create_dir_all(fixture.cwd().join(".git/refs")).expect("refs");
}

fn read_document(fixture: &CliProcessFixture) -> String {
    std::fs::read_to_string(fixture.cwd().join("doc.md")).expect("the document")
}

fn run(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    fixture.command().args(args).output().expect("md runs")
}

/// Stdout as the one JSON value it must be, with stderr empty.
fn one_value(output: &Output) -> Value {
    assert!(output.stderr.is_empty(), "a data output owns stderr too: {}", stderr_of(output));
    let text = stdout_of(output);
    let mut stream = serde_json::Deserializer::from_str(&text).into_iter::<Value>();
    let value = stream.next().expect("a value").unwrap_or_else(|error| panic!("{error}: {text}"));
    assert!(stream.next().is_none(), "one value only: {text}");
    value
}

/// Judges `value` against a schema file. A non-object root is judged as the
/// `value` field TOML would write it under, since a schema's root is an object.
fn validate_strictly(schema_text: &str, value: &Value) {
    let schema = SimplifiedSchema::try_from(schema_text).expect("the schema file parses");
    let subject = if value.is_object() { value.clone() } else { json!({ "value": value }) };
    if let Err(failure) = schema.validate_strict(&subject) {
        panic!("{failure:?}\n{subject:#}");
    }
}

/// Runs `args` and returns the value, having checked its status and its schema.
fn conforms(schema_text: &str, fixture: &CliProcessFixture, args: &[&str], status: i32) -> Value {
    let output = run(fixture, args);
    assert_eq!(output.status.code(), Some(status), "{}\n{}", stdout_of(&output), stderr_of(&output));
    let value = one_value(&output);
    validate_strictly(schema_text, &value);
    value
}

fn first_code(value: &Value) -> &str {
    value["diagnostics"][0]["code"].as_str().unwrap_or_default()
}

// Each conformance test spells its schema file itself: CI's test-input index
// maps an `include_str!` to the test holding it, and a helper or macro would
// widen a schema edit to the whole binary.

#[test]
fn conforms_clean_success() {
    let fixture = with_document("output-clean-success", GOOD);
    let value = conforms(include_str!("../../schemas/output/clean.yaml"), &fixture, &["clean", "doc.md", "--output", "json"], 0);
    assert_eq!(value["version"], 1);
    assert!(value.get("outcome").is_none(), "{value:#}");
}

#[test]
fn conforms_clean_failure_keeps_the_report_and_gains_its_outcome() {
    let fixture = with_document("output-clean-invalid", "---\ntitle: [unclosed\n---\n# Body\n");
    let value = conforms(include_str!("../../schemas/output/clean.yaml"), &fixture, &["clean", "doc.md", "--output", "json"], 1);
    assert_eq!(value["version"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["changed"], false);
}

#[test]
fn conforms_clean_with_a_run_warning_keeps_its_own_diagnostics() {
    let fixture = with_document("output-clean-warning", GOOD);
    let value =
        conforms(include_str!("../../schemas/output/clean.yaml"), &fixture, &["clean", "doc.md", "--output", "json", "--width", "9.5"], 0);
    assert_eq!(value["outcome"], "success");
    assert_eq!(value["diagnostics"], json!([]));
    assert_eq!(value["run_diagnostics"][0]["code"], "dm.output.fractional_width");
}

#[test]
fn conforms_clean_missing_input() {
    let fixture = CliProcessFixture::named("output-clean-missing");
    let value = conforms(include_str!("../../schemas/output/clean.yaml"), &fixture, &["clean", "missing.md", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["value"], Value::Null);
}

#[test]
fn conforms_toc_success() {
    let fixture = with_document("output-toc-success", GOOD);
    let value = conforms(include_str!("../../schemas/output/toc.yaml"), &fixture, &["toc", "doc.md", "--output", "json"], 0);
    assert_eq!(value["structure"][0]["title"], "Head");
    assert!(value.get("outcome").is_none());
}

#[test]
fn conforms_toc_missing_input() {
    let fixture = CliProcessFixture::named("output-toc-missing");
    let value = conforms(include_str!("../../schemas/output/toc.yaml"), &fixture, &["toc", "missing.md", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
}

/// A TOML integer is signed 64-bit, and a TOC's hashes are unsigned: those above
/// the range arrive as decimal text, so either form must carry the JSON's digits.
#[test]
fn toc_toml_carries_its_hashes_without_failing() {
    let fixture = with_document("output-toc-toml", GOOD);
    let json = conforms(include_str!("../../schemas/output/toc.yaml"), &fixture, &["toc", "doc.md", "--output", "json"], 0);
    let output = run(&fixture, &["toc", "doc.md", "--output", "toml"]);
    assert_eq!(output.status.code(), Some(0), "{}", stdout_of(&output));
    let parsed: toml::Value = toml::from_str(&stdout_of(&output)).unwrap();
    for hash in ["page_hash", "page_hash_trimmed", "preamble_hash"] {
        assert_eq!(parsed[hash].to_string().trim_matches('"'), json[hash].to_string(), "{hash}");
    }
    assert_eq!(parsed["structure"][0]["title"].as_str(), Some("Head"));
}

#[test]
fn conforms_toc_fractional_width() {
    let fixture = with_document("output-toc-fractional", GOOD);
    let value =
        conforms(include_str!("../../schemas/output/toc.yaml"), &fixture, &["toc", "doc.md", "--output", "json", "--width", "9.5"], 0);
    assert_eq!(value["outcome"], "success");
    assert_eq!(first_code(&value), "dm.output.fractional_width");
}

#[test]
fn conforms_delta_success() {
    let fixture = with_document("output-delta-success", GOOD);
    fixture.write_file("cwd/other.md", BROKEN);
    let value = conforms(include_str!("../../schemas/output/delta.yaml"), &fixture, &["delta", "doc.md", "other.md", "--output", "json"], 0);
    assert!(value["classification"].is_string(), "{value:#}");
}

#[test]
fn conforms_delta_missing_input() {
    let fixture = with_document("output-delta-missing", GOOD);
    let value = conforms(include_str!("../../schemas/output/delta.yaml"), &fixture, &["delta", "doc.md", "gone.md", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
}

#[test]
fn conforms_get_scalar_is_unchanged() {
    let fixture = with_document("output-get-scalar", GOOD);
    let value = conforms(include_str!("../../schemas/output/get.yaml"), &fixture, &["get", "doc.md", "title", "--output", "json"], 0);
    assert_eq!(value, json!("T"));
}

#[test]
fn conforms_get_several_keys_with_a_missing_one_as_empty() {
    let fixture = with_document("output-get-keys", GOOD);
    let value =
        conforms(include_str!("../../schemas/output/get.yaml"), &fixture, &["get", "doc.md", "title", "nope", "--output", "json"], 0);
    assert_eq!(value, json!({ "title": "T", "nope": "" }));
}

#[test]
fn conforms_get_failure_has_a_null_value() {
    let fixture = CliProcessFixture::named("output-get-failure");
    let value = conforms(include_str!("../../schemas/output/get.yaml"), &fixture, &["get", "missing.md", "title"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["value"], Value::Null);
}

/// The class a formatted error prints as its `failure:` row is what a script
/// reads from a data output.
#[test]
fn a_file_argument_failure_carries_its_class_in_data() {
    let fixture = CliProcessFixture::named("output-argument-class");
    let missing = conforms(include_str!("../../schemas/output/get.yaml"), &fixture, &["get", "missing.md", "title"], 1);
    assert_eq!(missing["diagnostics"][0]["code"], "dm.input.read_failure");
    assert_eq!(missing["diagnostics"][0]["failure"], "no-match");
    assert_eq!(missing["diagnostics"][0]["reference"], "missing.md");

    let invalid = conforms(include_str!("../../schemas/output/compose.yaml"), &fixture, &["compose", "@", "--output", "json"], 1);
    assert_eq!(invalid["diagnostics"][0]["failure"], "invalid-reference");
    assert_eq!(invalid["diagnostics"][0]["reference"], "@");

    let detect = conforms(include_str!("../../schemas/output/schema-detect.yaml"), &fixture, &["schema", "detect", "@", "--output", "json"], 1);
    assert_eq!(detect["diagnostics"][0]["failure"], "invalid-reference");
}

#[test]
fn conforms_get_warning_wraps_a_scalar() {
    let fixture = with_document("output-get-warning", GOOD);
    let value =
        conforms(include_str!("../../schemas/output/get.yaml"), &fixture, &["get", "doc.md", "title", "--output", "json", "--width", "9.5"], 0);
    assert_eq!(value["outcome"], "success");
    assert_eq!(value["value"], "T");
    assert_eq!(first_code(&value), "dm.output.fractional_width");
}

#[test]
fn conforms_set_keeps_the_file_without_save() {
    let fixture = with_document("output-set-success", GOOD);
    let value = conforms(include_str!("../../schemas/output/set.yaml"), &fixture, &["set", "doc.md", "extra", "1", "--output", "json"], 0);
    assert_eq!(value["outcome"], "success");
    assert_eq!(value["property"], "extra");
    assert_eq!(value["value"], 1);
    assert_eq!(value["saved"], false);
    assert!(value["document"].as_str().unwrap().contains("extra: 1"), "{value:#}");
    assert!(!read_document(&fixture).contains("extra"));
}

#[test]
fn conforms_set_save_reports_the_document_it_wrote() {
    let fixture = with_document("output-set-save", GOOD);
    let value =
        conforms(include_str!("../../schemas/output/set.yaml"), &fixture, &["set", "doc.md", "extra", "1", "--save", "--output", "json"], 0);
    assert_eq!(value["saved"], true);
    assert_eq!(value["document"].as_str().unwrap(), read_document(&fixture));
}

#[test]
fn conforms_set_failure_names_what_it_tried() {
    let fixture = CliProcessFixture::named("output-set-failure");
    let value = conforms(include_str!("../../schemas/output/set.yaml"), &fixture, &["set", "missing.md", "a", "1", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["property"], "a");
    assert_eq!(value["document"], Value::Null);
    assert_eq!(value["saved"], false);
}

#[test]
fn conforms_rm_success_is_unchanged() {
    let fixture = with_document("output-rm-success", GOOD);
    let value = conforms(include_str!("../../schemas/output/rm.yaml"), &fixture, &["rm", "doc.md", "tags", "--output", "json"], 0);
    assert_eq!(value["removed"], json!(["tags"]));
    assert_eq!(value["remaining"], json!(["title"]));
    assert!(value.get("outcome").is_none());
}

#[test]
fn conforms_rm_missing_property_fails() {
    let fixture = with_document("output-rm-failure", GOOD);
    let value = conforms(include_str!("../../schemas/output/rm.yaml"), &fixture, &["rm", "doc.md", "nope", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(read_document(&fixture), GOOD, "a failed removal writes nothing");
}

#[test]
fn conforms_get_nested_paths_stay_flat() {
    let fixture = with_document("output-get-nested", "---\nauthor:\n  name: Alice\ntitle: Notes\n---\n# Doc\n");
    let value = conforms(
        include_str!("../../schemas/output/get.yaml"),
        &fixture,
        &["get", "doc.md", "author.name", "title", "--output", "json"],
        0,
    );
    assert_eq!(value, json!({ "author.name": "Alice", "title": "Notes" }));
}

#[test]
fn conforms_set_nested_path_is_written_as_given() {
    let fixture = with_document("output-set-nested", GOOD);
    let value = conforms(
        include_str!("../../schemas/output/set.yaml"),
        &fixture,
        &["set", "doc.md", "extra.deep", "1", "--output", "json"],
        0,
    );
    assert_eq!(value["property"], "extra.deep");
    assert_eq!(value["value"], 1);
}

#[test]
fn conforms_set_rejected_value_has_no_document() {
    let source = "---\n$schema:\n  count: number\n---\n# Doc\n";
    let fixture = with_document("output-set-rejected", source);
    let value = conforms(
        include_str!("../../schemas/output/set.yaml"),
        &fixture,
        &["set", "doc.md", "count", "abc", "--save", "--output", "json"],
        1,
    );
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["document"], Value::Null);
    assert_eq!(read_document(&fixture), source);
}

#[test]
fn conforms_rm_nested_path_keeps_top_level_remaining() {
    let fixture = with_document("output-rm-nested", "---\nauthor:\n  name: Alice\ntitle: Notes\n---\n# Doc\n");
    let value = conforms(include_str!("../../schemas/output/rm.yaml"), &fixture, &["rm", "doc.md", "author.name", "--output", "json"], 0);
    assert_eq!(value["removed"], json!(["author.name"]));
    assert_eq!(value["remaining"], json!(["author", "title"]));
}

#[test]
fn conforms_rm_conflict_fails() {
    let source = "---\nauthor:\n  name: Alice\n---\n# Doc\n";
    let fixture = with_document("output-rm-conflict", source);
    let value = conforms(include_str!("../../schemas/output/rm.yaml"), &fixture, &["rm", "doc.md", "author", "author.name", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(read_document(&fixture), source);
}

/// `md edit` with the argv recorder as its editor.
#[cfg(unix)]
fn edit(fixture: &CliProcessFixture, args: &[&str]) -> Output {
    use std::os::unix::fs::PermissionsExt as _;
    let editor = fixture.write_file("cwd/fake-editor.sh", "#!/bin/sh\nprintf 'edited\\n' >> \"$1\"\n");
    std::fs::set_permissions(&editor, std::fs::Permissions::from_mode(0o755)).unwrap();
    fixture.command().env("EDITOR", editor).args(args).output().expect("md runs")
}

#[cfg(unix)]
#[test]
fn conforms_edit_reports_a_created_file() {
    let fixture = CliProcessFixture::named("output-edit-created");
    let output = edit(&fixture, &["edit", "new.md", "--output", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let value = one_value(&output);
    validate_strictly(include_str!("../../schemas/output/edit.yaml"), &value);
    assert_eq!(value["created"], true);
    assert!(value["editor"].as_str().unwrap().ends_with("fake-editor.sh"), "{value:#}");
    assert!(value["file"].as_str().unwrap().ends_with("new.md"), "{value:#}");
}

#[cfg(unix)]
#[test]
fn conforms_edit_reports_an_existing_file() {
    let fixture = with_document("output-edit-existing", GOOD);
    let output = edit(&fixture, &["edit", "doc.md", "--output", "json"]);
    let value = one_value(&output);
    validate_strictly(include_str!("../../schemas/output/edit.yaml"), &value);
    assert_eq!(value["created"], false);
    assert_eq!(value["outcome"], "success");
}

#[test]
fn conforms_edit_failure_launches_nothing() {
    let fixture = CliProcessFixture::named("output-edit-failure");
    let output = fixture.command().env("EDITOR", "definitely-not-an-editor").env("VISUAL", "definitely-not-an-editor").args(["edit", "%none.md", "--output", "json"]).output().expect("md runs");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    let value = one_value(&output);
    validate_strictly(include_str!("../../schemas/output/edit.yaml"), &value);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["file"], Value::Null);
}

#[test]
fn conforms_hash_compute() {
    let fixture = with_document("output-hash-compute", GOOD);
    let value = conforms(include_str!("../../schemas/output/hash.yaml"), &fixture, &["hash", "doc.md", "--output", "json"], 0);
    assert_eq!(value["mode"], "compute");
    assert_eq!(value["kind"], "simple");
    assert!(value["computed"]["value"].is_string());
    assert_eq!(value["saved"], false);
}

#[test]
fn conforms_hash_diff_with_nothing_stored_fails() {
    let fixture = with_document("output-hash-no-stored", GOOD);
    let value = conforms(include_str!("../../schemas/output/hash.yaml"), &fixture, &["hash", "doc.md", "--diff", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(first_code(&value), "dm.hash.no_stored_hash");
    assert_eq!(value["comparison"], Value::Null);
}

#[test]
fn conforms_hash_save_then_diff_round_trips() {
    let fixture = with_document("output-hash-save", GOOD);
    let saved = conforms(include_str!("../../schemas/output/hash.yaml"), &fixture, &["hash", "doc.md", "--save", "--output", "json"], 0);
    assert_eq!(saved["mode"], "save");
    assert_eq!(saved["saved"], true);

    let same = conforms(include_str!("../../schemas/output/hash.yaml"), &fixture, &["hash", "doc.md", "--diff", "--output", "json"], 0);
    assert_eq!(same["comparison"]["body_changed"], false);
    assert_eq!(same["comparison"]["detail"]["level"], "coarse");
    assert_eq!(same["stored"]["value"], same["computed"]["value"]);

    let written = read_document(&fixture);
    fixture.write_file("cwd/doc.md", &written.replace("body", "changed body"));
    let changed = conforms(include_str!("../../schemas/output/hash.yaml"), &fixture, &["hash", "doc.md", "--diff", "--output", "json"], 1);
    assert_eq!(changed["outcome"], "failure");
    assert_eq!(first_code(&changed), "dm.hash.changed");
    assert_eq!(changed["comparison"]["body_changed"], true);
}

#[test]
fn conforms_hash_directory() {
    let fixture = with_document("output-hash-directory", GOOD);
    let value = conforms(include_str!("../../schemas/output/hash.yaml"), &fixture, &["hash", ".", "--output", "json"], 0);
    assert_eq!(value["mode"], "directory");
    assert!(value["computed"]["value"].is_string());
}

#[test]
fn conforms_hash_detailed_writes_its_mixed_tuples_as_toml() {
    let fixture = with_document("output-hash-detailed", GOOD);
    let value = conforms(include_str!("../../schemas/output/hash.yaml"), &fixture, &["hash", "doc.md", "--kind", "detailed", "--output", "json"], 0);
    assert_eq!(value["computed"]["value"]["sections"][0][1], "Head");
    let toml = run(&fixture, &["hash", "doc.md", "--kind", "detailed", "--output", "toml"]);
    assert_eq!(toml.status.code(), Some(0), "{}", stdout_of(&toml));
    assert!(toml.stderr.is_empty(), "{}", stderr_of(&toml));
    let parsed = biscuit_file::Toml::from_str(stdout_of(&toml)).expect("TOML parses");
    let round_trip = serde_json::to_value(parsed.value()).unwrap();
    assert_eq!(round_trip["computed"]["value"]["sections"][0][1], "Head");
    assert_eq!(round_trip.get("stored"), None, "a null field is omitted");
}

#[test]
fn conforms_hash_missing_input() {
    let fixture = CliProcessFixture::named("output-hash-missing");
    let value = conforms(include_str!("../../schemas/output/hash.yaml"), &fixture, &["hash", "missing.md", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["computed"], Value::Null);
}

#[test]
fn conforms_code_block_success() {
    let fixture = CliProcessFixture::named("output-code-success");
    let value = conforms(
        include_str!("../../schemas/output/code-block.yaml"),
        &fixture,
        &["code-block", "--content", "x = 1", "--language", "py", "--title", "T", "--line-numbering", "--highlight", "1,3-4", "--output", "json"],
        0,
    );
    assert_eq!(value["code"], "x = 1");
    assert_eq!(value["language"], "python");
    assert_eq!(value["title"], "T");
    assert_eq!(value["line_numbering"], true);
    assert_eq!(value["highlight"], json!([{ "start": 1, "end": 1 }, { "start": 3, "end": 4 }]));
}

#[test]
fn conforms_code_block_failure() {
    let fixture = CliProcessFixture::named("output-code-failure");
    let value =
        conforms(include_str!("../../schemas/output/code-block.yaml"), &fixture, &["code-block", "missing.rs", "--file", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["code"], Value::Null);
}

#[test]
fn conforms_graph_success() {
    let fixture = with_document("output-graph-success", GOOD);
    let value = conforms(include_str!("../../schemas/output/graph.yaml"), &fixture, &["graph", "doc.md", "--output", "json"], 0);
    assert!(value["references"].is_array());
    assert!(value.get("outcome").is_none());
}

#[test]
fn conforms_graph_validate_failure() {
    let fixture = with_document("output-graph-failure", BROKEN);
    let value = conforms(include_str!("../../schemas/output/graph.yaml"), &fixture, &["graph", "doc.md", "--validate", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["validation"]["valid"], false);
    assert_eq!(first_code(&value), "dm.reference.missing_hyperlink");
    assert_eq!(value["diagnostics"][0]["source"]["line"], 6);
}

#[test]
fn conforms_graph_missing_input() {
    let fixture = CliProcessFixture::named("output-graph-missing");
    let value = conforms(include_str!("../../schemas/output/graph.yaml"), &fixture, &["graph", "missing.md", "--output", "json"], 1);
    assert_eq!(value["value"], Value::Null);
}

#[test]
fn conforms_validate_refs_success() {
    let fixture = with_document("output-refs-success", GOOD);
    let value = conforms(include_str!("../../schemas/output/validate-refs.yaml"), &fixture, &["validate", "refs", "doc.md", "--output", "json"], 0);
    assert_eq!(value["valid"], true);
    assert!(value.get("outcome").is_none());
    assert!(value.get("references").is_none());
}

#[test]
fn conforms_validate_refs_failure_has_one_diagnostic_per_problem() {
    let fixture = with_document("output-refs-failure", BROKEN);
    let value = conforms(include_str!("../../schemas/output/validate-refs.yaml"), &fixture, &["validate", "refs", "doc.md", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["diagnostics"].as_array().unwrap().len(), 1);
    assert_eq!(value["issues"].as_array().unwrap().len(), 1);
}

#[test]
fn conforms_validate_refs_show_all_lists_references() {
    let fixture = with_document("output-refs-show-all", BROKEN);
    let value =
        conforms(include_str!("../../schemas/output/validate-refs.yaml"), &fixture, &["validate", "refs", "doc.md", "--show-all", "--output", "json"], 1);
    assert_eq!(value["references"].as_array().unwrap().len(), 1);
}

#[test]
fn conforms_validate_refs_graph_is_the_diagram_alone() {
    let fixture = with_document("output-refs-graph", BROKEN);
    let value =
        conforms(include_str!("../../schemas/output/validate-refs.yaml"), &fixture, &["validate", "refs", "doc.md", "--graph", "dot", "--output", "json"], 0);
    assert_eq!(value["graph"]["format"], "dot");
    assert!(value["graph"]["source"].as_str().unwrap().contains("digraph"), "{value:#}");
    assert!(value.get("issues").is_none(), "--graph validates nothing");
}

#[test]
fn conforms_validate_refs_missing_input() {
    let fixture = CliProcessFixture::named("output-refs-missing");
    let value = conforms(include_str!("../../schemas/output/validate-refs.yaml"), &fixture, &["validate", "refs", "missing.md", "--output", "json"], 1);
    assert_eq!(value["value"], Value::Null);
}

#[test]
fn conforms_schema_validate_success() {
    let fixture = with_document("output-validate-success", GOOD);
    let value = conforms(include_str!("../../schemas/output/schema-validate.yaml"), &fixture, &["schema", "validate", "doc.md", "--output", "json"], 0);
    assert_eq!(value["files"][0]["valid"], true);
    assert!(value.get("outcome").is_none());
}

#[test]
fn conforms_schema_validate_keeps_a_failing_and_an_unopened_file_apart() {
    let fixture = with_document("output-validate-failure", "---\n$schema:\n  name: string(required)\n---\n# H\n");
    fixture.write_file("cwd/good.md", GOOD);
    let value = conforms(
        include_str!("../../schemas/output/schema-validate.yaml"),
        &fixture,
        &["schema", "validate", "doc.md", "missing.md", "good.md", "--quiet", "--output", "json"],
        1,
    );
    assert_eq!(value["outcome"], "failure");
    let files = value["files"].as_array().unwrap();
    assert_eq!(files.len(), 3, "--quiet removes no data record: {value:#}");
    assert_eq!(files[0]["problems"][0]["kind"], "missing");
    assert_eq!(files[1]["error"], "file_reference");
    assert_eq!(files[2]["valid"], true);
}

#[test]
fn conforms_schema_validate_unloadable_baseline() {
    let fixture = with_document("output-validate-baseline", GOOD);
    let value = conforms(
        include_str!("../../schemas/output/schema-validate.yaml"),
        &fixture,
        &["schema", "validate", "doc.md", "--schema", "missing.yaml", "--output", "json"],
        1,
    );
    assert_eq!(value["files"], json!([]));
    assert_eq!(first_code(&value), "dm.schema.baseline");
}

#[test]
fn conforms_schema_validate_fractional_width() {
    let fixture = with_document("output-validate-warning", GOOD);
    let value = conforms(include_str!("../../schemas/output/schema-validate.yaml"), &fixture, &["schema", "validate", "doc.md", "--output", "json", "--width", "1.5"], 0);
    assert_eq!(value["outcome"], "success");
    assert_eq!(first_code(&value), "dm.output.fractional_width");
}

#[test]
fn conforms_schema_detect_single_is_the_schema() {
    let fixture = with_document("output-detect-single", GOOD);
    let value = conforms(include_str!("../../schemas/output/schema-detect.yaml"), &fixture, &["schema", "detect", "doc.md", "--output", "json"], 0);
    assert_eq!(value["$schema"]["title"], "string");
}

#[test]
fn conforms_schema_detect_several_files_in_input_order() {
    let fixture = with_document("output-detect-several", GOOD);
    fixture.write_file("cwd/other.md", "---\ncount: 1\n---\n# O\n");
    let value = conforms(include_str!("../../schemas/output/schema-detect.yaml"), &fixture, &["schema", "detect", "doc.md", "other.md", "--output", "json"], 0);
    assert_eq!(value["files"][0]["file"], "doc.md");
    assert_eq!(value["files"][1]["file"], "other.md");
    assert!(value["files"][1]["schema"]["$schema"]["count"].is_string(), "{value:#}");
}

#[test]
fn conforms_schema_detect_merge_is_one_schema() {
    let fixture = with_document("output-detect-merge", GOOD);
    fixture.write_file("cwd/other.md", "---\ncount: 1\n---\n# O\n");
    let value = conforms(include_str!("../../schemas/output/schema-detect.yaml"), &fixture, &["schema", "detect", "doc.md", "other.md", "--merge", "--output", "json"], 0);
    assert!(value.get("files").is_none());
    assert!(value["$schema"]["count"].is_string(), "{value:#}");
}

#[test]
fn conforms_schema_detect_failure_reports_every_input() {
    let fixture = with_document("output-detect-failure", "---\ntitle: [unclosed\n---\n# Body\n");
    let value = conforms(include_str!("../../schemas/output/schema-detect.yaml"), &fixture, &["schema", "detect", "doc.md", "missing.md", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["value"], Value::Null);
    assert_eq!(value["diagnostics"].as_array().unwrap().len(), 2, "{value:#}");
}

#[test]
fn conforms_schema_triggers_success() {
    let fixture = with_document("output-triggers-success", GOOD);
    init_repository(&fixture);
    let value = conforms(include_str!("../../schemas/output/schema-triggers.yaml"), &fixture, &["schema", "triggers", "doc.md", "--output", "json"], 0);
    assert_eq!(value["outcome"], "success");
    assert_eq!(value["roots"].as_array().unwrap().len(), 5);
    assert_eq!(value["roots"][0]["kind"], "SCHEMAS_DIR");
}

#[test]
fn conforms_schema_triggers_outside_a_repository_fails() {
    let fixture = with_document("output-triggers-failure", GOOD);
    let value = conforms(include_str!("../../schemas/output/schema-triggers.yaml"), &fixture, &["schema", "triggers", "doc.md", "--output", "json"], 1);
    assert_eq!(value["outcome"], "failure");
    assert_eq!(value["roots"], json!([]));
}

#[test]
fn conforms_schema_triggers_reports_a_skipped_trigger_as_a_warning() {
    let fixture = with_document("output-triggers-warning", GOOD);
    init_repository(&fixture);
    fixture.write_file("cwd/schemas/broken.yaml", "kind: schema-trigger\n$match: [unclosed\n");
    let value = conforms(include_str!("../../schemas/output/schema-triggers.yaml"), &fixture, &["schema", "triggers", "doc.md", "--output", "json"], 0);
    assert_eq!(value["outcome"], "success");
    assert_eq!(value["skipped"].as_array().unwrap().len(), 1, "{value:#}");
    assert_eq!(value["diagnostics"][0]["severity"], "warning");
    assert_eq!(value["diagnostics"][0]["code"], value["skipped"][0]["code"]);
}

#[test]
fn conforms_schema_about_carries_the_whole_catalog_at_any_verbosity() {
    let fixture = CliProcessFixture::named("output-about");
    let quiet = conforms(include_str!("../../schemas/output/schema-about.yaml"), &fixture, &["schema", "about", "--output", "json"], 0);
    let verbose = conforms(include_str!("../../schemas/output/schema-about.yaml"), &fixture, &["schema", "about", "-v", "--output", "json"], 0);
    assert_eq!(quiet, verbose);
    let ids: Vec<&str> = quiet["sections"].as_array().unwrap().iter().map(|section| section["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&"expression_functions"), "{ids:?}");
    assert!(ids.contains(&"context_variables"), "{ids:?}");
}

#[test]
fn every_schema_file_shares_one_diagnostic_type() {
    let compose = include_str!("../../schemas/output/compose.yaml");
    // A diagnostic's `data_location` is a named type of its own, shared too.
    let block = |text: &'static str| -> serde_json::Value {
        let document: serde_json::Value = serde_yaml_ng::from_str(text).expect("YAML");
        serde_json::json!({ "diagnostic": document["types"]["diagnostic"], "data-location": document["types"]["data-location"] })
    };
    let expected = block(compose);
    for (name, text) in [
        ("clean", include_str!("../../schemas/output/clean.yaml")),
        ("code-block", include_str!("../../schemas/output/code-block.yaml")),
        ("delta", include_str!("../../schemas/output/delta.yaml")),
        ("edit", include_str!("../../schemas/output/edit.yaml")),
        ("get", include_str!("../../schemas/output/get.yaml")),
        ("graph", include_str!("../../schemas/output/graph.yaml")),
        ("hash", include_str!("../../schemas/output/hash.yaml")),
        ("rm", include_str!("../../schemas/output/rm.yaml")),
        ("schema-about", include_str!("../../schemas/output/schema-about.yaml")),
        ("schema-detect", include_str!("../../schemas/output/schema-detect.yaml")),
        ("schema-triggers", include_str!("../../schemas/output/schema-triggers.yaml")),
        ("schema-validate", include_str!("../../schemas/output/schema-validate.yaml")),
        ("set", include_str!("../../schemas/output/set.yaml")),
        ("toc", include_str!("../../schemas/output/toc.yaml")),
        ("validate-refs", include_str!("../../schemas/output/validate-refs.yaml")),
    ] {
        let mut actual = block(text);
        let mut wanted = expected.clone();
        // Only the example in `code`'s description differs between files.
        actual["diagnostic"]["code"] = Value::Null;
        wanted["diagnostic"]["code"] = Value::Null;
        assert_eq!(actual, wanted, "{name}");
    }
}

// ---------------------------------------------------------------------------
// The guard: clap, the table, the schema files, and the tests above agree.
// ---------------------------------------------------------------------------

/// Every subcommand path that has no subcommands of its own.
fn clap_leaves() -> Vec<Vec<String>> {
    fn walk(command: &clap::Command, path: &mut Vec<String>, out: &mut Vec<Vec<String>>) {
        let subcommands: Vec<&clap::Command> = command.get_subcommands().collect();
        if subcommands.is_empty() {
            if !path.is_empty() {
                out.push(path.clone());
            }
            return;
        }
        for subcommand in subcommands {
            path.push(subcommand.get_name().to_string());
            walk(subcommand, path, out);
            path.pop();
        }
    }
    let mut out = Vec::new();
    walk(&darkmatter_cli::Cli::command(), &mut Vec::new(), &mut out);
    out.sort();
    out
}

fn leaf_command(leaf: &CommandLeaf) -> clap::Command {
    let mut command = darkmatter_cli::Cli::command();
    for name in leaf.path {
        command = command.find_subcommand(name).unwrap_or_else(|| panic!("{name}")).clone();
    }
    command
}

#[test]
fn every_leaf_has_a_schema_file_and_a_conformance_test() {
    let mut declared: Vec<Vec<String>> = COMMAND_LEAVES.iter().map(|leaf| leaf.path.iter().map(ToString::to_string).collect()).collect();
    declared.sort();
    assert_eq!(declared, clap_leaves(), "a new command needs a row in COMMAND_LEAVES");

    let sources = concat!(
        include_str!("output_commands.rs"),
        include_str!("output_envelope.rs"),
    );
    let directory = biscuit_test_harness::manifest_dir!().join("schemas/output");
    let mut files: Vec<String> = std::fs::read_dir(&directory)
        .expect("the schema directory")
        .map(|entry| entry.expect("an entry").file_name().to_string_lossy().into_owned())
        .collect();
    files.sort();
    let mut schemas: Vec<String> = COMMAND_LEAVES.iter().map(|leaf| leaf.schema.to_string()).collect();
    schemas.sort();
    assert_eq!(files, schemas, "every schema file belongs to exactly one command");

    for leaf in COMMAND_LEAVES {
        let spelled = format!("schemas/output/{}", leaf.schema);
        assert!(sources.contains(&spelled), "{} has no conformance test reading {spelled}", leaf.schema);
    }
}

#[test]
fn every_leaf_declares_its_output_and_width() {
    let root = darkmatter_cli::Cli::command();
    let width = root.get_arguments().find(|arg| arg.get_id() == "width").expect("--width");
    assert!(width.is_global_set(), "--width reaches every command");

    for leaf in COMMAND_LEAVES {
        let command = leaf_command(leaf);
        let output = command
            .get_arguments()
            .find(|arg| arg.get_id() == "output")
            .unwrap_or_else(|| panic!("{:?} has no --output", leaf.path));
        let values: Vec<String> = output.get_possible_values().iter().map(|value| value.get_name().to_string()).collect();
        for data in ["json", "json5", "yaml", "toml"] {
            assert!(values.iter().any(|value| value == data), "{:?}: {values:?}", leaf.path);
        }
        for formatted in ["terminal", "plain", "markdown-plus", "html"] {
            assert_eq!(values.iter().any(|value| value == formatted), leaf.formatted_family, "{:?}: {formatted}", leaf.path);
        }
        if let Some(extra) = leaf.extra_value {
            assert!(values.iter().any(|value| value == extra), "{:?}: {values:?}", leaf.path);
        }
    }
}

/// Runs one `--help` per leaf, spreading the spawns over nextest's cores.
macro_rules! help_shows_the_default {
    ($($name:ident => [$($word:literal),+], $default:literal;)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = CliProcessFixture::named(concat!("output-help-", stringify!($name)));
                let output = run(&fixture, &[$($word),+, "--help"]);
                assert_eq!(output.status.code(), Some(0));
                let text = stdout_of(&output);
                assert!(text.contains(concat!("[default: ", $default, "]")), "{text}");
                assert!(text.contains("--width"), "{text}");
                assert!(text.contains("--output"), "{text}");
            }
        )+

        #[test]
        fn the_help_rows_cover_every_leaf() {
            let rows: &[(&[&str], &str)] = &[$((&[$($word),+], $default)),+];
            for leaf in COMMAND_LEAVES.iter().filter(|leaf| leaf.path != ["render"]) {
                let row = rows.iter().find(|row| row.0 == leaf.path).unwrap_or_else(|| panic!("{:?}", leaf.path));
                assert_eq!(row.1, leaf.default_target.as_str(), "{:?}", leaf.path);
            }
            assert_eq!(rows.len(), COMMAND_LEAVES.len() - 1, "every leaf but render, which has no clap default");
        }
    };
}

help_shows_the_default! {
    help_compose => ["compose"], "markdown";
    help_clean => ["clean"], "markdown";
    help_set => ["set"], "markdown";
    help_get => ["get"], "json";
    help_toc => ["toc"], "terminal";
    help_delta => ["delta"], "terminal";
    help_rm => ["rm"], "terminal";
    help_edit => ["edit"], "terminal";
    help_hash => ["hash"], "terminal";
    help_graph => ["graph"], "terminal";
    help_code_block => ["code-block"], "terminal";
    help_validate_refs => ["validate", "refs"], "terminal";
    help_schema_validate => ["schema", "validate"], "terminal";
    help_schema_triggers => ["schema", "triggers"], "terminal";
    help_schema_detect => ["schema", "detect"], "terminal";
    help_schema_about => ["schema", "about"], "terminal";
}

#[test]
fn render_help_states_its_default() {
    let fixture = CliProcessFixture::named("output-help-render");
    let text = stdout_of(&run(&fixture, &["render", "--help"]));
    assert!(text.contains("default: terminal"), "{text}");
}

// ---------------------------------------------------------------------------
// Removed spellings and accepted-but-conflicting arguments are usage errors.
// ---------------------------------------------------------------------------

/// A usage error: status 2, the message on stderr, nothing on stdout, no effect.
fn assert_usage_error(fixture: &CliProcessFixture, args: &[&str], names: &str) {
    let output = run(fixture, args);
    assert_eq!(output.status.code(), Some(2), "{args:?}: {}", stderr_of(&output));
    assert!(output.stdout.is_empty(), "{args:?}: {}", stdout_of(&output));
    assert!(stderr_of(&output).contains(names), "{args:?}: {}", stderr_of(&output));
}

macro_rules! usage_errors {
    ($($name:ident: [$($word:literal),+] names $names:literal;)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = with_document(concat!("output-usage-", stringify!($name)), GOOD);
                assert_usage_error(&fixture, &[$($word),+], $names);
                assert_eq!(read_document(&fixture), GOOD, "a usage error has no effect");
            }
        )+
    };
}

usage_errors! {
    clean_json: ["clean", "doc.md", "--json"] names "--json";
    toc_json: ["toc", "doc.md", "--json"] names "--json";
    delta_json: ["delta", "doc.md", "doc.md", "--json"] names "--json";
    rm_json: ["rm", "doc.md", "title", "--json"] names "--json";
    graph_json: ["graph", "doc.md", "--json"] names "--json";
    get_json5: ["get", "doc.md", "title", "--json5"] names "--json5";
    get_yaml: ["get", "doc.md", "title", "--yaml"] names "--yaml";
    get_toml: ["get", "doc.md", "title", "--toml"] names "--toml";
    get_raw: ["get", "doc.md", "title", "--raw"] names "--raw";
    get_plain: ["get", "doc.md", "title", "--plain"] names "--plain";
    validate_format: ["validate", "refs", "doc.md", "--format", "json"] names "--format";
    schema_validate_format: ["schema", "validate", "doc.md", "--format", "json"] names "--format";
    schema_detect_format: ["schema", "detect", "doc.md", "--format", "json"] names "--format";
    clean_fixed_width: ["clean", "doc.md", "--fixed-width", "80"] names "--fixed-width";
    clean_incidental: ["clean", "doc.md", "--ignore-incidental-newlines"] names "--ignore-incidental-newlines";
    output_auto: ["toc", "doc.md", "--output", "auto"] names "invalid value 'auto'";
    output_ast: ["toc", "doc.md", "--output", "ast"] names "invalid value 'ast'";
    output_text: ["clean", "doc.md", "--output", "text"] names "invalid value 'text'";
    output_report_json: ["schema", "validate", "doc.md", "--output", "report-json"] names "invalid value 'report-json'";
    json_schema_is_only_detect: ["toc", "doc.md", "--output", "json-schema"] names "invalid value 'json-schema'";
    compact_with_yaml: ["get", "doc.md", "title", "--output", "yaml", "--compact"] names "--compact";
    compact_with_plain: ["get", "doc.md", "title", "--output", "plain", "--compact"] names "--compact";
    subcommand_with_top_level_output: ["--output", "json", "toc", "doc.md"] names "top-level options";
    list_themes_with_output: ["--list-themes", "--output", "json"] names "--output";
    completions_with_output: ["--completions", "bash", "--output", "json"] names "--output";
    invalid_assignment: ["schema", "validate", "doc.md", "a=[1"] names "invalid assignment";
    bad_highlight: ["code-block", "--content", "x", "--highlight", "6-4"] names "--highlight";
    hash_directory_save: ["hash", ".", "--save"] names "--save";
    hash_directory_kind: ["hash", ".", "--kind", "detailed"] names "--kind";
}

#[test]
fn a_yaml_document_refuses_assignments_as_a_usage_error() {
    let fixture = with_document("output-usage-yaml", GOOD);
    fixture.write_file("cwd/doc.yaml", "name: x\n");
    assert_usage_error(&fixture, &["schema", "validate", "doc.yaml", "a=1"], "assignments");
    assert_usage_error(&fixture, &["schema", "validate", "doc.yaml", "--schema", "s.yaml"], "--schema");
}

#[test]
fn compact_prints_one_line_for_json_and_json5() {
    let fixture = with_document("output-get-compact", GOOD);
    for target in ["json", "json5"] {
        let output = run(&fixture, &["get", "doc.md", "tags", "--output", target, "--compact"]);
        assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
        assert_eq!(stdout_of(&output), "[\"a\",\"b\"]\n", "{target}");
    }
}

// ---------------------------------------------------------------------------
// `md get`: `--output plain` is today's `--raw`.
// ---------------------------------------------------------------------------

macro_rules! get_plain {
    ($($name:ident: $property:literal => $expected:literal;)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = with_document(
                    concat!("output-get-plain-", stringify!($name)),
                    "---\ntext: hello\nnothing: null\nlist:\n  - a\n  - 2\nmap:\n  k: v\n---\n# H\n",
                );
                for target in ["plain", "terminal"] {
                    let output = run(&fixture, &["get", "doc.md", $property, "--output", target]);
                    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
                    assert_eq!(stdout_of(&output), $expected, "{target}");
                    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
                }
            }
        )+
    };
}

get_plain! {
    get_plain_string: "text" => "hello\n";
    get_plain_null: "nothing" => "\n";
    get_plain_list: "list" => "a\n2\n";
    get_plain_object: "map" => "k: v\n";
}

// ---------------------------------------------------------------------------
// Terminal on a pipe is plain, and `--width` follows one rule.
// ---------------------------------------------------------------------------

macro_rules! terminal_is_plain {
    ($($name:ident: [$($word:literal),+];)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = with_document(concat!("output-plain-", stringify!($name)), BROKEN);
                fixture.write_file("cwd/other.md", GOOD);
                init_repository(&fixture);
                let args: &[&str] = &[$($word),+];
                let terminal = run(&fixture, &[args, &["--output", "terminal"]].concat());
                let plain = run(&fixture, &[args, &["--output", "plain"]].concat());
                assert_eq!(stdout_of(&terminal), stdout_of(&plain), "{args:?}");
                assert_eq!(terminal.status.code(), plain.status.code(), "{args:?}");
                assert!(!stdout_of(&plain).contains('\u{1b}'), "{args:?}: {}", stdout_of(&plain));
                assert!(!stdout_of(&plain).is_empty(), "{args:?}");
            }
        )+
    };
}

terminal_is_plain! {
    plain_toc: ["toc", "doc.md"];
    plain_delta: ["delta", "doc.md", "other.md"];
    plain_graph: ["graph", "doc.md", "--validate"];
    plain_validate_refs: ["validate", "refs", "doc.md"];
    plain_hash: ["hash", "doc.md"];
    plain_code_block: ["code-block", "--content", "x = 1", "--language", "py"];
    plain_schema_validate: ["schema", "validate", "doc.md"];
    plain_schema_triggers: ["schema", "triggers", "doc.md"];
    plain_schema_detect: ["schema", "detect", "doc.md"];
    plain_schema_about: ["schema", "about"];
}

macro_rules! width_changes_nothing {
    ($($name:ident: [$($word:literal),+];)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = with_document(concat!("output-width-", stringify!($name)), LEGACY_WIDE);
                let args: &[&str] = &[$($word),+];
                let plain = run(&fixture, args);
                fixture.write_file("cwd/doc.md", LEGACY_WIDE);
                let narrow = run(&fixture, &[args, &["--width", "20"]].concat());
                assert_eq!(plain.status.code(), Some(0), "{}", stderr_of(&plain));
                assert_eq!(stdout_of(&plain), stdout_of(&narrow), "{args:?}");
            }
        )+
    };
}

width_changes_nothing! {
    width_hash: ["hash", "doc.md"];
    width_rm: ["rm", "doc.md", "title", "--output", "json"];
    width_get: ["get", "doc.md", "title"];
    width_get_plain: ["get", "doc.md", "title", "--output", "plain"];
    width_schema_detect: ["schema", "detect", "doc.md"];
    width_schema_detect_yaml: ["schema", "detect", "doc.md", "--output", "yaml"];
}

/// A command with prose accepts every spelling; what it does with it is pinned
/// by the rendering tests.
macro_rules! accepts_width {
    ($($name:ident: [$($word:literal),+];)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = with_document(concat!("output-accepts-", stringify!($name)), GOOD);
                fixture.write_file("cwd/other.md", GOOD);
                init_repository(&fixture);
                let args: &[&str] = &[$($word),+];
                for width in ["72", "50%", "auto", "source", "unlimited"] {
                    let output = run(&fixture, &[args, &["--width", width]].concat());
                    assert_eq!(output.status.code(), Some(0), "{width}: {}", stderr_of(&output));
                }
            }
        )+
    };
}

accepts_width! {
    accepts_clean: ["clean", "doc.md"];
    accepts_toc: ["toc", "doc.md"];
    accepts_delta: ["delta", "doc.md", "other.md"];
    accepts_set: ["set", "doc.md", "a", "1"];
    accepts_graph: ["graph", "doc.md"];
    accepts_code_block: ["code-block", "--content", "x = 1"];
    accepts_validate_refs: ["validate", "refs", "doc.md"];
    accepts_schema_validate: ["schema", "validate", "doc.md"];
    accepts_schema_triggers: ["schema", "triggers", "doc.md"];
    accepts_schema_about: ["schema", "about"];
}

#[test]
fn a_report_has_its_natural_size_on_a_pipe_with_no_width() {
    let fixture = with_document("output-width-about", GOOD);
    let unlimited = stdout_of(&run(&fixture, &["schema", "about", "--output", "plain"]));
    let widest = unlimited.lines().map(str::len).max().unwrap_or(0);
    assert!(widest > 100, "nothing wraps with no width: {widest}");
    assert!(unlimited.len() < 200_000, "unlimited has its natural size, not a filled width: {}", unlimited.len());
    for line in unlimited.lines() {
        assert_eq!(line, line.trim_end(), "no trailing whitespace");
    }
}

#[test]
fn a_report_wraps_at_an_explicit_width() {
    let fixture = with_document("output-width-wraps", GOOD);
    let wrapped = stdout_of(&run(&fixture, &["schema", "about", "--output", "plain", "--width", "120"]));
    let widest = wrapped.lines().map(|line| line.chars().count()).max().unwrap_or(0);
    assert!(widest <= 122, "{widest}");
}

#[test]
fn explicit_width_reflows_clean_and_set_and_overrides_frontmatter() {
    let fixture = with_document("output-width-documents", "---\nstyle:\n  markdown:\n    fixed-width: 200\n---\none two three four five\n");
    let clean = stdout_of(&run(&fixture, &["clean", "doc.md", "--width", "10"]));
    assert!(clean.contains("one two\nthree four\nfive"), "{clean}");

    let set = stdout_of(&run(&fixture, &["set", "doc.md", "a", "1", "--width", "10"]));
    assert!(set.contains("one two\nthree four\nfive"), "{set}");
    assert!(set.contains("a: 1"), "{set}");

    let data = one_value(&run(&fixture, &["set", "doc.md", "a", "1", "--width", "10", "--output", "json"]));
    assert_eq!(data["document"].as_str().unwrap(), set, "data carries what the Markdown output prints");
}

#[test]
fn explicit_source_keeps_breaks_a_document_would_have_unwrapped() {
    let fixture = with_document("output-width-source", "---\nstyle:\n  markdown:\n    unwrap: true\n---\none two\nthree four\n");
    let source = stdout_of(&run(&fixture, &["clean", "doc.md", "--width", "source"]));
    assert!(source.contains("one two\nthree four"), "{source}");
    let unlimited = stdout_of(&run(&fixture, &["clean", "doc.md", "--width", "unlimited"]));
    assert!(unlimited.contains("one two three four"), "{unlimited}");
    let default = stdout_of(&run(&fixture, &["clean", "doc.md"]));
    assert!(default.contains("one two three four"), "the document's rule applies with no --width: {default}");
}

#[test]
fn a_fractional_width_warns_on_stderr_for_a_formatted_output() {
    let fixture = with_document("output-width-stderr", GOOD);
    let output = run(&fixture, &["toc", "doc.md", "--width", "9.5"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stderr_of(&output).contains("--width"), "{}", stderr_of(&output));
    assert!(!stdout_of(&output).contains("--width"));
}

// ---------------------------------------------------------------------------
// One failure, in each family, for commands whose report is the deliverable.
// ---------------------------------------------------------------------------

#[test]
fn a_report_that_fails_is_on_stdout_with_status_one() {
    let fixture = with_document("output-report-stdout", BROKEN);
    for args in [
        vec!["validate", "refs", "doc.md"],
        vec!["validate", "refs", "doc.md", "--output", "markdown"],
        vec!["graph", "doc.md", "--validate"],
    ] {
        let output = run(&fixture, &args);
        assert_eq!(output.status.code(), Some(1), "{args:?}");
        assert!(output.stderr.is_empty(), "{args:?}: {}", stderr_of(&output));
        assert!(stdout_of(&output).contains("Broken references"), "{args:?}: {}", stdout_of(&output));
    }
}

#[test]
fn a_formatted_failure_with_no_result_is_on_stderr_without_escapes() {
    let fixture = CliProcessFixture::named("output-formatted-failure");
    let output = run(&fixture, &["toc", "missing.md"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
    assert!(!stderr_of(&output).is_empty());
    assert!(!stderr_of(&output).contains('\u{1b}'), "stderr is no terminal here: {}", stderr_of(&output));
}

#[test]
fn a_report_row_holding_an_error_block_has_no_escapes_on_a_pipe() {
    let fixture = with_document("output-report-block", GOOD);
    let output = run(&fixture, &["schema", "validate", "missing.md"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stderr.is_empty(), "{}", stderr_of(&output));
    assert!(stdout_of(&output).contains("could not be opened"), "{}", stdout_of(&output));
    assert!(!stdout_of(&output).contains('\u{1b}'), "{}", stdout_of(&output));
}

#[test]
fn an_unloadable_baseline_is_a_failure_on_stderr_without_escapes() {
    let fixture = with_document("output-baseline-stderr", GOOD);
    let output = run(&fixture, &["schema", "validate", "doc.md", "--schema", "missing.yaml"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
    assert!(!stderr_of(&output).contains('\u{1b}'), "{}", stderr_of(&output));
}

// ---------------------------------------------------------------------------
// Every data format carries the same data; one command per shape is enough
// because the serializers are tested in `biscuit-terminal`.
// ---------------------------------------------------------------------------

fn parsed(format: &str, text: &str) -> Value {
    match format {
        "json5" => biscuit_file::Json5::from_str(text).expect("JSON5").value().clone(),
        "yaml" => serde_json::to_value(biscuit_file::Yaml::from_str(text).expect("YAML").value()).unwrap(),
        "toml" => serde_json::to_value(biscuit_file::Toml::from_str(text).expect("TOML").value()).unwrap(),
        other => panic!("{other}"),
    }
}

fn without_nulls(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(map.iter().filter(|(_, field)| !field.is_null()).map(|(key, field)| (key.clone(), without_nulls(field))).collect()),
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        other => other.clone(),
    }
}

macro_rules! data_parity {
    ($($name:ident: [$($word:literal),+];)+) => {
        $(
            #[test]
            fn $name() {
                let fixture = with_document(concat!("output-parity-", stringify!($name)), GOOD);
                init_repository(&fixture);
                let args: &[&str] = &[$($word),+];
                let json = one_value(&run(&fixture, &[args, &["--output", "json"]].concat()));
                for format in ["json5", "yaml", "toml"] {
                    let output = run(&fixture, &[args, &["--output", format]].concat());
                    assert!(output.stderr.is_empty(), "{format}: {}", stderr_of(&output));
                    let value = parsed(format, &stdout_of(&output));
                    let expected = if format == "toml" { without_nulls(&json) } else { json.clone() };
                    assert_eq!(value, expected, "{format}: {}", stdout_of(&output));
                }
            }
        )+
    };
}

data_parity! {
    parity_set: ["set", "doc.md", "a", "1"];
    parity_rm_failure: ["rm", "doc.md", "nope"];
    parity_graph: ["graph", "doc.md"];
    parity_schema_triggers: ["schema", "triggers", "doc.md"];
    parity_schema_detect: ["schema", "detect", "doc.md"];
}

#[test]
fn a_toml_scalar_is_wrapped_as_value() {
    let fixture = with_document("output-parity-get", GOOD);
    assert_eq!(stdout_of(&run(&fixture, &["get", "doc.md", "title", "--output", "toml"])), "value = \"T\"\n");
}

#[test]
fn yaml_is_the_bare_schema_and_json_schema_is_its_draft() {
    let fixture = with_document("output-detect-forms", GOOD);
    let yaml = stdout_of(&run(&fixture, &["schema", "detect", "doc.md", "--output", "yaml"]));
    assert!(yaml.starts_with("$schema:\n"), "{yaml}");
    assert!(!yaml.contains("```"), "{yaml}");
    let plain = stdout_of(&run(&fixture, &["schema", "detect", "doc.md", "--output", "plain"]));
    assert!(plain.starts_with("```yaml\n$schema:\n"), "{plain}");
    let draft = one_value(&run(&fixture, &["schema", "detect", "doc.md", "--output", "json-schema"]));
    assert_eq!(draft["$schema"], "https://json-schema.org/draft/2020-12/schema");
    let several = one_value(&run(&fixture, &["schema", "detect", "doc.md", "doc.md", "--output", "json-schema"]));
    assert_eq!(several["files"][1]["schema"]["$schema"], "https://json-schema.org/draft/2020-12/schema");
}

// ---------------------------------------------------------------------------
// Formatted families of the non-document commands.
// ---------------------------------------------------------------------------

#[test]
fn a_toc_is_a_linked_list_in_markdown_and_html() {
    let fixture = with_document("output-toc-formats", GOOD);
    let markdown = stdout_of(&run(&fixture, &["toc", "doc.md", "--output", "markdown"]));
    assert!(markdown.contains("[Sub](#sub)"), "{markdown}");
    let html = stdout_of(&run(&fixture, &["toc", "doc.md", "--output", "browser"]));
    assert!(html.contains("href=\"#sub\""), "{html}");
}

#[test]
fn code_keeps_its_exact_markdown_and_a_long_line_is_whole_in_plain() {
    let fixture = CliProcessFixture::named("output-code-formats");
    let long = "x".repeat(200);
    let markdown = stdout_of(&run(&fixture, &["code-block", "--content", &long, "--language", "py", "--output", "markdown"]));
    assert_eq!(markdown, format!("```python\n{long}\n```\n"));
    let plain = stdout_of(&run(&fixture, &["code-block", "--content", &long, "--language", "py", "--output", "plain", "--width", "20"]));
    assert!(plain.contains(&long), "{plain}");
    let html = stdout_of(&run(&fixture, &["code-block", "--content", "x", "--language", "py", "--output", "html"]));
    assert!(html.contains("<pre"), "{html}");
}

#[test]
fn a_graph_notation_is_its_source_in_a_terminal_and_a_fence_in_markdown() {
    let fixture = with_document("output-refs-notation", GOOD);
    let terminal = stdout_of(&run(&fixture, &["validate", "refs", "doc.md", "--graph", "mermaid"]));
    assert!(terminal.starts_with("flowchart"), "{terminal}");
    let markdown = stdout_of(&run(&fixture, &["validate", "refs", "doc.md", "--graph", "mermaid", "--output", "markdown"]));
    assert!(markdown.starts_with("```mermaid\nflowchart"), "{markdown}");
}

#[test]
fn saving_prints_nothing_formatted_for_set_and_rm() {
    let fixture = with_document("output-save-quiet", GOOD);
    let set = run(&fixture, &["set", "doc.md", "a", "1", "--save"]);
    assert_eq!(set.status.code(), Some(0));
    assert!(set.stdout.is_empty(), "{}", stdout_of(&set));
    let rm = run(&fixture, &["rm", "doc.md", "a"]);
    assert_eq!(rm.status.code(), Some(0));
    assert!(rm.stdout.is_empty(), "{}", stdout_of(&rm));
    let html = run(&fixture, &["rm", "doc.md", "title", "--output", "markdown"]);
    assert!(stdout_of(&html).contains("`title`"), "{}", stdout_of(&html));
}

#[test]
fn the_top_level_save_alias_forwards_output_and_width() {
    let fixture = with_document("output-save-alias", "one two three four\n");
    let output = run(&fixture, &["--save", "doc.md", "--width", "10", "--output", "json"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(one_value(&output)["version"], 1);
    assert!(read_document(&fixture).contains("one two\nthree four"));
}

// Defects the Phase 6 verification pass reproduced through the real binary.

#[test]
fn schema_about_markdown_keeps_each_entry_on_its_own_list_row() {
    let fixture = CliProcessFixture::named("output-about-markdown-rows");
    let output = run(&fixture, &["schema", "about", "--output", "markdown"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let text = stdout_of(&output);
    assert!(!text.contains("&#32;"), "{text}");
    assert!(text.lines().any(|line| line.starts_with("- `") && line.contains(" — ")), "{text}");
}

#[test]
fn schema_about_honors_a_width_narrower_than_a_page() {
    let fixture = CliProcessFixture::named("output-about-narrow");
    let output = run(&fixture, &["schema", "about", "--width", "30", "--output", "plain"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let text = stdout_of(&output);
    let introduction: Vec<&str> = text.lines().take_while(|line| !line.contains("Schema Shapes")).collect();
    assert!(introduction.len() > 5, "{text}");
    for line in introduction {
        assert!(line.chars().count() <= 30, "{line:?}");
    }
    // A table too narrow for its columns keeps its rows.
    assert!(!text.contains("could not be rendered"), "{text}");
    assert!(text.contains("Constraints:"), "{text}");
    assert!(!text.contains('\u{1b}'), "{text}");
}

#[test]
fn schema_triggers_markdown_keeps_its_rows_apart() {
    let fixture = with_document("output-triggers-markdown-rows", GOOD);
    init_repository(&fixture);
    fixture.write_file("cwd/schemas/bad.yaml", "kind: schema-trigger\n$path: \"*.md\"\n");
    let output = run(&fixture, &["schema", "triggers", "doc.md", "--output", "markdown"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    let text = stdout_of(&output);
    assert!(!text.contains("&#32;"), "{text}");
    assert!(!text.contains("\\["), "{text}");
    let lines: Vec<&str> = text.lines().collect();
    for heading in ["**Schema roots (search order):**", "**Shadowed envelopes:**", "**Skipped invalid triggers:**", "**Triggers:**"] {
        assert!(lines.contains(&heading), "{heading} is not its own line: {text}");
    }
    assert!(lines.iter().any(|line| line.starts_with("- `") && line.ends_with("`dm.schema.invalid_trigger`")), "{text}");
}

#[test]
fn code_block_markdown_has_no_blank_line_before_its_closing_fence() {
    let fixture = CliProcessFixture::named("output-code-block-fence");
    fixture.write_file("cwd/x.rs", "fn a() {}\n");
    let output = run(&fixture, &["code-block", "x.rs", "--output", "markdown"]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(stdout_of(&output), "```rust\nfn a() {}\n```\n");
    // A blank last line of the file is content, so one blank line stays.
    fixture.write_file("cwd/y.rs", "fn a() {}\n\n");
    assert_eq!(stdout_of(&run(&fixture, &["code-block", "y.rs", "--output", "markdown"])), "```rust\nfn a() {}\n\n```\n");
    // The data value keeps the file's bytes.
    assert_eq!(one_value(&run(&fixture, &["code-block", "x.rs", "--output", "json"]))["code"], "fn a() {}\n");
}

#[test]
fn delta_never_calls_a_changed_preamble_no_changes() {
    let fixture = CliProcessFixture::named("output-delta-preamble");
    fixture.write_file("cwd/a.md", "Some intro\n\n# Hello\n");
    fixture.write_file("cwd/b.md", "Different intro\n\n# Hello\n");
    let output = run(&fixture, &["delta", "a.md", "b.md", "--output", "plain"]);
    let text = stdout_of(&output);
    assert!(text.contains("Preamble: modified"), "{text}");
    assert!(!text.contains("No changes"), "{text}");
}

#[test]
fn compose_shell_refuses_show() {
    let fixture = with_document("output-compose-shell-show", GOOD);
    // A regression that launched a viewer would skip it.
    let output = fixture.command_builder().application_input("MD_DRY_RUN", "1").build().args(["compose", "doc.md", "--shell", "--show"]).output().expect("md runs");
    assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
    assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
    assert!(stderr_of(&output).contains("--show"), "{}", stderr_of(&output));
}

#[test]
fn render_deny_takes_the_style_codes_it_reports() {
    let fixture = with_document("output-render-deny-style-code", "---\nstyle:\n  page:\n    bogus: 1\n---\n# Head\n");
    // The denied code blocks.
    let denied = run(&fixture, &["render", "doc.md", "--deny", "dm.style.unknown_key", "--output", "json"]);
    assert_eq!(denied.status.code(), Some(1), "{}", stderr_of(&denied));
    assert_eq!(one_value(&denied)["outcome"], "failure");
    let formatted = run(&fixture, &["render", "doc.md", "--deny", "dm.style.unknown_key", "--output", "plain"]);
    assert_eq!(formatted.status.code(), Some(1), "{}", stderr_of(&formatted));
    assert!(formatted.stdout.is_empty(), "{}", stdout_of(&formatted));
    // Another code leaves the same warning a warning.
    let other = run(&fixture, &["render", "doc.md", "--deny", "dm.style.deprecated", "--output", "json"]);
    assert_eq!(other.status.code(), Some(0), "{}", stderr_of(&other));
    let value = one_value(&other);
    assert_eq!(value["outcome"], "success");
    assert_eq!(first_code(&value), "dm.style.unknown_key");
    // A code the command does not report stays a usage error.
    let unknown = run(&fixture, &["render", "doc.md", "--deny", "dm.style.nonsense"]);
    assert_eq!(unknown.status.code(), Some(2), "{}", stderr_of(&unknown));
}

#[test]
fn edit_refuses_a_configured_editor_that_is_missing() {
    let fixture = CliProcessFixture::named("output-edit-missing-editor");
    let output = fixture.command().env("EDITOR", "/definitely/missing/editor").env("VISUAL", "/definitely/missing/visual").args(["edit", "new.md", "--output", "json"]).output().expect("md runs");
    assert_eq!(output.status.code(), Some(1), "{}", stderr_of(&output));
    let value = one_value(&output);
    validate_strictly(include_str!("../../schemas/output/edit.yaml"), &value);
    assert!(value["diagnostics"][0]["message"].as_str().unwrap().contains("/definitely/missing/editor"), "{value:#}");
    assert!(!fixture.cwd().join("new.md").exists(), "a refused edit leaves nothing behind");
}

/// `md edit` with an editor running `script`.
#[cfg(unix)]
fn edit_with_script(fixture: &CliProcessFixture, script: &str, file: &str) -> Output {
    use std::os::unix::fs::PermissionsExt as _;
    let editor = fixture.write_file("cwd/scripted-editor.sh", script);
    std::fs::set_permissions(&editor, std::fs::Permissions::from_mode(0o755)).unwrap();
    fixture.command().env("EDITOR", editor).args(["edit", file, "--output", "json"]).output().expect("md runs")
}

#[cfg(unix)]
#[test]
fn edit_removes_the_file_it_created_when_the_edit_fails() {
    let fixture = CliProcessFixture::named("output-edit-failed-created");
    let failed = edit_with_script(&fixture, "#!/bin/sh\nexit 3\n", "new.md");
    assert_eq!(failed.status.code(), Some(1), "{}", stderr_of(&failed));
    assert_eq!(one_value(&failed)["outcome"], "failure");
    assert!(!fixture.cwd().join("new.md").exists(), "an editor failure leaves no empty file");

    let abandoned = edit_with_script(&fixture, "#!/bin/sh\nexit 0\n", "other.md");
    assert_eq!(abandoned.status.code(), Some(1), "{}", stderr_of(&abandoned));
    assert!(!fixture.cwd().join("other.md").exists(), "an abandoned new file is not kept");
}

#[cfg(unix)]
#[test]
fn edit_keeps_an_existing_file_and_a_created_file_with_content_when_the_edit_fails() {
    let fixture = with_document("output-edit-failed-existing", GOOD);
    let failed = edit_with_script(&fixture, "#!/bin/sh\nexit 3\n", "doc.md");
    assert_eq!(failed.status.code(), Some(1), "{}", stderr_of(&failed));
    assert_eq!(read_document(&fixture), GOOD);

    let wrote = edit_with_script(&fixture, "#!/bin/sh\nprintf 'kept\\n' >> \"$1\"\nexit 3\n", "new.md");
    assert_eq!(wrote.status.code(), Some(1), "{}", stderr_of(&wrote));
    assert_eq!(std::fs::read_to_string(fixture.cwd().join("new.md")).unwrap(), "kept\n");
}
