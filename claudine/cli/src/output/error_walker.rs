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

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use claudine::composition::lifecycle_context::LifecycleErrorInfo;
use claudine::composition::{CompositionError, FrontmatterExcerpt};
use color_eyre::eyre::Report;
use darkmatter::markdown::errors::as_block_error;
use std::error::Error as StdError;
use std::path::Path;

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

    // A frontmatter-enriched error wraps its real cause; render from the inner
    // error so the deepest typed block still wins, then append the captured
    // frontmatter excerpt after it.
    let wrapper = find_frontmatter_wrapper(root);
    let start: &(dyn StdError + 'static) = match wrapper {
        Some((inner, _)) => inner,
        None => root,
    };

    let deepest = deepest_block_error(start)?;
    let mut out = deepest.report_block_error(term);

    if let Some((_, excerpt)) = wrapper {
        let appendix = excerpt.render_appendix(term);
        if !appendix.is_empty() {
            out = out.trim_end_matches('\n').to_string();
            out.push_str(&appendix);
            out.push('\n');
        }
    }

    // Ensure plain-mode / NO_COLOR output contains no ANSI escape bytes even
    // when the deepest block error (e.g. MarkdownError) renders with styles.
    if matches!(term.color_depth, ColorDepth::None) {
        out = strip_escape_codes(&out);
    }

    Some(out)
}

/// Render a lifecycle evaluation error to stderr **at its catch point**,
/// before any catch events (`failure`/`finalize`) fire (Decision #2).
///
/// Builds the original [`CompositionError::lifecycle_evaluation`] block and
/// renders it through the same styled `BlockError` surface the outer CLI
/// renderer uses, so TTY-gating and `ColorDepth::None` escape-stripping are
/// shared (no hand-rolled ANSI). Returns the same error wrapped in
/// [`CompositionError::already_emitted`] so the caller can thread it on and the
/// outer renderer suppresses the duplicate styled block.
///
/// Emits exactly once per call; callers invoke it once, before running catch
/// events, so the original crash is visible ahead of any `finalize` output.
pub(crate) fn emit_lifecycle_evaluation_error_early(
    source_path: &Path,
    event: &str,
    info: &LifecycleErrorInfo,
    term: &Terminal,
) -> CompositionError {
    let error = CompositionError::lifecycle_evaluation(event, source_path, info);
    let rendered = error.report_block_error(term);
    crate::log::message("");
    crate::log::message(&rendered);
    crate::log::message("");
    error.already_emitted()
}

/// Render a lifecycle evaluation error's styled block to stderr at its catch
/// point and return it marked already-emitted (Decision #2).
///
/// Use at any catch/surface site that is about to return a
/// [`CompositionError::LifecycleEvaluationError`] to the run's outer renderer,
/// when no further lifecycle events fire afterwards (a raise inside the catch
/// `failure`/`finalize` event, or a loop-engine error consumed by the CLI). The
/// styled block goes through the same `BlockError` surface and TTY/`NO_COLOR`
/// gating as the outer renderer. Any error that is **not** an un-emitted
/// `LifecycleEvaluationError` (including one already marked) is returned
/// unchanged, so this is safe to apply uniformly.
pub(crate) fn emit_lifecycle_evaluation_error_block(
    error: CompositionError,
    term: &Terminal,
) -> CompositionError {
    if matches!(error, CompositionError::LifecycleEvaluationError { .. }) {
        let rendered = error.report_block_error(term);
        crate::log::message("");
        crate::log::message(&rendered);
        crate::log::message("");
        return error.already_emitted();
    }
    error
}

/// Whether the report's cause chain carries an already-emitted lifecycle
/// evaluation error, so the outer renderer must not re-render its styled block.
pub(crate) fn evaluation_error_already_emitted(report: &Report) -> bool {
    let mut current: Option<&(dyn StdError + 'static)> = Some(report.as_ref());
    while let Some(err) = current {
        if let Some(comp) = err.downcast_ref::<CompositionError>()
            && comp.is_already_emitted()
        {
            return true;
        }
        current = err.source();
    }
    false
}

/// Find the first [`CompositionError::WithFrontmatter`] in the cause chain,
/// returning its inner error and captured excerpt.
fn find_frontmatter_wrapper<'a>(
    root: &'a (dyn StdError + 'static),
) -> Option<(&'a CompositionError, &'a FrontmatterExcerpt)> {
    let mut current = Some(root);
    while let Some(err) = current {
        if let Some(CompositionError::WithFrontmatter { inner, excerpt }) =
            err.downcast_ref::<CompositionError>()
        {
            return Some((inner.as_ref(), excerpt));
        }
        current = err.source();
    }
    None
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
mod tests;
