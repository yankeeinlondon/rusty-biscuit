//! Working-tree status, dirty file, and conflict detection helpers.
//!
//! This module collects per-file status information from the working tree,
//! deriving counts and `FileChange` lists used by higher-level callers. Status
//! classification walks the repository once using `gix::Repository::status`,
//! with rename tracking disabled so renames surface as separate delete/add
//! pairs. Per-file diff stats and unified patch text are computed on demand
//! with `gix::diff::blob`.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::Result;
use crate::performance::{self, counters};

use super::types::*;

/// Test-only registry recording every working-tree status walk, keyed by the
/// walked repository's working directory.
///
/// Used by unit tests to prove that [`GitRequest::identity()`] never triggers a
/// status walk. Keying by repo path (rather than a single global counter) keeps
/// each test isolated even under `cargo test`, which runs the whole binary's
/// tests concurrently in one process: every test uses its own temp repo, so a
/// concurrent test's walk lands under a different key and is never counted by
/// [`status_walk_count`]. Walks are also recorded regardless of which thread
/// performs them, so plan-level proofs (whose git stage runs on a scoped
/// thread) work without any cross-thread counter propagation.
#[cfg(test)]
mod walk_probe {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    static WALKS: Mutex<Vec<PathBuf>> = Mutex::new(Vec::new());

    fn key_for(repo: &gix::Repository) -> PathBuf {
        let raw = repo
            .workdir()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| repo.git_dir().to_path_buf());
        raw.canonicalize().unwrap_or(raw)
    }

    /// Record a status walk against `repo`'s working directory.
    pub(crate) fn record(repo: &gix::Repository) {
        if let Ok(mut walks) = WALKS.lock() {
            walks.push(key_for(repo));
        }
    }

    /// Number of status walks recorded so far for the repository at `root`.
    ///
    /// Tests measure a before/after delta on their own repo path rather than
    /// resetting a shared counter, so concurrent tests cannot perturb the count.
    pub(crate) fn count_under(root: &Path) -> usize {
        let target = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
        WALKS
            .lock()
            .map(|walks| walks.iter().filter(|p| **p == target).count())
            .unwrap_or(0)
    }
}

#[cfg(test)]
pub(crate) use walk_probe::count_under as status_walk_count;

/// Per-file line stats accumulated from a diff.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct LineStats {
    pub(crate) added: usize,
    pub(crate) removed: usize,
}

/// One file side's observation: statistics always, patch text only when asked.
///
/// Both fields are derived from a single load of each buffer, and — for a
/// modification — from a single diff of them. Splitting stats and patch across
/// two independent functions is what made every dirty side load its blobs twice
/// and diff them twice.
#[derive(Debug, Default)]
struct SideDiff {
    stats: LineStats,
    patch: String,
}

/// Inputs a status call resolves once and every dirty file side then borrows.
///
/// The HEAD tree and index snapshot were previously re-resolved per file *per
/// side*, making index snapshots O(dirty files) where one per status walk is
/// sufficient. `head_tree`/`index` are `None` only when the repository has no
/// HEAD tree or no readable index; every side treats that as "no evidence" and
/// degrades to empty stats, matching the prior per-call error fallbacks.
struct StatusContext<'repo> {
    repo: &'repo gix::Repository,
    head_tree: Option<gix::Tree<'repo>>,
    index: Option<gix::worktree::Index>,
    workdir: Option<PathBuf>,
}

impl<'repo> StatusContext<'repo> {
    fn new(repo: &'repo gix::Repository) -> Self {
        let head_tree = repo
            .head_tree_id_or_empty()
            .ok()
            .and_then(|id| repo.find_tree(id).ok());
        let index = repo.index_or_empty().ok();
        Self {
            repo,
            head_tree,
            index,
            workdir: repo.workdir().map(Path::to_path_buf),
        }
    }

    /// Index entry for `path` at the unconflicted stage.
    fn index_entry(&self, path: &gix::bstr::BStr) -> Option<&gix::index::Entry> {
        self.index
            .as_ref()?
            .entry_by_path_and_stage(path, gix::index::entry::Stage::Unconflicted)
    }

    /// HEAD-tree entry for `path`.
    fn head_entry(&self, path: &gix::bstr::BStr) -> Option<gix::object::tree::EntryRef<'_, '_>> {
        self.head_tree
            .as_ref()?
            .find_entry(gix::bstr::BString::from(path.to_vec()))
    }

    /// Read the worktree side of `path`, counting the load.
    fn read_worktree(&self, path: &gix::bstr::BStr) -> Option<Vec<u8>> {
        let workdir = self.workdir.as_deref()?;
        performance::increment_counter(counters::GIT_BLOB_LOADS, 1);
        std::fs::read(workdir.join(bstr_to_path(path))).ok()
    }
}

impl std::ops::Add for LineStats {
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            added: self.added + rhs.added,
            removed: self.removed + rhs.removed,
        }
    }
}

/// A staged change discovered by the tree-to-index status walk.
#[derive(Debug, Clone, Copy)]
enum StagedKind {
    Added,
    Modified,
    Deleted,
}

/// An unstaged change discovered by the index-to-worktree status walk.
#[derive(Debug, Clone, Copy)]
enum UnstagedKind {
    Modified,
    Deleted,
}

/// Gathers repository status including staged, unstaged, and untracked changes.
/// Also returns file changes with their status for rich output.
///
/// Walks the repository once using `gix::Repository::status`, classifies each
/// item into staged/unstaged/untracked/conflicted categories, and computes
/// per-file line stats and (when `include_diffs` is true) unified patch text
/// from the relevant blobs and worktree files. Rename tracking is disabled,
/// so a rename appears as a separate delete and add — matching the existing
/// output contract.
pub(crate) fn get_repo_status_with_changes(
    repo: &gix::Repository,
    include_diffs: bool,
) -> Result<(RepoStatus, Vec<FileChange>)> {
    #[cfg(test)]
    walk_probe::record(repo);
    performance::increment_counter(counters::GIT_STATUS_WALKS, 1);
    use gix::bstr::{BString, ByteSlice};
    let workdir = repo.workdir().map(Path::to_path_buf);

    // Keys stay byte-native (`BString`) through lookups and worktree reads;
    // the lossy UTF-8 conversion happens only when building public output, so a
    // non-UTF-8 path resolves the correct index/tree entry and on-disk file and
    // two distinct byte paths cannot collapse to one replacement-char key.
    let mut staged: HashMap<BString, StagedKind> = HashMap::new();
    let mut unstaged: HashMap<BString, UnstagedKind> = HashMap::new();
    let mut untracked_paths: Vec<BString> = Vec::new();
    let mut conflicted_paths: Vec<BString> = Vec::new();

    // A single status walk covers both tree-to-index (staged) and
    // index-to-worktree (unstaged) changes. Untracked files are emitted as
    // individual paths via `UntrackedFiles::Files`. Rename tracking is
    // disabled in both directions so renames surface as delete+add pairs.
    let platform = repo
        .status(gix::progress::Discard)
        .map_err(|e| crate::SniffError::git("status", e))?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
        .index_worktree_rewrites(None);

    let iter = platform
        .into_iter(Vec::<gix::bstr::BString>::new())
        .map_err(|e| crate::SniffError::git("status", e))?;

    for item in iter {
        let item = item.map_err(|e| crate::SniffError::git("status", e))?;
        match item {
            gix::status::Item::TreeIndex(change) => {
                let path = change.location().to_owned();
                match change {
                    gix::diff::index::Change::Addition { .. } => {
                        staged.insert(path, StagedKind::Added);
                    }
                    gix::diff::index::Change::Deletion { .. } => {
                        staged.insert(path, StagedKind::Deleted);
                    }
                    gix::diff::index::Change::Modification { .. } => {
                        staged.insert(path, StagedKind::Modified);
                    }
                    gix::diff::index::Change::Rewrite { .. } => {
                        // Rename tracking is disabled; this should not
                        // occur, but treat it as a modification if it does.
                        staged.insert(path, StagedKind::Modified);
                    }
                }
            }
            gix::status::Item::IndexWorktree(item) => match item {
                gix::status::index_worktree::Item::Modification {
                    status, rela_path, ..
                } => match status {
                    gix::status::plumbing::index_as_worktree::EntryStatus::Conflict { .. } => {
                        conflicted_paths.push(rela_path.as_bstr().to_owned());
                    }
                    gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                        gix::status::plumbing::index_as_worktree::Change::Removed,
                    ) => {
                        unstaged.insert(rela_path.as_bstr().to_owned(), UnstagedKind::Deleted);
                    }
                    gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                        gix::status::plumbing::index_as_worktree::Change::Modification { .. },
                    ) => {
                        unstaged.insert(rela_path.as_bstr().to_owned(), UnstagedKind::Modified);
                    }
                    _ => {}
                },
                gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                    if matches!(entry.status, gix::dir::entry::Status::Untracked) {
                        untracked_paths.push(entry.rela_path.as_bstr().to_owned());
                    }
                }
                gix::status::index_worktree::Item::Rewrite { .. } => {
                    // Rename tracking is disabled for index-to-worktree;
                    // ignore unexpected rewrite items.
                }
            },
        }
    }

    let staged_count = staged.len();
    let unstaged_count = unstaged.len();
    let untracked_count = untracked_paths.len();

    // Collect the unique dirty paths (staged or unstaged, not untracked) for
    // per-file diff computation.
    let dirty_set: HashSet<BString> = staged.keys().chain(unstaged.keys()).cloned().collect();
    let dirty_paths: Vec<BString> = dirty_set.into_iter().collect();

    // Compute line stats and, if requested, unified patch text for every
    // dirty path. A path can have both staged and unstaged changes; stats
    // are summed and patches are concatenated with a separating newline.
    let mut diff_stats: HashMap<BString, LineStats> = HashMap::with_capacity(dirty_paths.len());
    let mut staged_patches: HashMap<BString, String> = HashMap::new();
    let mut unstaged_patches: HashMap<BString, String> = HashMap::new();

    if include_diffs {
        staged_patches.reserve(dirty_paths.len());
        unstaged_patches.reserve(dirty_paths.len());
    }

    // The HEAD tree, index snapshot, and workdir are resolved once here rather
    // than per file per side, which is what made index snapshots scale with the
    // number of dirty files.
    let ctx = StatusContext::new(repo);

    for path in &dirty_paths {
        if let Some(kind) = staged.get(path) {
            let side = staged_side(&ctx, path.as_bstr(), *kind, include_diffs);
            diff_stats
                .entry(path.clone())
                .and_modify(|s| *s = *s + side.stats)
                .or_insert(side.stats);
            if !side.patch.is_empty() {
                staged_patches.insert(path.clone(), side.patch);
            }
        }
        if let Some(kind) = unstaged.get(path) {
            let side = unstaged_side(&ctx, path.as_bstr(), *kind, include_diffs);
            diff_stats
                .entry(path.clone())
                .and_modify(|s| *s = *s + side.stats)
                .or_insert(side.stats);
            if !side.patch.is_empty() {
                unstaged_patches.insert(path.clone(), side.patch);
            }
        }
    }

    let mut file_changes: Vec<FileChange> = Vec::new();

    // Conflicts take precedence and are emitted first.
    for path in &conflicted_paths {
        file_changes.push(FileChange {
            path: lossy_path(path.as_bstr()),
            status: FileStatus::Conflicted,
            action: FileAction::Modified,
            lines_added: 0,
            lines_removed: 0,
        });
    }

    for path in &dirty_paths {
        let is_staged = staged.contains_key(path);
        let is_unstaged = unstaged.contains_key(path);
        let staged_action = staged.get(path).map(|k| match k {
            StagedKind::Added => FileAction::Created,
            StagedKind::Deleted => FileAction::Deleted,
            StagedKind::Modified => FileAction::Modified,
        });
        let unstaged_action = unstaged.get(path).map(|k| match k {
            UnstagedKind::Modified => FileAction::Modified,
            UnstagedKind::Deleted => FileAction::Deleted,
        });

        let (status, action) = if is_staged && is_unstaged {
            (
                FileStatus::Both,
                staged_action.unwrap_or(FileAction::Modified),
            )
        } else if is_staged {
            (
                FileStatus::Staged,
                staged_action.unwrap_or(FileAction::Modified),
            )
        } else {
            (
                FileStatus::Modified,
                unstaged_action.unwrap_or(FileAction::Modified),
            )
        };

        let LineStats {
            added: lines_added,
            removed: lines_removed,
        } = diff_stats.get(path).copied().unwrap_or_default();

        file_changes.push(FileChange {
            path: lossy_path(path.as_bstr()),
            status,
            action,
            lines_added,
            lines_removed,
        });
    }

    for path in &untracked_paths {
        file_changes.push(FileChange {
            path: lossy_path(path.as_bstr()),
            status: FileStatus::Untracked,
            action: FileAction::Created,
            lines_added: 0,
            lines_removed: 0,
        });
    }

    let head_sha = repo.head_id().map(|id| id.to_string()).unwrap_or_default();
    let origin_commit = None; // Preserved for backward compat; upstream resolution is phase 6.

    let dirty = if include_diffs {
        build_dirty_files_from_patches(
            &dirty_paths,
            &staged_patches,
            &unstaged_patches,
            &head_sha,
            &origin_commit,
            &workdir,
        )
    } else {
        Vec::new()
    };

    let untracked = if include_diffs {
        build_untracked_files(&untracked_paths, &workdir)
    } else {
        Vec::new()
    };

    let is_dirty = staged_count > 0
        || unstaged_count > 0
        || untracked_count > 0
        || !conflicted_paths.is_empty();

    let repo_status = RepoStatus {
        is_dirty,
        staged_count,
        unstaged_count,
        untracked_count,
        dirty,
        untracked,
        is_behind: None, // Populated by detect_git when deep=true
    };

    Ok((repo_status, file_changes))
}

/// Convert a byte path from `gix` to a `PathBuf` using an explicit lossy
/// UTF-8 conversion at the public string boundary.
fn lossy_path(bytes: &gix::bstr::BStr) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes.as_ref()).as_ref())
}

/// Observe one staged side (HEAD tree vs index): load each blob once, diff once.
///
/// `want_patch` adds the unified patch without re-loading or re-diffing: for a
/// modification the hunks are rendered from the very diff the statistics were
/// counted from.
fn staged_side(
    ctx: &StatusContext<'_>,
    path: &gix::bstr::BStr,
    kind: StagedKind,
    want_patch: bool,
) -> SideDiff {
    match kind {
        StagedKind::Added => {
            let Some(entry) = ctx.index_entry(path) else {
                return SideDiff::default();
            };
            let (id, mode) = (entry.id, index_mode(entry));
            let new = read_blob_or_empty(ctx.repo, id);
            added_side(
                PatchHeader {
                    path,
                    op: PatchOp::Added,
                    old_id: None,
                    new_id: Some(id),
                    mode: &mode,
                },
                &new,
                want_patch,
            )
        }
        StagedKind::Deleted => {
            let Some(entry) = ctx.head_entry(path) else {
                return SideDiff::default();
            };
            let old = read_blob_or_empty(ctx.repo, entry.id());
            let mut buf = [0u8; 6];
            let mode = String::from_utf8_lossy(entry.mode().as_bytes(&mut buf)).into_owned();
            deleted_side(
                PatchHeader {
                    path,
                    op: PatchOp::Deleted,
                    old_id: Some(entry.id().into()),
                    new_id: None,
                    mode: &mode,
                },
                &old,
                want_patch,
            )
        }
        StagedKind::Modified => {
            let Some(head_entry) = ctx.head_entry(path) else {
                return SideDiff::default();
            };
            let head_id: gix::ObjectId = head_entry.id().into();
            let Some(index_entry) = ctx.index_entry(path) else {
                return SideDiff::default();
            };
            let (index_id, mode) = (index_entry.id, index_mode(index_entry));
            let old = read_blob_or_empty(ctx.repo, head_id);
            let new = read_blob_or_empty(ctx.repo, index_id);
            modified_side(
                PatchHeader {
                    path,
                    op: PatchOp::Modified,
                    old_id: Some(head_id),
                    new_id: Some(index_id),
                    mode: &mode,
                },
                &old,
                &new,
                want_patch,
            )
        }
    }
}

/// Observe one unstaged side (index vs worktree): load each side once, diff once.
fn unstaged_side(
    ctx: &StatusContext<'_>,
    path: &gix::bstr::BStr,
    kind: UnstagedKind,
    want_patch: bool,
) -> SideDiff {
    let Some(entry) = ctx.index_entry(path) else {
        return SideDiff::default();
    };
    let (index_id, mode) = (entry.id, index_mode(entry));

    match kind {
        UnstagedKind::Deleted => {
            let old = read_blob_or_empty(ctx.repo, index_id);
            deleted_side(
                PatchHeader {
                    path,
                    op: PatchOp::Deleted,
                    old_id: Some(index_id),
                    new_id: None,
                    mode: &mode,
                },
                &old,
                want_patch,
            )
        }
        UnstagedKind::Modified => {
            let Some(new) = ctx.read_worktree(path) else {
                return SideDiff::default();
            };
            let old = read_blob_or_empty(ctx.repo, index_id);
            let new_id = blob_oid(ctx.repo, &new);
            modified_side(
                PatchHeader {
                    path,
                    op: PatchOp::Modified,
                    old_id: Some(index_id),
                    new_id,
                    mode: &mode,
                },
                &old,
                &new,
                want_patch,
            )
        }
    }
}

/// A wholly-added side: every line of `new` is an addition.
///
/// Stats are a line count rather than a diff against emptiness — the same
/// cheaper-and-exact accounting this path always used. A diff runs only to
/// render the patch, so a stats-only request costs no diff at all.
fn added_side(header: PatchHeader<'_>, new: &[u8], want_patch: bool) -> SideDiff {
    SideDiff {
        stats: LineStats {
            added: countable_lines(new),
            removed: 0,
        },
        patch: build_patch(header, &[], new, want_patch),
    }
}

/// A wholly-deleted side: every line of `old` is a removal.
fn deleted_side(header: PatchHeader<'_>, old: &[u8], want_patch: bool) -> SideDiff {
    SideDiff {
        stats: LineStats {
            added: 0,
            removed: countable_lines(old),
        },
        patch: build_patch(header, old, &[], want_patch),
    }
}

/// A modified side: one diff supplies both the statistics and the hunks.
fn modified_side(
    header: PatchHeader<'_>,
    old: &[u8],
    new: &[u8],
    want_patch: bool,
) -> SideDiff {
    let (stats, hunks) = diff_once(old, new, want_patch);
    SideDiff {
        stats,
        patch: if want_patch {
            git_patch(header, old, new, &hunks)
        } else {
            String::new()
        },
    }
}

/// Render a patch for a whole-file add/delete, diffing only when one is wanted.
fn build_patch(header: PatchHeader<'_>, old: &[u8], new: &[u8], want_patch: bool) -> String {
    if !want_patch {
        return String::new();
    }
    let (_, hunks) = diff_once(old, new, true);
    git_patch(header, old, new, &hunks)
}

/// Exact filesystem path from repo-relative git bytes.
///
/// Byte-exact on Unix (`OsStr::from_bytes`); lossy on platforms whose native
/// paths are not byte-oriented (e.g. Windows).
fn bstr_to_path(bytes: &gix::bstr::BStr) -> PathBuf {
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        PathBuf::from(std::ffi::OsStr::from_bytes(bytes))
    }
    #[cfg(not(unix))]
    {
        PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
    }
}

/// Read a blob's content, returning an empty vec on failure.
fn read_blob_or_empty(repo: &gix::Repository, id: impl Into<gix::ObjectId>) -> Vec<u8> {
    // Sole chokepoint for object-side loads: every blob-reading helper routes
    // here, so counting deeper would double-count the same load.
    performance::increment_counter(counters::GIT_BLOB_LOADS, 1);
    repo.find_blob(id.into())
        .map(|mut b| b.take_data())
        .unwrap_or_default()
}

/// Lines of `content` that count toward add/delete stats.
///
/// Returns zero for binary content so add/delete line stats stay consistent
/// with Git's "binary files differ" behavior.
fn countable_lines(content: &[u8]) -> usize {
    if is_binary(content) {
        0
    } else {
        byte_lines(content)
    }
}

/// Count newline-terminated lines in a byte buffer.
fn byte_lines(content: &[u8]) -> usize {
    if content.is_empty() {
        return 0;
    }
    let count = content.iter().filter(|&&b| b == b'\n').count();
    // If the content does not end with a newline, the last line is still
    // counted as a line (matching Git's line-counting convention).
    if content.last() == Some(&b'\n') {
        count
    } else {
        count + 1
    }
}

/// Diff two byte buffers **once**, returning line stats and, when `want_hunks`,
/// the unified hunks rendered from that same diff.
///
/// The single diff site for a file side. Previously the stats pass and the
/// patch pass each built their own `InternedInput` and ran their own
/// `diff_with_slider_heuristics` over identical bytes; only the stats pass
/// incremented [`counters::GIT_FILE_DIFFS`], so the patch-side diff was real
/// work that no baseline ever saw.
///
/// Returns zero counts if either buffer appears to be binary (contains a null
/// byte in the first 8 KB), matching Git's heuristic and the prior git2-based
/// behavior.
fn diff_once(old: &[u8], new: &[u8], want_hunks: bool) -> (LineStats, String) {
    performance::increment_counter(counters::GIT_FILE_DIFFS, 1);
    if is_binary(old) || is_binary(new) {
        // Binary sides carry no stats and no hunks; `git_patch` renders the
        // "Binary files … differ" line from the buffers instead.
        return (LineStats::default(), String::new());
    }
    if old == new {
        return (LineStats::default(), String::new());
    }

    let input = gix::diff::blob::InternedInput::new(old, new);
    let diff =
        gix::diff::blob::diff_with_slider_heuristics(gix::diff::blob::Algorithm::Histogram, &input);
    let stats = LineStats {
        added: diff.count_additions() as usize,
        removed: diff.count_removals() as usize,
    };
    let hunks = if want_hunks {
        render_hunks(&diff, &input)
    } else {
        String::new()
    };
    (stats, hunks)
}

/// Git's binary heuristic: a buffer is binary if it contains a null byte in
/// the first 8 KB.
fn is_binary(buf: &[u8]) -> bool {
    let sample = &buf[..buf.len().min(8000)];
    sample.contains(&0)
}

/// Change category for git-style patch header construction.
#[derive(Clone, Copy)]
enum PatchOp {
    Added,
    Deleted,
    Modified,
}

/// The file-level facts a patch header needs, independent of its content.
#[derive(Clone, Copy)]
struct PatchHeader<'a> {
    path: &'a gix::bstr::BStr,
    op: PatchOp,
    old_id: Option<gix::ObjectId>,
    new_id: Option<gix::ObjectId>,
    /// 6-digit octal blob mode, e.g. `100644`.
    mode: &'a str,
}

/// 7-hex-character object abbreviation matching libgit2's default `index`-line
/// width, or the all-zero abbreviation for an absent side.
fn abbrev7(id: Option<gix::ObjectId>) -> String {
    match id {
        Some(id) => id.to_string()[..7].to_string(),
        None => "0000000".to_string(),
    }
}

/// Build a full git-format patch (file header + hunks) byte-compatible with
/// `git2`'s `DiffFormat::Patch` output: the `diff --git`, mode, `index`, and
/// `---`/`+++` markers a `git diff` produces, followed by `hunks`.
///
/// `hunks` is rendered by the caller from the one diff it already ran, so this
/// function never diffs. Binary content yields the `Binary files … differ` line
/// instead of hunks, matching libgit2 — the `old`/`new` buffers are still
/// needed for that heuristic.
fn git_patch(header: PatchHeader<'_>, old: &[u8], new: &[u8], hunks: &str) -> String {
    use std::fmt::Write;
    let PatchHeader {
        path,
        op,
        old_id,
        new_id,
        mode,
    } = header;
    // Path appears only in display-oriented header text; lossy is acceptable
    // here (git quotes non-UTF-8 paths — an extreme edge we do not replicate).
    let p = String::from_utf8_lossy(path);
    let mut s = String::new();
    let _ = writeln!(s, "diff --git a/{p} b/{p}");
    match op {
        PatchOp::Added => {
            let _ = writeln!(s, "new file mode {mode}");
        }
        PatchOp::Deleted => {
            let _ = writeln!(s, "deleted file mode {mode}");
        }
        PatchOp::Modified => {}
    }
    let old7 = abbrev7(old_id);
    let new7 = abbrev7(new_id);
    match op {
        PatchOp::Modified => {
            let _ = writeln!(s, "index {old7}..{new7} {mode}");
        }
        _ => {
            let _ = writeln!(s, "index {old7}..{new7}");
        }
    }

    if is_binary(old) || is_binary(new) {
        let a = if matches!(op, PatchOp::Added) {
            "/dev/null".to_string()
        } else {
            format!("a/{p}")
        };
        let b = if matches!(op, PatchOp::Deleted) {
            "/dev/null".to_string()
        } else {
            format!("b/{p}")
        };
        let _ = writeln!(s, "Binary files {a} and {b} differ");
        return s;
    }

    match op {
        PatchOp::Added => {
            let _ = write!(s, "--- /dev/null\n+++ b/{p}\n");
        }
        PatchOp::Deleted => {
            let _ = write!(s, "--- a/{p}\n+++ /dev/null\n");
        }
        PatchOp::Modified => {
            let _ = write!(s, "--- a/{p}\n+++ b/{p}\n");
        }
    }
    s.push_str(hunks);
    s
}

/// SHA-1 a blob's worktree bytes without writing to the object database, so the
/// `index` line can name the workdir side the way `git2` does.
fn blob_oid(repo: &gix::Repository, bytes: &[u8]) -> Option<gix::ObjectId> {
    gix::objs::compute_hash(repo.object_hash(), gix::object::Kind::Blob, bytes).ok()
}

/// Octal blob mode string (e.g. `100644`) for an index entry.
fn index_mode(entry: &gix::index::Entry) -> String {
    format!("{:o}", entry.mode.bits())
}

/// Render only the unified hunks (`@@` headers + content lines) of an
/// already-computed diff, without the file-level header.
fn render_hunks(
    diff: &gix::diff::blob::Diff,
    input: &gix::diff::blob::InternedInput<&[u8]>,
) -> String {
    struct PatchCollector(String);
    impl gix::diff::blob::unified_diff::ConsumeHunk for PatchCollector {
        type Out = String;
        fn consume_hunk(
            &mut self,
            header: gix::diff::blob::unified_diff::HunkHeader,
            lines: &[(gix::diff::blob::unified_diff::DiffLineKind, &[u8])],
        ) -> std::io::Result<()> {
            use std::fmt::Write;
            // git/GNU unified-diff range convention (gix's own HunkHeader
            // Display does not apply it): a length-1 range omits the count and
            // shows only its start; a length-0 range shows `,0` against the
            // line *before* the range (start - 1).
            fn fmt_range(start: u32, len: u32) -> String {
                match len {
                    1 => format!("{start}"),
                    0 => format!("{},0", start.saturating_sub(1)),
                    n => format!("{start},{n}"),
                }
            }
            let _ = writeln!(
                self.0,
                "@@ -{} +{} @@",
                fmt_range(header.before_hunk_start, header.before_hunk_len),
                fmt_range(header.after_hunk_start, header.after_hunk_len),
            );
            for (kind, content) in lines {
                let prefix = match kind {
                    gix::diff::blob::unified_diff::DiffLineKind::Context => ' ',
                    gix::diff::blob::unified_diff::DiffLineKind::Add => '+',
                    gix::diff::blob::unified_diff::DiffLineKind::Remove => '-',
                };
                self.0.push(prefix);
                self.0.push_str(&String::from_utf8_lossy(content));
                if !content.ends_with(b"\n") {
                    self.0.push('\n');
                }
            }
            Ok(())
        }
        fn finish(self) -> Self::Out {
            self.0
        }
    }

    let collector = PatchCollector(String::new());
    gix::diff::blob::UnifiedDiff::new(
        diff,
        input,
        collector,
        gix::diff::blob::unified_diff::ContextSize::default(),
    )
    .consume()
    .unwrap_or_default()
}

/// Assemble per-file `DirtyFile` entries from the staged and unstaged patch
/// strings collected for each dirty path.
fn build_dirty_files_from_patches(
    paths: &[gix::bstr::BString],
    staged_patches: &HashMap<gix::bstr::BString, String>,
    unstaged_patches: &HashMap<gix::bstr::BString, String>,
    head_sha: &str,
    origin_commit: &Option<String>,
    repo_root: &Option<PathBuf>,
) -> Vec<DirtyFile> {
    use gix::bstr::ByteSlice;
    paths
        .iter()
        .map(|key| {
            let mut diff = String::new();
            if let Some(staged) = staged_patches.get(key)
                && !staged.is_empty()
            {
                diff.push_str(staged);
            }
            if let Some(unstaged) = unstaged_patches.get(key)
                && !unstaged.is_empty()
            {
                if !diff.is_empty() {
                    diff.push('\n');
                }
                diff.push_str(unstaged);
            }

            let filepath = lossy_path(key.as_bstr());
            // Absolute path is for on-disk access, so keep it byte-exact.
            let absolute_filepath = repo_root
                .as_ref()
                .map(|root| root.join(bstr_to_path(key.as_bstr())))
                .unwrap_or_else(|| filepath.clone());

            DirtyFile {
                filepath,
                absolute_filepath,
                diff,
                last_local_commit: head_sha.to_string(),
                origin_commit: origin_commit.clone(),
            }
        })
        .collect()
}

/// Builds detailed information for untracked files.
fn build_untracked_files(
    paths: &[gix::bstr::BString],
    repo_root: &Option<PathBuf>,
) -> Vec<UntrackedFile> {
    use gix::bstr::ByteSlice;
    paths
        .iter()
        .map(|key| {
            let filepath = lossy_path(key.as_bstr());
            let absolute_filepath = repo_root
                .as_ref()
                .map(|root| root.join(bstr_to_path(key.as_bstr())))
                .unwrap_or_else(|| filepath.clone());

            UntrackedFile {
                filepath,
                absolute_filepath,
            }
        })
        .collect()
}

/// Lightweight status check that only counts files by category.
///
/// Avoids the cost of per-file diff stat computation and unified diff
/// generation. Use this when you only need `is_dirty` and file counts.
pub(crate) fn get_repo_status_counts(repo: &gix::Repository) -> crate::Result<(bool, usize)> {
    let (is_dirty, staged, unstaged, untracked) = get_repo_status_counts_detailed(repo)?;
    Ok((is_dirty, staged + unstaged + untracked))
}

/// Fast dirty check that stops at the first staged, unstaged, untracked, or
/// conflicted change.
///
/// Backs `GitRequest::minimal()`/`summary()`: those requests only need the
/// `is_dirty` flag, so counting every change (as
/// [`get_repo_status_counts_detailed`] does) is wasted work. Preserves sniff's
/// untracked-inclusive dirty definition, unlike gix's tracked-only
/// [`gix::Repository::is_dirty`] (which disables the directory walk).
pub(crate) fn is_repo_dirty(repo: &gix::Repository) -> Result<bool> {
    #[cfg(test)]
    walk_probe::record(repo);
    performance::increment_counter(counters::GIT_STATUS_WALKS, 1);
    let platform = repo
        .status(gix::progress::Discard)
        .map_err(|e| crate::SniffError::git("status", e))?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
        .index_worktree_rewrites(None);

    let iter = platform
        .into_iter(Vec::<gix::bstr::BString>::new())
        .map_err(|e| crate::SniffError::git("status", e))?;

    for item in iter {
        match item.map_err(|e| crate::SniffError::git("status", e))? {
            gix::status::Item::TreeIndex(_) => return Ok(true),
            gix::status::Item::IndexWorktree(item) => match item {
                gix::status::index_worktree::Item::Modification { status, .. } => match status {
                    gix::status::plumbing::index_as_worktree::EntryStatus::Conflict { .. }
                    | gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                        gix::status::plumbing::index_as_worktree::Change::Removed
                        | gix::status::plumbing::index_as_worktree::Change::Modification { .. },
                    ) => return Ok(true),
                    _ => {}
                },
                gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                    if matches!(entry.status, gix::dir::entry::Status::Untracked) {
                        return Ok(true);
                    }
                }
                gix::status::index_worktree::Item::Rewrite { .. } => {}
            },
        }
    }
    Ok(false)
}

/// Lightweight status check returning individual category counts.
pub(crate) fn get_repo_status_counts_detailed(
    repo: &gix::Repository,
) -> Result<(bool, usize, usize, usize)> {
    #[cfg(test)]
    walk_probe::record(repo);
    performance::increment_counter(counters::GIT_STATUS_WALKS, 1);
    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    let platform = repo
        .status(gix::progress::Discard)
        .map_err(|e| crate::SniffError::git("status", e))?
        .untracked_files(gix::status::UntrackedFiles::Files)
        .tree_index_track_renames(gix::status::tree_index::TrackRenames::Disabled)
        .index_worktree_rewrites(None);

    let iter = platform
        .into_iter(Vec::<gix::bstr::BString>::new())
        .map_err(|e| crate::SniffError::git("status", e))?;

    for item in iter {
        let item = item.map_err(|e| crate::SniffError::git("status", e))?;
        match item {
            gix::status::Item::TreeIndex(_) => staged += 1,
            gix::status::Item::IndexWorktree(item) => match item {
                gix::status::index_worktree::Item::Modification { status, .. } => match status {
                    gix::status::plumbing::index_as_worktree::EntryStatus::Conflict { .. }
                    | gix::status::plumbing::index_as_worktree::EntryStatus::Change(
                        gix::status::plumbing::index_as_worktree::Change::Removed
                        | gix::status::plumbing::index_as_worktree::Change::Modification { .. },
                    ) => unstaged += 1,
                    _ => {}
                },
                gix::status::index_worktree::Item::DirectoryContents { entry, .. } => {
                    if matches!(entry.status, gix::dir::entry::Status::Untracked) {
                        untracked += 1;
                    }
                }
                gix::status::index_worktree::Item::Rewrite { .. } => {}
            },
        }
    }

    let is_dirty = staged > 0 || unstaged > 0 || untracked > 0;
    Ok((is_dirty, staged, unstaged, untracked))
}

/// Detect unmerged (conflicted) files in the repository index.
///
/// Returns the relative paths of files that have merge conflict markers
/// in the index (i.e., are in an unmerged state from a merge, rebase,
/// cherry-pick, or revert).
///
/// Errors are suppressed: a corrupt or unreadable index yields an empty list.
/// For error propagation, use [`detect_merge_conflicts_fallible`].
pub fn detect_merge_conflicts(repo: &gix::Repository) -> Vec<PathBuf> {
    detect_merge_conflicts_fallible(repo).unwrap_or_default()
}

/// Fallible variant of [`detect_merge_conflicts`].
///
/// An index-read failure propagates as [`SniffError::Git`] rather than being
/// reported as "no conflicts".
pub(crate) fn detect_merge_conflicts_fallible(repo: &gix::Repository) -> Result<Vec<PathBuf>> {
    // Malformed index bytes are external input. Some truncated forms panic in
    // gix's decoder, so keep that upstream bug behind Sniff's fallible API.
    let index = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        repo.index_or_empty()
            .map_err(|error| crate::SniffError::git("index", error))
    }))
        .map_err(|_| {
            crate::SniffError::git(
                "index",
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "Git index decoder panicked on malformed input",
                ),
            )
        })??;

    let mut conflicted = Vec::new();
    let mut seen = HashSet::new();
    let state: &gix::index::State = &index;
    for entry in index.entries() {
        if entry.stage() != gix::index::entry::Stage::Unconflicted {
            let path = lossy_path(entry.path(state));
            if seen.insert(path.clone()) {
                conflicted.push(path);
            }
        }
    }
    Ok(conflicted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn open_gix(repo_path: &Path) -> gix::Repository {
        gix::open(repo_path).expect("open with gix")
    }

    /// Creates a temporary git repo with a single file committed.
    fn setup_repo() -> (TempDir, gix::Repository) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        let repo = git2::Repository::init(&path).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = path.join("test.txt");
        std::fs::write(&file_path, "initial content\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (dir, open_gix(&path))
    }

    /// Stage a path (`git add <path>`).
    fn stage_path(repo_path: &Path, relative: &str) {
        let repo = git2::Repository::open(repo_path).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }

    /// Stage a deletion (`git rm <path>` equivalent at the index level).
    fn stage_delete(repo_path: &Path, relative: &str) {
        let repo = git2::Repository::open(repo_path).unwrap();
        let mut index = repo.index().unwrap();
        index.remove_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }

    fn find_change<'a>(changes: &'a [FileChange], path: &str) -> &'a FileChange {
        changes
            .iter()
            .find(|c| c.path == Path::new(path))
            .unwrap_or_else(|| panic!("expected change for {}", path))
    }

    /// Counters recorded while `f` runs.
    fn work_counts<T>(f: impl FnOnce() -> T) -> std::collections::BTreeMap<String, u64> {
        let collector = performance::PerformanceCollector::new_shared();
        performance::with_current_collector(Some(collector.clone()), f);
        collector
            .snapshot(std::time::Duration::from_secs(0))
            .counters
    }

    fn count_of(counts: &std::collections::BTreeMap<String, u64>, name: &str) -> u64 {
        counts.get(name).copied().unwrap_or(0)
    }

    /// R8.3/R8.4: one blob/worktree load and one diff per dirty file side.
    ///
    /// A file that is both staged and modified has two sides, so the bound is
    /// two loads per side (old + new) and one diff per side. Before Phase 5 the
    /// stats pass and the patch pass each loaded and diffed independently, so
    /// this cost twice as much — and half the diffs were never counted.
    #[test]
    fn each_dirty_side_loads_and_diffs_once() {
        let (dir, repo) = setup_repo();

        // Stage a modification, then modify again in the worktree: one path,
        // two sides (HEAD→index and index→worktree).
        std::fs::write(dir.path().join("test.txt"), "staged content\n").unwrap();
        stage_path(dir.path(), "test.txt");
        std::fs::write(dir.path().join("test.txt"), "worktree content\n").unwrap();

        let counts = work_counts(|| get_repo_status_with_changes(&repo, true).unwrap());

        assert_eq!(
            count_of(&counts, counters::GIT_FILE_DIFFS),
            2,
            "two sides must produce exactly two diffs, not four: {counts:?}"
        );
        assert_eq!(
            count_of(&counts, counters::GIT_BLOB_LOADS),
            4,
            "each side loads its old and new buffer exactly once: {counts:?}"
        );
    }

    /// R8.6: shallow requests must not pay for blobs or diffs.
    #[test]
    fn counts_only_status_loads_no_blobs_and_runs_no_diffs() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("test.txt"), "modified\n").unwrap();

        let counts = work_counts(|| get_repo_status_counts_detailed(&repo).unwrap());

        assert_eq!(count_of(&counts, counters::GIT_BLOB_LOADS), 0);
        assert_eq!(count_of(&counts, counters::GIT_FILE_DIFFS), 0);

        let dirty_counts = work_counts(|| is_repo_dirty(&repo).unwrap());
        assert_eq!(count_of(&dirty_counts, counters::GIT_BLOB_LOADS), 0);
        assert_eq!(count_of(&dirty_counts, counters::GIT_FILE_DIFFS), 0);
    }

    /// A stats-only request on a wholly-added file needs no diff at all: the
    /// additions are a line count of the new blob.
    #[test]
    fn added_file_stats_without_diffs_runs_no_diff() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("added.txt"), "a\nb\nc\n").unwrap();
        stage_path(dir.path(), "added.txt");

        let counts = work_counts(|| get_repo_status_with_changes(&repo, false).unwrap());
        assert_eq!(
            count_of(&counts, counters::GIT_FILE_DIFFS),
            0,
            "a whole-file addition is counted, not diffed: {counts:?}"
        );

        let (_, changes) = get_repo_status_with_changes(&repo, false).unwrap();
        assert_eq!(find_change(&changes, "added.txt").lines_added, 3);
    }

    #[test]
    fn batched_diff_attributes_lines_to_unstaged_only_files() {
        let (dir, repo) = setup_repo();

        std::fs::write(dir.path().join("test.txt"), "modified content\n").unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.unstaged_count, 1);
        assert_eq!(status.staged_count, 0);
        let change = find_change(&changes, "test.txt");
        assert_eq!(change.status, FileStatus::Modified);
        assert_eq!(change.lines_added, 1);
        assert_eq!(change.lines_removed, 1);
    }

    #[test]
    fn batched_diff_sums_staged_and_unstaged_for_combined_changes() {
        let (dir, repo) = setup_repo();

        std::fs::write(dir.path().join("test.txt"), "staged content\n").unwrap();
        stage_path(dir.path(), "test.txt");
        std::fs::write(
            dir.path().join("test.txt"),
            "unstaged content\nmore lines\n",
        )
        .unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 1);
        assert_eq!(status.unstaged_count, 1);

        let change = find_change(&changes, "test.txt");
        assert_eq!(change.status, FileStatus::Both);
        // Staged: 1 add / 1 remove. Unstaged (index→workdir): 2 add / 1 remove.
        assert_eq!(change.lines_added, 3);
        assert_eq!(change.lines_removed, 2);
    }

    #[test]
    fn batched_diff_handles_staged_deletes() {
        let (dir, repo) = setup_repo();

        std::fs::remove_file(dir.path().join("test.txt")).unwrap();
        stage_delete(dir.path(), "test.txt");

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 1);
        assert_eq!(status.unstaged_count, 0);

        let change = find_change(&changes, "test.txt");
        assert_eq!(change.status, FileStatus::Staged);
        assert_eq!(change.action, FileAction::Deleted);
        assert_eq!(change.lines_added, 0);
        assert_eq!(change.lines_removed, 1);
    }

    #[test]
    fn batched_diff_handles_unstaged_deletes_with_concurrent_modify() {
        let (dir, repo) = setup_repo();

        let git2_repo = git2::Repository::open(dir.path()).unwrap();
        std::fs::write(dir.path().join("other.txt"), "alpha\nbeta\n").unwrap();
        stage_path(dir.path(), "other.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = git2_repo.index().unwrap().write_tree().unwrap();
        {
            let tree = git2_repo.find_tree(tree_id).unwrap();
            let parent = git2_repo.head().unwrap().peel_to_commit().unwrap();
            git2_repo
                .commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
                .unwrap();
        }

        std::fs::remove_file(dir.path().join("test.txt")).unwrap();
        std::fs::write(dir.path().join("other.txt"), "alpha\ngamma\n").unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, true).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.unstaged_count, 2);

        let deleted = find_change(&changes, "test.txt");
        assert_eq!(deleted.action, FileAction::Deleted);
        assert!(deleted.lines_removed >= 1);

        let modified = find_change(&changes, "other.txt");
        assert_eq!(modified.action, FileAction::Modified);
        assert_eq!(modified.lines_added, 1);
        assert_eq!(modified.lines_removed, 1);

        let dirty_test = status
            .dirty
            .iter()
            .find(|d| d.filepath == Path::new("test.txt"))
            .expect("dirty entry for test.txt");
        assert!(dirty_test.diff.contains("-initial content"));

        let dirty_other = status
            .dirty
            .iter()
            .find(|d| d.filepath == Path::new("other.txt"))
            .expect("dirty entry for other.txt");
        assert!(dirty_other.diff.contains("-beta"));
        assert!(dirty_other.diff.contains("+gamma"));
    }

    #[test]
    fn get_repo_status_with_changes_resolves_head_once() {
        let (dir, repo) = setup_repo();

        for i in 0..3 {
            let name = format!("file{}.txt", i);
            let path = dir.path().join(&name);
            std::fs::write(&path, format!("content {}\n", i)).unwrap();
            stage_path(dir.path(), &name);
        }

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 3);
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn batched_diff_handles_staged_rename_as_delete_and_add() {
        let (dir, repo) = setup_repo();

        let old_path = dir.path().join("test.txt");
        let new_path = dir.path().join("renamed.txt");
        std::fs::rename(&old_path, &new_path).unwrap();

        let git2_repo = git2::Repository::open(dir.path()).unwrap();
        let mut index = git2_repo.index().unwrap();
        index.remove_path(Path::new("test.txt")).unwrap();
        index.add_path(Path::new("renamed.txt")).unwrap();
        index.write().unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 2);
        assert_eq!(changes.len(), 2);

        let deleted = find_change(&changes, "test.txt");
        assert_eq!(deleted.status, FileStatus::Staged);
        assert_eq!(deleted.action, FileAction::Deleted);
        assert_eq!(deleted.lines_removed, 1);
        assert_eq!(deleted.lines_added, 0);

        let created = find_change(&changes, "renamed.txt");
        assert_eq!(created.status, FileStatus::Staged);
        assert_eq!(created.action, FileAction::Created);
        assert_eq!(created.lines_added, 1);
        assert_eq!(created.lines_removed, 0);
    }

    #[test]
    fn batched_diff_mixed_binary_and_text_deltas() {
        let (dir, repo) = setup_repo();

        let git2_repo = git2::Repository::open(dir.path()).unwrap();
        let binary_path = dir.path().join("data.bin");
        let text_path = dir.path().join("other.txt");
        std::fs::write(&binary_path, b"\x00\x01\x02\x03\n").unwrap();
        std::fs::write(&text_path, "alpha\nbeta\n").unwrap();
        stage_path(dir.path(), "data.bin");
        stage_path(dir.path(), "other.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = git2_repo.index().unwrap().write_tree().unwrap();
        {
            let tree = git2_repo.find_tree(tree_id).unwrap();
            let parent = git2_repo.head().unwrap().peel_to_commit().unwrap();
            git2_repo
                .commit(
                    Some("HEAD"),
                    &sig,
                    &sig,
                    "add binary and text",
                    &tree,
                    &[&parent],
                )
                .unwrap();
        }

        std::fs::write(&binary_path, b"\x00\x01\x02\xFF\n").unwrap();
        std::fs::write(&text_path, "alpha\ngamma\n").unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.unstaged_count, 2);

        let text_change = find_change(&changes, "other.txt");
        assert_eq!(text_change.status, FileStatus::Modified);
        assert_eq!(text_change.action, FileAction::Modified);
        assert_eq!(text_change.lines_added, 1);
        assert_eq!(text_change.lines_removed, 1);

        let binary_change = find_change(&changes, "data.bin");
        assert_eq!(binary_change.status, FileStatus::Modified);
        assert_eq!(binary_change.action, FileAction::Modified);
        // Binary files produce no countable text lines.
        assert_eq!(binary_change.lines_added, 0);
        assert_eq!(binary_change.lines_removed, 0);
    }

    /// git2's `DiffFormat::Patch` rendered the way the prior `aggregate_diff`
    /// produced `DirtyFile.diff`: content-line prefixes plus raw header/hunk
    /// text. This is the byte-for-byte oracle the gix patch must match.
    fn git2_patch(diff: &git2::Diff) -> String {
        let mut out = String::new();
        diff.print(git2::DiffFormat::Patch, |_d, _h, line| {
            let o = line.origin();
            if matches!(o, '+' | '-' | ' ') {
                out.push(o);
            }
            out.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
            true
        })
        .unwrap();
        out
    }

    fn dirty_diff(status: &RepoStatus, path: &str) -> String {
        status
            .dirty
            .iter()
            .find(|d| d.filepath == Path::new(path))
            .unwrap_or_else(|| panic!("no dirty entry for {path}"))
            .diff
            .clone()
    }

    #[test]
    fn patch_parity_unstaged_modify_matches_git2() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("test.txt"), "changed content\n").unwrap();

        let (status, _) = get_repo_status_with_changes(&repo, true).unwrap();
        let actual = dirty_diff(&status, "test.txt");

        let g2 = git2::Repository::open(dir.path()).unwrap();
        let expected = git2_patch(&g2.diff_index_to_workdir(None, None).unwrap());
        assert_eq!(actual, expected);
    }

    #[test]
    fn patch_parity_single_line_hunk_header_matches_git2() {
        // A 1-line region exercises git's `,1`-omitting hunk header (`@@ -1 +1 @@`).
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        let g2 = git2::Repository::init(&path).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        std::fs::write(path.join("one.txt"), "only\n").unwrap();
        let mut index = g2.index().unwrap();
        index.add_path(Path::new("one.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = g2.find_tree(tree_id).unwrap();
            g2.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        std::fs::write(path.join("one.txt"), "changed\n").unwrap();

        let repo = open_gix(&path);
        let (status, _) = get_repo_status_with_changes(&repo, true).unwrap();
        let actual = dirty_diff(&status, "one.txt");
        let expected = git2_patch(&g2.diff_index_to_workdir(None, None).unwrap());
        assert_eq!(actual, expected);
    }

    #[test]
    fn patch_parity_staged_add_matches_git2() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("added.txt"), "one\ntwo\n").unwrap();
        stage_path(dir.path(), "added.txt");

        let (status, _) = get_repo_status_with_changes(&repo, true).unwrap();
        let actual = dirty_diff(&status, "added.txt");

        let g2 = git2::Repository::open(dir.path()).unwrap();
        let head_tree = g2.head().unwrap().peel_to_tree().unwrap();
        let expected = git2_patch(&g2.diff_tree_to_index(Some(&head_tree), None, None).unwrap());
        assert_eq!(actual, expected);
    }

    #[test]
    fn patch_parity_staged_delete_matches_git2() {
        let (dir, repo) = setup_repo();
        std::fs::remove_file(dir.path().join("test.txt")).unwrap();
        stage_delete(dir.path(), "test.txt");

        let (status, _) = get_repo_status_with_changes(&repo, true).unwrap();
        let actual = dirty_diff(&status, "test.txt");

        let g2 = git2::Repository::open(dir.path()).unwrap();
        let head_tree = g2.head().unwrap().peel_to_tree().unwrap();
        let expected = git2_patch(&g2.diff_tree_to_index(Some(&head_tree), None, None).unwrap());
        assert_eq!(actual, expected);
    }

    #[test]
    fn patch_parity_unstaged_binary_modify_matches_git2() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().to_path_buf();
        let g2 = git2::Repository::init(&path).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        std::fs::write(path.join("b.bin"), b"\x00\x01\x02\x03\n").unwrap();
        let mut index = g2.index().unwrap();
        index.add_path(Path::new("b.bin")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = g2.find_tree(tree_id).unwrap();
            g2.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
                .unwrap();
        }
        std::fs::write(path.join("b.bin"), b"\x00\x01\x02\xFF\n").unwrap();

        let repo = open_gix(&path);
        let (status, _) = get_repo_status_with_changes(&repo, true).unwrap();
        let actual = dirty_diff(&status, "b.bin");
        let expected = git2_patch(&g2.diff_index_to_workdir(None, None).unwrap());
        assert_eq!(actual, expected);
    }

    #[test]
    fn is_repo_dirty_clean_repo_is_false() {
        let (_dir, repo) = setup_repo();
        assert!(!is_repo_dirty(&repo).unwrap());
    }

    #[test]
    fn is_repo_dirty_untracked_only_is_true() {
        // gix's own Repository::is_dirty() disables the dirwalk and would miss
        // this; sniff treats untracked files as dirty.
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("new_untracked.txt"), "x\n").unwrap();
        assert!(is_repo_dirty(&repo).unwrap());
    }

    #[test]
    fn is_repo_dirty_staged_change_is_true() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("staged.txt"), "x\n").unwrap();
        stage_path(dir.path(), "staged.txt");
        assert!(is_repo_dirty(&repo).unwrap());
    }

    #[test]
    fn is_repo_dirty_unstaged_change_is_true() {
        let (dir, repo) = setup_repo();
        std::fs::write(dir.path().join("test.txt"), "mutated\n").unwrap();
        assert!(is_repo_dirty(&repo).unwrap());
    }

    #[test]
    fn clean_repo_early_exit_returns_not_dirty() {
        let (_dir, repo) = setup_repo();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(!status.is_dirty);
        assert_eq!(status.staged_count, 0);
        assert_eq!(status.unstaged_count, 0);
        assert_eq!(status.untracked_count, 0);
        assert!(status.dirty.is_empty());
        assert!(status.untracked.is_empty());
        assert!(changes.is_empty());
    }
}
