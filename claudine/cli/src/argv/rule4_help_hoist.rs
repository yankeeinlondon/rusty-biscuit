//! Rule 4: `--help` / `-h` hoisting for composition subcommands, and the
//! executable's earlier composition help selection.
//!
//! [`select_composition_help`] reads the original argv and decides which help
//! screen a request names, before normalization or the ownership partition can
//! lose the token's position or reject the invocation. The executable answers
//! a selected request there and never reaches the hoist.
//!
//! [`hoist_composition_help`] is the retained normalization rewrite: it moves a
//! trailing `--help` or `-h` to argv position 1 so a normalized argv never
//! traps the token inside clap's greedy positional collector. The completion
//! engine reuses normalization, so the rewrite stays a syntactic contract.

use std::ffi::OsString;

use crate::argv::{
    COMPOSITION_SUBCOMMANDS, find_subcommand, first_dash_dash_index, is_global_flag_with_value,
};

/// Which help screen an argv requests, decided by [`select_composition_help`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HelpDestination {
    /// The grouped root overview: help was requested before the command.
    Root,
    /// The named composition command's own clap help screen.
    Composition(&'static str),
}

/// Select the help screen a composition invocation requests, from the
/// original argv.
///
/// The command is recognized the way [`find_subcommand`] recognizes it (root
/// options and the value of `--debug` are skipped, the first command-shaped
/// token decides, and nothing after the first literal `--` counts). The first
/// exact standalone `-h`/`--help` before that `--` decides the destination:
/// before the command it is [`HelpDestination::Root`], after it the command's
/// own screen. An exact root `-V`/`--version` before the command, or before
/// any help token, leaves the argv to the ordinary parser.
///
/// Tokens are compared exactly and never interpreted, so an incomplete or
/// invalid option beside the help token cannot prevent help, while
/// `name=--help`, `--help=…`, and `-hh` are not help requests.
///
/// Returns `None` when completion is active, when no composition command is
/// recognized, or when no help token precedes the `--` boundary.
pub(crate) fn select_composition_help(
    raw: &[OsString],
    completion_active: bool,
) -> Option<HelpDestination> {
    if completion_active {
        return None;
    }
    let (command_index, command) = find_subcommand(raw, COMPOSITION_SUBCOMMANDS)?;
    let stop = first_dash_dash_index(raw).unwrap_or(raw.len());
    let mut cursor = 1;
    while cursor < stop {
        let Some(token) = raw[cursor].to_str() else {
            cursor += 1;
            continue;
        };
        let before_command = cursor < command_index;
        if before_command && is_global_flag_with_value(token) {
            // Mirror `find_subcommand`: the next token is this option's value.
            cursor += 2;
            continue;
        }
        if token == "-h" || token == "--help" {
            return Some(if before_command {
                HelpDestination::Root
            } else {
                HelpDestination::Composition(command)
            });
        }
        if before_command && (token == "-V" || token == "--version") {
            return None;
        }
        cursor += 1;
    }
    None
}

/// Apply Rule 4: hoist `-h`/`--help` on composition subcommands to argv
/// position 1.
///
/// The executable selects composition help earlier, from the original argv
/// ([`select_composition_help`]), so its normalized argv never carries a help
/// token before the `--` boundary. This rewrite remains the normalization
/// contract for other callers, notably the completion engine's ownership
/// classification: a trailing help token must not be read as a positional.
///
/// No-ops when:
///
/// - The argv does not target a composition subcommand.
/// - No `-h`/`--help` token is present between the subcommand and the
///   first literal `--`.
/// - An earlier pass already hoisted a help token to position 1.
pub(crate) fn hoist_composition_help(argv: Vec<OsString>) -> Vec<OsString> {
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    fn argv(tokens: &[&str]) -> Vec<OsString> {
        tokens.iter().map(OsString::from).collect()
    }

    use super::{HelpDestination, select_composition_help};

    fn select(tokens: &[&str]) -> Option<HelpDestination> {
        select_composition_help(&argv(tokens), false)
    }

    #[test]
    fn selection_follows_the_destination_table() {
        let compose = Some(HelpDestination::Composition("compose"));
        assert_eq!(select(&["claudine"]), None);
        assert_eq!(select(&["claudine", "--help"]), None);
        assert_eq!(select(&["claudine", "--help", "compose", "x.md"]), Some(HelpDestination::Root));
        assert_eq!(select(&["claudine", "compose", "--help"]), compose);
        assert_eq!(select(&["claudine", "compose", "x.md", "-h"]), compose);
        assert_eq!(
            select(&["claudine", "--plain", "inline-compose", "x.md", "--help"]),
            Some(HelpDestination::Composition("inline-compose"))
        );
        assert_eq!(
            select(&["claudine", "sequence", "--help", "--", "--help"]),
            Some(HelpDestination::Composition("sequence"))
        );
        assert_eq!(select(&["claudine", "compose", "x.md", "--", "--help"]), None);
    }

    #[test]
    fn selection_ignores_invalid_arguments_but_not_embedded_text() {
        let compose = Some(HelpDestination::Composition("compose"));
        assert_eq!(select(&["claudine", "compose", "--provider", "--help"]), compose);
        assert_eq!(select(&["claudine", "compose", "--set", "{broken", "-h"]), compose);
        assert_eq!(select(&["claudine", "compose", "x.md", "name=--help"]), None);
        assert_eq!(select(&["claudine", "compose", "x.md", "--help=yes", "-hh"]), None);
    }

    #[test]
    fn selection_defers_to_completion_version_and_other_commands() {
        let input = argv(&["claudine", "compose", "--help"]);
        assert_eq!(select_composition_help(&input, true), None);
        assert_eq!(select(&["claudine", "--version", "compose", "-h"]), None);
        assert_eq!(select(&["claudine", "mcp", "compose", "--help"]), None);
        assert_eq!(select(&["claudine", "__complete", "compose", "--help"]), None);
        assert_eq!(
            select(&["claudine", "--debug", "trace", "sequence", "-h"]),
            Some(HelpDestination::Composition("sequence"))
        );
    }

    /// One `(argv, destination)` row per case; argv[0] is the binary name.
    fn assert_rows(rows: &[(&[&str], Option<HelpDestination>)]) {
        for (tokens, expected) in rows {
            assert_eq!(select(tokens), *expected, "argv: {tokens:?}");
        }
    }

    /// A token that is not valid UTF-8 on this OS: an invalid byte on Unix,
    /// an unpaired surrogate on Windows.
    fn opaque_token() -> OsString {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(vec![0x66, 0x6f, 0xff, 0x6f])
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[0x66, 0xD800, 0x6f])
        }
    }

    #[test]
    fn selection_covers_every_destination_for_every_command() {
        for command in crate::argv::COMPOSITION_SUBCOMMANDS {
            let own = Some(HelpDestination::Composition(command));
            let root = Some(HelpDestination::Root);
            assert_rows(&[
                (&["claudine", command, "-h"], own),
                (&["claudine", command, "--help"], own),
                (&["claudine", command, "x.md", "-h"], own),
                (&["claudine", command, "x.md", "--help"], own),
                (&["claudine", command, "--claude", "x.md", "name=Ken", "--help"], own),
                (&["claudine", "--plain", command, "x.md", "--help"], own),
                (&["claudine", command, "x.md", "--plain", "-h"], own),
                (&["claudine", "-h", command, "x.md"], root),
                (&["claudine", "--help", command], root),
                (&["claudine", command, "-h", "--", "--help"], own),
                (&["claudine", command, "x.md", "--", "--help"], None),
                (&["claudine", command, "x.md", "--", "-h"], None),
                (&["claudine", command, "x.md"], None),
            ]);
        }
    }

    #[test]
    fn selection_skips_root_options_and_their_values() {
        let sequence = Some(HelpDestination::Composition("sequence"));
        assert_rows(&[
            (&["claudine", "--debug", "trace", "sequence", "-h"], sequence),
            (&["claudine", "--debug=trace", "sequence", "-h"], sequence),
            (&["claudine", "-v", "--plain", "sequence", "x.md", "--help"], sequence),
            (&["claudine", "--verbose", "--debug", "trace", "--help", "sequence"], Some(HelpDestination::Root)),
        ]);
    }

    #[test]
    fn selection_recognizes_only_the_first_command() {
        assert_rows(&[
            (&[], None),
            (&["claudine"], None),
            (&["claudine", "-h"], None),
            (&["claudine", "mcp", "compose", "--help"], None),
            (&["claudine", "claude", "compose", "--help"], None),
            (&["claudine", "skills", "sequence", "-h"], None),
            (&["claudine", "__complete", "inline-compose", "-h"], None),
            (&["claudine", "--", "compose", "--help"], None),
        ]);
    }

    #[test]
    fn selection_takes_the_first_of_repeated_help_tokens() {
        let compose = Some(HelpDestination::Composition("compose"));
        assert_rows(&[
            (&["claudine", "compose", "-h", "--help", "-h"], compose),
            (&["claudine", "compose", "--help", "x.md", "-h"], compose),
            (&["claudine", "--help", "compose", "-h"], Some(HelpDestination::Root)),
            (&["claudine", "-h", "--help", "compose", "x.md", "--help"], Some(HelpDestination::Root)),
        ]);
    }

    #[test]
    fn selection_treats_embedded_and_malformed_help_text_as_data() {
        assert_rows(&[
            (&["claudine", "compose", "x.md", "name=--help"], None),
            (&["claudine", "compose", "x.md", "name=-h"], None),
            (&["claudine", "compose", "--set", r#"{"a":"--help"}"#, "x.md"], None),
            (&["claudine", "compose", r#"--set={"a":"--help"}"#, "x.md"], None),
            (&["claudine", "compose", "x.md", "--help=yes"], None),
            (&["claudine", "compose", "x.md", "--help="], None),
            (&["claudine", "compose", "x.md", "-hh"], None),
            (&["claudine", "compose", "x.md", "-h=1"], None),
            (&["claudine", "compose", "x.md", "---help"], None),
            (&["claudine", "compose", "x.md", "--HELP"], None),
            (&["claudine", "compose", "x.md", "help"], None),
        ]);
    }

    #[test]
    fn selection_passes_over_opaque_tokens_without_panicking() {
        let compose = Some(HelpDestination::Composition("compose"));
        let with_opaque = |before: &[&str], after: &[&str]| {
            let mut tokens = argv(before);
            tokens.push(opaque_token());
            tokens.extend(argv(after));
            select_composition_help(&tokens, false)
        };
        assert_eq!(with_opaque(&["claudine", "compose"], &[]), None);
        assert_eq!(with_opaque(&["claudine", "compose"], &["--help"]), compose);
        assert_eq!(with_opaque(&["claudine", "compose", "x.md"], &["-h"]), compose);
        assert_eq!(with_opaque(&["claudine"], &["compose", "-h"]), compose);
        assert_eq!(with_opaque(&["claudine", "compose", "x.md", "--"], &["--help"]), None);
    }

    #[test]
    fn selection_yields_to_active_completion() {
        for tokens in [
            &["claudine", "compose", "--help"][..],
            &["claudine", "--help", "sequence"],
            &["claudine", "inline-compose", "x.md", "-h"],
        ] {
            let input = argv(tokens);
            assert!(select_composition_help(&input, false).is_some(), "{tokens:?}");
            assert_eq!(select_composition_help(&input, true), None, "{tokens:?}");
        }
    }

    #[test]
    fn selection_yields_to_a_root_version_request() {
        let compose = Some(HelpDestination::Composition("compose"));
        assert_rows(&[
            (&["claudine", "--version", "compose", "-h"], None),
            (&["claudine", "-V", "compose", "--help"], None),
            (&["claudine", "-V", "sequence", "x.md", "-h"], None),
            (&["claudine", "--help", "--version", "compose"], Some(HelpDestination::Root)),
            // After the command a version token is the command's argument,
            // not a root request.
            (&["claudine", "compose", "-V", "--help"], compose),
            (&["claudine", "compose", "x.md", "--version", "-h"], compose),
        ]);
    }

    #[test]
    fn rule_4_hoists_help_to_position_one_on_composition() {
        let input = argv(&["claudine", "compose", "file.md", "--help"]);
        let expected = argv(&["claudine", "--help", "compose", "file.md"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_4_hoists_short_help_flag() {
        let input = argv(&["claudine", "compose", "file.md", "-h"]);
        let expected = argv(&["claudine", "-h", "compose", "file.md"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }

    #[test]
    fn rule_4_is_idempotent_when_help_is_already_at_position_one() {
        let input = argv(&["claudine", "--help", "compose", "file.md"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_4_does_not_fire_on_wrapper_subcommand() {
        // Wrappers forward `--help` to the child CLI; the normalizer must
        // not hoist it or clap would short-circuit before the child runs.
        let input = argv(&["claudine", "claude", "--help"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_4_does_not_fire_on_non_composition_subcommand() {
        let input = argv(&["claudine", "hooks", "--describe", "--help"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_4_does_not_touch_help_after_user_dash_dash() {
        // A user-provided `--` ends the rule window; `--help` beyond it is
        // a trailing raw value and must stay put.
        let input = argv(&["claudine", "compose", "file.md", "--", "--help"]);
        assert_eq!(crate::argv::normalize(input.clone()), input);
    }

    #[test]
    fn rule_4_hoists_only_the_first_help_token() {
        // A duplicate `--help` later in argv is left alone — clap already
        // tolerates a boolean flag appearing twice.
        let input = argv(&["claudine", "compose", "--help", "file.md", "--help"]);
        let expected = argv(&["claudine", "--help", "compose", "file.md", "--help"]);
        assert_eq!(crate::argv::normalize(input), expected);
    }
}
