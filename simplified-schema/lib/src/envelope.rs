//! Admission of standalone schema envelopes against the library's own
//! definitions.
//!
//! The `kind: schema` and `kind: yaml-schema` envelopes are part of the
//! SimplifiedSchema language, so their definitions — `schema.yaml` and
//! `yaml-schema.yaml` in `simplified-schema/schemas/` — are compiled into the
//! library ([`ENVELOPE_DEFINITIONS`]). Every recognized envelope, on every
//! load path (the [`SimplifiedSchema`](crate::SimplifiedSchema) factories,
//! `$schema` targets, `Name@file` imports, embedded documents), is admitted
//! by soft validation against its definition before its payload is read:
//!
//! | Envelope | Definition |
//! |---|---|
//! | `kind: schema` | `schema.yaml` |
//! | informal (`$schema` without `kind`) | `schema.yaml`, judged as if it declared `kind: schema` |
//! | `kind: yaml-schema` | `yaml-schema.yaml` |
//!
//! ## Bootstrap
//!
//! The two definitions are themselves `kind: yaml-schema` documents. They are
//! parsed and lowered once, without admission, the first time any envelope is
//! admitted; nothing consults them before they exist. That the definitions
//! satisfy themselves is checked by the library's tests, not at run time.
//!
//! ## Side-effect freedom
//!
//! Admission reads no files and evaluates nothing: the definitions are
//! compiled in and their checks are passive.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use serde_json::Value;

use crate::embedded::{EmbeddedDocument, ShippedSources};
use crate::errors::SchemaError;
use crate::outcome::ValidationFailure;
use crate::prepared::Prepared;
use crate::simplified::standalone::{RecognizedStandalone, parse_standalone_unadmitted, recognize_standalone};
use crate::simplified::{SchemaDeclaration, StandaloneSchemaEnvelope};
use crate::positions::{PositionMap, build_position_map};
use crate::validation::{ValidationProblem, ValidationProblemCode};

/// Virtual path of the definition of `kind: schema` documents.
pub const SCHEMA_DEFINITION_PATH: &str = "simplified-schema/schemas/schema.yaml";

/// Virtual path of the definition of `kind: yaml-schema` documents.
pub const YAML_SCHEMA_DEFINITION_PATH: &str = "simplified-schema/schemas/yaml-schema.yaml";

/// The library's envelope definitions, keyed by repository-relative path, the
/// keying a host's own embedded table uses.
pub const ENVELOPE_DEFINITIONS: &[EmbeddedDocument<'static>] = &[
    EmbeddedDocument { path: SCHEMA_DEFINITION_PATH, text: include_str!("../../schemas/schema.yaml") },
    EmbeddedDocument { path: YAML_SCHEMA_DEFINITION_PATH, text: include_str!("../../schemas/yaml-schema.yaml") },
];

/// The outcome of judging one envelope against its definition.
#[derive(Debug, Clone)]
pub struct EnvelopeAdmission {
    /// The envelope recognition found.
    pub envelope: StandaloneSchemaEnvelope,
    /// Virtual path of the definition the envelope was judged against.
    pub definition: &'static str,
    /// The admitted document after soft conversion when admitted (a
    /// `description: 123` reads as `"123"`), the document as written
    /// otherwise. Schema declarations (`$schema`, `types`) are never
    /// converted. An informal envelope's value carries no `kind`.
    pub value: Value,
    /// Positions of `value`'s nodes in the authored text.
    pub positions: PositionMap,
    /// The definition's problems, located against the authored text; empty
    /// when the envelope is admitted.
    pub problems: Vec<ValidationProblem>,
}

impl EnvelopeAdmission {
    /// Whether the envelope satisfies its definition.
    pub fn is_admitted(&self) -> bool {
        self.problems.is_empty()
    }

    /// Whether a problem is with the envelope itself rather than inside one
    /// of its schema-declaration carriers (`$schema`, `types`): an unknown
    /// key, metadata of the wrong shape, or a missing payload.
    pub(crate) fn has_envelope_problem(&self) -> bool {
        self.problems.iter().any(is_envelope_problem)
    }

    /// [`Self::to_error`] over the envelope's own problems; `None` when every
    /// problem is inside a declaration carrier.
    pub(crate) fn envelope_error(&self, path: &Path, source: &str) -> Option<SchemaError> {
        let problems: Vec<&ValidationProblem> = self.problems.iter().filter(|problem| is_envelope_problem(problem)).collect();
        (!problems.is_empty()).then(|| self.error_for(&problems, path, source))
    }

    /// The [`SchemaError::SchemaDocument`] for the reported problem, with its
    /// byte position in `source` as the span.
    pub(crate) fn to_error(&self, path: &Path, source: &str) -> SchemaError {
        let problems: Vec<&ValidationProblem> = self.problems.iter().collect();
        self.error_for(&problems, path, source)
    }

    /// The error for the reported one of `problems`: the first that is not a
    /// missing required key, when there is one. A missing key is often the
    /// consequence of a misspelled one (`typed:` for `types:`), and the
    /// misspelling is the defect to point at.
    fn error_for(&self, problems: &[&ValidationProblem], path: &Path, source: &str) -> SchemaError {
        let reported = problems
            .iter()
            .find(|problem| problem.code != ValidationProblemCode::MissingRequired)
            .or(problems.first())
            .copied();
        let Some(problem) = reported else {
            return SchemaError::SchemaDocument {
                path: path.to_path_buf(),
                message: format!("the envelope satisfies `{}`", self.definition),
                source: None,
                span: None,
            };
        };
        let mut location = problem.path.clone();
        if let Some(key) = problem.offending_property.as_deref().or(problem.property.as_deref()) {
            location = format!("{location}/{key}");
        }
        let location = if location.is_empty() { String::new() } else { format!("`{location}`: ") };
        let span = self.positions.byte_range(problem).map(|range| first_line(source, range));
        SchemaError::SchemaDocument {
            path: path.to_path_buf(),
            message: format!(
                "the {} envelope does not satisfy its definition `{}`: {location}{}",
                envelope_label(self.envelope),
                self.definition,
                problem.message
            ),
            source: None,
            span,
        }
    }
}

/// Recognizes `text` as a standalone schema envelope and judges it against
/// its definition.
///
/// ## Returns
///
/// `Ok(None)` when no envelope claims `text` (ordinary YAML, raw JSON Schema,
/// a non-mapping root); otherwise the admission, whose
/// [`problems`](EnvelopeAdmission::problems) are empty when the envelope is
/// admitted.
///
/// ## Errors
///
/// [`SchemaError::SchemaDocument`] when `text` is not valid YAML.
pub fn admit_envelope_text(text: &str, path: impl AsRef<Path>) -> Result<Option<EnvelopeAdmission>, SchemaError> {
    let path = path.as_ref();
    let Some(recognized) = recognize_standalone(text).map_err(|message| document_error(path, message))? else {
        return Ok(None);
    };
    admit_recognized(&recognized, text, path).map(Some)
}

/// Judges an already recognized envelope against its definition.
pub(crate) fn admit_recognized(
    recognized: &RecognizedStandalone,
    source: &str,
    path: &Path,
) -> Result<EnvelopeAdmission, SchemaError> {
    let definitions = definitions().as_ref().map_err(|message| document_error(path, message.clone()))?;
    let (prepared, definition) = match recognized.envelope {
        StandaloneSchemaEnvelope::YamlSchema => (&definitions.yaml_schema, YAML_SCHEMA_DEFINITION_PATH),
        StandaloneSchemaEnvelope::Tagged | StandaloneSchemaEnvelope::Pure => {
            (&definitions.schema, SCHEMA_DEFINITION_PATH)
        }
    };
    let mut document = document_json(recognized, path)?;
    let informal = recognized.envelope == StandaloneSchemaEnvelope::Pure;
    if informal && let Value::Object(map) = &mut document {
        map.shift_insert(0, "kind".into(), Value::String("schema".into()));
    }
    let (mut value, mut problems) = match prepared.validate_soft(&document) {
        Ok(success) => (success.value.into_owned(), Vec::new()),
        Err(ValidationFailure::Unprepared { error }) => {
            return Err(document_error(path, format!("`{definition}` could not be prepared: {error}")));
        }
        Err(failure) => (document.clone(), failure.into_problems()),
    };
    if informal && let Value::Object(map) = &mut value {
        map.shift_remove("kind");
    }
    let positions = build_position_map(source);
    positions.locate_all(&mut problems);
    Ok(EnvelopeAdmission { envelope: recognized.envelope, definition, value, positions, problems })
}

/// The recognized document as JSON, the form the definitions judge.
pub(crate) fn document_json(recognized: &RecognizedStandalone, path: &Path) -> Result<Value, SchemaError> {
    serde_json::to_value(recognized.mapping())
        .map_err(|error| document_error(path, format!("the envelope is not representable as a JSON object: {error}")))
}

/// The two compiled definitions, prepared once.
struct Definitions {
    schema: Prepared,
    yaml_schema: Prepared,
}

fn definitions() -> &'static Result<Definitions, String> {
    static DEFINITIONS: OnceLock<Result<Definitions, String>> = OnceLock::new();
    DEFINITIONS.get_or_init(|| {
        Ok(Definitions {
            schema: prepare_definition(SCHEMA_DEFINITION_PATH).map_err(|error| error.to_string())?,
            yaml_schema: prepare_definition(YAML_SCHEMA_DEFINITION_PATH).map_err(|error| error.to_string())?,
        })
    })
}

/// Parses and lowers one compiled definition without admission (it cannot be
/// admitted against itself before it exists) and compiles it for validation.
fn prepare_definition(path: &'static str) -> Result<Prepared, SchemaError> {
    let text = crate::embedded::find(ENVELOPE_DEFINITIONS, path)
        .map(|document| document.text)
        .ok_or_else(|| document_error(Path::new(path), "not a compiled envelope definition"))?;
    let document = parse_standalone_unadmitted(text, Path::new(path))?
        .ok_or_else(|| document_error(Path::new(path), "not a standalone schema document"))?;
    let SchemaDeclaration::Schema(schema) = document.declaration else {
        return Err(document_error(Path::new(path), "an envelope definition cannot be a whole-file reference"));
    };
    let resolved = crate::resolve::lower_embedded_definition(
        schema,
        document.local_types.as_ref(),
        path,
        ENVELOPE_DEFINITIONS,
        ShippedSources::NONE,
    )?;
    let private = crate::validate::ValidatorCache::new();
    let prepared =
        Prepared::compiled_through(Arc::new(resolved.json_schema), None, crate::schema::content_context(), &private)?;
    Ok(prepared.without_file_checks())
}

fn envelope_label(envelope: StandaloneSchemaEnvelope) -> &'static str {
    match envelope {
        StandaloneSchemaEnvelope::Pure => "informal schema",
        StandaloneSchemaEnvelope::Tagged => "`kind: schema`",
        StandaloneSchemaEnvelope::YamlSchema => "`kind: yaml-schema`",
    }
}

/// Whether `problem` is with the envelope rather than inside a `$schema` or
/// `types` carrier: its instance path starts elsewhere, or it is reported on
/// the root mapping (an unknown key, a missing required key).
fn is_envelope_problem(problem: &ValidationProblem) -> bool {
    !matches!(problem.instance_path.first(), Some("$schema" | "types"))
}

/// `range` cut to its first line: a block collection is reported at its
/// opening line, not across every line it spans.
fn first_line(source: &str, range: std::ops::Range<usize>) -> std::ops::Range<usize> {
    let text = &source[range.clone()];
    let end = text.find(['\n', '\r']).map_or(range.end, |newline| range.start + text[..newline].trim_end().len());
    range.start..end
}

fn document_error(path: &Path, message: impl Into<String>) -> SchemaError {
    SchemaError::SchemaDocument { path: path.to_path_buf(), message: message.into(), source: None, span: None }
}

