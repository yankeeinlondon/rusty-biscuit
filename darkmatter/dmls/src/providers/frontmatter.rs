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
use crate::overlay::{FmEntry, FmValueKind, FrontmatterAst, expressions};
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
            let ancestors = enclosing_path(ctx.text, line_start, indent);
            value_completions(ctx, offset, &ancestors, &key, value_partial)
        }
        None if indent == 0 => top_level_key_completions(ctx, ast, offset, trimmed),
        None => nested_key_completions(ctx, ast, offset, line_start, indent, trimmed),
    }
}

/// Top-level schema keys not yet present, filtered by the typed prefix.
fn top_level_key_completions(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    offset: usize,
    partial: &str,
) -> Vec<CompletionItem> {
    let present: Vec<&str> = ast.top_level().map(|entry| entry.key.as_str()).collect();
    let shape = known_shape(ctx);
    shape_key_completions(ctx, offset, &shape, &present, partial)
}

/// Nested-mapping key completion. When the enclosing parent path resolves to an
/// inline-object shape, offers that shape's not-yet-present keys; otherwise
/// falls back to the `style.*` descriptor catalog when the immediate parent is
/// the opaque `style` object (which is not an inline object in the schema).
fn nested_key_completions(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    offset: usize,
    line_start: usize,
    indent: usize,
    partial: &str,
) -> Vec<CompletionItem> {
    let ancestors = enclosing_path(ctx.text, line_start, indent);
    let ancestor_refs: Vec<&str> = ancestors.iter().map(String::as_str).collect();
    let shape = known_shape(ctx);
    if let Some(nested) = nested_shape(&shape, &ancestor_refs) {
        let present = present_child_keys(ast, &ancestors);
        return shape_key_completions(ctx, offset, nested, &present, partial);
    }
    if ancestor_refs.last() == Some(&"style") {
        return style_key_completions(ctx, offset, partial);
    }
    Vec::new()
}

/// A shape's not-yet-present property keys, filtered by `partial` and marked
/// `(required)`, as `FIELD` completions with an eager `key: ` text edit.
fn shape_key_completions(
    ctx: &DocumentContext,
    offset: usize,
    shape: &SchemaShape,
    present: &[&str],
    partial: &str,
) -> Vec<CompletionItem> {
    let start = offset - partial.len();
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

/// The already-authored direct child keys of the mapping at `ancestors`, so
/// completion can exclude them. The dotted-prefix guard keeps a same-named
/// mapping elsewhere in the tree (e.g. under `$schema`) from leaking in.
fn present_child_keys<'a>(ast: &'a FrontmatterAst, ancestors: &[String]) -> Vec<&'a str> {
    let depth = ancestors.len();
    let prefix = format!("{}.", ancestors.join("."));
    ast.entries()
        .iter()
        .filter(|entry| entry.depth == depth)
        .filter_map(|entry| entry.dotted.strip_prefix(&prefix).filter(|rest| !rest.contains('.')))
        .collect()
}

/// Enum members, boolish scaffolds, or file paths for a key's value. `ancestors`
/// is the parent mapping path (empty at the top level); the leaf atom is
/// resolved by descending inline objects.
fn value_completions(
    ctx: &DocumentContext,
    offset: usize,
    ancestors: &[String],
    key: &str,
    partial: &str,
) -> Vec<CompletionItem> {
    let start = offset - partial.len();
    let shape = known_shape(ctx);
    let mut path: Vec<&str> = ancestors.iter().map(String::as_str).collect();
    path.push(key);
    let Some(atom) = def_at_path(&shape, &path).and_then(completable_atom) else {
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

/// Hover content for a schema-declared property, at any nesting depth.
fn schema_hover(ctx: &DocumentContext, entry: &FmEntry) -> Option<Hover> {
    let shape = known_shape(ctx);
    let path: Vec<&str> = entry.dotted.split('.').collect();
    let def = def_at_path(&shape, &path)?;
    let body = schema_hover_body(&entry.key, def)?;
    markup_hover(ctx, entry.key_span.clone(), body)
}

/// The Markdown body for a schema-declared property's hover.
///
/// Style rule — bounded by what LSP-Markdown can express (color and dim are the
/// editor theme's decision, not ours; see `docs/hover.md`): inline-code box =
/// the property being described; **bold** = its type; _italic_ = its enum and
/// default values. Kept pure (no `DocumentContext`) so the formatting rule is
/// unit-testable without an LSP session.
pub(crate) fn schema_hover_body(key: &str, def: &PropertyDef) -> Option<String> {
    let atom = primary_atom(def)?;
    let mut lines = vec![format!("**`{key}`**")];

    let type_line = match &atom.ty {
        TypeExpr::Primitive(ty) => {
            let suffix = if atom.is_array { "[]" } else { "" };
            format!("Type: **{}{}**", ty.as_keyword(), suffix)
        }
        TypeExpr::InlineObject(_) => "Type: **object**".to_string(),
        TypeExpr::Imported { name, reference } => {
            let suffix = if atom.is_array { "[]" } else { "" };
            format!("Type: **{name}{suffix}@{reference}**")
        }
    };
    lines.push(type_line);

    if is_required(def) {
        lines.push("Required".to_string());
    }
    if let Some(members) = enum_members(atom) {
        let italicized: Vec<String> = members.iter().map(|m| format!("_{m}_")).collect();
        lines.push(format!("Values: {}", italicized.join(", ")));
    }
    if let Some(default) = default_value(atom) {
        lines.push(format!("Default: _{default}_"));
    }
    if let Some(description) = &atom.description {
        lines.push(String::new());
        lines.push(description.clone());
    }
    Some(lines.join("\n\n"))
}

/// Hover content for a `ctx.*` generated key (read-only, Darkmatter-owned).
fn ctx_hover(ctx: &DocumentContext, entry: &FmEntry) -> Option<Hover> {
    let mut value = ctx_hover_markdown(&entry.dotted, &entry.key);
    value.push('\n');
    markup_hover(ctx, entry.key_span.clone(), value)
}

/// The Markdown body of a frontmatter `ctx.*` hover.
///
/// The catalog-backed block comes from the shared Phase-1 formatter
/// ([`expressions::format_ctx_hover_block`]) so it is byte-identical to the
/// interpolation hover's block for the same variable; the compose-time note is
/// interpolation-specific and never appears here.
fn ctx_hover_markdown(dotted: &str, key: &str) -> String {
    if dotted == "ctx" {
        return "**`ctx`** — Darkmatter-generated context (read-only)".to_string();
    }
    match expressions::ctx_descriptor(key) {
        Some(descriptor) => expressions::format_ctx_hover_block(descriptor),
        None => format!("**`ctx.{key}`** — Darkmatter-generated (read-only)"),
    }
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

    // `file(...)`-typed scalar values at any depth: a top-level key is a
    // single-segment path, a nested one resolves through the schema's inline
    // objects. Splitting `dotted` on `.` mirrors the completion/hover path. Any
    // union arm being a `file` type makes the value a navigable reference.
    let shape = known_shape(ctx);
    for entry in ast.entries() {
        if entry.kind != FmValueKind::Scalar {
            continue;
        }
        let path: Vec<&str> = entry.dotted.split('.').collect();
        if def_at_path(&shape, &path).and_then(file_atom).is_some()
            && let Some(value) = &entry.scalar
            && is_schema_file_value(value)
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
///
/// Exposed for reuse by other frontmatter-aware providers (e.g. navigation).
pub(crate) fn known_shape(ctx: &DocumentContext) -> SchemaShape {
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

/// The nested [`SchemaShape`] reached by walking each `ancestors` segment into
/// its inline-object type, selecting the first inline-object arm of a union. An
/// empty path yields `root`; `None` when any segment is absent or has no
/// inline-object arm (e.g. the opaque `object`-typed `style`).
pub(crate) fn nested_shape<'a>(root: &'a SchemaShape, ancestors: &[&str]) -> Option<&'a SchemaShape> {
    let mut shape = root;
    for segment in ancestors {
        shape = shape.properties.get(*segment).and_then(inline_object_shape)?;
    }
    Some(shape)
}

/// The [`PropertyDef`] at a full key `path` (ancestor segments followed by the
/// leaf key), descending inline objects for the ancestors. `None` when any
/// ancestor is missing/not an inline object, or the leaf key is absent.
///
/// This is the reusable nested schema-path resolver: split a `FrontmatterAst`
/// entry's `dotted` path on `.` and pass it here to reach the schema definition
/// governing that value at any nesting depth.
pub(crate) fn def_at_path<'a>(root: &'a SchemaShape, path: &[&str]) -> Option<&'a PropertyDef> {
    let (leaf, ancestors) = path.split_last()?;
    let shape = nested_shape(root, ancestors)?;
    shape.properties.get(*leaf)
}

/// A property definition's arms as a slice (one element for a single atom).
fn atoms_of(def: &PropertyDef) -> &[PropertyAtom] {
    match def {
        PropertyDef::Single(atom) => std::slice::from_ref(atom),
        PropertyDef::Union(atoms) => atoms,
    }
}

/// The representative atom for hover rendering: the first arm of a union.
/// Hover only needs one coherent shape to describe, so first-arm is fine.
pub(crate) fn primary_atom(def: &PropertyDef) -> Option<&PropertyAtom> {
    atoms_of(def).first()
}

/// The first arm whose type drives a value completion (enum members, boolish
/// scaffold, or `file(...)` paths). Mirrors the schema library's
/// `first_completable_atom` arm search so a union whose completable arm is not
/// first still offers value completion.
fn completable_atom(def: &PropertyDef) -> Option<&PropertyAtom> {
    atoms_of(def)
        .iter()
        .find(|atom| enum_members(atom).is_some() || is_boolish(atom) || is_file(atom))
}

/// The first `file(...)`-typed arm, if any — so navigation and document links
/// treat the value as a file reference even when the file arm is not first.
fn file_atom(def: &PropertyDef) -> Option<&PropertyAtom> {
    atoms_of(def).iter().find(|atom| is_file(atom))
}

/// The shape of the first inline-object arm, if any — the deterministic arm a
/// nested-key lookup descends through even when it is not first.
fn inline_object_shape(def: &PropertyDef) -> Option<&SchemaShape> {
    atoms_of(def).iter().find_map(|atom| match &atom.ty {
        TypeExpr::InlineObject(inner) => Some(inner),
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } => None,
    })
}

/// Whether any arm of a property is required.
fn is_required(def: &PropertyDef) -> bool {
    atoms_of(def)
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
pub(crate) fn is_file(atom: &PropertyAtom) -> bool {
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

/// The full chain of ancestor keys above `line_start`, outermost first, for a
/// line at column `indent` — one key per strictly-decreasing indent level, so
/// nested inline-object mappings resolve. Empty for a top-level line.
fn enclosing_path(text: &str, line_start: usize, indent: usize) -> Vec<String> {
    let mut path = Vec::new();
    let mut needed = indent;
    for line in text[..line_start].lines().rev() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        let line_indent = line.len() - trimmed.len();
        if line_indent < needed {
            let key = trimmed.split(':').next().unwrap_or("").trim();
            if !key.is_empty() {
                path.push(key.to_string());
            }
            needed = line_indent;
            if needed == 0 {
                break;
            }
        }
    }
    path.reverse();
    path
}

/// Whether a scalar value is plausibly a local file path (not a URL or an
/// obvious non-path token).
///
/// A dot/slash heuristic for callers with no schema type to trust — the
/// `$schema` source reference, whose value is a `.yaml`/`.md`/`.json` file or an
/// inline `{ ... }` mapping. A schema-confirmed `file(...)` value uses
/// [`is_schema_file_value`] instead, which does not require a dot or slash.
fn looks_like_path(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty()
        && !value.contains("://")
        && !value.starts_with('{')
        && (value.contains('/') || value.contains('.'))
}

/// Whether a value the schema already types as `file(...)` is a navigable local
/// reference.
///
/// Once the effective schema confirms the `file` type, a bare extensionless
/// filename (e.g. `LICENSE`, `Makefile`) is a valid relative reference — the
/// same implicit-relative form `FileReference` accepts — so only a URL or an
/// inline-object literal disqualifies it. The dot/slash heuristic in
/// [`looks_like_path`] must not gate this path.
fn is_schema_file_value(value: &str) -> bool {
    let value = value.trim();
    !value.is_empty() && !value.contains("://") && !value.starts_with('{')
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
    fn test_schema_hover_body_applies_style_rules() {
        // The rule from docs/hover.md, bounded by LSP-Markdown (no color, no
        // dim): property being described → inline-code box; type → bold (never
        // inline code); enum members and default → italic.
        let mut atom = PropertyAtom::bare(SimplifiedType::Enum);
        atom.constraints = vec![
            Constraint::Members(vec!["draft".to_string(), "published".to_string()]),
            Constraint::Default(serde_json::Value::from("draft")),
        ];
        atom.description = Some("the publication state".to_string());
        let def = PropertyDef::Single(atom);

        let body = schema_hover_body("status", &def).expect("hover body");

        assert!(body.contains("**`status`**"), "property → inline-code box: {body}");
        assert!(body.contains("Type: **"), "type → bold: {body}");
        assert!(!body.contains("Type: `"), "type must not be inline code: {body}");
        assert!(body.contains("Values: _draft_, _published_"), "enum → italic: {body}");
        assert!(body.contains("Default: _\"draft\"_"), "default → italic: {body}");
        assert!(body.contains("the publication state"), "description verbatim: {body}");
    }

    #[test]
    fn test_schema_hover_body_bare_type_is_bold() {
        let def = PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String));
        let body = schema_hover_body("title", &def).expect("hover body");
        assert_eq!(body, "**`title`**\n\nType: **string**", "{body}");
    }

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
    fn test_enclosing_path_builds_full_ancestor_chain() {
        let text = "---\nstyle:\n  page:\n    ";
        // A line indented under `page:` (indent 4) has ancestors `style` →
        // `page`, outermost first.
        assert_eq!(enclosing_path(text, text.len(), 4), vec!["style", "page"]);
        // A line under `style:` (indent 2) has just `style`.
        let under_style = "---\nstyle:\n".len();
        assert_eq!(enclosing_path(text, under_style + 2, 2), vec!["style"]);
        // A top-level line (indent 0) has no ancestors.
        assert!(enclosing_path(text, under_style, 0).is_empty());
    }

    fn nested_fixture() -> SchemaShape {
        let mut inner = SchemaShape::new();
        inner
            .properties
            .insert("mode".to_string(), PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Enum)));
        let mut root = SchemaShape::new();
        root.properties.insert(
            "settings".to_string(),
            PropertyDef::Single(PropertyAtom::bare_inline_object(inner)),
        );
        root.properties
            .insert("title".to_string(), PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String)));
        root
    }

    #[test]
    fn test_nested_shape_walks_inline_objects() {
        let root = nested_fixture();
        // Empty path is the root itself.
        assert!(nested_shape(&root, &[]).unwrap().properties.contains_key("settings"));
        // One inline-object segment descends into its shape.
        let settings = nested_shape(&root, &["settings"]).unwrap();
        assert!(settings.properties.contains_key("mode"));
    }

    #[test]
    fn test_nested_shape_missing_segment_is_none() {
        let root = nested_fixture();
        assert!(nested_shape(&root, &["absent"]).is_none());
    }

    #[test]
    fn test_nested_shape_non_inline_object_is_none() {
        let root = nested_fixture();
        // `title` is a primitive, not an inline object — cannot descend.
        assert!(nested_shape(&root, &["title"]).is_none());
    }

    #[test]
    fn test_def_at_path_resolves_leaf_atom() {
        let root = nested_fixture();
        let def = def_at_path(&root, &["settings", "mode"]).unwrap();
        assert_eq!(primary_atom(def).unwrap().ty, TypeExpr::Primitive(SimplifiedType::Enum));
        // A missing leaf under a valid inline object is `None`.
        assert!(def_at_path(&root, &["settings", "absent"]).is_none());
        // A top-level leaf resolves through an empty ancestor path.
        assert!(def_at_path(&root, &["title"]).is_some());
    }

    #[test]
    fn test_ctx_hover_markdown_is_the_shared_formatter_block() {
        // The catalog-backed block is exactly the shared Phase-1 formatter's
        // output, so it can never drift from the interpolation hover's block.
        let descriptor = expressions::ctx_descriptor("packages").unwrap();
        let markdown = ctx_hover_markdown("ctx.packages", "packages");
        assert_eq!(markdown, expressions::format_ctx_hover_block(descriptor));
        // The interpolation-specific compose-time note never appears here.
        assert!(!markdown.contains("compose time"));
        // Unknown tails and the `ctx` container keep their generic annotations.
        assert!(ctx_hover_markdown("ctx.nope", "nope").contains("Darkmatter-generated"));
        assert!(ctx_hover_markdown("ctx", "ctx").starts_with("**`ctx`**"));
    }

    #[test]
    fn test_looks_like_path() {
        assert!(looks_like_path("./schema.yaml"));
        assert!(looks_like_path("dir/file.md"));
        assert!(!looks_like_path("https://example.com/s.yaml"));
        assert!(!looks_like_path("plain"));
        assert!(!looks_like_path("{ inline: true }"));
    }

    #[test]
    fn test_is_schema_file_value_accepts_extensionless() {
        // A schema-confirmed `file` value navigates even without a dot or slash.
        assert!(is_schema_file_value("LICENSE"));
        assert!(is_schema_file_value("Makefile"));
        assert!(is_schema_file_value("./guide.md"));
        // URLs and inline-object literals are still rejected.
        assert!(!is_schema_file_value("https://example.com/s.yaml"));
        assert!(!is_schema_file_value("{ inline: true }"));
        assert!(!is_schema_file_value("   "));
    }

    /// An enum atom carrying its members, so [`enum_members`] resolves (a bare
    /// `PropertyAtom::bare(Enum)` has none and would not drive completion).
    fn enum_atom(members: &[&str]) -> PropertyAtom {
        let mut atom = PropertyAtom::bare(SimplifiedType::Enum);
        atom.constraints
            .push(Constraint::Members(members.iter().map(|m| m.to_string()).collect()));
        atom
    }

    #[test]
    fn test_completable_atom_selects_non_first_arm() {
        // `[string, enum(dev, prod)]` — the completable arm is second, but the
        // first (`string`) is not completable.
        let def = PropertyDef::Union(vec![
            PropertyAtom::bare(SimplifiedType::String),
            enum_atom(&["dev", "prod"]),
        ]);
        let atom = completable_atom(&def).expect("the enum arm is completable");
        assert_eq!(enum_members(atom), Some(["dev".to_string(), "prod".to_string()].as_slice()));
        // A non-completable single atom yields nothing.
        assert!(completable_atom(&PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String))).is_none());
    }

    #[test]
    fn test_file_atom_selects_non_first_arm() {
        // `[string, file]` — the file arm is second; `primary_atom` (first arm)
        // would miss it, so navigation must consult `file_atom`.
        let def = PropertyDef::Union(vec![
            PropertyAtom::bare(SimplifiedType::String),
            PropertyAtom::bare(SimplifiedType::File),
        ]);
        assert!(is_file(file_atom(&def).expect("the file arm is selectable")));
        // `primary_atom` still reports the first (string) arm, proving the two
        // selectors diverge for this union.
        assert!(!is_file(primary_atom(&def).unwrap()));
        // No file arm → `None`.
        assert!(file_atom(&PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String))).is_none());
    }

    #[test]
    fn test_inline_object_shape_selects_non_first_arm() {
        // `[string, { mode: enum }]` — the inline-object arm is second.
        let mut inner = SchemaShape::new();
        inner
            .properties
            .insert("mode".to_string(), PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Enum)));
        let def = PropertyDef::Union(vec![
            PropertyAtom::bare(SimplifiedType::String),
            PropertyAtom::bare_inline_object(inner),
        ]);
        let shape = inline_object_shape(&def).expect("the inline-object arm is selectable");
        assert!(shape.properties.contains_key("mode"));
        // No inline-object arm → `None`.
        assert!(inline_object_shape(&PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String))).is_none());
    }

    #[test]
    fn test_nested_shape_descends_through_union_inline_object_arm() {
        // A `settings` property that is a union whose inline-object arm is
        // second still resolves its nested keys.
        let mut inner = SchemaShape::new();
        inner
            .properties
            .insert("mode".to_string(), PropertyDef::Single(PropertyAtom::bare(SimplifiedType::Enum)));
        let mut root = SchemaShape::new();
        root.properties.insert(
            "settings".to_string(),
            PropertyDef::Union(vec![
                PropertyAtom::bare(SimplifiedType::String),
                PropertyAtom::bare_inline_object(inner),
            ]),
        );
        let settings = nested_shape(&root, &["settings"]).expect("descends the union inline arm");
        assert!(settings.properties.contains_key("mode"));
        // And `def_at_path` reaches the nested leaf through the same union arm.
        assert!(def_at_path(&root, &["settings", "mode"]).is_some());
    }
}
