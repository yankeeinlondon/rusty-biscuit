mod args;
mod commands;
mod env;
mod exit;
mod perf;
mod shell_integration;

use args::{Cli, Commands};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable as _;
use biscuit_terminal::terminal::Terminal;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;

fn main() {
    let process_start = std::time::Instant::now();

    CompleteEnv::with_factory(Cli::command).complete();

    if let Err(e) = run(process_start) {
        let terminal = Terminal::default();
        let msg = match &e {
            worktree::WorktreeError::Cancelled => "<dim>Cancelled.</dim>".to_string(),
            // These carry their own Prose markup and heading.
            worktree::WorktreeError::RefusedToLoseWork(markup)
            | worktree::WorktreeError::BlockedByEnvironment(markup) => markup.clone(),
            _ => format!(
                "<red><b>Error:</b></red> {}",
                Prose::escape_text(&e.to_string())
            ),
        };
        eprintln!("{}", Prose::new(msg).render(&terminal));
        std::process::exit(exit::exit_code(&e));
    }
}

fn run(process_start: std::time::Instant) -> Result<(), worktree::WorktreeError> {
    let cli = Cli::parse();

    // Handle --completions
    if let Some(shell) = cli.completions {
        shell_integration::print(shell);
        return Ok(());
    }

    reject_listing_flags(&cli);

    let width = cli.width.as_deref();
    let verbose = cli.verbose;
    let perf = cli.perf;
    let flags = commands::ListFlags {
        refresh: cli.refresh,
        ignore_api: cli.ignore_api,
        fast_forward: cli.fast_forward,
    };
    match cli.command.unwrap_or(Commands::List) {
        Commands::List => commands::list(width, verbose, perf, flags, process_start),
        Commands::Create { branch, from, stay } => {
            commands::create(&branch, from.as_deref(), stay)
        }
        Commands::Go { name, .. } => commands::go(&name),
        Commands::InternalRefresh { repo, attempt, force } => {
            commands::refresh_worker::run(&repo, attempt.as_deref(), force);
            Ok(())
        }
        Commands::Remove {
            name,
            force_worktree,
            force_branch,
            force_remote,
            handoff,
        } => match handoff {
            Some(token) => commands::remove_handoff(&token),
            None => commands::remove(
                name.as_deref().unwrap_or_default(),
                commands::RemoveFlags {
                    force_worktree,
                    force_branch,
                    force_remote,
                },
            ),
        },
    }
}

/// Exits 2, as clap does, when a listing-only flag is given to another
/// command. clap cannot express this itself: a global argument cannot
/// conflict with a subcommand.
fn reject_listing_flags(cli: &Cli) {
    let given: Vec<&str> = [
        (cli.refresh, "--refresh"),
        (cli.ignore_api, "--ignore-api"),
        (cli.fast_forward, "--fast-forward"),
    ]
    .into_iter()
    .filter_map(|(set, flag)| set.then_some(flag))
    .collect();
    let command = match &cli.command {
        None | Some(Commands::List) => return,
        Some(Commands::Create { .. }) => "create",
        Some(Commands::Go { .. }) => "go",
        Some(Commands::Remove { .. }) => "remove",
        Some(Commands::InternalRefresh { .. }) => commands::refresh_worker::SUBCOMMAND,
    };
    if let Some(flag) = given.first() {
        Cli::command()
            .error(
                clap::error::ErrorKind::ArgumentConflict,
                format!("{flag} applies only to listing (`wt` or `wt list`), not to `wt {command}`"),
            )
            .exit();
    }
}
