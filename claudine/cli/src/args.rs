use crate::commands;
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum DebugLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl DebugLevel {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

/// Claudine — cross-agent hook/event system for agentic CLIs.
#[derive(Parser)]
#[command(
    name = "claudine",
    version,
    about,
    disable_help_flag = true,
    disable_help_subcommand = true
)]
pub(crate) struct Cli {
    /// Increase presentation detail for human-facing output.
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Set Claudine's diagnostic tracing level.
    #[arg(long, value_enum, value_name = "LEVEL", global = true)]
    pub debug: Option<DebugLevel>,

    /// Strip ANSI escape codes from all output.
    #[arg(long, global = true)]
    pub plain: bool,

    /// Print help.
    #[arg(short, long)]
    pub help: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub(crate) enum Commands {
    /// Handle an incoming event from a provider hook.
    #[command(hide = true)]
    Handle(commands::handle::HandleArgs),
    /// Generate shell completions.
    Completions(commands::completions::CompletionsArgs),
    /// Hidden completion engine invoked by generated bash/zsh/fish scripts.
    #[command(name = "__complete", hide = true)]
    Complete(commands::completions::CompleteArgs),
    /// Manage Claudine configuration with a TUI.
    Config(commands::config_tui::ConfigArgs),
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
    #[allow(clippy::enum_variant_names)]
    SlashCommands(commands::slash_commands::SlashCommandsArgs),
    /// Show provider capability matrix (skill/slash/agent/hooks).
    Providers(commands::providers::ProvidersArgs),
    /// Inspect the compiled signal-detection tables and replay their
    /// evidence fixtures (dev/CI).
    Signals(commands::signals::SignalsArgs),
    /// Query and sync Claudine JSONL logs through the reporting index.
    Logs(commands::logs::LogsArgs),
    /// Remove Claudine hooks from all agents.
    Uninstall(commands::uninstall::UninstallArgs),
    /// Manage MCP (Model Context Protocol) servers.
    Mcp(commands::mcp::McpArgs),
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
    /// Wrap Kilo Code with Claudine preflight/env handling.
    Kilo(commands::wrap::WrapperArgs),
    /// Wrap Pi with Claudine preflight/env handling.
    Pi(commands::wrap::WrapperArgs),
    /// Wrap Antigravity (the `agy` CLI) with Claudine preflight/env handling.
    Antigravity(commands::wrap::WrapperArgs),
    /// Compose a Markdown document and send as prompt to an agentic CLI.
    Compose(commands::compose::ComposeArgs),
    /// Inline composition: use frontmatter prompt to generate and replace body.
    #[command(name = "inline-compose")]
    InlineCompose(commands::compose::InlineComposeArgs),
    /// Run a serial sequence of composition steps from a single document.
    Sequence(commands::sequence::SequenceArgs),
    /// Show the mesh NOW view: live sessions across rendezvous hosts.
    Dashboard(commands::dashboard::DashboardArgs),
    /// Show Darkmatter runtime context, expression engine, and side effects.
    Context(commands::context::ContextArgs),
    /// Show the diagnostic error-code contract (codes, dispositions, details).
    Errors(commands::errors::ErrorsArgs),
}
