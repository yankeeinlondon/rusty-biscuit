//! Content-based standalone SimplifiedSchema document recognition.
//!
//! The public entry point [`parse_standalone_schema_document`] classifies a
//! YAML source string as a standalone SimplifiedSchema authoring document by
//! its **content** — not by filename, glob, or consumer discovery.
//!
//! ## Recognition
//!
//! Three envelopes are recognized, by content alone:
//!
//! - **Tagged**: a YAML mapping containing `kind: schema`.
//! - **YamlSchema**: a YAML mapping containing `kind: yaml-schema`.
//! - **Pure** (informal): any other YAML mapping that has `$schema` and only
//!   keys [`StandaloneSchemaEnvelope::admits_top_level_key`] admits for it.
//!
//! ## Admission
//!
//! A recognized envelope is then **admitted** by validating it against its
//! definition, compiled into the library ([`crate::envelope`]): a tagged or
//! informal document against `schema.yaml`, a `yaml-schema` against
//! `yaml-schema.yaml`. The definitions own every structural rule (closed key
//! set, `$schema` or `types` present, `types` a mapping, metadata types,
//! no explicit `null`); this module keeps only the rules that need the parsed
//! document: category entry grammar, duplicate and undeclared categories,
//! and duplicate overloads. Metadata is read from the admitted, softly
//! converted copy, so a `description: 123` reads as `"123"`.
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
//! arms are syntax-checked and remote references rejected without I/O.
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
    /// A document containing `kind: yaml-schema`: the definition of a YAML
    /// document kind. It always exports `$schema` and carries the same
    /// metadata and `types` namespace as [`Tagged`](Self::Tagged), but it is
    /// applied to documents by their `kind`, never referenced.
    YamlSchema,
}

impl StandaloneSchemaEnvelope {
    /// Whether this envelope admits `key` at the top level of its mapping.
    ///
    /// Every envelope admits `$schema`, `types`, `description`, and
    /// `categories`; only [`Tagged`](Self::Tagged) and
    /// [`YamlSchema`](Self::YamlSchema) admit `kind`. Recognition and editor
    /// lexical claims read it to decide whether an untagged mapping is the
    /// informal envelope; a recognized envelope's keys are then judged by
    /// its definition ([`crate::envelope`]).
    pub fn admits_top_level_key(self, key: &str) -> bool {
        matches!(key, "$schema" | "types" | "description" | "categories")
            || (self != Self::Pure && key == "kind")
    }

    /// The `kind` value that claims this envelope, `None` for the informal
    /// [`Pure`](Self::Pure) envelope.
    pub fn kind(self) -> Option<&'static str> {
        match self {
            Self::Pure => None,
            Self::Tagged => Some("schema"),
            Self::YamlSchema => Some("yaml-schema"),
        }
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

/// Classifies, admits, and parses a standalone YAML SimplifiedSchema document.
///
/// Returns `Ok(None)` for ordinary YAML and raw JSON Schema documents
/// (including an untagged mapping with `$schema` and any key outside
/// `$schema`, `types`, `description`, and `categories`), and an error for text
/// that is not valid YAML. Once `kind: schema`, `kind: yaml-schema`, or the
/// informal key set claims a document, it is admitted against its definition
/// ([`crate::envelope`]) and malformed envelope content is returned as a
/// [`SchemaError::SchemaDocument`] instead of falling back to raw JSON Schema.
///
/// Every payload — mapping, root-union sequence, or whole-file reference
/// scalar — is parsed by the shared
/// [`parse_schema_declaration`](super::parse_schema_declaration) authority, so
/// reference arms are syntax-checked and HTTP(S) references rejected without
/// I/O.
///
/// ## Errors versus lint
///
/// Text that is not valid YAML, an envelope its definition rejects (located
/// at the definition's first problem), a malformed `categories` entry or an
/// undeclared category, two overloads with one binding contract in `types`,
/// an unparseable SimplifiedSchema payload, and an invalid or remote
/// reference return `Err(SchemaError::SchemaDocument)`.
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
    parse_standalone(source, path.as_ref(), Admission::Required)
}

/// [`parse_standalone_schema_document`] without admission: only the library's
/// own envelope definitions are read this way, to bootstrap the definitions
/// every other document is admitted against.
pub(crate) fn parse_standalone_unadmitted(source: &str, path: &Path) -> Result<Option<StandaloneSchemaDocument>, SchemaError> {
    parse_standalone(source, path, Admission::Skip)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Admission {
    Required,
    Skip,
}

fn parse_standalone(
    source: &str,
    path: &Path,
    admission: Admission,
) -> Result<Option<StandaloneSchemaDocument>, SchemaError> {
    let Some(recognized) =
        recognize_standalone(source).map_err(|message| schema_document_error(path, message))?
    else {
        return Ok(None);
    };
    // Metadata comes from the admitted (softly converted) copy; the payload
    // carriers (`$schema`, `types`) are never converted, so reading them from
    // the original is reading the admitted value.
    let metadata = match admission {
        Admission::Required => {
            let admitted = crate::envelope::admit_recognized(&recognized, source, path)?;
            if !admitted.problems.is_empty() {
                return Err(rejected_envelope(&recognized, source, path, &admitted));
            }
            admitted.value
        }
        Admission::Skip => crate::envelope::document_json(&recognized, path)?,
    };
    let envelope = recognized.envelope;
    let payload_key = recognized.payload_key();
    let map = recognized.mapping();
    let Some(payload) = recognized.payload() else {
        return Err(schema_document_error(path, "the envelope has neither `$schema` nor `types`"));
    };
    let description = metadata.get("description").and_then(serde_json::Value::as_str).map(str::to_string);
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
            // Admission rejects every other `types` carrier.
            _ => return Err(schema_document_error(path, "`types` was not admitted as a mapping")),
        },
        None => (None, None),
    };
    let categories = metadata
        .get("categories")
        .filter(|value| !value.is_null())
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
pub(crate) struct RecognizedStandalone {
    /// The whole document, a mapping.
    document: YamlValue,
    pub(crate) envelope: StandaloneSchemaEnvelope,
}

impl RecognizedStandalone {
    /// The envelope key holding the payload.
    pub(super) fn payload_key(&self) -> &'static str {
        if self.mapping().contains_key("$schema") { "$schema" } else { "types" }
    }

    pub(crate) fn mapping(&self) -> &serde_yaml_ng::Mapping {
        self.document.as_mapping().expect("recognition requires a mapping document")
    }

    /// The payload value; `None` for an envelope with neither `$schema` nor
    /// `types`, which admission rejects.
    pub(super) fn payload(&self) -> Option<&YamlValue> {
        self.mapping().get(self.payload_key())
    }

    pub(super) fn local_types(&self) -> Option<&YamlValue> {
        (self.payload_key() == "$schema").then(|| self.mapping().get("types")).flatten()
    }
}

/// Loads `source` with admitted numbers and decides which envelope, if any,
/// claims it.
///
/// `Ok(None)` is an ordinary document (not a mapping, or no envelope claims
/// it). `Err` carries the message for text that is not YAML. Recognition
/// reads only `kind` and the key set; whether the envelope is well formed is
/// admission's question ([`crate::envelope`]), and the payload is not parsed
/// here.
pub(crate) fn recognize_standalone(source: &str) -> Result<Option<RecognizedStandalone>, String> {
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
    let envelope = match map.get("kind").and_then(YamlValue::as_str) {
        Some("schema") => StandaloneSchemaEnvelope::Tagged,
        Some("yaml-schema") => StandaloneSchemaEnvelope::YamlSchema,
        _ if map.contains_key("$schema")
            && map
                .keys()
                .all(|key| key.as_str().is_some_and(|key| StandaloneSchemaEnvelope::Pure.admits_top_level_key(key))) =>
        {
            StandaloneSchemaEnvelope::Pure
        }
        _ => return Ok(None),
    };
    Ok(Some(RecognizedStandalone { document, envelope }))
}

/// Whether `source` is a YAML mapping claiming `kind: yaml-schema`, read
/// leniently on an error path: a target or import naming such a file is
/// refused as selected by `kind` even when its envelope is malformed.
pub(crate) fn claims_yaml_schema(source: &str) -> bool {
    matches!(
        recognize_standalone(source),
        Ok(Some(RecognizedStandalone { envelope: StandaloneSchemaEnvelope::YamlSchema, .. }))
    )
}

/// The error for an envelope its definition rejects, in this order:
///
/// 1. A problem with the envelope itself — an unknown key, metadata of the
///    wrong shape, a missing payload — at its authored position.
/// 2. Otherwise a payload definition the parser rejects: it fails the
///    definition's `schema` check too, and the parser's error locates the
///    rejected token inside the type string.
/// 3. Otherwise the payload carrier's own problem (`types: null`,
///    `types: [a, b]`).
fn rejected_envelope(
    recognized: &RecognizedStandalone,
    source: &str,
    path: &Path,
    admitted: &crate::envelope::EnvelopeAdmission,
) -> SchemaError {
    if let Some(error) = admitted.envelope_error(path, source) {
        return error;
    }
    let map = recognized.mapping();
    for key in ["$schema", "types"] {
        if let Some(value) = map.get(key)
            // A `types` that is not a mapping is wrong as a whole; its
            // entries are not definitions to blame.
            && (key != "types" || value.is_mapping())
            && let Err(failure) = parse_standalone_schema_payload_with_source(value, source, key)
            // Deeper than the carrier itself: inside one of its definitions.
            && failure.path.segments().len() > 1
        {
            // As when admitted: `types` beside an export is the named-type
            // table; without one it is the payload.
            let context = if key == recognized.payload_key() { "invalid SimplifiedSchema payload" } else { "invalid named types" };
            return failure.into_document_error(path, context);
        }
    }
    admitted.to_error(path, source)
}

/// Reads an admitted `categories:` list: each entry `Name` or
/// `Name -> description`, names unique and non-blank, order kept. Admission
/// already made the list one of strings.
fn parse_categories(path: &Path, value: &serde_json::Value) -> Result<Vec<SchemaCategory>, SchemaError> {
    let Some(entries) = value.as_array() else {
        return Err(schema_document_error(path, "`categories` was not admitted as a list"));
    };
    let mut categories: Vec<SchemaCategory> = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let at = |message: String| schema_document_error(path, format!("`categories[{index}]`: {message}"));
        let Some(text) = entry.as_str() else {
            return Err(at("a category entry was not admitted as a string".into()));
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
