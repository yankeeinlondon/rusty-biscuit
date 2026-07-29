use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::cache::{CACHE_FORMAT_VERSION, Cache, CacheKey, CacheValue, cache_path};
use crate::error::WorktreeError;
use crate::git::{git_command, git_command_in, repo_info};
use crate::util::dasherize;

#[derive(Debug, Clone)]
pub struct WorktreeEntry {
    /// Absolute path to the worktree
    pub path: PathBuf,
    /// Branch name (or HEAD commit if detached)
    pub branch: Option<String>,
    /// Full HEAD commit SHA, when present in git porcelain output.
    pub head_sha: Option<String>,
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
    /// When the branch already existed and was reused as-is, the short commit it
    /// points at. `None` when a fresh branch was forked from the current HEAD.
    pub reused_branch_at: Option<String>,
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

/// Resolve the current tip SHA for the default branch.
pub fn default_tip_sha(default_branch: &str) -> Result<String, WorktreeError> {
    git_command(&["rev-parse", default_branch])
}

/// Parse `git worktree list --porcelain` output into entries.
pub fn parse_worktree_list(porcelain_output: &str) -> Vec<WorktreeEntry> {
    let cwd = std::env::current_dir().unwrap_or_default();
    let cwd_canonical = std::fs::canonicalize(&cwd).unwrap_or(cwd.clone());

    let mut entries = Vec::new();
    let mut path: Option<PathBuf> = None;
    let mut branch: Option<String> = None;
    let mut head_sha: Option<String> = None;
    let mut is_main = false;
    let mut first = true;

    for line in porcelain_output.lines() {
        if let Some(rest) = line.strip_prefix("worktree ") {
            // Flush previous entry
            if let Some(p) = path.take() {
                let is_current = is_current_worktree(&cwd, &cwd_canonical, &p);
                entries.push(WorktreeEntry {
                    path: p,
                    branch: branch.take(),
                    head_sha: head_sha.take(),
                    is_main,
                    is_current,
                });
            }
            path = Some(PathBuf::from(rest));
            branch = None;
            head_sha = None;
            is_main = first;
            first = false;
        } else if let Some(rest) = line.strip_prefix("HEAD ") {
            head_sha = Some(rest.to_string());
        } else if let Some(rest) = line.strip_prefix("branch ") {
            // refs/heads/main -> main
            branch = Some(rest.strip_prefix("refs/heads/").unwrap_or(rest).to_string());
        }
        // We skip bare, detached, prunable lines.
    }

    // Flush last entry
    if let Some(p) = path {
        let is_current = is_current_worktree(&cwd, &cwd_canonical, &p);
        entries.push(WorktreeEntry {
            path: p,
            branch: branch.take(),
            head_sha: head_sha.take(),
            is_main,
            is_current,
        });
    }

    entries
}

/// Determine whether a worktree path is the current working directory.
///
/// On macOS, system temp directories live under `/var`, which is a symlink to
/// `/private/var`. `current_dir()` resolves the symlink while git stores the
/// unresolved path, so a naive `starts_with` check fails. We compare both the
/// raw and canonicalized forms to handle this.
fn is_current_worktree(cwd: &Path, cwd_canonical: &Path, worktree_path: &Path) -> bool {
    if cwd.starts_with(worktree_path) {
        return true;
    }
    if let Ok(wt_canonical) = std::fs::canonicalize(worktree_path)
        && cwd_canonical.starts_with(&wt_canonical)
    {
        return true;
    }
    false
}

/// A snapshot of the worktree listing for a single invocation.
///
/// Captures the default branch name resolved by [`list_worktrees`] so callers
/// do not need to re-derive it (and re-invoke git) downstream.
#[derive(Debug)]
pub struct WorktreeList {
    pub default_branch: String,
    entries: Vec<WorktreeEntry>,
    default_tip: Option<String>,
    cache_file: Option<PathBuf>,
    pub statuses: Vec<WorktreeStatus>,
}

/// Get status for all worktrees.
///
/// Each worktree's per-entry git work is dispatched in parallel via
/// `std::thread::scope`. Branch comparison uses a SHA-keyed cache when both the
/// default branch tip and worktree HEAD SHA are available. Working-tree
/// dirtiness is always measured live.
pub fn list_worktrees() -> Result<WorktreeList, WorktreeError> {
    let mut list = parse_worktree_state()?;
    fill_worktree_statuses(&mut list)?;
    Ok(list)
}

/// Parse the cheap git state needed before per-worktree status analysis.
pub fn parse_worktree_state() -> Result<WorktreeList, WorktreeError> {
    let porcelain = git_command(&["worktree", "list", "--porcelain"])?;
    let entries = parse_worktree_list(&porcelain);
    let default_branch = default_branch()?;
    let default_tip = default_tip_sha(&default_branch).ok();
    let cache_file = entries
        .first()
        .and_then(|entry| cache_path(&entry.path).ok())
        .filter(|_| default_tip.is_some());
    if let Some(parent) = cache_file.as_ref().and_then(|path| path.parent()) {
        let _ = std::fs::create_dir_all(parent);
    }

    Ok(WorktreeList {
        default_branch,
        entries,
        default_tip,
        cache_file,
        statuses: Vec::new(),
    })
}

/// Populate per-worktree statuses for a parsed worktree state.
pub fn fill_worktree_statuses(list: &mut WorktreeList) -> Result<(), WorktreeError> {
    let cache = Arc::new(Mutex::new(
        list.cache_file
            .as_deref()
            .map(Cache::load_or_default_from)
            .unwrap_or_default(),
    ));

    let entries = list.entries.clone();
    let default_branch = list.default_branch.clone();
    let default_tip = list.default_tip.clone();
    let statuses = std::thread::scope(|scope| {
        let handles: Vec<_> = entries
            .into_iter()
            .map(|entry| {
                let default = default_branch.as_str();
                let default_tip = default_tip.clone();
                let cache = Arc::clone(&cache);
                scope.spawn(move || {
                    let dirty_handle = {
                        let path = entry.path.clone();
                        std::thread::spawn(move || dirty_status(&path))
                    };

                    let (ahead, behind, is_clean) =
                        branch_status_with_cache(default, default_tip.as_deref(), &cache, &entry);

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

    if let Some(path) = list.cache_file.as_ref() {
        let _ = cache
            .lock()
            .expect("cache mutex poisoned")
            .save_atomic(path);
    }

    list.statuses = statuses;
    Ok(())
}

impl WorktreeList {
    /// Worktree entries parsed before status analysis.
    pub fn entries(&self) -> &[WorktreeEntry] {
        &self.entries
    }
}

fn branch_status_with_cache(
    default: &str,
    default_tip: Option<&str>,
    cache: &Arc<Mutex<Cache>>,
    entry: &WorktreeEntry,
) -> (usize, usize, bool) {
    if entry.is_main {
        return (0, 0, true);
    }

    let Some(ref branch) = entry.branch else {
        return (0, 0, true);
    };

    // A non-main branch with no HEAD SHA (porcelain omitted the `HEAD` line)
    // cannot form a cache key, so `key` is `None` below: the cache is skipped
    // and the live branch comparison runs. Turning "no cache key" into a clean
    // (0, 0, true) result would silently report the branch as equivalent to the
    // default branch.
    let key = default_tip
        .zip(entry.head_sha.as_deref())
        .map(|(default_tip_sha, branch_tip_sha)| CacheKey {
            default_tip_sha: default_tip_sha.to_string(),
            branch_tip_sha: branch_tip_sha.to_string(),
            version: CACHE_FORMAT_VERSION,
        });

    if let Some(value) = key
        .as_ref()
        .and_then(|key| cache.lock().expect("cache mutex poisoned").get(key).copied())
    {
        return (value.ahead, value.behind, value.is_clean);
    }

    let (ahead, behind, is_clean) = gather_ahead_behind_clean(default, branch);

    if let Some(key) = key {
        cache.lock().expect("cache mutex poisoned").put(
            key,
            CacheValue {
                ahead,
                behind,
                is_clean,
            },
        );
    }

    (ahead, behind, is_clean)
}

fn gather_ahead_behind_clean(default: &str, branch: &str) -> (usize, usize, bool) {
    // notes: optional per-call timing belongs behind internal instrumentation.
    // Keep public `--perf` reporting aggregated as `list gather`.
    std::thread::scope(|scope| {
        let ahead_behind_handle =
            scope.spawn(|| ahead_behind(default, branch).unwrap_or((0, 0)));
        let clean_merge_handle = scope.spawn(|| check_clean_merge(default, branch));

        let (ahead, behind) = ahead_behind_handle
            .join()
            .expect("ahead_behind thread panicked");
        let speculative_is_clean = clean_merge_handle
            .join()
            .expect("check_clean_merge thread panicked");
        let is_clean = if ahead == 0 || behind == 0 {
            true
        } else {
            speculative_is_clean
        };

        (ahead, behind, is_clean)
    })
}

/// A snapshot of the uncommitted files in a worktree, classified by content kind.
///
/// `paths` are repository-relative (as emitted by `git status --porcelain`); for
/// renames the new path is recorded. `has_source` is true if at least one path
/// classifies as source code under
/// [`sniff::filesystem::path_kind::is_source_code_path`].
#[derive(Debug, Clone, Default)]
pub struct DirtyFiles {
    pub paths: Vec<PathBuf>,
    pub has_source: bool,
}

impl DirtyFiles {
    /// Classify a worktree's `git status --porcelain` output.
    pub fn from_porcelain(porcelain: &str) -> Self {
        let mut paths = Vec::new();
        let mut has_source = false;
        for line in porcelain.lines() {
            let Some(file_path) = porcelain_path(line) else {
                continue;
            };
            let p = PathBuf::from(file_path);
            if !has_source && sniff::filesystem::path_kind::is_source_code_path(&p) {
                has_source = true;
            }
            paths.push(p);
        }
        Self { paths, has_source }
    }

    /// Folded summary equivalent to [`dirty_status`].
    pub fn status(&self) -> DirtyStatus {
        if self.paths.is_empty() {
            DirtyStatus::Clean
        } else if self.has_source {
            DirtyStatus::DirtySource
        } else {
            DirtyStatus::DirtyNonSource
        }
    }
}

/// List uncommitted files for the worktree rooted at `path`.
///
/// Returns an empty [`DirtyFiles`] (clean) if git fails, mirroring
/// [`dirty_status`]'s degraded-mode behavior so callers stay robust.
pub fn list_dirty_files(path: &Path) -> DirtyFiles {
    let Ok(output) = git_command_in(
        path,
        &["-c", "core.untrackedCache=true", "status", "--porcelain"],
    ) else {
        return DirtyFiles::default();
    };
    DirtyFiles::from_porcelain(&output)
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

    let reused_branch_at = if branch_exists {
        git_command(&[
            "worktree",
            "add",
            &target_path.display().to_string(),
            branch,
        ])?;
        // Reusing the branch as-is checks it out wherever it already points — it
        // is NOT forked from the current HEAD. Report the commit so callers can
        // warn about silently resurrecting a stale branch.
        git_command(&["rev-parse", "--short", branch]).ok()
    } else {
        git_command(&[
            "worktree",
            "add",
            &target_path.display().to_string(),
            "-b",
            branch,
        ])?;
        None
    };

    let target_cwd = target_path.join(&info.relative_path);

    Ok(CreateResult {
        worktree_path: target_path,
        target_cwd,
        branch: branch.to_string(),
        reused_branch_at,
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

/// Remove a worktree by absolute path.
///
/// When `force` is true, `git worktree remove --force` is used (drops any
/// uncommitted changes). When false, git's own safety check applies and the
/// command fails if the worktree has uncommitted changes or is locked.
pub fn remove_worktree(path: &std::path::Path, force: bool) -> Result<(), WorktreeError> {
    let path_str = path.display().to_string();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path_str);
    git_command(&args)?;
    Ok(())
}

/// Outcome of a soft branch delete attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeleteBranchOutcome {
    /// `git branch -d <branch>` succeeded.
    Deleted,
    /// `git branch -d` refused (e.g. not merged). The branch was preserved.
    Preserved { reason: String },
}

/// Attempt a soft delete of `branch` (`git branch -d`).
///
/// Soft delete fails if the branch is not fully merged into its upstream or
/// `HEAD`; in that case we report a [`DeleteBranchOutcome::Preserved`] with
/// git's stderr as the reason rather than escalating to `-D`.
pub fn delete_branch(branch: &str) -> DeleteBranchOutcome {
    match git_command(&["branch", "-d", branch]) {
        Ok(_) => DeleteBranchOutcome::Deleted,
        Err(WorktreeError::GitCommand(reason)) => DeleteBranchOutcome::Preserved { reason },
        Err(e) => DeleteBranchOutcome::Preserved {
            reason: e.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use crate::git::recorder;

    use super::*;

    const PORCELAIN_SAMPLE: &str = "\
worktree /Users/ken/code/my-project
HEAD 1111111111111111111111111111111111111111
branch refs/heads/main

worktree /tmp/worktrees/my-project/feature-auth
HEAD 2222222222222222222222222222222222222222
branch refs/heads/feature/auth

worktree /tmp/worktrees/my-project/fix-bug-42
HEAD 3333333333333333333333333333333333333333
branch refs/heads/fix/bug-42
";

    struct DirGuard {
        old: PathBuf,
    }

    impl DirGuard {
        fn enter(dir: &Path) -> Self {
            let old = std::env::current_dir().expect("get cwd");
            std::env::set_current_dir(dir).expect("set cwd");
            Self { old }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.old);
        }
    }

    fn run_git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(repo)
            .args(args)
            .status()
            .expect("git should be installed");
        assert!(status.success(), "git {args:?} failed in {repo:?}");
    }

    fn temp_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path();

        run_git(path, &["init", "-b", "main"]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "Test User"]);
        run_git(path, &["config", "commit.gpgsign", "false"]);
        run_git(path, &["config", "gc.auto", "0"]);
        run_git(path, &["config", "core.fsmonitor", "false"]);
        run_git(path, &["config", "core.commitGraph", "false"]);

        fs::write(path.join("file.txt"), "base\n").expect("write base file");
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "base"]);

        dir
    }

    fn temp_repo_with_diverged_worktrees() -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = temp_repo();
        let repo = dir.path();
        let feature_a = repo
            .parent()
            .expect("temp repo has parent")
            .join(format!("{}-feature-a", repo.file_name().unwrap().to_string_lossy()));
        let feature_b = repo
            .parent()
            .expect("temp repo has parent")
            .join(format!("{}-feature-b", repo.file_name().unwrap().to_string_lossy()));

        run_git(repo, &["branch", "feature-a"]);
        run_git(repo, &["branch", "feature-b"]);

        run_git(repo, &["checkout", "feature-a"]);
        fs::write(repo.join("feature-a.txt"), "feature a\n").expect("write feature a");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "feature a"]);

        run_git(repo, &["checkout", "feature-b"]);
        fs::write(repo.join("feature-b.txt"), "feature b\n").expect("write feature b");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "feature b"]);

        run_git(repo, &["checkout", "main"]);
        fs::write(repo.join("main.txt"), "main\n").expect("write main");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "main"]);

        run_git(repo, &["worktree", "add", feature_a.to_str().unwrap(), "feature-a"]);
        run_git(repo, &["worktree", "add", feature_b.to_str().unwrap(), "feature-b"]);

        (dir, feature_a, feature_b)
    }

    fn temp_repo_with_diverged_and_fast_forward_worktrees()
    -> (tempfile::TempDir, PathBuf, PathBuf) {
        let dir = temp_repo();
        let repo = dir.path();
        let diverged = repo
            .parent()
            .expect("temp repo has parent")
            .join(format!("{}-diverged", repo.file_name().unwrap().to_string_lossy()));
        let fast_forward = repo.parent().expect("temp repo has parent").join(format!(
            "{}-fast-forward",
            repo.file_name().unwrap().to_string_lossy()
        ));

        run_git(repo, &["branch", "diverged"]);
        run_git(repo, &["checkout", "diverged"]);
        fs::write(repo.join("diverged.txt"), "diverged\n").expect("write diverged");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "diverged"]);

        run_git(repo, &["checkout", "main"]);
        fs::write(repo.join("main.txt"), "main\n").expect("write main");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "main"]);

        run_git(repo, &["checkout", "-b", "fast-forward"]);
        fs::write(repo.join("fast-forward.txt"), "fast forward\n").expect("write fast forward");
        run_git(repo, &["add", "."]);
        run_git(repo, &["commit", "-m", "fast forward"]);

        run_git(repo, &["checkout", "main"]);
        run_git(repo, &["worktree", "add", diverged.to_str().unwrap(), "diverged"]);
        run_git(
            repo,
            &[
                "worktree",
                "add",
                fast_forward.to_str().unwrap(),
                "fast-forward",
            ],
        );

        (dir, diverged, fast_forward)
    }

    fn remove_cache_for(repo: &Path) {
        if let Ok(path) = cache_path(repo) {
            let _ = fs::remove_file(path);
        }
    }

    fn status_for_branch<'a>(list: &'a WorktreeList, branch: &str) -> &'a WorktreeStatus {
        list.statuses
            .iter()
            .find(|status| status.entry.branch.as_deref() == Some(branch))
            .unwrap_or_else(|| panic!("missing status for branch {branch}"))
    }

    fn rev_list_count(calls: &[Vec<String>]) -> usize {
        recorder::count_matching(calls, |args| {
            args.first().map(String::as_str) == Some("rev-list")
        })
    }

    fn merge_tree_count(calls: &[Vec<String>]) -> usize {
        recorder::count_matching(calls, |args| {
            args.len() >= 2 && args[0] == "merge-tree" && args[1] == "--write-tree"
        })
    }

    fn rev_list_branch_count(calls: &[Vec<String>], branch: &str) -> usize {
        recorder::count_matching(calls, |args| {
            args.first().map(String::as_str) == Some("rev-list")
                && args
                    .last()
                    .map(|range| range.ends_with(&format!("...{branch}")))
                    .unwrap_or(false)
        })
    }

    fn merge_tree_branch_count(calls: &[Vec<String>], branch: &str) -> usize {
        recorder::count_matching(calls, |args| {
            args.len() >= 4
                && args[0] == "merge-tree"
                && args[1] == "--write-tree"
                && args[3] == branch
        })
    }

    #[test]
    fn parse_porcelain_output() {
        let entries = parse_worktree_list(PORCELAIN_SAMPLE);
        assert_eq!(entries.len(), 3);

        assert!(entries[0].is_main);
        assert_eq!(entries[0].branch.as_deref(), Some("main"));
        assert_eq!(
            entries[0].head_sha.as_deref(),
            Some("1111111111111111111111111111111111111111")
        );

        assert!(!entries[1].is_main);
        assert_eq!(entries[1].branch.as_deref(), Some("feature/auth"));
        assert_eq!(
            entries[1].head_sha.as_deref(),
            Some("2222222222222222222222222222222222222222")
        );

        assert!(!entries[2].is_main);
        assert_eq!(entries[2].branch.as_deref(), Some("fix/bug-42"));
        assert_eq!(
            entries[2].head_sha.as_deref(),
            Some("3333333333333333333333333333333333333333")
        );
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
    #[serial_test::serial]
    fn default_branch_detection() {
        // A remote-less repo has no `refs/remotes/origin/HEAD`, so detection
        // falls through to probing for a local `main`/`master`.
        let repo = temp_repo();
        let _guard = DirGuard::enter(repo.path());

        assert_eq!(default_branch().expect("local main is detectable"), "main");
    }

    #[test]
    #[serial_test::serial]
    fn default_branch_prefers_remote_head_over_local_main() {
        let repo = temp_repo();
        let _guard = DirGuard::enter(repo.path());
        run_git(repo.path(), &["branch", "trunk"]);
        run_git(repo.path(), &["update-ref", "refs/remotes/origin/trunk", "trunk"]);
        run_git(repo.path(), &[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/trunk",
        ]);

        assert_eq!(default_branch().expect("origin/HEAD is detectable"), "trunk");
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_warm_run_skips_rev_list_and_merge_tree() {
        let (repo, _feature_a, _feature_b) = temp_repo_with_diverged_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        let cold = list_worktrees().expect("cold list should succeed");

        recorder::start_recording();
        let warm = list_worktrees().expect("warm list should succeed");
        let calls = recorder::finish_recording();

        assert_eq!(
            rev_list_count(&calls),
            0,
            "warm cache should skip rev-list, got {calls:?}"
        );
        assert_eq!(
            merge_tree_count(&calls),
            0,
            "warm cache should skip merge-tree, got {calls:?}"
        );
        for branch in ["feature-a", "feature-b"] {
            let cold_status = status_for_branch(&cold, branch);
            let warm_status = status_for_branch(&warm, branch);
            assert_eq!(warm_status.ahead, cold_status.ahead);
            assert_eq!(warm_status.behind, cold_status.behind);
            assert_eq!(warm_status.is_clean, cold_status.is_clean);
        }

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_cold_run_invokes_rev_list_and_merge_tree_per_branch() {
        let (repo, _diverged, _fast_forward) = temp_repo_with_diverged_and_fast_forward_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        recorder::start_recording();
        let list = list_worktrees().expect("cold list should succeed");
        let calls = recorder::finish_recording();

        assert_eq!(
            status_for_branch(&list, "diverged").ahead,
            1,
            "diverged branch should be ahead"
        );
        assert_eq!(
            status_for_branch(&list, "diverged").behind,
            1,
            "diverged branch should be behind"
        );
        assert_eq!(
            status_for_branch(&list, "fast-forward").ahead,
            1,
            "fast-forward branch should be ahead"
        );
        assert_eq!(
            status_for_branch(&list, "fast-forward").behind,
            0,
            "fast-forward branch should not be behind"
        );
        assert_eq!(
            rev_list_branch_count(&calls, "diverged"),
            1,
            "diverged branch should run one rev-list, got {calls:?}"
        );
        assert_eq!(
            merge_tree_branch_count(&calls, "diverged"),
            1,
            "diverged branch should run one merge-tree, got {calls:?}"
        );
        assert_eq!(
            rev_list_branch_count(&calls, "fast-forward"),
            1,
            "fast-forward branch should run one rev-list, got {calls:?}"
        );
        assert_eq!(
            merge_tree_branch_count(&calls, "fast-forward"),
            1,
            "fast-forward branch should run speculative merge-tree, got {calls:?}"
        );
        assert_eq!(
            rev_list_branch_count(&calls, "main"),
            0,
            "main branch should not run rev-list, got {calls:?}"
        );
        assert_eq!(
            merge_tree_branch_count(&calls, "main"),
            0,
            "main branch should not run merge-tree, got {calls:?}"
        );

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_fast_forward_branch_discards_merge_tree_result() {
        let (repo, _diverged, _fast_forward) = temp_repo_with_diverged_and_fast_forward_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        recorder::start_recording();
        let list = list_worktrees().expect("cold list should succeed");
        let calls = recorder::finish_recording();
        let fast_forward = status_for_branch(&list, "fast-forward");

        assert_eq!(fast_forward.ahead, 1);
        assert_eq!(fast_forward.behind, 0);
        assert!(fast_forward.is_clean);
        assert_eq!(
            merge_tree_branch_count(&calls, "fast-forward"),
            1,
            "fast-forward branch should still launch speculative merge-tree, got {calls:?}"
        );

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_warm_run_subprocess_count_unchanged() {
        let (repo, _diverged, _fast_forward) = temp_repo_with_diverged_and_fast_forward_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        recorder::start_recording();
        let _ = list_worktrees().expect("cold list should succeed");
        let cold_calls = recorder::finish_recording();

        recorder::start_recording();
        let _ = list_worktrees().expect("warm list should succeed");
        let warm_calls = recorder::finish_recording();

        assert_eq!(
            rev_list_count(&warm_calls),
            0,
            "warm cache should skip rev-list, got {warm_calls:?}"
        );
        assert_eq!(
            merge_tree_count(&warm_calls),
            0,
            "warm cache should skip merge-tree, got {warm_calls:?}"
        );
        assert!(
            warm_calls.len() < cold_calls.len(),
            "warm path should issue fewer git calls than cold path: cold={cold_calls:?}, warm={warm_calls:?}"
        );

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_cold_path_subprocess_counts_match_existing_sla() {
        let (repo, _diverged, _fast_forward) = temp_repo_with_diverged_and_fast_forward_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        recorder::start_recording();
        let _ = list_worktrees().expect("cold list should succeed");
        let calls = recorder::finish_recording();

        assert_eq!(
            rev_list_count(&calls),
            2,
            "cold run should make one rev-list call per non-main branch, got {calls:?}"
        );
        assert_eq!(
            merge_tree_count(&calls),
            2,
            "cold run should make one speculative merge-tree call per non-main branch, got {calls:?}"
        );

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_branch_tip_advance_invalidates_cache_entry() {
        let (repo, feature_a, _feature_b) = temp_repo_with_diverged_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        let cold = list_worktrees().expect("cold list should succeed");
        assert_eq!(status_for_branch(&cold, "feature-a").ahead, 1);

        fs::write(feature_a.join("feature-a-2.txt"), "feature a 2\n").expect("write feature a");
        run_git(&feature_a, &["add", "."]);
        run_git(&feature_a, &["commit", "-m", "feature a 2"]);

        recorder::start_recording();
        let updated = list_worktrees().expect("updated list should succeed");
        let calls = recorder::finish_recording();

        assert!(
            rev_list_count(&calls) >= 1,
            "branch tip move should recompute rev-list, got {calls:?}"
        );
        assert!(
            merge_tree_count(&calls) >= 1,
            "branch tip move should recompute merge-tree, got {calls:?}"
        );
        assert_eq!(status_for_branch(&updated, "feature-a").ahead, 2);

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_default_tip_advance_invalidates_cache_entry() {
        let (repo, _feature_a, _feature_b) = temp_repo_with_diverged_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        let cold = list_worktrees().expect("cold list should succeed");
        assert_eq!(status_for_branch(&cold, "feature-a").behind, 1);

        fs::write(repo.path().join("main-2.txt"), "main 2\n").expect("write main");
        run_git(repo.path(), &["add", "."]);
        run_git(repo.path(), &["commit", "-m", "main 2"]);

        recorder::start_recording();
        let updated = list_worktrees().expect("updated list should succeed");
        let calls = recorder::finish_recording();

        assert!(
            rev_list_count(&calls) >= 1,
            "default tip move should recompute rev-list, got {calls:?}"
        );
        assert!(
            merge_tree_count(&calls) >= 1,
            "default tip move should recompute merge-tree, got {calls:?}"
        );
        assert_eq!(status_for_branch(&updated, "feature-a").behind, 2);

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_missing_head_sha_runs_live_skipping_cache() {
        let (repo, feature_a, _feature_b) = temp_repo_with_diverged_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        // Warm the cache with feature-a's real HEAD. A regression that turns
        // "no cache key" into a clean (0, 0, true) result would surface here as
        // a bogus zero ahead/behind with no git calls instead of a live
        // recompute.
        let _ = list_worktrees().expect("populate cache");
        let default_tip = default_tip_sha("main").expect("default tip");
        let cache = Arc::new(Mutex::new(Cache::load_or_default_from(
            &cache_path(repo.path()).expect("cache path"),
        )));
        let entry = WorktreeEntry {
            path: feature_a,
            branch: Some("feature-a".to_string()),
            head_sha: None,
            is_main: false,
            is_current: false,
        };

        recorder::start_recording();
        let (ahead, behind, _is_clean) =
            branch_status_with_cache("main", Some(&default_tip), &cache, &entry);
        let calls = recorder::finish_recording();

        assert_eq!(
            ahead, 1,
            "missing HEAD SHA must run live ahead/behind, not report a bogus clean state"
        );
        assert_eq!(
            behind, 1,
            "missing HEAD SHA must run live ahead/behind, not report a bogus clean state"
        );
        assert!(
            rev_list_count(&calls) >= 1,
            "missing HEAD SHA should skip the cache but still run live rev-list, got {calls:?}"
        );
        assert!(
            merge_tree_count(&calls) >= 1,
            "missing HEAD SHA should skip the cache but still run live merge-tree, got {calls:?}"
        );

        remove_cache_for(repo.path());
    }

    #[test]
    fn classify_dirty_lines_clean() {
        let dirty = DirtyFiles::from_porcelain("");
        assert!(dirty.paths.is_empty());
        assert!(!dirty.has_source);
        assert_eq!(dirty.status(), DirtyStatus::Clean);
    }

    #[test]
    fn classify_dirty_lines_non_source_only() {
        let porcelain = " M README.md\n?? notes.txt\n";
        let dirty = DirtyFiles::from_porcelain(porcelain);
        assert_eq!(dirty.paths.len(), 2);
        assert!(!dirty.has_source);
        assert_eq!(dirty.status(), DirtyStatus::DirtyNonSource);
    }

    #[test]
    fn classify_dirty_lines_source_present() {
        let porcelain = " M README.md\n M src/lib.rs\n";
        let dirty = DirtyFiles::from_porcelain(porcelain);
        assert_eq!(dirty.paths.len(), 2);
        assert!(dirty.has_source);
        assert_eq!(dirty.status(), DirtyStatus::DirtySource);
    }

    #[test]
    fn classify_dirty_lines_rename() {
        let porcelain = "R  old/foo.rs -> new/foo.rs\n";
        let dirty = DirtyFiles::from_porcelain(porcelain);
        assert_eq!(dirty.paths.len(), 1);
        assert_eq!(dirty.paths[0], std::path::PathBuf::from("new/foo.rs"));
        assert!(dirty.has_source);
    }

    #[test]
    fn delete_branch_outcome_variants_construct() {
        // Smoke-test that the outcome enum is constructable + matchable.
        let merged = DeleteBranchOutcome::Deleted;
        let preserved = DeleteBranchOutcome::Preserved {
            reason: "not fully merged".into(),
        };
        assert!(matches!(merged, DeleteBranchOutcome::Deleted));
        assert!(matches!(preserved, DeleteBranchOutcome::Preserved { .. }));
    }
}
