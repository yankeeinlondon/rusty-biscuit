use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::errors::as_block_error as as_terminal_block_error;
use biscuit_terminal::terminal::Terminal;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use color_eyre::eyre::Result;
use darkmatter::markdown::errors::as_block_error as as_darkmatter_block_error;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter_cli::Cli;
use darkmatter_cli::commands::{
    CleanOptions, run_clean, run_render, run_subcommand, validate_subcommand_usage,
};
use std::io::{self, IsTerminal};
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing subscriber for developer debug output.
///
/// Triggered by `--debug <level>` or `RUST_LOG` env var. The `--verbose` flag
/// is NOT involved here — it controls styled user-facing output only.
///
/// ## Debug levels
///
/// - 1: INFO (phase transitions, high-level operations)
/// - 2: DEBUG (decisions, resolved values, cache hits/misses)
/// - 3: TRACE (per-item details, raw values)
/// - 4+: TRACE with file/line source locations
fn init_tracing(debug_level: Option<u8>) {
    // RUST_LOG takes precedence if set
    let env_log = std::env::var("RUST_LOG").ok();

    let filter_str = match (&env_log, debug_level) {
        // RUST_LOG is set — use it directly, ignore --debug flag
        (Some(rust_log), _) => rust_log.clone(),
        // --debug flag provided
        (None, Some(1)) => "warn,md=info,darkmatter=info".to_string(),
        (None, Some(2)) => "warn,md=debug,darkmatter=debug".to_string(),
        (None, Some(n)) if n >= 3 => "info,md=trace,darkmatter=trace".to_string(),
        // No debug output requested
        _ => return,
    };

    let filter = EnvFilter::try_new(&filter_str).unwrap_or_else(|_| EnvFilter::new("warn"));
    let show_locations = debug_level.unwrap_or(0) >= 4;

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(show_locations)
                .with_line_number(show_locations)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}

fn main() {
    // Reset SIGPIPE to default (terminate) so piping to `head`, `md toc`, etc.
    // exits cleanly instead of panicking on write to a closed pipe.
    reset_sigpipe();

    CompleteEnv::with_factory(Cli::command).complete();

    if let Err(e) = run() {
        // Prefer `BlockError` rendering when any error in the chain implements it.
        // Walk the cause chain so wrapper errors (eyre Report, Box<dyn Error>) don't
        // mask an inner BlockError implementation. Darkmatter's downcast registry
        // takes precedence; the terminal-local stub is checked only as a fallback
        // for non-darkmatter error types.
        // NOTE: biscuit_terminal::errors::as_block_error returns None unconditionally
        // (it is a documented stub); the call here is intentional to leave room for
        // future extension without adding a hard dependency.
        let block = e.chain().find_map(|cause| {
            as_darkmatter_block_error(cause).or_else(|| as_terminal_block_error(cause))
        });

        if let Some(block) = block {
            // TTY => full terminal detection; non-TTY => optimistic 80-column render.
            let rendered = if io::stderr().is_terminal() {
                block.report_block_error(&Terminal::new())
            } else {
                block.report_block_error_optimistic(None)
            };
            eprintln!("{rendered}");
            std::process::exit(1);
        }

        // Fallback: legacy Display-chain renderer.
        // Deduplicate chain: skip causes whose message is already contained in a prior message.
        let top = e.to_string();
        let mut seen = top.clone();
        let causes: Vec<String> = e
            .chain()
            .skip(1)
            .filter_map(|c| {
                let msg = c.to_string();
                if seen.contains(&msg) {
                    None
                } else {
                    seen.push_str(&msg);
                    Some(msg)
                }
            })
            .collect();
        let msg = if causes.is_empty() {
            format!("<red><b>Error:</b></red> {top}")
        } else {
            format!(
                "<red><b>Error:</b></red> {top}\n       {}",
                causes
                    .iter()
                    .map(|c| format!("<dim>▸</dim> {c}"))
                    .collect::<Vec<_>>()
                    .join("\n       ")
            )
        };
        let terminal = Terminal::default();
        eprintln!("{}", Prose::new(msg).render(&terminal));
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    color_eyre::install()?;

    let cli = Cli::parse();
    init_tracing(cli.debug_level);

    // Handle --completions first (no input needed)
    if let Some(shell) = cli.completions {
        print_completions(shell);
        return Ok(());
    }

    // Handle --list-themes (no input needed)
    if cli.list_themes {
        list_themes();
        return Ok(());
    }

    if let Some(command) = cli.command.clone() {
        validate_subcommand_usage(&cli)?;
        run_subcommand(command, &cli)?;
        return Ok(());
    }

    if cli.save {
        let options = CleanOptions {
            save: true,
            verbose: cli.verbose > 0,
            ..CleanOptions::default()
        };
        run_clean(cli.input.as_ref(), &options)?;
        return Ok(());
    }

    // No subcommand given — if no input and interactive terminal, show help
    if cli.input.is_none() && io::stdin().is_terminal() {
        Cli::command().print_help()?;
        return Ok(());
    }

    // Run as implicit render using top-level args
    run_render(cli.input.as_ref(), cli.output, cli.show, None, &cli)?;

    Ok(())
}

/// Lists all available themes with descriptions.
fn list_themes() {
    println!("Available themes:\n");
    for theme_pair in ThemePair::all() {
        println!(
            "  {:20} {}",
            theme_pair.kebab_name(),
            theme_pair.description(ColorMode::Dark)
        );
    }
    println!("\nUse --theme <name> to set prose theme");
    println!("Use --code-theme <name> to override code theme");
}

/// Prints shell completions setup instructions.
///
/// With dynamic completions, the shell sources a command that calls back to the CLI.
/// This outputs the appropriate setup command for each shell.
fn print_completions(shell: clap_complete::Shell) {
    use clap_complete::Shell;

    let (setup_cmd, config_file) = match shell {
        Shell::Bash => (r#"source <(COMPLETE=bash md)"#, "~/.bashrc"),
        Shell::Zsh => (r#"source <(COMPLETE=zsh md)"#, "~/.zshrc"),
        Shell::Fish => (r#"COMPLETE=fish md | source"#, "~/.config/fish/config.fish"),
        Shell::PowerShell => (
            r#"$env:COMPLETE = "powershell"; md | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE"#,
            "$PROFILE",
        ),
        Shell::Elvish => (r#"eval (E:COMPLETE=elvish md | slurp)"#, "~/.elvish/rc.elv"),
        _ => {
            eprintln!("Shell {:?} is not supported for dynamic completions", shell);
            return;
        }
    };

    println!("# Add this line to {}:", config_file);
    println!("{}", setup_cmd);
}

/// Resets SIGPIPE to the default handler so writes to closed pipes
/// terminate the process instead of panicking.
///
/// Rust ignores SIGPIPE by default, which causes `println!` to panic
/// with "broken pipe" when output is piped to a process that exits
/// early (e.g., `md compose test.md | md toc`).
#[cfg(unix)]
fn reset_sigpipe() {
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
}

#[cfg(not(unix))]
fn reset_sigpipe() {}
