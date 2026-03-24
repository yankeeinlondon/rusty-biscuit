use std::path::PathBuf;
use std::process::Command;

use crate::error::WorktreeError;

#[derive(Debug, Clone)]
pub struct RepoInfo {
    /// Name of the repository (directory name of the root)
    pub name: String,
    /// Absolute path to the repository root
    pub root: PathBuf,
    /// Relative path from repo root to CWD
    pub relative_path: PathBuf,
}

/// Verify git is installed and available on PATH.
pub fn ensure_git() -> Result<(), WorktreeError> {
    which::which("git").map_err(|_| WorktreeError::GitNotFound)?;
    Ok(())
}

/// Get repository info for the current working directory.
///
/// ## Errors
///
/// Returns `NotInGitRepo` if the current directory is not inside a git repository.
pub fn repo_info() -> Result<RepoInfo, WorktreeError> {
    ensure_git()?;

    let root = git_rev_parse("--show-toplevel")?;
    let root = PathBuf::from(root);

    // Derive the repo name from the main worktree path, not the current checkout.
    // Inside a linked worktree, --show-toplevel returns the worktree dir (e.g.
    // /tmp/wt/rusty-biscuit/feat-xyz), so file_name() would give "feat-xyz".
    // The first entry from `git worktree list` is always the main checkout.
    let name = main_worktree_name().unwrap_or_else(|_| {
        root.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    let cwd = std::env::current_dir()?;
    let relative_path = cwd
        .strip_prefix(&root)
        .unwrap_or_else(|_| std::path::Path::new(""))
        .to_path_buf();

    Ok(RepoInfo {
        name,
        root,
        relative_path,
    })
}

/// Derive the repository name from the main worktree path.
///
/// `git worktree list --porcelain` always lists the main checkout first,
/// so its directory name is the canonical repo name even when called from
/// inside a linked worktree.
fn main_worktree_name() -> Result<String, WorktreeError> {
    let output = git_command(&["worktree", "list", "--porcelain"])?;
    let main_path = output
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .ok_or_else(|| WorktreeError::GitParse("cannot find main worktree".into()))?;
    PathBuf::from(main_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| WorktreeError::GitParse("cannot determine repo name".into()))
}

/// Run `git rev-parse` with the given argument and return trimmed stdout.
fn git_rev_parse(arg: &str) -> Result<String, WorktreeError> {
    let output = Command::new("git")
        .args(["rev-parse", arg])
        .output()
        .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

    if !output.status.success() {
        return Err(WorktreeError::NotInGitRepo);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run an arbitrary git command and return trimmed stdout.
pub fn git_command(args: &[&str]) -> Result<String, WorktreeError> {
    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WorktreeError::GitCommand(stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run a git command in a specific directory.
pub fn git_command_in(dir: &std::path::Path, args: &[&str]) -> Result<String, WorktreeError> {
    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WorktreeError::GitCommand(stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ensure_git_succeeds() {
        assert!(ensure_git().is_ok());
    }

    #[test]
    fn repo_info_from_monorepo() {
        let info = repo_info().unwrap();
        assert_eq!(info.name, "rusty-biscuit");
        // Just verify we can access the relative path
        let _ = info.relative_path.to_string_lossy();
    }
}
