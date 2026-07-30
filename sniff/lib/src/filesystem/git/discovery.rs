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

use crate::performance::{self, counters};
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
/// hex SHA — and only when it has a valid object-ID prefix shape. A name that
/// is non-hex or of invalid prefix length (e.g. an absent branch like `add`)
/// is genuine branch absence (`Ok(None)`), not a malformed SHA lookup; an
/// unresolved branch must never be inferred as a corrupt SHA from its
/// characters alone.
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
            // Probe the object database only for SHA-shaped input. A name that
            // cannot form an object-ID prefix names no commit by definition, so
            // it is absence rather than a verifiable-against-the-odb SHA.
            match gix::hash::Prefix::from_hex(branch_name) {
                Ok(_) => resolve_single_opt(repo, branch_name),
                Err(_) => Ok(None),
            }
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
    Ok(super::remote_refresh::RefSnapshot::observe(repo, true, true, true)?
        .decorations()
        .clone())
}

/// Gets the last N commits from HEAD using a gix revwalk, attaching
/// `ref_decorations` to the commits they point at.
///
/// `ref_decorations` is the decoration source: `None` attaches none. It does
/// **not** mean "collect them for me" — a caller that wants decorations owns the
/// cache and passes it in, so the ref-store walk happens once per request rather
/// than once per query.
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
/// items, object decode, author, time, and message failures propagate as
/// [`SniffError::Git`] rather than truncating history into a successful-looking
/// partial result.
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

    for info_result in walk.take(count) {
        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);
        let commit = info.object().map_err(|e| SniffError::git("object", e))?;

        // Only the small vector attached to this commit is cloned; the map is
        // borrowed from the caller's cache rather than copied wholesale.
        let refs = ref_decorations
            .and_then(|d| d.get(&info.id).cloned())
            .unwrap_or_default();

        let author = commit.author().map_err(|e| SniffError::git("author", e))?;
        let time = commit
            .time()
            .map_err(|e| SniffError::git("commit_time", e))?;
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
    let time = commit
        .time()
        .map_err(|e| SniffError::git("commit_time", e))?;
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
    let (tree, parent_tree) = commit_trees(repo, commit_id)?;
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
/// Stops the tree diff at the **first** matching path rather than materializing
/// and sorting every changed path and filtering afterwards, which is what made
/// a path-history walk pay a whole-tree diff per commit regardless of how early
/// the answer was known.
///
/// The match is a string prefix over the same lossy path the file listing
/// produces, preserving the pre-existing filter semantics exactly; it is
/// deliberately not upgraded to a component-wise or pathspec match here, which
/// would change which commits a caller sees.
///
/// Propagates object/tree/diff failures as [`SniffError::Git`] so a corrupt
/// commit is not silently treated as "does not touch the path".
fn commit_touches_path(
    repo: &gix::Repository,
    cache: &mut gix::diff::blob::Platform,
    oid: gix::ObjectId,
    path_prefix: &str,
) -> Result<bool> {
    let (tree, parent_tree) = commit_trees(repo, oid)?;
    let empty_tree = repo.empty_tree();
    let old_tree = parent_tree.as_ref().unwrap_or(&empty_tree);

    let mut platform = old_tree.changes().map_err(|e| SniffError::git("diff", e))?;
    platform.options(|opts| {
        opts.track_path().track_rewrites(None);
    });

    let mut touched = false;
    let outcome = platform.for_each_to_obtain_tree_with_cache(
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
            if path.to_string_lossy().starts_with(path_prefix) {
                touched = true;
                return Ok(std::ops::ControlFlow::Break(()));
            }
            Ok(std::ops::ControlFlow::Continue(()))
        },
    );

    // `Break` surfaces from gix as a `Cancelled` diff error. Checking `touched`
    // first is what distinguishes our own deliberate early stop from a real
    // diff failure — propagating the cancellation would report a successful
    // match as a corrupt repository.
    if touched {
        return Ok(true);
    }
    outcome.map_err(|e| SniffError::git("diff", e))?;

    Ok(false)
}

/// Resolve a commit's tree and its first parent's tree.
///
/// A root commit reports `None`, which callers diff against the empty tree.
/// Only the first parent is considered, so a merge commit is compared against
/// its mainline — the pre-existing behavior of the file listing.
///
/// ## Notes
///
/// A commit at a **shallow boundary** also reports `None`. It names a parent
/// that the object database does not contain, so `try_find_object` returns
/// `None` rather than an error, and the commit is treated exactly as a root:
/// every file in its tree reads as added. That is the honest answer when the
/// preceding history is absent, and it is the only answer available.
///
/// This matters because a shallow clone is the *normal* CI checkout —
/// `actions/checkout@v4` fetches depth 1 by default. Resolving the parent with
/// `find_object` made `sniff repo --json` abort with "An object with id … could
/// not be found" on every runner, while `sniff repo` and `--plain` succeeded
/// because they do not collect commit history.
fn commit_trees(
    repo: &gix::Repository,
    commit_id: gix::ObjectId,
) -> Result<(gix::Tree<'_>, Option<gix::Tree<'_>>)> {
    let commit = repo
        .find_object(commit_id)
        .map_err(|e| SniffError::git("object", e))?
        .try_into_commit()
        .map_err(|e| SniffError::git("object", e))?;
    let tree = commit.tree().map_err(|e| SniffError::git("tree", e))?;

    let parent_tree = match commit.parent_ids().next() {
        Some(parent_id) => {
            let parent_object = repo
                .try_find_object(parent_id.detach())
                .map_err(|e| SniffError::git("object", e))?;
            match parent_object {
                Some(object) => {
                    let parent_commit = object
                        .try_into_commit()
                        .map_err(|e| SniffError::git("object", e))?;
                    Some(
                        parent_commit
                            .tree()
                            .map_err(|e| SniffError::git("tree", e))?,
                    )
                }
                None => None,
            }
        }
        None => None,
    };

    Ok((tree, parent_tree))
}

/// Default bound on commits a path-history walk will examine.
///
/// The bound exists for the sparse-match case, where the walk would otherwise
/// run to the root of history looking for matches that are not there. It caps
/// the tail; it is not what makes the common case fast — stopping at
/// [`PathHistoryOptions::count`] matches does that.
///
/// 10,000 is a policy default, not a measured optimum. See the Phase 5 sub-spec
/// (`sniff/features/2026-07-16-performance/phases/_completed/05-git-observation/spec.md`)
/// for why lowering it would make [`PathHistoryResult::limit_reached`] routine
/// enough that callers learn to ignore it.
pub const DEFAULT_PATH_HISTORY_SCAN_LIMIT: usize = 10_000;

/// How far a path-history query may look, and how much it should return.
///
/// ## Examples
///
/// ```
/// use sniff::filesystem::git::PathHistoryOptions;
///
/// // Ten matches, default scan bound.
/// let opts = PathHistoryOptions::new(10);
///
/// // A cheaper bound for a latency-sensitive caller.
/// let opts = PathHistoryOptions::new(10).scan_limit(500);
/// assert_eq!(opts.scan_limit_value(), 500);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathHistoryOptions {
    scan_limit: usize,
    count: usize,
}

impl PathHistoryOptions {
    /// Return at most `count` matching commits, scanning at most
    /// [`DEFAULT_PATH_HISTORY_SCAN_LIMIT`] commits.
    ///
    /// `count == 0` means "no match cap" — the walk is then bounded only by the
    /// scan limit.
    pub fn new(count: usize) -> Self {
        Self {
            scan_limit: DEFAULT_PATH_HISTORY_SCAN_LIMIT,
            count,
        }
    }

    /// Set the maximum number of commits to examine.
    ///
    /// A zero limit is rejected in favor of the default: it would return an
    /// empty history indistinguishable from "this path was never touched",
    /// which is the failure this bounded API exists to prevent.
    pub fn scan_limit(mut self, limit: usize) -> Self {
        self.scan_limit = if limit == 0 {
            DEFAULT_PATH_HISTORY_SCAN_LIMIT
        } else {
            limit
        };
        self
    }

    /// The effective scan bound.
    pub fn scan_limit_value(&self) -> usize {
        self.scan_limit
    }

    /// The effective match cap (`0` means uncapped).
    pub fn count(&self) -> usize {
        self.count
    }
}

impl Default for PathHistoryOptions {
    fn default() -> Self {
        Self::new(0)
    }
}

/// The outcome of a bounded path-history query.
///
/// `history_exhausted` and `limit_reached` are **not** complements: a walk that
/// collected `count` matches before reaching either boundary reports both as
/// `false`. Distinguishing those three outcomes is the entire reason this type
/// exists — a bare `Vec<CommitInfo>` cannot say whether a short result means
/// "that is all there is" or "we stopped looking".
#[derive(Debug, Clone, Default)]
pub struct PathHistoryResult {
    /// Matching commits, newest first.
    pub commits: Vec<CommitInfo>,
    /// How many commits the walk examined.
    pub commits_scanned: usize,
    /// The walk reached the end of history.
    pub history_exhausted: bool,
    /// The walk stopped at the scan limit rather than exhausting history.
    pub limit_reached: bool,
}

/// How often a long walk releases its bounded diff resource cache (R10.5).
const DIFF_CACHE_CLEAR_INTERVAL: usize = 1_000;

/// Get recent commits that touch files under a specific path prefix.
///
/// Walks the commit history from HEAD and includes commits where at least one
/// changed file starts with `path_prefix`. The walk is bounded by
/// `options.scan_limit`; consult [`PathHistoryResult::limit_reached`] to tell an
/// exhausted history from a truncated scan.
///
/// Ref decorations are collected once and reused for all matching commits.
///
/// Errors are suppressed: a corrupt revwalk, object, or ref store yields an
/// empty result. For error propagation, use [`get_commits_for_path_fallible`].
pub fn get_commits_for_path(
    repo: &gix::Repository,
    path_prefix: &str,
    options: PathHistoryOptions,
) -> PathHistoryResult {
    get_commits_for_path_with_decorations(repo, path_prefix, options, None)
}

/// Get recent commits for a path with optional pre-computed ref decorations.
///
/// Errors are suppressed; see [`get_commits_for_path_fallible`].
pub(crate) fn get_commits_for_path_with_decorations(
    repo: &gix::Repository,
    path_prefix: &str,
    options: PathHistoryOptions,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> PathHistoryResult {
    get_commits_for_path_fallible(repo, path_prefix, options, ref_decorations).unwrap_or_default()
}

/// Fallible variant of [`get_commits_for_path`].
///
/// An unborn HEAD (no history) is an `Ok` empty result with
/// `history_exhausted: true`. Revwalk, object, author, time, message,
/// ref-decoration, and per-commit diff failures propagate as
/// [`SniffError::Git`] rather than silently skipping commits.
pub(crate) fn get_commits_for_path_fallible(
    repo: &gix::Repository,
    path_prefix: &str,
    options: PathHistoryOptions,
    ref_decorations: Option<&HashMap<gix::ObjectId, Vec<RefDecoration>>>,
) -> Result<PathHistoryResult> {
    let mut result = PathHistoryResult::default();

    // An unborn HEAD has no history to walk — a legitimate empty result; a
    // malformed/unreadable HEAD propagates instead.
    let Some(head) = head_id_opt(repo)? else {
        result.history_exhausted = true;
        return Ok(result);
    };

    let walk = repo
        .rev_walk(Some(head))
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            gix::traverse::commit::simple::CommitTimeOrder::NewestFirst,
        ))
        .use_commit_graph(Some(true))
        .all()
        .map_err(|e| SniffError::git("revwalk", e))?;

    // Borrowed, never cloned: a caller that supplied a decoration cache did so
    // to avoid rebuilding it, and copying the whole map here defeated that.
    // Only the small per-commit vector attached to a match is cloned.
    let owned_decorations = match ref_decorations {
        Some(_) => None,
        None => Some(collect_ref_decorations_fallible(repo)?),
    };
    let decorations = ref_decorations.unwrap_or_else(|| {
        owned_decorations
            .as_ref()
            .expect("collected when none supplied")
    });

    let mut diff_cache = repo
        .diff_resource_cache_for_tree_diff()
        .map_err(|e| SniffError::git("diff", e))?;

    let capped = options.count > 0;
    let mut exhausted = true;

    for info_result in walk {
        if capped && result.commits.len() >= options.count {
            // Satisfied before either boundary: neither exhausted nor limited.
            exhausted = false;
            break;
        }
        if result.commits_scanned >= options.scan_limit {
            result.limit_reached = true;
            exhausted = false;
            break;
        }

        let info = info_result.map_err(|e| SniffError::git("revwalk", e))?;
        // Counted before the path filter: a commit the walk yielded and
        // rejected still cost a visit.
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);
        result.commits_scanned += 1;

        // The tree-diff cache grows with every commit examined; release it
        // periodically so a long walk's memory stays bounded.
        if result.commits_scanned % DIFF_CACHE_CLEAR_INTERVAL == 0 {
            diff_cache.clear_resource_cache();
        }

        if !commit_touches_path(repo, &mut diff_cache, info.id, path_prefix)? {
            continue;
        }

        let commit = info.object().map_err(|e| SniffError::git("object", e))?;
        let refs = decorations.get(&info.id).cloned().unwrap_or_default();

        let author = commit.author().map_err(|e| SniffError::git("author", e))?;
        let time = commit
            .time()
            .map_err(|e| SniffError::git("commit_time", e))?;
        let message = commit
            .message_raw()
            .map_err(|e| SniffError::git("message", e))?;

        result.commits.push(CommitInfo {
            sha: info.id.to_string(),
            message: String::from_utf8_lossy(message.trim()).to_string(),
            author: author.name.to_string(),
            timestamp: DateTime::from_timestamp(time.seconds, 0).unwrap_or_default(),
            remotes: None,
            refs,
        });
    }

    result.history_exhausted = exhausted;
    Ok(result)
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
        performance::increment_counter(counters::GIT_COMMIT_VISITS, 1);
        let commit = info.object().map_err(|e| SniffError::git("object", e))?;

        let refs = decorations.get(&info.id).cloned().unwrap_or_default();

        let author = commit.author().map_err(|e| SniffError::git("author", e))?;
        let time = commit
            .time()
            .map_err(|e| SniffError::git("commit_time", e))?;
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

#[cfg(test)]
mod path_history_tests {
    use super::*;
    use tempfile::TempDir;

    /// A repo whose history is `depth` commits, all touching `other/`, with the
    /// single oldest commit touching `wanted/` so a match is only found by
    /// walking to the very bottom.
    fn repo_with_sparse_match(depth: usize) -> (TempDir, gix::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        std::fs::create_dir_all(dir.path().join("wanted")).unwrap();
        std::fs::create_dir_all(dir.path().join("other")).unwrap();

        let mut parents: Vec<git2::Oid> = Vec::new();
        for i in 0..depth {
            // Only the first (oldest) commit touches `wanted/`.
            let rel = if i == 0 {
                "wanted/hit.txt"
            } else {
                "other/miss.txt"
            };
            std::fs::write(dir.path().join(rel), format!("v{i}\n")).unwrap();

            let mut index = repo.index().unwrap();
            index.add_path(Path::new(rel)).unwrap();
            index.write().unwrap();
            let tree_id = index.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();

            let parent_commits: Vec<git2::Commit> = parents
                .iter()
                .map(|id| repo.find_commit(*id).unwrap())
                .collect();
            let parent_refs: Vec<&git2::Commit> = parent_commits.iter().collect();
            let id = repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    &format!("commit {i}"),
                    &tree,
                    &parent_refs,
                )
                .unwrap();
            parents = vec![id];
        }

        let gix_repo = gix::open(dir.path()).unwrap();
        (dir, gix_repo)
    }

    /// R10.2: the scan limit bounds the walk, and says so.
    #[test]
    fn scan_limit_stops_the_walk_and_reports_incompleteness() {
        let (_dir, repo) = repo_with_sparse_match(6);

        let result = get_commits_for_path_fallible(
            &repo,
            "wanted/",
            PathHistoryOptions::new(10).scan_limit(3),
            None,
        )
        .unwrap();

        assert_eq!(result.commits_scanned, 3, "must stop at the bound");
        assert!(result.limit_reached, "stopping at the bound must be visible");
        assert!(
            !result.history_exhausted,
            "a bounded stop is not an exhausted history"
        );
        assert!(
            result.commits.is_empty(),
            "the only match is below the bound"
        );
    }

    /// The same query without a restrictive bound finds the match and reports
    /// an exhausted history — proving the flags above describe the bound, not
    /// the repository.
    #[test]
    fn exhausting_history_reports_exhausted_not_limited() {
        let (_dir, repo) = repo_with_sparse_match(6);

        let result =
            get_commits_for_path_fallible(&repo, "wanted/", PathHistoryOptions::new(10), None)
                .unwrap();

        assert_eq!(result.commits.len(), 1);
        assert_eq!(result.commits_scanned, 6);
        assert!(result.history_exhausted);
        assert!(!result.limit_reached);
    }

    /// R10: satisfying `count` is neither exhaustion nor truncation. This is the
    /// third state a bare `Vec<CommitInfo>` could not express.
    #[test]
    fn satisfying_the_count_reports_neither_boundary() {
        let (_dir, repo) = repo_with_sparse_match(6);

        let result =
            get_commits_for_path_fallible(&repo, "other/", PathHistoryOptions::new(2), None)
                .unwrap();

        assert_eq!(result.commits.len(), 2);
        assert!(!result.history_exhausted, "we stopped early, by choice");
        assert!(!result.limit_reached, "and not because of the scan bound");
    }

    /// R10.1: the walk must not visit more commits than its bound, whatever the
    /// history's size.
    #[test]
    fn commit_visits_are_bounded_by_the_scan_limit() {
        let (_dir, repo) = repo_with_sparse_match(8);

        let collector = crate::performance::PerformanceCollector::new_shared();
        crate::performance::with_current_collector(Some(collector.clone()), || {
            get_commits_for_path_fallible(
                &repo,
                "wanted/",
                PathHistoryOptions::new(10).scan_limit(4),
                None,
            )
            .unwrap()
        });
        let counters = collector
            .snapshot(std::time::Duration::from_secs(0))
            .counters;

        assert_eq!(
            counters
                .get(counters::GIT_COMMIT_VISITS)
                .copied()
                .unwrap_or(0),
            4,
            "the bound must cap visits: {counters:?}"
        );
    }

    /// An unborn HEAD has genuinely exhausted its (empty) history.
    #[test]
    fn unborn_head_is_exhausted_not_limited() {
        let dir = TempDir::new().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        let repo = gix::open(dir.path()).unwrap();

        let result =
            get_commits_for_path_fallible(&repo, "", PathHistoryOptions::new(10), None).unwrap();

        assert!(result.commits.is_empty());
        assert_eq!(result.commits_scanned, 0);
        assert!(result.history_exhausted);
        assert!(!result.limit_reached);
    }
}
