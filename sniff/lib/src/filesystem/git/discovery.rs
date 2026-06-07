//! Commit and ref discovery helpers.
//!
//! This module groups commit lookup, recent-commit walks, ref decoration,
//! base-branch resolution, and the `DeltaKind` enum.

use chrono::DateTime;
use gix::bstr::ByteSlice;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};

use crate::Result;
use crate::request::GitRequest;

use super::types::*;

pub fn detect_git(path: &Path, deep: bool, commit_count: usize) -> Result<Option<GitInfo>> {
    match GitRepo::discover(path)? {
        Some(handle) => handle.detect_full(deep, commit_count).map(Some),
        None => Ok(None),
    }
}

/// Detect git information for a path according to the given request.
#[instrument(skip_all, fields(path = %path.display()))]
pub fn detect_git_with_request(path: &Path, request: &GitRequest) -> Result<Option<GitInfo>> {
    match GitRepo::discover(path)? {
        Some(git) => Ok(Some(git.detect_with_request(request)?)),
        None => Ok(None),
    }
}

/// Collects all refs (branches, remote tracking, tags) pointing to each commit.
///
/// Returns a HashMap from commit OID to a vector of ref decorations.
pub(crate) fn collect_ref_decorations(
    repo: &gix::Repository,
) -> HashMap<gix::ObjectId, Vec<RefDecoration>> {
    let mut decorations: HashMap<gix::ObjectId, Vec<RefDecoration>> = HashMap::new();

    // Short name of the branch HEAD points to (None when HEAD is detached), so
    // the active branch can be marked.
    let head_target: Option<String> = repo
        .head_name()
        .map_err(|e| {
            debug!(error = %e, "could not read HEAD for decorations");
            e
        })
        .ok()
        .flatten()
        .map(|full| full.shorten().to_string());

    let Ok(platform) = repo.references() else {
        return decorations;
    };
    let Ok(iter) = platform.all() else {
        return decorations;
    };

    for reference in iter.flatten() {
        let full_name = reference.name().as_bstr().to_string();

        // Peel through annotated tags to the commit the ref ultimately names.
        let Ok(id) = reference.into_fully_peeled_id() else {
            continue;
        };
        let oid = id.detach();

        // Determine ref kind and display name
        let (kind, display_name) = if let Some(branch) = full_name.strip_prefix("refs/heads/") {
            (RefKind::LocalBranch, branch.to_string())
        } else if let Some(remote) = full_name.strip_prefix("refs/remotes/") {
            (RefKind::RemoteBranch, remote.to_string())
        } else if let Some(tag) = full_name.strip_prefix("refs/tags/") {
            (RefKind::Tag, tag.to_string())
        } else {
            continue; // Skip other refs (notes, stash, etc.)
        };

        // Check if this is the HEAD branch
        let is_head = kind == RefKind::LocalBranch
            && head_target.as_ref().is_some_and(|h| h == &display_name);

        let decoration = RefDecoration {
            name: display_name,
            kind,
            is_head,
        };

        decorations.entry(oid).or_default().push(decoration);
    }

    // Sort decorations: HEAD branch first, then local branches, remote branches, tags
    for refs in decorations.values_mut() {
        refs.sort_by(|a, b| {
            // HEAD branch comes first
            if a.is_head != b.is_head {
                return b.is_head.cmp(&a.is_head);
            }
            // Then by kind: LocalBranch < RemoteBranch < Tag
            match (a.kind, b.kind) {
                (RefKind::LocalBranch, RefKind::LocalBranch) => a.name.cmp(&b.name),
                (RefKind::LocalBranch, _) => std::cmp::Ordering::Less,
                (_, RefKind::LocalBranch) => std::cmp::Ordering::Greater,
                (RefKind::RemoteBranch, RefKind::RemoteBranch) => a.name.cmp(&b.name),
                (RefKind::RemoteBranch, _) => std::cmp::Ordering::Less,
                (_, RefKind::RemoteBranch) => std::cmp::Ordering::Greater,
                (RefKind::Tag, RefKind::Tag) => a.name.cmp(&b.name),
            }
        });
    }

    decorations
}

/// Gets the last N commits from HEAD using a gix revwalk.
pub(crate) fn get_recent_commits(repo: &gix::Repository, count: usize) -> Vec<CommitInfo> {
    get_recent_commits_with_decorations(repo, count, None)
}

/// Gets the last N commits from HEAD using a gix revwalk, with optional
/// pre-computed ref decorations.
pub(crate) fn get_recent_commits_with_decorations(
    repo: &gix::Repository,
    count: usize,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(head) = repo.head_id() else {
        return commits;
    };

    let Ok(walk) = repo
        .rev_walk(Some(head))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .all()
    else {
        return commits;
    };

    // Collect ref decorations once for all commits (if not provided).
    let cached = ref_decorations.cloned();
    let decorations = cached.unwrap_or_else(|| collect_ref_decorations(repo));

    for info_result in walk.take(count) {
        let Ok(info) = info_result else {
            continue;
        };
        let Ok(commit) = info.object() else {
            continue;
        };

        let refs = decorations.get(&info.id).cloned().unwrap_or_default();

        let Ok(author) = commit.author() else {
            continue;
        };
        let Ok(time) = commit.time() else {
            continue;
        };
        let Ok(message) = commit.message_raw() else {
            continue;
        };

        commits.push(CommitInfo {
            sha: info.id.to_string(),
            message: String::from_utf8_lossy(message.trim()).to_string(),
            author: author.name.to_string(),
            timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    commits
}

/// Resolves the base branch name and its commit OID for ahead/behind calculations.
///
/// When the repo is a worktree, finds the base repo's current branch. Otherwise
/// uses the current HEAD branch. Falls back to "main" or "master" if HEAD is
/// detached or unavailable.
pub(crate) fn resolve_base_branch(
    repo: &gix::Repository,
) -> crate::Result<(String, Option<gix::ObjectId>)> {
    // For a linked worktree, the base branch is the MAIN worktree's HEAD, found
    // via the shared common dir; otherwise it is this repo's own HEAD.
    let base_repo = if repo.git_dir() != repo.common_dir() {
        Some(super::open::trusted_open(repo.common_dir())?)
    } else {
        None
    };
    let effective = base_repo.as_ref().unwrap_or(repo);

    // Try the effective repo's current HEAD branch.
    if let Ok(Some(name)) = effective.head_name() {
        let branch = name.shorten().to_string();
        let oid = effective.head_id().ok().map(|id| id.detach());
        return Ok((branch, oid));
    }

    // Fallback: try "main", then "master".
    for candidate in ["main", "master"] {
        if let Ok(reference) = repo.find_reference(&format!("refs/heads/{candidate}")) {
            let oid = reference.into_fully_peeled_id().ok().map(|id| id.detach());
            return Ok((candidate.to_string(), oid));
        }
    }

    Ok(("main".to_string(), None))
}

/// Kind of change a file underwent in a commit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeltaKind {
    /// File was added.
    Added,
    /// File was modified.
    Modified,
    /// File was deleted.
    Deleted,
    /// File was renamed.
    Renamed,
    /// File was copied.
    Copied,
}

impl std::fmt::Display for DeltaKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Added => write!(f, "added"),
            Self::Modified => write!(f, "modified"),
            Self::Deleted => write!(f, "deleted"),
            Self::Renamed => write!(f, "renamed"),
            Self::Copied => write!(f, "copied"),
        }
    }
}

impl DeltaKind {
    /// Convert an attached gix `Change` to a `DeltaKind` without allocating.
    fn from_gix_change_attached(change: &gix::object::tree::diff::Change<'_, '_, '_>) -> Self {
        match change {
            gix::object::tree::diff::Change::Addition { .. } => Self::Added,
            gix::object::tree::diff::Change::Deletion { .. } => Self::Deleted,
            gix::object::tree::diff::Change::Modification { .. } => Self::Modified,
            // Rename tracking is disabled; treat any rewrite as a modification.
            gix::object::tree::diff::Change::Rewrite { .. } => Self::Modified,
        }
    }
}

/// Look up a single commit by full or abbreviated SHA.
///
/// Uses `repo.rev_parse_single()` to resolve abbreviated or full SHA strings,
/// then peels to a commit and builds a `CommitInfo` with ref decorations.
///
/// Returns `None` if the SHA doesn't resolve to a valid commit.
pub fn get_commit_by_sha(repo: &gix::Repository, sha_prefix: &str) -> Option<CommitInfo> {
    get_commit_by_sha_with_decorations(repo, sha_prefix, None)
}

/// Look up a single commit by SHA with optional pre-computed ref decorations.
pub(crate) fn get_commit_by_sha_with_decorations(
    repo: &gix::Repository,
    sha_prefix: &str,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Option<CommitInfo> {
    let id = repo
        .rev_parse_single(sha_prefix)
        .map_err(|e| {
            debug!(sha = sha_prefix, error = %e, "could not resolve SHA");
            e
        })
        .ok()?;
    let commit = id
        .object()
        .map_err(|e| {
            debug!(sha = sha_prefix, error = %e, "could not resolve object");
            e
        })
        .ok()?
        .into_commit();

    let decorations = ref_decorations
        .cloned()
        .unwrap_or_else(|| collect_ref_decorations(repo));
    let oid = id.detach();
    let refs = decorations.get(&oid).cloned().unwrap_or_default();

    let author = commit.author().ok()?;
    let time = commit.time().ok()?;
    let message = commit.message_raw().ok()?;
    Some(CommitInfo {
        sha: oid.to_string(),
        message: String::from_utf8_lossy(message.trim()).to_string(),
        author: author.name.to_string(),
        timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
        remotes: None,
        refs,
    })
}

/// Convert a byte path from `gix` to a `PathBuf` using an explicit lossy
/// UTF-8 conversion at the public string boundary.
fn lossy_path(bytes: &gix::bstr::BStr) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes.as_ref()).as_ref())
}

/// Get the list of files changed by a specific commit.
///
/// Computes a diff between the commit's tree and its first parent's tree.
/// For the initial commit (no parent), diffs against an empty tree.
/// Rename tracking is disabled so renames surface as separate delete/add
/// pairs — matching the existing output contract.
///
/// Returns path-ordered `(relative_path, DeltaKind)` pairs.
pub fn get_commit_files(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
) -> Vec<(PathBuf, DeltaKind)> {
    let mut cache = match repo.diff_resource_cache_for_tree_diff() {
        Ok(c) => c,
        Err(e) => {
            debug!(%commit_id, error = %e, "could not create diff resource cache");
            return Vec::new();
        }
    };
    get_commit_files_with_cache(repo, commit_id, &mut cache)
}

/// Like [`get_commit_files`], but reuses an existing diff resource cache.
///
/// Callers that diff many commits in a loop should create one cache with
/// [`Repository::diff_resource_cache_for_tree_diff`] and pass it to this
/// function for each commit. The cache should be cleared periodically
/// (e.g., with [`gix::diff::blob::Platform::clear_resource_cache`]) to
/// avoid unbounded growth when walking large histories.
pub fn get_commit_files_with_cache(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
    cache: &mut gix::diff::blob::Platform,
) -> Vec<(PathBuf, DeltaKind)> {
    let commit = match repo.find_object(commit_id) {
        Ok(o) => match o.try_into_commit() {
            Ok(c) => c,
            Err(e) => {
                debug!(%commit_id, error = %e, "object is not a commit");
                return Vec::new();
            }
        },
        Err(e) => {
            debug!(%commit_id, error = %e, "could not find commit");
            return Vec::new();
        }
    };
    let tree = match commit.tree() {
        Ok(t) => t,
        Err(e) => {
            debug!(%commit_id, error = %e, "could not get commit tree");
            return Vec::new();
        }
    };

    let parent_tree = commit.parent_ids().next().and_then(|parent_id| {
        match repo.find_object(parent_id.detach()) {
            Ok(o) => match o.try_into_commit() {
                Ok(parent_commit) => parent_commit.tree().ok(),
                Err(e) => {
                    debug!(%commit_id, error = %e, "parent object is not a commit");
                    None
                }
            },
            Err(e) => {
                debug!(%commit_id, error = %e, "could not find parent object");
                None
            }
        }
    });

    let empty_tree = repo.empty_tree();
    let old_tree = parent_tree.as_ref().unwrap_or(&empty_tree);

    let mut platform = match old_tree.changes() {
        Ok(p) => p,
        Err(e) => {
            debug!(%commit_id, error = %e, "could not create tree diff platform");
            return Vec::new();
        }
    };

    platform.options(|opts| {
        opts.track_path().track_rewrites(None);
    });

    let mut result: Vec<(PathBuf, DeltaKind)> = Vec::new();

    if let Err(e) = platform.for_each_to_obtain_tree_with_cache(
        &tree,
        cache,
        |change| -> std::result::Result<std::ops::ControlFlow<()>, std::convert::Infallible> {
            if change.entry_mode().is_tree() {
                return Ok(std::ops::ControlFlow::Continue(()));
            }
            let path = lossy_path(change.location());
            if path.as_os_str().is_empty() {
                return Ok(std::ops::ControlFlow::Continue(()));
            }
            result.push((path, DeltaKind::from_gix_change_attached(&change)));
            Ok(std::ops::ControlFlow::Continue(()))
        },
    ) {
        debug!(%commit_id, error = %e, "tree diff with cache failed");
    }

    // gix diff yields path-ordered results in the common case, but
    // explicit sort keeps the contract regardless of internal ordering.
    result.sort_by(|a, b| a.0.cmp(&b.0));
    result
}

/// Returns `true` if the commit `oid` touches any file whose path starts with
/// `path_prefix`.
fn commit_touches_path(
    repo: &gix::Repository,
    cache: &mut gix::diff::blob::Platform,
    oid: gix::ObjectId,
    path_prefix: &str,
) -> bool {
    let files = get_commit_files_with_cache(repo, oid, cache);
    files
        .iter()
        .any(|(p, _)| p.to_string_lossy().starts_with(path_prefix))
}

/// Get recent commits that touch files under a specific path prefix.
///
/// Walks the commit history from HEAD, computes diffs for each commit,
/// and includes commits where at least one changed file starts with `path_prefix`.
///
/// Ref decorations are collected once and reused for all matching commits.
pub fn get_commits_for_path(
    repo: &gix::Repository,
    path_prefix: &str,
    count: usize,
) -> Vec<CommitInfo> {
    get_commits_for_path_with_decorations(repo, path_prefix, count, None)
}

/// Get recent commits for a path with optional pre-computed ref decorations.
pub(crate) fn get_commits_for_path_with_decorations(
    repo: &gix::Repository,
    path_prefix: &str,
    count: usize,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(head) = repo.head_id() else {
        return commits;
    };

    let Ok(walk) = repo
        .rev_walk(Some(head))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .all()
    else {
        return commits;
    };

    let cached = ref_decorations.cloned();
    let decorations = cached.unwrap_or_else(|| collect_ref_decorations(repo));

    let mut diff_cache = match repo.diff_resource_cache_for_tree_diff() {
        Ok(c) => c,
        Err(_) => return commits,
    };

    for info_result in walk {
        if commits.len() >= count {
            break;
        }
        let Ok(info) = info_result else {
            continue;
        };

        if !commit_touches_path(repo, &mut diff_cache, info.id, path_prefix) {
            continue;
        }

        let Ok(commit) = info.object() else {
            continue;
        };

        let refs = decorations.get(&info.id).cloned().unwrap_or_default();

        let Ok(author) = commit.author() else {
            continue;
        };
        let Ok(time) = commit.time() else {
            continue;
        };
        let Ok(message) = commit.message_raw() else {
            continue;
        };

        commits.push(CommitInfo {
            sha: info.id.to_string(),
            message: String::from_utf8_lossy(message.trim()).to_string(),
            author: author.name.to_string(),
            timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    commits
}

/// Get the last N commits walked from a named branch's tip.
///
/// Looks up `branch_name` as a local branch ref first, then falls back to
/// `rev_parse_single` so callers may pass a remote-tracking name (e.g.
/// `origin/main`) or any other ref-like specifier. Returns an empty vector
/// when the branch cannot be resolved.
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::git::get_commits_for_branch;
///
/// let repo = gix::open(".").unwrap();
/// for commit in get_commits_for_branch(&repo, "main", 10) {
///     println!("{} {}", &commit.sha[..7], commit.message);
/// }
/// ```
pub fn get_commits_for_branch(
    repo: &gix::Repository,
    branch_name: &str,
    count: usize,
) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let start_oid = repo
        .find_reference(&format!("refs/heads/{branch_name}"))
        .ok()
        .and_then(|r| r.into_fully_peeled_id().ok().map(|id| id.detach()))
        .or_else(|| {
            repo.rev_parse_single(branch_name)
                .ok()
                .map(|id| id.detach())
        });

    let Some(oid) = start_oid else {
        debug!(branch = %branch_name, "could not resolve branch ref for commit walk");
        return commits;
    };

    let Ok(walk) = repo
        .rev_walk(Some(oid))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .all()
    else {
        return commits;
    };

    let decorations = collect_ref_decorations(repo);

    for info_result in walk.take(count) {
        let Ok(info) = info_result else {
            continue;
        };
        let Ok(commit) = info.object() else {
            continue;
        };

        let refs = decorations.get(&info.id).cloned().unwrap_or_default();

        let Ok(author) = commit.author() else {
            continue;
        };
        let Ok(time) = commit.time() else {
            continue;
        };
        let Ok(message) = commit.message_raw() else {
            continue;
        };

        commits.push(CommitInfo {
            sha: info.id.to_string(),
            message: String::from_utf8_lossy(message.trim()).to_string(),
            author: author.name.to_string(),
            timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    commits
}
