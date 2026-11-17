//! A property with a resolved `default(...)`, required or not, is present
//! while editing when its value is missing or `null`: expressions read it as
//! non-null and no missing-property diagnostic names it. Hover shows the default on the declaration and on a
//! use, and a default the schema refuses is reported at its declaration,
//! never at a position in the frontmatter that no text holds.
//!
//! Boundary: a real LSP session (`LspFixture`) over a temporary workspace,
//! with `expressions.strict` and `schema.strict` on so both the
//! possibly-null and the missing-required diagnostics are published.

use crate::common;

use common::{LspFixture, LspWorkspace};
use serde_json::{Value, json};

const UNCERTAIN: &str = "dm.expression.uncertain_call";
const MISSING_REQUIRED: &str = "dm.schema.missing_required";
const CONSTRAINT: &str = "dm.schema.constraint";

struct Session<'w> {
    fixture: LspFixture<'w>,
    uri: String,
    text: String,
}

impl<'w> Session<'w> {
    fn open(workspace: &'w LspWorkspace, text: &str) -> Self {
        std::fs::write(
            workspace.path().join(".dmls.toml"),
            "[expressions]\nstrict = true\n[schema]\nstrict = true\n",
        )
        .unwrap();
        let path = workspace.path().join("doc.md");
        std::fs::write(&path, text).unwrap();
        let uri = url::Url::from_file_path(&path).unwrap().as_str().to_string();
        let root_uri = url::Url::from_directory_path(workspace.path()).unwrap();
        let mut fixture = LspFixture::start(workspace);
        fixture.initialize(json!({
            "processId": null,
            "capabilities": { "general": { "positionEncodings": ["utf-8"] } },
            "workspaceFolders": [ { "uri": root_uri.as_str(), "name": "defaults" } ]
        }));
        fixture.notify(
            "textDocument/didOpen",
            json!({ "textDocument": { "uri": uri, "languageId": "markdown", "version": 1, "text": text } }),
        );
        Self { fixture, uri, text: text.to_string() }
    }

    /// The latest published diagnostics.
    fn diagnostics(&mut self) -> Vec<Value> {
        self.fixture.flush_server();
        let mut latest = self.fixture.wait_for_diagnostics_params(&self.uri);
        while let Some(params) = self.fixture.take_buffered_diagnostics(&self.uri) {
            latest = params;
        }
        latest["diagnostics"].as_array().cloned().unwrap_or_default()
    }

    /// The hover Markdown at `nudge` bytes into the first `needle` (ASCII
    /// fixtures, so bytes are UTF-8 characters).
    fn hover(&mut self, needle: &str, nudge: usize) -> String {
        let (line, character) = position(&self.text, needle, nudge);
        let response = self.fixture.request(
            "textDocument/hover",
            json!({ "textDocument": { "uri": self.uri }, "position": { "line": line, "character": character } }),
        );
        response
            .result
            .and_then(|result| result["contents"]["value"].as_str().map(str::to_string))
            .unwrap_or_default()
    }

    fn shutdown(self) {
        self.fixture.shutdown();
    }
}

/// `(line, character)` of `nudge` bytes into the first `needle` of `text`.
fn position(text: &str, needle: &str, nudge: usize) -> (u64, u64) {
    let offset = text.find(needle).unwrap_or_else(|| panic!("`{needle}` is in the document")) + nudge;
    let before = &text[..offset];
    let line = before.matches('\n').count() as u64;
    let character = before.rsplit_once('\n').map_or(before.len(), |(_, last)| last.len()) as u64;
    (line, character)
}

/// `(line, code)` of every diagnostic with one of `codes`, sorted.
fn findings(diagnostics: &[Value], codes: &[&str]) -> Vec<(u64, String)> {
    let mut found: Vec<(u64, String)> = diagnostics
        .iter()
        .filter(|diagnostic| codes.iter().any(|code| diagnostic["code"] == json!(code)))
        .map(|diagnostic| {
            (
                diagnostic["range"]["start"]["line"].as_u64().unwrap(),
                diagnostic["code"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    found.sort();
    found
}

/// `n` is required with a default, `o` optional with one, `w` required with
/// one and written `null`, `p` optional without one, `r` required without
/// one; only `w` is set.
const PRESENCE: &str = "---
$schema:
  n: number(required; default(3))
  o: number(default(3))
  w: number(required; default(3))
  p: number
  r: number(required)
w: null
---

n {{ min(n, 5) }}
o {{ min(o, 5) }}
w {{ min(w, 5) }}
p {{ min(p, 5) }}
r {{ min(r, 5) }}
";

#[test]
fn a_defaulted_property_is_present_and_the_controls_are_still_diagnosed() {
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, PRESENCE);
    let diagnostics = session.diagnostics();

    let (p_line, _) = position(PRESENCE, "p {{", 0);
    assert_eq!(
        findings(&diagnostics, &[UNCERTAIN]),
        [(p_line, UNCERTAIN.to_string())],
        "a default fills a missing or null value, required or not; only `p` may be null: {diagnostics:#?}"
    );
    let missing: Vec<&str> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!(MISSING_REQUIRED))
        .map(|diagnostic| diagnostic["message"].as_str().unwrap())
        .collect();
    assert_eq!(missing, ["\"r\" is a required property"], "the default answers `n` and `w`: {diagnostics:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| diagnostic["data"]["instance_path"] != json!("/w")),
        "the written null takes the default: {diagnostics:#?}"
    );
    session.shutdown();
}

/// An optional property's default is used now that a missing value takes it,
/// so a default its definition refuses is reported at the declaration. The
/// session publishes diagnostics only; the document on disk is never edited.
#[test]
fn a_refused_optional_default_is_reported_and_the_document_is_unchanged() {
    let text = "---\n$schema:\n  title: string\n  bad: number(min(5); default(3))\ntitle: Plan\n---\n\nbody\n";
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, text);
    let diagnostics = session.diagnostics();

    let refused: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!(CONSTRAINT)).collect();
    assert_eq!(refused.len(), 1, "{diagnostics:#?}");
    let declaration = "number(min(5); default(3))";
    let (line, start) = position(text, declaration, 0);
    assert_eq!(refused[0]["range"]["start"], json!({ "line": line, "character": start }), "{refused:#?}");
    assert_eq!(refused[0]["data"]["instance_path"], json!("/bad"));
    session.shutdown();
    assert_eq!(std::fs::read_to_string(workspace.path().join("doc.md")).unwrap(), text);
}

#[test]
fn hover_shows_the_default_on_the_declaration_and_on_a_use() {
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, PRESENCE);

    let declaration = session.hover("  n: number", 2);
    assert!(declaration.contains("Required"), "{declaration}");
    assert!(declaration.contains("Default: _3_"), "{declaration}");
    let usage = session.hover("min(n, 5)", 4);
    assert!(usage.contains("Default: _3_"), "{usage}");
    session.shutdown();
}

#[test]
fn a_refused_default_is_reported_at_its_schema_declaration() {
    let text = "---\n$schema:\n  title: string\n  bad: number(required; min(5); default(3))\ntitle: Plan\n---\n\nbody\n";
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, text);
    let diagnostics = session.diagnostics();

    let refused: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!(CONSTRAINT)).collect();
    assert_eq!(refused.len(), 1, "{diagnostics:#?}");
    let declaration = "number(required; min(5); default(3))";
    let (line, start) = position(text, declaration, 0);
    assert_eq!(
        refused[0]["range"],
        json!({
            "start": { "line": line, "character": start },
            "end": { "line": line, "character": start + declaration.len() as u64 }
        }),
        "the declaration's value, not the frontmatter mapping: {diagnostics:#?}"
    );
    assert_eq!(refused[0]["data"]["instance_path"], json!("/bad"));
    assert!(refused[0]["message"].as_str().unwrap().contains("/properties/bad/default"), "{refused:#?}");
    session.shutdown();
}

#[test]
fn a_refused_default_in_a_referenced_schema_is_reported_at_the_reference() {
    let text = "---\n$schema: ./defaults.yaml\ntitle: Plan\n---\n\nbody\n";
    let workspace = LspWorkspace::new();
    std::fs::write(
        workspace.path().join("defaults.yaml"),
        "$schema:\n  title: string\n  bad: number(required; min(5); default(3))\n",
    )
    .unwrap();
    let mut session = Session::open(&workspace, text);
    let diagnostics = session.diagnostics();

    let refused: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["code"] == json!(CONSTRAINT)).collect();
    assert_eq!(refused.len(), 1, "{diagnostics:#?}");
    let (line, start) = position(text, "./defaults.yaml", 0);
    assert_eq!(refused[0]["range"]["start"], json!({ "line": line, "character": start }), "{refused:#?}");
    assert_eq!(refused[0]["range"]["end"]["line"], json!(line), "{refused:#?}");
    session.shutdown();
}

/// A file default that is well-formed but cannot be resolved once the
/// completed value is judged (a `vault:` reference with no vault configured)
/// belongs to the default: it is ranged at the declaration and its data names
/// the default. A missing eager default target, like an authored one, is
/// left to `md`.
#[test]
fn an_unresolvable_file_default_is_attributed_to_its_declaration() {
    let declaration = "file(required; eager; default(vault:note.md))";
    let text = format!("---\n$schema:\n  title: string\n  doc: {declaration}\n  absent: file(required; eager; default(./gone.md))\ntitle: Plan\n---\n\nbody\n");
    let workspace = LspWorkspace::new();
    let mut session = Session::open(&workspace, &text);
    let diagnostics = session.diagnostics();

    let attributed: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["data"]["schema_default"].is_object()).collect();
    assert_eq!(attributed.len(), 1, "{diagnostics:#?}");
    let (line, start) = position(&text, declaration, 0);
    assert_eq!(attributed[0]["range"]["start"], json!({ "line": line, "character": start }), "{attributed:#?}");
    assert_eq!(
        attributed[0]["data"]["schema_default"],
        json!({ "path": "doc", "schema_location": "/properties/doc/default" }),
        "{attributed:#?}"
    );
    assert!(attributed[0]["message"].as_str().unwrap().contains("schema default"), "{attributed:#?}");
    assert!(
        diagnostics.iter().all(|diagnostic| diagnostic["data"]["instance_path"] != json!("/absent")),
        "a missing eager target is not an editor error: {diagnostics:#?}"
    );
    session.shutdown();
}

/// A default a `Name@file` import supplies is ranged at the importing
/// declaration, and its data names the file that declares the default.
#[test]
fn an_imported_file_default_names_the_type_file_that_declares_it() {
    let workspace = LspWorkspace::new();
    std::fs::write(
        workspace.path().join("types.yaml"),
        "kind: schema\ntypes:\n  Doc: file(required; eager; default(vault:note.md))\n",
    )
    .unwrap();
    let declaration = "Doc@./types.yaml";
    let text = format!("---\n$schema:\n  doc: {declaration}\n---\n\nbody\n");
    let mut session = Session::open(&workspace, &text);
    let diagnostics = session.diagnostics();

    let attributed: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["data"]["schema_default"].is_object()).collect();
    assert_eq!(attributed.len(), 1, "{diagnostics:#?}");
    let (line, start) = position(&text, declaration, 0);
    assert_eq!(attributed[0]["range"]["start"], json!({ "line": line, "character": start }), "{attributed:#?}");
    let named = attributed[0]["data"]["schema_default"]["schema_file"].as_str().expect("the type file is named");
    assert!(std::path::Path::new(named).ends_with("types.yaml"), "{attributed:#?}");
    assert!(attributed[0]["message"].as_str().unwrap().contains("types.yaml"), "{attributed:#?}");
    session.shutdown();
}

/// An existing entry a pattern key selects is completed with its required
/// child's default, so only the control `r` (required, no default) is
/// reported missing.
#[test]
fn a_required_default_beneath_a_pattern_key_is_present() {
    let text = "---\n$schema: ./defaults.yaml\nx-build: {}\n---\n\nRetries {{ x-build.retries }}\n";
    let workspace = LspWorkspace::new();
    std::fs::write(
        workspace.path().join("defaults.yaml"),
        "$schema:\n  \"<starting::x->\": \"{ retries: number(required; default(3)) }\"\n  r: number(required)\n",
    )
    .unwrap();
    let mut session = Session::open(&workspace, text);
    let diagnostics = session.diagnostics();

    let missing: Vec<(&str, &Value)> = diagnostics
        .iter()
        .filter(|diagnostic| diagnostic["code"] == json!(MISSING_REQUIRED))
        .map(|diagnostic| (diagnostic["message"].as_str().unwrap(), &diagnostic["data"]["instance_path"]))
        .collect();
    assert_eq!(missing, [("\"r\" is a required property", &json!(""))], "the default answers `x-build.retries`: {diagnostics:#?}");
    session.shutdown();
}
