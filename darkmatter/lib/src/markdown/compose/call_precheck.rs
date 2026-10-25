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
//! The same walk collects every call that is always `false` because an
//! authored literal argument is not a well-formed file reference
//! (`file_exists("https://")`), which composition reports as a
//! [`ALWAYS_FALSE_CALL_CODE`] warning.
//!
//! A body expression is checked under the guards of every enclosing
//! `::block` (when page blocks are enabled, the only case in which a false
//! block is removed). A transcluded child runs its own pipeline node, so it is
//! checked against its own schema and origin before its own evaluation;
//! nothing a parent inserts is the parent's syntax.

use super::body_origin::BodyOrigin;
use super::conditions::ConditionError;
use super::context::report::{ExpressionOrigin, WarningIdentity, WarningSubject};
use super::expression::{
    ALWAYS_FALSE_CALL_CODE, CallCheck, DeclaredTypes, ExpressionError, ExpressionFinder, ExpressionLocation,
    TypeScope, check_expression_calls, parse_condition_spanned, parse_spanned,
};
use super::frontmatter_interpolation::{FrontmatterExpressionLocation, LocatedFrontmatterError};
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

/// A call that is always `false`, where its scanned text holds it.
struct AlwaysFalse {
    function: String,
    argument: usize,
    /// The frontmatter key path whose string leaf holds it; `None` for the
    /// body.
    scope: Option<String>,
    /// The frontmatter key, for its line.
    key: Option<String>,
    /// Byte range of the malformed argument in the scanned text.
    origin: std::ops::Range<usize>,
}

/// Always-false calls found so far, in document order.
#[derive(Default)]
struct Advisories(Vec<AlwaysFalse>);

impl Advisories {
    fn record(&mut self, check: &CallCheck, offset: usize, scope: Option<&str>, key: Option<&str>) {
        if let Some(finding) = check.always_false_finding() {
            self.0.push(AlwaysFalse {
                function: check.function.clone(),
                argument: finding.argument,
                scope: scope.map(str::to_string),
                key: key.map(str::to_string),
                origin: offset + finding.span.start..offset + finding.span.end,
            });
        }
    }
}

/// Checks every executable authored call of `markdown` on the surface
/// `options` composes, against the declared types of every layer of
/// `schema` ([`DeclaredTypes::for_schema`]).
///
/// ## Returns
///
/// One [`ALWAYS_FALSE_CALL_CODE`] warning per always-false call, in
/// document order.
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
    Ok(always_false_warnings(markdown, options, advisories))
}

fn always_false_warnings(
    markdown: &Markdown,
    options: &super::ComposeRequest,
    advisories: Advisories,
) -> Vec<ComposeWarning> {
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
    let body = markdown.content();
    advisories
        .0
        .into_iter()
        .map(|advisory| {
            let line = match &advisory.key {
                Some(key) => super::shell_expansion::types::frontmatter_key_line(&source, key),
                None => Some(
                    markdown.frontmatter_line_count()
                        + body.get(..advisory.origin.start).map_or(0, |before| before.matches('\n').count())
                        + 1,
                ),
            };
            let location = match (display.as_deref(), line) {
                (Some(document), Some(line)) => format!(" at {document}:{line}"),
                (Some(document), None) => format!(" in {document}"),
                (None, Some(line)) => format!(" at line {line}"),
                (None, None) => String::new(),
            };
            let message = format!(
                "`{}(…)`{location}: argument {} is not a well-formed file reference, so the call is always false",
                advisory.function,
                advisory.argument + 1
            );
            ComposeWarning {
                source: Some(ComposeWarning::EXPRESSION_SOURCE.to_string()),
                code: Some(ALWAYS_FALSE_CALL_CODE.to_string()),
                identity: Some(WarningIdentity {
                    subject: WarningSubject::Expression {
                        document: None,
                        scope: advisory.scope,
                        origin: ExpressionOrigin::Authored(advisory.origin),
                    },
                }),
                path: document.clone(),
                line_number: line,
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
            let scope = path_label(path);
            let mut record = |check: &CallCheck, offset: usize| advisories.record(check, offset, Some(&scope), Some(key));
            match first_rejected(ExpressionFinder::scan_plain(text).expressions, |_| site.clone(), &mut record) {
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
            if let Some(check) = rejected_condition(when, &scope, &mut |check| {
                advisories.record(check, directive.span.start, None, None)
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
        let mut record = |check: &CallCheck, offset: usize| advisories.record(check, offset, None, None);
        if let Some((location, check)) =
            first_rejected(scan.expressions, |location| guards.scope_at(site.clone(), location.start), &mut record)
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
    for region in regions {
        if let Some(when) = &region.options.when_expr
            && let Some(check) = rejected_condition(when, &guards.scope_at(site.clone(), region.span.start), &mut |check| {
                advisories.record(check, region.span.start, None, None)
            })
        {
            let error = condition_rejection(markdown, when, region.start_line, check);
            return Err(PageBlockError::from(error).into());
        }
        check_block_conditions(markdown, &region.children, guards, site, advisories)?;
    }
    Ok(())
}

/// The first `{{ … }}` of `expressions` holding a rejected call, each
/// checked under the scope `scope_at` gives it; every check before it is
/// passed to `record` with its expression's offset in the scanned text. An
/// expression that does not parse is left to its evaluating stage, which
/// reports it.
fn first_rejected<'d>(
    expressions: Vec<ExpressionLocation>,
    scope_at: impl Fn(&ExpressionLocation) -> TypeScope<'d>,
    record: &mut impl FnMut(&CallCheck, usize),
) -> Option<(ExpressionLocation, CallCheck)> {
    expressions.into_iter().find_map(|location| {
        let expr = parse_spanned(&location.expression).ok()?;
        let checks = check_expression_calls(&expr, &scope_at(&location));
        if let Some(check) = checks.iter().find(|check| check.is_rejected()) {
            return Some((location, check.clone()));
        }
        checks.iter().for_each(|check| record(check, location.start));
        None
    })
}

/// The condition's first rejected call; every other check is passed to
/// `record`.
fn rejected_condition(when: &str, scope: &TypeScope<'_>, record: &mut impl FnMut(&CallCheck)) -> Option<CallCheck> {
    let expr = parse_condition_spanned(when).ok()?;
    let checks = check_expression_calls(&expr, scope);
    if let Some(check) = checks.iter().find(|check| check.is_rejected()) {
        return Some(check.clone());
    }
    checks.iter().for_each(record);
    None
}

/// The same error a failing condition evaluation raises, so its rendering
/// (line excerpt, source link) is shared.
fn condition_rejection(markdown: &Markdown, when: &str, line: usize, check: CallCheck) -> ConditionError {
    ConditionError::Eval {
        ctx: Box::new(markdown.source_context_for_errors()),
        expr: when.to_string(),
        line,
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
