//! Hover: heading anchors and link-target previews.
//!
//! Content is text-first Markdown everywhere (R-7 hover profile); a link's
//! preview (target title, heading, resolved path, existence) is drawn from the
//! already-indexed graph, so no target document is read from disk on hover.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};

use super::DocumentContext;
use super::definition::{link_at, resolved_targets};
use super::symbols::document_title;
use crate::graph::{DocumentId, LinkDiagnostic, LinkTarget, NodeId, NodePayload, WorkspaceGraph};

/// Hover for the heading or link under `offset`.
pub fn hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let doc_id = ctx.doc_id?;
    if let Some((span, value)) = heading_hover(ctx, doc_id, offset) {
        return Some(markdown_hover(ctx, span, value));
    }
    link_hover(ctx, doc_id, offset)
}

/// Hover content for a heading under the cursor.
fn heading_hover(
    ctx: &DocumentContext,
    doc_id: DocumentId,
    offset: usize,
) -> Option<(std::ops::Range<usize>, String)> {
    ctx.graph.headings(doc_id).find_map(|(_, node)| {
        let heading = node.as_heading()?;
        node.span.contains(&offset).then(|| {
            (
                node.span.clone(),
                format!("**Heading** (H{})\n\nAnchor: `#{}`", heading.level, heading.slug),
            )
        })
    })
}

/// Hover content for a link under the cursor.
fn link_hover(ctx: &DocumentContext, doc_id: DocumentId, offset: usize) -> Option<Hover> {
    let (link_id, target) = link_at(ctx, doc_id, offset)?;
    if matches!(target, LinkTarget::External) {
        return None;
    }
    let span = ctx.graph.node(link_id)?.span.clone();
    let value = match resolved_targets(ctx.graph, link_id).into_iter().next() {
        Some(node) => resolved_hover(ctx.graph, node),
        None => unresolved_hover(ctx.graph, doc_id, target),
    };
    Some(markdown_hover(ctx, span, value))
}

/// Hover for a link whose target resolved to a graph node.
fn resolved_hover(graph: &WorkspaceGraph, target: NodeId) -> String {
    let Some(node) = graph.node(target) else {
        return "Resolved link".to_string();
    };
    let title = document_title(graph, node.document).unwrap_or_else(|| "(untitled)".to_string());
    let path = graph
        .document(node.document)
        .map(|record| record.path.display().to_string())
        .unwrap_or_default();
    match &node.payload {
        NodePayload::Heading(heading) => format!(
            "**{title}**\n\n→ `{path}`\n\nHeading: {}",
            heading.title
        ),
        _ => format!("**{title}**\n\n→ `{path}`"),
    }
}

/// Hover for a link whose target did not resolve — explains why.
fn unresolved_hover(graph: &WorkspaceGraph, doc_id: DocumentId, target: &LinkTarget) -> String {
    match graph.diagnose_unresolved(doc_id, target) {
        Some(LinkDiagnostic::BrokenPath) => {
            let path = match target {
                LinkTarget::RelativePath { path, .. } => path.as_str(),
                _ => "",
            };
            format!("⚠️ Unresolved link: no document matches `{path}`")
        }
        Some(LinkDiagnostic::MissingAnchor) => {
            "⚠️ Unresolved anchor: the target document has no matching heading".to_string()
        }
        None => "Link".to_string(),
    }
}

fn markdown_hover(ctx: &DocumentContext, span: std::ops::Range<usize>, value: String) -> Hover {
    let range: Option<Range> = ctx.source_map.byte_range_to_lsp(span);
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range,
    }
}
