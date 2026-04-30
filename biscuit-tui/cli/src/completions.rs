//! Shell-completion generation with a small overlay that surfaces the
//! `[CTRL+`, `[ALT+`, and `[OPT+` hotkey-prefix candidates whenever the
//! user starts typing `[` in an option position.
//!
//! Spec reference: `2026-04-28-choose-one-improvements/spec.md` →
//! "Completions" section.
//!
//! The standard `clap_complete` script is emitted first, then a shell-
//! specific suffix wraps the generated `_question` completion function
//! to add the hotkey-prefix candidates after the standard ones. Shells
//! we don't have an overlay for (fish, powershell, elvish) get the
//! plain `clap_complete` script.

use std::io::{self, Write};

use clap::CommandFactory;
use clap_complete::Shell;

/// Writes a shell completion script to `writer`, augmented with the
/// hotkey-prefix overlay for supported shells (`bash`, `zsh`).
pub fn write_completions<C: CommandFactory, W: Write>(
    shell: Shell,
    bin: &str,
    writer: &mut W,
) -> io::Result<()> {
    let mut cmd = C::command();
    clap_complete::generate(shell, &mut cmd, bin, writer);
    if let Some(snippet) = hotkey_prefix_overlay(shell, bin) {
        writer.write_all(snippet.as_bytes())?;
    }
    Ok(())
}

/// Returns a shell-specific snippet that adds the `[CTRL+`, `[ALT+`,
/// `[OPT+` candidates when the current word starts with `[`.
///
/// Returns `None` for shells that don't yet have an overlay; those
/// shells still receive the standard `clap_complete` script.
fn hotkey_prefix_overlay(shell: Shell, bin: &str) -> Option<String> {
    match shell {
        Shell::Bash => Some(bash_overlay(bin)),
        Shell::Zsh => Some(zsh_overlay(bin)),
        _ => None,
    }
}

fn bash_overlay(bin: &str) -> String {
    format!(
        r#"
# Hotkey-prefix overlay (added by `{bin} completions bash`).
# When the current word begins with `[`, append `[CTRL+`, `[ALT+`,
# `[OPT+` to the candidates produced by the standard `_{bin}` function.
_{bin}_hotkey_overlay() {{
    _{bin} "$@"
    local cur="${{COMP_WORDS[COMP_CWORD]}}"
    if [[ "$cur" == \[* ]]; then
        local extra
        extra=$(compgen -W "[CTRL+ [ALT+ [OPT+" -- "$cur")
        if [[ -n "$extra" ]]; then
            COMPREPLY+=( $extra )
        fi
    fi
}}
if [[ "${{BASH_VERSINFO[0]}}" -eq 4 && "${{BASH_VERSINFO[1]}}" -ge 4 || "${{BASH_VERSINFO[0]}}" -gt 4 ]]; then
    complete -F _{bin}_hotkey_overlay -o nosort -o bashdefault -o default {bin}
else
    complete -F _{bin}_hotkey_overlay -o bashdefault -o default {bin}
fi
"#
    )
}

fn zsh_overlay(bin: &str) -> String {
    format!(
        r#"
# Hotkey-prefix overlay (added by `{bin} completions zsh`).
# When the current word prefix begins with `[`, also offer `[CTRL+`,
# `[ALT+`, `[OPT+` alongside the candidates produced by `_{bin}`.
_{bin}_hotkey_overlay() {{
    _{bin} "$@"
    if [[ "$PREFIX" == \[* ]]; then
        compadd -- '[CTRL+' '[ALT+' '[OPT+'
    fi
}}
compdef _{bin}_hotkey_overlay {bin}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    /// Minimal command surface used to exercise `write_completions`
    /// without depending on the full `Cli` struct from `main.rs`.
    #[derive(Parser)]
    #[command(name = "question")]
    struct TestCli {
        #[arg(value_name = "OPTIONS")]
        positional: Vec<String>,
    }

    #[test]
    fn bash_output_contains_hotkey_overlay() {
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Bash, "question", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.contains("_question_hotkey_overlay"),
            "bash overlay function should be defined",
        );
        assert!(
            text.contains("[CTRL+ [ALT+ [OPT+"),
            "bash overlay should advertise the hotkey prefixes",
        );
        assert!(
            text.contains("complete -F _question_hotkey_overlay"),
            "bash overlay must rebind `complete` to the wrapper",
        );
    }

    #[test]
    fn zsh_output_contains_hotkey_overlay() {
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Zsh, "question", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(
            text.contains("_question_hotkey_overlay"),
            "zsh overlay function should be defined",
        );
        assert!(
            text.contains("compadd -- '[CTRL+' '[ALT+' '[OPT+'"),
            "zsh overlay should compadd the hotkey prefixes",
        );
        assert!(
            text.contains("compdef _question_hotkey_overlay question"),
            "zsh overlay must compdef the wrapper",
        );
    }

    #[test]
    fn fish_output_skips_overlay_but_still_completes() {
        let mut buf = Vec::new();
        write_completions::<TestCli, _>(Shell::Fish, "question", &mut buf).unwrap();
        let text = String::from_utf8(buf).unwrap();
        assert!(!text.contains("_hotkey_overlay"), "fish has no overlay yet");
        assert!(
            text.contains("complete -c question"),
            "fish should still emit the standard clap_complete script",
        );
    }
}
