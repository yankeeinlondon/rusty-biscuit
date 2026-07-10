//! Substrate diagnostics: broken links, missing anchors, and duplicate heading
//! slugs.
//!
//! These are the tier-2 (post-index) graph diagnostics of the Phase-4 pipeline;
//! they range against the current document's concrete spans (via its source
//! map), never against message text. Duplicate-slug diagnostics carry
//! `relatedInformation` pointing at the colliding twins.

use lsp_types::{
    Diagnostic, DiagnosticRelatedInformation, DiagnosticSeverity, Location, NumberOrString,
};

use super::DocumentContext;
use crate::diagnostics::codes::{code, source};
use crate::graph::LinkDiagnostic;
use darkmatter::markdown::generate_heading_slug;

/// All substrate diagnostics for the current document.
pub fn diagnostics(ctx: &DocumentContext) -> Vec<Diagnostic> {
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let mut out = Vec::new();
    link_diagnostics(ctx, doc_id, &mut out);
    duplicate_heading_diagnostics(ctx, doc_id, &mut out);
    out
}

/// Broken relative paths and missing anchors.
fn link_diagnostics(ctx: &DocumentContext, doc_id: crate::graph::DocumentId, out: &mut Vec<Diagnostic>) {
    for (_, node) in ctx.graph.links(doc_id) {
        let Some(link) = node.as_link() else {
            continue;
        };
        let Some(kind) = ctx.graph.diagnose_unresolved(doc_id, &link.target) else {
            continue;
        };
        let Some(range) = ctx.source_map.byte_range_to_lsp(node.span.clone()) else {
            continue;
        };
        let (code_value, message) = match kind {
            LinkDiagnostic::BrokenPath => (
                code::BROKEN_PATH,
                format!("broken link: no document matches `{}`", link.raw_target),
            ),
            LinkDiagnostic::MissingAnchor => (
                code::MISSING_ANCHOR,
                format!("missing anchor: `{}` has no matching heading", link.raw_target),
            ),
        };
        out.push(Diagnostic {
            range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(code_value.to_string())),
            source: Some(source::LINKS.to_string()),
            message,
            ..Default::default()
        });
    }
}

/// Headings whose visible text generates the same GitHub anchor slug.
fn duplicate_heading_diagnostics(
    ctx: &DocumentContext,
    doc_id: crate::graph::DocumentId,
    out: &mut Vec<Diagnostic>,
) {
    // Group heading occurrences (range + base slug) by base slug.
    let mut occurrences: Vec<(String, lsp_types::Range)> = Vec::new();
    for (_, node) in ctx.graph.headings(doc_id) {
        if let Some(heading) = node.as_heading()
            && let Some(range) = ctx.source_map.byte_range_to_lsp(heading.title_span.clone())
        {
            occurrences.push((generate_heading_slug(&heading.title), range));
        }
    }

    for index in 0..occurrences.len() {
        let (slug, range) = &occurrences[index];
        let twins: Vec<&lsp_types::Range> = occurrences
            .iter()
            .enumerate()
            .filter(|(other, (other_slug, _))| *other != index && other_slug == slug)
            .map(|(_, (_, other_range))| other_range)
            .collect();
        if twins.is_empty() {
            continue;
        }
        let related = twins
            .into_iter()
            .map(|twin| DiagnosticRelatedInformation {
                location: Location::new(ctx.uri.clone(), *twin),
                message: "conflicting heading".to_string(),
            })
            .collect();
        out.push(Diagnostic {
            range: *range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String(code::DUPLICATE_HEADING.to_string())),
            source: Some(source::LINKS.to_string()),
            message: format!("duplicate heading slug `{slug}`: anchors are disambiguated `-1`, `-2`, …"),
            related_information: Some(related),
            ..Default::default()
        });
    }
}
