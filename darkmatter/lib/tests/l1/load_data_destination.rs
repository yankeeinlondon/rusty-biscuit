//! A `load_data` result judged by the destination's schema at composition's
//! normal frontmatter validation stage.
//!
//! Each test composes one document in its own temporary directory through
//! the public compose entry point and reads the public result: the composed
//! frontmatter, or the failure's diagnostic records
//! (`report_diagnostics::error_diagnostics`) and its status block. A
//! destination failure on a property one whole-value `{{ … }}` produced is
//! placed at that expression; a direct load (the call itself, parenthesized,
//! or the arm a ternary selected) also names the original data position,
//! while a derived value (a callback, a spread, `try`, a member) has the
//! expression location alone.

use std::path::{Path, PathBuf};

use biscuit_terminal::errors::BlockError;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::markdown::compose::expression::{DataLocation, Diagnostic};
use darkmatter::markdown::compose::report_diagnostics::error_diagnostics;
use darkmatter::markdown::compose::{ComposeContext, ComposeFailure, ComposeOptions};
use darkmatter::markdown::{Markdown, MarkdownError};
use serde_json::{Value, json};
use tempfile::TempDir;

/// The worked example's data: Grace's age is valid.
const PEOPLE: &str = "- name: Ada\n  age: 36\n- name: Grace\n  age: 85\n";
/// The same data with Grace's age (line 4) a string.
const PEOPLE_OLD: &str = "- name: Ada\n  age: 36\n- name: Grace\n  age: \"old\"\n";
/// The row declaration the worked example uses.
const ROW: &str = "{ name: string(required), age: number }[]";

fn write(dir: &Path, name: &str, content: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("create parent");
    std::fs::write(&path, content).expect("write fixture");
    path
}

/// The portable spelling `data_location.file` uses for `name` in `dir`.
fn entry(dir: &Path, name: &str) -> String {
    let canonical = biscuit_file::canonicalize_simplified(dir).expect("canonical dir");
    biscuit_file::to_forward_slash_string(&canonical.join(name))
}

/// A document declaring `key` as `declared` and authoring it as `value`.
fn document(key: &str, declared: &str, value: &str) -> String {
    format!("---\n$schema:\n    {key}: \"{declared}\"\n{key}: \"{value}\"\n---\nBody\n")
}

fn compose(dir: &Path, doc: &str) -> Result<serde_json::Map<String, Value>, Box<ComposeFailure>> {
    let path = write(dir, "doc.md", doc);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, "")).with_source_file(&path);
    let request = crate::request_support::request(options);
    Markdown::try_from(path.as_path())
        .expect("document loads")
        .compose_with_warnings(&request)
        .map(|(composed, _)| composed.frontmatter().as_map().clone().into_iter().collect())
}

fn composed(dir: &Path, doc: &str) -> serde_json::Map<String, Value> {
    compose(dir, doc).unwrap_or_else(|failure| panic!("{doc}\n{}", failure.error))
}

fn failure(dir: &Path, doc: &str) -> Box<ComposeFailure> {
    match compose(dir, doc) {
        Ok(frontmatter) => panic!("{doc} composed: {frontmatter:?}"),
        Err(failure) => failure,
    }
}

/// The one record of a destination schema failure.
fn only_record(dir: &Path, doc: &str) -> Diagnostic {
    let failure = failure(dir, doc);
    assert!(matches!(failure.error, MarkdownError::SchemaValidationFailed { .. }), "{}", failure.error);
    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    assert_ne!(records[0].code, "dm.schema.prepare", "an instance failure: {records:#?}");
    records.into_iter().next().expect("one record")
}

/// `(line, column)` of a record's authored source.
fn source_at(record: &Diagnostic) -> (Option<usize>, Option<usize>) {
    let source = record.source.as_ref().expect("a source");
    (source.line, source.column)
}

/// The 1-based `(line, column)` of the `{{` of the first `needle` in `doc`.
fn expression_at(doc: &str, needle: &str) -> (Option<usize>, Option<usize>) {
    let start = doc.find(needle).unwrap_or_else(|| panic!("no `{needle}`"));
    let start = start + needle.find("{{").expect("the needle holds the expression");
    let before = &doc[..start];
    let line_start = before.rfind('\n').map_or(0, |index| index + 1);
    (Some(before.matches('\n').count() + 1), Some(before[line_start..].chars().count() + 1))
}

fn located(file: String, pointer: &str, line: usize, column: usize) -> DataLocation {
    DataLocation { file, pointer: Some(pointer.to_string()), line: Some(line), column: Some(column), byte_offset: None }
}

/// A destination failure is a schema code, never a `dm.data.*` contract code.
fn assert_schema_code(record: &Diagnostic) {
    assert!(record.code.starts_with("dm.schema."), "{record:#?}");
}

// ── The worked example ─────────────────────────────────────────────────────

/// The worked example passes for valid data, every property composing to
/// the loaded rows (the conditional to its fallback once the file is gone).
#[test]
fn the_worked_example_passes_for_valid_data() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE);
    let doc = concat!(
        "---\n",
        "$schema:\n",
        "    direct: \"{ name: string(required), age: number }[]\"\n",
        "    additive: \"{ name: string(required), age: number, color: string }[]\"\n",
        "    conditional:\n",
        "        - \"{ name: string(required), age: number }[]\"\n",
        "        - \"literal(foobar)\"\n",
        "direct: \"{{ load_data('data.yaml') }}\"\n",
        "additive: \"{{ map(load_data('data.yaml'), i => { ...i, color: 'blue' }) }}\"\n",
        "conditional: \"{{ file_exists('data.yaml') ? load_data('data.yaml') : 'foobar' }}\"\n",
        "---\nBody\n",
    );
    let frontmatter = composed(dir.path(), doc);
    let rows = json!([{"name": "Ada", "age": 36}, {"name": "Grace", "age": 85}]);
    assert_eq!(frontmatter["direct"], rows);
    assert_eq!(frontmatter["conditional"], rows);
    assert_eq!(
        frontmatter["additive"],
        json!([{"name": "Ada", "age": 36, "color": "blue"}, {"name": "Grace", "age": 85, "color": "blue"}])
    );

    std::fs::remove_file(dir.path().join("data.yaml")).unwrap();
    let conditional = document(
        "conditional",
        "literal(foobar)",
        "{{ file_exists('data.yaml') ? load_data('data.yaml') : 'foobar' }}",
    );
    assert_eq!(composed(dir.path(), &conditional)["conditional"], json!("foobar"));
}

/// `direct` fails at its expression and at Grace's age on line 4 of the data.
#[test]
fn a_direct_load_failure_names_the_expression_and_the_data_position() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let doc = document("direct", ROW, "{{ load_data('data.yaml') }}");
    let record = only_record(dir.path(), &doc);
    assert_schema_code(&record);
    assert_eq!(source_at(&record), expression_at(&doc, "{{ load_data"), "{record:#?}");
    assert!(record.span.is_some(), "the expression's authored range: {record:#?}");
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "data.yaml"), "/1/age", 4, 8)));
}

/// `additive` builds each row with a callback and a spread: the failure has
/// the expression location alone.
#[test]
fn a_derived_result_failure_names_only_the_expression() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let doc = document(
        "additive",
        "{ name: string(required), age: number, color: string }[]",
        "{{ map(load_data('data.yaml'), i => { ...i, color: 'blue' }) }}",
    );
    let record = only_record(dir.path(), &doc);
    assert_schema_code(&record);
    assert_eq!(source_at(&record), expression_at(&doc, "{{ map"), "{record:#?}");
    assert_eq!(record.data_location, None, "{record:#?}");
}

/// The conditional selects the load while the file exists, so its failure
/// keeps the data file; with the file gone the fallback is used.
#[test]
fn a_selected_ternary_arm_keeps_its_data_position() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let doc = concat!(
        "---\n",
        "$schema:\n",
        "    conditional:\n",
        "        - \"{ name: string(required), age: number }[]\"\n",
        "        - \"literal(foobar)\"\n",
        "conditional: \"{{ file_exists('data.yaml') ? load_data('data.yaml') : 'foobar' }}\"\n",
        "---\nBody\n",
    );
    let failure = failure(dir.path(), doc);
    let records = error_diagnostics(&failure.error);
    assert!(!records.is_empty());
    for record in &records {
        assert_schema_code(record);
        assert_eq!(source_at(record), expression_at(doc, "{{ file_exists"), "{record:#?}");
        let data = record.data_location.as_ref().unwrap_or_else(|| panic!("a data location: {record:#?}"));
        assert_eq!(data.file, entry(dir.path(), "data.yaml"));
    }
}

/// Parentheses forward a direct load; `try`, `||`, and a member selection
/// derive a new value.
#[test]
fn only_parentheses_and_a_selected_arm_forward_the_load() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let paren = document("direct", ROW, "{{ (load_data('data.yaml')) }}");
    let record = only_record(dir.path(), &paren);
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "data.yaml"), "/1/age", 4, 8)));

    let ternary = document("direct", ROW, "{{ true ? (load_data('data.yaml')) : [] }}");
    let record = only_record(dir.path(), &ternary);
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "data.yaml"), "/1/age", 4, 8)));

    for derived in [
        "{{ try(load_data('data.yaml'), []) }}",
        "{{ load_data('data.yaml') || [] }}",
        "{{ [load_data('data.yaml')[1]] }}",
        "{{ false ? [] : filter(load_data('data.yaml'), p => true) }}",
    ] {
        let doc = document("direct", ROW, derived);
        let record = only_record(dir.path(), &doc);
        assert_eq!(source_at(&record), expression_at(&doc, "{{"), "{derived}: {record:#?}");
        assert_eq!(record.data_location, None, "{derived}: {record:#?}");
    }
}

/// An explicit contract's own failure keeps its `dm.data.*` code and data
/// position, distinct from a destination failure.
#[test]
fn a_contract_failure_stays_a_data_error() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let doc = document("direct", ROW, "{{ load_data('data.yaml', '{ name: string, age: number }[]') }}");
    let failure = failure(dir.path(), &doc);
    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    assert_eq!(records[0].code, "dm.data.contract_mismatch");
    assert_eq!(records[0].data_location.as_ref().and_then(|location| location.line), Some(4));
}

// ── Destination rules come from the ordinary validator ─────────────────────

#[test]
fn an_enum_destination_judges_each_loaded_value() {
    let dir = TempDir::new().unwrap();
    let doc = document("os", "enum(macOS, Linux)[]", "{{ load_data('os.yaml') }}");
    write(dir.path(), "os.yaml", "- macOS\n");
    assert_eq!(composed(dir.path(), &doc)["os"], json!(["macOS"]));

    write(dir.path(), "os.yaml", "- macOS\n- BeOS\n");
    let record = only_record(dir.path(), &doc);
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "os.yaml"), "/1", 2, 3)));
}

/// A CSV cell is text until the destination converts it; a declared default
/// fills a missing member.
#[test]
fn conversion_and_defaults_agree_with_the_destination() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "Name,Stars\nAda,1000\n");
    let doc = document("rows", "{ name: string, stars: number, color: string(default(blue)) }[]", "{{ load_data('rows.csv') }}");
    assert_eq!(composed(dir.path(), &doc)["rows"], json!([{"name": "Ada", "stars": 1000, "color": "blue"}]));
}

/// An unexpected member of a closed object fails at that member, placed at
/// its value as the loader places one.
#[test]
fn an_unexpected_member_fails_at_the_member() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.json", "[\n  {\"name\": \"Ada\", \"extra\": 1}\n]\n");
    let doc = document("rows", "{ name: string }[]", "{{ load_data('data.json') }}");
    let record = only_record(dir.path(), &doc);
    assert_schema_code(&record);
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "data.json"), "/0/extra", 2, 28)));
}

/// A header-derived key and an explicit name locate the original cell.
#[test]
fn tabular_keys_locate_the_original_cell() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "rows.csv", "First Name,Age\nAda,old\n");
    let doc = document("rows", "{ firstName: string, age: number }[]", "{{ load_data('rows.csv') }}");
    let record = only_record(dir.path(), &doc);
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "rows.csv"), "/0/age", 2, 5)));

    write(dir.path(), "plain.csv", "Ada,old\n");
    let doc = document("rows", "{ label: string, years: number }[]", "{{ load_data('plain.csv', null, {header: ['label', 'years']}) }}");
    let record = only_record(dir.path(), &doc);
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "plain.csv"), "/0/years", 1, 5)));
}

/// A wrapped scalar's element is the document's root.
#[test]
fn a_wrapped_scalar_is_located_at_the_root() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "n.yaml", "five\n");
    let doc = document("n", "number[]", "{{ load_data('n.yaml') }}");
    let record = only_record(dir.path(), &doc);
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "n.yaml"), "", 1, 1)));
    write(dir.path(), "n.yaml", "5\n");
    assert_eq!(composed(dir.path(), &doc)["n"], json!([5]));
}

/// An imported row type is the effective declaration.
#[test]
fn an_imported_destination_type_is_applied() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "types.yaml", "kind: schema\ntypes:\n  People: \"{ name: string(required), age: number }[]\"\n");
    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let doc = document("people", "People@./types.yaml", "{{ load_data('data.yaml') }}");
    let record = only_record(dir.path(), &doc);
    assert_eq!(record.data_location, Some(located(entry(dir.path(), "data.yaml"), "/1/age", 4, 8)));
    write(dir.path(), "data.yaml", PEOPLE);
    assert_eq!(composed(dir.path(), &doc)["people"][1]["age"], json!(85));
}

/// A trigger's declaration is part of the effective schema.
#[test]
fn a_trigger_destination_declaration_is_applied() {
    let dir = TempDir::new().unwrap();
    // A repository marker without spawning `git`.
    write(dir.path(), ".git/HEAD", "ref: refs/heads/main\n");
    write(dir.path(), ".git/config", "[core]\n\trepositoryformatversion = 0\n");
    std::fs::create_dir_all(dir.path().join(".git/objects")).unwrap();
    std::fs::create_dir_all(dir.path().join(".git/refs")).unwrap();
    write(
        dir.path(),
        "schemas/t.yaml",
        "kind: schema-trigger\nmatch:\n  stage: literal(draft; required)\n$schema:\n  people: \"{ name: string(required), age: number }[]\"\n",
    );
    let base = dir.path().join("docs");
    write(&base, "data.yaml", PEOPLE_OLD);
    let path = write(&base, "doc.md", "---\nstage: draft\npeople: \"{{ load_data('data.yaml') }}\"\n---\nBody\n");
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&base, ""))
        .with_source_file(&path)
        .with_trigger_schemas(true);
    let request = crate::request_support::request(options);
    let failure = Markdown::try_from(path.as_path())
        .unwrap()
        .compose_with_warnings(&request)
        .expect_err("the trigger's declaration rejects Grace's age");
    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    assert_eq!(records[0].data_location, Some(located(entry(&base, "data.yaml"), "/1/age", 4, 8)));
}

// ── The human-readable block ───────────────────────────────────────────────

/// The status block places the problem at the expression and links the data
/// position the structured record names.
#[test]
fn the_status_block_shows_both_locations() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let doc = document("direct", ROW, "{{ load_data('data.yaml') }}");
    let failure = failure(dir.path(), &doc);
    let block = strip_escape_codes(failure.error.report_block_error_optimistic(Some(200)));
    let (line, column) = expression_at(&doc, "{{ load_data");
    assert!(block.contains(&format!(" at {}:{}", line.unwrap(), column.unwrap())), "{block}");
    assert!(block.contains(&format!("data: {}:4:8 (/1/age)", entry(dir.path(), "data.yaml"))), "{block}");
}

// ── Validation passes ──────────────────────────────────────────────────────

/// Composes `doc` with every frontmatter command pre-approved, as a caller
/// that audits before composing does; `caller_owned` leaves the post-shell
/// verdict to the caller.
fn compose_with_shell(
    dir: &Path,
    doc: &str,
    caller_owned: bool,
) -> Result<serde_json::Map<String, Value>, Box<ComposeFailure>> {
    let path = write(dir, "doc.md", doc);
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
        .with_source_file(&path)
        .with_config_snapshot(crate::request_support::shell_config(dir, &[]))
        .with_caller_owned_post_shell_verdict(caller_owned);
    let document = Markdown::try_from(path.as_path()).expect("document loads");
    let approved = document
        .compose_preflight(&crate::request_support::request(options.clone()))
        .expect("pre-flight")
        .approval_set()
        .into_iter()
        .collect::<std::collections::HashSet<String>>();
    document
        .compose_with_warnings(&crate::request_support::request(options.with_pre_approved_commands(approved)))
        .map(|(composed, _)| composed.frontmatter().as_map().clone().into_iter().collect())
}

/// A property reading a frontmatter command's output is completed by
/// interpolation pass two, and judged with its expression's provenance by
/// the post-shell validation pass.
const PASS_TWO: &str = concat!(
    "---\n",
    "$schema:\n",
    "    gate: string\n",
    "    direct: \"{ name: string(required), age: number }[]\"\n",
    "gate: $(echo ok)\n",
    "direct: \"{{ gate == 'ok' ? load_data('data.yaml') : [] }}\"\n",
    "---\nBody\n",
);

#[test]
fn a_property_completed_in_pass_two_keeps_its_provenance() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE);
    let frontmatter = compose_with_shell(dir.path(), PASS_TWO, false).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(frontmatter["direct"][1]["age"], json!(85));

    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let failure = compose_with_shell(dir.path(), PASS_TWO, false).expect_err("Grace's age fails after pass two");
    let records = error_diagnostics(&failure.error);
    assert_eq!(records.len(), 1, "{records:#?}");
    assert_eq!(source_at(&records[0]), expression_at(PASS_TWO, "{{ gate"), "{records:#?}");
    assert_eq!(records[0].data_location, Some(located(entry(dir.path(), "data.yaml"), "/1/age", 4, 8)));
}

/// A caller that owns the post-shell verdict judges the pass-two property
/// itself: composition reports nothing for it.
#[test]
fn a_caller_owned_post_shell_verdict_is_the_callers() {
    let dir = TempDir::new().unwrap();
    write(dir.path(), "data.yaml", PEOPLE_OLD);
    let frontmatter = compose_with_shell(dir.path(), PASS_TWO, true).unwrap_or_else(|failure| panic!("{}", failure.error));
    assert_eq!(frontmatter["direct"][1]["age"], json!("old"));
}
