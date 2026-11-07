//! `--style` files and style setters read through the exported
//! `darkmatter_cli::render_style` API, in process: the reference/offset
//! adapter, each loader's control and failures, layer precedence, setters,
//! the document's schema, and the run records a failure carries.
//!
//! Every fixture and request is anchored in a temporary directory.

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use biscuit_file::{PathPosition, ResolutionFailure};
use color_eyre::eyre::Report;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::RequestSnapshot;
use darkmatter::markdown::compose::expression::{Diagnostic, Severity};
use darkmatter::style::{PreparedStyle, StylePreparationPolicy, StyleSourceKind};
use darkmatter_cli::args::RawStyleSetter;
use darkmatter_cli::io::{DocumentArgumentError, OpenedArgument, open_argument};
use darkmatter_cli::output::report::failure_diagnostics;
use darkmatter_cli::render_style::{CliStyleInputs, load_style_file, prepare_cli_style_inputs, setter_layer};
use darkmatter_cli::request::MdRequest;
use serde_json::{Value, json};
use tempfile::TempDir;

const PLAIN_DOCUMENT: &str = "# Notes\n";

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
    magic: Option<PathBuf>,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        std::fs::create_dir_all(root.join("home")).expect("home");
        Self { _dir: dir, root, magic: None }
    }

    /// A fixture whose request registers `shared/` as a magic root.
    fn with_magic_root() -> Self {
        let mut fixture = Self::new();
        let magic = fixture.root.join("shared");
        std::fs::create_dir_all(&magic).expect("magic root");
        fixture.magic = Some(magic);
        fixture
    }

    fn write(&self, relative: &str, contents: impl AsRef<[u8]>) -> PathBuf {
        let path = self.root.join(relative);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("folders");
        std::fs::write(&path, contents).expect("write");
        path
    }

    fn request(&self) -> MdRequest {
        let snapshot = RequestSnapshot::new(&self.root).with_home(Some(self.root.join("home")));
        let snapshot = match &self.magic {
            Some(magic) => snapshot.with_magic_root(magic, PathPosition::Start),
            None => snapshot,
        };
        MdRequest::new(snapshot)
    }

    /// The Markdown document `doc.md`, opened as an `md` argument.
    fn document(&self, text: &str, request: &MdRequest) -> (Markdown, OpenedArgument) {
        self.write("doc.md", text);
        let opened = open_argument(Path::new("doc.md"), request).expect("doc.md opens");
        let markdown = Markdown::try_from(opened.path()).expect("doc.md parses");
        (markdown, opened)
    }

    fn inputs(&self, document: &str, styles: &[&str], setters: &[&str]) -> Result<CliStyleInputs, Report> {
        let request = self.request();
        let (markdown, opened) = self.document(document, &request);
        let styles: Vec<OsString> = styles.iter().map(OsString::from).collect();
        prepare_cli_style_inputs(&markdown, Some(&opened), &styles, &raw_setters(setters), &request)
    }

    /// Prepares `styles` and `setters` over a document with no `style:`.
    fn prepare(&self, styles: &[&str], setters: &[&str]) -> Result<PreparedStyle, Report> {
        self.inputs(PLAIN_DOCUMENT, styles, setters)?.prepare(None, StylePreparationPolicy::render())
    }
}

fn raw_setters(setters: &[&str]) -> Vec<RawStyleSetter> {
    setters
        .iter()
        .map(|argument| {
            let (key, value) = argument.split_once('=').expect("a setter has `=`");
            RawStyleSetter { argument: (*argument).to_string(), key: key.to_string(), value: value.to_string() }
        })
        .collect()
}

fn margins(prepared: &PreparedStyle) -> Value {
    let page = &prepared.value()["page"];
    json!([page["margin-top"], page["margin-right"], page["margin-bottom"], page["margin-left"]])
}

fn failure(result: Result<PreparedStyle, Report>) -> Report {
    match result {
        Ok(prepared) => panic!("expected a failure, got {:?}", prepared.value()),
        Err(error) => error,
    }
}

fn codes(error: &Report) -> Vec<String> {
    failure_diagnostics(error)
        .into_iter()
        .filter(|record| record.severity == Severity::Error)
        .map(|record| record.code)
        .collect()
}

fn first_error(error: &Report) -> Diagnostic {
    failure_diagnostics(error)
        .into_iter()
        .find(|record| record.severity == Severity::Error)
        .unwrap_or_else(|| panic!("no error record: {error:#}"))
}

/// The failure of `result`, which must hold an error record with `code`.
fn fails_with(result: Result<PreparedStyle, Report>, code: &str) -> Report {
    let error = failure(result);
    assert!(codes(&error).iter().any(|seen| seen == code), "expected {code}: {:?} ({error:#})", codes(&error));
    error
}

fn assert_fails_with(result: Result<PreparedStyle, Report>, code: &str) {
    let _ = fails_with(result, code);
}

fn argument_failure(error: &Report) -> &DocumentArgumentError {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<DocumentArgumentError>())
        .unwrap_or_else(|| panic!("no DocumentArgumentError in the chain: {error:#}"))
}

// Reference and offset adapter

#[test]
fn a_whole_file_sets_every_page_margin() {
    let fixture = Fixture::new();
    fixture.write("style.yaml", "margin: 2\n");
    let prepared = fixture.prepare(&["style.yaml"], &[]).expect("prepares");
    assert_eq!(margins(&prepared), json!([2, 2, 2, 2]));
}

#[test]
fn an_offset_selects_a_mapping_inside_the_file() {
    let fixture = Fixture::new();
    fixture.write("style.yaml", "margin: 9\nstyle:\n  margin: 2\ntheme:\n  dark:\n    margin: 3\n");
    assert_eq!(margins(&fixture.prepare(&["style@style.yaml"], &[]).expect("style")), json!([2, 2, 2, 2]));
    assert_eq!(margins(&fixture.prepare(&["theme.dark@style.yaml"], &[]).expect("nested")), json!([3, 3, 3, 3]));
}

#[test]
fn an_offset_reads_escaped_dots_and_existing_array_indexes() {
    let fixture = Fixture::new();
    fixture.write("style.yaml", "a.b:\n  margin: 4\nthemes:\n  - margin: 1\n  - margin: 5\n");
    assert_eq!(margins(&fixture.prepare(&[r"a\.b@style.yaml"], &[]).expect("escaped")), json!([4, 4, 4, 4]));
    assert_eq!(margins(&fixture.prepare(&["themes.1@style.yaml"], &[]).expect("index")), json!([5, 5, 5, 5]));
    assert_fails_with(fixture.prepare(&["themes.2@style.yaml"], &[]), "dm.style.invalid_root");
}

#[test]
fn an_at_sign_after_a_separator_is_part_of_the_file_name() {
    let fixture = Fixture::new();
    fixture.write("user@host.yaml", "margin: 6\n");
    fixture.write("sub/user@host.yaml", "margin: 7\n");
    assert_eq!(margins(&fixture.prepare(&["./user@host.yaml"], &[]).expect("explicit")), json!([6, 6, 6, 6]));
    assert_eq!(margins(&fixture.prepare(&["sub/user@host.yaml"], &[]).expect("separator")), json!([7, 7, 7, 7]));
}

#[test]
fn a_magic_reference_keeps_its_at_sign_and_takes_an_offset() {
    let fixture = Fixture::with_magic_root();
    fixture.write("shared/shared.yaml", "margin: 1\nstyle:\n  margin: 8\n");
    assert_eq!(margins(&fixture.prepare(&["@/shared.yaml"], &[]).expect("magic")), json!([1, 1, 1, 1]));
    assert_eq!(margins(&fixture.prepare(&["style@@/shared.yaml"], &[]).expect("offset")), json!([8, 8, 8, 8]));
}

#[test]
fn a_leading_at_sign_is_a_magic_reference_not_an_empty_offset() {
    let fixture = Fixture::new();
    let error = failure(fixture.prepare(&["@missing.yaml"], &[]));
    assert_eq!(argument_failure(&error).resolution_failure(), ResolutionFailure::NoMatch);
}

#[test]
fn a_missing_or_non_mapping_offset_names_the_key_and_the_file() {
    let fixture = Fixture::new();
    fixture.write("style.yaml", "margin: 2\n");
    let missing = fails_with(fixture.prepare(&["nope@style.yaml"], &[]), "dm.style.invalid_root");
    let record = first_error(&missing);
    assert!(record.message.contains("`nope`") && record.message.contains("style.yaml"), "{}", record.message);
    let scalar = fails_with(fixture.prepare(&["margin@style.yaml"], &[]), "dm.style.invalid_root");
    assert!(first_error(&scalar).message.contains("not a mapping"), "{scalar:#}");
}

#[test]
fn an_empty_file_part_and_stdin_are_refused() {
    let fixture = Fixture::new();
    for argument in ["style@", "-", "style@-"] {
        let error = fails_with(fixture.prepare(&[argument], &[]), "dm.input.read_failure");
        assert!(!error.to_string().is_empty(), "{argument}");
    }
}

#[test]
fn a_remote_style_file_is_refused_before_any_request() {
    // Classification refuses it; nothing resolves or fetches the URL.
    let fixture = Fixture::new();
    let error = failure(fixture.prepare(&["https://example.com/s.yaml"], &[]));
    assert_eq!(argument_failure(&error).resolution_failure(), ResolutionFailure::UnsupportedRemote);
    assert_eq!(codes(&error), ["dm.input.read_failure"]);
}

#[test]
fn only_json_json5_yaml_and_toml_files_are_style_files() {
    let fixture = Fixture::new();
    for name in ["style.jsonl", "style.csv", "notes.md"] {
        fixture.write(name, "margin: 2\n");
        let error = fails_with(fixture.prepare(&[name], &[]), "dm.input.read_failure");
        assert!(error.to_string().contains("is not a style file"), "{name}: {error}");
    }
}

#[test]
fn a_missing_file_keeps_its_failure_class() {
    let fixture = Fixture::new();
    let error = failure(fixture.prepare(&["absent.yaml"], &[]));
    assert_eq!(argument_failure(&error).resolution_failure(), ResolutionFailure::NoMatch);
    let record = first_error(&error);
    assert_eq!(record.code, "dm.input.read_failure");
    assert_eq!(record.reference_details.and_then(|details| details.failure).as_deref(), Some("no-match"));
}

#[test]
fn an_inline_object_suggests_the_inline_setter() {
    let fixture = Fixture::new();
    let error = failure(fixture.prepare(&["{ margin: 4 }"], &[]));
    assert!(error.to_string().contains("style=\"{ margin: 4 }\""), "{error}");
}

// Loaders

#[test]
fn json_keeps_the_last_duplicate_key_and_rejects_trailing_text() {
    let fixture = Fixture::new();
    fixture.write("style.json", r#"{"margin": 1, "margin": 2}"#);
    assert_eq!(margins(&fixture.prepare(&["style.json"], &[]).expect("last wins")), json!([2, 2, 2, 2]));
    fixture.write("bad.json", r#"{"margin": 1} x"#);
    assert_fails_with(fixture.prepare(&["bad.json"], &[]), "dm.data.malformed");
}

#[test]
fn json5_reads_its_relaxed_syntax_and_rejects_trailing_text() {
    let fixture = Fixture::new();
    fixture.write("style.json5", "// a comment\n{ margin: 2, 'max-width': 40, }\n");
    let prepared = fixture.prepare(&["style.json5"], &[]).expect("relaxed syntax");
    assert_eq!(margins(&prepared), json!([2, 2, 2, 2]));
    assert_eq!(prepared.value()["page"]["max-width"], json!(40));
    fixture.write("bad.json5", "{ margin: 2 } x");
    assert_fails_with(fixture.prepare(&["bad.json5"], &[]), "dm.data.malformed");
}

#[test]
fn yaml_and_yml_read_one_document_without_duplicate_keys() {
    let fixture = Fixture::new();
    fixture.write("style.yml", "margin: 3\n");
    assert_eq!(margins(&fixture.prepare(&["style.yml"], &[]).expect("yml")), json!([3, 3, 3, 3]));
    fixture.write("duplicate.yaml", "margin: 1\nmargin: 2\n");
    assert_fails_with(fixture.prepare(&["duplicate.yaml"], &[]), "dm.data.malformed");
    fixture.write("two.yaml", "margin: 1\n---\nmargin: 2\n");
    assert_fails_with(fixture.prepare(&["two.yaml"], &[]), "dm.data.malformed");
}

#[test]
fn a_yaml_explicit_null_alone_is_an_empty_style_file() {
    let fixture = Fixture::new();
    fixture.write("style.yaml", "page:\n  color:\n");
    assert_fails_with(fixture.prepare(&["style.yaml"], &[]), "dm.style.empty_file");
}

#[test]
fn toml_reads_a_table_at_an_offset_and_rejects_duplicate_keys() {
    let fixture = Fixture::new();
    fixture.write("style.toml", "[theme]\nmargin = 2\n");
    assert_eq!(margins(&fixture.prepare(&["theme@style.toml"], &[]).expect("table")), json!([2, 2, 2, 2]));
    fixture.write("bad.toml", "margin = 1\nmargin = 2\n");
    assert_fails_with(fixture.prepare(&["bad.toml"], &[]), "dm.data.malformed");
}

#[test]
fn invalid_utf8_is_malformed_at_its_byte() {
    let fixture = Fixture::new();
    fixture.write("style.yaml", b"margin: \xFF\n");
    let error = fails_with(fixture.prepare(&["style.yaml"], &[]), "dm.data.malformed");
    let location = first_error(&error).data_location.expect("a data location");
    assert_eq!(location.byte_offset, Some(8));
}

#[test]
fn a_root_that_is_not_a_mapping_is_an_invalid_root() {
    let fixture = Fixture::new();
    fixture.write("list.json", "[1]");
    fixture.write("number.json", "42");
    assert_fails_with(fixture.prepare(&["list.json"], &[]), "dm.style.invalid_root");
    assert_fails_with(fixture.prepare(&["number.json"], &[]), "dm.style.invalid_root");
}

#[test]
fn a_file_with_no_admitted_value_is_empty() {
    let fixture = Fixture::new();
    fixture.write("empty.json", "{}");
    assert_fails_with(fixture.prepare(&["empty.json"], &[]), "dm.style.empty_file");
    fixture.write("unknown.yaml", "table:\n  bad: 1\n");
    let error = fails_with(fixture.prepare(&["unknown.yaml"], &[]), "dm.style.empty_file");
    let warnings: Vec<Diagnostic> =
        failure_diagnostics(&error).into_iter().filter(|record| record.severity == Severity::Warning).collect();
    assert!(warnings.iter().any(|record| record.code == "dm.style.unknown_key" && record.message.contains("bad")), "{warnings:?}");
}

#[test]
fn an_unknown_file_key_is_excluded_with_a_warning() {
    let fixture = Fixture::new();
    fixture.write("style.yaml", "margin: 2\nbad-mojo: true\n");
    let inputs = fixture.inputs(PLAIN_DOCUMENT, &["style.yaml"], &[]).expect("reads");
    let prepared = inputs.prepare(None, StylePreparationPolicy::render()).expect("prepares");
    assert_eq!(margins(&prepared), json!([2, 2, 2, 2]));
    assert!(prepared.value().get("bad-mojo").is_none());
    let warnings = inputs.warnings(&prepared);
    assert_eq!(warnings.iter().map(|record| record.code.as_str()).collect::<Vec<_>>(), ["dm.style.unknown_key"]);
    let source = warnings[0].source.as_ref().expect("the style file");
    assert_eq!(source.file.as_deref(), Some(Path::new(&biscuit_file::to_portable_string(&fixture.root.join("style.yaml")))));
    assert_eq!(source.line, Some(2));
}

// Precedence

#[test]
fn a_later_file_wins_key_by_key() {
    let fixture = Fixture::new();
    fixture.write("one.yaml", "margin: 1\nmax-width: 40\n");
    fixture.write("two.yaml", "ml: 5\n");
    let prepared = fixture.prepare(&["one.yaml", "two.yaml"], &[]).expect("prepares");
    assert_eq!(margins(&prepared), json!([1, 1, 1, 5]));
    assert_eq!(prepared.value()["page"]["max-width"], json!(40));
}

#[test]
fn a_file_value_keeps_its_own_context_after_a_later_unrelated_file() {
    let fixture = Fixture::new();
    fixture.write("one/style.yaml", "page:\n  stylesheet: ./theme.css\n");
    fixture.write("one/theme.css", "body {}\n");
    fixture.write("two/style.yaml", "margin: 1\n");
    let prepared = fixture.prepare(&["one/style.yaml", "two/style.yaml"], &[]).expect("prepares");
    let origin = prepared.origin("page.stylesheet").expect("a stylesheet origin");
    assert_eq!(origin.kind, StyleSourceKind::File);
    assert_eq!(origin.source, "one/style.yaml");
    assert_eq!(origin.context.cwd(), fixture.root.join("one"));
    assert_eq!(prepared.origin("page.margin-left").expect("margin").context.cwd(), fixture.root.join("two"));
}

#[test]
fn setters_beat_files() {
    let fixture = Fixture::new();
    fixture.write("style.yaml", "margin: 1\n");
    let prepared = fixture.prepare(&["style.yaml"], &["style.margin=3"]).expect("prepares");
    assert_eq!(margins(&prepared), json!([3, 3, 3, 3]));
    let origin = prepared.origin("page.margin-left").expect("origin");
    assert_eq!(origin.kind, StyleSourceKind::Setters);
    assert_eq!(origin.context.cwd(), fixture.root);
}

#[test]
fn an_invalid_file_fails_even_when_a_later_file_replaces_its_value() {
    let fixture = Fixture::new();
    fixture.write("one.yaml", "max-width: abc\n");
    fixture.write("two.yaml", "max-width: 50\n");
    let error = failure(fixture.prepare(&["one.yaml", "two.yaml"], &[]));
    assert!(error.to_string().contains("one.yaml"), "{error}");
}

// Setters

#[test]
fn a_dotted_setter_and_a_whole_style_setter_apply() {
    let fixture = Fixture::new();
    assert_eq!(margins(&fixture.prepare(&[], &["style.margin=2"]).expect("dotted")), json!([2, 2, 2, 2]));
    let whole = fixture.prepare(&[], &["style={ max-width: 50 }"]).expect("whole");
    assert_eq!(whole.value()["page"]["max-width"], json!(50));
}

#[test]
fn a_bad_setter_fails_with_a_schema_code_and_no_position() {
    let fixture = Fixture::new();
    for (setter, code) in [("style.magin=4", "dm.schema.unknown_key"), ("style.margin=abc", "dm.schema.type_mismatch")] {
        let error = fails_with(fixture.prepare(&[], &[setter]), code);
        let record = first_error(&error);
        assert!(record.source.is_none(), "{setter}: a setter is in no file: {record:?}");
        assert!(record.message.contains(setter), "{setter}: {}", record.message);
    }
}

#[test]
fn a_setter_key_that_is_not_a_property_path_is_refused() {
    let fixture = Fixture::new();
    let error = setter_layer(&raw_setters(&["style..x=1"]), &fixture.request()).expect_err("an empty segment");
    let record = first_error(&error);
    assert_eq!(record.code, "dm.style.invalid_setter");
    assert!(record.message.contains("style..x=1") && record.source.is_none(), "{record:?}");
}

// Schema selection

#[test]
fn the_documents_own_style_schema_decides_what_is_admitted() {
    let fixture = Fixture::new();
    let document = "---\n$schema:\n  style:\n    foo: string\n---\n# Notes\n";
    fixture.write("styles/style.yaml", "margin: 2\nfoo: bar\n");
    let inputs = fixture.inputs(document, &["styles/style.yaml"], &[]).expect("reads");
    let prepared = inputs.prepare(None, StylePreparationPolicy::render()).expect("foo is admitted");
    assert_eq!(prepared.value()["foo"], json!("bar"));
    assert!(prepared.value().get("page").is_none(), "margin is not admitted: {:?}", prepared.value());
    assert!(inputs.warnings(&prepared).iter().any(|record| record.code == "dm.style.unknown_key"));

    let setter = fixture.inputs(document, &[], &["style.margin=4"]).expect("reads");
    let error = failure(setter.prepare(None, StylePreparationPolicy::render()));
    assert!(codes(&error).iter().any(|code| code == "dm.schema.unknown_key"), "{error:#}");
}

#[test]
fn a_document_read_from_stdin_selects_its_schema_in_the_launch_context() {
    let fixture = Fixture::new();
    let request = fixture.request();
    let markdown = Markdown::try_from_content("---\n$schema:\n  style:\n    foo: string\n---\n".to_string()).expect("parses");
    let inputs = prepare_cli_style_inputs(&markdown, None, &[], &raw_setters(&["style.foo=x"]), &request).expect("reads");
    let prepared = inputs.prepare(None, StylePreparationPolicy::render()).expect("foo is admitted");
    assert_eq!(prepared.value()["foo"], json!("x"));
}

// Run records

#[test]
fn a_yaml_value_problem_points_at_the_style_file_and_its_line() {
    let fixture = Fixture::new();
    let path = fixture.write("style.yaml", "margin: 1\npage:\n  max-width: abc\n");
    let error = failure(fixture.prepare(&["style.yaml"], &[]));
    let record = first_error(&error);
    assert!(record.code.starts_with("dm.schema.") || record.code.starts_with("dm.style."), "{}", record.code);
    let source = record.source.expect("a located record");
    assert_eq!(source.file.as_deref(), Some(Path::new(&biscuit_file::to_portable_string(&path))));
    assert_eq!(source.line, Some(3));
}

#[test]
fn a_malformed_yaml_file_is_located() {
    let fixture = Fixture::new();
    let path = fixture.write("style.yaml", "margin: 1\npage: [\n");
    let error = fails_with(fixture.prepare(&["style.yaml"], &[]), "dm.data.malformed");
    let record = first_error(&error);
    let source = record.source.clone().expect("located");
    assert_eq!(source.file.as_deref(), Some(Path::new(&biscuit_file::to_portable_string(&path))));
    assert!(source.line.is_some(), "{record:?}");
}

#[test]
fn a_loaded_file_names_the_path_it_opened() {
    let fixture = Fixture::new();
    let path = fixture.write("style.yaml", "margin: 2\n");
    let loaded = load_style_file(std::ffi::OsStr::new("style.yaml"), &fixture.request()).expect("loads");
    assert_eq!(loaded.path, path);
    assert_eq!(loaded.layer.context().cwd(), fixture.root);
}
