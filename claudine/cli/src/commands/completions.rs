use clap::Args;
use color_eyre::eyre::Result;

/// Arguments for shell completion generation.
#[derive(Args)]
#[command(after_help = r#"EXAMPLES:
    # Bash: Add to ~/.bashrc
    source <(claudine completions bash)

    # Zsh: Add to ~/.zshrc
    source <(claudine completions zsh)

    # Fish: Add to ~/.config/fish/config.fish
    claudine completions fish | source

    # PowerShell: Add to your profile
    claudine completions powershell | Out-String | Invoke-Expression

    # Elvish: Add to ~/.elvish/rc.elv
    eval (claudine completions elvish | slurp)
"#)]
pub struct CompletionsArgs {
    /// Shell to generate completions for (bash, zsh, fish, powershell, elvish).
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

/// Generate shell completions.
pub fn run(args: CompletionsArgs) -> Result<()> {
    use clap::CommandFactory;
    let mut cmd = super::super::Cli::command();
    clap_complete::generate(args.shell, &mut cmd, "claudine", &mut std::io::stdout());
    Ok(())
}
