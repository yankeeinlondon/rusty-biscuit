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

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use darkmatter::markdown::schemas::completion::{
    AlternativeKey, AlternativeSite, DefinitionLookup, alternatives_for_def_in,
};
use darkmatter::markdown::schemas::simplified::query::resolved_alternatives;
use darkmatter::markdown::schemas::{
    Constraint, JsonPointer, PropertyAtom, PropertyDef, SchemaArm, SchemaCursor,
    SchemaCursorRole, SchemaDeclaration, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr,
    DecodedScalar, decode_plain_scalar_node, decode_scalar, locate_schema_declaration_cursor,
    locate_schema_declaration_cursor_in_scalar, mapping_separator, locate_type_definition_cursor,
    locate_type_definition_cursor_in_scalar, parse_property_definition, parse_schema_declaration,
    schema_constraint_descriptors, schema_type_descriptor, schema_type_descriptors,
    flattened_alternatives, select_literal_discriminant_arm, suggestions_for_def,
};
use serde_json::Value;
use darkmatter::markdown::span::SourceSpan;
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionTextEdit, Diagnostic, DocumentLink,
    Documentation, DocumentSymbol, FoldingRange, Hover, HoverContents, Location, MarkupContent,
    MarkupKind, Range, SymbolKind, TextEdit,
};

use super::DocumentContext;
use crate::overlay::{
    FmEntry, FmEntryRole, FmPathSegment, FmValueKind, FrontmatterAst, SchemaAuthoringState,
    doc_links, expressions, format_dotted,
};
use crate::overlay::FrontmatterAnalysis;
use crate::overlay::analysis::{ArmAdmission, ExpressionTyping, ScalarTyping, baseline_root};
use crate::overlay::named_types::AuthoredDeclaration;
use crate::overlay::schema::{FrontmatterSchemaValue, MetaSchemaKind, NoDefinitions, semantic_type_regions};
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
            let root = ancestors.first().map_or(key.as_str(), String::as_str);
            let event = crate::overlay::event_views::LifecycleHost::for_document(ctx).event_of_key(root);
            if let Some(event) = &event
                && let Some(items) = stack_completions(ctx, offset, &key, &value_partial, event)
            {
                return items;
            }
            value_completions(ctx, offset, &ancestors, &key, &value_partial, event.as_ref())
        }
        None if indent == 0 => top_level_key_completions(ctx, ast, offset, trimmed),
        None => {
            if let Some(items) = union_item_completions(ctx, ast, offset, line_start, indent, trimmed) {
                return items;
            }
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
    let cursor = match flow_cursor(ctx, offset) {
        Some((value_start, ancestors, key)) => SemanticValueCursor {
            value_start,
            kinds: meta_schema_kinds_for_line(overlay, &ancestors, &key, false)?,
            continued: None,
        },
        None => block_cursor(ctx, overlay, offset)?,
    };

    let value_start = cursor.value_start;
    let value_source = ctx.text.get(value_start..)?;
    let mut items = Vec::new();
    for kind in cursor.kinds {
        let state = match (kind, &cursor.continued) {
            (MetaSchemaKind::TypeDefinition, Some(scalar)) => {
                locate_type_definition_cursor_in_scalar(ctx.text, scalar, offset)
            }
            (MetaSchemaKind::Schema, Some(scalar)) => {
                locate_schema_declaration_cursor_in_scalar(ctx.text, scalar, offset)
            }
            (MetaSchemaKind::TypeDefinition, None) => {
                locate_type_definition_cursor(value_source, value_start, offset)
            }
            (MetaSchemaKind::Schema, None) => {
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

/// The semantic value a completion cursor sits in.
struct SemanticValueCursor {
    /// Document offset where the value's authored text begins.
    value_start: usize,
    kinds: Vec<MetaSchemaKind>,
    /// The decoded plain scalar the cursor continues onto a later line, when
    /// no loaded model describes it; the cursor is read through this map
    /// rather than from the raw text at `value_start`.
    continued: Option<DecodedScalar>,
}

/// The semantic value a block-presented cursor resolves to.
///
/// Standalone documents consult the shared source map first, which covers both
/// implicit and explicit mapping pairs. Frontmatter and incomplete source then
/// fall back to an implicit `key:` value or a `- ` sequence item.
fn block_cursor(
    ctx: &DocumentContext,
    overlay: &crate::overlay::DocumentOverlay,
    offset: usize,
) -> Option<SemanticValueCursor> {
    if let SchemaAuthoringState::Standalone { model: Some(model), .. } =
        &overlay.schema_authoring
        && let Some(schema) = model.schema()
        && let Some(region) = semantic_type_regions(schema, &model.source_map)
            .into_iter()
            .find(|region| {
                region.definition_span.start <= offset && offset <= region.definition_span.end
            })
    {
        return Some(SemanticValueCursor {
            value_start: region.definition_span.start,
            kinds: vec![region.kind],
            continued: None,
        });
    }
    line_cursor(ctx, overlay, offset)
}

/// [`block_cursor`]'s line-based reading, for frontmatter and for standalone
/// source no loaded model describes.
///
/// A line that is neither a `key:` value nor a `- ` item may continue a plain
/// scalar begun on an earlier line. The cursor then belongs to that scalar
/// only when the shared plain-scalar decoder, run from the holder's value
/// start, consumes the text up to the cursor.
fn line_cursor(
    ctx: &DocumentContext,
    overlay: &crate::overlay::DocumentOverlay,
    offset: usize,
) -> Option<SemanticValueCursor> {
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
        return continued_scalar_cursor(ctx, overlay, offset, line_start, indent);
    };
    Some(SemanticValueCursor { value_start, kinds, continued: None })
}

/// The semantic value whose plain scalar the cursor's line (starting at
/// `line_start`, indented `indent`) continues.
///
/// Holder selection is a backward indentation scan, but whether the holder's
/// value is a plain scalar that reaches the cursor is decided by
/// [`decode_plain_scalar_node`] alone: same-line anchors and tags are skipped,
/// while quoted and block scalars, collections, aliases, comments, empty
/// values, and a dedented sibling do not continue.
fn continued_scalar_cursor(
    ctx: &DocumentContext,
    overlay: &crate::overlay::DocumentOverlay,
    offset: usize,
    line_start: usize,
    indent: usize,
) -> Option<SemanticValueCursor> {
    let (holder_end, parent_indent) = continued_scalar_holder(ctx.text, line_start, indent)?;
    let holder = line_cursor(ctx, overlay, holder_end)?;
    let (scalar, _) = decode_plain_scalar_node(ctx.text, holder.value_start, parent_indent)?;
    // A declaration arm reads the whole scalar as one token, so this succeeds
    // exactly when the scalar reaches the cursor (or only blanks follow it).
    locate_schema_declaration_cursor_in_scalar(ctx.text, &scalar, offset)?;
    Some(SemanticValueCursor { continued: Some(scalar), ..holder })
}

/// The content end of the line holding the scalar that the line at
/// `line_start` (indented `indent`) may continue, and the column of the block
/// collection holding that value (its key, or the `-` of a sequence item), or
/// `None` when no such line precedes it.
///
/// The holder is the nearest earlier line indented less that carries a `key:`
/// or `- ` indicator; less-indented lines without one are earlier
/// continuations of the same scalar. Blank lines fold into the scalar, while a
/// comment line ends it.
fn continued_scalar_holder(text: &str, line_start: usize, indent: usize) -> Option<(usize, usize)> {
    let mut needed = indent;
    let mut end = line_start;
    while end > 0 {
        let start = text[..end - 1].rfind('\n').map_or(0, |index| index + 1);
        let line = text[start..end - 1].trim_end_matches('\r');
        end = start;
        let trimmed = line.trim_start();
        let line_indent = line.len() - trimmed.len();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with('#') {
            return None;
        }
        if line_indent >= needed {
            continue;
        }
        let mut value = trimmed;
        let mut holder_column = None;
        if let Some(after_dash) = value.strip_prefix("- ") {
            value = after_dash.trim_start();
            holder_column = Some(line_indent);
        }
        if mapping_separator(value, false).is_some() {
            holder_column = Some(line.len() - value.len());
        }
        let Some(holder_column) = holder_column else {
            needed = line_indent;
            continue;
        };
        return Some((start + line.len(), holder_column));
    }
    None
}

/// The `(value_start, ancestors, key)` a cursor inside a flow collection
/// resolves to, or `None` in ordinary block context.
///
/// [`schema::flow_value_cursor`] answers only *within* the flow collection; the
/// block ancestry above its opening delimiter still comes from the block
/// helpers, so `types:\n  entry: {title: string}` reports the full
/// `types → entry → title` path rather than a bare `title`.
fn flow_cursor(ctx: &DocumentContext, offset: usize) -> Option<(usize, Vec<String>, String)> {
    // A body offset is never frontmatter authoring: braces in prose must not
    // reach the semantic router. A standalone schema document has no AST and is
    // authoring everywhere.
    if overlay_ast(ctx).is_some_and(|ast| !ast.contains_offset(offset)) {
        return None;
    }
    let cursor = crate::overlay::schema::flow_value_cursor(ctx.text, offset)?;
    let (line_start, prefix) = line_prefix(ctx.text, cursor.root_start);
    let indent = prefix.len() - prefix.trim_start().len();
    let mut ancestors = enclosing_path(ctx, cursor.root_start, line_start, indent);
    if let Some((key, _)) = value_cursor(ctx, cursor.root_start, line_start, prefix.trim_start()) {
        ancestors.push(key);
    }
    ancestors.extend(cursor.ancestors);
    Some((cursor.value_start, ancestors, cursor.key))
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
            // A pure standalone document's whole payload may be a scalar
            // reference (`$schema: ./other.yaml`), whose value sits on the
            // top-level `$schema` line itself — a line `enclosing_path`
            // correctly reports as having no ancestors.
            if payload == "$schema" && !sequence_item && key == "$schema" && ancestors.is_empty() {
                return Some(vec![MetaSchemaKind::Schema]);
            }
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

            let mut path: Vec<&str> = ancestors.iter().map(String::as_str).collect();
            if !sequence_item && !key.is_empty() {
                path.push(key);
            }
            semantic_owner_kinds(values, &path)
        }
        SchemaAuthoringState::Inactive => None,
    }
}

/// The meta-schema grammar activated at one structural frontmatter key path by
/// the longest recorded semantic owner covering it.
///
/// `path` is matched against each recorded pointer segment-by-segment. Both
/// sides must be decoded: a recorded pointer is RFC 6901-encoded (`/a~1b`), an
/// authored key is not, and only the whole path — not its first segment — can
/// address a nested value like `/parameter/type`.
///
/// Shared by completion and hover so both activate over exactly the same
/// region: hovering inside a `schema`-typed property must describe the same
/// definitions completion offers there.
fn semantic_owner_kinds(
    values: &[FrontmatterSchemaValue],
    path: &[&str],
) -> Option<Vec<MetaSchemaKind>> {
    if path.is_empty() {
        return None;
    }
    let (owner_len, owner) = values
        .iter()
        .filter_map(|value| {
            let pointer = JsonPointer::parse(&value.pointer);
            let segments = pointer.segments();
            (!segments.is_empty()
                && segments.len() <= path.len()
                && segments
                    .iter()
                    .zip(path)
                    .all(|(recorded, authored)| recorded.as_str() == *authored))
            .then_some((segments.len(), value))
        })
        .max_by_key(|(len, _)| *len)?;
    // A strict ancestor means the cursor sits *inside* that owner's definition,
    // where every nested entry is one complete type definition regardless of
    // the owner's own kind.
    if owner_len < path.len() {
        return Some(vec![MetaSchemaKind::TypeDefinition]);
    }
    Some(owner.kinds.clone())
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
        SchemaCursorRole::Constraint { subject, array_level, leading_group } => {
            return constraint_completions(
                ctx,
                start,
                offset,
                partial,
                subject.as_deref(),
                *array_level,
                leading_group.as_deref(),
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
        for alias in descriptor.aliases {
            if alias.starts_with(partial)
                && let Some(completion) = item(
                    ctx,
                    start,
                    offset,
                    alias,
                    alias,
                    CompletionItemKind::TYPE_PARAMETER,
                    Some(format!("Alias of `{}`: {}", descriptor.keyword, descriptor.description)),
                )
            {
                items.push(completion);
            }
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
    let local = passive_namespace_names(ctx).into_iter().map(|name| {
        (
            named_references::local_reference_spelling(&name),
            "Named type from this schema".to_string(),
        )
    });
    let external = named_references::external_names(ctx)
        .into_iter()
        .map(|(label, target)| (label, format!("Named type from `{target}`")));
    for (label, detail) in local.chain(external) {
        if label.starts_with(partial)
            && let Some(completion) = item(
                ctx,
                start,
                offset,
                &label,
                &label,
                CompletionItemKind::REFERENCE,
                Some(detail),
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
///
/// A type with options (`alphanumeric`) offers them only in the item-level
/// list's leading group, before its first `;`: every option not yet written
/// there while the group holds only options, and constraints too while it is
/// still empty. After `;` it offers constraints only.
fn constraint_completions(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    partial: &str,
    subject: Option<&str>,
    array_level: bool,
    leading_group: Option<&[String]>,
) -> Vec<CompletionItem> {
    let item_descriptor = if array_level {
        None
    } else {
        let Some(descriptor) = subject.and_then(schema_type_descriptor) else {
            return Vec::new();
        };
        Some(descriptor)
    };
    let mut items = Vec::new();
    let mut offer_constraints = true;
    if let (Some(descriptor), Some(group)) = (item_descriptor, leading_group)
        && !descriptor.type_options.is_empty()
    {
        let is_option = |entry: &String| {
            descriptor.type_options.iter().any(|option| option.keyword() == entry)
        };
        if group.iter().all(is_option) {
            offer_constraints = group.is_empty();
            items.extend(
                descriptor
                    .type_options
                    .iter()
                    .filter(|option| !group.iter().any(|entry| entry == option.keyword()))
                    .filter(|option| option.keyword().starts_with(partial))
                    .filter_map(|option| {
                        item(
                            ctx,
                            start,
                            offset,
                            option.keyword(),
                            option.keyword(),
                            CompletionItemKind::ENUM_MEMBER,
                            Some(format!("`{}` option: {}", descriptor.keyword, option.effect())),
                        )
                    }),
            );
        }
    }
    if !offer_constraints {
        return items;
    }
    items.extend(schema_constraint_descriptors()
        .iter()
        .filter(|constraint| !constraint.keyword.starts_with('<'))
        .filter(|constraint| match item_descriptor {
            Some(descriptor) => descriptor.accepts_constraint(constraint.keyword),
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
        }));
    items
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
            if entry.depth == 1
                && entry.role == FmEntryRole::MappingProperty
                && entry.pointer.starts_with("/$schema/")
            {
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
    shape_key_completions(ctx, offset, shape, &present, partial)
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
    let segments = key_segments(&ancestor_refs);
    let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Transparent);
    if let Some(nested) = memo.shape(&segments) {
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
    event: Option<&crate::overlay::event_views::StackEvent>,
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

    let segments = key_segments(&path);
    let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Transparent);
    let Some(def) = memo.def_at(&segments) else {
        return Vec::new();
    };

    let mut items = Vec::new();

    // Each authored `literal(x)` value, preselected, with YAML insertion text
    // matching the value's scalar type. A union-typed reference contributes
    // its arms, and a recursive reference its definition's.
    let alternatives = resolved_alternatives(def, definitions(ctx));
    for atom in alternatives.iter().map(|alternative| alternative.atom) {
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
        items.extend(expression_value_completions(ctx, offset, partial, start, event));
    }

    // Scaffolds from EVERY non-Literal arm (enum members, boolish `true`/`false`,
    // file paths), accumulated with the items above rather than stopping at the
    // first completable arm — otherwise a mixed union would silently drop its
    // later arms. Alternatives are in declaration order, so a preselected
    // literal precedes any colliding scaffold and survives `dedup_completions`
    // below.
    for atom in alternatives.iter().map(|alternative| alternative.atom) {
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
    for record in ctx.graph.documents().iter().filter(|record| !record.dependency) {
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
    event: Option<&crate::overlay::event_views::StackEvent>,
) -> Vec<CompletionItem> {
    let (token_start, token) = expressions::value_completion_partial(value_partial, value_start);
    expression_candidates(ctx, token_start, offset, token, event)
}

/// The shared expression candidates for `token` (typed over
/// `token_start..offset`): frontmatter keys, `ctx.*`, functions, host
/// globals, and, inside `event`'s stack, the globals that event may read.
fn expression_candidates(
    ctx: &DocumentContext,
    token_start: usize,
    offset: usize,
    token: &str,
    event: Option<&crate::overlay::event_views::StackEvent>,
) -> Vec<CompletionItem> {
    let frontmatter_keys = expression_property_keys(ctx);
    let host = expressions::host_global_candidates(token, &crate::providers::dsl::host_catalogs(ctx));
    let lifecycle = event.map(|event| expressions::event_global_candidates(token, event)).unwrap_or_default();
    expressions::completion_candidates(token, &frontmatter_keys)
        .into_iter()
        .chain(lifecycle)
        .chain(host)
        .filter_map(|candidate| expr_completion_item(ctx, token_start, offset, candidate))
        .collect()
}

/// Completion inside `event`'s lifecycle stack value: inside an open `{{`
/// span, or in a `when` guard's condition text, the shared expression
/// candidates with the event's globals. `None` elsewhere in the stack, which
/// completes like any other value.
fn stack_completions(
    ctx: &DocumentContext,
    offset: usize,
    key: &str,
    value_partial: &str,
    event: &crate::overlay::event_views::StackEvent,
) -> Option<Vec<CompletionItem>> {
    let value_start = offset - value_partial.len();
    if let Some((token_start, token)) = expressions::completion_partial(value_partial, value_partial.len()) {
        return Some(expression_candidates(ctx, value_start + token_start, offset, token, Some(event)));
    }
    (key == "when").then(|| expression_value_completions(ctx, offset, value_partial, value_start, Some(event)))
}

/// The document properties an expression may name: the document's top-level
/// frontmatter keys, then the top-level properties its own `$schema` declares
/// but leaves unset (caller-supplied parameters). Base and extension baseline
/// properties are not offered.
pub(crate) fn expression_property_keys(ctx: &DocumentContext) -> Vec<String> {
    let mut keys: Vec<String> = ctx
        .overlay
        .and_then(|overlay| overlay.ast.as_ref())
        .map(|ast| ast.top_level().map(|entry| entry.key.clone()).collect())
        .unwrap_or_default();
    let bundle = ctx.overlay.and_then(|overlay| overlay.bundle());
    let schema = bundle.and_then(|bundle| bundle.effective.simplified.as_ref());
    let declared: Vec<String> = match schema {
        Some(SimplifiedSchema::Single(document)) => document.properties.keys().cloned().collect(),
        Some(SimplifiedSchema::Union(arms)) => merged_root_arm_shape(arms).properties.into_keys().collect(),
        None => Vec::new(),
    };
    for name in declared {
        if !keys.contains(&name) {
            keys.push(name);
        }
    }
    keys
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
        expressions::ExprCompletionKind::ReservedRoot => CompletionItemKind::MODULE,
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
/// parser-decoded expression text and its scalar-style projection back to
/// authored document bytes (YAML quotes excluded).
///
/// The decoded text and projection are borrowed from the snapshot's scalar
/// typing ([`ScalarTyping`]), so every request on one snapshot reads the same
/// stored values.
pub(crate) struct ExpressionValue<'a> {
    /// The authored entry — `value_span` is the whole authored value range.
    pub(crate) entry: &'a FmEntry,
    /// The entry's arena index, for O(depth) [`FrontmatterAst::path_at`].
    pub(crate) index: usize,
    /// The decoded expression text the compose parser sees.
    expression: &'a str,
    projection: &'a expressions::ScalarProjection,
}

impl ExpressionValue<'_> {
    /// The decoded expression text the compose parser sees.
    pub(crate) fn expression(&self) -> &str {
        self.expression
    }

    /// Whether ranges inside the value are exact: an untagged single-line
    /// plain or quoted scalar. Block, tagged, and multi-line values have no
    /// decoded-to-authored map.
    pub(crate) fn is_exact(&self) -> bool {
        self.projection.is_exact()
    }

    /// The scalar-style projection, for lints that encode an authored rewrite.
    pub(crate) fn projection(&self) -> &expressions::ScalarProjection {
        self.projection
    }

    /// Projects a decoded byte range to a document span, YAML quotes excluded.
    pub(crate) fn project(&self, range: std::ops::Range<usize>) -> Option<SourceSpan> {
        self.projection.project(range)
    }

    /// The whole decoded-expression authored span (quotes excluded), falling
    /// back to the whole value node when projection is impossible.
    pub(crate) fn expression_span(&self) -> SourceSpan {
        self.projection.range(0..self.expression.len())
    }

    /// The decoded byte offset for a document cursor inside the value, when
    /// the value is exactly projected.
    fn decoded_offset(&self, doc_offset: usize) -> Option<usize> {
        self.projection.analyzed_offset(doc_offset)
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

/// What the schema says about the scalar entry `index` of `ast`, resolved
/// through `memo` (explicit array crossing, the same resolver every
/// value-oriented capability uses). The one classifier behind both
/// [`scalar_typing`] and [`scalar_inventory`].
fn classify_scalar<'p>(
    memo: &mut ShapeMemo<'_, 'p>,
    ast: &'p FrontmatterAst,
    document: &str,
    index: usize,
) -> ScalarTyping {
    let path = ast.path_at(index);
    let Some(def) = memo.def_at(&path) else {
        return ScalarTyping::UNRESOLVED;
    };
    let expression = if expression_atom(def).is_none() {
        ExpressionTyping::No
    } else {
        match expressions::ScalarProjection::for_value(&ast.entries()[index], document) {
            Some((projection, text)) => ExpressionTyping::Decoded { text, projection },
            None => ExpressionTyping::Undecodable,
        }
    };
    ScalarTyping { file: file_atom(def).is_some(), expression }
}

/// The typing of the one scalar entry `index`, from the snapshot when any
/// request has classified it, else classified now and stored. Resolves only
/// that entry's path and never starts the whole-document inventory, so a
/// cursor request costs the same however many other scalars the document has.
/// `None` without a frontmatter tree.
pub(crate) fn scalar_typing<'c>(ctx: &DocumentContext<'c>, index: usize) -> Option<&'c ScalarTyping> {
    let overlay = ctx.overlay?;
    let ast = overlay.ast.as_deref()?;
    Some(overlay.analysis.scalar(index, || {
        classify_scalar(&mut ShapeMemo::new(ctx, ArrayCrossing::Explicit), ast, ctx.text, index)
    }))
}

/// Every scalar entry with its typing, in document order: the snapshot's
/// inventory, built by the first whole-document request and read by every
/// later one. Empty without a frontmatter tree.
fn scalar_inventory<'c>(ctx: &DocumentContext<'c>) -> impl Iterator<Item = (usize, &'c ScalarTyping)> + use<'c> {
    ctx.overlay
        .and_then(|overlay| {
            let ast = overlay.ast.as_deref()?;
            let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
            Some(overlay.analysis.inventory(&mut |index| classify_scalar(&mut memo, ast, ctx.text, index)))
        })
        .into_iter()
        .flatten()
}

/// The scalar entry whose authored value span contains `offset`.
fn cursor_scalar(ast: &FrontmatterAst, offset: usize) -> Option<usize> {
    ast.entries()
        .iter()
        .position(|entry| entry.kind == FmValueKind::Scalar && entry.value_span.contains(&offset))
}

/// The Expression-typed value at entry `index`, when `typing` decoded one.
fn expression_value<'a>(ast: &'a FrontmatterAst, index: usize, typing: &'a ScalarTyping) -> Option<ExpressionValue<'a>> {
    let ExpressionTyping::Decoded { text, projection } = &typing.expression else {
        return None;
    };
    Some(ExpressionValue { entry: &ast.entries()[index], index, expression: text, projection })
}

/// Every Expression-typed **scalar** frontmatter value, in document order. A
/// mapping/sequence value on an expression property is intentionally excluded —
/// it stays a schema type mismatch, not an expression.
///
/// Reads the snapshot's scalar inventory, initializing it on first use; a
/// cursor request uses [`scalar_typing`] instead.
pub(crate) fn expression_values<'c>(ctx: &DocumentContext<'c>) -> Vec<ExpressionValue<'c>> {
    let Some(ast) = ctx.overlay.and_then(|overlay| overlay.ast.as_deref()) else {
        return Vec::new();
    };
    scalar_inventory(ctx).filter_map(|(index, typing)| expression_value(ast, index, typing)).collect()
}

/// Hover on an Expression-typed frontmatter value, via the shared
/// [`expressions::hover_markdown_condition`] authority (condition dialect, so
/// `&&`/`||` values hover like any other expression). Fires only when the cursor
/// is inside the value (a cursor on the key falls through to the schema hover,
/// which describes the `expression` type itself). The value's own ternary and
/// `&&`/`||` facts narrow it; no enclosing lifecycle or task `when` does.
/// Inside a lifecycle stack, the event's globals shadow frontmatter keys
/// ([`expressions::hover_markdown_in_event`]).
fn expression_hover(ctx: &DocumentContext, ast: &FrontmatterAst, value: &ExpressionValue<'_>, offset: usize) -> Option<Hover> {
    let expr_offset = value.decoded_offset(offset)?;
    let host = crate::providers::dsl::host_catalogs(ctx);
    let event = crate::overlay::event_views::LifecycleHost::for_document(ctx).stack_event(&ast.path_at(value.index));
    let markdown = crate::providers::dsl::with_site_scope(ctx, None, |site| {
        let scalar = |name: &str| frontmatter_scalar(ctx, name);
        let property =
            |name: &str| expressions::host_global_hover(name, &host).or_else(|| schema_property_details(ctx, name));
        match event {
            Some(event) => expressions::hover_markdown_in_event(
                expressions::parse_condition,
                value.expression(),
                expr_offset,
                site,
                scalar,
                property,
                &event,
            ),
            None => expressions::hover_markdown_condition(value.expression(), expr_offset, site, scalar, property),
        }
    });
    markup_hover(ctx, value.expression_span(), markdown)
}

/// Hover on a `{{ … }}` span or `when` guard inside the lifecycle stack
/// scalar `index`, which is not an Expression-typed value: the shared
/// expression hover, with the globals of the stack's event
/// ([`expressions::hover_markdown_in_event`]). Unguarded, like every
/// frontmatter site. Only that scalar's expressions are extracted.
fn stack_expression_hover(ctx: &DocumentContext, ast: &FrontmatterAst, index: usize, offset: usize) -> Option<Hover> {
    let lifecycle = crate::overlay::event_views::LifecycleHost::for_document(ctx);
    let expression = crate::overlay::event_views::scalar_stack_expressions(&lifecycle, ctx.text, ast, index)
        .into_iter()
        .find(|expression| expression.reach().contains(&offset))?;
    let expr_offset = expression.text_offset(offset).unwrap_or(0).min(expression.text.len());
    let host = crate::providers::dsl::host_catalogs(ctx);
    let markdown = crate::providers::dsl::with_site_scope(ctx, None, |site| {
        expressions::hover_markdown_in_event(
            expression.dialect.parser(),
            &expression.text,
            expr_offset,
            site,
            |name| frontmatter_scalar(ctx, name),
            |name| expressions::host_global_hover(name, &host).or_else(|| schema_property_details(ctx, name)),
            &expression.event,
        )
    });
    markup_hover(ctx, expression.reach().clone(), markdown)
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
    let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
    let def = memo.def_at(&[FmPathSegment::Key(name)])?;
    schema_hover_details(def, definitions(ctx))
}

// ── Suggestion completion ──────────────────────────────────────────────────

/// Builds suggestion completion items for a property path, filtered by `prefix`.
/// Returns `None` when the property has no `suggest(...)` constraint or the path
/// does not resolve.
///
/// The property definition is resolved through the same selected-arm-aware
/// [`ShapeMemo::def_at`] every other value capability uses, then handed to the
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
    let segments = key_segments(property_path);
    let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Transparent);
    let def = memo.def_at(&segments)?;
    let query = suggestions_for_def(leaf, def)?;
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

/// Completion for one block-sequence item of an array whose items are a union
/// (`U[]`, or a property union of several array types) or an object (a
/// recursive `item[]` among them), from the library's
/// [`alternatives_for_def_in`].
///
/// On a `- ` line the item is still empty, so the keys of every object
/// alternative and the values of every scalar alternative are offered
/// together; an array of one object type offers that object's keys. On a
/// later line of a union item, the item's authored keys are excluded and an
/// unambiguous literal tag in the item narrows the keys to its alternative;
/// anything less keeps the combined keys. `None` hands the cursor to the
/// ordinary item and nested-key completion.
fn union_item_completions(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    offset: usize,
    line_start: usize,
    indent: usize,
    trimmed: &str,
) -> Option<Vec<CompletionItem>> {
    let (marker_len, partial) = match trimmed.strip_prefix("- ") {
        Some(after_dash) => (2, after_dash),
        None if trimmed == "-" => (1, ""),
        None => return object_item_key_completions(ctx, ast, offset, line_start, indent, trimmed),
    };
    let ancestors = enclosing_path(ctx, offset, line_start, indent);
    let path: Vec<&str> = ancestors.iter().map(String::as_str).collect();
    let segments = key_segments(&path);
    let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Transparent);
    let def = memo.def_at(&segments)?;
    if !has_union_items(def, definitions(ctx)) && !has_object_items(def, definitions(ctx)) {
        return None;
    }
    let completion = alternatives_for_def_in(def, AlternativeSite::Item, &Value::Null, definitions(ctx))?;
    let start = line_start + indent + marker_len;
    let mut items: Vec<CompletionItem> = completion
        .values
        .iter()
        .filter(|value| value.label.starts_with(partial) || value.insert_text.starts_with(partial))
        .filter_map(|value| {
            item(ctx, start, offset, &value.label, &value.insert_text, CompletionItemKind::VALUE, None)
        })
        .collect();
    items.extend(alternative_key_items(ctx, start, offset, &completion.keys, &[], partial));
    Some(dedup_completions(items))
}

/// Key completion on a continuation line of a block-sequence object item whose
/// array holds a union.
fn object_item_key_completions(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    offset: usize,
    line_start: usize,
    indent: usize,
    partial: &str,
) -> Option<Vec<CompletionItem>> {
    let item_index = enclosing_object_item(ctx, ast, line_start, indent)?;
    let item_path = ast.path_at(item_index);
    let (FmPathSegment::Index(_), array_path) = item_path.split_last()? else {
        return None;
    };
    let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
    let def = memo.def_at(array_path)?;
    if !has_union_items(def, definitions(ctx)) {
        return None;
    }
    let value = ctx
        .overlay
        .and_then(|overlay| overlay.bundle())
        .and_then(|bundle| navigate_json(&bundle.frontmatter_json, &item_path).cloned())
        .unwrap_or(Value::Null);
    let completion = alternatives_for_def_in(def, AlternativeSite::Item, &value, definitions(ctx))?;
    let present: Vec<&str> = ast
        .entries()
        .iter()
        .filter(|entry| entry.parent == Some(item_index))
        .map(|entry| entry.key.as_str())
        .collect();
    let start = offset - partial.len();
    Some(alternative_key_items(ctx, start, offset, &completion.keys, &present, partial))
}

/// The block-sequence object item a continuation line at column `indent`
/// belongs to: the last item mapping starting before the line at that column,
/// with no shallower line in between.
fn enclosing_object_item(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    line_start: usize,
    indent: usize,
) -> Option<usize> {
    let (index, entry) = ast
        .entries()
        .iter()
        .enumerate()
        .filter(|(_, entry)| {
            matches!(entry.role, FmEntryRole::SequenceItem { .. })
                && entry.kind == FmValueKind::Mapping
                && entry.value_span.start < line_start
                && source_column(ctx.text, entry.value_span.start) == indent
        })
        .max_by_key(|(_, entry)| entry.value_span.start)?;
    let between = ctx.text.get(entry.value_span.start..line_start)?;
    let leaves_item = between.lines().skip(1).any(|line| {
        let content = line.trim_start();
        !content.is_empty() && line.len() - content.len() < indent
    });
    (!leaves_item && still_placed(ctx, entry)).then_some(index)
}

/// Whether `def` is an array whose item has more than one alternative, a
/// recursive item reference counting its definition's alternatives.
fn has_union_items(def: &PropertyDef, definitions: &dyn DefinitionLookup) -> bool {
    resolved_alternatives(def, definitions).iter().filter(|alternative| alternative.is_array).count() > 1
}

/// Whether `def` is an array with an inline-object item alternative, a
/// recursive item reference counting its definition's alternatives.
fn has_object_items(def: &PropertyDef, definitions: &dyn DefinitionLookup) -> bool {
    resolved_alternatives(def, definitions)
        .iter()
        .any(|alternative| alternative.is_array && matches!(alternative.atom.ty, TypeExpr::InlineObject(_)))
}

/// `FIELD` completions for alternative keys not yet authored, filtered by
/// `partial` and marked `(required)` when every alternative offering the key
/// requires it.
fn alternative_key_items(
    ctx: &DocumentContext,
    start: usize,
    offset: usize,
    keys: &[AlternativeKey],
    present: &[&str],
    partial: &str,
) -> Vec<CompletionItem> {
    keys.iter()
        .filter(|key| key.name.starts_with(partial) && !present.contains(&key.name.as_str()))
        .filter_map(|key| {
            let detail = key.required.then(|| "(required)".to_string());
            item(
                ctx,
                start,
                offset,
                &key.name,
                &format!("{}: ", key.name),
                CompletionItemKind::FIELD,
                detail,
            )
        })
        .collect()
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
    // through to the schema hover describing the `expression` type. Any other
    // scalar inside a lifecycle stack gets the hover of its own `{{ … }}` span
    // or `when` guard. Only the cursor's scalar is typed.
    if let Some(index) = cursor_scalar(ast, offset)
        && let Some(typing) = scalar_typing(ctx, index)
    {
        let hover = match expression_value(ast, index, typing) {
            Some(value) => expression_hover(ctx, ast, &value, offset),
            None => stack_expression_hover(ctx, ast, index, offset),
        };
        if hover.is_some() {
            return hover;
        }
    }

    let entry = ast.entry_at_offset(offset)?;
    let on_key = entry.key_span.as_ref().is_some_and(|key| key.contains(&offset));
    // A cursor on a sequence's `- ` markers, or on an item that is itself a
    // collection, is on a synthetic item position with no authored key or
    // scalar to describe.
    let synthetic = match entry.role {
        FmEntryRole::SequenceItem { .. } => entry.kind != FmValueKind::Scalar,
        FmEntryRole::MappingProperty => entry.kind == FmValueKind::Sequence && !on_key,
    };
    if synthetic {
        return None;
    }
    let path = ast.path_of(entry);

    if path.first() == Some(&FmPathSegment::Key(expressions::context_root().name))
        && entry.key_span.is_some()
    {
        return ctx_hover(ctx, entry);
    }
    schema_hover(ctx, &path, entry)
}

fn meta_schema_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let overlay = ctx.overlay?;

    if let Some(hover) = named_references::hover(ctx, offset) {
        return Some(hover);
    }

    if let Some(hover) = frontmatter_definition_hover(ctx, overlay, offset) {
        return Some(hover);
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

    let parsed = AuthoredDeclaration::frontmatter_at(ctx.text, overlay.ast.as_deref()?, offset)?;
    let SchemaDeclaration::Schema(schema) = parsed.declaration else {
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

/// Hover for one complete type definition authored in Markdown frontmatter,
/// routed through the same semantic-owner activation completion uses.
///
/// Entries under the reserved `$schema` key are deliberately left to the
/// declaration reparse in [`meta_schema_hover`]: that path also covers pattern
/// keys and cursors that fall inside a definition the frontmatter AST records
/// no entry for.
fn frontmatter_definition_hover(
    ctx: &DocumentContext,
    overlay: &crate::overlay::DocumentOverlay,
    offset: usize,
) -> Option<Hover> {
    let SchemaAuthoringState::Frontmatter(values) = &overlay.schema_authoring else {
        return None;
    };
    let entry = overlay.ast.as_deref()?.entry_at_offset(offset)?;
    let pointer = JsonPointer::parse(&entry.pointer);
    let path: Vec<&str> = pointer.segments().iter().map(String::as_str).collect();
    if path.first() == Some(&"$schema") {
        return None;
    }
    if !semantic_owner_kinds(values, &path)?.contains(&MetaSchemaKind::TypeDefinition) {
        return None;
    }
    let source = ctx.text.get(entry.value_span.clone())?;
    let source = dedent_spanned_yaml(source, source_column(ctx.text, entry.value_span.start));
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(&source).ok()?;
    let definition = parse_property_definition(&entry.key, &yaml).ok()?;
    markup_hover(
        ctx,
        entry.key_span.clone()?,
        meta_schema_definition_hover_body(&entry.key, &definition),
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
            TypeExpr::Imported { name, reference } => {
                named_references::reference_spelling(name, reference, atom.is_array)
            }
            TypeExpr::Union(_) => "union".to_string(),
            TypeExpr::Tuple(_) => "tuple".to_string(),
            TypeExpr::Function(_) => "function".to_string(),
            TypeExpr::Ref(key) => key.name.clone(),
        };
        if atom.is_array && !matches!(atom.ty, TypeExpr::Imported { .. }) {
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
    lines.extend(eager_hover_line(def));
    let mut described = Vec::new();
    for atom in atoms_of(def) {
        if let TypeExpr::Primitive(ty) = &atom.ty
            && !described.contains(ty)
        {
            described.push(*ty);
            lines.extend(type_keyword_hover_line(*ty));
        }
        lines.extend(alphanumeric_hover_lines(atom));
    }
    lines.join("\n\n")
}

/// The catalog description of a built-in type, naming its aliases, so hover
/// on `any` shows the canonical `unknown`.
fn type_keyword_hover_line(ty: SimplifiedType) -> Option<String> {
    let descriptor = schema_type_descriptor(ty.as_keyword())?;
    let aliases = match descriptor.aliases {
        [] => String::new(),
        aliases => format!(
            " (alias {})",
            aliases.iter().map(|alias| format!("`{alias}`")).collect::<Vec<_>>().join(", ")
        ),
    };
    Some(format!("**`{}`**{aliases}: {}", descriptor.keyword, descriptor.description))
}

/// The effective character rules of an `alphanumeric` atom, from the
/// library's canonical options; empty for every other atom.
fn alphanumeric_hover_lines(atom: &PropertyAtom) -> Vec<String> {
    let Some(options) = atom.alphanumeric_options() else {
        return Vec::new();
    };
    let keywords: Vec<String> = options.keywords().map(|keyword| format!("`{keyword}`")).collect();
    let double_period = if options.forbids_double_period() { "forbidden" } else { "allowed" };
    vec![
        format!(
            "Options: {}",
            if keywords.is_empty() { "none".to_string() } else { keywords.join(", ") }
        ),
        format!("Characters: `{}`", options.allowed_characters()),
        format!("First character: `{}`", options.first_characters()),
        format!("`..`: {double_period}"),
    ]
}

/// Hover content for a schema-declared property, at any nesting depth.
///
/// A scalar sequence item hovers as its item definition, labeled with its
/// dotted path and ranged on the item value it describes.
///
/// Inside an item whose type is a union, the hover names an alternative only
/// when the authored item is valid against exactly that one; a missing,
/// invalid, or multiply matching item lists every alternative instead. A
/// field of an object item resolves through the matching alternative when
/// there is one, and otherwise through the usual literal-tag selection or the
/// merged object alternatives.
fn schema_hover(ctx: &DocumentContext, path: &[FmPathSegment<'_>], entry: &FmEntry) -> Option<Hover> {
    let mut memo = ShapeMemo::new(ctx, ArrayCrossing::Explicit);
    let item = union_item(ctx, path, &mut memo);
    let (span, label) = match &entry.key_span {
        Some(key_span) => (key_span.clone(), entry.key.as_str()),
        None => (entry.value_span.clone(), entry.dotted.as_str()),
    };
    let Some(item) = item else {
        let def = memo.def_at(path)?;
        return markup_hover(ctx, span, schema_hover_body(label, def, definitions(ctx))?);
    };
    let field = &path[item.depth..];
    if field.is_empty() {
        return markup_hover(ctx, span, union_item_hover_body(label, &item, definitions(ctx)));
    }
    let matched_shape = item.matched.and_then(|index| match &item.alternatives[index].ty {
        TypeExpr::InlineObject(arm) => Some(arm),
        _ => None,
    });
    // The arm-local lookup descends only the first object arm below the item,
    // so a key that only a later overload declares (`content` of
    // `ensure_file`'s long forms) falls back to the context-aware resolver,
    // which merges the arms.
    let def = match matched_shape.and_then(|arm| def_at_path(arm, field, definitions(ctx))) {
        Some(def) => def,
        None => memo.def_at(path)?,
    };
    let body = schema_hover_body(label, def, definitions(ctx))?;
    let note = union_item_note(&format_dotted(&path[..item.depth]), &item, definitions(ctx));
    markup_hover(ctx, span, format!("{body}\n\n{note}"))
}

/// The nearest sequence item enclosing a hovered path, when that item's type
/// is a union, with the alternative its authored value matches.
struct UnionItem<'c> {
    /// The item's flattened alternatives, in declaration order.
    alternatives: &'c [PropertyAtom],
    /// How the authored item relates to the alternatives.
    status: ItemMatch,
    /// The alternative `status` names when it is [`ItemMatch::Unique`].
    matched: Option<usize>,
    /// The length of the path prefix that addresses the item.
    depth: usize,
}

/// How an authored union item relates to its alternatives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ItemMatch {
    /// The item is absent or `null`.
    Missing,
    /// No alternative accepts the item.
    None,
    /// Exactly one alternative accepts the item.
    Unique,
    /// This many alternatives accept the item.
    Several(usize),
}

fn union_item<'c, 'p>(
    ctx: &DocumentContext<'c>,
    path: &[FmPathSegment<'p>],
    memo: &mut ShapeMemo<'c, 'p>,
) -> Option<UnionItem<'c>> {
    let index = path.iter().rposition(|segment| matches!(segment, FmPathSegment::Index(_)))?;
    let item_path = &path[..=index];
    let PropertyDef::Union(alternatives) = memo.def_at(item_path)? else {
        return None;
    };
    let value = ctx
        .overlay
        .and_then(|overlay| overlay.bundle())
        .and_then(|bundle| navigate_json(&bundle.frontmatter_json, item_path).cloned())
        .unwrap_or(Value::Null);
    let (status, matched) = if value.is_null() {
        (ItemMatch::Missing, None)
    } else {
        let accepting: Vec<usize> = alternatives
            .iter()
            .enumerate()
            .filter(|(_, alternative)| {
                ctx.overlay
                    .is_some_and(|overlay| overlay.analysis.shared().accepts(alternative, definitions(ctx), &value))
            })
            .map(|(index, _)| index)
            .collect();
        match accepting.as_slice() {
            [] => (ItemMatch::None, None),
            [only] => (ItemMatch::Unique, Some(*only)),
            several => (ItemMatch::Several(several.len()), None),
        }
    };
    Some(UnionItem { alternatives, status, matched, depth: index + 1 })
}

/// Hover body for a union item hovered directly (a scalar item).
fn union_item_hover_body(label: &str, item: &UnionItem, definitions: &dyn DefinitionLookup) -> String {
    match item.matched {
        Some(index) => {
            let arm = PropertyDef::Single(item.alternatives[index].clone());
            let details = schema_hover_details(&arm, definitions).unwrap_or_default();
            format!(
                "**`{label}`**\n\nMatches alternative {} of {}\n\n{details}",
                index + 1,
                item.alternatives.len()
            )
        }
        None => format!("**`{label}`**\n\n{}", union_item_alternatives(item, definitions)),
    }
}

/// The note under a field's hover naming the alternative its item matches,
/// or listing every alternative when no single one does.
fn union_item_note(item_label: &str, item: &UnionItem, definitions: &dyn DefinitionLookup) -> String {
    match item.matched {
        Some(index) => format!(
            "Item `{item_label}` matches alternative {} of {}:\n{}",
            index + 1,
            item.alternatives.len(),
            alternative_line(&item.alternatives[index], definitions)
        ),
        None => format!("Item `{item_label}`: {}", union_item_alternatives(item, definitions)),
    }
}

/// Why no single alternative was chosen, followed by every alternative.
fn union_item_alternatives(item: &UnionItem, definitions: &dyn DefinitionLookup) -> String {
    let count = item.alternatives.len();
    let reason = match item.status {
        ItemMatch::Missing => format!("no value; one of {count} alternatives:"),
        ItemMatch::None => format!("matches none of the {count} alternatives:"),
        ItemMatch::Several(matching) => format!("matches {matching} of the {count} alternatives:"),
        ItemMatch::Unique => format!("one of {count} alternatives:"),
    };
    let refs: Vec<&PropertyAtom> = item.alternatives.iter().collect();
    format!("{reason}\n{}", alternative_lines(&refs, definitions))
}

/// One Markdown list line per alternative.
fn alternative_lines(alternatives: &[&PropertyAtom], definitions: &dyn DefinitionLookup) -> String {
    alternatives
        .iter()
        .map(|alternative| alternative_line(alternative, definitions))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A list line naming an alternative's type (an object's keys) and its
/// description. A recursive reference keeps its name and lists the keys of
/// the object its definition declares.
fn alternative_line(alternative: &PropertyAtom, definitions: &dyn DefinitionLookup) -> String {
    let mut line = match &alternative.ty {
        TypeExpr::Primitive(ty) => format!("- **{}", ty.as_keyword()),
        TypeExpr::InlineObject(_) => "- **object".to_string(),
        TypeExpr::Imported { name, reference } => format!(
            "- **{}",
            named_references::reference_spelling(name, reference, alternative.is_array)
        ),
        // Callers pass flattened alternatives.
        TypeExpr::Union(_) => "- **union".to_string(),
        TypeExpr::Tuple(_) => "- **tuple".to_string(),
        TypeExpr::Function(_) => "- **function".to_string(),
        TypeExpr::Ref(key) => format!("- **{}", key.name),
    };
    if alternative.is_array && !matches!(alternative.ty, TypeExpr::Imported { .. }) {
        line.push_str("[]");
    }
    line.push_str("**");
    let object = match &alternative.ty {
        TypeExpr::InlineObject(shape) => Some(shape),
        TypeExpr::Ref(key) => definitions.lookup_definition(key).and_then(|definition| match &definition.body {
            PropertyDef::Single(PropertyAtom { ty: TypeExpr::InlineObject(shape), is_array: false, .. }) => Some(shape),
            _ => None,
        }),
        _ => None,
    };
    if let Some(shape) = object {
        let keys: Vec<String> = shape.properties.keys().map(|key| format!("`{key}`")).collect();
        if !keys.is_empty() {
            line.push_str(&format!(" with {}", keys.join(", ")));
        }
    }
    if let Some(value) = alternative.literal_value() {
        line.push_str(&format!(" {}", hover_italic(&value.to_string())));
    }
    if let Some(description) = &alternative.description {
        line.push_str(&format!(" — {description}"));
    }
    line
}

/// The Markdown body for a schema-declared property's hover.
///
/// Style rule — bounded by what LSP-Markdown can express (color and dim are the
/// editor theme's decision, not ours; see `docs/hover.md`): inline-code box =
/// the property being described; **bold** = its type; _italic_ = its enum and
/// default values. Kept pure (no `DocumentContext`) so the formatting rule is
/// unit-testable without an LSP session.
pub(crate) fn schema_hover_body(key: &str, def: &PropertyDef, definitions: &dyn DefinitionLookup) -> Option<String> {
    let details = schema_hover_details(def, definitions)?;
    Some(format!("**`{key}`**\n\n{details}"))
}

/// The schema-hover body **without** the leading `**`key`**` heading: type,
/// required, eager, enum, default, and description. A union-typed reference
/// (`U[]`, `U(required)`) is described by its flattened alternatives, each
/// with its own description, in place of a single type. A recursive reference
/// (`item[]`) is named as its type and followed by its definition's body
/// looked up in `definitions`, in which every recursive use is again written
/// by name, so the whole type shows once and never expands further. Callers
/// that already display the property name in their own header (e.g. the
/// interpolation hover's `**Expression**` block) use this to avoid repeating
/// the name.
pub(crate) fn schema_hover_details(def: &PropertyDef, definitions: &dyn DefinitionLookup) -> Option<String> {
    let atom = primary_atom(def)?;
    let mut lines = Vec::new();

    let type_line = match &atom.ty {
        TypeExpr::Primitive(ty) => {
            let suffix = if atom.is_array { "[]" } else { "" };
            format!("Type: **{}{}**", ty.as_keyword(), suffix)
        }
        TypeExpr::InlineObject(_) => "Type: **object**".to_string(),
        TypeExpr::Tuple(_) | TypeExpr::Function(_) | TypeExpr::Ref(_) => {
            let spelled = darkmatter::markdown::schemas::simplified::serialize_property_atom(&PropertyAtom {
                description: None,
                ..atom.clone()
            });
            format!("Type: **{spelled}**")
        }
        TypeExpr::Imported { name, reference } => format!(
            "Type: **{}**",
            named_references::reference_spelling(name, reference, atom.is_array)
        ),
        TypeExpr::Union(arms) => {
            let union = PropertyDef::Union(arms.clone());
            let alternatives: Vec<&PropertyAtom> = flattened_alternatives(&union)
                .into_iter()
                .map(|alternative| alternative.atom)
                .collect();
            let lead = if atom.is_array {
                "Type: **array**, each item one of:"
            } else {
                "Type: one of:"
            };
            format!("{lead}\n{}", alternative_lines(&alternatives, definitions))
        }
    };
    lines.push(type_line);
    if let TypeExpr::Ref(key) = &atom.ty
        && let Some(definition) = definitions.lookup_definition(key)
    {
        let body = darkmatter::markdown::schemas::simplified::serialize_property_def(&definition.body);
        lines.push(format!("`{}` is defined as: `{body}`", key.name));
    }

    // A `literal(x)` atom shows its exact pinned value directly under the type,
    // ahead of the shared constraint/description lines.
    if let Some(value) = atom.literal_value() {
        lines.push(format!("Value: {}", hover_italic(&value.to_string())));
    }

    if is_required(def) {
        lines.push("Required".to_string());
    }
    lines.extend(eager_hover_line(def));
    lines.extend(alphanumeric_hover_lines(atom));
    if let Some(members) = enum_members(atom) {
        let italicized: Vec<String> = members.iter().map(|m| hover_italic(m)).collect();
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
    markup_hover(ctx, entry.key_span.clone()?, value)
}

/// The Markdown body of a frontmatter `ctx.*` hover.
///
/// The catalog-backed block comes from the shared Phase-1 formatter
/// ([`expressions::format_ctx_hover_block`]) so it is byte-identical to the
/// interpolation hover's block for the same variable; the compose-time note is
/// interpolation-specific and never appears here.
fn ctx_hover_markdown(dotted: &str, key: &str) -> String {
    if dotted == expressions::context_root().name {
        return "**`ctx`** — Darkmatter-generated context (read-only)".to_string();
    }
    match expressions::ctx_descriptor(key) {
        Some(descriptor) => expressions::format_ctx_hover_block(descriptor),
        None => format!("**`ctx.{key}`** — Darkmatter-generated (read-only)"),
    }
}

// ── Navigation (definition + document links) ────────────────────────────────

/// Definition on a named-type reference, a `$schema` file reference, or a
/// `file(...)` value. Only the scalar under the cursor is typed and resolved.
pub fn definition(ctx: &DocumentContext, offset: usize) -> Vec<Location> {
    if let Some(location) = named_references::definition(ctx, offset) {
        return vec![location];
    }
    let Some(ast) = overlay_ast(ctx) else {
        return Vec::new();
    };
    if ctx.file_context().is_err() {
        return Vec::new();
    }
    let schema = schema_target(ctx, ast).filter(|(span, _)| span.contains(&offset));
    let file = cursor_scalar(ast, offset)
        .and_then(|index| file_target(ctx, ast, index, scalar_typing(ctx, index)?));
    schema
        .into_iter()
        .chain(file)
        .filter_map(|(_, path)| {
            let uri = file_path_to_uri(&path)?;
            Some(Location::new(uri, Range::default()))
        })
        .collect()
}

/// Document links for `$schema` file references and `file(...)` values.
pub fn document_links(ctx: &DocumentContext) -> Vec<DocumentLink> {
    nav_targets(ctx)
        .into_iter()
        .filter_map(|(span, path)| {
            let range = ctx.source_map.byte_range_to_lsp(span)?;
            let target = file_path_to_uri(&path)?;
            Some(DocumentLink { range, target: Some(target), tooltip: None, data: None })
        })
        .collect()
}

/// The `(value_span, resolved_path)` navigation targets in the frontmatter:
/// the `$schema` file reference plus every `file(...)`-typed scalar value,
/// typed through the snapshot's scalar inventory.
///
/// Each value resolves through the document's context to the existing file,
/// else to where it would be created. A document without a context has no
/// targets. Resolution is never cached: it reads the filesystem.
fn nav_targets(ctx: &DocumentContext) -> Vec<(SourceSpan, PathBuf)> {
    let Some(ast) = overlay_ast(ctx) else {
        return Vec::new();
    };
    if ctx.file_context().is_err() {
        return Vec::new();
    }
    schema_target(ctx, ast)
        .into_iter()
        .chain(scalar_inventory(ctx).filter_map(|(index, typing)| file_target(ctx, ast, index, typing)))
        .collect()
}

/// Where a navigable frontmatter value points, through the document's context.
fn navigation_path(ctx: &DocumentContext, value: &str) -> Option<PathBuf> {
    ctx.resolve_reference(value).and_then(|target| target.navigation_target().map(Path::to_path_buf))
}

/// The `$schema: ./file.yaml` target (an inline `{ ... }` mapping is not a
/// file reference).
fn schema_target(ctx: &DocumentContext, ast: &FrontmatterAst) -> Option<(SourceSpan, PathBuf)> {
    let entry = ast.schema_entry().filter(|entry| entry.kind == FmValueKind::Scalar)?;
    let value = entry.scalar.as_deref().filter(|value| looks_like_path(value))?;
    Some((entry.value_span.clone(), navigation_path(ctx, value)?))
}

/// The target of the `file(...)`-typed scalar entry `index`, at any depth:
/// any union arm being a `file` type makes the value a navigable reference.
fn file_target(
    ctx: &DocumentContext,
    ast: &FrontmatterAst,
    index: usize,
    typing: &ScalarTyping,
) -> Option<(SourceSpan, PathBuf)> {
    let entry = ast.entries().get(index).filter(|_| typing.file)?;
    let value = entry.scalar.as_deref().filter(|value| is_schema_file_value(value))?;
    Some((entry.value_span.clone(), navigation_path(ctx, value)?))
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
        // A sequence item has no key line to fold from; its nested properties
        // still fold.
        let Some(key_span) = &entry.key_span else {
            continue;
        };
        let Some(start) = ctx.source_map.byte_to_lsp(key_span.start) else {
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
            let key_span = entry.key_span.clone()?;
            let range = ctx
                .source_map
                .byte_range_to_lsp(key_span.start..entry.value_span.end)?;
            let selection = ctx.source_map.byte_range_to_lsp(key_span)?;
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
/// with each matched extension baseline (e.g. a configured extension), then the
/// matching trigger payloads, then the document's own `$schema` — the layer
/// order and per-property precedence validation uses
/// ([`EffectiveSchema::property_shape`](darkmatter::markdown::schemas::EffectiveSchema::property_shape)).
///
/// Assembled at most once per analysis snapshot and shared by every request
/// on it ([`FrontmatterAnalysis::root`]); a context with no overlay gets the
/// base properties alone.
///
/// Exposed for reuse by other frontmatter-aware providers (e.g. navigation).
pub(crate) fn known_shape<'c>(ctx: &DocumentContext<'c>) -> &'c SchemaShape {
    match ctx.overlay {
        Some(overlay) => overlay.analysis.root(),
        None => baseline_root(),
    }
}

/// Where the editor opens a [`TypeExpr::Ref`] met in [`known_shape`]: the
/// document bundle's definitions (see
/// [`SchemaBundle`](crate::overlay::SchemaBundle)'s lookup), or none without
/// a bundle. Every walk below a recursive type reads through it, so none
/// judges a reference without its definition.
pub(crate) fn definitions<'c>(ctx: &DocumentContext<'c>) -> &'c dyn DefinitionLookup {
    match ctx.overlay.and_then(|overlay| overlay.bundle()) {
        Some(bundle) => bundle,
        None => &NoDefinitions,
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

/// The nested [`SchemaShape`] reached by walking typed `ancestors`, selecting
/// the first inline-object arm of each property, a recursive reference opened
/// through `definitions` one step at a time. An empty path yields `root`.
///
/// A key followed by an [`FmPathSegment::Index`] descends an array arm's item
/// shape (`initialize[0]`); a bare key descends a non-array arm. Every
/// other shape fails closed with `None`: an index with no preceding key, an
/// index into a non-array arm, a key into an array-only property, a nested
/// array index, or a property with no inline-object arm (e.g. the opaque
/// `object`-typed `style`).
pub(crate) fn nested_shape<'a>(
    root: &'a SchemaShape,
    ancestors: &[FmPathSegment<'_>],
    definitions: &'a dyn DefinitionLookup,
) -> Option<&'a SchemaShape> {
    let mut shape = root;
    let mut rest = ancestors;
    while !rest.is_empty() {
        let (key, item, tail) = split_step(rest)?;
        shape = inline_object_shape(shape.properties.get(key)?, item, definitions)?;
        rest = tail;
    }
    Some(shape)
}

/// Splits one schema step off a typed path: a key plus an optional single
/// array index. `None` for a path that starts with an index or crosses a
/// nested array, neither of which a SimplifiedSchema atom can declare.
fn split_step<'p, 's>(
    path: &'p [FmPathSegment<'s>],
) -> Option<(&'s str, bool, &'p [FmPathSegment<'s>])> {
    let (FmPathSegment::Key(key), tail) = path.split_first()? else {
        return None;
    };
    match tail.split_first() {
        Some((FmPathSegment::Index(_), after)) => match after.first() {
            Some(FmPathSegment::Index(_)) => None,
            _ => Some((key, true, after)),
        },
        _ => Some((key, false, tail)),
    }
}

/// Key segments for a key-only path.
pub(crate) fn key_segments<'k>(keys: &[&'k str]) -> Vec<FmPathSegment<'k>> {
    keys.iter().map(|key| FmPathSegment::Key(key)).collect()
}

/// How a context-aware schema walk treats a key step on an array-typed arm.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ArrayCrossing {
    /// Addressed entry paths ([`FrontmatterAst::path_at`]) name every array
    /// crossing with an index, so an index must meet an array arm and a bare
    /// key must meet a non-array arm.
    Explicit,
    /// Completion's authoring ancestry is key-only: the indentation fallback
    /// cannot know which item a line belongs to, and a `- ` line authors an
    /// item of the array it sits under. A key step may therefore enter an
    /// array arm's item shape.
    Transparent,
}

/// Per-pass memo of context-aware nested shapes, keyed by typed ancestor path,
/// over the snapshot's shared schema state.
///
/// The memo only remembers which shape each instance path reached, so a pass
/// repeats no discriminant selection; it owns no shape. Every shape it hands
/// out is borrowed from the snapshot's [`FrontmatterAnalysis`]: an arm, inline
/// object, or opened recursive definition is borrowed where it lives, and a
/// merged view of several arms is built once per snapshot for its schema node
/// and admitted arms, then shared by every instance and request that reaches
/// it. Siblings that select different arms get different shapes; siblings
/// that select the same arm, and recursive depths that return to the same
/// definition, get the same one.
///
/// Create one per pass and drop it with the pass. A context with no overlay
/// has no frontmatter to descend into, so only its top-level properties
/// resolve.
pub(crate) struct ShapeMemo<'c, 'p> {
    crossing: ArrayCrossing,
    root: &'c SchemaShape,
    analysis: Option<&'c FrontmatterAnalysis>,
    /// The authored, typed frontmatter discriminants are read from.
    frontmatter: Option<&'c Value>,
    definitions: &'c dyn DefinitionLookup,
    shapes: HashMap<Vec<FmPathSegment<'p>>, Option<&'c SchemaShape>>,
}

impl<'c, 'p> ShapeMemo<'c, 'p> {
    pub(crate) fn new(ctx: &DocumentContext<'c>, crossing: ArrayCrossing) -> Self {
        Self {
            crossing,
            root: known_shape(ctx),
            analysis: ctx.overlay.map(|overlay| &*overlay.analysis),
            frontmatter: ctx.overlay.and_then(|overlay| overlay.bundle()).map(|bundle| &bundle.frontmatter_json),
            definitions: definitions(ctx),
            shapes: HashMap::new(),
        }
    }

    /// The context-aware shape at `ancestors`, resolved at most once per
    /// distinct ancestor path (prefixes included) for the life of the memo.
    ///
    /// When an ancestor property is a union of inline-object arms tagged by a
    /// shared `literal(...)` discriminant, and the authored sibling values in
    /// that mapping select exactly one arm (via the Phase-4
    /// [`select_literal_discriminant_arm`] — never a second, DMLS-only
    /// algorithm), the walk descends into that arm. When narrowing is
    /// unavailable (absent/unknown/duplicate/conflicting discriminant, or an
    /// ordinary non-discriminated union), it descends into a MERGED view of
    /// every admissible inline-object arm instead — the root-union policy of
    /// [`EffectiveSchema::property_shape`](darkmatter::markdown::schemas::EffectiveSchema::property_shape) applied to nested unions (spec D3 / AC-10).
    pub(crate) fn shape(&mut self, ancestors: &[FmPathSegment<'p>]) -> Option<&'c SchemaShape> {
        if ancestors.is_empty() {
            return Some(self.root);
        }
        self.ensure(ancestors);
        self.shapes.get(ancestors).copied().flatten()
    }

    fn ensure(&mut self, ancestors: &[FmPathSegment<'p>]) {
        if ancestors.is_empty() || self.shapes.contains_key(ancestors) {
            return;
        }
        let Some(step_start) = ancestors
            .iter()
            .rposition(|segment| matches!(segment, FmPathSegment::Key(_)))
        else {
            self.shapes.insert(ancestors.to_vec(), None);
            return;
        };
        let parent = &ancestors[..step_start];
        let next = self.shape(parent).and_then(|shape| {
            let (key, item, tail) = split_step(&ancestors[step_start..])?;
            if !tail.is_empty() || (item && self.crossing == ArrayCrossing::Transparent) {
                return None;
            }
            self.descend(shape.properties.get(key)?, ancestors, item)
        });
        self.shapes.insert(ancestors.to_vec(), next);
    }

    /// One context-aware step into `def`'s inline-object arms, for the mapping
    /// authored at `path`. `item` is whether the step consumed an array index.
    ///
    /// A recursive reference at the top of `def` is opened through the bundle's
    /// definitions, so each step opens at most the one definition it stands on;
    /// the referenced body's own references wait for a later step.
    fn descend(&self, def: &'c PropertyDef, path: &[FmPathSegment<'_>], item: bool) -> Option<&'c SchemaShape> {
        let admission = match self.crossing {
            ArrayCrossing::Transparent => ArmAdmission::Any,
            ArrayCrossing::Explicit if item => ArmAdmission::Items,
            ArrayCrossing::Explicit => ArmAdmission::Values,
        };
        // Aligned with the flattened alternatives so a selected discriminant
        // index maps back; the object arms of `U[]` are the item's alternatives.
        let arms: Vec<Option<&'c SchemaShape>> = resolved_alternatives(def, self.definitions)
            .into_iter()
            .map(|alternative| match &alternative.atom.ty {
                TypeExpr::InlineObject(inner)
                    if admission == ArmAdmission::Any || alternative.is_array == item =>
                {
                    Some(inner)
                }
                _ => None,
            })
            .collect();
        if let Some(selected) = self.discriminated_arm(def, admission, &arms, path) {
            return Some(selected);
        }
        let mut admitted = arms.iter().flatten();
        match (admitted.next(), admitted.next()) {
            (None, _) => None,
            // One admissible arm is its own merged view.
            (Some(only), None) => Some(*only),
            (Some(_), Some(_)) => self
                .analysis?
                .merged_shape(def, admission, &arms, || merged_inline_object_shape(&arms)),
        }
    }

    /// The arm shape a shared literal discriminant selects for the mapping at
    /// `path`, or `None` when fewer than two arms exist or no arm is
    /// unambiguously selected. `arms` is aligned with `def`'s alternatives; an
    /// inadmissible arm is `None` and contributes no discriminant properties.
    ///
    /// The authored instance is the already-parsed, correctly-typed
    /// frontmatter mapping at `path` (from the effective schema bundle), so a
    /// string `'2'` never matches a numeric `literal(2)`. Only the arms'
    /// discriminant JSON is shared across instances; the selection is made
    /// for each instance.
    fn discriminated_arm(
        &self,
        def: &'c PropertyDef,
        admission: ArmAdmission,
        arms: &[Option<&'c SchemaShape>],
        path: &[FmPathSegment<'_>],
    ) -> Option<&'c SchemaShape> {
        if arms.len() < 2 {
            return None;
        }
        let instance = navigate_json(self.frontmatter?, path)?;
        let build = || {
            arms.iter()
                .map(|arm| arm.map_or_else(|| serde_json::json!({}), shape_discriminant_json))
                .collect()
        };
        let index = match self.analysis {
            Some(analysis) => {
                select_literal_discriminant_arm(analysis.discriminants(def, admission, build), instance)
            }
            None => select_literal_discriminant_arm(&build(), instance),
        }?;
        arms[index]
    }

    /// The [`PropertyDef`] at a full typed `path`, context-aware at each
    /// ancestor.
    ///
    /// This is the single context-aware schema-path resolver shared by every
    /// value-oriented capability (value completion, hover, expression
    /// gating/diagnostics, file navigation) so they resolve against the same
    /// selected arm key completion narrows to. A leaf key present in more than
    /// one merged arm with divergent types resolves to the union of every
    /// arm's atoms. A trailing index (an addressed sequence item) resolves to
    /// the array arms' item definition, synthesized once per distinct content
    /// and shared across snapshots. The result is borrowed from the snapshot;
    /// nothing is cloned to return it.
    pub(crate) fn def_at(&mut self, path: &[FmPathSegment<'p>]) -> Option<&'c PropertyDef> {
        let leaf_at = path.iter().rposition(|segment| matches!(segment, FmPathSegment::Key(_)))?;
        let FmPathSegment::Key(leaf) = path[leaf_at] else {
            return None;
        };
        let def = self.shape(&path[..leaf_at])?.properties.get(leaf)?;
        match &path[leaf_at + 1..] {
            [] => Some(def),
            [FmPathSegment::Index(_)] if self.crossing == ArrayCrossing::Explicit => self
                .analysis?
                .item_definition(def, || array_item_atoms(def, self.definitions), array_item_def),
            _ => None,
        }
    }
}

/// A merged view of every admissible inline-object arm's properties, for an
/// ancestor union whose discriminant does not select a single arm. Keys appear
/// in arm-declaration then property-declaration order; a key contributed by more
/// than one arm resolves to the union of those arms' atoms (via [`merge_defs`]),
/// so a same-named property whose type diverges across arms stays a union rather
/// than collapsing to the first arm. `None` when no arm is admissible.
fn merged_inline_object_shape(arms: &[Option<&SchemaShape>]) -> Option<SchemaShape> {
    let mut merged: Option<SchemaShape> = None;
    for inner in arms.iter().flatten() {
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

/// Walks a typed `path` into JSON: keys index objects, indices index arrays.
fn navigate_json<'a>(root: &'a Value, path: &[FmPathSegment<'_>]) -> Option<&'a Value> {
    let mut current = root;
    for segment in path {
        current = match segment {
            FmPathSegment::Key(key) => current.as_object()?.get(*key)?,
            FmPathSegment::Index(index) => current.as_array()?.get(*index)?,
        };
    }
    Some(current)
}

/// The [`PropertyDef`] at a full `path` whose last segment is a key, descending
/// the **first** admissible inline-object arm of each ancestor. This is the
/// context-free resolver: it never consults the authored mapping, so a
/// discriminated ancestor union always resolves against its first arm.
///
/// Callers that have a [`DocumentContext`] and must honor a selected
/// discriminated arm use [`ShapeMemo::def_at`] instead; this pure variant survives
/// for the context-free code-action/DSL callers and the unit tests.
pub(crate) fn def_at_path<'a>(
    root: &'a SchemaShape,
    path: &[FmPathSegment<'_>],
    definitions: &'a dyn DefinitionLookup,
) -> Option<&'a PropertyDef> {
    let (FmPathSegment::Key(leaf), ancestors) = path.split_last()? else {
        return None;
    };
    nested_shape(root, ancestors, definitions)?.properties.get(*leaf)
}

/// The atoms of `def`'s array arms, which one item's definition is built
/// from ([`array_item_def`]). The item of an array of a union (`U[]`) has its
/// flattened alternatives, and the item of a recursive `item[]` its
/// definition's alternatives.
fn array_item_atoms<'a>(def: &'a PropertyDef, definitions: &'a dyn DefinitionLookup) -> Vec<&'a PropertyAtom> {
    resolved_alternatives(def, definitions)
        .into_iter()
        .filter(|alternative| alternative.is_array)
        .map(|alternative| alternative.atom)
        .collect()
}

/// The definition of one item built from its array arms' `atoms`, or `None`
/// when there is none: a property-level union of several. The container's
/// markers, array constraints, and description describe the array and are
/// not carried onto the item.
fn array_item_def(atoms: &[&PropertyAtom]) -> Option<PropertyDef> {
    let mut items: Vec<PropertyAtom> = atoms
        .iter()
        .map(|atom| PropertyAtom {
            is_array: false,
            array_constraints: Vec::new(),
            ..(*atom).clone()
        })
        .collect();
    match items.len() {
        0 => None,
        1 => items.pop().map(PropertyDef::Single),
        _ => Some(PropertyDef::Union(items)),
    }
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

/// The shape of the first inline-object arm whose array-ness matches `item`,
/// if any — the deterministic arm a context-free nested lookup descends
/// through even when it is not first. The arms of a union-typed reference
/// count, so `U[]`'s object arms are item arms, and so do a recursive
/// reference's, opened through `definitions`.
fn inline_object_shape<'a>(
    def: &'a PropertyDef,
    item: bool,
    definitions: &'a dyn DefinitionLookup,
) -> Option<&'a SchemaShape> {
    resolved_alternatives(def, definitions).into_iter().find_map(|alternative| match &alternative.atom.ty {
        TypeExpr::InlineObject(inner) if alternative.is_array == item => Some(inner),
        // Resolution has already replaced a union type with its arms; a
        // reference left in place is one with no definition, offering nothing.
        TypeExpr::InlineObject(_)
        | TypeExpr::Primitive(_)
        | TypeExpr::Imported { .. }
        | TypeExpr::Union(_)
        | TypeExpr::Tuple(_)
        | TypeExpr::Function(_)
        | TypeExpr::Ref(_) => None,
    })
}

/// Whether any arm of a property declares a constraint matching `predicate`,
/// inspecting an atom's item-level constraints *and* its postfix array
/// constraints. Both lists must be read: `file(eager)[]` writes `eager` to the
/// item list while `file[](eager)` writes it to the array list, and the same
/// split applies to `required`. Reading only one list silently drops the marker
/// for the other spelling.
///
/// A union-typed reference's own markers count, and so do its arms', the way
/// the library hoists an inherited marker (`U[]` over a `required` arm is a
/// required property).
fn declares_constraint(def: &PropertyDef, predicate: fn(&Constraint) -> bool) -> bool {
    fn atom_declares(atom: &PropertyAtom, predicate: fn(&Constraint) -> bool) -> bool {
        atom.constraints.iter().chain(atom.array_constraints.iter()).any(predicate)
            || atom
                .union_arms()
                .is_some_and(|arms| arms.iter().any(|arm| atom_declares(arm, predicate)))
    }
    atoms_of(def).iter().any(|atom| atom_declares(atom, predicate))
}

/// Whether a property is required, by the library's shared presence
/// calculation: a null-admitting whole value is never required.
fn is_required(def: &PropertyDef) -> bool {
    def.is_required()
}

/// Whether any arm of a property declares `eager`, at item or array level.
///
/// Disclosure only — this says nothing about presence. Darkmatter's phase
/// projection remains the sole authority for which placement makes the
/// *property* launch-critical.
fn is_eager(def: &PropertyDef) -> bool {
    declares_constraint(def, |c| matches!(c, Constraint::Eager))
}

/// The hover line explaining `eager` timing, worded by the Darkmatter
/// descriptor catalog so completion and both hover surfaces stay identical.
fn eager_hover_line(def: &PropertyDef) -> Option<String> {
    if !is_eager(def) {
        return None;
    }
    schema_constraint_descriptors()
        .iter()
        .find(|descriptor| descriptor.keyword == "eager")
        .map(|descriptor| format!("Eager: {}", descriptor.description))
}

/// The enum members declared on an atom, if any.
fn enum_members(atom: &PropertyAtom) -> Option<&[String]> {
    atom.constraints.iter().find_map(|constraint| match constraint {
        Constraint::Members(members) => Some(members.as_slice()),
        _ => None,
    })
}

/// Italicizes `text` for a hover line, where it follows a space and precedes
/// `,` or the line end.
///
/// A `_` or `\` at either edge is backslash-escaped so it cannot join or
/// escape the `_` delimiters (`_x` → `_\_x_`, not `__x_`). Text that is
/// empty or has edge whitespace, where no delimiter can open or close, is
/// written plain.
fn hover_italic(text: &str) -> String {
    if text.is_empty()
        || text.starts_with(char::is_whitespace)
        || text.ends_with(char::is_whitespace)
    {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len() + 4);
    out.push('_');
    let last = text.char_indices().next_back().map(|(index, _)| index);
    for (index, c) in text.char_indices() {
        if (index == 0 || Some(index) == last) && matches!(c, '_' | '\\') {
            out.push('\\');
        }
        out.push(c);
    }
    out.push('_');
    out
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
/// a top-level key line; a column-0 `- ` item belongs to the key above it.
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
        match ast.container_at_offset(offset, line_start) {
            // Authoring ancestry is key-only, matching the indentation
            // fallback below, which cannot know an item index.
            Some(container) if still_placed(ctx, container) => {
                return ast
                    .path_of(container)
                    .into_iter()
                    .filter_map(|segment| match segment {
                        FmPathSegment::Key(key) => Some(key.to_string()),
                        FmPathSegment::Index(_) => None,
                    })
                    .collect();
            }
            // No enclosing mapping at column 0 is a real answer, not a gap,
            // except for an indentless sequence item, whose parent key
            // shares its column.
            None if indent == 0 && !is_sequence_item_line(ctx.text, line_start) => {
                return Vec::new();
            }
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
///
/// A sequence item has no key token, so it is placed when the nearest
/// enclosing mapping property still is.
fn still_placed(ctx: &DocumentContext, entry: &FmEntry) -> bool {
    let mut cursor = Some(entry);
    while let Some(candidate) = cursor {
        if let Some(key_span) = &candidate.key_span {
            return ctx
                .text
                .get(key_span.clone())
                .and_then(decode_scalar)
                .is_some_and(|decoded| decoded.decoded() == candidate.key);
        }
        cursor = candidate
            .parent
            .and_then(|parent| overlay_ast(ctx)?.entries().get(parent));
    }
    false
}

/// The `(key, value-partial)` pair when the cursor sits in a value position.
///
/// Structural first: an authored entry yields the entire *decoded* key, so
/// `"build.target": …` is one key and `"host: port": …` is not split at its
/// embedded colon. The lexical split at the line's YAML key separator, which
/// keeps a content colon (`a:b: …`) in the key, is the fallback for a cursor no
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
        && let Some(key_span) = &entry.key_span
        && let Some(rest) = ctx.text.get(key_span.end..offset)
        && let Some((_, partial)) = rest.split_once(':')
    {
        return Some((entry.key.clone(), partial.trim_start().to_string()));
    }
    let colon = mapping_separator(trimmed, false)?;
    Some((
        trimmed[..colon].trim().to_string(),
        trimmed[colon + 1..].trim_start().to_string(),
    ))
}

/// The full chain of ancestor keys above `line_start`, outermost first, for a
/// line at column `indent` — one key per strictly-decreasing indent level, so
/// nested inline-object mappings resolve. Empty for a top-level line.
///
/// A `- ` item line may also be held by an earlier `key:` line at its own
/// column (an indentless block sequence); sibling items at that column are
/// skipped on the way there.
fn enclosing_path_by_indent(text: &str, line_start: usize, indent: usize) -> Vec<String> {
    let mut path = Vec::new();
    let mut needed = indent;
    let mut indentless_item = is_sequence_item_line(text, line_start);
    for line in text[..line_start].lines().rev() {
        let trimmed = line.trim_start();
        if trimmed.is_empty() || trimmed == "---" {
            continue;
        }
        let line_indent = line.len() - trimmed.len();
        let held_indentless = indentless_item
            && line_indent == needed
            && !(trimmed == "-" || trimmed.starts_with("- "));
        if line_indent < needed || held_indentless {
            indentless_item = false;
            let key = mapping_separator(trimmed, false)
                .map_or(trimmed, |colon| &trimmed[..colon])
                .trim();
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

/// Whether the line starting at `line_start` is a block sequence item.
fn is_sequence_item_line(text: &str, line_start: usize) -> bool {
    let line = text[line_start..].lines().next().unwrap_or_default().trim();
    line == "-" || line.starts_with("- ")
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

mod named_references;

#[cfg(test)]
mod sequence_tests;

#[cfg(test)]
mod snapshot_tests;

#[cfg(test)]
mod tests {
    use super::*;

    use darkmatter::markdown::schemas::alphanumeric_option_descriptors;
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
        let overlay = state.for_document(&uri, text, path, &config, &roots, &crate::context::test_support::resolution_for(path));
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let mut indices = BTreeMap::new();
        for (doc_path, source) in docs {
            indices.insert(
                PathBuf::from(doc_path),
                crate::graph::index_document(Path::new(doc_path), source),
            );
        }
        let graph = WorkspaceGraph::build(&indices, 1, &crate::context::test_support::workspace_contexts());
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
            resolution: &crate::context::test_support::resolution_for(path),
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
            let declared = darkmatter::markdown::compose::expression::DeclaredTypes::new(None);
            let expected = expressions::hover_markdown(
                "ctx.today",
                2,
                &darkmatter::markdown::compose::expression::TypeScope::unguarded(&declared),
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
                let targets = nav_targets(ctx);
                assert!(
                    targets.iter().any(|(_, path)| path.ends_with("target.md")),
                    "deleted arm `asset` navigates (deleted_first={deleted_first}): {targets:#?}"
                );
            });
            let created = change_union_doc(deleted_first, "  kind: created\n  asset: ./target.md\n");
            with_ctx(&created, |ctx| {
                let targets = nav_targets(ctx);
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
                let targets = nav_targets(ctx);
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

        let body = schema_hover_body("status", &def, &NoDefinitions).expect("hover body");

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
        let body = schema_hover_body("title", &def, &NoDefinitions).expect("hover body");
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

    /// The one authoritative `eager` wording. Every hover surface must reuse
    /// this string byte-for-byte instead of keeping a DMLS-local copy.
    fn eager_catalog_description() -> &'static str {
        schema_constraint_descriptors()
            .iter()
            .find(|descriptor| descriptor.keyword == "eager")
            .expect("the Darkmatter catalog declares `eager`")
            .description
    }

    /// A root `$schema` document declaring a single property `p` as `def`.
    fn schema_def(def: &str) -> PropertyDef {
        let text = format!("---\n$schema:\n  p: {def}\n---\n\nbody\n");
        with_ctx(&text, |ctx| {
            known_shape(ctx)
                .properties
                .get("p")
                .cloned()
                .unwrap_or_else(|| panic!("`{def}` parses into the root shape"))
        })
    }

    /// A root `$schema` document declaring `p` as a property-level union
    /// (a YAML sequence of type expressions).
    fn schema_union_def(arms: &[&str]) -> PropertyDef {
        let body: String = arms.iter().map(|arm| format!("    - {arm}\n")).collect();
        let text = format!("---\n$schema:\n  p:\n{body}---\n\nbody\n");
        with_ctx(&text, |ctx| {
            known_shape(ctx)
                .properties
                .get("p")
                .cloned()
                .unwrap_or_else(|| panic!("`{arms:?}` parses into the root shape"))
        })
    }

    #[test]
    fn constraint_markers_are_detected_at_item_and_array_level_and_across_union_arms() {
        // `file(eager)[]` writes `eager` to the item list; `file[](eager)`
        // writes it to the postfix array list. Reading only one list drops the
        // marker for the other spelling — the pre-fix DMLS defect.
        for placement in ["file(eager)[]", "file[](eager)"] {
            let def = schema_def(placement);
            assert!(is_eager(&def), "eager is detected for `{placement}`: {def:?}");
            assert!(
                !is_required(&def),
                "eager placement alone never implies presence for `{placement}`: {def:?}"
            );
        }

        for placement in ["file(required)[]", "file[](required)"] {
            let def = schema_def(placement);
            assert!(is_required(&def), "required is detected for `{placement}`: {def:?}");
            assert!(
                !is_eager(&def),
                "required placement alone never implies eager timing for `{placement}`: {def:?}"
            );
        }

        let both = schema_def("file[](required; eager)");
        assert!(is_required(&both) && is_eager(&both), "{both:?}");

        // A union discloses a marker declared on any arm, at either level.
        let item_arm = schema_union_def(&["string", "file(eager)[]"]);
        assert_eq!(atoms_of(&item_arm).len(), 2, "{item_arm:?}");
        assert!(is_eager(&item_arm), "{item_arm:?}");
        let array_arm = schema_union_def(&["string", "file[](required)"]);
        assert_eq!(atoms_of(&array_arm).len(), 2, "{array_arm:?}");
        assert!(is_required(&array_arm), "{array_arm:?}");

        let bare = schema_def("file[]");
        assert!(!is_eager(&bare) && !is_required(&bare), "{bare:?}");
    }

    #[test]
    fn eager_hover_prose_is_taken_from_the_darkmatter_descriptor_catalog() {
        let def = schema_def("string(eager)");
        assert_eq!(
            eager_hover_line(&def),
            Some(format!("Eager: {}", eager_catalog_description()))
        );
        assert_eq!(eager_hover_line(&schema_def("string")), None);

        let definition_hover = meta_schema_definition_hover_body("p", &def);
        let instance_hover = schema_hover_body("p", &def, &NoDefinitions).expect("hover body");
        for body in [&definition_hover, &instance_hover] {
            assert!(
                body.contains(eager_catalog_description()),
                "hover reuses the catalog description verbatim: {body}"
            );
        }
    }

    #[test]
    fn schema_definition_hover_separates_eager_timing_from_required_presence() {
        let eager_only = meta_schema_definition_hover_body("p", &schema_def("string(eager)"));
        assert!(eager_only.contains(eager_catalog_description()), "{eager_only}");
        assert!(
            !eager_only.contains("Required"),
            "eager alone must not display the presence marker: {eager_only}"
        );

        let combined = meta_schema_definition_hover_body("p", &schema_def("string(required; eager)"));
        assert!(combined.contains("Required"), "{combined}");
        assert!(combined.contains(eager_catalog_description()), "{combined}");

        let required_only = meta_schema_definition_hover_body("p", &schema_def("string(required)"));
        assert!(required_only.contains("Required"), "{required_only}");
        assert!(
            !required_only.contains(eager_catalog_description()),
            "{required_only}"
        );
    }

    #[test]
    fn schema_bound_instance_hover_separates_eager_timing_from_required_presence() {
        let eager_only = schema_hover_body("p", &schema_def("string(eager)"), &NoDefinitions).expect("hover body");
        assert!(eager_only.contains(eager_catalog_description()), "{eager_only}");
        assert!(
            !eager_only.contains("Required"),
            "eager alone must not display the presence marker: {eager_only}"
        );

        let combined =
            schema_hover_body("p", &schema_def("string(required; eager)"), &NoDefinitions).expect("hover body");
        assert!(combined.contains("Required"), "{combined}");
        assert!(combined.contains(eager_catalog_description()), "{combined}");

        // Existing formatting is preserved alongside the new line.
        let annotated = schema_hover_body(
            "p",
            &schema_def("enum(draft, published; eager; default(draft)) -> the publication state"),
            &NoDefinitions,
        )
        .expect("hover body");
        assert!(annotated.contains("Values: _draft_, _published_"), "{annotated}");
        assert!(annotated.contains("Default: _\"draft\"_"), "{annotated}");
        assert!(annotated.contains("the publication state"), "{annotated}");
        assert!(annotated.contains(eager_catalog_description()), "{annotated}");
    }

    #[test]
    fn both_hover_surfaces_disclose_array_placements_without_claiming_presence() {
        for placement in ["file(eager)[]", "file[](eager)"] {
            let def = schema_def(placement);
            let definition_hover = meta_schema_definition_hover_body("p", &def);
            let instance_hover = schema_hover_body("p", &def, &NoDefinitions).expect("hover body");
            for body in [&definition_hover, &instance_hover] {
                assert!(
                    body.contains(eager_catalog_description()),
                    "`{placement}` discloses eager timing: {body}"
                );
                assert!(
                    !body.contains("Required"),
                    "`{placement}` must not claim presence: {body}"
                );
            }
        }

        let array_required = schema_def("file[](required)");
        let definition_hover = meta_schema_definition_hover_body("p", &array_required);
        let instance_hover = schema_hover_body("p", &array_required, &NoDefinitions).expect("hover body");
        for body in [&definition_hover, &instance_hover] {
            assert!(
                body.contains("Required"),
                "array-level `required` keeps its presence marker: {body}"
            );
        }
    }

    /// Punctuation inside a quoted enum member neither ends the member list nor
    /// starts a union alternative, so type keywords are offered only once the
    /// cursor is in a new alternative.
    #[test]
    fn a_quoted_enum_member_keeps_the_type_definition_completion_context() {
        let rows: &[(&str, bool)] = &[
            ("[enum(a), s", true),
            ("[enum(a, s", false),
            ("[enum('a)b', c), s", true),
            ("[enum(\"a]b\", c), s", true),
            ("[enum('a(b', c), s", true),
            ("[enum('a)b', s", false),
            ("[enum(\"a)b\", s", false),
            ("[enum('a]b', s", false),
        ];
        for (value, offers_types) in rows {
            let text = format!("---\n$schema:\n  title: {value}]\n---\n\nbody\n");
            with_ctx(&text, |ctx| {
                let offset = text.find(value).unwrap() + value.len();
                let items = completion(ctx, offset);
                let string = items.iter().find(|item| item.label == "string");
                assert_eq!(string.is_some(), *offers_types, "{value:?}: {items:#?}");
                if let Some(string) = string {
                    let Some(CompletionTextEdit::Edit(edit)) = &string.text_edit else {
                        panic!("{value:?}: type completion has an eager edit");
                    };
                    let end = ("  title: ".len() + value.len()) as u32;
                    assert_eq!(edit.range.start.character, end - 1, "{value:?}");
                    assert_eq!(edit.range.end.character, end, "{value:?}");
                }
            });
        }
    }

    /// Description prose and imported filenames are not argument syntax, so
    /// their punctuation never hides the next alternative or property. Each
    /// failing shape offers the same type completion and replacement range as
    /// the controls beside it.
    #[test]
    fn descriptions_and_file_references_keep_the_type_definition_completion_context() {
        let flow_rows = [
            // Controls.
            "[string -> plain, s",
            "[string -> (plain), s",
            "[string -> it's fine, s",
            "[Name@./a(b)c.yaml, s",
            // Prose and filename punctuation.
            "[string -> (it's fine), s",
            "[string -> (say \"hi), s",
            "[Name@./a(b.yaml, s",
            // YAML ends a plain flow scalar at `{` or `[`, so those filenames
            // are written as quoted arms (asserted below).
            "['Name@./a{b.yaml', s",
            "[\"Name@./a[b.yaml\", s",
        ];
        for invalid in ["[Name@./a{b.yaml, s]", "[Name@./a[b.yaml, s]"] {
            assert!(
                serde_yaml_ng::from_str::<serde_yaml_ng::Value>(invalid).is_err(),
                "{invalid:?} is not a YAML flow sequence"
            );
        }
        let scalar_rows = [
            // Controls.
            "'{ a: string -> plain, b: s",
            "'{ a: string -> (plain), b: s",
            // Prose and filename punctuation.
            "'{ a: string -> (it''s fine), b: s",
            "\"{ a: string -> (say \\\"hi), b: s",
            "'{ r: Name@./a(b.yaml, b: s",
            "'{ r: Name@./a{b.yaml -> (it''s), b: s",
        ];
        let rows = flow_rows
            .iter()
            .map(|value| (*value, format!("{value}]")))
            .chain(scalar_rows.iter().map(|value| {
                let quote = &value[..1];
                (*value, format!("{value} }}{quote}"))
            }));
        for (value, authored) in rows {
            let text = format!("---\n$schema:\n  title: {authored}\n---\n\nbody\n");
            with_ctx(&text, |ctx| {
                let offset = text.find(value).unwrap() + value.len();
                let items = completion(ctx, offset);
                let Some(string) = items.iter().find(|item| item.label == "string") else {
                    panic!("{value:?} offers `string`: {items:#?}");
                };
                let Some(CompletionTextEdit::Edit(edit)) = &string.text_edit else {
                    panic!("{value:?}: type completion has an eager edit");
                };
                let end = ("  title: ".len() + value.len()) as u32;
                assert_eq!(edit.range.start.line, 2, "{value:?}");
                assert_eq!(edit.range.start.character, end - 1, "{value:?}");
                assert_eq!(edit.range.end.character, end, "{value:?}");
            });
        }
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
                for alias in descriptor.aliases {
                    assert!(labels.contains(alias), "alias `{alias}` is offered: {items:#?}");
                }
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

        // `alphanumeric` options come from the catalog's typed option
        // descriptors, before the first `;` only.
        let text = "---\n$schema:\n  slug: alphanumeric(leading-alpha, \n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("alpha, ").unwrap() + "alpha, ".len();
            let labels: Vec<String> = completion(ctx, offset).into_iter().map(|item| item.label).collect();
            let expected: Vec<String> = alphanumeric_option_descriptors()
                .iter()
                .map(|option| option.keyword().to_string())
                .filter(|keyword| keyword != "leading-alpha")
                .collect();
            assert_eq!(labels, expected);
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
                labels.iter().any(|label| label == "Base"),
                "the current parsed schema offers its names bare: {labels:?}"
            );
            assert!(
                labels.iter().all(|label| label != "Base@this"),
                "a non-colliding local name is never spelled `@this`: {labels:?}"
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

    /// An ancestor key holding a content colon or a quoted colon keeps it.
    #[test]
    fn enclosing_path_by_indent_keeps_a_colon_inside_an_ancestor_key() {
        let text = "---\na:b:\n  'x: y':\n    ";
        assert_eq!(enclosing_path_by_indent(text, text.len(), 4), vec!["a:b", "'x: y'"]);
    }

    /// The lexical fallback for an unparseable buffer splits a key line at
    /// its YAML separator, not at a content colon inside the key.
    #[test]
    fn value_cursor_fallback_keeps_a_colon_inside_the_key() {
        for (line, key, partial) in [
            ("title: val", "title", "val"),
            ("a:'b: val", "a:'b", "val"),
            ("a:b: val", "a:b", "val"),
            ("\"x: y\": val", "\"x: y\"", "val"),
            ("key:", "key", ""),
        ] {
            let text = format!("---\nbroken: [unclosed\n{line}");
            with_ctx(&text, |ctx| {
                let line_start = text.len() - line.len();
                let cursor = value_cursor(ctx, text.len(), line_start, line);
                assert_eq!(
                    cursor,
                    Some((key.to_string(), partial.to_string())),
                    "{line:?}"
                );
            });
        }
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
        assert!(nested_shape(&root, &key_segments(&[]), &NoDefinitions).unwrap().properties.contains_key("settings"));
        // One inline-object segment descends into its shape.
        let settings = nested_shape(&root, &key_segments(&["settings"]), &NoDefinitions).unwrap();
        assert!(settings.properties.contains_key("mode"));
    }

    #[test]
    fn test_base_style_schema_exposes_nested_completion_shapes() {
        let darkmatter::markdown::schemas::SimplifiedSchema::Single(root) =
            darkmatter::markdown::schemas::darkmatter_base_schema()
        else {
            panic!("base schema must be a single object shape");
        };

        let block_quote = nested_shape(&root, &key_segments(&["style", "block-quote"]), &NoDefinitions)
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

        let emphasis = nested_shape(&root, &key_segments(&["style", "block-quote", "emphasis"]), &NoDefinitions)
            .expect("compound emphasis must expose nested keys");
        assert!(emphasis.properties.contains_key("italic"));
        assert!(emphasis.properties.contains_key("underline"));

        let alignment = def_at_path(&root, &key_segments(&["style", "block-quote", "alignment"]), &NoDefinitions)
            .and_then(primary_atom)
            .and_then(enum_members)
            .expect("alignment must offer enum values");
        assert_eq!(alignment, ["left", "center", "right"]);
    }

    #[test]
    fn test_nested_shape_missing_segment_is_none() {
        let root = nested_fixture();
        assert!(nested_shape(&root, &key_segments(&["absent"]), &NoDefinitions).is_none());
    }

    #[test]
    fn test_nested_shape_non_inline_object_is_none() {
        let root = nested_fixture();
        // `title` is a primitive, not an inline object — cannot descend.
        assert!(nested_shape(&root, &key_segments(&["title"]), &NoDefinitions).is_none());
    }

    #[test]
    fn test_def_at_path_resolves_leaf_atom() {
        let root = nested_fixture();
        let def = def_at_path(&root, &key_segments(&["settings", "mode"]), &NoDefinitions).unwrap();
        assert_eq!(primary_atom(def).unwrap().ty, TypeExpr::Primitive(SimplifiedType::Enum));
        // A missing leaf under a valid inline object is `None`.
        assert!(def_at_path(&root, &key_segments(&["settings", "absent"]), &NoDefinitions).is_none());
        // A top-level leaf resolves through an empty ancestor path.
        assert!(def_at_path(&root, &key_segments(&["title"]), &NoDefinitions).is_some());
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
        let shape = inline_object_shape(&def, false, &NoDefinitions).expect("the inline-object arm is selectable");
        assert!(shape.properties.contains_key("mode"));
        // No inline-object arm → `None`.
        assert!(inline_object_shape(&PropertyDef::Single(PropertyAtom::bare(SimplifiedType::String)), false, &NoDefinitions).is_none());
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
        let settings = nested_shape(&root, &key_segments(&["settings"]), &NoDefinitions).expect("descends the union inline arm");
        assert!(settings.properties.contains_key("mode"));
        // And `def_at_path` reaches the nested leaf through the same union arm.
        assert!(def_at_path(&root, &key_segments(&["settings", "mode"]), &NoDefinitions).is_some());
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

    #[test]
    fn hover_italic_keeps_its_delimiters_able_to_open_and_close() {
        let cases = [
            ("draft", "_draft_"),
            ("\"published\"", "_\"published\"_"),
            ("snake_case", "_snake_case_"),
            // An edge `_` would join the delimiter run, an edge `\` would
            // escape it.
            ("_private", "_\\_private_"),
            ("trailing_", "_trailing\\__"),
            ("_", "_\\__"),
            ("a\\", "_a\\\\_"),
            // No delimiter opens before or closes after whitespace.
            (" padded", " padded"),
            ("", ""),
        ];
        for (text, expected) in cases {
            assert_eq!(hover_italic(text), expected, "{text:?}");
        }
    }

    #[test]
    fn a_required_union_reference_activates_authoring_in_its_object_arm() {
        // `holder: pair(required)` resolves to one atom whose type is the
        // union; activation must read its object arm to find `def`.
        let text = "---\n$schema:\n  pair: ['{ def: type-definition }', number]\n  holder: pair(required)\nholder:\n  def: str\n---\n\nbody\n";
        with_ctx(text, |ctx| {
            let offset = text.find("def: str").unwrap() + "def: str".len();
            let items = completion(ctx, offset);
            assert!(
                items.iter().any(|item| item.label == "string"),
                "type keyword completion inside the union's object arm: {items:#?}"
            );
        });
    }
}
