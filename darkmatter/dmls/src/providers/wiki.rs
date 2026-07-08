//! Layer-1 wiki-link provider (R-8).
//!
//! Wiki links become [`NodeKind::WikiLink`](crate::graph::NodeKind) nodes with
//! resolved `references` edges at snapshot assembly, so backlinks to a heading
//! or document already surface through the substrate provider's reverse-index
//! query. This provider answers the *cursor-on-a-wiki-link* half: definition,
//! hover, completion, document links, and the `wiki.*` diagnostic taxonomy —
//! all read from the pre-computed resolution on each node, never by touching
//! the filesystem.

use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Diagnostic,
    DiagnosticRelatedInformation, DiagnosticSeverity, DocumentHighlight, DocumentHighlightKind,
    DocumentLink, Hover, HoverContents, Location, MarkupContent, MarkupKind, NumberOrString,
    Position, Range, TextEdit,
};

use super::DocumentContext;
use super::definition::resolved_targets;
use super::location::node_location;
use crate::config::HeadingCompletionStyle;
use crate::diagnostics::codes::{code, source};
use crate::graph::{
    DocumentId, EdgeKind, NodeId, NodePayload, WikiInfo, WikiLinkPayload, WikiResolution,
    WorkspaceGraph,
};
use crate::wiki::{self, Match, ParseOutcome};

use insertion::insertion_path;

/// The wiki node whose span contains `offset`, with its payload.
fn wiki_link_at<'a>(
    ctx: &'a DocumentContext,
    doc_id: DocumentId,
    offset: usize,
) -> Option<(NodeId, &'a WikiLinkPayload)> {
    ctx.graph.wiki_links(doc_id).find_map(|(id, node)| {
        node.span
            .contains(&offset)
            .then(|| node.as_wiki_link().map(|payload| (id, payload)))
            .flatten()
    })
}

/// Definition locations for a wiki link under `offset` (the resolved target, or
/// every candidate for an ambiguous link).
pub fn definition(ctx: &DocumentContext, offset: usize) -> Vec<Location> {
    if !ctx.config.wiki.enable {
        return Vec::new();
    }
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let Some((node, _)) = wiki_link_at(ctx, doc_id, offset) else {
        return Vec::new();
    };
    resolved_targets(ctx.graph, node)
        .into_iter()
        .filter_map(|target| node_location(ctx.graph, target))
        .collect()
}

/// Eagerly-resolved document links for wiki links with a single target.
pub fn document_links(ctx: &DocumentContext) -> Vec<DocumentLink> {
    if !ctx.config.wiki.enable {
        return Vec::new();
    }
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (node_id, node) in ctx.graph.wiki_links(doc_id) {
        let single = match resolved_targets(ctx.graph, node_id).as_slice() {
            [target] => Some(*target),
            _ => None,
        };
        let Some(target) = single else { continue };
        let Some(range) = ctx.source_map.byte_range_to_lsp(node.span.clone()) else {
            continue;
        };
        let Some(location) = node_location(ctx.graph, target) else {
            continue;
        };
        out.push(DocumentLink {
            range,
            target: Some(location.uri),
            tooltip: None,
            data: None,
        });
    }
    out
}

/// References (backlinks) for the target of the wiki link under `offset`.
pub fn references(
    ctx: &DocumentContext,
    offset: usize,
    include_declaration: bool,
) -> Vec<Location> {
    if !ctx.config.wiki.enable {
        return Vec::new();
    }
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let Some((node, _)) = wiki_link_at(ctx, doc_id, offset) else {
        return Vec::new();
    };
    let Some(target) = resolved_targets(ctx.graph, node).into_iter().next() else {
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

/// Same-document highlights for the wiki link under `offset` and its siblings
/// that resolve to the same target.
pub fn document_highlights(ctx: &DocumentContext, offset: usize) -> Vec<DocumentHighlight> {
    if !ctx.config.wiki.enable {
        return Vec::new();
    }
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let Some((node, _)) = wiki_link_at(ctx, doc_id, offset) else {
        return Vec::new();
    };
    let Some(target) = resolved_targets(ctx.graph, node).into_iter().next() else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (other, node) in ctx.graph.wiki_links(doc_id) {
        if resolved_targets(ctx.graph, other).contains(&target)
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

/// Hover for the wiki link under `offset`, explaining its resolution.
pub fn hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    if !ctx.config.wiki.enable {
        return None;
    }
    let doc_id = ctx.doc_id?;
    let (node_id, payload) = wiki_link_at(ctx, doc_id, offset)?;
    let value = hover_value(ctx.graph, payload);
    let range = ctx
        .graph
        .node(node_id)
        .and_then(|node| ctx.source_map.byte_range_to_lsp(node.span.clone()));
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range,
    })
}

/// The Markdown hover body for a wiki link's resolution.
fn hover_value(graph: &WorkspaceGraph, payload: &WikiLinkPayload) -> String {
    let mut body = match &payload.resolution {
        WikiResolution::Resolved(target) => resolved_hover(graph, *target),
        WikiResolution::HeadingMissing(file) => {
            let path = document_path(graph, *file);
            format!("⚠️ Heading not found in wiki target\n\n→ `{path}`")
        }
        WikiResolution::Ambiguous(candidates) => {
            let mut lines = String::from("⚠️ **Ambiguous wiki target**\n");
            for candidate in candidates {
                lines.push_str(&format!("\n- `{}`", document_path(graph, *candidate)));
            }
            lines
        }
        WikiResolution::Unresolved => {
            format!("⚠️ Unresolved wiki target: `[[{}]]`", payload.target)
        }
        WikiResolution::EmptyTarget => "⚠️ Empty wiki target".to_string(),
        WikiResolution::EmptyHeading(_) => "Wiki heading target is empty".to_string(),
        WikiResolution::Unsupported => {
            "This wiki-link form is not supported by DMLS v1".to_string()
        }
    };
    if let Some(alias) = &payload.alias {
        body.push_str(&format!("\n\nAlias: **{alias}**"));
    }
    if let Some(info) = payload.info {
        body.push_str("\n\n");
        body.push_str(info_message(info));
    }
    body
}

/// Hover body for a wiki link that resolved to a document root or heading.
fn resolved_hover(graph: &WorkspaceGraph, target: NodeId) -> String {
    let Some(node) = graph.node(target) else {
        return "Resolved wiki link".to_string();
    };
    let path = document_path(graph, target);
    match &node.payload {
        NodePayload::Heading(heading) => {
            format!("**{}**\n\n→ `{path}`\n\nHeading: {}", heading.title, heading.title)
        }
        _ => {
            let title = super::symbols::document_title(graph, node.document)
                .unwrap_or_else(|| "(untitled)".to_string());
            format!("**{title}**\n\n→ `{path}`")
        }
    }
}

/// The display path of the document owning `node`.
fn document_path(graph: &WorkspaceGraph, node: NodeId) -> String {
    graph
        .node(node)
        .and_then(|node| graph.document(node.document))
        .map(|record| record.path.display().to_string())
        .unwrap_or_default()
}

/// Completion items for a wiki link being typed at `offset`.
pub fn completion(ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
    if !ctx.config.wiki.enable {
        return Vec::new();
    }
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let line_start = ctx.text[..offset].rfind('\n').map(|index| index + 1).unwrap_or(0);
    let prefix = &ctx.text[line_start..offset];
    let Some(context) = completion_context(prefix) else {
        return Vec::new();
    };
    match context {
        WikiCompletion::File(partial) => file_completions(ctx, doc_id, offset, partial),
        WikiCompletion::Heading { target, partial } => {
            heading_completions(ctx, doc_id, offset, target, partial)
        }
    }
}

/// What is being completed inside an open `[[…`.
enum WikiCompletion<'a> {
    /// A file target; `&str` is the text typed after `[[`.
    File(&'a str),
    /// A heading fragment; `target` is the file text before `#`.
    Heading { target: &'a str, partial: &'a str },
}

/// Classifies the completion context from the line prefix, or `None` when the
/// cursor is not inside an open wiki target (or is in the alias portion).
fn completion_context(prefix: &str) -> Option<WikiCompletion<'_>> {
    let open = prefix.rfind("[[")?;
    let rest = &prefix[open + 2..];
    if rest.contains("]]") || rest.contains('|') {
        return None;
    }
    match rest.rfind('#') {
        Some(hash) => Some(WikiCompletion::Heading {
            target: &rest[..hash],
            partial: &rest[hash + 1..],
        }),
        None => Some(WikiCompletion::File(rest)),
    }
}

/// File-target completions inserting per `wiki.path_style`.
fn file_completions(
    ctx: &DocumentContext,
    doc_id: DocumentId,
    offset: usize,
    partial: &str,
) -> Vec<CompletionItem> {
    let wiki_docs = ctx.graph.wiki_docs();
    let source = doc_id.0 as usize;
    let start = offset - partial.len();
    let Some(range) = ctx.source_map.byte_range_to_lsp(start..offset) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (index, record) in ctx.graph.documents().iter().enumerate() {
        if index == source {
            continue;
        }
        let insertion = insertion_path(&wiki_docs, index, source, ctx.config.wiki.path_style);
        if !partial.is_empty() && !insertion.starts_with(partial) {
            continue;
        }
        out.push(CompletionItem {
            label: insertion.clone(),
            kind: Some(CompletionItemKind::FILE),
            detail: Some(record.path.display().to_string()),
            text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                range,
                new_text: insertion,
            })),
            ..Default::default()
        });
    }
    out
}

/// Heading-fragment completions for the resolved target document (`target`
/// empty → the source document), inserting text or slug per config.
fn heading_completions(
    ctx: &DocumentContext,
    doc_id: DocumentId,
    offset: usize,
    target: &str,
    partial: &str,
) -> Vec<CompletionItem> {
    let target_doc = if target.is_empty() {
        Some(doc_id)
    } else {
        match wiki::parse_file_target(target) {
            ParseOutcome::Target(parsed) => {
                match wiki::resolve_file(&parsed, &ctx.graph.wiki_docs(), doc_id.0 as usize) {
                    Match::One(index) => Some(DocumentId(index as u32)),
                    _ => None,
                }
            }
            _ => None,
        }
    };
    let Some(target_doc) = target_doc else {
        return Vec::new();
    };
    let start = offset - partial.len();
    let Some(range) = ctx.source_map.byte_range_to_lsp(start..offset) else {
        return Vec::new();
    };
    let use_slug = matches!(
        ctx.config.wiki.heading_completion_style,
        HeadingCompletionStyle::Slug
    );
    ctx.graph
        .headings(target_doc)
        .filter_map(|(_, node)| node.as_heading())
        .filter_map(|heading| {
            let insertion = if use_slug { &heading.slug } else { &heading.title };
            if !partial.is_empty() && !insertion.starts_with(partial) {
                return None;
            }
            Some(CompletionItem {
                label: insertion.clone(),
                kind: Some(CompletionItemKind::REFERENCE),
                detail: Some(format!("H{}", heading.level)),
                text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                    range,
                    new_text: insertion.clone(),
                })),
                ..Default::default()
            })
        })
        .collect()
}

/// Diagnostics for every wiki link in the document plus workspace-scope
/// portability collisions.
pub fn diagnostics(ctx: &DocumentContext) -> Vec<Diagnostic> {
    if !ctx.config.wiki.enable {
        return Vec::new();
    }
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (_, node) in ctx.graph.wiki_links(doc_id) {
        let Some(payload) = node.as_wiki_link() else {
            continue;
        };
        let Some(range) = ctx.source_map.byte_range_to_lsp(node.span.clone()) else {
            continue;
        };
        push_resolution_diagnostic(ctx, payload, range, &mut out);
        push_info_diagnostic(payload, range, &mut out);
    }
    push_portability_diagnostic(ctx, doc_id, &mut out);
    out
}

/// The main resolution diagnostic for one wiki link.
fn push_resolution_diagnostic(
    ctx: &DocumentContext,
    payload: &WikiLinkPayload,
    range: Range,
    out: &mut Vec<Diagnostic>,
) {
    let (severity, code_value, message, related) = match &payload.resolution {
        WikiResolution::Resolved(_) => return,
        WikiResolution::Unresolved => (
            DiagnosticSeverity::WARNING,
            code::WIKI_UNRESOLVED_TARGET,
            format!("Unresolved wiki target: [[{}]]", payload.target),
            Vec::new(),
        ),
        WikiResolution::Ambiguous(candidates) => (
            DiagnosticSeverity::WARNING,
            code::WIKI_AMBIGUOUS_TARGET,
            format!(
                "Ambiguous wiki target: [[{}]] resolves to multiple files",
                payload.target
            ),
            candidate_related(ctx.graph, candidates, "candidate"),
        ),
        WikiResolution::HeadingMissing(file) => (
            DiagnosticSeverity::WARNING,
            code::WIKI_HEADING_MISSING,
            format!(
                "Heading not found in wiki target: #{}",
                payload.heading.as_deref().unwrap_or_default()
            ),
            candidate_related(ctx.graph, std::slice::from_ref(file), "target file"),
        ),
        WikiResolution::EmptyTarget => (
            DiagnosticSeverity::WARNING,
            code::WIKI_EMPTY_TARGET,
            "Wiki link target is empty".to_string(),
            Vec::new(),
        ),
        WikiResolution::EmptyHeading(_) => (
            DiagnosticSeverity::WARNING,
            code::WIKI_EMPTY_HEADING,
            "Wiki heading target is empty".to_string(),
            Vec::new(),
        ),
        WikiResolution::Unsupported => (
            DiagnosticSeverity::INFORMATION,
            code::WIKI_UNSUPPORTED_SYNTAX,
            "This wiki-link form is not supported by DMLS v1".to_string(),
            Vec::new(),
        ),
    };
    out.push(Diagnostic {
        range,
        severity: Some(severity),
        code: Some(NumberOrString::String(code_value.to_string())),
        source: Some(source::WIKI.to_string()),
        message,
        related_information: (!related.is_empty()).then_some(related),
        ..Default::default()
    });
}

/// The low-severity info diagnostic (percent escape, confusing extension,
/// heading spelling), when the link carries one.
fn push_info_diagnostic(payload: &WikiLinkPayload, range: Range, out: &mut Vec<Diagnostic>) {
    let Some(info) = payload.info else {
        return;
    };
    let code_value = match info {
        WikiInfo::InvalidPercentEscape => code::WIKI_INVALID_PERCENT_ESCAPE,
        WikiInfo::ConfusingExtension => code::WIKI_CONFUSING_EXTENSION,
        WikiInfo::HeadingSpellingConflict => code::WIKI_HEADING_SPELLING,
    };
    out.push(Diagnostic {
        range,
        severity: Some(DiagnosticSeverity::INFORMATION),
        code: Some(NumberOrString::String(code_value.to_string())),
        source: Some(source::WIKI.to_string()),
        message: info_message(info).to_string(),
        ..Default::default()
    });
}

/// A workspace-scope portability collision diagnostic when this document's
/// canonical logical path collides with another under case-fold/NFC.
fn push_portability_diagnostic(
    ctx: &DocumentContext,
    doc_id: DocumentId,
    out: &mut Vec<Diagnostic>,
) {
    let twins = ctx.graph.portability_twins(doc_id);
    if twins.is_empty() {
        return;
    }
    let related = candidate_related_docs(ctx.graph, twins, "colliding file");
    out.push(Diagnostic {
        range: Range::new(Position::new(0, 0), Position::new(0, 0)),
        severity: Some(DiagnosticSeverity::WARNING),
        code: Some(NumberOrString::String(
            code::WIKI_PORTABILITY_COLLISION.to_string(),
        )),
        source: Some(source::WIKI.to_string()),
        message: "Wiki target differs only by case or Unicode normalization and may not \
                  round-trip across macOS, Windows, and Linux"
            .to_string(),
        related_information: (!related.is_empty()).then_some(related),
        ..Default::default()
    });
}

/// Related-information entries for a set of candidate document roots.
fn candidate_related(
    graph: &WorkspaceGraph,
    candidates: &[NodeId],
    label: &str,
) -> Vec<DiagnosticRelatedInformation> {
    candidates
        .iter()
        .filter_map(|candidate| {
            node_location(graph, *candidate).map(|location| DiagnosticRelatedInformation {
                location,
                message: label.to_string(),
            })
        })
        .collect()
}

/// Related-information entries for a set of candidate documents (by id).
fn candidate_related_docs(
    graph: &WorkspaceGraph,
    documents: &[DocumentId],
    label: &str,
) -> Vec<DiagnosticRelatedInformation> {
    let roots: Vec<NodeId> = documents
        .iter()
        .filter_map(|doc| graph.document(*doc).map(|record| record.root))
        .collect();
    candidate_related(graph, &roots, label)
}

/// The user-facing message for a low-severity wiki note.
fn info_message(info: WikiInfo) -> &'static str {
    match info {
        WikiInfo::InvalidPercentEscape => {
            "Invalid percent escape in wiki target; treating it literally"
        }
        WikiInfo::ConfusingExtension => {
            "Wiki target resolved to a file whose name still ends in a Markdown extension"
        }
        WikiInfo::HeadingSpellingConflict => {
            "Heading resolved by exact text; a different heading matches by slug"
        }
    }
}

pub(crate) mod insertion {
    use crate::config::WikiPathStyle;
    use crate::wiki::{WikiDoc, shortest_unique_suffix};

    /// The insertion text for completing document `target` from `source` under
    /// `style` (never includes a Markdown extension).
    pub(crate) fn insertion_path(
        docs: &[WikiDoc],
        target: usize,
        source: usize,
        style: WikiPathStyle,
    ) -> String {
        match style {
            WikiPathStyle::Shortest => shortest_unique_suffix(docs, target, source).join("/"),
            WikiPathStyle::RootRelative => format!("/{}", docs[target].canonical.join("/")),
            WikiPathStyle::Relative => relative_or_shortest(docs, target, source),
        }
    }

    /// A POSIX relative insertion when it needs no `..`, else the shortest
    /// unique suffix (R-8 completion rule 6).
    fn relative_or_shortest(docs: &[WikiDoc], target: usize, source: usize) -> String {
        if docs[target].workspace_id == docs[source].workspace_id
            && let Some(relative) = relative_down_only(&docs[source].canonical, &docs[target].canonical)
        {
            return relative;
        }
        shortest_unique_suffix(docs, target, source).join("/")
    }

    /// The relative path from `source`'s directory to `target`, only when it
    /// descends (no `..`).
    fn relative_down_only(source: &[String], target: &[String]) -> Option<String> {
        let source_dir = &source[..source.len().saturating_sub(1)];
        let common = source_dir
            .iter()
            .zip(target.iter())
            .take_while(|(a, b)| a == b)
            .count();
        if common < source_dir.len() {
            return None; // would require `..`
        }
        Some(target[common..].join("/"))
    }
}
