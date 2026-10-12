use thiserror::Error;

use crate::config::BaseDirSource;

#[derive(Debug, Error)]
pub enum WorktreeError {
    #[error("git is not installed or not found on PATH")]
    GitNotFound,

    #[error("This directory is not inside a Git repository. Change to a repository or one of its worktrees, then run wt again.")]
    NotInGitRepo,

    #[error(
        "base directory not configured: set the WT environment variable or create ~/.worktree.json"
    )]
    BaseDirectoryNotConfigured,

    /// The base-directory value is empty or only whitespace.
    #[error("The base directory from {origin} is empty. {}", remedy(origin))]
    BaseDirectoryEmpty { origin: BaseDirSource },

    /// The value is written in another operating system's path syntax, such
    /// as `b:\wt` on a POSIX host or `/home/me/wt` on Windows.
    #[error(
        "Base directory '{value}' from {origin} uses {} path syntax on this {} host. {}",
        foreign_style(),
        host_style(),
        remedy(origin)
    )]
    BaseDirectoryForeignPath { value: String, origin: BaseDirSource },

    /// The value is in this host's syntax but is not an absolute path.
    #[error(
        "Base directory '{value}' from {origin} is not an absolute path.{} {}",
        home_hint(value),
        remedy(origin)
    )]
    BaseDirectoryNotAbsolute { value: String, origin: BaseDirSource },

    /// The value names something that exists but is not a directory.
    #[error(
        "Base directory '{value}' from {origin} exists but is not a directory. {}",
        remedy(origin)
    )]
    BaseDirectoryNotDirectory { value: String, origin: BaseDirSource },

    /// The value is not valid Unicode, so it can be neither read as text nor
    /// saved to `~/.worktree.json` without changing it.
    #[error(
        "Base directory '{}' from {origin} is not valid Unicode, which wt needs to read and save it. {}",
        .value.display(),
        remedy(origin)
    )]
    BaseDirectoryInvalidEncoding {
        value: std::path::PathBuf,
        origin: BaseDirSource,
    },

    /// Nothing exists at the base-directory path.
    #[error(
        "Base directory '{value}' from {origin} does not exist. If it is on a drive or mount that is not available, restore the mount. Otherwise {}",
        remedy_lowercase(origin)
    )]
    BaseDirectoryNotFound { value: String, origin: BaseDirSource },

    /// The base directory's state could not be read for a reason other than
    /// its absence (for example, permission denied).
    #[error("Cannot check base directory '{value}' from {origin}: {cause}. {}", remedy(origin))]
    BaseDirectoryUnreadable {
        value: String,
        origin: BaseDirSource,
        #[source]
        cause: std::io::Error,
    },

    /// The base directory has a `.git` file or folder directly inside it.
    /// Directories further inside a repository are not checked.
    #[error(
        "Base directory '{value}' from {origin} is the root of a Git repository (it has .git directly inside). {}",
        remedy(origin)
    )]
    BaseDirectoryIsGitRepo { value: String, origin: BaseDirSource },

    /// `~/.worktree.json` exists but cannot be parsed or is missing `base_dir`.
    #[error("~/.worktree.json has an invalid format: {message}")]
    ConfigInvalidFormat {
        config_path: std::path::PathBuf,
        message: String,
    },

    /// `~/.worktree.json` `base_dir` points to a directory with `.git`
    /// directly inside it (a repository root).
    #[error(
        "Base directory '{dir}' from the config file {} is the root of a Git repository (it has .git directly inside). {}",
        .config_path.display(),
        remedy(&BaseDirSource::ConfigFile(config_path.clone()))
    )]
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

    /// The command stopped because continuing would lose work, or because
    /// what it would delete could not be checked. Nothing was removed, but
    /// "nothing removed" is not "nothing changed": `wt remove` may already
    /// have run `git worktree repair` to make the files checkable, and that
    /// repair is never rolled back. The message is Prose markup, one line per
    /// `\n`, explaining what to do, and says when Git metadata may have
    /// changed.
    #[error("{0}")]
    RefusedToLoseWork(String),

    /// The environment prevents the command (for example, no shell wrapper to
    /// move the caller); nothing was removed and no `--force-*` flag helps.
    /// The message is Prose markup, one line per `\n`, explaining what to do.
    #[error("{0}")]
    BlockedByEnvironment(String),

    /// The worktree's path no longer holds a real directory when removal is
    /// about to run (for example, a link replaced the checkout after it was
    /// checked). Nothing was removed.
    #[error("{} is no longer the worktree's own directory (it is a link or not a directory), so nothing was removed", .0.display())]
    NotARealDirectory(std::path::PathBuf),

    /// A branch about to be deleted no longer points at the tip its deletion
    /// was decided on. The branch is kept.
    #[error("branch {branch} moved from {expected} to {found} after it was checked, so it was kept")]
    BranchMoved { branch: String, expected: String, found: String },

    /// Other programs work in the worktree directory: `processes` stand in
    /// it, or, when empty, the Windows probe found it held (an open file).
    /// Nothing was removed.
    #[error("{}", in_use_message(path, processes))]
    DirectoryInUse {
        path: std::path::PathBuf,
        processes: Vec<sniff::os::WorkingProcess>,
    },

    /// `git worktree remove` unregistered the worktree and then failed, and
    /// the rest of its folder could not be deleted either. Nothing after the
    /// worktree (branch, origin) was removed.
    #[error(
        "git unregistered worktree {} but could not delete all of its folder ({reason}). \
        Nothing else was removed. Delete what is left with: {}",
        .path.display(),
        delete_command(path)
    )]
    FolderNotFullyRemoved { path: std::path::PathBuf, reason: String },

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

    /// `~/.wt.json` could not be updated; an existing file is left as it was.
    /// `--ignore-api` needs an `origin` that identifies a repository.
    #[error("--ignore-api needs an origin remote that names a repository on a host; {0}")]
    NoRepositoryIdentity(String),

    #[error("cannot record the preference in {}: {reason}", path.display())]
    PreferenceUnwritable { path: std::path::PathBuf, reason: String },

    #[error("failed to execute git command: {0}")]
    GitCommand(String),

    #[error("failed to parse git output: {0}")]
    GitParse(String),

    #[error("cannot read include rules at {}: {reason}", path.display())]
    IncludeRulesIndeterminate { path: std::path::PathBuf, reason: String },

    #[error("cannot discover included files: {0}")]
    IncludeSetDiscovery(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Requested timings could not be stated exactly in the timings
    /// document.
    #[error(transparent)]
    Timings(#[from] crate::timing::TimingsError),
}

/// One worktree listed in an [`WorktreeError::AmbiguousWorktree`] error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeCandidate {
    pub branch: Option<String>,
    pub basename: String,
    pub path: std::path::PathBuf,
}

/// What to do about a rejected base directory, by where it came from.
fn remedy(origin: &BaseDirSource) -> String {
    let example = example_base_dir();
    match origin {
        BaseDirSource::Env => format!(
            "Set WT to an absolute directory on this host (for example {example}), or unset it. \
            Editing ~/.worktree.json alone does not override a nonempty WT."
        ),
        BaseDirSource::ConfigFile(path) => format!(
            "Set base_dir in {} to an absolute directory on this host (for example {example}), \
            or remove that file and run wt create again in an interactive terminal to choose one.",
            path.display()
        ),
        BaseDirSource::Prompt => {
            format!("Choose an absolute directory on this host (for example {example}).")
        }
    }
}

fn remedy_lowercase(origin: &BaseDirSource) -> String {
    let remedy = remedy(origin);
    let mut chars = remedy.chars();
    match chars.next() {
        Some(first) => first.to_lowercase().chain(chars).collect(),
        None => remedy,
    }
}

fn example_base_dir() -> &'static str {
    if cfg!(windows) {
        r"C:\Users\me\wt"
    } else if cfg!(target_os = "macos") {
        "/Users/me/wt"
    } else {
        "/home/me/wt"
    }
}

fn host_style() -> &'static str {
    if cfg!(windows) { "Windows" } else { "POSIX" }
}

fn foreign_style() -> &'static str {
    if cfg!(windows) { "POSIX" } else { "Windows" }
}

/// wt never expands `~`, so a home-relative value gets a pointer to the fix.
fn home_hint(value: &str) -> &'static str {
    if value.trim_start().starts_with('~') {
        " wt does not expand ~; write the full home-directory path instead."
    } else {
        ""
    }
}

fn in_use_message(path: &std::path::Path, processes: &[sniff::os::WorkingProcess]) -> String {
    let path = path.display();
    if processes.is_empty() {
        return format!("Nothing was removed: the folder {path} is in use by another program.");
    }
    // One entry per program name, keeping the order processes were found in.
    let mut programs: Vec<(&str, Vec<u32>)> = Vec::new();
    for process in processes {
        match programs.iter_mut().find(|(name, _)| *name == process.name) {
            Some((_, pids)) => pids.push(process.pid),
            None => programs.push((&process.name, vec![process.pid])),
        }
    }
    let listed = programs
        .iter()
        .map(|(name, pids)| {
            let ids = pids.iter().map(u32::to_string).collect::<Vec<_>>().join(", ");
            format!("{name} (pid{} {ids})", if pids.len() == 1 { "" } else { "s" })
        })
        .collect::<Vec<_>>()
        .join(", ");
    let who = if processes.len() == 1 { "another program" } else { "other programs" };
    format!(
        "Nothing was removed: the folder {path} is in use by {who}: {listed}. \
        Close them, or move them out of it, and run wt remove again."
    )
}

fn delete_command(path: &std::path::Path) -> String {
    if cfg!(windows) {
        format!("Remove-Item -Recurse -Force \"{}\"", path.display())
    } else {
        format!("rm -rf \"{}\"", path.display())
    }
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn config() -> BaseDirSource {
        BaseDirSource::ConfigFile(PathBuf::from("/tmp/home/.worktree.json"))
    }

    #[test]
    fn origin_display_names_each_source() {
        assert_eq!(BaseDirSource::Env.to_string(), "the WT environment variable");
        assert_eq!(config().to_string(), "the config file /tmp/home/.worktree.json");
        assert_eq!(BaseDirSource::Prompt.to_string(), "the setup prompt");
    }

    #[test]
    fn incident_message_names_value_config_and_remedy() {
        let error = WorktreeError::BaseDirectoryForeignPath {
            value: r"b:\wt".into(),
            origin: config(),
        };
        let message = error.to_string();
        assert!(message.contains(r"'b:\wt'"), "{message}");
        assert!(message.contains("/tmp/home/.worktree.json"), "{message}");
        assert!(message.contains("Set base_dir in /tmp/home/.worktree.json"), "{message}");
        let (foreign, host) = if cfg!(windows) { ("POSIX", "Windows") } else { ("Windows", "POSIX") };
        assert!(
            message.contains(&format!("uses {foreign} path syntax on this {host} host")),
            "{message}"
        );
    }

    #[test]
    fn foreign_message_never_calls_a_drive_relative_value_absolute() {
        let message = WorktreeError::BaseDirectoryForeignPath {
            value: "b:wt".into(),
            origin: BaseDirSource::Env,
        }
        .to_string();
        assert!(!message.contains("is absolute"), "{message}");
        assert!(!message.contains("an absolute path on"), "{message}");
    }

    #[test]
    fn env_remedy_says_editing_json_does_not_override() {
        let message = WorktreeError::BaseDirectoryNotAbsolute {
            value: "wt".into(),
            origin: BaseDirSource::Env,
        }
        .to_string();
        assert!(message.contains("'wt' from the WT environment variable"), "{message}");
        assert!(message.contains("or unset it"), "{message}");
        assert!(message.contains("Editing ~/.worktree.json alone does not override a nonempty WT"), "{message}");
    }

    #[test]
    fn home_relative_value_gets_full_path_hint_from_every_source() {
        for origin in [BaseDirSource::Env, config(), BaseDirSource::Prompt] {
            let message = WorktreeError::BaseDirectoryNotAbsolute {
                value: "~/wt".into(),
                origin: origin.clone(),
            }
            .to_string();
            assert!(message.contains("write the full home-directory path"), "{message}");
            assert!(message.contains(&origin.to_string()), "{message}");
        }
        let plain = WorktreeError::BaseDirectoryNotAbsolute {
            value: "./wt".into(),
            origin: BaseDirSource::Prompt,
        }
        .to_string();
        assert!(!plain.contains("home-directory"), "{plain}");
    }

    #[test]
    fn missing_directory_names_config_and_mount_recovery() {
        let message = WorktreeError::BaseDirectoryNotFound {
            value: "/mnt/b/wt".into(),
            origin: config(),
        }
        .to_string();
        assert!(message.contains("'/mnt/b/wt' from the config file /tmp/home/.worktree.json does not exist"), "{message}");
        assert!(message.contains("restore the mount"), "{message}");
        assert!(message.contains("set base_dir in /tmp/home/.worktree.json"), "{message}");
    }

    #[test]
    fn every_value_variant_names_value_origin_and_remedy() {
        let origin = BaseDirSource::Prompt;
        let remedy = "Choose an absolute directory on this host";
        let errors = [
            WorktreeError::BaseDirectoryNotDirectory { value: "/srv/file".into(), origin: origin.clone() },
            WorktreeError::BaseDirectoryInvalidEncoding { value: "/srv/file".into(), origin: origin.clone() },
            WorktreeError::BaseDirectoryUnreadable {
                value: "/srv/file".into(),
                origin: origin.clone(),
                cause: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
            },
            WorktreeError::BaseDirectoryIsGitRepo { value: "/srv/file".into(), origin: origin.clone() },
        ];
        for error in errors {
            let message = error.to_string();
            assert!(message.contains("'/srv/file' from the setup prompt"), "{message}");
            assert!(message.contains(remedy), "{message}");
        }
        let empty = WorktreeError::BaseDirectoryEmpty { origin }.to_string();
        assert!(empty.contains("from the setup prompt is empty"), "{empty}");
        assert!(empty.contains(remedy), "{empty}");
    }

    #[test]
    fn unreadable_keeps_its_io_cause() {
        use std::error::Error as _;
        let error = WorktreeError::BaseDirectoryUnreadable {
            value: "/srv/wt".into(),
            origin: BaseDirSource::Env,
            cause: std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        };
        let cause = error.source().expect("an underlying I/O cause");
        assert_eq!(
            cause.downcast_ref::<std::io::Error>().map(std::io::Error::kind),
            Some(std::io::ErrorKind::PermissionDenied)
        );
    }

    #[test]
    fn repository_root_messages_identify_origin_and_root_only_check() {
        let message = WorktreeError::ConfigBaseDirIsGitRepo {
            config_path: "/tmp/home/.worktree.json".into(),
            dir: "/srv/repo".into(),
        }
        .to_string();
        assert!(message.contains("'/srv/repo' from the config file /tmp/home/.worktree.json"), "{message}");
        assert!(message.contains(".git directly inside"), "{message}");
        assert!(message.contains("Set base_dir in /tmp/home/.worktree.json"), "{message}");
    }
}
