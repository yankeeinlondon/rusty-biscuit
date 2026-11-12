//! Admission of standalone schema envelopes against the library's own
//! definitions.
//!
//! The `kind: schema` and `kind: yaml-schema` envelopes are part of the
//! SimplifiedSchema language, so their definitions — `schema.yaml` and
//! `yaml-schema.yaml` in `simplified-schema/schemas/` — are compiled into the
//! library ([`ENVELOPE_DEFINITIONS`]). Every recognized envelope, on every
//! load path (the [`SimplifiedSchema`](crate::SimplifiedSchema) factories,
//! `$schema` targets, `Name@file` imports, embedded documents), is admitted
//! by soft validation against its definition before its payload is read
//! (only a host that already judged a file against a definition it selected
//! reads it unadmitted, through
//! [`parse_selected_standalone_schema_document`](crate::parse_selected_standalone_schema_document)):
//!
//! | Envelope | Definition |
//! |---|---|
//! | `kind: schema` | `schema.yaml` |
//! | informal (`$schema` without `kind`) | `schema.yaml`, judged as if it declared `kind: schema` |
//! | `kind: yaml-schema` | `yaml-schema.yaml` |
//!
//! ## Bootstrap
//!
//! The two definitions are themselves `kind: yaml-schema` documents. Their
//! lowered form is loaded from the library's shipped artifact
//! ([`crate::shipped::registry`]) the first time any envelope is admitted.
//! When that artifact is refused, or when an artifact is generated, they are
//! parsed and lowered from their text without admission
//! ([`EnvelopeDefinitions::prepare_uncached`]); nothing consults them before
//! they exist. Generation then checks that each definition satisfies its own
//! contract. Their validators compile on first use either way.
//!
//! ## Side-effect freedom
//!
//! Admission reads no files and evaluates nothing: the definitions are
//! compiled in and their checks are passive.

use std::path::Path;
use std::sync::{Arc, OnceLock};

use crate::shipped::PreparationMode;

use serde_json::Value;

use crate::embedded::{EmbeddedDocument, ShippedSources};
use crate::errors::SchemaError;
use crate::outcome::ValidationFailure;
use crate::prepared::Prepared;
use crate::simplified::standalone::{RecognizedStandalone, parse_standalone_unadmitted, recognize_standalone};
use crate::simplified::{SchemaDeclaration, StandaloneSchemaEnvelope};
use crate::positions::PositionMap;
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
    /// Positions of `value`'s nodes in the authored text, built when a
    /// coordinate is first requested.
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
    admit_envelope_text_with_mode(text, path, PreparationMode::Runtime)
}

/// [`admit_envelope_text`] against the definitions `mode` prepares with:
/// the process's loaded definitions, or a generation's freshly prepared ones.
///
/// ## Errors
///
/// As [`admit_envelope_text`].
pub fn admit_envelope_text_with_mode(
    text: &str,
    path: impl AsRef<Path>,
    mode: PreparationMode<'_>,
) -> Result<Option<EnvelopeAdmission>, SchemaError> {
    let path = path.as_ref();
    let Some(recognized) = recognize_standalone(text).map_err(|message| document_error(path, message))? else {
        return Ok(None);
    };
    admit_recognized_with_mode(&recognized, text, path, mode).map(Some)
}

/// Judges an already recognized envelope against its definition.
pub(crate) fn admit_recognized(
    recognized: &RecognizedStandalone,
    source: &str,
    path: &Path,
) -> Result<EnvelopeAdmission, SchemaError> {
    admit_recognized_with_mode(recognized, source, path, PreparationMode::Runtime)
}

/// [`admit_recognized`] against the definitions `mode` prepares with.
pub(crate) fn admit_recognized_with_mode(
    recognized: &RecognizedStandalone,
    source: &str,
    path: &Path,
    mode: PreparationMode<'_>,
) -> Result<EnvelopeAdmission, SchemaError> {
    let definitions = crate::shipped::registry::envelopes(mode).map_err(|message| document_error(path, message))?;
    admit_with(definitions, recognized, source, path)
}

/// Judges `recognized` against `definitions`.
fn admit_with(
    definitions: &EnvelopeDefinitions,
    recognized: &RecognizedStandalone,
    source: &str,
    path: &Path,
) -> Result<EnvelopeAdmission, SchemaError> {
    let (lowered, definition) = definitions.for_envelope(recognized.envelope);
    let prepared = lowered
        .prepared()
        .map_err(|error| document_error(path, format!("`{definition}` could not be prepared: {error}")))?;
    let mut document = document_json(recognized, path).map_err(|error| slot_key_error(source, path).unwrap_or(error))?;
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
    let positions = PositionMap::deferred(std::sync::Arc::from(source));
    positions.locate_all(&mut problems);
    Ok(EnvelopeAdmission { envelope: recognized.envelope, definition, value, positions, problems })
}

/// The recognized document as JSON, the form the definitions judge.
pub(crate) fn document_json(recognized: &RecognizedStandalone, path: &Path) -> Result<Value, SchemaError> {
    serde_json::to_value(recognized.mapping())
        .map_err(|error| document_error(path, format!("the envelope is not representable as a JSON object: {error}")))
}

/// The "quote the slot" error for a document [`document_json`] rejects when
/// its non-string keys are unquoted `{{ … }}` values: YAML read each as a
/// flow mapping keyed by a mapping. Proven by the same text with every such
/// value single-quoted converting; located at the first of them.
fn slot_key_error(source: &str, path: &Path) -> Option<SchemaError> {
    let slots = crate::slots::unquoted_slot_spans(source);
    let first = slots.first()?.clone();
    let (quoted, _) = crate::slots::quote_slots(source, &slots);
    let quoted: serde_yaml_ng::Value = serde_yaml_ng::from_str(&quoted).ok()?;
    serde_json::to_value(&quoted).ok()?;
    Some(SchemaError::SchemaDocument {
        path: path.to_path_buf(),
        message: crate::slots::quote_the_slot_message(source, &first),
        source: None,
        span: Some(first),
    })
}

/// The two compiled envelope definitions, lowered. Each compiles its
/// validator on first admission.
pub struct EnvelopeDefinitions {
    schema: LoweredDefinition,
    yaml_schema: LoweredDefinition,
}

impl std::fmt::Debug for EnvelopeDefinitions {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("EnvelopeDefinitions")
    }
}

/// One definition's lowered JSON Schema and its lazily compiled validators.
struct LoweredDefinition {
    json_schema: Arc<Value>,
    prepared: OnceLock<Result<Prepared, String>>,
}

impl LoweredDefinition {
    fn new(json_schema: Arc<Value>) -> Self {
        Self { json_schema, prepared: OnceLock::new() }
    }

    /// The definition compiled for validation through a private cache: the
    /// definitions live for the process, so they must not occupy, or be
    /// counted in, the cache a host resets and bounds.
    fn prepared(&self) -> Result<&Prepared, &str> {
        self.prepared
            .get_or_init(|| {
                let private = crate::validate::ValidatorCache::new();
                Prepared::compiled_through(
                    Arc::clone(&self.json_schema),
                    None,
                    crate::schema::content_context(),
                    &private,
                )
                .map(Prepared::without_file_checks)
                .map_err(|error| error.to_string())
            })
            .as_ref()
            .map_err(String::as_str)
    }
}

impl EnvelopeDefinitions {
    /// Definitions whose lowered schemas were already prepared (an accepted
    /// artifact's records).
    pub(crate) fn from_lowered(schema: Arc<Value>, yaml_schema: Arc<Value>) -> Self {
        Self { schema: LoweredDefinition::new(schema), yaml_schema: LoweredDefinition::new(yaml_schema) }
    }

    /// Parses and lowers both compiled definitions from their text, without
    /// admission (neither can be admitted before both exist) and without
    /// consulting the library's artifact or registry.
    ///
    /// ## Errors
    ///
    /// The parser's or lowering's error for a definition that does not
    /// prepare.
    pub(crate) fn prepare_uncached() -> Result<Self, SchemaError> {
        Ok(Self::from_lowered(
            Arc::new(lower_definition(SCHEMA_DEFINITION_PATH)?),
            Arc::new(lower_definition(YAML_SCHEMA_DEFINITION_PATH)?),
        ))
    }

    /// [`admit_envelope_text`] against these definitions.
    ///
    /// ## Errors
    ///
    /// As [`admit_envelope_text`].
    pub fn admit(&self, text: &str, path: impl AsRef<Path>) -> Result<Option<EnvelopeAdmission>, SchemaError> {
        let path = path.as_ref();
        let Some(recognized) = recognize_standalone(text).map_err(|message| document_error(path, message))? else {
            return Ok(None);
        };
        admit_with(self, &recognized, text, path).map(Some)
    }

    /// The lowered `schema.yaml` and `yaml-schema.yaml`.
    pub(crate) fn json_schemas(&self) -> (&Value, &Value) {
        (&self.schema.json_schema, &self.yaml_schema.json_schema)
    }

    fn for_envelope(&self, envelope: StandaloneSchemaEnvelope) -> (&LoweredDefinition, &'static str) {
        match envelope {
            StandaloneSchemaEnvelope::YamlSchema => (&self.yaml_schema, YAML_SCHEMA_DEFINITION_PATH),
            StandaloneSchemaEnvelope::Tagged | StandaloneSchemaEnvelope::Pure => (&self.schema, SCHEMA_DEFINITION_PATH),
        }
    }

    /// The generation check: each compiled definition is an envelope its
    /// own definitions admit.
    ///
    /// ## Errors
    ///
    /// The first definition's admission error.
    pub(crate) fn check_contracts(&self) -> Result<(), SchemaError> {
        for document in ENVELOPE_DEFINITIONS {
            let path = Path::new(document.path);
            let recognized = recognize_standalone(document.text)
                .map_err(|message| document_error(path, message))?
                .ok_or_else(|| document_error(path, "not a standalone schema document"))?;
            let admission = admit_with(self, &recognized, document.text, path)?;
            if !admission.is_admitted() {
                return Err(admission.to_error(path, document.text));
            }
        }
        Ok(())
    }
}

/// Parses and lowers one compiled definition without admission. Its imports
/// would be admitted against definitions that do not exist yet, so it is
/// lowered against an empty document table: a definition that imports
/// fails here.
fn lower_definition(path: &'static str) -> Result<Value, SchemaError> {
    let text = crate::embedded::find(ENVELOPE_DEFINITIONS, path)
        .map(|document| document.text)
        .ok_or_else(|| document_error(Path::new(path), "not a compiled envelope definition"))?;
    let document = parse_standalone_unadmitted(text, Path::new(path))?
        .ok_or_else(|| document_error(Path::new(path), "not a standalone schema document"))?;
    let SchemaDeclaration::Schema(schema) = document.declaration else {
        return Err(document_error(Path::new(path), "an envelope definition cannot be a whole-file reference"));
    };
    let resolved =
        crate::resolve::lower_embedded_definition(schema, document.local_types.as_ref(), path, &[], ShippedSources::NONE)?;
    Ok(resolved.json_schema)
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

