//! Lazy, bounded sibling-file suggestions for missing file references
//! (Phase 4 of the "real errors" design, §8).
//!
//! When a [`FileReferenceDiagnostic`] reports [`FileRefFailure::NotFound`], the
//! renderer offers a "did you mean …?" hint by listing the *siblings* of the
//! missing path and ranking them against its leaf name with
//! [`crate::catalog::suggest_strings`].
//!
//! ## Why render-time only
//!
//! The listing is done **at render time only** (design Principle 4): the hot
//! evaluation loop never touches the filesystem to compute help text. A
//! diagnostic block is rendered at most once per failure, so the directory read
//! happens once, never per evaluation. The search is also:
//!
//! - **non-recursive** — only the immediate entries of one directory are read,
//!   so a deep tree can't turn a diagnostic into a walk; and
//! - **bounded** — capped at [`MAX_SIBLING_ENTRIES`] so a pathologically large
//!   directory can't stall the diagnostic.
//!
//! ## Nearest existing ancestor
//!
//! A miss is often a *directory* typo, not a filename typo — e.g. a stale dated
//! directory in `features/2026-06-21-…/spec.md`. When the reference's parent
//! directory does not exist, the search walks up to the nearest existing
//! ancestor and lists *that* directory, so the author still sees real neighbors
//! instead of an empty hint.
//!
//! [`FileReferenceDiagnostic`]: super::error::FileReferenceDiagnostic
//! [`FileRefFailure::NotFound`]: super::error::FileRefFailure::NotFound

use std::path::Path;

use crate::catalog::suggest_strings;

/// Maximum directory entries scanned when collecting sibling candidates.
///
/// Bounds the cost of a diagnostic in a pathologically large directory; the
/// design budgets "the first ~1–2k entries" (§8).
pub const MAX_SIBLING_ENTRIES: usize = 2000;

/// Default number of did-you-mean suggestions surfaced for a missing file.
pub const DEFAULT_MAX_SUGGESTIONS: usize = 3;

/// Collect up to [`MAX_SIBLING_ENTRIES`] leaf names that are siblings of
/// `reference` — the immediate entries of its parent directory.
///
/// `reference` is the *expected resolved path* (base directory joined with the
/// raw reference argument), not a bare leaf. When the parent directory does not
/// exist, the search walks up to the nearest existing ancestor and lists that
/// directory instead. The search is non-recursive, and `reference`'s own leaf
/// name is excluded (it does not exist — that is the failure being diagnosed).
///
/// ## Returns
///
/// The sibling leaf names in directory-iteration order (unranked). An empty
/// vector when `reference` has no usable parent, no existing ancestor, or the
/// chosen directory cannot be read.
pub fn collect_sibling_candidates(reference: &Path) -> Vec<String> {
    let Some(parent) = reference.parent() else {
        return Vec::new();
    };
    // A relative leaf (`spec.md`) yields an empty parent; treat that as the
    // current directory so the search still has somewhere real to look.
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    // Walk up to the nearest existing directory so a typo'd directory segment
    // still surfaces neighbors from the closest real ancestor.
    let mut dir = parent;
    while !dir.is_dir() {
        match dir.parent() {
            Some(p) if !p.as_os_str().is_empty() => dir = p,
            _ => return Vec::new(),
        }
    }

    let own_leaf = reference.file_name().map(|n| n.to_string_lossy().into_owned());

    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .take(MAX_SIBLING_ENTRIES)
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|name| own_leaf.as_deref() != Some(name.as_str()))
        .collect()
}

/// Rank the siblings of a missing `reference` against its leaf name and return
/// up to `max` "did you mean" candidate leaf names.
///
/// Combines [`collect_sibling_candidates`] with the
/// [`crate::catalog::suggest_strings`] quality gate, so only genuinely close
/// neighbors are returned. Leaf-name matching is intentional (design §8: "start
/// leaf-name-only"): a missing `spec.md` next to a real `specs.md` matches,
/// while a dated-directory ancestor's children (whose names look nothing like
/// the filename) correctly produce nothing.
///
/// ## Returns
///
/// Up to `max` candidate leaf names, closest first. Empty when `reference` has
/// no leaf name, no siblings exist, or none clears the suggestion quality gate.
pub fn suggest_sibling_files(reference: &Path, max: usize) -> Vec<String> {
    let Some(leaf) = reference.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };
    let candidates = collect_sibling_candidates(reference);
    suggest_strings(&candidates, leaf, max)
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn touch(dir: &Path, name: &str) {
        fs::write(dir.join(name), b"x").expect("write fixture file");
    }

    mod collect {
        use super::*;

        #[test]
        fn lists_siblings_and_excludes_own_leaf() {
            let tmp = TempDir::new().unwrap();
            touch(tmp.path(), "spec.md");
            touch(tmp.path(), "plan.md");
            // The missing reference itself must not appear among its siblings.
            let missing = tmp.path().join("specs.md");

            let mut found = collect_sibling_candidates(&missing);
            found.sort();
            assert_eq!(found, vec!["plan.md".to_string(), "spec.md".to_string()]);
        }

        #[test]
        fn is_non_recursive() {
            let tmp = TempDir::new().unwrap();
            touch(tmp.path(), "spec.md");
            let nested = tmp.path().join("nested");
            fs::create_dir(&nested).unwrap();
            touch(&nested, "buried.md");

            let missing = tmp.path().join("specs.md");
            let found = collect_sibling_candidates(&missing);
            // The immediate subdirectory name appears; its contents do not.
            assert!(found.contains(&"nested".to_string()));
            assert!(!found.contains(&"buried.md".to_string()));
        }

        #[test]
        fn walks_up_to_nearest_existing_ancestor() {
            let tmp = TempDir::new().unwrap();
            let features = tmp.path().join("features");
            fs::create_dir(&features).unwrap();
            fs::create_dir(features.join("2026-06-28-real-errors")).unwrap();

            // Parent dir (`2026-06-21-…`) does not exist; the search must fall
            // back to the existing `features/` ancestor and list its children.
            let missing = features
                .join("2026-06-21-opencode-log-fix")
                .join("spec.md");
            let found = collect_sibling_candidates(&missing);
            assert_eq!(found, vec!["2026-06-28-real-errors".to_string()]);
        }

        #[test]
        fn unreadable_or_rootless_yields_empty() {
            let tmp = TempDir::new().unwrap();
            // Parent exists but contains nothing besides the missing file.
            let missing = tmp.path().join("only.md");
            assert!(collect_sibling_candidates(&missing).is_empty());
        }
    }

    mod suggest {
        use super::*;

        #[test]
        fn suggests_near_filename() {
            let tmp = TempDir::new().unwrap();
            touch(tmp.path(), "spec.md");
            touch(tmp.path(), "readme.md");

            // Author referenced `specs.md`; the real neighbor is `spec.md`.
            let missing = tmp.path().join("specs.md");
            let suggestions = suggest_sibling_files(&missing, DEFAULT_MAX_SUGGESTIONS);
            assert_eq!(suggestions, vec!["spec.md".to_string()]);
        }

        #[test]
        fn dated_directory_ancestor_does_not_suggest_unrelated_files() {
            // Calibration (design §8): when the parent is a stale dated dir, the
            // nearest-ancestor fallback lists *directory* names. Those look
            // nothing like the missing filename, so the gate must produce no
            // confident-but-wrong suggestion.
            let tmp = TempDir::new().unwrap();
            let features = tmp.path().join("features");
            fs::create_dir(&features).unwrap();
            fs::create_dir(features.join("2026-06-28-real-errors")).unwrap();

            let missing = features
                .join("2026-06-21-opencode-log-fix")
                .join("spec.md");
            let suggestions = suggest_sibling_files(&missing, DEFAULT_MAX_SUGGESTIONS);
            assert!(
                suggestions.is_empty(),
                "dated-dir neighbors must not be suggested as filename matches: {suggestions:?}"
            );
        }

        #[test]
        fn no_close_match_yields_empty() {
            let tmp = TempDir::new().unwrap();
            touch(tmp.path(), "completely-different.txt");
            let missing = tmp.path().join("spec.md");
            assert!(suggest_sibling_files(&missing, DEFAULT_MAX_SUGGESTIONS).is_empty());
        }
    }
}
