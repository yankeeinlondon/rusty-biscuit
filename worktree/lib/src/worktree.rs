use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use crate::cache::{Cache, cache_path};
use crate::default_target::{DefaultTarget, choose_default_target};
use crate::error::WorktreeError;
use crate::fork_origin::ForkOriginStore;
use crate::listing::{
    BranchComparisons, Caption, ParentComparison, RefTips, TreeRow, build_tree, compare_cached,
};
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
    /// Working-tree dirtiness in this worktree's checkout
    pub dirty: DirtyStatus,
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
    /// points at. `None` when a fresh branch was forked.
    pub reused_branch_at: Option<String>,
    /// The local branch a fresh branch was forked from. `None` when an
    /// existing branch was reused.
    pub forked_from: Option<String>,
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
/// [`parse_worktree_state`] fills the cheap facts (entries, default branch,
/// ref tips, fork-origin records) and [`fill_worktree_statuses`] the rest, so
/// callers can start other work from the parsed state while the expensive
/// per-worktree pass runs.
#[derive(Debug)]
pub struct WorktreeList {
    pub default_branch: String,
    entries: Vec<WorktreeEntry>,
    refs: RefTips,
    /// `for-each-ref` succeeded, so a branch missing from `refs` is deleted.
    refs_read: bool,
    forks: ForkOriginStore,
    fork_file: Option<PathBuf>,
    cache_file: Option<PathBuf>,
    /// One per entry, in entry order.
    pub statuses: Vec<WorktreeStatus>,
    /// The tip the `-> {default}` column compares against.
    pub target: Option<DefaultTarget>,
    /// The local default branch against `origin/<default>`; `None` without
    /// both refs.
    pub caption: Option<Caption>,
    /// The Branch column's rows, in display order.
    pub tree: Vec<TreeRow>,
    /// Keyed by branch name, for every existing non-default branch in the tree.
    pub comparisons: HashMap<String, BranchComparisons>,
}

/// Get status for all worktrees.
///
/// Dirty status is always measured live. Branch comparisons go through the
/// SHA-pair cache (see [`crate::cache`]).
pub fn list_worktrees() -> Result<WorktreeList, WorktreeError> {
    let mut list = parse_worktree_state()?;
    fill_worktree_statuses(&mut list)?;
    Ok(list)
}

/// Parse the cheap git state needed before per-worktree status analysis:
/// `worktree list`, the default branch, one `for-each-ref`, and the
/// fork-origin records.
pub fn parse_worktree_state() -> Result<WorktreeList, WorktreeError> {
    let porcelain = git_command(&["worktree", "list", "--porcelain"])?;
    let entries = parse_worktree_list(&porcelain);
    let default_branch = default_branch()?;
    let refs = RefTips::read();
    let refs_read = refs.is_some();
    let main_path = entries.first().map(|entry| entry.path.clone());
    let cache_file = main_path.as_deref().and_then(|path| cache_path(path).ok());
    if let Some(parent) = cache_file.as_ref().and_then(|path| path.parent()) {
        let _ = std::fs::create_dir_all(parent);
    }
    let fork_file = main_path
        .as_deref()
        .and_then(|path| crate::fork_origin::fork_origin_path(path).ok());
    let forks = fork_file
        .as_deref()
        .map(ForkOriginStore::load_from)
        .unwrap_or_default();

    Ok(WorktreeList {
        default_branch,
        entries,
        refs: refs.unwrap_or_default(),
        refs_read,
        forks,
        fork_file,
        cache_file,
        statuses: Vec::new(),
        target: None,
        caption: None,
        tree: Vec::new(),
        comparisons: HashMap::new(),
    })
}

/// Populate dirty status, the caption, the default-branch target, the fork
/// tree, and every branch comparison for a parsed worktree state.
///
/// Records of deleted branches are pruned from the fork-origin store here.
pub fn fill_worktree_statuses(list: &mut WorktreeList) -> Result<(), WorktreeError> {
    let cache = Mutex::new(
        list.cache_file
            .as_deref()
            .map(Cache::load_or_default_from)
            .unwrap_or_default(),
    );
    let refs = list.refs.clone();
    let default_branch = list.default_branch.clone();
    let entries = list.entries.clone();

    let tree = build_tree(&entries, &default_branch, &refs.local, &list.forks);

    let (statuses, caption, target, comparisons) = std::thread::scope(|scope| {
        let dirty_handles: Vec<_> = entries
            .iter()
            .map(|entry| {
                let path = entry.path.clone();
                scope.spawn(move || dirty_status(&path))
            })
            .collect();

        let local_tip = refs.local(&default_branch).map(str::to_string);
        let remote_name = format!("origin/{default_branch}");
        let remote_tip = refs.remote(&remote_name).map(str::to_string);
        let caption = local_tip.as_deref().zip(remote_tip.as_deref()).and_then(|(local, remote)| {
            compare_cached(&cache, remote, local).map(|comparison| Caption {
                local: default_branch.clone(),
                remote: remote_name.clone(),
                ahead: comparison.ahead,
                behind: comparison.behind,
            })
        });
        let target = choose_default_target(&default_branch, local_tip, remote_tip, |ancestor, descendant| {
            match &caption {
                // The caption compared the local tip (branch) with the remote
                // tip (target): the remote contains the local tip when the
                // local side is 0 ahead, and vice versa.
                Some(caption) if refs.remote(&remote_name) == Some(ancestor) => caption.behind == 0,
                Some(caption) if refs.remote(&remote_name) == Some(descendant) => caption.ahead == 0,
                _ => git_command(&["merge-base", "--is-ancestor", ancestor, descendant]).is_ok(),
            }
        });

        let comparisons = compare_tree(&tree, &refs, &default_branch, target.as_ref(), &cache, scope);

        let statuses: Vec<WorktreeStatus> = entries
            .iter()
            .cloned()
            .zip(dirty_handles)
            .map(|(entry, handle)| WorktreeStatus {
                entry,
                dirty: handle.join().expect("dirty_status thread panicked"),
            })
            .collect();
        (statuses, caption, target, comparisons)
    });

    if let Some(path) = list.cache_file.as_ref() {
        let _ = cache.lock().expect("cache mutex poisoned").save_atomic(path);
    }
    // Without a successful `for-each-ref` every branch would look deleted.
    if let (true, Some(path)) = (list.refs_read, list.fork_file.as_ref()) {
        let live: HashSet<String> = list.refs.local.keys().cloned().collect();
        if list.forks.prune(&live) > 0 {
            let _ = list.forks.save_atomic(path);
        }
    }

    list.statuses = statuses;
    list.caption = caption;
    list.target = target;
    list.tree = tree;
    list.comparisons = comparisons;
    Ok(())
}

/// Compares every existing non-default branch in the tree with the target and,
/// when its tree parent is another existing branch, with that parent. Each
/// branch runs on its own thread.
fn compare_tree<'scope>(
    tree: &[TreeRow],
    refs: &'scope RefTips,
    default_branch: &str,
    target: Option<&DefaultTarget>,
    cache: &'scope Mutex<Cache>,
    scope: &'scope std::thread::Scope<'scope, '_>,
) -> HashMap<String, BranchComparisons> {
    let mut seen = HashSet::new();
    let mut handles = Vec::new();
    for row in tree {
        let Some(branch) = row.branch() else { continue };
        if branch == default_branch || !seen.insert(branch.to_string()) {
            continue;
        }
        let Some(tip) = refs.local(branch) else { continue };
        let target_sha = target.map(|target| target.sha.clone());
        let parent = match (&row.parent, row.parent_deleted) {
            (Some(_), true) => Err(ParentComparison::Deleted),
            (Some(parent), false) if parent != default_branch => refs
                .local(parent)
                .map(str::to_string)
                .ok_or(ParentComparison::Deleted),
            _ => Err(ParentComparison::NotApplicable),
        };
        let branch = branch.to_string();
        handles.push(scope.spawn(move || {
            let target = target_sha.and_then(|sha| compare_cached(cache, &sha, tip));
            let parent = match parent {
                Ok(parent_sha) => ParentComparison::Compared(compare_cached(cache, &parent_sha, tip)),
                Err(fixed) => fixed,
            };
            (branch, BranchComparisons { target, parent })
        }));
    }
    handles
        .into_iter()
        .map(|handle| handle.join().expect("comparison thread panicked"))
        .collect()
}

impl WorktreeList {
    /// Worktree entries parsed before status analysis.
    pub fn entries(&self) -> &[WorktreeEntry] {
        &self.entries
    }

    /// Branch tips from the parse step; empty when `for-each-ref` failed.
    pub fn refs(&self) -> &RefTips {
        &self.refs
    }

    /// Fork-origin records as loaded (pruned once statuses are filled).
    pub fn fork_origins(&self) -> &ForkOriginStore {
        &self.forks
    }
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

/// Create a new worktree under `base`.
///
/// The worktree is placed at `{base}/{repo-name}/{dasherized-branch}/`.
/// Callers are responsible for resolving `base` (e.g. via
/// [`worktree::config::resolve_base_dir`] or an interactive prompt).
///
/// A new branch forks from the local branch `from`, or from the current
/// branch when `from` is `None`, and its fork origin is recorded (see
/// [`crate::fork_origin`]). An existing branch is reused as-is and records
/// nothing.
///
/// ## Errors
///
/// - [`WorktreeError::WorktreeAlreadyExists`] when the directory exists.
/// - [`WorktreeError::FromWithExistingBranch`] when `branch` exists and `from`
///   was given, since `from` would be ignored.
/// - [`WorktreeError::FromBranchNotFound`] when `from` is not a local branch.
/// - [`WorktreeError::DetachedHeadWithoutFrom`] when a new branch is requested
///   from a detached HEAD without `from`.
/// - Any git failure.
pub fn create_worktree(
    branch: &str,
    base: &Path,
    from: Option<&str>,
) -> Result<CreateResult, WorktreeError> {
    let info = repo_info()?;

    let dir_name = dasherize(branch);
    let target_path = base.join(&info.name).join(&dir_name);

    if target_path.exists() {
        return Err(WorktreeError::WorktreeAlreadyExists(dir_name));
    }

    // Check if the branch already exists
    let branch_exists = git_command(&["rev-parse", "--verify", branch]).is_ok();

    if branch_exists {
        if let Some(from) = from {
            return Err(WorktreeError::FromWithExistingBranch {
                branch: branch.to_string(),
                from: from.to_string(),
            });
        }
        git_command(&[
            "worktree",
            "add",
            &target_path.display().to_string(),
            branch,
        ])?;
        // Reusing the branch as-is checks it out wherever it already points — it
        // is NOT forked from the current HEAD. Report the commit so callers can
        // warn about silently resurrecting a stale branch.
        let reused_branch_at = git_command(&["rev-parse", "--short", branch]).ok();
        return Ok(CreateResult {
            target_cwd: target_path.join(&info.relative_path),
            worktree_path: target_path,
            branch: branch.to_string(),
            reused_branch_at,
            forked_from: None,
        });
    }

    let (fork_base, base_sha) = match from {
        Some(from) => {
            let full_ref = format!("refs/heads/{from}");
            let sha = git_command(&["rev-parse", "--verify", "--quiet", &full_ref])
                .map_err(|_| WorktreeError::FromBranchNotFound(from.to_string()))?;
            (from.to_string(), sha)
        }
        None => {
            // A detached HEAD would otherwise be recorded as a fork parent,
            // writing a SHA where a branch name belongs.
            let current = git_command(&["symbolic-ref", "--quiet", "--short", "HEAD"])
                .map_err(|_| WorktreeError::DetachedHeadWithoutFrom(branch.to_string()))?;
            (current, git_command(&["rev-parse", "HEAD"])?)
        }
    };

    let target = target_path.display().to_string();
    let mut add_args = vec!["worktree", "add", target.as_str(), "-b", branch];
    // The full ref keeps a same-named tag from shadowing the branch.
    let start_point = format!("refs/heads/{fork_base}");
    if from.is_some() {
        add_args.push(&start_point);
    }
    git_command(&add_args)?;

    // Fork-origin records live in the user cache; failing to write one only
    // loses the branch's place in the listing's tree, so it never fails the
    // create after the worktree exists.
    if let Some(repo_root) = crate::cache::main_worktree_path()
        && let Ok(path) = crate::fork_origin::fork_origin_path(&repo_root)
    {
        let created_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|elapsed| elapsed.as_secs())
            .unwrap_or_default();
        let origin = crate::fork_origin::ForkOrigin {
            base_branch: fork_base.clone(),
            base_sha,
            created_at,
        };
        let _ = crate::fork_origin::record(&path, branch, origin);
    }

    Ok(CreateResult {
        target_cwd: target_path.join(&info.relative_path),
        worktree_path: target_path,
        branch: branch.to_string(),
        reused_branch_at: None,
        forked_from: Some(fork_base),
    })
}

/// Find a worktree by name; see [`resolve_worktree`] for the rules.
pub fn find_worktree(name: &str) -> Result<WorktreeEntry, WorktreeError> {
    let porcelain = git_command(&["worktree", "list", "--porcelain"])?;
    resolve_worktree(&parse_worktree_list(&porcelain), name)
}

/// Resolve `name` against `entries` (main checkout first).
///
/// - `base` is always the main checkout, whatever branch it has checked out.
/// - Otherwise every worktree whose branch equals `name`, and every linked
///   worktree whose directory basename equals `name` or its dasherized form,
///   matches. Exactly one distinct worktree resolves; more than one is
///   [`WorktreeError::AmbiguousWorktree`], never a silent pick.
///
/// The main checkout's basename is the repository's name, not a worktree
/// name, so it does not match; `base` is its stable name.
pub fn resolve_worktree(
    entries: &[WorktreeEntry],
    name: &str,
) -> Result<WorktreeEntry, WorktreeError> {
    if name == "base" {
        return entries
            .iter()
            .find(|e| e.is_main)
            .cloned()
            .ok_or_else(|| WorktreeError::WorktreeNotFound("base".into()));
    }

    let dasherized_name = dasherize(name);
    let mut matches: Vec<&WorktreeEntry> = Vec::new();
    for entry in entries {
        let branch_match = entry.branch.as_deref() == Some(name);
        let basename_match = !entry.is_main
            && basename(entry).is_some_and(|dir| dir == name || dir == dasherized_name);
        if (branch_match || basename_match) && !matches.iter().any(|m| m.path == entry.path) {
            matches.push(entry);
        }
    }

    match matches.as_slice() {
        [] => Err(WorktreeError::WorktreeNotFound(name.into())),
        [only] => Ok((*only).clone()),
        many => Err(WorktreeError::AmbiguousWorktree {
            name: name.to_string(),
            candidates: many
                .iter()
                .map(|entry| crate::error::WorktreeCandidate {
                    branch: entry.branch.clone(),
                    basename: basename(entry).unwrap_or_default(),
                    path: entry.path.clone(),
                })
                .collect(),
        }),
    }
}

/// List worktree names for shell completions; see [`completion_names`].
pub fn worktree_names() -> Vec<String> {
    let Ok(porcelain) = git_command(&["worktree", "list", "--porcelain"]) else {
        return vec!["base".to_string()];
    };
    completion_names(&parse_worktree_list(&porcelain))
}

/// Every name [`resolve_worktree`] accepts for `entries`, deduplicated:
/// `base`, the main checkout's branch when attached, and each linked
/// worktree's branch (when attached) and directory basename.
///
/// The default branch is offered only when some checkout has it, because a
/// branch name resolves by actual checkout.
pub fn completion_names(entries: &[WorktreeEntry]) -> Vec<String> {
    let mut names = vec!["base".to_string()];
    for entry in entries {
        let candidates = [
            entry.branch.clone(),
            (!entry.is_main).then(|| basename(entry)).flatten(),
        ];
        for name in candidates.into_iter().flatten() {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
}

fn basename(entry: &WorktreeEntry) -> Option<String> {
    entry
        .path
        .file_name()
        .map(|dir| dir.to_string_lossy().into_owned())
}

/// Every local branch name, for `--from` completion. Empty when git fails.
pub fn local_branches() -> Vec<String> {
    git_command(&["for-each-ref", "--format=%(refname:short)", "refs/heads"])
        .map(|out| out.lines().map(str::to_string).collect())
        .unwrap_or_default()
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

    /// The `-> {default}` comparison for `branch`.
    fn target_of(list: &WorktreeList, branch: &str) -> crate::listing::Comparison {
        list.comparisons
            .get(branch)
            .and_then(|comparisons| comparisons.target)
            .unwrap_or_else(|| panic!("missing target comparison for branch {branch}"))
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

    /// `rev-list` calls whose symmetric range ends at `branch_sha`.
    fn rev_list_branch_count(calls: &[Vec<String>], branch_sha: &str) -> usize {
        recorder::count_matching(calls, |args| {
            args.first().map(String::as_str) == Some("rev-list")
                && args
                    .last()
                    .is_some_and(|range| range.ends_with(&format!("...{branch_sha}")))
        })
    }

    fn merge_tree_branch_count(calls: &[Vec<String>], branch_sha: &str) -> usize {
        recorder::count_matching(calls, |args| {
            args.len() >= 4
                && args[0] == "merge-tree"
                && args[1] == "--write-tree"
                && args[3] == branch_sha
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

        assert_eq!(rev_list_count(&calls), 0, "warm cache should skip rev-list, got {calls:?}");
        assert_eq!(merge_tree_count(&calls), 0, "warm cache should skip merge-tree, got {calls:?}");
        for branch in ["feature-a", "feature-b"] {
            assert_eq!(target_of(&warm, branch), target_of(&cold, branch));
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

        let diverged = target_of(&list, "diverged");
        assert_eq!((diverged.ahead, diverged.behind), (1, 1));
        let fast_forward = target_of(&list, "fast-forward");
        assert_eq!((fast_forward.ahead, fast_forward.behind), (1, 0));

        let main_sha = rev_parse(repo.path(), "main");
        for branch in ["diverged", "fast-forward"] {
            let sha = rev_parse(repo.path(), branch);
            assert_eq!(rev_list_branch_count(&calls, &sha), 1, "{branch}: one rev-list, got {calls:?}");
            assert_eq!(merge_tree_branch_count(&calls, &sha), 1, "{branch}: one merge-tree, got {calls:?}");
        }
        assert_eq!(rev_list_branch_count(&calls, &main_sha), 0, "main compares with nothing, got {calls:?}");
        assert_eq!(merge_tree_branch_count(&calls, &main_sha), 0, "main compares with nothing, got {calls:?}");

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
        let fast_forward = target_of(&list, "fast-forward");

        assert_eq!((fast_forward.ahead, fast_forward.behind), (1, 0));
        assert!(fast_forward.is_clean);
        assert_eq!(
            merge_tree_branch_count(&calls, &rev_parse(repo.path(), "fast-forward")),
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

        assert_eq!(rev_list_count(&warm_calls), 0, "warm cache should skip rev-list, got {warm_calls:?}");
        assert_eq!(merge_tree_count(&warm_calls), 0, "warm cache should skip merge-tree, got {warm_calls:?}");
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

        assert_eq!(rev_list_count(&calls), 2, "one rev-list per non-main branch, got {calls:?}");
        assert_eq!(merge_tree_count(&calls), 2, "one merge-tree per non-main branch, got {calls:?}");

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_reads_tips_from_one_for_each_ref() {
        let (repo, _feature_a, _feature_b) = temp_repo_with_diverged_worktrees();
        let _guard = DirGuard::enter(repo.path());

        recorder::start_recording();
        let list = list_worktrees().expect("list should succeed");
        let calls = recorder::finish_recording();

        let count = |command: &str| {
            recorder::count_matching(&calls, |args| args.first().map(String::as_str) == Some(command))
        };
        assert_eq!(count("for-each-ref"), 1, "got {calls:?}");
        // `default_branch()` probes `rev-parse --verify main` without a remote;
        // no tip is resolved with a bare `rev-parse <branch>` any more.
        let tip_lookups = recorder::count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("rev-parse") && args.get(1).map(String::as_str) != Some("--verify")
        });
        assert_eq!(tip_lookups, 0, "tips come from for-each-ref, got {calls:?}");
        assert_eq!(list.refs().local("feature-a"), Some(rev_parse(repo.path(), "feature-a").as_str()));
        assert_eq!(list.target.as_ref().map(|t| t.reference.as_str()), Some("main"));
        assert_eq!(list.caption, None, "no origin, no caption");

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_branch_tip_advance_invalidates_cache_entry() {
        let (repo, feature_a, _feature_b) = temp_repo_with_diverged_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        let cold = list_worktrees().expect("cold list should succeed");
        assert_eq!(target_of(&cold, "feature-a").ahead, 1);

        fs::write(feature_a.join("feature-a-2.txt"), "feature a 2\n").expect("write feature a");
        run_git(&feature_a, &["add", "."]);
        run_git(&feature_a, &["commit", "-m", "feature a 2"]);

        recorder::start_recording();
        let updated = list_worktrees().expect("updated list should succeed");
        let calls = recorder::finish_recording();

        assert!(rev_list_count(&calls) >= 1, "branch tip move should recompute rev-list, got {calls:?}");
        assert!(merge_tree_count(&calls) >= 1, "branch tip move should recompute merge-tree, got {calls:?}");
        assert_eq!(target_of(&updated, "feature-a").ahead, 2);

        remove_cache_for(repo.path());
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_default_tip_advance_invalidates_cache_entry() {
        let (repo, _feature_a, _feature_b) = temp_repo_with_diverged_worktrees();
        let _guard = DirGuard::enter(repo.path());
        remove_cache_for(repo.path());

        let cold = list_worktrees().expect("cold list should succeed");
        assert_eq!(target_of(&cold, "feature-a").behind, 1);

        fs::write(repo.path().join("main-2.txt"), "main 2\n").expect("write main");
        run_git(repo.path(), &["add", "."]);
        run_git(repo.path(), &["commit", "-m", "main 2"]);

        recorder::start_recording();
        let updated = list_worktrees().expect("updated list should succeed");
        let calls = recorder::finish_recording();

        assert!(rev_list_count(&calls) >= 1, "default tip move should recompute rev-list, got {calls:?}");
        assert!(merge_tree_count(&calls) >= 1, "default tip move should recompute merge-tree, got {calls:?}");
        assert_eq!(target_of(&updated, "feature-a").behind, 2);

        remove_cache_for(repo.path());
    }

    #[test]
    fn porcelain_path_reads_plain_and_renamed_entries() {
        assert_eq!(porcelain_path(" M README.md"), Some("README.md"));
        assert_eq!(porcelain_path("?? notes.txt"), Some("notes.txt"));
        assert_eq!(porcelain_path("R  old/foo.rs -> new/foo.rs"), Some("new/foo.rs"));
        assert_eq!(porcelain_path("M"), None);
    }

    // --- Resolution and completions (item 2) --------------------------------

    fn entries(porcelain: &str) -> Vec<WorktreeEntry> {
        parse_worktree_list(porcelain)
    }

    fn resolved_path(entries: &[WorktreeEntry], name: &str) -> PathBuf {
        resolve_worktree(entries, name)
            .unwrap_or_else(|e| panic!("{name} should resolve: {e}"))
            .path
    }

    fn ambiguous_paths(entries: &[WorktreeEntry], name: &str) -> Vec<PathBuf> {
        match resolve_worktree(entries, name) {
            Err(WorktreeError::AmbiguousWorktree { candidates, .. }) => {
                candidates.into_iter().map(|c| c.path).collect()
            }
            other => panic!("{name} should be ambiguous, got {other:?}"),
        }
    }

    #[test]
    fn resolve_by_branch_and_by_basename_reach_the_same_worktree() {
        let list = entries(PORCELAIN_SAMPLE);
        let auth = PathBuf::from("/tmp/worktrees/my-project/feature-auth");

        assert_eq!(resolved_path(&list, "feature/auth"), auth);
        assert_eq!(resolved_path(&list, "feature-auth"), auth);
        // The input is dasherized for the basename comparison.
        assert_eq!(resolved_path(&list, "Feature/Auth"), auth);
        assert_eq!(
            resolved_path(&list, "base"),
            PathBuf::from("/Users/ken/code/my-project")
        );
        assert_eq!(
            resolved_path(&list, "main"),
            PathBuf::from("/Users/ken/code/my-project")
        );
    }

    #[test]
    fn resolve_unknown_name_is_not_found() {
        let list = entries(PORCELAIN_SAMPLE);
        assert!(matches!(
            resolve_worktree(&list, "nope"),
            Err(WorktreeError::WorktreeNotFound(name)) if name == "nope"
        ));
        // The main checkout's basename is the repository name, not a worktree name.
        assert!(matches!(
            resolve_worktree(&list, "my-project"),
            Err(WorktreeError::WorktreeNotFound(_))
        ));
    }

    #[test]
    fn two_worktrees_on_one_branch_are_ambiguous() {
        let list = entries(
            "worktree /repo\nHEAD 1111\nbranch refs/heads/main\n\n\
             worktree /wt/repo/feat-a\nHEAD 2222\nbranch refs/heads/feat/a\n\n\
             worktree /wt/repo/feat-a-copy\nHEAD 2222\nbranch refs/heads/feat/a\n",
        );
        assert_eq!(
            ambiguous_paths(&list, "feat/a"),
            vec![
                PathBuf::from("/wt/repo/feat-a"),
                PathBuf::from("/wt/repo/feat-a-copy")
            ]
        );
        // Each basename still names exactly one of them.
        assert_eq!(
            resolved_path(&list, "feat-a-copy"),
            PathBuf::from("/wt/repo/feat-a-copy")
        );
    }

    #[test]
    fn basename_collision_across_parent_directories_is_ambiguous() {
        let list = entries(
            "worktree /repo\nHEAD 1111\nbranch refs/heads/main\n\n\
             worktree /one/spike\nHEAD 2222\nbranch refs/heads/spike/one\n\n\
             worktree /two/spike\nHEAD 3333\nbranch refs/heads/spike/two\n",
        );
        assert_eq!(
            ambiguous_paths(&list, "spike"),
            vec![PathBuf::from("/one/spike"), PathBuf::from("/two/spike")]
        );
        assert_eq!(resolved_path(&list, "spike/two"), PathBuf::from("/two/spike"));
    }

    #[test]
    fn branch_name_colliding_with_another_basename_is_ambiguous() {
        let list = entries(
            "worktree /repo\nHEAD 1111\nbranch refs/heads/main\n\n\
             worktree /wt/repo/parser-work\nHEAD 2222\nbranch refs/heads/parser\n\n\
             worktree /wt/repo/parser\nHEAD 3333\nbranch refs/heads/other\n",
        );
        assert_eq!(
            ambiguous_paths(&list, "parser"),
            vec![
                PathBuf::from("/wt/repo/parser-work"),
                PathBuf::from("/wt/repo/parser")
            ]
        );
    }

    #[test]
    fn ambiguity_error_lists_branch_basename_and_path() {
        let list = entries(
            "worktree /repo\nHEAD 1111\nbranch refs/heads/main\n\n\
             worktree /one/spike\nHEAD 2222\nbranch refs/heads/spike/one\n\n\
             worktree /two/spike\nHEAD 3333\ndetached\n",
        );
        let message = resolve_worktree(&list, "spike").unwrap_err().to_string();
        assert!(message.contains("'spike' matches more than one worktree"), "{message}");
        assert!(
            message.contains("branch spike/one, directory spike, at /one/spike"),
            "{message}"
        );
        assert!(
            message.contains("branch (detached), directory spike, at /two/spike"),
            "{message}"
        );
    }

    #[test]
    fn base_on_another_branch_while_default_is_checked_out_elsewhere() {
        let list = entries(
            "worktree /repo\nHEAD 1111\nbranch refs/heads/develop\n\n\
             worktree /wt/repo/main\nHEAD 2222\nbranch refs/heads/main\n\n\
             worktree /wt/repo/feat-x\nHEAD 3333\nbranch refs/heads/feat/x\n",
        );
        assert_eq!(resolved_path(&list, "base"), PathBuf::from("/repo"));
        assert_eq!(resolved_path(&list, "develop"), PathBuf::from("/repo"));
        // `main` resolves by actual checkout, not to the base checkout.
        assert_eq!(resolved_path(&list, "main"), PathBuf::from("/wt/repo/main"));

        assert_eq!(
            completion_names(&list),
            ["base", "develop", "main", "feat/x", "feat-x"]
        );
    }

    #[test]
    fn detached_worktrees_resolve_and_complete_by_basename_only() {
        let list = entries(
            "worktree /repo\nHEAD 1111\ndetached\n\n\
             worktree /wt/repo/bisect\nHEAD 2222\ndetached\n",
        );
        assert_eq!(resolved_path(&list, "bisect"), PathBuf::from("/wt/repo/bisect"));
        assert_eq!(resolved_path(&list, "base"), PathBuf::from("/repo"));
        // A detached base checkout offers only `base`; the default branch is not
        // checked out anywhere, so it is not offered.
        assert_eq!(completion_names(&list), ["base", "bisect"]);
    }

    #[test]
    fn completion_names_offer_branch_and_basename_deduplicated() {
        let list = entries(
            "worktree /repo\nHEAD 1111\nbranch refs/heads/main\n\n\
             worktree /wt/repo/feature-auth\nHEAD 2222\nbranch refs/heads/feature/auth\n\n\
             worktree /wt/repo/simple\nHEAD 3333\nbranch refs/heads/simple\n\n\
             worktree /wt/repo/feature-auth-2\nHEAD 2222\nbranch refs/heads/feature/auth\n",
        );
        assert_eq!(
            completion_names(&list),
            [
                "base",
                "main",
                "feature/auth",
                "feature-auth",
                "simple",
                "feature-auth-2"
            ]
        );
        // Every offered name resolves or reports ambiguity; none is not-found.
        for name in completion_names(&list) {
            assert!(
                !matches!(
                    resolve_worktree(&list, &name),
                    Err(WorktreeError::WorktreeNotFound(_))
                ),
                "{name}"
            );
        }
    }

    #[test]
    fn find_worktree_and_worktree_names_read_the_real_repository() {
        let repo = temp_repo();
        let linked = repo.path().join("linked-dir");
        run_git(
            repo.path(),
            &["worktree", "add", linked.to_str().unwrap(), "-b", "feat/linked"],
        );
        let _guard = DirGuard::enter(repo.path());

        let by_branch = find_worktree("feat/linked").expect("branch resolves");
        let by_dir = find_worktree("linked-dir").expect("basename resolves");
        assert_eq!(by_branch.path, by_dir.path);
        assert!(find_worktree("base").expect("base resolves").is_main);
        assert_eq!(
            worktree_names(),
            ["base", "main", "feat/linked", "linked-dir"]
        );
    }

    // --- create --from and fork-origin records (item 6) ---------------------

    struct ForkStoreCleanup(PathBuf);

    impl Drop for ForkStoreCleanup {
        fn drop(&mut self) {
            let _ = fs::remove_file(&self.0);
        }
    }

    fn fork_store(repo: &Path) -> (PathBuf, ForkStoreCleanup) {
        let path = crate::fork_origin::fork_origin_path(repo).expect("fork store path");
        let _ = fs::remove_file(&path);
        (path.clone(), ForkStoreCleanup(path))
    }

    fn rev_parse(repo: &Path, rev: &str) -> String {
        let out = Command::new("git")
            .current_dir(repo)
            .args(["rev-parse", rev])
            .output()
            .expect("git rev-parse");
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn commit_file(repo: &Path, name: &str) {
        fs::write(repo.join(name), name).expect("write file");
        run_git(repo, &["add", name]);
        run_git(repo, &["commit", "-m", name]);
    }

    /// A repo on `main` with a `feat/theme` branch one commit ahead.
    fn repo_with_theme_branch() -> tempfile::TempDir {
        let repo = temp_repo();
        run_git(repo.path(), &["checkout", "-b", "feat/theme"]);
        commit_file(repo.path(), "theme.txt");
        run_git(repo.path(), &["checkout", "main"]);
        repo
    }

    #[test]
    #[serial_test::serial]
    fn create_from_forks_the_named_branch_and_records_it() {
        let repo = repo_with_theme_branch();
        let base = tempfile::tempdir().expect("base dir");
        let (store_path, _cleanup) = fork_store(repo.path());
        let _guard = DirGuard::enter(repo.path());

        let result =
            create_worktree("fix/x", base.path(), Some("feat/theme")).expect("create --from");

        assert_eq!(result.forked_from.as_deref(), Some("feat/theme"));
        assert!(result.reused_branch_at.is_none());
        let theme_tip = rev_parse(repo.path(), "feat/theme");
        assert_eq!(rev_parse(&result.worktree_path, "HEAD"), theme_tip);
        assert_ne!(theme_tip, rev_parse(repo.path(), "main"));

        let store = crate::fork_origin::ForkOriginStore::load_from(&store_path);
        let origin = store.get("fix/x").expect("fork origin recorded");
        assert_eq!(origin.base_branch, "feat/theme");
        assert_eq!(origin.base_sha, theme_tip);
        assert!(origin.created_at > 0);
    }

    #[test]
    #[serial_test::serial]
    fn create_without_from_forks_and_records_the_current_branch() {
        let repo = repo_with_theme_branch();
        let base = tempfile::tempdir().expect("base dir");
        let (store_path, _cleanup) = fork_store(repo.path());
        let _guard = DirGuard::enter(repo.path());

        let result = create_worktree("fix/y", base.path(), None).expect("create");

        assert_eq!(result.forked_from.as_deref(), Some("main"));
        let main_tip = rev_parse(repo.path(), "main");
        assert_eq!(rev_parse(&result.worktree_path, "HEAD"), main_tip);
        let store = crate::fork_origin::ForkOriginStore::load_from(&store_path);
        assert_eq!(store.get("fix/y").unwrap().base_branch, "main");
        assert_eq!(store.get("fix/y").unwrap().base_sha, main_tip);
    }

    #[test]
    #[serial_test::serial]
    fn create_from_with_existing_branch_fails_and_creates_nothing() {
        let repo = repo_with_theme_branch();
        let base = tempfile::tempdir().expect("base dir");
        let (store_path, _cleanup) = fork_store(repo.path());
        let _guard = DirGuard::enter(repo.path());

        let error = create_worktree("feat/theme", base.path(), Some("main")).unwrap_err();

        assert_eq!(
            error.to_string(),
            "`feat/theme` already exists, so `--from main` would be ignored. \
             Drop `--from` to reuse it."
        );
        assert!(!base.path().join(repo_name(repo.path())).join("feat-theme").exists());
        assert!(!store_path.exists());
    }

    #[test]
    #[serial_test::serial]
    fn create_reusing_an_existing_branch_records_nothing() {
        let repo = repo_with_theme_branch();
        let base = tempfile::tempdir().expect("base dir");
        let (store_path, _cleanup) = fork_store(repo.path());
        let _guard = DirGuard::enter(repo.path());

        let result = create_worktree("feat/theme", base.path(), None).expect("reuse");

        assert!(result.reused_branch_at.is_some());
        assert!(result.forked_from.is_none());
        assert!(crate::fork_origin::ForkOriginStore::load_from(&store_path).is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn create_from_a_missing_or_remote_only_branch_names_it() {
        let repo = repo_with_theme_branch();
        // A remote-tracking ref with no local branch of the same name.
        let main_tip = rev_parse(repo.path(), "main");
        run_git(
            repo.path(),
            &["update-ref", "refs/remotes/origin/remote-only", &main_tip],
        );
        // A tag must not stand in for a local branch either.
        run_git(repo.path(), &["-c", "tag.gpgSign=false", "tag", "v1"]);
        let base = tempfile::tempdir().expect("base dir");
        let (store_path, _cleanup) = fork_store(repo.path());
        let _guard = DirGuard::enter(repo.path());

        for from in ["no-such-branch", "remote-only", "origin/remote-only", "v1"] {
            let error = create_worktree("fix/z", base.path(), Some(from)).unwrap_err();
            assert!(
                matches!(&error, WorktreeError::FromBranchNotFound(name) if name == from),
                "{from}: {error:?}"
            );
            assert_eq!(
                error.to_string(),
                format!("`--from {from}` does not name an existing local branch")
            );
        }
        assert!(!base.path().join(repo_name(repo.path())).join("fix-z").exists());
        assert!(!store_path.exists());
    }

    #[test]
    #[serial_test::serial]
    fn create_on_detached_head_requires_from() {
        let repo = repo_with_theme_branch();
        let main_tip = rev_parse(repo.path(), "main");
        run_git(repo.path(), &["checkout", "--detach", &main_tip]);
        let base = tempfile::tempdir().expect("base dir");
        let (store_path, _cleanup) = fork_store(repo.path());
        let _guard = DirGuard::enter(repo.path());

        let error = create_worktree("fix/d", base.path(), None).unwrap_err();
        assert!(matches!(error, WorktreeError::DetachedHeadWithoutFrom(_)));
        let message = error.to_string();
        assert!(message.contains("HEAD is detached"), "{message}");
        assert!(message.contains("`fix/d`"), "{message}");
        assert!(message.contains("--from <branch>"), "{message}");
        assert!(!store_path.exists());

        let result =
            create_worktree("fix/d", base.path(), Some("feat/theme")).expect("detached + --from");
        assert_eq!(result.forked_from.as_deref(), Some("feat/theme"));
        let store = crate::fork_origin::ForkOriginStore::load_from(&store_path);
        assert_eq!(store.get("fix/d").unwrap().base_branch, "feat/theme");
    }

    fn repo_name(repo: &Path) -> String {
        repo.file_name().unwrap().to_string_lossy().into_owned()
    }
}
