//! Document symbols (heading hierarchy) and workspace symbols.
//!
//! Setext and ATX headings are already unified by the Phase-1 `extract_headings`
//! authority, so both appear here identically. The document outline nests by
//! heading level; workspace symbols expose document titles plus every heading,
//! subsequence-matched, from each document's headings alone (the index
//! store's stored headings, or a graph's through [`graph_symbol_documents`]).

use lsp_types::{
    DocumentSymbol, Location, OneOf, SymbolKind, WorkspaceSymbol,
};

use std::path::PathBuf;

use super::DocumentContext;
use super::location::line_range;
use crate::graph::{HeadingFact, HeadingPayload, WorkspaceGraph};
use crate::workspace::file_path_to_uri;

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

/// One document as workspace symbols read it: its path and its headings in
/// source order.
pub type SymbolDocument = (PathBuf, Vec<HeadingFact>);

/// Workspace symbols: document titles plus every heading, subsequence-matched
/// against `query` (case-insensitive; an empty query matches everything).
///
/// `documents` holds each document once; symbols come out in `documents`
/// order, so the registry's cap keeps the same ones on every call when the
/// caller sorts it. A title symbol sits on the document's first line and a
/// heading symbol on the heading's line, with the file name as container.
pub fn workspace_symbols(query: &str, documents: &[SymbolDocument]) -> Vec<WorkspaceSymbol> {
    let mut out = Vec::new();
    for (path, headings) in documents {
        let uri = file_path_to_uri(path).unwrap_or_else(dummy_uri);
        let file_name = path
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| path.to_string_lossy().to_string());
        let title = title_of(headings.iter().map(|heading| (heading.level, heading.title.as_str())))
            .unwrap_or_else(|| file_name.clone());

        if subsequence_match(query, &title) {
            out.push(symbol(title, SymbolKind::FILE, None, &uri, 1));
        }
        for heading in headings {
            if subsequence_match(query, &heading.title) {
                out.push(symbol(heading.title.clone(), SymbolKind::STRING, Some(file_name.clone()), &uri, heading.line));
            }
        }
    }
    out
}

/// The [`SymbolDocument`]s of `graph`: every document that is not a
/// referenced dependency, in document order.
pub fn graph_symbol_documents(graph: &WorkspaceGraph) -> Vec<SymbolDocument> {
    let mut out = Vec::new();
    for record in graph.documents().filter(|record| !record.dependency) {
        let Some(doc_id) = graph.document_id(record.path) else {
            continue;
        };
        let headings = graph
            .headings(doc_id)
            .filter_map(|(_, node)| {
                let heading = node.as_heading()?;
                Some(HeadingFact {
                    level: heading.level,
                    title: heading.title.clone(),
                    slug: heading.slug.clone(),
                    span: node.span.clone(),
                    title_span: heading.title_span.clone(),
                    line: heading.line,
                })
            })
            .collect();
        out.push((record.path.to_path_buf(), headings));
    }
    out
}

/// The document's title: its first level-1 heading, else its first heading.
pub(super) fn document_title(graph: &WorkspaceGraph, doc_id: crate::graph::DocumentId) -> Option<String> {
    title_of(
        graph
            .headings(doc_id)
            .filter_map(|(_, node)| node.as_heading())
            .map(|heading| (heading.level, heading.title.as_str())),
    )
}

/// The first level-1 title among `(level, title)` headings, else the first.
fn title_of<'a>(headings: impl Iterator<Item = (u8, &'a str)>) -> Option<String> {
    let mut first: Option<&str> = None;
    for (level, title) in headings {
        if level == 1 {
            return Some(title.to_string());
        }
        first.get_or_insert(title);
    }
    first.map(str::to_string)
}

fn symbol(
    name: String,
    kind: SymbolKind,
    container_name: Option<String>,
    uri: &lsp_types::Uri,
    line: usize,
) -> WorkspaceSymbol {
    WorkspaceSymbol {
        name,
        kind,
        tags: None,
        container_name,
        location: OneOf::Left(Location::new(uri.clone(), line_range(line))),
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
