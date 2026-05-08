//! Rule 3: `--` insertion for interleaved setters on composition subcommands.
//!
//! Pulls late composition flags ahead of the first shorthand setter, then
//! inserts a single `--` separator before the first positional setter that
//! follows an interleaved flag.

use std::ffi::OsString;

use crate::argv::{
    COMPOSITION_SUBCOMMANDS, as_utf8, find_subcommand, first_dash_dash_index,
    is_composition_flag_with_value, looks_like_flag, looks_like_setter,
};

/// Apply the first half of Rule 3: when composition flags appear after the
/// first shorthand setter, move those flags back ahead of that setter so
/// later flags do not become opaque trailing raw values once the `--`
/// separator is inserted.
///
/// The rewrite preserves token order within the moved-flag segment and only
/// consumes a following value slot for known composition flags whose value
/// lives in the next argv token.
pub(crate) fn pull_late_composition_flags(argv: Vec<OsString>) -> Vec<OsString> {
    if first_dash_dash_index(&argv).is_some() {
        return argv;
    }
    let Some((sub_idx, _)) = find_subcommand(&argv, COMPOSITION_SUBCOMMANDS) else {
        return argv;
    };

    let first_setter_idx = (sub_idx + 1..argv.len())
        .find(|idx| as_utf8(&argv[*idx]).map(looks_like_setter).unwrap_or(false));
    let Some(first_setter_idx) = first_setter_idx else {
        return argv;
    };

    let mut before_setter: Vec<OsString> = argv[..first_setter_idx].to_vec();
    let mut trailing_non_flags: Vec<OsString> = Vec::new();
    let mut moved_flags: Vec<OsString> = Vec::new();

    let mut cursor = first_setter_idx;
    while cursor < argv.len() {
        let token = &argv[cursor];
        let Some(text) = as_utf8(token) else {
            trailing_non_flags.push(token.clone());
            cursor += 1;
            continue;
        };

        if is_composition_flag_with_value(text) {
            moved_flags.push(token.clone());
            if let Some(value) = argv.get(cursor + 1) {
                moved_flags.push(value.clone());
                cursor += 2;
            } else {
                cursor += 1;
            }
            continue;
        }

        if looks_like_flag(text) {
            moved_flags.push(token.clone());
            cursor += 1;
            continue;
        }

        trailing_non_flags.push(token.clone());
        cursor += 1;
    }

    if moved_flags.is_empty() {
        return argv;
    }

    before_setter.extend(moved_flags);
    before_setter.extend(trailing_non_flags);
    before_setter
}

/// Apply the second half of Rule 3: insert a single `--` before the first
/// setter-shaped token on `compose`, `inline-compose`, or `sequence` that
/// follows an interleaved flag after a previously seen positional.
///
/// No-ops when:
///
/// - The argv already contains a `--` separator.
/// - The argv does not target a composition subcommand.
/// - No positional was seen before the first setter-shaped token.
/// - No flag interleaved between that positional and the setter.
/// - The subcommand token is preceded by a non-flag, non-matching token
///   (i.e. an unknown subcommand owns position 1; clap handles it).
pub(crate) fn apply_composition_separator(argv: Vec<OsString>) -> Vec<OsString> {
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
            // Non-UTF-8 tokens are pattern-invisible. Rule 3 leaves them
            // opaque: the state machine does not advance (so Rule 3 will
            // fire if the rest of argv warrants it), but the token itself
            // is never matched as a positional or flag. On Unix, a user
            // with non-UTF-8 filenames may hit this path — we surface a
            // `debug!` so the bypass is diagnosable without cluttering
            // normal logs.
            tracing::debug!(
                cursor,
                "argv::normalize: skipping non-UTF-8 token; Rule 3 state machine did not advance",
            );
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::OsString;

    fn argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

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
        assert_eq!(crate::argv::normalize(input), expected);
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
        assert_eq!(crate::argv::normalize(input), expected);
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
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_3_does_not_fire_without_flag_between_positional_and_setter() {
        let input = argv(&["claudine", "compose", "file.md", "key=val"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_does_not_fire_when_setter_precedes_positional() {
        let input = argv(&["claudine", "compose", "key=val", "file.md"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
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
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_3_is_noop_when_dash_dash_already_present() {
        // `--gemini` is before the user-provided `--`, so Rule 1 still
        // rewrites it. Rule 3 must not insert a second `--`.
        let input = argv(&[
            "claudine", "compose", "file.md", "--gemini", "--", "name=Ken",
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
        assert_eq!(crate::argv::normalize(input), expected);
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
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_does_not_fire_on_non_composition_subcommand() {
        // Since Rule 1, Rule 3, and Rule 4 are all gated to composition
        // subcommands, a `hooks` argv with a provider boolean and trailing
        // `--help` must pass through unchanged.
        let input = argv(&["claudine", "hooks", "file.md", "--gemini", "k=v", "--help"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_does_not_fire_without_positional() {
        // No positional means Rule 3 doesn't fire, but Rule 4 still hoists
        // `--help` to position 1 so the root help handler renders.
        let input = argv(&["claudine", "compose", "--help"]);
        let expected = argv(&["claudine", "--help", "compose"]);
        assert_eq!(crate::argv::normalize(input), expected);
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
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_3_skips_value_of_flag_with_value() {
        // `--model gpt-4` consumes its value; `gpt-4` must not be treated as
        // a positional, so Rule 3 should not fire.
        let input = argv(&["claudine", "compose", "file.md", "--model", "gpt-4"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
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
        assert_eq!(crate::argv::normalize(input), expected);
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
        assert_eq!(crate::argv::normalize(input), expected);
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
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_3_ignores_setter_before_positional() {
        // Leading setter does not count as a positional on its own. With
        // `pull_late_composition_flags` in the pipeline, `--provider gemini`
        // is pulled back ahead of the first setter (`k=early`), which also
        // means no flag ever trails the real positional (`file.md`). The
        // `--` separator therefore does not need to fire — clap collects
        // `k=early`, `file.md`, and `k=late` together as the compose
        // positional `Vec<String>`. `--help` is hoisted by Rule 4.
        let input = argv(&[
            "claudine", "compose", "k=early", "file.md", "--gemini", "k=late", "--help",
        ]);
        let expected = argv(&[
            "claudine",
            "--help",
            "compose",
            "--provider",
            "gemini",
            "k=early",
            "file.md",
            "k=late",
        ]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_3_ignores_boolean_flag_before_positional() {
        // `--yolo` before the positional should not flip the "flag after
        // positional" flag, so no insertion.
        let input = argv(&["claudine", "compose", "--yolo", "file.md", "k=v"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_3_pulls_late_boolean_flags_ahead_of_first_setter() {
        let input = argv(&[
            "claudine",
            "compose",
            "file.md",
            "--gemini",
            "doc=@pkg/spec.md",
            "-y",
            "-i",
        ]);
        let expected = argv(&[
            "claudine",
            "compose",
            "file.md",
            "--provider",
            "gemini",
            "-y",
            "-i",
            "--",
            "doc=@pkg/spec.md",
        ]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_3_pulls_late_flags_when_setter_comes_before_them() {
        let input = argv(&[
            "claudine",
            "compose",
            "file.md",
            "doc=@pkg/spec.md",
            "-y",
            "--model",
            "gpt-5",
        ]);
        let expected = argv(&[
            "claudine",
            "compose",
            "file.md",
            "-y",
            "--model",
            "gpt-5",
            "--",
            "doc=@pkg/spec.md",
        ]);
        assert_eq!(crate::argv::normalize(input), expected);
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
        // Every clap-derived value-bearing composition flag must be
        // recognized by the runtime classifier. Sourcing the list from
        // the same derivation prevents drift between the two.
        for flag in crate::argv::collect_composition_value_flags() {
            assert!(
                is_composition_flag_with_value(&flag),
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
}
