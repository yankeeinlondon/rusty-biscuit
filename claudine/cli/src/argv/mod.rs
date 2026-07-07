//! Pre-clap argv normalization for the claudine CLI.
//!
//! [`normalize`] is the single entry point above clap. It accepts the raw
//! `Vec<OsString>` produced by [`std::env::args_os`], applies a curated set
//! of purely syntactic rewrite rules, and returns the rewritten argv for
//! clap to parse.
//!
//! The rewrite rules (feature `2026-04-17-cli-pre-processing`) are:
//!
//! - **Rule 1** — on composition subcommands only, provider boolean flags
//!   (`--claude`, `--codex`, …) rewrite to `--provider <slug>`. Gating on
//!   composition subcommands preserves wrapper passthrough: a user typing
//!   `claudine claude --gemini file.md` would otherwise see `--gemini`
//!   silently rewritten into the child CLI's argv.
//! - **Rule 2** — fuzzy `--provider <value>` values are canonicalized to a
//!   known slug via `Provider::fuzzy_match_cli_name`.
//! - **Rule 3** — on composition subcommands (`compose`, `inline-compose`,
//!   `sequence`), pull composition flags that appear after the first
//!   shorthand setter back ahead of that setter, then insert a single `--`
//!   separator before the first positional setter that follows an
//!   interleaved flag.
//! - **Rule 4** — on composition subcommands, hoist a trailing `--help` or
//!   `-h` token to position 1 so the root help flag fires instead of being
//!   trapped inside clap's greedy positional collector.
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
//!
//! ## Rule ordering invariant
//!
//! Rule 4 must run before Rule 3 so `--help` is hoisted off the trailing
//! setter region before the `--` separator is inserted. Otherwise Rule 3
//! would bury `--help` inside clap's trailing raw-value bucket and the
//! downstream positional parser would misclassify it as a second file
//! reference.

use std::ffi::OsString;
use std::sync::LazyLock;

use crate::commands::compose::ComposeArgs;
use crate::commands::sequence::SequenceArgs;
use claudine::provider::Provider;

#[cfg(test)]
mod flag_surface;
mod rule1_provider_bool;
mod rule2_canonicalize;
mod rule3_separator;
mod rule4_help_hoist;

pub(crate) use rule1_provider_bool::provider_for_boolean_flag;
pub(crate) use rule2_canonicalize::is_fuzzy_provider_value;
pub(crate) use rule3_separator::{apply_composition_separator, pull_late_composition_flags};
pub(crate) use rule4_help_hoist::hoist_composition_help;

/// Wrapper subcommands that hand off to an external agent CLI.
pub(crate) const WRAPPER_SUBCOMMANDS: &[&str] = &[
    "claude", "codex", "gemini", "kimi", "qwen", "opencode", "goose", "kilo", "pi",
];

/// Composition subcommands that collect positional args plus `key=value`
/// setters in any order. Rule 3 only fires on these subcommands.
pub(crate) const COMPOSITION_SUBCOMMANDS: &[&str] = &["compose", "inline-compose", "sequence"];

/// Claudine root-level global long flags that consume the following token as
/// their value (distinct from the `--flag=value` form, which is one token).
const GLOBAL_FLAGS_WITH_VALUE: &[&str] = &["--debug"];

/// Composition-subcommand flags whose value lives in the next argv slot
/// rather than in a `--flag=value` pair. Rule 3 uses this to skip over
/// flag values so that a value like `gpt-4` after `-m` is not misclassified
/// as a positional token.
///
/// The surface is derived directly from clap's `ComposeArgs` and
/// `SequenceArgs` definitions at first use, so a new value-bearing
/// composition flag added to the clap surface is automatically tracked
/// without a separate constant to keep in lockstep.
static COMPOSITION_FLAGS_WITH_VALUE: LazyLock<Vec<String>> =
    LazyLock::new(collect_composition_value_flags);

/// Build the value-bearing composition flag surface by introspecting
/// clap's `ComposeArgs` and `SequenceArgs`.
pub(crate) fn collect_composition_value_flags() -> Vec<String> {
    use clap::{ArgAction, Args};

    fn collect<A: Args>(out: &mut Vec<String>) {
        let cmd = A::augment_args(clap::Command::new("__argv_introspect"));
        for arg in cmd.get_arguments() {
            // Skip boolean-style args; they do not consume the next argv token.
            if matches!(
                arg.get_action(),
                ArgAction::SetTrue
                    | ArgAction::SetFalse
                    | ArgAction::Count
                    | ArgAction::Help
                    | ArgAction::HelpShort
                    | ArgAction::HelpLong
                    | ArgAction::Version
            ) {
                continue;
            }
            // Skip positionals (no flag name).
            let has_flag_name = arg.get_long().is_some() || arg.get_short().is_some();
            if !has_flag_name {
                continue;
            }
            if let Some(long) = arg.get_long() {
                out.push(format!("--{long}"));
            }
            if let Some(aliases) = arg.get_visible_aliases() {
                for alias in aliases {
                    out.push(format!("--{alias}"));
                }
            }
            if let Some(short) = arg.get_short() {
                out.push(format!("-{short}"));
            }
        }
    }

    let mut flags = Vec::new();
    collect::<ComposeArgs>(&mut flags);
    collect::<SequenceArgs>(&mut flags);
    flags.sort();
    flags.dedup();
    flags
}

/// Normalize raw argv before clap parses it.
///
/// Applies Rules 1 and 2 left-to-right, stopping at the first literal `--`,
/// then Rule 3 and Rule 4 on the rewritten argv. See [the module docs](self)
/// for pass-through guarantees.
pub(crate) fn normalize(raw: Vec<OsString>) -> Vec<OsString> {
    normalize_inner(raw, completion_mode_active())
}

/// Dependency-injected variant of [`normalize`] that avoids reading the
/// process environment. Tests use this so they can assert the COMPLETE
/// pass-through guarantee without racing against other tests on a
/// shared env var.
///
/// Only compiled under `#[cfg(test)]` because it is exclusively a test-only
/// entry point; production code uses [`normalize`].
#[cfg(test)]
pub(crate) fn normalize_with_completion(
    raw: Vec<OsString>,
    completion_active: bool,
) -> Vec<OsString> {
    normalize_inner(raw, completion_active)
}

/// Shared core of [`normalize`] and the test-only
/// [`normalize_with_completion`]. Kept private so production callers cannot
/// accidentally bypass the env-var read.
fn normalize_inner(raw: Vec<OsString>, completion_active: bool) -> Vec<OsString> {
    if completion_active {
        return raw;
    }
    if raw.len() < 2 {
        return raw;
    }

    // The hidden `__complete` subcommand receives the user's original argv
    // as its trailing value. Rewriting any of those tokens (Rule 2's
    // `--provider` canonicalization in particular) would corrupt the
    // payload the supplement engine classifies against, so normalize is a
    // full no-op for `__complete` subcommand invocations.
    if find_subcommand(&raw, &["__complete"]).is_some() {
        return raw;
    }

    // Rule 1 is gated on composition subcommands so wrapper passthrough
    // never sees its own flags silently rewritten into `--provider <slug>`.
    let is_composition = find_subcommand(&raw, COMPOSITION_SUBCOMMANDS).is_some();

    let stop = first_dash_dash_index(&raw).unwrap_or(raw.len());
    let mut out: Vec<OsString> = Vec::with_capacity(raw.len());
    let mut index = 0;
    while index < stop {
        let token = &raw[index];
        match as_utf8(token) {
            Some(text) => {
                if is_composition && let Some(provider) = provider_for_boolean_flag(text) {
                    // Rule 1: `--claude` → `--provider claude`
                    out.push(OsString::from("--provider"));
                    out.push(OsString::from(provider.as_slug()));
                    index += 1;
                    continue;
                }

                if text == "--provider" {
                    // Rule 2 (space form): `--provider cl` → `--provider claude`
                    out.push(token.clone());
                    index += 1;
                    if index < stop {
                        let value_token = &raw[index];
                        match as_utf8(value_token) {
                            Some(value_text) if is_fuzzy_provider_value(value_text) => {
                                let rewritten = Provider::fuzzy_match_cli_name(value_text)
                                    .map(|p| OsString::from(p.as_slug()))
                                    .unwrap_or_else(|| value_token.clone());
                                out.push(rewritten);
                            }
                            _ => out.push(value_token.clone()),
                        }
                        index += 1;
                    }
                    continue;
                }

                if let Some(value_text) = text.strip_prefix("--provider=") {
                    // Rule 2 (equals form): `--provider=oc` → `--provider=opencode`
                    if is_fuzzy_provider_value(value_text)
                        && let Some(provider) = Provider::fuzzy_match_cli_name(value_text)
                    {
                        out.push(OsString::from(format!("--provider={}", provider.as_slug())));
                        index += 1;
                        continue;
                    }
                    out.push(token.clone());
                    index += 1;
                    continue;
                }

                out.push(token.clone());
                index += 1;
            }
            None => {
                out.push(token.clone());
                index += 1;
            }
        }
    }

    // Copy the `--` separator and everything after it verbatim.
    while index < raw.len() {
        out.push(raw[index].clone());
        index += 1;
    }

    // Rule 4 must precede Rule 3 so `--help` is hoisted off the trailing
    // setter region before the `--` separator is inserted. Otherwise Rule 3
    // would bury `--help` inside clap's trailing raw-value bucket and the
    // downstream positional parser would misclassify it as a second file
    // reference.
    let with_help_hoisted = hoist_composition_help(out);
    let with_late_flags_pulled = pull_late_composition_flags(with_help_hoisted);
    apply_composition_separator(with_late_flags_pulled)
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
pub(crate) fn first_dash_dash_index(raw: &[OsString]) -> Option<usize> {
    raw.iter().position(is_dash_dash)
}

/// UTF-8 accessor that returns `None` for tokens that are not valid UTF-8.
///
/// All rewrite rules are pattern-based on `&str`, so non-UTF-8 tokens are
/// deliberately skipped and passed through unchanged. Centralizing the
/// conversion here keeps the "we deliberately leave opaque tokens alone"
/// contract in one place.
pub(crate) fn as_utf8(token: &OsString) -> Option<&str> {
    token.to_str()
}

/// True when the token is exactly `--` (the wrapper separator).
pub(crate) fn is_dash_dash(token: &OsString) -> bool {
    token.to_str() == Some("--")
}

/// True when the token looks like a flag (`-x`, `--long`, `--long=value`).
///
/// Bare `-` and `--` are not flags; both are reserved conventions that clap
/// handles itself.
pub(crate) fn looks_like_flag(token: &str) -> bool {
    token.starts_with('-') && token != "-" && token != "--"
}

/// True when `token` is a known Claudine root global flag whose value lives
/// in the next argv slot rather than in a `--flag=value` pair.
pub(crate) fn is_global_flag_with_value(token: &str) -> bool {
    if token.contains('=') {
        return false;
    }
    GLOBAL_FLAGS_WITH_VALUE.contains(&token)
}

/// True when `token` is a composition-surface flag whose value lives in the
/// next argv slot rather than in a `--flag=value` pair.
pub(crate) fn is_composition_flag_with_value(token: &str) -> bool {
    if token.contains('=') {
        return false;
    }
    COMPOSITION_FLAGS_WITH_VALUE
        .iter()
        .any(|flag| flag.as_str() == token)
}

/// True when `token` matches the composition shorthand-setter key pattern
/// `^[A-Za-z_][A-Za-z0-9_-]*=`.
///
/// This is the same key validation used by
/// `crate::commands::compose`'s `parse_compose_setter`; keeping them in lockstep
/// guarantees that any token Rule 3 protects behind a `--` separator will be
/// classified the same way by the downstream positional parser.
pub(crate) fn looks_like_setter(token: &str) -> bool {
    let Some(eq_pos) = token.find('=') else {
        return false;
    };
    let key = &token[..eq_pos];
    let mut chars = key.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
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
        let input = argv(&["claudine", "claude", "--", "--help", "--gemini", "name=Ken"]);
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

    #[test]
    fn normalize_is_noop_when_completion_mode_is_injected() {
        // `normalize_with_completion` lets tests assert the COMPLETE
        // guarantee without touching process-wide env vars (which would
        // race against other parallel tests on the same `normalize`).
        let input = argv(&[
            "claudine", "compose", "file.md", "--gemini", "name=Ken", "--help",
        ]);
        assert_eq!(
            normalize_with_completion(input.clone(), true),
            input,
            "argv must pass through untouched while completion mode is active"
        );
    }

    #[test]
    fn normalize_with_completion_off_matches_normalize_happy_path() {
        // Lock in that `normalize_with_completion(_, false)` produces the
        // exact argv `normalize` would emit for the same input when the
        // environment is clean — otherwise the test-only entry point
        // could silently drift from the production path. This also
        // exercises the full Rule 1 → Rule 4 → Rule 3 chain: `--gemini`
        // is rewritten to `--provider gemini`, `--help` is hoisted to
        // position 1, and `--` is inserted before the trailing setter.
        let input = argv(&[
            "claudine", "compose", "file.md", "--gemini", "name=Ken", "--help",
        ]);
        let expected = argv(&[
            "claudine",
            "--help",
            "compose",
            "file.md",
            "--provider",
            "gemini",
            "--",
            "name=Ken",
        ]);
        assert_eq!(normalize_with_completion(input, false), expected);
    }

    // ── Pass-through assertions for non-provider argv ────────────────

    #[test]
    fn normalize_leaves_version_argv_untouched() {
        let input = argv(&["claudine", "--version"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn normalize_leaves_hooks_describe_untouched() {
        let input = argv(&["claudine", "hooks", "--describe"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn normalize_leaves_wrapper_passthrough_untouched() {
        let input = argv(&["claudine", "claude", "--", "--resume", "some-session-id"]);
        assert_eq!(normalize(input.clone()), input);
    }

    // ── Combined Rule 1 + Rule 2 flow ────────────────────────────────

    #[test]
    fn rules_compose_boolean_then_fuzzy_value() {
        let input = argv(&[
            "claudine",
            "compose",
            "--gemini",
            "file.md",
            "--provider",
            "cl",
        ]);
        let expected = argv(&[
            "claudine",
            "compose",
            "--provider",
            "gemini",
            "file.md",
            "--provider",
            "claude",
        ]);
        assert_eq!(normalize(input), expected);
    }

    // ── Gap 7: `find_subcommand` handles --debug=LEVEL after subcommand ──

    #[test]
    fn find_subcommand_handles_equals_form_after_composition_subcommand() {
        // `--debug` is a global flag that can appear after the subcommand
        // as well. The equals form (`--debug=trace`) is a single token, so
        // `find_subcommand` already handles it, but lock that in so a
        // future regression is caught by the test suite.
        let input = argv(&["claudine", "compose", "--debug=trace", "file.md"]);
        assert_eq!(
            find_subcommand(&input, COMPOSITION_SUBCOMMANDS),
            Some((1, "compose"))
        );
    }
}
