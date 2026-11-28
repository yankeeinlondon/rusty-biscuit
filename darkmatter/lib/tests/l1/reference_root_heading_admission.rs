//! A root document's generated headings are prepared from the source the
//! request admits for it, in every validator that checks a root fragment.
//!
//! A heading written as `# {{ label }}` takes its text from a schema default,
//! and a bare `$schema` name resolves only from the document's own file. The
//! same note given as a loaded file, a relative `Markdown` source, or a
//! buffer whose path is only on the request (relative or absolute) must
//! accept `#ready` and report `#gone` as the one missing heading, through
//! FileTree, fresh validation, and prebuilt-graph validation alike.

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, ComposeSource};
use darkmatter::markdown::reference::file_tree::FileTree;
use darkmatter::markdown::reference::types::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::{ReferenceIssueCode, ReferenceValidationOptions};
use darkmatter::markdown::Markdown;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// Where the document's path is carried.
#[derive(Clone, Copy, Debug)]
enum SourceForm {
    /// Read from its file; no path on the request.
    LoadedFile,
    /// A buffer whose `Markdown` source is relative to the request directory.
    RelativeMarkdown,
    /// A source-less buffer; the relative path only on the request.
    RequestOnlyRelative,
    /// A source-less buffer; the absolute path only on the request.
    RequestOnlyAbsolute,
}

impl SourceForm {
    const ALL: [Self; 4] = [Self::LoadedFile, Self::RelativeMarkdown, Self::RequestOnlyRelative, Self::RequestOnlyAbsolute];
}

/// A repository (with a `.git` directory) whose tree-root `schemas/` folder
/// holds `labels.yaml`, beside a separate home with no schemas of its own.
struct Fixture {
    root: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().unwrap();
        let repo = root.path().join("repo");
        std::fs::create_dir_all(repo.join(".git")).unwrap();
        std::fs::create_dir_all(repo.join("schemas")).unwrap();
        std::fs::create_dir_all(root.path().join("home")).unwrap();
        std::fs::write(repo.join("schemas/labels.yaml"), "$schema:\n  label: string(default(Ready))\n").unwrap();
        Self { root }
    }

    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn note(&self) -> PathBuf {
        self.repo().join("note.md")
    }

    fn write_note(&self, schema: &str, fragment: &str) -> String {
        let text = format!("---\n$schema: {schema}\n---\n# {{{{ label }}}}\n\n[Jump to heading](#{fragment})\n");
        std::fs::write(self.note(), &text).unwrap();
        text
    }

    /// A request in the repository's frozen context (no trigger schemas, as
    /// the reference commands prepare documents), so nothing scans the real
    /// working tree.
    fn request(&self, form: SourceForm) -> ComposeRequest {
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(&self.repo(), ""))
            .with_trigger_schemas(false);
        let options = match form {
            SourceForm::RequestOnlyRelative => options.with_source_file(Path::new("note.md")),
            SourceForm::RequestOnlyAbsolute => options.with_source_file(self.note()),
            SourceForm::LoadedFile | SourceForm::RelativeMarkdown => options,
        };
        let context = FileResolutionContext::from_snapshot(
            self.repo(),
            Some(self.root.path().join("home")),
            std::collections::HashMap::new(),
        )
        .with_repository_root(self.repo());
        crate::request_support::request_in(options, context)
    }

    fn document(&self, form: SourceForm, text: &str) -> Markdown {
        match form {
            SourceForm::LoadedFile => Markdown::try_from(self.note().as_path()).unwrap(),
            SourceForm::RelativeMarkdown => Markdown::from(text).with_source(ComposeSource::File(PathBuf::from("note.md"))),
            SourceForm::RequestOnlyRelative | SourceForm::RequestOnlyAbsolute => Markdown::from(text),
        }
    }

    /// `(reader, issue codes)` for FileTree, fresh, and prebuilt validation
    /// with fragment checking on.
    fn verdicts(&self, form: SourceForm, text: &str) -> Vec<(&'static str, Vec<ReferenceIssueCode>)> {
        let (document, request) = (self.document(form, text), self.request(form));
        let graph_options = ReferenceGraphOptions::with_compose(&request);
        let options =
            || ReferenceValidationOptions { validate_fragments: true, ..ReferenceValidationOptions::with_graph(graph_options.clone()) };
        let codes = |issues: &[darkmatter::markdown::reference::validate::ReferenceIssue]| {
            issues.iter().map(|issue| issue.code).collect::<Vec<_>>()
        };
        let mut tree = FileTree::from_markdown(document.clone(), &request).validate().validation_options(options());
        tree.ensure_built().unwrap_or_else(|error| panic!("{form:?}: file tree: {error}"));
        let tree_issues = codes(&tree.validation_report().expect("a validating tree has a report").issues);
        let fresh = document.validate_references(options()).unwrap_or_else(|error| panic!("{form:?}: fresh: {error}"));
        let graph = document.reference_graph(graph_options.clone()).unwrap_or_else(|error| panic!("{form:?}: graph: {error}"));
        let prebuilt = document
            .validate_references_with_graph(&graph, options())
            .unwrap_or_else(|error| panic!("{form:?}: prebuilt: {error}"));
        vec![("file tree", tree_issues), ("fresh", codes(&fresh.issues)), ("prebuilt", codes(&prebuilt.issues))]
    }
}

fn assert_every_form(schema: &str) {
    let fixture = Fixture::new();
    for (fragment, expected) in [("ready", vec![]), ("gone", vec![ReferenceIssueCode::MissingFragmentTarget])] {
        let text = fixture.write_note(schema, fragment);
        for form in SourceForm::ALL {
            for (reader, issues) in fixture.verdicts(form, &text) {
                assert_eq!(issues, expected, "{schema} #{fragment} {form:?} {reader}");
            }
        }
    }
}

/// R11-F3.1/.2: the bare schema name resolves from the admitted root in
/// every reader and source form.
#[test]
fn root_heading_from_a_bare_schema_is_admitted_in_every_source_form() {
    assert_every_form("labels.yaml");
}

/// R11-F3.1 control: the explicit relative spelling agrees.
#[test]
fn root_heading_from_an_explicit_schema_is_admitted_in_every_source_form() {
    assert_every_form("./schemas/labels.yaml");
}
