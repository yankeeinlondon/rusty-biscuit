//! L1 integration fixtures for the Phase 3 workspace graph.
//!
//! Exercises the public discover → index → graph pipeline over real temp
//! workspaces (graph edge extraction) and the invalidation matrix (edit,
//! delete, rename) through the `WorkspaceIndex` public API. These are pure
//! (in-memory graph, on-disk fixtures) and need no LSP session, so they stay
//! ungated.

use std::path::{Path, PathBuf};

use dmls::config::WorkspaceConfig;
use dmls::graph::{EdgeKind, WorkspaceIndex};
use dmls::workspace::{SilentProgress, index_workspace};

fn write(root: &Path, rel: &str, body: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, body).unwrap();
}

#[test]
fn cross_file_link_edges_resolve_through_discovery() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "index.md", "# Home\n\nSee [setup](guide/setup.md#install).\n");
    write(root, "guide/setup.md", "# Setup\n\n## Install\n\nSteps.\n");

    let index = index_workspace(&[root.to_path_buf()], &WorkspaceConfig::default(), &SilentProgress);
    let snapshot = index.snapshot();
    assert_eq!(snapshot.document_count(), 2);

    let setup = snapshot.document_id(&root.join("guide/setup.md")).unwrap();
    // The `## Install` heading is referenced once (the cross-file fragment).
    let install = snapshot
        .headings(setup)
        .find(|(_, node)| node.as_heading().is_some_and(|h| h.slug == "install"))
        .map(|(id, _)| id)
        .unwrap();
    assert_eq!(snapshot.incoming(install, EdgeKind::References).count(), 1);
}

#[test]
fn wiki_basename_index_flags_ambiguity() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    write(root, "a/note.md", "# A note\n");
    write(root, "b/note.md", "# B note\n");
    write(root, "c/unique.md", "# Unique\n");

    let index = index_workspace(&[root.to_path_buf()], &WorkspaceConfig::default(), &SilentProgress);
    let keys = index.snapshot();
    let keys = keys.key_index();
    assert!(keys.is_ambiguous("note"));
    assert!(!keys.is_ambiguous("unique"));
}

#[test]
fn invalidation_matrix_edit_delete_rename() {
    let mut index = WorkspaceIndex::new();
    let a = PathBuf::from("/w/a.md");
    let b = PathBuf::from("/w/b.md");

    // Seed two documents.
    index.set_document(&a, "# A\n\n[to b](b.md)\n");
    index.set_document(&b, "# B\n");
    let after_seed = index.generation();

    // Edit: a no-op re-set (identical bytes) does not bump the generation.
    assert!(matches!(
        index.set_document(&b, "# B\n"),
        dmls::graph::Invalidation::Unchanged
    ));
    assert_eq!(index.generation(), after_seed);

    // Edit with new content re-indexes.
    index.set_document(&b, "# B renamed heading\n");
    assert!(index.generation() > after_seed);

    // Delete removes the document from the snapshot.
    index.remove_document(&b);
    assert!(index.snapshot().document_id(&b).is_none());

    // Rename = delete old + add new; the graph reflects only the new path.
    index.remove_document(&a);
    index.set_document(&PathBuf::from("/w/a-renamed.md"), "# A\n");
    let snapshot = index.snapshot();
    assert!(snapshot.document_id(&a).is_none());
    assert!(snapshot.document_id(Path::new("/w/a-renamed.md")).is_some());
}

#[test]
fn generated_corpus_indexes_without_panic() {
    // The pathological tier (duplicate headings, Unicode, broken links, deep
    // nesting) must index cleanly — it is the adversarial fixture.
    let temp = tempfile::tempdir().unwrap();
    // A small slice: generate tiny-100 and index it end to end.
    dmls::generate_corpus(dmls::CorpusTier::Tiny100, temp.path()).unwrap();
    let index = index_workspace(
        &[temp.path().to_path_buf()],
        &WorkspaceConfig::default(),
        &SilentProgress,
    );
    assert_eq!(index.len(), 100);
    assert!(index.snapshot().edge_count() > 0);
}
