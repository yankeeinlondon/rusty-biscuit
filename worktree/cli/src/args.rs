use clap::{Parser, Subcommand};
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
    List,

    /// Create a new worktree from a branch name
    Create {
        /// Branch name for the new worktree
        branch: String,

        /// Create the worktree but don't change into it
        #[arg(long)]
        stay: bool,
    },

    /// Navigate to a worktree or the base checkout
    Go {
        /// Worktree name or "base" for the main checkout
        name: String,
    },
}

const AFTER_HELP: &str = "\
Examples:
  wt                    List all worktrees (default)
  wt list               List all worktrees with status
  wt create feature/x   Create a new worktree for branch feature/x
  wt create fix/y --stay Create without changing directory
  wt go feature-x       Navigate to a worktree
  wt go base            Navigate back to the base checkout

Shell Completions:
  wt --completions bash   Show setup for Bash
  wt --completions zsh    Show setup for Zsh
  wt --completions fish   Show setup for Fish";
