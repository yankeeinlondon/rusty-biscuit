//! Definition navigation and document links.
//!
//! Both work off the substrate's `references` edges: the link node under the
//! cursor already carries a resolved target node (heading or document root) or
//! stays unresolved. Links inside code spans and fenced code blocks never reach
//! here because the CommonMark parser does not emit link events inside code, so
//! no suppression logic is needed.

use lsp_types::{DocumentLink, Location};

use super::DocumentContext;
use super::location::node_location;
use crate::graph::{EdgeKind, LinkTarget, NodeId, WorkspaceGraph};

/// Definition locations for a link under `offset` (heading, document, or —
/// when the link is ambiguous later — multiple locations).
pub fn definition(ctx: &DocumentContext, offset: usize) -> Vec<Location> {
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let Some((link_node, _)) = link_at(ctx, doc_id, offset) else {
        return Vec::new();
    };
    resolved_targets(ctx.graph, link_node)
        .into_iter()
        .filter_map(|target| node_location(ctx.graph, target))
        .collect()
}

/// Eagerly-resolved document links for every local/external link in the
/// document. Targets are cheap lexical path joins, so there is nothing worth
/// deferring to `documentLink/resolve` (advertised `resolveProvider: false`).
pub fn document_links(ctx: &DocumentContext) -> Vec<DocumentLink> {
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let mut links = Vec::new();
    for (node_id, node) in ctx.graph.links(doc_id) {
        let Some(payload) = node.as_link() else {
            continue;
        };
        let Some(range) = ctx.source_map.byte_range_to_lsp(node.span.clone()) else {
            continue;
        };
        let target = match &payload.target {
            LinkTarget::External => payload.raw_target.parse().ok(),
            LinkTarget::SameDocumentAnchor { slug } => {
                file_uri_with_fragment(ctx.path, Some(slug))
            }
            LinkTarget::RelativePath { fragment, .. } => {
                resolved_document_path(ctx.graph, node_id).and_then(|path| {
                    file_uri_with_fragment(&path, fragment.as_deref())
                })
            }
        };
        if let Some(target) = target {
            links.push(DocumentLink {
                range,
                target: Some(target),
                tooltip: None,
                data: None,
            });
        }
    }
    links
}

/// The link node whose span contains `offset`, with its payload target.
pub(super) fn link_at<'a>(
    ctx: &'a DocumentContext,
    doc_id: crate::graph::DocumentId,
    offset: usize,
) -> Option<(NodeId, &'a LinkTarget)> {
    ctx.graph.links(doc_id).find_map(|(id, node)| {
        (node.span.contains(&offset))
            .then(|| node.as_link().map(|link| (id, &link.target)))
            .flatten()
    })
}

/// Resolved target nodes of a link's `references` edges (usually one).
pub(super) fn resolved_targets(graph: &WorkspaceGraph, link: NodeId) -> Vec<NodeId> {
    graph
        .outgoing(link, EdgeKind::References)
        .filter_map(|edge| edge.target.node())
        .collect()
}

/// The filesystem path of a link's resolved target document, if any.
fn resolved_document_path(graph: &WorkspaceGraph, link: NodeId) -> Option<std::path::PathBuf> {
    let target = resolved_targets(graph, link).into_iter().next()?;
    let document = graph.node(target)?.document;
    graph.document(document).map(|record| record.path.clone())
}

/// Builds a `file://` URI for `path`, attaching `fragment` when present.
fn file_uri_with_fragment(path: &std::path::Path, fragment: Option<&str>) -> Option<lsp_types::Uri> {
    let mut url = url::Url::from_file_path(path).ok()?;
    if let Some(fragment) = fragment {
        url.set_fragment(Some(fragment));
    }
    url.as_str().parse().ok()
}
