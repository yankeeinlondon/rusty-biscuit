// Production builds may not construct or clear a child command directly; see
// `child_environment` and `clippy.toml`. Tests keep the plain constructors.
#![cfg_attr(not(test), deny(clippy::disallowed_methods))]

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
mod request;
mod shutdown;
mod steering;
mod table_utils;
mod telemetry;
mod terminal_gate;

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

/// Attach the arguments after the composition file
/// ([`argv::partition_composition_tail`]) to the composition command's shared
/// args, for type-aware ownership once the file is read. A no-op for every
/// other command.
fn inject_caller_arguments(cli: &mut Cli, arguments: claudine::composition::ArgumentsAfterFile) {
    let Some(command) = cli.command.as_mut() else {
        return;
    };
    let shared = match command {
        Commands::Compose(args) => &mut args.shared,
        Commands::InlineCompose(args) => &mut args.shared,
        Commands::Sequence(args) => &mut args.shared,
        _ => return,
    };
    shared.caller_arguments = arguments;
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

/// The clap command model shared by normal parsing and composition help.
///
/// Returned unbuilt: [`root_help_request`] adds `ignore_errors`, a global
/// setting clap propagates to subcommands only while building, so a caller
/// that renders from the model calls `Command::build` itself.
fn build_cli_command(is_wrapper: bool) -> clap::Command {
    use clap::CommandFactory;

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
    cmd
}

/// The built clap command for composition subcommand `command_name`.
///
/// The root is built first so the child carries the inherited global options
/// and the parent-qualified usage line (`claudine compose …`).
fn composition_help_command(command_name: &str, plain: bool) -> clap::Command {
    let mut root = build_cli_command(false);
    root.build();
    let child = root
        .find_subcommand_mut(command_name)
        .unwrap_or_else(|| unreachable!("`{command_name}` is a composition subcommand"));
    let child = std::mem::replace(child, clap::Command::new("__placeholder__"));
    if plain {
        child.color(clap::ColorChoice::Never)
    } else {
        child
    }
}

/// Print `command_name`'s compact clap help to stdout. No argument is parsed,
/// so nothing about the user's invocation can fail here.
fn print_composition_help(command_name: &str, plain: bool) -> std::io::Result<()> {
    composition_help_command(command_name, plain).print_help()?;
    println!();
    Ok(())
}

/// Answer a help request [`argv::select_composition_help`] selected, then
/// exit 0 before any composition bootstrap.
///
/// `plain` comes from an exact `--plain` before the first literal `--`, on
/// either side of the command.
fn answer_selected_help(destination: argv::HelpDestination, raw: &[OsString]) -> Result<()> {
    let stop = argv::first_dash_dash_index(raw).unwrap_or(raw.len());
    let plain = raw[..stop].iter().any(|token| token.to_str() == Some("--plain"));
    match destination {
        argv::HelpDestination::Root => {
            log::set_plain(plain);
            commands::help::run()
        }
        argv::HelpDestination::Composition(command) => {
            Ok(print_composition_help(command, plain)?)
        }
    }
}

/// CLI parsing over an already-normalized argv.
///
/// Both wrapper and non-wrapper paths go through [`build_cli_command`] so we
/// can:
///
/// - Inject `--help`/`-h` (an `ArgAction::Help`) on every non-wrapper
///   subcommand, nested ones included. The root `Cli` sets
///   `disable_help_flag = true` for custom help routing, and clap propagates
///   that disable through to subcommands, so without injection
///   `claudine <subcmd> --help` errors with `unexpected argument '--help'`.
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
/// - `Err(_)` with a root help request → the grouped help, via
///   [`root_help_request`]; help never waits on a required argument.
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
    use clap::{FromArgMatches, Parser};

    let is_wrapper = argv::find_subcommand(argv, argv::WRAPPER_SUBCOMMANDS).is_some();
    let cmd = build_cli_command(is_wrapper);

    match cmd.clone().try_get_matches_from(argv.iter().cloned()) {
        Ok(matches) => Cli::from_arg_matches(&matches)
            .unwrap_or_else(|_| Cli::parse_from(argv.iter().cloned())),
        Err(err) => {
            if let Some(cli) = root_help_request(cmd, argv) {
                return cli;
            }
            match err.kind() {
                ErrorKind::DisplayHelp
                | ErrorKind::DisplayVersion
                | ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand => err.exit(),
                _ => Cli::parse_from(argv.iter().cloned()),
            }
        }
    }
}

/// The root help request in `argv`, read without subcommand validation.
///
/// The root `help` is a plain Boolean so the grouped help screen can render
/// it, which means clap only reports it after validating the selected
/// subcommand, so `claudine --help mcp alias` would otherwise fail on the
/// missing required argument before help could display. A root help request
/// outranks every subcommand validation error; the global presentation
/// options are kept so `--plain` and `--debug` still apply to the help screen.
/// Composition help never reaches this point: [`run`] answers it from the
/// original argv first.
fn root_help_request(cmd: clap::Command, argv: &[OsString]) -> Option<Cli> {
    let matches = cmd
        .ignore_errors(true)
        .try_get_matches_from(argv.iter().cloned())
        .ok()?;
    matches.get_flag("help").then(|| Cli {
        verbose: matches.get_count("verbose"),
        debug: matches
            .try_get_one::<args::DebugLevel>("debug")
            .ok()
            .flatten()
            .copied(),
        plain: matches.get_flag("plain"),
        help: true,
        command: None,
    })
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
/// [`skip_help_injection`] returns `false`, and to every subcommand nested
/// beneath one (`mcp alias`, …).
///
/// This is necessary because the root `Cli` sets `disable_help_flag = true`
/// (so the custom grouped help can fire on `claudine` / `claudine --help`),
/// and clap propagates that setting to subcommands at every depth. Without
/// re-injection, `claudine <subcmd> --help` errors with
/// `unexpected argument '--help'`. Clap acts on an `ArgAction::Help` before
/// it validates required arguments, so `mcp alias --help` needs no server.
fn inject_subcommand_help_flag(cmd: &mut clap::Command) {
    let names: Vec<String> = cmd
        .get_subcommands()
        .filter(|sub| !skip_help_injection(sub.get_name()))
        .map(|sub| sub.get_name().to_string())
        .collect();

    for name in names {
        if let Some(sub) = cmd.find_subcommand_mut(&name) {
            let owned = std::mem::replace(sub, clap::Command::new("__placeholder__"));
            let _ = std::mem::replace(sub, with_help_flag(owned));
        }
    }
}

fn with_help_flag(cmd: clap::Command) -> clap::Command {
    use clap::{Arg, ArgAction};

    cmd.arg(
        Arg::new("help")
            .short('h')
            .long("help")
            .action(ArgAction::Help)
            .help("Print help"),
    )
    .mut_subcommands(with_help_flag)
}

fn main() -> Result<()> {
    if let Some(code) = run_audio_worker_if_requested()? {
        std::process::exit(code);
    }
    #[cfg(feature = "test-fixtures")]
    commands::wrap::exec::completion_fixture::initialize()?;
    rustls::crypto::ring::default_provider()
        .install_default()
        .ok();
    terminal_gate::install_report_hooks()?;
    claudine::render::console::set_console_writer(terminal_gate::write_library_line);

    // `run` returns only for an error raised before the runtime exists; every
    // other exit goes through `shutdown::finish` inside the runtime.
    let Err(report) = run();
    render_top_level_error(&report);
    shutdown::exit_before_runtime(1)
}

fn run_audio_worker_if_requested() -> Result<Option<i32>> {
    if let Some(code) = playa::detached::run_if_worker() {
        return Ok(Some(code));
    }

    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    Ok(runtime.block_on(biscuit_speaks::run_if_worker()))
}

/// Try to render `report` as a darkmatter `BlockError` via the cause-chain
/// walker. Falls back to the CLI's generic `log::error` formatting
/// (preserving the styled `Error:` label) when no cause in the chain
/// implements `BlockError`.
fn render_top_level_error(report: &Report) {
    #[cfg(feature = "test-fixtures")]
    write_diagnostic_snapshot_for_tests(report);

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

/// Test seam, compiled only with the `test-fixtures` feature (never into a
/// default or installed build): with `CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT=<path>`
/// set, writes the structured diagnostic selected for rendering to `<path>`.
///
/// Capture failures are deliberately ignored so this seam cannot replace the
/// original error.
#[cfg(feature = "test-fixtures")]
fn write_diagnostic_snapshot_for_tests(report: &Report) {
    use claudine::diagnostics::DiagnosticSnapshotExt;

    if let Some(path) = std::env::var_os("CLAUDINE_TEST_DIAGNOSTIC_SNAPSHOT")
        && let Some(snapshot) =
            claudine::diagnostics::DiagnosticSnapshot::select(report.as_ref())
        && let Ok(encoded) = serde_json::to_vec(&snapshot)
    {
        let _ = std::fs::write(path, encoded);
    }
}

fn run() -> Result<std::convert::Infallible> {
    let process_start = std::time::Instant::now();

    // When invoked as a completion subprocess (COMPLETE=<shell> claudine …),
    // `maybe_complete` writes either a registration snippet or candidate
    // list to stdout and exits before returning. In normal runs it is a
    // no-op and control falls through to argv normalization. Must run
    // *before* `argv::normalize(...)` so the normalizer's COMPLETE guard
    // never has to absorb a completion subprocess on the happy path.
    completion::maybe_complete();

    let raw_argv: Vec<OsString> = std::env::args_os().collect();

    // Composition help is selected from the original argv, before
    // normalization moves the token and before the ownership partition can
    // reject an incomplete invocation. It reads no input and exits here.
    if let Some(destination) =
        argv::select_composition_help(&raw_argv, argv::completion_mode_active())
    {
        answer_selected_help(destination, &raw_argv)?;
        shutdown::exit_before_runtime(0);
    }

    let perf_bootstrap = perf::scan_perf_bootstrap(&raw_argv);

    let arg_parse_start = std::time::Instant::now();
    let normalized: Vec<OsString> = argv::normalize(raw_argv);

    // Ownership partition (replaces the retired Rule 3): split composition argv
    // into the Claudine argv handed to clap and the agent tail forwarded to the
    // provider. Non-composition argv passes through unchanged with an empty tail.
    let (argv, caller_arguments) = argv::partition_composition_tail(normalized)?;

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
    Ok(runtime.block_on(async_main(
        argv,
        caller_arguments,
        perf_bootstrap,
        arg_parse_start,
        process_start,
    )))
}

/// Run the command, render any error, and exit through the shutdown path.
///
/// The error block renders before the drain so it precedes any
/// pending-delivery warning.
async fn async_main(
    argv: Vec<OsString>,
    caller_arguments: claudine::composition::ArgumentsAfterFile,
    perf_bootstrap: perf::PerfBootstrap,
    arg_parse_start: std::time::Instant,
    process_start: std::time::Instant,
) -> std::convert::Infallible {
    let code = match dispatch(
        argv,
        caller_arguments,
        perf_bootstrap,
        arg_parse_start,
        process_start,
    )
    .await
    {
        Ok(code) => code,
        Err(report) => {
            render_top_level_error(&report);
            1
        }
    };
    shutdown::finish(code).await
}

/// Parse the CLI and run the selected command, returning its exit code.
async fn dispatch(
    argv: Vec<OsString>,
    caller_arguments: claudine::composition::ArgumentsAfterFile,
    perf_bootstrap: perf::PerfBootstrap,
    arg_parse_start: std::time::Instant,
    process_start: std::time::Instant,
) -> Result<i32> {
    let mut cli = parse_cli_from(&argv);
    let launch_mode = if matches!(cli.command, Some(Commands::Handle(_))) {
        claudine::child_environment::LaunchDirectoryMode::ProviderHook
    } else {
        claudine::child_environment::LaunchDirectoryMode::Ordinary
    };
    let launch_dir = claudine::child_environment::initialize_process_launch_directory(launch_mode)?;
    request::initialize(launch_dir)?;
    // Attach the partitioned agent tail to the composition command. The tail is
    // captured before clap and never reconstructed from clap matches or argv.
    inject_caller_arguments(&mut cli, caller_arguments);
    let perf_arg_parsing = arg_parse_start.elapsed();
    log::set_plain(cli.plain);

    let tracing_start = std::time::Instant::now();
    telemetry::init_tracing(cli.debug);
    let root_span = telemetry::root_span(&cli);
    let _root_guard = root_span.enter();
    let perf_tracing_init = tracing_start.elapsed();

    if cli.help || cli.command.is_none() {
        return commands::help::run().map(|()| 0);
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
    // pipeline, and steer must never change configuration or prompt a
    // `--json` caller). Everything else requires config, and a missing one
    // starts the interactive setup wizard.
    let needs_config = !matches!(
        command,
        Commands::Handle(_) | Commands::Completions(_) | Commands::Complete(_) | Commands::Steer(_)
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

    let result = match command {
        Commands::Handle(args) => return commands::handle::run(args).await,
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
            return commands::compose::run_compose(args, cli.verbose, startup_timings);
        }
        Commands::InlineCompose(args) => {
            return commands::compose::run_inline_compose(args, cli.verbose, startup_timings);
        }
        Commands::Sequence(args) => {
            return commands::sequence::run_sequence(args, cli.verbose, startup_timings);
        }
        Commands::Steer(args) => return Ok(commands::steer::run(args).await),
        Commands::Dashboard(args) => commands::dashboard::run(args).await,
        Commands::Context(args) => commands::context::run(args),
        Commands::Errors(args) => commands::errors::run(args),
    };
    result.map(|()| 0)
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    /// Clap checks a command's definition lazily, only for the subcommand an
    /// argv selects; this walks the whole tree so an injected `-h` that
    /// collides with a nested subcommand's own short flag fails here.
    #[test]
    fn injected_help_flags_are_valid_on_every_subcommand() {
        let mut cmd = <super::Cli as CommandFactory>::command();
        super::inject_subcommand_help_flag(&mut cmd);
        cmd.debug_assert();
    }
}

#[cfg(test)]
mod composition_help {
    use std::collections::BTreeSet;
    use std::ffi::OsString;

    use clap::ArgAction;
    use clap::error::ErrorKind;
    use claudine::provider::PROVIDERS_DISPLAY_ORDER;

    use super::{argv::COMPOSITION_SUBCOMMANDS, build_cli_command, composition_help_command};

    fn rendered(command: &str) -> String {
        composition_help_command(command, true)
            .render_help()
            .to_string()
    }

    #[test]
    fn shared_model_builds_without_definition_failures() {
        // `debug_assert` builds the tree and panics on duplicate arguments,
        // groups naming missing arguments, or conflicting short flags.
        build_cli_command(false).debug_assert();
        build_cli_command(true).debug_assert();
    }

    #[test]
    fn each_command_renders_its_own_parent_qualified_screen() {
        for command in COMPOSITION_SUBCOMMANDS {
            let help = rendered(command);
            assert!(
                help.contains(&format!("Usage: claudine {command} [OPTIONS] <ARG>...")),
                "{command}: {help}"
            );
            for option in ["--help", "--plain", "--verbose", "--debug", "--provider", "--set"] {
                assert!(help.contains(option), "{command} lacks {option}: {help}");
            }
            assert!(
                help.contains(&format!("claudine {command} ")) && help.contains("-- --help"),
                "{command} lacks its forwarding example: {help}"
            );
            assert!(!help.contains("Composition:"), "{command}: {help}");
        }
    }

    fn os_argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

    /// The `--<first alias>` switch of every provider, derived the way
    /// `argv::provider_for_boolean_flag` derives the accepted shortcuts.
    fn catalog_shortcuts() -> BTreeSet<String> {
        PROVIDERS_DISPLAY_ORDER
            .iter()
            .map(|provider| {
                let alias = provider.cli_aliases().first().unwrap_or_else(|| {
                    panic!("{} has no catalog alias", provider.as_slug())
                });
                format!("--{alias}")
            })
            .collect()
    }

    #[test]
    fn every_catalog_shortcut_is_a_visible_option_of_every_command() {
        let expected = catalog_shortcuts();
        assert_eq!(expected.len(), PROVIDERS_DISPLAY_ORDER.len());
        for command in COMPOSITION_SUBCOMMANDS {
            let child = composition_help_command(command, true);
            let group = child
                .get_groups()
                .find(|group| group.get_id() == "compose_provider")
                .unwrap_or_else(|| panic!("{command} lacks the provider group"));
            let declared: BTreeSet<String> = group
                .get_args()
                .filter(|id| *id != "provider")
                .map(|id| {
                    let arg = child
                        .get_arguments()
                        .find(|arg| arg.get_id() == id)
                        .unwrap_or_else(|| panic!("{command}: group names missing `{id}`"));
                    assert!(!arg.is_hide_set(), "{command}: `{id}` is hidden");
                    assert!(
                        matches!(arg.get_action(), ArgAction::SetTrue),
                        "{command}: `{id}` is not a Boolean switch"
                    );
                    format!("--{}", arg.get_long().unwrap_or_default())
                })
                .collect();
            assert_eq!(declared, expected, "{command}: shortcut switches drifted from the catalog");
            let help = rendered(command);
            for shortcut in &expected {
                assert!(help.contains(&format!("{shortcut} ")), "{command} omits {shortcut}: {help}");
            }
        }
    }

    #[test]
    fn every_visible_option_is_described_and_skipped_fields_are_absent() {
        const SKIPPED: [&str; 4] =
            ["set_records", "caller_arguments", "provider_tail", "provider_tail_notices"];
        for command in COMPOSITION_SUBCOMMANDS {
            let child = composition_help_command(command, true);
            for arg in child.get_arguments().filter(|arg| !arg.is_hide_set()) {
                let help = arg.get_help().map(ToString::to_string).unwrap_or_default();
                assert!(!help.trim().is_empty(), "{command}: `{}` has no help", arg.get_id());
            }
            let ids: Vec<&str> = child.get_arguments().map(|arg| arg.get_id().as_str()).collect();
            let rendered = rendered(command);
            for skipped in SKIPPED {
                assert!(!ids.contains(&skipped), "{command} declares skipped `{skipped}`");
                let spelled = skipped.replace('_', "-");
                assert!(!rendered.contains(&spelled), "{command} renders `{spelled}`: {rendered}");
            }
            for (id, alias) in [
                ("operation", "op"),
                ("append_system_prompt", "asp"),
                ("replace_system_prompt", "rsp"),
            ] {
                let arg = child
                    .get_arguments()
                    .find(|arg| arg.get_id() == id)
                    .unwrap_or_else(|| panic!("{command} lacks `{id}`"));
                assert_eq!(arg.get_visible_aliases(), Some(vec![alias]), "{command} `{id}`");
                // Clap may wrap `[aliases: --op]` across lines; the alias and
                // its closing bracket stay together.
                assert!(rendered.contains(&format!("--{alias}]")), "{command} hides --{alias}: {rendered}");
            }
        }
    }

    #[test]
    fn enumerated_values_stay_canonical() {
        let canonical: Vec<String> =
            PROVIDERS_DISPLAY_ORDER.iter().map(|provider| provider.as_slug().to_string()).collect();
        for command in COMPOSITION_SUBCOMMANDS {
            let child = composition_help_command(command, true);
            let values_of = |id: &str| -> Vec<String> {
                child
                    .get_arguments()
                    .find(|arg| arg.get_id() == id)
                    .unwrap_or_else(|| panic!("{command} lacks `{id}`"))
                    .get_possible_values()
                    .iter()
                    .filter(|value| !value.is_hide_set())
                    .map(|value| value.get_name().to_string())
                    .collect()
            };
            assert_eq!(values_of("provider"), canonical, "{command} --provider values");
            assert_eq!(values_of("exclude"), canonical, "{command} --exclude values");
            assert_eq!(values_of("on_rate_limit"), ["pause", "abort", "continue"], "{command}");
        }
    }

    #[test]
    fn both_help_spellings_select_the_one_compact_renderer() {
        for command in COMPOSITION_SUBCOMMANDS {
            let child = composition_help_command(command, true);
            let help = child
                .get_arguments()
                .find(|arg| arg.get_id() == "help")
                .unwrap_or_else(|| panic!("{command} lacks its help option"));
            assert_eq!(help.get_short(), Some('h'), "{command}");
            assert_eq!(help.get_long(), Some("help"), "{command}");
            assert!(matches!(help.get_action(), ArgAction::Help), "{command}");

            // Both spellings reach the same destination, so the printer's
            // single `render_help` call answers both; it is stable across
            // builds and is the compact screen, never a long manual.
            let short = super::argv::select_composition_help(&os_argv(&["claudine", command, "-h"]), false);
            let long = super::argv::select_composition_help(&os_argv(&["claudine", command, "--help"]), false);
            assert_eq!(short, long, "{command}");
            assert_eq!(rendered(command), rendered(command), "{command}");
            let compact = rendered(command);
            let long_manual = composition_help_command(command, true).render_long_help().to_string();
            assert!(
                long_manual.contains("The commands have real effects"),
                "{command}: the long-help paragraph this check relies on moved: {long_manual}"
            );
            assert!(compact.contains("--dry-run-shell"), "{command}: {compact}");
            assert!(
                !compact.contains("The commands have real effects"),
                "{command} renders the long help paragraph: {compact}"
            );
        }
    }

    #[test]
    fn user_text_carries_no_history_or_internal_paths() {
        for command in COMPOSITION_SUBCOMMANDS {
            let mut child = composition_help_command(command, true);
            for text in [child.render_help().to_string(), child.render_long_help().to_string()] {
                let lower = text.to_lowercase();
                for word in ["deprecated", "migration", "migrate", "previously", "retired", "legacy"] {
                    assert!(!lower.contains(word), "{command} help mentions `{word}`: {text}");
                }
                for fragment in ["::", "crate::", ".rs"] {
                    assert!(!text.contains(fragment), "{command} help shows `{fragment}`: {text}");
                }
            }
        }
    }

    /// Run the ordinary pre-parse pipeline (normalize, partition) and the
    /// shared clap model on `tokens`, with no help token present.
    fn parse_without_help(tokens: &[&str]) -> Result<clap::ArgMatches, String> {
        let normalized = super::argv::normalize(os_argv(tokens));
        let (claudine_argv, _) =
            super::argv::partition_composition_tail(normalized).map_err(|err| format!("partition: {err}"))?;
        build_cli_command(false)
            .try_get_matches_from(claudine_argv)
            .map_err(|err| format!("clap {:?}: {err}", err.kind()))
    }

    #[test]
    fn invalid_invocations_without_help_still_fail_validation() {
        for command in COMPOSITION_SUBCOMMANDS {
            let cases: [(&[&str], ErrorKind); 5] = [
                (&["claudine", command, "x.md", "--provider", "nope"], ErrorKind::InvalidValue),
                (&["claudine", command, "x.md", "--provider"], ErrorKind::InvalidValue),
                (&["claudine", command, "x.md", "--dry-run-shell"], ErrorKind::MissingRequiredArgument),
                (&["claudine", command, "x.md", "-i", "--no-interactive"], ErrorKind::ArgumentConflict),
                (&["claudine", command, "x.md", "--quiet", "--silent"], ErrorKind::ArgumentConflict),
            ];
            for (tokens, kind) in cases {
                let normalized = super::argv::normalize(os_argv(tokens));
                let (claudine_argv, _) = super::argv::partition_composition_tail(normalized)
                    .unwrap_or_else(|err| panic!("{tokens:?} failed partition: {err}"));
                let err = build_cli_command(false)
                    .try_get_matches_from(claudine_argv)
                    .expect_err(&format!("{tokens:?} parsed"));
                assert_eq!(err.kind(), kind, "{tokens:?}: {err}");
            }
            // The controls are valid once the defect is removed.
            assert!(parse_without_help(&["claudine", command, "x.md", "--provider", "claude"]).is_ok());
            assert!(parse_without_help(&["claudine", command, "x.md", "--dry-run", "--dry-run-shell"]).is_ok());

            // `PartitionError` is private to `argv`; its derived `Debug` names
            // the variant and both fields.
            let switch_first = super::argv::partition_composition_tail(os_argv(&["claudine", command, "--bogus", "x.md"]))
                .expect_err(&format!("{command}: a switch before the file was accepted"));
            assert_eq!(
                format!("{switch_first:?}"),
                format!("SwitchBeforeFile {{ subcommand: {command:?}, switch: \"--bogus\" }}"),
            );
        }
    }

    #[test]
    fn malformed_setter_json_still_fails_without_help() {
        let err = super::commands::compose::merge_set_overrides(
            Some("{broken"),
            Default::default(),
            Vec::new(),
        )
        .expect_err("malformed --set JSON was accepted");
        assert!(!err.to_string().is_empty());
        assert!(
            super::commands::compose::merge_set_overrides(Some(r#"{"a":"--help"}"#), Default::default(), Vec::new())
                .is_ok()
        );
    }

    #[test]
    fn ordinary_version_parsing_is_untouched() {
        for tokens in [&["claudine", "--version"][..], &["claudine", "-V"], &["claudine", "-V", "compose", "-h"]] {
            let err = build_cli_command(false)
                .try_get_matches_from(os_argv(tokens))
                .expect_err(&format!("{tokens:?} parsed"));
            assert_eq!(err.kind(), ErrorKind::DisplayVersion, "{tokens:?}");
        }
    }
}
