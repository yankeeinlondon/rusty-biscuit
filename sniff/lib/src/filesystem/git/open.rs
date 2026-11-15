//! Centralized trusted gix repository discovery and opening.
//!
//! Every production repository open rejects untrusted repositories via
//! `bail_if_untrusted`, preserving libgit2's ownership-validation contract so
//! sensitive config cannot silently differ on a hostile checkout. Upward-search
//! exhaustion maps to `Ok(None)` ("not a repository"); trust, permission, I/O,
//! and corruption failures (including a broken `.git` file the upward search
//! skipped) surface as operation-tagged [`SniffError::Git`].

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
/// reported as repository absence — and for a `.git` file between `path` and
/// the repository found that points at no repository.
pub(crate) fn trusted_discover(path: &Path) -> Result<Option<gix::Repository>> {
    use gix::discover::upwards::Error as Up;
    // Counts the attempt, not the success: an exhausted upward search still
    // walked every parent directory, which is the work being measured.
    performance::increment_counter(counters::GIT_DISCOVERIES, 1);
    match gix::ThreadSafeRepository::discover_opts(path, Default::default(), trusted_options()) {
        Ok(repo) => {
            let repo = repo.to_thread_local();
            let found = repo.workdir().unwrap_or_else(|| repo.git_dir()).to_path_buf();
            reject_skipped_gitfile(path, Some(&found))?;
            Ok(Some(repo))
        }
        // Upward-search exhaustion is the only "not a repository" outcome.
        Err(e)
            if matches!(
                e.downcast_any_ref::<Up>(),
                Some(
                    Up::NoGitRepository { .. }
                        | Up::NoGitRepositoryWithinCeiling { .. }
                        | Up::NoGitRepositoryWithinFs { .. }
                )
            ) =>
        {
            reject_skipped_gitfile(path, None)?;
            Ok(None)
        }
        Err(e) => Err(SniffError::git("discover", e)),
    }
}

/// Refuse a discovery that walked past a `.git` *file* below the repository
/// it found (`found`, `None` when the search found none).
///
/// gix's upward search skips a gitfile that does not point at a repository
/// and keeps walking, so it can report an enclosing repository (or none) for
/// a directory that git itself refuses ("not a git repository"). A caller
/// that bounds writes by the discovered root would then widen the boundary
/// past the broken checkout, so this is a discovery failure, as it is in git.
/// A `.git` *directory* gix skipped is not a repository to git either, so it
/// is not refused.
fn reject_skipped_gitfile(path: &Path, found: Option<&Path>) -> Result<()> {
    // Both sides canonical, so a symlinked spelling (macOS `/var` versus
    // `/private/var`) cannot walk past the repository that was found.
    let Ok(start) = path.canonicalize() else {
        return Ok(());
    };
    let found = found.and_then(|found| found.canonicalize().ok());
    for dir in start.ancestors() {
        if found.as_deref() == Some(dir) {
            return Ok(());
        }
        let gitfile = dir.join(".git");
        if std::fs::symlink_metadata(&gitfile).is_ok_and(|meta| meta.is_file()) {
            return Err(SniffError::git(
                "discover",
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("gitfile {} does not point to a valid repository", gitfile.display()),
                ),
            ));
        }
    }
    Ok(())
}

/// Sizes the repository object cache for tree-diff workloads if none is set.
///
/// gix performs best when repeated object lookups are cached. This heuristic
/// sizes the cache proportionally to the index entry count (~10 MB per 10 k
/// tracked files), matching the recommendation in the migration spec.
///
/// An unreadable or malformed index skips this optional optimization; metadata
/// queries remain usable. Status readers independently propagate index failures.
///
/// Callers should invoke this before object-intensive operations (revwalk,
/// diff, ref enumeration) rather than unconditionally at open time.
pub(crate) fn configure_cache(repo: &mut gix::Repository) {
    if let Ok(index) = read_index(repo) {
        let size = repo.compute_object_cache_size_for_tree_diffs(&index);
        repo.object_cache_size_if_unset(size);
    }
}

/// Contain the upstream index decoder's panic on malformed external bytes.
///
/// The closure must only initialize an index-dependent operation: this boundary
/// must not turn unrelated programming errors into repository corruption.
pub(crate) fn guard_index_read<T>(read: impl FnOnce() -> Result<T>) -> Result<T> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(read)).map_err(|_| {
        SniffError::git(
            "index",
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Git index decoder panicked on malformed input",
            ),
        )
    })?
}

/// Missing indexes are valid empty state; malformed indexes are Git errors.
pub(crate) fn read_index(repo: &gix::Repository) -> Result<gix::worktree::Index> {
    guard_index_read(|| repo.index_or_empty().map_err(|error| SniffError::git("index", error)))
}

/// gix builds attribute mappings from the live index even for committed diffs.
pub(crate) fn tree_diff_cache(repo: &gix::Repository) -> Result<gix::diff::blob::Platform> {
    guard_index_read(|| {
        repo.diff_resource_cache_for_tree_diff()
            .map_err(|error| SniffError::git("diff index", error))
    })
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

/// Open a registered worktree target, omitting a stale checkout.
///
/// Linked-worktree registrations can outlive their checkouts until Git prunes
/// them. Staleness is keyed on the checkout's `.git` file, as `git worktree
/// list --porcelain` does when it reports `prunable`: a missing directory or a
/// missing `.git` file is stale and yields `Ok(None)`. A `.git` file that
/// exists but does not open as a repository is corrupt, not stale; trust,
/// permission, I/O, and repository-open failures remain errors.
pub(crate) fn trusted_open_registered_worktree(path: &Path) -> Result<Option<gix::Repository>> {
    if !path
        .join(".git")
        .try_exists()
        .map_err(|error| SniffError::git("open", error))?
    {
        return Ok(None);
    }

    trusted_open(path).map(Some)
}
