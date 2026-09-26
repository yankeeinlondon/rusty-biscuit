//! How `wt` reads its caller: whether a person can answer prompts, and whether
//! a shell wrapper will act on the `cd:` protocol lines printed to stdout.

use std::ffi::OsStr;
use std::io::IsTerminal as _;

/// The variable every generated shell wrapper sets for the one `wt`
/// invocation it makes.
pub const SHELL_WRAPPER_VAR: &str = "WT_SHELL_WRAPPER";

/// Whether prompts can be shown and answered.
///
/// Stdout is deliberately not consulted: the shell wrapper always captures it,
/// and `inquire` draws its prompts on stderr.
pub fn is_interactive() -> bool {
    interactive_from(
        std::io::stdin().is_terminal(),
        std::io::stderr().is_terminal(),
        std::env::var_os("CI").as_deref(),
    )
}

/// Whether a shell wrapper will act on `cd:` and `remove-handoff:` lines.
///
/// Only the wrapper's explicit announcement counts. A captured stdout is not
/// proof: scripts, `just` recipes, and agents capture it too but never `cd`.
pub fn shell_wrapper_active() -> bool {
    wrapper_active_from(std::env::var_os(SHELL_WRAPPER_VAR).as_deref())
}

/// `CI` counts as set when it is present and not empty.
fn interactive_from(stdin_tty: bool, stderr_tty: bool, ci: Option<&OsStr>) -> bool {
    let ci_set = ci.is_some_and(|value| !value.is_empty());
    stdin_tty && stderr_tty && !ci_set
}

fn wrapper_active_from(value: Option<&OsStr>) -> bool {
    value == Some(OsStr::new("1"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interactive_requires_both_terminals_and_no_ci() {
        let cases: &[(bool, bool, Option<&str>, bool)] = &[
            (true, true, None, true),
            (true, true, Some(""), true),
            (true, true, Some("true"), false),
            (true, true, Some("1"), false),
            (true, true, Some("false"), false),
            (false, true, None, false),
            (true, false, None, false),
            (false, false, None, false),
        ];
        for &(stdin_tty, stderr_tty, ci, expected) in cases {
            assert_eq!(
                interactive_from(stdin_tty, stderr_tty, ci.map(OsStr::new)),
                expected,
                "stdin_tty={stdin_tty} stderr_tty={stderr_tty} CI={ci:?}"
            );
        }
    }

    #[test]
    fn wrapper_is_active_only_for_exact_one() {
        let cases: &[(Option<&str>, bool)] = &[
            (Some("1"), true),
            (None, false),
            (Some(""), false),
            (Some("0"), false),
            (Some("true"), false),
            (Some(" 1"), false),
            (Some("1 "), false),
        ];
        for &(value, expected) in cases {
            assert_eq!(
                wrapper_active_from(value.map(OsStr::new)),
                expected,
                "{SHELL_WRAPPER_VAR}={value:?}"
            );
        }
    }
}
