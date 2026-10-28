//! The composition-time call check: a document whose authored syntax makes a
//! call the design-time check proves wrong (category 5) is rejected before
//! any of its expressions is evaluated.
//!
//! The check is passive. It reads the document as prepared for composition,
//! its effective schema's declared types (never its frontmatter values), and
//! the enclosing `::block when` guards, and it runs no expression. It covers
//! the syntax the enabled stages would evaluate, in every branch, including
//! blocks whose condition is false:
//!
//! | Surface | Checked when | Not checked |
//! |---|---|---|
//! | frontmatter `{{ … }}` (every authored string leaf) | frontmatter interpolation is enabled | `exclude_keys`, data leaves (inherited or `--data`), literal tokens, `{{{ … }}}` |
//! | body `{{ … }}` | body interpolation is enabled | fenced/indented code (unless `interpolate_code_blocks`), `{{{ … }}}`, literal tokens |
//! | `::block when="…"` | page blocks are enabled | |
//! | `::file`/`::code` `when=…` | a transclusion stage is enabled | |
//! | `::map`/`::filter` gates and callbacks | collection blocks are enabled | |
//! | body surfaces inside a `::map`/`::filter` body | its own stage is enabled | |
//!
//! The shell-command discovery pass composes a narrowed surface, but its
//! check covers the surfaces of the composition it prepares, so it rejects
//! the same call with the same earlier warnings.
//!
//! The same walk collects every warning the shared call diagnostics show
//! without `expressions.strict` ([`CallCheck::diagnostics`]): an argument of
//! category 3 ([`UNCERTAIN_CALL_CODE`]), a call that is always `false`
//! because an authored literal argument is not a well-formed file reference
//! (`file_exists("https://")`, [`ALWAYS_FALSE_CALL_CODE`]), and a call to a
//! host function whose analysis is incomplete. Each warning carries the
//! shared record ([`ComposeWarning::diagnostic`]) located at its authored
//! argument, so composition and DMLS report the same fields.
//!
//! A body expression is checked under the guards of every enclosing
//! `::block` (when page blocks are enabled, the only case in which a false
//! block is removed) and the locals of every enclosing `::map` / `::filter`
//! body ([`BodyScopes`]), including bodies and branches no gate would reach:
//! a runtime gate never excuses a statically invalid call. A region's static
//! defects (source typing, protected names, patterns, sort keys) are
//! rejected once, before any expression runs, and a waived untyped source is
//! a warning the blocking policy sees with the others. A transcluded child runs its own pipeline node, so it is
//! checked against its own schema and origin before its own evaluation;
//! nothing a parent inserts is the parent's syntax.

use super::body_origin::BodyOrigin;
use super::conditions::ConditionError;
use super::context::report::{ExpressionOrigin, WarningIdentity, WarningSubject};
#[cfg(doc)]
use super::expression::{ALWAYS_FALSE_CALL_CODE, UNCERTAIN_CALL_CODE};
use super::expression::{
    CallCheck, DeclaredTypes, ExpressionDiagnostic, ExpressionError, ExpressionFinder, ExpressionLocation,
    IncompleteHostCall, SpannedExpr, TypeScope, check_expression_calls, incomplete_host_calls, one_based_position,
    parse_condition_spanned, parse_spanned,
};
use super::frontmatter_interpolation::{FrontmatterExpressionLocation, LocatedFrontmatterError};
use super::value_origin::ValuePathSegment as Segment;
use super::collection_blocks::{
    BodyScopes, CollectionAnalysisOptions, CollectionError, analyze_collection_blocks, diagnostics::COLLECTION_SOURCE,
};
use super::page_blocks::{PageBlockError, PageBlockRegion};
use super::pipeline::operations::ComposeOperationSet;
use super::value_origin::{FrontmatterProvenance, ValuePathSegment};
use super::{ComposeOperation, ComposeSource, ComposeWarning, transclusion};
use crate::markdown::Markdown;
use crate::markdown::schemas::EffectiveSchema;
use crate::markdown::types::{AuthoredSpan, MarkdownError, SourceRef};
use serde_json::Value;

/// A category 5 call the precheck found, or a policy stop.
///
/// A rejection carries `earlier`: the warnings the walk found before the
/// rejected call in authored order, which a failure report shows before it.
pub(crate) enum PrecheckFailure {
    /// In a frontmatter value; the pipeline attributes it like an
    /// interpolation failure (override supplier or authored span).
    Frontmatter {
        failure: LocatedFrontmatterError,
        earlier: Vec<ComposeWarning>,
    },
    /// In the body, already located.
    Body {
        error: Box<MarkdownError>,
        earlier: Vec<ComposeWarning>,
    },
    /// Warnings the request's blocking policy selects
    /// ([`ComposeOptions::with_blocking_policy`](super::ComposeOptions::with_blocking_policy)),
    /// with the precheck's other warnings.
    Blocked {
        blocking: Vec<ComposeWarning>,
        rest: Vec<ComposeWarning>,
    },
}

/// A rejected call before its warnings are attributed.
enum Rejection {
    Frontmatter(LocatedFrontmatterError),
    Body(MarkdownError),
}

/// Authored order of a finding: the frontmatter string leaf it is in, in
/// walk order ([`BODY_RANK`] for the body), then its byte offset in that
/// leaf or in the body.
type Order = (usize, usize);

/// The rank of every body finding: after every frontmatter leaf.
const BODY_RANK: usize = usize::MAX;

/// A shown warning of one call, where its scanned text holds it.
struct Advisory {
    order: Order,
    function: String,
    finding: ExpressionDiagnostic,
    /// The frontmatter key path whose string leaf holds it, and that string;
    /// `None` for the body.
    leaf: Option<(Vec<Segment>, String)>,
    /// The frontmatter key, for its line.
    key: Option<String>,
    /// Byte range of the finding in the scanned text.
    origin: std::ops::Range<usize>,
}

/// Warnings found so far, in scan order; [`advisory_warnings`] puts them in
/// authored order. The surfaces are scanned one after another, so a block
/// condition's warning is found before an earlier interpolation's.
#[derive(Default)]
struct Advisories {
    found: Vec<Advisory>,
    /// Waived untyped collection sources, ranked by their body offset.
    recovered: Vec<(Order, super::collection_blocks::RecoveredSource)>,
    /// Frontmatter string leaves ranked so far.
    leaves: usize,
}

impl Advisories {
    /// The rank of the next frontmatter string leaf scanned.
    fn next_leaf(&mut self) -> usize {
        self.leaves += 1;
        self.leaves - 1
    }

    /// Records every warning of `checks` and of `incomplete` calls, for an
    /// expression starting at byte `offset` of the scanned text ranked
    /// `rank`.
    #[allow(clippy::too_many_arguments)]
    fn record(
        &mut self,
        checks: &[CallCheck],
        incomplete: &[IncompleteHostCall],
        rank: usize,
        offset: usize,
        leaf: Option<(&[Segment], &str)>,
        key: Option<&str>,
    ) {
        for check in checks {
            for finding in check.diagnostics(false) {
                self.push(&check.function, finding, rank, offset, leaf, key);
            }
        }
        for call in incomplete {
            self.push(&call.function, call.diagnostic(), rank, offset, leaf, key);
        }
    }

    fn push(
        &mut self,
        function: &str,
        finding: ExpressionDiagnostic,
        rank: usize,
        offset: usize,
        leaf: Option<(&[Segment], &str)>,
        key: Option<&str>,
    ) {
        self.found.push(Advisory {
            order: (rank, offset + finding.span.start),
            function: function.to_string(),
            origin: offset + finding.span.start..offset + finding.span.end,
            finding,
            leaf: leaf.map(|(path, text)| (path.to_vec(), text.to_string())),
            key: key.map(str::to_string),
        });
    }
}

/// Byte offset of `location`'s trimmed expression text in `text`, the
/// scanned text holding its `{{ … }}`; the call check's spans are relative
/// to that expression.
fn expression_start(text: &str, location: &ExpressionLocation) -> usize {
    let inner = location.start + 2;
    inner
        + text
            .get(inner..location.end)
            .and_then(|construct| construct.find(location.expression.as_str()))
            .unwrap_or(0)
}

/// Checks every executable authored call of `markdown` on the surface
/// `options` composes, against the declared types of every layer of
/// `schema` ([`DeclaredTypes::for_schema`]).
///
/// Authored order is frontmatter first, leaf by leaf in key order, then the
/// body by offset, with interpolations and block and transclusion
/// conditions interleaved.
///
/// ## Returns
///
/// One warning per finding [`CallCheck::diagnostics`] shows without
/// `expressions.strict`, plus one per incomplete host call, in authored
/// order.
///
/// ## Errors
///
/// The first rejected call in authored order, carrying
/// [`ExpressionError::CallCheck`] and the warnings found before it in
/// authored order (including those earlier in its own expression; none
/// after it); or, when no call is rejected, the warnings the request's
/// blocking policy selects.
pub(crate) fn check_document(
    markdown: &Markdown,
    provenance: &FrontmatterProvenance,
    options: &super::ComposeRequest,
    schema: Option<&EffectiveSchema>,
) -> Result<Vec<ComposeWarning>, PrecheckFailure> {
    let declared = DeclaredTypes::for_schema(schema);
    // A declared host global is a declared root: a collection may iterate it,
    // and a local may not shadow it.
    let declared = match options.host_globals.as_ref() {
        Some(globals) => declared.with_binding_view(globals.view()),
        None => declared,
    };
    let site = TypeScope::unguarded(&declared);
    let mut advisories = Advisories::default();
    let mut rejected = None;
    if options.call_check_covers(ComposeOperation::FrontmatterInterpolation) {
        rejected = check_frontmatter(markdown, provenance, &options.exclude_keys, &site, &mut advisories)
            .map(|(order, failure)| (order, Rejection::Frontmatter(failure)));
    }
    if rejected.is_none() {
        rejected = check_body(markdown, options, &site, &mut advisories)
            .map(|(offset, error)| ((BODY_RANK, offset), Rejection::Body(error)));
    }
    if let Some((order, rejection)) = rejected {
        advisories.found.retain(|advisory| advisory.order < order);
        advisories.recovered.retain(|(at, _)| *at < order);
        let earlier = advisory_warnings(markdown, options, advisories);
        return Err(match rejection {
            Rejection::Frontmatter(failure) => PrecheckFailure::Frontmatter { failure, earlier },
            Rejection::Body(error) => PrecheckFailure::Body { error: Box::new(error), earlier },
        });
    }
    let warnings = advisory_warnings(markdown, options, advisories);
    let policy = options.blocking_policy();
    if policy.blocking(&warnings).is_empty() {
        return Ok(warnings);
    }
    let (blocking, rest) = warnings.into_iter().partition(|warning| policy.blocks_warning(warning));
    Err(PrecheckFailure::Blocked { blocking, rest })
}

fn advisory_warnings(markdown: &Markdown, options: &super::ComposeRequest, advisories: Advisories) -> Vec<ComposeWarning> {
    use biscuit_terminal::components::prose::Prose;
    if advisories.found.is_empty() && advisories.recovered.is_empty() {
        return Vec::new();
    }
    let recovered = advisories.recovered;
    let mut found = advisories.found;
    found.sort_by_key(|advisory| advisory.order);
    let document = match markdown.source().as_ref().unwrap_or(&options.source) {
        ComposeSource::File(path) => Some(path.clone()),
        ComposeSource::Url(url) => Some(std::path::PathBuf::from(url.to_string())),
        ComposeSource::Unknown => None,
    };
    let home = options.resolution_context().home_dir();
    let display = document.as_deref().map(|path| Prose::escape_text(&super::util::abbreviate_path(path, home)));
    let source = markdown.full_source_context_for_errors();
    let loaded = markdown.loaded_source_context_for_errors();
    let body_origin = BodyOrigin::capture(markdown);
    let body = markdown.content();
    let phrased: Vec<(Order, ComposeWarning)> = found
        .into_iter()
        .map(|advisory| {
            let order = advisory.order;
            // The authored position of the finding, projected through the
            // YAML scalar or the body's rewrites into the loaded text.
            let position = loaded.as_ref().and_then(|context| {
                let range = match &advisory.leaf {
                    Some((path, scanned)) => FrontmatterExpressionLocation {
                        path: path.clone(),
                        scanned: scanned.clone(),
                        span: advisory.origin.clone(),
                    }
                    .project(&context.content)?,
                    None => body_origin.as_ref()?.project(&advisory.origin)?,
                };
                one_based_position(&context.content, range.start)
            });
            let line = position.map(|(line, _)| line).or_else(|| match &advisory.key {
                Some(key) => super::shell_expansion::types::frontmatter_key_line(&source, key),
                None => Some(
                    markdown.frontmatter_line_count()
                        + body.get(..advisory.origin.start).map_or(0, |before| before.matches('\n').count())
                        + 1,
                ),
            });
            let location = match (display.as_deref(), line) {
                (Some(document), Some(line)) => format!(" at {document}:{line}"),
                (Some(document), None) => format!(" in {document}"),
                (None, Some(line)) => format!(" at line {line}"),
                (None, None) => String::new(),
            };
            let function = &advisory.function;
            let detail = advisory.finding.message.strip_prefix(&format!("{function}: ")).unwrap_or(&advisory.finding.message);
            let message = format!("`{function}(…)`{location}: {}", Prose::escape_text(detail));
            let diagnostic = advisory.finding.at(document.clone(), position);
            let warning = ComposeWarning {
                source: Some(ComposeWarning::EXPRESSION_SOURCE.to_string()),
                code: Some(advisory.finding.code.to_string()),
                identity: Some(WarningIdentity {
                    subject: WarningSubject::Expression {
                        document: None,
                        scope: advisory.leaf.map(|(path, _)| path_label(&path)),
                        origin: ExpressionOrigin::Authored(advisory.origin),
                    },
                }),
                path: document.clone(),
                line_number: line,
                diagnostic: Some(Box::new(diagnostic)),
                ..ComposeWarning::new("interpolation", message)
            };
            (order, warning)
        })
        .collect();
    let recovered = recovered.into_iter().map(|(order, found)| {
        let line = markdown.frontmatter_line_count() + body.get(..found.span.start).map_or(0, |before| before.matches('\n').count()) + 1;
        let mut diagnostic = found.diagnostic;
        if let Some(source) = diagnostic.source.as_mut() {
            source.file = document.clone();
            source.line = line;
        }
        let location = match display.as_deref() {
            Some(document) => format!(" at {document}:{line}"),
            None => format!(" at line {line}"),
        };
        let warning = ComposeWarning {
            source: Some(COLLECTION_SOURCE.to_string()),
            code: Some(diagnostic.code.clone()),
            identity: Some(WarningIdentity {
                subject: WarningSubject::Expression {
                    document: None,
                    scope: Some("collection-source".to_string()),
                    origin: ExpressionOrigin::Authored(found.span),
                },
            }),
            path: document.clone(),
            line_number: Some(line),
            diagnostic: Some(Box::new(diagnostic)),
            ..ComposeWarning::new("collection", format!("{}{location}", Prose::escape_text(&found_message(&found.reference, found.kind))))
        };
        (order, warning)
    });
    let mut all: Vec<(Order, ComposeWarning)> = phrased.into_iter().chain(recovered).collect();
    all.sort_by_key(|(order, _)| *order);
    all.into_iter().map(|(_, warning)| warning).collect()
}

fn found_message(reference: &str, kind: super::collection_blocks::CollectionKind) -> String {
    format!("`::{}` source `{reference}` has no declared iterable type", kind.opener())
}

fn check_frontmatter(
    markdown: &Markdown,
    provenance: &FrontmatterProvenance,
    exclude_keys: &std::collections::HashSet<String>,
    site: &TypeScope<'_>,
    advisories: &mut Advisories,
) -> Option<(Order, LocatedFrontmatterError)> {
    markdown.frontmatter().as_map().iter().filter(|(key, _)| !exclude_keys.contains(*key)).find_map(|(key, value)| {
        let mut path = vec![ValuePathSegment::Key(key.clone())];
        check_frontmatter_value(key, value, &mut path, provenance, site, advisories)
    })
}

fn check_frontmatter_value(
    key: &str,
    value: &Value,
    path: &mut Vec<ValuePathSegment>,
    provenance: &FrontmatterProvenance,
    site: &TypeScope<'_>,
    advisories: &mut Advisories,
) -> Option<(Order, LocatedFrontmatterError)> {
    if provenance.data().is_data(path) {
        return None;
    }
    match value {
        Value::String(text) if text.contains("{{") => {
            let rank = advisories.next_leaf();
            let leaf = Some((path.as_slice(), text.as_str()));
            let mut record = |checks: &[CallCheck], incomplete: &[IncompleteHostCall], offset: usize| {
                advisories.record(checks, incomplete, rank, offset, leaf, Some(key));
            };
            let (location, check, at) =
                first_rejected(text, ExpressionFinder::scan_plain(text).expressions, |_| site.clone(), &mut record)?;
            Some((
                (rank, at),
                LocatedFrontmatterError {
                    error: Box::new(rejection(Some(key), &location.expression, check)),
                    location: Some(FrontmatterExpressionLocation {
                        path: path.clone(),
                        scanned: text.clone(),
                        span: location.start..location.end,
                    }),
                },
            ))
        }
        Value::Array(items) => items.iter().enumerate().find_map(|(index, item)| {
            path.push(ValuePathSegment::Index(index));
            let outcome = check_frontmatter_value(key, item, path, provenance, site, advisories);
            path.pop();
            outcome
        }),
        Value::Object(map) => map.iter().find_map(|(child, item)| {
            path.push(ValuePathSegment::Key(child.clone()));
            let outcome = check_frontmatter_value(key, item, path, provenance, site, advisories);
            path.pop();
            outcome
        }),
        _ => None,
    }
}

/// A frontmatter leaf's key path as text, which scopes its offsets.
fn path_label(path: &[ValuePathSegment]) -> String {
    path.iter()
        .map(|segment| match segment {
            ValuePathSegment::Key(key) => format!("/{key}"),
            ValuePathSegment::Index(index) => format!("/{index}"),
        })
        .collect()
}

/// The body's first rejected call in authored order and its body offset.
/// Each surface is scanned up to its own first rejection; the earliest of
/// those is the body's.
fn check_body(
    markdown: &Markdown,
    options: &super::ComposeRequest,
    site: &TypeScope<'_>,
    advisories: &mut Advisories,
) -> Option<(usize, MarkdownError)> {
    let body = markdown.content();
    let page_blocks = options.call_check_covers(ComposeOperation::PageBlocks);
    let collections = options.call_check_covers(ComposeOperation::CollectionBlocks);
    let mut covered = ComposeOperationSet::empty();
    if page_blocks {
        covered.insert(ComposeOperation::PageBlocks);
    }
    if collections {
        covered.insert(ComposeOperation::CollectionBlocks);
    }
    let mut rejections = Vec::new();
    let mut guards = BodyScopes::scan(body, &covered);
    if collections && (body.contains("::map") || body.contains("::filter")) {
        let analysis_options = CollectionAnalysisOptions {
            operations: covered,
            binding_view: options.host_globals.as_ref().map(|globals| globals.view()),
            allow_untyped_map_variables: options.allow_untyped_map_variables(),
            allow_untyped_filter_variables: options.allow_untyped_filter_variables(),
            // The error is re-anchored to the loaded file below; this names
            // the span in the body.
            source_path: Some(std::path::Path::new("document")),
        };
        match analyze_collection_blocks(body, site, &analysis_options) {
            Ok(analysis) => {
                for recovered in analysis.recovered {
                    advisories.recovered.push(((BODY_RANK, recovered.span.start), recovered));
                }
                guards = analysis.scopes;
            }
            Err(error) => {
                let at = collection_offset(&error);
                rejections.push((at, collection_rejection(markdown, error)));
            }
        }
        for expression_site in guards.expression_sites(site) {
            let (checks, rejected) = checked(&expression_site.expression, &expression_site.scope);
            advisories.record(
                &checks,
                &incomplete_host_calls(&expression_site.expression),
                BODY_RANK,
                expression_site.origin,
                None,
                None,
            );
            if let Some((check, at)) = rejected {
                let start = expression_site.origin + at;
                let error = rejection(None, &expression_site.text, check);
                rejections.push((start, anchor_body(markdown, error, start..start + 1)));
            }
        }
    }

    if page_blocks
        && body.contains("::block")
        && let Ok(regions) = super::page_blocks::parser::parse_page_blocks(body, markdown.source_context_for_errors())
    {
        rejections.extend(check_block_conditions(markdown, &regions, &guards, site, advisories));
    }

    if (options.call_check_covers(ComposeOperation::BlockTransclusion)
        || options.call_check_covers(ComposeOperation::CodeTransclusion))
        && body.contains("when=")
        && let Ok(directives) = transclusion::parse_directives(body, markdown.source_context_for_errors())
    {
        rejections.extend(directives.iter().find_map(|directive| {
            let when = directive.options.when_expr.as_ref()?;
            let scope = guards.scope_at(site.clone(), directive.span.start);
            let offset = condition_start(body, directive.span.start, when);
            let (check, at) = rejected_condition(when, &scope, &mut |checks, incomplete| {
                advisories.record(checks, incomplete, BODY_RANK, offset, None, None)
            })?;
            let error = condition_rejection(markdown, when, directive.line, check);
            Some((offset + at, transclusion::TransclusionError::from(error).into()))
        }));
    }

    if options.call_check_covers(ComposeOperation::Interpolation) && body.contains("{{") {
        let scan = if super::inline::interpolation::resolve_interpolate_code_blocks(markdown, options) {
            ExpressionFinder::scan_plain(body)
        } else {
            ExpressionFinder::new(body).scan()
        };
        let mut record = |checks: &[CallCheck], incomplete: &[IncompleteHostCall], offset: usize| {
            advisories.record(checks, incomplete, BODY_RANK, offset, None, None);
        };
        if let Some((location, check, at)) =
            first_rejected(body, scan.expressions, |location| guards.scope_at(site.clone(), location.start), &mut record)
        {
            let error = rejection(None, &location.expression, check);
            rejections.push((at, anchor_body(markdown, error, location.start..location.end)));
        }
    }
    rejections.into_iter().min_by_key(|(at, _)| *at)
}

/// Each region's condition is checked under the guards of the blocks that
/// enclose it, which must hold for it to be evaluated; the first rejected
/// one, in authored order, with its body offset.
fn check_block_conditions(
    markdown: &Markdown,
    regions: &[PageBlockRegion],
    guards: &BodyScopes,
    site: &TypeScope<'_>,
    advisories: &mut Advisories,
) -> Option<(usize, MarkdownError)> {
    let body = markdown.content();
    regions.iter().find_map(|region| {
        if let Some(when) = &region.options.when_expr {
            let offset = condition_start(body, region.span.start, when);
            if let Some((check, at)) =
                rejected_condition(when, &guards.scope_at(site.clone(), region.span.start), &mut |checks, incomplete| {
                    advisories.record(checks, incomplete, BODY_RANK, offset, None, None)
                })
            {
                let error = condition_rejection(markdown, when, region.start_line, check);
                return Some((offset + at, PageBlockError::from(error).into()));
            }
        }
        check_block_conditions(markdown, &region.children, guards, site, advisories)
    })
}

/// The first `{{ … }}` of `expressions` in `text` holding a rejected call,
/// each checked under the scope `scope_at` gives it, with the rejection's
/// byte offset in `text`. The checks and incomplete host calls of that
/// expression and every one before it are passed to `record` with the
/// expression's offset in `text`; the caller keeps only the warnings before
/// the rejection. An expression that does not parse is left to its
/// evaluating stage, which reports it.
fn first_rejected<'d>(
    text: &str,
    expressions: Vec<ExpressionLocation>,
    scope_at: impl Fn(&ExpressionLocation) -> TypeScope<'d>,
    record: &mut impl FnMut(&[CallCheck], &[IncompleteHostCall], usize),
) -> Option<(ExpressionLocation, CallCheck, usize)> {
    expressions.into_iter().find_map(|location| {
        let expr = parse_spanned(&location.expression).ok()?;
        let (checks, rejected) = checked(&expr, &scope_at(&location));
        let offset = expression_start(text, &location);
        record(&checks, &incomplete_host_calls(&expr), offset);
        rejected.map(|(check, at)| (location, check, offset + at))
    })
}

/// The condition's first rejected call and its offset in `when`; its checks
/// and incomplete host calls are passed to `record` either way.
fn rejected_condition(
    when: &str,
    scope: &TypeScope<'_>,
    record: &mut impl FnMut(&[CallCheck], &[IncompleteHostCall]),
) -> Option<(CallCheck, usize)> {
    let expr = parse_condition_spanned(when).ok()?;
    let (checks, rejected) = checked(&expr, scope);
    record(&checks, &incomplete_host_calls(&expr));
    rejected
}

/// Every call check of `expr` under `scope`, and the rejected one whose
/// category 5 finding starts first, with that finding's offset.
fn checked(expr: &SpannedExpr, scope: &TypeScope<'_>) -> (Vec<CallCheck>, Option<(CallCheck, usize)>) {
    let checks = check_expression_calls(expr, scope);
    let rejected = checks
        .iter()
        .filter(|check| check.is_rejected())
        .map(|check| (check, check.rejection_diagnostic().map_or(check.span.start, |finding| finding.span.start)))
        .min_by_key(|(_, at)| *at)
        .map(|(check, at)| (check.clone(), at));
    (checks, rejected)
}

/// Byte offset in `body` of the `when` condition text of the directive
/// starting at `directive`; the directive's start when the authored text is
/// escaped.
fn condition_start(body: &str, directive: usize, when: &str) -> usize {
    directive + body.get(directive..).and_then(|rest| rest.find(when)).unwrap_or(0)
}

/// The same error a failing condition evaluation raises, so its rendering
/// (line excerpt, source link) is shared. `line` is the condition's body
/// line; the error carries the whole file and its file line, so the excerpt
/// and the shared diagnostic ([`MarkdownError::call_diagnostic`]) point
/// where the author wrote it.
fn condition_rejection(markdown: &Markdown, when: &str, line: usize, check: CallCheck) -> ConditionError {
    ConditionError::Eval {
        ctx: Box::new(markdown.full_source_context_for_errors()),
        expr: when.to_string(),
        line: markdown.frontmatter_line_count() + line,
        cause: Box::new(ExpressionError::CallCheck(Box::new(check))),
    }
}

/// The interpolation failure the rejected call is reported as: the same
/// variant (and so the same rendering and key attribution) a runtime type
/// error in that expression raises.
fn rejection(key: Option<&str>, expression: &str, check: CallCheck) -> MarkdownError {
    MarkdownError::Interpolation {
        key: key.map(str::to_string),
        expression: expression.to_string(),
        source: Box::new(SourceRef::Effective {
            rendered: expression.to_string(),
            origin_key: key.map(str::to_string),
        }),
        cause: Box::new(ExpressionError::CallCheck(Box::new(check))),
    }
}

/// Anchors a body rejection to its authored span, as body interpolation
/// anchors its own failures; the precheck sees the body before any stage
/// rewrote it.
fn anchor_body(markdown: &Markdown, error: MarkdownError, span: std::ops::Range<usize>) -> MarkdownError {
    let anchored = match (markdown.loaded_source_context_for_errors(), BodyOrigin::capture(markdown)) {
        (Some(context), Some(origin)) => match origin
            .project(&span)
            .and_then(|range| AuthoredSpan::locate(&context.content, range))
        {
            Some(authored) => error.with_authored_span(context, authored),
            None => error,
        },
        _ => error,
    };
    anchored.with_on_disk_source(&markdown.full_source_context_for_errors())
}

/// The body offset a collection failure points at; `0` when it names none.
fn collection_offset(error: &CollectionError) -> usize {
    match error.site().map(|site| site.source.as_ref()) {
        Some(SourceRef::OnDiskSpan { span, .. }) => span.range().start,
        _ => 0,
    }
}

/// A collection failure as the composition error, located in the loaded
/// file the way body interpolation locates its own: analysis located it in
/// the body text it was given.
fn collection_rejection(markdown: &Markdown, error: CollectionError) -> MarkdownError {
    let span = match error.site().map(|site| site.source.as_ref()) {
        Some(SourceRef::OnDiskSpan { span, .. }) => Some(span.range()),
        _ => None,
    };
    let located = match (span, markdown.loaded_source_context_for_errors(), BodyOrigin::capture(markdown)) {
        (Some(span), Some(context), Some(origin)) => origin
            .project(&span)
            .and_then(|range| AuthoredSpan::locate(&context.content, range))
            .map(|authored| SourceRef::OnDiskSpan { context, span: authored }),
        _ => None,
    };
    match located {
        Some(source) => error.with_source(source).into(),
        None => error.into(),
    }
}
