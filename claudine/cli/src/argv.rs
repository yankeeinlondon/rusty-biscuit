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
//!   `sequence`), insert a single `--` separator before the first positional
//!   setter that follows an interleaved flag.
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

use std::ffi::OsString;

use claudine::events::{PROVIDERS_DISPLAY_ORDER, Provider};

/// Wrapper subcommands that hand off to an external agent CLI.
pub(crate) const WRAPPER_SUBCOMMANDS: &[&str] = &[
    "claude", "codex", "gemini", "kimi", "qwen", "opencode", "goose",
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
/// The list mirrors the `#[arg(...)]` surface of `SharedComposeArgs` (and
/// `sequence`'s `--fail-fast`). When a new value-bearing composition flag is
/// added to the clap surface, add it here so Rule 3 keeps classifying its
/// value correctly.
const COMPOSITION_FLAGS_WITH_VALUE: &[&str] = &[
    "--provider",
    "--exclude",
    "--include",
    "--model",
    "-m",
    "--output",
    "-o",
    "--append-system-prompt",
    "--asp",
    "--replace-system-prompt",
    "--rsp",
    "--timeout",
    "-t",
    "--operation",
    "--op",
    "--set",
    "--use",
    "--fail-fast",
];

// Provider boolean flag mapping (Rule 1 surface) is derived from
// [`Provider::cli_aliases`] — see [`provider_for_boolean_flag`].

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
fn normalize_with_completion(raw: Vec<OsString>, completion_active: bool) -> Vec<OsString> {
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
                if is_composition
                    && let Some(provider) = provider_for_boolean_flag(text)
                {
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
                    // Rule 2 (equals form): `--provider=oc` → `--provider=open_code`
                    if is_fuzzy_provider_value(value_text)
                        && let Some(provider) = Provider::fuzzy_match_cli_name(value_text)
                    {
                        out.push(OsString::from(format!(
                            "--provider={}",
                            provider.as_slug()
                        )));
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
    apply_composition_separator(with_help_hoisted)
}

/// Apply Rule 3: insert a single `--` before the first setter-shaped token on
/// `compose`, `inline-compose`, or `sequence` that follows an interleaved
/// flag after a previously seen positional.
///
/// No-ops when:
///
/// - The argv already contains a `--` separator.
/// - The argv does not target a composition subcommand.
/// - No positional was seen before the first setter-shaped token.
/// - No flag interleaved between that positional and the setter.
/// - The subcommand token is preceded by a non-flag, non-matching token
///   (i.e. an unknown subcommand owns position 1; clap handles it).
fn apply_composition_separator(argv: Vec<OsString>) -> Vec<OsString> {
    if first_dash_dash_index(&argv).is_some() {
        return argv;
    }
    let Some((sub_idx, _)) = find_subcommand(&argv, COMPOSITION_SUBCOMMANDS) else {
        return argv;
    };

    let mut saw_positional = false;
    let mut saw_flag_after_positional = false;
    let mut insertion_index: Option<usize> = None;

    let mut cursor = sub_idx + 1;
    while cursor < argv.len() {
        let Some(token) = as_utf8(&argv[cursor]) else {
            // Non-UTF-8 tokens are pattern-invisible. Treat them as opaque
            // and do not let them advance the state machine.
            cursor += 1;
            continue;
        };

        if is_composition_flag_with_value(token) {
            if saw_positional {
                saw_flag_after_positional = true;
            }
            // Skip the flag token and its value slot.
            cursor += 2;
            continue;
        }

        if looks_like_flag(token) {
            if saw_positional {
                saw_flag_after_positional = true;
            }
            cursor += 1;
            continue;
        }

        if looks_like_setter(token) {
            if saw_positional && saw_flag_after_positional {
                insertion_index = Some(cursor);
                break;
            }
            cursor += 1;
            continue;
        }

        saw_positional = true;
        cursor += 1;
    }

    let Some(idx) = insertion_index else {
        return argv;
    };

    let mut result = Vec::with_capacity(argv.len() + 1);
    result.extend(argv[..idx].iter().cloned());
    result.push(OsString::from("--"));
    result.extend(argv[idx..].iter().cloned());
    result
}

/// Apply Rule 4: hoist `-h`/`--help` on composition subcommands to argv
/// position 1 so the root [`Cli::help`] handler fires.
///
/// `args.rs` sets `disable_help_flag = true` on the root `Cli` and declares
/// a non-global `help: bool`, which means composition subcommands never
/// inherit a working `--help` handler. Without this rule, a user typing
/// `claudine compose file.md --gemini name=Ken --help` lands in clap's
/// greedy positional collector and sees either the misleading "unexpected
/// argument" tip or a downstream "expected at most one file reference"
/// error.
///
/// Hoisting `--help` / `-h` to position 1 converts the same argv into a
/// root-help invocation, which `main.rs` catches and forwards to
/// [`crate::commands::help::run`]. The rest of the argv still parses
/// cleanly under clap (compose accepts the remaining positionals), but
/// `cli.help == true` short-circuits into the grouped help screen before
/// any subcommand runs.
///
/// No-ops when:
///
/// - The argv does not target a composition subcommand.
/// - No `-h`/`--help` token is present between the subcommand and the
///   first literal `--`.
/// - An earlier pass already hoisted a help token to position 1.
fn hoist_composition_help(argv: Vec<OsString>) -> Vec<OsString> {
    let Some((sub_idx, _)) = find_subcommand(&argv, COMPOSITION_SUBCOMMANDS) else {
        return argv;
    };

    if argv.get(1).and_then(|t| t.to_str()) == Some("--help")
        || argv.get(1).and_then(|t| t.to_str()) == Some("-h")
    {
        // Already hoisted; nothing to do.
        return argv;
    }

    let stop = first_dash_dash_index(&argv).unwrap_or(argv.len());
    let help_index = argv
        .iter()
        .enumerate()
        .skip(sub_idx + 1)
        .take_while(|(idx, _)| *idx < stop)
        .find_map(|(idx, token)| {
            let text = token.to_str()?;
            (text == "--help" || text == "-h").then_some(idx)
        });

    let Some(idx) = help_index else {
        return argv;
    };

    let mut result = Vec::with_capacity(argv.len());
    result.push(argv[0].clone());
    result.push(argv[idx].clone());
    result.extend(argv[1..idx].iter().cloned());
    result.extend(argv[idx + 1..].iter().cloned());
    result
}

/// Returns the [`Provider`] that a boolean flag token selects, if any.
///
/// The boolean flag surface is derived from [`Provider::cli_aliases`]: the
/// first alias of each provider (`claude`, `codex`, `gemini`, `goose`,
/// `kimi`, `opencode`, `qwen`, `roo`) is the user-facing flag name, and
/// the normalizer rewrites `--<first-alias>` to `--provider <as_slug()>`.
/// Keeping the mapping derived means a new provider added with a matching
/// clap boolean flag declaration in `SharedComposeArgs` automatically
/// inherits Rule 1 coverage.
fn provider_for_boolean_flag(token: &str) -> Option<Provider> {
    let name = token.strip_prefix("--")?;
    if name.is_empty() || name.starts_with('-') {
        return None;
    }
    PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .find(|provider| provider.cli_aliases().first().copied() == Some(name))
}

/// True when a `--provider` value token is a candidate for fuzzy rewrite.
///
/// Empty values and hyphen-prefixed tokens (which look like flags) are left
/// alone so clap can emit its native "a value is required" / invalid-value
/// errors instead of the normalizer silently swallowing them.
fn is_fuzzy_provider_value(value: &str) -> bool {
    !value.is_empty() && !value.starts_with('-')
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
///
/// All rewrite rules are pattern-based on `&str`, so non-UTF-8 tokens are
/// deliberately skipped and passed through unchanged. Centralizing the
/// conversion here keeps the "we deliberately leave opaque tokens alone"
/// contract in one place.
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

/// True when `token` is a composition-surface flag whose value lives in the
/// next argv slot rather than in a `--flag=value` pair.
fn is_composition_flag_with_value(token: &str) -> bool {
    if token.contains('=') {
        return false;
    }
    COMPOSITION_FLAGS_WITH_VALUE.contains(&token)
}

/// True when `token` matches the composition shorthand-setter key pattern
/// `^[A-Za-z_][A-Za-z0-9_-]*=`.
///
/// This is the same key validation used by
/// [`crate::commands::compose::parse_compose_setter`]; keeping them in lockstep
/// guarantees that any token Rule 3 protects behind a `--` separator will be
/// classified the same way by the downstream positional parser.
fn looks_like_setter(token: &str) -> bool {
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

    // ── Rule 1: provider boolean → --provider <slug> ─────────────────

    #[test]
    fn rule_1_rewrites_claude_boolean_to_provider_slug() {
        let input = argv(&["claudine", "compose", "file.md", "--claude"]);
        let expected = argv(&["claudine", "compose", "file.md", "--provider", "claude"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_1_rewrites_gemini_boolean_in_interleaved_position() {
        let input = argv(&["claudine", "compose", "--gemini", "file.md"]);
        let expected = argv(&["claudine", "compose", "--provider", "gemini", "file.md"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_1_preserves_canonical_kimi_slug() {
        let input = argv(&["claudine", "compose", "--kimi"]);
        let expected = argv(&["claudine", "compose", "--provider", "kimi_code"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_1_preserves_canonical_opencode_slug() {
        let input = argv(&["claudine", "compose", "--opencode"]);
        let expected = argv(&["claudine", "compose", "--provider", "open_code"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_1_preserves_canonical_qwen_slug() {
        let input = argv(&["claudine", "compose", "--qwen"]);
        let expected = argv(&["claudine", "compose", "--provider", "qwen_code"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_1_preserves_canonical_roo_slug() {
        let input = argv(&["claudine", "compose", "--roo"]);
        let expected = argv(&["claudine", "compose", "--provider", "roo_code"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_1_rewrites_every_provider_boolean() {
        let booleans = [
            ("--claude", "claude"),
            ("--codex", "codex"),
            ("--gemini", "gemini"),
            ("--goose", "goose"),
            ("--kimi", "kimi_code"),
            ("--opencode", "open_code"),
            ("--qwen", "qwen_code"),
            ("--roo", "roo_code"),
        ];
        for (flag, slug) in booleans {
            let input = argv(&["claudine", "compose", flag]);
            let expected = argv(&["claudine", "compose", "--provider", slug]);
            assert_eq!(normalize(input), expected, "flag {flag}");
        }
    }

    #[test]
    fn rule_1_duplicates_survive_to_clap() {
        let input = argv(&["claudine", "compose", "--claude", "--gemini"]);
        let expected = argv(&[
            "claudine",
            "compose",
            "--provider",
            "claude",
            "--provider",
            "gemini",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_1_coexists_with_explicit_provider_flag() {
        let input = argv(&[
            "claudine",
            "compose",
            "--provider",
            "claude",
            "--gemini",
            "file.md",
        ]);
        let expected = argv(&[
            "claudine",
            "compose",
            "--provider",
            "claude",
            "--provider",
            "gemini",
            "file.md",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_1_does_not_fuzzy_match_near_miss_flags() {
        let input = argv(&["claudine", "compose", "--claud", "file.md"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_1_ignores_provider_booleans_after_dash_dash() {
        let input = argv(&["claudine", "claude", "--", "--gemini", "file.md"]);
        assert_eq!(normalize(input.clone()), input);
    }

    // ── Rule 2: fuzzy --provider <value> canonicalization ────────────

    #[test]
    fn rule_2_rewrites_cl_to_claude_via_space_form() {
        let input = argv(&["claudine", "compose", "--provider", "cl"]);
        let expected = argv(&["claudine", "compose", "--provider", "claude"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_2_rewrites_gem_to_gemini_via_space_form() {
        let input = argv(&["claudine", "compose", "--provider", "gem"]);
        let expected = argv(&["claudine", "compose", "--provider", "gemini"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_2_rewrites_gem_via_equals_form() {
        let input = argv(&["claudine", "compose", "--provider=gem"]);
        let expected = argv(&["claudine", "compose", "--provider=gemini"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_2_canonicalizes_exact_slug_unchanged() {
        let input = argv(&["claudine", "compose", "--provider", "claude"]);
        let expected = argv(&["claudine", "compose", "--provider", "claude"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_2_leaves_unknown_value_untouched() {
        let input = argv(&["claudine", "compose", "--provider", "nonesuch"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_leaves_missing_value_untouched() {
        let input = argv(&["claudine", "compose", "--provider"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_leaves_empty_equals_value_untouched() {
        let input = argv(&["claudine", "compose", "--provider="]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_leaves_hyphen_prefixed_next_token_untouched() {
        let input = argv(&["claudine", "compose", "--provider", "-x"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_does_not_cross_dash_dash() {
        let input = argv(&["claudine", "claude", "--", "--provider", "cl"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_2_skips_value_after_dash_dash_boundary() {
        // `--provider` immediately before a `--` still advertises a value token;
        // the `--` is after the scan range, so the flag is treated as
        // value-less and the separator passes through untouched.
        let input = argv(&["claudine", "compose", "--provider", "--", "name=Ken"]);
        let expected = argv(&["claudine", "compose", "--provider", "--", "name=Ken"]);
        assert_eq!(normalize(input), expected);
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
        let input = argv(&[
            "claudine", "claude", "--", "--resume", "some-session-id",
        ]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn provider_for_boolean_flag_matches_every_entry() {
        for provider in PROVIDERS_DISPLAY_ORDER {
            let first_alias = provider
                .cli_aliases()
                .first()
                .copied()
                .expect("every Provider exposes at least one cli alias");
            let flag = format!("--{first_alias}");
            assert_eq!(
                provider_for_boolean_flag(&flag),
                Some(provider),
                "flag {flag}"
            );
        }
    }

    #[test]
    fn provider_for_boolean_flag_rejects_unknown() {
        assert_eq!(provider_for_boolean_flag("--claud"), None);
        assert_eq!(provider_for_boolean_flag("--anthropic"), None);
        assert_eq!(provider_for_boolean_flag("--plain"), None);
    }

    #[test]
    fn is_fuzzy_provider_value_accepts_typical_tokens() {
        assert!(is_fuzzy_provider_value("cl"));
        assert!(is_fuzzy_provider_value("claude"));
        assert!(is_fuzzy_provider_value("open_code"));
    }

    #[test]
    fn is_fuzzy_provider_value_rejects_empty_and_flag_like() {
        assert!(!is_fuzzy_provider_value(""));
        assert!(!is_fuzzy_provider_value("-x"));
        assert!(!is_fuzzy_provider_value("--x"));
    }

    // ── Rule 3: `--` insertion for interleaved setters ───────────────

    #[test]
    fn rule_3_inserts_separator_before_help_after_flag() {
        // Headline case: Rule 4 hoists `--help` to position 1 so the root
        // help handler fires, then Rule 3 inserts `--` before the setter.
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
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_inserts_separator_on_inline_compose() {
        let input = argv(&[
            "claudine",
            "inline-compose",
            "file.md",
            "--gemini",
            "k=v",
            "--help",
        ]);
        let expected = argv(&[
            "claudine",
            "--help",
            "inline-compose",
            "file.md",
            "--provider",
            "gemini",
            "--",
            "k=v",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_inserts_separator_on_sequence() {
        let input = argv(&[
            "claudine", "sequence", "file.md", "--gemini", "k=v", "--help",
        ]);
        let expected = argv(&[
            "claudine",
            "--help",
            "sequence",
            "file.md",
            "--provider",
            "gemini",
            "--",
            "k=v",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_does_not_fire_without_flag_between_positional_and_setter() {
        let input = argv(&["claudine", "compose", "file.md", "key=val"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_does_not_fire_when_setter_precedes_positional() {
        let input = argv(&["claudine", "compose", "key=val", "file.md"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_does_not_fire_for_non_setter_trailing_token() {
        // `other.md` is not a setter; the second-file clap error is the
        // intended pre-existing behavior.
        let input = argv(&["claudine", "compose", "file.md", "--gemini", "other.md"]);
        let expected = argv(&[
            "claudine",
            "compose",
            "file.md",
            "--provider",
            "gemini",
            "other.md",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_is_noop_when_dash_dash_already_present() {
        // `--gemini` is before the user-provided `--`, so Rule 1 still
        // rewrites it. Rule 3 must not insert a second `--`.
        let input = argv(&[
            "claudine",
            "compose",
            "file.md",
            "--gemini",
            "--",
            "name=Ken",
        ]);
        let expected = argv(&[
            "claudine",
            "compose",
            "file.md",
            "--provider",
            "gemini",
            "--",
            "name=Ken",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_is_noop_when_user_provided_dash_dash_precedes_setter() {
        // Pure Rule-3 no-op: a `--` already brackets the setter, so the
        // normalizer must not rewrite anything.
        let input = argv(&[
            "claudine",
            "compose",
            "file.md",
            "--provider",
            "gemini",
            "--",
            "name=Ken",
        ]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_does_not_fire_on_non_composition_subcommand() {
        // Since Rule 1, Rule 3, and Rule 4 are all gated to composition
        // subcommands, a `hooks` argv with a provider boolean and trailing
        // `--help` must pass through unchanged.
        let input = argv(&["claudine", "hooks", "file.md", "--gemini", "k=v", "--help"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_does_not_fire_without_positional() {
        // No positional means Rule 3 doesn't fire, but Rule 4 still hoists
        // `--help` to position 1 so the root help handler renders.
        let input = argv(&["claudine", "compose", "--help"]);
        let expected = argv(&["claudine", "--help", "compose"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_handles_root_globals_before_subcommand() {
        // Real-tree regression case from the phase plan: root `--plain` must
        // not stop Rule 3 from firing on `compose`. `--help` is hoisted to
        // position 1 by Rule 4 (ahead of any user-supplied root globals).
        let input = argv(&[
            "claudine", "--plain", "compose", "file.md", "--gemini", "name=Ken", "--help",
        ]);
        let expected = argv(&[
            "claudine",
            "--help",
            "--plain",
            "compose",
            "file.md",
            "--provider",
            "gemini",
            "--",
            "name=Ken",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_skips_value_of_flag_with_value() {
        // `--model gpt-4` consumes its value; `gpt-4` must not be treated as
        // a positional, so Rule 3 should not fire.
        let input = argv(&["claudine", "compose", "file.md", "--model", "gpt-4"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_fires_after_short_flag_with_value() {
        // `-m gpt-4` consumes its value; the setter afterward still warrants
        // the separator. `--help` is hoisted by Rule 4.
        let input = argv(&[
            "claudine", "compose", "file.md", "-m", "gpt-4", "k=v", "--help",
        ]);
        let expected = argv(&[
            "claudine", "--help", "compose", "file.md", "-m", "gpt-4", "--", "k=v",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_fires_after_equals_form_flag() {
        // `--provider=gemini` is a single token; Rule 3 must still treat it
        // as an interleaved flag. `--help` is hoisted by Rule 4.
        let input = argv(&[
            "claudine",
            "compose",
            "file.md",
            "--provider=gemini",
            "k=v",
            "--help",
        ]);
        let expected = argv(&[
            "claudine",
            "--help",
            "compose",
            "file.md",
            "--provider=gemini",
            "--",
            "k=v",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_inserts_at_first_qualifying_setter_only() {
        // Two setters after the flag; `--` must land only before the first.
        // `--help` is hoisted by Rule 4.
        let input = argv(&[
            "claudine", "compose", "file.md", "--gemini", "a=1", "b=2", "--help",
        ]);
        let expected = argv(&[
            "claudine",
            "--help",
            "compose",
            "file.md",
            "--provider",
            "gemini",
            "--",
            "a=1",
            "b=2",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_ignores_setter_before_positional() {
        // Leading setter does not count as a positional; the later setter
        // after the flag fires Rule 3 because a real positional was seen in
        // between. `--help` is hoisted by Rule 4.
        let input = argv(&[
            "claudine",
            "compose",
            "k=early",
            "file.md",
            "--gemini",
            "k=late",
            "--help",
        ]);
        let expected = argv(&[
            "claudine",
            "--help",
            "compose",
            "k=early",
            "file.md",
            "--provider",
            "gemini",
            "--",
            "k=late",
        ]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_3_ignores_boolean_flag_before_positional() {
        // `--yolo` before the positional should not flip the "flag after
        // positional" flag, so no insertion.
        let input = argv(&["claudine", "compose", "--yolo", "file.md", "k=v"]);
        assert_eq!(normalize(input.clone()), input);
    }

    // ── Rule 3 helpers ────────────────────────────────────────────────

    #[test]
    fn looks_like_setter_matches_parse_compose_setter_pattern() {
        assert!(looks_like_setter("key=val"));
        assert!(looks_like_setter("_private=true"));
        assert!(looks_like_setter("my-key=value"));
        assert!(looks_like_setter("key="));
    }

    #[test]
    fn looks_like_setter_rejects_non_setter_tokens() {
        assert!(!looks_like_setter("file.md"));
        assert!(!looks_like_setter("compose"));
        assert!(!looks_like_setter("--gemini"));
        assert!(!looks_like_setter("=foo"));
        assert!(!looks_like_setter("9key=val"));
        assert!(!looks_like_setter("foo.bar=baz"));
        assert!(!looks_like_setter("/path=val"));
        assert!(!looks_like_setter(""));
    }

    #[test]
    fn is_composition_flag_with_value_matches_known_flags() {
        for flag in [
            "--provider",
            "--exclude",
            "--include",
            "--model",
            "-m",
            "--output",
            "-o",
            "--append-system-prompt",
            "--asp",
            "--replace-system-prompt",
            "--rsp",
            "--timeout",
            "-t",
            "--operation",
            "--op",
            "--set",
            "--use",
            "--fail-fast",
        ] {
            assert!(
                is_composition_flag_with_value(flag),
                "expected flag {flag} to be value-bearing"
            );
        }
    }

    #[test]
    fn is_composition_flag_with_value_rejects_equals_and_unknown() {
        assert!(!is_composition_flag_with_value("--provider=gemini"));
        assert!(!is_composition_flag_with_value("--yolo"));
        assert!(!is_composition_flag_with_value("--plain"));
        assert!(!is_composition_flag_with_value("file.md"));
    }

    // ── Rule 4: `--help` / `-h` hoisting for composition subcommands ─

    #[test]
    fn rule_4_hoists_help_to_position_one_on_composition() {
        let input = argv(&["claudine", "compose", "file.md", "--help"]);
        let expected = argv(&["claudine", "--help", "compose", "file.md"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_4_hoists_short_help_flag() {
        let input = argv(&["claudine", "compose", "file.md", "-h"]);
        let expected = argv(&["claudine", "-h", "compose", "file.md"]);
        assert_eq!(normalize(input), expected);
    }

    #[test]
    fn rule_4_is_idempotent_when_help_is_already_at_position_one() {
        let input = argv(&["claudine", "--help", "compose", "file.md"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_4_does_not_fire_on_wrapper_subcommand() {
        // Wrappers forward `--help` to the child CLI; the normalizer must
        // not hoist it or clap would short-circuit before the child runs.
        let input = argv(&["claudine", "claude", "--help"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_4_does_not_fire_on_non_composition_subcommand() {
        let input = argv(&["claudine", "hooks", "--describe", "--help"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_4_does_not_touch_help_after_user_dash_dash() {
        // A user-provided `--` ends the rule window; `--help` beyond it is
        // a trailing raw value and must stay put.
        let input = argv(&[
            "claudine", "compose", "file.md", "--", "--help",
        ]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_4_hoists_only_the_first_help_token() {
        // A duplicate `--help` later in argv is left alone — clap already
        // tolerates a boolean flag appearing twice.
        let input = argv(&["claudine", "compose", "--help", "file.md", "--help"]);
        let expected = argv(&["claudine", "--help", "compose", "file.md", "--help"]);
        assert_eq!(normalize(input), expected);
    }

    // ── Gap 3: wrapper safety — Rule 1 is gated on composition ───────

    #[test]
    fn rule_1_does_not_rewrite_provider_boolean_on_wrapper_subcommand() {
        // A user typing `claudine claude --gemini file.md` is sending
        // `--gemini` to the child CLI (the wrapper forwards unknown args).
        // Rule 1 must leave the token alone, otherwise the child sees
        // `--provider gemini` instead of `--gemini`.
        let input = argv(&["claudine", "claude", "--gemini", "file.md"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_1_does_not_rewrite_on_unknown_subcommand() {
        let input = argv(&["claudine", "unknown-subcommand", "--gemini", "file.md"]);
        assert_eq!(normalize(input.clone()), input);
    }

    #[test]
    fn rule_1_does_not_rewrite_on_hooks_subcommand() {
        let input = argv(&["claudine", "hooks", "--gemini"]);
        assert_eq!(normalize(input.clone()), input);
    }

    // ── Gap 5: parametric coverage for Rule 3 value-bearing flags ────

    #[test]
    fn rule_3_skips_value_for_every_composition_flag_with_value() {
        // For each value-bearing composition flag, construct
        // `claudine compose file.md <flag> VAL k=v` and assert Rule 3 does
        // not fire (the flag consumes `VAL`, so no positional-after-flag
        // sequence appears before the setter in a way that should trip the
        // rule). This locks the `COMPOSITION_FLAGS_WITH_VALUE` contract so
        // adding a new value-bearing flag without also adding it to the
        // constant becomes a test failure instead of a latent bug.
        let cases: &[(&str, &str)] = &[
            ("--provider", "claude"),
            ("--exclude", "claude"),
            ("--include", "FOO"),
            ("--model", "gpt-4"),
            ("-m", "gpt-4"),
            ("--output", "json"),
            ("-o", "json"),
            ("--append-system-prompt", "prompt.md"),
            ("--asp", "prompt.md"),
            ("--replace-system-prompt", "prompt.md"),
            ("--rsp", "prompt.md"),
            ("--timeout", "30"),
            ("-t", "30"),
            ("--operation", "ship"),
            ("--op", "ship"),
            ("--set", "{\"k\":\"v\"}"),
            ("--use", "id1,id2"),
            ("--fail-fast", "true"),
        ];
        for (flag, value) in cases {
            let input = argv(&[
                "claudine",
                "compose",
                "file.md",
                flag,
                value,
                "k=v",
            ]);
            // No `--help`, so Rule 4 is a no-op. Rule 3 should fire here
            // (positional `file.md`, interleaved flag+value, setter
            // `k=v`) and insert a `--` before the setter — except when
            // `flag` is `--provider`, in which case Rule 2 also rewrites
            // nothing because `claude` is already canonical.
            let result = normalize(input);
            assert!(
                result.contains(&OsString::from("--")),
                "flag {flag} with value {value}: Rule 3 must insert `--`, \
                 got result = {result:?}"
            );
            assert!(
                result.contains(&OsString::from(*flag)) || *flag == "--provider",
                "flag {flag}: token must survive normalization verbatim"
            );
        }
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

    // ── Gap 6: drift detection between clap surface and the value-table ──

    #[test]
    fn composition_flags_with_value_matches_clap_surface() {
        use crate::commands::compose::ComposeArgs;
        use crate::commands::sequence::SequenceArgs;

        fn build_cmd<A: clap::Args>() -> clap::Command {
            A::augment_args(clap::Command::new("test"))
        }

        fn collect_value_flags(cmd: &clap::Command) -> Vec<String> {
            use clap::ArgAction;
            let mut out = Vec::new();
            for arg in cmd.get_arguments() {
                // Skip boolean-style args: they do not consume the next
                // argv token, so Rule 3 must NOT skip past them.
                if matches!(
                    arg.get_action(),
                    ArgAction::SetTrue | ArgAction::SetFalse | ArgAction::Count | ArgAction::Help
                    | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
                ) {
                    continue;
                }
                // Only flag-shaped args contribute to the value-flag surface;
                // positional args like compose's `args: Vec<String>` have no
                // long/short name.
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
            out
        }

        let mut clap_flags: Vec<String> = Vec::new();
        clap_flags.extend(collect_value_flags(&build_cmd::<ComposeArgs>()));
        clap_flags.extend(collect_value_flags(&build_cmd::<SequenceArgs>()));
        clap_flags.sort();
        clap_flags.dedup();

        assert!(
            !clap_flags.is_empty(),
            "expected the clap surface to expose at least one value-bearing \
             flag; the test-wiring must be broken"
        );

        for flag in &clap_flags {
            // `--help` / `-h` are not value-bearing on clap's side; skip
            // any stray entries just in case.
            if flag == "--help" || flag == "-h" {
                continue;
            }
            assert!(
                COMPOSITION_FLAGS_WITH_VALUE.contains(&flag.as_str()),
                "clap surface exposes value-bearing flag `{flag}` that is \
                 missing from `COMPOSITION_FLAGS_WITH_VALUE`; add it there \
                 so Rule 3 keeps skipping its value correctly"
            );
        }
    }
}
