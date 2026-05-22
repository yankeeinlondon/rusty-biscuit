//! Composed fold-then-render result type for the experimental render-tree
//! pipeline.
//!
//! The tree pipeline has two phases that each produce non-fatal
//! [`Diagnostic`]s:
//!
//! 1. **Fold diagnostics** — raised by
//!    [`fold_markdown_to_document`](super::fold_markdown_to_document) during
//!    event-stream → [`renderable::tree::Document`] construction.
//! 2. **Render diagnostics** — raised by `renderable::tree::render_*_document`
//!    (or `biscuit_terminal::render_tree::render_terminal_document`) while
//!    lowering [`renderable::tree::NodeKind`] variants to a target.
//!
//! Merging the two into one `Vec<Diagnostic>` loses the origin: a
//! [`DiagnosticKind::Lossy`](renderable::tree::DiagnosticKind::Lossy) from the
//! fold is a tree-model limitation; one from the render boundary is a target
//! limitation. The composed entry points
//! (`render_tree::entrypoints::render_tree_*`) keep the two streams side by
//! side via [`PipelineResult`].
//!
//! See `renderable/features/2026-05-20-darkmatter-tree/diagnostic-model.md`.

use renderable::tree::{Diagnostic, RenderError};

/// Result of the fold-then-render pipeline.
///
/// Holds the rendered `output` and the two diagnostic streams unmerged so a
/// caller can tell whether a finding came from fold-time tree construction or
/// from render-time target lowering. Diagnostics keep phase-local ordering:
/// fold diagnostics appear in source/event order, render diagnostics appear in
/// renderer traversal order.
#[derive(Debug, Clone)]
pub struct PipelineResult<T> {
    /// The rendered output.
    pub output: T,
    /// Non-fatal diagnostics from the fold phase (parse/structural).
    pub fold_diagnostics: Vec<Diagnostic>,
    /// Non-fatal diagnostics from the render phase (lowering/target-loss).
    pub render_diagnostics: Vec<Diagnostic>,
}

impl<T> PipelineResult<T> {
    /// Builds a result from an `output` value and the two diagnostic vectors.
    #[must_use]
    pub fn new(
        output: T,
        fold_diagnostics: Vec<Diagnostic>,
        render_diagnostics: Vec<Diagnostic>,
    ) -> Self {
        Self {
            output,
            fold_diagnostics,
            render_diagnostics,
        }
    }

    /// Returns `true` when neither phase emitted any non-fatal diagnostic.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.fold_diagnostics.is_empty() && self.render_diagnostics.is_empty()
    }
}

/// Result alias for composed fold-then-render entry points.
///
/// Fatal render errors stay in the `Err` channel: a `RenderError::InvalidTree`
/// is never silently demoted to an in-band diagnostic. Fold-phase issues are
/// always non-fatal in this pipeline (the fold itself never errors).
pub type PipelineRenderResult<T> = Result<PipelineResult<T>, RenderError>;

#[cfg(test)]
mod tests {
    use super::*;
    use renderable::tree::{Diagnostic, Severity};

    #[test]
    fn is_clean_reports_true_when_both_streams_empty() {
        let result: PipelineResult<&str> = PipelineResult::new("ok", Vec::new(), Vec::new());
        assert!(result.is_clean());
    }

    #[test]
    fn is_clean_reports_false_when_fold_has_findings() {
        let result =
            PipelineResult::new("ok", vec![Diagnostic::structural("ouch", None)], Vec::new());
        assert!(!result.is_clean());
    }

    #[test]
    fn is_clean_reports_false_when_render_has_findings() {
        let result = PipelineResult::new(
            "ok",
            Vec::new(),
            vec![Diagnostic::validation(Severity::Warning, "warn", None)],
        );
        assert!(!result.is_clean());
    }

    #[test]
    fn pipeline_result_preserves_phase_local_ordering() {
        let fold = vec![
            Diagnostic::structural("a", None),
            Diagnostic::structural("b", None),
        ];
        let render = vec![Diagnostic::validation(Severity::Warning, "x", None)];
        let result = PipelineResult::new((), fold.clone(), render.clone());
        // The vectors are stored as-given, not merged or sorted.
        assert_eq!(result.fold_diagnostics.len(), 2);
        assert_eq!(result.render_diagnostics.len(), 1);
        assert_eq!(result.fold_diagnostics[0].message, "a");
        assert_eq!(result.fold_diagnostics[1].message, "b");
        assert_eq!(result.render_diagnostics[0].message, "x");
    }
}
