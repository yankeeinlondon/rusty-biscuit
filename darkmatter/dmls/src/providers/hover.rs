//! Hover: heading anchors and link-target previews.
//!
//! Content is text-first Markdown everywhere (R-7 hover profile); a link's
//! preview (target title, heading, resolved path, existence) is drawn from the
//! graph, so the provider reads nothing from disk (the router parses an
//! unparsed target before hover runs). A target outside the graph's documents
//! shows only its path.
//!
//! A link resolved to a Markdown document also shows that document's
//! frontmatter summary (`excerpt`, `summary`, `description`, or `about`),
//! appended unmodified below the title, path, and heading line. The summary
//! is captured when the target is parsed, so it follows the target's open
//! buffer, disk refreshes, and deletion with the rest of its graph state.

use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Range};

use super::DocumentContext;
use super::definition::{link_at, resolved_file, resolved_targets};
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
        None => match resolved_file(ctx.graph, link_id) {
            Some(path) => format!("→ `{}`", path.display()),
            None => unresolved_hover(ctx, doc_id, target),
        },
    };
    Some(markdown_hover(ctx, span, value))
}

/// Hover for a link whose target resolved to a graph node.
fn resolved_hover(graph: &WorkspaceGraph, target: NodeId) -> String {
    let Some(node) = graph.node(target) else {
        return "Resolved link".to_string();
    };
    let title = document_title(graph, node.document).unwrap_or_else(|| "(untitled)".to_string());
    let record = graph.document(node.document);
    let path = record
        .map(|record| record.path.display().to_string())
        .unwrap_or_default();
    let mut value = match &node.payload {
        NodePayload::Heading(heading) => format!(
            "**{title}**\n\n→ `{path}`\n\nHeading: {}",
            heading.title
        ),
        _ => format!("**{title}**\n\n→ `{path}`"),
    };
    if let Some(summary) = record.and_then(|record| record.summary) {
        value.push_str("\n\n");
        value.push_str(summary);
    }
    value
}

/// Hover for a link whose target did not resolve — explains why.
fn unresolved_hover(ctx: &DocumentContext, doc_id: DocumentId, target: &LinkTarget) -> String {
    match ctx.graph.diagnose_unresolved(doc_id, target, ctx.file_context()) {
        Some(LinkDiagnostic::BrokenPath { .. }) => {
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::context::test_support::{abs, workspace_contexts};
    use crate::graph::substrate::index_document;

    fn graph(docs: &[(&str, &str)]) -> WorkspaceGraph {
        let mut indices = BTreeMap::new();
        for (path, source) in docs {
            let path = abs(path);
            indices.insert(path.clone(), index_document(&path, source));
        }
        WorkspaceGraph::build(&indices, 1, &workspace_contexts())
    }

    /// The hover of a link to `target.md`'s document node, or to its heading
    /// with `slug`.
    fn target_hover(target_source: &str, slug: Option<&str>) -> String {
        let g = graph(&[("/w/target.md", target_source)]);
        let doc = g.document_id(&abs("/w/target.md")).unwrap();
        let node = match slug {
            None => g.document(doc).unwrap().root,
            Some(slug) => g
                .headings(doc)
                .find(|(_, node)| node.as_heading().is_some_and(|heading| heading.slug == slug))
                .map(|(id, _)| id)
                .unwrap(),
        };
        resolved_hover(&g, node)
    }

    fn path() -> String {
        abs("/w/target.md").display().to_string()
    }

    #[test]
    fn a_document_target_appends_each_summary_field() {
        for field in ["excerpt", "summary", "description", "about"] {
            let hover = target_hover(&format!("---\n{field}: The *gist*.\n---\n# Target\n"), None);
            assert_eq!(hover, format!("**Target**\n\n→ `{}`\n\nThe *gist*.", path()), "{field}");
        }
    }

    #[test]
    fn the_highest_precedence_summary_is_shown() {
        let source = "---\nabout: About\ndescription: Description\nsummary: Summary\nexcerpt: Excerpt\n---\n# Target\n";
        assert!(target_hover(source, None).ends_with("\n\nExcerpt"));
        let source = "---\nabout: About\ndescription: Description\n---\n# Target\n";
        assert!(target_hover(source, None).ends_with("\n\nDescription"));
    }

    #[test]
    fn an_unusable_summary_leaves_the_hover_byte_identical() {
        let plain = target_hover("# Target\n", None);
        assert_eq!(plain, format!("**Target**\n\n→ `{}`", path()));
        for frontmatter in ["summary: \"   \"", "summary: 42", "summary: [a]", "summary: null", "summary: \"{{ x }}\"", "summary: $(cmd)"] {
            let hover = target_hover(&format!("---\n{frontmatter}\n---\n# Target\n"), None);
            assert_eq!(hover, plain, "{frontmatter:?}");
        }
    }

    #[test]
    fn a_heading_target_keeps_its_heading_line_before_the_summary() {
        let source = "---\nsummary: Line one.\n---\n# Target\n\n## Install\n";
        assert_eq!(
            target_hover(source, Some("install")),
            format!("**Target**\n\n→ `{}`\n\nHeading: Install\n\nLine one.", path())
        );
        assert_eq!(
            target_hover("# Target\n\n## Install\n", Some("install")),
            format!("**Target**\n\n→ `{}`\n\nHeading: Install", path())
        );
    }
}
