//! Body interpolation compose stage.

use super::super::super::Markdown;
use super::super::super::types::{AuthoredSpan, MarkdownError, MarkdownResult};
use super::super::context::effective_state as state;
use super::super::body_origin::{BodyOrigin, BodyProvenance, DataRanges, PendingRead, PendingReads, ScopeRanges};
use super::super::directive_targets::{DirectiveTargetRewrite, rewrite_directive_targets};
use super::super::interpolation;
use super::super::shell_expansion;
use super::super::expression::{
    BindingView, EvaluationLookup, ExpressionError, ResolutionContext, ResolvedBinding,
};
use super::super::context::report::CandidateLocus;
use super::super::expression::absence::MissingRoot;
use super::super::member_sites::MemberFrame;
use super::super::pipeline::{FinalVerdict, blocking_guard};
use super::super::schema_validation::PreparedSchemas;
use super::super::ComposeWarning;
use super::super::site_scope::{SiteHook, SiteLookup};
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
/// current body back to the loaded text) proves one, and a failure inside a
/// repeated `::map` / `::filter` copy keeps that copy's items
/// ([`MarkdownError::InIteration`]).
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
    use interpolation::{Evaluator, ScanMode};

    let scan_mode = if resolve_interpolate_code_blocks(markdown, options) {
        ScanMode::Plain
    } else {
        ScanMode::MarkdownAware
    };

    // Wrap the effective state with a resolution context so read-side
    // expression functions (`frontmatter`, `file_exists`, `markdown_title`, …)
    // resolve filesystem paths and — when remote reads are enabled — HTTP(S)
    // URL arguments through the run's remote-fetch runtime.
    let document = DeferrableLookup {
        inner: state::ResolvingLookup::new(
            state,
            options.expression_resolution_context(&runtime.remote_fetch),
        ),
        defer_not_captured: options.defer_missing_runtime_context,
    };
    body.data.ensure_describes(markdown.content())?;
    let origin = body.origin.as_ref().filter(|origin| origin.describes(markdown.content()));
    let verdict = FinalVerdict { markdown, state, schemas };
    // A read after a command that has not run yet waits for it: the shell
    // stages evaluate it once the commands before it have run.
    let target_floor = first_command_end(markdown, markdown.content(), &body.data, options);
    let deferred_targets = std::cell::RefCell::new(Vec::new());
    let defer_target = |span: std::ops::Range<usize>, source: Option<&str>| {
        let late = is_late_read(target_floor, &span, source);
        if late {
            deferred_targets.borrow_mut().push(PendingRead { range: span, target: true });
        }
        !late
    };
    // A repeated body's sites read the frame of the copy they lie in; the
    // text the target pass leaves has shifted, so the second pass reads the
    // ranges carried past its edits.
    let targets = {
        let sited = SiteLookup::new(&document, &body.scopes, Some(&runtime.iteration));
        let evaluator = Evaluator::new(&sited)
            .with_presentation_values(state.presentation_values())
            .observing_missing_roots();
        let targets = rewrite_targets_guarded(
            markdown,
            &body.data,
            &evaluator,
            origin,
            &body.scopes,
            report,
            options,
            verdict,
            Some(&sited),
            target_floor.map(|_| &defer_target as _),
        )?;
        // An absent property in a whole-value directive target already warns as a
        // skipped nullable target; a second warning would report one issue twice.
        drop(evaluator.take_missing_roots());
        targets
    };
    report.add_warnings(targets.warnings);
    let scopes = body.scopes.after_edits(&targets.edits);
    let origin = origin.map(|origin| origin.after_edits(&targets.edits, &targets.output));
    let data = body.data.after_edits(&targets.edits, &targets.output);
    let mut waiting_targets = PendingReads::new(deferred_targets.into_inner()).after_edits(&targets.edits);
    let floor = first_command_end(markdown, &targets.output, &data, options);
    let deferred = std::cell::RefCell::new(Vec::new());
    let defer = |span: std::ops::Range<usize>, source: Option<&str>| {
        // A waiting target's expression is the target's to evaluate.
        if waiting_targets.intersects(&span) {
            return false;
        }
        let late = is_late_read(floor, &span, source);
        if late {
            deferred.borrow_mut().push(PendingRead { range: span, target: false });
        }
        !late
    };
    let select: interpolation::rewrite::SpanSelection<'_> = floor.map(|_| &defer as _);
    let result = rewrite_body(
        BodyRewrite { markdown, state, schemas, options, document: &document, runtime, scan_mode },
        &targets.output,
        &data,
        origin.as_ref(),
        &scopes,
        report,
        select,
    )?;

    let data = data.after_edits(&result.edits, &result.output);
    body.origin = origin.as_ref().map(|origin| origin.after_edits(&result.edits, &result.output));
    body.authored = body.authored.take().and_then(|authored| {
        authored
            .advanced(markdown.content(), &targets.edits, &targets.output)?
            .advanced(&targets.output, &result.edits, &result.output)
    });
    body.scopes = scopes.after_edits(&result.edits);
    let mut pending = deferred.into_inner();
    pending.extend(waiting_targets.take(|_| true));
    body.pending = PendingReads::new(pending).after_edits(&result.edits);
    body.data = data;
    *markdown.content_mut() = result.output;
    let replacements = targets.replacements + result.replacements;
    debug!(
        count = replacements,
        "compose: interpolations applied"
    );
    Ok(replacements)
}

/// What one body rewrite reads: the document, its state, and how to scan.
struct BodyRewrite<'a, L> {
    markdown: &'a Markdown,
    state: &'a EffectiveState,
    schemas: &'a PreparedSchemas,
    options: &'a crate::markdown::compose::ComposeRequest,
    document: &'a L,
    runtime: &'a shell_expansion::types::PipelineRuntime,
    scan_mode: interpolation::ScanMode,
}

/// Rewrites the expressions of `input` (the current body, or the text the
/// target pass left of it) that `select` accepts, reporting their warnings
/// and undeclared reads to `report`. Every site reads the frame of the
/// repeated copy (`scopes`) it lies in; `origin` anchors a failure to its
/// authored span.
fn rewrite_body<L: EvaluationLookup>(
    rewrite: BodyRewrite<'_, L>,
    input: &str,
    data: &DataRanges,
    origin: Option<&BodyOrigin>,
    scopes: &ScopeRanges,
    report: &mut ComposeReport,
    select: interpolation::rewrite::SpanSelection<'_>,
) -> MarkdownResult<interpolation::rewrite::InterpolationRewrite> {
    let BodyRewrite { markdown, state, schemas, options, document, runtime, scan_mode } = rewrite;
    let verdict = FinalVerdict { markdown, state, schemas };
    let lookup = SiteLookup::new(document, scopes, Some(&runtime.iteration));
    let mut evaluator = interpolation::Evaluator::new(&lookup)
        .with_presentation_values(state.presentation_values())
        .observing_missing_roots();
    // A discovery pass reports no member reads; a body with no map to its
    // authored text names no site.
    if let Some(origin) = origin.filter(|_| !options.defer_expression_failures) {
        evaluator = evaluator.with_member_sites(report.member_candidates.clone(), MemberFrame::Body(origin));
    }
    let authored = markdown.loaded_source_context_for_errors();
    let locate = |root: &MissingRoot| {
        root.span
            .as_ref()
            .zip(authored.as_ref())
            .and_then(|(span, context)| {
                origin
                    .and_then(|origin| origin.project(span))
                    .and_then(|range| AuthoredSpan::locate(&context.content, range))
                    .map(|authored| authored.line())
                    .or_else(|| {
                        super::super::unknown_identifiers::unique_authored_line(
                            &context.content,
                            input.get(span.clone())?,
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
    let mut result = interpolation::interpolate_text_selected(
        input,
        Some(data),
        &evaluator,
        scan_mode,
        options.expression_failure_policy(),
        "interpolation",
        guard.as_deref_mut().map(|guard| guard as &mut dyn FnMut(&[ComposeWarning]) -> MarkdownResult<()>),
        Some(&lookup),
        select,
    )
    .map_err(|failure| anchor_authored_failure(markdown, origin, scopes, failure))?;
    drop(guard);
    report.add_warnings(std::mem::take(&mut result.warnings));
    // The scanner evaluates spans end to start; restore document order so the
    // first authored read of a root is the one kept.
    let mut roots = evaluator.take_missing_roots();
    roots.sort_by_key(|root| root.span.as_ref().map_or(usize::MAX, |span| span.start));
    report.add_unknown_root_candidates("interpolation", roots, locate);
    Ok(result)
}

/// Evaluates the reads body interpolation left pending (see
/// [`BodyProvenance::pending`]) that `ready` accepts, in the current body.
///
/// A shell stage calls this once every command before a read has run, so the
/// read sees what those commands wrote; the data memo was already emptied as
/// each of them ran. The evaluation is body interpolation's own: the same
/// failure policy, blocking checks, warnings, and iteration frames.
///
/// ## Returns
///
/// The number of interpolations applied.
#[allow(clippy::too_many_arguments)]
pub(crate) fn resolve_pending(
    markdown: &mut Markdown,
    state: &EffectiveState,
    options: &crate::markdown::compose::ComposeRequest,
    runtime: &shell_expansion::types::PipelineRuntime,
    report: &mut ComposeReport,
    body: &mut BodyProvenance,
    schemas: &PreparedSchemas,
    ready: impl Fn(&std::ops::Range<usize>) -> bool,
) -> MarkdownResult<usize> {
    let selected = body.pending.take(ready);
    if selected.is_empty() {
        return Ok(0);
    }
    body.data.ensure_describes(markdown.content())?;
    let scan_mode = if resolve_interpolate_code_blocks(markdown, options) {
        interpolation::ScanMode::Plain
    } else {
        interpolation::ScanMode::MarkdownAware
    };
    let document = DeferrableLookup {
        inner: state::ResolvingLookup::new(state, options.expression_resolution_context(&runtime.remote_fetch)),
        defer_not_captured: options.defer_missing_runtime_context,
    };
    let (targets, plain): (Vec<_>, Vec<_>) = selected.into_iter().partition(|read| read.target);
    let mut plain = PendingReads::new(plain);
    let mut replacements = 0;
    if !targets.is_empty() {
        let chosen = |span: std::ops::Range<usize>, _: Option<&str>| targets.iter().any(|read| read.range == span);
        let origin = body.origin.clone().filter(|origin| origin.describes(markdown.content()));
        let rewrite = {
            let sited = SiteLookup::new(&document, &body.scopes, Some(&runtime.iteration));
            let evaluator = interpolation::Evaluator::new(&sited)
                .with_presentation_values(state.presentation_values())
                .observing_missing_roots();
            let rewrite = rewrite_targets_guarded(
                markdown,
                &body.data,
                &evaluator,
                origin.as_ref(),
                &body.scopes,
                report,
                options,
                FinalVerdict { markdown, state, schemas },
                Some(&sited),
                Some(&chosen),
            )
            .map_err(|error| error.with_on_disk_source(&markdown.full_source_context_for_errors()))?;
            // As in `run_stage`: an absent target root warns as a skipped target.
            drop(evaluator.take_missing_roots());
            rewrite
        };
        report.add_warnings(rewrite.warnings);
        replacements += rewrite.replacements;
        plain = plain.after_edits(&rewrite.edits);
        let before = std::mem::replace(markdown.content_mut(), rewrite.output);
        body.advance(&before, &rewrite.edits, markdown.content())?;
    }
    let plain = plain.take(|_| true);
    if !plain.is_empty() {
        let chosen = |span: std::ops::Range<usize>, source: Option<&str>| {
            source.is_some() && plain.iter().any(|read| read.range == span)
        };
        let origin = body.origin.clone().filter(|origin| origin.describes(markdown.content()));
        let result = rewrite_body(
            BodyRewrite { markdown, state, schemas, options, document: &document, runtime, scan_mode },
            markdown.content(),
            &body.data,
            origin.as_ref(),
            &body.scopes,
            report,
            Some(&chosen),
        )
        .map_err(|error| error.with_on_disk_source(&markdown.full_source_context_for_errors()))?;
        replacements += result.replacements;
        let before = std::mem::replace(markdown.content_mut(), result.output);
        body.advance(&before, &result.edits, markdown.content())?;
    }
    report.interpolations_applied += replacements;
    Ok(replacements)
}

/// The end of the first command a later stage runs in `content` (the
/// current text of `markdown`'s body), when it holds an expression: a read
/// that starts there or later waits for that command.
fn first_command_end(
    markdown: &Markdown,
    content: &str,
    data: &DataRanges,
    options: &crate::markdown::compose::ComposeRequest,
) -> Option<usize> {
    if !content.contains("{{") {
        return None;
    }
    super::shell_expansion::command_sites(markdown, content, data, options)
        .first()
        .map(|site| site.span.end)
}

/// Whether the expression `source` at `span` reads a file (calls a
/// read-side function) and starts at or after `floor`.
fn is_late_read(floor: Option<usize>, span: &std::ops::Range<usize>, source: Option<&str>) -> bool {
    floor.is_some_and(|floor| span.start >= floor)
        && source
            .and_then(|source| interpolation::parse(source).ok())
            .is_some_and(|expr| super::super::remote::calls_read_side_function(&expr))
}

/// The directive-target pass of [`run_stage`], checking the blocking policy
/// before each target.
///
/// Out of line so its guard and failure temporaries stay out of
/// `run_stage`'s frame, which recurs once per nested composition.
#[inline(never)]
#[allow(clippy::too_many_arguments)]
fn rewrite_targets_guarded<L: EvaluationLookup>(
    markdown: &Markdown,
    data: &DataRanges,
    evaluator: &interpolation::Evaluator<'_, L>,
    origin: Option<&BodyOrigin>,
    ranges: &ScopeRanges,
    report: &ComposeReport,
    options: &crate::markdown::compose::ComposeRequest,
    verdict: FinalVerdict<'_>,
    site: Option<&dyn SiteHook>,
    select: interpolation::rewrite::SpanSelection<'_>,
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
        site,
        select,
    )
    .map_err(|failure| anchor_authored_failure(markdown, origin, ranges, failure))
}

/// Anchors a body failure to its authored span when `origin` proves one, and
/// gives a failure inside a repeated copy (`ranges`) that copy's items.
///
/// `origin` maps the text the failing span indexes back to the loaded
/// document. An expression inside text an earlier stage inserted (an authored
/// replacement value) has no origin and keeps its file-only presentation
/// instead of a guessed position.
fn anchor_authored_failure(
    markdown: &Markdown,
    origin: Option<&BodyOrigin>,
    ranges: &ScopeRanges,
    failure: interpolation::LocatedInterpolationError,
) -> MarkdownError {
    let interpolation::LocatedInterpolationError { error, span } = failure;
    let mut error = *error;
    if let Some(context) = markdown.loaded_source_context_for_errors()
        && let Some(authored) = span
            .clone()
            .zip(origin)
            .and_then(|(span, origin)| origin.project(&span))
            .and_then(|range| AuthoredSpan::locate(&context.content, range))
    {
        error = error.with_authored_span(context, authored);
    }
    match span {
        Some(span) => super::super::site_scope::scope_failure(error, markdown, origin, ranges, span),
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
