use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::WorktreeError;

#[derive(Debug, Deserialize, Serialize)]
pub struct WorktreeConfig {
    pub base_dir: String,
}

/// Returns the path to the user-level config file (`~/.worktree.json`).
pub fn config_path() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".worktree.json"))
}

/// Returns the candidate directories shown during interactive setup.
///
/// Candidates are `~/worktrees`, `~/wt`, and `~/.claudine/worktrees`.
/// The list may be empty if the home directory cannot be determined.
pub fn considered_dirs() -> Vec<PathBuf> {
    let Some(home) = dirs::home_dir() else {
        return vec![];
    };
    vec![
        home.join("worktrees"),
        home.join("wt"),
        home.join(".claudine").join("worktrees"),
    ]
}

/// Saves `base_dir` to `~/.worktree.json`.
pub fn save_config(base_dir: &Path) -> Result<(), WorktreeError> {
    let config_path = config_path().ok_or(WorktreeError::BaseDirectoryNotConfigured)?;
    let config = WorktreeConfig {
        base_dir: base_dir.display().to_string(),
    };
    let contents = serde_json::to_string_pretty(&config)?;
    std::fs::write(&config_path, contents)?;
    Ok(())
}

/// Resolves the base directory for worktree storage.
///
/// ## Resolution Order
///
/// 1. `WT` environment variable — must point to an existing, non-git directory.
/// 2. `~/.worktree.json` — `{ "base_dir": "/path" }` — same constraints apply.
///
/// ## Errors
///
/// - [`WorktreeError::BaseDirectoryNotConfigured`] — neither source is set; caller
///   should run interactive setup and then call [`save_config`].
/// - [`WorktreeError::ConfigInvalidFormat`] — config file exists but is malformed.
/// - [`WorktreeError::ConfigBaseDirIsGitRepo`] — config `base_dir` is a git repo.
/// - [`WorktreeError::BaseDirectoryNotFound`] — the resolved path does not exist.
/// - [`WorktreeError::BaseDirectoryIsGitRepo`] — `WT` env var points to a git repo.
pub fn resolve_base_dir() -> Result<PathBuf, WorktreeError> {
    // 1. WT environment variable
    if let Ok(wt) = std::env::var("WT")
        && !wt.is_empty()
    {
        let path = PathBuf::from(wt);
        if !path.exists() {
            return Err(WorktreeError::BaseDirectoryNotFound(
                path.display().to_string(),
            ));
        }
        if path.join(".git").exists() {
            return Err(WorktreeError::BaseDirectoryIsGitRepo(
                path.display().to_string(),
            ));
        }
        return Ok(path);
    }

    // 2. ~/.worktree.json
    let Some(config_path) = config_path() else {
        return Err(WorktreeError::BaseDirectoryNotConfigured);
    };

    if !config_path.exists() {
        return Err(WorktreeError::BaseDirectoryNotConfigured);
    }

    let contents = std::fs::read_to_string(&config_path)?;
    let config: WorktreeConfig =
        serde_json::from_str(&contents).map_err(|e| WorktreeError::ConfigInvalidFormat {
            config_path: config_path.clone(),
            message: e.to_string(),
        })?;

    let path = PathBuf::from(&config.base_dir);

    if path.join(".git").exists() {
        return Err(WorktreeError::ConfigBaseDirIsGitRepo {
            config_path,
            dir: config.base_dir,
        });
    }

    if !path.exists() {
        return Err(WorktreeError::BaseDirectoryNotFound(
            path.display().to_string(),
        ));
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial_test::serial(wt_env)]
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
    #[serial_test::serial(wt_env)]
    fn resolve_base_dir_rejects_nonexistent() {
        unsafe { std::env::set_var("WT", "/tmp/definitely-does-not-exist-wt-test") };
        let result = resolve_base_dir();
        unsafe { std::env::remove_var("WT") };

        assert!(matches!(
            result,
            Err(WorktreeError::BaseDirectoryNotFound(_))
        ));
    }

    #[test]
    #[serial_test::serial(wt_env)]
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
