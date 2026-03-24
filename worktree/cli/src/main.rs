mod args;
mod commands;

use args::{Cli, Commands};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable as _;
use biscuit_terminal::terminal::Terminal;
use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;

fn main() {
    CompleteEnv::with_factory(Cli::command).complete();

    if let Err(e) = run() {
        let msg = format!("<red><b>Error:</b></red> {e}");
        let terminal = Terminal::default();
        eprintln!("{}", Prose::new(msg).render(&terminal));
        std::process::exit(1);
    }
}

fn run() -> Result<(), worktree::WorktreeError> {
    let cli = Cli::parse();

    // Handle --completions
    if let Some(shell) = cli.completions {
        print_completions(shell);
        return Ok(());
    }

    match cli.command.unwrap_or(Commands::List) {
        Commands::List => commands::list(),
        Commands::Create { branch, stay } => commands::create(&branch, stay),
        Commands::Go { name, .. } => commands::go(&name),
    }
}

/// Prints shell completions setup instructions.
fn print_completions(shell: clap_complete::Shell) {
    use clap_complete::Shell;

    let (setup_cmd, config_file) = match shell {
        Shell::Bash => ("source <(COMPLETE=bash wt)", "~/.bashrc"),
        Shell::Zsh => ("source <(COMPLETE=zsh wt)", "~/.zshrc"),
        Shell::Fish => ("COMPLETE=fish wt | source", "~/.config/fish/config.fish"),
        Shell::PowerShell => (
            r#"$env:COMPLETE = "powershell"; wt | Out-String | Invoke-Expression; Remove-Item Env:\COMPLETE"#,
            "$PROFILE",
        ),
        Shell::Elvish => ("eval (E:COMPLETE=elvish wt | slurp)", "~/.elvish/rc.elv"),
        _ => {
            eprintln!("Shell {:?} is not supported for dynamic completions", shell);
            return;
        }
    };

    println!("# Add this line to {}:", config_file);
    println!("{}", setup_cmd);
}
