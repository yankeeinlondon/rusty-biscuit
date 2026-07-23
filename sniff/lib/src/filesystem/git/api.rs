//! Backend-neutral, path-based git queries for CLI consumers.
//!
//! These wrap the git helpers behind plain path arguments and plain return
//! types so `sniff-cli` needs no git backend dependency (no `git2`/`gix`
//! imports). All helpers open through the centralized trusted gix discovery
//! ([`open::trusted_discover`]), so trust/permission/I/O/corruption failures
//! surface distinctly from genuine repository absence.

use std::path::{Path, PathBuf};

use gix::bstr::ByteSlice;

use super::discovery::{
    DeltaKind, PathHistoryOptions, PathHistoryResult, get_commit_by_sha_fallible,
    get_commit_files_fallible, get_commits_for_branch_fallible, get_commits_for_path_fallible,
};
use super::open;
use super::merge_conflicts::merge_conflicts_between;
use super::status::detect_merge_conflicts_fallible;
use super::types::{BranchInfo, CommitInfo, GitHostingProvider};
use crate::Result;

/// Working-directory root of the repository containing `path`.
///
/// The returned path is absolute. gix reports a workdir relative to the
/// discovery path, so a relative `path` (e.g. the default `"."`) would
/// otherwise yield a relative root such as `".."`; discovering from an
/// absolutized path keeps the result absolute without resolving symlinks.
///
/// ## Returns
///
/// `Ok(None)` when `path` is not inside a (trusted) git repository, or the
/// repository is bare (no working directory).
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`crate::SniffError::Git`]; genuine repository absence is `Ok(None)`.
pub fn repo_root(path: &Path) -> Result<Option<PathBuf>> {
    let path = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
    Ok(open::trusted_discover(&path)?.and_then(|repo| repo.workdir().map(Path::to_path_buf)))
}

/// Discover a trusted gix handle for the port-in-progress helpers.
///
/// Trust, permission, I/O, and corruption failures propagate as
/// [`SniffError::Git`]; genuine repository absence is `Ok(None)`.
fn open_gix(path: &Path) -> Result<Option<gix::Repository>> {
    open::trusted_discover(path)
}

/// Metadata for a single commit resolved by (possibly abbreviated) SHA.
///
/// ## Returns
///
/// `Ok(None)` when `path` is not a repository or no commit matches `sha`.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn commit_by_sha_at(path: &Path, sha: &str) -> Result<Option<CommitInfo>> {
    let Some(repo) = open_gix(path)? else {
        return Ok(None);
    };
    get_commit_by_sha_fallible(&repo, sha, None)
}

/// Files changed by the commit with the given full `sha`, paired with the
/// kind of change.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn commit_files_at(path: &Path, sha: &str) -> Result<Vec<(PathBuf, DeltaKind)>> {
    let Some(repo) = open_gix(path)? else {
        return Ok(Vec::new());
    };
    let Ok(oid) = gix::ObjectId::from_hex(sha.as_bytes()) else {
        return Ok(Vec::new());
    };
    get_commit_files_fallible(&repo, oid)
}

/// Recent commits touching `path_prefix`, newest first.
///
/// The walk is bounded by `options`; the returned
/// [`PathHistoryResult`](crate::filesystem::git::PathHistoryResult) reports
/// whether it exhausted history or stopped at its scan limit, so a short result
/// is never mistaken for a complete one.
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::git::{commits_for_path_at, PathHistoryOptions};
/// use std::path::Path;
///
/// let history = commits_for_path_at(Path::new("."), "src/", PathHistoryOptions::new(10))?;
/// if history.limit_reached {
///     eprintln!("scanned {} commits without exhausting history", history.commits_scanned);
/// }
/// # Ok::<(), sniff::SniffError>(())
/// ```
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn commits_for_path_at(
    path: &Path,
    path_prefix: &str,
    options: PathHistoryOptions,
) -> Result<PathHistoryResult> {
    let Some(repo) = open_gix(path)? else {
        return Ok(PathHistoryResult::default());
    };
    get_commits_for_path_fallible(&repo, path_prefix, options, None)
}

/// Recent commits on `branch`, newest first.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn commits_for_branch_at(path: &Path, branch: &str, count: usize) -> Result<Vec<CommitInfo>> {
    let Some(repo) = open_gix(path)? else {
        return Ok(Vec::new());
    };
    get_commits_for_branch_fallible(&repo, branch, count)
}

/// Paths currently in the repository index's unmerged stages, empty when none.
///
/// This observes actual merge, rebase, cherry-pick, or revert state in the live
/// index. Use [`merge_conflicts_with_branch_at`] to predict a committed-tip
/// branch merge without consulting that index. Returned paths are sorted,
/// deduplicated, repository-relative paths with portable separators.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`]; genuine repository absence yields an empty list.
pub fn merge_conflicts_at(path: &Path) -> Result<Vec<PathBuf>> {
    let Some(repo) = open_gix(path)? else {
        return Ok(Vec::new());
    };
    detect_merge_conflicts_fallible(&repo)
}

/// Predicts unresolved paths when `incoming_branch` is merged into the current branch.
///
/// Both sides are captured local commit tips. The analysis ignores the live
/// index and worktree, performs no fetch or command execution, and keeps all
/// synthesized objects in probe-local memory.
///
/// Unlike [`merge_conflicts_at`], this does not observe an in-progress operation
/// or any staged, unstaged, untracked, or already-conflicted worktree state.
/// Applicable external merge drivers, filters, and renormalization are rejected
/// because command-free prediction cannot safely reproduce them.
///
/// `incoming_branch` accepts an exact local branch name with or without one
/// leading `refs/heads/`. Tags, object IDs, remote-tracking refs, abbreviated
/// names, and other revision expressions are not resolved.
///
/// ## Errors
///
/// Returns an error outside a repository, for detached or unborn HEAD, for an
/// invalid or missing local branch, for unrelated or corrupt histories, or
/// when an applicable external merge driver/filter or renormalization setting
/// prevents command-free prediction.
pub fn merge_conflicts_with_branch_at(
    path: &Path,
    incoming_branch: &str,
) -> Result<Vec<PathBuf>> {
    let Some(repo) = open_gix(path)? else {
        return Err(crate::SniffError::NotARepository(path.to_path_buf()));
    };
    let mut head = repo.head().map_err(|error| crate::SniffError::git("head", error))?;
    if head.is_detached() {
        return Err(crate::SniffError::git(
            "merge_current_branch",
            std::io::Error::other("HEAD is detached"),
        ));
    }
    if head.is_unborn() {
        return Err(crate::SniffError::git(
            "merge_current_branch",
            std::io::Error::other("HEAD is unborn"),
        ));
    }
    let current_name = head
        .referent_name()
        .expect("attached born HEAD has a referent")
        .as_bstr();
    if !current_name.starts_with_str("refs/heads/") {
        return Err(crate::SniffError::git(
            "merge_current_branch",
            std::io::Error::other("HEAD is not attached to a local branch"),
        ));
    }
    let ours = head
        .peel_to_commit()
        .map_err(|error| crate::SniffError::git("merge_current_branch", error))?
        .id()
        .detach();

    let short = incoming_branch
        .strip_prefix("refs/heads/")
        .unwrap_or(incoming_branch);
    let full = format!("refs/heads/{short}");
    let full_name = gix::refs::FullName::try_from(full.as_str())
        .map_err(|error| crate::SniffError::git("merge_branch_name", error))?;
    gix::validate::reference::branch_name(full_name.as_bstr())
        .map_err(|error| crate::SniffError::git("merge_branch_name", error))?;
    let theirs = repo
        .find_reference(full_name.as_bstr())
        .map_err(|error| crate::SniffError::git("merge_branch", error))?
        .into_fully_peeled_id()
        .map_err(|error| crate::SniffError::git("merge_branch", error))?
        .detach();

    merge_conflicts_between(&repo, ours, theirs)
}

/// Local branch projection for the repository containing `path`.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`]; genuine repository absence yields `Ok(None)`.
pub fn branches_at(path: &Path, refresh_remotes: bool) -> Result<Option<Vec<BranchInfo>>> {
    let Some(repo) = super::types::GitRepo::discover(path)? else {
        return Ok(None);
    };
    Ok(Some(repo.branch_info(refresh_remotes)?))
}

/// Preferred remote URL for the repository: `origin` if present, otherwise the
/// first configured remote with a URL.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn preferred_remote_url(path: &Path) -> Result<Option<String>> {
    Ok(super::remote_resolver::resolve_remote_at(path, None)?
        .map(|remote| remote.fetch_url))
}

/// [`preferred_remote_url`] against an already-discovered repository.
///
/// Resolves through the same preferred-remote authority as the path-based form,
/// so the two cannot drift apart on which remote counts as preferred.
pub(crate) fn preferred_remote_url_with_repo(repo: &super::GitRepo) -> Option<String> {
    repo.with_cached_gix(|repo| {
        super::remote_resolver::resolve_remote(repo, None)
            .ok()
            .flatten()
            .map(|remote| remote.fetch_url)
    })
}

/// URL of the named remote, or `None` if the remote is absent or has no URL.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn remote_url(path: &Path, name: &str) -> Result<Option<String>> {
    let Some(repo) = open::trusted_discover(path)? else {
        return Ok(None);
    };
    Ok(remote_url_from_config(&repo, name))
}

/// Browser URL for viewing `sha` on the repository's `origin` provider.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn commit_browser_url(path: &Path, sha: &str) -> Result<Option<String>> {
    let Some(repo) = open::trusted_discover(path)? else {
        return Ok(None);
    };
    Ok(remote_url_from_config(&repo, "origin").and_then(|url| browser_url_from_url(&url, sha)))
}

/// Read `remote.<name>.url` straight from config so the exact stored string is
/// returned (rather than gix's reserialized `Url` form).
fn remote_url_from_config(repo: &gix::Repository, name: &str) -> Option<String> {
    repo.config_snapshot()
        .string(format!("remote.{name}.url").as_str())
        .map(|v| v.to_string())
}

/// Compose the `origin`-provider browser URL for `sha`, or `None` when `url`
/// has no recognized provider.
fn browser_url_from_url(url: &str, sha: &str) -> Option<String> {
    let provider = GitHostingProvider::from_url(url);
    let base = provider.browser_base_url()?;

    let owner_repo = if url.contains('@') && url.contains(':') {
        url.split(':')
            .next_back()
            .map(|s| s.trim_end_matches(".git").to_string())
    } else if url.contains("://") {
        let segment = url.split('/').skip(3).collect::<Vec<_>>().join("/");
        Some(segment.trim_end_matches(".git").to_string())
    } else {
        None
    }?;

    Some(format!(
        "{base}/{owner_repo}/{}/{sha}",
        provider.commit_path_segment()
    ))
}
