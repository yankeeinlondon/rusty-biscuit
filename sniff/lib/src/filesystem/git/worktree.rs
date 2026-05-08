//! Worktree detection helpers.
//!
//! Provides lightweight functions for detecting whether the current directory
//! is inside a linked Git worktree and extracting its name.

use std::error::Error;
use std::path::Path;

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
    let repo = match git2::Repository::discover(cwd) {
        Ok(r) => r,
        Err(_) => return Ok(None),
    };

    // Not a linked worktree — either the main repo or a bare repo.
    if !repo.is_worktree() {
        return Ok(None);
    }

    let workdir = match repo.workdir() {
        Some(wd) => wd,
        None => return Ok(None),
    };

    let name = workdir
        .file_name()
        .and_then(|n| n.to_str())
        .map(String::from);

    Ok(name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
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
}
