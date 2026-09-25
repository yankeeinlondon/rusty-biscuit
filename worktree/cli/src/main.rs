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

    let width = cli.width.as_deref();
    let verbose = cli.verbose;
    let perf = cli.perf;
    match cli.command.unwrap_or(Commands::List) {
        Commands::List => commands::list(width, verbose, perf, process_start),
        Commands::Create { branch, from, stay } => {
            commands::create(&branch, from.as_deref(), stay)
        }
        Commands::Go { name, .. } => commands::go(&name),
        Commands::Remove {
            name,
            force,
            branch,
        } => commands::remove(&name, force, branch),
    }
}
