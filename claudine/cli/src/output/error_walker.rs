//! Top-level cause-chain walker that renders darkmatter `BlockError`
//! reports for any error produced by the CLI's subcommands.
//!
//! The walker iterates a [`color_eyre::Report`]'s cause chain from
//! outermost to innermost, calls
//! [`darkmatter::markdown::errors::as_block_error`] on each cause, and
//! renders the deepest typed match via its
//! [`BlockError::report_block_error`] impl.
//!
//! If no cause implements [`BlockError`], the caller falls back to
//! `color_eyre`'s default `Debug` output.

use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use claudine::composition::CompositionError;
use color_eyre::eyre::Report;
use darkmatter::markdown::errors::as_block_error;
use std::error::Error as StdError;

/// Try to render `report` as a darkmatter [`BlockError`] report.
///
/// Walks the report's cause chain and returns the rendered string for the
/// **deepest** cause that implements [`BlockError`]. Deepest is preferred
/// so wrappers (`ClaudineError`, `CompositionError`, `MarkdownError`, …)
/// do not shadow the richer leaf error's metadata.
///
/// Returns `None` when no cause implements [`BlockError`].
pub(crate) fn try_render_block_report(report: &Report, term: &Terminal) -> Option<String> {
    let root: &(dyn StdError + 'static) = report.as_ref();
    let deepest = deepest_block_error(root)?;
    Some(deepest.report_block_error(term))
}

fn deepest_block_error<'a>(
    err: &'a (dyn StdError + 'static),
) -> Option<&'a (dyn BlockError + 'static)> {
    let mut deepest = discover_block_error(err);
    let mut current = err.source();
    while let Some(next) = current {
        if let Some(found) = discover_block_error(next) {
            deepest = Some(found);
        }
        current = next.source();
    }
    deepest
}

fn discover_block_error<'a>(
    err: &'a (dyn StdError + 'static),
) -> Option<&'a (dyn BlockError + 'static)> {
    if let Some(found) = as_block_error(err) {
        return Some(found);
    }
    if let Some(v) = err.downcast_ref::<CompositionError>() {
        return Some(v);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_terminal::utils::escape_codes::strip_escape_codes;
    use color_eyre::eyre::eyre;
    use darkmatter::markdown::MarkdownError;
    use darkmatter::markdown::compose::{ShellCommandOrigin, ShellExpansionError};
    use std::path::PathBuf;

    fn width80() -> Terminal {
        Terminal::new_optimistic(80)
    }

    #[test]
    fn walks_past_wrapper_error_to_typed_inner() {
        let shell = ShellExpansionError::ApprovalRequired {
            command: "gh repo list".into(),
            whitelist_path: PathBuf::from("/tmp/wl"),
            blacklist_path: PathBuf::from("/tmp/bl"),
            origin: ShellCommandOrigin::Body { line: 7 },
        };
        let md: MarkdownError = shell.into();
        let cli_err = claudine::error::ClaudineError::SystemPromptComposition(md);
        let report: Report = eyre!(cli_err);

        let rendered = try_render_block_report(&report, &width80()).expect("block error found");
        let plain = strip_escape_codes(&rendered);
        assert!(plain.contains("approval required"), "got:\n{plain}");
        assert!(plain.contains("/tmp/wl"), "got:\n{plain}");
    }

    #[test]
    fn returns_none_for_untyped_error() {
        let report: Report = eyre!("bare non-block error");
        assert!(try_render_block_report(&report, &width80()).is_none());
    }

    #[test]
    fn renders_transclusion_cycle_chain() {
        use darkmatter::markdown::compose::TransclusionError;
        let inner = TransclusionError::CycleDetected {
            chain: vec![
                (PathBuf::from("a.md"), 3),
                (PathBuf::from("b.md"), 7),
                (PathBuf::from("a.md"), 3),
            ],
        };
        let md: MarkdownError = MarkdownError::Transclusion(Box::new(inner));
        let report: Report = eyre!(claudine::composition::CompositionError::ComposeFailed(md));

        let rendered = try_render_block_report(&report, &width80()).expect("block error found");
        let plain = strip_escape_codes(&rendered);
        assert!(plain.contains("a.md"), "got:\n{plain}");
        assert!(plain.contains("b.md"), "got:\n{plain}");
        assert!(plain.contains("cycle detected"), "got:\n{plain}");
    }
}
