//! Content-hash invalidation and immutable snapshot swap.
//!
//! [`WorkspaceIndex`] owns the workspace file list, the per-document parse
//! products (the source of truth), and the current immutable
//! [`WorkspaceGraph`] snapshot. A change re-parses one document, compares its
//! xxHash against the stored one, and *only* rebuilds a fresh snapshot when
//! the content actually changed — keystrokes that don't alter bytes (or a
//! save with no edits) are free. Each rebuilt snapshot carries an incremented
//! generation so superseded async work (diagnostics, indexing) can be
//! discarded (AD-3).
//!
//! Parsing is lazy. Every listed Markdown file is a graph document, but a
//! closed one is parsed only when a feature needs its contents
//! ([`WorkspaceIndex::prepare_targets`], [`WorkspaceIndex::ensure_parsed`]).
//! A disk change to a parsed closed document marks it stale: the prior parse
//! and hash are kept, and the next need re-reads the file and reuses the
//! parse when the hash is unchanged. Open buffers override disk until they
//! close. Workspace-wide answers (references, renames, workspace symbols)
//! never read this index's closed documents; they come from the index store
//! ([`crate::index_store::WorkspaceFacts`]).
//!
//! A referenced target need not be listed: a Markdown file that an open
//! document's link or transclusion resolves to (excluded from discovery, or
//! outside every workspace folder) is parsed on the same terms and kept as a
//! [dependency](super::DocumentRecord::dependency). Dependencies are never
//! part of the file list.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::arena::{DocumentContexts, Listing, WorkspaceGraph, is_markdown_path};
use crate::wiki::WikiUniverse;
use super::edge::{EdgeKind, EdgeTarget};
use super::node::{LinkTarget, NodeId, WikiResolution};
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
    /// Parsed documents: open buffers and disk parses.
    documents: BTreeMap<PathBuf, DocumentIndex>,
    /// The listed Markdown files; each is a graph document, parsed or not.
    listed: BTreeSet<PathBuf>,
    /// The listed YAML files (never graph documents).
    yaml: BTreeSet<PathBuf>,
    listing: Listing,
    /// Documents whose contents come from an open buffer.
    open: HashSet<PathBuf>,
    /// Disk parses a change has made stale; re-read on the next need.
    stale: HashSet<PathBuf>,
    /// Paths read from disk while neither listed nor open: referenced
    /// targets resolution found outside the file list. Each is a
    /// [dependency](Self::is_dependency) while it stays parsed, unlisted,
    /// and closed.
    unlisted_reads: BTreeSet<PathBuf>,
    snapshot: Arc<WorkspaceGraph>,
    /// Wiki roots each document's canonical logical path is resolved against
    /// (R-8); empty until the server sets them from the initialize handshake.
    wiki_roots: Vec<PathBuf>,
    /// Each document's file-resolution context, which every rebuild resolves
    /// path references through.
    contexts: Arc<dyn DocumentContexts>,
    generation: u64,
    /// `Some` between [`begin_batch`](Self::begin_batch) and
    /// [`end_batch`](Self::end_batch): whether a rebuild was asked for.
    batch: Option<bool>,
}

impl WorkspaceIndex {
    /// An empty index (generation 0) resolving references through
    /// `contexts`.
    pub fn new(contexts: Arc<dyn DocumentContexts>) -> Self {
        let documents = BTreeMap::new();
        let snapshot = Arc::new(WorkspaceGraph::build_listed(
            &documents,
            &BTreeSet::new(),
            &BTreeSet::new(),
            Listing::Pending,
            0,
            WikiUniverse::default(),
            contexts.as_ref(),
        ));
        Self {
            documents,
            listed: BTreeSet::new(),
            yaml: BTreeSet::new(),
            listing: Listing::Pending,
            open: HashSet::new(),
            stale: HashSet::new(),
            unlisted_reads: BTreeSet::new(),
            snapshot,
            wiki_roots: Vec::new(),
            contexts,
            generation: 0,
            batch: None,
        }
    }

    /// Defers every rebuild until [`end_batch`](Self::end_batch), so a
    /// notification of many changes rebuilds once. Inside a batch,
    /// [`snapshot`](Self::snapshot) is the one before it.
    pub fn begin_batch(&mut self) {
        self.batch = Some(false);
    }

    /// Ends a batch, running the one rebuild it deferred, if any.
    ///
    /// ## Returns
    ///
    /// Whether the snapshot changed.
    pub fn end_batch(&mut self) -> bool {
        let rebuild = self.batch.take().unwrap_or(false);
        if rebuild {
            self.rebuild();
        }
        rebuild
    }

    /// Sets the wiki roots and rebuilds so already-indexed documents pick up
    /// their canonical logical paths.
    pub fn set_wiki_roots(&mut self, wiki_roots: Vec<PathBuf>) {
        self.wiki_roots = wiki_roots;
        self.rebuild();
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

    /// Indexes an open buffer's text; the buffer overrides disk until
    /// [`close_buffer`](Self::close_buffer).
    pub fn open_buffer(&mut self, path: &Path, source: &str) -> Invalidation {
        self.open.insert(path.to_path_buf());
        self.stale.remove(path);
        self.set_document(path, source)
    }

    /// Restores the disk view of a closed buffer: a listed document becomes
    /// unparsed again, and an unsaved-only or unlisted document leaves the
    /// index.
    pub fn close_buffer(&mut self, path: &Path) {
        if !self.open.remove(path) {
            return;
        }
        if self.documents.remove(path).is_some() {
            self.rebuild();
        }
    }

    /// Replaces the file list with a fresh walk's result.
    ///
    /// Paths that left the list lose their parse unless an open buffer holds
    /// them. A walk reads no contents, so documents that stayed keep their
    /// parse and documents that joined stay unparsed. The snapshot is
    /// rebuilt once.
    ///
    /// ## Returns
    ///
    /// The paths that joined or left the Markdown list.
    pub fn install_listing(
        &mut self,
        markdown: BTreeSet<PathBuf>,
        yaml: BTreeSet<PathBuf>,
        complete: bool,
    ) -> Vec<PathBuf> {
        let mut changed: Vec<PathBuf> = markdown.difference(&self.listed).cloned().collect();
        let vanished: Vec<PathBuf> = self.listed.difference(&markdown).cloned().collect();
        for path in &vanished {
            if !self.open.contains(path) {
                self.documents.remove(path);
                self.stale.remove(path);
            }
        }
        changed.extend(vanished);
        changed.sort();
        self.listed = markdown;
        self.yaml = yaml;
        self.listing = Listing::Ready { complete };
        self.rebuild();
        changed
    }

    /// Records a created or changed Markdown file from a watched event: it
    /// joins the list, and a parsed closed copy is marked stale.
    ///
    /// ## Returns
    ///
    /// Whether the snapshot changed.
    pub fn disk_changed(&mut self, path: &Path) -> bool {
        let joined = self.listed.insert(path.to_path_buf());
        // An open document is already a graph document, and its buffer
        // stays authoritative.
        if self.open.contains(path) {
            return false;
        }
        let parsed = self.documents.contains_key(path);
        if parsed {
            self.stale.insert(path.to_path_buf());
        }
        if joined {
            self.rebuild();
        }
        joined
    }

    /// Records a listed YAML file's creation (`present`) or deletion.
    pub fn yaml_changed(&mut self, path: &Path, present: bool) {
        if present {
            self.yaml.insert(path.to_path_buf());
        } else {
            self.yaml.remove(path);
        }
    }

    /// Drops a deleted Markdown file (listed, or a dependency) from the list
    /// and the index, unless an open buffer holds it.
    ///
    /// ## Returns
    ///
    /// Whether the snapshot changed.
    pub fn disk_deleted(&mut self, path: &Path) -> bool {
        let unlisted = self.listed.remove(path);
        self.stale.remove(path);
        self.unlisted_reads.remove(path);
        if self.open.contains(path) {
            if unlisted {
                self.rebuild();
            }
            return unlisted;
        }
        let dropped = self.documents.remove(path).is_some();
        if unlisted || dropped {
            self.rebuild();
        }
        unlisted || dropped
    }

    /// Re-reads every parsed closed document (a rescan cannot say which
    /// changed), re-parsing only those whose bytes changed. Unparsed
    /// documents stay unparsed.
    ///
    /// ## Returns
    ///
    /// The paths whose parse changed.
    pub fn recheck_parsed(&mut self) -> Vec<PathBuf> {
        let paths: Vec<PathBuf> = self
            .documents
            .keys()
            .filter(|path| !self.open.contains(*path))
            .cloned()
            .collect();
        self.stale.extend(paths.iter().cloned());
        self.ensure_parsed(&paths)
    }

    /// Whether `path` must be read before its contents are known: unparsed
    /// or stale, and not an open buffer.
    ///
    /// Membership in the file list is not asked: a caller passes a listed
    /// document or a Markdown target that resolution found, and either may
    /// be parsed.
    pub fn needs_parse(&self, path: &Path) -> bool {
        !self.open.contains(path)
            && (self.stale.contains(path) || !self.documents.contains_key(path))
    }

    /// Whether `path` is parsed as a [dependency](super::DocumentRecord::dependency):
    /// a referenced target read from disk that is neither listed nor open.
    pub fn is_dependency(&self, path: &Path) -> bool {
        self.unlisted_reads.contains(path)
            && self.documents.contains_key(path)
            && !self.listed.contains(path)
            && !self.open.contains(path)
    }

    /// The parsed [dependencies](Self::is_dependency), in path order.
    pub fn dependencies(&self) -> impl Iterator<Item = &PathBuf> {
        self.unlisted_reads.iter().filter(|path| self.is_dependency(path))
    }

    /// Records a watched change or creation of a file outside the file list.
    ///
    /// A parsed dependency is marked stale and re-read on its next need,
    /// keeping its parse and hash until then. A created Markdown file that is
    /// not parsed rebuilds the snapshot when a reference failed to resolve
    /// to it (see [`WorkspaceGraph::awaits`]), so that reference now finds
    /// it. Any other path is ignored, so files no link names (build output,
    /// generated docs) cost nothing.
    ///
    /// ## Returns
    ///
    /// Whether the snapshot changed.
    pub fn unlisted_changed(&mut self, path: &Path, created: bool) -> bool {
        if self.is_dependency(path) {
            self.stale.insert(path.to_path_buf());
            return false;
        }
        if created && is_markdown_path(path) && !self.documents.contains_key(path) && self.snapshot.awaits(path) {
            self.rebuild();
            return true;
        }
        false
    }

    /// Re-reads every parsed dependency, re-parsing only those whose bytes
    /// changed and dropping those that can no longer be read: the save
    /// fallback for a client that cannot watch a dependency's folder.
    ///
    /// ## Returns
    ///
    /// The paths whose parse changed.
    pub fn recheck_dependencies(&mut self) -> Vec<PathBuf> {
        let paths: Vec<PathBuf> = self.dependencies().cloned().collect();
        self.stale.extend(paths.iter().cloned());
        self.ensure_parsed(&paths)
    }

    /// Re-resolves every reference against the disk as it is now, so a
    /// referenced file created where no watcher reports it becomes a target:
    /// the save fallback's counterpart of
    /// [`unlisted_changed`](Self::unlisted_changed) for a creation.
    pub fn reresolve(&mut self) {
        self.rebuild();
    }

    /// Reads and parses each of `paths` that [needs it](Self::needs_parse),
    /// rebuilding the snapshot once.
    ///
    /// A re-read whose xxHash equals the stored parse's reuses that parse. A
    /// failed read drops the old parse, so stale headings are never served:
    /// a listed document stays listed and unparsed; a dependency leaves the
    /// graph.
    ///
    /// ## Returns
    ///
    /// The paths whose parse changed.
    pub fn ensure_parsed(&mut self, paths: &[PathBuf]) -> Vec<PathBuf> {
        let mut changed = Vec::new();
        for path in paths {
            if !self.needs_parse(path) {
                continue;
            }
            self.stale.remove(path);
            if !self.listed.contains(path) {
                self.unlisted_reads.insert(path.clone());
            }
            match crate::work_counters::read_to_string(path) {
                Ok(source) => {
                    let hash = crate::work_counters::hash_bytes(source.as_bytes());
                    if self.documents.get(path).is_some_and(|existing| existing.content_hash == hash) {
                        continue;
                    }
                    self.documents.insert(path.clone(), index_document(path, &source));
                    changed.push(path.clone());
                }
                Err(error) => {
                    tracing::debug!("unreadable target {}: {error}", path.display());
                    self.unlisted_reads.remove(path);
                    if self.documents.remove(path).is_some() {
                        changed.push(path.clone());
                    }
                }
            }
        }
        if !changed.is_empty() {
            self.rebuild();
        }
        changed
    }

    /// Parses what the open document at `path` needs from the documents it
    /// references: the targets of its fragment links and heading wiki links
    /// (their headings), and its `::file`/`::code` transclusion closure
    /// (followed outward with a visited set, for cycle detection).
    ///
    /// A target is a graph document or an existing Markdown file resolution
    /// found outside the file list, which becomes a dependency; a target in
    /// any other format is never parsed.
    ///
    /// ## Returns
    ///
    /// Whether any parse changed, so the snapshot was rebuilt.
    pub fn prepare_targets(&mut self, path: &Path) -> bool {
        let mut changed = false;
        let mut visited: HashSet<PathBuf> = HashSet::new();
        let mut frontier = vec![path.to_path_buf()];
        let mut first = true;
        while let Some(current) = frontier.pop() {
            if !visited.insert(current.clone()) {
                continue;
            }
            let (headings, transcluded) = self.target_paths(&current, first);
            first = false;
            let wanted: Vec<PathBuf> = headings
                .iter()
                .chain(&transcluded)
                .filter(|target| self.needs_parse(target))
                .cloned()
                .collect();
            if !wanted.is_empty() {
                changed |= !self.ensure_parsed(&wanted).is_empty();
            }
            frontier.extend(transcluded);
        }
        changed
    }

    /// The documents `path` needs headings from (only when `headings` is
    /// asked for) and the documents it transcludes, from the current
    /// snapshot: graph documents, and Markdown files outside the graph.
    fn target_paths(&self, path: &Path, headings: bool) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let snapshot = &self.snapshot;
        let Some(doc) = snapshot.document_id(path) else {
            return (Vec::new(), Vec::new());
        };
        let document_of = |target: &EdgeTarget| match target {
            EdgeTarget::Node(node) => {
                let node = snapshot.node(*node)?;
                snapshot.document(node.document).map(|record| record.path.clone())
            }
            EdgeTarget::File(file) => is_markdown_path(file).then(|| file.clone()),
            EdgeTarget::Unresolved(_) => None,
        };
        let mut heading_targets = Vec::new();
        if headings {
            for (node_id, node) in snapshot.links(doc) {
                let fragment = node.as_link().is_some_and(|link| {
                    matches!(link.target, LinkTarget::RelativePath { fragment: Some(_), .. })
                });
                if !fragment {
                    continue;
                }
                for edge in snapshot.outgoing(node_id, EdgeKind::References) {
                    heading_targets.extend(document_of(&edge.target));
                }
            }
            for (_, node) in snapshot.wiki_links(doc) {
                if let Some(wiki) = node.as_wiki_link()
                    && wiki.heading.is_some()
                    && let WikiResolution::Resolved(target) = wiki.resolution
                {
                    heading_targets.extend(document_of(&EdgeTarget::Node(target)));
                }
            }
        }
        let mut transcluded = Vec::new();
        for (node_id, _) in snapshot.transclusions(doc) {
            for edge in snapshot.outgoing(node_id, EdgeKind::Transcludes) {
                transcluded.extend(document_of(&edge.target));
            }
        }
        (heading_targets, transcluded)
    }

    /// The file list's state.
    pub fn listing(&self) -> Listing {
        self.listing
    }

    /// The listed Markdown files.
    pub fn listed(&self) -> &BTreeSet<PathBuf> {
        &self.listed
    }

    /// The listed YAML files.
    pub fn listed_yaml(&self) -> &BTreeSet<PathBuf> {
        &self.yaml
    }

    /// Drops a document (deletion), rebuilding the snapshot if it was present.
    pub fn remove_document(&mut self, path: &Path) -> Invalidation {
        self.listed.remove(path);
        self.open.remove(path);
        self.stale.remove(path);
        self.unlisted_reads.remove(path);
        if self.documents.remove(path).is_none() {
            return Invalidation::Unchanged;
        }
        let dependents = self.dependents_of(path);
        self.rebuild();
        Invalidation::Reindexed { dependents }
    }

    /// Rebuilds the snapshot so every path reference re-resolves, after the
    /// contexts behind them were dropped.
    pub fn relink(&mut self) {
        self.rebuild();
    }

    /// The parse of every open buffer, saved or not.
    pub fn open_buffers(&self) -> impl Iterator<Item = &DocumentIndex> {
        self.open.iter().filter_map(|path| self.documents.get(path))
    }

    /// `path`'s disk parse: `None` when it is unparsed or an open buffer
    /// holds it. A stale parse is returned too; its hash tells whether it
    /// still matches the bytes.
    pub fn disk_parse(&self, path: &Path) -> Option<&DocumentIndex> {
        if self.open.contains(path) {
            return None;
        }
        self.documents.get(path)
    }

    /// Whether a document is parsed.
    pub fn contains(&self, path: &Path) -> bool {
        self.documents.contains_key(path)
    }

    /// Number of parsed documents.
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Whether no documents are parsed.
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
        if let Some(requested) = self.batch.as_mut() {
            *requested = true;
            return;
        }
        self.generation += 1;
        let dependencies: BTreeSet<PathBuf> = self.dependencies().cloned().collect();
        // The universe is this index's own wiki candidates: the file list
        // plus every other document that is not a referenced dependency
        // (open buffers outside the list). It is exactly the set the graph
        // derived for itself before the universe became a parameter, so
        // open-document diagnostics are unchanged.
        let universe = WikiUniverse::new(
            self.listed
                .iter()
                .chain(self.documents.keys().filter(|path| !dependencies.contains(*path)))
                .cloned(),
            &self.wiki_roots,
        );
        self.snapshot = Arc::new(WorkspaceGraph::build_listed(
            &self.documents,
            &self.listed,
            &dependencies,
            self.listing,
            self.generation,
            universe,
            self.contexts.as_ref(),
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::test_support::{abs, workspace_contexts};

    #[test]
    fn test_unchanged_content_skips_rebuild() {
        let mut index = WorkspaceIndex::new(Arc::new(workspace_contexts()));
        let path = abs("/w/a.md");
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
        let mut index = WorkspaceIndex::new(Arc::new(workspace_contexts()));
        let path = abs("/w/a.md");
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
        let mut index = WorkspaceIndex::new(Arc::new(workspace_contexts()));
        let path = abs("/w/a.md");
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
        let mut index = WorkspaceIndex::new(Arc::new(workspace_contexts()));
        let old = abs("/w/old.md");
        let new = abs("/w/new.md");
        index.set_document(&old, "# Title\n");
        index.remove_document(&old);
        index.set_document(&new, "# Title\n");
        let snapshot = index.snapshot();
        assert!(snapshot.document_id(&old).is_none());
        assert!(snapshot.document_id(&new).is_some());
    }

    /// A real directory, canonicalized so graph paths and resolved
    /// candidates agree, with contexts resolving inside it.
    fn disk_workspace() -> (tempfile::TempDir, PathBuf, Arc<dyn DocumentContexts>) {
        let temp = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        let context = biscuit_file::FileResolutionContext::from_snapshot(
            root.clone(),
            None,
            std::collections::HashMap::new(),
        );
        (temp, root, Arc::new(crate::context::FixedContext(context)))
    }

    fn listed(index: &mut WorkspaceIndex, paths: &[&PathBuf]) -> Vec<PathBuf> {
        index.install_listing(paths.iter().map(|path| (*path).clone()).collect(), BTreeSet::new(), true)
    }

    fn heading_titles(index: &WorkspaceIndex, path: &Path) -> Vec<String> {
        let snapshot = index.snapshot();
        let Some(doc) = snapshot.document_id(path) else {
            return Vec::new();
        };
        snapshot
            .headings(doc)
            .filter_map(|(_, node)| node.as_heading().map(|heading| heading.title.clone()))
            .collect()
    }

    #[test]
    fn test_listed_documents_are_unparsed_graph_documents() {
        let (_temp, root, contexts) = disk_workspace();
        let a = root.join("a.md");
        std::fs::write(&a, "# A\n").unwrap();
        let mut index = WorkspaceIndex::new(contexts);
        assert_eq!(index.snapshot().listing(), Listing::Pending);
        let before = crate::work_counters::work_counts();

        assert_eq!(listed(&mut index, &[&a]), vec![a.clone()]);

        assert_eq!(crate::work_counters::work_counts().since(before).reads, 0);
        let snapshot = index.snapshot();
        let record = snapshot.document(snapshot.document_id(&a).unwrap()).unwrap();
        assert!(!record.parsed);
        assert_eq!(record.content_hash, 0);
        assert_eq!(snapshot.listing(), Listing::Ready { complete: true });
        assert!(index.needs_parse(&a));
    }

    #[test]
    fn test_install_listing_adopts_created_drops_vanished_and_keeps_open_buffers() {
        let (_temp, root, contexts) = disk_workspace();
        let (a, b, open) = (root.join("a.md"), root.join("b.md"), root.join("open.md"));
        let mut index = WorkspaceIndex::new(contexts);
        index.open_buffer(&open, "# Buffer edit\n");
        listed(&mut index, &[&a, &open]);

        // b.md joins, a.md leaves; the open buffer survives a list without it.
        let changed = listed(&mut index, &[&b]);
        assert_eq!(changed, vec![a.clone(), b.clone(), open.clone()]);
        let snapshot = index.snapshot();
        assert!(snapshot.document_id(&a).is_none());
        assert!(snapshot.document_id(&b).is_some());
        assert_eq!(heading_titles(&index, &open), vec!["Buffer edit".to_string()]);

        // The same list again changes nothing.
        assert!(listed(&mut index, &[&b]).is_empty());
    }

    #[test]
    fn test_ensure_parsed_reuses_an_unchanged_parse_and_never_serves_a_failed_read() {
        let (_temp, root, contexts) = disk_workspace();
        let a = root.join("a.md");
        std::fs::write(&a, "# First\n").unwrap();
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &[&a]);

        let before = crate::work_counters::work_counts();
        assert_eq!(index.ensure_parsed(std::slice::from_ref(&a)), vec![a.clone()]);
        assert_eq!(crate::work_counters::work_counts().since(before).parses, 1);
        assert_eq!(heading_titles(&index, &a), vec!["First".to_string()]);

        // Parsed and current: nothing is read.
        let before = crate::work_counters::work_counts();
        assert!(index.ensure_parsed(std::slice::from_ref(&a)).is_empty());
        assert_eq!(crate::work_counters::work_counts().since(before).reads, 0);

        // A watched change marks it stale; unchanged bytes reuse the parse.
        assert!(!index.disk_changed(&a));
        assert!(index.needs_parse(&a));
        let before = crate::work_counters::work_counts();
        assert!(index.ensure_parsed(std::slice::from_ref(&a)).is_empty());
        let work = crate::work_counters::work_counts().since(before);
        assert_eq!((work.reads, work.parses), (1, 0));

        // Changed bytes re-parse.
        std::fs::write(&a, "# Second\n").unwrap();
        index.disk_changed(&a);
        assert_eq!(index.ensure_parsed(std::slice::from_ref(&a)), vec![a.clone()]);
        assert_eq!(heading_titles(&index, &a), vec!["Second".to_string()]);

        // A failed read drops the old headings; the file stays listed.
        std::fs::remove_file(&a).unwrap();
        index.disk_changed(&a);
        assert_eq!(index.ensure_parsed(std::slice::from_ref(&a)), vec![a.clone()]);
        assert!(heading_titles(&index, &a).is_empty());
        let snapshot = index.snapshot();
        assert!(!snapshot.document(snapshot.document_id(&a).unwrap()).unwrap().parsed);

        // Deletion removes it.
        assert!(index.disk_deleted(&a));
        assert!(index.snapshot().document_id(&a).is_none());
    }

    #[test]
    fn test_prepare_targets_parses_fragment_targets_and_the_transclusion_closure_only() {
        let (_temp, root, contexts) = disk_workspace();
        let files = [
            ("source.md", "[plain](plain.md) [anchor](anchored.md#part)\n\n::file ./inner.md\n"),
            ("plain.md", "# Plain\n"),
            ("anchored.md", "# Part\n"),
            ("inner.md", "::file ./deep.md\n"),
            ("deep.md", "# Deep\n"),
            ("unrelated.md", "# Unrelated\n"),
        ];
        let paths: Vec<PathBuf> = files.iter().map(|(name, _)| root.join(name)).collect();
        for (name, text) in files {
            std::fs::write(root.join(name), text).unwrap();
        }
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &paths.iter().collect::<Vec<_>>());
        let source = &paths[0];
        index.open_buffer(source, files[0].1);

        let before = crate::work_counters::work_counts();
        assert!(index.prepare_targets(source));
        assert_eq!(crate::work_counters::work_counts().since(before).parses, 3);
        for (path, parsed) in paths.iter().zip([true, false, true, true, true, false]) {
            assert_eq!(index.contains(path), parsed, "{}", path.display());
        }
        // The anchor now resolves to the target's heading.
        let snapshot = index.snapshot();
        let doc = snapshot.document_id(source).unwrap();
        let (anchor, _) = snapshot.links(doc).nth(1).unwrap();
        let target = snapshot.outgoing(anchor, EdgeKind::References).next().unwrap();
        let heading = snapshot.node(target.target.node().unwrap()).unwrap();
        assert_eq!(heading.as_heading().unwrap().title, "Part");

        // A second preparation reads nothing.
        let before = crate::work_counters::work_counts();
        assert!(!index.prepare_targets(source));
        assert_eq!(crate::work_counters::work_counts().since(before).reads, 0);
    }

    #[test]
    fn test_unlisted_markdown_targets_become_dependencies() {
        let (_temp, root, contexts) = disk_workspace();
        let files = [
            ("source.md", "[a](target.md#beta) [[target]]\n\n::file ./target.md\n::code ./mod.rs\n"),
            ("target.md", "# Alpha\n\n## Beta\n\n[back](source.md#nowhere) [[source]]\n\n::file ./source.md\n"),
            ("mod.rs", "# not a heading\n"),
        ];
        for (name, text) in files {
            std::fs::write(root.join(name), text).unwrap();
        }
        let (source, target) = (root.join("source.md"), root.join("target.md"));
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &[&source]);
        index.open_buffer(&source, files[0].1);

        let before = crate::work_counters::work_counts();
        assert!(index.prepare_targets(&source));
        let work = crate::work_counters::work_counts().since(before);
        assert_eq!((work.parses, work.reads), (1, 1), "target.md only, never mod.rs: {work:?}");
        assert!(index.is_dependency(&target));
        assert_eq!(index.dependencies().collect::<Vec<_>>(), vec![&target]);

        let snapshot = index.snapshot();
        let target_doc = snapshot.document_id(&target).unwrap();
        let record = snapshot.document(target_doc).unwrap();
        assert!(record.dependency && record.parsed);
        assert_eq!(heading_titles(&index, &target), vec!["Alpha".to_string(), "Beta".to_string()]);
        // Headings and transclusions only: no backlinks or wiki candidacy.
        assert_eq!(snapshot.links(target_doc).count() + snapshot.wiki_links(target_doc).count(), 0);
        assert!(snapshot.transclusion_cycle(snapshot.document_id(&source).unwrap()).is_some());
        let target_slot = snapshot.wiki_slot(target_doc).unwrap();
        assert!(snapshot.wiki_universe().docs()[target_slot].canonical.is_empty());
        let source_doc = snapshot.document_id(&source).unwrap();
        let (_, wiki) = snapshot.wiki_links(source_doc).next().unwrap();
        assert!(!matches!(wiki.as_wiki_link().unwrap().resolution, WikiResolution::Resolved(_)));
        assert!(snapshot.transcluded_by(source_doc).is_empty());

        // A watched change marks it stale; the next need re-reads it.
        std::fs::write(&target, "# Gamma\n").unwrap();
        assert!(!index.unlisted_changed(&target, false), "a stale mark keeps the snapshot");
        assert!(!index.unlisted_changed(&root.join("mod.rs"), true));
        assert!(index.prepare_targets(&source));
        assert_eq!(heading_titles(&index, &target), vec!["Gamma".to_string()]);

        // The save fallback re-reads it; a failed read drops it.
        std::fs::remove_file(&target).unwrap();
        assert_eq!(index.recheck_dependencies(), vec![target.clone()]);
        assert!(!index.contains(&target));

        // A creation re-resolves the source's references to it; joining the
        // list then turns the dependency into a listed document.
        std::fs::write(&target, "# Alpha\n").unwrap();
        assert!(index.unlisted_changed(&target, true));
        index.prepare_targets(&source);
        assert!(index.is_dependency(&target));
        listed(&mut index, &[&source, &target]);
        assert!(!index.is_dependency(&target));
        assert!(!index.snapshot().document(index.snapshot().document_id(&target).unwrap()).unwrap().dependency);
    }

    /// Created Markdown nothing links to (build output, generated docs) never
    /// rebuilds; creating a file a link is waiting for rebuilds once and
    /// resolves it.
    #[test]
    fn test_only_an_awaited_unlisted_creation_rebuilds() {
        let (_temp, root, contexts) = disk_workspace();
        let (source, target) = (root.join("source.md"), root.join("target.md"));
        std::fs::write(&source, "[a](target.md)\n").unwrap();
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &[&source]);
        index.open_buffer(&source, "[a](target.md)\n");
        assert!(index.snapshot().awaits(&target), "the broken link waits for target.md");

        let generated = root.join("target-dir/doc");
        std::fs::create_dir_all(&generated).unwrap();
        let generation = index.generation();
        for n in 0..20 {
            let path = generated.join(format!("gen{n}.md"));
            std::fs::write(&path, "# generated\n").unwrap();
            assert!(!index.unlisted_changed(&path, true), "nothing links to {path:?}");
        }
        assert_eq!(index.generation(), generation, "unreferenced creations never rebuild");

        std::fs::write(&target, "# Target\n").unwrap();
        assert!(index.unlisted_changed(&target, true));
        assert_eq!(index.generation(), generation + 1);
        assert!(!index.snapshot().awaits(&target), "the link now finds target.md");
    }

    /// A batch defers every rebuild its changes ask for to one at its end.
    #[test]
    fn test_a_batch_of_joined_files_rebuilds_once() {
        let (_temp, root, contexts) = disk_workspace();
        let paths: Vec<PathBuf> = ["a.md", "b.md", "c.md", "d.md"].iter().map(|name| root.join(name)).collect();
        for path in &paths {
            std::fs::write(path, "# Doc\n").unwrap();
        }
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &[&paths[0]]);
        let generation = index.generation();

        index.begin_batch();
        for path in &paths[1..] {
            assert!(index.disk_changed(path), "{path:?} joins the list");
        }
        assert_eq!(index.generation(), generation, "nothing rebuilt inside the batch");
        assert!(index.end_batch());
        assert_eq!(index.generation(), generation + 1, "one rebuild for the batch");
        for path in &paths {
            assert!(index.snapshot().document_id(path).is_some(), "{path:?} is a graph document");
        }
        index.begin_batch();
        assert!(!index.end_batch(), "an empty batch rebuilds nothing");
        assert_eq!(index.generation(), generation + 1);
    }

    #[test]
    fn test_close_restores_the_disk_view_or_drops_an_unsaved_buffer() {
        let (_temp, root, contexts) = disk_workspace();
        let (saved, unsaved) = (root.join("saved.md"), root.join("unsaved.md"));
        std::fs::write(&saved, "# Disk\n").unwrap();
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &[&saved]);
        index.open_buffer(&saved, "# Buffer\n");
        index.open_buffer(&unsaved, "# Unsaved\n");
        assert_eq!(heading_titles(&index, &saved), vec!["Buffer".to_string()]);

        index.close_buffer(&saved);
        index.close_buffer(&unsaved);
        assert!(index.snapshot().document_id(&unsaved).is_none());
        assert!(index.needs_parse(&saved));
        index.ensure_parsed(std::slice::from_ref(&saved));
        assert_eq!(heading_titles(&index, &saved), vec!["Disk".to_string()]);
    }

    #[test]
    fn test_watched_changes_and_relists_read_nothing_until_a_need() {
        let (_temp, root, contexts) = disk_workspace();
        let (a, b) = (root.join("a.md"), root.join("b.md"));
        std::fs::write(&a, "# A\n").unwrap();
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &[&a]);
        index.ensure_parsed(std::slice::from_ref(&a));

        let before = crate::work_counters::work_counts();
        std::fs::write(&a, "# A changed\n").unwrap();
        std::fs::write(&b, "# B\n").unwrap();
        assert!(!index.disk_changed(&a), "a stale mark keeps the snapshot");
        assert!(index.disk_changed(&b), "a joined file is a new graph document");
        listed(&mut index, &[&a, &b]);
        assert_eq!(crate::work_counters::work_counts().since(before).reads, 0);
        assert!(index.needs_parse(&a) && index.needs_parse(&b));
        assert_eq!(heading_titles(&index, &a), vec!["A".to_string()], "the stale parse is kept until a need");

        index.ensure_parsed(&[a.clone(), b.clone()]);
        assert_eq!(heading_titles(&index, &a), vec!["A changed".to_string()]);
        assert_eq!(heading_titles(&index, &b), vec!["B".to_string()]);
    }

    const UNREADABLE: [u8; 4] = [0xff, 0xfe, 0x00, 0x80];

    #[test]
    fn test_a_failed_read_drops_the_parse_and_a_rescan_never_reads_an_unparsed_file() {
        let (_temp, root, contexts) = disk_workspace();
        let [a, b] = ["a.md", "b.md"].map(|name| root.join(name));
        std::fs::write(&a, "# A\n").unwrap();
        std::fs::write(&b, "# B\n").unwrap();
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &[&a, &b]);
        index.ensure_parsed(std::slice::from_ref(&a));

        // Rescan: the parsed file is found unreadable and loses its parse;
        // the unparsed one is not read.
        std::fs::write(&a, UNREADABLE).unwrap();
        let before = crate::work_counters::work_counts();
        assert_eq!(index.recheck_parsed(), vec![a.clone()]);
        assert_eq!(crate::work_counters::work_counts().since(before).reads, 1);
        assert!(!index.contains(&a) && !index.contains(&b));
        assert!(index.listed().contains(&a), "an unreadable file stays listed");

        // Closing a buffer over unreadable disk bytes leaves it unparsed.
        index.open_buffer(&b, "# Buffer\n");
        std::fs::write(&b, UNREADABLE).unwrap();
        index.close_buffer(&b);
        assert!(!index.contains(&b) && index.needs_parse(&b));
    }

    #[test]
    fn test_unparsed_targets_leave_anchors_unchecked() {
        let (_temp, root, contexts) = disk_workspace();
        let (source, target) = (root.join("source.md"), root.join("target.md"));
        std::fs::write(&target, "# Real\n").unwrap();
        let mut index = WorkspaceIndex::new(contexts);
        listed(&mut index, &[&target]);
        index.open_buffer(&source, "[x](target.md#missing) [[target#missing]]\n");
        let snapshot = index.snapshot();
        let doc = snapshot.document_id(&source).unwrap();
        let target_root = snapshot.document(snapshot.document_id(&target).unwrap()).unwrap().root;
        let (link, _) = snapshot.links(doc).next().unwrap();
        let edge = snapshot.outgoing(link, EdgeKind::References).next().unwrap();
        assert_eq!(edge.target.node(), Some(target_root));
        let (_, wiki) = snapshot.wiki_links(doc).next().unwrap();
        assert_eq!(wiki.as_wiki_link().unwrap().resolution, WikiResolution::Resolved(target_root));

        // Once parsed, the missing anchor no longer resolves.
        index.prepare_targets(&source);
        let snapshot = index.snapshot();
        let doc = snapshot.document_id(&source).unwrap();
        let (link, _) = snapshot.links(doc).next().unwrap();
        let edge = snapshot.outgoing(link, EdgeKind::References).next().unwrap();
        assert!(edge.target.node().is_none());
        let (_, wiki) = snapshot.wiki_links(doc).next().unwrap();
        assert!(!matches!(wiki.as_wiki_link().unwrap().resolution, WikiResolution::Resolved(_)));
    }

    #[test]
    fn test_set_wiki_roots_changes_wiki_resolution() {
        use crate::graph::WikiResolution;

        let mut index = WorkspaceIndex::new(Arc::new(workspace_contexts()));
        let target = abs("/w/notes/Target.md");
        let source = abs("/w/notes/Source.md");
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
        index.set_wiki_roots(vec![abs("/w")]);
        assert!(matches!(resolution(&index), WikiResolution::Resolved(_)));

        // Narrowing the wiki root to `/w/notes` shortens Target's canonical path
        // to `Target`, so the same root-relative link no longer matches — the
        // reload-triggered rebuild flips resolution without any document edit.
        index.set_wiki_roots(vec![abs("/w/notes")]);
        assert_eq!(resolution(&index), WikiResolution::Unresolved);
    }

    #[test]
    fn test_plain_link_change_reports_no_dependents() {
        // A plain Markdown link is a `references` edge, not a compositional
        // (`transcludes`/`uses_file`) edge, so changing its target reports no
        // dependents to re-diagnose.
        let mut index = WorkspaceIndex::new(Arc::new(workspace_contexts()));
        let a = abs("/w/a.md");
        let b = abs("/w/b.md");
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
        let mut index = WorkspaceIndex::new(Arc::new(workspace_contexts()));
        let a = abs("/w/a.md");
        let b = abs("/w/b.md");
        index.set_document(&a, "---\n$schema:\n  include: \"file\"\ninclude: ./b.md\n---\n\n# A\n");
        index.set_document(&b, "# B\n");
        let outcome = index.set_document(&b, "# B changed\n");
        assert_eq!(outcome, Invalidation::Reindexed { dependents: vec![a] });
    }

    #[test]
    fn test_transclude_change_reports_dependents() {
        // a.md transcludes b.md, so editing b.md fans out to a.md.
        let mut index = WorkspaceIndex::new(Arc::new(workspace_contexts()));
        let a = abs("/w/a.md");
        let b = abs("/w/b.md");
        index.set_document(&a, "::file ./b.md\n");
        index.set_document(&b, "# B\n");
        let outcome = index.set_document(&b, "# B changed\n");
        assert_eq!(outcome, Invalidation::Reindexed { dependents: vec![a] });
    }
}
