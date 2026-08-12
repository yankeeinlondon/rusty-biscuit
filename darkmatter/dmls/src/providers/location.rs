//! Shared node → LSP location helpers.
//!
//! Cross-document navigation results (definition, references, workspace
//! symbols) are **line-based**: only the current open document carries a
//! source map, so a result in another document is anchored at its 1-indexed
//! line, column 0. That is navigable in every editor and needs no per-closed-
//! document source map. Same-document features that *do* have a source map
//! (folding, symbols, links, highlights, diagnostics) use precise ranges.

use lsp_types::{Location, Position, Range};

use crate::graph::{NodeId, NodePayload, WorkspaceGraph};
use crate::workspace::file_path_to_uri;

/// Zero-width LSP range at the start of a 1-indexed source `line`.
pub fn line_range(line: usize) -> Range {
    let zero_based = line.saturating_sub(1) as u32;
    Range::new(Position::new(zero_based, 0), Position::new(zero_based, 0))
}

/// The 1-indexed source line a node starts on.
pub fn node_line(graph: &WorkspaceGraph, id: NodeId) -> usize {
    match graph.node(id).map(|node| &node.payload) {
        Some(NodePayload::Heading(heading)) => heading.line,
        Some(NodePayload::Link(link)) => link.line,
        Some(NodePayload::WikiLink(wiki)) => wiki.line,
        Some(NodePayload::Transclusion(transclusion)) => transclusion.line,
        Some(NodePayload::FileRef(file)) => file.line,
        Some(NodePayload::VariableUse(variable)) => variable.line,
        Some(NodePayload::FrontmatterKey(key)) => key.line,
        _ => 1,
    }
}

/// A line-based [`Location`] for a graph node in any document.
///
/// ## Returns
///
/// `None` when the node or its document is absent, or the document path cannot
/// form a `file://` URI (non-absolute).
pub fn node_location(graph: &WorkspaceGraph, id: NodeId) -> Option<Location> {
    let node = graph.node(id)?;
    let record = graph.document(node.document)?;
    let uri = file_path_to_uri(&record.path)?;
    Some(Location::new(uri, line_range(node_line(graph, id))))
}
