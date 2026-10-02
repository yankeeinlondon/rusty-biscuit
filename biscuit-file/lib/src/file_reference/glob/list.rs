//! Walking a prepared pattern set in native order.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use super::error::GlobReferenceError;
use super::roots::{PreparedPattern, canonical_identity};
use super::{GlobListing, SkippedEntry};
use crate::canonicalize_simplified;
use crate::file_reference::PathIdentity;

/// Every pattern of one reference, resolved against one context.
pub(crate) struct PreparedSet {
    pub patterns: Vec<PreparedPattern>,
}

impl PreparedSet {
    /// The first positive pattern that admits the file, when no negative
    /// pattern rejects it.
    pub(crate) fn admitting(&self, file: &PathIdentity, name: &std::ffi::OsStr) -> Option<usize> {
        let rejected = self
            .patterns
            .iter()
            .filter(|pattern| pattern.negated)
            .any(|pattern| pattern.judge(file, name) == Some(true));
        if rejected {
            return None;
        }
        self.patterns
            .iter()
            .position(|pattern| !pattern.negated && pattern.judge(file, name) == Some(true))
    }

    /// The positive patterns' roots, merged in precedence order: each
    /// pattern's roots in turn, a root already seen through an earlier
    /// pattern keeping its first place. Each carries every walk directory
    /// any pattern starts under it.
    fn merged_roots(&self) -> Vec<MergedRoot> {
        let mut merged: Vec<MergedRoot> = Vec::new();
        for pattern in self.patterns.iter().filter(|pattern| !pattern.negated) {
            for root in &pattern.roots {
                match merged.iter_mut().find(|seen| seen.identity == root.identity) {
                    Some(seen) => seen.walks.push(root.walk.clone()),
                    None => merged.push(MergedRoot {
                        root: root.root.clone(),
                        identity: root.identity.clone(),
                        walks: vec![root.walk.clone()],
                    }),
                }
            }
        }
        merged
    }

    /// The positive patterns' roots in precedence order, as the context
    /// spells them.
    pub(crate) fn roots(&self) -> Vec<PathBuf> {
        self.merged_roots().into_iter().map(|root| root.root).collect()
    }

    /// List every admitted file in native order: root precedence, then
    /// fewest components below the root, then component-wise. With
    /// `first_root_only`, stop after the first root that has a match.
    pub(crate) fn list(&self, first_root_only: bool) -> Result<GlobListing, GlobReferenceError> {
        let mut listing = GlobListing::default();
        // A file is examined once: the first pass that reaches it is its
        // owner's (nearest-root judgment makes `admitting` agree), so a
        // later root never re-includes a file an earlier root excluded.
        let mut examined: HashSet<PathIdentity> = HashSet::new();
        for root in self.merged_roots() {
            let mut pass: Vec<(Vec<OsString>, PathBuf)> = Vec::new();
            for walk in outermost(&root.walks) {
                self.walk(&root, &walk, &mut examined, &mut pass, &mut listing.skipped)?;
            }
            pass.sort_by(|(left, _), (right, _)| {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            });
            listing.matches.extend(pass.into_iter().map(|(_, path)| path));
            if first_root_only && !listing.matches.is_empty() {
                break;
            }
        }
        Ok(listing)
    }

    fn walk(
        &self,
        root: &MergedRoot,
        walk: &Path,
        examined: &mut HashSet<PathIdentity>,
        pass: &mut Vec<(Vec<OsString>, PathBuf)>,
        skipped: &mut Vec<SkippedEntry>,
    ) -> Result<(), GlobReferenceError> {
        let canonical_walk = match canonicalize_simplified(walk) {
            Ok(canonical) => canonical,
            // A root or literal directory that does not exist (or is a file)
            // holds no matches.
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
                ) =>
            {
                return Ok(());
            }
            Err(source) => {
                return Err(GlobReferenceError::Io {
                    path: walk.to_path_buf(),
                    source,
                });
            }
        };
        // Directory symlinks below the walk directory are never followed, so
        // every directory reached is canonical under `canonical_walk`.
        for entry in WalkDir::new(walk).follow_links(false).into_iter().filter_map(Result::ok) {
            let file_type = entry.file_type();
            let is_link = file_type.is_symlink();
            if !(file_type.is_file()
                || is_link && std::fs::metadata(entry.path()).is_ok_and(|meta| meta.is_file()))
            {
                continue;
            }
            let Ok(below_walk) = entry.path().strip_prefix(walk) else {
                continue;
            };
            let file = PathIdentity::new(&canonical_walk.join(below_walk));
            let Some(relative) = file.strip_prefix(&root.identity).map(<[OsString]>::to_vec)
            else {
                // The literal directory reached outside the root through a
                // directory symlink; the root does not own what lies there.
                continue;
            };
            if !examined.insert(file.clone()) {
                continue;
            }
            let Some(admitting) = self.admitting(&file, entry.file_name()) else {
                continue;
            };
            let listed = relative.iter().fold(root.root.clone(), |dir, name| dir.join(name));
            if is_link
                && let Some(boundary) = &self.patterns[admitting].tree_boundary
                && let Ok(target) = canonicalize_simplified(entry.path())
                && !PathIdentity::new(&target).starts_with(boundary)
            {
                skipped.push(SkippedEntry {
                    link: listed,
                    target,
                });
                continue;
            }
            pass.push((relative, listed));
        }
        Ok(())
    }
}

/// One root shared by the positive patterns that search it.
struct MergedRoot {
    root: PathBuf,
    identity: PathIdentity,
    walks: Vec<PathBuf>,
}

/// The walk directories that no other walk directory contains, so a subtree
/// is walked once.
fn outermost(walks: &[PathBuf]) -> Vec<PathBuf> {
    let mut by_depth: Vec<(PathIdentity, &PathBuf)> =
        walks.iter().map(|walk| (canonical_identity(walk), walk)).collect();
    by_depth.sort_by_key(|(identity, _)| identity.components().len());
    let mut kept: Vec<(PathIdentity, PathBuf)> = Vec::new();
    for (identity, walk) in by_depth {
        if kept.iter().all(|(outer, _)| !identity.starts_with(outer)) {
            kept.push((identity, walk.clone()));
        }
    }
    kept.into_iter().map(|(_, walk)| walk).collect()
}
