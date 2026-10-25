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
//! block is removed). A transcluded child runs its own pipeline node, so it is
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
use super::page_blocks::{BodyGuards, PageBlockError, PageBlockRegion};
use super::value_origin::{FrontmatterProvenance, ValuePathSegment};
use super::{ComposeOperation, ComposeSource, ComposeWarning, transclusion};
use crate::markdown::Markdown;
use crate::markdown::schemas::EffectiveSchema;
use crate::markdown::types::{AuthoredSpan, MarkdownError, SourceRef};
use serde_json::Value;

/// A category 5 call the precheck found.
pub(crate) enum PrecheckFailure {
    /// In a frontmatter value; the pipeline attributes it like an
    /// interpolation failure (override supplier or authored span).
    Frontmatter(LocatedFrontmatterError),
    /// In the body, already located.
    Body(MarkdownError),
}

/// A shown warning of one call, where its scanned text holds it.
struct Advisory {
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

/// Warnings found so far, in document order.
#[derive(Default)]
struct Advisories(Vec<Advisory>);

impl Advisories {
    /// Records every warning of `checks` and of `incomplete` calls, for an
    /// expression starting at byte `offset` of the scanned text.
    fn record(
        &mut self,
        checks: &[CallCheck],
        incomplete: &[IncompleteHostCall],
        offset: usize,
        leaf: Option<(&[Segment], &str)>,
        key: Option<&str>,
    ) {
        for check in checks {
            for finding in check.diagnostics(false) {
                self.push(&check.function, finding, offset, leaf, key);
            }
        }
        for call in incomplete {
            self.push(&call.function, call.diagnostic(), offset, leaf, key);
        }
    }

    fn push(
        &mut self,
        function: &str,
        finding: ExpressionDiagnostic,
        offset: usize,
        leaf: Option<(&[Segment], &str)>,
        key: Option<&str>,
    ) {
        self.0.push(Advisory {
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
/// ## Returns
///
/// One warning per finding [`CallCheck::diagnostics`] shows without
/// `expressions.strict`, plus one per incomplete host call, in document
/// order.
///
/// ## Errors
///
/// The first rejected call (frontmatter first, then the body in document
/// order), carrying [`ExpressionError::CallCheck`].
pub(crate) fn check_document(
    markdown: &Markdown,
    provenance: &FrontmatterProvenance,
    options: &super::ComposeRequest,
    schema: Option<&EffectiveSchema>,
) -> Result<Vec<ComposeWarning>, PrecheckFailure> {
    let declared = DeclaredTypes::for_schema(schema);
    let site = TypeScope::unguarded(&declared);
    let mut advisories = Advisories::default();
    if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
        check_frontmatter(markdown, provenance, &options.exclude_keys, &site, &mut advisories)
            .map_err(PrecheckFailure::Frontmatter)?;
    }
    check_body(markdown, options, &site, &mut advisories).map_err(PrecheckFailure::Body)?;
    Ok(advisory_warnings(markdown, options, advisories))
}

fn advisory_warnings(markdown: &Markdown, options: &super::ComposeRequest, advisories: Advisories) -> Vec<ComposeWarning> {
    use biscuit_terminal::components::prose::Prose;
    if advisories.0.is_empty() {
        return Vec::new();
    }
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
    advisories
        .0
        .into_iter()
        .map(|advisory| {
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
            ComposeWarning {
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
            }
        })
        .collect()
}

fn check_frontmatter(
    markdown: &Markdown,
    provenance: &FrontmatterProvenance,
    exclude_keys: &std::collections::HashSet<String>,
    site: &TypeScope<'_>,
    advisories: &mut Advisories,
) -> Result<(), LocatedFrontmatterError> {
    for (key, value) in markdown.frontmatter().as_map() {
        if exclude_keys.contains(key) {
            continue;
        }
        let mut path = vec![ValuePathSegment::Key(key.clone())];
        check_frontmatter_value(key, value, &mut path, provenance, site, advisories)?;
    }
    Ok(())
}

fn check_frontmatter_value(
    key: &str,
    value: &Value,
    path: &mut Vec<ValuePathSegment>,
    provenance: &FrontmatterProvenance,
    site: &TypeScope<'_>,
    advisories: &mut Advisories,
) -> Result<(), LocatedFrontmatterError> {
    if provenance.data().is_data(path) {
        return Ok(());
    }
    match value {
        Value::String(text) if text.contains("{{") => {
            let leaf = Some((path.as_slice(), text.as_str()));
            let mut record = |checks: &[CallCheck], incomplete: &[IncompleteHostCall], offset: usize| {
                advisories.record(checks, incomplete, offset, leaf, Some(key));
            };
            match first_rejected(text, ExpressionFinder::scan_plain(text).expressions, |_| site.clone(), &mut record) {
                Some((location, check)) => Err(LocatedFrontmatterError {
                    error: Box::new(rejection(Some(key), &location.expression, check)),
                    location: Some(FrontmatterExpressionLocation {
                        path: path.clone(),
                        scanned: text.clone(),
                        span: location.start..location.end,
                    }),
                }),
                None => Ok(()),
            }
        }
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                path.push(ValuePathSegment::Index(index));
                let outcome = check_frontmatter_value(key, item, path, provenance, site, advisories);
                path.pop();
                outcome?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for (child, item) in map {
                path.push(ValuePathSegment::Key(child.clone()));
                let outcome = check_frontmatter_value(key, item, path, provenance, site, advisories);
                path.pop();
                outcome?;
            }
            Ok(())
        }
        _ => Ok(()),
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

fn check_body(
    markdown: &Markdown,
    options: &super::ComposeRequest,
    site: &TypeScope<'_>,
    advisories: &mut Advisories,
) -> Result<(), MarkdownError> {
    let body = markdown.content();
    let page_blocks = options.is_enabled(ComposeOperation::PageBlocks);
    let guards = if page_blocks { BodyGuards::scan(body) } else { BodyGuards::default() };

    if page_blocks
        && body.contains("::block")
        && let Ok(regions) = super::page_blocks::parser::parse_page_blocks(body, markdown.source_context_for_errors())
    {
        check_block_conditions(markdown, &regions, &guards, site, advisories)?;
    }

    if (options.is_enabled(ComposeOperation::BlockTransclusion)
        || options.is_enabled(ComposeOperation::CodeTransclusion))
        && body.contains("when=")
        && let Ok(directives) = transclusion::parse_directives(body, markdown.source_context_for_errors())
    {
        for directive in &directives {
            let Some(when) = &directive.options.when_expr else {
                continue;
            };
            let scope = guards.scope_at(site.clone(), directive.span.start);
            let offset = condition_start(body, directive.span.start, when);
            if let Some(check) = rejected_condition(when, &scope, &mut |checks, incomplete| {
                advisories.record(checks, incomplete, offset, None, None)
            }) {
                let error = condition_rejection(markdown, when, directive.line, check);
                return Err(transclusion::TransclusionError::from(error).into());
            }
        }
    }

    if options.is_enabled(ComposeOperation::Interpolation) && body.contains("{{") {
        let scan = if super::inline::interpolation::resolve_interpolate_code_blocks(markdown, options) {
            ExpressionFinder::scan_plain(body)
        } else {
            ExpressionFinder::new(body).scan()
        };
        let mut record = |checks: &[CallCheck], incomplete: &[IncompleteHostCall], offset: usize| {
            advisories.record(checks, incomplete, offset, None, None);
        };
        if let Some((location, check)) =
            first_rejected(body, scan.expressions, |location| guards.scope_at(site.clone(), location.start), &mut record)
        {
            let error = rejection(None, &location.expression, check);
            return Err(anchor_body(markdown, error, location.start..location.end));
        }
    }
    Ok(())
}

/// Each region's condition is checked under the guards of the blocks that
/// enclose it, which must hold for it to be evaluated.
fn check_block_conditions(
    markdown: &Markdown,
    regions: &[PageBlockRegion],
    guards: &BodyGuards,
    site: &TypeScope<'_>,
    advisories: &mut Advisories,
) -> Result<(), MarkdownError> {
    let body = markdown.content();
    for region in regions {
        if let Some(when) = &region.options.when_expr
            && let Some(check) =
                rejected_condition(when, &guards.scope_at(site.clone(), region.span.start), &mut |checks, incomplete| {
                    advisories.record(checks, incomplete, condition_start(body, region.span.start, when), None, None)
                })
        {
            let error = condition_rejection(markdown, when, region.start_line, check);
            return Err(PageBlockError::from(error).into());
        }
        check_block_conditions(markdown, &region.children, guards, site, advisories)?;
    }
    Ok(())
}

/// The first `{{ … }}` of `expressions` in `text` holding a rejected call,
/// each checked under the scope `scope_at` gives it; the checks and
/// incomplete host calls of every expression before it are passed to
/// `record` with the expression's offset in `text`. An expression that does
/// not parse is left to its evaluating stage, which reports it.
fn first_rejected<'d>(
    text: &str,
    expressions: Vec<ExpressionLocation>,
    scope_at: impl Fn(&ExpressionLocation) -> TypeScope<'d>,
    record: &mut impl FnMut(&[CallCheck], &[IncompleteHostCall], usize),
) -> Option<(ExpressionLocation, CallCheck)> {
    expressions.into_iter().find_map(|location| {
        let expr = parse_spanned(&location.expression).ok()?;
        let (checks, rejected) = checked(&expr, &scope_at(&location));
        if let Some(check) = rejected {
            return Some((location, check));
        }
        record(&checks, &incomplete_host_calls(&expr), expression_start(text, &location));
        None
    })
}

/// The condition's first rejected call; otherwise its checks and incomplete
/// host calls are passed to `record`.
fn rejected_condition(
    when: &str,
    scope: &TypeScope<'_>,
    record: &mut impl FnMut(&[CallCheck], &[IncompleteHostCall]),
) -> Option<CallCheck> {
    let expr = parse_condition_spanned(when).ok()?;
    let (checks, rejected) = checked(&expr, scope);
    if rejected.is_none() {
        record(&checks, &incomplete_host_calls(&expr));
    }
    rejected
}

/// Every call check of `expr` under `scope`, and the first rejected one.
fn checked(expr: &SpannedExpr, scope: &TypeScope<'_>) -> (Vec<CallCheck>, Option<CallCheck>) {
    let checks = check_expression_calls(expr, scope);
    let rejected = checks.iter().find(|check| check.is_rejected()).cloned();
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
