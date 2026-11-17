//! The `darkmatter-host` manifest envelope: its shipped schema and the
//! validation a manifest passes before any member is read.

use std::path::Path;
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;
use simplified_schema::yaml_documents::{LoadedYaml, load_yaml_document, locating_problem};
use simplified_schema::{SchemaError, SimplifiedSchema, ValidationFailure, ValidationProblem};

use super::{HOST_REGISTRATION_CODE, Refusal};
use crate::config::load::line_column;

/// The virtual path of the shipped manifest schema.
pub const HOST_MANIFEST_SCHEMA_PATH: &str = crate::markdown::schemas::embedded::HOST_MANIFEST_SCHEMA_PATH;

/// The manifest schema's text, as shipped.
pub const HOST_MANIFEST_SCHEMA_SOURCE: &str = include_str!("../../../../schemas/darkmatter-host.yaml");

/// The named type a whole manifest satisfies.
pub const HOST_MANIFEST_TYPE: &str = "darkmatter-host";

/// The manifest format this reader understands.
pub const HOST_MANIFEST_FORMAT_VERSION: u64 = 1;

/// The prepared contract of a whole manifest.
///
/// ## Panics
///
/// When the shipped schema does not prepare, which the shipped artifact and
/// the registration tests rule out.
fn contract() -> &'static SimplifiedSchema {
    static CONTRACT: OnceLock<SimplifiedSchema> = OnceLock::new();
    CONTRACT.get_or_init(|| {
        SimplifiedSchema::try_from(HOST_MANIFEST_SCHEMA_SOURCE)
            .and_then(|schema| schema.select(HOST_MANIFEST_TYPE))
            .unwrap_or_else(|error| panic!("the shipped `{HOST_MANIFEST_TYPE}` type prepares: {error}"))
    })
}

/// A manifest the contract accepted. Every field is required by the schema,
/// so none is defaulted here.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Manifest {
    #[serde(rename = "kind")]
    _kind: String,
    #[serde(rename = "format_version")]
    _format_version: u64,
    pub host: String,
    pub version: String,
    pub identify: String,
    pub triggers: Vec<String>,
    pub globals: String,
    pub lifecycle: String,
    pub functions: Vec<FunctionPair>,
}

/// One `functions` item.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct FunctionPair {
    pub catalog: String,
    pub facts: String,
}

/// A manifest as read: its text, its positions, and its accepted fields.
pub(super) struct ReadManifest {
    pub text: String,
    pub loaded: LoadedYaml,
    pub manifest: Manifest,
}

impl ReadManifest {
    /// The 1-based line and column of the value at `segments`.
    pub fn locate(&self, segments: &[&str]) -> Option<(usize, usize)> {
        locate(&self.loaded, &self.text, segments)
    }

    /// The manifest as the schema judged it: its meaning, without comments
    /// or layout.
    pub fn meaning(&self) -> &Value {
        &self.loaded.json
    }
}

fn locate(loaded: &LoadedYaml, text: &str, segments: &[&str]) -> Option<(usize, usize)> {
    let segments: Vec<String> = segments.iter().map(|segment| (*segment).to_string()).collect();
    loaded.byte_range(&locating_problem(&segments)).map(|range| line_column(text, range.start))
}

/// Validates the manifest `bytes` read from `path`: one admissible YAML
/// mapping that the closed `darkmatter-host` contract accepts.
pub(super) fn read(path: &Path, bytes: Vec<u8>) -> Result<ReadManifest, Refusal> {
    let refuse = |at: Option<(usize, usize)>, message: String| Refusal {
        code: HOST_REGISTRATION_CODE,
        file: path.to_path_buf(),
        at,
        member: None,
        message,
    };
    let text = String::from_utf8(bytes).map_err(|error| {
        let at = error.utf8_error().valid_up_to();
        let text = String::from_utf8_lossy(error.as_bytes()).into_owned();
        refuse(Some(line_column(&text, at)), "the manifest is not valid UTF-8".to_string())
    })?;
    let loaded = load_yaml_document(&text, path).map_err(|error| match error {
        SchemaError::SchemaDocument { message, span, .. } => {
            refuse(span.map(|span| line_column(&text, span.start)), format!("the manifest is not valid YAML: {message}"))
        }
        other => refuse(None, format!("the manifest is not valid YAML: {other}")),
    })?;
    if !loaded.json.is_object() {
        let what = if loaded.json.is_null() { "is empty" } else { "is not a mapping" };
        return Err(refuse(Some((1, 1)), format!("the manifest {what}; a `{HOST_MANIFEST_TYPE}` manifest is a mapping")));
    }
    if let Err(failure) = contract().validate_strict(&loaded.json) {
        let problems = match &failure {
            ValidationFailure::InvalidData { problems } | ValidationFailure::UnavailableCheck { problems } => {
                problems.as_slice()
            }
            _ => &[],
        };
        let Some(first) = problems.first() else {
            return Err(refuse(None, format!("the manifest schema cannot judge it: {failure:?}")));
        };
        let at = problem_location(&loaded, &text, first);
        let message = problems.iter().map(describe).collect::<Vec<_>>().join("; ");
        return Err(refuse(at, message));
    }
    // The contract admitted the shape; this typed read refuses only what a
    // JSON Schema `const` cannot (a `format_version` written `1.0`).
    let manifest: Manifest = serde_json::from_value(loaded.json.clone()).map_err(|error| {
        refuse(locate(&loaded, &text, &["format_version"]), format!("the manifest is not a `{HOST_MANIFEST_TYPE}` manifest: {error}"))
    })?;
    Ok(ReadManifest { text, loaded, manifest })
}

/// Where `problem` is: the offending key for an unknown one, else its value
/// (a missing field's parent).
fn problem_location(loaded: &LoadedYaml, text: &str, problem: &ValidationProblem) -> Option<(usize, usize)> {
    let mut segments: Vec<&str> = problem.instance_path.segments().iter().map(String::as_str).collect();
    if let Some(key) = unknown_key(problem) {
        segments.push(key);
    }
    locate(loaded, text, &segments).or(Some((1, 1)))
}

fn unknown_key(problem: &ValidationProblem) -> Option<&str> {
    problem
        .offending_properties
        .first()
        .or(problem.offending_property.as_ref())
        .map(String::as_str)
}

/// One problem, worded for a manifest author.
fn describe(problem: &ValidationProblem) -> String {
    let at = simplified_schema::yaml_documents::pointer_string(problem.instance_path.segments());
    if let Some(key) = unknown_key(problem) {
        let parent = if at.is_empty() { String::new() } else { format!(" at `{at}`") };
        return format!("`{key}` is not a manifest field{parent}");
    }
    if let Some(missing) = &problem.property {
        let parent = if at.is_empty() { String::new() } else { format!(" at `{at}`") };
        return format!("`{missing}` is required{parent}");
    }
    if at.is_empty() { problem.message.clone() } else { format!("`{at}`: {}", problem.message) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_shipped_document_is_the_text_the_namespace_serves() {
        let shipped = crate::markdown::schemas::embedded::SHIPPED_SOURCES
            .partial("host")
            .expect("`darkmatter:host` names a shipped document");
        assert_eq!((shipped.path, shipped.text), (HOST_MANIFEST_SCHEMA_PATH, HOST_MANIFEST_SCHEMA_SOURCE));
        contract();
    }
}
