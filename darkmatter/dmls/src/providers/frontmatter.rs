//! Layer-2 frontmatter provider: schema-driven completion, hover, navigation,
//! folding, symbols, and diagnostics.
//!
//! Every capability self-gates on the presence of a
//! [`DocumentOverlay`](crate::overlay::DocumentOverlay) and only fires when the
//! cursor is inside the frontmatter block, so it composes cleanly on top of the
//! substrate/wiki providers (which never match a frontmatter offset). The
//! effective schema (base baseline + extension baselines + document `$schema`)
//! is the semantic authority for keys, types, constraints, enums, and defaults;
//! the [`FrontmatterAst`] supplies every range. Completion items carry an eager
//! `textEdit` and no snippets (Zed-safe).

use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::context::context_variable_descriptors;
use darkmatter::markdown::schemas::{
    Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr,
    darkmatter_base_schema,
};
use darkmatter::markdown::span::SourceSpan;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Diagnostic, DocumentLink,
    DocumentSymbol, FoldingRange, Hover, HoverContents, Location, MarkupContent, MarkupKind, Range,
    SymbolKind, TextEdit,
};

use super::DocumentContext;
use crate::graph::normalize_join;
use crate::overlay::{FmEntry, FmValueKind, FrontmatterAst};
use crate::workspace::file_path_to_uri;

/// Frontmatter/schema diagnostics (delegated to the diagnostics module).
pub fn diagnostics(ctx: &DocumentContext) -> Vec<Diagnostic> {
    crate::diagnostics::frontmatter::diagnostics(ctx)
}

// ── Completion ────────────────────────────────────────────────────────────

/// Completion inside the frontmatter block: schema keys, enum values, boolish
/// scaffolds, file paths, and `style.*` keys.
pub fn completion(ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
    let Some(ast) = overlay_ast(ctx) else {
        return Vec::new();
    };
    if !ast.contains_offset(offset) {
        return Vec::new();
    }
    let (line_start, prefix) = line_prefix(ctx.text, offset);
    let indent = prefix.len() - prefix.trim_start().len();
    let trimmed = prefix.trim_start();

    match trimmed.find(':') {
        Some(colon) => {
            let key = trimmed[..colon].trim().to_string();
            let value_partial = trimmed[colon + 1..].trim_start();
            value_completions(ctx, offset, &key, value_partial)
        }
        None if indent == 0 => top_level_key_completions(ctx, ast, offset, trimmed),
        None => match enclosing_key(ctx.text, line_start, indent) {
            Some(parent) if parent == "style" => style_key_completions(ctx, offset, trimmed),
            _ => Vec::new(),
        },
    }
}

/// Schema keys not yet present, filtered by the typed prefix.
fn top_level_key_completions(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    offset: usize,
    partial: &str,
) -> Vec<CompletionItem> {
    let present: Vec<&str> = ast.top_level().map(|entry| entry.key.as_str()).collect();
    let start = offset - partial.len();
    let shape = known_shape(ctx);
    shape
        .properties
        .iter()
        .filter(|(name, _)| name.starts_with(partial) && !present.contains(&name.as_str()))
        .filter_map(|(name, def)| {
            let required = is_required(def);
            let detail = required.then(|| "(required)".to_string());
            item(
                ctx,
                start,
                offset,
                name,
                &format!("{name}: "),
                CompletionItemKind::FIELD,
                detail,
            )
        })
        .collect()
}

/// Enum members, boolish scaffolds, or file paths for a top-level key's value.
fn value_completions(
    ctx: &DocumentContext,
    offset: usize,
    key: &str,
    partial: &str,
) -> Vec<CompletionItem> {
    let start = offset - partial.len();
    let shape = known_shape(ctx);
    let Some(atom) = shape.properties.get(key).and_then(primary_atom) else {
        return Vec::new();
    };

    if let Some(members) = enum_members(atom) {
        return members
            .iter()
            .filter(|member| member.starts_with(partial))
            .filter_map(|member| {
                item(ctx, start, offset, member, member, CompletionItemKind::ENUM_MEMBER, None)
            })
            .collect();
    }
    if is_boolish(atom) {
        return ["true", "false"]
            .into_iter()
            .filter(|value| value.starts_with(partial))
            .filter_map(|value| {
                item(ctx, start, offset, value, value, CompletionItemKind::VALUE, None)
            })
            .collect();
    }
    if is_file(atom) {
        return file_path_completions(ctx, offset, partial);
    }
    Vec::new()
}

/// Workspace document paths (relative to the current document) for a
/// `file(...)`-typed value.
fn file_path_completions(ctx: &DocumentContext, offset: usize, partial: &str) -> Vec<CompletionItem> {
    let Some(base_dir) = ctx.path.parent() else {
        return Vec::new();
    };
    let start = offset - partial.len();
    let mut out = Vec::new();
    for record in ctx.graph.documents() {
        if record.path == ctx.path {
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

/// `style.*` container keys from the style descriptor catalog.
fn style_key_completions(ctx: &DocumentContext, offset: usize, partial: &str) -> Vec<CompletionItem> {
    let start = offset - partial.len();
    let mut containers: Vec<&str> = darkmatter::style::descriptor::SCHEMA
        .iter()
        .filter_map(|leaf| leaf.canonical.split('.').next())
        .collect();
    containers.sort_unstable();
    containers.dedup();
    containers
        .into_iter()
        .filter(|name| name.starts_with(partial))
        .filter_map(|name| {
            item(ctx, start, offset, name, &format!("{name}:"), CompletionItemKind::FIELD, None)
        })
        .collect()
}

// ── Hover ─────────────────────────────────────────────────────────────────

/// Hover on a frontmatter key: schema type/constraints/default/description, or
/// a `ctx.*` generated-key annotation.
pub fn hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let ast = overlay_ast(ctx)?;
    if !ast.contains_offset(offset) {
        return None;
    }
    let entry = ast.entry_at_offset(offset)?;

    if entry.dotted == "ctx" || entry.dotted.starts_with("ctx.") {
        return ctx_hover(ctx, entry);
    }
    schema_hover(ctx, entry)
}

/// Hover content for a schema-declared property.
fn schema_hover(ctx: &DocumentContext, entry: &FmEntry) -> Option<Hover> {
    if entry.depth != 0 {
        return None;
    }
    let shape = known_shape(ctx);
    let atom = shape.properties.get(&entry.key).and_then(primary_atom)?;
    let mut lines = vec![format!("**`{}`**", entry.key)];

    let type_line = match &atom.ty {
        TypeExpr::Primitive(ty) => {
            let suffix = if atom.is_array { "[]" } else { "" };
            format!("Type: `{}{}`", ty.as_keyword(), suffix)
        }
        TypeExpr::InlineObject(_) => "Type: `object`".to_string(),
    };
    lines.push(type_line);

    if is_required(shape.properties.get(&entry.key)?) {
        lines.push("Required".to_string());
    }
    if let Some(members) = enum_members(atom) {
        lines.push(format!("Values: {}", members.join(", ")));
    }
    if let Some(default) = default_value(atom) {
        lines.push(format!("Default: `{default}`"));
    }
    if let Some(description) = &atom.description {
        lines.push(String::new());
        lines.push(description.clone());
    }
    markup_hover(ctx, entry.key_span.clone(), lines.join("\n\n"))
}

/// Hover content for a `ctx.*` generated key (read-only, Darkmatter-owned).
fn ctx_hover(ctx: &DocumentContext, entry: &FmEntry) -> Option<Hover> {
    let mut value = if entry.dotted == "ctx" {
        "**`ctx`** — Darkmatter-generated context (read-only)".to_string()
    } else {
        let descriptor = context_variable_descriptors()
            .iter()
            .find(|descriptor| descriptor.name == entry.key);
        match descriptor {
            Some(descriptor) => format!(
                "**`ctx.{}`** ({:?}) — read-only, Darkmatter-owned\n\n{}",
                descriptor.name, descriptor.display_type, descriptor.description
            ),
            None => format!("**`ctx.{}`** — Darkmatter-generated (read-only)", entry.key),
        }
    };
    value.push('\n');
    markup_hover(ctx, entry.key_span.clone(), value)
}

// ── Navigation (definition + document links) ────────────────────────────────

/// Definition on a `$schema` file reference or a `file(...)` value.
pub fn definition(ctx: &DocumentContext, offset: usize) -> Vec<Location> {
    let Some(ast) = overlay_ast(ctx) else {
        return Vec::new();
    };
    nav_targets(ctx, ast)
        .into_iter()
        .filter(|(span, _)| span.contains(&offset))
        .filter_map(|(_, path)| {
            let uri = file_path_to_uri(&path)?;
            Some(Location::new(uri, Range::default()))
        })
        .collect()
}

/// Document links for `$schema` file references and `file(...)` values.
pub fn document_links(ctx: &DocumentContext) -> Vec<DocumentLink> {
    let Some(ast) = overlay_ast(ctx) else {
        return Vec::new();
    };
    nav_targets(ctx, ast)
        .into_iter()
        .filter_map(|(span, path)| {
            let range = ctx.source_map.byte_range_to_lsp(span)?;
            let target = file_path_to_uri(&path)?;
            Some(DocumentLink { range, target: Some(target), tooltip: None, data: None })
        })
        .collect()
}

/// The `(value_span, resolved_path)` navigation targets in the frontmatter:
/// the `$schema` file reference plus every `file(...)`-typed scalar value.
fn nav_targets(ctx: &DocumentContext, ast: &FrontmatterAst) -> Vec<(SourceSpan, PathBuf)> {
    let Some(base_dir) = ctx.path.parent() else {
        return Vec::new();
    };
    let mut targets = Vec::new();

    // `$schema: ./file.yaml` (an inline `{ ... }` mapping is not a file ref).
    if let Some(entry) = ast.schema_entry()
        && entry.kind == FmValueKind::Scalar
        && let Some(value) = &entry.scalar
        && looks_like_path(value)
    {
        targets.push((entry.value_span.clone(), normalize_join(base_dir, value)));
    }

    // `file(...)`-typed top-level scalar values.
    let shape = known_shape(ctx);
    for entry in ast.top_level() {
        if entry.kind != FmValueKind::Scalar {
            continue;
        }
        let Some(atom) = shape.properties.get(&entry.key).and_then(primary_atom) else {
            continue;
        };
        if is_file(atom)
            && let Some(value) = &entry.scalar
            && looks_like_path(value)
        {
            targets.push((entry.value_span.clone(), normalize_join(base_dir, value)));
        }
    }
    targets
}

// ── Folding + symbols ───────────────────────────────────────────────────────

/// Folds for nested frontmatter mappings and sequences (the whole-block fold is
/// the substrate's; these add the `style:` / `ctx:` sub-folds).
pub fn folding_ranges(ctx: &DocumentContext) -> Vec<FoldingRange> {
    if !ctx.profile.supports_folding {
        return Vec::new();
    }
    let Some(ast) = overlay_ast(ctx) else {
        return Vec::new();
    };
    let mut folds = Vec::new();
    for entry in ast.entries() {
        if !matches!(entry.kind, FmValueKind::Mapping | FmValueKind::Sequence) {
            continue;
        }
        let Some(start) = ctx.source_map.byte_to_lsp(entry.key_span.start) else {
            continue;
        };
        let end_byte = entry.value_span.end.saturating_sub(1);
        let Some(end) = ctx.source_map.byte_to_lsp(end_byte) else {
            continue;
        };
        if end.line > start.line {
            folds.push(FoldingRange {
                start_line: start.line,
                start_character: None,
                end_line: end.line,
                end_character: None,
                kind: None,
                collapsed_text: None,
            });
        }
    }
    folds
}

/// Top-level frontmatter keys as document symbols (config-gated).
pub fn document_symbols(ctx: &DocumentContext) -> Vec<DocumentSymbol> {
    if !ctx.config.symbols.frontmatter {
        return Vec::new();
    }
    let Some(ast) = overlay_ast(ctx) else {
        return Vec::new();
    };
    ast.top_level()
        .filter_map(|entry| {
            let range = ctx
                .source_map
                .byte_range_to_lsp(entry.key_span.start..entry.value_span.end)?;
            let selection = ctx.source_map.byte_range_to_lsp(entry.key_span.clone())?;
            #[allow(deprecated)]
            Some(DocumentSymbol {
                name: entry.key.clone(),
                detail: None,
                kind: SymbolKind::FIELD,
                tags: None,
                deprecated: None,
                range,
                selection_range: selection,
                children: None,
            })
        })
        .collect()
}

// ── Schema-shape helpers ────────────────────────────────────────────────────

/// The effective completion shape: the Darkmatter base properties, overlaid
/// with each matched extension baseline (e.g. Claudine), then the document's
/// own `$schema` (document > extension > base — compose precedence).
fn known_shape(ctx: &DocumentContext) -> SchemaShape {
    let mut shape = match darkmatter_base_schema() {
        SimplifiedSchema::Single(shape) => shape,
        SimplifiedSchema::Union(_) => SchemaShape::default(),
    };
    if let Some(bundle) = ctx.overlay.and_then(|overlay| overlay.bundle()) {
        for extension in &bundle.extension_shapes {
            for (name, def) in &extension.properties {
                shape.properties.insert(name.clone(), def.clone());
            }
        }
        if let Some(SimplifiedSchema::Single(document)) = &bundle.effective.simplified {
            for (name, def) in &document.properties {
                shape.properties.insert(name.clone(), def.clone());
            }
        }
    }
    shape
}

/// The primary atom of a property definition (the first arm of a union).
fn primary_atom(def: &PropertyDef) -> Option<&PropertyAtom> {
    match def {
        PropertyDef::Single(atom) => Some(atom),
        PropertyDef::Union(atoms) => atoms.first(),
    }
}

/// Whether any arm of a property is required.
fn is_required(def: &PropertyDef) -> bool {
    let atoms: &[PropertyAtom] = match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    };
    atoms
        .iter()
        .any(|atom| atom.constraints.iter().any(|c| matches!(c, Constraint::Required)))
}

/// The enum members declared on an atom, if any.
fn enum_members(atom: &PropertyAtom) -> Option<&[String]> {
    atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Members(members) => Some(members.as_slice()),
        _ => None,
    })
}

/// The declared default value of an atom, if any.
fn default_value(atom: &PropertyAtom) -> Option<&serde_json::Value> {
    atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Default(value) => Some(value),
        _ => None,
    })
}

/// Whether an atom is a boolean/boolish scalar.
fn is_boolish(atom: &PropertyAtom) -> bool {
    matches!(
        atom.ty,
        TypeExpr::Primitive(SimplifiedType::Boolean | SimplifiedType::Boolish)
    )
}

/// Whether an atom is a `file(...)` value.
fn is_file(atom: &PropertyAtom) -> bool {
    matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File))
}

// ── Small helpers ───────────────────────────────────────────────────────────

/// The frontmatter tree for this context, if any.
fn overlay_ast<'a>(ctx: &'a DocumentContext) -> Option<&'a FrontmatterAst> {
    ctx.overlay.and_then(|overlay| overlay.ast.as_deref())
}

/// `(line_start, prefix)` where `prefix` is the text of the cursor's line up to
/// `offset`.
fn line_prefix(text: &str, offset: usize) -> (usize, &str) {
    let line_start = text[..offset].rfind('\n').map(|index| index + 1).unwrap_or(0);
    (line_start, &text[line_start..offset])
}

/// The nearest less-indented parent key above `line_start`.
fn enclosing_key(text: &str, line_start: usize, indent: usize) -> Option<String> {
    for line in text[..line_start].lines().rev() {
        let line_indent = line.len() - line.trim_start().len();
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        if line_indent < indent {
            let key = trimmed.split(':').next().unwrap_or("").trim();
            return (!key.is_empty()).then(|| key.to_string());
        }
    }
    None
}

/// Whether a scalar value is plausibly a local file path (not a URL or an
/// obvious non-path token).
fn looks_like_path(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && !value.contains("://")
        && !value.starts_with('{')
        && (value.contains('/') || value.contains('.'))
}

/// Builds a completion item with an eager text edit over `start..offset`.
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

/// Portable (forward-slash) relative path from `base_dir` to `target`.
fn relativize(base_dir: &Path, target: &Path) -> String {
    use std::path::Component;
    let base: Vec<Component> = base_dir.components().collect();
    let dest: Vec<Component> = target.components().collect();
    let shared = base.iter().zip(dest.iter()).take_while(|(a, b)| a == b).count();
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

/// A Markdown hover over `span`.
fn markup_hover(ctx: &DocumentContext, span: SourceSpan, value: String) -> Option<Hover> {
    let range = ctx.source_map.byte_range_to_lsp(span);
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_prefix() {
        let text = "---\ntitle: X\n---\n";
        // offset at end of "title: X"
        let offset = "---\ntitle: X".len();
        let (start, prefix) = line_prefix(text, offset);
        assert_eq!(start, 4);
        assert_eq!(prefix, "title: X");
    }

    #[test]
    fn test_enclosing_key_finds_style_parent() {
        let text = "---\nstyle:\n  page:\n";
        // A line indented under `page:` — enclosing key at indent 2 is `page`.
        let line_start = text.len();
        assert_eq!(enclosing_key(text, line_start, 4), Some("page".to_string()));
        // At indent 2 (under `style:`), the enclosing key is `style`.
        let under_style = "---\nstyle:\n".len();
        assert_eq!(enclosing_key(text, under_style + 2, 2), Some("style".to_string()));
    }

    #[test]
    fn test_looks_like_path() {
        assert!(looks_like_path("./schema.yaml"));
        assert!(looks_like_path("dir/file.md"));
        assert!(!looks_like_path("https://example.com/s.yaml"));
        assert!(!looks_like_path("plain"));
        assert!(!looks_like_path("{ inline: true }"));
    }
}
