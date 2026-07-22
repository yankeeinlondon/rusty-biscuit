//! Top-level cause-chain walker that renders a [`BlockError`] report for any
//! error produced by the CLI's subcommands.
//!
//! The walker resolves the **effective diagnostic** through Claudine's shared
//! [`select_effective_diagnostic`], which is also what builds
//! [`LifecycleErrorInfo`] and the machine-facing snapshot — so a route cannot
//! classify one cause while rendering another. Selection composes Claudine's
//! diagnostic registry with Darkmatter's `as_block_error`, so this module keeps
//! no downcast list of its own; a second, partial list here is exactly how
//! `ClaudineError` and `HarnessError` became unrenderable.
//!
//! If nothing in the chain is renderable, the caller falls back to
//! `color_eyre`'s default `Debug` output.

use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use claudine::composition::lifecycle_context::LifecycleErrorInfo;
use claudine::composition::{CompositionError, FrontmatterExcerpt};
use claudine::diagnostics::select_effective_diagnostic;
use color_eyre::eyre::Report;
use std::error::Error as StdError;
use std::path::Path;

/// Try to render `report` as a [`BlockError`] report.
///
/// Returns the rendered string for the diagnostic
/// [`select_effective_diagnostic`] chose to speak for the failure, with a
/// captured frontmatter excerpt appended when the chain carries one.
///
/// Returns `None` when nothing in the chain is renderable.
pub(crate) fn try_render_block_report(report: &Report, term: &Terminal) -> Option<String> {
    let root: &(dyn StdError + 'static) = report.as_ref();

    // Selection walks *through* a frontmatter wrapper on its own — the wrapper
    // is `Transparent`, so it never becomes the selected diagnostic. This
    // lookup is only for the excerpt it captured.
    let excerpt = find_frontmatter_excerpt(root);

    let selected = select_effective_diagnostic(root)?;
    let mut out = selected.block_error().report_block_error(term);

    if let Some(excerpt) = excerpt {
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

/// Find the excerpt captured by the first [`CompositionError::WithFrontmatter`]
/// in the cause chain.
///
/// Walks `Error::source` rather than `diagnostic_source` because the wrapper
/// may sit below an error that is not a `Diagnostic` at all.
fn find_frontmatter_excerpt<'a>(
    root: &'a (dyn StdError + 'static),
) -> Option<&'a FrontmatterExcerpt> {
    let mut current = Some(root);
    while let Some(err) = current {
        if let Some(CompositionError::WithFrontmatter { excerpt, .. }) =
            err.downcast_ref::<CompositionError>()
        {
            return Some(excerpt);
        }
        current = err.source();
    }
    None
}

#[cfg(test)]
mod tests;
