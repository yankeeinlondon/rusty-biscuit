use clap::Parser;
use claudine::events::Provider;
use color_eyre::eyre::Result;

mod args;
mod cli_utils;
mod commands;
mod log;
mod output;
mod provider_values;
mod table_utils;
mod telemetry;

use args::{Cli, Commands};

fn wrapper_command(
    command: Commands,
) -> std::result::Result<(Provider, commands::wrap::WrapperArgs), Commands> {
    match command {
        Commands::Claude(args) => Ok((Provider::Claude, args)),
        Commands::Codex(args) => Ok((Provider::Codex, args)),
        Commands::Gemini(args) => Ok((Provider::Gemini, args)),
        Commands::Kimi(args) => Ok((Provider::KimiCode, args)),
        Commands::Qwen(args) => Ok((Provider::QwenCode, args)),
        Commands::Opencode(args) => Ok((Provider::OpenCode, args)),
        Commands::Goose(args) => Ok((Provider::Goose, args)),
        other => Err(other),
    }
}

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

    let command = match wrapper_command(cli.command.unwrap()) {
        Ok((provider, args)) => {
            return commands::wrap::run_provider_wrapper(provider, args, cli.verbose);
        }
        Err(command) => command,
    };

    match command {
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
        Commands::Claude(_)
        | Commands::Codex(_)
        | Commands::Gemini(_)
        | Commands::Kimi(_)
        | Commands::Qwen(_)
        | Commands::Opencode(_)
        | Commands::Goose(_) => unreachable!("wrapper commands are handled before this match"),
        Commands::Compose(args) => commands::compose::run_compose(args, cli.verbose),
        Commands::InlineCompose(args) => commands::compose::run_inline_compose(args, cli.verbose),
        Commands::Sequence(args) => commands::sequence::run_sequence(args, cli.verbose),
    }
}
