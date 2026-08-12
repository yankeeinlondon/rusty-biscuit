//! Completion: link paths, anchors, and fenced-code language tokens.
//!
//! The completion context is inferred from the text on the cursor's line: an
//! open link destination `](…` triggers path completion, a `#` inside it
//! switches to anchor completion against the referenced document, and an
//! opening code fence completes language tokens. Every item carries an eager
//! `textEdit` (never deferred to resolve — the Zed gate), so no snippet or
//! resolve support is required.

use std::path::Path;

use lsp_types::{CompletionItem, CompletionItemKind, CompletionTextEdit, TextEdit};

use super::DocumentContext;
use crate::graph::{DocumentId, WorkspaceGraph, normalize_join};

/// Fenced-code language tokens offered by completion (canonical
/// `LanguageGrammar` tokens plus a few common aliases). Each resolves through
/// `LanguageGrammar::from_token_or_plain_text` at fold time.
const LANGUAGE_TOKENS: &[&str] = &[
    "rust", "javascript", "typescript", "go", "php", "python", "bash", "sh", "html", "css",
    "markdown", "yaml", "json", "toml", "text",
];

/// Completion items at `offset`.
pub fn completion(ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
    let Some(doc_id) = ctx.doc_id else {
        return Vec::new();
    };
    let line_start = ctx.text[..offset].rfind('\n').map(|index| index + 1).unwrap_or(0);
    let prefix = &ctx.text[line_start..offset];

    if let Some(partial) = fence_language_partial(prefix) {
        return language_completions(ctx, offset, partial);
    }
    if let Some(dest) = link_destination_partial(prefix) {
        return match dest.rfind('#') {
            Some(hash) => anchor_completions(ctx, doc_id, offset, &dest[..hash], &dest[hash + 1..]),
            None => path_completions(ctx, doc_id, offset, dest),
        };
    }
    Vec::new()
}

/// The language token typed so far on an opening code-fence line, if the cursor
/// is still inside the info string's first token.
fn fence_language_partial(prefix: &str) -> Option<&str> {
    let trimmed = prefix.trim_start();
    let rest = trimmed
        .strip_prefix("```")
        .map(|r| r.trim_start_matches('`'))
        .or_else(|| trimmed.strip_prefix("~~~").map(|r| r.trim_start_matches('~')))?;
    (!rest.contains(char::is_whitespace)).then_some(rest)
}

/// The destination text typed so far inside an unclosed link `](…`.
fn link_destination_partial(prefix: &str) -> Option<&str> {
    let open = prefix.rfind("](")?;
    let dest = &prefix[open + 2..];
    (!dest.contains(')') && !dest.contains(char::is_whitespace)).then_some(dest)
}

/// Language-token completions filtered by the typed prefix.
fn language_completions(ctx: &DocumentContext, offset: usize, partial: &str) -> Vec<CompletionItem> {
    let start = offset - partial.len();
    LANGUAGE_TOKENS
        .iter()
        .filter(|token| token.starts_with(&partial.to_lowercase()))
        .filter_map(|token| {
            item(ctx, start, offset, token, token, CompletionItemKind::VALUE, None)
        })
        .collect()
}

/// Workspace document paths (relative to the current document), for path
/// completion inside a link destination.
fn path_completions(
    ctx: &DocumentContext,
    doc_id: DocumentId,
    offset: usize,
    partial: &str,
) -> Vec<CompletionItem> {
    let Some(base_dir) = ctx.path.parent() else {
        return Vec::new();
    };
    let start = offset - partial.len();
    let mut out = Vec::new();
    for record in ctx.graph.documents() {
        if ctx.graph.document_id(&record.path) == Some(doc_id) {
            continue;
        }
        let relative = relativize(base_dir, &record.path);
        if !partial.is_empty() && !relative.starts_with(partial) {
            continue;
        }
        if let Some(completion) =
            item(ctx, start, offset, &relative, &relative, CompletionItemKind::FILE, None)
        {
            out.push(completion);
        }
    }
    out
}

/// Heading-anchor completions for the referenced document (`path_part` empty →
/// the current document).
fn anchor_completions(
    ctx: &DocumentContext,
    doc_id: DocumentId,
    offset: usize,
    path_part: &str,
    anchor_partial: &str,
) -> Vec<CompletionItem> {
    let target_doc = if path_part.is_empty() {
        Some(doc_id)
    } else {
        ctx.path
            .parent()
            .map(|base| normalize_join(base, path_part))
            .and_then(|resolved| ctx.graph.document_id(&resolved))
    };
    let Some(target_doc) = target_doc else {
        return Vec::new();
    };
    let start = offset - anchor_partial.len();
    heading_slugs(ctx.graph, target_doc)
        .into_iter()
        .filter(|(slug, _)| slug.starts_with(anchor_partial))
        .filter_map(|(slug, title)| {
            item(ctx, start, offset, &slug, &slug, CompletionItemKind::REFERENCE, Some(title))
        })
        .collect()
}

/// `(slug, title)` for every heading in `document`.
fn heading_slugs(graph: &WorkspaceGraph, document: DocumentId) -> Vec<(String, String)> {
    graph
        .headings(document)
        .filter_map(|(_, node)| {
            node.as_heading().map(|heading| (heading.slug.clone(), heading.title.clone()))
        })
        .collect()
}

/// Builds a completion item with an eager text edit replacing `start..offset`.
fn item(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    label: &str,
    new_text: &str,
    kind: CompletionItemKind,
    detail: Option<String>,
) -> Option<CompletionItem> {
    let range = ctx.source_map.byte_range_to_lsp(start..offset)?;
    Some(CompletionItem {
        label: label.to_string(),
        kind: Some(kind),
        detail,
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range,
            new_text: new_text.to_string(),
        })),
        ..Default::default()
    })
}

/// Portable (forward-slash) relative path from `base_dir` to `target`,
/// computed lexically. Falls back to the target's file name when the paths
/// share no common root.
fn relativize(base_dir: &Path, target: &Path) -> String {
    use std::path::Component;

    let base: Vec<Component> = base_dir.components().collect();
    let dest: Vec<Component> = target.components().collect();
    let shared = base
        .iter()
        .zip(dest.iter())
        .take_while(|(a, b)| a == b)
        .count();

    // Roots differ entirely (e.g. different Windows drives): use the bare name.
    if shared == 0 {
        return target
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| target.to_string_lossy().to_string());
    }

    let mut segments: Vec<String> = Vec::new();
    for _ in shared..base.len() {
        segments.push("..".to_string());
    }
    for component in &dest[shared..] {
        segments.push(component.as_os_str().to_string_lossy().to_string());
    }
    if segments.is_empty() {
        ".".to_string()
    } else {
        segments.join("/")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_fence_language_partial() {
        assert_eq!(fence_language_partial("```ru"), Some("ru"));
        assert_eq!(fence_language_partial("   ~~~"), Some(""));
        assert_eq!(fence_language_partial("```rust title"), None);
        assert_eq!(fence_language_partial("text"), None);
    }

    #[test]
    fn test_link_destination_partial() {
        assert_eq!(link_destination_partial("see [x](gui"), Some("gui"));
        assert_eq!(link_destination_partial("[x](a.md#hea"), Some("a.md#hea"));
        assert_eq!(link_destination_partial("[x](done) more"), None);
        assert_eq!(link_destination_partial("no link here"), None);
    }

    #[test]
    fn test_relativize() {
        assert_eq!(
            relativize(Path::new("/w/docs"), &PathBuf::from("/w/docs/a.md")),
            "a.md"
        );
        assert_eq!(
            relativize(Path::new("/w/docs"), &PathBuf::from("/w/notes/b.md")),
            "../notes/b.md"
        );
    }
}
