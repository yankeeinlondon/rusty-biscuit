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

/// Arguments for the hidden `__complete` subcommand.
///
/// Invoked by generated bash/zsh/fish completion scripts at the argument
/// positions listed in the supplement spec (the positional file slot on
/// `compose` / `inline-compose` / `sequence`, plus `--append-system-prompt` /
/// `--replace-system-prompt` values on those three and every wrapper
/// subcommand). The CLI surface is intentionally terse and must stay stable
/// so generated scripts keep working:
///
/// - `--current <INDEX>` identifies the argv element being completed.
/// - Trailing positional args after `--` are the full original argv the user
///   typed (including the binary name at position 0), so the engine can run
///   its own lightweight classifier on them without re-parsing through clap.
#[derive(Args)]
pub struct CompleteArgs {
    /// 0-based index in `argv` of the token being completed.
    #[arg(long = "current", value_name = "INDEX")]
    pub current: usize,

    /// The full argv to classify and complete against. Pass everything the
    /// user typed — binary name at position 0, subcommand at position 1, etc.
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub argv: Vec<String>,
}

/// Run the supplement completion engine.
///
/// Emits one candidate per line on stdout. Never returns a non-zero exit
/// status for "no candidates" — completion scripts treat an empty stdout as
/// "fall back to static clap completion". The only error paths are I/O
/// failures while writing to stdout.
pub fn run_complete(args: CompleteArgs) -> Result<()> {
    use std::io::Write;

    let candidates = crate::completion::supplement::run(&args.argv, args.current);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for candidate in candidates {
        writeln!(handle, "{candidate}")?;
    }
    Ok(())
}
