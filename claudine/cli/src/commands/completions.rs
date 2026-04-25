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
    bootstrap. Those shells fall back to whatever `clap_complete`
    derives from the command tree — subcommand names and flag names —
    with no composition-specific completion behavior.
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
/// `<TAB>`. The callback routes through [`crate::completion::engine::run`]
/// against the full argv the shell passes in.
///
/// For `powershell` and `elvish` this remains a one-line `COMPLETE=<shell>`
/// bootstrap that activates the legacy `CompleteEnv` runtime path in
/// [`crate::completion::maybe_complete`]. Those two shells defer to
/// clap-derived completion only (subcommand and flag names).
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
/// Invoked by generated bash/zsh/fish completion scripts on every `<TAB>`.
/// The engine classifies which slot the cursor is in (root menu,
/// composition positional, `@`-gated setter value, or "other") and emits
/// one candidate per line on stdout. Any slot the engine does not
/// recognize produces zero candidates so the shell's native file / flag
/// completion takes over. The CLI surface is intentionally terse and must
/// stay stable so generated scripts keep working:
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

/// Run the dynamic completion engine.
///
/// Emits one candidate per line on stdout. Never returns a non-zero exit
/// status for "no candidates" — completion scripts treat an empty stdout as
/// "fall back to static clap completion". The only error paths are I/O
/// failures while writing to stdout.
///
/// Routing flows through [`crate::completion::engine::run`], which
/// classifies the cursor slot and dispatches to a slot-specific completer
/// (root menu today; composition-positional and setter-value completers in
/// subsequent phases).
pub fn run_complete(args: CompleteArgs) -> Result<()> {
    use std::io::Write;

    let candidates = crate::completion::engine::run(&args.argv, args.current);
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    for candidate in candidates {
        writeln!(handle, "{candidate}")?;
    }
    Ok(())
}
