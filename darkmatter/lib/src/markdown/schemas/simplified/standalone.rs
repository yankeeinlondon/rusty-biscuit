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
//! - **Tagged**: a YAML mapping containing exactly `kind: schema` and a
//!   `types` mapping.
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
//! the document as ordinary YAML or raw JSON Schema.
//!
//! ## Side-effect freedom
//!
//! Classification, parsing, and suggestion linting perform no file reads,
//! shell execution, composition directive evaluation, or network access. The
//! caller supplies the source string; all work is in-memory.

use std::path::{Path, PathBuf};

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::{
    SchemaDeclaration, SchemaSourceMap, SimplifiedSchema, SuggestionLintProblem, lint_suggestions,
    project_suggestion_spans, source::parse_standalone_schema_payload_with_source,
};

/// The content envelope used by a standalone SimplifiedSchema document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandaloneSchemaEnvelope {
    /// A document whose sole top-level key is `$schema`.
    Pure,
    /// A document containing `kind: schema` and a `types` mapping.
    Tagged,
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
/// Returns `Ok(None)` for ordinary YAML and raw JSON Schema documents. Once
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
/// A malformed recognized envelope (missing/malformed `types`, unsupported
/// tagged-envelope keys, unparseable SimplifiedSchema payload, invalid or
/// remote reference) returns
/// `Err(SchemaError::SchemaDocument)`. Invalid `suggest(...)` candidates
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
    let value: YamlValue = match serde_yaml_ng::from_str(source) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(map) = value.as_mapping() else {
        return Ok(None);
    };

    let kind_key = YamlValue::String("kind".into());
    let types_key = YamlValue::String("types".into());
    let schema_key = YamlValue::String("$schema".into());
    let tagged = map.get(&kind_key).and_then(YamlValue::as_str) == Some("schema");

    let (envelope, payload) = if tagged {
        let unsupported: Vec<String> = map
            .keys()
            .filter_map(YamlValue::as_str)
            .filter(|key| !matches!(*key, "kind" | "types"))
            .map(str::to_string)
            .collect();
        if !unsupported.is_empty() || map.len() != 2 {
            return Err(schema_document_error(
                path,
                format!(
                    "tagged schema documents support only `kind` and `types`; found unsupported keys: {}",
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
    let suggestion_lints = match &mut parsed.value {
        SchemaDeclaration::Schema(schema) => {
            project_suggestion_spans(schema, source, 0).map_err(|error| {
                schema_document_error(path, format!("invalid SimplifiedSchema payload: {error}"))
            })?;
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
    }))
}

fn schema_document_error(path: &Path, message: impl Into<String>) -> SchemaError {
    SchemaError::SchemaDocument {
        path: path.to_path_buf(),
        message: message.into(),
    }
}
