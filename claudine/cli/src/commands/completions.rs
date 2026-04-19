use clap::Args;
use color_eyre::eyre::Result;

/// Arguments for shell completion generation.
#[derive(Args)]
#[command(after_help = r#"EXAMPLES:
    # Bash: Redirect into your bash-completion directory
    claudine completions bash > ~/.local/share/bash-completion/completions/claudine

    # Zsh: Redirect into the first directory in $fpath
    claudine completions zsh > "${fpath[1]}/_claudine"

    # Fish: Redirect into the fish completion directory
    claudine completions fish > ~/.config/fish/completions/claudine.fish

    # PowerShell (legacy bootstrap): Add to your profile
    claudine completions powershell >> $PROFILE

    # Elvish (legacy bootstrap): Add to ~/.elvish/rc.elv
    claudine completions elvish >> ~/.elvish/rc.elv

    For bash, zsh, and fish, the emitted script registers a completion
    callback that shells out to `claudine __complete` on every <TAB>.
    The hidden engine tracks the running binary, so regenerating the
    script is only required after a Claudine upgrade that changes the
    callback contract — the completion output itself always reflects
    the currently installed binary.

    PowerShell and Elvish retain the legacy one-line `COMPLETE=<shell>`
    bootstrap because the supplement acceptance matrix covers
    bash/zsh/fish only. Stale installed scripts on any shell continue
    to reach the legacy completion path until regenerated.
"#)]
pub struct CompletionsArgs {
    /// Shell to generate completions for (bash, zsh, fish, powershell, elvish).
    #[arg(value_enum)]
    pub shell: clap_complete::Shell,
}

/// Emit the shell completion script for the requested shell.
///
/// For `bash`, `zsh`, and `fish` this is a full completion script that
/// registers a callback shelling out to `claudine __complete` at every
/// `<TAB>`. The callback runs the supplement engine in
/// [`crate::completion::supplement`] against the full argv the shell
/// passes in.
///
/// For `powershell` and `elvish` this remains a one-line `COMPLETE=<shell>`
/// bootstrap that activates the legacy `CompleteEnv` runtime path in
/// [`crate::completion::maybe_complete`]. Those two shells are outside the
/// supplement spec's bash/zsh/fish acceptance matrix.
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
