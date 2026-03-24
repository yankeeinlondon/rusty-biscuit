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

    let name = root
        .file_name()
        .ok_or_else(|| WorktreeError::GitParse("cannot determine repo name".into()))?
        .to_string_lossy()
        .to_string();

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
