//! Safe rename for heading anchors and files (spec criterion 9, R-8 rename
//! rules).
//!
//! Two rename surfaces:
//!
//! - **Heading rename** (`prepareRename` + `rename`): rewrites the heading and
//!   every Markdown `#slug` and wiki `#heading` reference that uniquely resolves
//!   to it, preserving each link's spelling class (text-form links get the new
//!   text, slug-form links get the new slug). Refused when duplicate headings
//!   make the reference ambiguous.
//! - **File rename** (`workspace/willRenameFiles`): the R-8 simulate-post-rename
//!   algorithm — a wiki link is rewritten only when it resolved uniquely to the
//!   old file *and* a replacement spelling resolves uniquely to the new file in
//!   the simulated post-rename index; Markdown links to the file are re-pathed
//!   relative to their document. The whole operation is **atomic**: if any
//!   participating wiki link cannot be safely rewritten, or the destination
//!   already exists, DMLS produces *no* edits rather than a partial rename.
//!
//! Cross-document edits need precise ranges in documents other than the active
//! buffer, so each affected document is loaded (open buffer first, else disk)
//! and given its own [`SourceMap`]. Resource operations and change annotations
//! follow the client profile through [`EditBuilder`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use darkmatter::markdown::generate_heading_slug;
use lsp_types::{FileRename, Position, PrepareRenameResponse, TextEdit, Uri, WorkspaceEdit};

use super::edits::EditBuilder;
use crate::capabilities::ClientProfile;
use crate::config::{DmlsConfig, WikiPathStyle};
use crate::graph::{
    DocumentId, EdgeKind, HeadingPayload, NodeId, NodeKind, WikiLinkPayload, WikiResolution,
    WorkspaceGraph,
};
use crate::source_map::{PositionEncoding, SourceMap};
use crate::wiki::{self, Match, ParseOutcome, WikiDoc, shortest_unique_suffix};
use crate::workspace::{DocumentStore, file_path_to_uri, resolve_wiki_roots};

/// Everything the rename providers need beyond the request payload: the open
/// buffers, the current graph snapshot, config/profile, workspace roots, and the
/// negotiated encoding (for building source maps of closed documents).
pub struct RenameEnv<'a> {
    /// Open-buffer store (authoritative over disk).
    pub documents: &'a DocumentStore,
    /// The current workspace graph snapshot.
    pub graph: &'a WorkspaceGraph,
    /// Effective configuration.
    pub config: &'a DmlsConfig,
    /// Per-client capability gates.
    pub profile: &'a ClientProfile,
    /// Workspace roots (for wiki-root resolution of the renamed path).
    pub roots: &'a [PathBuf],
    /// Negotiated position encoding.
    pub encoding: PositionEncoding,
}

/// Why a rename could not proceed. The router maps each to an LSP error so the
/// client shows the reason instead of applying a partial edit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenameError {
    /// The cursor is not on a renameable construct.
    NotRenameable,
    /// Duplicate headings make the reference ambiguous (R-8 heading rule 6).
    AmbiguousHeading,
}

impl RenameError {
    /// A user-facing message for the LSP error response.
    pub fn message(&self) -> &'static str {
        match self {
            RenameError::NotRenameable => "no renameable symbol at this position",
            RenameError::AmbiguousHeading => {
                "cannot rename: duplicate headings make this reference ambiguous"
            }
        }
    }
}

/// `prepareRename`: allow a rename only when the cursor is on a heading whose
/// slug is unambiguous, returning the heading's text range and current title.
pub fn prepare_rename(
    env: &RenameEnv,
    uri: &Uri,
    position: Position,
) -> Option<PrepareRenameResponse> {
    let path = crate::workspace::uri_to_file_path(uri)?;
    let (_, map) = document_source(env, &path)?;
    let offset = map.lsp_to_byte(position)?;
    let doc_id = env.graph.document_id(&path)?;
    let (_, heading) = heading_at(env.graph, doc_id, offset)?;
    if heading_is_ambiguous(env.graph, doc_id, &heading.title) {
        return None;
    }
    let range = map.byte_range_to_lsp(heading.title_span.clone())?;
    Some(PrepareRenameResponse::RangeWithPlaceholder {
        range,
        placeholder: heading.title.clone(),
    })
}

/// `rename`: rewrite a heading and all of its unique references.
///
/// ## Errors
///
/// [`RenameError::NotRenameable`] when the cursor is not on a heading, and
/// [`RenameError::AmbiguousHeading`] when duplicate headings make the reference
/// ambiguous (R-8 heading rule 6) — in both cases no edit is produced.
pub fn rename(
    env: &RenameEnv,
    uri: &Uri,
    position: Position,
    new_name: &str,
) -> Result<WorkspaceEdit, RenameError> {
    let path = crate::workspace::uri_to_file_path(uri).ok_or(RenameError::NotRenameable)?;
    let (_, map) = document_source(env, &path).ok_or(RenameError::NotRenameable)?;
    let offset = map.lsp_to_byte(position).ok_or(RenameError::NotRenameable)?;
    let doc_id = env.graph.document_id(&path).ok_or(RenameError::NotRenameable)?;
    let (heading_node, heading) =
        heading_at(env.graph, doc_id, offset).ok_or(RenameError::NotRenameable)?;
    if heading_is_ambiguous(env.graph, doc_id, &heading.title) {
        return Err(RenameError::AmbiguousHeading);
    }

    let old_slug = heading.slug.clone();
    let old_title = heading.title.clone();
    let new_slug = generate_heading_slug(new_name);

    let mut builder = EditBuilder::new();

    // 1. The heading itself, in its own document.
    if let (Some(uri), Some(range)) = (
        file_path_to_uri(&path),
        map.byte_range_to_lsp(heading.title_span.clone()),
    ) {
        builder.edit(
            uri,
            TextEdit {
                range,
                new_text: new_name.to_string(),
            },
        );
    }

    // 2. Every reference that uniquely resolved to this heading.
    for edge in env.graph.incoming(heading_node, EdgeKind::References) {
        rewrite_heading_reference(
            env,
            edge.source,
            &old_slug,
            &old_title,
            &new_slug,
            new_name,
            &mut builder,
        );
    }

    Ok(builder.build(env.profile))
}

/// Rewrites one heading reference (Markdown `#slug` or wiki `#heading`),
/// preserving its spelling class.
fn rewrite_heading_reference(
    env: &RenameEnv,
    source: NodeId,
    old_slug: &str,
    old_title: &str,
    new_slug: &str,
    new_title: &str,
    builder: &mut EditBuilder,
) {
    let Some(node) = env.graph.node(source) else {
        return;
    };
    let Some(record) = env.graph.document(node.document) else {
        return;
    };
    let Some((text, map)) = document_source(env, &record.path) else {
        return;
    };
    let Some(uri) = file_path_to_uri(&record.path) else {
        return;
    };

    match node.kind {
        NodeKind::Link => {
            // Markdown links always use slug anchors; replace `#old` → `#new`
            // inside the link's destination.
            let slice = &text[node.span.clone()];
            let needle = format!("#{old_slug}");
            if let Some(local) = slice.rfind(&needle) {
                let start = node.span.start + local + 1; // skip the `#`
                let end = start + old_slug.len();
                if let Some(range) = map.byte_range_to_lsp(start..end) {
                    builder.edit(
                        uri,
                        TextEdit {
                            range,
                            new_text: new_slug.to_string(),
                        },
                    );
                }
            }
        }
        NodeKind::WikiLink => {
            let Some(payload) = node.as_wiki_link() else {
                return;
            };
            let Some(heading_span) = payload.heading_span.clone() else {
                return;
            };
            let current = &text[heading_span.clone()];
            // Preserve the spelling class: an exact-text link gets the new text,
            // a slug-form link gets the new slug. A spelling that matches neither
            // is left untouched (it did not uniquely name this heading by name).
            let new_text = if wiki::nfc(current) == wiki::nfc(old_title) {
                Some(new_title.to_string())
            } else if current == old_slug {
                Some(new_slug.to_string())
            } else {
                None
            };
            if let (Some(new_text), Some(range)) =
                (new_text, map.byte_range_to_lsp(heading_span))
            {
                builder.edit(uri, TextEdit { range, new_text });
            }
        }
        _ => {}
    }
}

/// `workspace/willRenameFiles`: the reference-update `WorkspaceEdit` for one or
/// more file renames, or `None` when there is nothing safe to do.
pub fn will_rename_files(env: &RenameEnv, files: &[FileRename]) -> Option<WorkspaceEdit> {
    let mut builder = EditBuilder::new();
    for file in files {
        if !apply_file_rename(env, file, &mut builder) {
            // Any unsafe rename aborts the whole operation — no partial edits.
            return None;
        }
    }
    if builder.is_empty() {
        return None;
    }
    Some(builder.build(env.profile))
}

/// Accumulates the reference edits for one file rename, returning `false` (abort)
/// when the rename is unsafe (destination exists, or a participating wiki link
/// cannot be uniquely re-spelled).
fn apply_file_rename(env: &RenameEnv, file: &FileRename, builder: &mut EditBuilder) -> bool {
    let (Some(old_path), Some(new_path)) = (
        uri_str_to_path(&file.old_uri),
        uri_str_to_path(&file.new_uri),
    ) else {
        return true; // not a file:// rename we can reason about; nothing to do
    };
    let Some(old_doc) = env.graph.document_id(&old_path) else {
        return true; // the renamed file is not an indexed document
    };
    // Filesystem/index conflict: the destination is already a tracked document.
    if env.graph.document_id(&new_path).is_some() {
        return false;
    }

    // The post-rename wiki universe: the old document's slot now holds the new
    // path's canonical logical path.
    let wiki_roots = resolve_wiki_roots(env.roots, env.config.wiki.wiki_root.as_deref());
    let mut post_docs = env.graph.wiki_docs();
    let old_index = old_doc.0 as usize;
    if let Some(slot) = post_docs.get_mut(old_index) {
        *slot = wiki_doc_for(&new_path, &wiki_roots);
    }

    // Rewrite every wiki link that uniquely resolved to the old document.
    for document in 0..env.graph.document_count() {
        let doc_id = DocumentId(document as u32);
        for (node_id, node) in env.graph.wiki_links(doc_id) {
            let Some(payload) = node.as_wiki_link() else {
                continue;
            };
            if !resolves_uniquely_to(env.graph, payload, old_doc) {
                continue;
            }
            let _ = node_id;
            let source = document;
            let Some(spelling) = safe_new_spelling(&post_docs, old_index, source, env.config) else {
                return false; // no unique replacement spelling → abort
            };
            if !push_target_edit(env, node.document, &payload.target_span, &spelling, builder) {
                return false;
            }
        }
    }

    // Best-effort Markdown link re-pathing (always unambiguous by path, so it
    // never aborts the rename).
    rewrite_markdown_links_to(env, old_doc, &new_path, builder);
    true
}

/// Whether a wiki link resolved uniquely to `target` document.
fn resolves_uniquely_to(
    graph: &WorkspaceGraph,
    payload: &WikiLinkPayload,
    target: DocumentId,
) -> bool {
    let node = match &payload.resolution {
        WikiResolution::Resolved(node)
        | WikiResolution::HeadingMissing(node)
        | WikiResolution::EmptyHeading(Some(node)) => *node,
        _ => return false,
    };
    graph
        .node(node)
        .map(|node| node.document == target)
        .unwrap_or(false)
}

/// The preferred replacement spelling for `target` from `source` under the
/// configured path style, verified unique in the post-rename index — escalating
/// to the shortest unique suffix, and returning `None` when none is unique.
fn safe_new_spelling(
    docs: &[WikiDoc],
    target: usize,
    source: usize,
    config: &DmlsConfig,
) -> Option<String> {
    let preferred = preferred_spelling(docs, target, source, config.wiki.path_style);
    if resolves_spelling(docs, &preferred, source) == Some(target) {
        return Some(preferred);
    }
    let fallback = shortest_unique_suffix(docs, target, source).join("/");
    (resolves_spelling(docs, &fallback, source) == Some(target)).then_some(fallback)
}

/// The insertion spelling for `target` from `source` under `style`.
fn preferred_spelling(
    docs: &[WikiDoc],
    target: usize,
    source: usize,
    style: WikiPathStyle,
) -> String {
    super::wiki::insertion::insertion_path(docs, target, source, style)
}

/// The document a spelling resolves to from `source`, when it is unique.
fn resolves_spelling(docs: &[WikiDoc], spelling: &str, source: usize) -> Option<usize> {
    match wiki::parse_file_target(spelling) {
        ParseOutcome::Target(parsed) => match wiki::resolve_file(&parsed, docs, source) {
            Match::One(index) => Some(index),
            _ => None,
        },
        _ => None,
    }
}

/// Records a text edit replacing a wiki target span with `spelling`.
fn push_target_edit(
    env: &RenameEnv,
    document: DocumentId,
    target_span: &std::ops::Range<usize>,
    spelling: &str,
    builder: &mut EditBuilder,
) -> bool {
    let Some(record) = env.graph.document(document) else {
        return true;
    };
    let Some((_, map)) = document_source(env, &record.path) else {
        return true;
    };
    let Some(uri) = file_path_to_uri(&record.path) else {
        return true;
    };
    match map.byte_range_to_lsp(target_span.clone()) {
        Some(range) => {
            builder.edit(
                uri,
                TextEdit {
                    range,
                    new_text: spelling.to_string(),
                },
            );
            true
        }
        None => true,
    }
}

/// Re-paths every Markdown link that resolved to `old_doc` so it points at
/// `new_path` relative to the referencing document.
fn rewrite_markdown_links_to(
    env: &RenameEnv,
    old_doc: DocumentId,
    new_path: &Path,
    builder: &mut EditBuilder,
) {
    let Some(old_root) = env.graph.document(old_doc).map(|record| record.root) else {
        return;
    };
    for edge in env.graph.incoming(old_root, EdgeKind::References) {
        let source = edge.source;
        let Some(node) = env.graph.node(source) else {
            continue;
        };
        if node.kind != NodeKind::Link {
            continue;
        }
        let Some(payload) = node.as_link() else {
            continue;
        };
        let Some(record) = env.graph.document(node.document) else {
            continue;
        };
        let Some(source_dir) = record.path.parent() else {
            continue;
        };
        let Some(new_rel) = posix_relative(source_dir, new_path) else {
            continue;
        };
        // Replace only the path portion of the raw target (preserve any
        // `#fragment`).
        let old_path_portion = payload.raw_target.split('#').next().unwrap_or(&payload.raw_target);
        let Some((text, map)) = document_source(env, &record.path) else {
            continue;
        };
        let Some(uri) = file_path_to_uri(&record.path) else {
            continue;
        };
        let slice = &text[node.span.clone()];
        if let Some(local) = slice.rfind(old_path_portion) {
            let start = node.span.start + local;
            let end = start + old_path_portion.len();
            if let Some(range) = map.byte_range_to_lsp(start..end) {
                builder.edit(
                    uri,
                    TextEdit {
                        range,
                        new_text: new_rel,
                    },
                );
            }
        }
    }
}

/// The heading node whose title span contains `offset`, with its payload.
fn heading_at(
    graph: &WorkspaceGraph,
    doc_id: DocumentId,
    offset: usize,
) -> Option<(NodeId, &HeadingPayload)> {
    graph.headings(doc_id).find_map(|(id, node)| {
        let heading = node.as_heading()?;
        (heading.title_span.contains(&offset) || node.span.contains(&offset)).then_some((id, heading))
    })
}

/// Whether another heading in the document shares `title`'s exact text — the
/// R-8 heading-rename ambiguity condition.
fn heading_is_ambiguous(graph: &WorkspaceGraph, doc_id: DocumentId, title: &str) -> bool {
    graph
        .headings(doc_id)
        .filter(|(_, node)| node.as_heading().is_some_and(|heading| heading.title == title))
        .count()
        > 1
}

/// Loads a document's text and a matching source map (open buffer first, else
/// disk).
fn document_source(env: &RenameEnv, path: &Path) -> Option<(String, SourceMap)> {
    let uri = file_path_to_uri(path)?;
    let text = match env.documents.get(&uri) {
        Some(open) => open.text().to_string(),
        None => std::fs::read_to_string(path).ok()?,
    };
    let map = SourceMap::new(uri, 0, env.encoding, Arc::from(text.as_str()));
    Some((text, map))
}

/// A [`WikiDoc`] for `path` against the wiki roots (R-8 canonicalization).
fn wiki_doc_for(path: &Path, wiki_roots: &[PathBuf]) -> WikiDoc {
    for (index, root) in wiki_roots.iter().enumerate() {
        if path.starts_with(root) {
            return WikiDoc {
                canonical: wiki::canonical_segments(path, Some(root)),
                workspace_id: index,
            };
        }
    }
    WikiDoc {
        canonical: wiki::canonical_segments(path, None),
        workspace_id: WikiDoc::NO_ROOT,
    }
}

/// Parses a `file://` URI string into a path.
fn uri_str_to_path(uri: &str) -> Option<PathBuf> {
    let parsed: Uri = uri.parse().ok()?;
    crate::workspace::uri_to_file_path(&parsed)
}

/// A POSIX relative path from `from_dir` to `to_file` (with `..` as needed).
fn posix_relative(from_dir: &Path, to_file: &Path) -> Option<String> {
    let from: Vec<_> = from_dir.components().collect();
    let to: Vec<_> = to_file.components().collect();
    let common = from
        .iter()
        .zip(to.iter())
        .take_while(|(a, b)| a == b)
        .count();
    let ups = from.len() - common;
    let mut segments: Vec<String> = std::iter::repeat_n("..".to_string(), ups).collect();
    for component in &to[common..] {
        segments.push(component.as_os_str().to_str()?.to_string());
    }
    if segments.is_empty() {
        return None;
    }
    Some(segments.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_posix_relative_same_dir() {
        assert_eq!(
            posix_relative(Path::new("/w/notes"), Path::new("/w/notes/renamed.md")).as_deref(),
            Some("renamed.md")
        );
    }

    #[test]
    fn test_posix_relative_parent_traversal() {
        assert_eq!(
            posix_relative(Path::new("/w/docs"), Path::new("/w/notes/x.md")).as_deref(),
            Some("../notes/x.md")
        );
    }

    #[test]
    fn test_rename_error_messages() {
        assert!(!RenameError::NotRenameable.message().is_empty());
        assert!(!RenameError::AmbiguousHeading.message().is_empty());
    }
}
