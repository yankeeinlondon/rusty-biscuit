mod args;
mod commands;
mod env;
mod exit;
mod perf;
mod shell_integration;

use args::{Cli, Commands};
use biscuit_terminal::components::prose::{LineBreaks, Prose};
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
            // These carry their own Prose markup and heading, and are printed
            // after one blank line.
            worktree::WorktreeError::RefusedToLoseWork(markup)
            | worktree::WorktreeError::BlockedByEnvironment(markup) => {
                eprintln!();
                markup.clone()
            }
            _ => error_markup(&e.to_string()),
        };
        let rendered = Prose::new(msg).with_line_breaks(LineBreaks::Hard).render(&terminal);
        eprintln!("{rendered}");
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
        Commands::InternalRefresh { repo, attempt, timings } => {
            commands::refresh_worker::run(&repo, attempt.as_deref(), timings);
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

/// The markup for a plain error message.
///
/// Backtick spans in `message` stay code spans, so their contents are passed
/// through unescaped: a code span shows backslashes literally.
fn error_markup(message: &str) -> String {
    format!("<red><b>Error:</b></red> {}", Prose::escape_text_outside_code_spans(message))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_markup_leaves_code_spans_unescaped_and_escapes_the_rest() {
        let message = "HEAD is detached, so there is no current branch to fork `fix/f_x` from. \
            Pass `--from <branch>` to choose the branch it starts from. a_b <c>";

        let markup = error_markup(message);

        assert!(markup.contains("`fix/f_x`"), "{markup}");
        assert!(markup.contains("`--from <branch>`"), "{markup}");
        assert!(markup.ends_with(r"a\_b \<c\>"), "{markup}");
    }

    #[test]
    fn error_markup_renders_code_span_contents_without_backslashes() {
        let terminal = Terminal::new_optimistic(200);
        let message = "`feat_x` already exists, so `--from <base>` would be ignored.";

        let rendered = Prose::new(error_markup(message)).render(&terminal);
        let plain = biscuit_terminal::utils::escape_codes::strip_escape_codes(rendered);

        assert_eq!(plain, "Error: feat_x already exists, so --from <base> would be ignored.");
    }
}
