//! Body interpolation compose stage.

use super::super::super::Markdown;
use super::super::super::types::{AuthoredSpan, MarkdownError, MarkdownResult};
use super::super::context::effective_state as state;
use super::super::body_origin::BodyOrigin;
use super::super::directive_targets::rewrite_directive_targets;
use super::super::interpolation;
use super::super::shell_expansion;
use super::super::expression::{EvaluationLookup, ExpressionError, ResolutionContext};
use super::super::context::report::CandidateLocus;
use super::super::{ComposeOptions, ComposeReport, EffectiveState};
use serde_json::Value;
use tracing::debug;

/// Runs the interpolation stage.
///
/// Finds `{{ expression }}` patterns in content and evaluates them against the
/// effective state. Inline code spans (single backticks) are always scanned.
/// Fenced and indented code blocks are skipped by default; set
/// `interpolate_code_blocks` (via options or frontmatter) to scan them too.
///
/// A span that cannot be parsed or evaluated fails the stage regardless of
/// `ComposeOptions::fail_fast`, and the body is left unrewritten. Only the
/// shell-command discovery pass defers such failures to warnings. The failure
/// is anchored to its authored span when `origin` (the map from the current
/// body back to the loaded text) proves one.
///
/// ## Returns
///
/// The number of interpolations applied.
pub(crate) fn run_stage(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &ComposeOptions,
    runtime: &shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
    origin: Option<&BodyOrigin>,
) -> MarkdownResult<usize> {
    use interpolation::{Evaluator, ScanMode, interpolate_text_located};

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
    let origin = origin.filter(|origin| origin.describes(markdown.content()));
    let targets = rewrite_directive_targets(
        markdown.content(),
        &evaluator,
        markdown.frontmatter_line_count(),
    )
    .map_err(|failure| anchor_authored_failure(markdown, origin, failure))?;
    report.add_warnings(targets.warnings);
    // An unknown root in a whole-value directive target already warns as a
    // skipped nullable target; a second warning would report one issue twice.
    drop(evaluator.take_missing_roots());
    let origin = origin.map(|origin| origin.after_edits(&targets.edits, &targets.output));
    let result = interpolate_text_located(
        &targets.output,
        &evaluator,
        scan_mode,
        options.expression_failure_policy(),
        "interpolation",
    )
    .map_err(|failure| anchor_authored_failure(markdown, origin.as_ref(), failure))?;

    if targets.replacements > 0 || result.replacements > 0 {
        *markdown.content_mut() = result.output;
    }
    report.add_warnings(result.warnings);
    let authored = markdown.loaded_source_context_for_errors();
    // The scanner evaluates spans end to start; restore document order so the
    // first authored read of a root is the one kept.
    let mut roots = evaluator.take_missing_roots();
    roots.sort_by_key(|root| root.span.as_ref().map_or(usize::MAX, |span| span.start));
    report.add_unknown_root_candidates("interpolation", roots, |root| {
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
    });
    let replacements = targets.replacements + result.replacements;
    debug!(
        count = replacements,
        "compose: interpolations applied"
    );
    Ok(replacements)
}

/// Anchors a body failure to its authored span when `origin` proves one.
///
/// `origin` maps the text the failing span indexes back to the loaded
/// document. A rescan-generated expression has no span, and one inside text an
/// earlier stage inserted has no origin; either keeps its file-only
/// presentation instead of a guessed position.
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

impl EvaluationLookup for DeferrableLookup<'_> {
    fn get(&self, path: &str) -> Option<Value> {
        self.inner.get(path)
    }

    fn get_checked(&self, path: &str) -> Result<Option<Value>, ExpressionError> {
        match self.inner.get_checked(path) {
            Err(ExpressionError::ContextNotCaptured { .. }) if self.defer_not_captured => Ok(None),
            other => other,
        }
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

    fn is_known_variable_root(&self, root: &str) -> bool {
        self.inner.is_known_variable_root(root)
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
fn resolve_interpolate_code_blocks(markdown: &Markdown, options: &ComposeOptions) -> bool {
    if options.interpolate_code_blocks {
        return true;
    }

    if let Ok(Some(value)) = markdown.fm_get::<bool>("interpolate_code_blocks") {
        return value;
    }

    false
}
