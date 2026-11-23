//! Host-global hover and completion come from the namespace files of the
//! registered host that identifies the document ([`crate::claudine_host`]:
//! `seq`, `loop`, and `group`): DMLS reads their members and descriptions
//! from those files, never from a list of its own. A catalog in a schema root
//! supplies no host globals.

use serde_json::Value;

use crate::claudine_host::{bundle, bundle_with_seq, install, marked, namespace};
use crate::common::{LspFixture, LspWorkspace};
use crate::host_fixtures::Bundle;
use crate::registered_host_views::{ROOT_CATALOG, repository, root_catalog};
use crate::union_item_editor::{initialize_params, offset_of, open_text, request};

/// A repository whose home registers `bundle`, if any.
fn session<'w>(workspace: &'w LspWorkspace, bundle: Option<&Bundle>) -> LspFixture<'w> {
    let root = workspace.path();
    match bundle {
        Some(bundle) => install(root, bundle),
        None => repository(root, &[]),
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

fn document() -> String {
    marked("---\ntitle: steps\n---\n\n{{ seq.index }} and {{ group.failed }} and {{ loop }}\n")
}

#[test]
fn host_global_hover_comes_from_the_registered_namespace_files() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let text = document();
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, Some(&bundle()));
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", &text);

    let index = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(&text, "index", false)));
    assert!(index.contains("**`seq.index`** — `number(integer;min(1))` — The step's one-based position."), "{index}");
    let failed = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(&text, "failed", false)));
    assert!(failed.contains("**`group.failed`** — `string[]`"), "{failed}");
    let looped = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(&text, "{{ loop", true) - 1));
    assert!(looped.contains("**`loop`** — host global") && looped.contains("`last_exit_code`"), "{looped}");
    fixture.shutdown();
}

/// Republishing the `seq` namespace with an edited description changes the
/// hover: nothing else supplies it. Without the registration, a root-local
/// catalog declaring `seq` gives none.
#[test]
fn an_edited_namespace_changes_the_hover_and_a_root_catalog_gives_none() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let text = document();
    let seq = namespace("seq");
    let edited = seq.replace("The step's one-based position.", "Edited in the namespace.");
    assert_ne!(edited, seq);
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, Some(&bundle_with_seq(&edited)));
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", &text);
    let index = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(&text, "index", false)));
    assert!(index.contains("Edited in the namespace."), "{index}");
    fixture.shutdown();

    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, None);
    crate::registered_host_views::write(
        workspace.path(),
        "schemas/globals.yaml",
        &ROOT_CATALOG.replace("    seq:\n", "    seq:\n        index: \"number -> Edited in the root.\"\n"),
    );
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", &text);
    let index = hover_text(&request(&mut fixture, "textDocument/hover", &document, offset_of(&text, "index", false)));
    assert!(!index.contains("seq.index`** —") && !index.contains("Edited in the root."), "{index}");
    fixture.shutdown();
}

#[test]
fn host_global_completion_offers_the_namespaces_and_members() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, Some(&bundle()));
    let text = marked("---\ntitle: steps\n---\n\n{{ seq.in }} {{ gro }}\n");
    let (document, _) = open_text(&mut fixture, &workspace, "doc.md", &text);
    let members = labels(&request(&mut fixture, "textDocument/completion", &document, offset_of(&text, "seq.in", true)));
    assert!(members.contains(&"seq.index".to_string()), "{members:?}");
    assert!(!members.contains(&"seq.name".to_string()), "{members:?}");
    let roots = labels(&request(&mut fixture, "textDocument/completion", &document, offset_of(&text, "gro", true)));
    assert!(roots.contains(&"group".to_string()), "{roots:?}");
    fixture.shutdown();
}

/// A catalog and its lifecycle artifact in the tree root's `schemas/`, as a
/// host installed them before registration, are neither a trigger nor a
/// host: an authored `seq` is plain data, with nothing reported.
#[test]
fn a_root_local_catalog_is_neither_a_trigger_nor_a_host() {
    let _ = include_str!("../../../lib/tests/host_fixtures/mod.rs");
    let workspace = LspWorkspace::new();
    let mut fixture = session(&workspace, None);
    root_catalog(workspace.path());
    let text = "---\nseq: authored\n---\n\nBody\n";
    let (_, diagnostics) = open_text(&mut fixture, &workspace, "doc.md", text);
    assert!(diagnostics.is_empty(), "the catalog applies no schema to `seq`: {diagnostics:?}");
    fixture.shutdown();
}
