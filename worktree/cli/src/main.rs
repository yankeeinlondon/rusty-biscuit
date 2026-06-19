mod args;
mod commands;
mod perf;

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
        let msg = format!("<red><b>Error:</b></red> {e}");
        let terminal = Terminal::default();
        eprintln!("{}", Prose::new(msg).render(&terminal));
        std::process::exit(1);
    }
}

fn run(process_start: std::time::Instant) -> Result<(), worktree::WorktreeError> {
    let cli = Cli::parse();

    // Handle --completions
    if let Some(shell) = cli.completions {
        print_completions(shell);
        return Ok(());
    }

    let width = cli.width.as_deref();
    let verbose = cli.verbose;
    let perf = cli.perf;
    match cli.command.unwrap_or(Commands::List) {
        Commands::List => commands::list(width, verbose, perf, process_start),
        Commands::Create { branch, stay } => commands::create(&branch, stay),
        Commands::Go { name, .. } => commands::go(&name),
        Commands::Remove {
            name,
            force,
            branch,
        } => commands::remove(&name, force, branch),
    }
}

/// Prints a sourceable shell integration script (cd wrapper + completions).
///
/// The output is designed to be sourced directly, e.g.:
///   source <(wt --completions zsh)
fn print_completions(shell: clap_complete::Shell) {
    use clap_complete::Shell;

    match shell {
        Shell::Bash => println!(
            r#"# wt shell integration — add to ~/.bashrc:
#   source <(wt --completions bash)
wt() {{
    local output exit_code
    output="$(command wt "$@")"
    exit_code=$?
    (( exit_code != 0 )) && return $exit_code
    if [[ "$output" == cd:* ]]; then
        builtin cd -- "${{output#cd:}}"
    elif [[ -n "$output" ]]; then
        echo "$output"
    fi
    return 0
}}
source <(COMPLETE=bash command wt)"#
        ),
        Shell::Zsh => println!(
            r#"# wt shell integration — add to ~/.zshrc:
#   source <(wt --completions zsh)
wt() {{
    local output exit_code
    output="$(command wt "$@")"
    exit_code=$?
    (( exit_code != 0 )) && return $exit_code
    if [[ "$output" == cd:* ]]; then
        builtin cd -- "${{output#cd:}}"
    elif [[ -n "$output" ]]; then
        print -- "$output"
    fi
    return 0
}}
source <(COMPLETE=zsh command wt)"#
        ),
        Shell::Fish => println!(
            r#"# wt shell integration — add to ~/.config/fish/config.fish:
#   source (wt --completions fish | psub)
function wt
    set output (command wt $argv)
    set exit_code $status
    if test $exit_code -ne 0
        return $exit_code
    end
    if string match -q 'cd:*' -- $output
        builtin cd (string replace 'cd:' '' -- $output)
    else if test -n "$output"
        echo $output
    end
    return 0
end
COMPLETE=fish command wt | source"#
        ),
        _ => {
            eprintln!("Shell {:?} is not supported", shell);
        }
    }
}
