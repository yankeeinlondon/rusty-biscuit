//! Bootstrap snippet rendering for `claudine completions <shell>`.
//!
//! Phase 4 of the `2026-04-17-file-completion` feature replaces the
//! previous static `clap_complete::generate(...)` output with one-liner
//! bootstrap snippets that activate dynamic completion at shell startup.
//! The snippets rely on clap_complete's `CompleteEnv` runtime path, which
//! is wired up in [`crate::completion::maybe_complete`] and fires
//! whenever `COMPLETE` is set in the environment.
//!
//! ## Snippet shape per shell
//!
//! | Shell        | Emitted snippet                                       |
//! | ------------ | ----------------------------------------------------- |
//! | `bash`       | `source <(COMPLETE=bash claudine)`                    |
//! | `zsh`        | `source <(COMPLETE=zsh claudine)`                     |
//! | `fish`       | `COMPLETE=fish claudine \| source`                    |
//! | `powershell` | `& { $env:COMPLETE="powershell"; claudine } \| Out-String \| Invoke-Expression` |
//! | `elvish`     | `eval (E:COMPLETE=elvish claudine \| slurp)`          |
//!
//! Users add the one-liner to their shell rc file once; Claudine owns the
//! actual completion output via the `COMPLETE` hook in `main()` and never
//! needs the user to regenerate a script when the binary changes.
//!
//! ## PowerShell note
//!
//! PowerShell does not support an inline `VAR=value` prefix for child
//! process invocation the way POSIX shells do. The snippet scopes the env
//! var to a script block (`& { ... }`) so `COMPLETE` only exists for the
//! duration of the `claudine` invocation, then pipes the output through
//! `Invoke-Expression` to register the handler in the user's session.

use clap_complete::Shell;

/// Render the bootstrap snippet for `shell`.
///
/// Returned string includes a trailing newline so it prints cleanly as a
/// CLI subcommand output. The output deliberately does not include any
/// framing prose (no "# paste this in ~/.bashrc" comment) — the command's
/// `--help` / `EXAMPLES` section already carries the human-facing guidance,
/// and keeping the stdout shape pure makes it trivial to pipe the output
/// directly into a shell rc file.
pub(crate) fn render(shell: Shell) -> String {
    match shell {
        Shell::Bash => "source <(COMPLETE=bash claudine)\n".to_string(),
        Shell::Zsh => "source <(COMPLETE=zsh claudine)\n".to_string(),
        Shell::Fish => "COMPLETE=fish claudine | source\n".to_string(),
        Shell::PowerShell => {
            "& { $env:COMPLETE=\"powershell\"; claudine } | Out-String | Invoke-Expression\n"
                .to_string()
        }
        Shell::Elvish => "eval (E:COMPLETE=elvish claudine | slurp)\n".to_string(),
        // `clap_complete::Shell` is `#[non_exhaustive]`; any future shell
        // that clap adds falls back to the bash-style snippet. That is the
        // closest documented shape for POSIX-like shells.
        _ => "source <(COMPLETE=bash claudine)\n".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_snippet_is_a_source_completion_one_liner() {
        assert_eq!(render(Shell::Bash), "source <(COMPLETE=bash claudine)\n");
    }

    #[test]
    fn zsh_snippet_is_a_source_completion_one_liner() {
        assert_eq!(render(Shell::Zsh), "source <(COMPLETE=zsh claudine)\n");
    }

    #[test]
    fn fish_snippet_pipes_into_source() {
        assert_eq!(render(Shell::Fish), "COMPLETE=fish claudine | source\n");
    }

    #[test]
    fn elvish_snippet_uses_eval_slurp() {
        assert_eq!(
            render(Shell::Elvish),
            "eval (E:COMPLETE=elvish claudine | slurp)\n",
        );
    }

    #[test]
    fn powershell_snippet_directs_user_through_invoke_expression() {
        let out = render(Shell::PowerShell);
        assert!(out.contains("Invoke-Expression"));
        assert!(out.contains("claudine"));
        assert!(out.contains("$env:COMPLETE"));
        assert!(out.ends_with('\n'));
    }

    #[test]
    fn every_snippet_ends_with_a_newline() {
        for shell in [
            Shell::Bash,
            Shell::Zsh,
            Shell::Fish,
            Shell::PowerShell,
            Shell::Elvish,
        ] {
            assert!(
                render(shell).ends_with('\n'),
                "snippet for {shell:?} missing trailing newline",
            );
        }
    }
}
