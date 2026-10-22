//! Host-global hover and completion come from the host catalog in the
//! document's schema roots: Claudine's `claudine/schemas/globals.yaml`
//! declares `seq`, `loop`, and `group`, and DMLS reads their members and
//! descriptions from that file, never from a list of its own.

use serde_json::Value;

use crate::common::{LspFixture, LspWorkspace};
use crate::union_item_editor::{initialize_params, offset_of, open_text, request};

/// A repository whose tree-root `schemas/` holds `catalog`, if any.
fn session<'w>(workspace: &'w LspWorkspace, catalog: Option<&str>) -> LspFixture<'w> {
    let root = workspace.path();
    for dir in [".git/objects", ".git/refs/heads", "schemas"] {
        std::fs::create_dir_all(root.join(dir)).unwrap();
    }
    std::fs::write(root.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
    if let Some(catalog) = catalog {
        std::fs::write(root.join("schemas/globals.yaml"), catalog).unwrap();
    }
    let mut fixture = LspFixture::start(workspace);
    fixture.initialize(initialize_params(root));
    fixture
}

fn hover_text(result: &Value) -> String {
    result["contents"]["value"].as_str().unwrap_or_default().to_string()
}

fn labels(result: &Value) -> Vec<String> {
    let items = result.get("items").unwrap_or(result);
    items
        .as_array()
        .map(|items| items.iter().filter_map(|item| item["label"].as_str().map(str::to_string)).collect())
        .unwrap_or_default()
}

const DOCUMENT: &str = "---\ntitle: steps\n---\n\n{{ seq.index }} and {{ group.failed }} and {{ loop }}\n";

#[test]
fn host_global_hover_comes_from_the_host_catalog_file() {
    let shipped = include_str!("../../../../claudine/schemas/globals.yaml");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, Some(shipped));
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", DOCUMENT);

    let index = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(DOCUMENT, "index", false)));
    assert!(index.contains("**`seq.index`** — `number(integer;min(1))` — The step's one-based position."), "{index}");
    let failed = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(DOCUMENT, "failed", false)));
    assert!(failed.contains("**`group.failed`** — `string[]`"), "{failed}");
    let looped = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(DOCUMENT, "{{ loop", true) - 1));
    assert!(looped.contains("**`loop`** — host global") && looped.contains("`last_exit_code`"), "{looped}");
    fixture.shutdown();
}

/// Editing the catalog's description changes the hover: nothing else
/// supplies it.
#[test]
fn an_edited_catalog_changes_the_hover_and_no_catalog_gives_none() {
    let shipped = include_str!("../../../../claudine/schemas/globals.yaml");
    let edited = shipped.replace("The step's one-based position.", "Edited in the catalog.");
    assert_ne!(edited, shipped);
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, Some(&edited));
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", DOCUMENT);
    let index = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(DOCUMENT, "index", false)));
    assert!(index.contains("Edited in the catalog."), "{index}");
    fixture.shutdown();

    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, None);
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", DOCUMENT);
    let index = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(DOCUMENT, "index", false)));
    assert!(!index.contains("seq.index`** —"), "{index}");
    fixture.shutdown();
}

#[test]
fn host_global_completion_offers_the_catalogs_roots_and_members() {
    let shipped = include_str!("../../../../claudine/schemas/globals.yaml");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, Some(shipped));
    let text = "---\ntitle: steps\n---\n\n{{ seq.in }} {{ gro }}\n";
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", text);
    let members = labels(&request(&mut fixture, "textDocument/completion", &document, offset_of(text, "seq.in", true)));
    assert!(members.contains(&"seq.index".to_string()), "{members:?}");
    assert!(!members.contains(&"seq.name".to_string()), "{members:?}");
    let roots = labels(&request(&mut fixture, "textDocument/completion", &document, offset_of(text, "gro", true)));
    assert!(roots.contains(&"group".to_string()), "{roots:?}");
    fixture.shutdown();
}

/// The catalog's file is not reported as a schema dependency of a document.
#[test]
fn the_host_catalog_is_not_a_trigger_for_a_document() {
    let shipped = include_str!("../../../../claudine/schemas/globals.yaml");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, Some(shipped));
    let text = "---\nseq: authored\n---\n\nBody\n";
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "doc.md", text);
    assert!(diagnostics.is_empty(), "the catalog applies no schema to `seq`: {diagnostics:?}");
    fixture.shutdown();
}
