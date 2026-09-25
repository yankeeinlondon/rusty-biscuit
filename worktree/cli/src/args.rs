use clap::{Parser, Subcommand};
use clap_complete::Shell;
use clap_complete::engine::ArgValueCompleter;

/// A simple CLI for working with git worktrees
#[derive(Parser)]
#[command(
    name = "wt",
    version,
    about = "A simple CLI for working with git worktrees",
    after_help = AFTER_HELP,
    disable_help_subcommand = true,
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Override the git graph width (e.g. "70", "70ch", "50%")
    #[arg(long, short = 'w', value_name = "WIDTH", global = true)]
    pub width: Option<String>,

    /// Show detailed commit history for the current worktree
    #[arg(long, short = 'v', global = true)]
    pub verbose: bool,

    /// Emit a performance report to stderr after command completion.
    #[arg(long, global = true)]
    pub perf: bool,

    /// Print the shell integration (cd wrapper + completions) for a shell
    #[arg(long, value_name = "SHELL", hide = true, value_parser = parse_shell)]
    pub completions: Option<Shell>,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// List all worktrees with status indicators
    List,

    /// Create a new worktree from a branch name
    Create {
        /// Branch name for the new worktree
        branch: String,

        /// Local branch to fork the new branch from (default: the current branch)
        #[arg(long, value_name = "BASE", add = ArgValueCompleter::new(complete_local_branches))]
        from: Option<String>,

        /// Create the worktree but don't change into it
        #[arg(long)]
        stay: bool,
    },

    /// Navigate to a worktree or the base checkout
    #[command(disable_help_flag = true)]
    Go {
        /// Worktree name or "base" for the main checkout
        #[arg(add = ArgValueCompleter::new(complete_worktree_names))]
        name: String,

        #[arg(long, action = clap::ArgAction::Help, hide = true)]
        help: Option<bool>,
    },

    /// Remove a worktree and, when its commits are safe elsewhere, its branch
    Remove {
        /// Worktree name (branch or directory) to remove
        #[arg(
            required_unless_present = "handoff",
            add = ArgValueCompleter::new(complete_worktree_names)
        )]
        name: Option<String>,

        /// Remove the worktree even if it has uncommitted or ignored files
        #[arg(long)]
        force_worktree: bool,

        /// Delete the local branch even if its commits exist nowhere else
        #[arg(long)]
        force_branch: bool,

        /// Also delete the branch on origin (closes any open PR for it)
        #[arg(long)]
        force_remote: bool,

        /// Finish a removal the shell wrapper handed off (internal)
        #[arg(
            long,
            hide = true,
            value_name = "TOKEN",
            conflicts_with_all = ["name", "force_worktree", "force_branch", "force_remote"]
        )]
        handoff: Option<String>,
    },
}

fn complete_worktree_names(_current: &std::ffi::OsStr) -> Vec<clap_complete::CompletionCandidate> {
    worktree::worktree::worktree_names()
        .into_iter()
        .map(clap_complete::CompletionCandidate::new)
        .collect()
}

fn complete_local_branches(_current: &std::ffi::OsStr) -> Vec<clap_complete::CompletionCandidate> {
    worktree::worktree::local_branches()
        .into_iter()
        .map(clap_complete::CompletionCandidate::new)
        .collect()
}

fn parse_shell(value: &str) -> Result<Shell, String> {
    let supported = crate::shell_integration::SUPPORTED_SHELLS;
    if !supported.contains(&value) {
        return Err(format!("expected one of: {}", supported.join(", ")));
    }
    value.parse()
}

const AFTER_HELP: &str = "\
Examples:
  wt                    List all worktrees (default)
  wt list               List all worktrees with status
  wt create feature/x   Create a new worktree for branch feature/x
  wt create fix/y --from feat/theme
                        Fork fix/y from feat/theme instead of the current branch
  wt create fix/y --stay Create without changing directory
  wt go feature-x       Navigate to a worktree
  wt go base            Navigate back to the base checkout
  wt remove old-cleanup Remove the worktree and its branch when the branch's
                        commits are safe elsewhere (merged, pushed, or on
                        another branch); otherwise keep the branch and say why
  wt remove spike-parser --force-branch
                        Also delete a branch whose commits exist nowhere else
  wt remove fix-wt-ux --force-worktree --force-branch --force-remote
                        Remove everything: uncommitted files, the branch, and
                        the branch on origin (closing any open PR)

Shell Integration (cd wrapper + completions; `wt go` needs it):
  source <(wt --completions bash)              Add to ~/.bashrc
  source <(wt --completions zsh)               Add to ~/.zshrc
  wt --completions fish | source               Add to config.fish
  wt --completions powershell | Out-String | Invoke-Expression
                                               Add to $PROFILE";
