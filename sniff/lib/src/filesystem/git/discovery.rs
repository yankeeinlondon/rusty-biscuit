//! Commit and ref discovery helpers.
//!
//! This module groups commit lookup, recent-commit walks, ref decoration,
//! base-branch resolution, and the `DeltaKind` enum.

use chrono::DateTime;
use git2::Repository;
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
pub(crate) fn collect_ref_decorations(repo: &Repository) -> HashMap<git2::Oid, Vec<RefDecoration>> {
    let mut decorations: HashMap<git2::Oid, Vec<RefDecoration>> = HashMap::new();

    // Get current HEAD target to mark the active branch
    let head_target = repo
        .head()
        .map_err(|e| {
            debug!(error = %e, "could not read HEAD for decorations");
            e
        })
        .ok()
        .and_then(|h| {
            if h.is_branch() {
                h.shorthand().map(String::from)
            } else {
                None
            }
        });

    // Iterate all references
    let Ok(refs) = repo.references() else {
        return decorations;
    };

    for reference in refs.flatten() {
        let Some(name) = reference.name() else {
            continue;
        };

        // Resolve the reference to its target commit
        let Ok(target) = reference.peel_to_commit() else {
            continue;
        };
        let oid = target.id();

        // Determine ref kind and display name
        let (kind, display_name) = if let Some(branch) = name.strip_prefix("refs/heads/") {
            (RefKind::LocalBranch, branch.to_string())
        } else if let Some(remote) = name.strip_prefix("refs/remotes/") {
            (RefKind::RemoteBranch, remote.to_string())
        } else if let Some(tag) = name.strip_prefix("refs/tags/") {
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

/// Gets the last N commits from HEAD using revwalk.
pub(crate) fn get_recent_commits(repo: &Repository, count: usize) -> Vec<CommitInfo> {
    get_recent_commits_with_decorations(repo, count, None)
}

/// Gets the last N commits from HEAD using revwalk, with optional pre-computed ref decorations.
pub(crate) fn get_recent_commits_with_decorations(
    repo: &Repository,
    count: usize,
    ref_decorations: Option<&HashMap<git2::Oid, Vec<RefDecoration>>>,
) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };

    if revwalk.push_head().is_err() {
        return commits;
    }

    // Collect ref decorations once for all commits (if not provided)
    let cached = ref_decorations.cloned();
    let decorations = cached.unwrap_or_else(|| collect_ref_decorations(repo));

    for oid_result in revwalk.take(count) {
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };

        // Get refs pointing to this commit
        let refs = decorations.get(&oid).cloned().unwrap_or_default();

        let author = commit.author();
        commits.push(CommitInfo {
            sha: commit.id().to_string(),
            message: commit.message().unwrap_or("").trim().to_string(),
            author: author.name().unwrap_or("Unknown").to_string(),
            timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    commits
}

/// Gets HEAD commit SHA and upstream tracking branch commit SHA.
pub(crate) fn get_commit_refs(repo: &Repository) -> (String, Option<String>) {
    // Get HEAD commit SHA
    let head_sha = repo
        .head()
        .map_err(|e| {
            debug!(error = %e, "could not read HEAD for commit refs");
            e
        })
        .ok()
        .and_then(|h| {
            h.peel_to_commit()
                .map_err(|e| {
                    debug!(error = %e, "could not peel HEAD to commit");
                    e
                })
                .ok()
        })
        .map(|c| c.id().to_string())
        .unwrap_or_default();

    // Get upstream tracking branch commit using dynamic remote discovery
    let origin_commit = get_upstream_commit(repo);

    (head_sha, origin_commit)
}

/// Gets the upstream tracking branch commit SHA using dynamic remote discovery.
pub(crate) fn get_upstream_commit(repo: &Repository) -> Option<String> {
    let head = repo
        .head()
        .map_err(|e| {
            debug!(error = %e, "could not read HEAD for upstream commit");
            e
        })
        .ok()?;

    // Only works for branch references, not detached HEAD
    if !head.is_branch() {
        return None;
    }

    let branch_name = head.shorthand()?;
    let branch = repo
        .find_branch(branch_name, git2::BranchType::Local)
        .map_err(|e| {
            debug!(branch = branch_name, error = %e, "could not find local branch");
            e
        })
        .ok()?;

    // Get the upstream branch (this handles dynamic remote discovery)
    let upstream = branch
        .upstream()
        .map_err(|e| {
            debug!(error = %e, "branch has no upstream");
            e
        })
        .ok()?;
    let upstream_commit = upstream
        .get()
        .peel_to_commit()
        .map_err(|e| {
            debug!(error = %e, "could not peel upstream to commit");
            e
        })
        .ok()?;

    Some(upstream_commit.id().to_string())
}

/// Resolves the base branch name and its commit OID for ahead/behind calculations.
///
/// When the repo is a worktree, finds the base repo's current branch. Otherwise
/// uses the current HEAD branch. Falls back to "main" or "master" if HEAD is
/// detached or unavailable.
pub(crate) fn resolve_base_branch(repo: &Repository) -> (String, Option<git2::Oid>) {
    // If we're in a worktree, open the base repo to get its HEAD branch
    let base_repo = if repo.is_worktree() {
        repo.commondir().parent().and_then(|p| {
            Repository::open(p)
                .map_err(|e| {
                    debug!(error = %e, "could not open base repository");
                    e
                })
                .ok()
        })
    } else {
        None
    };
    let effective_repo = base_repo.as_ref().unwrap_or(repo);

    // Try the base repo's current HEAD branch
    if let Ok(head) = effective_repo.head()
        && let Some(name) = head.shorthand()
    {
        let oid = head
            .peel_to_commit()
            .map_err(|e| {
                debug!(error = %e, "could not peel base branch HEAD to commit");
                e
            })
            .ok()
            .map(|c| c.id());
        return (name.to_string(), oid);
    }

    // Fallback: try "main", then "master"
    for candidate in &["main", "master"] {
        let refname = format!("refs/heads/{candidate}");
        if let Ok(reference) = repo.find_reference(&refname) {
            let oid = reference
                .peel_to_commit()
                .map_err(|e| {
                    debug!(branch = candidate, error = %e, "could not peel fallback branch to commit");
                    e
                })
                .ok()
                .map(|c| c.id());
            return (candidate.to_string(), oid);
        }
    }

    ("main".to_string(), None)
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
    /// Convert a git2 Delta status to a DeltaKind.
    fn from_delta(delta: git2::Delta) -> Self {
        match delta {
            git2::Delta::Added => Self::Added,
            git2::Delta::Deleted => Self::Deleted,
            git2::Delta::Renamed => Self::Renamed,
            git2::Delta::Copied => Self::Copied,
            _ => Self::Modified,
        }
    }
}

/// Look up a single commit by full or abbreviated SHA.
///
/// Uses `repo.revparse_single()` to resolve abbreviated or full SHA strings,
/// then peels to a commit and builds a `CommitInfo` with ref decorations.
///
/// Returns `None` if the SHA doesn't resolve to a valid commit.
pub fn get_commit_by_sha(repo: &Repository, sha_prefix: &str) -> Option<CommitInfo> {
    get_commit_by_sha_with_decorations(repo, sha_prefix, None)
}

/// Look up a single commit by SHA with optional pre-computed ref decorations.
pub(crate) fn get_commit_by_sha_with_decorations(
    repo: &Repository,
    sha_prefix: &str,
    ref_decorations: Option<&HashMap<git2::Oid, Vec<RefDecoration>>>,
) -> Option<CommitInfo> {
    let obj = repo
        .revparse_single(sha_prefix)
        .map_err(|e| {
            debug!(sha = sha_prefix, error = %e, "could not resolve SHA");
            e
        })
        .ok()?;
    let commit = obj
        .peel_to_commit()
        .map_err(|e| {
            debug!(sha = sha_prefix, error = %e, "could not peel object to commit");
            e
        })
        .ok()?;

    let decorations = ref_decorations
        .cloned()
        .unwrap_or_else(|| collect_ref_decorations(repo));
    let oid = commit.id();
    let refs = decorations.get(&oid).cloned().unwrap_or_default();

    let author = commit.author();
    Some(CommitInfo {
        sha: oid.to_string(),
        message: commit.message().unwrap_or("").trim().to_string(),
        author: author.name().unwrap_or("Unknown").to_string(),
        timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
        remotes: None,
        refs,
    })
}

/// Get the list of files changed by a specific commit.
///
/// Computes a diff between the commit's tree and its first parent's tree.
/// For the initial commit (no parent), diffs against an empty tree.
///
/// Returns a list of `(relative_path, DeltaKind)` pairs.
pub fn get_commit_files(repo: &Repository, full_sha: &str) -> Vec<(PathBuf, DeltaKind)> {
    let Ok(oid) = git2::Oid::from_str(full_sha) else {
        return Vec::new();
    };
    let Ok(commit) = repo.find_commit(oid) else {
        return Vec::new();
    };
    let Ok(tree) = commit.tree() else {
        return Vec::new();
    };

    let parent_tree = commit
        .parent(0)
        .map_err(|e| {
            debug!(sha = full_sha, error = %e, "could not get parent commit");
            e
        })
        .ok()
        .and_then(|p| {
            p.tree()
                .map_err(|e| {
                    debug!(sha = full_sha, error = %e, "could not get parent tree");
                    e
                })
                .ok()
        });
    let diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
        .map_err(|e| {
            debug!(sha = full_sha, error = %e, "could not create diff");
            e
        })
        .ok();

    let Some(diff) = diff else {
        return Vec::new();
    };

    diff.deltas()
        .filter_map(|delta| {
            let path = delta.new_file().path().unwrap_or(Path::new(""));
            if path.as_os_str().is_empty() {
                None
            } else {
                Some((path.to_path_buf(), DeltaKind::from_delta(delta.status())))
            }
        })
        .collect()
}

/// Get recent commits that touch files under a specific path prefix.
///
/// Walks the commit history from HEAD, computes diffs for each commit,
/// and includes commits where at least one changed file starts with `path_prefix`.
///
/// Ref decorations are collected once and reused for all matching commits.
pub fn get_commits_for_path(repo: &Repository, path_prefix: &str, count: usize) -> Vec<CommitInfo> {
    get_commits_for_path_with_decorations(repo, path_prefix, count, None)
}

/// Get recent commits for a path with optional pre-computed ref decorations.
pub(crate) fn get_commits_for_path_with_decorations(
    repo: &Repository,
    path_prefix: &str,
    count: usize,
    ref_decorations: Option<&HashMap<git2::Oid, Vec<RefDecoration>>>,
) -> Vec<CommitInfo> {
    let mut commits = Vec::new();

    let Ok(mut revwalk) = repo.revwalk() else {
        return commits;
    };
    if revwalk.push_head().is_err() {
        return commits;
    }

    let cached = ref_decorations.cloned();
    let decorations = cached.unwrap_or_else(|| collect_ref_decorations(repo));

    for oid_result in revwalk {
        if commits.len() >= count {
            break;
        }
        let Ok(oid) = oid_result else {
            continue;
        };
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        let Ok(tree) = commit.tree() else {
            continue;
        };

        let parent_tree = commit
            .parent(0)
            .map_err(|e| {
                debug!(sha = %oid, error = %e, "could not get parent for path commit");
                e
            })
            .ok()
            .and_then(|p| {
                p.tree()
                    .map_err(|e| {
                        debug!(sha = %oid, error = %e, "could not get parent tree for path commit");
                        e
                    })
                    .ok()
            });
        let Ok(diff) = repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) else {
            continue;
        };

        let touches_path = diff.deltas().any(|delta| {
            let new_path = delta
                .new_file()
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            let old_path = delta
                .old_file()
                .path()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            new_path.starts_with(path_prefix) || old_path.starts_with(path_prefix)
        });

        if touches_path {
            let refs = decorations.get(&oid).cloned().unwrap_or_default();
            let author = commit.author();
            commits.push(CommitInfo {
                sha: oid.to_string(),
                message: commit.message().unwrap_or("").trim().to_string(),
                author: author.name().unwrap_or("Unknown").to_string(),
                timestamp: DateTime::from_timestamp(commit.time().seconds(), 0).unwrap_or_default(),
                remotes: None,
                refs,
            });
        }
    }

    commits
}
