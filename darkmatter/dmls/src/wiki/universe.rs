//! The wiki universe: every file a wiki link is matched and judged ambiguous
//! against.
//!
//! A [`WorkspaceGraph`](crate::graph::WorkspaceGraph) no longer derives its
//! wiki candidates from the documents it holds. The caller supplies a
//! [`WikiUniverse`], so a graph built over a few request-scoped documents
//! still resolves `[[name]]`, reports ambiguity, and picks rename spellings
//! against the whole file list, exactly as a graph over every file would.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::logical_path::{canonical_segments, portability_key};
use super::resolve::WikiDoc;

/// The wiki-name universe, one slot per file.
///
/// Slots are assigned in path order and are stable for the life of the
/// value; [`docs`](Self::docs) is indexed by slot, which is the index
/// [`resolve_file`](super::resolve_file) and
/// [`shortest_unique_suffix`](super::shortest_unique_suffix) take and return.
/// Each slot's [portability key](Self::portability_key) is computed once, when
/// the slot is added.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WikiUniverse {
    wiki_roots: Vec<PathBuf>,
    paths: Vec<PathBuf>,
    docs: Vec<WikiDoc>,
    /// Indexed by slot, aligned with `paths` and `docs`.
    portability_keys: Vec<String>,
    by_path: HashMap<PathBuf, usize>,
}

impl WikiUniverse {
    /// A universe over `paths` (logical workspace paths; duplicates are
    /// dropped), each canonicalized against `wiki_roots` (R-8).
    pub fn new(paths: impl IntoIterator<Item = PathBuf>, wiki_roots: &[PathBuf]) -> Self {
        crate::work_counters::record_wiki_universe();
        let mut paths: Vec<PathBuf> = paths.into_iter().collect();
        paths.sort();
        paths.dedup();
        let mut universe = Self {
            wiki_roots: wiki_roots.to_vec(),
            ..Self::default()
        };
        for path in paths {
            let doc = WikiDoc::for_path(&path, wiki_roots);
            universe.push(path, doc);
        }
        universe
    }

    /// The wiki roots every slot's canonical logical path is relative to.
    pub fn wiki_roots(&self) -> &[PathBuf] {
        &self.wiki_roots
    }

    /// Every slot as the matcher sees it.
    pub fn docs(&self) -> &[WikiDoc] {
        &self.docs
    }

    /// The logical path of `slot`.
    pub fn path(&self, slot: usize) -> Option<&Path> {
        self.paths.get(slot).map(PathBuf::as_path)
    }

    /// The [portability key](super::portability_key) of `slot`'s canonical
    /// logical path.
    pub(crate) fn portability_key(&self, slot: usize) -> Option<&str> {
        self.portability_keys.get(slot).map(String::as_str)
    }

    /// The slot holding `path`.
    pub fn slot(&self, path: &Path) -> Option<usize> {
        self.by_path.get(path).copied()
    }

    #[allow(missing_docs)]
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    #[allow(missing_docs)]
    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }

    /// Appends `path` as a new last slot holding `doc`, returning the slot.
    ///
    /// Used by the graph build for a document missing from the supplied
    /// universe, so every graph document has a slot to resolve from. Slots
    /// already handed out keep their meaning.
    pub(crate) fn push(&mut self, path: PathBuf, doc: WikiDoc) -> usize {
        let slot = self.docs.len();
        self.by_path.insert(path.clone(), slot);
        self.paths.push(path);
        self.portability_keys.push(counted_portability_key(&doc.canonical));
        self.docs.push(doc);
        slot
    }
}

impl WikiDoc {
    /// `path` as the matcher sees it under `wiki_roots`: relative to the first
    /// root that contains it, else an absolute-path tail with
    /// [`WikiDoc::NO_ROOT`].
    pub fn for_path(path: &Path, wiki_roots: &[PathBuf]) -> Self {
        crate::work_counters::record_wiki_path_facts();
        for (index, root) in wiki_roots.iter().enumerate() {
            if path.starts_with(root) {
                return Self {
                    canonical: canonical_segments(path, Some(root)),
                    workspace_id: index,
                };
            }
        }
        Self {
            canonical: canonical_segments(path, None),
            workspace_id: Self::NO_ROOT,
        }
    }

    /// A slot no wiki target ever matches (a referenced dependency has no
    /// logical path).
    pub(crate) fn unmatchable() -> Self {
        Self {
            canonical: Vec::new(),
            workspace_id: Self::NO_ROOT,
        }
    }

    /// Whether this is an [unmatchable](Self::unmatchable) slot. Every real
    /// path has at least one canonical segment.
    pub(crate) fn is_unmatchable(&self) -> bool {
        self.canonical.is_empty()
    }
}

/// [`portability_key`], counted.
pub(crate) fn counted_portability_key(canonical: &[String]) -> String {
    crate::work_counters::record_wiki_portability_key();
    portability_key(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::context::test_support::abs;

    #[test]
    fn test_slots_follow_path_order_and_drop_duplicates() {
        let universe = WikiUniverse::new(
            [abs("/w/b.md"), abs("/w/a.md"), abs("/w/b.md")],
            &[abs("/w")],
        );
        assert_eq!(universe.len(), 2);
        assert_eq!(universe.slot(&abs("/w/a.md")), Some(0));
        assert_eq!(universe.slot(&abs("/w/b.md")), Some(1));
        assert_eq!(universe.path(1), Some(abs("/w/b.md").as_path()));
        assert_eq!(universe.docs()[1].canonical, vec!["b".to_string()]);
        assert_eq!(universe.docs()[1].workspace_id, 0);
    }

    #[test]
    fn test_path_outside_every_root_has_no_root() {
        let universe = WikiUniverse::new([abs("/elsewhere/n.md")], &[abs("/w")]);
        assert_eq!(universe.docs()[0].workspace_id, WikiDoc::NO_ROOT);
        assert_eq!(universe.docs()[0].canonical.last().map(String::as_str), Some("n"));
    }

    #[test]
    fn test_first_containing_root_wins() {
        let universe = WikiUniverse::new([abs("/w/notes/n.md")], &[abs("/w"), abs("/w/notes")]);
        assert_eq!(universe.docs()[0].workspace_id, 0);
        assert_eq!(universe.docs()[0].canonical, vec!["notes".to_string(), "n".to_string()]);
    }

    #[test]
    fn test_every_slot_carries_its_portability_key_once() {
        use crate::work_counters::work_counts;

        let before = work_counts();
        let mut universe = WikiUniverse::new(
            [abs("/w/Notes/Topic.md"), abs("/w/notes/topic.md")],
            &[abs("/w")],
        );
        let built = work_counts().since(before);
        assert_eq!(
            (built.wiki_universes, built.wiki_path_facts, built.wiki_portability_keys),
            (1, 2, 2)
        );
        // A case-only difference collides under the portability key.
        assert_eq!(universe.portability_key(0), Some("notes/topic"));
        assert_eq!(universe.portability_key(1), Some("notes/topic"));

        // An appended slot, unmatchable ones included, keeps every slot
        // array aligned.
        let slot = universe.push(abs("/elsewhere/dep.md"), WikiDoc::unmatchable());
        assert_eq!(slot, 2);
        assert_eq!(universe.portability_key(2), Some(""));
        assert!(universe.docs()[2].is_unmatchable());
        assert_eq!(universe.path(2), Some(abs("/elsewhere/dep.md").as_path()));
        assert_eq!(universe.portability_key(3), None);
        assert_eq!(work_counts().since(before).wiki_portability_keys, 3);

        // Reading keys computes nothing; a clone is equal and aligned.
        let read = work_counts();
        let copy = universe.clone();
        assert_eq!(copy, universe);
        assert_eq!(copy.portability_key(2), Some(""));
        assert_eq!(work_counts().since(read).wiki_portability_keys, 0);
        assert_eq!(WikiUniverse::default().portability_key(0), None);
    }
}
