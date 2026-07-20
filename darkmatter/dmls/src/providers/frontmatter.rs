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

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::schemas::{
    Constraint, DecodedScalar, PropertyAtom, PropertyDef, SchemaArm, SchemaCursor,
    SchemaCursorRole, SchemaDeclaration, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr,
    darkmatter_base_schema, decode_scalar, locate_schema_declaration_cursor,
    locate_type_definition_cursor, parse_property_definition, parse_schema_declaration,
    parse_schema_declaration_with_source, schema_constraint_descriptors, schema_type_descriptors,
    select_literal_discriminant_arm, suggestions_for_def,
};
use serde_json::Value;
use darkmatter::markdown::span::SourceSpan;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Diagnostic, DocumentLink,
    Documentation, DocumentSymbol, FoldingRange, Hover, HoverContents, Location, MarkupContent,
    MarkupKind, Range, SymbolKind, TextEdit,
};

use super::DocumentContext;
use crate::graph::normalize_join;
use crate::overlay::{
    FmEntry, FmValueKind, FrontmatterAst, SchemaAuthoringState, doc_links, expressions,
};
use crate::overlay::schema::{MetaSchemaKind, semantic_type_regions};
use crate::workspace::file_path_to_uri;

/// Frontmatter/schema diagnostics (delegated to the diagnostics module).
pub fn diagnostics(ctx: &DocumentContext) -> Vec<Diagnostic> {
    crate::diagnostics::frontmatter::diagnostics(ctx)
}

// ── Completion ────────────────────────────────────────────────────────────

/// Completion inside the frontmatter block: schema keys, enum values, boolish
/// scaffolds, file paths, `style.*` keys, and `suggest(...)` candidates.
///
/// Semantic meta-type candidates are **merged** with the ordinary schema-driven
/// ones rather than replacing them. A property typed
/// `[type-definition, enum(foo, bar)]` activates semantic authoring on one arm
/// only, so short-circuiting there would silently discard every sibling arm's
/// candidates. Semantic items lead so the pure-`type-definition` ordering is
/// unchanged, and `dedup_completions` keeps first-seen order.
pub fn completion(ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
    let mut items = meta_schema_completion(ctx, offset).unwrap_or_default();
    items.extend(schema_completion(ctx, offset));
    dedup_completions(items)
}

/// The ordinary (non-semantic) frontmatter completion candidates: schema keys,
/// enum values, boolish scaffolds, file paths, `style.*` keys, and suggestions.
fn schema_completion(ctx: &DocumentContext, offset: usize) -> Vec<CompletionItem> {
    let Some(ast) = overlay_ast(ctx) else {
        return Vec::new();
    };
    if !ast.contains_offset(offset) {
        return Vec::new();
    }
    let (line_start, prefix) = line_prefix(ctx.text, offset);
    let indent = prefix.len() - prefix.trim_start().len();
    let trimmed = prefix.trim_start();

    match value_cursor(ctx, offset, line_start, trimmed) {
        Some((key, value_partial)) => {
            let ancestors = enclosing_path(ctx, offset, line_start, indent);
            value_completions(ctx, offset, &ancestors, &key, &value_partial)
        }
        None if indent == 0 => top_level_key_completions(ctx, ast, offset, trimmed),
        None => {
            if let Some(items) = block_array_suggestions(ctx, offset, line_start, indent, trimmed) {
                return items;
            }
            nested_key_completions(ctx, ast, offset, line_start, indent, trimmed)
        }
    }
}

/// Completion for values interpreted as SimplifiedSchema authoring syntax.
///
/// Activation (which values carry a semantic meta-type at all) comes from the
/// overlay's typed [`SchemaAuthoringState`]. Everything past that — what the
/// cursor is in the middle of authoring, and the range a text edit may replace
/// — comes from the shared tolerant parser-state authority
/// ([`locate_type_definition_cursor`] / [`locate_schema_declaration_cursor`]),
/// never from searching the value text for a delimiter.
fn meta_schema_completion(ctx: &DocumentContext, offset: usize) -> Option<Vec<CompletionItem>> {
    let overlay = ctx.overlay?;
    let (line_start, prefix) = line_prefix(ctx.text, offset);
    let indent = prefix.len() - prefix.trim_start().len();
    let trimmed = prefix.trim_start();
    let ancestors = enclosing_path(ctx, offset, line_start, indent);

    let (value_start, kinds) = if let Some((key, partial)) =
        value_cursor(ctx, offset, line_start, trimmed)
    {
        let kinds = meta_schema_kinds_for_line(overlay, &ancestors, &key, false)?;
        (semantic_value_start(ctx, offset, line_start, partial.len()), kinds)
    } else if let Some(after_dash) = trimmed.strip_prefix("- ") {
        let kinds = meta_schema_kinds_for_line(overlay, &ancestors, "", true)?;
        (offset - after_dash.len(), kinds)
    } else if trimmed == "-" {
        let kinds = meta_schema_kinds_for_line(overlay, &ancestors, "", true)?;
        (offset, kinds)
    } else {
        return None;
    };

    let value_source = ctx.text.get(value_start..)?;
    let mut items = Vec::new();
    for kind in kinds {
        let state = match kind {
            MetaSchemaKind::TypeDefinition => {
                locate_type_definition_cursor(value_source, value_start, offset)
            }
            MetaSchemaKind::Schema => {
                locate_schema_declaration_cursor(value_source, value_start, offset)
            }
        };
        // A cursor the grammar cannot speak to (inside a `-> description`)
        // deliberately offers nothing rather than guessing.
        let Some(state) = state else { continue };
        let start = state.token_span.start;
        match kind {
            MetaSchemaKind::TypeDefinition => {
                items.extend(type_definition_completions(ctx, start, offset, &state));
            }
            MetaSchemaKind::Schema => {
                items.extend(schema_value_completions(ctx, start, offset, &state.token));
            }
        }
    }
    Some(dedup_completions(items))
}

/// The document byte offset where a `key:` line's value begins.
///
/// The frontmatter AST's value span is authoritative whenever the buffer's YAML
/// still parses; a value being typed into a not-yet-parseable buffer (an open
/// `{`, a half-written flow sequence) has no entry, so the start implied by
/// `partial_len` (the already-typed value text before the cursor) is the
/// fallback.
fn semantic_value_start(
    ctx: &DocumentContext,
    offset: usize,
    line_start: usize,
    partial_len: usize,
) -> usize {
    let fallback = offset - partial_len;
    overlay_ast(ctx)
        .and_then(|ast| ast.key_entry_on_line(line_start, offset))
        .filter(|entry| still_placed(ctx, entry))
        .filter(|entry| entry.value_span.start <= offset && entry.value_span.start >= fallback)
        .map(|entry| entry.value_span.start)
        .unwrap_or(fallback)
}

fn meta_schema_kinds_for_line(
    overlay: &crate::overlay::DocumentOverlay,
    ancestors: &[String],
    key: &str,
    sequence_item: bool,
) -> Option<Vec<MetaSchemaKind>> {
    match &overlay.schema_authoring {
        SchemaAuthoringState::Standalone { envelope, .. } => {
            let payload = match envelope {
                darkmatter::markdown::schemas::StandaloneSchemaEnvelope::Pure => "$schema",
                darkmatter::markdown::schemas::StandaloneSchemaEnvelope::Tagged => "types",
            };
            if ancestors.first().map(String::as_str) != Some(payload) {
                return None;
            }
            if sequence_item && ancestors.len() == 1 && payload == "$schema" {
                Some(vec![MetaSchemaKind::Schema])
            } else {
                Some(vec![MetaSchemaKind::TypeDefinition])
            }
        }
        SchemaAuthoringState::Frontmatter(values) => {
            if key == "$schema" && ancestors.is_empty() {
                return Some(vec![MetaSchemaKind::Schema]);
            }
            if ancestors.first().map(String::as_str) == Some("$schema") {
                return if sequence_item && ancestors.len() == 1 {
                    Some(vec![MetaSchemaKind::Schema])
                } else {
                    Some(vec![MetaSchemaKind::TypeDefinition])
                };
            }

            let owner = ancestors
                .first()
                .map(String::as_str)
                .or_else(|| (!key.is_empty()).then_some(key))?;
            let value = values.iter().find(|value| {
                value.pointer.strip_prefix('/') == Some(owner)
            })?;
            if (ancestors.len() > 1
                || (!ancestors.is_empty() && !sequence_item && !key.is_empty() && key != owner))
                && value.kinds.contains(&MetaSchemaKind::Schema)
            {
                return Some(vec![MetaSchemaKind::TypeDefinition]);
            }
            Some(value.kinds.clone())
        }
        SchemaAuthoringState::Inactive => None,
    }
}

/// Completion for a cursor inside a `type-definition` value, dispatched on the
/// structural role the shared parser-state authority reports.
fn type_definition_completions(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    state: &SchemaCursor,
) -> Vec<CompletionItem> {
    let partial = state.token.as_str();
    match &state.role {
        SchemaCursorRole::Constraint { subject, array_level } => {
            return constraint_completions(
                ctx,
                start,
                offset,
                partial,
                subject.as_deref(),
                *array_level,
            );
        }
        // A constraint's arguments are author-supplied values (a regex body, a
        // glob, an enum member); the catalog has nothing to offer there.
        SchemaCursorRole::Argument { .. } => return Vec::new(),
        // An inline object's keys are the author's own property names.
        SchemaCursorRole::InlineObjectKey => return Vec::new(),
        // The file half of `Name@reference` names a schema file, not a type
        // keyword — the catalog must not leak into it.
        SchemaCursorRole::ImportReference { .. } => return Vec::new(),
        SchemaCursorRole::Type => {}
    }

    let mut items = Vec::new();
    for descriptor in schema_type_descriptors() {
        if descriptor.keyword.starts_with(partial)
            && let Some(completion) = item(
                ctx,
                start,
                offset,
                descriptor.keyword,
                descriptor.keyword,
                CompletionItemKind::TYPE_PARAMETER,
                Some(descriptor.description.to_string()),
            )
        {
            items.push(completion);
        }
        let array = format!("{}[]", descriptor.keyword);
        if array.starts_with(partial)
            && let Some(completion) = item(
                ctx,
                start,
                offset,
                &array,
                &array,
                CompletionItemKind::TYPE_PARAMETER,
                Some(format!("Array of {} values", descriptor.keyword)),
            )
        {
            items.push(completion);
        }
    }
    for scaffold in ["{}", "[]", "Name@./types.yaml"] {
        if scaffold.starts_with(partial)
            && let Some(completion) = item(
                ctx,
                start,
                offset,
                scaffold,
                scaffold,
                CompletionItemKind::SNIPPET,
                Some("SimplifiedSchema definition scaffold".to_string()),
            )
        {
            items.push(completion);
        }
    }
    for name in passive_namespace_names(ctx) {
        let imported = format!("{name}@this");
        if imported.starts_with(partial)
            && let Some(completion) = item(
                ctx,
                start,
                offset,
                &imported,
                &imported,
                CompletionItemKind::REFERENCE,
                Some("Named type from the current passive schema namespace".to_string()),
            )
        {
            items.push(completion);
        }
    }
    items
}

/// Constraint-keyword completion for a cursor inside a `(…)` list.
///
/// Which constraints are legal depends on the list's level, which only the
/// parser can decide: an item-level list is bounded by the subject type's
/// `accepted_constraints`, while the postfix `[](…)` list carries the separate
/// array surface ([`SchemaConstraintDescriptor::accepts_array_level`]). A
/// subject the catalog does not know (a half-typed keyword, an import name)
/// offers nothing.
fn constraint_completions(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    partial: &str,
    subject: Option<&str>,
    array_level: bool,
) -> Vec<CompletionItem> {
    let item_descriptor = if array_level {
        None
    } else {
        let Some(descriptor) = subject.and_then(|subject| {
            schema_type_descriptors()
                .iter()
                .find(|descriptor| descriptor.keyword == subject)
        }) else {
            return Vec::new();
        };
        Some(descriptor)
    };
    schema_constraint_descriptors()
        .iter()
        .filter(|constraint| !constraint.keyword.starts_with('<'))
        .filter(|constraint| match item_descriptor {
            Some(descriptor) => accepts_constraint(descriptor, constraint.keyword),
            None => constraint.accepts_array_level(),
        })
        .filter(|constraint| constraint.keyword.starts_with(partial))
        .filter_map(|constraint| {
            let insert = match constraint.keyword {
                "default" => "default()".to_string(),
                _ if constraint.form.contains('(') => constraint.form.to_string(),
                keyword => keyword.to_string(),
            };
            item(
                ctx,
                start,
                offset,
                constraint.keyword,
                &insert,
                CompletionItemKind::PROPERTY,
                Some(constraint.description.to_string()),
            )
        })
        .collect()
}

/// Whether a type descriptor's published item-level constraint list names
/// `keyword`.
///
/// The list is prose (`"eager, match(glob, ...), default, required"`), so each
/// entry is compared as a whole keyword rather than as a substring — otherwise
/// `match`'s own argument list would make unrelated keywords appear accepted.
fn accepts_constraint(
    descriptor: &darkmatter::markdown::schemas::SchemaTypeDescriptor,
    keyword: &str,
) -> bool {
    descriptor
        .accepted_constraints
        .split([',', ';'])
        .filter_map(|entry| entry.trim().split('(').next())
        .any(|entry| entry.trim() == keyword)
}

fn passive_namespace_names(ctx: &DocumentContext) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Some(overlay) = ctx.overlay else {
        return names;
    };
    let schema = match &overlay.schema_authoring {
        // A whole-file reference document is active but declares no inline
        // properties, so it contributes no passive namespace names.
        SchemaAuthoringState::Standalone { model: Some(model), .. } => model.schema(),
        SchemaAuthoringState::Frontmatter(_) => None,
        _ => None,
    };
    if let Some(schema) = schema {
        collect_schema_property_names(schema, &mut names);
        return names;
    }

    let Some(schema_entry) = overlay.ast.as_deref().and_then(FrontmatterAst::schema_entry) else {
        return names;
    };
    if let Some(ast) = overlay.ast.as_deref() {
        for entry in ast.entries() {
            if entry.depth == 1 && entry.pointer.starts_with("/$schema/") {
                names.insert(entry.key.clone());
            }
        }
        if !names.is_empty() {
            return names;
        }
    }
    let Some(source) = ctx.text.get(schema_entry.value_span.clone()) else {
        return names;
    };
    let source = dedent_spanned_yaml(source, source_column(ctx.text, schema_entry.value_span.start));
    let Ok(yaml) = serde_yaml_ng::from_str(&source) else {
        return names;
    };
    if let Ok(SchemaDeclaration::Schema(schema)) = parse_schema_declaration(&yaml) {
        collect_schema_property_names(&schema, &mut names);
    }
    names
}

fn collect_schema_property_names(schema: &SimplifiedSchema, names: &mut BTreeSet<String>) {
    match schema {
        SimplifiedSchema::Single(shape) => names.extend(shape.properties.keys().cloned()),
        SimplifiedSchema::Union(arms) => {
            for arm in arms {
                if let SchemaArm::Inline(shape) = arm {
                    names.extend(shape.properties.keys().cloned());
                }
            }
        }
    }
}

fn schema_value_completions(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    partial: &str,
) -> Vec<CompletionItem> {
    let mut items = file_path_completions(ctx, offset, partial);
    for scaffold in ["{}", "[]", "./schema.yaml"] {
        if scaffold.starts_with(partial)
            && let Some(completion) = item(
                ctx,
                start,
                offset,
                scaffold,
                scaffold,
                CompletionItemKind::FILE,
                Some("Schema declaration scaffold".to_string()),
            )
        {
            items.push(completion);
        }
    }
    items
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
    let ancestors = enclosing_path(ctx, offset, line_start, indent);
    let ancestor_refs: Vec<&str> = ancestors.iter().map(String::as_str).collect();
    let shape = known_shape(ctx);
    if let Some(nested) = nested_shape_for_completion(ctx, &shape, &ancestor_refs) {
        let present = present_child_keys(ast, &ancestors);
        return shape_key_completions(ctx, offset, &nested, &present, partial);
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
/// completion can exclude them.
///
/// Parent identity is structural, so a same-named mapping elsewhere in the tree
/// (e.g. under `$schema`) cannot leak in and a key containing `.` is reported
/// as itself rather than as two nested segments.
fn present_child_keys<'a>(ast: &'a FrontmatterAst, ancestors: &[String]) -> Vec<&'a str> {
    let path: Vec<&str> = ancestors.iter().map(String::as_str).collect();
    ast.children_of_key_path(&path)
        .into_iter()
        .map(|entry| entry.key.as_str())
        .collect()
}

/// Enum members, boolish scaffolds, file paths, or `suggest(...)` candidates for
/// a key's value. `ancestors` is the parent mapping path (empty at the top
/// level); the leaf atom is resolved by descending inline objects.
fn value_completions(
    ctx: &DocumentContext,
    offset: usize,
    ancestors: &[String],
    key: &str,
    partial: &str,
) -> Vec<CompletionItem> {
    let start = offset - partial.len();
    let mut path: Vec<&str> = ancestors.iter().map(String::as_str).collect();
    path.push(key);

    // Flow-array item: `key: [partial_or_empty, more]`
    if partial.trim_start().starts_with('[') {
        let (element_partial, element_start) = flow_array_element(partial, start);
        return suggestion_completions(ctx, &path, element_start, offset, &element_partial)
            .unwrap_or_default();
    }

    // Suggestion candidates for scalar values (checked before enum/boolish/file
    // so a suggest-bearing union arm wins per the spec).
    if let Some(items) = suggestion_completions(ctx, &path, start, offset, partial) {
        return items;
    }

    let shape = known_shape(ctx);
    let Some(def) = def_at_path_ctx(ctx, &shape, &path) else {
        return Vec::new();
    };
    let def: &PropertyDef = &def;

    let mut items = Vec::new();

    // Each authored `literal(x)` value, preselected, with YAML insertion text
    // matching the value's scalar type.
    for atom in atoms_of(def) {
        if let Some(value) = atom.literal_value() {
            let insert = yaml_scalar_literal(value);
            if insert.starts_with(partial)
                && let Some(completion) = literal_value_item(ctx, start, offset, &insert)
            {
                items.push(completion);
            }
        }
    }

    // Expression-typed arm: the shared catalog completion (see
    // `expression_value_completions`).
    if expression_atom(def).is_some() {
        items.extend(expression_value_completions(ctx, offset, partial, start));
    }

    // Scaffolds from EVERY non-Literal arm (enum members, boolish `true`/`false`,
    // file paths), accumulated with the items above rather than stopping at the
    // first completable arm — otherwise a mixed union would silently drop its
    // later arms. `atoms_of` is in declaration order, so a preselected literal
    // precedes any colliding scaffold and survives `dedup_completions` below.
    for atom in atoms_of(def) {
        if let Some(members) = enum_members(atom) {
            items.extend(
                members
                    .iter()
                    .filter(|member| member.starts_with(partial))
                    .filter_map(|member| {
                        item(ctx, start, offset, member, member, CompletionItemKind::ENUM_MEMBER, None)
                    }),
            );
        } else if is_boolish(atom) {
            items.extend(["true", "false"].into_iter().filter(|value| value.starts_with(partial)).filter_map(
                |value| item(ctx, start, offset, value, value, CompletionItemKind::VALUE, None),
            ));
        } else if is_file(atom) {
            items.extend(file_path_completions(ctx, offset, partial));
        }
    }

    dedup_completions(items)
}

/// Deduplicates completion items by label and inserted text, preserving
/// first-seen order. A merged union list (literal + expression + scaffold arms)
/// can otherwise repeat an item two arms both produce; keeping the first
/// occurrence lets a preselected literal offered ahead of a colliding
/// expression candidate retain its preselection.
fn dedup_completions(items: Vec<CompletionItem>) -> Vec<CompletionItem> {
    let mut seen = std::collections::HashSet::new();
    items
        .into_iter()
        .filter(|item| {
            let insert = match &item.text_edit {
                Some(CompletionTextEdit::Edit(edit)) => edit.new_text.clone(),
                _ => item.label.clone(),
            };
            seen.insert((item.label.clone(), insert))
        })
        .collect()
}

/// A preselected completion item offering exactly one authored `literal(x)`
/// value, with an eager text edit inserting valid YAML for its scalar type.
fn literal_value_item(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    insert_text: &str,
) -> Option<CompletionItem> {
    let range = ctx.source_map.byte_range_to_lsp(start..offset)?;
    Some(CompletionItem {
        label: insert_text.to_string(),
        kind: Some(CompletionItemKind::VALUE),
        preselect: Some(true),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range,
            new_text: insert_text.to_string(),
        })),
        ..Default::default()
    })
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

// ── Expression-typed values ─────────────────────────────────────────────────

/// Completion inside an Expression-typed frontmatter value: the shared
/// `ctx.*` / function / frontmatter-key catalog, scoped to the trailing
/// identifier token of the value text (so a `.` after `ctx` or a `(` after a
/// function name re-triggers cleanly). `value_partial` is the value text after
/// `key:` up to the cursor and `value_start` its document offset.
fn expression_value_completions(
    ctx: &DocumentContext,
    offset: usize,
    value_partial: &str,
    value_start: usize,
) -> Vec<CompletionItem> {
    let (token_start, token) = expressions::value_completion_partial(value_partial, value_start);
    let frontmatter_keys: Vec<String> = ctx
        .overlay
        .and_then(|overlay| overlay.ast.as_ref())
        .map(|ast| ast.top_level().map(|entry| entry.key.clone()).collect())
        .unwrap_or_default();
    expressions::completion_candidates(token, &frontmatter_keys)
        .into_iter()
        .filter_map(|candidate| expr_completion_item(ctx, token_start, offset, candidate))
        .collect()
}

/// Lowers a neutral [`expressions::ExprCompletion`] to an LSP item with an eager
/// text edit over `start..offset` and eager Markdown documentation.
fn expr_completion_item(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    candidate: expressions::ExprCompletion,
) -> Option<CompletionItem> {
    let range = ctx.source_map.byte_range_to_lsp(start..offset)?;
    let kind = match candidate.kind {
        expressions::ExprCompletionKind::FrontmatterKey => CompletionItemKind::FIELD,
        expressions::ExprCompletionKind::ContextVariable => CompletionItemKind::VARIABLE,
        expressions::ExprCompletionKind::Function => CompletionItemKind::FUNCTION,
    };
    Some(CompletionItem {
        label: candidate.label,
        kind: Some(kind),
        detail: candidate.detail,
        documentation: candidate.documentation.map(|value| {
            Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: doc_links::resolve(&value, ctx.path).into_owned(),
            })
        }),
        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
            range,
            new_text: candidate.insert_text,
        })),
        ..Default::default()
    })
}

/// An Expression-typed scalar frontmatter value: the authored entry plus the
/// decoded expression text with a projection from decoded byte offsets back to
/// authored document bytes (YAML quotes excluded).
pub(crate) struct ExpressionValue<'a> {
    /// The authored entry — `value_span` is the whole authored value range.
    pub(crate) entry: &'a FmEntry,
    /// The decoded expression + byte map, relative to `value_span.start`.
    decoded: DecodedScalar,
}

impl ExpressionValue<'_> {
    /// The decoded expression text the compose parser sees.
    pub(crate) fn expression(&self) -> &str {
        self.decoded.decoded()
    }

    /// Projects a decoded byte range to a document span, YAML quotes excluded.
    pub(crate) fn project(&self, range: std::ops::Range<usize>) -> Option<SourceSpan> {
        let base = self.entry.value_span.start;
        self.decoded.project(range).map(|r| base + r.start..base + r.end)
    }

    /// The whole decoded-expression authored span (quotes excluded), falling
    /// back to the whole value node when projection is impossible.
    pub(crate) fn expression_span(&self) -> SourceSpan {
        self.project(0..self.decoded.decoded().len())
            .unwrap_or_else(|| self.entry.value_span.clone())
    }

    /// The decoded byte offset for a document cursor inside the value.
    fn decoded_offset(&self, doc_offset: usize) -> usize {
        self.decoded
            .decoded_offset(doc_offset.saturating_sub(self.entry.value_span.start))
    }
}

/// Whether an atom is an `expression`-typed scalar.
fn is_expression(atom: &PropertyAtom) -> bool {
    matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::Expression))
}

/// The first `expression`-typed arm of a property, if any — so a union whose
/// expression arm is not first is still recognized.
fn expression_atom(def: &PropertyDef) -> Option<&PropertyAtom> {
    atoms_of(def).iter().find(|atom| is_expression(atom))
}

/// Every Expression-typed **scalar** frontmatter value, in document order. A
/// mapping/sequence value on an expression property is intentionally excluded —
/// it stays a schema type mismatch, not an expression.
pub(crate) fn expression_values<'a>(
    ctx: &DocumentContext,
    ast: &'a FrontmatterAst,
) -> Vec<ExpressionValue<'a>> {
    let shape = known_shape(ctx);
    let mut out = Vec::new();
    for (index, entry) in ast.entries().iter().enumerate() {
        if entry.kind != FmValueKind::Scalar {
            continue;
        }
        let path = ast.key_path_at(index);
        let Some(def) = def_at_path_ctx(ctx, &shape, &path) else {
            continue;
        };
        if expression_atom(&def).is_none() {
            continue;
        }
        let Some(raw) = ctx.text.get(entry.value_span.clone()) else {
            continue;
        };
        let Some(decoded) = decode_scalar(raw) else {
            continue;
        };
        out.push(ExpressionValue { entry, decoded });
    }
    out
}

/// The Expression-typed scalar value whose authored value span contains
/// `offset`.
fn expression_value_at<'a>(
    ctx: &DocumentContext,
    ast: &'a FrontmatterAst,
    offset: usize,
) -> Option<ExpressionValue<'a>> {
    expression_values(ctx, ast)
        .into_iter()
        .find(|value| value.entry.value_span.contains(&offset))
}

/// Hover on an Expression-typed frontmatter value, via the shared
/// [`expressions::hover_markdown_condition`] authority (condition dialect, so
/// `&&`/`||` values hover like any other expression). Fires only when the cursor
/// is inside the value (a cursor on the key falls through to the schema hover,
/// which describes the `expression` type itself).
fn expression_hover(ctx: &DocumentContext, ast: &FrontmatterAst, offset: usize) -> Option<Hover> {
    let value = expression_value_at(ctx, ast, offset)?;
    let expr_offset = value.decoded_offset(offset);
    let markdown = expressions::hover_markdown_condition(
        value.expression(),
        expr_offset,
        |name| frontmatter_scalar(ctx, name),
        |name| schema_property_details(ctx, name),
    );
    markup_hover(ctx, value.expression_span(), markdown)
}

/// The static scalar value of a top-level frontmatter key, for expression hover.
fn frontmatter_scalar(ctx: &DocumentContext, name: &str) -> Option<String> {
    ctx.overlay
        .and_then(|overlay| overlay.ast.as_ref())
        .and_then(|ast| ast.entry_by_dotted(name))
        .and_then(|entry| entry.scalar.clone())
}

/// The heading-less schema hover details for a declared top-level property, for
/// a bare expression identifier that names a caller-supplied parameter.
fn schema_property_details(ctx: &DocumentContext, name: &str) -> Option<String> {
    let shape = known_shape(ctx);
    let def = def_at_path_ctx(ctx, &shape, &[name])?;
    schema_hover_details(&def)
}

// ── Suggestion completion ──────────────────────────────────────────────────

/// Builds suggestion completion items for a property path, filtered by `prefix`.
/// Returns `None` when the property has no `suggest(...)` constraint or the path
/// does not resolve.
///
/// The property definition is resolved through the same selected-arm-aware
/// [`def_at_path_ctx`] every other value capability uses, then handed to the
/// library's context-aware [`suggestions_for_def`]. This keeps `suggest(...)`
/// candidates on the arm the discriminant selects (and, when narrowing is
/// unavailable, on the merged view of every arm) instead of the context-free
/// first-arm choice a raw path query would make.
fn suggestion_completions(
    ctx: &DocumentContext,
    property_path: &[&str],
    start: usize,
    offset: usize,
    prefix: &str,
) -> Option<Vec<CompletionItem>> {
    let (leaf, _) = property_path.split_last()?;
    let shape = known_shape(ctx);
    let def = def_at_path_ctx(ctx, &shape, property_path)?;
    let query = suggestions_for_def(leaf, &def)?;
    let items: Vec<CompletionItem> = query
        .items
        .iter()
        .filter(|suggestion| suggestion.decoded.starts_with(prefix))
        .filter_map(|suggestion| {
            item(
                ctx,
                start,
                offset,
                &suggestion.label,
                &suggestion.insert_text,
                CompletionItemKind::VALUE,
                None,
            )
        })
        .collect();
    Some(items)
}

/// Detects a block-sequence item (`- value`) and returns suggestion completions
/// when the enclosing property is suggestion-eligible.
fn block_array_suggestions(
    ctx: &DocumentContext,
    offset: usize,
    line_start: usize,
    indent: usize,
    trimmed: &str,
) -> Option<Vec<CompletionItem>> {
    let (after_dash, marker_len) = if let Some(after_dash) = trimmed.strip_prefix("- ") {
        (after_dash, 2)
    } else if trimmed == "-" {
        ("", 1)
    } else {
        return None;
    };
    let ancestors = enclosing_path(ctx, offset, line_start, indent);
    let path: Vec<&str> = ancestors.iter().map(String::as_str).collect();
    let item_start = line_start + indent + marker_len;
    suggestion_completions(ctx, &path, item_start, offset, after_dash)
}

/// Extracts the current element text and its byte-offset start from a flow-array
/// value partial (the text after `key:` up to the cursor, starting with `[`).
///
/// Returns `(element_partial, element_start_byte)` where `element_partial` is
/// the text of the element being typed (possibly empty) and `element_start_byte`
/// is the absolute byte offset where a text edit should begin.
fn flow_array_element(value_partial: &str, value_start: usize) -> (String, usize) {
    let last_sep = value_partial.rfind([',', '[']);
    match last_sep {
        Some(pos) => {
            let after_sep = &value_partial[pos + 1..];
            let trimmed_element = after_sep.trim_start();
            let leading_ws = after_sep.len() - trimmed_element.len();
            let element_start = value_start + pos + 1 + leading_ws;
            (trimmed_element.to_string(), element_start)
        }
        None => (value_partial.to_string(), value_start),
    }
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
    if let Some(hover) = meta_schema_hover(ctx, offset) {
        return Some(hover);
    }
    let ast = overlay_ast(ctx)?;
    if !ast.contains_offset(offset) {
        return None;
    }

    // A cursor inside an Expression-typed value gets the shared expression hover
    // (parsed form + `ctx.*`/function catalog); a cursor on its key falls
    // through to the schema hover describing the `expression` type.
    if let Some(hover) = expression_hover(ctx, ast, offset) {
        return Some(hover);
    }

    let entry = ast.entry_at_offset(offset)?;
    let path = ast.key_path(entry);

    if path.first() == Some(&"ctx") {
        return ctx_hover(ctx, entry);
    }
    schema_hover(ctx, &path, entry)
}

fn meta_schema_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let overlay = ctx.overlay?;

    if let Some(ast) = overlay.ast.as_deref()
        && let Some(entry) = ast.entry_at_offset(offset)
        && let SchemaAuthoringState::Frontmatter(values) = &overlay.schema_authoring
        && let Some(value) = values.iter().find(|value| value.pointer == entry.pointer)
        && value.kinds.contains(&MetaSchemaKind::TypeDefinition)
    {
        let source = ctx.text.get(entry.value_span.clone())?;
        let source = dedent_spanned_yaml(source, source_column(ctx.text, entry.value_span.start));
        let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source).ok()?;
        let definition = parse_property_definition(&entry.key, &yaml).ok()?;
        return markup_hover(
            ctx,
            entry.key_span.clone(),
            meta_schema_definition_hover_body(&entry.key, &definition),
        );
    }

    if let SchemaAuthoringState::Standalone { model: Some(model), .. } =
        &overlay.schema_authoring
        && let Some(schema) = model.schema()
        && let Some(region) = semantic_type_regions(schema, &model.source_map)
            .into_iter()
            .find(|region| {
                region.key_span.contains(&offset) || region.definition_span.contains(&offset)
            })
    {
        return markup_hover(
            ctx,
            region.key_span,
            meta_schema_definition_hover_body(&region.name, &region.definition),
        );
    }

    let ast = overlay.ast.as_deref()?;
    let schema_entry = ast.schema_entry()?;
    let source = ctx.text.get(schema_entry.value_span.clone())?;
    let source = dedent_spanned_yaml(
        source,
        source_column(ctx.text, schema_entry.value_span.start),
    );
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source).ok()?;
    let parsed = parse_schema_declaration_with_source(
        &yaml,
        &source,
        schema_entry.value_span.start,
    )
    .ok()?;
    let SchemaDeclaration::Schema(schema) = parsed.value else {
        return None;
    };
    let region = semantic_type_regions(&schema, &parsed.source_map)
        .into_iter()
        .find(|region| {
            region.key_span.contains(&offset) || region.definition_span.contains(&offset)
        })?;
    markup_hover(
        ctx,
        region.key_span,
        meta_schema_definition_hover_body(&region.name, &region.definition),
    )
}

fn source_column(text: &str, offset: usize) -> usize {
    text[..offset].rsplit_once('\n').map_or(offset, |(_, line)| line.len())
}

fn dedent_spanned_yaml(source: &str, indent: usize) -> String {
    let mut normalized = String::with_capacity(source.len());
    for (index, line) in source.lines().enumerate() {
        if index > 0 {
            normalized.push('\n');
        }
        normalized.push_str(if index == 0 {
            line
        } else {
            line.strip_prefix(&" ".repeat(indent)).unwrap_or(line)
        });
    }
    if source.ends_with('\n') {
        normalized.push('\n');
    }
    normalized
}

fn meta_schema_definition_hover_body(key: &str, def: &PropertyDef) -> String {
    let mut denoted = Vec::new();
    for atom in atoms_of(def) {
        let mut name = match &atom.ty {
            TypeExpr::Primitive(ty) => ty.as_keyword().to_string(),
            TypeExpr::InlineObject(_) => "object".to_string(),
            TypeExpr::Imported { name, reference } => format!("{name}@{reference}"),
        };
        if atom.is_array {
            name.push_str("[]");
        }
        if !denoted.contains(&name) {
            denoted.push(name);
        }
    }
    let mut lines = vec![
        format!("**`{key}`**"),
        "Type: **type-definition**".to_string(),
        format!("Declares: **{}**", denoted.join(" | ")),
    ];
    if is_required(def) {
        lines.push("Required".to_string());
    }
    lines.join("\n\n")
}

/// Hover content for a schema-declared property, at any nesting depth.
fn schema_hover(ctx: &DocumentContext, path: &[&str], entry: &FmEntry) -> Option<Hover> {
    let shape = known_shape(ctx);
    let def = def_at_path_ctx(ctx, &shape, path)?;
    let body = schema_hover_body(&entry.key, &def)?;
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
    let details = schema_hover_details(def)?;
    Some(format!("**`{key}`**\n\n{details}"))
}

/// The schema-hover body **without** the leading `**`key`**` heading: type,
/// required, enum, default, and description. Callers that already display the
/// property name in their own header (e.g. the interpolation hover's
/// `**Expression**` block) use this to avoid repeating the name.
pub(crate) fn schema_hover_details(def: &PropertyDef) -> Option<String> {
    let atom = primary_atom(def)?;
    let mut lines = Vec::new();

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

    // A `literal(x)` atom shows its exact pinned value directly under the type,
    // ahead of the shared constraint/description lines.
    if let Some(value) = atom.literal_value() {
        lines.push(format!("Value: _{value}_"));
    }

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
    // objects. Any union arm being a `file` type makes the value a navigable
    // reference.
    let shape = known_shape(ctx);
    for (index, entry) in ast.entries().iter().enumerate() {
        if entry.kind != FmValueKind::Scalar {
            continue;
        }
        let path = ast.key_path_at(index);
        if def_at_path_ctx(ctx, &shape, &path).is_some_and(|def| file_atom(&def).is_some())
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
        match &bundle.effective.simplified {
            Some(SimplifiedSchema::Single(document)) => {
                for (name, def) in &document.properties {
                    shape.properties.insert(name.clone(), def.clone());
                }
            }
            // A root `$schema` union: overlay the arm a shared literal
            // discriminant selects (so top-level completion/hover narrows), or
            // every arm's keys merged when no arm is unambiguously selected.
            Some(SimplifiedSchema::Union(arms)) => {
                overlay_root_union(&mut shape, arms, &bundle.frontmatter_json);
            }
            None => {}
        }
    }
    shape
}

/// Overlays a root `$schema` union's arm properties onto the effective
/// top-level shape.
///
/// When the top-level frontmatter mapping selects exactly one arm via the
/// shared literal discriminant (the Phase-4 [`select_literal_discriminant_arm`]
/// — never a second, DMLS-only algorithm), only that arm's properties overlay,
/// so top-level key completion offers just the matched arm's remaining keys.
/// Before a discriminant is present, or for an unknown, duplicate, or
/// conflicting discriminant, every inline arm's properties merge instead via
/// [`merged_root_arm_shape`]: a key present in more than one arm resolves to the
/// union of those arms' atoms (via [`merge_defs`]), so a shared property whose
/// type diverges across arms stays a property union rather than collapsing to
/// the last arm. A file-reference arm contributes no directly-known properties
/// here.
fn overlay_root_union(shape: &mut SchemaShape, arms: &[SchemaArm], frontmatter_json: &Value) {
    let arm_json: Vec<Value> = arms.iter().map(root_arm_discriminant_json).collect();
    match select_literal_discriminant_arm(&arm_json, frontmatter_json) {
        Some(index) => overlay_arm(shape, &arms[index]),
        // The merge is across arms only; the merged document shape still takes
        // precedence over the base/extension baseline it overlays.
        None => {
            for (name, def) in merged_root_arm_shape(arms).properties {
                shape.properties.insert(name, def);
            }
        }
    }
}

/// Overlays one inline root-union arm's properties, ignoring a file-reference
/// arm (its properties are not resolved into the arm shape here).
fn overlay_arm(shape: &mut SchemaShape, arm: &SchemaArm) {
    if let SchemaArm::Inline(arm_shape) = arm {
        for (name, def) in &arm_shape.properties {
            shape.properties.insert(name.clone(), def.clone());
        }
    }
}

/// A merged view of every inline root-union arm's properties, for a root
/// `$schema` union whose discriminant does not select a single arm. Keys appear
/// in arm-declaration then property-declaration order; a key contributed by more
/// than one arm resolves to the union of those arms' atoms (via [`merge_defs`]),
/// so a same-named property whose type diverges across arms stays a union rather
/// than collapsing to the last arm. File-reference arms contribute no
/// properties. The root-`SchemaArm` companion to [`merged_inline_object_shape`].
fn merged_root_arm_shape(arms: &[SchemaArm]) -> SchemaShape {
    let mut merged = SchemaShape::default();
    for arm in arms {
        let SchemaArm::Inline(arm_shape) = arm else {
            continue;
        };
        for (name, child) in &arm_shape.properties {
            let combined = match merged.properties.get(name) {
                Some(existing) => merge_defs(existing, child),
                None => child.clone(),
            };
            merged.properties.insert(name.clone(), combined);
        }
    }
    merged
}

/// The minimal discriminant JSON for one root-union arm, aligned by index with
/// the arm slice so the selector's returned index maps back. A file-reference
/// arm contributes no discriminant properties but still occupies its slot.
fn root_arm_discriminant_json(arm: &SchemaArm) -> Value {
    match arm {
        SchemaArm::Inline(shape) => shape_discriminant_json(shape),
        SchemaArm::FileRef(_) => serde_json::json!({}),
    }
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

/// The nested completion shape for `ancestors`, like [`nested_shape`] but
/// context-aware at each level.
///
/// When an ancestor property is a union of inline-object arms tagged by a shared
/// `literal(...)` discriminant, and the authored sibling values in the current
/// mapping select exactly one arm (via the Phase-4
/// [`select_literal_discriminant_arm`] — never a second, DMLS-only algorithm),
/// the walk descends into that arm's shape so only its keys are offered. When
/// narrowing is unavailable (absent/unknown/duplicate/conflicting discriminant,
/// or an ordinary non-discriminated union), it descends into a MERGED view of
/// every inline-object arm instead — the [`overlay_root_union`] policy applied
/// to nested unions — so sibling completion/hover/navigation retain union
/// behavior rather than guessing the first arm (spec D3 / AC-10). The merged
/// shape is owned, so this returns a [`SchemaShape`] by value.
fn nested_shape_for_completion(
    ctx: &DocumentContext,
    root: &SchemaShape,
    ancestors: &[&str],
) -> Option<SchemaShape> {
    let mut shape = root.clone();
    for depth in 0..ancestors.len() {
        let def = shape.properties.get(ancestors[depth])?;
        let path = &ancestors[..=depth];
        let next = match discriminated_arm_shape(ctx, def, path) {
            Some(selected) => selected.clone(),
            None => merged_inline_object_shape(def)?,
        };
        shape = next;
    }
    Some(shape)
}

/// A merged view of every inline-object arm's properties, for an ancestor union
/// whose discriminant does not select a single arm. Keys appear in
/// arm-declaration then property-declaration order; a key contributed by more
/// than one arm resolves to the union of those arms' atoms (via [`merge_defs`]),
/// so a same-named property whose type diverges across arms stays a union rather
/// than collapsing to the first arm. `None` when the property has no
/// inline-object arm.
fn merged_inline_object_shape(def: &PropertyDef) -> Option<SchemaShape> {
    let mut merged: Option<SchemaShape> = None;
    for atom in atoms_of(def) {
        let TypeExpr::InlineObject(inner) = &atom.ty else {
            continue;
        };
        let shape = merged.get_or_insert_with(SchemaShape::default);
        for (name, child) in &inner.properties {
            let combined = match shape.properties.get(name) {
                Some(existing) => merge_defs(existing, child),
                None => child.clone(),
            };
            shape.properties.insert(name.clone(), combined);
        }
    }
    merged
}

/// Merges two property definitions: `existing`'s atoms followed by any of
/// `incoming`'s atoms not already present, deduped by value and kept in
/// declaration order. Collapses to [`PropertyDef::Single`] when exactly one atom
/// survives.
fn merge_defs(existing: &PropertyDef, incoming: &PropertyDef) -> PropertyDef {
    let mut atoms: Vec<PropertyAtom> = atoms_of(existing).to_vec();
    for atom in atoms_of(incoming) {
        if !atoms.contains(atom) {
            atoms.push(atom.clone());
        }
    }
    match atoms.len() {
        1 => PropertyDef::Single(atoms.into_iter().next().expect("one atom")),
        _ => PropertyDef::Union(atoms),
    }
}

/// The inline-object shape of the union arm a shared literal discriminant selects
/// for the mapping at `path`, or `None` when the property is not a discriminated
/// inline-object union or no arm is unambiguously selected.
///
/// The authored instance is the already-parsed, correctly-typed frontmatter
/// mapping at `path` (from the effective schema bundle), so a string `'2'` never
/// matches a numeric `literal(2)`. Arm projection preserves atom order so the
/// selector's returned index lines up with [`atoms_of`].
fn discriminated_arm_shape<'a>(
    ctx: &DocumentContext,
    def: &'a PropertyDef,
    path: &[&str],
) -> Option<&'a SchemaShape> {
    let atoms = atoms_of(def);
    if atoms.len() < 2 {
        return None;
    }
    let arms: Vec<Value> = atoms.iter().map(arm_discriminant_json).collect();
    let bundle = ctx.overlay.and_then(|overlay| overlay.bundle())?;
    let instance = navigate_json(&bundle.frontmatter_json, path)?;
    let index = select_literal_discriminant_arm(&arms, instance)?;
    match &atoms[index].ty {
        TypeExpr::InlineObject(shape) => Some(shape),
        _ => None,
    }
}

/// Projects one property-union arm to the minimal JSON the discriminant selector
/// reads: `{ "properties": { key: { "const": <value> } } }` for each of the
/// arm's `literal(...)`-typed properties. A non-inline-object arm yields an empty
/// object (no discriminant properties) but still occupies its slot so the
/// selector's arm index stays aligned with [`atoms_of`].
fn arm_discriminant_json(atom: &PropertyAtom) -> Value {
    match &atom.ty {
        TypeExpr::InlineObject(shape) => shape_discriminant_json(shape),
        _ => serde_json::json!({}),
    }
}

/// The minimal discriminant JSON the selector reads for one inline-object
/// shape: `{ "properties": { key: { "const": <value> } } }` for each of its
/// `literal(...)`-typed properties.
fn shape_discriminant_json(shape: &SchemaShape) -> Value {
    let mut props = serde_json::Map::new();
    for (key, def) in &shape.properties {
        if let PropertyDef::Single(child) = def
            && let Some(value) = child.literal_value()
        {
            props.insert(key.clone(), serde_json::json!({ "const": value }));
        }
    }
    serde_json::json!({ "properties": props })
}

/// Walks `path` into a JSON object, returning the value at that key path.
fn navigate_json<'a>(root: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = current.as_object()?.get(*segment)?;
    }
    Some(current)
}

/// The [`PropertyDef`] at a full key `path` (ancestor segments followed by the
/// leaf key), descending the **first** inline-object arm of each ancestor. This
/// is the context-free resolver: it never consults the authored mapping, so a
/// discriminated ancestor union always resolves against its first arm.
///
/// `None` when any ancestor is missing/not an inline object, or the leaf key is
/// absent.
///
/// Callers that have a [`DocumentContext`] and must honor a selected
/// discriminated arm use [`def_at_path_ctx`] instead; this pure variant survives
/// for the context-free code-action/DSL callers and the unit tests.
pub(crate) fn def_at_path<'a>(root: &'a SchemaShape, path: &[&str]) -> Option<&'a PropertyDef> {
    let (leaf, ancestors) = path.split_last()?;
    let shape = nested_shape(root, ancestors)?;
    shape.properties.get(*leaf)
}

/// The [`PropertyDef`] at a full key `path`, like [`def_at_path`] but
/// context-aware at each ancestor (via [`nested_shape_for_completion`] → the
/// Phase-4 [`select_literal_discriminant_arm`]).
///
/// This is the single context-aware schema-path resolver shared by every
/// value-oriented capability (value completion, hover, expression
/// gating/diagnostics, file navigation) so they resolve against the same
/// selected arm key completion narrows to. A discriminated ancestor a selected
/// arm resolves descends into exactly that arm; when narrowing is unavailable
/// the ancestor's inline-object arms merge, so a leaf key present in more than
/// one arm with divergent types resolves to the union of every arm's atoms (spec
/// D3 / AC-10). The result is [`Cow`] because a merged leaf must be owned; a
/// top-level leaf (empty ancestor path) borrows from `root`.
fn def_at_path_ctx<'a>(
    ctx: &DocumentContext,
    root: &'a SchemaShape,
    path: &[&str],
) -> Option<Cow<'a, PropertyDef>> {
    let (leaf, ancestors) = path.split_last()?;
    if ancestors.is_empty() {
        return root.properties.get(*leaf).map(Cow::Borrowed);
    }
    let shape = nested_shape_for_completion(ctx, root, ancestors)?;
    shape.properties.get(*leaf).cloned().map(Cow::Owned)
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

/// The sole `literal(x)` value of a property, or `None` when the property is not
/// a single unambiguous literal (a union with several literal arms has no single
/// value, so no correct-by-construction insertion exists). Used by the
/// add-missing-required-key code action to insert the literal instead of an
/// empty scaffold.
pub(crate) fn sole_literal_value(def: &PropertyDef) -> Option<&Value> {
    let mut values = atoms_of(def).iter().filter_map(PropertyAtom::literal_value);
    let first = values.next()?;
    values.next().is_none().then_some(first)
}

/// Serializes a scalar literal value to valid YAML source text: booleans and
/// numbers verbatim, strings bare unless YAML would reparse the bare form as a
/// non-string (then single-quoted — e.g. `'2'`, `'true'`). Backed by the YAML
/// emitter so quoting is correct for every edge case (colons, leading/trailing
/// space, indicator characters), not a hand-rolled heuristic.
pub(crate) fn yaml_scalar_literal(value: &Value) -> String {
    serde_yaml_ng::to_string(value)
        .ok()
        .map(|rendered| rendered.trim_end_matches('\n').to_string())
        .filter(|rendered| !rendered.is_empty())
        .unwrap_or_else(|| value.to_string())
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

/// The chain of ancestor keys enclosing the cursor, outermost first. Empty for
/// a top-level line.
///
/// The frontmatter AST is the authority whenever it describes the *current*
/// buffer: its containment is structural and its key segments are decoded, so a
/// quoted ancestor (`"$schema"`) matches the reserved `$schema` path and an
/// ancestor whose key contains `:` or `.` survives intact. This also costs no
/// reverse line scan.
///
/// A cursor no authored mapping encloses (a blank line being opened), and an
/// entry a last-good tree no longer places correctly, fall back to the
/// indentation walk in [`enclosing_path_by_indent`].
fn enclosing_path(
    ctx: &DocumentContext,
    offset: usize,
    line_start: usize,
    indent: usize,
) -> Vec<String> {
    if let Some(ast) = overlay_ast(ctx) {
        match ast.container_at_offset(offset) {
            Some(container) if still_placed(ctx, container) => {
                return ast.key_path(container).into_iter().map(str::to_string).collect();
            }
            // No enclosing mapping at column 0 is a real answer, not a gap.
            None if indent == 0 => return Vec::new(),
            _ => {}
        }
    }
    enclosing_path_by_indent(ctx.text, line_start, indent)
}

/// Whether `entry`'s key token is still exactly where the tree says it is.
///
/// A last-good tree describes an *earlier* buffer, so a blanket "is it stale"
/// veto would surrender every structural answer the moment one key is mid-edit.
/// Re-reading the key token at its recorded span turns staleness into a
/// per-entry question instead: a malformed edit elsewhere in the block leaves
/// the line being authored structurally owned, and a span that has genuinely
/// moved is rejected rather than trusted.
fn still_placed(ctx: &DocumentContext, entry: &FmEntry) -> bool {
    ctx.text
        .get(entry.key_span.clone())
        .and_then(decode_scalar)
        .is_some_and(|decoded| decoded.decoded() == entry.key)
}

/// The `(key, value-partial)` pair when the cursor sits in a value position.
///
/// Structural first: an authored entry yields the entire *decoded* key, so
/// `"build.target": …` is one key and `"host: port": …` is not split at its
/// embedded colon. The lexical `key:` split is the fallback for a cursor no
/// still-placed entry describes — most often a key part-way through being
/// typed, which leaves the buffer unparseable.
fn value_cursor(
    ctx: &DocumentContext,
    offset: usize,
    line_start: usize,
    trimmed: &str,
) -> Option<(String, String)> {
    if let Some(entry) = overlay_ast(ctx)
        .and_then(|ast| ast.key_entry_on_line(line_start, offset))
        .filter(|entry| still_placed(ctx, entry))
        && let Some(rest) = ctx.text.get(entry.key_span.end..offset)
        && let Some((_, partial)) = rest.split_once(':')
    {
        return Some((entry.key.clone(), partial.trim_start().to_string()));
    }
    let colon = trimmed.find(':')?;
    Some((
        trimmed[..colon].trim().to_string(),
        trimmed[colon + 1..].trim_start().to_string(),
    ))
}

/// The full chain of ancestor keys above `line_start`, outermost first, for a
/// line at column `indent` — one key per strictly-decreasing indent level, so
/// nested inline-object mappings resolve. Empty for a top-level line.
fn enclosing_path_by_indent(text: &str, line_start: usize, indent: usize) -> Vec<String> {
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
            value: doc_links::resolve(&value, ctx.path).into_owned(),
        }),
        range,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::BTreeMap;
    use std::sync::Arc;

    use lsp_types::{InitializeParams, Uri};

    use crate::capabilities::ClientProfile;
    use crate::config::DmlsConfig;
    use crate::graph::WorkspaceGraph;
    use crate::overlay::OverlayState;
    use crate::source_map::{PositionEncoding, SourceMap};

    /// Builds a real overlay-backed [`DocumentContext`] for `text` and runs `f`.
    /// Used by the Expression-typed frontmatter tests, which need the effective
    /// schema and the [`FrontmatterAst`], not just a pure helper.
    fn with_ctx<R>(text: &str, f: impl FnOnce(&DocumentContext) -> R) -> R {
        with_ctx_docs(text, &[], f)
    }

    /// Like [`with_ctx`] but seeds additional workspace documents (`path`,
    /// `source` pairs) into the graph so `file(...)` value completion has
    /// candidate paths to offer.
    fn with_ctx_docs<R>(
        text: &str,
        docs: &[(&str, &str)],
        f: impl FnOnce(&DocumentContext) -> R,
    ) -> R {
        let path = Path::new("/w/doc.md");
        let uri: Uri = "file:///w/doc.md".parse().unwrap();
        let config = DmlsConfig::default();
        let roots = [PathBuf::from("/w")];
        let state = OverlayState::default();
        let overlay = state.for_document(&uri, text, path, &config, &roots);
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let mut indices = BTreeMap::new();
        for (doc_path, source) in docs {
            indices.insert(
                PathBuf::from(doc_path),
                crate::graph::index_document(Path::new(doc_path), source),
            );
        }
        let graph = WorkspaceGraph::build(&indices, 1);
        let profile = ClientProfile::from_initialize(&InitializeParams::default(), PositionEncoding::Utf16);
        let ctx = DocumentContext {
            uri: &uri,
            path,
            text,
            source_map: &source_map,
            graph: &graph,
            doc_id: None,
            config: &config,
            profile: &profile,
            overlay: overlay.as_ref(),
        };
        f(&ctx)
    }

    /// A document whose inline `$schema` types `when` as `expression` and leaves
    /// `title` an ordinary string, so schema gating can be asserted.
    fn expression_doc(value_line: &str) -> String {
        format!("---\n$schema:\n  when: expression\n  title: string\n{value_line}\n---\n\nbody\n")
    }

    #[test]
    fn expression_value_completion_offers_ctx_functions_and_keys() {
        let text = expression_doc("when: ctx.");
        with_ctx(&text, |ctx| {
            let offset = text.find("ctx.").unwrap() + "ctx.".len();
            let items = completion(ctx, offset);
            // Every offered `ctx.*` item carries its rendered type in `detail`.
            let packages = items
                .iter()
                .find(|item| item.label == "ctx.packages")
                .expect("expression value completion offers ctx.packages");
            assert_eq!(packages.detail.as_deref(), Some("string[]"));
            assert_eq!(packages.kind, Some(CompletionItemKind::VARIABLE));
            // The eager text edit replaces exactly the typed `ctx.` token.
            let Some(CompletionTextEdit::Edit(edit)) = &packages.text_edit else {
                panic!("eager text edit");
            };
            assert_eq!(edit.new_text, "ctx.packages");
        });

        // A bare partial also offers functions and same-document frontmatter keys.
        let text = expression_doc("title: hello\nwhen: len");
        with_ctx(&text, |ctx| {
            let offset = text.rfind("len").unwrap() + "len".len();
            let items = completion(ctx, offset);
            assert!(
                items.iter().any(|item| item.label.starts_with("length(")),
                "functions are offered inside an expression value"
            );
        });
        let text = expression_doc("title: hello\nwhen: ti");
        with_ctx(&text, |ctx| {
            let offset = text.rfind("ti").unwrap() + "ti".len();
            let items = completion(ctx, offset);
            assert!(
                items.iter().any(|item| item.label == "title"),
                "same-document frontmatter keys are offered inside an expression value"
            );
        });
    }

    #[test]
    fn non_expression_value_offers_no_expression_completion() {
        // Schema gating: `title` is a plain string, so a `.` in its value offers
        // no `ctx.*`/function completion.
        let text = expression_doc("title: ctx.");
        with_ctx(&text, |ctx| {
            let offset = text.find("ctx.").unwrap() + "ctx.".len();
            let items = completion(ctx, offset);
            assert!(
                items.iter().all(|item| !item.label.starts_with("ctx.")),
                "a non-expression value must not offer expression completion: {items:#?}"
            );
        });
    }

    #[test]
    fn expression_value_hover_is_byte_identical_to_interpolation() {
        let text = expression_doc("when: ctx.today");
        with_ctx(&text, |ctx| {
            let offset = text.find("ctx.today").unwrap() + 2; // on `x` in `ctx`
            let hover = hover(ctx, offset).expect("expression value hover");
            let HoverContents::Markup(MarkupContent { value, .. }) = hover.contents else {
                panic!("markdown hover");
            };
            // Byte-identical to the shared interpolation hover for the same text.
            let expected = expressions::hover_markdown(
                "ctx.today",
                2,
                |_| None,
                |_| None,
            );
            assert_eq!(value, expected);
            let today = expressions::ctx_descriptor("today").unwrap();
            assert!(value.contains(&expressions::format_ctx_hover_block(today)));
            // The hover range excludes the key and covers just the value.
            let value_range = ctx
                .source_map
                .byte_range_to_lsp({
                    let start = text.find("ctx.today").unwrap();
                    start..start + "ctx.today".len()
                });
            assert_eq!(hover.range, value_range);
        });
    }

    #[test]
    fn hover_on_expression_key_keeps_schema_type_hover() {
        // A cursor on the key describes the `expression` type, not the value.
        let text = expression_doc("when: ctx.today");
        with_ctx(&text, |ctx| {
            let offset = text.rfind("when:").unwrap() + 1;
            let hover = hover(ctx, offset).expect("schema key hover");
            let HoverContents::Markup(MarkupContent { value, .. }) = hover.contents else {
                panic!("markdown hover");
            };
            assert!(value.contains("Type: **expression**"), "{value}");
        });
    }

    #[test]
    fn condition_dialect_expression_value_hover_uses_condition_grammar() {
        // The `expression` schema format validates with `parse_condition`; the
        // value hover must parse the same dialect, so `||` lowers to a logical
        // `or(...)` (not the value-dialect fallback) and the `is_empty` function
        // call under the cursor is still enriched.
        let text = expression_doc(r#"when: 'is_empty(title) || is_string(title)'"#);
        with_ctx(&text, |ctx| {
            let offset = text.find("is_empty").unwrap() + 2;
            let hover = hover(ctx, offset).expect("condition expression hover");
            let HoverContents::Markup(MarkupContent { value, .. }) = hover.contents else {
                panic!("markdown hover");
            };
            assert!(value.contains("or("), "condition dialect lowers || to or(): {value}");
            let is_empty = expressions::function_descriptor("is_empty").unwrap();
            assert!(value.contains(&expressions::format_function_block(is_empty)), "{value}");
        });
    }

    #[test]
    fn condition_dialect_expression_value_completion_offers_catalog() {
        // Completion is token-based and dialect-agnostic: a `&&`-containing value
        // still offers `ctx.*` for the trailing token.
        let text = expression_doc(r#"when: 'is_agent() && ctx.'"#);
        with_ctx(&text, |ctx| {
            let offset = text.find("ctx.'").unwrap() + "ctx.".len();
            let items = completion(ctx, offset);
            assert!(
                items.iter().any(|item| item.label == "ctx.packages"),
                "completion inside a condition expression still offers ctx.*: {items:#?}"
            );
        });
    }

    // ── Literal UX (Phase 6) ──

    #[test]
    fn literal_value_completion_offers_exact_value_preselected() {
        let text =
            "---\n$schema:\n  status: literal(published)\n  title: string\nstatus: \n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("status: \n").unwrap() + "status: ".len();
            let items = completion(ctx, offset);
            let published = items
                .iter()
                .find(|item| item.label == "published")
                .expect("literal value is offered");
            assert_eq!(published.preselect, Some(true), "the literal value is preselected");
            let Some(CompletionTextEdit::Edit(edit)) = &published.text_edit else {
                panic!("eager text edit");
            };
            assert_eq!(edit.new_text, "published");
        });
    }

    #[test]
    fn literal_string_value_completion_is_yaml_quoted() {
        // A string literal that looks numeric must insert quoted YAML so it
        // reparses as a string, not a number.
        let text = "---\n$schema:\n  tag: literal('2')\ntag: \n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("tag: \n").unwrap() + "tag: ".len();
            let items = completion(ctx, offset);
            let item = items.iter().find(|item| item.label == "'2'").expect("quoted literal offered");
            let Some(CompletionTextEdit::Edit(edit)) = &item.text_edit else {
                panic!("eager text edit");
            };
            assert_eq!(edit.new_text, "'2'");
        });
    }

    #[test]
    fn literal_union_combines_each_value_with_scaffolds() {
        // A `number` arm contributes no value scaffold, so a literal+number union
        // offers only the literal; an all-literal union offers each arm's value.
        let text =
            "---\n$schema:\n  width:\n    - literal(auto)\n    - number\nwidth: \n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("width: \n").unwrap() + "width: ".len();
            let items = completion(ctx, offset);
            assert!(items.iter().any(|item| item.label == "auto"), "literal arm offered: {items:#?}");
        });

        let text = concat!(
            "---\n$schema:\n  kind:\n    - literal(created)\n    - literal(deleted)\n",
            "kind: \n---\n\nbody\n",
        );
        with_ctx(text, |ctx| {
            let offset = text.find("kind: \n").unwrap() + "kind: ".len();
            let items = completion(ctx, offset);
            assert!(items.iter().any(|item| item.label == "created"), "{items:#?}");
            assert!(items.iter().any(|item| item.label == "deleted"), "{items:#?}");
        });
    }

    /// The deduplicated `(label, inserted-text)` pairs of a completion list, so a
    /// merged union list can be asserted duplicate-free.
    fn completion_keys(items: &[CompletionItem]) -> Vec<(String, String)> {
        items
            .iter()
            .map(|item| {
                let insert = match &item.text_edit {
                    Some(CompletionTextEdit::Edit(edit)) => edit.new_text.clone(),
                    _ => item.label.clone(),
                };
                (item.label.clone(), insert)
            })
            .collect()
    }

    #[test]
    fn literal_expression_union_offers_literal_and_expression_candidates() {
        // A mixed union must not let the Expression arm suppress the Literal value
        // completions, and the merged list must stay duplicate-free — either arm order.
        for arms in [
            "    - literal(auto)\n    - expression\n",
            "    - expression\n    - literal(auto)\n",
        ] {
            let text = format!("---\n$schema:\n  width:\n{arms}width: \n---\n\nbody\n");
            with_ctx(&text, |ctx| {
                let offset = text.find("width: \n").unwrap() + "width: ".len();
                let items = completion(ctx, offset);

                let auto = items
                    .iter()
                    .find(|item| item.label == "auto")
                    .unwrap_or_else(|| panic!("literal `auto` offered for arms `{arms}`: {items:#?}"));
                assert_eq!(
                    auto.preselect,
                    Some(true),
                    "the literal value stays preselected for arms `{arms}`",
                );

                assert!(
                    items.iter().any(|item| item.label == "ctx.packages"),
                    "a catalog-backed `ctx.*` candidate is offered for arms `{arms}`: {items:#?}",
                );
                assert!(
                    items.iter().any(|item| item.label.starts_with("length(")),
                    "an expression function is offered for arms `{arms}`: {items:#?}",
                );

                let keys = completion_keys(&items);
                let mut deduped = keys.clone();
                deduped.sort();
                deduped.dedup();
                assert_eq!(
                    keys.len(),
                    deduped.len(),
                    "the merged union list has no duplicates for arms `{arms}`: {keys:#?}",
                );
            });
        }
    }

    #[test]
    fn literal_enum_boolean_union_offers_every_arm_candidate() {
        // Every non-Literal arm contributes its candidates, not just the first
        // completable one, and the merged list stays duplicate-free.
        let text = concat!(
            "---\n$schema:\n  width:\n",
            "    - literal(auto)\n    - enum(fit, fill)\n    - boolean\n",
            "width: \n---\n\nbody\n",
        );
        with_ctx(text, |ctx| {
            let offset = text.find("width: \n").unwrap() + "width: ".len();
            let items = completion(ctx, offset);
            let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
            for expected in ["auto", "fit", "fill", "true", "false"] {
                assert!(labels.contains(&expected), "`{expected}` offered: {labels:?}");
            }
            let auto = items.iter().find(|item| item.label == "auto").expect("literal offered");
            assert_eq!(auto.preselect, Some(true), "the literal stays preselected");

            let keys = completion_keys(&items);
            let mut deduped = keys.clone();
            deduped.sort();
            deduped.dedup();
            assert_eq!(keys.len(), deduped.len(), "the merged list has no duplicates: {keys:#?}");
        });
    }

    #[test]
    fn reversed_literal_enum_boolean_union_offers_every_arm_candidate() {
        // Arm order must not change the offered set nor the literal's preselection.
        let text = concat!(
            "---\n$schema:\n  width:\n",
            "    - boolean\n    - enum(fit, fill)\n    - literal(auto)\n",
            "width: \n---\n\nbody\n",
        );
        with_ctx(text, |ctx| {
            let offset = text.find("width: \n").unwrap() + "width: ".len();
            let items = completion(ctx, offset);
            let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
            for expected in ["auto", "fit", "fill", "true", "false"] {
                assert!(labels.contains(&expected), "`{expected}` offered: {labels:?}");
            }
            let auto = items.iter().find(|item| item.label == "auto").expect("literal offered");
            assert_eq!(auto.preselect, Some(true), "the literal stays preselected");
        });
    }

    #[test]
    fn file_enum_union_offers_file_paths_and_enum_members() {
        // A union mixing `file` with an enum arm, in either order: the file arm
        // does not suppress the enum arm (nor the reverse).
        for arms in ["    - file\n    - enum(fit, fill)\n", "    - enum(fit, fill)\n    - file\n"] {
            let text = format!("---\n$schema:\n  asset:\n{arms}asset: \n---\n\nbody\n");
            with_ctx_docs(&text, &[("/w/target.md", "# target\n")], |ctx| {
                let offset = text.find("asset: \n").unwrap() + "asset: ".len();
                let items = completion(ctx, offset);
                let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
                assert!(labels.contains(&"fit"), "enum member offered for arms `{arms}`: {labels:?}");
                assert!(labels.contains(&"fill"), "enum member offered for arms `{arms}`: {labels:?}");
                assert!(
                    items.iter().any(|item| item.kind == Some(CompletionItemKind::FILE)),
                    "a file path is offered for arms `{arms}`: {items:#?}",
                );
            });
        }
    }

    #[test]
    fn duplicate_member_across_two_enum_arms_is_deduplicated() {
        // Two enum arms share the member `fit`; the merged list offers `fit`
        // exactly once while every distinct member from both arms survives.
        let text = concat!(
            "---\n$schema:\n  width:\n",
            "    - enum(fit, fill)\n    - enum(fit, snap)\n",
            "width: \n---\n\nbody\n",
        );
        with_ctx(text, |ctx| {
            let offset = text.find("width: \n").unwrap() + "width: ".len();
            let items = completion(ctx, offset);
            let fit_count = items.iter().filter(|item| item.label == "fit").count();
            assert_eq!(fit_count, 1, "the shared `fit` member appears once: {items:#?}");
            let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
            for expected in ["fit", "fill", "snap"] {
                assert!(labels.contains(&expected), "`{expected}` offered: {labels:?}");
            }
        });
    }

    #[test]
    fn literal_colliding_with_enum_member_keeps_preselected_literal() {
        // When a preselected literal collides with an enum member, the surviving
        // deduped entry keeps the literal's preselection.
        let text = concat!(
            "---\n$schema:\n  width:\n",
            "    - literal(fit)\n    - enum(fit, fill)\n",
            "width: \n---\n\nbody\n",
        );
        with_ctx(text, |ctx| {
            let offset = text.find("width: \n").unwrap() + "width: ".len();
            let items = completion(ctx, offset);
            let fits: Vec<&CompletionItem> = items.iter().filter(|item| item.label == "fit").collect();
            assert_eq!(fits.len(), 1, "the colliding `fit` appears once: {items:#?}");
            assert_eq!(fits[0].preselect, Some(true), "the preselected literal wins the collision");
            assert!(items.iter().any(|item| item.label == "fill"), "enum `fill` still offered: {items:#?}");
        });
    }

    #[test]
    fn literal_hover_shows_type_and_exact_value() {
        let text = "---\n$schema:\n  status: literal(published)\nstatus: published\n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.rfind("status:").unwrap() + 1; // on the key
            let hover = hover(ctx, offset).expect("literal key hover");
            let HoverContents::Markup(MarkupContent { value, .. }) = hover.contents else {
                panic!("markdown hover");
            };
            assert!(value.contains("Type: **literal**"), "{value}");
            assert!(value.contains("Value: _\"published\"_"), "{value}");
        });
    }

    #[test]
    fn sibling_completion_narrows_to_discriminated_arm() {
        // `change` is a union of two inline-object arms tagged by `kind`. With
        // `kind: created` authored, sibling completion offers only the created
        // arm's `path`, never the deleted arm's `reason`.
        let text = concat!(
            "---\n",
            "$schema:\n",
            "  change:\n",
            "    - \"{ kind: literal(created), path: string }\"\n",
            "    - \"{ kind: literal(deleted), reason: string }\"\n",
            "change:\n",
            "  kind: created\n",
            "  \n",
            "---\n\nbody\n",
        );
        with_ctx(text, |ctx| {
            // The cursor sits on the two-space empty line under `change`.
            let offset = text.find("  kind: created\n  \n").unwrap() + "  kind: created\n  ".len();
            let items = completion(ctx, offset);
            let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
            assert!(labels.contains(&"path"), "created arm key offered: {labels:?}");
            assert!(!labels.contains(&"reason"), "deleted arm key suppressed: {labels:?}");
        });
    }

    /// A `change` union of two inline-object arms `{ kind: literal(created),
    /// path: string }` and `{ kind: literal(deleted), reason: string }`, whose
    /// only shared key is the `kind` discriminant. `deleted_first` swaps arm
    /// order. `mapping` is the authored `change:` sub-mapping (each line already
    /// indented two spaces, with a trailing newline); it must end with a bare
    /// two-space cursor line for [`nested_sibling_labels`].
    fn path_reason_union_doc(deleted_first: bool, mapping: &str) -> String {
        let created = r#"    - "{ kind: literal(created), path: string }""#;
        let deleted = r#"    - "{ kind: literal(deleted), reason: string }""#;
        let (first, second) = if deleted_first { (deleted, created) } else { (created, deleted) };
        format!("---\n$schema:\n  change:\n{first}\n{second}\nchange:\n{mapping}---\n\nbody\n")
    }

    /// Sibling key-completion labels with the cursor on the trailing bare
    /// two-space line of a `change:` mapping.
    fn nested_sibling_labels(text: &str) -> Vec<String> {
        with_ctx(text, |ctx| {
            let offset = text.rfind("  \n").expect("trailing cursor line") + "  ".len();
            completion(ctx, offset).into_iter().map(|item| item.label).collect()
        })
    }

    #[test]
    fn sibling_completion_without_discriminant_keeps_union_behavior() {
        // Narrowing unavailable (no discriminant authored): every arm's keys
        // merge — the created arm's `path` AND the deleted arm's `reason`, plus
        // the shared discriminant `kind` — in either arm order (D3 / AC-10).
        for deleted_first in [false, true] {
            let text = path_reason_union_doc(deleted_first, "  \n");
            let labels = nested_sibling_labels(&text);
            assert!(labels.iter().any(|l| l == "kind"), "discriminant offered ({deleted_first}): {labels:?}");
            assert!(labels.iter().any(|l| l == "path"), "created arm key offered ({deleted_first}): {labels:?}");
            assert!(labels.iter().any(|l| l == "reason"), "deleted arm key offered ({deleted_first}): {labels:?}");
        }
    }

    #[test]
    fn sibling_completion_unknown_discriminant_keeps_union_behavior() {
        // `kind: renamed` matches no arm → union: both `path` and `reason` offered.
        for deleted_first in [false, true] {
            let text = path_reason_union_doc(deleted_first, "  kind: renamed\n  \n");
            let labels = nested_sibling_labels(&text);
            assert!(labels.iter().any(|l| l == "path"), "created arm key ({deleted_first}): {labels:?}");
            assert!(labels.iter().any(|l| l == "reason"), "deleted arm key ({deleted_first}): {labels:?}");
        }
    }

    #[test]
    fn sibling_completion_duplicate_discriminant_keeps_union_behavior() {
        // Both arms tag `kind: created`, so `kind: created` is ambiguous → union.
        let text = concat!(
            "---\n$schema:\n  change:\n",
            "    - \"{ kind: literal(created), path: string }\"\n",
            "    - \"{ kind: literal(created), reason: string }\"\n",
            "change:\n  kind: created\n  \n---\n\nbody\n",
        );
        let labels = nested_sibling_labels(text);
        assert!(labels.iter().any(|l| l == "path"), "first arm key offered: {labels:?}");
        assert!(labels.iter().any(|l| l == "reason"), "second arm key offered: {labels:?}");
    }

    #[test]
    fn sibling_completion_type_mismatched_discriminant_keeps_union_behavior() {
        // Numeric `literal(2)`/`literal(3)` discriminants; a string `'2'` matches
        // neither const → union.
        let text = concat!(
            "---\n$schema:\n  change:\n",
            "    - \"{ kind: literal(2), path: string }\"\n",
            "    - \"{ kind: literal(3), reason: string }\"\n",
            "change:\n  kind: '2'\n  \n---\n\nbody\n",
        );
        let labels = nested_sibling_labels(text);
        assert!(labels.iter().any(|l| l == "path"), "arm-0 key offered under union: {labels:?}");
        assert!(labels.iter().any(|l| l == "reason"), "arm-1 key offered under union: {labels:?}");
    }

    #[test]
    fn sibling_completion_conflicting_discriminants_keeps_union_behavior() {
        // `kind` selects arm 0, `mode` selects arm 1 → conflict → union.
        let text = concat!(
            "---\n$schema:\n  change:\n",
            "    - \"{ kind: literal(created), mode: literal(fast), path: string }\"\n",
            "    - \"{ kind: literal(deleted), mode: literal(slow), reason: string }\"\n",
            "change:\n  kind: created\n  mode: slow\n  \n---\n\nbody\n",
        );
        let labels = nested_sibling_labels(text);
        assert!(labels.iter().any(|l| l == "path"), "arm-0 key offered under union: {labels:?}");
        assert!(labels.iter().any(|l| l == "reason"), "arm-1 key offered under union: {labels:?}");
    }

    #[test]
    fn sibling_completion_narrows_to_selected_arm_in_both_orders() {
        // Regression: an unambiguous `kind: deleted` still descends into exactly
        // the deleted arm — only `reason`, never `path` — regardless of arm order.
        for deleted_first in [false, true] {
            let text = path_reason_union_doc(deleted_first, "  kind: deleted\n  \n");
            let labels = nested_sibling_labels(&text);
            assert!(labels.iter().any(|l| l == "reason"), "deleted arm key offered ({deleted_first}): {labels:?}");
            assert!(!labels.iter().any(|l| l == "path"), "created arm key suppressed ({deleted_first}): {labels:?}");
        }
    }

    // ── Nested discriminated-arm value intelligence (selected-arm resolver) ──

    /// The Markdown body of the hover at `offset`.
    fn hover_markup(ctx: &DocumentContext, offset: usize) -> String {
        let hover = hover(ctx, offset).expect("hover present");
        match hover.contents {
            HoverContents::Markup(MarkupContent { value, .. }) => value,
            _ => panic!("markdown hover"),
        }
    }

    /// A `change` union of two inline-object arms that share property names with
    /// divergent types (`when`: string vs expression, `asset`: string vs file,
    /// `state`: literal(open) vs literal(done)), tagged by a shared literal
    /// `kind` discriminant. `deleted_first` swaps arm order so a test can assert
    /// the selected arm drives value intelligence from either position.
    /// `mapping` is the authored `change:` sub-mapping (each line already
    /// indented two spaces, with a trailing newline).
    fn change_union_doc(deleted_first: bool, mapping: &str) -> String {
        let created =
            r#"    - "{ kind: literal(created), when: string, asset: string, state: literal(open) }""#;
        let deleted =
            r#"    - "{ kind: literal(deleted), when: expression, asset: file, state: literal(done) }""#;
        let (first, second) = if deleted_first { (deleted, created) } else { (created, deleted) };
        format!("---\n$schema:\n  change:\n{first}\n{second}\nchange:\n{mapping}---\n\nbody\n")
    }

    #[test]
    fn selected_arm_expression_value_offers_catalog_completion() {
        // With `kind: deleted`, the deleted arm's `when: expression` drives value
        // completion — `ctx.` offers the shared catalog — in BOTH arm orders.
        for deleted_first in [false, true] {
            let text = change_union_doc(deleted_first, "  kind: deleted\n  when: ctx.\n");
            with_ctx(&text, |ctx| {
                let offset = text.find("when: ctx.").unwrap() + "when: ctx.".len();
                let items = completion(ctx, offset);
                assert!(
                    items.iter().any(|item| item.label == "ctx.packages"),
                    "selected deleted arm (deleted_first={deleted_first}) offers expression completion: {items:#?}"
                );
            });
        }
    }

    #[test]
    fn non_selected_arm_expression_type_does_not_leak_to_completion() {
        // With `kind: created`, the created arm's `when: string` governs, so the
        // value is not an expression and `ctx.` offers no catalog even though the
        // sibling deleted arm types `when` as expression.
        for deleted_first in [false, true] {
            let text = change_union_doc(deleted_first, "  kind: created\n  when: ctx.\n");
            with_ctx(&text, |ctx| {
                let offset = text.find("when: ctx.").unwrap() + "when: ctx.".len();
                let items = completion(ctx, offset);
                assert!(
                    items.iter().all(|item| !item.label.starts_with("ctx.")),
                    "selected created arm (deleted_first={deleted_first}) offers no expression completion: {items:#?}"
                );
            });
        }
    }

    #[test]
    fn selected_arm_expression_value_gets_expression_hover() {
        // A cursor in the deleted arm's `when` value renders the shared
        // expression catalog hover, in BOTH arm orders.
        for deleted_first in [false, true] {
            let text = change_union_doc(deleted_first, "  kind: deleted\n  when: ctx.today\n");
            with_ctx(&text, |ctx| {
                let offset = text.find("ctx.today").unwrap() + 2; // on `x` in `ctx`
                let value = hover_markup(ctx, offset);
                let today = expressions::ctx_descriptor("today").unwrap();
                assert!(
                    value.contains(&expressions::format_ctx_hover_block(today)),
                    "selected deleted arm (deleted_first={deleted_first}) hovers the expression catalog: {value}"
                );
            });
        }
    }

    #[test]
    fn schema_hover_on_property_key_resolves_selected_arm() {
        // Hover on the `when` key resolves the selected arm's type: expression
        // for the deleted arm, string for the created arm — proving the
        // non-selected arm's divergent `when` type never leaks.
        for deleted_first in [false, true] {
            let deleted = change_union_doc(deleted_first, "  kind: deleted\n  when: x\n");
            with_ctx(&deleted, |ctx| {
                let offset = deleted.find("when: x").unwrap() + 1; // on the key
                let value = hover_markup(ctx, offset);
                assert!(
                    value.contains("Type: **expression**"),
                    "deleted arm `when` is expression (deleted_first={deleted_first}): {value}"
                );
            });
            let created = change_union_doc(deleted_first, "  kind: created\n  when: x\n");
            with_ctx(&created, |ctx| {
                let offset = created.find("when: x").unwrap() + 1;
                let value = hover_markup(ctx, offset);
                assert!(
                    value.contains("Type: **string**"),
                    "created arm `when` is string (deleted_first={deleted_first}): {value}"
                );
            });
        }
    }

    #[test]
    fn selected_arm_literal_value_completion_is_offered_preselected() {
        // The deleted arm types `state: literal(done)`; completing its value
        // offers `done` (preselected), never the created arm's `open`.
        for deleted_first in [false, true] {
            let text = change_union_doc(deleted_first, "  kind: deleted\n  state: \n");
            with_ctx(&text, |ctx| {
                let offset = text.find("state: \n").unwrap() + "state: ".len();
                let items = completion(ctx, offset);
                let done = items.iter().find(|item| item.label == "done").unwrap_or_else(|| {
                    panic!("deleted arm literal `done` offered (deleted_first={deleted_first}): {items:#?}")
                });
                assert_eq!(done.preselect, Some(true), "the literal value is preselected");
                assert!(
                    items.iter().all(|item| item.label != "open"),
                    "the created arm's literal `open` must not leak: {items:#?}"
                );
            });
        }
    }

    #[test]
    fn selected_arm_file_value_is_navigable() {
        // The deleted arm types `asset: file`; its value becomes a navigation
        // target. The created arm types `asset: string`, so the same value is
        // inert.
        for deleted_first in [false, true] {
            let deleted = change_union_doc(deleted_first, "  kind: deleted\n  asset: ./target.md\n");
            with_ctx(&deleted, |ctx| {
                let ast = overlay_ast(ctx).expect("frontmatter ast");
                let targets = nav_targets(ctx, ast);
                assert!(
                    targets.iter().any(|(_, path)| path.ends_with("target.md")),
                    "deleted arm `asset` navigates (deleted_first={deleted_first}): {targets:#?}"
                );
            });
            let created = change_union_doc(deleted_first, "  kind: created\n  asset: ./target.md\n");
            with_ctx(&created, |ctx| {
                let ast = overlay_ast(ctx).expect("frontmatter ast");
                let targets = nav_targets(ctx, ast);
                assert!(
                    targets.iter().all(|(_, path)| !path.ends_with("target.md")),
                    "created arm `asset` is a plain string, not navigable (deleted_first={deleted_first}): {targets:#?}"
                );
            });
        }
    }

    #[test]
    fn unresolved_union_offers_expression_completion_from_divergent_arm() {
        // No `kind` authored: `when` merges to `string | expression`, so its
        // value offers the expression catalog in BOTH arm orders (the expression
        // arm is not dropped).
        for deleted_first in [false, true] {
            let text = change_union_doc(deleted_first, "  when: ctx.\n");
            with_ctx(&text, |ctx| {
                let offset = text.find("when: ctx.").unwrap() + "when: ctx.".len();
                let items = completion(ctx, offset);
                assert!(
                    items.iter().any(|item| item.label == "ctx.packages"),
                    "unresolved union keeps the expression arm (deleted_first={deleted_first}): {items:#?}"
                );
            });
        }
    }

    #[test]
    fn unresolved_union_expression_value_gets_expression_hover() {
        // The merged `when: string | expression` value still hovers the shared
        // expression catalog in BOTH arm orders.
        for deleted_first in [false, true] {
            let text = change_union_doc(deleted_first, "  when: ctx.today\n");
            with_ctx(&text, |ctx| {
                let offset = text.find("ctx.today").unwrap() + 2; // on `x` in `ctx`
                let value = hover_markup(ctx, offset);
                let today = expressions::ctx_descriptor("today").unwrap();
                assert!(
                    value.contains(&expressions::format_ctx_hover_block(today)),
                    "unresolved union hovers the expression catalog (deleted_first={deleted_first}): {value}"
                );
            });
        }
    }

    #[test]
    fn unresolved_union_offers_both_arms_literal_values() {
        // `state: literal(open) | literal(done)` — the unresolved union offers
        // BOTH literal values in either arm order.
        for deleted_first in [false, true] {
            let text = change_union_doc(deleted_first, "  state: \n");
            with_ctx(&text, |ctx| {
                let offset = text.find("state: \n").unwrap() + "state: ".len();
                let items = completion(ctx, offset);
                let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
                assert!(labels.contains(&"open"), "created arm literal ({deleted_first}): {labels:?}");
                assert!(labels.contains(&"done"), "deleted arm literal ({deleted_first}): {labels:?}");
            });
        }
    }

    #[test]
    fn unresolved_union_file_arm_makes_value_navigable() {
        // `asset: string | file` — the unresolved union treats the value as a
        // file reference (the file arm survives) in either arm order.
        for deleted_first in [false, true] {
            let text = change_union_doc(deleted_first, "  asset: ./target.md\n");
            with_ctx(&text, |ctx| {
                let ast = overlay_ast(ctx).expect("frontmatter ast");
                let targets = nav_targets(ctx, ast);
                assert!(
                    targets.iter().any(|(_, path)| path.ends_with("target.md")),
                    "unresolved union file arm navigates (deleted_first={deleted_first}): {targets:#?}"
                );
            });
        }
    }

    // ── Nested discriminated-arm `suggest(...)` completion ──

    /// A `change` union of two inline-object arms that both declare a `palette`
    /// property with divergent `suggest(...)` candidates, tagged by a shared
    /// literal `kind` discriminant. `deleted_first` swaps arm order so a test can
    /// assert suggestion completion follows the selected arm from either position.
    fn palette_union_doc(deleted_first: bool, mapping: &str) -> String {
        let created = r#"    - "{ kind: literal(created), palette: string(suggest(red, green)) }""#;
        let deleted = r#"    - "{ kind: literal(deleted), palette: string(suggest(cyan, magenta)) }""#;
        let (first, second) = if deleted_first { (deleted, created) } else { (created, deleted) };
        format!("---\n$schema:\n  change:\n{first}\n{second}\nchange:\n{mapping}---\n\nbody\n")
    }

    /// Value-completion labels for the `palette` value of a `change:` mapping.
    fn palette_value_labels(text: &str) -> Vec<String> {
        with_ctx(text, |ctx| {
            let offset = text.find("palette: \n").expect("palette value line") + "palette: ".len();
            completion(ctx, offset).into_iter().map(|item| item.label).collect()
        })
    }

    #[test]
    fn selected_arm_suggest_follows_discriminant_in_both_orders() {
        // Selection follows the discriminant, not arm position: the non-selected
        // arm's `suggest(...)` never leaks even when the selected arm is the SECOND
        // arm (half the cases here).
        for deleted_first in [false, true] {
            let deleted = palette_union_doc(deleted_first, "  kind: deleted\n  palette: \n");
            let labels = palette_value_labels(&deleted);
            assert!(
                labels.iter().any(|l| l == "cyan") && labels.iter().any(|l| l == "magenta"),
                "deleted arm suggestions offered (deleted_first={deleted_first}): {labels:?}"
            );
            assert!(
                labels.iter().all(|l| l != "red" && l != "green"),
                "created arm suggestions must not leak (deleted_first={deleted_first}): {labels:?}"
            );

            let created = palette_union_doc(deleted_first, "  kind: created\n  palette: \n");
            let labels = palette_value_labels(&created);
            assert!(
                labels.iter().any(|l| l == "red") && labels.iter().any(|l| l == "green"),
                "created arm suggestions offered (deleted_first={deleted_first}): {labels:?}"
            );
            assert!(
                labels.iter().all(|l| l != "cyan" && l != "magenta"),
                "deleted arm suggestions must not leak (deleted_first={deleted_first}): {labels:?}"
            );
        }
    }

    #[test]
    fn unresolved_union_suggest_merges_every_arm_candidate() {
        // No `kind` authored: `palette` merges to a union carrying both arms'
        // `suggest(...)` atoms, so its value completion offers every arm's
        // candidates in BOTH arm orders (spec D3 / AC-10 merged behavior).
        for deleted_first in [false, true] {
            let text = palette_union_doc(deleted_first, "  palette: \n");
            let labels = palette_value_labels(&text);
            for expected in ["red", "green", "cyan", "magenta"] {
                assert!(
                    labels.iter().any(|l| l == expected),
                    "merged union offers `{expected}` (deleted_first={deleted_first}): {labels:?}"
                );
            }
        }
    }

    #[test]
    fn unknown_discriminant_suggest_merges_every_arm_candidate() {
        // `kind: renamed` matches no arm → union: every arm's `palette`
        // suggestions merge, in BOTH arm orders.
        for deleted_first in [false, true] {
            let text = palette_union_doc(deleted_first, "  kind: renamed\n  palette: \n");
            let labels = palette_value_labels(&text);
            for expected in ["red", "green", "cyan", "magenta"] {
                assert!(
                    labels.iter().any(|l| l == expected),
                    "unknown discriminant merges `{expected}` (deleted_first={deleted_first}): {labels:?}"
                );
            }
        }
    }

    // ── Root `$schema` union key completion (Phase 6, D3) ──

    /// Top-level completion labels with the cursor on the empty line that
    /// immediately follows `marker` (a whole authored line including its
    /// trailing newline). The empty line is a bare top-level key position.
    fn top_level_labels_after(text: &str, marker: &str) -> Vec<String> {
        with_ctx(text, |ctx| {
            let offset = text.find(marker).expect("marker present") + marker.len();
            completion(ctx, offset).into_iter().map(|item| item.label).collect()
        })
    }

    /// A root `$schema` union of two inline arms tagged by a shared literal
    /// `kind` discriminant, with the authored top-level frontmatter `body` and a
    /// trailing empty line for the cursor.
    fn kind_root_union_doc(body: &str) -> String {
        format!(
            "---\n$schema:\n  - kind: literal(created)\n    path: string\n  \
             - kind: literal(deleted)\n    reason: string\n{body}\n---\n\nbody\n"
        )
    }

    #[test]
    fn root_union_key_completion_narrows_to_matched_arm() {
        // `kind: created` selects the first arm, so only its `path` key is
        // offered — never the deleted arm's `reason`.
        let text = kind_root_union_doc("kind: created");
        let labels = top_level_labels_after(&text, "kind: created\n");
        assert!(labels.iter().any(|l| l == "path"), "created arm key offered: {labels:?}");
        assert!(!labels.iter().any(|l| l == "reason"), "deleted arm key suppressed: {labels:?}");
    }

    #[test]
    fn root_union_key_completion_absent_discriminant_keeps_union() {
        // No discriminant authored: every arm's keys merge (union behavior).
        let text = kind_root_union_doc("");
        let labels = top_level_labels_after(&text, "reason: string\n");
        assert!(labels.iter().any(|l| l == "path"), "created arm key offered: {labels:?}");
        assert!(labels.iter().any(|l| l == "reason"), "deleted arm key offered: {labels:?}");
        assert!(labels.iter().any(|l| l == "kind"), "the discriminant key is offered: {labels:?}");
    }

    #[test]
    fn root_union_key_completion_unknown_value_keeps_union() {
        // `kind: renamed` matches no arm → union behavior.
        let text = kind_root_union_doc("kind: renamed");
        let labels = top_level_labels_after(&text, "kind: renamed\n");
        assert!(labels.iter().any(|l| l == "path"), "created arm key offered: {labels:?}");
        assert!(labels.iter().any(|l| l == "reason"), "deleted arm key offered: {labels:?}");
    }

    #[test]
    fn root_union_key_completion_duplicate_value_keeps_union() {
        // Both arms tag `kind: created`, so the value is ambiguous → union.
        let text = concat!(
            "---\n$schema:\n  - kind: literal(created)\n    path: string\n  ",
            "- kind: literal(created)\n    reason: string\nkind: created\n\n---\n\nbody\n",
        );
        let labels = top_level_labels_after(text, "kind: created\n");
        assert!(labels.iter().any(|l| l == "path"), "first arm key offered: {labels:?}");
        assert!(labels.iter().any(|l| l == "reason"), "second arm key offered: {labels:?}");
    }

    #[test]
    fn root_union_key_completion_is_type_sensitive() {
        // A numeric `literal(2)` discriminant. The string `'2'` must not select
        // the numeric arm (union behavior), while the number `2` narrows.
        let arms = "  - version: literal(2)\n    path: string\n  \
                    - version: literal(3)\n    reason: string\n";

        let mismatch = format!("---\n$schema:\n{arms}version: '2'\n\n---\n\nbody\n");
        let labels = top_level_labels_after(&mismatch, "version: '2'\n");
        assert!(labels.iter().any(|l| l == "path"), "arm-0 key offered under union: {labels:?}");
        assert!(labels.iter().any(|l| l == "reason"), "arm-1 key offered under union: {labels:?}");

        let matched = format!("---\n$schema:\n{arms}version: 2\n\n---\n\nbody\n");
        let labels = top_level_labels_after(&matched, "version: 2\n");
        assert!(labels.iter().any(|l| l == "path"), "numeric 2 narrows to arm 0: {labels:?}");
        assert!(!labels.iter().any(|l| l == "reason"), "arm-1 key suppressed: {labels:?}");
    }

    #[test]
    fn root_union_key_completion_conflicting_discriminants_keeps_union() {
        // `kind` selects arm 0, `mode` selects arm 1 — the two discriminants
        // disagree, so narrowing is abandoned (union behavior).
        let text = concat!(
            "---\n$schema:\n  ",
            "- kind: literal(created)\n    mode: literal(fast)\n    path: string\n  ",
            "- kind: literal(deleted)\n    mode: literal(slow)\n    reason: string\n",
            "kind: created\nmode: slow\n\n---\n\nbody\n",
        );
        let labels = top_level_labels_after(text, "mode: slow\n");
        assert!(labels.iter().any(|l| l == "path"), "arm-0 key offered under union: {labels:?}");
        assert!(labels.iter().any(|l| l == "reason"), "arm-1 key offered under union: {labels:?}");
    }

    #[test]
    fn root_union_key_completion_sparse_unknown_discriminant_keeps_union() {
        // Sparse arms: `kind` tags arms 0/1, `mode` tags arms 2/3. `kind: created`
        // alone selects arm 0, but `mode: unknown` is a qualifying discriminant
        // matching no arm, so narrowing is abandoned (union behavior) rather than
        // narrowing to arm 0 on `kind` alone.
        let text = concat!(
            "---\n$schema:\n  ",
            "- kind: literal(created)\n    one: string\n  ",
            "- kind: literal(deleted)\n    two: string\n  ",
            "- mode: literal(fast)\n    three: string\n  ",
            "- mode: literal(slow)\n    four: string\n",
            "kind: created\nmode: unknown\n\n---\n\nbody\n",
        );
        let labels = top_level_labels_after(text, "mode: unknown\n");
        assert!(labels.iter().any(|l| l == "one"), "arm-0 key offered under union: {labels:?}");
        assert!(labels.iter().any(|l| l == "three"), "arm-2 key offered under union: {labels:?}");
    }

    // ── Root `$schema` union shared-property merge (D3 / AC-10) ──

    /// The effective top-level `PropertyDef` for `key` in the document's known
    /// shape (with the root-`$schema`-union overlay applied).
    fn root_shape_def(text: &str, key: &str) -> PropertyDef {
        with_ctx(text, |ctx| {
            known_shape(ctx)
                .properties
                .get(key)
                .cloned()
                .unwrap_or_else(|| panic!("root shape has `{key}`"))
        })
    }

    /// A root `$schema` union of two inline arms tagged by a shared literal
    /// `kind` discriminant, each declaring the shared property `shared` with a
    /// (possibly divergent) definition. `deleted_first` swaps arm declaration
    /// order; `body` is the authored top-level frontmatter that sets the
    /// discriminant state (empty, or `kind:`/`mode:` lines with trailing
    /// newlines).
    fn shared_root_union_doc(
        created_def: &str,
        deleted_def: &str,
        deleted_first: bool,
        body: &str,
    ) -> String {
        let created = format!("  - kind: literal(created)\n    shared: {created_def}");
        let deleted = format!("  - kind: literal(deleted)\n    shared: {deleted_def}");
        let (first, second) =
            if deleted_first { (&deleted, &created) } else { (&created, &deleted) };
        format!("---\n$schema:\n{first}\n{second}\n{body}---\n\nbody\n")
    }

    #[test]
    fn root_union_shared_property_merges_divergent_defs_when_unresolved() {
        // No discriminant authored: a property declared in BOTH root-union arms
        // merges to the union of both arms' atoms (never last-wins) — proven for
        // Literal, Expression, File, and suggest() divergences, in either arm
        // order.
        for deleted_first in [false, true] {
            let literals = shared_root_union_doc("literal(created)", "literal(deleted)", deleted_first, "");
            let def = root_shape_def(&literals, "shared");
            let values: Vec<&Value> =
                atoms_of(&def).iter().filter_map(PropertyAtom::literal_value).collect();
            assert!(
                values.contains(&&Value::from("created")) && values.contains(&&Value::from("deleted")),
                "both literal arms survive (deleted_first={deleted_first}): {def:?}"
            );

            let expr = shared_root_union_doc("string", "expression", deleted_first, "");
            let def = root_shape_def(&expr, "shared");
            assert_eq!(atoms_of(&def).len(), 2, "string|expression keeps both (deleted_first={deleted_first}): {def:?}");
            assert!(expression_atom(&def).is_some(), "the expression arm survives (deleted_first={deleted_first}): {def:?}");

            let file = shared_root_union_doc("string", "file", deleted_first, "");
            let def = root_shape_def(&file, "shared");
            assert_eq!(atoms_of(&def).len(), 2, "string|file keeps both (deleted_first={deleted_first}): {def:?}");
            assert!(file_atom(&def).is_some(), "the file arm survives (deleted_first={deleted_first}): {def:?}");

            let suggest = shared_root_union_doc(
                "string(suggest(red, green))",
                "string(suggest(cyan, magenta))",
                deleted_first,
                "",
            );
            let def = root_shape_def(&suggest, "shared");
            assert_eq!(atoms_of(&def).len(), 2, "both suggest arms survive (deleted_first={deleted_first}): {def:?}");
        }
    }

    #[test]
    fn root_union_shared_property_merges_for_every_unresolved_state() {
        // Every discriminant state that fails to select one arm merges the
        // shared property to the union of both arms' atoms (string | number).
        for deleted_first in [false, true] {
            let absent = shared_root_union_doc("string", "number", deleted_first, "");
            assert_eq!(atoms_of(&root_shape_def(&absent, "shared")).len(), 2, "absent (deleted_first={deleted_first})");
            let unknown = shared_root_union_doc("string", "number", deleted_first, "kind: renamed\n");
            assert_eq!(atoms_of(&root_shape_def(&unknown, "shared")).len(), 2, "unknown (deleted_first={deleted_first})");
        }

        // Both arms tag `kind: created`, so the discriminant is ambiguous.
        let duplicate = concat!(
            "---\n$schema:\n",
            "  - kind: literal(created)\n    shared: string\n",
            "  - kind: literal(created)\n    shared: number\n",
            "kind: created\n---\n\nbody\n",
        );
        assert_eq!(atoms_of(&root_shape_def(duplicate, "shared")).len(), 2, "duplicate discriminant merges");

        // String `'2'` matches neither numeric const.
        let mismatch = concat!(
            "---\n$schema:\n",
            "  - kind: literal(2)\n    shared: string\n",
            "  - kind: literal(3)\n    shared: number\n",
            "kind: '2'\n---\n\nbody\n",
        );
        assert_eq!(atoms_of(&root_shape_def(mismatch, "shared")).len(), 2, "type-mismatched discriminant merges");

        // `kind` selects arm 0, `mode` selects arm 1 — the discriminants disagree.
        let conflicting = concat!(
            "---\n$schema:\n",
            "  - kind: literal(created)\n    mode: literal(fast)\n    shared: string\n",
            "  - kind: literal(deleted)\n    mode: literal(slow)\n    shared: number\n",
            "kind: created\nmode: slow\n---\n\nbody\n",
        );
        assert_eq!(atoms_of(&root_shape_def(conflicting, "shared")).len(), 2, "conflicting discriminants merge");
    }

    #[test]
    fn root_union_matched_arm_exposes_only_selected_shape() {
        // Regression: an unambiguous `kind: created` still narrows the root to
        // arm 0, so only its `shared` atom (string) survives — the deleted arm's
        // `number` atom is never merged in — regardless of arm order.
        for deleted_first in [false, true] {
            let text = shared_root_union_doc("string", "number", deleted_first, "kind: created\n");
            let def = root_shape_def(&text, "shared");
            assert_eq!(atoms_of(&def).len(), 1, "matched root exposes one arm (deleted_first={deleted_first}): {def:?}");
            assert_eq!(
                primary_atom(&def).unwrap().ty,
                TypeExpr::Primitive(SimplifiedType::String),
                "matched arm keeps the created arm's string type (deleted_first={deleted_first})"
            );
        }
    }

    #[test]
    fn test_yaml_scalar_literal_quotes_when_required() {
        assert_eq!(yaml_scalar_literal(&Value::from("created")), "created");
        assert_eq!(yaml_scalar_literal(&Value::from("2")), "'2'");
        assert_eq!(yaml_scalar_literal(&Value::from("true")), "'true'");
        assert_eq!(yaml_scalar_literal(&Value::from(2)), "2");
        assert_eq!(yaml_scalar_literal(&Value::from(true)), "true");
    }

    #[test]
    fn test_sole_literal_value_is_none_for_multi_literal_union() {
        let single = PropertyDef::Single({
            let mut atom = PropertyAtom::bare(SimplifiedType::Literal);
            atom.constraints.push(Constraint::LiteralValue(Value::from("x")));
            atom
        });
        assert_eq!(sole_literal_value(&single), Some(&Value::from("x")));

        let literal = |value: &str| {
            let mut atom = PropertyAtom::bare(SimplifiedType::Literal);
            atom.constraints.push(Constraint::LiteralValue(Value::from(value)));
            atom
        };
        let multi = PropertyDef::Union(vec![literal("a"), literal("b")]);
        assert!(sole_literal_value(&multi).is_none());
    }

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
    fn meta_schema_hover_renders_every_denoted_arm_and_constraints() {
        let mut string = PropertyAtom::bare(SimplifiedType::String);
        string.constraints.push(Constraint::Required);
        let mut nested = SchemaShape::new();
        nested.properties.insert(
            "bar".to_string(),
            PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String)),
        );
        let def = PropertyDef::Union(vec![string, PropertyAtom::bare_inline_object(nested)]);

        let body = meta_schema_definition_hover_body("foo", &def);

        assert!(body.contains("Type: **type-definition**"), "{body}");
        assert!(body.contains("Declares: **string | object**"), "{body}");
        assert!(body.contains("Required"), "{body}");
    }

    #[test]
    fn meta_schema_completion_catalog_is_descriptor_driven() {
        let text = "---\n$schema:\n  title: type-definition\ntitle: \n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("title: \n").unwrap() + "title: ".len();
            let items = completion(ctx, offset);
            let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
            for descriptor in schema_type_descriptors() {
                assert!(
                    labels.contains(&descriptor.keyword),
                    "descriptor `{}` is offered: {items:#?}",
                    descriptor.keyword
                );
            }
            for scaffold in ["{}", "[]", "Name@./types.yaml"] {
                assert!(labels.contains(&scaffold), "scaffold `{scaffold}` is offered: {items:#?}");
            }
        });

        let text = "---\n$schema:\n  title: str\n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("str").unwrap() + "str".len();
            let items = completion(ctx, offset);
            let labels: Vec<&str> = items.iter().map(|item| item.label.as_str()).collect();
            assert!(labels.contains(&"string"), "type keyword completion: {items:#?}");
            assert!(labels.contains(&"string[]"), "array scaffold completion: {items:#?}");
        });

        let text = "---\n$schema:\n  title: string(re\n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("string(re").unwrap() + "string(re".len();
            let items = completion(ctx, offset);
            assert!(
                items.iter().any(|item| item.label == "required"),
                "the selected string type offers its valid required constraint: {items:#?}"
            );
            assert!(
                items.iter().all(|item| item.label != "integer"),
                "number-only constraints must not leak into string parser state: {items:#?}"
            );
        });

        let text = "---\n$schema:\n  definitions: type-definition[]\ndefinitions:\n  - sch\n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("- sch").unwrap() + "- sch".len();
            let items = completion(ctx, offset);
            assert!(
                items.iter().any(|item| item.label == "schema"),
                "an outer semantic-array item delegates to scalar definition completion: {items:#?}"
            );
        });

        let text = "---\n$schema:\n  title: type-definition\ntitle: 'str'\n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("'str").unwrap() + "'str".len();
            let items = completion(ctx, offset);
            let string = items
                .iter()
                .find(|item| item.label == "string")
                .expect("quoted scalar completion");
            let Some(CompletionTextEdit::Edit(edit)) = &string.text_edit else {
                panic!("quoted completion has eager edit");
            };
            assert_eq!(edit.new_text, "string");
            assert_eq!(edit.range.start.character, 8, "the opening quote is preserved");
        });

        let text = "---\n$schema: \n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("$schema: ").unwrap() + "$schema: ".len();
            let labels: Vec<String> = completion(ctx, offset)
                .into_iter()
                .map(|item| item.label)
                .collect();
            for scaffold in ["{}", "[]", "./schema.yaml"] {
                assert!(labels.iter().any(|label| label == scaffold), "outer `{scaffold}` scaffold: {labels:?}");
            }
        });

        let text = "---\n$schema:\n  Base: string\n  alias: Ba\n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("alias: Ba").unwrap() + "alias: Ba".len();
            let labels: Vec<String> = completion(ctx, offset)
                .into_iter()
                .map(|item| item.label)
                .collect();
            assert!(
                labels.iter().any(|label| label == "Base@this"),
                "the current parsed schema is a passive named-type namespace: {labels:?}"
            );
        });

        let text = "---\n$schema:\n  declarations: schema[]\ndeclarations:\n  - cus\n---\n\nbody\n";
        with_ctx_docs(text, &[("/w/custom.yaml", "$schema:\n  title: string\n")], |ctx| {
            let offset = text.find("- cus").unwrap() + "- cus".len();
            let items = completion(ctx, offset);
            assert!(
                items.iter().any(|item| {
                    item.label == "custom.yaml" && item.kind == Some(CompletionItemKind::FILE)
                }),
                "a schema-array item reuses passive file-path completion: {items:#?}"
            );
        });
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
    fn test_enclosing_path_by_indent_builds_full_ancestor_chain() {
        let text = "---\nstyle:\n  page:\n    ";
        // A line indented under `page:` (indent 4) has ancestors `style` →
        // `page`, outermost first.
        assert_eq!(enclosing_path_by_indent(text, text.len(), 4), vec!["style", "page"]);
        // A line under `style:` (indent 2) has just `style`.
        let under_style = "---\nstyle:\n".len();
        assert_eq!(enclosing_path_by_indent(text, under_style + 2, 2), vec!["style"]);
        // A top-level line (indent 0) has no ancestors.
        assert!(enclosing_path_by_indent(text, under_style, 0).is_empty());
    }

    #[test]
    fn test_key_path_survives_dot_and_colon_in_keys() {
        // The structural key chain never splits an authored key: `build.target`
        // is one segment, and a quoted key carrying `:` keeps its colon.
        let text = "---\n\"build.target\": x\nouter:\n  \"host: port\": y\n---\n";
        let ast = FrontmatterAst::parse(text).unwrap().ast.unwrap();
        let dotted = ast.entry_by_key_path(&["build.target"]).unwrap();
        assert_eq!(dotted.key, "build.target");
        let nested = ast.entry_by_key_path(&["outer", "host: port"]).unwrap();
        assert_eq!(ast.key_path(nested), vec!["outer", "host: port"]);
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
    fn test_base_style_schema_exposes_nested_completion_shapes() {
        let darkmatter::markdown::schemas::SimplifiedSchema::Single(root) =
            darkmatter::markdown::schemas::darkmatter_base_schema()
        else {
            panic!("base schema must be a single object shape");
        };

        let block_quote = nested_shape(&root, &["style", "block-quote"])
            .expect("block-quote must be a typed style bucket");
        for key in [
            "width",
            "max-width",
            "alignment",
            "color",
            "bg-color",
            "margin",
            "padding",
            "border",
            "emphasis",
            "word-wrap",
        ] {
            assert!(
                block_quote.properties.contains_key(key),
                "style.block-quote must complete `{key}`"
            );
        }

        let emphasis = nested_shape(&root, &["style", "block-quote", "emphasis"])
            .expect("compound emphasis must expose nested keys");
        assert!(emphasis.properties.contains_key("italic"));
        assert!(emphasis.properties.contains_key("underline"));

        let alignment = def_at_path(&root, &["style", "block-quote", "alignment"])
            .and_then(primary_atom)
            .and_then(enum_members)
            .expect("alignment must offer enum values");
        assert_eq!(alignment, ["left", "center", "right"]);
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

    // ── Suggestion completion helper tests ──

    #[test]
    fn test_flow_array_element_empty_after_comma() {
        // `[0.,` with cursor after the comma — element is empty.
        let (element, start) = flow_array_element("[0.,", 100);
        assert_eq!(element, "");
        assert_eq!(start, 104); // 100 + 3 (comma pos) + 1 = 104
    }

    #[test]
    fn test_flow_array_element_first_element() {
        // `[red` — cursor inside the first element.
        let (element, start) = flow_array_element("[red", 0);
        assert_eq!(element, "red");
        assert_eq!(start, 1); // after `[`
    }

    #[test]
    fn test_flow_array_element_after_bracket() {
        // `[` alone — cursor right after opening bracket.
        let (element, start) = flow_array_element("[", 0);
        assert_eq!(element, "");
        assert_eq!(start, 1);
    }

    #[test]
    fn test_flow_array_element_partial_after_comma() {
        // `[a, b` — cursor inside the second element.
        let (element, start) = flow_array_element("[a, b", 0);
        assert_eq!(element, "b");
        assert_eq!(start, 4); // after `, `
    }

    #[test]
    fn test_flow_array_element_with_spaces() {
        // `[a,    b` — multiple spaces after comma.
        let (element, start) = flow_array_element("[a,    b", 0);
        assert_eq!(element, "b");
        assert_eq!(start, 7); // after `,    `
    }
}
