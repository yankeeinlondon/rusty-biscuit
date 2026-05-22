use std::path::{Path, PathBuf};

use crate::error::WorktreeError;
use crate::git::{git_command, git_command_in, repo_info};
use crate::util::dasherize;

#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    /// Absolute path to the worktree
    pub path: PathBuf,
    /// Branch name (or HEAD commit if detached)
    pub branch: Option<String>,
    /// Whether this is the main/base checkout
    pub is_main: bool,
    /// Whether the worktree is the one the user is currently in
    pub is_current: bool,
}

/// Working-tree dirtiness, classified by file kind.
///
/// Reflects modified, added, deleted, renamed, and untracked files in a
/// worktree's checkout — independent of any merge or branch comparison.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirtyStatus {
    /// No modified, added, deleted, or untracked files.
    Clean,
    /// Dirty files exist, but none are source code (e.g. docs, configs, assets).
    DirtyNonSource,
    /// At least one dirty file is source code.
    DirtySource,
}

#[derive(Debug, Clone)]
pub struct WorktreeStatus {
    pub entry: WorktreeEntry,
    /// Whether the branch can merge cleanly into the default branch
    pub is_clean: bool,
    /// Working-tree dirtiness in this worktree's checkout
    pub dirty: DirtyStatus,
    /// Commits ahead of default branch
    pub ahead: usize,
    /// Commits behind default branch
    pub behind: usize,
}

#[derive(Debug)]
pub struct CreateResult {
    /// Path to the new worktree
    pub worktree_path: PathBuf,
    /// Path the user should cd to (preserving relative position)
    pub target_cwd: PathBuf,
    /// Branch name
    pub branch: String,
}

/// Detect the default branch name (main or master).
pub fn default_branch() -> Result<String, WorktreeError> {
    // Try symbolic-ref for the remote HEAD
    if let Ok(output) = git_command(&["symbolic-ref", "refs/remotes/origin/HEAD"])
        && let Some(branch) = output.strip_prefix("refs/remotes/origin/")
    {
        return Ok(branch.to_string());
    }

    // Fall back to checking if main or master exist
    for candidate in &["main", "master"] {
        if git_command(&["rev-parse", "--verify", candidate]).is_ok() {
            return Ok(candidate.to_string());
        }
    }

    Err(WorktreeError::GitParse(
        "cannot determine default branch".into(),
    ))
}

/// Parse `git worktree list --porcelain` output into entries.
pub fn parse_worktree_list(porcelain_output: &str) -> Vec<WorktreeEntry> {
    let cwd = std::env::current_dir().unwrap_or_default();

    let mut entries = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut is_main = false;
    let mut first = true;

    for line in porcelain_output.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            // Flush previous entry
            if let Some(p) = path.take() {
                let is_current = cwd.starts_with(&p);
                entries.push(WorktreeEntry {
                    path: p,
                    branch: branch.take(),
                    is_main,
                    is_current,
                });
            }
            path = Some(PathBuf::from(rest));
            branch = None;
            is_main = first;
            first = false;
        } else if let Some(rest) = line.strip_prefix("branch ") {
            // refs/heads/main -> main
            branch = Some(rest.strip_prefix("refs/heads/").unwrap_or(rest).to_string());
        }
        // We skip HEAD, bare, detached, prunable lines
    }

    // Flush last entry
    if let Some(p) = path {
        let is_current = cwd.starts_with(&p);
        entries.push(WorktreeEntry {
            path: p,
            branch: branch.take(),
            is_main,
            is_current,
        });
    }

    entries
}

/// Get status for all worktrees.
///
/// Each worktree's per-entry git work is dispatched in parallel via
/// `std::thread::scope`. Within each worktree, `ahead_behind` and `dirty_status`
/// run concurrently on sub-threads. `check_clean_merge` is the most expensive
/// call (`git merge-tree --write-tree` does a real 3-way merge) so it's only
/// invoked when both `ahead > 0` and `behind > 0` — otherwise the merge is
/// trivially a fast-forward and known clean.
pub fn list_worktrees() -> Result<Vec<WorktreeStatus>, WorktreeError> {
    let porcelain = git_command(&["worktree", "list", "--porcelain"])?;
    let entries = parse_worktree_list(&porcelain);
    let default = default_branch()?;

    let statuses = std::thread::scope(|scope| {
        let handles: Vec<_> = entries
            .into_iter()
            .map(|entry| {
                let default = default.as_str();
                scope.spawn(move || {
                    let dirty_handle = {
                        let path = entry.path.clone();
                        std::thread::spawn(move || dirty_status(&path))
                    };

                    let (ahead, behind, is_clean) = if entry.is_main {
                        (0, 0, true)
                    } else if let Some(ref branch) = entry.branch {
                        let (a, b) = ahead_behind(default, branch).unwrap_or((0, 0));
                        // Fast-forward in either direction is trivially clean;
                        // only invoke merge-tree when histories actually diverge.
                        let clean = if a == 0 || b == 0 {
                            true
                        } else {
                            check_clean_merge(default, branch)
                        };
                        (a, b, clean)
                    } else {
                        (0, 0, true)
                    };

                    let dirty = dirty_handle.join().expect("dirty_status thread panicked");

                    WorktreeStatus {
                        entry,
                        is_clean,
                        dirty,
                        ahead,
                        behind,
                    }
                })
            })
            .collect();

        handles
            .into_iter()
            .map(|h| h.join().expect("worktree status thread panicked"))
            .collect::<Vec<_>>()
    });

    Ok(statuses)
}

/// Inspect a worktree's working tree and classify its dirtiness.
///
/// Runs `git status --porcelain` in `path` and partitions changed paths into
/// source-code files (via `sniff::filesystem::path_kind::is_source_code_path`)
/// and everything else. Falls back to [`DirtyStatus::Clean`] on git failure so
/// listing still works in degraded environments.
pub fn dirty_status(path: &Path) -> DirtyStatus {
    // `core.untrackedCache=true` enables git's untracked-files cache (persisted
    // in the worktree's `.git/index`). Walking untracked files in a large
    // monorepo is the dominant cost of `git status`; the cache cuts subsequent
    // calls 4-5x. Passing `-c` here is enough to enable and populate the cache.
    let Ok(output) = git_command_in(
        path,
        &["-c", "core.untrackedCache=true", "status", "--porcelain"],
    ) else {
        return DirtyStatus::Clean;
    };

    let mut any_dirty = false;
    let mut any_source = false;
    for line in output.lines() {
        let Some(file_path) = porcelain_path(line) else {
            continue;
        };
        any_dirty = true;
        if sniff::filesystem::path_kind::is_source_code_path(Path::new(file_path)) {
            any_source = true;
            break;
        }
    }

    if !any_dirty {
        DirtyStatus::Clean
    } else if any_source {
        DirtyStatus::DirtySource
    } else {
        DirtyStatus::DirtyNonSource
    }
}

/// Extract the file path from a `git status --porcelain` line.
///
/// Porcelain v1 format is `XY <path>` (or `XY <orig> -> <new>` for renames),
/// where `XY` is exactly two status characters followed by a single space.
fn porcelain_path(line: &str) -> Option<&str> {
    if line.len() < 4 {
        return None;
    }
    let rest = &line[3..];
    // For renames/copies the new path follows " -> "; classify on the new path.
    if let Some(idx) = rest.find(" -> ") {
        Some(&rest[idx + 4..])
    } else {
        Some(rest)
    }
}

/// Get ahead/behind counts for a branch relative to the default branch.
fn ahead_behind(default_branch: &str, branch: &str) -> Result<(usize, usize), WorktreeError> {
    let range = format!("{default_branch}...{branch}");
    let output = git_command(&["rev-list", "--left-right", "--count", &range]);

    match output {
        Ok(text) => {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() == 2 {
                let behind = parts[0].parse().unwrap_or(0);
                let ahead = parts[1].parse().unwrap_or(0);
                Ok((ahead, behind))
            } else {
                Ok((0, 0))
            }
        }
        Err(_) => Ok((0, 0)),
    }
}

/// Check if a branch can merge cleanly into the default branch.
fn check_clean_merge(default_branch: &str, branch: &str) -> bool {
    // Use git merge-tree (available since git 2.38)
    git_command(&["merge-tree", "--write-tree", default_branch, branch]).is_ok()
}

/// Create a new worktree under `base`.
///
/// The worktree is placed at `{base}/{repo-name}/{dasherized-branch}/`.
/// Callers are responsible for resolving `base` (e.g. via
/// [`worktree::config::resolve_base_dir`] or an interactive prompt).
///
/// ## Errors
///
/// Returns an error if the worktree already exists or git commands fail.
pub fn create_worktree(branch: &str, base: &Path) -> Result<CreateResult, WorktreeError> {
    let info = repo_info()?;

    let dir_name = dasherize(branch);
    let target_path = base.join(&info.name).join(&dir_name);

    if target_path.exists() {
        return Err(WorktreeError::WorktreeAlreadyExists(dir_name));
    }

    // Check if the branch already exists
    let branch_exists = git_command(&["rev-parse", "--verify", branch]).is_ok();

    if branch_exists {
        git_command(&[
            "worktree",
            "add",
            &target_path.display().to_string(),
            branch,
        ])?;
    } else {
        git_command(&[
            "worktree",
            "add",
            &target_path.display().to_string(),
            "-b",
            branch,
        ])?;
    }

    let target_cwd = target_path.join(&info.relative_path);

    Ok(CreateResult {
        worktree_path: target_path,
        target_cwd,
        branch: branch.to_string(),
    })
}

/// Find a worktree by name.
///
/// Matches against the branch name, dasherized directory name, or "base" for the main checkout.
pub fn find_worktree(name: &str) -> Result<WorktreeEntry, WorktreeError> {
    let porcelain = git_command(&["worktree", "list", "--porcelain"])?;
    let entries = parse_worktree_list(&porcelain);

    // "base" matches the main checkout
    if name == "base" {
        return entries
            .into_iter()
            .find(|e| e.is_main)
            .ok_or_else(|| WorktreeError::WorktreeNotFound("base".into()));
    }

    let dasherized_name = dasherize(name);

    for entry in &entries {
        // Match by branch name
        if entry.branch.as_deref() == Some(name) {
            return Ok(entry.clone());
        }

        // Match by dasherized directory name
        if let Some(dir_name) = entry.path.file_name()
            && dir_name.to_string_lossy() == dasherized_name
        {
            return Ok(entry.clone());
        }
    }

    Err(WorktreeError::WorktreeNotFound(name.into()))
}

/// List worktree names for shell completions.
pub fn worktree_names() -> Vec<String> {
    let Ok(porcelain) = git_command(&["worktree", "list", "--porcelain"]) else {
        return vec!["base".to_string()];
    };

    let entries = parse_worktree_list(&porcelain);
    let mut names = vec!["base".to_string()];

    for entry in entries {
        if entry.is_main {
            continue;
        }
        if let Some(branch) = entry.branch {
            names.push(branch);
        } else if let Some(dir) = entry.path.file_name() {
            names.push(dir.to_string_lossy().to_string());
        }
    }

    names
}

/// Remove a worktree by path (used in tests).
pub fn remove_worktree(path: &std::path::Path) -> Result<(), WorktreeError> {
    git_command(&["worktree", "remove", "--force", &path.display().to_string()])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const PORCELAIN_SAMPLE: &str = "\
worktree /Users/ken/code/my-project
HEAD abc123def456
branch refs/heads/main

worktree /tmp/worktrees/my-project/feature-auth
HEAD def456abc789
branch refs/heads/feature/auth

worktree /tmp/worktrees/my-project/fix-bug-42
HEAD 789abcdef012
branch refs/heads/fix/bug-42
";

    #[test]
    fn parse_porcelain_output() {
        let entries = parse_worktree_list(PORCELAIN_SAMPLE);
        assert_eq!(entries.len(), 3);

        assert!(entries[0].is_main);
        assert_eq!(entries[0].branch.as_deref(), Some("main"));

        assert!(!entries[1].is_main);
        assert_eq!(entries[1].branch.as_deref(), Some("feature/auth"));

        assert!(!entries[2].is_main);
        assert_eq!(entries[2].branch.as_deref(), Some("fix/bug-42"));
    }

    #[test]
    fn parse_empty_output() {
        let entries = parse_worktree_list("");
        assert!(entries.is_empty());
    }

    #[test]
    fn porcelain_path_extracts_modified_file() {
        assert_eq!(porcelain_path(" M src/lib.rs"), Some("src/lib.rs"));
        assert_eq!(porcelain_path("?? notes.md"), Some("notes.md"));
        assert_eq!(porcelain_path("A  added.rs"), Some("added.rs"));
    }

    #[test]
    fn porcelain_path_handles_rename() {
        // For renames the porcelain line is `R  old -> new` — we want the new path.
        assert_eq!(
            porcelain_path("R  old/path.rs -> new/path.rs"),
            Some("new/path.rs")
        );
    }

    #[test]
    fn porcelain_path_rejects_short_line() {
        assert_eq!(porcelain_path(""), None);
        assert_eq!(porcelain_path("X"), None);
    }

    #[test]
    fn default_branch_detection() {
        // Should work inside this monorepo
        let result = default_branch();
        assert!(result.is_ok());
        assert!(["main", "master"].contains(&result.unwrap().as_str()));
    }
}
