//! Body interpolation compose stage.

use super::super::super::Markdown;
use super::super::super::types::{AuthoredSpan, MarkdownError, MarkdownResult};
use super::super::context::effective_state as state;
use super::super::body_origin::{BodyOrigin, BodyProvenance, DataRanges};
use super::super::directive_targets::{DirectiveTargetRewrite, rewrite_directive_targets};
use super::super::interpolation;
use super::super::shell_expansion;
use super::super::expression::{
    BindingView, EvaluationLookup, ExpressionError, ResolutionContext, ResolvedBinding,
};
use super::super::context::report::CandidateLocus;
use super::super::expression::absence::MissingRoot;
use super::super::pipeline::{FinalVerdict, blocking_guard};
use super::super::schema_validation::PreparedSchemas;
use super::super::ComposeWarning;
use super::super::{ComposeReport, EffectiveState};
use serde_json::Value;
use tracing::debug;

/// Runs the interpolation stage.
///
/// Finds `{{ expression }}` patterns in content and evaluates them against the
/// effective state. Inline code spans (single backticks) are always scanned.
/// Fenced and indented code blocks are skipped by default; set
/// `interpolate_code_blocks` (via options or frontmatter) to scan them too.
///
/// The body is scanned once: text an expression or literal inserts, like any
/// data an earlier stage inserted, is never scanned again.
///
/// A span that cannot be parsed or evaluated fails the stage regardless of
/// `ComposeOptions::fail_fast`, and the body is left unrewritten. Only the
/// shell-command discovery pass defers such failures to warnings. The failure
/// is anchored to its authored span when `body.origin` (the map from the
/// current body back to the loaded text) proves one.
///
/// Under a blocking policy, a warning or undeclared read an evaluated
/// expression reported stops the stage before the next expression is
/// evaluated ([`MarkdownError::BlockedDiagnostics`]): a later one may compose
/// nested content that runs a shell. Spans are evaluated end to start, so
/// "next" is the previous span in the document.
///
/// ## Returns
///
/// The number of interpolations applied.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &crate::markdown::compose::ComposeRequest,
    runtime: &shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
    body: &mut BodyProvenance,
    schemas: &PreparedSchemas,
) -> MarkdownResult<usize> {
    use interpolation::{Evaluator, ScanMode, interpolate_text_guarded};

    let scan_mode = if resolve_interpolate_code_blocks(markdown, options) {
        ScanMode::Plain
    } else {
        ScanMode::MarkdownAware
    };

    // Wrap the effective state with a resolution context so read-side
    // expression functions (`frontmatter`, `file_exists`, `markdown_title`, …)
    // resolve filesystem paths and — when remote reads are enabled — HTTP(S)
    // URL arguments through the run's remote-fetch runtime.
    let lookup = DeferrableLookup {
        inner: state::ResolvingLookup::new(
            state,
            options.expression_resolution_context(&runtime.remote_fetch),
        ),
        defer_not_captured: options.defer_missing_runtime_context,
    };
    let evaluator = Evaluator::new(&lookup)
        .with_presentation_values(state.presentation_values())
        .observing_missing_roots();
    body.data.ensure_describes(markdown.content())?;
    let origin = body.origin.as_ref().filter(|origin| origin.describes(markdown.content()));
    let verdict = FinalVerdict { markdown, state, schemas };
    let targets = rewrite_targets_guarded(markdown, &body.data, &evaluator, origin, report, options, verdict)?;
    report.add_warnings(targets.warnings);
    // An absent property in a whole-value directive target already warns as a
    // skipped nullable target; a second warning would report one issue twice.
    drop(evaluator.take_missing_roots());
    let origin = origin.map(|origin| origin.after_edits(&targets.edits, &targets.output));
    let data = body.data.after_edits(&targets.edits, &targets.output);
    let authored = markdown.loaded_source_context_for_errors();
    let locate = |root: &MissingRoot| {
        root.span
            .as_ref()
            .zip(authored.as_ref())
            .and_then(|(span, context)| {
                origin
                    .as_ref()
                    .and_then(|origin| origin.project(span))
                    .and_then(|range| AuthoredSpan::locate(&context.content, range))
                    .map(|authored| authored.line())
                    .or_else(|| {
                        super::super::unknown_identifiers::unique_authored_line(
                            &context.content,
                            targets.output.get(span.clone())?,
                        )
                    })
            })
            .map_or(CandidateLocus::Document, CandidateLocus::Line)
    };
    let missing_roots = |mark| {
        let roots = evaluator.missing_roots_since(mark);
        roots.iter().map(|root| (root.root.clone(), locate(root))).collect()
    };
    let mut guard =
        blocking_guard(report, options, Some(verdict), Some((evaluator.missing_root_mark(), &missing_roots)));
    let result = interpolate_text_guarded(
        &targets.output,
        Some(&data),
        &evaluator,
        scan_mode,
        options.expression_failure_policy(),
        "interpolation",
        guard.as_deref_mut().map(|guard| guard as &mut dyn FnMut(&[ComposeWarning]) -> MarkdownResult<()>),
    )
    .map_err(|failure| anchor_authored_failure(markdown, origin.as_ref(), failure))?;
    drop(guard);

    let data = data.after_edits(&result.edits, &result.output);
    body.origin = origin.as_ref().map(|origin| origin.after_edits(&result.edits, &result.output));
    body.authored = body.authored.take().and_then(|authored| {
        authored
            .advanced(markdown.content(), &targets.edits, &targets.output)?
            .advanced(&targets.output, &result.edits, &result.output)
    });
    body.data = data;
    *markdown.content_mut() = result.output;
    report.add_warnings(result.warnings);
    // The scanner evaluates spans end to start; restore document order so the
    // first authored read of a root is the one kept.
    let mut roots = evaluator.take_missing_roots();
    roots.sort_by_key(|root| root.span.as_ref().map_or(usize::MAX, |span| span.start));
    report.add_unknown_root_candidates("interpolation", roots, locate);
    let replacements = targets.replacements + result.replacements;
    debug!(
        count = replacements,
        "compose: interpolations applied"
    );
    Ok(replacements)
}

/// The directive-target pass of [`run_stage`], checking the blocking policy
/// before each target.
///
/// Out of line so its guard and failure temporaries stay out of
/// `run_stage`'s frame, which recurs once per nested composition.
#[inline(never)]
fn rewrite_targets_guarded<L: EvaluationLookup>(
    markdown: &Markdown,
    data: &DataRanges,
    evaluator: &interpolation::Evaluator<'_, L>,
    origin: Option<&BodyOrigin>,
    report: &ComposeReport,
    options: &crate::markdown::compose::ComposeRequest,
    verdict: FinalVerdict<'_>,
) -> MarkdownResult<DirectiveTargetRewrite> {
    // A target's absent root is reported as a skipped target, never as an
    // undeclared read, so this pass judges warnings alone.
    let mut guard = blocking_guard(report, options, Some(verdict), None);
    rewrite_directive_targets(
        markdown.content(),
        Some(data),
        evaluator,
        markdown.frontmatter_line_count(),
        guard.as_deref_mut().map(|guard| guard as &mut dyn FnMut(&[ComposeWarning]) -> MarkdownResult<()>),
    )
    .map_err(|failure| anchor_authored_failure(markdown, origin, failure))
}

/// Anchors a body failure to its authored span when `origin` proves one.
///
/// `origin` maps the text the failing span indexes back to the loaded
/// document. An expression inside text an earlier stage inserted (an authored
/// replacement value) has no origin and keeps its file-only presentation
/// instead of a guessed position.
fn anchor_authored_failure(
    markdown: &Markdown,
    origin: Option<&BodyOrigin>,
    failure: interpolation::LocatedInterpolationError,
) -> MarkdownError {
    let interpolation::LocatedInterpolationError { error, span } = failure;
    let error = *error;
    let Some(context) = markdown.loaded_source_context_for_errors() else {
        return error;
    };
    match span
        .zip(origin)
        .and_then(|(span, origin)| origin.project(&span))
        .and_then(|range| AuthoredSpan::locate(&context.content, range))
    {
        Some(span) => error.with_authored_span(context, span),
        None => error,
    }
}

/// [`ResolvingLookup`](state::ResolvingLookup) that can answer an uncaptured
/// group with `null` (see `ComposeOptions::defer_missing_runtime_context`).
///
/// A projection-invariant failure is never deferred: it is a Darkmatter bug,
/// not a verdict a later pass could reach differently.
struct DeferrableLookup<'a> {
    inner: state::ResolvingLookup<'a>,
    defer_not_captured: bool,
}

/// A [`DeferrableLookup`] for the lookup parity inventory
/// (`compose::tests::lookup_parity`).
#[cfg(test)]
pub(crate) fn deferrable_lookup_for_tests<'a>(
    state: &'a crate::markdown::compose::EffectiveState,
    resolution_context: ResolutionContext,
) -> impl EvaluationLookup + 'a {
    DeferrableLookup {
        inner: state::ResolvingLookup::new(state, resolution_context),
        defer_not_captured: true,
    }
}

impl EvaluationLookup for DeferrableLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        self.inner.get(path)
    }

    fn resolve(&self, path: &str) -> Result<ResolvedBinding, ExpressionError> {
        match self.inner.resolve(path) {
            Err(ExpressionError::ContextNotCaptured { .. }) if self.defer_not_captured => {
                Ok(ResolvedBinding::classify(path, None))
            }
            other => other,
        }
    }

    fn binding_view(&self) -> Option<&BindingView> {
        self.inner.binding_view()
    }

    fn format_resolved(&self, path: &str, value: &Value) -> String {
        self.inner.format_resolved(path, value)
    }

    fn get_string(&self, path: &str) -> String {
        self.inner.get_string(path)
    }

    fn resolution_context(&self) -> Option<ResolutionContext> {
        self.inner.resolution_context()
    }

    fn resolution_context_ref(&self) -> Option<&ResolutionContext> {
        self.inner.resolution_context_ref()
    }

    fn is_valid_context_variable(&self, name: &str) -> bool {
        self.inner.is_valid_context_variable(name)
    }

    fn context_variable_names(&self) -> &[&'static str] {
        self.inner.context_variable_names()
    }

    fn begin_expression_scope(&self) {
        self.inner.begin_expression_scope();
    }
}

/// Resolves whether interpolation should process fenced/indented code blocks.
///
/// Inline code spans are always interpolated; this only governs fenced and
/// indented code blocks.
///
/// Checks (in priority order):
/// 1. `ComposeOptions::interpolate_code_blocks`
/// 2. Frontmatter `interpolate_code_blocks` key
pub(crate) fn resolve_interpolate_code_blocks(markdown: &Markdown, options: &crate::markdown::compose::ComposeRequest) -> bool {
    if options.interpolate_code_blocks {
        return true;
    }

    if let Ok(Some(value)) = markdown.fm_get::<bool>("interpolate_code_blocks") {
        return value;
    }

    false
}
