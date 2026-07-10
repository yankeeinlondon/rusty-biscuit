//! Content-hash invalidation and immutable snapshot swap.
//!
//! [`WorkspaceIndex`] owns the per-document parse products (the source of
//! truth) and the current immutable [`WorkspaceGraph`] snapshot. A change
//! re-parses one document, compares its xxHash against the stored one, and
//! *only* rebuilds a fresh snapshot when the content actually changed —
//! keystrokes that don't alter bytes (or a save with no edits) are free. Each
//! rebuilt snapshot carries an incremented generation so superseded async work
//! (diagnostics, indexing) can be discarded (AD-3).

use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::arena::WorkspaceGraph;
use super::edge::EdgeKind;
use super::node::NodeId;
use super::substrate::{DocumentIndex, index_document};

/// The result of applying a document change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invalidation {
    /// The content hash was unchanged; no snapshot was rebuilt.
    Unchanged,
    /// The document was re-indexed and a new snapshot generation published.
    Reindexed {
        /// Other documents whose analysis depends on the changed one
        /// (reachable over `transcludes`/`uses_file` edges — e.g. a document
        /// that transcludes it or references it through a frontmatter
        /// `file(...)` value). Their diagnostics should be refreshed.
        dependents: Vec<PathBuf>,
    },
}

/// The mutable workspace index: parse products plus the live snapshot.
#[derive(Debug)]
pub struct WorkspaceIndex {
    documents: BTreeMap<PathBuf, DocumentIndex>,
    snapshot: Arc<WorkspaceGraph>,
    /// Wiki roots each document's canonical logical path is resolved against
    /// (R-8); empty until the server sets them from the initialize handshake.
    wiki_roots: Vec<PathBuf>,
    generation: u64,
}

impl Default for WorkspaceIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceIndex {
    /// An empty index (generation 0).
    pub fn new() -> Self {
        let documents = BTreeMap::new();
        let snapshot = Arc::new(WorkspaceGraph::build(&documents, 0));
        Self {
            documents,
            snapshot,
            wiki_roots: Vec::new(),
            generation: 0,
        }
    }

    /// Sets the wiki roots and rebuilds so already-indexed documents pick up
    /// their canonical logical paths.
    pub fn set_wiki_roots(&mut self, wiki_roots: Vec<PathBuf>) {
        self.wiki_roots = wiki_roots;
        self.rebuild();
    }

    /// Builds an index from a batch of already-parsed documents (the startup
    /// path: workers parse in parallel, the result assembles once).
    pub fn from_indices(indices: BTreeMap<PathBuf, DocumentIndex>) -> Self {
        let mut index = Self {
            documents: indices,
            snapshot: Arc::new(WorkspaceGraph::build(&BTreeMap::new(), 0)),
            wiki_roots: Vec::new(),
            generation: 1,
        };
        index.rebuild();
        index
    }

    /// Adopts disk-derived documents that are not already present, rebuilding
    /// the snapshot once.
    ///
    /// Startup indexing runs concurrently with the first `didOpen`s: any
    /// document already indexed (from an open buffer, the authority over disk)
    /// is kept; only missing paths are adopted. A no-op merge does not bump the
    /// generation.
    pub fn adopt_missing(&mut self, indices: BTreeMap<PathBuf, DocumentIndex>) {
        use std::collections::btree_map::Entry;
        let mut adopted = 0;
        for (path, index) in indices {
            if let Entry::Vacant(slot) = self.documents.entry(path) {
                slot.insert(index);
                adopted += 1;
            }
        }
        if adopted > 0 {
            self.rebuild();
        }
    }

    /// The current immutable snapshot (cheap `Arc` clone for readers).
    pub fn snapshot(&self) -> Arc<WorkspaceGraph> {
        Arc::clone(&self.snapshot)
    }

    /// The current snapshot generation.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Re-indexes a document from fresh `source`, rebuilding the snapshot only
    /// when the content hash changed.
    pub fn set_document(&mut self, path: &Path, source: &str) -> Invalidation {
        let fresh = index_document(path, source);
        if let Some(existing) = self.documents.get(path)
            && existing.content_hash == fresh.content_hash
        {
            return Invalidation::Unchanged;
        }
        let dependents = self.dependents_of(path);
        self.documents.insert(path.to_path_buf(), fresh);
        self.rebuild();
        Invalidation::Reindexed { dependents }
    }

    /// Reconciles the index against a fresh disk scan for the server-side
    /// rescan fallback (clients without reliable file watching).
    ///
    /// Adopts created and content-changed files (xxHash compare), drops files
    /// that vanished from disk, and never touches an open buffer — the client
    /// buffer stays authoritative over disk (`open_paths` names those buffers,
    /// which may not exist on disk yet). The snapshot is rebuilt once if
    /// anything changed.
    ///
    /// ## Returns
    ///
    /// `true` when the snapshot was rebuilt (something changed), so the caller
    /// knows to refresh diagnostics.
    pub fn reconcile_disk(
        &mut self,
        disk: BTreeMap<PathBuf, DocumentIndex>,
        open_paths: &HashSet<PathBuf>,
    ) -> bool {
        let mut changed = false;
        let mut on_disk: HashSet<PathBuf> = HashSet::with_capacity(disk.len());
        for (path, fresh) in disk {
            on_disk.insert(path.clone());
            if open_paths.contains(&path) {
                continue;
            }
            match self.documents.get(&path) {
                Some(existing) if existing.content_hash == fresh.content_hash => {}
                _ => {
                    self.documents.insert(path, fresh);
                    changed = true;
                }
            }
        }
        let vanished: Vec<PathBuf> = self
            .documents
            .keys()
            .filter(|path| !on_disk.contains(*path) && !open_paths.contains(*path))
            .cloned()
            .collect();
        for path in vanished {
            self.documents.remove(&path);
            changed = true;
        }
        if changed {
            self.rebuild();
        }
        changed
    }

    /// Drops a document (deletion), rebuilding the snapshot if it was present.
    pub fn remove_document(&mut self, path: &Path) -> Invalidation {
        if self.documents.remove(path).is_none() {
            return Invalidation::Unchanged;
        }
        let dependents = self.dependents_of(path);
        self.rebuild();
        Invalidation::Reindexed { dependents }
    }

    /// Whether a document is indexed.
    pub fn contains(&self, path: &Path) -> bool {
        self.documents.contains_key(path)
    }

    /// Number of indexed documents.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether no documents are indexed.
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }

    /// Documents that depend on `path` over compositional edges
    /// (`transcludes`/`uses_file`) in the *current* snapshot — computed before
    /// the swap so the caller knows whose diagnostics to refresh.
    fn dependents_of(&self, path: &Path) -> Vec<PathBuf> {
        let Some(doc_id) = self.snapshot.document_id(path) else {
            return Vec::new();
        };
        let Some(record) = self.snapshot.document(doc_id) else {
            return Vec::new();
        };
        let mut dependents: Vec<PathBuf> = Vec::new();
        for node in self.dependent_nodes(record.root) {
            if let Some(source) = self.snapshot.node(node)
                && let Some(source_doc) = self.snapshot.document(source.document)
                && source_doc.path != path
                && !dependents.contains(&source_doc.path)
            {
                dependents.push(source_doc.path.clone());
            }
        }
        dependents
    }

    /// Source nodes reaching `target` over compositional edge kinds.
    fn dependent_nodes(&self, target: NodeId) -> Vec<NodeId> {
        [EdgeKind::Transcludes, EdgeKind::UsesFile]
            .into_iter()
            .flat_map(|kind| {
                self.snapshot
                    .incoming(target, kind)
                    .map(|edge| edge.source)
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    fn rebuild(&mut self) {
        self.generation += 1;
        self.snapshot = Arc::new(WorkspaceGraph::build_with_roots(
            &self.documents,
            self.generation,
            &self.wiki_roots,
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_unchanged_content_skips_rebuild() {
        let mut index = WorkspaceIndex::new();
        let path = PathBuf::from("/w/a.md");
        assert!(matches!(
            index.set_document(&path, "# A\n"),
            Invalidation::Reindexed { .. }
        ));
        let generation = index.generation();
        // Same bytes → no rebuild, generation frozen.
        assert_eq!(index.set_document(&path, "# A\n"), Invalidation::Unchanged);
        assert_eq!(index.generation(), generation);
    }

    #[test]
    fn test_edit_bumps_generation_and_reindexes() {
        let mut index = WorkspaceIndex::new();
        let path = PathBuf::from("/w/a.md");
        index.set_document(&path, "# A\n");
        let before = index.generation();
        assert!(matches!(
            index.set_document(&path, "# A changed\n"),
            Invalidation::Reindexed { .. }
        ));
        assert_eq!(index.generation(), before + 1);
        let snapshot = index.snapshot();
        let doc = snapshot.document_id(&path).unwrap();
        assert_eq!(
            snapshot.headings(doc).next().unwrap().1.as_heading().unwrap().title,
            "A changed"
        );
    }

    #[test]
    fn test_delete_removes_document() {
        let mut index = WorkspaceIndex::new();
        let path = PathBuf::from("/w/a.md");
        index.set_document(&path, "# A\n");
        assert!(index.contains(&path));
        assert!(matches!(
            index.remove_document(&path),
            Invalidation::Reindexed { .. }
        ));
        assert!(!index.contains(&path));
        assert!(index.snapshot().document_id(&path).is_none());
        // Deleting again is a no-op.
        assert_eq!(index.remove_document(&path), Invalidation::Unchanged);
    }

    #[test]
    fn test_rename_matrix_as_delete_then_add() {
        let mut index = WorkspaceIndex::new();
        let old = PathBuf::from("/w/old.md");
        let new = PathBuf::from("/w/new.md");
        index.set_document(&old, "# Title\n");
        index.remove_document(&old);
        index.set_document(&new, "# Title\n");
        let snapshot = index.snapshot();
        assert!(snapshot.document_id(&old).is_none());
        assert!(snapshot.document_id(&new).is_some());
    }

    #[test]
    fn test_from_indices_batch_startup() {
        let mut indices = BTreeMap::new();
        for name in ["a", "b", "c"] {
            let path = PathBuf::from(format!("/w/{name}.md"));
            indices.insert(path.clone(), index_document(&path, "# H\n"));
        }
        let index = WorkspaceIndex::from_indices(indices);
        assert_eq!(index.len(), 3);
        assert_eq!(index.snapshot().document_count(), 3);
    }

    fn disk_scan(entries: &[(&str, &str)]) -> BTreeMap<PathBuf, DocumentIndex> {
        entries
            .iter()
            .map(|(path, source)| {
                let path = PathBuf::from(path);
                let index = index_document(&path, source);
                (path, index)
            })
            .collect()
    }

    #[test]
    fn test_reconcile_disk_adopts_created_and_changed_drops_deleted() {
        let mut index = WorkspaceIndex::new();
        let a = PathBuf::from("/w/a.md");
        index.set_document(&a, "# A\n");
        let before = index.generation();

        // b.md is new, a.md changed; both are adopted and the snapshot rebuilds.
        let changed =
            index.reconcile_disk(disk_scan(&[("/w/a.md", "# A2\n"), ("/w/b.md", "# B\n")]), &HashSet::new());
        assert!(changed);
        assert!(index.generation() > before);
        assert!(index.contains(&PathBuf::from("/w/b.md")));
        let snapshot = index.snapshot();
        let a_id = snapshot.document_id(&a).unwrap();
        assert_eq!(
            snapshot.headings(a_id).next().unwrap().1.as_heading().unwrap().title,
            "A2"
        );

        // A scan missing a.md (and no open buffers) drops it.
        let changed = index.reconcile_disk(disk_scan(&[("/w/b.md", "# B\n")]), &HashSet::new());
        assert!(changed);
        assert!(!index.contains(&a));
        assert!(index.contains(&PathBuf::from("/w/b.md")));
    }

    #[test]
    fn test_reconcile_disk_no_change_is_free() {
        let mut index = WorkspaceIndex::new();
        index.set_document(&PathBuf::from("/w/a.md"), "# A\n");
        let generation = index.generation();
        // The same bytes on disk → no rebuild, generation frozen.
        let changed = index.reconcile_disk(disk_scan(&[("/w/a.md", "# A\n")]), &HashSet::new());
        assert!(!changed);
        assert_eq!(index.generation(), generation);
    }

    #[test]
    fn test_reconcile_disk_never_touches_open_buffers() {
        let mut index = WorkspaceIndex::new();
        let open = PathBuf::from("/w/open.md");
        // The open buffer holds unsaved edits; disk still has the old bytes.
        index.set_document(&open, "# Buffer edit\n");
        let mut open_paths = HashSet::new();
        open_paths.insert(open.clone());

        // A scan whose disk copy differs and that omits the open buffer entirely
        // must neither overwrite nor drop it.
        let changed = index.reconcile_disk(disk_scan(&[("/w/open.md", "# Stale disk\n")]), &open_paths);
        assert!(!changed);
        let snapshot = index.snapshot();
        let id = snapshot.document_id(&open).unwrap();
        assert_eq!(
            snapshot.headings(id).next().unwrap().1.as_heading().unwrap().title,
            "Buffer edit"
        );

        // An empty scan (buffer never saved to disk) still keeps the buffer.
        let changed = index.reconcile_disk(BTreeMap::new(), &open_paths);
        assert!(!changed);
        assert!(index.contains(&open));
    }

    #[test]
    fn test_set_wiki_roots_changes_wiki_resolution() {
        use crate::graph::WikiResolution;

        let mut index = WorkspaceIndex::new();
        let target = PathBuf::from("/w/notes/Target.md");
        let source = PathBuf::from("/w/notes/Source.md");
        index.set_document(&target, "# Target\n");
        index.set_document(&source, "[[/notes/Target]]\n");

        let resolution = |index: &WorkspaceIndex| {
            let snapshot = index.snapshot();
            let doc = snapshot.document_id(&source).unwrap();
            let (_, node) = snapshot.wiki_links(doc).next().expect("a wiki link");
            node.as_wiki_link().unwrap().resolution.clone()
        };

        // Workspace-root wiki roots give Target the canonical path `notes/Target`,
        // so the two-segment root-relative link resolves.
        index.set_wiki_roots(vec![PathBuf::from("/w")]);
        assert!(matches!(resolution(&index), WikiResolution::Resolved(_)));

        // Narrowing the wiki root to `/w/notes` shortens Target's canonical path
        // to `Target`, so the same root-relative link no longer matches — the
        // reload-triggered rebuild flips resolution without any document edit.
        index.set_wiki_roots(vec![PathBuf::from("/w/notes")]);
        assert_eq!(resolution(&index), WikiResolution::Unresolved);
    }

    #[test]
    fn test_plain_link_change_reports_no_dependents() {
        // A plain Markdown link is a `references` edge, not a compositional
        // (`transcludes`/`uses_file`) edge, so changing its target reports no
        // dependents to re-diagnose.
        let mut index = WorkspaceIndex::new();
        let a = PathBuf::from("/w/a.md");
        let b = PathBuf::from("/w/b.md");
        index.set_document(&a, "[to b](b.md)\n");
        index.set_document(&b, "# B\n");
        let outcome = index.set_document(&b, "# B changed\n");
        assert_eq!(outcome, Invalidation::Reindexed { dependents: vec![] });
    }

    #[test]
    fn test_uses_file_change_reports_dependents() {
        // a.md references b.md through a frontmatter `file(...)` value (declared
        // by a's inline `$schema`), so editing b.md fans out to a.md over the
        // `uses_file` edge.
        let mut index = WorkspaceIndex::new();
        let a = PathBuf::from("/w/a.md");
        let b = PathBuf::from("/w/b.md");
        index.set_document(&a, "---\n$schema:\n  include: \"file\"\ninclude: ./b.md\n---\n\n# A\n");
        index.set_document(&b, "# B\n");
        let outcome = index.set_document(&b, "# B changed\n");
        assert_eq!(outcome, Invalidation::Reindexed { dependents: vec![a] });
    }

    #[test]
    fn test_transclude_change_reports_dependents() {
        // a.md transcludes b.md, so editing b.md fans out to a.md.
        let mut index = WorkspaceIndex::new();
        let a = PathBuf::from("/w/a.md");
        let b = PathBuf::from("/w/b.md");
        index.set_document(&a, "::file ./b.md\n");
        index.set_document(&b, "# B\n");
        let outcome = index.set_document(&b, "# B changed\n");
        assert_eq!(outcome, Invalidation::Reindexed { dependents: vec![a] });
    }
}
