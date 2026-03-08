use clap::{CommandFactory, Parser};
use claudine::events::Provider;
use color_eyre::eyre::Result;
use tracing::level_filters::LevelFilter;

mod args;
mod commands;
mod log;
mod output;
mod provider_values;

use args::{Cli, Commands};

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();

    // Default levels come from -v/-vv and can be overridden by RUST_LOG/DEBUG.
    let env_filter = build_env_filter(cli.verbose);

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
        Some(Commands::Actions(args)) => commands::actions::run(args, cli.verbose > 0),
        Some(Commands::Skills(args)) => commands::skills::run(args, cli.verbose > 0).await,
        Some(Commands::Agents(args)) => commands::agents::run(args, cli.verbose > 0).await,
        Some(Commands::SlashCommands(args)) => {
            commands::slash_commands::run(args, cli.verbose > 0).await
        }
        Some(Commands::Providers) => commands::providers::run(),
        Some(Commands::Logs(args)) => commands::logs::run(args).await,
        Some(Commands::Uninstall(args)) => commands::uninstall::run(args),
        Some(Commands::Claude(args)) => {
            commands::wrap::run_provider_wrapper(Provider::Claude, args)
        }
        Some(Commands::Codex(args)) => commands::wrap::run_provider_wrapper(Provider::Codex, args),
        Some(Commands::Gemini(args)) => {
            commands::wrap::run_provider_wrapper(Provider::Gemini, args)
        }
        Some(Commands::Kimi(args)) => {
            commands::wrap::run_provider_wrapper(Provider::KimiCode, args)
        }
        Some(Commands::Qwen(args)) => {
            commands::wrap::run_provider_wrapper(Provider::QwenCode, args)
        }
        Some(Commands::Opencode(args)) => {
            commands::wrap::run_provider_wrapper(Provider::OpenCode, args)
        }
        Some(Commands::Goose(args)) => commands::wrap::run_provider_wrapper(Provider::Goose, args),
        None => {
            // No subcommand given - show help
            Cli::command().print_help()?;
            Ok(())
        }
    }
}

fn build_env_filter(verbose: u8) -> tracing_subscriber::EnvFilter {
    let default_directive = verbosity_level(verbose).into();

    if std::env::var_os("RUST_LOG").is_some() {
        return tracing_subscriber::EnvFilter::builder()
            .with_default_directive(default_directive)
            .from_env_lossy();
    }

    if let Some(debug) = std::env::var("DEBUG")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
    {
        let directive = normalize_debug_override(&debug).unwrap_or(debug);
        return tracing_subscriber::EnvFilter::builder()
            .with_default_directive(default_directive)
            .parse_lossy(directive);
    }

    tracing_subscriber::EnvFilter::builder()
        .with_default_directive(default_directive)
        .from_env_lossy()
}

fn verbosity_level(verbose: u8) -> LevelFilter {
    match verbose {
        0 => LevelFilter::WARN,
        1 => LevelFilter::INFO,
        _ => LevelFilter::DEBUG,
    }
}

fn normalize_debug_override(raw: &str) -> Option<String> {
    match raw.to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some("debug".to_string()),
        "0" | "false" | "no" | "off" => Some("warn".to_string()),
        _ => None,
    }
}
