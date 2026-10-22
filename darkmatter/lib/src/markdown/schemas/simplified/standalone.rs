//! Content-based standalone SimplifiedSchema document recognition.
//!
//! The public entry point [`parse_standalone_schema_document`] classifies a
//! YAML source string as a standalone SimplifiedSchema authoring document by
//! its **content** — not by filename, glob, or consumer discovery.
//!
//! ## Recognition
//!
//! Two envelopes are recognized:
//!
//! - **Pure**: a YAML mapping whose sole top-level key is `$schema`.
//! - **Tagged**: a YAML mapping containing `kind: schema`, a `types`
//!   mapping, and optionally a `categories` list (see
//!   [`StandaloneSchemaDocument::categories`]).
//!
//! Both mapping envelopes behave identically for `Name@fileref` named
//! imports. A pure sequence payload is a root-level schema union for
//! whole-file use only, and a pure scalar payload is a whole-file schema
//! reference. Every pure payload is parsed by the shared
//! [`parse_schema_declaration`](super::parse_schema_declaration) authority —
//! the same parser that classifies an inline `$schema` value — so reference
//! arms are syntax-checked and remote references rejected without I/O. A
//! tagged `types` payload must remain a mapping.
//!
//! Once an envelope is recognized, a malformed payload is a
//! [`SchemaError::SchemaDocument`] — the library never silently reinterprets
//! the document as ordinary YAML or raw JSON Schema. Text that is not YAML at
//! all is a [`SchemaError::SchemaDocument`] too, never a silent decline.
//!
//! ## Side-effect freedom
//!
//! Classification, parsing, and suggestion linting perform no file reads,
//! shell execution, composition directive evaluation, or network access. The
//! caller supplies the source string; all work is in-memory.

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::{
    Constraint, PropertyAtom, PropertyDef, SchemaDeclaration, SchemaShape, SchemaSourceMap,
    SimplifiedSchema, SuggestionLintProblem, TypeExpr, lint_suggestions,
    signature::duplicate_overload, source::parse_standalone_schema_payload_with_source,
};

/// The content envelope used by a standalone SimplifiedSchema document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandaloneSchemaEnvelope {
    /// A document whose sole top-level key is `$schema`.
    Pure,
    /// A document containing `kind: schema` and a `types` mapping.
    Tagged,
}

/// One entry of a tagged schema document's `categories:` list, written
/// `Name` or `Name -> description`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaCategory {
    /// The trimmed text before `->`; case-sensitive.
    pub name: String,
    /// The trimmed text after `->`, which may span lines.
    pub description: Option<String>,
}

/// A parsed standalone SimplifiedSchema authoring document.
///
/// Produced by [`parse_standalone_schema_document`]. Carries the shared
/// [`SchemaDeclaration`] product for the envelope payload, structural source
/// map, the content envelope that claimed the document, and suggestion lint
/// problems with byte spans relative to `source`.
///
/// Candidate spans are byte ranges in the authoring source, and lint problems
/// retain those same authoring-document ranges. Parsing and linting perform no
/// file reads, shell, network, or composition operations.
#[derive(Debug, Clone, PartialEq)]
pub struct StandaloneSchemaDocument {
    /// Path of the authoring document supplied by the caller.
    pub path: PathBuf,
    /// Content envelope that claimed the document.
    pub envelope: StandaloneSchemaEnvelope,
    /// The shared declaration product for the envelope payload: an inline
    /// schema (mapping or root-union sequence) or a syntax-checked whole-file
    /// reference. A tagged `types` payload is always an inline schema.
    pub declaration: SchemaDeclaration,
    /// Structural authored spans for the parsed schema payload.
    pub source_map: SchemaSourceMap,
    /// Invalid advisory suggestion metadata in declaration order.
    ///
    /// These are lint problems, not errors — the schema is still usable for
    /// validation and completion. See [`SuggestionLintProblem`].
    pub suggestion_lints: Vec<SuggestionLintProblem>,
    /// The tagged envelope's `categories:` list in authored order, or `None`
    /// when the document declares none. A declared list (even `[]`) limits
    /// every `category(...)` in `types` to its names; without one any name is
    /// accepted.
    pub categories: Option<Vec<SchemaCategory>>,
}

impl StandaloneSchemaDocument {
    /// Borrows the parsed inline schema when the payload is a mapping or a
    /// root-union sequence; `None` when the payload is a whole-file reference.
    pub fn schema(&self) -> Option<&SimplifiedSchema> {
        match &self.declaration {
            SchemaDeclaration::Schema(schema) => Some(schema),
            SchemaDeclaration::Reference(_) => None,
        }
    }
}

/// Classifies and parses a standalone YAML SimplifiedSchema document.
///
/// Returns `Ok(None)` for ordinary YAML and raw JSON Schema documents, and an
/// error for text that is not valid YAML. Once
/// `kind: schema` claims a tagged document, or `$schema` is the sole top-level
/// key of a pure document, malformed envelope content is returned as a
/// [`SchemaError::SchemaDocument`] instead of falling back to raw JSON Schema.
///
/// Every pure payload — mapping, root-union sequence, or whole-file reference
/// scalar — is parsed by the shared
/// [`parse_schema_declaration`](super::parse_schema_declaration) authority, so
/// reference arms are syntax-checked and HTTP(S) references rejected without
/// I/O. A tagged `types` payload must be a mapping.
///
/// ## Errors versus lint
///
/// Text that is not valid YAML, and a malformed recognized envelope
/// (missing/malformed `types`, unsupported tagged-envelope keys, a malformed
/// `categories` list or an undeclared category, two overloads with one
/// binding contract, unparseable SimplifiedSchema payload, invalid or
/// remote reference) returns
/// `Err(SchemaError::SchemaDocument)`, as does a global catalog
/// ([`crate::markdown::schemas::catalog`]), which is never a document schema.
/// Invalid `suggest(...)` candidates
/// within a valid envelope are returned as
/// [`StandaloneSchemaDocument::suggestion_lints`] — they are lint data, not
/// errors.
///
/// ## Side-effect freedom
///
/// Performs no file reads, shell execution, composition directive evaluation,
/// or network access. The caller supplies `source`; all work is in-memory.
pub fn parse_standalone_schema_document(
    source: &str,
    path: impl AsRef<Path>,
) -> Result<Option<StandaloneSchemaDocument>, SchemaError> {
    let path = path.as_ref();
    // Numbers are admitted here too, so a schema file's numbers follow the
    // same rules whichever loader reads the file first.
    let value: YamlValue = crate::markdown::numeric::load_admitted_yaml(source).map_err(|error| {
        if crate::markdown::numeric::is_yaml_admission_error(&error) {
            schema_document_error(path, error.to_string())
        } else {
            schema_document_error(path, format!("not valid YAML: {error}"))
        }
    })?;
    let Some(map) = value.as_mapping() else {
        return Ok(None);
    };

    let kind_key = YamlValue::String("kind".into());
    let types_key = YamlValue::String("types".into());
    let categories_key = YamlValue::String("categories".into());
    let schema_key = YamlValue::String("$schema".into());
    let tagged = map.get(&kind_key).and_then(YamlValue::as_str) == Some("schema");

    let (envelope, payload) = if tagged {
        if matches!(map.get(&schema_key), Some(YamlValue::Mapping(_))) && !map.contains_key(&types_key) {
            return Err(schema_document_error(
                path,
                "is a global catalog (`kind: schema` with `$schema` declarations); a catalog declares \
                 globals for a host to register and never applies as a document schema",
            ));
        }
        let unsupported: Vec<String> = map
            .keys()
            .filter_map(YamlValue::as_str)
            .filter(|key| !matches!(*key, "kind" | "types" | "categories"))
            .map(str::to_string)
            .collect();
        let expected_len = 2 + usize::from(map.contains_key(&categories_key));
        if !unsupported.is_empty() || map.len() != expected_len {
            return Err(schema_document_error(
                path,
                format!(
                    "tagged schema documents support only `kind`, `types`, and `categories`; \
                     found unsupported keys: {}",
                    unsupported.join(", ")
                ),
            ));
        }
        let Some(payload) = map.get(&types_key) else {
            return Err(schema_document_error(
                path,
                "tagged schema document is missing the `types` mapping",
            ));
        };
        if !matches!(payload, YamlValue::Mapping(_)) {
            return Err(schema_document_error(
                path,
                "tagged schema document `types` must be a mapping",
            ));
        }
        (StandaloneSchemaEnvelope::Tagged, payload)
    } else if map.len() == 1 && map.contains_key(&schema_key) {
        let payload = map.get(&schema_key).expect("key presence checked above");
        (StandaloneSchemaEnvelope::Pure, payload)
    } else {
        return Ok(None);
    };

    let payload_key = match envelope {
        StandaloneSchemaEnvelope::Pure => "$schema",
        StandaloneSchemaEnvelope::Tagged => "types",
    };
    let mut parsed = parse_standalone_schema_payload_with_source(payload, source, payload_key)
        .map_err(|error| {
            schema_document_error(path, format!("invalid SimplifiedSchema payload: {error}"))
        })?;
    let categories = map
        .get(&categories_key)
        .map(|value| parse_categories(path, value))
        .transpose()?;
    if let SchemaDeclaration::Schema(SimplifiedSchema::Single(shape)) = &parsed.value
        && envelope == StandaloneSchemaEnvelope::Tagged
    {
        check_named_types(path, shape, categories.as_deref())?;
    }
    let suggestion_lints = match &mut parsed.value {
        SchemaDeclaration::Schema(schema) => {
            lint_suggestions(schema).map_err(|error| {
                schema_document_error(path, format!("could not lint SimplifiedSchema payload: {error}"))
            })?
        }
        // A whole-file reference carries no suggestion metadata to lint.
        SchemaDeclaration::Reference(_) => Vec::new(),
    };
    Ok(Some(StandaloneSchemaDocument {
        path: path.to_path_buf(),
        envelope,
        declaration: parsed.value,
        source_map: parsed.source_map,
        suggestion_lints,
        categories,
    }))
}

/// Reads a `categories:` list: each entry `Name` or `Name -> description`,
/// names unique and non-blank, order kept.
fn parse_categories(path: &Path, value: &YamlValue) -> Result<Vec<SchemaCategory>, SchemaError> {
    let YamlValue::Sequence(entries) = value else {
        return Err(schema_document_error(
            path,
            "`categories` must be a list of `Name` or `Name -> description` entries \
             (`categories: []` declares none)",
        ));
    };
    let mut categories: Vec<SchemaCategory> = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let at = |message: String| schema_document_error(path, format!("`categories[{index}]`: {message}"));
        let YamlValue::String(text) = entry else {
            return Err(at("a category entry must be a string, `Name` or `Name -> description`".into()));
        };
        let (name, description) = match text.split_once("->") {
            Some((name, description)) => {
                let description = description.trim();
                if description.is_empty() {
                    return Err(at("`->` must be followed by a description".into()));
                }
                (name.trim(), Some(description.to_string()))
            }
            None => (text.trim(), None),
        };
        if name.is_empty() {
            return Err(at("a category name cannot be blank".into()));
        }
        if let Some(first) = categories.iter().position(|category| category.name == name) {
            return Err(at(format!(
                "duplicate category `{name}` (first declared at `categories[{first}]`)"
            )));
        }
        categories.push(SchemaCategory {
            name: name.to_string(),
            description,
        });
    }
    Ok(categories)
}

/// The checks that need a whole `types` mapping: every `category(...)` names
/// a declared category (when a list is declared), and no overload set has two
/// options with one binding contract once local references are resolved.
fn check_named_types(
    path: &Path,
    types: &SchemaShape,
    categories: Option<&[SchemaCategory]>,
) -> Result<(), SchemaError> {
    let named: &IndexMap<String, PropertyDef> = &types.properties;
    for (name, def) in named {
        if let PropertyDef::Union(arms) = def
            && let Some(message) = duplicate_overload(arms, Some(named))
        {
            return Err(schema_document_error(path, format!("`types.{name}`: {message}")));
        }
        if let Some(categories) = categories {
            let mut undeclared = None;
            visit_def(def, &mut |atom| {
                for constraint in atom.constraints.iter().chain(&atom.array_constraints) {
                    if let Constraint::Category(category) = constraint
                        && undeclared.is_none()
                        && !categories.iter().any(|declared| &declared.name == category)
                    {
                        undeclared = Some(category.clone());
                    }
                }
            });
            if let Some(category) = undeclared {
                return Err(schema_document_error(
                    path,
                    format!("`types.{name}`: category `{category}` is not declared in `categories:`"),
                ));
            }
        }
    }
    Ok(())
}

/// Calls `visit` with every atom of `def`, including those nested in inline
/// objects, unions, tuples, and function signatures.
fn visit_def(def: &PropertyDef, visit: &mut impl FnMut(&PropertyAtom)) {
    match def {
        PropertyDef::Single(atom) => visit_atom(atom, visit),
        PropertyDef::Union(arms) => arms.iter().for_each(|atom| visit_atom(atom, visit)),
    }
}

fn visit_atom(atom: &PropertyAtom, visit: &mut impl FnMut(&PropertyAtom)) {
    visit(atom);
    match &atom.ty {
        TypeExpr::InlineObject(shape) => shape
            .properties
            .values()
            .chain(shape.pattern_keys.iter().map(|pattern| &pattern.def))
            .for_each(|def| visit_def(def, visit)),
        TypeExpr::Union(arms) => arms.iter().for_each(|atom| visit_atom(atom, visit)),
        TypeExpr::Tuple(tuple) => {
            tuple.elements.iter().for_each(|element| visit_def(&element.def, visit));
            if let Some(spread) = &tuple.spread {
                visit_atom(spread, visit);
            }
        }
        TypeExpr::Function(function) => {
            function
                .parameters
                .as_slice()
                .iter()
                .for_each(|parameter| visit_def(&parameter.def, visit));
            visit_def(&function.returns, visit);
        }
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } => {}
    }
}

fn schema_document_error(path: &Path, message: impl Into<String>) -> SchemaError {
    SchemaError::SchemaDocument {
        path: path.to_path_buf(),
        message: message.into(),
    }
}
