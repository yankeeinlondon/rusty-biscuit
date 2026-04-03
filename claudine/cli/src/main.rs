use clap::Parser;
use claudine::events::Provider;
use color_eyre::eyre::Result;

mod args;
mod commands;
mod log;
mod output;
mod provider_values;
mod telemetry;

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
    telemetry::init_tracing(cli.debug);
    let root_span = telemetry::root_span(&cli);
    let _root_guard = root_span.enter();

    if cli.help || cli.command.is_none() {
        return commands::help::run();
    }

    match cli.command.unwrap() {
        Commands::Handle(args) => commands::handle::run(args).await,
        Commands::Completions(args) => commands::completions::run(args),
        Commands::Init(args) => commands::init::run(args).await,
        Commands::Sync(args) => commands::sync::run(args).await,
        Commands::Hooks(args) => commands::hooks::run(args, cli.verbose > 0),
        Commands::Actions(args) => commands::actions::run(args, cli.verbose > 0),
        Commands::Skills(args) => commands::skills::run(args, cli.verbose > 0).await,
        Commands::Agents(args) => commands::agents::run(args, cli.verbose > 0).await,
        Commands::SlashCommands(args) => commands::slash_commands::run(args, cli.verbose > 0).await,
        Commands::Providers => commands::providers::run(),
        Commands::Logs(args) => commands::logs::run(args).await,
        Commands::Uninstall(args) => commands::uninstall::run(args),
        Commands::Mcp(args) => commands::mcp::run(args),
        Commands::Claude(args) => {
            commands::wrap::run_provider_wrapper(Provider::Claude, args, cli.verbose)
        }
        Commands::Codex(args) => {
            commands::wrap::run_provider_wrapper(Provider::Codex, args, cli.verbose)
        }
        Commands::Gemini(args) => {
            commands::wrap::run_provider_wrapper(Provider::Gemini, args, cli.verbose)
        }
        Commands::Kimi(args) => {
            commands::wrap::run_provider_wrapper(Provider::KimiCode, args, cli.verbose)
        }
        Commands::Qwen(args) => {
            commands::wrap::run_provider_wrapper(Provider::QwenCode, args, cli.verbose)
        }
        Commands::Opencode(args) => {
            commands::wrap::run_provider_wrapper(Provider::OpenCode, args, cli.verbose)
        }
        Commands::Goose(args) => {
            commands::wrap::run_provider_wrapper(Provider::Goose, args, cli.verbose)
        }
        Commands::Compose(args) => commands::compose::run_compose(args, cli.verbose),
        Commands::InlineCompose(args) => commands::compose::run_inline_compose(args, cli.verbose),
    }
}
