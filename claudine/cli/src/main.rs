use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

mod commands;
mod log;

/// Claudine — cross-agent hook/event system for agentic CLIs.
#[derive(Parser)]
#[command(name = "claudine", version, about)]
pub(crate) struct Cli {
    /// Increase verbosity (-v for info, -vv for debug).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,

    #[command(subcommand)]
    command: Option<Commands>,
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
    /// Remove Claudine hooks from all agents.
    Uninstall(commands::uninstall::UninstallArgs),
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Initialize tracing based on verbosity
    let filter = match cli.verbose {
        0 => "warn",
        1 => "info",
        _ => "debug",
    };
    // Allow DEBUG env var override
    let env_filter = tracing_subscriber::EnvFilter::try_from_env("DEBUG")
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(filter));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    match cli.command {
        Some(Commands::Handle(args)) => commands::handle::run(args).await,
        Some(Commands::DryRun(args)) => commands::dry_run::run(args).await,
        Some(Commands::Completions(args)) => commands::completions::run(args),
        Some(Commands::About) => commands::about::run(),
        Some(Commands::Init(args)) => commands::init::run(args).await,
        Some(Commands::Link(args)) => commands::link::run(args),
        Some(Commands::Sync(args)) => commands::sync::run(args).await,
        Some(Commands::Hooks(args)) => commands::hooks::run(args, cli.verbose > 0),
        Some(Commands::Uninstall(args)) => commands::uninstall::run(args),
        None => {
            // Default: read from stdin as handle command
            commands::handle::run_default().await
        }
    }
}
