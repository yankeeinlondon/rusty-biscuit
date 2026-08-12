//! Wiki-link basename index.
//!
//! Maps a document's *basename* (final path segment with a `.md`/`.markdown`
//! extension elided) to every document that shares it, so a bare `[[note]]`
//! wiki link can be resolved by basename. This is **storage only** in Phase 3;
//! the R-8 matching, ranking, and case/NFC rules land in Phase 5.
//!
//! The basename-bucketing algorithm is adapted from the Apache-2.0 `iwe`
//! project's `KeyIndex` (IWE, Dmytro Halichenko,
//! <https://github.com/iwe-org/iwe>) per the AD-1 decision record; the storage
//! here is keyed on DMLS document ids rather than IWE keys.

use std::collections::HashMap;
use std::path::Path;

use super::arena::DocumentId;

/// Basename → documents index for wiki-link resolution.
///
/// Basenames are stored **case-sensitively** (R-8: case-sensitive everywhere);
/// only the `.md`/`.markdown` extension is elided (case-insensitively, since
/// filesystem extension casing varies).
#[derive(Debug, Default)]
pub struct KeyIndex {
    by_basename: HashMap<String, Vec<DocumentId>>,
}

impl KeyIndex {
    /// Builds the index from `(id, path)` pairs.
    pub fn build<'a>(documents: impl Iterator<Item = (DocumentId, &'a Path)>) -> Self {
        let mut index = Self::default();
        for (id, path) in documents {
            if let Some(basename) = basename_of(path) {
                index.by_basename.entry(basename).or_default().push(id);
            }
        }
        // Deterministic bucket order regardless of discovery order.
        for bucket in index.by_basename.values_mut() {
            bucket.sort_unstable();
            bucket.dedup();
        }
        index
    }

    /// Documents sharing `basename` (empty slice when none).
    pub fn documents_for_basename(&self, basename: &str) -> &[DocumentId] {
        self.by_basename
            .get(basename)
            .map_or(&[], Vec::as_slice)
    }

    /// Whether more than one document shares `basename` (an ambiguous wiki
    /// target the Phase 5 ranker must disambiguate).
    pub fn is_ambiguous(&self, basename: &str) -> bool {
        self.documents_for_basename(basename).len() > 1
    }

    /// Number of distinct basenames indexed.
    pub fn len(&self) -> usize {
        self.by_basename.len()
    }

    /// Whether the index is empty.
    pub fn is_empty(&self) -> bool {
        self.by_basename.is_empty()
    }
}

/// The wiki basename for `path`: the final segment with a trailing
/// `.md`/`.markdown` (any case) removed.
///
/// ## Returns
///
/// `None` for a path with no file-name component.
pub fn basename_of(path: &Path) -> Option<String> {
    let name = path.file_name()?.to_str()?;
    for ext in [".markdown", ".md"] {
        if name.len() > ext.len() && name[name.len() - ext.len()..].eq_ignore_ascii_case(ext) {
            return Some(name[..name.len() - ext.len()].to_string());
        }
    }
    Some(name.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_basename_elides_md_extensions() {
        assert_eq!(basename_of(Path::new("a/b/note.md")).as_deref(), Some("note"));
        assert_eq!(
            basename_of(Path::new("a/Guide.MARKDOWN")).as_deref(),
            Some("Guide")
        );
        assert_eq!(basename_of(Path::new("a/plain")).as_deref(), Some("plain"));
    }

    #[test]
    fn test_basename_is_case_sensitive_in_stem() {
        assert_ne!(basename_of(Path::new("Note.md")), basename_of(Path::new("note.md")));
    }

    #[test]
    fn test_build_and_ambiguity() {
        let paths: Vec<(DocumentId, PathBuf)> = vec![
            (DocumentId(0), PathBuf::from("a/note.md")),
            (DocumentId(1), PathBuf::from("b/note.md")),
            (DocumentId(2), PathBuf::from("c/unique.md")),
        ];
        let index = KeyIndex::build(paths.iter().map(|(id, p)| (*id, p.as_path())));
        assert_eq!(
            index.documents_for_basename("note"),
            &[DocumentId(0), DocumentId(1)]
        );
        assert!(index.is_ambiguous("note"));
        assert!(!index.is_ambiguous("unique"));
        assert!(index.documents_for_basename("missing").is_empty());
        assert_eq!(index.len(), 2);
    }
}
