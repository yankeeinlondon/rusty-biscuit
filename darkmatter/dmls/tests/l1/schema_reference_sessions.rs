//! `$schema` links, definition, and diagnostics refresh together as schema files change.
//!
//! Each test opens `repo/docs/x.md` declaring a bare-name `$schema` in a live
//! session and changes the schema files on disk, reporting each change
//! through `workspace/didChangeWatchedFiles`. After every step the document
//! link, the definition at the `$schema` value, the hover on that value, and
//! every published diagnostic that names a schema file must name the same
//! file, and all four must already reflect the step.
//!
//! The fixture holds a repository (`repo`, the only workspace folder) and,
//! outside it, a fixture `HOME` and the folder `SCHEMAS_DIR` names
//! (`defs`). Every snapshot is built here; the process environment, home,
//! and current directory are never read.

use crate::common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use biscuit_file::canonicalize_simplified;
use common::{LspFixture, LspWorkspace};
use darkmatter::markdown::compose::RequestSnapshot;
use serde_json::{Value, json};

const CREATED: u32 = 1;
const CHANGED: u32 = 2;
const DELETED: u32 = 3;

const TYPE_MISMATCH: &str = "dm.schema.type_mismatch";
const PREPARE: &str = "dm.schema.prepare";

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn uri(path: &Path) -> String {
    url::Url::from_file_path(path).unwrap().to_string()
}

fn path_of(uri: &str) -> PathBuf {
    let path = url::Url::parse(uri).unwrap().to_file_path().unwrap();
    canonicalize_simplified(&path).unwrap_or(path)
}

/// A YAML schema declaring `title` with `kind`.
fn yaml_schema(kind: &str) -> String {
    format!("$schema:\n  title: {kind}\n")
}

/// A JSON Schema 2020-12 document declaring `title` with `kind`.
fn json_schema(kind: &str) -> String {
    format!(
        "{{\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n  \"type\": \"object\",\n  \"properties\": {{ \"title\": {{ \"type\": \"{kind}\" }} }}\n}}\n"
    )
}

struct Fixture {
    root: PathBuf,
}

impl Fixture {
    /// A minimal repository Git discovery accepts (without running `git`)
    /// beside an empty fixture home. Neither `repo/schemas`, `home/schemas`,
    /// nor `defs` exists until a test writes into it.
    fn new(workspace: &Path) -> Self {
        let fixture = Self { root: canonicalize_simplified(workspace).unwrap() };
        let repo = fixture.repo();
        std::fs::create_dir_all(repo.join(".git/objects")).unwrap();
        std::fs::create_dir_all(repo.join(".git/refs/heads")).unwrap();
        write(&repo.join(".git/HEAD"), "ref: refs/heads/main\n");
        write(&repo.join(".git/config"), "[core]\n\trepositoryformatversion = 0\n\tbare = false\n");
        std::fs::create_dir_all(fixture.home()).unwrap();
        fixture
    }

    fn repo(&self) -> PathBuf {
        self.root.join("repo")
    }

    fn home(&self) -> PathBuf {
        self.root.join("home")
    }

    /// The folder `SCHEMAS_DIR` names, outside the workspace folder.
    fn defs(&self) -> PathBuf {
        self.root.join("defs")
    }

    /// The tree root, `{repo}/schemas`.
    fn tree(&self) -> PathBuf {
        self.repo().join("schemas")
    }

    fn snapshot(&self) -> RequestSnapshot {
        RequestSnapshot::new(&self.root).with_home(Some(self.home())).with_env(HashMap::from([(
            "SCHEMAS_DIR".to_string(),
            self.defs().to_string_lossy().into_owned(),
        )]))
    }
}

/// A watching client whose only workspace folder is the repository.
fn initialize_params(folder: &Path) -> Value {
    json!({
        "processId": null,
        "clientInfo": { "name": "Watched Client", "version": "1.0" },
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] },
            "workspace": { "didChangeWatchedFiles": { "dynamicRegistration": true, "relativePatternSupport": true } }
        },
        "workspaceFolders": [ { "uri": url::Url::from_directory_path(folder).unwrap().as_str(), "name": "repo" } ]
    })
}

/// A live session over `repo/docs/x.md`, whose `$schema` names `reference`
/// and whose `title` is a string.
struct Session<'workspace> {
    lsp: LspFixture<'workspace>,
    doc: String,
    repo: PathBuf,
    /// The `$schema` value's line and its first and last-plus-one columns.
    value: (u32, u32, u32),
}

impl<'workspace> Session<'workspace> {
    fn open(workspace: &'workspace LspWorkspace, fixture: &Fixture, reference: &str) -> Self {
        let document = fixture.repo().join("docs/x.md");
        let text = format!("---\n$schema: {reference}\ntitle: hello\n---\n\nBody\n");
        write(&document, &text);
        let doc = uri(&document);
        let mut lsp = LspFixture::start_with_snapshot(workspace, fixture.snapshot());
        lsp.initialize(initialize_params(&fixture.repo()));
        lsp.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": doc, "languageId": "markdown", "version": 1, "text": text } }),
        );
        let start = "$schema: ".len() as u32;
        Self { lsp, doc, repo: fixture.repo(), value: (1, start, start + reference.len() as u32) }
    }

    /// Writes `text` to `path` and reports it, created or changed.
    fn write(&self, path: &Path, text: &str) {
        let kind = if path.exists() { CHANGED } else { CREATED };
        write(path, text);
        self.watched(path, kind);
    }

    fn delete(&self, path: &Path) {
        std::fs::remove_file(path).unwrap();
        self.watched(path, DELETED);
    }

    fn watched(&self, path: &Path, kind: u32) {
        self.lsp.notify("workspace/didChangeWatchedFiles", json!({ "changes": [ { "uri": uri(path), "type": kind } ] }));
    }

    /// The diagnostics last published once the server has handled every
    /// earlier message and finished the registry jobs they queued, then the
    /// links and the definition the server answers after them.
    fn observe(&mut self) -> Observation {
        let doc = self.doc.clone();
        self.lsp.flush_server_with("textDocument/documentSymbol", json!({ "textDocument": { "uri": doc } }));
        let published = self.lsp.take_buffered_diagnostics(&doc).expect("the step published diagnostics");
        while self.lsp.take_buffered_diagnostics(&doc).is_some() {}
        let diagnostics = published["diagnostics"].as_array().cloned().unwrap_or_default();

        let links = self.lsp.request("textDocument/documentLink", json!({ "textDocument": { "uri": doc } }));
        let links = links.result.unwrap_or(Value::Null).as_array().cloned().unwrap_or_default();
        let (line, start, end) = self.value;
        let definition = self.lsp.request(
            "textDocument/definition",
            json!({ "textDocument": { "uri": doc }, "position": { "line": line, "character": (start + end) / 2 } }),
        );
        let definition = definition
            .result
            .unwrap_or(Value::Null)
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|location| location["uri"].as_str().map(path_of))
            .collect();
        let hover = self.lsp.request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": doc }, "position": { "line": line, "character": (start + end) / 2 } }),
        );
        let hover = hover.result.and_then(|hover| hover["contents"]["value"].as_str().map(str::to_string)).unwrap_or_default();
        Observation { diagnostics, links, definition, hover, value_range: self.value_range(), repo: self.repo.clone() }
    }

    fn value_range(&self) -> Value {
        let (line, start, end) = self.value;
        json!({ "start": { "line": line, "character": start }, "end": { "line": line, "character": end } })
    }

    fn shutdown(self) {
        self.lsp.shutdown();
    }
}

/// What the server answers for the document after one step.
#[derive(Debug)]
struct Observation {
    diagnostics: Vec<Value>,
    links: Vec<Value>,
    definition: Vec<PathBuf>,
    /// The Markdown of the hover on the `$schema` value.
    hover: String,
    value_range: Value,
    repo: PathBuf,
}

impl Observation {
    /// Asserts the one document link spans the `$schema` value and names
    /// `expected`, the definition names `expected`, and every diagnostic that
    /// names a schema file (a type mismatch's related location, a load
    /// failure's message) names `expected` too.
    fn assert_agree_on(&self, expected: &Path, step: &str) {
        let expected = canonicalize_simplified(expected).unwrap_or_else(|_| expected.to_path_buf());
        let [link] = self.links.as_slice() else {
            panic!("{step}: one `$schema` link expected: {self:#?}");
        };
        assert_eq!(link["range"], self.value_range, "{step}: the link spans the value");
        assert_eq!(path_of(link["target"].as_str().unwrap()), expected, "{step}: link target");
        assert_eq!(self.definition, vec![expected.clone()], "{step}: definition target");
        assert_eq!(self.hover_target().as_ref(), Some(&expected), "{step}: hover names the file: {}", self.hover);
        for diagnostic in &self.diagnostics {
            for related in diagnostic["relatedInformation"].as_array().into_iter().flatten() {
                let related = path_of(related["location"]["uri"].as_str().unwrap());
                assert_eq!(related, expected, "{step}: related location of {diagnostic:#?}");
            }
        }
        for message in self.messages(PREPARE) {
            if message.starts_with("invalid ") {
                let named = expected.to_string_lossy();
                assert!(message.contains(named.as_ref()), "{step}: `{message}` names {named}");
            }
        }
    }

    /// The file the value hover names: its link for an existing file, else
    /// the `Missing:` creation path (repository-relative inside the
    /// repository).
    fn hover_target(&self) -> Option<PathBuf> {
        if let Some((_, rest)) = self.hover.rsplit_once("](") {
            return Some(path_of(rest.strip_suffix(')').unwrap_or(rest).split(')').next().unwrap()));
        }
        let (_, rest) = self.hover.split_once("Missing: `")?;
        let display = rest.split('`').next()?;
        let path = Path::new(display);
        Some(if path.is_absolute() { path.to_path_buf() } else { self.repo.join(display) })
    }

    /// Asserts the value hover previews a valid schema whose definition
    /// holds `line`.
    fn assert_hover_previews(&self, line: &str, step: &str) {
        assert!(self.hover.contains("Schema Definition:") && self.hover.contains(line), "{step}: {}", self.hover);
    }

    fn codes(&self) -> Vec<&str> {
        self.diagnostics.iter().filter_map(|diagnostic| diagnostic["code"].as_str()).collect()
    }

    fn messages(&self, code: &str) -> Vec<&str> {
        self.diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["code"] == json!(code))
            .filter_map(|diagnostic| diagnostic["message"].as_str())
            .collect()
    }

    /// The type `title` was checked against, from the one type mismatch.
    fn mismatch(&self) -> Option<&str> {
        let messages = self.messages(TYPE_MISMATCH);
        assert!(messages.len() <= 1, "{self:#?}");
        messages.first().map(|message| message.rsplit(' ').next().unwrap().trim_matches('"'))
    }

    /// Asserts the only diagnostic is the missing-`$schema` error, ranged on
    /// the value like the link.
    fn assert_missing_schema(&self, reference: &str, step: &str) {
        let [diagnostic] = self.diagnostics.as_slice() else {
            panic!("{step}: one missing-schema diagnostic expected: {self:#?}");
        };
        assert_eq!(diagnostic["code"], json!(PREPARE), "{step}");
        assert_eq!(
            diagnostic["message"],
            json!(format!("could not resolve $schema reference `{reference}`")),
            "{step}"
        );
        assert_eq!(diagnostic["range"], self.value_range, "{step}: ranged on the value");
        assert!(
            self.hover.starts_with(&format!("`{reference}` does not match a schema file.")),
            "{step}: the hover explains the miss: {}",
            self.hover
        );
    }

    /// Asserts the only diagnostic is a load failure of the selected file.
    fn assert_invalid_target(&self, step: &str) {
        let messages = self.messages(PREPARE);
        assert_eq!(self.codes(), [PREPARE], "{step}: {self:#?}");
        assert!(messages[0].starts_with("invalid "), "{step}: a load failure, not a miss: {messages:?}");
        assert_eq!(self.diagnostics[0]["range"], self.value_range, "{step}: ranged on the value");
        assert!(self.hover.contains("could not be used as a schema"), "{step}: {}", self.hover);
    }
}

#[test]
fn editing_the_selected_file_refreshes_diagnostics_and_keeps_navigation() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let shape = fixture.tree().join("shape.yaml");
    write(&shape, &yaml_schema("string"));
    let mut session = Session::open(&workspace, &fixture, "shape.yaml");

    let initial = session.observe();
    assert!(initial.diagnostics.is_empty(), "`title: string` accepts the text: {initial:#?}");
    initial.assert_agree_on(&shape, "initial");

    session.write(&shape, &yaml_schema("number"));
    let edited = session.observe();
    assert_eq!(edited.mismatch(), Some("number"), "the edit is validated: {edited:#?}");
    edited.assert_hover_previews("title: number", "edited");
    edited.assert_agree_on(&shape, "edited");

    session.write(&shape, &yaml_schema("string"));
    let reverted = session.observe();
    assert!(reverted.diagnostics.is_empty(), "{reverted:#?}");
    reverted.assert_agree_on(&shape, "reverted");
    session.shutdown();
}

/// With the file in the tree root and in the home root, deleting the tree
/// copy selects the home copy; deleting that leaves a clean miss whose
/// link and definition open the first searched root's path.
#[test]
fn deleting_the_selected_file_moves_to_the_next_candidate_then_the_creation_path() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let tree = fixture.tree().join("shape.yaml");
    let home = fixture.home().join("schemas/shape.yaml");
    write(&tree, &yaml_schema("string"));
    write(&home, &yaml_schema("number"));
    let mut session = Session::open(&workspace, &fixture, "shape.yaml");

    let initial = session.observe();
    assert!(initial.diagnostics.is_empty(), "the tree copy shadows home: {initial:#?}");
    initial.assert_agree_on(&tree, "initial");

    session.delete(&tree);
    let fallback = session.observe();
    assert_eq!(fallback.mismatch(), Some("number"), "the home copy is selected: {fallback:#?}");
    fallback.assert_hover_previews("title: number", "tree copy deleted");
    fallback.assert_agree_on(&home, "tree copy deleted");

    session.delete(&home);
    let missing = session.observe();
    missing.assert_missing_schema("shape.yaml", "home copy deleted");
    // `defs` does not exist, so the tree root is the first searched root.
    missing.assert_agree_on(&tree, "home copy deleted");
    session.shutdown();
}

/// `SCHEMAS_DIR` names an existing, empty folder outside the workspace
/// folder; a file created there outranks the tree root's, and removing it
/// selects the tree file again.
#[test]
fn a_schemas_dir_candidate_outside_the_workspace_takes_over_and_hands_back() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    std::fs::create_dir_all(fixture.defs()).unwrap();
    let tree = fixture.tree().join("shape.yaml");
    let nearer = fixture.defs().join("shape.yaml");
    write(&tree, &yaml_schema("string"));
    let mut session = Session::open(&workspace, &fixture, "shape.yaml");

    let initial = session.observe();
    assert!(initial.diagnostics.is_empty(), "{initial:#?}");
    initial.assert_agree_on(&tree, "initial");

    session.write(&nearer, &yaml_schema("boolean"));
    let shadowed = session.observe();
    assert_eq!(shadowed.mismatch(), Some("boolean"), "the `SCHEMAS_DIR` file is selected: {shadowed:#?}");
    shadowed.assert_hover_previews("title: boolean", "SCHEMAS_DIR file created");
    shadowed.assert_agree_on(&nearer, "SCHEMAS_DIR file created");

    session.delete(&nearer);
    let restored = session.observe();
    assert!(restored.diagnostics.is_empty(), "the tree file is selected again: {restored:#?}");
    restored.assert_agree_on(&tree, "SCHEMAS_DIR file deleted");
    session.shutdown();
}

/// The `SCHEMAS_DIR` folder does not exist at startup, so it is no root;
/// creating it with the file adds the root, which then outranks the tree.
#[test]
fn a_created_schemas_dir_root_outside_the_workspace_takes_over() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let tree = fixture.tree().join("shape.yaml");
    let nearer = fixture.defs().join("shape.yaml");
    write(&tree, &yaml_schema("string"));
    let mut session = Session::open(&workspace, &fixture, "shape.yaml");

    let initial = session.observe();
    assert!(initial.diagnostics.is_empty(), "{initial:#?}");
    initial.assert_agree_on(&tree, "initial");

    session.write(&nearer, &yaml_schema("boolean"));
    let shadowed = session.observe();
    assert_eq!(shadowed.mismatch(), Some("boolean"), "the new root's file is selected: {shadowed:#?}");
    shadowed.assert_hover_previews("title: boolean", "SCHEMAS_DIR root created");
    shadowed.assert_agree_on(&nearer, "SCHEMAS_DIR root created");
    session.shutdown();
}

/// A missing reference in a repository with a tree root links to the tree
/// root's path; creating the file in the not-yet-existing home root, outside
/// the workspace folder, selects it and clears the missing-schema error.
#[test]
fn a_candidate_in_a_created_home_root_resolves_a_missing_reference() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    std::fs::create_dir_all(fixture.tree()).unwrap();
    let home = fixture.home().join("schemas/shape.yaml");
    let mut session = Session::open(&workspace, &fixture, "shape.yaml");

    let missing = session.observe();
    missing.assert_missing_schema("shape.yaml", "initial");
    missing.assert_agree_on(&fixture.tree().join("shape.yaml"), "initial");

    session.write(&home, &yaml_schema("number"));
    let found = session.observe();
    assert_eq!(found.mismatch(), Some("number"), "the home file is selected: {found:#?}");
    found.assert_hover_previews("title: number", "home root created");
    found.assert_agree_on(&home, "home root created");
    session.shutdown();
}

#[test]
fn repairing_a_malformed_target_clears_the_error_and_keeps_the_link() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let shape = fixture.tree().join("shape.yaml");
    write(&shape, "$schema:\n  title: [unclosed\n");
    let mut session = Session::open(&workspace, &fixture, "shape.yaml");

    let malformed = session.observe();
    malformed.assert_invalid_target("malformed");
    malformed.assert_agree_on(&shape, "malformed");

    session.write(&shape, &yaml_schema("string"));
    let repaired = session.observe();
    assert!(repaired.diagnostics.is_empty(), "the repair clears the error: {repaired:#?}");
    repaired.assert_hover_previews("title: string", "repaired");
    repaired.assert_agree_on(&shape, "repaired");

    session.write(&shape, "$schema:\n  title: [unclosed\n");
    let broken = session.observe();
    broken.assert_invalid_target("broken again");
    broken.assert_agree_on(&shape, "broken again");
    session.shutdown();
}

/// A JSON Schema 2020-12 target follows the same steps: an edit, a
/// malformed save and its repair, and a deletion.
#[test]
fn a_json_target_refreshes_like_a_yaml_one() {
    let workspace = LspWorkspace::new();
    let fixture = Fixture::new(workspace.path());
    let shape = fixture.tree().join("shape.json");
    write(&shape, &json_schema("string"));
    let mut session = Session::open(&workspace, &fixture, "shape.json");

    let initial = session.observe();
    assert!(initial.diagnostics.is_empty(), "{initial:#?}");
    initial.assert_agree_on(&shape, "initial");

    session.write(&shape, &json_schema("number"));
    let edited = session.observe();
    assert_eq!(edited.mismatch(), Some("number"), "{edited:#?}");
    edited.assert_hover_previews("\"type\": \"number\"", "edited");
    edited.assert_agree_on(&shape, "edited");

    session.write(&shape, "{ \"$schema\": \"https://json-schema.org/draft/2020-12/schema\", ");
    let malformed = session.observe();
    malformed.assert_invalid_target("malformed");
    malformed.assert_agree_on(&shape, "malformed");

    session.write(&shape, &json_schema("string"));
    let repaired = session.observe();
    assert!(repaired.diagnostics.is_empty(), "{repaired:#?}");
    repaired.assert_agree_on(&shape, "repaired");

    session.delete(&shape);
    let missing = session.observe();
    missing.assert_missing_schema("shape.json", "deleted");
    missing.assert_agree_on(&shape, "deleted");
    session.shutdown();
}
