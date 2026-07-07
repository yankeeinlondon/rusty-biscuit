//! References and same-document highlights, both from the single reverse index.
//!
//! A cursor on a heading answers "who links here" (its incoming `references`
//! edges); a cursor on a link answers the same question about the link's
//! target, so sibling links to the same destination surface together.

use lsp_types::{DocumentHighlight, DocumentHighlightKind, Location};

use super::DocumentContext;
use super::definition::{link_at, resolved_targets};
use super::location::node_location;
use crate::graph::{DocumentId, EdgeKind, NodeId};

/// Reference locations for the heading or link under `offset`.
pub fn references(ctx: &DocumentContext, offset: usize, include_declaration: bool) -> Vec<Location> {
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let Some(target) = symbol_target(ctx, doc_id, offset) else {
        return Vec::new();
    };
    let mut out: Vec<Location> = ctx
        .graph
        .incoming(target, EdgeKind::References)
        .map(|edge| edge.source)
        .filter_map(|source| node_location(ctx.graph, source))
        .collect();
    if include_declaration
        && let Some(declaration) = node_location(ctx.graph, target)
    {
        out.push(declaration);
    }
    out
}

/// Same-document highlights for the heading or link under `offset`.
pub fn document_highlights(ctx: &DocumentContext, offset: usize) -> Vec<DocumentHighlight> {
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let Some(target) = symbol_target(ctx, doc_id, offset) else {
        return Vec::new();
    };
    let mut out = Vec::new();

    // The declaration itself, when it lives in this document.
    if let Some(node) = ctx.graph.node(target)
        && node.document == doc_id
        && let Some(range) = ctx.source_map.byte_range_to_lsp(node.span.clone())
    {
        out.push(DocumentHighlight {
            range,
            kind: Some(DocumentHighlightKind::TEXT),
        });
    }

    // Every link in this document that resolves to the same target.
    for (link_id, node) in ctx.graph.links(doc_id) {
        if resolved_targets(ctx.graph, link_id).contains(&target)
            && let Some(range) = ctx.source_map.byte_range_to_lsp(node.span.clone())
        {
            out.push(DocumentHighlight {
                range,
                kind: Some(DocumentHighlightKind::READ),
            });
        }
    }
    out
}

/// The graph node a references/highlight request is "about": the heading under
/// the cursor, or the target of the link under the cursor.
fn symbol_target(ctx: &DocumentContext, doc_id: DocumentId, offset: usize) -> Option<NodeId> {
    if let Some((heading_id, _)) = heading_at(ctx, doc_id, offset) {
        return Some(heading_id);
    }
    let (link_id, _) = link_at(ctx, doc_id, offset)?;
    resolved_targets(ctx.graph, link_id).into_iter().next()
}

/// The heading node whose span contains `offset`.
fn heading_at(ctx: &DocumentContext, doc_id: DocumentId, offset: usize) -> Option<(NodeId, ())> {
    ctx.graph
        .headings(doc_id)
        .find(|(_, node)| node.span.contains(&offset))
        .map(|(id, _)| (id, ()))
}
