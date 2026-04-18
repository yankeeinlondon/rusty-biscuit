use clap::Args;
use color_eyre::eyre::Result;

/// Arguments for shell completion generation.
#[derive(Args)]
#[command(after_help = r#"EXAMPLES:
    # Bash: Add to ~/.bashrc
    claudine completions bash >> ~/.bashrc

    # Zsh: Add to ~/.zshrc
    claudine completions zsh >> ~/.zshrc

    # Fish: Add to ~/.config/fish/config.fish
    claudine completions fish >> ~/.config/fish/config.fish

    # PowerShell: Add to your profile
    claudine completions powershell >> $PROFILE

    # Elvish: Add to ~/.elvish/rc.elv
    claudine completions elvish >> ~/.elvish/rc.elv

    The emitted line is a one-time bootstrap: it activates Claudine's
    dynamic completer, which re-queries the running binary on every
    <TAB>. You never need to regenerate a script when Claudine ships
    new composition commands or file-reference behavior.
"#)]
pub struct CompletionsArgs {
    /// Shell to generate completions for (bash, zsh, fish, powershell, elvish).
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

/// Emit a one-time bootstrap snippet that activates dynamic completion.
///
/// The snippet wires the user's shell to invoke `COMPLETE=<shell> claudine`
/// on every `<TAB>`. The actual completion logic lives in
/// [`crate::completion::maybe_complete`], which is dispatched from `main()`
/// before any normal CLI startup, so the completion surface tracks the
/// running binary automatically.
pub fn run(args: CompletionsArgs) -> Result<()> {
    use std::io::Write;

    let snippet = super::super::completion::bootstrap::render(args.shell);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(snippet.as_bytes())?;
    Ok(())
}
