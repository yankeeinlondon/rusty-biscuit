use std::path::PathBuf;

use serde::Deserialize;

use crate::error::WorktreeError;

#[derive(Debug, Deserialize)]
pub struct WorktreeConfig {
    pub base_dir: String,
}

/// Resolves the base directory for worktree storage.
///
/// ## Resolution Order
///
/// 1. `WT` environment variable
/// 2. `~/.worktree.json` file with `{ "base_dir": "/path" }`
///
/// ## Errors
///
/// Returns an error if:
/// - Neither `WT` nor `~/.worktree.json` provides a base directory
/// - The resolved directory does not exist
/// - The resolved directory is itself a git repository
pub fn resolve_base_dir() -> Result<PathBuf, WorktreeError> {
    let path = resolve_base_dir_raw()?;
    validate_base_dir(&path)?;
    Ok(path)
}

fn resolve_base_dir_raw() -> Result<PathBuf, WorktreeError> {
    // 1. Check WT environment variable
    if let Ok(wt) = std::env::var("WT")
        && !wt.is_empty()
    {
        return Ok(PathBuf::from(wt));
    }

    // 2. Fall back to ~/.worktree.json
    let home = dirs::home_dir().ok_or(WorktreeError::BaseDirectoryNotConfigured)?;
    let config_path = home.join(".worktree.json");

    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        let config: WorktreeConfig = serde_json::from_str(&contents)?;
        return Ok(PathBuf::from(config.base_dir));
    }

    Err(WorktreeError::BaseDirectoryNotConfigured)
}

fn validate_base_dir(path: &std::path::Path) -> Result<(), WorktreeError> {
    if !path.exists() {
        return Err(WorktreeError::BaseDirectoryNotFound(
            path.display().to_string(),
        ));
    }

    // Must not be a git repo
    if path.join(".git").exists() {
        return Err(WorktreeError::BaseDirectoryIsGitRepo(
            path.display().to_string(),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn resolve_base_dir_from_env() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().to_path_buf();

        unsafe { std::env::set_var("WT", &path) };
        let result = resolve_base_dir();
        unsafe { std::env::remove_var("WT") };

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), path);
    }

    #[test]
    fn resolve_base_dir_rejects_nonexistent() {
        unsafe { std::env::set_var("WT", "/tmp/definitely-does-not-exist-wt-test") };
        let result = resolve_base_dir();
        unsafe { std::env::remove_var("WT") };

        assert!(matches!(result, Err(WorktreeError::BaseDirectoryNotFound(_))));
    }

    #[test]
    fn resolve_base_dir_rejects_git_repo() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir(tmp.path().join(".git")).unwrap();

        unsafe { std::env::set_var("WT", tmp.path()) };
        let result = resolve_base_dir();
        unsafe { std::env::remove_var("WT") };

        assert!(matches!(
            result,
            Err(WorktreeError::BaseDirectoryIsGitRepo(_))
        ));
    }
}
