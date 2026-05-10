//! Install subcommand argument types.

use clap::Subcommand;

/// Shared flag group for `install` and `install-plan` subcommands across every
/// program category.
#[derive(clap::Args, Debug, Clone, Default)]
pub struct InstallCommandArgs {
    /// Program name to install (binary name or identifier)
    pub program: Option<String>,

    /// Build the plan and print what would happen; do not execute
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the interactive confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Force a specific package manager (e.g. `brew`, `cargo`, `pnpm`)
    #[arg(long, value_name = "MANAGER")]
    pub via: Option<String>,

    /// Force the plan builder to treat sudo as unavailable
    #[arg(long)]
    pub no_sudo: bool,

    /// Bypass the host capability cache and rebuild it
    #[arg(short = 'f', long)]
    pub force: bool,
}

/// Discriminator returned by `Commands::install_command_args()` so dispatch
/// can tell `install` and `install-plan` apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstallCommandKind {
    Install,
    InstallPlan,
}

#[derive(Subcommand, Debug, Clone)]
pub enum AllProgramAction {
    /// Install a program (interactive picker if no name given)
    Install(InstallCommandArgs),

    /// Show the install plan without executing anything
    #[command(name = "install-plan")]
    InstallPlan(InstallCommandArgs),
}
