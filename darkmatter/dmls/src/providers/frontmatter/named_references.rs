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

use darkmatter::markdown::schemas::{PropertyDef, SchemaSpanKind, SimplifiedType};
use lsp_types::{Hover, Location};

use super::{markup_hover, meta_schema_definition_hover_body};
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
/// `None` when it has none or the current buffer did not parse.
fn authored_declaration(ctx: &DocumentContext) -> Option<AuthoredDeclaration> {
    let overlay = ctx.overlay?;
    match &overlay.schema_authoring {
        SchemaAuthoringState::Standalone { model: Some(model), stale: false, .. } => {
            Some(AuthoredDeclaration::standalone(model))
        }
        SchemaAuthoringState::Standalone { .. } => None,
        _ if overlay.stale => None,
        _ => AuthoredDeclaration::frontmatter(ctx.text, overlay.ast.as_deref()?),
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
    let declaration = authored_declaration(ctx)?;
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

/// Hover on a reference name: the definition it selects and where that is.
pub(super) fn hover(ctx: &DocumentContext, offset: usize) -> Option<Hover> {
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
