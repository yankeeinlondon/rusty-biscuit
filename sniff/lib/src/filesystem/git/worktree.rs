//! Worktree detection helpers.
//!
//! Provides lightweight functions for detecting whether the current directory
//! is inside a linked Git worktree and extracting its name, as well as
//! listing all worktrees in a repository.

use crate::SniffError;
use crate::performance::{self, counters};
use gix::bstr::ByteSlice;
use std::error::Error;
use std::path::{Path, PathBuf};

/// A single worktree entry returned by [`list_worktrees`].
///
/// Represents either the main worktree or a linked worktree, with enough
/// information to render it in multiple output formats.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// Worktree name — the directory basename for the main worktree, or the
    /// worktree name given at creation for linked worktrees.
    pub name: String,
    /// Branch name checked out in this worktree (`None` when in detached HEAD).
    pub branch: Option<String>,
    /// Absolute path to the worktree's working directory.
    pub path: PathBuf,
    /// Whether this worktree is the one the current process is running from.
    pub is_current: bool,
    /// Whether this worktree is in detached HEAD state.
    pub is_detached: bool,
}

/// Returns the base directory name of the current linked worktree.
///
/// Discovers the Git repository containing `cwd`. If the repository is a
/// linked worktree (not the main/original worktree), returns the basename
/// of the worktree's working directory. Returns `None` when inside the
/// main worktree, outside any repository, or when the worktree path has
/// no valid basename.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
///
/// if let Some(name) = sniff::filesystem::git::get_current_worktree_name(Path::new(".")).unwrap() {
///     println!("In worktree: {}", name);
/// }
/// ```
pub fn get_current_worktree_name(cwd: &Path) -> Result<Option<String>, Box<dyn Error>> {
    let Some(repo) = super::open::trusted_discover(cwd)? else {
        return Ok(None);
    };
    Ok(current_worktree_name_from_gix(&repo))
}

/// True when `repo` is a linked worktree rather than the main worktree.
fn is_linked_worktree(repo: &gix::Repository) -> bool {
    repo.git_dir() != repo.common_dir()
}

/// Returns the name and fully-qualified path of the current linked worktree.
///
/// Similar to [`get_current_worktree_name`], but also returns the absolute
/// path to the worktree's root directory. Returns `None` when inside the
/// main worktree, outside any repository, or when the worktree path has no
/// valid basename.
pub fn get_current_worktree_info(cwd: &Path) -> Result<Option<(String, String)>, Box<dyn Error>> {
    let Some(repo) = super::open::trusted_discover(cwd)? else {
        return Ok(None);
    };

    if !is_linked_worktree(&repo) {
        return Ok(None);
    }

    let Some(workdir) = repo.workdir() else {
        return Ok(None);
    };

    // Canonicalize first: gix may report a relative workdir (no `file_name`).
    let canonical = std::fs::canonicalize(workdir).unwrap_or_else(|_| workdir.to_path_buf());
    let name = canonical
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from);
    let path = canonical.to_string_lossy().to_string();

    match name {
        Some(n) => Ok(Some((n, path))),
        None => Ok(None),
    }
}

/// Discovers the repository containing `base_dir` and returns all worktrees.
///
/// The returned list includes the main worktree plus any linked worktrees.
/// Entries are sorted alphabetically by worktree name. Exactly one entry
/// is marked as `is_current` by comparing canonicalized paths.
///
/// Returns `Ok(None)` when `base_dir` is not inside a git repository.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
///
/// if let Some(worktrees) = sniff::filesystem::git::list_worktrees(Path::new(".")).unwrap() {
///     for wt in &worktrees {
///         let marker = if wt.is_current { "* " } else { "  " };
///         println!("{}{}", marker, wt.name);
///     }
/// }
/// ```
pub fn list_worktrees(base_dir: &Path) -> Result<Option<Vec<WorktreeEntry>>, Box<dyn Error>> {
    let Some(discovered) = super::open::trusted_discover(base_dir)? else {
        return Ok(None);
    };
    list_worktrees_from_gix(&discovered).map(Some)
}

/// [`list_worktrees`] against an already-discovered repository.
///
/// Identical output to `list_worktrees(repo.repo_root())`, without the second
/// `trusted_discover`. Exists so an aggregate observation can enumerate
/// worktrees on the one handle it already holds.
pub fn list_worktrees_with_repo(repo: &super::GitRepo) -> Result<Vec<WorktreeEntry>, Box<dyn Error>> {
    repo.with_cached_gix(list_worktrees_from_gix)
}

/// [`get_current_worktree_name`] against an already-discovered repository.
pub fn current_worktree_name_with_repo(repo: &super::GitRepo) -> Option<String> {
    repo.with_cached_gix(current_worktree_name_from_gix)
}

fn current_worktree_name_from_gix(repo: &gix::Repository) -> Option<String> {
    // Not a linked worktree — either the main repo or a bare repo.
    if !is_linked_worktree(repo) {
        return None;
    }
    let workdir = repo.workdir()?;
    // Canonicalize first: gix may report a relative workdir (no `file_name`).
    let canonical = std::fs::canonicalize(workdir).unwrap_or_else(|_| workdir.to_path_buf());
    canonical
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from)
}

fn list_worktrees_from_gix(discovered: &gix::Repository) -> Result<Vec<WorktreeEntry>, Box<dyn Error>> {
    // If discovery landed on a linked worktree, open the base repository so
    // that worktree enumeration includes the main worktree as well.
    let base_repo = if is_linked_worktree(discovered) {
        Some(super::open::trusted_open(discovered.common_dir())?)
    } else {
        None
    };
    let repo = base_repo.as_ref().unwrap_or(discovered);

    // Determine the current workdir so we can mark exactly one entry as current.
    let current_workdir = std::env::current_dir().ok();
    let current_canonical = current_workdir
        .as_ref()
        .and_then(|p| std::fs::canonicalize(p).ok());

    let mut entries = Vec::new();

    // Add the main worktree, naming it from the workdir directory basename.
    if let Some(main_workdir) = repo.workdir() {
        let main_canonical =
            std::fs::canonicalize(main_workdir).unwrap_or_else(|_| main_workdir.to_path_buf());
        // Name from the canonical path: gix may report a relative workdir (e.g.
        // `.`) when discovered from a relative path, which has no `file_name`.
        let name = main_canonical
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("main")
            .to_string();

        let (branch, is_detached) = resolve_branch_and_detached(repo);
        let is_current = is_worktree_current(&main_canonical, current_canonical.as_deref());

        entries.push(WorktreeEntry {
            name,
            branch,
            path: main_canonical,
            is_current,
            is_detached,
        });
    }

    // Add linked worktrees.
    let proxies = repo
        .worktrees()
        .map_err(|e| SniffError::git("worktrees", e))?;

    for proxy in proxies {
        let name = proxy.id().to_str_lossy().into_owned();
        let path = proxy
            .base()
            .map_err(|e| SniffError::git("worktree_base", e))?;
        let canonical = std::fs::canonicalize(&path).unwrap_or(path);

        // Open the worktree as its own repository to read HEAD.
        let (branch, is_detached) = {
            performance::increment_counter(counters::GIT_WORKTREE_OPENS, 1);
            let wt_repo = super::open::trusted_open(&canonical)?;
            resolve_branch_and_detached(&wt_repo)
        };

        let is_current = is_worktree_current(&canonical, current_canonical.as_deref());

        entries.push(WorktreeEntry {
            name,
            branch,
            path: canonical,
            is_current,
            is_detached,
        });
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(entries)
}

/// Resolves the branch name and detached-HEAD state for a repository.
fn resolve_branch_and_detached(repo: &gix::Repository) -> (Option<String>, bool) {
    let Ok(head) = repo.head() else {
        return (None, false);
    };
    let is_detached = head.is_detached();
    let branch = if is_detached {
        None
    } else {
        head.referent_name().map(|name| name.shorten().to_string())
    };
    (branch, is_detached)
}

/// Checks whether a worktree's canonical path matches the current directory.
fn is_worktree_current(wt_canonical: &Path, current_canonical: Option<&Path>) -> bool {
    current_canonical.is_some_and(|current| current == wt_canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;
    use tempfile::TempDir;

    /// Creates a temporary git repo with a single file committed.
    fn setup_repo() -> (TempDir, git2::Repository) {
        let dir = TempDir::new().unwrap();
        let repo = git2::Repository::init(dir.path()).unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();

        let file_path = dir.path().join("test.txt");
        fs::write(&file_path, "content\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(Path::new("test.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }

        (dir, repo)
    }

    #[test]
    fn inside_linked_worktree_returns_name() {
        let (dir, repo) = setup_repo();

        // Create a linked worktree
        let worktree_path = dir.path().join("feature-branch");
        let _wt = repo
            .worktree("feature-branch", &worktree_path, None)
            .unwrap();

        // Verify the worktree exists
        assert!(worktree_path.exists());

        let name = get_current_worktree_name(&worktree_path).unwrap();
        assert_eq!(name, Some("feature-branch".to_string()));
    }

    #[test]
    fn inside_main_worktree_returns_none() {
        let (dir, _repo) = setup_repo();

        let name = get_current_worktree_name(dir.path()).unwrap();
        assert_eq!(name, None, "main worktree should return None");
    }

    #[test]
    fn outside_repo_returns_none() {
        let tmp = TempDir::new().unwrap();

        let name = get_current_worktree_name(tmp.path()).unwrap();
        assert_eq!(name, None, "outside any repo should return None");
    }

    #[test]
    fn info_inside_linked_worktree_returns_name_and_path() {
        let (dir, repo) = setup_repo();

        let worktree_path = dir.path().join("feature-branch");
        let _wt = repo
            .worktree("feature-branch", &worktree_path, None)
            .unwrap();

        let info = get_current_worktree_info(&worktree_path).unwrap();
        let (name, path) = info.expect("should return info");
        assert_eq!(name, "feature-branch");
        assert!(path.ends_with("feature-branch"));
        assert!(std::path::Path::new(&path).is_absolute());
    }

    #[test]
    fn info_inside_main_worktree_returns_none() {
        let (dir, _repo) = setup_repo();

        let info = get_current_worktree_info(dir.path()).unwrap();
        assert_eq!(info, None, "main worktree should return None");
    }

    // ------------------------------------------------------------------
    // list_worktrees tests
    // ------------------------------------------------------------------

    #[test]
    fn list_worktrees_outside_repo_returns_none() {
        let tmp = TempDir::new().unwrap();
        let result = list_worktrees(tmp.path()).unwrap();
        assert_eq!(result, None, "outside any repo should return None");
    }

    #[test]
    fn list_worktrees_main_only() {
        let (dir, repo) = setup_repo();

        let worktrees = list_worktrees(dir.path())
            .unwrap()
            .expect("should find repo");
        assert_eq!(
            worktrees.len(),
            1,
            "main-only repo has exactly one worktree"
        );

        let main = &worktrees[0];
        // The temp dir basename becomes the main worktree name.
        let expected_name = dir.path().file_name().unwrap().to_str().unwrap();
        let expected_branch = repo
            .head()
            .unwrap()
            .shorthand()
            .expect("fixture repo has a named branch")
            .to_string();
        assert_eq!(main.name, expected_name);
        assert_eq!(main.branch, Some(expected_branch));
        assert!(!main.is_detached);
        // is_current depends on where the test process is running from.
        // We only assert the structure is consistent.
        assert!(main.path.is_absolute());
    }

    #[test]
    fn list_worktrees_with_linked_worktrees() {
        let (dir, repo) = setup_repo();

        let wt_a_path = dir.path().join("wt-a");
        let _wt_a = repo.worktree("wt-a", &wt_a_path, None).unwrap();

        let wt_b_path = dir.path().join("wt-b");
        let _wt_b = repo.worktree("wt-b", &wt_b_path, None).unwrap();

        let worktrees = list_worktrees(dir.path())
            .unwrap()
            .expect("should find repo");
        assert_eq!(worktrees.len(), 3, "main + 2 linked worktrees");

        let names: Vec<&str> = worktrees.iter().map(|w| w.name.as_str()).collect();
        let main_name = dir.path().file_name().unwrap().to_str().unwrap();
        assert_eq!(
            names,
            vec![main_name, "wt-a", "wt-b"],
            "main + linked worktrees sorted alphabetically"
        );
    }

    #[test]
    fn list_worktrees_detached_head() {
        let (dir, repo) = setup_repo();

        // Detach HEAD by checking out the current commit directly.
        let head_commit = repo.head().unwrap().peel_to_commit().unwrap();
        repo.set_head_detached(head_commit.id()).unwrap();

        let worktrees = list_worktrees(dir.path())
            .unwrap()
            .expect("should find repo");
        assert_eq!(worktrees.len(), 1);

        let main = &worktrees[0];
        assert!(main.is_detached, "main worktree should be detached");
        assert_eq!(main.branch, None, "branch should be None when detached");
    }

    #[test]
    fn list_worktrees_linked_worktree_detached_head() {
        let (dir, repo) = setup_repo();

        let wt_path = dir.path().join("detached-wt");
        let _wt = repo.worktree("detached-wt", &wt_path, None).unwrap();

        // Open the worktree repo and detach its HEAD.
        let wt_repo = git2::Repository::open(&wt_path).unwrap();
        let head_commit = wt_repo.head().unwrap().peel_to_commit().unwrap();
        wt_repo.set_head_detached(head_commit.id()).unwrap();

        let worktrees = list_worktrees(dir.path())
            .unwrap()
            .expect("should find repo");
        let detached = worktrees
            .iter()
            .find(|w| w.name == "detached-wt")
            .expect("detached worktree should exist");
        assert!(detached.is_detached, "linked worktree should be detached");
        assert_eq!(detached.branch, None, "branch should be None when detached");
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_current_highlighting_from_main_worktree() {
        let (dir, _repo) = setup_repo();

        // Change to the main worktree directory.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let worktrees = list_worktrees(dir.path())
            .unwrap()
            .expect("should find repo");
        let current_count = worktrees.iter().filter(|w| w.is_current).count();
        assert_eq!(current_count, 1, "exactly one worktree should be current");

        let main = worktrees
            .iter()
            .find(|w| w.path == std::fs::canonicalize(dir.path()).unwrap())
            .expect("main should exist");
        assert!(main.is_current, "main worktree should be marked current");

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    #[serial_test::serial]
    fn list_worktrees_current_highlighting_from_linked_worktree() {
        let (dir, repo) = setup_repo();

        let wt_path = dir.path().join("linked-wt");
        let _wt = repo.worktree("linked-wt", &wt_path, None).unwrap();

        // Change to the linked worktree directory.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(&wt_path).unwrap();

        let worktrees = list_worktrees(&wt_path).unwrap().expect("should find repo");
        let current_count = worktrees.iter().filter(|w| w.is_current).count();
        assert_eq!(current_count, 1, "exactly one worktree should be current");

        let linked = worktrees
            .iter()
            .find(|w| w.name == "linked-wt")
            .expect("linked should exist");
        assert!(
            linked.is_current,
            "linked worktree should be marked current"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn list_worktrees_alphabetical_sorting() {
        let (dir, repo) = setup_repo();

        // Create linked worktrees in non-alphabetical order.
        let wt_z_path = dir.path().join("zulu");
        let _wt_z = repo.worktree("zulu", &wt_z_path, None).unwrap();

        let wt_a_path = dir.path().join("alpha");
        let _wt_a = repo.worktree("alpha", &wt_a_path, None).unwrap();

        let wt_m_path = dir.path().join("mike");
        let _wt_m = repo.worktree("mike", &wt_m_path, None).unwrap();

        let worktrees = list_worktrees(dir.path())
            .unwrap()
            .expect("should find repo");
        let names: Vec<String> = worktrees.iter().map(|w| w.name.clone()).collect();

        let mut sorted_names = names.clone();
        sorted_names.sort();
        assert_eq!(
            names, sorted_names,
            "worktrees should be sorted alphabetically"
        );
    }

    #[test]
    fn list_worktrees_main_worktree_named_from_basename() {
        let (dir, _repo) = setup_repo();

        let worktrees = list_worktrees(dir.path())
            .unwrap()
            .expect("should find repo");
        let main = &worktrees[0];

        let expected = dir.path().file_name().unwrap().to_str().unwrap();
        assert_eq!(
            main.name, expected,
            "main worktree should be named from directory basename"
        );
    }

    #[cfg(unix)]
    #[test]
    fn list_worktrees_propagates_registry_error() {
        let (dir, repo) = setup_repo();
        let wt_path = dir.path().join("linked-wt");
        let _wt = repo.worktree("linked-wt", &wt_path, None).unwrap();

        let worktrees_dir = dir.path().join(".git").join("worktrees");
        std::fs::set_permissions(&worktrees_dir, std::fs::Permissions::from_mode(0o000)).unwrap();

        let result = list_worktrees(dir.path());

        // Restore permissions so TempDir can clean up.
        std::fs::set_permissions(&worktrees_dir, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert!(
            result.is_err(),
            "unreadable worktree registry must surface an error, got {result:?}"
        );
    }

    #[test]
    fn list_worktrees_propagates_proxy_base_error() {
        let (dir, repo) = setup_repo();
        let wt_path = dir.path().join("linked-wt");
        let _wt = repo.worktree("linked-wt", &wt_path, None).unwrap();

        // Empty the gitdir file so proxy.base() fails (the proxy still
        // exists because the file was present when worktrees() scanned).
        let gitdir_file = dir
            .path()
            .join(".git")
            .join("worktrees")
            .join("linked-wt")
            .join("gitdir");
        std::fs::write(&gitdir_file, b"").expect("empty worktree gitdir file");

        let result = list_worktrees(dir.path());
        assert!(
            result.is_err(),
            "empty worktree gitdir must surface an error, got {result:?}"
        );
    }
}
