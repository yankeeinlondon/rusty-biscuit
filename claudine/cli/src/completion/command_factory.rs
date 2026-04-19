//! Builds the completion-aware `clap::Command` for `CompleteEnv`.
//!
//! The production command tree is the source of truth for argument shapes
//! (see [`crate::args::Cli`]), but completion needs two small adjustments
//! before `clap_complete` consumes it:
//!
//! 1. Wrapper subcommands must be marked `ignore_errors(true)` so that
//!    unknown tokens destined for the wrapped provider CLI do not short-
//!    circuit the completion walk. This mirrors the lenient wrapper parse
//!    in [`crate::parse_cli_from`].
//! 2. The shared `args` positional on `compose`, `inline-compose`, and
//!    `sequence` gets a mode-tagged [`ArgValueCompleter`] attached so that
//!    Phase 2 can plug in file-reference discovery per command.

use std::ffi::OsStr;

use clap::CommandFactory;
use clap_complete::{ArgValueCompleter, CompletionCandidate};

use crate::argv;
use crate::completion::file_reference;
use crate::completion::validate;

/// Discriminates per-command behavior for the completer attached to the
/// shared `args` positional. Phase 1 uses this only for a stable marker;
/// Phase 2 wires in token classification, scope discovery, and
/// mode-specific file validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ComposeMode {
    Compose,
    InlineCompose,
    Sequence,
}

impl ComposeMode {
    fn subcommand_name(self) -> &'static str {
        match self {
            Self::Compose => "compose",
            Self::InlineCompose => "inline-compose",
            Self::Sequence => "sequence",
        }
    }
}

/// Assemble the completion-aware command tree.
///
/// This entry point is handed to `CompleteEnv::with_factory` in
/// [`super::maybe_complete`]. Everything outside the wrapper and
/// composition subcommands is left untouched so non-target commands keep
/// whatever completion behavior clap derives by default.
pub(crate) fn completion_command() -> clap::Command {
    let mut cmd = <crate::args::Cli as CommandFactory>::command();

    for name in argv::WRAPPER_SUBCOMMANDS {
        if let Some(sub) = cmd.find_subcommand_mut(*name) {
            let muted = std::mem::replace(sub, clap::Command::new("__placeholder__"));
            let _ = std::mem::replace(sub, muted.ignore_errors(true));
        }
    }

    for mode in [
        ComposeMode::Compose,
        ComposeMode::InlineCompose,
        ComposeMode::Sequence,
    ] {
        attach_mode_completer(&mut cmd, mode);
    }

    cmd
}

fn attach_mode_completer(cmd: &mut clap::Command, mode: ComposeMode) {
    let Some(sub) = cmd.find_subcommand_mut(mode.subcommand_name()) else {
        return;
    };

    let completer = ArgValueCompleter::new(move |current: &OsStr| dispatch_complete(mode, current));

    let owned = std::mem::replace(sub, clap::Command::new("__placeholder__"));
    let rebuilt = owned.mut_arg("args", |arg| arg.add(completer));
    let _ = std::mem::replace(sub, rebuilt);
}

/// Dispatch a completion request into the Phase 2 discovery layer and apply
/// the Phase 3 per-mode validator.
///
/// Phase 2 owns token classification, scope discovery, and bounded directory
/// walking; Phase 3 narrows the walked set down to the files that actually
/// satisfy each composition mode's runtime contract:
///
/// - `compose` keeps every `.md` / `.markdown` file (no frontmatter parse).
/// - `inline-compose` keeps files with a non-empty string `prompt:` field.
/// - `sequence` keeps files whose frontmatter resolves to a real sequence
///   plan (inline list or external YAML reference).
///
/// Directories are always emitted as-is so a single `<TAB>` keeps descending
/// into a subtree. File validation is fail-closed — every I/O or parse error
/// silently drops the candidate rather than surfacing shell-visible noise.
fn dispatch_complete(mode: ComposeMode, current: &OsStr) -> Vec<CompletionCandidate> {
    let Some(partial) = current.to_str() else {
        return Vec::new();
    };
    let ctx = file_reference::CompletionRepoContext::discover();
    file_reference::discover_candidates(partial, &ctx)
        .into_iter()
        .filter(|entry| entry.is_dir || validate::is_valid_for_mode(&entry.resolved_path, mode))
        .map(|entry| CompletionCandidate::new(entry.value))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_command_has_compose_subcommand() {
        let cmd = completion_command();
        assert!(cmd.find_subcommand("compose").is_some());
        assert!(cmd.find_subcommand("inline-compose").is_some());
        assert!(cmd.find_subcommand("sequence").is_some());
    }

    #[test]
    fn completion_command_retains_wrapper_subcommands_with_ignore_errors() {
        // The completion command tree must still expose wrapper subcommands
        // so that `COMPLETE=bash claudine claude …` does not crash on
        // unknown provider-specific tokens. `ignore_errors(true)` is the
        // same escape hatch the production lenient parse uses.
        let cmd = completion_command();
        for name in argv::WRAPPER_SUBCOMMANDS {
            assert!(
                cmd.find_subcommand(*name).is_some(),
                "wrapper subcommand `{name}` missing from completion tree",
            );
        }
    }

    #[test]
    fn compose_mode_names_match_composition_subcommands() {
        assert_eq!(ComposeMode::Compose.subcommand_name(), "compose");
        assert_eq!(
            ComposeMode::InlineCompose.subcommand_name(),
            "inline-compose",
        );
        assert_eq!(ComposeMode::Sequence.subcommand_name(), "sequence");
    }
}
