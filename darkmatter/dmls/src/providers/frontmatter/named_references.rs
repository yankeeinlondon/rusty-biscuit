//! Hover, go-to-definition, and completion for named-type references written
//! in a schema declaration: a frontmatter `$schema` or a standalone schema
//! document.
//!
//! References are found through the library's source map (see
//! [`AuthoredDeclaration::reference_at`]); a cross-file target is read from
//! the overlay's [`NamedTypeCatalog`](crate::overlay::NamedTypeCatalog), so no
//! request reads a file.

use std::ops::Range;
use std::sync::Arc;

use std::collections::BTreeSet;

use simplified_schema::simplified::LOCAL_REFERENCE;

use simplified_schema::{
    CompositionOperandPart, PropertyDef, SchemaCursor, SchemaDeclaration, SchemaRoot, SchemaShape,
    SchemaSourcePath, SchemaSourcePathSegment, SchemaSpanKind, SimplifiedType, TypeExpr, mapping_separator,
    parse_property_definition,
};
use lsp_types::{CompletionItem, CompletionItemKind, Hover, Location};

use super::{file_path_completions, item, markup_hover, meta_schema_definition_hover_body};
use crate::overlay::SchemaAuthoringState;
use crate::overlay::named_types::{AuthoredDeclaration, ExternalNamespace, NamedReference};
use crate::providers::DocumentContext;
use crate::source_map::SourceMap;
use crate::workspace::file_path_to_uri;

/// The canonical spelling of a local reference: bare, unless the name is a
/// built-in type keyword, which a bare spelling would select instead.
pub(super) fn local_reference_spelling(name: &str) -> String {
    if SimplifiedType::is_keyword(name) {
        format!("{name}@this")
    } else {
        name.to_string()
    }
}

/// The canonical spelling of a reference as hover shows it, with `[]` placed
/// where the grammar accepts it (`Name[]@target`).
pub(super) fn reference_spelling(name: &str, reference: &str, is_array: bool) -> String {
    let suffix = if is_array { "[]" } else { "" };
    if reference.trim() == "this" && !SimplifiedType::is_keyword(name) {
        format!("{name}{suffix}")
    } else {
        format!("{name}{suffix}@{reference}")
    }
}

/// The declaration this document authors, with spans in document offsets, or
/// `None` when it has none, the current buffer did not parse, or `offset`
/// lies outside a frontmatter declaration.
fn authored_declaration(ctx: &DocumentContext, offset: usize) -> Option<AuthoredDeclaration> {
    let overlay = ctx.overlay?;
    match &overlay.schema_authoring {
        SchemaAuthoringState::Standalone { model: Some(model), stale: false, .. } => {
            Some(AuthoredDeclaration::standalone(model))
        }
        SchemaAuthoringState::Standalone { .. } => None,
        _ if overlay.stale => None,
        _ => AuthoredDeclaration::frontmatter_at(ctx.text, overlay.ast.as_deref()?, offset),
    }
}

/// Where a reference's type is defined.
enum Definition<'a> {
    /// In this document's own declaration.
    Local { definition: PropertyDef, key_span: Range<usize> },
    /// In an imported schema file.
    External {
        external: &'a ExternalNamespace,
        definition: PropertyDef,
        key_span: Range<usize>,
    },
}

/// The reference under `offset` and the definition it selects.
fn reference_definition<'a>(
    ctx: &'a DocumentContext,
    offset: usize,
) -> Option<(NamedReference, Definition<'a>)> {
    let declaration = authored_declaration(ctx, offset)?;
    let reference = declaration.reference_at(ctx.text, offset)?;
    let definition = match &reference.target {
        None => {
            let (namespace, root) = declaration.local_namespace(&reference.path)?;
            let definition = namespace.properties.get(&reference.name)?.clone();
            let key_span = declaration
                .source_map
                .spans(&root.property(&reference.name), SchemaSpanKind::MappingKey)
                .first()?
                .clone();
            Definition::Local { definition, key_span }
        }
        Some(target) => {
            let external = ctx.overlay?.named_types.get(target)?;
            let (definition, key_span) = external.definition(&reference.name)?;
            Definition::External {
                external,
                definition: definition.clone(),
                key_span,
            }
        }
    };
    Some((reference, definition))
}

/// Hover on a reference name: the definition it selects and where that is,
/// or why the file it imports from is not a usable schema document.
pub(super) fn hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    if let Some(hover) = unusable_target_hover(ctx, offset) {
        return Some(hover);
    }
    let (reference, definition) = reference_definition(ctx, offset)?;
    let (definition, defined_in) = match &definition {
        Definition::Local { definition, .. } => (definition, "this schema".to_string()),
        Definition::External { external, definition, .. } => {
            (definition, format!("`{}`", external.target))
        }
    };
    let body = format!(
        "{}\n\nDefined in: {defined_in}",
        meta_schema_definition_hover_body(&reference.name, definition)
    );
    markup_hover(ctx, reference.name_span, body)
}

/// Hover on a reference into a file whose text is not YAML or whose envelope
/// is malformed: the parse error, which would otherwise read as a file that
/// exports nothing.
fn unusable_target_hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
    let declaration = authored_declaration(ctx, offset)?;
    let reference = declaration.reference_at(ctx.text, offset)?;
    let external = ctx.overlay?.named_types.get(reference.target.as_deref()?)?;
    let error = external.error.as_ref()?;
    let body = format!(
        "**`{}`**\n\n`{}` is not a usable schema document: {error}",
        reference.name, external.target
    );
    markup_hover(ctx, reference.name_span, body)
}

/// Go-to-definition on a reference name: the definition's key in the file
/// that defines it.
pub(super) fn definition(ctx: &DocumentContext, offset: usize) -> Option<Location> {
    let (_, definition) = reference_definition(ctx, offset)?;
    match definition {
        Definition::Local { key_span, .. } => {
            let range = ctx.source_map.byte_range_to_lsp(key_span)?;
            Some(Location::new(ctx.uri.clone(), range))
        }
        Definition::External { external, key_span, .. } => {
            let uri = file_path_to_uri(&external.path)?;
            let map = SourceMap::new(
                uri.clone(),
                0,
                ctx.source_map.encoding(),
                Arc::clone(&external.text),
            );
            Some(Location::new(uri, map.byte_range_to_lsp(key_span)?))
        }
    }
}

/// Every name an imported file exports, spelled with the target that imports
/// it (`Name@./types.yaml`).
pub(super) fn external_names(ctx: &DocumentContext) -> Vec<(String, String)> {
    let Some(overlay) = ctx.overlay else {
        return Vec::new();
    };
    overlay
        .named_types
        .externals()
        .iter()
        .filter_map(|external| Some((external, external.exports()?)))
        .flat_map(|(external, exports)| {
            exports
                .properties
                .keys()
                .map(move |name| (format!("{name}@{}", external.target), external.target.clone()))
        })
        .collect()
}

// ── Composition operands ──────────────────────────────────────────────────

/// What passive completion knows about whether a named type can be composed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Eligibility {
    /// It is, after its aliases, one inline object.
    Object,
    /// It is known not to be: a union, array, primitive, opaque object,
    /// function, or template, an alias cycle, or a cycle back to the
    /// receiving object with no child-value hop.
    Ineligible,
    /// Its terminal definition is not available without the resolver.
    Unknown,
}

/// The local namespace a composition operand at the cursor is looked up in,
/// read from the buffer without the resolver.
struct CompositionScope {
    /// Local names in authored order, each with its parsed definition, or
    /// `None` when only the name could be read.
    local: Vec<(String, Option<PropertyDef>)>,
    /// The named type whose own object receives the operand, which neither
    /// it nor anything composing it may name.
    receiver: Option<String>,
}

impl CompositionScope {
    /// `Some(None)` for a name whose definition is unknown, `None` for a
    /// name the namespace does not hold.
    fn definition(&self, name: &str) -> Option<Option<&PropertyDef>> {
        self.local
            .iter()
            .find(|(local, _)| local == name)
            .map(|(_, definition)| definition.as_ref())
    }
}

/// Completion for a composition operand: eligible named object types for its
/// name, and for the part after `@`, `this` or a file that may define it.
///
/// A name replaces the whole operand typed so far, so choosing
/// `audit@./audit.yaml` over `aud` is one edit; a reference replaces only
/// the text after `@`.
pub(super) fn operand_completions(
    ctx: &DocumentContext,
    offset: usize,
    state: &SchemaCursor,
    part: &CompositionOperandPart,
    operand_span: &Range<usize>,
) -> Vec<CompletionItem> {
    let scope = composition_scope(ctx, offset, state);
    let partial = state.token.as_str();
    let mut items = Vec::new();
    match part {
        CompositionOperandPart::Name => {
            let local = scope
                .local
                .iter()
                .filter(|(name, _)| eligibility(ctx, &scope, Hop::Local(name)) != Eligibility::Ineligible)
                .map(|(name, _)| (local_reference_spelling(name), "Named type from this schema".to_string()));
            let external = exported_candidates(ctx, &scope, None)
                .into_iter()
                .map(|(external, name)| {
                    (format!("{name}@{}", external.target), format!("Named type from `{}`", external.target))
                });
            for (label, detail) in local.chain(external) {
                if label.starts_with(partial)
                    && let Some(completion) =
                        item(ctx, operand_span.start, offset, &label, &label, CompletionItemKind::REFERENCE, Some(detail))
                {
                    items.push(completion);
                }
            }
        }
        CompositionOperandPart::Reference { name } => {
            let start = state.token_span.start;
            let mut targets = Vec::new();
            if scope.definition(name).is_some()
                && eligibility(ctx, &scope, Hop::Local(name)) != Eligibility::Ineligible
            {
                targets.push((LOCAL_REFERENCE.to_string(), CompletionItemKind::REFERENCE, "This schema"));
            }
            targets.extend(
                exported_candidates(ctx, &scope, Some(name))
                    .into_iter()
                    .map(|(external, _)| (external.target.clone(), CompletionItemKind::FILE, "Defines this type")),
            );
            for (label, kind, detail) in targets {
                if label.starts_with(partial)
                    && let Some(completion) = item(ctx, start, offset, &label, &label, kind, Some(detail.to_string()))
                {
                    items.push(completion);
                }
            }
            // A file the catalog has not read may still define the name.
            let typed = ctx.text.get(start..offset).unwrap_or_default();
            items.extend(file_path_completions(ctx, offset, typed));
        }
    }
    items
}

/// Each `(file, name)` an imported file exports, restricted to `only`, that
/// is not known to be ineligible.
fn exported_candidates<'c>(
    ctx: &DocumentContext<'c>,
    scope: &CompositionScope,
    only: Option<&str>,
) -> Vec<(&'c ExternalNamespace, String)> {
    let Some(overlay) = ctx.overlay else {
        return Vec::new();
    };
    overlay
        .named_types
        .externals()
        .iter()
        .filter_map(|external| Some((external, external.exports()?)))
        .flat_map(|(external, exports)| exports.properties.keys().map(move |name| (external, name)))
        .filter(|(_, name)| only.is_none_or(|only| only == name.as_str()))
        .filter(|(external, name)| eligibility(ctx, scope, Hop::External(external, name)) != Eligibility::Ineligible)
        .map(|(external, name)| (external, name.clone()))
        .collect()
}

/// One step of an alias chain: a name in the local namespace or in an
/// imported file's exports.
#[derive(Clone, Copy)]
enum Hop<'a> {
    Local(&'a str),
    External(&'a ExternalNamespace, &'a str),
}

impl PartialEq for Hop<'_> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Local(a), Self::Local(b)) => a == b,
            (Self::External(a, x), Self::External(b, y)) => std::ptr::eq(*a, *b) && x == y,
            _ => false,
        }
    }
}

/// Whether the named type at `hop` can be composed, following its single
/// non-array alias hops through the namespaces already in hand.
///
/// An alias into a file named from another file is relative to that file,
/// which the catalog is not keyed by, so it is unknown rather than guessed.
fn eligibility<'a>(ctx: &DocumentContext<'a>, scope: &'a CompositionScope, mut hop: Hop<'a>) -> Eligibility {
    let catalog = ctx.overlay.map(|overlay| &*overlay.named_types);
    let mut visited: Vec<Hop<'a>> = Vec::new();
    loop {
        if visited.contains(&hop) {
            return Eligibility::Ineligible;
        }
        visited.push(hop);
        let definition = match hop {
            Hop::Local(name) => {
                if scope.receiver.as_deref() == Some(name) {
                    return Eligibility::Ineligible;
                }
                scope.definition(name).flatten()
            }
            Hop::External(external, name) => external.exports().and_then(|exports| exports.properties.get(name)),
        };
        let Some(definition) = definition else {
            return Eligibility::Unknown;
        };
        let PropertyDef::Single(atom) = definition else {
            return Eligibility::Ineligible;
        };
        if atom.is_array {
            return Eligibility::Ineligible;
        }
        match &atom.ty {
            TypeExpr::InlineObject(shape) => {
                let cyclic = matches!(hop, Hop::Local(_))
                    && scope.receiver.as_deref().is_some_and(|receiver| {
                        composes(scope, shape, receiver, &mut BTreeSet::new())
                    });
                return if cyclic { Eligibility::Ineligible } else { Eligibility::Object };
            }
            TypeExpr::Imported { name, reference } => {
                let local = reference.trim() == LOCAL_REFERENCE;
                hop = match (hop, local) {
                    (Hop::Local(_), true) => Hop::Local(name),
                    (Hop::External(external, _), true) => Hop::External(external, name),
                    (Hop::Local(_), false) => match catalog.and_then(|catalog| catalog.get(reference)) {
                        Some(external) => Hop::External(external, name),
                        None => return Eligibility::Unknown,
                    },
                    (Hop::External(..), false) => return Eligibility::Unknown,
                };
            }
            _ => return Eligibility::Ineligible,
        }
    }
}

/// Whether `shape`'s local composition operands reach `receiver` through
/// aliases and further composition alone.
fn composes(scope: &CompositionScope, shape: &SchemaShape, receiver: &str, visited: &mut BTreeSet<String>) -> bool {
    let Some(composition) = shape.composition.as_deref() else {
        return false;
    };
    composition
        .operands()
        .into_iter()
        .filter(|operand| operand.resolved_reference().trim() == LOCAL_REFERENCE)
        .any(|operand| reaches(scope, &operand.name, receiver, visited))
}

fn reaches(scope: &CompositionScope, name: &str, receiver: &str, visited: &mut BTreeSet<String>) -> bool {
    if name == receiver {
        return true;
    }
    if !visited.insert(name.to_string()) {
        return false;
    }
    let Some(Some(PropertyDef::Single(atom))) = scope.definition(name) else {
        return false;
    };
    match &atom.ty {
        _ if atom.is_array => false,
        TypeExpr::Imported { name, reference } if reference.trim() == LOCAL_REFERENCE => {
            reaches(scope, name, receiver, visited)
        }
        TypeExpr::InlineObject(shape) => composes(scope, shape, receiver, visited),
        _ => false,
    }
}

/// The local namespace for an operand at `offset`, and the type receiving it.
///
/// The current buffer's declaration is used when it parses. Otherwise each
/// entry is parsed on its own from the buffer with the cursor's lines blanked,
/// since they hold the operand being typed; and when even that is not
/// YAML, only the names the tolerant passive reading finds are offered.
/// For an implicit root union, only the arm holding the cursor is local.
fn composition_scope(ctx: &DocumentContext, offset: usize, state: &SchemaCursor) -> CompositionScope {
    let (local, types_table) = fresh_namespace(ctx, offset)
        .or_else(|| tolerant_namespace(ctx, offset))
        .unwrap_or_else(|| {
            let names = super::passive_namespace_names(ctx).into_iter().map(|name| (name, None)).collect();
            (names, standalone_types_table(ctx))
        });
    let receiver = receiving_object_path(ctx, offset, state).and_then(|path| {
        let table = if types_table { "types" } else { "$schema" };
        match path.as_slice() {
            [first, name] if first == table => Some(name.clone()),
            _ => None,
        }
    });
    CompositionScope { local, receiver }
}

/// The key path from the document root to the object that receives the
/// operand, or `None` when a child property lies between them (a spread in
/// a nested inline object), which no receiver cycle can cross.
fn receiving_object_path(ctx: &DocumentContext, offset: usize, state: &SchemaCursor) -> Option<Vec<String>> {
    let segments = state.path.segments();
    let spread = matches!(segments.last(), Some(SchemaSourcePathSegment::Spread(_)));
    if segments.iter().any(|segment| matches!(segment, SchemaSourcePathSegment::Property(_))) {
        return None;
    }
    let mut path = super::authoring_key_path(ctx, offset);
    if !spread && path.last().is_some_and(|key| key == super::EXTENDS_KEY) {
        path.pop();
    }
    Some(path)
}

fn standalone_types_table(ctx: &DocumentContext) -> bool {
    matches!(
        ctx.overlay.map(|overlay| &overlay.schema_authoring),
        Some(SchemaAuthoringState::Standalone { model: Some(model), .. }) if names_types_table(model)
    )
}

/// Whether a standalone document's local namespace is its `types` table:
/// beside an exported `$schema`, or as the payload of a `types`-only
/// document, whose declaration is that table.
fn names_types_table(model: &simplified_schema::StandaloneSchemaDocument) -> bool {
    model.local_types.is_some() || !model.exports_schema
}

type LocalNames = (Vec<(String, Option<PropertyDef>)>, bool);

/// The namespace of the declaration the current buffer authors, and whether
/// it is an explicit `types` table.
fn fresh_namespace(ctx: &DocumentContext, offset: usize) -> Option<LocalNames> {
    let overlay = ctx.overlay?;
    let mut types_only = false;
    let declaration = match &overlay.schema_authoring {
        SchemaAuthoringState::Standalone { model: Some(model), stale: false, .. } => {
            types_only = names_types_table(model);
            AuthoredDeclaration::standalone(model)
        }
        SchemaAuthoringState::Standalone { .. } => return None,
        _ if overlay.stale => return None,
        _ => AuthoredDeclaration::frontmatter(ctx.text, overlay.ast.as_deref()?)?,
    };
    let arm = match &declaration.declaration {
        SchemaDeclaration::Schema(SchemaRoot::Union(arms)) => (0..arms.len()).find(|index| {
            declaration
                .source_map
                .spans(&SchemaSourcePath::root().union_arm(*index), SchemaSpanKind::UnionArm)
                .iter()
                .any(|span| span.start <= offset && offset <= span.end)
        }),
        _ => None,
    };
    let path = arm.map_or_else(SchemaSourcePath::root, |index| SchemaSourcePath::root().union_arm(index));
    let (shape, root) = declaration.local_namespace(&path)?;
    let types_table = types_only || root == SchemaSourcePath::root().property("types");
    let names = shape
        .properties
        .iter()
        .map(|(name, definition)| (name.clone(), Some(definition.clone())))
        .collect();
    Some((names, types_table))
}

/// The namespace read entry by entry from the buffer with the cursor's
/// lines blanked, and whether it is an explicit `types` table.
///
/// The blanked lines run from the one the cursor's value starts on, so that
/// a flow sequence or quoted scalar continued onto the cursor's line does
/// not leave its opening half behind unclosed.
fn tolerant_namespace(ctx: &DocumentContext, offset: usize) -> Option<LocalNames> {
    let overlay = ctx.overlay?;
    let from = super::continued_value_anchor(ctx, offset).unwrap_or(offset);
    let blanked = blank_lines(ctx.text, from, offset);
    let standalone = matches!(overlay.schema_authoring, SchemaAuthoringState::Standalone { .. });
    let source = if standalone { blanked.as_str() } else { frontmatter_body(&blanked) };
    let yaml: serde_yaml_ng::Value = serde_yaml_ng::from_str(source).ok()?;
    let types = standalone.then(|| yaml.get("types").filter(|types| types.is_mapping())).flatten();
    let types_table = types.is_some();
    let container = match types {
        Some(types) => types,
        None => yaml.get("$schema")?,
    };
    let mapping = match container {
        serde_yaml_ng::Value::Mapping(mapping) => mapping,
        serde_yaml_ng::Value::Sequence(arms) => arms.get(block_arm_index(ctx.text, offset)?)?.as_mapping()?,
        _ => return None,
    };
    let names = mapping
        .iter()
        .filter_map(|(key, value)| Some((key.as_str()?, value)))
        // Patterns and `$constraints` are never names (a `types` table
        // refuses them); `$extends` is a name only in a `types` table.
        .filter(|(key, _)| !(key.starts_with('<') || *key == "$constraints" || (!types_table && *key == "$extends")))
        .map(|(key, value)| (key.to_string(), parse_property_definition(key, value).ok()))
        .collect();
    Some((names, types_table))
}

/// `text` with the lines from the one holding `from` through the one holding
/// `offset` replaced by spaces, keeping every byte offset and line break.
fn blank_lines(text: &str, from: usize, offset: usize) -> String {
    let start = text[..from].rfind('\n').map_or(0, |index| index + 1);
    let end = text[offset..].find('\n').map_or(text.len(), |index| offset + index);
    let mut blanked = String::with_capacity(text.len());
    blanked.push_str(&text[..start]);
    blanked.extend(text[start..end].bytes().map(|byte| if byte == b'\n' { '\n' } else { ' ' }));
    blanked.push_str(&text[end..]);
    blanked
}

/// The YAML between a Markdown document's frontmatter fences, or all of
/// `text` when it does not open with one.
fn frontmatter_body(text: &str) -> &str {
    let Some(rest) = text
        .strip_prefix("---")
        .and_then(|rest| rest.strip_prefix('\n').or_else(|| rest.strip_prefix("\r\n")))
    else {
        return text;
    };
    let body_start = text.len() - rest.len();
    let mut line_start = body_start;
    for line in rest.split_inclusive('\n') {
        if matches!(line.trim_end_matches(['\n', '\r']), "---" | "...") {
            return &text[body_start..line_start];
        }
        line_start += line.len();
    }
    &text[body_start..]
}

/// The index of the block-sequence arm of a top-level `$schema` that holds
/// the line at `offset`, or `None` outside one.
fn block_arm_index(text: &str, offset: usize) -> Option<usize> {
    let mut in_schema = false;
    let mut item_indent = None;
    let mut index: Option<usize> = None;
    let mut line_start = 0;
    for line in text.split_inclusive('\n') {
        if line_start > offset {
            break;
        }
        line_start += line.len();
        let content = line.trim_end_matches(['\n', '\r']);
        let trimmed = content.trim_start();
        let indent = content.len() - trimmed.len();
        let is_item = trimmed == "-" || trimmed.starts_with("- ");
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if indent == 0 && !is_item {
            in_schema = mapping_separator(trimmed, false)
                .is_some_and(|colon| super::yaml_key(&trimmed[..colon]) == "$schema" && trimmed[colon + 1..].trim().is_empty());
            item_indent = None;
            index = None;
        } else if in_schema && is_item && item_indent.is_none_or(|column| column == indent) {
            item_indent = Some(indent);
            index = Some(index.map_or(0, |index| index + 1));
        }
    }
    if in_schema { index } else { None }
}
