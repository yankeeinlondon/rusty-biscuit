//! Passive site analysis of `::map` / `::filter` regions.
//!
//! [`analyze_collection_blocks`] checks every authored region of a body
//! against the declared types of its site, without evaluating a callback,
//! running a handler, or resolving a file or service. Each static defect is
//! reported once, at its authored span, before any effect can run:
//!
//! | Defect | Code | Under a recovery boolean |
//! |---|---|---|
//! | source has no iterable declaration | `dm.collection.untyped_map_source` / `untyped_filter_source` | warning |
//! | source is a scalar or has a never-iterable alternative | `dm.collection.invalid_source` | error |
//! | parameter names a reserved root, `null`, or a host global | `dm.expression.invalid_callback` | error |
//! | pattern cannot bind the entry, or the map result | `dm.collection.pattern_mismatch` | error |
//! | sort key is provably not a number or a string | `dm.collection.invalid_sort_key` | error |
//!
//! Header grammar defects come from the shared parser as
//! `dm.collection.malformed` / `dm.expression.invalid_callback`. A region
//! whose gate is false, or whose source is empty, is checked like any other;
//! a runtime gate never excuses a static defect.
//!
//! Calls inside the callbacks and bodies are not judged here: the
//! composition call check and editors walk [`BodyScopes::expression_sites`]
//! and [`BodyScopes::scope_at`] and apply the existing call diagnostics under
//! the exact scope each site is evaluated in.

use std::ops::Range;
use std::path::Path;

use super::error::CollectionError;
use super::parser::{parse_collection_regions_in, site};
use super::scopes::{BodyScopes, ExpressionRole, ParsedCallback, Problem, RegionPlan};
use super::types::{
    CollectionAnalysis, CollectionAnalysisOptions, CollectionKind, QuotedExpression, RecoveredSource,
};
use crate::markdown::compose::expression::{
    Diagnostic, DiagnosticSource, Severity, SpannedBindingPattern, TypeScope, check_local_names, one_based_position,
};
use crate::markdown::compose::page_blocks::BodyGuards;
use crate::markdown::compose::pipeline::operations::ComposeOperation;

/// Analyzes the `::map` / `::filter` regions of `source` at `site`.
///
/// `site` is the scope of the whole body: the document's declared types and
/// any host globals it declares. Returns the regions, the scope inventory
/// ([`BodyScopes`]), and one warning per untyped source a recovery boolean
/// waived.
///
/// With [`ComposeOperation::CollectionBlocks`] disabled the wrappers are
/// inert text: no region is parsed, even a malformed one.
///
/// ## Errors
///
/// The first defect in document order, as a located [`CollectionError`]: a
/// malformed region or header, an untyped source (unless waived), an invalid
/// source, a protected parameter name, a pattern mismatch, or an unsupported
/// sort key. The warnings authored before it are dropped; a consumer that
/// reports them with the defect uses [`analyze_collection_blocks_report`].
pub fn analyze_collection_blocks(
    source: &str,
    site: &TypeScope<'_>,
    options: &CollectionAnalysisOptions<'_>,
) -> Result<CollectionAnalysis, CollectionError> {
    let report = analyze_collection_blocks_report(source, site, options);
    match report.rejection {
        Some(error) => Err(error),
        None => Ok(report.analysis),
    }
}

/// [`analyze_collection_blocks`] with the warnings found before its defect.
#[derive(Debug, Clone, Default)]
pub struct CollectionAnalysisReport {
    /// The analysis. With a `rejection`, `recovered` and `diagnostics` hold
    /// only the waived sources authored before the rejected span, and
    /// `regions` and `scopes` are empty when the regions did not parse.
    pub analysis: CollectionAnalysis,
    /// The first defect in document order, as [`analyze_collection_blocks`]
    /// returns it.
    pub rejection: Option<CollectionError>,
}

/// Analyzes like [`analyze_collection_blocks`], keeping the recovery warnings
/// authored before the first defect alongside it, so a report can show them
/// with the failure without analyzing or evaluating anything again.
pub fn analyze_collection_blocks_report(
    source: &str,
    site: &TypeScope<'_>,
    options: &CollectionAnalysisOptions<'_>,
) -> CollectionAnalysisReport {
    let operations = &options.operations;
    let guards = if operations.contains(ComposeOperation::PageBlocks) {
        BodyGuards::scan(source)
    } else {
        BodyGuards::default()
    };
    if !operations.contains(ComposeOperation::CollectionBlocks) {
        return CollectionAnalysisReport {
            analysis: CollectionAnalysis { scopes: BodyScopes::from_parts(guards, &[]), ..CollectionAnalysis::default() },
            rejection: None,
        };
    }

    let regions = match parse_collection_regions_in(source, None, operations, options.source_path) {
        Ok(regions) => regions,
        Err(error) => return CollectionAnalysisReport { analysis: CollectionAnalysis::default(), rejection: Some(error) },
    };
    let scopes = BodyScopes::from_parts(guards, &regions);

    let mut errors: Vec<(usize, CollectionError)> = Vec::new();
    let mut recovered = Vec::new();
    for plan in scopes.plans() {
        check_region(source, site, options, &scopes, plan, &mut errors, &mut recovered);
    }
    let rejection = errors.into_iter().min_by_key(|(at, _)| *at);
    if let Some((at, _)) = &rejection {
        recovered.retain(|found: &RecoveredSource| found.span.start < *at);
    }
    let diagnostics = recovered.iter().map(|found: &RecoveredSource| found.diagnostic.clone()).collect();
    CollectionAnalysisReport {
        analysis: CollectionAnalysis { regions, scopes, diagnostics, recovered },
        rejection: rejection.map(|(_, error)| error),
    }
}

/// The authored span of a binding pattern: its identifier, or first through
/// last field.
fn pattern_span(quoted: &QuotedExpression, pattern: &SpannedBindingPattern) -> Range<usize> {
    let decoded = match pattern {
        SpannedBindingPattern::Identifier(name) => name.span.clone(),
        SpannedBindingPattern::Fields(fields) => match (fields.first(), fields.last()) {
            (Some(first), Some(last)) => first.span.start..last.span.end,
            _ => 0..0,
        },
    };
    quoted.authored_range(decoded).unwrap_or_else(|| quoted.span.clone())
}

fn callback_for(plan: &RegionPlan, role: ExpressionRole) -> Option<&ParsedCallback> {
    plan.callbacks().find(|(found, _)| *found == role).map(|(_, callback)| callback)
}

fn check_region(
    source: &str,
    root: &TypeScope<'_>,
    options: &CollectionAnalysisOptions<'_>,
    scopes: &BodyScopes,
    plan: &RegionPlan,
    errors: &mut Vec<(usize, CollectionError)>,
    recovered: &mut Vec<RecoveredSource>,
) {
    let path = options.source_path;
    let located = |span: Range<usize>| site(source, path, span);
    let reference = plan.header.source.value.as_str();

    // Parameter names are checked against the binding view actually in force.
    for (_, callback) in plan.callbacks() {
        let names: Vec<(&str, Range<usize>)> = match &callback.spanned.pattern {
            SpannedBindingPattern::Identifier(name) => vec![(name.value.as_str(), name.span.clone())],
            SpannedBindingPattern::Fields(fields) => {
                fields.iter().map(|field| (field.value.as_str(), field.span.clone())).collect()
            }
        };
        for (name, decoded) in names {
            if let Err(protected) = check_local_names([name], options.binding_view) {
                let span = callback.quoted.authored_range(decoded).unwrap_or_else(|| callback.quoted.span.clone());
                errors.push((
                    span.start,
                    CollectionError::InvalidCallback { site: located(span), message: protected.to_string() },
                ));
            }
        }
    }

    let outer = scopes.scope_at(root.clone(), plan.span.start);
    for problem in plan.evaluate(&outer).problems {
        match problem {
            Problem::UntypedSource => {
                let span = plan.header.source.span.clone();
                let waived = match plan.kind {
                    CollectionKind::Map => options.allow_untyped_map_variables,
                    CollectionKind::Filter => options.allow_untyped_filter_variables,
                };
                if waived {
                    recovered.push(recovered_source(source, path, plan, span));
                } else {
                    errors.push((
                        span.start,
                        CollectionError::UntypedSource {
                            site: located(span),
                            kind: plan.kind,
                            reference: reference.to_string(),
                        },
                    ));
                }
            }
            Problem::InvalidSource(arms) => {
                let span = plan.header.source.span.clone();
                errors.push((
                    span.start,
                    CollectionError::InvalidSource {
                        site: located(span),
                        kind: plan.kind,
                        reference: reference.to_string(),
                        message: format!(
                            "`{reference}` cannot be iterated by `::{}`: its type includes {}",
                            plan.kind.opener(),
                            arms.iter().map(|arm| format!("`{arm}`")).collect::<Vec<_>>().join(", ")
                        ),
                    },
                ));
            }
            Problem::Pattern(role, mismatch) => {
                let Some(callback) = callback_for(plan, role) else { continue };
                let span = pattern_span(&callback.quoted, &callback.spanned.pattern);
                errors.push((
                    span.start,
                    CollectionError::PatternMismatch {
                        site: located(span),
                        message: format!("the {} pattern cannot bind `{reference}`: {mismatch}", role.label()),
                    },
                ));
            }
            Problem::Result(mismatch) => {
                let Some(callback) = callback_for(plan, ExpressionRole::Positional) else { continue };
                let span = pattern_span(&callback.quoted, &callback.spanned.pattern);
                errors.push((
                    span.start,
                    CollectionError::PatternMismatch {
                        site: located(span),
                        message: format!("the mapped result cannot satisfy the body pattern: {mismatch}"),
                    },
                ));
            }
            Problem::SortKey(ty) => {
                let Some(callback) = callback_for(plan, ExpressionRole::Sort) else { continue };
                let body = &callback.spanned.body.span;
                let span = callback
                    .quoted
                    .authored_range(body.clone())
                    .unwrap_or_else(|| callback.quoted.span.clone());
                errors.push((
                    span.start,
                    CollectionError::InvalidSortKey {
                        site: located(span),
                        message: format!("the sort key has type `{ty}`, which is never a number or a string"),
                    },
                ));
            }
        }
    }
}

fn recovered_source(source: &str, path: Option<&Path>, plan: &RegionPlan, span: Range<usize>) -> RecoveredSource {
    let reference = plan.header.source.value.clone();
    let code = match plan.kind {
        CollectionKind::Map => super::diagnostics::UNTYPED_MAP_SOURCE_CODE,
        CollectionKind::Filter => super::diagnostics::UNTYPED_FILTER_SOURCE_CODE,
    };
    let diagnostic = Diagnostic {
        code: code.to_string(),
        severity: Severity::Warning,
        message: format!(
            "`{reference}` has no declared iterable type; `::{}` discovers its entries at runtime because the untyped-source recovery is allowed",
            plan.kind.opener()
        ),
        source: one_based_position(source, span.start).map(|(line, column)| DiagnosticSource {
            file: path.map(Path::to_path_buf),
            line,
            column,
        }),
        expected_type: None,
        actual_type: None,
        span: None,
        iteration_path: Default::default(),
    };
    RecoveredSource { kind: plan.kind, reference, span, diagnostic }
}
