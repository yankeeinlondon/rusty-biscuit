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

    for path in &dirty_paths {
        if let Some(kind) = staged.get(path) {
            let stats = staged_diff_stats(repo, path.as_bstr(), *kind);
            let patch = if include_diffs {
                staged_diff_patch(repo, path.as_bstr(), *kind)
            } else {
                String::new()
            };
            diff_stats
                .entry(path.clone())
                .and_modify(|s| *s = *s + stats)
                .or_insert(stats);
            if !patch.is_empty() {
                staged_patches.insert(path.clone(), patch);
            }
        }
        if let Some(kind) = unstaged.get(path) {
            let stats = unstaged_diff_stats(repo, path.as_bstr(), *kind, workdir.as_deref());
            let patch = if include_diffs {
                unstaged_diff_patch(repo, path.as_bstr(), *kind, workdir.as_deref())
            } else {
                String::new()
            };
            diff_stats
                .entry(path.clone())
                .and_modify(|s| *s = *s + stats)
                .or_insert(stats);
            if !patch.is_empty() {
                unstaged_patches.insert(path.clone(), patch);
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

/// Compute line stats for a staged change (HEAD tree vs index).
fn staged_diff_stats(
    repo: &gix::Repository,
    path: &gix::bstr::BStr,
    kind: StagedKind,
) -> LineStats {
    let Ok(head_tree_id) = repo.head_tree_id_or_empty() else {
        return LineStats::default();
    };
    let head_tree = match repo.find_tree(head_tree_id) {
        Ok(t) => t,
        Err(_) => return LineStats::default(),
    };
    let index = match repo.index_or_empty() {
        Ok(idx) => idx,
        Err(_) => return LineStats::default(),
    };

    match kind {
        StagedKind::Added => {
            let entry = match index_entry_by_path(&index, path) {
                Some(e) => e,
                None => return LineStats::default(),
            };
            LineStats {
                added: line_count_of_blob_content(repo, entry.id),
                removed: 0,
            }
        }
        StagedKind::Deleted => {
            let entry = match head_tree.find_entry(gix::bstr::BString::from(path.to_vec())) {
                Some(e) => e,
                None => return LineStats::default(),
            };
            LineStats {
                added: 0,
                removed: line_count_of_blob_content(repo, entry.id().into()),
            }
        }
        StagedKind::Modified => {
            let head_entry = head_tree.find_entry(gix::bstr::BString::from(path.to_vec()));
            let index_entry = index_entry_by_path(&index, path);
            match (head_entry, index_entry) {
                (Some(h), Some(i)) => diff_blobs(repo, h.id().into(), i.id),
                _ => LineStats::default(),
            }
        }
    }
}

/// Compute line stats for an unstaged change (index vs worktree).
fn unstaged_diff_stats(
    repo: &gix::Repository,
    path: &gix::bstr::BStr,
    kind: UnstagedKind,
    workdir: Option<&Path>,
) -> LineStats {
    let index = match repo.index_or_empty() {
        Ok(idx) => idx,
        Err(_) => return LineStats::default(),
    };
    let index_entry = index_entry_by_path(&index, path);

    match kind {
        UnstagedKind::Deleted => {
            let Some(entry) = index_entry else {
                return LineStats::default();
            };
            LineStats {
                added: 0,
                removed: line_count_of_blob_content(repo, entry.id),
            }
        }
        UnstagedKind::Modified => {
            let Some(entry) = index_entry else {
                return LineStats::default();
            };
            let Some(workdir) = workdir else {
                return LineStats::default();
            };
            let worktree_content = match std::fs::read(workdir.join(bstr_to_path(path))) {
                Ok(c) => c,
                Err(_) => return LineStats::default(),
            };
            diff_blob_against_bytes(repo, entry.id, &worktree_content)
        }
    }
}

/// Build a unified patch for a staged change.
fn staged_diff_patch(repo: &gix::Repository, path: &gix::bstr::BStr, kind: StagedKind) -> String {
    let Ok(head_tree_id) = repo.head_tree_id_or_empty() else {
        return String::new();
    };
    let head_tree = match repo.find_tree(head_tree_id) {
        Ok(t) => t,
        Err(_) => return String::new(),
    };
    let index = match repo.index_or_empty() {
        Ok(idx) => idx,
        Err(_) => return String::new(),
    };

    match kind {
        StagedKind::Added => {
            let Some(entry) = index_entry_by_path(&index, path) else {
                return String::new();
            };
            let new = read_blob_or_empty(repo, entry.id);
            git_patch(
                path,
                PatchOp::Added,
                None,
                Some(entry.id),
                &index_mode(entry),
                &[],
                &new,
            )
        }
        StagedKind::Deleted => {
            let Some(entry) = head_tree.find_entry(gix::bstr::BString::from(path.to_vec())) else {
                return String::new();
            };
            let old = read_blob_or_empty(repo, entry.id());
            let mut buf = [0u8; 6];
            let mode = String::from_utf8_lossy(entry.mode().as_bytes(&mut buf)).into_owned();
            git_patch(
                path,
                PatchOp::Deleted,
                Some(entry.id().into()),
                None,
                &mode,
                &old,
                &[],
            )
        }
        StagedKind::Modified => {
            let Some(head_entry) = head_tree.find_entry(gix::bstr::BString::from(path.to_vec()))
            else {
                return String::new();
            };
            let Some(index_entry) = index_entry_by_path(&index, path) else {
                return String::new();
            };
            let old = read_blob_or_empty(repo, head_entry.id());
            let new = read_blob_or_empty(repo, index_entry.id);
            git_patch(
                path,
                PatchOp::Modified,
                Some(head_entry.id().into()),
                Some(index_entry.id),
                &index_mode(index_entry),
                &old,
                &new,
            )
        }
    }
}

/// Build a unified patch for an unstaged change.
fn unstaged_diff_patch(
    repo: &gix::Repository,
    path: &gix::bstr::BStr,
    kind: UnstagedKind,
    workdir: Option<&Path>,
) -> String {
    let index = match repo.index_or_empty() {
        Ok(idx) => idx,
        Err(_) => return String::new(),
    };
    let Some(index_entry) = index_entry_by_path(&index, path) else {
        return String::new();
    };

    match kind {
        UnstagedKind::Deleted => {
            let old = read_blob_or_empty(repo, index_entry.id);
            git_patch(
                path,
                PatchOp::Deleted,
                Some(index_entry.id),
                None,
                &index_mode(index_entry),
                &old,
                &[],
            )
        }
        UnstagedKind::Modified => {
            let Some(workdir) = workdir else {
                return String::new();
            };
            let new = std::fs::read(workdir.join(bstr_to_path(path))).unwrap_or_default();
            let old = read_blob_or_empty(repo, index_entry.id);
            let new_id = blob_oid(repo, &new);
            git_patch(
                path,
                PatchOp::Modified,
                Some(index_entry.id),
                new_id,
                &index_mode(index_entry),
                &old,
                &new,
            )
        }
    }
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

/// Look up an index entry by its repository-relative byte path.
fn index_entry_by_path<'a>(
    index: &'a gix::worktree::Index,
    path: &gix::bstr::BStr,
) -> Option<&'a gix::index::Entry> {
    index.entry_by_path_and_stage(path, gix::index::entry::Stage::Unconflicted)
}

/// Read a blob's content, returning an empty vec on failure.
fn read_blob_or_empty(repo: &gix::Repository, id: impl Into<gix::ObjectId>) -> Vec<u8> {
    repo.find_blob(id.into())
        .map(|mut b| b.take_data())
        .unwrap_or_default()
}

/// Count lines in a blob content, returning just the line count.
///
/// Returns zero for binary content so add/delete line stats stay consistent
/// with Git's "binary files differ" behavior.
fn line_count_of_blob_content(repo: &gix::Repository, id: gix::ObjectId) -> usize {
    let content = read_blob_or_empty(repo, id);
    if is_binary(&content) {
        0
    } else {
        byte_lines(&content)
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

/// Diff two blobs and return line stats.
fn diff_blobs(repo: &gix::Repository, old_id: gix::ObjectId, new_id: gix::ObjectId) -> LineStats {
    let old = read_blob_or_empty(repo, old_id);
    let new = read_blob_or_empty(repo, new_id);
    diff_bytes(&old, &new)
}

/// Diff an index blob against worktree bytes.
fn diff_blob_against_bytes(repo: &gix::Repository, old_id: gix::ObjectId, new: &[u8]) -> LineStats {
    let old = read_blob_or_empty(repo, old_id);
    diff_bytes(&old, new)
}

/// Compute line stats by diffing two byte buffers.
///
/// Returns zero counts if either buffer appears to be binary (contains a null
/// byte in the first 8 KB), matching Git's heuristic and the prior git2-based
/// behavior.
fn diff_bytes(old: &[u8], new: &[u8]) -> LineStats {
    if is_binary(old) || is_binary(new) {
        return LineStats::default();
    }
    let input = gix::diff::blob::InternedInput::new(old, new);
    let diff =
        gix::diff::blob::diff_with_slider_heuristics(gix::diff::blob::Algorithm::Histogram, &input);
    LineStats {
        added: diff.count_additions() as usize,
        removed: diff.count_removals() as usize,
    }
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
/// `---`/`+++` markers a `git diff` produces, followed by the unified hunks.
///
/// Binary content yields the `Binary files … differ` line instead of hunks,
/// matching libgit2. `mode` is the 6-digit octal blob mode (e.g. `100644`).
fn git_patch(
    path: &gix::bstr::BStr,
    op: PatchOp,
    old_id: Option<gix::ObjectId>,
    new_id: Option<gix::ObjectId>,
    mode: &str,
    old: &[u8],
    new: &[u8],
) -> String {
    use std::fmt::Write;
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
    s.push_str(&text_hunks(old, new));
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

/// Render only the unified hunks (`@@` headers + content lines) of a diff
/// between two byte buffers, without the file-level header.
fn text_hunks(old: &[u8], new: &[u8]) -> String {
    if old == new {
        return String::new();
    }
    let input = gix::diff::blob::InternedInput::new(old, new);
    let diff =
        gix::diff::blob::diff_with_slider_heuristics(gix::diff::blob::Algorithm::Histogram, &input);

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
        &diff,
        &input,
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
