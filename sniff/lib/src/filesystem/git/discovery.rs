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

use crate::request::GitRequest;
use crate::{Result, SniffError};

use super::types::*;

/// Resolve HEAD's commit id, distinguishing an unborn HEAD from a real failure.
///
/// `Ok(None)` means HEAD is unborn (a freshly-initialized branch with no
/// commits) — a legitimately empty history. A missing/malformed `HEAD`,
/// permission, I/O, or corruption failure surfaces as [`SniffError::Git`]
/// rather than collapsing into an empty walk.
pub(crate) fn head_id_opt(repo: &gix::Repository) -> Result<Option<gix::ObjectId>> {
    match repo.head_id() {
        Ok(id) => Ok(Some(id.detach())),
        // Only an unborn branch is "no history"; the symbolic HEAD exists but
        // its branch has no commits yet.
        Err(gix::reference::head_id::Error::PeelToId(
            gix::head::peel::into_id::Error::Unborn { .. },
        )) => Ok(None),
        Err(e) => Err(SniffError::git("head", e)),
    }
}

/// Resolve a revision spec to a single object id, distinguishing a genuine
/// "no object matches" (`Ok(None)`) from ambiguity, I/O, or corruption (`Err`).
///
/// `rev_parse_single` collapses every failure into one opaque error, so a
/// failure is re-checked against a structured object-prefix lookup: a hex
/// prefix that matches no object is true absence, while an ambiguous prefix or
/// an object-database read error means the lookup failed for a reason other
/// than absence and must surface.
fn resolve_single_opt(repo: &gix::Repository, spec: &str) -> Result<Option<gix::ObjectId>> {
    use gix::hash::prefix::from_hex::Error as HexError;

    match repo.rev_parse_single(spec) {
        Ok(id) => Ok(Some(id.detach())),
        Err(err) => match gix::hash::Prefix::from_hex(spec) {
            Ok(prefix) => match repo.objects.lookup_prefix(prefix, None) {
                // No object carries this prefix — genuine absence.
                Ok(None) => Ok(None),
                // A unique or ambiguous match, or a read error: rev_parse did
                // not fail merely because the object was absent.
                _ => Err(SniffError::git("revparse", err)),
            },
            // A non-hex spec (e.g. a ref name) that did not resolve names no
            // commit — genuine absence.
            Err(HexError::Invalid) => Ok(None),
            // A too-short/too-long hex prefix cannot be verified against the
            // object database, so absence cannot be proven: surface the error
            // rather than masquerade an ambiguous lookup as "not found".
            Err(HexError::TooShort { .. } | HexError::TooLong { .. }) => {
                Err(SniffError::git("revparse", err))
            }
        },
    }
}

/// Resolve a non-local-branch specifier to a starting commit id.
///
/// A remote-tracking name (e.g. `origin/main`) is resolved by structured
/// `refs/remotes/<name>` lookup so a malformed ref, or one peeling to a missing
/// object, surfaces as [`SniffError::Git`] instead of collapsing into an empty
/// history. Only when no such ref exists is the input treated as a possible
/// hex SHA via the object-prefix probe; a non-hex name that matches no ref is
/// genuine absence (`Ok(None)`).
fn resolve_remote_or_sha(
    repo: &gix::Repository,
    branch_name: &str,
) -> Result<Option<gix::ObjectId>> {
    match repo.find_reference(&format!("refs/remotes/{branch_name}")) {
        Ok(r) => Ok(Some(
            r.into_fully_peeled_id()
                .map_err(|e| SniffError::git("peel", e))?
                .detach(),
        )),
        Err(gix::reference::find::existing::Error::NotFound { .. }) => {
            resolve_single_opt(repo, branch_name)
        }
        Err(e) => Err(SniffError::git("find_reference", e)),
    }
}

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
///
/// Errors are suppressed: corrupt or unreadable ref stores yield an empty map.
/// For error propagation, use [`collect_ref_decorations_fallible`].
pub(crate) fn collect_ref_decorations(
    repo: &gix::Repository,
) -> HashMap<gix::ObjectId, Vec<RefDecoration>> {
    collect_ref_decorations_fallible(repo).unwrap_or_default()
}

/// Fallible variant of [`collect_ref_decorations`].
///
/// Propagates ref-store, ref-iteration, and peel failures as [`SniffError::Git`]
/// rather than returning a partial or empty map. The HEAD lookup that marks the
/// active branch stays best-effort: a detached or unreadable HEAD simply leaves
/// no decoration flagged as `is_head`.
pub(crate) fn collect_ref_decorations_fallible(
    repo: &gix::Repository,
) -> Result<HashMap<gix::ObjectId, Vec<RefDecoration>>> {
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

    let platform = repo
        .references()
        .map_err(|e| SniffError::git("references", e))?;
    let iter = platform.all().map_err(|e| SniffError::git("references", e))?;

    for reference in iter {
        let reference = reference.map_err(|e| SniffError::git("references", e))?;
        let full_name = reference.name().as_bstr().to_string();

        // Peel through annotated tags to the commit the ref ultimately names.
        let oid = reference
            .into_fully_peeled_id()
            .map_err(|e| SniffError::git("peel", e))?
            .detach();

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

    Ok(decorations)
}

/// Gets the last N commits from HEAD using a gix revwalk, with optional
/// pre-computed ref decorations.
///
/// Errors are suppressed: an unreadable HEAD, revwalk, or commit object yields
/// an empty or truncated history. For error propagation, use
/// [`get_recent_commits_fallible`].
pub(crate) fn get_recent_commits_with_decorations(
    repo: &gix::Repository,
    count: usize,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Vec<CommitInfo> {
    get_recent_commits_fallible(repo, count, ref_decorations).unwrap_or_default()
}

/// Fallible variant of [`get_recent_commits_with_decorations`].
///
/// An unborn HEAD (no commits) is `Ok(empty)`. HEAD, revwalk creation, revwalk
/// items, object decode, author, time, message, and ref-decoration failures
/// propagate as [`SniffError::Git`] rather than truncating history into a
/// successful-looking partial result.
pub(crate) fn get_recent_commits_fallible(
    repo: &gix::Repository,
    count: usize,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Result<Vec<CommitInfo>> {
    let mut commits = Vec::new();

    let Some(head) = head_id_opt(repo)? else {
        return Ok(commits);
    };

    let walk = repo
        .rev_walk(Some(head))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;

    // Collect ref decorations once for all commits (if not provided).
    let cached = ref_decorations.cloned();
    let decorations = match cached {
        Some(d) => d,
        None => collect_ref_decorations_fallible(repo)?,
    };

    for info_result in walk.take(count) {
        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;
        let commit = info.object().map_err(|e| SniffError::git("object", e))?;

        let refs = decorations.get(&info.id).cloned().unwrap_or_default();

        let author = commit.author().map_err(|e| SniffError::git("author", e))?;
        let time = commit.time().map_err(|e| SniffError::git("commit_time", e))?;
        let message = commit
            .message_raw()
            .map_err(|e| SniffError::git("message", e))?;

        commits.push(CommitInfo {
            sha: info.id.to_string(),
            message: String::from_utf8_lossy(message.trim()).to_string(),
            author: author.name.to_string(),
            timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    Ok(commits)
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
///
/// Errors are suppressed: a corrupt object or ref store also yields `None`. For
/// error propagation, use [`get_commit_by_sha_fallible`].
pub fn get_commit_by_sha(repo: &gix::Repository, sha_prefix: &str) -> Option<CommitInfo> {
    get_commit_by_sha_with_decorations(repo, sha_prefix, None)
}

/// Look up a single commit by SHA with optional pre-computed ref decorations.
///
/// Errors are suppressed; see [`get_commit_by_sha_fallible`] for the fallible
/// variant.
pub(crate) fn get_commit_by_sha_with_decorations(
    repo: &gix::Repository,
    sha_prefix: &str,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Option<CommitInfo> {
    get_commit_by_sha_fallible(repo, sha_prefix, ref_decorations)
        .ok()
        .flatten()
}

/// Fallible variant of [`get_commit_by_sha`].
///
/// `Ok(None)` covers the legitimately-empty cases: a SHA that does not resolve
/// to any object, or one that names a non-commit. Object-decode, author, time,
/// message, and ref-decoration failures propagate as [`SniffError::Git`] rather
/// than collapsing into "commit not found".
pub(crate) fn get_commit_by_sha_fallible(
    repo: &gix::Repository,
    sha_prefix: &str,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Result<Option<CommitInfo>> {
    // A SHA that matches no object is "no commit matches" — Ok(None); an
    // ambiguous prefix or an object-database read error surfaces instead.
    let Some(oid) = resolve_single_opt(repo, sha_prefix)? else {
        debug!(sha = sha_prefix, "could not resolve SHA");
        return Ok(None);
    };
    let object = repo
        .find_object(oid)
        .map_err(|e| SniffError::git("object", e))?;
    // A SHA naming a tree/blob is "no commit matches", not corruption.
    let Ok(commit) = object.try_into_commit() else {
        return Ok(None);
    };

    let decorations = match ref_decorations {
        Some(d) => d.clone(),
        None => collect_ref_decorations_fallible(repo)?,
    };
    let refs = decorations.get(&oid).cloned().unwrap_or_default();

    let author = commit.author().map_err(|e| SniffError::git("author", e))?;
    let time = commit.time().map_err(|e| SniffError::git("commit_time", e))?;
    let message = commit
        .message_raw()
        .map_err(|e| SniffError::git("message", e))?;
    Ok(Some(CommitInfo {
        sha: oid.to_string(),
        message: String::from_utf8_lossy(message.trim()).to_string(),
        author: author.name.to_string(),
        timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
        remotes: None,
        refs,
    }))
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
///
/// Errors are suppressed: a missing/corrupt commit, tree, or parent yields an
/// empty list. For error propagation, use [`get_commit_files_fallible`].
pub fn get_commit_files(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
) -> Vec<(PathBuf, DeltaKind)> {
    get_commit_files_fallible(repo, commit_id).unwrap_or_default()
}

/// Like [`get_commit_files`], but reuses an existing diff resource cache.
///
/// Callers that diff many commits in a loop should create one cache with
/// [`Repository::diff_resource_cache_for_tree_diff`] and pass it to this
/// function for each commit. The cache should be cleared periodically
/// (e.g., with [`gix::diff::blob::Platform::clear_resource_cache`]) to
/// avoid unbounded growth when walking large histories.
///
/// Errors are suppressed; see [`get_commit_files_with_cache_fallible`].
pub fn get_commit_files_with_cache(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
    cache: &mut gix::diff::blob::Platform,
) -> Vec<(PathBuf, DeltaKind)> {
    get_commit_files_with_cache_fallible(repo, commit_id, cache).unwrap_or_default()
}

/// Fallible variant of [`get_commit_files`].
///
/// A missing or corrupt commit, tree, parent commit, parent tree, or diff
/// failure propagates as [`SniffError::Git`] rather than producing an empty or
/// partial change list.
pub(crate) fn get_commit_files_fallible(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
) -> Result<Vec<(PathBuf, DeltaKind)>> {
    let mut cache = repo
        .diff_resource_cache_for_tree_diff()
        .map_err(|e| SniffError::git("diff", e))?;
    get_commit_files_with_cache_fallible(repo, commit_id, &mut cache)
}

/// Fallible variant of [`get_commit_files_with_cache`].
pub(crate) fn get_commit_files_with_cache_fallible(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
    cache: &mut gix::diff::blob::Platform,
) -> Result<Vec<(PathBuf, DeltaKind)>> {
    let commit = repo
        .find_object(commit_id)
        .map_err(|e| SniffError::git("object", e))?
        .try_into_commit()
        .map_err(|e| SniffError::git("object", e))?;
    let tree = commit.tree().map_err(|e| SniffError::git("tree", e))?;

    let parent_tree = match commit.parent_ids().next() {
        Some(parent_id) => {
            let parent_commit = repo
                .find_object(parent_id.detach())
                .map_err(|e| SniffError::git("object", e))?
                .try_into_commit()
                .map_err(|e| SniffError::git("object", e))?;
            Some(parent_commit.tree().map_err(|e| SniffError::git("tree", e))?)
        }
        None => None,
    };

    let empty_tree = repo.empty_tree();
    let old_tree = parent_tree.as_ref().unwrap_or(&empty_tree);

    let mut platform = old_tree.changes().map_err(|e| SniffError::git("diff", e))?;

    platform.options(|opts| {
        opts.track_path().track_rewrites(None);
    });

    let mut result: Vec<(PathBuf, DeltaKind)> = Vec::new();

    platform
        .for_each_to_obtain_tree_with_cache(
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
        )
        .map_err(|e| SniffError::git("diff", e))?;

    // gix diff yields path-ordered results in the common case, but
    // explicit sort keeps the contract regardless of internal ordering.
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(result)
}

/// Returns `true` if the commit `oid` touches any file whose path starts with
/// `path_prefix`.
///
/// Propagates object/tree/diff failures as [`SniffError::Git`] so a corrupt
/// commit is not silently treated as "does not touch the path".
fn commit_touches_path(
    repo: &gix::Repository,
    cache: &mut gix::diff::blob::Platform,
    oid: gix::ObjectId,
    path_prefix: &str,
) -> Result<bool> {
    let files = get_commit_files_with_cache_fallible(repo, oid, cache)?;
    Ok(files
        .iter()
        .any(|(p, _)| p.to_string_lossy().starts_with(path_prefix)))
}

/// Get recent commits that touch files under a specific path prefix.
///
/// Walks the commit history from HEAD, computes diffs for each commit,
/// and includes commits where at least one changed file starts with `path_prefix`.
///
/// Ref decorations are collected once and reused for all matching commits.
///
/// Errors are suppressed: a corrupt revwalk, object, or ref store yields an
/// empty list. For error propagation, use [`get_commits_for_path_fallible`].
pub fn get_commits_for_path(
    repo: &gix::Repository,
    path_prefix: &str,
    count: usize,
) -> Vec<CommitInfo> {
    get_commits_for_path_with_decorations(repo, path_prefix, count, None)
}

/// Get recent commits for a path with optional pre-computed ref decorations.
///
/// Errors are suppressed; see [`get_commits_for_path_fallible`].
pub(crate) fn get_commits_for_path_with_decorations(
    repo: &gix::Repository,
    path_prefix: &str,
    count: usize,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Vec<CommitInfo> {
    get_commits_for_path_fallible(repo, path_prefix, count, ref_decorations).unwrap_or_default()
}

/// Fallible variant of [`get_commits_for_path`].
///
/// An unborn HEAD (no history) is `Ok(empty)`. Revwalk, object, author, time,
/// message, ref-decoration, and per-commit diff failures propagate as
/// [`SniffError::Git`] rather than silently skipping commits.
pub(crate) fn get_commits_for_path_fallible(
    repo: &gix::Repository,
    path_prefix: &str,
    count: usize,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Result<Vec<CommitInfo>> {
    let mut commits = Vec::new();

    // An unborn HEAD has no history to walk — a legitimate empty result; a
    // malformed/unreadable HEAD propagates instead.
    let Some(head) = head_id_opt(repo)? else {
        return Ok(commits);
    };

    let walk = repo
        .rev_walk(Some(head))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;

    let decorations = match ref_decorations {
        Some(d) => d.clone(),
        None => collect_ref_decorations_fallible(repo)?,
    };

    let mut diff_cache = repo
        .diff_resource_cache_for_tree_diff()
        .map_err(|e| SniffError::git("diff", e))?;

    for info_result in walk {
        if commits.len() >= count {
            break;
        }
        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;

        if !commit_touches_path(repo, &mut diff_cache, info.id, path_prefix)? {
            continue;
        }

        let commit = info.object().map_err(|e| SniffError::git("object", e))?;
        let refs = decorations.get(&info.id).cloned().unwrap_or_default();

        let author = commit.author().map_err(|e| SniffError::git("author", e))?;
        let time = commit.time().map_err(|e| SniffError::git("commit_time", e))?;
        let message = commit
            .message_raw()
            .map_err(|e| SniffError::git("message", e))?;

        commits.push(CommitInfo {
            sha: info.id.to_string(),
            message: String::from_utf8_lossy(message.trim()).to_string(),
            author: author.name.to_string(),
            timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    Ok(commits)
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
    get_commits_for_branch_fallible(repo, branch_name, count).unwrap_or_default()
}

/// Fallible variant of [`get_commits_for_branch`].
///
/// A branch that cannot be resolved is `Ok(empty)`. Revwalk, object, author,
/// time, message, and ref-decoration failures propagate as [`SniffError::Git`]
/// rather than silently skipping commits.
pub(crate) fn get_commits_for_branch_fallible(
    repo: &gix::Repository,
    branch_name: &str,
    count: usize,
) -> Result<Vec<CommitInfo>> {
    let mut commits = Vec::new();

    // Prefer a local branch ref. A missing local branch falls through to the
    // remote-tracking/SHA fallback (so `origin/main` and bare SHAs still
    // resolve), but a malformed branch ref, or one peeling to a missing object,
    // must surface rather than silently producing an empty history.
    let local = match repo.find_reference(&format!("refs/heads/{branch_name}")) {
        Ok(r) => Some(r),
        Err(gix::reference::find::existing::Error::NotFound { .. }) => None,
        Err(e) => return Err(SniffError::git("find_reference", e)),
    };

    let start_oid = match local {
        Some(r) => Some(
            r.into_fully_peeled_id()
                .map_err(|e| SniffError::git("peel", e))?
                .detach(),
        ),
        None => resolve_remote_or_sha(repo, branch_name)?,
    };

    let Some(oid) = start_oid else {
        debug!(branch = %branch_name, "could not resolve branch ref for commit walk");
        return Ok(commits);
    };

    let walk = repo
        .rev_walk(Some(oid))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;

    let decorations = collect_ref_decorations_fallible(repo)?;

    for info_result in walk.take(count) {
        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;
        let commit = info.object().map_err(|e| SniffError::git("object", e))?;

        let refs = decorations.get(&info.id).cloned().unwrap_or_default();

        let author = commit.author().map_err(|e| SniffError::git("author", e))?;
        let time = commit.time().map_err(|e| SniffError::git("commit_time", e))?;
        let message = commit
            .message_raw()
            .map_err(|e| SniffError::git("message", e))?;

        commits.push(CommitInfo {
            sha: info.id.to_string(),
            message: String::from_utf8_lossy(message.trim()).to_string(),
            author: author.name.to_string(),
            timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    Ok(commits)
}
