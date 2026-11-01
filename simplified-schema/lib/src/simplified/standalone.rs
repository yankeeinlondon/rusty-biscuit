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
//! - **Pure** (informal): a YAML mapping without `kind: schema` that has
//!   `$schema` and whose only other keys are an optional `types` mapping, an
//!   optional string `description`, and an optional `categories` list (see
//!   [`StandaloneSchemaDocument::categories`]).
//! - **Tagged**: a YAML mapping containing `kind: schema` and at least one of
//!   an exported `$schema` and a `types` mapping, plus the same optional
//!   `description` and `categories`.
//!
//! [`StandaloneSchemaEnvelope::admits_top_level_key`] is the one definition
//! of each envelope's key set.
//!
//! Only a document with a `$schema` property exports document properties
//! ([`StandaloneSchemaDocument::exports_schema`]); a tagged `types`-only
//! document is a valid envelope that serves named-type imports alone.
//!
//! A separate `types` mapping supplies local names and named imports without
//! adding document properties. Otherwise mapping payload entries serve both
//! roles. A sequence export is a root-level schema union for whole-file use,
//! and a scalar export is a whole-file schema
//! reference. Every pure payload is parsed by the shared
//! [`parse_schema_declaration`](super::parse_schema_declaration) authority —
//! the same parser that classifies an inline `$schema` value — so reference
//! arms are syntax-checked and remote references rejected without I/O. A
//! tagged `types` payload must remain a mapping.
//!
//! Once an envelope is recognized, a malformed payload is a
//! [`SchemaError::SchemaDocument`] — the library never silently reinterprets
//! the document as ordinary YAML or raw JSON Schema. Text that is not YAML at
//! all is a [`SchemaError::SchemaDocument`] too, never a silent decline. A
//! payload or `types` definition the grammar rejects keeps the parser's typed
//! error as the `source` and its byte range in `source` text as the `span`.
//!
//! ## Side-effect freedom
//!
//! Classification, parsing, and suggestion linting perform no file reads,
//! shell execution, composition directive evaluation, or network access. The
//! caller supplies the source string; all work is in-memory.

use std::path::{Path, PathBuf};

use indexmap::IndexMap;
use serde_yaml_ng::Value as YamlValue;

use crate::errors::SchemaError;

use super::{
    Constraint, PropertyAtom, PropertyDef, SchemaDeclaration, SchemaShape, SchemaSourceMap, SchemaSourcePath,
    SchemaRoot, SuggestionLintProblem, TypeExpr, lint_suggestions,
    signature::duplicate_overload, source::parse_standalone_schema_payload_with_source,
};

/// The content envelope used by a standalone SimplifiedSchema document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandaloneSchemaEnvelope {
    /// A document with `$schema`, an optional reusable `types` mapping, and
    /// optional `description` and `categories` metadata.
    Pure,
    /// A document containing `kind: schema`, `types`, and an optional export.
    Tagged,
}

impl StandaloneSchemaEnvelope {
    /// Whether this envelope admits `key` at the top level of its mapping.
    ///
    /// Both envelopes admit `$schema`, `types`, `description`, and
    /// `categories`; only [`Tagged`](Self::Tagged) admits `kind`. This is the
    /// one definition of each envelope's key set: recognition, `$schema`
    /// target classification, and editor-side lexical claims all read it.
    pub fn admits_top_level_key(self, key: &str) -> bool {
        matches!(key, "$schema" | "types" | "description" | "categories")
            || (self == Self::Tagged && key == "kind")
    }
}

/// One entry of a schema document's `categories:` list, written
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
    /// Whether the document has a `$schema` property, and so can be the
    /// target of a document's `$schema`. A `types`-only document serves
    /// named-type imports only.
    pub exports_schema: bool,
    /// The top-level `description` string, `None` when absent. An empty
    /// string is kept as `Some("")`.
    pub description: Option<String>,
    /// The shared declaration product for the envelope payload: an inline
    /// schema (mapping or root-union sequence) or a syntax-checked whole-file
    /// reference. Without an explicit export, a tagged `types` table is the payload.
    pub declaration: SchemaDeclaration,
    /// Structural authored spans for the parsed schema payload.
    pub source_map: SchemaSourceMap,
    /// Reusable definitions declared beside an exported `$schema`. These
    /// supply its local namespace without becoming document properties.
    pub local_types: Option<SchemaShape>,
    /// Authored spans of `local_types`, relative to that mapping's root.
    pub local_type_source_map: Option<SchemaSourceMap>,
    /// Invalid advisory suggestion metadata in declaration order.
    ///
    /// These are lint problems, not errors — the schema is still usable for
    /// validation and completion. See [`SuggestionLintProblem`].
    pub suggestion_lints: Vec<SuggestionLintProblem>,
    /// The envelope's `categories:` list in authored order, or `None`
    /// when the document declares none. A declared list (even `[]`) limits
    /// every `category(...)` in `types` to its names; without one any name is
    /// accepted.
    pub categories: Option<Vec<SchemaCategory>>,
}

impl StandaloneSchemaDocument {
    /// The namespace available to local references and named imports. An
    /// explicit `types` table is independent of the exported properties.
    pub fn named_types(&self) -> Option<&SchemaShape> {
        self.local_types.as_ref().or_else(|| match self.schema()? {
            SchemaRoot::Single(shape) => Some(shape),
            SchemaRoot::Union(_) => None,
        })
    }

    /// Authored spans of the table returned by [`Self::named_types`].
    pub fn named_type_source_map(&self) -> &SchemaSourceMap {
        self.local_type_source_map.as_ref().unwrap_or(&self.source_map)
    }

    /// Source locations for resolution, keeping exported properties and
    /// local types distinct when they share a name.
    pub fn resolution_source_map(&self) -> SchemaSourceMap {
        let Some(types) = &self.local_type_source_map else {
            return self.source_map.clone();
        };
        let mut map = SchemaSourceMap::default();
        map.extend_prefixed(&self.source_map, &SchemaSourcePath::root().property("$schema"));
        map.extend_prefixed(types, &SchemaSourcePath::root().property("types"));
        map
    }

    /// Borrows the parsed inline schema when the payload is a mapping or a
    /// root-union sequence; `None` when the payload is a whole-file reference.
    pub fn schema(&self) -> Option<&SchemaRoot> {
        match &self.declaration {
            SchemaDeclaration::Schema(schema) => Some(schema),
            SchemaDeclaration::Reference(_) => None,
        }
    }
}

/// Classifies and parses a standalone YAML SimplifiedSchema document.
///
/// Returns `Ok(None)` for ordinary YAML and raw JSON Schema documents
/// (including an untagged mapping with `$schema` and any key outside
/// `$schema`, `types`, `description`, and `categories`), and an error for text
/// that is not valid YAML. Once `kind: schema` claims a tagged document, or
/// `$schema` with optional `types`, `description`, and `categories` claims an
/// untagged document, malformed
/// envelope content is returned as a [`SchemaError::SchemaDocument`] instead
/// of falling back to raw JSON Schema.
///
/// Every pure payload — mapping, root-union sequence, or whole-file reference
/// scalar — is parsed by the shared
/// [`parse_schema_declaration`](super::parse_schema_declaration) authority, so
/// reference arms are syntax-checked and HTTP(S) references rejected without
/// I/O. A tagged `types` payload must be a mapping.
///
/// ## Errors versus lint
///
/// Text that is not valid YAML, and a malformed recognized envelope (a
/// tagged document with neither `$schema` nor `types`, a malformed `types`,
/// a `description` that is not a string, unsupported tagged-envelope keys, a
/// malformed `categories` list or an undeclared category, two overloads with
/// one binding contract in `types`, unparseable SimplifiedSchema payload,
/// invalid or remote reference) returns `Err(SchemaError::SchemaDocument)`.
/// Invalid `suggest(...)` candidates
/// within a valid envelope are returned as
/// [`StandaloneSchemaDocument::suggestion_lints`] — they are lint data, not
/// errors.
///
/// A payload or `types` definition the parser rejects keeps that typed error
/// (usually [`SchemaError::Grammar`], whose own span is relative to the type
/// string) as the `SchemaDocument` `source`, and its byte range in `source`
/// as the `span`. A failure to lint a valid payload keeps its typed cause
/// without a span.
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
    let Some(recognized) =
        recognize_standalone(source).map_err(|message| schema_document_error(path, message))?
    else {
        return Ok(None);
    };
    let (envelope, payload_key) = (recognized.envelope, recognized.payload_key());
    let map = recognized.mapping();
    let payload = recognized.payload();
    let description = map.get("description").and_then(YamlValue::as_str).map(str::to_string);
    let categories_key = YamlValue::String("categories".into());
    let mut parsed = parse_standalone_schema_payload_with_source(payload, source, payload_key)
        .map_err(|failure| failure.into_document_error(path, "invalid SimplifiedSchema payload"))?;
    let local = if payload_key == "$schema" {
        map.get("types")
            .map(|value| parse_standalone_schema_payload_with_source(value, source, "types"))
            .transpose()
            .map_err(|failure| failure.into_document_error(path, "invalid named types"))?
    } else {
        None
    };
    let (local_types, local_type_source_map) = match local {
        Some(parsed) => match parsed.value {
            SchemaDeclaration::Schema(SchemaRoot::Single(shape)) => (Some(shape), Some(parsed.source_map)),
            _ => return Err(schema_document_error(path, "`types` must be a mapping")),
        },
        None => (None, None),
    };
    let categories = map
        .get(&categories_key)
        .map(|value| parse_categories(path, value))
        .transpose()?;
    // Only a `types` table is checked as one; `$schema` entries are document
    // properties, never overloads or categorized types.
    if let SchemaDeclaration::Schema(SchemaRoot::Single(shape)) = &parsed.value
        && payload_key == "types"
    {
        check_named_types(path, shape, categories.as_deref())?;
    }
    if let Some(shape) = &local_types {
        check_named_types(path, shape, categories.as_deref())?;
    }
    let mut suggestion_lints = match &mut parsed.value {
        SchemaDeclaration::Schema(schema) => {
            lint_suggestions(schema).map_err(|error| SchemaError::SchemaDocument {
                path: path.to_path_buf(),
                message: format!("could not lint SimplifiedSchema payload: {error}"),
                source: Some(Box::new(error)),
                span: None,
            })?
        }
        // A whole-file reference carries no suggestion metadata to lint.
        SchemaDeclaration::Reference(_) => Vec::new(),
    };
    if let Some(shape) = &local_types {
        suggestion_lints.extend(lint_suggestions(&SchemaRoot::Single(shape.clone()))?);
    }
    Ok(Some(StandaloneSchemaDocument {
        path: path.to_path_buf(),
        envelope,
        exports_schema: payload_key == "$schema",
        description,
        declaration: parsed.value,
        source_map: parsed.source_map,
        local_types,
        local_type_source_map,
        suggestion_lints,
        categories,
    }))
}

/// A YAML document whose content claims a standalone schema envelope.
pub(super) struct RecognizedStandalone {
    /// The whole document, a mapping.
    document: YamlValue,
    envelope: StandaloneSchemaEnvelope,
}

impl RecognizedStandalone {
    /// The envelope key holding the payload.
    pub(super) fn payload_key(&self) -> &'static str {
        if self.mapping().contains_key("$schema") { "$schema" } else { "types" }
    }

    fn mapping(&self) -> &serde_yaml_ng::Mapping {
        self.document.as_mapping().expect("recognition requires a mapping document")
    }

    /// The payload value; recognition checked that the key is present.
    pub(super) fn payload(&self) -> &YamlValue {
        self.mapping()
            .get(self.payload_key())
            .expect("recognition checked the payload key")
    }

    pub(super) fn local_types(&self) -> Option<&YamlValue> {
        (self.payload_key() == "$schema").then(|| self.mapping().get("types")).flatten()
    }
}

/// Loads `source` with admitted numbers and decides which envelope, if any,
/// claims it.
///
/// `Ok(None)` is an ordinary document (not a mapping, or no envelope claims
/// it). `Err` carries the message of an outer failure: text that is not YAML,
/// a malformed tagged envelope, or a recognized envelope whose `types` is not
/// a mapping or whose `description` is not a string. The payload itself is
/// not parsed here.
pub(super) fn recognize_standalone(source: &str) -> Result<Option<RecognizedStandalone>, String> {
    // Numbers are admitted here too, so a schema file's numbers follow the
    // same rules whichever loader reads the file first.
    let document: YamlValue = crate::numeric::load_admitted_yaml(source).map_err(|error| {
        if crate::numeric::is_yaml_admission_error(&error) {
            error.to_string()
        } else {
            format!("not valid YAML: {error}")
        }
    })?;
    let Some(map) = document.as_mapping() else {
        return Ok(None);
    };

    let types_key = YamlValue::String("types".into());
    let schema_key = YamlValue::String("$schema".into());
    let tagged = map.get("kind").and_then(YamlValue::as_str) == Some("schema");

    let envelope = if tagged {
        let unsupported: Vec<String> = map
            .keys()
            .filter(|key| {
                !key.as_str()
                    .is_some_and(|key| StandaloneSchemaEnvelope::Tagged.admits_top_level_key(key))
            })
            .map(|key| key.as_str().map_or_else(|| format!("{key:?}"), str::to_string))
            .collect();
        if !unsupported.is_empty() {
            return Err(format!(
                "tagged schema documents support only `kind`, `types`, `$schema`, `description`, and \
                 `categories`; found unsupported keys: {}",
                unsupported.join(", ")
            ));
        }
        if !map.contains_key(&schema_key) && !map.contains_key(&types_key) {
            return Err("tagged schema document needs a `$schema` or `types` property".into());
        }
        if map.get(&types_key).is_some_and(|types| !matches!(types, YamlValue::Mapping(_))) {
            return Err("tagged schema document `types` must be a mapping".into());
        }
        StandaloneSchemaEnvelope::Tagged
    } else if map.contains_key(&schema_key)
        && map
            .keys()
            .all(|key| key.as_str().is_some_and(|key| StandaloneSchemaEnvelope::Pure.admits_top_level_key(key)))
    {
        if map.get(&types_key).is_some_and(|types| !matches!(types, YamlValue::Mapping(_))) {
            return Err("`types` must be a mapping".into());
        }
        StandaloneSchemaEnvelope::Pure
    } else {
        return Ok(None);
    };
    if map.get("description").is_some_and(|description| !matches!(description, YamlValue::String(_))) {
        return Err("`description` must be a string".into());
    }
    Ok(Some(RecognizedStandalone { document, envelope }))
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
pub(crate) fn visit_def(def: &PropertyDef, visit: &mut impl FnMut(&PropertyAtom)) {
    match def {
        PropertyDef::Single(atom) => visit_atom(atom, visit),
        PropertyDef::Union(arms) => arms.iter().for_each(|atom| visit_atom(atom, visit)),
    }
}

pub(crate) fn visit_atom(atom: &PropertyAtom, visit: &mut impl FnMut(&PropertyAtom)) {
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
        TypeExpr::Primitive(_) | TypeExpr::Imported { .. } | TypeExpr::Ref(_) => {}
    }
}

fn schema_document_error(path: &Path, message: impl Into<String>) -> SchemaError {
    SchemaError::SchemaDocument {
        path: path.to_path_buf(),
        message: message.into(),
        source: None,
        span: None,
    }
}
