use std::ffi::OsString;

use claudine::events::Provider;
use color_eyre::eyre::Result;

mod args;
mod argv;
mod cli_utils;
mod commands;
mod completion;
mod log;
mod output;
mod provider_values;
mod table_utils;
mod telemetry;

use args::{Cli, Commands};

fn wrapper_command(
    command: Commands,
) -> std::result::Result<(Provider, commands::wrap::WrapperArgs), Box<Commands>> {
    match command {
        Commands::Claude(args) => Ok((Provider::Claude, args)),
        Commands::Codex(args) => Ok((Provider::Codex, args)),
        Commands::Gemini(args) => Ok((Provider::Gemini, args)),
        Commands::Kimi(args) => Ok((Provider::KimiCode, args)),
        Commands::Qwen(args) => Ok((Provider::QwenCode, args)),
        Commands::Opencode(args) => Ok((Provider::OpenCode, args)),
        Commands::Goose(args) => Ok((Provider::Goose, args)),
        other => Err(Box::new(other)),
    }
}

/// Check if the Claudine config file exists and is valid. If not (missing or
/// old-format that was backed up), run the initialization process so a
/// config is available for the command about to run.
async fn ensure_config_exists() -> Result<()> {
    let config_path = claudine::dispatch::loader::user_config_path();
    if !config_path.exists() {
        commands::init_wizard::run_initialization().await?;
        return Ok(());
    }

    // The file exists, but it may be old-format. Attempt a load —
    // load_claudine_config backs up stale configs and returns ConfigNotFound.
    match claudine::dispatch::loader::load_claudine_config(Some(&config_path), None) {
        Ok(_) => Ok(()),
        Err(claudine::error::ClaudineError::ConfigNotFound(_)) => {
            // Old-format was detected and backed up; re-run initialization.
            commands::init_wizard::run_initialization().await?;
            Ok(())
        }
        // Other errors (parse, validation) should propagate.
        Err(e) => Err(e.into()),
    }
}

/// CLI parsing over an already-normalized argv.
///
/// Non-wrapper subcommands go through a single strict `Cli::parse_from`
/// call — that's the common path and the one that produces rich clap
/// errors on unknown args or invalid values.
///
/// Wrapper subcommands (`claude`, `codex`, …) need a lenient pass so that
/// unknown flags destined for the wrapped agent CLI flow into the
/// `passthrough` bucket instead of aborting with a clap error. That lenient
/// pass is constructed by cloning the clap `Command` and marking each
/// wrapper subcommand with `ignore_errors(true)`, then calling
/// `try_get_matches_from` + `Cli::from_arg_matches`.
///
/// Both `unwrap_or_else` / `Err(_) =>` branches fall back to
/// `Cli::parse_from(...)` on the same normalized argv. Those fallbacks are
/// defensive: in practice the lenient pass cannot fail for
/// wrapper-targeted argv, because `ignore_errors(true)` absorbs every
/// unknown token. The fallbacks exist so a future clap upgrade that tightens
/// `from_arg_matches` cannot silently drop into an `unwrap()` panic — the
/// strict pass then produces the user-facing clap error message. This does
/// mean the lenient diagnostic is swallowed on the rare failure path; if
/// that becomes a problem, emit a `tracing::debug!` before falling through.
fn parse_cli_from(argv: &[OsString]) -> Cli {
    use clap::{CommandFactory, FromArgMatches, Parser};

    let is_wrapper = argv::find_subcommand(argv, argv::WRAPPER_SUBCOMMANDS).is_some();

    if !is_wrapper {
        return Cli::parse_from(argv.iter().cloned());
    }

    // Lenient pass: allow unknown args so they flow into passthrough.
    // Build a command tree where wrapper subcommands ignore unknown args.
    let mut cmd = <Cli as CommandFactory>::command();
    for name in argv::WRAPPER_SUBCOMMANDS {
        if let Some(sub) = cmd.find_subcommand_mut(*name) {
            let muted = std::mem::replace(sub, clap::Command::new("__placeholder__"));
            let _ = std::mem::replace(sub, muted.ignore_errors(true));
        }
    }

    match cmd.try_get_matches_from(argv.iter().cloned()) {
        Ok(matches) => Cli::from_arg_matches(&matches)
            .unwrap_or_else(|_| Cli::parse_from(argv.iter().cloned())),
        Err(_) => Cli::parse_from(argv.iter().cloned()),
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;

    // When invoked as a completion subprocess (COMPLETE=<shell> claudine …),
    // `maybe_complete` writes either a registration snippet or candidate
    // list to stdout and exits before returning. In normal runs it is a
    // no-op and control falls through to argv normalization. Must run
    // *before* `argv::normalize(...)` so the normalizer's COMPLETE guard
    // never has to absorb a completion subprocess on the happy path.
    completion::maybe_complete();

    let argv: Vec<OsString> = argv::normalize(std::env::args_os().collect());

    // Pre-scan the normalized argv for --plain so clap's ANSI styling is
    // disabled before parsing. Uses the same token stream the parse will see.
    let is_plain = argv.iter().any(|tok| tok.to_str() == Some("--plain"));
    if is_plain {
        // SAFETY: this runs during single-threaded process bootstrap, before
        // the Tokio runtime is constructed and before any worker threads or
        // background tasks exist. No concurrent environment access is possible
        // yet, so mutating the process environment upholds Rust 2024's
        // `std::env::set_var` safety contract.
        unsafe {
            std::env::set_var("NO_COLOR", "1");
        }
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    runtime.block_on(async_main(argv))
}

async fn async_main(argv: Vec<OsString>) -> Result<()> {
    let cli = parse_cli_from(&argv);
    log::set_plain(cli.plain);
    telemetry::init_tracing(cli.debug);
    let root_span = telemetry::root_span(&cli);
    let _root_guard = root_span.enter();

    if cli.help || cli.command.is_none() {
        return commands::help::run();
    }

    let command = match wrapper_command(cli.command.unwrap()) {
        Ok((provider, args)) => {
            // Wrapper commands also need config — check before launching
            ensure_config_exists().await?;
            return commands::wrap::run_provider_wrapper(provider, args, cli.verbose);
        }
        Err(command) => *command,
    };

    // Commands that must work without config (handle is a hook callback,
    // completions is shell setup, __complete runs under a shell completion
    // pipeline). Everything else requires config.
    let needs_config = !matches!(
        command,
        Commands::Handle(_) | Commands::Completions(_) | Commands::Complete(_)
    );
    if needs_config {
        ensure_config_exists().await?;
    }

    match command {
        Commands::Handle(args) => commands::handle::run(args).await,
        Commands::Completions(args) => commands::completions::run(args),
        Commands::Complete(args) => commands::completions::run_complete(args),
        Commands::Config(args) => commands::config_tui::run(args).await,
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
