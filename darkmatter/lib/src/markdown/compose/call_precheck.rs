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
//! A body expression is checked under the guards of every enclosing
//! `::block` (when page blocks are enabled, the only case in which a false
//! block is removed). A transcluded child runs its own pipeline node, so it is
//! checked against its own schema and origin before its own evaluation;
//! nothing a parent inserts is the parent's syntax.

use super::body_origin::BodyOrigin;
use super::conditions::ConditionError;
use super::expression::{
    CallCheck, DeclaredTypes, ExpressionError, ExpressionFinder, ExpressionLocation, TypeScope, check_expression_calls,
    parse_condition_spanned, parse_spanned,
};
use super::frontmatter_interpolation::{FrontmatterExpressionLocation, LocatedFrontmatterError};
use super::page_blocks::{BodyGuards, PageBlockError, PageBlockRegion};
use super::value_origin::{FrontmatterProvenance, ValuePathSegment};
use super::{ComposeOperation, transclusion};
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

/// Checks every executable authored call of `markdown` on the surface
/// `options` composes, against `schema`'s declared types; a recursive-typed
/// property resolves in `schema`'s definition table.
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
) -> Result<(), PrecheckFailure> {
    let mut declared = DeclaredTypes::new(schema.and_then(|schema| schema.simplified.as_ref()));
    if let Some(schema) = schema {
        declared = declared.with_definitions(schema.definitions());
    }
    let site = TypeScope::unguarded(&declared);
    if options.is_enabled(ComposeOperation::FrontmatterInterpolation) {
        check_frontmatter(markdown, provenance, &options.exclude_keys, &site).map_err(PrecheckFailure::Frontmatter)?;
    }
    check_body(markdown, options, &site).map_err(PrecheckFailure::Body)
}

fn check_frontmatter(
    markdown: &Markdown,
    provenance: &FrontmatterProvenance,
    exclude_keys: &std::collections::HashSet<String>,
    site: &TypeScope<'_>,
) -> Result<(), LocatedFrontmatterError> {
    for (key, value) in markdown.frontmatter().as_map() {
        if exclude_keys.contains(key) {
            continue;
        }
        let mut path = vec![ValuePathSegment::Key(key.clone())];
        check_frontmatter_value(key, value, &mut path, provenance, site)?;
    }
    Ok(())
}

fn check_frontmatter_value(
    key: &str,
    value: &Value,
    path: &mut Vec<ValuePathSegment>,
    provenance: &FrontmatterProvenance,
    site: &TypeScope<'_>,
) -> Result<(), LocatedFrontmatterError> {
    if provenance.data().is_data(path) {
        return Ok(());
    }
    match value {
        Value::String(text) if text.contains("{{") => {
            match first_rejected(ExpressionFinder::scan_plain(text).expressions, |_| site.clone()) {
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
                let outcome = check_frontmatter_value(key, item, path, provenance, site);
                path.pop();
                outcome?;
            }
            Ok(())
        }
        Value::Object(map) => {
            for (child, item) in map {
                path.push(ValuePathSegment::Key(child.clone()));
                let outcome = check_frontmatter_value(key, item, path, provenance, site);
                path.pop();
                outcome?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn check_body(markdown: &Markdown, options: &super::ComposeRequest, site: &TypeScope<'_>) -> Result<(), MarkdownError> {
    let body = markdown.content();
    let page_blocks = options.is_enabled(ComposeOperation::PageBlocks);
    let guards = if page_blocks { BodyGuards::scan(body) } else { BodyGuards::default() };

    if page_blocks
        && body.contains("::block")
        && let Ok(regions) = super::page_blocks::parser::parse_page_blocks(body, markdown.source_context_for_errors())
    {
        check_block_conditions(markdown, &regions, &guards, site)?;
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
            if let Some(check) = rejected_condition(when, &guards.scope_at(site.clone(), directive.span.start)) {
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
        if let Some((location, check)) =
            first_rejected(scan.expressions, |location| guards.scope_at(site.clone(), location.start))
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
) -> Result<(), MarkdownError> {
    for region in regions {
        if let Some(when) = &region.options.when_expr
            && let Some(check) = rejected_condition(when, &guards.scope_at(site.clone(), region.span.start))
        {
            let error = condition_rejection(markdown, when, region.start_line, check);
            return Err(PageBlockError::from(error).into());
        }
        check_block_conditions(markdown, &region.children, guards, site)?;
    }
    Ok(())
}

/// The first `{{ … }}` of `expressions` holding a rejected call, each
/// checked under the scope `scope_at` gives it. An expression that does not
/// parse is left to its evaluating stage, which reports it.
fn first_rejected<'d>(
    expressions: Vec<ExpressionLocation>,
    scope_at: impl Fn(&ExpressionLocation) -> TypeScope<'d>,
) -> Option<(ExpressionLocation, CallCheck)> {
    expressions.into_iter().find_map(|location| {
        let expr = parse_spanned(&location.expression).ok()?;
        let check = check_expression_calls(&expr, &scope_at(&location)).into_iter().find(CallCheck::is_rejected)?;
        Some((location, check))
    })
}

fn rejected_condition(when: &str, scope: &TypeScope<'_>) -> Option<CallCheck> {
    let expr = parse_condition_spanned(when).ok()?;
    check_expression_calls(&expr, scope).into_iter().find(CallCheck::is_rejected)
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
