//! How `wt` reads its caller: whether a person can answer prompts, and whether
//! a shell wrapper will act on the `cd:` protocol lines printed to stdout.

use std::ffi::OsStr;
use std::io::IsTerminal as _;
#[cfg(debug_assertions)]
use std::time::Duration;

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

/// A debug build's override of `wt list`'s ordinary wait, in milliseconds.
///
/// Binary tests set it on a listing whose worker they hold past the wait, so
/// that listing ends sooner, or raise it where a slow runner's worker must
/// still land inside the wait. Only debug builds read it (the `dev` and `test`
/// profiles, which local and CI test runs use); a release build always waits
/// the real budget. The `--refresh`/`--ff` wait is never shortened.
pub const TEST_WAIT_BUDGET_VAR: &str = "WT_TEST_WAIT_BUDGET_MS";

/// The ordinary wait a debug build's test asked for through
/// [`TEST_WAIT_BUDGET_VAR`], if any.
///
/// ## Panics
///
/// When the variable is set to anything but a positive whole number of
/// milliseconds: a mistyped override would otherwise pass silently on the
/// real budget.
#[cfg(debug_assertions)]
pub fn test_wait_budget() -> Option<Duration> {
    test_wait_budget_from(std::env::var_os(TEST_WAIT_BUDGET_VAR).as_deref())
}

#[cfg(debug_assertions)]
fn test_wait_budget_from(value: Option<&OsStr>) -> Option<Duration> {
    milliseconds_from(TEST_WAIT_BUDGET_VAR, value)
}

/// A debug build's override of the refresh worker's PR request deadline
/// ([`worktree::pull_requests::REFRESH_DEADLINE`]), in milliseconds.
///
/// Binary tests whose stand-in holds the worker's request set it past their
/// own waits, so a held worker ends when the test releases it (or its
/// teardown reaps it), never because the production deadline ran out first
/// on a loaded host. Only debug builds read it.
pub const TEST_REFRESH_DEADLINE_VAR: &str = "WT_TEST_REFRESH_DEADLINE_MS";

/// The PR request deadline a debug build's test asked for through
/// [`TEST_REFRESH_DEADLINE_VAR`], if any.
///
/// ## Panics
///
/// As [`test_wait_budget`].
#[cfg(debug_assertions)]
pub fn test_refresh_deadline() -> Option<Duration> {
    milliseconds_from(TEST_REFRESH_DEADLINE_VAR, std::env::var_os(TEST_REFRESH_DEADLINE_VAR).as_deref())
}

/// A debug build's stand-in for the free space `wt remove` reads before it
/// writes a deletion journal, in bytes.
///
/// Binary tests set it to prove the space refusal through the real binary;
/// no test can make a real filesystem report a chosen figure. Only debug
/// builds read it; a release build always reads the filesystem.
pub const TEST_AVAILABLE_SPACE_VAR: &str = "WT_TEST_AVAILABLE_SPACE_BYTES";

/// The free space a debug build's test asked for through
/// [`TEST_AVAILABLE_SPACE_VAR`], if any.
///
/// ## Panics
///
/// When the variable is set to anything but a whole number of bytes.
#[cfg(debug_assertions)]
pub fn test_available_space() -> Option<u64> {
    test_available_space_from(std::env::var_os(TEST_AVAILABLE_SPACE_VAR).as_deref())
}

#[cfg(debug_assertions)]
fn test_available_space_from(value: Option<&OsStr>) -> Option<u64> {
    let value = value?;
    let bytes = value
        .to_str()
        .and_then(|text| text.parse::<u64>().ok())
        .unwrap_or_else(|| panic!("{TEST_AVAILABLE_SPACE_VAR} must be a whole number of bytes, not {value:?}"));
    Some(bytes)
}

#[cfg(debug_assertions)]
fn milliseconds_from(name: &str, value: Option<&OsStr>) -> Option<Duration> {
    let value = value?;
    let millis = value
        .to_str()
        .and_then(|text| text.parse::<u64>().ok())
        .filter(|millis| *millis > 0)
        .unwrap_or_else(|| panic!("{name} must be a positive whole number of milliseconds, not {value:?}"));
    Some(Duration::from_millis(millis))
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

    #[cfg(debug_assertions)]
    #[test]
    fn the_test_wait_budget_is_read_only_when_set_to_positive_milliseconds() {
        assert_eq!(test_wait_budget_from(None), None);
        assert_eq!(test_wait_budget_from(Some(OsStr::new("250"))), Some(Duration::from_millis(250)));
        for refused in ["", "0", "-5", "1.5", "3s", " 250"] {
            let parsed = std::panic::catch_unwind(|| test_wait_budget_from(Some(OsStr::new(refused))));
            assert!(parsed.is_err(), "{TEST_WAIT_BUDGET_VAR}={refused:?} was accepted");
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn the_test_refresh_deadline_is_read_only_when_set_to_positive_milliseconds() {
        let read = |value: Option<&str>| milliseconds_from(TEST_REFRESH_DEADLINE_VAR, value.map(OsStr::new));
        assert_eq!(read(None), None);
        assert_eq!(read(Some("60000")), Some(Duration::from_secs(60)));
        for refused in ["", "0", "-5", "10s"] {
            let parsed = std::panic::catch_unwind(|| read(Some(refused)));
            assert!(parsed.is_err(), "{TEST_REFRESH_DEADLINE_VAR}={refused:?} was accepted");
        }
    }

    #[cfg(debug_assertions)]
    #[test]
    fn the_test_available_space_is_read_only_when_set_to_whole_bytes() {
        assert_eq!(test_available_space_from(None), None);
        assert_eq!(test_available_space_from(Some(OsStr::new("0"))), Some(0));
        assert_eq!(test_available_space_from(Some(OsStr::new("4096"))), Some(4096));
        for refused in ["", "-5", "1.5", "4k", " 10"] {
            let parsed = std::panic::catch_unwind(|| test_available_space_from(Some(OsStr::new(refused))));
            assert!(parsed.is_err(), "{TEST_AVAILABLE_SPACE_VAR}={refused:?} was accepted");
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
