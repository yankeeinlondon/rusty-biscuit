use std::ffi::OsString;

use claudine::provider::Provider;
use color_eyre::eyre::{Report, Result};

mod args;
mod argv;
mod cli_utils;
mod commands;
mod completion;
mod log;
mod output;
mod perf;
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
        Commands::Kilo(args) => Ok((Provider::Kilo, args)),
        Commands::Pi(args) => Ok((Provider::Pi, args)),
        Commands::Antigravity(args) => Ok((Provider::Antigravity, args)),
        other => Err(Box::new(other)),
    }
}

/// Attach the pre-clap agent tail ([`argv::partition_composition_tail`]) to
/// the composition command's shared args. A no-op for every other command and
/// when the tail is empty.
fn inject_provider_tail(cli: &mut Cli, tail: argv::ProviderArgs) {
    let Some(command) = cli.command.as_mut() else {
        return;
    };
    let shared = match command {
        Commands::Compose(args) => &mut args.shared,
        Commands::InlineCompose(args) => &mut args.shared,
        Commands::Sequence(args) => &mut args.shared,
        _ => return,
    };
    shared.provider_args = tail.args;
    shared.provider_args_explicit = tail.explicit;
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
/// Both wrapper and non-wrapper paths go through a manually constructed
/// `clap::Command` so we can:
///
/// - Inject `--help`/`-h` (an `ArgAction::Help`) on every non-wrapper
///   subcommand. The root `Cli` sets `disable_help_flag = true` for custom
///   help routing, and clap propagates that disable through to subcommands,
///   so without injection `claudine <subcmd> --help` errors with
///   `unexpected argument '--help'`.
/// - Mark each wrapper subcommand with `ignore_errors(true)` so unknown
///   flags destined for the wrapped agent CLI flow into the `passthrough`
///   bucket instead of aborting with a clap error.
///
/// Wrapper subcommands deliberately keep their own `help: bool` field plus
/// `print_wrapper_help`; we do not inject `ArgAction::Help` on those, so
/// `claudine codex --help` still routes through the custom wrapper-help
/// renderer.
///
/// On `try_get_matches_from`:
///
/// - `Ok` → build `Cli` from matches.
/// - `Err(DisplayHelp | DisplayVersion | DisplayHelpOnMissingArgumentOrSubcommand)`
///   → call `err.exit()` so the injected `ArgAction::Help` prints clap's
///   per-subcommand help screen and exits (matching `Cli::parse_from`'s
///   exit-on-help semantics).
/// - Any other error → fall back to `Cli::parse_from`, which produces the
///   standard clap error message and exits. The fallback is defensive: in
///   practice the manual pass cannot fail for argv we accept here, but a
///   future clap upgrade that tightens `from_arg_matches` cannot silently
///   drop into an `unwrap()` panic.
fn parse_cli_from(argv: &[OsString]) -> Cli {
    use clap::error::ErrorKind;
    use clap::{CommandFactory, FromArgMatches, Parser};

    let is_wrapper = argv::find_subcommand(argv, argv::WRAPPER_SUBCOMMANDS).is_some();

    let mut cmd = <Cli as CommandFactory>::command();
    inject_subcommand_help_flag(&mut cmd);

    if is_wrapper {
        for name in argv::WRAPPER_SUBCOMMANDS {
            if let Some(sub) = cmd.find_subcommand_mut(*name) {
                let muted = std::mem::replace(sub, clap::Command::new("__placeholder__"));
                let _ = std::mem::replace(sub, muted.ignore_errors(true));
            }
        }
    }

    match cmd.try_get_matches_from(argv.iter().cloned()) {
        Ok(matches) => Cli::from_arg_matches(&matches)
            .unwrap_or_else(|_| Cli::parse_from(argv.iter().cloned())),
        Err(err) => match err.kind() {
            ErrorKind::DisplayHelp
            | ErrorKind::DisplayVersion
            | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => err.exit(),
            _ => Cli::parse_from(argv.iter().cloned()),
        },
    }
}

/// Subcommand names that must NOT receive an `ArgAction::Help` injection.
///
/// Wrapper subcommands manage their own help via a manual `help: bool` field
/// plus `print_wrapper_help`. Hidden subcommands either declare their own help
/// surface or are never invoked interactively by users.
fn skip_help_injection(name: &str) -> bool {
    argv::WRAPPER_SUBCOMMANDS.contains(&name) || name == "__complete"
}

/// Add `--help`/`-h` (an `ArgAction::Help`) to every subcommand for which
/// [`skip_help_injection`] returns `false`.
///
/// This is necessary because the root `Cli` sets `disable_help_flag = true`
/// (so the custom grouped help can fire on `claudine` / `claudine --help`),
/// and clap propagates that setting to subcommands. Without re-injection,
/// `claudine <subcmd> --help` errors with `unexpected argument '--help'`.
fn inject_subcommand_help_flag(cmd: &mut clap::Command) {
    use clap::{Arg, ArgAction};

    let names: Vec<String> = cmd
        .get_subcommands()
        .filter(|sub| !skip_help_injection(sub.get_name()))
        .map(|sub| sub.get_name().to_string())
        .collect();

    for name in names {
        if let Some(sub) = cmd.find_subcommand_mut(&name) {
            let owned = std::mem::replace(sub, clap::Command::new("__placeholder__"));
            let injected = owned.arg(
                Arg::new("help")
                    .short('h')
                    .long("help")
                    .action(ArgAction::Help)
                    .help("Print help"),
            );
            let _ = std::mem::replace(sub, injected);
        }
    }
}

fn main() -> Result<()> {
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();
    color_eyre::install()?;

    match run() {
        Ok(()) => Ok(()),
        Err(report) => {
            render_top_level_error(&report);
            std::process::exit(1);
        }
    }
}

/// Try to render `report` as a darkmatter `BlockError` via the cause-chain
/// walker. Falls back to the CLI's generic `log::error` formatting
/// (preserving the styled `Error:` label) when no cause in the chain
/// implements `BlockError`.
///
/// Process-level tests may set `CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT` to capture
/// the same structured diagnostic selected for rendering. Capture failures are
/// deliberately ignored so this test seam cannot replace the original error.
fn render_top_level_error(report: &Report) {
    if let Some(path) = std::env::var_os("CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT")
        && let Some(snapshot) =
            claudine::diagnostics::DiagnosticSnapshot::select(report.as_ref())
        && let Ok(encoded) = serde_json::to_vec(&snapshot)
    {
        let _ = std::fs::write(path, encoded);
    }

    // A lifecycle evaluation error already rendered its styled block to stderr
    // at its catch point (Decision #2), before any catch events fired. Suppress
    // the duplicate styled block here while still exiting non-zero.
    if output::error_walker::evaluation_error_already_emitted(report) {
        return;
    }
    let term = log::terminal();
    if let Some(rendered) = output::error_walker::try_render_block_report(report, &term) {
        log::message("");
        log::message(&rendered);
        log::message("");
        return;
    }
    log::error(&report.to_string());
}

fn run() -> Result<()> {
    let process_start = std::time::Instant::now();

    // When invoked as a completion subprocess (COMPLETE=<shell> claudine …),
    // `maybe_complete` writes either a registration snippet or candidate
    // list to stdout and exits before returning. In normal runs it is a
    // no-op and control falls through to argv normalization. Must run
    // *before* `argv::normalize(...)` so the normalizer's COMPLETE guard
    // never has to absorb a completion subprocess on the happy path.
    completion::maybe_complete();

    let raw_argv: Vec<OsString> = std::env::args_os().collect();
    let perf_bootstrap = perf::scan_perf_bootstrap(&raw_argv);

    let arg_parse_start = std::time::Instant::now();
    let normalized: Vec<OsString> = argv::normalize(raw_argv);

    // Ownership partition (replaces the retired Rule 3): split composition argv
    // into the Claudine argv handed to clap and the agent tail forwarded to the
    // provider. Non-composition argv passes through unchanged with an empty tail.
    let (argv, provider_tail) = argv::partition_composition_tail(normalized)?;

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
    runtime.block_on(async_main(
        argv,
        provider_tail,
        perf_bootstrap,
        arg_parse_start,
        process_start,
    ))
}

async fn async_main(
    argv: Vec<OsString>,
    provider_tail: argv::ProviderArgs,
    perf_bootstrap: perf::PerfBootstrap,
    arg_parse_start: std::time::Instant,
    process_start: std::time::Instant,
) -> Result<()> {
    let mut cli = parse_cli_from(&argv);
    let launch_mode = if matches!(cli.command, Some(Commands::Handle(_))) {
        claudine::child_environment::LaunchDirectoryMode::ProviderHook
    } else {
        claudine::child_environment::LaunchDirectoryMode::Ordinary
    };
    claudine::child_environment::initialize_process_launch_directory(launch_mode)?;
    // Attach the partitioned agent tail to the composition command. The tail is
    // captured before clap and never reconstructed from clap matches or argv.
    inject_provider_tail(&mut cli, provider_tail);
    let perf_arg_parsing = arg_parse_start.elapsed();
    log::set_plain(cli.plain);

    let tracing_start = std::time::Instant::now();
    telemetry::init_tracing(cli.debug);
    let root_span = telemetry::root_span(&cli);
    let _root_guard = root_span.enter();
    let perf_tracing_init = tracing_start.elapsed();

    if cli.help || cli.command.is_none() {
        return commands::help::run();
    }

    let perf_enabled = perf_bootstrap.enabled;

    let pre_dispatch = process_start.elapsed();

    let command = match wrapper_command(cli.command.unwrap()) {
        Ok((provider, args)) => {
            // Wrapper commands also need config — check before launching
            let config_start = std::time::Instant::now();
            ensure_config_exists().await?;
            let perf_config_loading = config_start.elapsed();

            let startup_timings = if perf_enabled {
                Some(perf::StartupTimings {
                    arg_parsing: perf_arg_parsing,
                    tracing_init: perf_tracing_init,
                    config_loading: perf_config_loading,
                    pre_dispatch,
                    prep_phase: std::time::Duration::ZERO,
                    process_start,
                    prep_substages: Vec::new(),
                })
            } else {
                None
            };

            return commands::wrap::run_provider_wrapper(
                provider,
                args,
                cli.verbose,
                startup_timings,
            );
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

    let perf_config_loading = if needs_config {
        let config_start = std::time::Instant::now();
        ensure_config_exists().await?;
        config_start.elapsed()
    } else {
        std::time::Duration::ZERO
    };

    let startup_timings = if perf_enabled {
        Some(perf::StartupTimings {
            arg_parsing: perf_arg_parsing,
            tracing_init: perf_tracing_init,
            config_loading: perf_config_loading,
            pre_dispatch,
            prep_phase: std::time::Duration::ZERO,
            process_start,
            prep_substages: Vec::new(),
        })
    } else {
        None
    };

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
        Commands::Providers(args) => commands::providers::run(args),
        Commands::Signals(args) => commands::signals::run(args),
        Commands::Logs(args) => commands::logs::run(args).await,
        Commands::Uninstall(args) => commands::uninstall::run(args),
        Commands::Mcp(args) => commands::mcp::run(args),
        Commands::Claude(_)
        | Commands::Codex(_)
        | Commands::Gemini(_)
        | Commands::Kimi(_)
        | Commands::Qwen(_)
        | Commands::Opencode(_)
        | Commands::Goose(_)
        | Commands::Kilo(_)
        | Commands::Pi(_)
        | Commands::Antigravity(_) => unreachable!("wrapper commands are handled before this match"),
        Commands::Compose(args) => {
            commands::compose::run_compose(args, cli.verbose, startup_timings)
        }
        Commands::InlineCompose(args) => {
            commands::compose::run_inline_compose(args, cli.verbose, startup_timings)
        }
        Commands::Sequence(args) => {
            commands::sequence::run_sequence(args, cli.verbose, startup_timings)
        }
        Commands::Dashboard(args) => commands::dashboard::run(args).await,
        Commands::Context(args) => commands::context::run(args),
        Commands::Errors(args) => commands::errors::run(args),
    }
}
