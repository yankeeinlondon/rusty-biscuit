//! A caller-supplied file value keeps its launch-directory meaning when the
//! property's file type is reached through a reference.
//!
//! Every case composes one prompt in `document/` from a request captured in
//! `launch/`; both folders hold a different `spec.md`, so a value left
//! relative would silently select the document's file. Each recursive or
//! named spelling must project exactly what its direct control projects.

use std::path::{Path, PathBuf};
use std::process::Command;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};
use serde_json::{Value, json};

struct Layout {
    _dir: tempfile::TempDir,
    root: PathBuf,
    launch: PathBuf,
}

impl Layout {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        let status = Command::new("git")
            .args(["init", "-q"])
            .current_dir(&root)
            .status()
            .expect("run git");
        assert!(status.success());
        let launch = root.join("launch");
        for folder in ["launch", "document"] {
            std::fs::create_dir_all(root.join(folder)).expect("mkdir");
            std::fs::write(root.join(folder).join("spec.md"), format!("# {folder}\n")).expect("write spec");
        }
        Self { _dir: dir, root, launch }
    }

    /// The launch file's identity, as an eager file projects it.
    fn launch_spec(&self) -> Value {
        json!(self.launch.join("spec.md").to_string_lossy())
    }

    /// The composed `spec` for a prompt whose `$schema` block is `schema`
    /// (indented YAML) and a caller override `spec`.
    fn compose(&self, schema: &str, spec: Value) -> Result<Value, String> {
        let document = self.root.join("document/prompt.md");
        std::fs::write(&document, format!("---\n$schema:\n{schema}---\nBody\n")).expect("write prompt");
        let options = ComposeOptions::new()
            .with_source_file(&document)
            .with_set_overrides(json!({ "spec": spec }));
        let snapshot = RequestSnapshot::new(&self.launch).with_home(biscuit_file::home_dir());
        let request = ComposeRequest::prepare(options, &snapshot).expect("request");
        let markdown = Markdown::try_from(document.as_path()).expect("read prompt");
        markdown
            .compose_with(&request)
            .map(|(composed, _)| composed.frontmatter().as_map()["spec"].clone())
            .map_err(|error| format!("{error:?}"))
    }

    fn composed(&self, schema: &str, spec: Value) -> Value {
        self.compose(schema, spec)
            .unwrap_or_else(|error| panic!("compose `{schema}`: {error}"))
    }
}

fn assert_launch_candidate(value: &Value, launch: &Path) {
    let text = value.as_str().unwrap_or_else(|| panic!("a path, got {value}"));
    let path = Path::new(text);
    assert!(path.is_absolute(), "{text} is absolute");
    assert_eq!(
        biscuit_file::canonicalize_simplified(path).expect("candidate exists"),
        launch.join("spec.md"),
        "{text} names the launch file",
    );
}

const DIRECT_EAGER: &str = "  spec: file(eager)\n";
const DIRECT_LAZY: &str = "  spec: file\n";
const RECURSIVE_EAGER: &str = "  node: \"file(eager) | { children: node[] }\"\n  spec: node\n";
const RECURSIVE_LAZY: &str = "  node: \"file | { children: node[] }\"\n  spec: node\n";

/// R1-F3.1 and its R1-F3.5 control.
#[test]
fn a_caller_file_through_a_recursive_eager_reference_names_the_launch_file() {
    let layout = Layout::new();
    let direct = layout.composed(DIRECT_EAGER, json!("spec.md"));
    assert_eq!(direct, layout.launch_spec(), "direct control");
    assert_eq!(layout.composed(RECURSIVE_EAGER, json!("spec.md")), direct);
}

/// R1-F3.2 and its R1-F3.5 control.
#[test]
fn a_caller_file_through_a_recursive_lazy_reference_names_the_launch_candidate() {
    let layout = Layout::new();
    let direct = layout.composed(DIRECT_LAZY, json!("spec.md"));
    assert_launch_candidate(&direct, &layout.launch);
    assert_eq!(layout.composed(RECURSIVE_LAZY, json!("spec.md")), direct);
}

/// R1-F3.3 and its R1-F3.7 control.
#[test]
fn caller_items_through_a_recursive_eager_reference_name_the_launch_file() {
    let layout = Layout::new();
    let direct = layout.composed("  spec: file(eager)[]\n", json!(["spec.md"]));
    assert_eq!(direct, json!([layout.launch_spec()]), "direct control");
    let recursive = "  node: \"file(eager) | { children: node[] }\"\n  spec: node[]\n";
    assert_eq!(layout.composed(recursive, json!(["spec.md"])), direct);
}

/// R1-F3.4 and its R1-F3.7 control.
#[test]
fn caller_items_through_a_recursive_lazy_reference_name_the_launch_candidate() {
    let layout = Layout::new();
    let direct = layout.composed("  spec: file[]\n", json!(["spec.md"]));
    assert_launch_candidate(&direct[0], &layout.launch);
    let recursive = "  node: \"file | { children: node[] }\"\n  spec: node[]\n";
    assert_eq!(layout.composed(recursive, json!(["spec.md"])), direct);
}

/// R1-F3.6: an acyclic named type is inlined, so it is the control for
/// "named" rather than "recursive".
#[test]
fn a_caller_file_through_an_acyclic_named_type_names_the_launch_file() {
    let layout = Layout::new();
    let named = "  doc: \"file(eager) | { title: string }\"\n  spec: doc\n";
    assert_eq!(layout.composed(named, json!("spec.md")), layout.launch_spec());
}

/// A recursive child that is not a string is not a caller file: the value
/// is kept as supplied, and no absent child is materialized.
#[test]
fn a_caller_object_through_a_recursive_reference_is_left_as_supplied() {
    let layout = Layout::new();
    let supplied = json!({ "children": [] });
    assert_eq!(layout.composed(RECURSIVE_EAGER, supplied.clone()), supplied);
}

/// The union stays ambiguous when a string matches both a file alternative
/// and a plain string alternative: no arm is guessed, so the value is kept.
#[test]
fn an_ambiguous_recursive_union_keeps_the_caller_value() {
    let layout = Layout::new();
    let ambiguous = "  node: \"file(eager) | string | { children: node[] }\"\n  spec: node\n";
    assert_eq!(layout.composed(ambiguous, json!("spec.md")), json!("spec.md"));
}

/// R1-F3.9: in a root-union arm the file alternative is reached through a
/// `$ref` to the arm's enforcing copy of `node` (which carries `match`),
/// directly or as one alternative of a property union. A caller file inside
/// the glob is judged from the launch directory and projected to the launch
/// file, not the document's own `spec.md`.
///
/// `notes/notes.md` exists only in `launch/` and is outside the glob, so the
/// union fails; the glob does not stop the arm being a file arm, so the
/// verdict names the projected launch file.
#[test]
fn a_caller_file_through_a_root_union_match_copy_names_the_launch_file() {
    let layout = Layout::new();
    std::fs::create_dir_all(layout.launch.join("notes")).expect("mkdir");
    std::fs::write(layout.launch.join("notes/notes.md"), "notes\n").expect("write notes");
    for spelling in ["node(required)", "\"node | number\""] {
        let schema = format!(
            "  - node: \"file(eager; match(./*.md)) | {{ children: node[] }}\"\n    spec: {spelling}\n  - spec: file(eager; required; match(./*.txt))\n"
        );
        assert_eq!(layout.composed(&schema, json!("spec.md")), layout.launch_spec(), "{spelling}");
        let error = layout.compose(&schema, json!("notes/notes.md")).expect_err("no arm applies");
        // Which arm the verdict reports is closest-arm policy; only the
        // projected identity matters here.
        // `compose` renders the error with `{:?}`. The `match` verdict names
        // the path as written, while the `anyOf` verdict quotes the instance
        // as JSON, so on Windows its separators are escaped twice.
        let launch_notes = layout.launch.join("notes").join("notes.md").to_string_lossy().into_owned();
        let as_written = format!("{launch_notes:?}");
        let as_json = format!("{:?}", serde_json::to_string(&launch_notes).expect("json path"));
        let as_written = as_written.trim_matches('"');
        let as_json = as_json.trim_matches('"').trim_start_matches("\\\"").trim_end_matches("\\\"");
        assert!(error.contains(as_written) || error.contains(as_json), "{spelling}: {error}");
    }
}
