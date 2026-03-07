use crate::commands;
use clap::{Parser, Subcommand};

/// Claudine — cross-agent hook/event system for agentic CLIs.
#[derive(Parser)]
#[command(name = "claudine", version, about)]
pub(crate) struct Cli {
    /// Increase verbosity (-v for info, -vv for debug).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Handle an incoming event from a provider hook.
    Handle(commands::handle::HandleArgs),
    /// Show what would happen for an event (no side effects).
    DryRun(commands::dry_run::DryRunArgs),
    /// Generate shell completions.
    Completions(commands::completions::CompletionsArgs),
    /// Show detailed help and usage information.
    About,
    /// Interactive setup wizard.
    Init(commands::init::InitArgs),
    /// Link skills and commands across providers.
    Link(commands::link::LinkArgs),
    /// Re-sync hook registrations with detected agents.
    Sync(commands::sync::SyncArgs),
    /// Show registered hooks for all detected agents.
    Hooks(commands::hooks::HooksArgs),
    /// Show which actions are configured and for which events.
    Actions(commands::actions::ActionsArgs),
    /// List available skills and their scopes.
    Skills(commands::skills::SkillsArgs),
    /// List available agent definitions and their scopes.
    Agents(commands::agents::AgentsArgs),
    /// List available slash commands and their scopes.
    #[command(name = "commands")]
    SlashCommands(commands::slash_commands::SlashCommandsArgs),
    /// Show provider capability matrix (skill/slash/agent/hooks).
    Providers,
    /// Query and sync Claudine JSONL logs through the reporting index.
    Logs(commands::logs::LogsArgs),
    /// Remove Claudine hooks from all agents.
    Uninstall(commands::uninstall::UninstallArgs),
    /// Wrap Claude Code with Claudine preflight/env handling.
    Claude(commands::wrap::WrapperArgs),
    /// Wrap Codex CLI with Claudine preflight/env handling.
    Codex(commands::wrap::WrapperArgs),
    /// Wrap Gemini CLI with Claudine preflight/env handling.
    Gemini(commands::wrap::WrapperArgs),
    /// Wrap Kimi Code with Claudine preflight/env handling.
    Kimi(commands::wrap::WrapperArgs),
    /// Wrap Qwen Code with Claudine preflight/env handling.
    Qwen(commands::wrap::WrapperArgs),
    /// Wrap OpenCode with Claudine preflight/env handling.
    Opencode(commands::wrap::WrapperArgs),
    /// Wrap Goose with Claudine preflight/env handling.
    Goose(commands::wrap::WrapperArgs),
}
