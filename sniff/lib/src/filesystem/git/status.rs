//! Working-tree status, dirty file, and conflict detection helpers.
//!
//! This module collects per-file status information from the working tree,
//! deriving counts and `FileChange` lists used by higher-level callers. The
//! heavy diff walking is delegated to [`super::diff`].

use git2::{Repository, StatusOptions};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::Result;

use super::diff::{LineStats, aggregate_diff};
use super::discovery::get_commit_refs;
use super::types::*;

/// Gathers repository status including staged, unstaged, and untracked changes.
/// Also returns file changes with their status for rich output.
///
/// Builds the staged and unstaged diffs once for the whole repository and
/// walks each diff a single time to accumulate per-file line stats and (when
/// `include_diffs` is true) per-file unified patch text. This avoids the
/// previous O(dirty_files * diff_setup_cost) scaling of issuing one
/// pathspec-restricted diff per dirty file.
///
/// When `include_diffs` is true, `RepoStatus.dirty` and `RepoStatus.untracked` are
/// populated with full unified diff payloads. When false, those fields are
/// empty `Vec`s and only the cheaper per-file stats (paths, status, line counts)
/// are computed.
pub(crate) fn get_repo_status_with_changes(
    repo: &Repository,
    include_diffs: bool,
) -> Result<(RepoStatus, Vec<FileChange>)> {
    use std::collections::HashSet;

    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    // Recurse into untracked directories to get individual file paths
    opts.recurse_untracked_dirs(true);

    let statuses = repo.statuses(Some(&mut opts))?;

    // Resolve HEAD tree once upfront so the staged diff can be built without
    // repeating tree resolution per file.
    let head_tree = repo.head().and_then(|h| h.peel_to_tree()).ok();

    // Build staged (HEAD -> index) and unstaged (index -> workdir) diffs once
    // for the whole repository, then walk each diff a single time to fill the
    // per-path accumulators below.
    let staged_diff = head_tree
        .as_ref()
        .and_then(|tree| repo.diff_tree_to_index(Some(tree), None, None).ok());
    let unstaged_diff = repo.diff_index_to_workdir(None, None).ok();

    let mut diff_stats: HashMap<PathBuf, LineStats> = HashMap::new();
    let mut staged_patches: HashMap<PathBuf, String> = HashMap::new();
    let mut unstaged_patches: HashMap<PathBuf, String> = HashMap::new();

    if let Some(diff) = staged_diff.as_ref() {
        let patch_sink = include_diffs.then_some(&mut staged_patches);
        aggregate_diff(diff, &mut diff_stats, patch_sink)?;
    }
    if let Some(diff) = unstaged_diff.as_ref() {
        let patch_sink = include_diffs.then_some(&mut unstaged_patches);
        aggregate_diff(diff, &mut diff_stats, patch_sink)?;
    }

    let mut staged = 0;
    let mut unstaged = 0;
    let mut untracked_count = 0;

    // Use HashSet for O(1) deduplication instead of Vec::contains which is O(n)
    let mut dirty_set: HashSet<PathBuf> = HashSet::new();
    let mut untracked_paths: Vec<PathBuf> = Vec::new();
    let mut file_changes: Vec<FileChange> = Vec::new();

    for entry in statuses.iter() {
        let status = entry.status();
        let path = entry.path().map(PathBuf::from);

        let is_staged =
            status.is_index_new() || status.is_index_modified() || status.is_index_deleted();
        let is_unstaged = status.is_wt_modified() || status.is_wt_deleted();
        let is_untracked = status.is_wt_new();

        // Determine the specific action for staged and unstaged changes
        let staged_action = if status.is_index_new() {
            Some(FileAction::Created)
        } else if status.is_index_deleted() {
            Some(FileAction::Deleted)
        } else if status.is_index_modified() {
            Some(FileAction::Modified)
        } else {
            None
        };

        let unstaged_action = if status.is_wt_deleted() {
            Some(FileAction::Deleted)
        } else if status.is_wt_modified() {
            Some(FileAction::Modified)
        } else {
            None
        };

        if is_staged {
            staged += 1;
        }
        if is_unstaged {
            unstaged += 1;
        }
        if is_untracked {
            untracked_count += 1;
            if let Some(ref p) = path {
                untracked_paths.push(p.clone());
                file_changes.push(FileChange {
                    path: p.clone(),
                    status: FileStatus::Untracked,
                    action: FileAction::Created,
                    lines_added: 0,
                    lines_removed: 0,
                });
            }
        }

        // Add to dirty set if staged or unstaged (but not untracked)
        if let Some(ref p) = path
            && !is_untracked
        {
            let LineStats {
                added: lines_added,
                removed: lines_removed,
            } = diff_stats.get(p).copied().unwrap_or_default();

            let (file_status, action) = if is_staged && is_unstaged {
                (
                    FileStatus::Both,
                    staged_action.unwrap_or(FileAction::Modified),
                )
            } else if is_staged {
                (
                    FileStatus::Staged,
                    staged_action.unwrap_or(FileAction::Modified),
                )
            } else if is_unstaged {
                (
                    FileStatus::Modified,
                    unstaged_action.unwrap_or(FileAction::Modified),
                )
            } else {
                continue;
            };

            file_changes.push(FileChange {
                path: p.clone(),
                status: file_status,
                action,
                lines_added,
                lines_removed,
            });
            dirty_set.insert(p.clone());
        }
    }

    let conflicted_paths = detect_merge_conflicts(repo);
    if !conflicted_paths.is_empty() {
        let conflicted_set: HashSet<_> = conflicted_paths.iter().cloned().collect();
        file_changes.retain(|change| !conflicted_set.contains(&change.path));

        let conflicted_changes = conflicted_paths.into_iter().map(|path| FileChange {
            path,
            status: FileStatus::Conflicted,
            action: FileAction::Modified,
            lines_added: 0,
            lines_removed: 0,
        });

        file_changes = conflicted_changes.chain(file_changes).collect();
    }

    // Convert HashSet to Vec for downstream processing
    let dirty_paths: Vec<PathBuf> = dirty_set.into_iter().collect();

    // Get HEAD commit SHA and upstream commit
    let (head_sha, origin_commit) = get_commit_refs(repo);

    // Get repository root for absolute paths
    let repo_root = repo.workdir().map(Path::to_path_buf);

    // Build dirty file details with diffs (only when requested). The patch
    // strings were captured in the single diff walk above, so this does not
    // re-run any libgit2 diff machinery.
    let dirty = if include_diffs {
        build_dirty_files_from_patches(
            &dirty_paths,
            &staged_patches,
            &unstaged_patches,
            &head_sha,
            &origin_commit,
            &repo_root,
        )
    } else {
        Vec::new()
    };

    // Build untracked file details (only when requested)
    let untracked = if include_diffs {
        build_untracked_files(&untracked_paths, &repo_root)
    } else {
        Vec::new()
    };

    let repo_status = RepoStatus {
        is_dirty: staged > 0
            || unstaged > 0
            || untracked_count > 0
            || file_changes
                .iter()
                .any(|change| change.status == FileStatus::Conflicted),
        staged_count: staged,
        unstaged_count: unstaged,
        untracked_count,
        dirty,
        untracked,
        is_behind: None, // Populated by detect_git when deep=true
    };

    Ok((repo_status, file_changes))
}

/// Assemble per-file `DirtyFile` entries from the staged and unstaged patch
/// strings collected by [`aggregate_diff`].
fn build_dirty_files_from_patches(
    paths: &[PathBuf],
    staged_patches: &HashMap<PathBuf, String>,
    unstaged_patches: &HashMap<PathBuf, String>,
    head_sha: &str,
    origin_commit: &Option<String>,
    repo_root: &Option<PathBuf>,
) -> Vec<DirtyFile> {
    paths
        .iter()
        .map(|filepath| {
            let mut diff = String::new();
            if let Some(staged) = staged_patches.get(filepath)
                && !staged.is_empty()
            {
                diff.push_str(staged);
            }
            if let Some(unstaged) = unstaged_patches.get(filepath)
                && !unstaged.is_empty()
            {
                if !diff.is_empty() {
                    diff.push('\n');
                }
                diff.push_str(unstaged);
            }

            let absolute_filepath = repo_root
                .as_ref()
                .map(|root| root.join(filepath))
                .unwrap_or_else(|| filepath.clone());

            DirtyFile {
                filepath: filepath.clone(),
                absolute_filepath,
                diff,
                last_local_commit: head_sha.to_string(),
                origin_commit: origin_commit.clone(),
            }
        })
        .collect()
}

/// Builds detailed information for untracked files.
fn build_untracked_files(paths: &[PathBuf], repo_root: &Option<PathBuf>) -> Vec<UntrackedFile> {
    paths
        .iter()
        .map(|filepath| {
            let absolute_filepath = repo_root
                .as_ref()
                .map(|root| root.join(filepath))
                .unwrap_or_else(|| filepath.clone());

            UntrackedFile {
                filepath: filepath.clone(),
                absolute_filepath,
            }
        })
        .collect()
}

/// Lightweight status check that only counts files by category.
///
/// Avoids the cost of per-file diff stat computation and unified diff
/// generation. Use this when you only need `is_dirty` and file counts.
pub(crate) fn get_repo_status_counts(repo: &Repository) -> (bool, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return (false, 0),
    };

    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged += 1;
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
    }

    let total = staged + unstaged + untracked;
    (total > 0, total)
}

/// Lightweight status check returning individual category counts.
pub(crate) fn get_repo_status_counts_detailed(repo: &Repository) -> (bool, usize, usize, usize) {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);

    let statuses = match repo.statuses(Some(&mut opts)) {
        Ok(s) => s,
        Err(_) => return (false, 0, 0, 0),
    };

    let mut staged = 0usize;
    let mut unstaged = 0usize;
    let mut untracked = 0usize;

    for entry in statuses.iter() {
        let status = entry.status();
        if status.is_index_new() || status.is_index_modified() || status.is_index_deleted() {
            staged += 1;
        }
        if status.is_wt_modified() || status.is_wt_deleted() {
            unstaged += 1;
        }
        if status.is_wt_new() {
            untracked += 1;
        }
    }

    let is_dirty = staged > 0 || unstaged > 0 || untracked > 0;
    (is_dirty, staged, unstaged, untracked)
}

/// Detect unmerged (conflicted) files in the repository index.
///
/// Returns the relative paths of files that have merge conflict markers
/// in the index (i.e., are in an unmerged state from a merge, rebase,
/// cherry-pick, or revert).
pub fn detect_merge_conflicts(repo: &Repository) -> Vec<PathBuf> {
    let index = match repo.index() {
        Ok(idx) => idx,
        Err(_) => return Vec::new(),
    };

    let conflicts = match index.conflicts() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let mut conflicted = Vec::new();
    for entry in conflicts {
        let Ok(conflict) = entry else {
            continue;
        };
        let path = conflict
            .our
            .as_ref()
            .or(conflict.their.as_ref())
            .or(conflict.ancestor.as_ref());
        if let Some(entry) = path {
            let path = PathBuf::from(std::str::from_utf8(&entry.path).unwrap_or_default());
            if !conflicted.contains(&path) {
                conflicted.push(path);
            }
        }
    }

    conflicted
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// Creates a temporary git repo with a single file committed.
    fn setup_repo() -> (TempDir, Repository) {
        let dir = TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        // Create initial file and commit
        let file_path = dir.path().join("test.txt");
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

        (dir, repo)
    }

    /// Stage a path (`git add <path>`).
    fn stage_path(repo: &Repository, relative: &str) {
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(relative)).unwrap();
        index.write().unwrap();
    }

    /// Stage a deletion (`git rm <path>` equivalent at the index level).
    fn stage_delete(repo: &Repository, relative: &str) {
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

        // Modify the file (unstaged change only)
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

        // Stage one modification, then add additional unstaged edits.
        std::fs::write(dir.path().join("test.txt"), "staged content\n").unwrap();
        stage_path(&repo, "test.txt");
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
        // Combined totals must match the legacy per-file path.
        assert_eq!(change.lines_added, 3);
        assert_eq!(change.lines_removed, 2);
    }

    #[test]
    fn batched_diff_handles_staged_deletes() {
        let (dir, repo) = setup_repo();

        // Stage deletion of the committed file.
        std::fs::remove_file(dir.path().join("test.txt")).unwrap();
        stage_delete(&repo, "test.txt");

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 1);
        assert_eq!(status.unstaged_count, 0);

        let change = find_change(&changes, "test.txt");
        assert_eq!(change.status, FileStatus::Staged);
        assert_eq!(change.action, FileAction::Deleted);
        // The deleted file had a single line, so the diff records exactly one removal.
        assert_eq!(change.lines_added, 0);
        assert_eq!(change.lines_removed, 1);
    }

    #[test]
    fn batched_diff_handles_unstaged_deletes_with_concurrent_modify() {
        let (dir, repo) = setup_repo();

        // Add a second file and commit so we can mix delete + modify cases.
        std::fs::write(dir.path().join("other.txt"), "alpha\nbeta\n").unwrap();
        stage_path(&repo, "other.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&parent])
                .unwrap();
        }

        // Unstaged delete of `test.txt` and unstaged modify of `other.txt`.
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

        // include_diffs=true must emit per-file unified patches assembled from
        // the same batched diff walk.
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

        // Create and stage multiple files
        for i in 0..3 {
            let name = format!("file{}.txt", i);
            let path = dir.path().join(&name);
            std::fs::write(&path, format!("content {}\n", i)).unwrap();
            stage_path(&repo, &name);
        }

        // This should work without error and resolve HEAD only once
        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 3);
        assert_eq!(changes.len(), 3);
    }

    #[test]
    fn batched_diff_handles_staged_rename_as_delete_and_add() {
        let (dir, repo) = setup_repo();

        // Rename the committed file on disk and stage the change.
        let old_path = dir.path().join("test.txt");
        let new_path = dir.path().join("renamed.txt");
        std::fs::rename(&old_path, &new_path).unwrap();

        let mut index = repo.index().unwrap();
        index.remove_path(Path::new("test.txt")).unwrap();
        index.add_path(Path::new("renamed.txt")).unwrap();
        index.write().unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.staged_count, 2);
        // One delete + one add because rename detection is off by default.
        assert_eq!(changes.len(), 2);

        let deleted = find_change(&changes, "test.txt");
        assert_eq!(deleted.status, FileStatus::Staged);
        assert_eq!(deleted.action, FileAction::Deleted);
        assert_eq!(deleted.lines_removed, 1);
        assert_eq!(deleted.lines_added, 0);

        let created = find_change(&changes, "renamed.txt");
        assert_eq!(created.status, FileStatus::Staged);
        assert_eq!(created.action, FileAction::Created);
        // The added side of the rename carries the original line content.
        assert_eq!(created.lines_added, 1);
        assert_eq!(created.lines_removed, 0);
    }

    #[test]
    fn batched_diff_mixed_binary_and_text_deltas() {
        let (dir, repo) = setup_repo();

        // Add a binary file and a second text file, then commit.
        let binary_path = dir.path().join("data.bin");
        let text_path = dir.path().join("other.txt");
        std::fs::write(&binary_path, b"\x00\x01\x02\x03\n").unwrap();
        std::fs::write(&text_path, "alpha\nbeta\n").unwrap();
        stage_path(&repo, "data.bin");
        stage_path(&repo, "other.txt");
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let tree_id = repo.index().unwrap().write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let parent = repo.head().unwrap().peel_to_commit().unwrap();
            repo.commit(
                Some("HEAD"),
                &sig,
                &sig,
                "add binary and text",
                &tree,
                &[&parent],
            )
            .unwrap();
        }

        // Modify both files in the working tree (unstaged).
        std::fs::write(&binary_path, b"\x00\x01\x02\xFF\n").unwrap();
        std::fs::write(&text_path, "alpha\ngamma\n").unwrap();

        let (status, changes) = get_repo_status_with_changes(&repo, false).unwrap();

        assert!(status.is_dirty);
        assert_eq!(status.unstaged_count, 2);

        let text_change = find_change(&changes, "other.txt");
        assert_eq!(text_change.status, FileStatus::Modified);
        assert_eq!(text_change.action, FileAction::Modified);
        // text delta: 1 add, 1 remove
        assert_eq!(text_change.lines_added, 1);
        assert_eq!(text_change.lines_removed, 1);

        let binary_change = find_change(&changes, "data.bin");
        assert_eq!(binary_change.status, FileStatus::Modified);
        assert_eq!(binary_change.action, FileAction::Modified);
        // Binary files produce no countable text lines.
        assert_eq!(binary_change.lines_added, 0);
        assert_eq!(binary_change.lines_removed, 0);
    }
}
