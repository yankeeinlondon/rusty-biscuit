use thiserror::Error;

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

fn in_use_message(path: &std::path::Path, processes: &[sniff::os::WorkingProcess]) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    in_use_message_at(path, processes, now)
}

/// Longest command line shown per program before it is cut with `…`.
const COMMAND_SHOWN: usize = 160;

/// Lists only the topmost process of each tree standing in the folder: a
/// script's loop starts a fresh child every second, so its children would
/// show pids that are gone by the time the user reads them, and ending the
/// parent ends them.
fn in_use_message_at(
    path: &std::path::Path,
    processes: &[sniff::os::WorkingProcess],
    now: u64,
) -> String {
    let path = path.display();
    if processes.is_empty() {
        return format!("Nothing was removed: the folder {path} is in use by another program.");
    }
    let is_listed = |pid: Option<u32>| pid.is_some_and(|pid| processes.iter().any(|p| p.pid == pid));
    let (roots, children): (Vec<_>, Vec<_>) =
        processes.iter().partition(|process| !is_listed(process.parent));

    let who = match roots.len() {
        1 => "another program".to_string(),
        count => format!("{count} other programs"),
    };
    let started = match children.len() {
        0 => String::new(),
        1 => " (and 1 process they started)".to_string(),
        count => format!(" (and {count} processes they started)"),
    };
    let mut message = format!("Nothing was removed: the folder {path} is in use by {who}{started}:");
    for process in &roots {
        let mut facts = vec![format!("pid {}", process.pid)];
        if process.started > 0 {
            let age = crate::util::age_text(now.saturating_sub(process.started));
            facts.push(format!("started {age} ago"));
        }
        // On Unix an orphan is adopted by pid 1; one still standing in a
        // worktree is usually a leaked script nobody will come back for.
        if cfg!(unix) && process.parent == Some(1) {
            facts.push("orphaned".to_string());
        }
        message.push_str(&format!("\n  - {}: {}", facts.join(", "), command_text(process)));
    }
    let pids = roots.iter().map(|p| p.pid.to_string()).collect::<Vec<_>>();
    let end = if cfg!(windows) {
        format!("Stop-Process -Id {}", pids.join(", "))
    } else {
        format!("kill {}", pids.join(" "))
    };
    message.push_str(&format!(
        "\nClose them, or move them out of it, and run wt remove again. To end them: {end}"
    ));
    message
}

/// The command line, or the executable's name when the OS hides it.
fn command_text(process: &sniff::os::WorkingProcess) -> String {
    if process.command.is_empty() {
        return process.name.clone();
    }
    let mut text = process.command.join(" ");
    if text.chars().count() > COMMAND_SHOWN {
        text = text.chars().take(COMMAND_SHOWN).collect::<String>() + "…";
    }
    text
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
    use super::*;
    use sniff::os::WorkingProcess;
    use std::path::Path;

    const NOW: u64 = 1_000_000;

    fn process(pid: u32, parent: u32, command: &[&str], age: u64) -> WorkingProcess {
        WorkingProcess {
            pid,
            name: command.first().map_or("x", |c| c).rsplit('/').next().unwrap().to_string(),
            command: command.iter().map(|arg| arg.to_string()).collect(),
            parent: Some(parent),
            started: NOW - age,
            cwd: "/wt/feat-x".into(),
        }
    }

    #[test]
    fn a_script_is_shown_by_its_command_line_and_its_children_are_folded_into_it() {
        let processes = [
            process(11231, 1, &["/bin/sh", "/tmp/.tmpA/bin/opencode", "run"], 4 * 86_400),
            process(78796, 11231, &["/bin/sleep", "1"], 0),
        ];

        let message = in_use_message_at(Path::new("/wt/feat-x"), &processes, NOW);

        assert!(message.contains("in use by another program (and 1 process they started):"), "{message}");
        assert!(message.contains("/bin/sh /tmp/.tmpA/bin/opencode run"), "{message}");
        assert!(!message.contains("78796"), "{message}");
    }

    #[test]
    fn each_program_shows_its_age_and_whether_its_parent_exited() {
        let processes = [
            process(11231, 1, &["/bin/sh", "a.sh"], 4 * 86_400),
            process(25749, 900, &["rust-analyzer"], 120),
        ];

        let message = in_use_message_at(Path::new("/wt/feat-x"), &processes, NOW);

        let orphan = if cfg!(unix) { ", orphaned" } else { "" };
        assert!(
            message.contains(&format!("pid 11231, started 4 days ago{orphan}: /bin/sh a.sh")),
            "{message}"
        );
        assert!(message.contains("pid 25749, started 2 min ago: rust-analyzer"), "{message}");
        let end = if cfg!(windows) { "Stop-Process -Id 11231, 25749" } else { "kill 11231 25749" };
        assert!(message.ends_with(&format!("To end them: {end}")), "{message}");
    }

    #[test]
    fn a_hidden_command_line_falls_back_to_the_name_and_a_long_one_is_cut() {
        let mut hidden = process(7, 1, &[], 0);
        hidden.name = "launchd-helper".into();
        let long_arg = "x".repeat(400);
        let long = process(8, 1, &["/bin/sh", &long_arg], 0);

        let message = in_use_message_at(Path::new("/wt/feat-x"), &[hidden, long], NOW);

        assert!(message.contains(": launchd-helper\n"), "{message}");
        assert!(message.contains(&format!("/bin/sh {}…", "x".repeat(COMMAND_SHOWN - 8))), "{message}");
    }
}
