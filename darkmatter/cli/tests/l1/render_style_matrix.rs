//! The render style input-robustness matrix at the CLI, in process through
//! `darkmatter_cli::render_style`: the document shapes of each `--style`
//! loader and the CLI-only layer rules.
//!
//! The field-shape cells of every validator are the library's
//! (`darkmatter`'s `style_layer_matrix`); the reference/offset adapter, the
//! basic loader controls, and their failure records are in
//! `render_style_files`; the process wiring is in `render_style_wiring`.
//! Every fixture and request is anchored in a temporary directory.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use color_eyre::eyre::Report;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::compose::expression::{Diagnostic, Severity};
use darkmatter::style::{PreparedStyle, StylePreparationPolicy};
use darkmatter_cli::args::RawStyleSetter;
use darkmatter_cli::io::open_argument;
use darkmatter_cli::output::report::failure_diagnostics;
use darkmatter_cli::render_style::{document_style_layer, prepare_cli_style_inputs};
use darkmatter_cli::request::MdRequest;
use serde_json::{Value, json};
use tempfile::TempDir;

/// In-process fixture: a canonical temporary root holding `doc.md` and the
/// style files, with its own home.
struct Local {
    _dir: TempDir,
    root: PathBuf,
}

impl Local {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("home")).expect("home");
        Self { _dir: dir, root }
    }

    fn write(&self, relative: &str, contents: &str) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        std::fs::write(&path, contents).expect("write");
        path
    }

    /// Prepares `doc.md` (holding `document`) with its own `style:` as the
    /// bottom layer, under `styles` and `setters`.
    fn prepare(&self, document: &str, styles: &[&str], setters: &[&str]) -> Result<PreparedStyle, Report> {
        let request = MdRequest::new(RequestSnapshot::new(&self.root).with_home(Some(self.root.join("home"))));
        self.write("doc.md", document);
        let opened = open_argument(Path::new("doc.md"), &request).expect("doc.md opens");
        let markdown = Markdown::try_from(opened.path()).expect("doc.md parses");
        let styles: Vec<OsString> = styles.iter().map(OsString::from).collect();
        let setters: Vec<RawStyleSetter> = setters
            .iter()
            .map(|argument| {
                let (key, value) = argument.split_once('=').expect("a setter has `=`");
                RawStyleSetter { argument: (*argument).to_string(), key: key.to_string(), value: value.to_string() }
            })
            .collect();
        let inputs = prepare_cli_style_inputs(&markdown, Some(&opened), &styles, &setters, &request)?;
        let layer = document_style_layer(&markdown, Some(opened.path()), opened.document_context(&request)?);
        inputs.prepare(Some(layer), StylePreparationPolicy::render())
    }
}

const PLAIN: &str = "# Notes\n";

fn page(prepared: &PreparedStyle) -> &Value {
    &prepared.value()["page"]
}

/// The first error record of a failed preparation.
fn first_error(result: Result<PreparedStyle, Report>) -> Diagnostic {
    let error = match result {
        Ok(prepared) => panic!("expected a failure, got {:#}", prepared.value()),
        Err(error) => error,
    };
    failure_diagnostics(&error)
        .into_iter()
        .find(|record| record.severity == Severity::Error)
        .unwrap_or_else(|| panic!("no error record: {error:#}"))
}

/// Asserts `record` has `code` and names the style file `name` (by its
/// opened path's file name).
fn assert_in_file(record: &Diagnostic, code: &str, name: &str) {
    assert_eq!(record.code, code, "{record:?}");
    let file = record.source.as_ref().and_then(|source| source.file.as_ref()).unwrap_or_else(|| panic!("no file: {record:?}"));
    assert_eq!(file.file_name().and_then(|name| name.to_str()), Some(name), "{record:?}");
}

// JSON loader

#[test]
fn json_selects_an_offset_inside_a_real_fixture() {
    let local = Local::new();
    local.write("style.json", r#"{"themes": {"dark": {"margin": 2, "page": {"max-width": 60}}}}"#);
    let prepared = local.prepare(PLAIN, &["themes.dark@style.json"], &[]).expect("prepares");
    assert_eq!(page(&prepared)["margin-top"], json!(2));
    assert_eq!(page(&prepared)["max-width"], json!(60));
}

#[test]
fn json_alias_collision_fails_after_parsing() {
    let local = Local::new();
    local.write("style.json", "{\n  \"ml\": 1,\n  \"page\": {\"margin-left\": 2}\n}\n");
    let record = first_error(local.prepare(PLAIN, &["style.json"], &[]));
    assert_in_file(&record, "dm.style.conflict", "style.json");
    assert!(record.message.contains("style.page.margin-left"), "{}", record.message);
}

// JSON5 loader

#[test]
fn json5_keeps_the_last_duplicate_key() {
    let local = Local::new();
    local.write("style.json5", "{ margin: 1, margin: 3 }\n");
    let prepared = local.prepare(PLAIN, &["style.json5"], &[]).expect("last wins");
    assert_eq!(page(&prepared)["margin-top"], json!(3));
}

#[test]
fn json5_rejects_a_non_finite_number() {
    let local = Local::new();
    local.write("style.json5", "{\n  margin: Infinity,\n}\n");
    let record = first_error(local.prepare(PLAIN, &["style.json5"], &[]));
    assert_in_file(&record, "dm.data.malformed", "style.json5");
    assert_eq!(record.source.as_ref().and_then(|source| source.line), Some(2), "{record:?}");
}

#[test]
fn json5_rejects_an_oversized_number() {
    let local = Local::new();
    local.write("style.json5", "{ margin: 1e400 }\n");
    assert_in_file(&first_error(local.prepare(PLAIN, &["style.json5"], &[])), "dm.data.malformed", "style.json5");
}

// YAML loader

#[test]
fn yaml_reads_a_flow_mapping() {
    let local = Local::new();
    local.write("style.yaml", "{ margin: 2, page: { max-width: 40 } }\n");
    let prepared = local.prepare(PLAIN, &["style.yaml"], &[]).expect("flow mapping");
    assert_eq!((page(&prepared)["margin-top"].clone(), page(&prepared)["max-width"].clone()), (json!(2), json!(40)));
}

#[test]
fn yaml_reads_an_alias_at_an_offset() {
    let local = Local::new();
    local.write("style.yaml", "base: &base\n  margin: 4\nstyle: *base\n");
    let prepared = local.prepare(PLAIN, &["style@style.yaml"], &[]).expect("an alias");
    assert_eq!(page(&prepared)["margin-top"], json!(4));
}

#[test]
fn yaml_rejects_a_non_finite_number() {
    let local = Local::new();
    local.write("style.yaml", "margin: .inf\n");
    assert_in_file(&first_error(local.prepare(PLAIN, &["style.yaml"], &[])), "dm.data.malformed", "style.yaml");
}

#[test]
fn yaml_rejects_a_tag() {
    let local = Local::new();
    local.write("style.yaml", "page:\n  color: !shade red\n");
    assert_in_file(&first_error(local.prepare(PLAIN, &["style.yaml"], &[])), "dm.data.malformed", "style.yaml");
}

// TOML loader

#[test]
fn toml_rejects_trailing_content() {
    let local = Local::new();
    local.write("style.toml", "margin = 1 x\n");
    assert_in_file(&first_error(local.prepare(PLAIN, &["style.toml"], &[])), "dm.data.malformed", "style.toml");
}

#[test]
fn toml_rejects_a_non_finite_number() {
    let local = Local::new();
    local.write("style.toml", "[page]\nmax-width = inf\n");
    assert_in_file(&first_error(local.prepare(PLAIN, &["style.toml"], &[])), "dm.data.malformed", "style.toml");
}

#[test]
fn toml_rejects_an_inexact_integer() {
    let local = Local::new();
    local.write("style.toml", "margin = 9007199254740993\n");
    assert_in_file(&first_error(local.prepare(PLAIN, &["style.toml"], &[])), "dm.data.malformed", "style.toml");
}

#[test]
fn toml_absent_members_stay_absent() {
    let local = Local::new();
    local.write("style.toml", "[page]\nmargin-left = 2\n");
    let document = "---\nstyle:\n  page:\n    margin-top: 3\n---\n# Notes\n";
    let prepared = local.prepare(document, &["style.toml"], &[]).expect("prepares");
    assert_eq!((page(&prepared)["margin-left"].clone(), page(&prepared)["margin-top"].clone()), (json!(2), json!(3)));
}

// CLI layer rules

#[test]
fn a_repeated_setter_keeps_its_last_value() {
    let local = Local::new();
    let prepared = local.prepare(PLAIN, &[], &["style.margin=2", "style.margin=4"]).expect("prepares");
    assert_eq!(page(&prepared)["margin-top"], json!(4));
}

#[test]
fn a_repeated_whole_style_setter_replaces_the_whole_assignment() {
    let local = Local::new();
    let prepared = local.prepare(PLAIN, &[], &["style={ margin: 2 }", "style={ max-width: 50 }"]).expect("prepares");
    assert_eq!(page(&prepared), &json!({"max-width": 50}));
}

#[test]
fn an_inline_empty_style_is_a_no_op_and_an_empty_file_fails() {
    let local = Local::new();
    let document = "---\nstyle:\n  margin: 3\n---\n# Notes\n";
    let prepared = local.prepare(document, &[], &["style={}"]).expect("a no-op");
    assert_eq!(page(&prepared)["margin-top"], json!(3));
    local.write("empty.yaml", "{}\n");
    assert_in_file(&first_error(local.prepare(document, &["empty.yaml"], &[])), "dm.style.empty_file", "empty.yaml");
}

#[test]
fn an_invalid_overridden_file_fails_at_its_own_line() {
    let local = Local::new();
    local.write("one.yaml", "margin: 1\nmax-width: abc\n");
    local.write("two.yaml", "max-width: 50\n");
    let record = first_error(local.prepare(PLAIN, &["one.yaml", "two.yaml"], &[]));
    assert_in_file(&record, "dm.schema.constraint", "one.yaml");
    assert_eq!(record.source.as_ref().and_then(|source| source.line), Some(2), "{record:?}");
}

#[test]
fn a_custom_open_style_schema_admits_custom_keys_silently() {
    let local = Local::new();
    let document = "---\n$schema:\n  style:\n    foo: string\n    \"<string>\": unknown\n---\n# Notes\n";
    local.write("style.yaml", "foo: bar\nextra: 1\nmargin: 2\n");
    let prepared = local.prepare(document, &["style.yaml"], &["style.mx=3"]).expect("admitted");
    assert!(prepared.diagnostics().iter().all(|diagnostic| diagnostic.code.is_none()), "{:?}", prepared.diagnostics());
    assert_eq!(prepared.retained(), &json!({"foo": "bar", "extra": 1}));
    assert_eq!((page(&prepared)["margin-top"].clone(), page(&prepared)["margin-left"].clone()), (json!(2), json!(3)));
}

#[test]
fn a_setter_problem_names_its_argument_and_canonical_path_and_no_position() {
    let local = Local::new();
    let record = first_error(local.prepare(PLAIN, &[], &["style.mb=-4"]));
    assert_eq!(record.code, "dm.schema.type_mismatch", "{record:?}");
    assert!(record.source.is_none(), "a setter is in no file: {record:?}");
    assert!(record.message.contains("`style.page.mb`") && record.message.contains("style.mb=-4"), "{}", record.message);
}
