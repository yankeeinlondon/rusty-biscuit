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

    /// The real target of the file symlink `link` when the pattern that
    /// admits it is bound to the tree and the target lies outside it: the
    /// one rule that makes a listing skip a link.
    pub(crate) fn escaping_target(&self, admitting: usize, link: &Path) -> Option<PathBuf> {
        let boundary = self.patterns[admitting].tree_boundary.as_ref()?;
        let target = canonicalize_simplified(link).ok()?;
        (!PathIdentity::new(&target).starts_with(boundary)).then_some(target)
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
    ///
    /// A directory the walk cannot read fails the listing. With
    /// `first_root_only` it fails only when the files it hides could precede
    /// the root's first match: an unreadable directory whose children are
    /// all deeper than that match cannot change the result.
    pub(crate) fn list(&self, first_root_only: bool) -> Result<GlobListing, GlobReferenceError> {
        let mut listing = GlobListing::default();
        // A file is examined once: the first pass that reaches it is its
        // owner's (nearest-root judgment makes `admitting` agree), so a
        // later root never re-includes a file an earlier root excluded.
        let mut examined: HashSet<PathIdentity> = HashSet::new();
        for root in self.merged_roots() {
            let mut pass: Vec<(Vec<OsString>, PathBuf)> = Vec::new();
            let mut failures: Vec<WalkFailure> = Vec::new();
            for walk in outermost(&root.walks) {
                self.walk(&root, &walk, &mut examined, &mut pass, &mut listing.skipped, &mut failures)?;
            }
            pass.sort_by(|(left, _), (right, _)| {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            });
            let first_depth = pass.first().map(|(relative, _)| relative.len());
            if let Some(failure) = failures.into_iter().find(|failure| {
                !first_root_only || first_depth.is_none_or(|depth| failure.hidden_depth <= depth)
            }) {
                return Err(failure.error);
            }
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
        failures: &mut Vec<WalkFailure>,
    ) -> Result<(), GlobReferenceError> {
        let canonical_walk = match canonicalize_simplified(walk) {
            Ok(canonical) => canonical,
            // A root or literal directory that does not exist (or is a file)
            // holds no matches.
            Err(error) if vanished(&error) => return Ok(()),
            Err(source) => {
                return Err(GlobReferenceError::Io {
                    path: walk.to_path_buf(),
                    source,
                });
            }
        };
        // How many components below the root the unreadable directory `dir`
        // (spelled under `walk`) lies, when a positive pattern could match a
        // file inside it; `None` when nothing it holds could be listed.
        let relevant_depth = |dir: &Path| -> Option<usize> {
            let below_walk = dir.strip_prefix(walk).ok()?;
            let identity = PathIdentity::new(&canonical_walk.join(below_walk));
            let depth = identity.strip_prefix(&root.identity)?.len();
            self.patterns
                .iter()
                .any(|pattern| !pattern.negated && pattern.could_match_below(&identity))
                .then_some(depth)
        };
        // Directory symlinks below the walk directory are never followed, so
        // every directory reached is canonical under `canonical_walk`.
        for entry in WalkDir::new(walk).follow_links(false) {
            let entry = match entry {
                Ok(entry) => entry,
                Err(error) => {
                    let unreadable = error.path().unwrap_or(walk).to_path_buf();
                    let Some(error) = walk_failure(&unreadable, error) else {
                        continue;
                    };
                    // A directory outside the root (reached through a
                    // directory symlink), or deeper than any pattern reaches,
                    // holds nothing the listing could include.
                    if let Some(depth) = relevant_depth(&unreadable) {
                        failures.push(WalkFailure {
                            hidden_depth: depth + 1,
                            error,
                        });
                    }
                    continue;
                }
            };
            let file_type = entry.file_type();
            let is_link = file_type.is_symlink();
            if !(file_type.is_file() || is_link) {
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
            if is_link {
                match link_to_file(entry.path()) {
                    Ok(true) => {}
                    Ok(false) => continue,
                    Err(error) => {
                        failures.push(WalkFailure {
                            hidden_depth: relative.len(),
                            error,
                        });
                        continue;
                    }
                }
            }
            let listed = relative.iter().fold(root.root.clone(), |dir, name| dir.join(name));
            if is_link && let Some(target) = self.escaping_target(admitting, entry.path()) {
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

/// A walk step that failed, and the depth below the root of the shallowest
/// file it may have hidden.
struct WalkFailure {
    hidden_depth: usize,
    error: GlobReferenceError,
}

/// The error a failed walk step reports, or `None` for an entry that
/// vanished (or stopped being a directory) after its parent was listed: the
/// listing then describes the tree as it stood when each directory was read.
/// Any other failure, such as a directory that exists but cannot be read,
/// fails the listing rather than leaving its entries out of a result that
/// would look complete.
fn walk_failure(path: &Path, error: walkdir::Error) -> Option<GlobReferenceError> {
    let source = match error.into_io_error() {
        Some(source) if vanished(&source) => return None,
        Some(source) => source,
        // A filesystem loop; unreachable while directory symlinks are not
        // followed, but never silenced if that changes.
        None => std::io::Error::other("filesystem loop"),
    };
    Some(GlobReferenceError::Io {
        path: path.to_path_buf(),
        source,
    })
}

/// Whether the symlink `link` leads to a regular file. A dangling link (its
/// target, or a directory on the way, is missing) leads to no file; a target
/// that cannot be examined, such as one inside an unreadable directory or a
/// cycle of links, is an error naming the link.
fn link_to_file(link: &Path) -> Result<bool, GlobReferenceError> {
    match std::fs::metadata(link) {
        Ok(meta) => Ok(meta.is_file()),
        Err(source) if vanished(&source) => Ok(false),
        Err(source) => Err(GlobReferenceError::Io {
            path: link.to_path_buf(),
            source,
        }),
    }
}

fn vanished(error: &std::io::Error) -> bool {
    matches!(
        error.kind(),
        std::io::ErrorKind::NotFound | std::io::ErrorKind::NotADirectory
    )
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
