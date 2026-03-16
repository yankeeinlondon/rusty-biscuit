use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable as _;
use biscuit_terminal::terminal::Terminal;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use color_eyre::eyre::Result;
use darkmatter::markdown::highlighting::{ColorMode, ThemePair};
use darkmatter_cli::Cli;
use darkmatter_cli::commands::{run_clean, run_read, run_subcommand, validate_subcommand_usage};
use std::io::{self, IsTerminal};
use tracing_subscriber::{filter::EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Initialize tracing subscriber based on verbosity level.
///
/// Verbosity levels:
/// - 0 (default): WARN only (errors and warnings)
/// - 1 (-v): INFO (tool calls, phase transitions)
/// - 2 (-vv): DEBUG (tool arguments, API requests)
/// - 3 (-vvv): TRACE (request/response bodies)
/// - 4+ (-vvvv): TRACE with file/line numbers
fn init_tracing(verbose: u8) {
    // Only initialize if verbose mode is enabled
    if verbose == 0 {
        return;
    }

    let base_filter = match std::env::var("RUST_LOG") {
        Ok(filter) => filter,
        Err(_) => match verbose {
            // -v: Show INFO for progress and tool calls
            1 => "info,md=info,darkmatter=info".to_string(),
            // -vv: Show DEBUG for tool arguments and requests
            2 => "info,md=debug,darkmatter=debug".to_string(),
            // -vvv+: Show TRACE for detailed debugging
            _ => "debug,md=trace,darkmatter=trace".to_string(),
        },
    };

    let filter = EnvFilter::try_new(&base_filter).unwrap_or_else(|_| EnvFilter::new("warn"));

    tracing_subscriber::registry()
        .with(filter)
        .with(
            fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_thread_ids(false)
                .with_file(verbose >= 4)
                .with_line_number(verbose >= 4)
                .with_writer(std::io::stderr)
                .compact(),
        )
        .init();
}

fn main() {
    CompleteEnv::with_factory(Cli::command).complete();

    if let Err(e) = run() {
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
    init_tracing(cli.verbose);

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
        run_clean(cli.input.as_ref(), true, None, cli.verbose > 0)?;
        return Ok(());
    }

    // No subcommand given — if no input and interactive terminal, show help
    if cli.input.is_none() && io::stdin().is_terminal() {
        Cli::command().print_help()?;
        return Ok(());
    }

    // Run as implicit read using top-level args
    run_read(cli.input.as_ref(), cli.output, cli.show, &cli)?;

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
