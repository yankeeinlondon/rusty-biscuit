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

    /// `~/.worktree.json` exists but cannot be parsed or is missing `base_dir`.
    #[error("~/.worktree.json has an invalid format: {message}")]
    ConfigInvalidFormat {
        config_path: std::path::PathBuf,
        message: String,
    },

    /// `~/.worktree.json` `base_dir` points to a directory that is itself a git repo.
    #[error("base directory '{dir}' (from ~/.worktree.json) is itself a git repository")]
    ConfigBaseDirIsGitRepo {
        config_path: std::path::PathBuf,
        dir: String,
    },

    #[error("operation cancelled")]
    Cancelled,

    #[error("worktree '{0}' already exists")]
    WorktreeAlreadyExists(String),

    #[error("worktree '{0}' not found")]
    WorktreeNotFound(String),

    /// More than one worktree matches `name` by branch or directory basename.
    #[error("{}", ambiguous_message(name, candidates))]
    AmbiguousWorktree {
        name: String,
        candidates: Vec<WorktreeCandidate>,
    },

    /// The command stopped because continuing would lose work; nothing was
    /// changed. The message is Prose markup explaining what to do.
    #[error("{0}")]
    RefusedToLoseWork(String),

    /// The environment prevents the command (for example, no shell wrapper to
    /// move the caller); nothing was changed and no `--force-*` flag helps.
    /// The message is Prose markup explaining what to do.
    #[error("{0}")]
    BlockedByEnvironment(String),

    /// Another program holds the worktree directory (Windows only: its
    /// current directory, or an open file inside). Nothing was removed.
    #[error("the folder {} is in use by another program", .0.display())]
    DirectoryInUse(std::path::PathBuf),

    /// The Windows lock probe renamed the worktree and could not rename it
    /// back. Nothing was removed; `move` restores the original name.
    #[error(
        "the worktree was left at {} after a lock check and could not be renamed back; \
        restore it with: move \"{}\" \"{}\"",
        .temporary.display(),
        .temporary.display(),
        .original.display()
    )]
    LockProbeRenameBack {
        original: std::path::PathBuf,
        temporary: std::path::PathBuf,
    },

    /// `wt create --from` was given for a branch that already exists.
    #[error(
        "`{branch}` already exists, so `--from {from}` would be ignored. Drop `--from` to reuse it."
    )]
    FromWithExistingBranch { branch: String, from: String },

    /// The `--from` base is not an existing local branch.
    #[error("`--from {0}` does not name an existing local branch")]
    FromBranchNotFound(String),

    /// A new branch was requested from a detached HEAD without `--from`.
    #[error(
        "HEAD is detached, so there is no current branch to fork `{0}` from. \
        Pass `--from <branch>` to choose the branch it starts from."
    )]
    DetachedHeadWithoutFrom(String),

    #[error("failed to execute git command: {0}")]
    GitCommand(String),

    #[error("failed to parse git output: {0}")]
    GitParse(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

/// One worktree listed in an [`WorktreeError::AmbiguousWorktree`] error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeCandidate {
    pub branch: Option<String>,
    pub basename: String,
    pub path: std::path::PathBuf,
}

fn ambiguous_message(name: &str, candidates: &[WorktreeCandidate]) -> String {
    let mut message = format!("'{name}' matches more than one worktree:");
    for candidate in candidates {
        let branch = candidate.branch.as_deref().unwrap_or("(detached)");
        message.push_str(&format!(
            "\n  - branch {branch}, directory {}, at {}",
            candidate.basename,
            candidate.path.display()
        ));
    }
    message.push_str("\nUse a branch or directory name that matches only one of them.");
    message
}
