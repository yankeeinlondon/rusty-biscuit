//! Build-time prepared-heading snapshot carried by a reference graph.
//!
//! Graph construction already runs InlinePre preparation and parses the TOC
//! for every visited node; this snapshot retains the resulting lowercased
//! heading slugs so fragment validation checks the composed heading set
//! against the build's own content instead of rereading children from disk,
//! and each prepared heading's offset, final slug, title, and level so a
//! reference's section can be named from its prepared byte offset.
//! A post-build heading edit therefore cannot leak into a fresh one-step
//! report, and the checked prebuilt path only reaches the snapshot after
//! descendant verification has proven disk content identical to it.
//!
//! It also keeps, per generated `::toc-linking` reference, the raw heading
//! slugs of the target the anchor was built from: the generator, like
//! composition, links raw headings, so those anchors are checked against the
//! raw set while every other link keeps the prepared one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::types::ReferenceRecord;

use crate::markdown::normalize::HeadingLevel;

/// Canonical key for heading-slug lookups.
///
/// Shared by the graph-owned snapshot and the per-run validation cache so a
/// document reached as both a graph descendant and a `path#fragment` target
/// resolves to one entry. Best-effort canonicalization matches how node
/// source paths are resolved at build time, keeping keys stable across
/// macOS (`/var` vs `/private/var`), Linux, and Windows.
pub(crate) fn heading_slug_key(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

/// A heading of a prepared body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedHeading {
    /// Byte offset of the heading in the prepared body.
    pub(crate) start: usize,
    /// The final anchor slug, with duplicate numbering.
    pub(crate) slug: String,
    pub(crate) title: String,
    pub(crate) level: HeadingLevel,
}

/// The headings of a prepared body, in document order.
///
/// `extract_headings` over the prepared text is the slug and span authority:
/// a heading in a removed block is not in that text, and an interpolated one
/// is read after interpolation.
pub(crate) fn prepared_headings_of(prepared: &str) -> Vec<PreparedHeading> {
    crate::markdown::toc::extract_headings(prepared)
        .into_iter()
        .map(|heading| PreparedHeading {
            start: heading.heading_span.start,
            slug: heading.slug,
            title: heading.title,
            level: heading.level,
        })
        .collect()
}

/// The innermost heading at or before byte `offset` of the prepared body
/// `headings` (in document order) came from.
pub(crate) fn heading_at(headings: &[PreparedHeading], offset: usize) -> Option<&PreparedHeading> {
    let after = headings.partition_point(|heading| heading.start <= offset);
    after.checked_sub(1).map(|index| &headings[index])
}

/// The recorded headings of one file-sourced node.
///
/// `slugs` is what fragment validation checks and keeps the TOC's spelling
/// (lowercased, duplicates not numbered), which `headings` does not.
#[derive(Debug, Clone, Default)]
struct PreparedDocumentHeadings {
    slugs: Vec<String>,
    headings: Vec<PreparedHeading>,
}

/// Prepared headings for every file-sourced node visited by a graph build,
/// keyed by canonical path.
///
/// Only production graph construction populates this; synthetic graphs in
/// tests use the `Default` empty snapshot, and validation keeps its
/// disk-loading fallback for any node with no recorded entry.
#[derive(Debug, Clone, Default)]
pub(crate) struct PreparedHeadingSnapshot {
    documents: HashMap<PathBuf, PreparedDocumentHeadings>,
    /// The raw heading slugs of each `::toc-linking` reference's target,
    /// keyed by the generated reference's id and target. Copies of one
    /// holder share reference ids, and each copy's directive can resolve to
    /// another target.
    toc_references: HashMap<(String, String), Arc<[String]>>,
}

impl PreparedHeadingSnapshot {
    /// Records the prepared headings for one file-sourced node.
    ///
    /// A file included more than once is prepared once per inclusion, and
    /// each copy can inherit other values: its slugs accumulate, so a heading
    /// any copy composes is a fragment target. `headings` keeps the first
    /// copy's, whose offsets describe that copy's prepared body.
    pub(crate) fn record(&mut self, path: &Path, slugs: Vec<String>, headings: Vec<PreparedHeading>) {
        match self.documents.entry(heading_slug_key(path)) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                let kept = &mut entry.get_mut().slugs;
                for slug in slugs {
                    if !kept.contains(&slug) {
                        kept.push(slug);
                    }
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(PreparedDocumentHeadings { slugs, headings });
            }
        }
    }

    /// Records the raw heading slugs of the target a `::toc-linking`
    /// reference was generated from.
    ///
    /// The generator links raw headings, which the target's own replacement,
    /// interpolation, or conditional blocks can change or remove in its
    /// prepared copy, so the prepared slugs cannot judge these anchors.
    pub(crate) fn record_toc_reference(&mut self, record: &ReferenceRecord, slugs: Arc<[String]>) {
        let target = record.target.raw().unwrap_or_default().to_string();
        self.toc_references.insert((record.id.clone(), target), slugs);
    }

    /// Returns the raw heading slugs `record` must be checked against when it
    /// is a generated `::toc-linking` reference, else `None`: every other
    /// reference, including an authored link to the same target, is checked
    /// against prepared headings.
    pub(crate) fn toc_slugs_for(&self, record: &ReferenceRecord) -> Option<&[String]> {
        if !record.attributes.contains_key("toc_synthesized") {
            return None;
        }
        let key = (record.id.clone(), record.target.raw().unwrap_or_default().to_string());
        self.toc_references.get(&key).map(|slugs| &**slugs)
    }

    /// Returns the recorded prepared slugs for `path`, if any.
    pub(crate) fn slugs_for(&self, path: &Path) -> Option<&[String]> {
        self.documents.get(&heading_slug_key(path)).map(|document| document.slugs.as_slice())
    }

    /// Returns the recorded prepared headings for `path`, if any.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn headings_for(&self, path: &Path) -> Option<&[PreparedHeading]> {
        self.documents.get(&heading_slug_key(path)).map(|document| document.headings.as_slice())
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
        snapshot.record(&path, vec!["child".to_string()], Vec::new());

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
        snapshot.record(&path, vec!["child".to_string()], Vec::new());

        let dotted = dir.path().join(".").join("child.md");
        assert!(snapshot.slugs_for(&dotted).is_some());
    }

    #[test]
    fn headings_keep_final_slugs_and_offsets() {
        let prepared = "intro\n\n# Setup\n\ntext\n\n## Setup\n";
        let headings = prepared_headings_of(prepared);
        assert_eq!(
            headings.iter().map(|heading| (heading.start, heading.slug.as_str())).collect::<Vec<_>>(),
            vec![(7, "setup"), (22, "setup-1")],
        );
        assert_eq!(heading_at(&headings, 0), None);
        assert_eq!(heading_at(&headings, 7).map(|heading| heading.slug.as_str()), Some("setup"));
        assert_eq!(heading_at(&headings, 21).map(|heading| heading.slug.as_str()), Some("setup"));
        assert_eq!(heading_at(&headings, 22).map(|heading| heading.slug.as_str()), Some("setup-1"));
    }

    #[test]
    fn recorded_headings_round_trip_beside_the_slugs() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("child.md");
        std::fs::write(&path, "# Child\n").unwrap();
        let headings = prepared_headings_of("# Child\n");

        let mut snapshot = PreparedHeadingSnapshot::default();
        snapshot.record(&path, vec!["child".to_string()], headings.clone());

        assert_eq!(snapshot.headings_for(&path), Some(headings.as_slice()));
    }
}
