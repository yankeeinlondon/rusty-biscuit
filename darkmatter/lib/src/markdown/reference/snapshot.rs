//! Build-time prepared-heading snapshot carried by a reference graph.
//!
//! Graph construction already runs InlinePre preparation and parses the TOC
//! for every visited node; this snapshot retains the resulting lowercased
//! heading slugs so fragment validation checks the composed heading set
//! against the build's own content instead of rereading children from disk.
//! A post-build heading edit therefore cannot leak into a fresh one-step
//! report, and the checked prebuilt path only reaches the snapshot after
//! descendant verification has proven disk content identical to it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Canonical key for heading-slug lookups.
///
/// Shared by the graph-owned snapshot and the per-run validation cache so a
/// document reached as both a graph descendant and a `path#fragment` target
/// resolves to one entry. Best-effort canonicalization matches how node
/// source paths are resolved at build time, keeping keys stable across
/// macOS (`/var` vs `/private/var`), Linux, and Windows.
pub(crate) fn heading_slug_key(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// Prepared heading slugs for every file-sourced node visited by a graph
/// build, keyed by canonical path.
///
/// Only production graph construction populates this; synthetic graphs in
/// tests use the `Default` empty snapshot, and validation keeps its
/// disk-loading fallback for any node with no recorded entry.
#[derive(Debug, Clone, Default)]
pub(crate) struct PreparedHeadingSnapshot {
    slugs: HashMap<PathBuf, Vec<String>>,
}

impl PreparedHeadingSnapshot {
    /// Records the prepared slugs for one file-sourced node.
    pub(crate) fn record(&mut self, path: &Path, slugs: Vec<String>) {
        self.slugs.insert(heading_slug_key(path), slugs);
    }

    /// Returns the recorded prepared slugs for `path`, if any.
    pub(crate) fn slugs_for(&self, path: &Path) -> Option<&[String]> {
        self.slugs.get(&heading_slug_key(path)).map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_then_lookup_round_trips() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("child.md");
        std::fs::write(&path, "# Child\n").unwrap();

        let mut snapshot = PreparedHeadingSnapshot::default();
        snapshot.record(&path, vec!["child".to_string()]);

        assert_eq!(snapshot.slugs_for(&path), Some(&["child".to_string()][..]));
        assert!(snapshot.slugs_for(&dir.path().join("absent.md")).is_none());
    }

    #[test]
    fn lookup_matches_across_path_spelling() {
        // A non-canonical spelling of the same file must hit the same entry;
        // this is what keeps the snapshot and the per-run cache interchangeable.
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("child.md");
        std::fs::write(&path, "# Child\n").unwrap();

        let mut snapshot = PreparedHeadingSnapshot::default();
        snapshot.record(&path, vec!["child".to_string()]);

        let dotted = dir.path().join(".").join("child.md");
        assert!(snapshot.slugs_for(&dotted).is_some());
    }
}
