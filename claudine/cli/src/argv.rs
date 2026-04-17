//! Pre-clap argv normalization for the claudine CLI.
//!
//! [`normalize`] is the single entry point above clap. It accepts the raw
//! `Vec<OsString>` produced by [`std::env::args_os`], applies a curated set
//! of purely syntactic rewrite rules, and returns the rewritten argv for
//! clap to parse.
//!
//! The planned rewrite rules (feature `2026-04-17-cli-pre-processing`) are:
//!
//! - **Rule 1** — provider boolean flags (`--claude`, `--codex`, …) rewrite
//!   to `--provider <slug>`.
//! - **Rule 2** — fuzzy `--provider <value>` values are canonicalized to a
//!   known slug via `Provider::fuzzy_match_cli_name`.
//! - **Rule 3** — on composition subcommands (`compose`, `inline-compose`,
//!   `sequence`), insert a single `--` separator before the first positional
//!   setter that follows an interleaved flag.
//!
//! Phase 1 lands the module plumbing and the shared ingress seam only.
//! `normalize` is intentionally behaviorally neutral here; Rules 1-3 land
//! in later phases.
//!
//! ## Pass-through guarantees
//!
//! The normalizer must **never** mutate argv in any of these situations:
//!
//! - `COMPLETE` is set in the process environment (dynamic shell completion).
//! - Tokens at or after the first literal `--` (the wrapper separator).
//! - Non-UTF-8 tokens (rules are pattern-based on `&str`).
//! - argv has fewer than two elements (nothing downstream for clap to parse).
//!
//! When a new rule is added, a matching pass-through unit test MUST be added
//! alongside it so the normalizer cannot silently start rewriting inputs it
//! should leave alone.

use std::ffi::OsString;

/// Wrapper subcommands that hand off to an external agent CLI.
pub(crate) const WRAPPER_SUBCOMMANDS: &[&str] = &[
    "claude", "codex", "gemini", "kimi", "qwen", "opencode", "goose",
];

/// Composition subcommands that collect positional args plus `key=value`
/// setters in any order. Rule 3 only fires on these subcommands.
#[allow(dead_code)]
pub(crate) const COMPOSITION_SUBCOMMANDS: &[&str] = &["compose", "inline-compose", "sequence"];

/// Claudine root-level global long flags that consume the following token as
/// their value (distinct from the `--flag=value` form, which is one token).
const GLOBAL_FLAGS_WITH_VALUE: &[&str] = &["--debug"];

/// Normalize raw argv before clap parses it.
///
/// In Phase 1 this is a no-op beyond the pass-through guards. Later phases
/// plug in Rules 1-3 against the scanner helpers below.
pub(crate) fn normalize(raw: Vec<OsString>) -> Vec<OsString> {
    if completion_mode_active() {
        return raw;
    }
    if raw.len() < 2 {
        return raw;
    }
    raw
}

/// Locate the first subcommand token in argv that matches any candidate.
///
/// The scanner skips argv[0] and every known Claudine root global flag that
/// appears before the subcommand (`--verbose`, `-v`, `--debug [LEVEL]`,
/// `--debug=LEVEL`, `--plain`, `--help`, `-h`). It stops at the first literal
/// `--` so nothing after the wrapper separator is ever considered.
///
/// Returns the index of the matched token in `raw` together with the
/// candidate string that matched (borrowed from `candidates`).
pub(crate) fn find_subcommand<'a>(
    raw: &[OsString],
    candidates: &'a [&'a str],
) -> Option<(usize, &'a str)> {
    let end = first_dash_dash_index(raw).unwrap_or(raw.len());
    let mut cursor = 1;
    while cursor < end {
        let Some(token) = as_utf8(&raw[cursor]) else {
            cursor += 1;
            continue;
        };
        if is_global_flag_with_value(token) {
            cursor += 2;
            continue;
        }
        if looks_like_flag(token) {
            cursor += 1;
            continue;
        }
        return candidates
            .iter()
            .find(|candidate| **candidate == token)
            .map(|candidate| (cursor, *candidate));
    }
    None
}

/// True when `clap_complete::CompleteEnv` is driving the CLI.
///
/// `clap_complete` exposes completion mode through the `COMPLETE` environment
/// variable; its presence is the documented signal that argv must not be
/// reshaped.
pub(crate) fn completion_mode_active() -> bool {
    std::env::var_os("COMPLETE").is_some()
}

/// Index of the first literal `--` token in `raw`, if any.
fn first_dash_dash_index(raw: &[OsString]) -> Option<usize> {
    raw.iter().position(is_dash_dash)
}

/// UTF-8 accessor that returns `None` for tokens that are not valid UTF-8.
fn as_utf8(token: &OsString) -> Option<&str> {
    token.to_str()
}

/// True when the token is exactly `--` (the wrapper separator).
fn is_dash_dash(token: &OsString) -> bool {
    token.to_str() == Some("--")
}

/// True when the token looks like a flag (`-x`, `--long`, `--long=value`).
///
/// Bare `-` and `--` are not flags; both are reserved conventions that clap
/// handles itself.
fn looks_like_flag(token: &str) -> bool {
    token.starts_with('-') && token != "-" && token != "--"
}

/// True when `token` is a known Claudine root global flag whose value lives
/// in the next argv slot rather than in a `--flag=value` pair.
fn is_global_flag_with_value(token: &str) -> bool {
    if token.contains('=') {
        return false;
    }
    GLOBAL_FLAGS_WITH_VALUE.contains(&token)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

    #[test]
    fn normalize_is_behaviorally_neutral_on_typical_argv() {
        let input = argv(&["claudine", "compose", "file.md", "key=val"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn normalize_passes_empty_argv_through() {
        let input = argv(&[]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn normalize_passes_bare_bin_through() {
        let input = argv(&["claudine"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn normalize_preserves_tokens_after_dash_dash() {
        let input = argv(&[
            "claudine",
            "claude",
            "--",
            "--help",
            "--gemini",
            "name=Ken",
        ]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[cfg(unix)]
    #[test]
    fn normalize_preserves_non_utf8_tokens() {
        use std::os::unix::ffi::OsStringExt;
        let input = vec![
            OsString::from("claudine"),
            OsString::from("compose"),
            OsString::from_vec(vec![0xff]),
        ];
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn find_subcommand_matches_wrapper_at_position_one() {
        let input = argv(&["claudine", "claude", "--yolo"]);
        assert_eq!(
            find_subcommand(&input, WRAPPER_SUBCOMMANDS),
            Some((1, "claude"))
        );
    }

    #[test]
    fn find_subcommand_skips_global_bool_flag() {
        let input = argv(&["claudine", "--plain", "codex"]);
        assert_eq!(
            find_subcommand(&input, WRAPPER_SUBCOMMANDS),
            Some((2, "codex"))
        );
    }

    #[test]
    fn find_subcommand_skips_short_bool_flag() {
        let input = argv(&["claudine", "-v", "gemini"]);
        assert_eq!(
            find_subcommand(&input, WRAPPER_SUBCOMMANDS),
            Some((2, "gemini"))
        );
    }

    #[test]
    fn find_subcommand_skips_global_flag_with_value_token() {
        let input = argv(&["claudine", "--debug", "trace", "opencode"]);
        assert_eq!(
            find_subcommand(&input, WRAPPER_SUBCOMMANDS),
            Some((3, "opencode"))
        );
    }

    #[test]
    fn find_subcommand_handles_equals_form_for_global_value_flag() {
        let input = argv(&["claudine", "--debug=trace", "goose"]);
        assert_eq!(
            find_subcommand(&input, WRAPPER_SUBCOMMANDS),
            Some((2, "goose"))
        );
    }

    #[test]
    fn find_subcommand_returns_none_when_candidate_is_unknown() {
        let input = argv(&["claudine", "unknown-subcommand"]);
        assert_eq!(find_subcommand(&input, WRAPPER_SUBCOMMANDS), None);
    }

    #[test]
    fn find_subcommand_returns_none_when_only_flags_present() {
        let input = argv(&["claudine", "--plain", "--help"]);
        assert_eq!(find_subcommand(&input, WRAPPER_SUBCOMMANDS), None);
    }

    #[test]
    fn find_subcommand_stops_at_dash_dash() {
        let input = argv(&["claudine", "--", "claude"]);
        assert_eq!(find_subcommand(&input, WRAPPER_SUBCOMMANDS), None);
    }

    #[test]
    fn find_subcommand_matches_composition_surface() {
        let input = argv(&["claudine", "compose", "file.md"]);
        assert_eq!(
            find_subcommand(&input, COMPOSITION_SUBCOMMANDS),
            Some((1, "compose"))
        );
    }

    #[test]
    fn find_subcommand_matches_composition_after_global_flags() {
        let input = argv(&["claudine", "--plain", "sequence", "file.md"]);
        assert_eq!(
            find_subcommand(&input, COMPOSITION_SUBCOMMANDS),
            Some((2, "sequence"))
        );
    }

    #[test]
    fn first_dash_dash_index_finds_separator() {
        let input = argv(&["claudine", "compose", "--", "name=Ken"]);
        assert_eq!(first_dash_dash_index(&input), Some(2));
    }

    #[test]
    fn first_dash_dash_index_returns_none_when_missing() {
        let input = argv(&["claudine", "compose", "file.md"]);
        assert_eq!(first_dash_dash_index(&input), None);
    }

    #[test]
    fn looks_like_flag_accepts_long_short_and_equals() {
        assert!(looks_like_flag("--help"));
        assert!(looks_like_flag("--debug=trace"));
        assert!(looks_like_flag("-v"));
        assert!(looks_like_flag("-vv"));
    }

    #[test]
    fn looks_like_flag_rejects_dash_dash_and_bare_dash() {
        assert!(!looks_like_flag("--"));
        assert!(!looks_like_flag("-"));
    }

    #[test]
    fn looks_like_flag_rejects_non_flag_positional() {
        assert!(!looks_like_flag("name=Ken"));
        assert!(!looks_like_flag("file.md"));
        assert!(!looks_like_flag("compose"));
    }

    #[test]
    fn is_global_flag_with_value_matches_known_flag() {
        assert!(is_global_flag_with_value("--debug"));
    }

    #[test]
    fn is_global_flag_with_value_rejects_equals_form() {
        assert!(!is_global_flag_with_value("--debug=trace"));
    }

    #[test]
    fn is_global_flag_with_value_rejects_unknown_flag() {
        assert!(!is_global_flag_with_value("--plain"));
        assert!(!is_global_flag_with_value("--verbose"));
    }

    #[test]
    fn completion_mode_active_is_safe_to_call() {
        let _ = completion_mode_active();
    }
}
