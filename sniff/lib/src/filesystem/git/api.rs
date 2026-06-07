//! Backend-neutral, path-based git queries for CLI consumers.
//!
//! These wrap the git helpers behind plain path arguments and plain return
//! types so `sniff-cli` needs no git backend dependency (no `git2`/`gix`
//! imports). Repository-root resolution and commit file diff use the
//! centralized trusted gix discovery ([`open::trusted_discover`]); the
//! remaining helpers (history, remote, worktree) are still git2-backed
//! internally and are replaced as later migration phases port each helper to
//! gix.

use std::path::{Path, PathBuf};

use super::discovery::{
    DeltaKind, get_commit_by_sha, get_commit_files, get_commits_for_branch, get_commits_for_path,
};
use super::open;
use super::status::detect_merge_conflicts;
use super::types::{CommitInfo, GitHostingProvider};
use crate::{Result, SniffError};

/// Working-directory root of the repository containing `path`.
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
    Ok(open::trusted_discover(path)?.and_then(|repo| repo.workdir().map(Path::to_path_buf)))
}

/// Discover a trusted gix handle for the port-in-progress helpers.
///
/// Trust, permission, I/O, and corruption failures propagate as
/// [`SniffError::Git`]; genuine repository absence is `Ok(None)`.
fn open_gix(path: &Path) -> Result<Option<gix::Repository>> {
    open::trusted_discover(path)
}

/// Discover a git2 handle for the (currently git2-backed) transitional helpers.
///
/// Maps `NotFound` to `Ok(None)` (genuine absence) and every other discovery
/// failure — ownership/trust, permission, I/O, corruption — to
/// [`SniffError::Git`], so transitional remote queries honor the same error
/// contract as the gix opener.
fn open_git2(path: &Path) -> Result<Option<git2::Repository>> {
    match git2::Repository::discover(path) {
        Ok(repo) => Ok(Some(repo)),
        Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(None),
        Err(e) => Err(SniffError::git("discover", e)),
    }
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
    Ok(get_commit_by_sha(&repo, sha))
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
    Ok(get_commit_files(&repo, oid))
}

/// Recent commits touching `path_prefix`, newest first.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn commits_for_path_at(
    path: &Path,
    path_prefix: &str,
    count: usize,
) -> Result<Vec<CommitInfo>> {
    Ok(open_gix(path)?
        .map(|repo| get_commits_for_path(&repo, path_prefix, count))
        .unwrap_or_default())
}

/// Recent commits on `branch`, newest first.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn commits_for_branch_at(path: &Path, branch: &str, count: usize) -> Result<Vec<CommitInfo>> {
    Ok(open_gix(path)?
        .map(|repo| get_commits_for_branch(&repo, branch, count))
        .unwrap_or_default())
}

/// Paths of files in an unmerged (merge-conflict) state, empty when none.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`]; genuine repository absence yields an empty list.
pub fn merge_conflicts_at(path: &Path) -> Result<Vec<PathBuf>> {
    Ok(open_gix(path)?
        .map(|repo| detect_merge_conflicts(&repo))
        .unwrap_or_default())
}

/// Preferred remote URL for the repository: `origin` if present, otherwise the
/// first configured remote with a URL.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn preferred_remote_url(path: &Path) -> Result<Option<String>> {
    Ok(open_git2(path)?.as_ref().and_then(resolve_origin_or_first))
}

/// URL of the named remote, or `None` if the remote is absent or has no URL.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn remote_url(path: &Path, name: &str) -> Result<Option<String>> {
    let Some(repo) = open_git2(path)? else {
        return Ok(None);
    };
    let Ok(remote) = repo.find_remote(name) else {
        return Ok(None);
    };
    Ok(remote.url().map(String::from))
}

/// Browser URL for viewing `sha` on the repository's `origin` provider.
///
/// ## Errors
///
/// Trust/ownership, permission, I/O, and corruption failures surface as
/// [`SniffError::Git`].
pub fn commit_browser_url(path: &Path, sha: &str) -> Result<Option<String>> {
    let Some(repo) = open_git2(path)? else {
        return Ok(None);
    };
    Ok(browser_url_from_repo(&repo, sha))
}

/// Compose the `origin`-provider browser URL for `sha`, or `None` when the
/// repository has no usable `origin` remote/provider.
fn browser_url_from_repo(repo: &git2::Repository, sha: &str) -> Option<String> {
    let remote = repo.find_remote("origin").ok()?;
    let url = remote.url()?;
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

/// `origin` URL if present, otherwise the first configured remote with a URL.
fn resolve_origin_or_first(repo: &git2::Repository) -> Option<String> {
    if let Ok(remote) = repo.find_remote("origin")
        && let Some(url) = remote.url()
    {
        return Some(url.to_string());
    }

    for remote_name in repo.remotes().ok()?.iter().flatten() {
        if let Ok(remote) = repo.find_remote(remote_name)
            && let Some(url) = remote.url()
        {
            return Some(url.to_string());
        }
    }

    None
}
