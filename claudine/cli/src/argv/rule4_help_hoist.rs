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
