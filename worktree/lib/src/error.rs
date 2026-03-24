use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorktreeError {
    #[error("git is not installed or not found on PATH")]
    GitNotFound,

    #[error("not inside a git repository")]
    NotInGitRepo,

    #[error(
        "base directory not configured: set the WT environment variable or create ~/.worktree.json"
    )]
    BaseDirectoryNotConfigured,

    #[error("base directory '{0}' does not exist")]
    BaseDirectoryNotFound(String),

    #[error("base directory '{0}' is itself a git repository -- it must be a plain directory")]
    BaseDirectoryIsGitRepo(String),

    #[error("worktree '{0}' already exists")]
    WorktreeAlreadyExists(String),

    #[error("worktree '{0}' not found")]
    WorktreeNotFound(String),

    #[error("failed to execute git command: {0}")]
    GitCommand(String),

    #[error("failed to parse git output: {0}")]
    GitParse(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),
}
