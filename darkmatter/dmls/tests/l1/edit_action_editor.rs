//! DMLS over the lifecycle `edit` action, in an `md` document (Darkmatter's
//! baseline) and in a Claudine document (typed and analyzed under Claudine's
//! registration, which the [`Fixture`] repository registers):
//! completion offers `edit`, hover shows its schema description, and a
//! malformed operand is diagnosed where it is written, in a branch the run
//! would not take as well as one it would.
//!
//! No request launches an editor or writes a file: every session runs with
//! `EDITOR` and `VISUAL` naming a script that would record its launch, and the
//! repository tree is compared before and after.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};

use serde_json::{Value, json};

use crate::claudine_schema_discovery::{Fixture, initialize_params, open, uri_of, write};
use crate::common::{LspFixture, LspWorkspace};

/// Two malformed operands in `initialize`: a numeric `gui` in the branch the
/// run would not take, and a blank target in the one it would.
const MALFORMED: &str = "---\n\
initialize:\n  \
  - when: \"false\"\n    \
    then:\n      \
      - edit:\n          \
          file: notes.md\n          \
          gui: 5\n    \
    else:\n      \
      - edit: \"   \"\n  \
  - skip\n\
---\nBody\n";

/// The motivating workflow, unedited.
const MOTIVATING: &str = "---\n\
fqn: \"{{ ctx.cwd }}/specs/2026-11-14-example/spec.md\"\n\
initialize:\n  \
  - when: file_exists(fqn)\n    \
    then:\n      \
      - warn: \"The spec already exists: {{ fqn }}\"\n      \
      - skip\n    \
    else:\n      \
      - edit:\n          \
          file: \"{{ fqn }}\"\n          \
          create_new: true\n      \
      - info: \"The spec is ready at {{ fqn }}\"\n      \
      - skip\n\
---\nBody\n";

#[derive(Clone, Copy, Debug)]
enum Host {
    /// A repository with no registered host: Darkmatter's baseline types
    /// the document.
    Md,
    /// Claudine's registration, whose `composition.yaml` trigger the
    /// document's selector matches.
    Claudine,
}

/// `text` as `host` opens it: a Claudine document carries `agent`, which
/// selects `composition.yaml` (the shared `initialize` event alone selects no
/// Claudine trigger).
fn host_text(host: Host, text: &str) -> String {
    match host {
        Host::Md => text.to_string(),
        Host::Claudine => {
            let body = text.strip_prefix("---\n").expect("the document opens with frontmatter");
            format!("---\nagent: claude\n{body}")
        }
    }
}

#[derive(Clone, Copy)]
enum Ask {
    /// Every diagnostic published for the document.
    Diagnostics,
    /// Completion labels at the offset.
    Complete(usize),
    /// Hover Markdown at the offset.
    Hover(usize),
}

/// A script that appends to `marker` when launched, named by `EDITOR` and
/// `VISUAL` in the server's environment.
fn recorder(dir: &Path, marker: &Path) -> PathBuf {
    #[cfg(windows)]
    {
        let script = dir.join("editor.cmd");
        std::fs::write(&script, format!("@echo launched>> \"{}\"\r\n", marker.display())).unwrap();
        script
    }
    #[cfg(not(windows))]
    {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("editor");
        std::fs::write(&script, format!("#!/bin/sh\necho launched >> '{}'\n", marker.display())).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }
}

/// Every file under `root` with its bytes, skipping the fixture home.
fn tree(root: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    let mut found = BTreeMap::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(dir) = pending.pop() {
        for entry in std::fs::read_dir(&dir).unwrap() {
            let path = entry.unwrap().path();
            if path.file_name().is_some_and(|name| name == "home") {
                continue;
            }
            if path.is_dir() {
                pending.push(path);
            } else {
                found.insert(path.clone(), std::fs::read(&path).unwrap());
            }
        }
    }
    found
}

/// Opens `text` as one document under `host`, answers each of `asks` in
/// order, and asserts nothing launched the recorder or changed a file.
fn answers(host: Host, text: &str, asks: &[Ask]) -> Vec<Value> {
    let workspace = LspWorkspace::new();
    let outside = tempfile::tempdir().unwrap();
    let marker = outside.path().join("launched");
    let editor = recorder(outside.path(), &marker).to_string_lossy().into_owned();
    let env = HashMap::from([("EDITOR".to_string(), editor.clone()), ("VISUAL".to_string(), editor)]);

    let (repo, document, snapshot) = match host {
        Host::Claudine => {
            let fixture = Fixture::new(workspace.path());
            (fixture.repo(), fixture.document("doc.md", text), fixture.snapshot().with_env(env))
        }
        Host::Md => {
            let root = workspace.path().to_path_buf();
            std::fs::create_dir_all(root.join(".git/objects")).unwrap();
            std::fs::create_dir_all(root.join(".git/refs/heads")).unwrap();
            write(&root.join(".git/HEAD"), "ref: refs/heads/main\n");
            let document = root.join("doc.md");
            write(&document, text);
            (root, document, LspFixture::fixture_snapshot(&workspace).with_env(env))
        }
    };
    let before = tree(&repo);

    let mut session = LspFixture::start_with_snapshot(&workspace, snapshot);
    session.initialize(initialize_params(&repo));
    let uri = uri_of(&document);
    open(&session, &uri, text);
    let diagnostics = session.wait_for_diagnostics(&uri);
    let found = asks
        .iter()
        .map(|ask| {
            let (method, offset) = match *ask {
                Ask::Diagnostics => return Value::Array(diagnostics.clone()),
                Ask::Complete(offset) => ("textDocument/completion", offset),
                Ask::Hover(offset) => ("textDocument/hover", offset),
            };
            let result = session
                .request(method, json!({ "textDocument": { "uri": uri }, "position": position(text, offset) }))
                .result
                .unwrap_or(Value::Null);
            match ask {
                Ask::Complete(_) => {
                    let items = result.as_array().or_else(|| result["items"].as_array()).cloned().unwrap_or_default();
                    Value::Array(items.iter().map(|item| item["label"].clone()).collect())
                }
                _ => result["contents"]["value"].clone(),
            }
        })
        .collect();
    session.shutdown();

    assert!(!marker.exists(), "{host:?}: an editor was launched");
    assert_eq!(tree(&repo), before, "{host:?}: a file was written");
    found
}

/// The LSP position (ASCII fixtures, so bytes are UTF-16 units) of `offset`.
fn position(text: &str, offset: usize) -> Value {
    let before = &text[..offset];
    json!({
        "line": before.matches('\n').count(),
        "character": offset - before.rfind('\n').map_or(0, |index| index + 1),
    })
}

/// The byte offset where `needle` starts, searched after `after`.
fn at(text: &str, after: &str, needle: &str) -> usize {
    let start = text.find(after).unwrap_or_else(|| panic!("`{after}` in {text:?}")) + after.len();
    start + text[start..].find(needle).unwrap_or_else(|| panic!("`{needle}` after `{after}` in {text:?}"))
}

fn schema_diagnostics(diagnostics: &Value) -> Vec<&Value> {
    diagnostics
        .as_array()
        .expect("a diagnostics list")
        .iter()
        .filter(|diagnostic| diagnostic["code"].as_str().is_some_and(|code| code.starts_with("dm.schema.")))
        .collect()
}

/// Asserts the malformed fixture's two operands are each diagnosed, by
/// schema, at the value written there.
fn assert_malformed_operands_diagnosed(host: Host) {
    let text = host_text(host, MALFORMED);
    let text = text.as_str();
    let found = answers(host, text, &[Ask::Diagnostics]);
    let schema = schema_diagnostics(&found[0]);
    let paths: Vec<&str> = schema.iter().filter_map(|diagnostic| diagnostic["data"]["instance_path"].as_str()).collect();
    for (pointer, offset, needle) in [
        ("/initialize/0/then/0/edit/gui", at(text, "gui: ", "5"), "must be a boolean"),
        ("/initialize/0/else/0/edit", at(text, "else:", "\"   \""), "must not be blank"),
    ] {
        let diagnostic = schema
            .iter()
            .find(|diagnostic| diagnostic["data"]["instance_path"] == json!(pointer))
            .unwrap_or_else(|| panic!("{host:?}: no schema diagnostic at `{pointer}` among {paths:?}: {schema:#?}"));
        assert_eq!(diagnostic["range"]["start"], position(text, offset), "{host:?} {pointer}: {diagnostic:#?}");
        assert!(
            diagnostic["message"].as_str().is_some_and(|message| message.contains(needle)),
            "{host:?} {pointer}: {diagnostic:#?}"
        );
    }
    // Beside each operand's own problem, the action may carry the union's
    // summary; nothing outside the two actions is diagnosed.
    assert!(
        paths.iter().all(|path| ["/initialize/0/then/0/edit", "/initialize/0/else/0/edit"]
            .iter()
            .any(|action| path.starts_with(action))),
        "{host:?}: {paths:?}"
    );
}

fn assert_motivating_workflow_is_clean(host: Host) {
    let found = answers(host, &host_text(host, MOTIVATING), &[Ask::Diagnostics]);
    let schema = schema_diagnostics(&found[0]);
    assert!(schema.is_empty(), "{host:?}: {schema:#?}");
}

fn labels(found: &Value) -> Vec<&str> {
    found.as_array().expect("labels").iter().filter_map(Value::as_str).collect()
}

#[test]
fn md_malformed_operands_are_diagnosed_where_written_in_both_branches() {
    assert_malformed_operands_diagnosed(Host::Md);
}

#[test]
fn claudine_malformed_operands_are_diagnosed_where_written_in_both_branches() {
    assert_malformed_operands_diagnosed(Host::Claudine);
}

#[test]
fn md_motivating_workflow_raises_no_schema_diagnostic() {
    assert_motivating_workflow_is_clean(Host::Md);
}

#[test]
fn claudine_motivating_workflow_raises_no_schema_diagnostic() {
    assert_motivating_workflow_is_clean(Host::Claudine);
}

/// In an `md` document, `initialize` and `composed` both offer `edit`, and
/// hover on it shows the schema's description.
#[test]
fn md_completion_offers_edit_in_initialize_and_composed_and_hover_describes_it() {
    let text = "---\ninitialize:\n  - \n  - edit: notes.md\ncomposed:\n  - info: hi\n  - \n---\nBody\n";
    let found = answers(
        Host::Md,
        text,
        &[
            Ask::Complete(at(text, "initialize:\n", "- ") + 2),
            Ask::Complete(at(text, "info: hi\n", "- ") + 2),
            Ask::Hover(at(text, "  - ", "edit")),
        ],
    );
    assert!(labels(&found[0]).contains(&"edit"), "initialize: {:?}", found[0]);
    assert!(labels(&found[1]).contains(&"edit"), "composed: {:?}", found[1]);
    let hover = found[2].as_str().unwrap_or_default();
    assert!(hover.contains("open this file in the editor"), "{hover:?}");
}

/// In a Claudine document, `initialize` and `start` both offer `edit`, and
/// hover on it shows the schema's description.
#[test]
fn claudine_completion_offers_edit_in_initialize_and_start_and_hover_describes_it() {
    let text = host_text(Host::Claudine, "---\ninitialize:\n  - \n  - edit: notes.md\nstart:\n  - info: hi\n  - \n---\nBody\n");
    let text = text.as_str();
    let found = answers(
        Host::Claudine,
        text,
        &[
            Ask::Complete(at(text, "initialize:\n", "- ") + 2),
            Ask::Complete(at(text, "info: hi\n", "- ") + 2),
            Ask::Hover(at(text, "  - ", "edit")),
        ],
    );
    assert!(labels(&found[0]).contains(&"edit"), "initialize: {:?}", found[0]);
    assert!(labels(&found[1]).contains(&"edit"), "start: {:?}", found[1]);
    let hover = found[2].as_str().unwrap_or_default();
    assert!(hover.contains("open this file in the editor"), "{hover:?}");
}
