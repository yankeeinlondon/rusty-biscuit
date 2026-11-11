//! Dynamic shell completion for Claudine's CLI surface.
//!
//! This module serves two distinct completion paths:
//!
//! 1. **`claudine __complete`** (every generated script) — the hidden
//!    subcommand that generated completion scripts shell out to on every
//!    `<TAB>`. Routing flows through [`engine::run`], which classifies
//!    the cursor slot and dispatches to a slot-specific completer
//!    ([`root_menu`] for the subcommand menu, [`composition`] for the
//!    composition positional, [`setter_value`] for `@`-gated file
//!    completion inside `name=value` setters). Any slot the new engine
//!    does not recognize (wrapper flag values, administrative
//!    subcommands, etc.) emits zero candidates so the shell's native
//!    file / flag completion takes over.
//!
//! 2. **`CompleteEnv`** — the legacy `COMPLETE=<shell> claudine`
//!    bootstrap that activates at process startup via [`maybe_complete`].
//!    No generated script uses it; it stays alive for shell profiles that
//!    still carry an old `COMPLETE=<shell>` line. It delegates completion candidates
//!    to whatever `clap_complete` derives from the [`crate::args::Cli`]
//!    command tree, with `ignore_errors(true)` applied to the wrapper
//!    subcommands so unknown passthrough tokens do not crash the walk.
//!    No composition-specific candidates are produced through this path.

pub(crate) mod autocomplete_ui;
pub(crate) mod bootstrap;
pub(crate) mod composition;
pub(crate) mod default_glob;
pub(crate) mod engine;
pub(crate) mod frontmatter;
pub(crate) mod fuzzy;
pub(crate) mod operation_file;
pub(crate) mod root_menu;
pub(crate) mod schema_completion;
pub(crate) mod scopes;
pub(crate) mod setter_value;
pub(crate) mod walker;

use clap::CommandFactory;
use clap_complete::CompleteEnv;

use crate::argv;

/// Intercept `COMPLETE=<shell> claudine` startup and exit.
///
/// ## Notes
///
/// - Must be called before `argv::normalize(...)` so the normalizer's
///   `COMPLETE` guard is never exercised on the happy completion path.
/// - Must be called before any stdout output — clap_complete reserves the
///   completion stdout channel.
/// - Only shell profiles carrying an old `COMPLETE=<shell>` line still rely
///   on this path; every generated script shells out to
///   `claudine __complete` and dispatches through [`engine::run`].
pub(crate) fn maybe_complete() {
    CompleteEnv::with_factory(completion_command).complete();
}

/// Assemble the command tree used by the legacy `CompleteEnv` path.
///
/// The only adjustment to the derived tree is `ignore_errors(true)` on
/// each wrapper subcommand, which mirrors the lenient wrapper parse in
/// [`crate::parse_cli_from`]. Without it, a legacy `COMPLETE=<shell>`
/// completion walk over `claudine claude <unknown provider flag>` would
/// short-circuit on the unknown token.
fn completion_command() -> clap::Command {
    let mut cmd = <crate::args::Cli as CommandFactory>::command();
    for name in argv::WRAPPER_SUBCOMMANDS {
        if let Some(sub) = cmd.find_subcommand_mut(*name) {
            let muted = std::mem::replace(sub, clap::Command::new("__placeholder__"));
            let _ = std::mem::replace(sub, muted.ignore_errors(true));
        }
    }
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_command_retains_wrapper_subcommands_with_ignore_errors() {
        // The legacy CompleteEnv tree must still expose wrapper subcommands
        // so `COMPLETE=<shell> claudine claude …` does not crash on unknown
        // provider-specific tokens. `ignore_errors(true)` is the same
        // escape hatch the production lenient parse uses.
        let cmd = completion_command();
        for name in argv::WRAPPER_SUBCOMMANDS {
            assert!(
                cmd.find_subcommand(*name).is_some(),
                "wrapper subcommand `{name}` missing from completion tree",
            );
        }
    }

    #[test]
    fn completion_command_includes_composition_subcommands() {
        let cmd = completion_command();
        assert!(cmd.find_subcommand("compose").is_some());
        assert!(cmd.find_subcommand("inline-compose").is_some());
        assert!(cmd.find_subcommand("sequence").is_some());
    }

    /// The legacy tree exposes every catalog-derived provider shortcut, the
    /// same surface Rule 1 of the argv normalizer accepts, on every
    /// composition child, and carries none of `main.rs`'s injected
    /// `ArgAction::Help` arguments (the root's `disable_help_flag` reaches
    /// the children, so the tree has no help argument at all).
    #[test]
    fn completion_command_exposes_provider_shortcuts_without_injected_help() {
        use claudine::provider::PROVIDERS_DISPLAY_ORDER;

        let expected: Vec<&str> = PROVIDERS_DISPLAY_ORDER
            .iter()
            .filter_map(|provider| provider.cli_aliases().first().copied())
            .collect();
        assert_eq!(expected.len(), PROVIDERS_DISPLAY_ORDER.len());
        for shortcut in ["kilo", "pi", "antigravity"] {
            assert!(expected.contains(&shortcut), "{shortcut} not derived: {expected:?}");
        }

        let mut cmd = completion_command();
        // clap_complete builds the tree before walking it; check what it sees.
        cmd.build();
        for name in ["compose", "inline-compose", "sequence"] {
            let child = cmd
                .find_subcommand(name)
                .unwrap_or_else(|| panic!("`{name}` missing from completion tree"));
            for shortcut in &expected {
                assert!(
                    child.get_arguments().any(|arg| arg.get_long() == Some(*shortcut)),
                    "`{name}` lacks `--{shortcut}` in the completion tree",
                );
            }
            let help_args: Vec<&str> = child
                .get_arguments()
                .filter(|arg| {
                    matches!(arg.get_action(), clap::ArgAction::Help)
                        || arg.get_long() == Some("help")
                        || arg.get_short() == Some('h')
                })
                .map(|arg| arg.get_id().as_str())
                .collect();
            assert!(
                help_args.is_empty(),
                "`{name}` carries help arguments in the completion tree: {help_args:?}",
            );
        }
    }
}
