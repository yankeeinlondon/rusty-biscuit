use clap::{Parser, Subcommand};
use clap_complete::engine::ArgValueCompleter;
use clap_complete::Shell;

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

    /// Generate shell completions for the specified shell
    #[arg(long, value_name = "SHELL", hide = true)]
    pub completions: Option<Shell>,
}

#[derive(Subcommand, Clone)]
pub enum Commands {
    /// List all worktrees with status indicators
    List {
        /// Override the git graph width (e.g. "70", "70ch", "50%")
        #[arg(long, short = 'w', value_name = "WIDTH")]
        width: Option<String>,
    },

    /// Create a new worktree from a branch name
    Create {
        /// Branch name for the new worktree
        branch: String,

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
}

fn complete_worktree_names(_current: &std::ffi::OsStr) -> Vec<clap_complete::CompletionCandidate> {
    worktree::worktree::worktree_names()
        .into_iter()
        .map(clap_complete::CompletionCandidate::new)
        .collect()
}

const AFTER_HELP: &str = "\
Examples:
  wt                    List all worktrees (default)
  wt list               List all worktrees with status
  wt create feature/x   Create a new worktree for branch feature/x
  wt create fix/y --stay Create without changing directory
  wt go feature-x       Navigate to a worktree
  wt go base            Navigate back to the base checkout

Shell Integration (cd wrapper + completions):
  source <(wt --completions bash)              Add to ~/.bashrc
  source <(wt --completions zsh)               Add to ~/.zshrc
  source (wt --completions fish | psub)        Add to config.fish";
