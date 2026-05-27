//! Schema-aware composition preparation.
//!
//! Darkmatter's compose pipeline runs schema validation as an always-on stage
//! between override application and shell expansion (see
//! `darkmatter::markdown::compose::schema_validation`). When a document
//! declares a `$schema` and the (post-override) frontmatter does not satisfy
//! it, compose aborts with `MarkdownError::SchemaValidationFailed`, which
//! the prepare layer surfaces as `CompositionError::ComposeFailed`.
//!
//! This module wraps [`prepare_direct`] / [`prepare_inline`] so callers get
//! claudine's typed schema errors instead of a generic compose failure:
//!
//! - [`CompositionError::SchemaLoad`] when the `$schema` reference itself
//!   could not be resolved or compiled.
//! - [`CompositionError::SchemaValidation`] when one or more **required**
//!   properties have the wrong type.
//! - [`CompositionError::MissingProperties`] when one or more **required**
//!   properties are missing. Carries the property metadata Phase 3 needs
//!   to drive the interactive TUI.
//!
//! Invalid **optional** values are dropped from the markdown's frontmatter
//! and composition is retried once. The dropped properties are reported via
//! `tracing::warn!` so users can see what was elided from the prompt
//! context.
//!
//! See [`claudine/features/2026-05-15-schemas/plan.md`][plan] for the
//! Phase 2 contract.
//!
//! [plan]: ../../../../features/2026-05-15-schemas/plan.md

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, EffectiveSchema, PropertyAtom, PropertyDef, SchemaShape,
    SimplifiedSchema, SimplifiedType, ValidationProblem, ValidationProblemKind,
};

use super::error::{
    CompositionError, InteractiveShape, MissingProperty, TextFormat,
};
use super::prepare::{PrepareOptions, prepare_direct, prepare_inline};
use super::types::{PreparedComposition, ResolvedCompositionSource};

/// Inputs controlling whether interactive collection of missing required
/// properties is permitted for this run.
///
/// Per Phase 2 Task 4, Interactive Mode is allowed only when **all** flags
/// resolve to `true`: the user has not disabled `prompt_for_missing`, stdin
/// and stderr are both TTYs, and `--silent` is not in effect.
///
/// The library itself never decides whether to enter the interactive loop —
/// the CLI does, by checking [`Self::allowed`] and either prompting the
/// user (Phase 3) or surfacing the [`CompositionError::MissingProperties`]
/// returned by [`prepare_direct_with_schema`] / [`prepare_inline_with_schema`].
///
/// All fields default to `false` so a freshly-constructed value is always
/// in the "interactive denied" state — callers must opt in to each signal
/// explicitly. The user config's `prompt_for_missing` defaults to `true`,
/// but the CLI is expected to populate this field from the resolved config
/// before calling [`Self::allowed`].
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct InteractiveSchemaOptions {
    /// User config `prompt_for_missing`.
    pub prompt_for_missing: bool,
    /// `true` when stdin is attached to a TTY.
    pub stdin_is_tty: bool,
    /// `true` when stderr is attached to a TTY.
    pub stderr_is_tty: bool,
    /// `true` when `--silent` was passed to the command.
    pub silent: bool,
}

impl InteractiveSchemaOptions {
    /// Returns `true` when the four input flags permit prompting.
    ///
    /// The plan's full rule also requires that at least one required
    /// property be missing and that no required values be present-but-invalid.
    /// Those extra conditions are checked by the validation layer (which
    /// only emits [`CompositionError::MissingProperties`] when the missing
    /// case applies) and by callers (which know whether they are already
    /// rendering a hard validation error).
    pub const fn allowed(self) -> bool {
        self.prompt_for_missing && self.stdin_is_tty && self.stderr_is_tty && !self.silent
    }
}

/// Wrap [`prepare_direct`] with schema-aware error categorization and
/// automatic drop-and-retry for invalid optional frontmatter values.
///
/// ## Errors
///
/// Returns [`CompositionError::SchemaLoad`] when the document's `$schema`
/// reference cannot be loaded or compiled,
/// [`CompositionError::SchemaValidation`] when a required value has the
/// wrong type, [`CompositionError::MissingProperties`] when a required
/// value is absent, or any other [`CompositionError`] surfaced by
/// composition (shell expansion, etc.).
pub fn prepare_direct_with_schema(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError> {
    prepare_with_schema(source, options, PrepareMode::Direct)
}

/// Wrap [`prepare_inline`] with schema-aware error categorization and
/// automatic drop-and-retry for invalid optional frontmatter values.
///
/// ## Errors
///
/// See [`prepare_direct_with_schema`]; the same set of typed schema errors
/// is surfaced for inline composition.
pub fn prepare_inline_with_schema(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
) -> Result<PreparedComposition, CompositionError> {
    prepare_with_schema(source, options, PrepareMode::Inline)
}

#[derive(Clone, Copy)]
enum PrepareMode {
    Direct,
    Inline,
}

fn prepare_with_schema(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode,
) -> Result<PreparedComposition, CompositionError> {
    match run_prepare(source, options.clone(), mode) {
        Ok(prepared) => Ok(prepared),
        Err(err) => handle_compose_error(source, options, mode, err),
    }
}

fn run_prepare(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode,
) -> Result<PreparedComposition, CompositionError> {
    match mode {
        PrepareMode::Direct => prepare_direct(source, options),
        PrepareMode::Inline => prepare_inline(source, options),
    }
}

fn handle_compose_error(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode,
    err: CompositionError,
) -> Result<PreparedComposition, CompositionError> {
    let CompositionError::ComposeFailed(markdown_err) = err else {
        return Err(err);
    };

    match markdown_err {
        MarkdownError::SchemaValidationFailed {
            problems, summary, ..
        } => translate_schema_failure(source, options, mode, problems, summary),
        other => Err(CompositionError::ComposeFailed(other)),
    }
}

fn translate_schema_failure(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode,
    problems: Vec<ValidationProblem>,
    summary: String,
) -> Result<PreparedComposition, CompositionError> {
    // Empty problems list signals a preparation failure (e.g. `$schema: 42`,
    // unresolvable file reference). Surface as `SchemaLoad`.
    if problems.is_empty() {
        return Err(CompositionError::SchemaLoad {
            source_path: source.resolved_path.clone(),
            message: summary,
        });
    }

    let effective = load_effective_schema(source)?;
    let categorized = categorize_problems(&problems, effective.as_ref());

    if !categorized.invalid_required.is_empty() {
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_required,
        ));
    }

    if !categorized.invalid_optional.is_empty() {
        // Drop invalid optionals from a clone of the source, then retry the
        // prepare step exactly once. If composition still fails, fall
        // through to surface the residual problem (likely a missing
        // required property).
        let retry_source = source_with_dropped_optionals(source, &categorized.invalid_optional);
        return match run_prepare(&retry_source, options.clone(), mode) {
            Ok(prepared) => Ok(prepared),
            Err(retry_err) => handle_retry_error(source, retry_err),
        };
    }

    if !categorized.missing_required.is_empty() {
        return Err(build_missing_properties_error(
            source,
            &categorized.missing_required,
            &categorized.pointer_paths,
        ));
    }

    // Fall-through: problems list contained only categories we couldn't
    // act on (e.g. arm-index union problems with no `kind` mapping). Surface
    // a generic SchemaValidation so the user still sees the failure.
    Err(CompositionError::SchemaValidation {
        source_path: source.resolved_path.clone(),
        message: summary,
        problems: problems.iter().map(|p| p.path.clone()).collect(),
    })
}

fn handle_retry_error(
    source: &ResolvedCompositionSource,
    err: CompositionError,
) -> Result<PreparedComposition, CompositionError> {
    let CompositionError::ComposeFailed(MarkdownError::SchemaValidationFailed {
        problems,
        summary,
        ..
    }) = err
    else {
        return Err(err);
    };

    let effective = load_effective_schema(source)?;
    let categorized = categorize_problems(&problems, effective.as_ref());

    if !categorized.invalid_required.is_empty() {
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_required,
        ));
    }

    if !categorized.missing_required.is_empty() {
        return Err(build_missing_properties_error(
            source,
            &categorized.missing_required,
            &categorized.pointer_paths,
        ));
    }

    Err(CompositionError::SchemaValidation {
        source_path: source.resolved_path.clone(),
        message: summary,
        problems: problems.iter().map(|p| p.path.clone()).collect(),
    })
}

fn load_effective_schema(
    source: &ResolvedCompositionSource,
) -> Result<Option<EffectiveSchema>, CompositionError> {
    DarkmatterSchemas::new()
        .effective_for(&source.markdown)
        .map_err(|err| CompositionError::SchemaLoad {
            source_path: source.resolved_path.clone(),
            message: err.to_string(),
        })
}

fn build_schema_validation_error(
    source_path: &std::path::Path,
    invalid: &[ValidationProblem],
) -> CompositionError {
    let message = invalid
        .iter()
        .map(|p| {
            if p.path.is_empty() {
                p.message.clone()
            } else {
                format!("{}: {}", p.path, p.message)
            }
        })
        .collect::<Vec<_>>()
        .join("; ");
    let problems = invalid.iter().map(|p| p.path.clone()).collect();
    CompositionError::SchemaValidation {
        source_path: source_path.to_path_buf(),
        message,
        problems,
    }
}

fn build_missing_properties_error(
    source: &ResolvedCompositionSource,
    missing: &[MissingProperty],
    pointer_paths: &[String],
) -> CompositionError {
    let frontmatter_description = source
        .markdown
        .frontmatter()
        .as_map()
        .get("description")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    CompositionError::MissingProperties {
        source_path: source.resolved_path.clone(),
        missing: missing.to_vec(),
        frontmatter_description,
        pointer_paths: pointer_paths.to_vec(),
    }
}

/// Build a clone of `source` with each invalid-optional key removed from
/// the markdown's frontmatter.
fn source_with_dropped_optionals(
    source: &ResolvedCompositionSource,
    invalid_optional: &[ValidationProblem],
) -> ResolvedCompositionSource {
    let mut clone = source.clone();
    let map = clone.markdown.frontmatter_mut().as_map_mut();
    for problem in invalid_optional {
        let Some(name) = top_level_pointer_segment(&problem.path) else {
            continue;
        };
        if map.shift_remove(&name).is_some() {
            tracing::warn!(
                property = %name,
                pointer = %problem.path,
                message = %problem.message,
                "dropping optional schema property with invalid value",
            );
        }
    }
    clone
}

// -- categorization ---------------------------------------------------------

struct CategorizedProblems {
    missing_required: Vec<MissingProperty>,
    invalid_required: Vec<ValidationProblem>,
    invalid_optional: Vec<ValidationProblem>,
    pointer_paths: Vec<String>,
}

fn categorize_problems(
    problems: &[ValidationProblem],
    effective: Option<&EffectiveSchema>,
) -> CategorizedProblems {
    let mut missing_required = Vec::new();
    let mut invalid_required = Vec::new();
    let mut invalid_optional = Vec::new();
    let mut pointer_paths = Vec::new();

    let shape: Option<&SchemaShape> = effective.and_then(|e| match e.simplified.as_ref() {
        Some(SimplifiedSchema::Single(s)) => Some(s),
        Some(SimplifiedSchema::Union(_)) | None => None,
    });

    for problem in problems {
        pointer_paths.push(problem.path.clone());
        match problem.kind {
            ValidationProblemKind::Missing => {
                let name = problem
                    .property
                    .clone()
                    .unwrap_or_else(|| problem.path.clone());
                let atom = shape.and_then(|s| atom_for_property(s, &name));
                let (type_label, description, interactive_shape) = match atom {
                    Some(a) => (
                        Some(type_label_for_atom(a)),
                        a.description.clone(),
                        interactive_shape_for_atom(a),
                    ),
                    None => (None, None, None),
                };
                missing_required.push(MissingProperty {
                    name,
                    type_label,
                    description,
                    interactive_shape,
                });
            }
            ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
                let top = top_level_pointer_segment(&problem.path);
                let required = top
                    .as_deref()
                    .map(|name| is_required(shape, name))
                    .unwrap_or(true);
                if required {
                    invalid_required.push(problem.clone());
                } else {
                    invalid_optional.push(problem.clone());
                }
            }
        }
    }

    CategorizedProblems {
        missing_required,
        invalid_required,
        invalid_optional,
        pointer_paths,
    }
}

fn atom_for_property<'s>(shape: &'s SchemaShape, name: &str) -> Option<&'s PropertyAtom> {
    let def = shape.properties.get(name)?;
    match def {
        PropertyDef::Single(atom) => Some(atom),
        // Property-level unions cannot be reduced to a single atom; the
        // type label is left blank so the renderer falls back to a
        // generic "(unknown type)" hint.
        PropertyDef::Union(_) => None,
    }
}

fn is_required(shape: Option<&SchemaShape>, name: &str) -> bool {
    let Some(shape) = shape else {
        // Without typed metadata we can't distinguish optional from required.
        // Be conservative: treat the property as required so unexpected
        // invalid values surface loudly instead of being silently dropped.
        return true;
    };
    let Some(def) = shape.properties.get(name) else {
        // Property not declared in the shape — extras are neither required
        // nor optional in our vocabulary. Treat as required for the same
        // reason.
        return true;
    };
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(a) => vec![a],
        PropertyDef::Union(items) => items.iter().collect(),
    };
    atoms.iter().any(|atom| {
        atom.constraints
            .iter()
            .any(|c| matches!(c, Constraint::Required))
    })
}

/// Map a [`PropertyAtom`] to an [`InteractiveShape`] for CLI prompting.
///
/// Returns `None` when the atom describes a shape that cannot be
/// collected via a single TUI widget (e.g. `object`, `any`).
fn interactive_shape_for_atom(atom: &PropertyAtom) -> Option<InteractiveShape> {
    match atom.ty {
        SimplifiedType::Enum => {
            let members: Vec<String> = atom
                .constraints
                .iter()
                .find_map(|c| match c {
                    Constraint::Members(m) => Some(m.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            if members.is_empty() {
                return None;
            }
            if atom.is_array {
                Some(InteractiveShape::EnumMany { members })
            } else {
                Some(InteractiveShape::EnumOne { members })
            }
        }
        SimplifiedType::Boolean | SimplifiedType::Boolish => Some(InteractiveShape::Boolean),
        SimplifiedType::Number | SimplifiedType::NumberLike => {
            let integer = atom
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Integer));
            Some(InteractiveShape::Number { integer })
        }
        SimplifiedType::String => Some(InteractiveShape::Text {
            format: TextFormat::Plain,
        }),
        SimplifiedType::Date => Some(InteractiveShape::Text {
            format: TextFormat::Date,
        }),
        SimplifiedType::DateTime => Some(InteractiveShape::Text {
            format: TextFormat::DateTime,
        }),
        SimplifiedType::Time => Some(InteractiveShape::Text {
            format: TextFormat::Time,
        }),
        SimplifiedType::Url => Some(InteractiveShape::Text {
            format: TextFormat::Url,
        }),
        SimplifiedType::Email => Some(InteractiveShape::Text {
            format: TextFormat::Email,
        }),
        SimplifiedType::File => Some(InteractiveShape::Text {
            format: TextFormat::File,
        }),
        SimplifiedType::Object | SimplifiedType::Any => None,
    }
}

fn type_label_for_atom(atom: &PropertyAtom) -> String {
    let suffix = if atom.is_array { "[]" } else { "" };
    if matches!(atom.ty, SimplifiedType::Enum) {
        let members = atom.constraints.iter().find_map(|c| match c {
            Constraint::Members(m) => Some(m.join("|")),
            _ => None,
        });
        match members {
            Some(m) => format!("enum({m}){suffix}"),
            None => format!("enum(){suffix}"),
        }
    } else {
        format!("{base}{suffix}", base = atom.ty.as_keyword())
    }
}

// -- status report ----------------------------------------------------------

/// Structured per-property status of a schema-validated composition.
///
/// Used by the CLI to render the diagnostic status report (Phase 3 Task 1)
/// before driving interactive collection of missing required properties.
/// Tests should assert on the structured fields rather than rendered
/// terminal output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaStatusReport {
    /// The prompt file whose schema was evaluated.
    pub source_path: std::path::PathBuf,
    /// Required properties in declaration order.
    pub required: Vec<PropertyStatus>,
    /// Optional properties in declaration order.
    pub optional: Vec<PropertyStatus>,
    /// `true` when at least one optional property had an invalid value
    /// (these are dropped by the validation retry but worth surfacing).
    pub has_invalid_optional: bool,
    /// `true` when the schema is raw JSON Schema (no SimplifiedSchema
    /// metadata is available, so property listing falls back to the
    /// validator's pointer paths).
    pub raw_json_schema: bool,
}

/// Per-property status entry inside a [`SchemaStatusReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyStatus {
    /// Property name as declared in the schema.
    pub name: String,
    /// Schema type label (e.g. `string`, `enum(a|b|c)`, `number[]`).
    pub type_label: String,
    /// Schema-declared description for the property, if any.
    pub description: Option<String>,
    /// Validation outcome for the property under current frontmatter.
    pub state: PropertyState,
}

/// Validation outcome for a single property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyState {
    /// The property is present and validates against the schema.
    Valid,
    /// The property is present but violates a schema constraint.
    Invalid,
    /// The property is absent from the (effective) frontmatter.
    Missing,
}

/// Build a [`SchemaStatusReport`] for `source` under the supplied
/// `set_overrides`, mirroring the validation Darkmatter would run during
/// composition.
///
/// Returns `Ok(None)` when the document has no `$schema` declaration.
///
/// ## Errors
///
/// Returns [`CompositionError::SchemaLoad`] when the document's `$schema`
/// reference cannot be resolved or compiled.
pub fn build_schema_status_report(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
) -> Result<Option<SchemaStatusReport>, CompositionError> {
    // Skip when the document has no `$schema`.
    if source
        .markdown
        .frontmatter()
        .as_map()
        .get("$schema")
        .is_none()
    {
        return Ok(None);
    }

    let effective = load_effective_schema(source)?;
    let Some(effective) = effective else {
        // Raw JSON Schema (no SimplifiedSchema projection): we can still
        // run validation, but without typed metadata for per-property
        // categorization. Return a minimal report flagged as raw.
        return Ok(Some(SchemaStatusReport {
            source_path: source.resolved_path.clone(),
            required: Vec::new(),
            optional: Vec::new(),
            has_invalid_optional: false,
            raw_json_schema: true,
        }));
    };

    // Build a frontmatter view with `set_overrides` applied so the
    // validation result reflects what the user has supplied on the
    // command line so far.
    let mut fm_map: serde_json::Map<String, serde_json::Value> = source
        .markdown
        .frontmatter()
        .as_map()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if let Some(serde_json::Value::Object(overrides)) = set_overrides {
        for (k, v) in overrides {
            fm_map.insert(k.clone(), v.clone());
        }
    }

    let instance = serde_json::Value::Object(fm_map.clone());
    let report = effective.validate(&instance);

    // Walk problems and build a per-property index keyed by top-level
    // segment / property name.
    let mut missing_by_name: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    let mut invalid_by_name: std::collections::BTreeSet<String> =
        std::collections::BTreeSet::new();
    for problem in &report.problems {
        match problem.kind {
            ValidationProblemKind::Missing => {
                let name = problem
                    .property
                    .clone()
                    .unwrap_or_else(|| problem.path.clone());
                missing_by_name.insert(name);
            }
            ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
                if let Some(name) = top_level_pointer_segment(&problem.path) {
                    invalid_by_name.insert(name);
                }
            }
        }
    }

    let shape = match effective.simplified.as_ref() {
        Some(SimplifiedSchema::Single(s)) => s,
        // Root-level unions don't expose a single property table; report
        // as raw so the CLI falls back to a minimal listing.
        _ => {
            return Ok(Some(SchemaStatusReport {
                source_path: source.resolved_path.clone(),
                required: Vec::new(),
                optional: Vec::new(),
                has_invalid_optional: false,
                raw_json_schema: true,
            }));
        }
    };

    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut has_invalid_optional = false;

    for (name, def) in &shape.properties {
        let is_present = fm_map.contains_key(name);
        let is_missing = missing_by_name.contains(name);
        let is_invalid = invalid_by_name.contains(name);
        let state = if is_invalid {
            PropertyState::Invalid
        } else if is_missing || !is_present {
            PropertyState::Missing
        } else {
            PropertyState::Valid
        };

        let (type_label, description) = match def {
            PropertyDef::Single(atom) => (type_label_for_atom(atom), atom.description.clone()),
            PropertyDef::Union(_) => (
                "<union>".to_string(),
                None,
            ),
        };

        let entry = PropertyStatus {
            name: name.clone(),
            type_label,
            description,
            state,
        };

        if is_required(Some(shape), name) {
            required.push(entry);
        } else {
            if state == PropertyState::Invalid {
                has_invalid_optional = true;
            }
            optional.push(entry);
        }
    }

    Ok(Some(SchemaStatusReport {
        source_path: source.resolved_path.clone(),
        required,
        optional,
        has_invalid_optional,
        raw_json_schema: false,
    }))
}

fn top_level_pointer_segment(pointer: &str) -> Option<String> {
    let stripped = pointer.strip_prefix('/')?;
    let first = stripped.split('/').next()?;
    if first.is_empty() {
        return None;
    }
    Some(first.replace("~1", "/").replace("~0", "~"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::resolve::resolve_composition_source;
    use std::fs;
    use tempfile::TempDir;

    fn make_source(dir: &TempDir, document: &str) -> ResolvedCompositionSource {
        let file = dir.path().join("test.md");
        fs::write(&file, document).unwrap();
        resolve_composition_source(file.to_str().unwrap()).unwrap()
    }

    #[test]
    fn interactive_options_allowed_only_when_all_flags_true() {
        let permissive = InteractiveSchemaOptions {
            prompt_for_missing: true,
            stdin_is_tty: true,
            stderr_is_tty: true,
            silent: false,
        };
        assert!(permissive.allowed());

        assert!(
            !InteractiveSchemaOptions {
                silent: true,
                ..permissive
            }
            .allowed()
        );
        assert!(
            !InteractiveSchemaOptions {
                stdin_is_tty: false,
                ..permissive
            }
            .allowed()
        );
        assert!(
            !InteractiveSchemaOptions {
                stderr_is_tty: false,
                ..permissive
            }
            .allowed()
        );
        assert!(
            !InteractiveSchemaOptions {
                prompt_for_missing: false,
                ..permissive
            }
            .allowed()
        );
    }

    #[test]
    fn interactive_options_default_is_denied() {
        let opts = InteractiveSchemaOptions::default();
        assert!(!opts.allowed());
    }

    #[test]
    fn no_schema_passes_through_unchanged() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, "---\ntitle: Hello\n---\nbody\n");

        let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
        assert!(prepared.prompt.contains("body"));
    }

    #[test]
    fn valid_required_property_passes() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\ntitle: Plan a feature\n---\nbody\n",
        );

        let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();
        assert_eq!(
            fm.get("title").and_then(|v| v.as_str()),
            Some("Plan a feature")
        );
    }

    #[test]
    fn missing_required_returns_missing_properties_error() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        );

        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties {
                missing,
                pointer_paths,
                ..
            } => {
                assert_eq!(missing.len(), 1);
                assert_eq!(missing[0].name, "title");
                assert_eq!(missing[0].type_label.as_deref(), Some("string"));
                assert!(!pointer_paths.is_empty());
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn invalid_required_returns_schema_validation_error() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  count: 'number(required)'\ncount: not-a-number\n---\nbody\n",
        );

        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        assert!(
            matches!(err, CompositionError::SchemaValidation { .. }),
            "got: {err:?}"
        );
    }

    #[test]
    fn invalid_optional_is_dropped_and_retried() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n  count: 'number'\ntitle: Plan\ncount: not-a-number\n---\nbody\n",
        );

        let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Plan"));
        assert!(
            !fm.contains_key("count"),
            "invalid optional `count` should have been dropped"
        );
    }

    #[test]
    fn invalid_optional_drop_leaves_missing_required_surfaced() {
        // The optional `count` is invalid AND a different required value
        // is missing. After the drop+retry, the missing-required error
        // should surface so the user (or interactive loop) can fix it.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n  count: 'number'\ncount: not-a-number\n---\nbody\n",
        );

        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert_eq!(missing.len(), 1);
                assert_eq!(missing[0].name, "title");
            }
            other => panic!("expected MissingProperties after drop+retry, got {other:?}"),
        }
    }

    #[test]
    fn schema_load_error_for_invalid_schema_value() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, "---\n$schema: 42\n---\nbody\n");

        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        assert!(
            matches!(err, CompositionError::SchemaLoad { .. }),
            "got: {err:?}"
        );
    }

    #[test]
    fn missing_required_surfaces_description_metadata() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required) -> The page title'\n---\nbody\n",
        );

        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert_eq!(missing[0].description.as_deref(), Some("The page title"));
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_required_surfaces_frontmatter_description() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\ndescription: Plan a feature implementation\n---\nbody\n",
        );

        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties {
                frontmatter_description,
                ..
            } => {
                assert_eq!(
                    frontmatter_description.as_deref(),
                    Some("Plan a feature implementation")
                );
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn enum_missing_required_includes_members_in_type_label() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\n---\nbody\n",
        );

        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                let label = missing[0]
                    .type_label
                    .as_deref()
                    .expect("expected typed enum label");
                assert!(label.starts_with("enum("), "got: {label}");
                assert!(label.contains("small"), "got: {label}");
                assert!(label.contains("large"), "got: {label}");
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn set_overrides_can_supply_missing_required() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        );

        let options = PrepareOptions {
            set_overrides: Some(serde_json::json!({ "title": "Plan" })),
            ..Default::default()
        };
        let prepared = prepare_direct_with_schema(&source, options).unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Plan"));
    }

    #[test]
    fn inline_compose_with_schema_validates_after_prompt_check() {
        // `inline-compose` already requires a frontmatter `prompt` property.
        // Schema validation runs after that check; absent `prompt` still
        // surfaces as `PromptPropertyMissing` rather than as a generic
        // schema problem.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  prompt: 'string(required)'\n---\nbody\n",
        );

        let err = prepare_inline_with_schema(&source, PrepareOptions::default()).unwrap_err();
        // The schema declares `prompt` as required, but `prepare_inline`
        // checks `PromptPropertyMissing` first against the raw source.
        // Darkmatter, however, runs schema validation during compose for
        // direct paths; for inline the temp markdown is built with
        // `fm.clone()` and re-validated. Either error is acceptable here;
        // we just want to make sure a typed error surfaces.
        assert!(
            matches!(
                err,
                CompositionError::PromptPropertyMissing
                    | CompositionError::MissingProperties { .. }
            ),
            "got: {err:?}"
        );
    }

    #[test]
    fn inline_compose_with_valid_prompt_and_schema_succeeds() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  prompt: 'string(required)'\nprompt: List three colors\n---\nbody\n",
        );

        let prepared = prepare_inline_with_schema(&source, PrepareOptions::default()).unwrap();
        assert!(prepared.prompt.contains("List three colors"));
    }

    // -- interactive_shape -----------------------------------------------

    #[test]
    fn missing_string_property_maps_to_text_plain_shape() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert_eq!(
                    missing[0].interactive_shape,
                    Some(InteractiveShape::Text {
                        format: TextFormat::Plain
                    })
                );
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_number_property_maps_to_number_shape() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  count: 'number(required; integer)'\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert_eq!(
                    missing[0].interactive_shape,
                    Some(InteractiveShape::Number { integer: true })
                );
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_boolean_property_maps_to_boolean_shape() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  ready: 'boolean(required)'\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert_eq!(missing[0].interactive_shape, Some(InteractiveShape::Boolean));
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_enum_property_maps_to_enum_one_shape() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  tier: 'enum(small, medium, large; required)'\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => match &missing[0]
                .interactive_shape
            {
                Some(InteractiveShape::EnumOne { members }) => {
                    assert_eq!(members.len(), 3);
                    assert!(members.iter().any(|m| m == "small"));
                    assert!(members.iter().any(|m| m == "large"));
                }
                other => panic!("expected EnumOne shape, got {other:?}"),
            },
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_enum_array_property_maps_to_enum_many_shape() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  tags: 'enum(a, b, c)[](required)'\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert!(matches!(
                    missing[0].interactive_shape,
                    Some(InteractiveShape::EnumMany { .. })
                ));
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_file_property_maps_to_text_file_shape() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  template: 'file(required)'\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert_eq!(
                    missing[0].interactive_shape,
                    Some(InteractiveShape::Text {
                        format: TextFormat::File
                    })
                );
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_object_property_has_no_interactive_shape() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  config: 'object(required)'\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert_eq!(missing[0].interactive_shape, None);
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    // -- build_schema_status_report ---------------------------------------

    #[test]
    fn status_report_is_none_when_no_schema() {
        let dir = TempDir::new().unwrap();
        let source = make_source(&dir, "---\ntitle: hi\n---\nbody\n");
        let report = build_schema_status_report(&source, None).unwrap();
        assert!(report.is_none());
    }

    #[test]
    fn status_report_categorizes_required_and_optional() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n  description: 'string'\ntitle: Plan\n---\nbody\n",
        );
        let report = build_schema_status_report(&source, None).unwrap().unwrap();
        assert_eq!(report.required.len(), 1);
        assert_eq!(report.required[0].name, "title");
        assert_eq!(report.required[0].state, PropertyState::Valid);
        assert_eq!(report.optional.len(), 1);
        assert_eq!(report.optional[0].name, "description");
        assert_eq!(report.optional[0].state, PropertyState::Missing);
    }

    #[test]
    fn status_report_marks_missing_required_correctly() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        );
        let report = build_schema_status_report(&source, None).unwrap().unwrap();
        assert_eq!(report.required[0].state, PropertyState::Missing);
    }

    #[test]
    fn status_report_marks_invalid_required_correctly() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  count: 'number(required)'\ncount: not-a-number\n---\nbody\n",
        );
        let report = build_schema_status_report(&source, None).unwrap().unwrap();
        assert_eq!(report.required[0].state, PropertyState::Invalid);
    }

    #[test]
    fn status_report_overrides_supply_missing_required() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        );
        let overrides = serde_json::json!({ "title": "supplied" });
        let report = build_schema_status_report(&source, Some(&overrides))
            .unwrap()
            .unwrap();
        assert_eq!(report.required[0].state, PropertyState::Valid);
    }

    #[test]
    fn status_report_flags_invalid_optional() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n  count: 'number'\ntitle: Plan\ncount: nope\n---\nbody\n",
        );
        let report = build_schema_status_report(&source, None).unwrap().unwrap();
        assert!(report.has_invalid_optional);
    }

    #[test]
    fn text_format_label_returns_human_strings() {
        assert_eq!(TextFormat::Plain.label(), "string");
        assert_eq!(TextFormat::Date.label(), "date (YYYY-MM-DD)");
        assert!(TextFormat::File.label().contains("file"));
    }

    #[test]
    fn top_level_pointer_segment_handles_escaped_keys() {
        assert_eq!(
            top_level_pointer_segment("/title"),
            Some("title".to_string())
        );
        assert_eq!(
            top_level_pointer_segment("/nested/inner"),
            Some("nested".to_string())
        );
        assert_eq!(
            top_level_pointer_segment("/has~1slash"),
            Some("has/slash".to_string())
        );
        assert_eq!(top_level_pointer_segment(""), None);
        assert_eq!(top_level_pointer_segment("/"), None);
    }
}
