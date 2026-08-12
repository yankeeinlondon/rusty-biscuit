//! Lazy, bounded sibling-file suggestions for missing file references
//! (Phase 4 of the "real errors" design, §8).
//!
//! When a [`FileReferenceDiagnostic`] reports [`FileRefFailure::NotFound`], the
//! renderer offers a "did you mean …?" hint by ranking *siblings* of the
//! missing path against the missing component with [`crate::catalog::levenshtein`].
//!
//! ## Two miss shapes
//!
//! The suggestion logic branches on which part of the path is wrong:
//!
//! - **Leaf-name miss** — the reference's direct parent directory exists; only
//!   the filename is wrong (e.g. `specs.md` for `spec.md`). Sibling **leaf
//!   names** are ranked against the missing leaf with
//!   [`crate::catalog::suggest_strings`] and its `max(2, chars/3)` quality gate.
//!   Returned suggestions are bare leaf names.
//! - **Stale-directory miss** — the reference's direct parent does not exist;
//!   the search walks up to the nearest existing ancestor (e.g. `features/`)
//!   and ranks **sibling directories** of that ancestor against the missing
//!   directory segment. A sibling is a candidate only when it *contains a file
//!   matching the reference's leaf name* (e.g.
//!   `<ancestor>/2026-06-28-real-errors/spec.md`); that leaf-existence check is
//!   the quality signal, so the strict filename gate does not apply (dated
//!   folder names like `2026-06-21-…` and `2026-06-28-…` share too little with
//!   each other to clear `max(2, chars/3)`). Returned suggestions are relative
//!   paths of the form `sibling_dir/leaf`.
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
//! [`FileReferenceDiagnostic`]: super::error::FileReferenceDiagnostic
//! [`FileRefFailure::NotFound`]: super::error::FileRefFailure::NotFound

use std::path::{Path, PathBuf};

use crate::catalog::{levenshtein, suggest_strings};

/// Maximum directory entries scanned when collecting sibling candidates.
///
/// Bounds the cost of a diagnostic in a pathologically large directory; the
/// design budgets "the first ~1–2k entries" (§8).
pub const MAX_SIBLING_ENTRIES: usize = 2000;

/// Default number of did-you-mean suggestions surfaced for a missing file.
pub const DEFAULT_MAX_SUGGESTIONS: usize = 3;

/// Collect up to [`MAX_SIBLING_ENTRIES`] leaf names that are siblings of
/// `reference` — the immediate entries of the nearest existing directory on
/// its parent chain.
///
/// `reference` is the *expected resolved path* (base directory joined with the
/// raw reference argument), not a bare leaf. When the direct parent directory
/// does not exist, the search walks up to the nearest existing ancestor and
/// lists that directory's entries instead, so a stale-directory reference
/// still surfaces real neighbors. The search is non-recursive, and
/// `reference`'s own leaf name is excluded (it does not exist — that is the
/// failure being diagnosed).
///
/// ## Returns
///
/// The sibling leaf names in directory-iteration order (unranked). An empty
/// vector when `reference` has no usable parent, no existing ancestor, or the
/// chosen directory cannot be read.
///
/// ## Notes
///
/// This helper lists *all* immediate entries (files and directories) of the
/// nearest existing ancestor; it does not distinguish the leaf-name arm from
/// the stale-directory arm. The arm-specific ranking lives in
/// [`suggest_sibling_files`], which uses the parent-existed flag from
/// [`nearest_existing_ancestor`] to decide which entries to rank and how.
pub fn collect_sibling_candidates(reference: &Path) -> Vec<String> {
    let Some((dir, _parent_existed)) = nearest_existing_ancestor(reference) else {
        return Vec::new();
    };
    let own_leaf = reference.file_name().map(|n| n.to_string_lossy().into_owned());

    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .take(MAX_SIBLING_ENTRIES)
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .filter(|name| own_leaf.as_deref() != Some(name.as_str()))
        .collect()
}

/// Rank the siblings of a missing `reference` and return up to `max` "did you
/// mean" candidate strings.
///
/// Branches on the miss shape (see the module docs):
///
/// - When the reference's **direct parent exists**, the miss is a filename
///   typo: sibling **leaf names** are ranked against the missing leaf with
///   [`crate::catalog::suggest_strings`] and its `max(2, chars/3)` quality
///   gate. Each returned suggestion is a bare leaf name (e.g. `spec.md`).
/// - When the reference's **direct parent does not exist** (a stale-directory
///   reference), the search walks up to the nearest existing ancestor and
///   ranks **sibling directories** of that ancestor against the missing
///   directory segment. A sibling is a candidate only when it contains a file
///   matching the reference's leaf name (e.g.
///   `<ancestor>/2026-06-28-real-errors/spec.md`); the leaf-existence check is
///   the quality signal, so no strict filename gate applies. Each returned
///   suggestion is a relative path of the form `sibling_dir/leaf` (e.g.
///   `2026-06-28-real-errors/spec.md`).
///
/// ## Returns
///
/// Up to `max` candidate strings, closest first. Empty when `reference` has
/// no leaf name, no siblings exist, or none clears the matching quality gate
/// for the fired arm.
///
/// ## Notes
///
/// The stale-directory arm ranks purely by [`levenshtein`] distance with no
/// distance threshold: the leaf-existence gate is itself a strong quality
/// signal (a sibling directory that genuinely holds `spec.md` is almost
/// certainly relevant), and dated-folder names routinely exceed the strict
/// `max(2, chars/3)` gate even for the correct sibling. The list is bounded
/// by `max` and ordered closest-first to keep noise off the rendered report.
pub fn suggest_sibling_files(reference: &Path, max: usize) -> Vec<String> {
    if max == 0 {
        return Vec::new();
    }
    let Some(leaf) = reference.file_name().and_then(|n| n.to_str()) else {
        return Vec::new();
    };

    let Some((ancestor, parent_existed)) = nearest_existing_ancestor(reference) else {
        return Vec::new();
    };

    if parent_existed {
        // Leaf-name arm: the direct parent exists, so rank immediate leaf
        // neighbors against the missing leaf with the strict filename gate.
        let candidates = collect_sibling_candidates(reference);
        return suggest_strings(&candidates, leaf, max)
            .into_iter()
            .map(str::to_owned)
            .collect();
    }

    // Stale-directory arm: the direct parent did not exist; the search walked
    // up to `ancestor`. Rank sibling directories of `ancestor` whose name is
    // close to the missing directory segment AND that contain the leaf file.
    // Leaf-existence is the quality signal — a sibling dir that genuinely
    // holds `spec.md` is almost certainly what the author meant even when the
    // directory names share only a short common prefix (e.g. dated folders).
    let Some(missing_segment) = missing_directory_segment(reference, &ancestor) else {
        return Vec::new();
    };

    let Ok(entries) = std::fs::read_dir(&ancestor) else {
        return Vec::new();
    };

    let query = missing_segment.to_lowercase();
    let mut scored: Vec<(usize, String)> = Vec::new();
    for entry in entries.flatten().take(MAX_SIBLING_ENTRIES) {
        let Ok(ft) = entry.file_type() else {
            continue;
        };
        if !ft.is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        // Leaf-existence gate — the strong quality signal for this arm.
        if !ancestor.join(&name).join(leaf).is_file() {
            continue;
        }
        let distance = levenshtein(&query, &name.to_lowercase());
        scored.push((distance, name));
    }

    scored.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    scored
        .into_iter()
        .take(max)
        .map(|(_, name)| format!("{name}/{leaf}"))
        .collect()
}

/// Resolve the nearest existing directory on `reference`'s parent chain and
/// report whether the direct parent was already that directory (no walk-up).
///
/// Returns `None` when `reference` has no usable parent or no existing
/// ancestor is reachable by walking up.
///
/// The second tuple element is `true` when the reference's *direct* parent
/// existed (the leaf-name arm); `false` when at least one walk-up step was
/// required (the stale-directory arm).
fn nearest_existing_ancestor(reference: &Path) -> Option<(PathBuf, bool)> {
    let parent = reference.parent()?;
    // A relative leaf (`spec.md`) yields an empty parent; treat that as the
    // current directory so the search still has somewhere real to look.
    let parent = if parent.as_os_str().is_empty() {
        Path::new(".")
    } else {
        parent
    };

    let mut dir = parent.to_path_buf();
    let parent_existed = dir.is_dir();
    while !dir.is_dir() {
        match dir.parent() {
            Some(p) if !p.as_os_str().is_empty() => {
                dir = p.to_path_buf();
            }
            _ => return None,
        }
    }
    Some((dir, parent_existed))
}

/// The first directory segment of `reference` that lies below `ancestor` —
/// the component a stale-directory suggestion should be ranked against.
///
/// For `features/2026-06-21-opencode-log-fix/spec.md` against ancestor
/// `features/`, this is `2026-06-21-opencode-log-fix`. Returns `None` when
/// `reference` does not pass below `ancestor` or has no such component.
fn missing_directory_segment(reference: &Path, ancestor: &Path) -> Option<String> {
    let rel = reference.strip_prefix(ancestor).ok()?;
    // `rel` looks like `missing_dir/.../leaf`. The first component is the
    // segment we rank sibling directory names against; deeper missing
    // segments are ignored (the leaf-existence check is against
    // `<ancestor>/<sibling>/<leaf>`, so a multi-segment missing path degrades
    // gracefully rather than fabricating matches).
    rel.iter()
        .next()
        .map(|c| c.to_string_lossy().into_owned())
        .filter(|s| !s.is_empty())
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

        // The motivating real-errors case: a stale dated directory in the
        // reference path. The direct parent (`2026-06-21-opencode-log-fix`)
        // does not exist, so the stale-directory arm fires, walks up to
        // `features/`, enumerates sibling directories, requires the leaf
        // (`spec.md`) to live inside a candidate, and ranks by directory-name
        // similarity. The real sibling (`2026-06-28-real-errors`) holds the
        // leaf, so it is suggested as a relative path.
        #[test]
        fn stale_directory_ancestor_suggests_real_sibling_path() {
            let tmp = TempDir::new().unwrap();
            let features = tmp.path().join("features");
            fs::create_dir(&features).unwrap();
            let real = features.join("2026-06-28-real-errors");
            fs::create_dir(&real).unwrap();
            touch(&real, "spec.md");

            let missing = features
                .join("2026-06-21-opencode-log-fix")
                .join("spec.md");
            let suggestions = suggest_sibling_files(&missing, DEFAULT_MAX_SUGGESTIONS);
            assert_eq!(
                suggestions,
                vec!["2026-06-28-real-errors/spec.md".to_string()],
                "stale-dated-directory case must surface the real sibling path that \
                 contains the leaf file: {suggestions:?}"
            );
        }

        // Calibration: leaf-existence inside a sibling directory is the
        // quality signal for the stale-directory arm. A sibling whose name is
        // one edit from the missing segment but does NOT contain the leaf must
        // not be suggested — otherwise the arm would fire on any directory
        // with a coincidentally similar name and produce noise.
        #[test]
        fn stale_directory_arm_requires_leaf_inside_sibling() {
            let tmp = TempDir::new().unwrap();
            let features = tmp.path().join("features");
            fs::create_dir(&features).unwrap();
            // Sibling dir whose name is one edit from the missing segment, but
            // with NO `spec.md` inside.
            fs::create_dir(features.join("2026-06-21-opencode-log-fox")).unwrap();

            let missing = features
                .join("2026-06-21-opencode-log-fix")
                .join("spec.md");
            let suggestions = suggest_sibling_files(&missing, DEFAULT_MAX_SUGGESTIONS);
            assert!(
                suggestions.is_empty(),
                "sibling without the leaf file must not be suggested: {suggestions:?}"
            );
        }

        // Bounded / non-recursive: the stale-directory arm reads only direct
        // sibling directories of the nearest existing ancestor. A nested dir
        // that also contains the leaf must NOT be considered (it is not a
        // direct sibling).
        #[test]
        fn stale_directory_arm_is_non_recursive() {
            let tmp = TempDir::new().unwrap();
            let features = tmp.path().join("features");
            fs::create_dir(&features).unwrap();
            let real = features.join("2026-06-28-real-errors");
            fs::create_dir(&real).unwrap();
            touch(&real, "spec.md");

            // A nested directory under `features/` also contains `spec.md`,
            // but it is not a direct sibling dir of the nearest ancestor.
            let nested = features.join("nested").join("deeper-dir");
            fs::create_dir_all(&nested).unwrap();
            touch(&nested, "spec.md");

            let missing = features
                .join("2026-06-21-opencode-log-fix")
                .join("spec.md");
            let suggestions = suggest_sibling_files(&missing, DEFAULT_MAX_SUGGESTIONS);
            // Only the direct sibling dir that holds the leaf is suggested.
            assert_eq!(
                suggestions,
                vec!["2026-06-28-real-errors/spec.md".to_string()],
                "stale-directory arm must not recurse into nested subdirs: {suggestions:?}"
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

    mod helpers {
        use super::*;

        #[test]
        fn nearest_existing_ancestor_marks_leaf_name_arm() {
            let tmp = TempDir::new().unwrap();
            touch(tmp.path(), "spec.md");

            // Direct parent (tmp) exists → leaf-name arm.
            let missing = tmp.path().join("specs.md");
            let (dir, parent_existed) = nearest_existing_ancestor(&missing).unwrap();
            assert_eq!(dir, tmp.path());
            assert!(parent_existed);
        }

        #[test]
        fn nearest_existing_ancestor_marks_stale_directory_arm() {
            let tmp = TempDir::new().unwrap();
            let features = tmp.path().join("features");
            fs::create_dir(&features).unwrap();

            // Direct parent (`2026-06-21-…`) does not exist → stale-directory
            // arm with the ancestor walked-up to `features/`.
            let missing = features
                .join("2026-06-21-opencode-log-fix")
                .join("spec.md");
            let (dir, parent_existed) = nearest_existing_ancestor(&missing).unwrap();
            assert_eq!(dir, features);
            assert!(!parent_existed);
        }

        #[test]
        fn missing_directory_segment_strips_ancestor_and_leaf() {
            let features = Path::new("/repo/features");
            let reference = features.join("2026-06-21-opencode-log-fix").join("spec.md");
            assert_eq!(
                missing_directory_segment(&reference, features),
                Some("2026-06-21-opencode-log-fix".to_string())
            );
        }
    }
}
