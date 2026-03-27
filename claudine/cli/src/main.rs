use clap::Parser;
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

    // Pre-scan for --plain to disable clap ANSI styling before parsing
    let is_plain = std::env::args().any(|a| a == "--plain");
    if is_plain {
        // NO_COLOR is a well-established convention for disabling terminal colors
        unsafe { std::env::set_var("NO_COLOR", "1") };
    }

    let cli = Cli::parse();
    log::set_plain(cli.plain);

    // Default levels come from -v/-vv and can be overridden by RUST_LOG/DEBUG.
    let env_filter = build_env_filter(cli.verbose);

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_writer(std::io::stderr)
        .init();

    match cli.command {
        Some(Commands::Handle(args)) => commands::handle::run(args).await,
        Some(Commands::Completions(args)) => commands::completions::run(args),
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
        Some(Commands::Mcp(args)) => commands::mcp::run(args),
        Some(Commands::Claude(args)) => {
            commands::wrap::run_provider_wrapper(Provider::Claude, args, cli.verbose)
        }
        Some(Commands::Codex(args)) => {
            commands::wrap::run_provider_wrapper(Provider::Codex, args, cli.verbose)
        }
        Some(Commands::Gemini(args)) => {
            commands::wrap::run_provider_wrapper(Provider::Gemini, args, cli.verbose)
        }
        Some(Commands::Kimi(args)) => {
            commands::wrap::run_provider_wrapper(Provider::KimiCode, args, cli.verbose)
        }
        Some(Commands::Qwen(args)) => {
            commands::wrap::run_provider_wrapper(Provider::QwenCode, args, cli.verbose)
        }
        Some(Commands::Opencode(args)) => {
            commands::wrap::run_provider_wrapper(Provider::OpenCode, args, cli.verbose)
        }
        Some(Commands::Goose(args)) => {
            commands::wrap::run_provider_wrapper(Provider::Goose, args, cli.verbose)
        }
        Some(Commands::Compose(args)) => commands::compose::run_compose(args, cli.verbose),
        Some(Commands::ComposeInline(args)) => {
            commands::compose::run_compose_inline(args, cli.verbose)
        }
        None => commands::help::run(),
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
        0 | 1 => LevelFilter::WARN,
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
