//! The v1 code-action set.
//!
//! Actions are **diagnostic-driven**: each keys off a diagnostic the client
//! carries in the request context (matched by stable `code`), so an action only
//! appears where its problem does. Every edit is cheap (a lexical insertion or a
//! new-file template), so all are computed eagerly — nothing is deferred to
//! `codeAction/resolve` (Rule 2: there is nothing expensive to defer, so
//! `resolveProvider` stays off).
//!
//! The set (spec "Editing", plan Phase 10):
//!
//! - **create missing linked file / wiki note** — from a broken Markdown link
//!   or an unresolved wiki target; a `CreateFile` plus an `# H1` template.
//! - **add missing schema-required key** — from a missing-required diagnostic;
//!   inserts the key at the parent mapping.
//! - **migrate a deprecated `style:` key** — from a deprecated-style diagnostic;
//!   renames the key to its canonical spelling.
//! - **close an unclosed directive block** — from an unclosed-block diagnostic;
//!   appends the matching `::end-block` closer.

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, Diagnostic, NumberOrString, Position, Range,
    TextEdit, Uri,
};

use super::DocumentContext;
use super::edits::EditBuilder;
use crate::config::DmlsConfig;
use crate::diagnostics::codes::code;
use crate::graph::{DocumentId, LinkTarget, NodeId, WorkspaceGraph, normalize_join};
use crate::overlay::FrontmatterAst;
use crate::overlay::expressions;
use crate::workspace::file_path_to_uri;

/// Config category slug for a code action; an action is offered unless its
/// category is explicitly disabled in `code_actions.categories`.
mod category {
    pub const CREATE_FILE: &str = "create-missing-file";
    pub const ADD_KEY: &str = "add-missing-key";
    pub const MIGRATE_STYLE: &str = "migrate-deprecated-style";
    pub const CLOSE_BLOCK: &str = "close-directive-block";
    pub const WRAP_LITERAL: &str = "wrap-in-interpolation-literal";
}

/// Code actions for the current document, driven by the request-context
/// diagnostics.
pub fn code_actions(ctx: &DocumentContext, diagnostics: &[Diagnostic]) -> Vec<CodeActionOrCommand> {
    let mut out = Vec::new();
    let mut added_missing_keys = false;
    for diag in diagnostics {
        let Some(code_value) = string_code(diag) else {
            continue;
        };
        let action = match code_value {
            code::BROKEN_PATH if enabled(ctx.config, category::CREATE_FILE) => {
                create_missing_markdown_file(ctx, diag)
            }
            code::WIKI_UNRESOLVED_TARGET if enabled(ctx.config, category::CREATE_FILE) => {
                create_missing_wiki_note(ctx, diag)
            }
            code::SCHEMA_MISSING_REQUIRED
                if enabled(ctx.config, category::ADD_KEY) && !added_missing_keys =>
            {
                // All root missing-required diagnostics share the parent-mapping
                // range, so emit one "add all missing keys" action once.
                added_missing_keys = true;
                add_missing_required_keys(ctx, diagnostics)
            }
            code::STYLE_DEPRECATED_KEY if enabled(ctx.config, category::MIGRATE_STYLE) => {
                migrate_deprecated_style_key(ctx, diag)
            }
            code::DIRECTIVE_UNCLOSED_BLOCK if enabled(ctx.config, category::CLOSE_BLOCK) => {
                close_unclosed_block(ctx, diag)
            }
            code::EXPRESSION_MALFORMED
                if enabled(ctx.config, category::WRAP_LITERAL) =>
            {
                wrap_in_interpolation_literal(ctx, diag)
            }
            _ => None,
        };
        if let Some(action) = action {
            out.push(CodeActionOrCommand::CodeAction(action));
        }
    }
    out
}

/// Whether a code-action category is enabled (categories are on unless a config
/// entry disables them).
fn enabled(config: &DmlsConfig, category: &str) -> bool {
    config.code_actions.categories.get(category) != Some(&false)
}

/// The stable string code of a diagnostic, if it has one.
fn string_code(diag: &Diagnostic) -> Option<&str> {
    match &diag.code {
        Some(NumberOrString::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

// ── create missing file / wiki note ──

/// A "create missing file" action for a broken Markdown link.
fn create_missing_markdown_file(ctx: &DocumentContext, diag: &Diagnostic) -> Option<CodeAction> {
    if !ctx.profile.supports_resource_operations {
        return None;
    }
    let doc_id = ctx.doc_id?;
    let offset = ctx.source_map.lsp_to_byte(diag.range.start)?;
    let (_, target) = link_at(ctx.graph, doc_id, offset)?;
    let LinkTarget::RelativePath { path, .. } = target else {
        return None;
    };
    let base_dir = ctx.path.parent()?;
    let resolved = normalize_join(base_dir, path);
    if resolved.exists() {
        return None;
    }
    if !is_creatable_filename(&resolved) {
        return None;
    }
    let uri = file_path_to_uri(&resolved)?;
    let title = title_from_path(&resolved);
    Some(new_file_action(
        format!("Create file `{}`", display_target(path)),
        uri,
        &title,
        diag,
        ctx,
    ))
}

/// A "create missing wiki note" action for an unresolved `[[target]]`.
fn create_missing_wiki_note(ctx: &DocumentContext, diag: &Diagnostic) -> Option<CodeAction> {
    if !ctx.profile.supports_resource_operations {
        return None;
    }
    let doc_id = ctx.doc_id?;
    let offset = ctx.source_map.lsp_to_byte(diag.range.start)?;
    let payload = ctx
        .graph
        .wiki_links(doc_id)
        .find(|(_, node)| node.span.contains(&offset))
        .and_then(|(_, node)| node.as_wiki_link())?;
    // The note is created in the source document's directory (R-8: configured
    // new-note location falls back to the source directory), named after the
    // target's final segment.
    let basename = payload.target.rsplit('/').next().unwrap_or(&payload.target);
    if basename.is_empty() {
        return None;
    }
    let base_dir = ctx.path.parent()?;
    let filename = format!("{basename}.md");
    let resolved = base_dir.join(&filename);
    if resolved.exists() || !is_creatable_filename(&resolved) {
        return None;
    }
    let uri = file_path_to_uri(&resolved)?;
    Some(new_file_action(
        format!("Create wiki note `{basename}`"),
        uri,
        basename,
        diag,
        ctx,
    ))
}

/// Builds a create-file action inserting an `# H1` title into the new file.
fn new_file_action(
    action_title: String,
    file_uri: Uri,
    heading: &str,
    diag: &Diagnostic,
    ctx: &DocumentContext,
) -> CodeAction {
    let mut builder = EditBuilder::new();
    builder.create_file(file_uri.clone());
    builder.edit(
        file_uri,
        TextEdit {
            range: Range::new(Position::new(0, 0), Position::new(0, 0)),
            new_text: format!("# {heading}\n"),
        },
    );
    CodeAction {
        title: action_title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(builder.build(ctx.profile)),
        ..Default::default()
    }
}

// ── add missing schema-required keys ──

/// An action inserting every root-level missing-required key at the end of the
/// root frontmatter mapping.
fn add_missing_required_keys(ctx: &DocumentContext, diagnostics: &[Diagnostic]) -> Option<CodeAction> {
    let ast = ctx.overlay.and_then(|overlay| overlay.ast.as_deref())?;
    let mut keys: Vec<String> = diagnostics
        .iter()
        .filter(|diag| string_code(diag) == Some(code::SCHEMA_MISSING_REQUIRED))
        .filter_map(missing_key_from_message)
        .collect();
    keys.sort();
    keys.dedup();
    if keys.is_empty() {
        return None;
    }
    let insert_offset = root_insertion_offset(ast)?;
    let position = ctx.source_map.byte_to_lsp(insert_offset)?;
    let new_text: String = keys.iter().map(|key| format!("\n{key}: ")).collect();
    let related: Vec<Diagnostic> = diagnostics
        .iter()
        .filter(|diag| string_code(diag) == Some(code::SCHEMA_MISSING_REQUIRED))
        .cloned()
        .collect();
    let title = if keys.len() == 1 {
        format!("Add required key `{}`", keys[0])
    } else {
        format!("Add {} required keys", keys.len())
    };
    let mut builder = EditBuilder::new();
    builder.edit(
        ctx.uri.clone(),
        TextEdit {
            range: Range::new(position, position),
            new_text,
        },
    );
    Some(CodeAction {
        title,
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(related),
        edit: Some(builder.build(ctx.profile)),
        ..Default::default()
    })
}

/// The byte offset to insert a new top-level key at: just past the last
/// top-level entry's value (a fresh line follows), else the root mapping start.
fn root_insertion_offset(ast: &FrontmatterAst) -> Option<usize> {
    ast.top_level()
        .map(|entry| entry.value_span.end)
        .max()
        .or(Some(ast.root_span().start))
}

/// Extracts the missing key name from a validator message (`"key" is a required
/// property`), reading the first quoted or backtick-delimited token.
fn missing_key_from_message(diag: &Diagnostic) -> Option<String> {
    quoted_token(&diag.message, '"').or_else(|| quoted_token(&diag.message, '`'))
}

// ── migrate a deprecated style key ──

/// An action renaming a deprecated `style:` key to its canonical spelling.
fn migrate_deprecated_style_key(ctx: &DocumentContext, diag: &Diagnostic) -> Option<CodeAction> {
    let ast = ctx.overlay.and_then(|overlay| overlay.ast.as_deref())?;
    // The diagnostic message is `deprecated style key `path`; use `replacement``.
    let mut tokens = backtick_tokens(&diag.message);
    let path = tokens.next()?;
    let replacement = tokens.next()?;
    let new_key = replacement.rsplit('.').next().unwrap_or(&replacement).to_string();
    let entry = ast.entry_by_dotted(&path)?;
    let range = ctx.source_map.byte_range_to_lsp(entry.key_span.clone())?;
    let mut builder = EditBuilder::new();
    builder.edit(
        ctx.uri.clone(),
        TextEdit {
            range,
            new_text: new_key.clone(),
        },
    );
    Some(CodeAction {
        title: format!("Rename `{path}` to `{new_key}`"),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(builder.build(ctx.profile)),
        ..Default::default()
    })
}

// ── close an unclosed directive block ──

/// An action appending the matching `::end-block` closer for an unclosed
/// `::block` / `::shell-block`.
fn close_unclosed_block(ctx: &DocumentContext, diag: &Diagnostic) -> Option<CodeAction> {
    // The opener line is the diagnostic's range; mirror its block-quote marker.
    let opener_offset = ctx.source_map.lsp_to_byte(diag.range.start)?;
    let opener_line = line_at(ctx.text, opener_offset);
    let quote_prefix = if opener_line.trim_start().starts_with('>') {
        "> "
    } else {
        ""
    };
    let end = ctx.source_map.byte_to_lsp(ctx.text.len())?;
    // A closer must sit on its own line; add a leading newline when the buffer
    // does not already end with one.
    let leading = if ctx.text.ends_with('\n') || ctx.text.is_empty() {
        ""
    } else {
        "\n"
    };
    let new_text = format!("{leading}{quote_prefix}::end-block\n");
    let mut builder = EditBuilder::new();
    builder.edit(
        ctx.uri.clone(),
        TextEdit {
            range: Range::new(end, end),
            new_text,
        },
    );
    Some(CodeAction {
        title: "Close block with `::end-block`".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(builder.build(ctx.profile)),
        ..Default::default()
    })
}

// ── wrap an interpolation in a literal ──

/// A quick-fix that wraps the malformed `{{ … }}` span in triple braces so it
/// becomes an inert interpolation literal (`{{{ … }}}`).
fn wrap_in_interpolation_literal(ctx: &DocumentContext, diag: &Diagnostic) -> Option<CodeAction> {
    let offset = ctx.source_map.lsp_to_byte(diag.range.start)?;
    let body_base = super::dsl::body_base(ctx.text);
    let interpolation = expressions::interpolation_at(ctx.text, body_base, offset)?;
    let range = ctx.source_map.byte_range_to_lsp(interpolation.outer.clone())?;
    let new_text = format!("{{{}}}", &ctx.text[interpolation.outer]);
    let mut builder = EditBuilder::new();
    builder.edit(
        ctx.uri.clone(),
        TextEdit {
            range,
            new_text,
        },
    );
    Some(CodeAction {
        title: "Wrap in interpolation literal".to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diag.clone()]),
        edit: Some(builder.build(ctx.profile)),
        ..Default::default()
    })
}

// ── shared helpers ──

/// The link node whose span contains `offset`, with its target.
fn link_at(
    graph: &WorkspaceGraph,
    doc_id: DocumentId,
    offset: usize,
) -> Option<(NodeId, &LinkTarget)> {
    graph.links(doc_id).find_map(|(id, node)| {
        node.span
            .contains(&offset)
            .then(|| node.as_link().map(|link| (id, &link.target)))
            .flatten()
    })
}

/// A human-friendly H1 title derived from a path's file stem
/// (`my-notes.md` → `My notes`).
fn title_from_path(path: &std::path::Path) -> String {
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("Untitled");
    humanize(stem)
}

/// Turns a file-stem slug into a title-cased phrase.
fn humanize(stem: &str) -> String {
    let spaced = stem.replace(['-', '_'], " ");
    let mut chars = spaced.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => "Untitled".to_string(),
    }
}

/// The `#fragment`-stripped display of a link target path.
fn display_target(path: &str) -> &str {
    path.split('#').next().unwrap_or(path)
}

/// Whether a path's final segment is a filename DMLS may create on every OS
/// (R-8 cross-platform gotcha 4/5): no Windows-invalid characters, no `:`, and
/// no trailing space or dot.
fn is_creatable_filename(path: &std::path::Path) -> bool {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    if name.is_empty() {
        return false;
    }
    if name.ends_with(' ') || name.ends_with('.') {
        return false;
    }
    !name.chars().any(|ch| matches!(ch, '<' | '>' | ':' | '"' | '\\' | '|' | '?' | '*'))
}

/// The source line containing `offset` (no trailing newline).
fn line_at(text: &str, offset: usize) -> &str {
    let start = text[..offset].rfind('\n').map(|index| index + 1).unwrap_or(0);
    let end = text[offset..]
        .find('\n')
        .map(|index| offset + index)
        .unwrap_or(text.len());
    &text[start..end]
}

/// The first `delimiter`-quoted token in `text`.
fn quoted_token(text: &str, delimiter: char) -> Option<String> {
    let start = text.find(delimiter)? + delimiter.len_utf8();
    let end = text[start..].find(delimiter)? + start;
    Some(text[start..end].to_string())
}

/// An iterator over the backtick-delimited tokens in `text`.
fn backtick_tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split('`')
        .enumerate()
        // Odd-indexed segments sit between a pair of backticks.
        .filter(|(index, _)| index % 2 == 1)
        .map(|(_, segment)| segment.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_creatable_filename_rejects_windows_invalid() {
        assert!(is_creatable_filename(std::path::Path::new("/w/note.md")));
        assert!(!is_creatable_filename(std::path::Path::new("/w/no:te.md")));
        assert!(!is_creatable_filename(std::path::Path::new("/w/bad?.md")));
        assert!(!is_creatable_filename(std::path::Path::new("/w/pi|pe.md")));
    }

    #[test]
    fn test_is_creatable_rejects_trailing_space_or_dot() {
        assert!(!is_creatable_filename(std::path::Path::new("/w/name ")));
        assert!(!is_creatable_filename(std::path::Path::new("/w/name.")));
    }

    #[test]
    fn test_humanize_stem() {
        assert_eq!(humanize("my-getting-started"), "My getting started");
        assert_eq!(humanize("notes_index"), "Notes index");
    }

    #[test]
    fn test_quoted_token_and_backticks() {
        assert_eq!(
            quoted_token("\"title\" is a required property", '"').as_deref(),
            Some("title")
        );
        let tokens: Vec<String> =
            backtick_tokens("deprecated style key `style.page.max_width`; use `max-width`").collect();
        assert_eq!(tokens, vec!["style.page.max_width", "max-width"]);
    }

    #[test]
    fn test_line_at_finds_opener() {
        let text = "# Doc\n::block when=\"x\"\nbody\n";
        let offset = text.find("::block").unwrap();
        assert_eq!(line_at(text, offset), "::block when=\"x\"");
    }

    #[test]
    #[allow(clippy::mutable_key_type)]
    fn test_wrap_in_interpolation_literal_offers_quick_fix() {
        use std::collections::BTreeMap;
        use std::path::Path;
        use std::sync::Arc;

        use lsp_types::{NumberOrString, Uri};

        use crate::capabilities::{ClientProfile, HoverMediaProfile};
        use crate::graph::WorkspaceGraph;
        use crate::source_map::{PositionEncoding, SourceMap};

        let text = "See {{ > invalid }} text.\n";
        let uri: Uri = "file:///t.md".parse().unwrap();
        let path = Path::new("/t.md");
        let source_map =
            SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let graph = WorkspaceGraph::build_with_roots(&BTreeMap::new(), 0, &[]);
        let config = DmlsConfig::default();
        let profile = ClientProfile {
            client_name: None,
            client_version: None,
            position_encoding: PositionEncoding::Utf16,
            supports_resource_operations: false,
            supports_change_annotations: false,
            supports_code_action_resolve: false,
            supports_completion_resolve: false,
            resolve_provides_text_edit: false,
            supports_snippets: false,
            client_watches_files: false,
            needs_watch_fallback: false,
            supports_file_operations: false,
            supports_workspace_configuration: false,
            supports_folding: false,
            folding_line_only: false,
            supports_selection_range: false,
            supports_linked_editing: false,
            supports_work_done_progress: false,
            hover_media: HoverMediaProfile::default(),
            helix_one_char_selection_is_empty: false,
        };

        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: None,
        };

        let diagnostics = crate::providers::dsl::diagnostics(&ctx);
        let malformed = diagnostics
            .iter()
            .find(|d| {
                matches!(
                    &d.code,
                    Some(NumberOrString::String(s)) if s == code::EXPRESSION_MALFORMED
                )
            })
            .expect("malformed expression diagnostic")
            .clone();

        let actions = code_actions(&ctx, &[malformed]);
        let action = actions
            .iter()
            .find_map(|a| match a {
                CodeActionOrCommand::CodeAction(a)
                    if a.title == "Wrap in interpolation literal" =>
                {
                    Some(a)
                }
                _ => None,
            })
            .expect("wrap action");

        let edit = action.edit.as_ref().expect("edit");
        let changes = edit.changes.as_ref().expect("changes");
        let uri_edits = changes.get(&uri).expect("uri edits");
        assert_eq!(uri_edits.len(), 1);
        assert_eq!(uri_edits[0].new_text, "{{{ > invalid }}}");
    }
}
