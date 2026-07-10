//! Content-based standalone SimplifiedSchema document recognition.

use std::path::{Path, PathBuf};

use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;

use super::{
    SimplifiedSchema, SuggestionLintProblem, lint_suggestions, parse_yaml_schema_with_source,
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
/// Candidate spans are byte ranges in `source`, and lint problems retain those
/// same authoring-document ranges. Parsing and linting perform no file, shell,
/// network, or composition operations.
#[derive(Debug, Clone, PartialEq)]
pub struct StandaloneSchemaDocument {
    /// Path of the authoring document supplied by the caller.
    pub path: PathBuf,
    /// Content envelope that claimed the document.
    pub envelope: StandaloneSchemaEnvelope,
    /// Source-aware parsed SimplifiedSchema payload.
    pub schema: SimplifiedSchema,
    /// Invalid advisory suggestion metadata in declaration order.
    pub suggestion_lints: Vec<SuggestionLintProblem>,
}

/// Classifies and parses a standalone YAML SimplifiedSchema document.
///
/// Returns `Ok(None)` for ordinary YAML and raw JSON Schema documents. Once
/// `kind: schema` claims a tagged document, or `$schema` is the sole top-level
/// key of a pure document, malformed envelope content is returned as a
/// [`SchemaError::SchemaDocument`] instead of falling back to raw JSON Schema.
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
        if !matches!(payload, YamlValue::Mapping(_) | YamlValue::Sequence(_)) {
            return Err(schema_document_error(
                path,
                "pure schema document `$schema` must be a mapping or sequence",
            ));
        }
        (StandaloneSchemaEnvelope::Pure, payload)
    } else {
        return Ok(None);
    };

    let schema = parse_yaml_schema_with_source(payload, source, 0).map_err(|error| {
        schema_document_error(path, format!("invalid SimplifiedSchema payload: {error}"))
    })?;
    let suggestion_lints = lint_suggestions(&schema).map_err(|error| {
        schema_document_error(path, format!("could not lint SimplifiedSchema payload: {error}"))
    })?;
    Ok(Some(StandaloneSchemaDocument {
        path: path.to_path_buf(),
        envelope,
        schema,
        suggestion_lints,
    }))
}

fn schema_document_error(path: &Path, message: impl Into<String>) -> SchemaError {
    SchemaError::SchemaDocument {
        path: path.to_path_buf(),
        message: message.into(),
    }
}
