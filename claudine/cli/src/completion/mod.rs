//! Dynamic shell completion for Claudine's CLI surface.
//!
//! This module serves two distinct completion paths:
//!
//! 1. **`claudine __complete`** (bash / zsh / fish) — the hidden
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
//! 2. **`CompleteEnv`** (powershell / elvish) — the legacy
//!    `COMPLETE=<shell> claudine` bootstrap that activates at process
//!    startup via [`maybe_complete`]. This path stays alive for shells
//!    the new engine does not target; it delegates completion candidates
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
/// - Only PowerShell and Elvish still rely on this path; bash / zsh / fish
///   shell out to `claudine __complete` and dispatch through [`engine::run`].
pub(crate) fn maybe_complete() {
    CompleteEnv::with_factory(completion_command).complete();
}

/// Assemble the command tree used by the legacy `CompleteEnv` path.
///
/// The only adjustment to the derived tree is `ignore_errors(true)` on
/// each wrapper subcommand, which mirrors the lenient wrapper parse in
/// [`crate::parse_cli_from`]. Without it, a PowerShell / Elvish
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
}
