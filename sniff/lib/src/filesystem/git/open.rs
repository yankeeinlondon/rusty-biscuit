//! Centralized trusted gix repository discovery and opening.
//!
//! Every production repository open rejects untrusted repositories via
//! `bail_if_untrusted`, preserving libgit2's ownership-validation contract so
//! sensitive config cannot silently differ on a hostile checkout. Upward-search
//! exhaustion maps to `Ok(None)` ("not a repository"); trust, permission, I/O,
//! and corruption failures surface as operation-tagged [`SniffError::Git`].

use std::path::Path;

use crate::{
    Result, SniffError,
    performance::{self, counters},
};

/// Trust mapping that refuses to open untrusted repositories at any trust level.
fn trusted_options() -> gix::sec::trust::Mapping<gix::open::Options> {
    gix::sec::trust::Mapping {
        full: gix::open::Options::default().bail_if_untrusted(true),
        reduced: gix::open::Options::default().bail_if_untrusted(true),
    }
}

/// Discover a trusted repository containing `path`, walking parent directories.
///
/// ## Returns
///
/// `Ok(None)` when the upward search finds no repository.
///
/// ## Errors
///
/// Returns [`SniffError::Git`] tagged `"discover"` for trust/ownership,
/// permission, I/O, and repository-corruption failures — these are *not*
/// reported as repository absence.
pub(crate) fn trusted_discover(path: &Path) -> Result<Option<gix::Repository>> {
    use gix::discover::upwards::Error as Up;
    // Counts the attempt, not the success: an exhausted upward search still
    // walked every parent directory, which is the work being measured.
    performance::increment_counter(counters::GIT_DISCOVERIES, 1);
    match gix::ThreadSafeRepository::discover_opts(path, Default::default(), trusted_options()) {
        Ok(repo) => Ok(Some(repo.to_thread_local())),
        // Upward-search exhaustion is the only "not a repository" outcome.
        Err(gix::discover::Error::Discover(
            Up::NoGitRepository { .. }
            | Up::NoGitRepositoryWithinCeiling { .. }
            | Up::NoGitRepositoryWithinFs { .. },
        )) => Ok(None),
        Err(e) => Err(SniffError::git("discover", e)),
    }
}

/// Sizes the repository object cache for tree-diff workloads if none is set.
///
/// gix performs best when repeated object lookups are cached. This heuristic
/// sizes the cache proportionally to the index entry count (~10 MB per 10 k
/// tracked files), matching the recommendation in the migration spec.
///
/// Callers should invoke this before object-intensive operations (revwalk,
/// diff, ref enumeration) rather than unconditionally at open time.
pub(crate) fn configure_cache(repo: &mut gix::Repository) {
    if let Ok(index) = repo.index_or_empty() {
        let size = repo.compute_object_cache_size_for_tree_diffs(&index);
        repo.object_cache_size_if_unset(size);
    }
}

/// Open a trusted repository at a known `path` without an upward search.
///
/// ## Errors
///
/// Returns [`SniffError::Git`] tagged `"open"` for trust/ownership, permission,
/// I/O, and corruption failures.
#[allow(dead_code)] // Used by worktree/known-path opens ported in later phases.
pub(crate) fn trusted_open(path: &Path) -> Result<gix::Repository> {
    performance::increment_counter(counters::GIT_OPENS, 1);
    let opts = gix::open::Options::default().bail_if_untrusted(true);
    gix::open_opts(path, opts).map_err(|e| SniffError::git("open", e))
}

/// Open a registered worktree target, omitting an absent checkout.
///
/// Linked-worktree registrations can outlive their checkouts until Git prunes
/// them. A target that exists but does not open as a repository is corrupt, not
/// stale; trust, permission, I/O, and repository-open failures remain errors.
pub(crate) fn trusted_open_registered_worktree(path: &Path) -> Result<Option<gix::Repository>> {
    if !path
        .try_exists()
        .map_err(|error| SniffError::git("open", error))?
    {
        return Ok(None);
    }

    trusted_open(path).map(Some)
}
