//! Document symbols (heading hierarchy) and workspace symbols.
//!
//! Setext and ATX headings are already unified by the Phase-1 `extract_headings`
//! authority, so both appear here identically. The document outline nests by
//! heading level; workspace symbols expose document titles plus every heading,
//! subsequence-matched.

use lsp_types::{
    DocumentSymbol, Location, OneOf, SymbolKind, WorkspaceSymbol,
};

use super::DocumentContext;
use super::location::{line_range, node_location};
use crate::graph::{HeadingPayload, WorkspaceGraph};

/// One flattened heading, ready to nest.
struct Heading {
    level: u8,
    title: String,
    detail: Option<String>,
    range: lsp_types::Range,
    selection: lsp_types::Range,
}

/// Builds the nested heading outline for the current document.
pub fn document_symbols(ctx: &DocumentContext) -> Vec<DocumentSymbol> {
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };

    // Collect headings with their byte spans so section ranges can extend to
    // the next same-or-higher heading.
    let raw: Vec<(HeadingPayload, std::ops::Range<usize>)> = ctx
        .graph
        .headings(doc_id)
        .filter_map(|(_, node)| node.as_heading().map(|h| (h.clone(), node.span.clone())))
        .collect();

    let mut headings: Vec<Heading> = Vec::with_capacity(raw.len());
    for (position, (heading, span)) in raw.iter().enumerate() {
        // Section end: the start of the next heading whose level is <= this
        // one, else end of document.
        let section_end = raw
            .iter()
            .skip(position + 1)
            .find(|(next, _)| next.level <= heading.level)
            .map(|(_, next_span)| next_span.start)
            .unwrap_or(ctx.text.len());
        let Some(range) = ctx.source_map.byte_range_to_lsp(span.start..section_end) else {
            continue;
        };
        let selection = ctx
            .source_map
            .byte_range_to_lsp(heading.title_span.clone())
            .unwrap_or(range);
        headings.push(Heading {
            level: heading.level,
            title: heading.title.clone(),
            detail: Some(format!("H{}", heading.level)),
            range,
            selection,
        });
    }

    let mut cursor = 0;
    build_tree(&headings, &mut cursor, 0)
}

/// Consumes headings whose level is deeper than `parent_level` as a nested
/// list, recursing for each heading's own children.
fn build_tree(headings: &[Heading], cursor: &mut usize, parent_level: u8) -> Vec<DocumentSymbol> {
    let mut out = Vec::new();
    while *cursor < headings.len() {
        let level = headings[*cursor].level;
        if level <= parent_level {
            break;
        }
        let heading = &headings[*cursor];
        let (name, detail, range, selection) = (
            heading.title.clone(),
            heading.detail.clone(),
            heading.range,
            heading.selection,
        );
        *cursor += 1;
        let children = build_tree(headings, cursor, level);
        #[allow(deprecated)]
        out.push(DocumentSymbol {
            name,
            detail,
            kind: SymbolKind::STRING,
            tags: None,
            deprecated: None,
            range,
            selection_range: selection,
            children: (!children.is_empty()).then_some(children),
        });
    }
    out
}

/// Workspace symbols: document titles plus every heading, subsequence-matched
/// against `query` (case-insensitive; an empty query matches everything).
pub fn workspace_symbols(query: &str, graph: &WorkspaceGraph) -> Vec<WorkspaceSymbol> {
    let mut out = Vec::new();
    for record in graph.documents() {
        let Some(doc_id) = graph.document_id(&record.path) else {
            continue;
        };
        let file_name = record
            .path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| record.path.to_string_lossy().to_string());
        let title = document_title(graph, doc_id).unwrap_or_else(|| file_name.clone());

        if subsequence_match(query, &title) {
            out.push(symbol(title.clone(), SymbolKind::FILE, None, record.root, graph));
        }
        for (node_id, node) in graph.headings(doc_id) {
            if let Some(heading) = node.as_heading()
                && subsequence_match(query, &heading.title)
            {
                out.push(symbol(
                    heading.title.clone(),
                    SymbolKind::STRING,
                    Some(file_name.clone()),
                    node_id,
                    graph,
                ));
            }
        }
    }
    out
}

/// The document's title: its first level-1 heading, else its first heading.
pub(super) fn document_title(graph: &WorkspaceGraph, doc_id: crate::graph::DocumentId) -> Option<String> {
    let mut first: Option<String> = None;
    for (_, node) in graph.headings(doc_id) {
        if let Some(heading) = node.as_heading() {
            if heading.level == 1 {
                return Some(heading.title.clone());
            }
            first.get_or_insert_with(|| heading.title.clone());
        }
    }
    first
}

fn symbol(
    name: String,
    kind: SymbolKind,
    container_name: Option<String>,
    node: crate::graph::NodeId,
    graph: &WorkspaceGraph,
) -> WorkspaceSymbol {
    let location = node_location(graph, node)
        .unwrap_or_else(|| Location::new(dummy_uri(), line_range(1)));
    WorkspaceSymbol {
        name,
        kind,
        tags: None,
        container_name,
        location: OneOf::Left(location),
        data: None,
    }
}

/// A stand-in URI for the unreachable no-path case (documents always have a
/// resolvable path in practice; this keeps the type total).
fn dummy_uri() -> lsp_types::Uri {
    "file:///".parse().expect("static uri parses")
}

/// Case-insensitive subsequence test: every char of `query`, in order,
/// appears somewhere in `candidate`.
fn subsequence_match(query: &str, candidate: &str) -> bool {
    if query.is_empty() {
        return true;
    }
    let mut needle = query.chars().flat_map(char::to_lowercase).peekable();
    for haystack_char in candidate.chars().flat_map(char::to_lowercase) {
        if needle.peek() == Some(&haystack_char) {
            needle.next();
        }
    }
    needle.peek().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subsequence_match() {
        assert!(subsequence_match("gs", "Getting Started"));
        assert!(subsequence_match("", "anything"));
        assert!(subsequence_match("SETUP", "setup guide"));
        assert!(!subsequence_match("xyz", "setup"));
    }
}
