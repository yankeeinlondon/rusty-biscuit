//! Rule 4: `--help` / `-h` hoisting for composition subcommands.
//!
//! Hoists a trailing `--help` or `-h` token to argv position 1 so the root
//! help handler fires instead of being trapped inside clap's greedy
//! positional collector.

use std::ffi::OsString;

use crate::argv::{
    COMPOSITION_SUBCOMMANDS, find_subcommand, first_dash_dash_index,
};

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
