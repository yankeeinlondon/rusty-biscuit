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
//! and composition is retried once. After composition succeeds, the
//! post-shell-expanded effective frontmatter is re-validated against the
//! same schema so values produced by frontmatter `$(...)` expressions are
//! judged on their final form. Dropped properties are captured as
//! [`super::error::DroppedOptional`] entries on the returned
//! [`PreparedComposition`] / [`PreValidatedSchema`] so the CLI can render a
//! user-visible warning to stderr; `tracing::warn!` events still fire for
//! anyone running with structured tracing enabled.
//!
//! See [`claudine/features/2026-05-15-schemas/plan.md`][plan] for the
//! Phase 2 contract.
//!
//! [plan]: ../../../../features/2026-05-15-schemas/plan.md

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, EffectiveSchema, PropertyAtom, PropertyDef, SchemaError,
    SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr, ValidationProblem,
    ValidationProblemKind,
};

use super::error::{
    CompositionError, DroppedOptional, DroppedOptionalSource, DroppedOptionalStage,
    InteractiveShape, MissingProperty, TextFormat,
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
    /// The decision to prompt depends **only** on the four documented
    /// signals held in this struct:
    ///
    /// - `prompt_for_missing` (user-configurable default `true`)
    /// - `stdin_is_tty`
    /// - `stderr_is_tty`
    /// - `!silent`
    ///
    /// It **must not** depend on the resolved session interactivity value
    /// (`session_interactive`). Schema collection is a pre-session
    /// preparation step: this function runs (and any interactive prompt
    /// completes) before the provider child process is spawned, regardless
    /// of whether the eventual session mode is interactive or
    /// non-interactive. Callers must ensure that ordering.
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
    let mut dropped: Vec<DroppedOptional> = Vec::new();
    let file_ref_fallback_dir = options.file_ref_fallback_dir.clone();
    let prepared = match run_prepare(source, options.clone(), mode) {
        Ok(prepared) => prepared,
        Err(err) => handle_compose_error(source, options, mode, err, &mut dropped)?,
    };
    // Re-validate the post-shell-expanded effective frontmatter and apply
    // the same typed error / drop-and-retry rules so values that became
    // invalid (or now satisfy the schema) after `$(...)` expansion are
    // judged on their final form.
    post_shell_validate(source, prepared, dropped, mode, file_ref_fallback_dir.as_deref())
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

/// Classify a schema preparation failure into the typed
/// [`CompositionError::SchemaParse`] (a body-syntax error) or
/// [`CompositionError::SchemaLoad`] (a reference-resolution error), keyed on the
/// typed [`SchemaError`] cause.
///
/// Grammar / conversion / shape errors are body-syntax problems and carry the
/// constraint-grammar remediation; everything else (missing file, remote URL,
/// ambiguous reference, I/O, validator construction) — and the no-typed-cause
/// case — keeps the path-focused `SchemaLoad` with `fallback_message`.
///
/// [`SchemaError`]: darkmatter::markdown::schemas::SchemaError
fn schema_error_to_composition_error(
    source_path: &std::path::Path,
    fallback_message: String,
    schema_error: Option<&SchemaError>,
) -> CompositionError {
    // Grammar/Convert attach a synthetic name (`<root>`, `<arm[N]>`) for purely
    // structural failures; only a real, user-addressable property is worth
    // surfacing as scope.
    fn real_property(name: &str) -> Option<String> {
        (!name.starts_with('<')).then(|| name.to_string())
    }

    match schema_error {
        Some(SchemaError::Grammar {
            property,
            message,
            span,
        }) => CompositionError::SchemaParse {
            source_path: source_path.to_path_buf(),
            property: real_property(property),
            message: message.clone(),
            span: Some(span.clone()),
        },
        Some(SchemaError::Convert { property, message }) => CompositionError::SchemaParse {
            source_path: source_path.to_path_buf(),
            property: real_property(property),
            message: message.clone(),
            span: None,
        },
        Some(SchemaError::FrontmatterShape { message }) => CompositionError::SchemaParse {
            source_path: source_path.to_path_buf(),
            property: None,
            message: message.clone(),
            span: None,
        },
        _ => CompositionError::SchemaLoad {
            source_path: source_path.to_path_buf(),
            message: fallback_message,
        },
    }
}

fn handle_compose_error(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode,
    err: CompositionError,
    dropped: &mut Vec<DroppedOptional>,
) -> Result<PreparedComposition, CompositionError> {
    let CompositionError::ComposeFailed(markdown_err) = err else {
        return Err(err);
    };

    match markdown_err {
        MarkdownError::SchemaValidationFailed {
            problems,
            summary,
            source: schema_source,
            ..
        } => translate_schema_failure(
            source,
            options,
            mode,
            problems,
            summary,
            schema_source,
            dropped,
        ),
        other => Err(CompositionError::ComposeFailed(other)),
    }
}

fn translate_schema_failure(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode,
    problems: Vec<ValidationProblem>,
    summary: String,
    schema_source: Option<Box<dyn std::error::Error + Send + Sync + 'static>>,
    dropped: &mut Vec<DroppedOptional>,
) -> Result<PreparedComposition, CompositionError> {
    // Empty problems list signals a preparation failure: either a *parse* error
    // in the schema body, or a *reference-resolution* failure. The typed
    // `SchemaError` carried on the source distinguishes them — a grammar/convert/
    // shape error is a body-syntax problem (`SchemaParse`, with constraint-grammar
    // remediation), whereas a missing file / remote URL / ambiguous reference is a
    // resolution failure (`SchemaLoad`, with path remediation). Without a typed
    // cause, fall back to `SchemaLoad`.
    if problems.is_empty() {
        let typed = schema_source
            .as_deref()
            .and_then(|err| err.downcast_ref::<SchemaError>());
        return Err(schema_error_to_composition_error(
            &source.resolved_path,
            summary,
            typed,
        ));
    }

    let effective = load_effective_schema(source, options.file_ref_fallback_dir.as_deref())?;
    let categorized = categorize_problems(&problems, effective.as_ref());

    if !categorized.invalid_required.is_empty() {
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_required,
        ));
    }

    if !categorized.invalid_optional.is_empty() {
        let droppable =
            filter_droppable_invalid_optionals(&categorized.invalid_optional, effective.as_ref());
        if droppable.len() != categorized.invalid_optional.len() {
            let hard: Vec<_> = categorized
                .invalid_optional
                .iter()
                .filter(|problem| {
                    !droppable.iter().any(|droppable_problem| {
                        droppable_problem.path == problem.path
                            && droppable_problem.message == problem.message
                    })
                })
                .cloned()
                .collect();
            return Err(build_schema_validation_error(&source.resolved_path, &hard));
        }
        // Drop invalid optionals from a clone of the source AND from the
        // run's `set_overrides` map. Source-only removal is not enough:
        // overrides land on top of frontmatter during compose, so a bad
        // `key=value` or `--set` value would otherwise re-fail validation.
        // Retry the prepare step exactly once. If composition still fails,
        // fall through to surface the residual problem (likely a missing
        // required property).
        let (retry_source, source_drops) = source_with_dropped_optionals(source, &droppable);
        let (retry_options, override_drops) =
            options_with_dropped_optionals(options.clone(), &droppable);
        dropped.extend(source_drops);
        dropped.extend(override_drops);
        return match run_prepare(&retry_source, retry_options, mode) {
            Ok(prepared) => Ok(prepared),
            Err(retry_err) => {
                handle_retry_error(source, retry_err, options.file_ref_fallback_dir.as_deref())
            }
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
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<PreparedComposition, CompositionError> {
    let CompositionError::ComposeFailed(MarkdownError::SchemaValidationFailed {
        problems,
        summary,
        ..
    }) = err
    else {
        return Err(err);
    };

    let effective = load_effective_schema(source, file_ref_fallback_dir)?;
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

/// Validate `prepared.effective_frontmatter` after Darkmatter composition
/// (including frontmatter shell expansion) has finished, and apply the
/// same typed error / drop-and-retry rules as the prepare-time validator.
///
/// Darkmatter's compose pipeline runs schema validation AFTER frontmatter
/// interpolation but BEFORE frontmatter shell expansion (see
/// `darkmatter::markdown::compose::schema_validation`). Values produced by
/// `$(...)` expressions therefore never satisfy or violate the schema
/// during compose itself. This step closes that gap: it validates the
/// final effective frontmatter against the resolved schema and:
///
/// - Returns [`CompositionError::SchemaValidation`] if a required value
///   is now invalid (e.g. shell command produced bad output).
/// - Returns [`CompositionError::MissingProperties`] if a required value
///   is now missing (e.g. shell command emitted empty string and the
///   schema required a non-empty value).
/// - Drops invalid optionals from `effective_frontmatter` in place,
///   tracks each as a [`DroppedOptional`] tagged
///   [`DroppedOptionalStage::PostShellExpansion`].
fn post_shell_validate(
    source: &ResolvedCompositionSource,
    mut prepared: PreparedComposition,
    mut dropped: Vec<DroppedOptional>,
    _mode: PrepareMode,
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<PreparedComposition, CompositionError> {
    // No `$schema` → no validation work to do.
    if !source
        .markdown
        .frontmatter()
        .as_map()
        .contains_key("$schema")
    {
        prepared.dropped_optionals = dropped;
        return Ok(prepared);
    }

    let Some(effective) = load_effective_schema(source, file_ref_fallback_dir)? else {
        // Raw JSON Schema (no SimplifiedSchema): nothing else to do here.
        prepared.dropped_optionals = dropped;
        return Ok(prepared);
    };

    let report = effective.validate(&prepared.effective_frontmatter);
    if report.valid {
        prepared.dropped_optionals = dropped;
        return Ok(prepared);
    }

    let categorized = categorize_problems(&report.problems, Some(&effective));

    if !categorized.invalid_required.is_empty() {
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_required,
        ));
    }

    if !categorized.invalid_optional.is_empty() {
        let droppable = filter_droppable_invalid_optionals(&categorized.invalid_optional, Some(&effective));
        if droppable.len() != categorized.invalid_optional.len() {
            let hard: Vec<_> = categorized
                .invalid_optional
                .iter()
                .filter(|problem| {
                    !droppable.iter().any(|droppable_problem| {
                        droppable_problem.path == problem.path
                            && droppable_problem.message == problem.message
                    })
                })
                .cloned()
                .collect();
            return Err(build_schema_validation_error(&source.resolved_path, &hard));
        }
        // Drop invalid optionals from the composed effective frontmatter
        // and revalidate. Track each as a post-shell drop so the CLI can
        // surface a warning to the user.
        let map = match prepared.effective_frontmatter.as_object_mut() {
            Some(m) => m,
            None => {
                prepared.dropped_optionals = dropped;
                return Ok(prepared);
            }
        };
        for problem in &droppable {
            let Some(name) = top_level_pointer_segment(&problem.path) else {
                continue;
            };
            if map.remove(&name).is_some() {
                tracing::warn!(
                    property = %name,
                    pointer = %problem.path,
                    message = %problem.message,
                    "dropping optional schema property with invalid value (post-shell expansion)",
                );
                dropped.push(DroppedOptional {
                    property: name,
                    source: DroppedOptionalSource::Composed,
                    stage: DroppedOptionalStage::PostShellExpansion,
                    reason: problem.message.clone(),
                });
            }
        }

        // Re-validate after dropping invalid optionals. If a required
        // value is still missing or invalid, surface it.
        let report2 = effective.validate(&prepared.effective_frontmatter);
        if !report2.valid {
            let categorized2 = categorize_problems(&report2.problems, Some(&effective));
            if !categorized2.invalid_required.is_empty() {
                return Err(build_schema_validation_error(
                    &source.resolved_path,
                    &categorized2.invalid_required,
                ));
            }
            if !categorized2.missing_required.is_empty() {
                return Err(build_missing_properties_error(
                    source,
                    &categorized2.missing_required,
                    &categorized2.pointer_paths,
                ));
            }
        }

        prepared.dropped_optionals = dropped;
        return Ok(prepared);
    }

    if !categorized.missing_required.is_empty() {
        return Err(build_missing_properties_error(
            source,
            &categorized.missing_required,
            &categorized.pointer_paths,
        ));
    }

    // Fall-through: residual problems we couldn't classify. Surface a
    // generic SchemaValidation so the user still sees the failure.
    Err(CompositionError::SchemaValidation {
        source_path: source.resolved_path.clone(),
        message: "post-shell schema validation failed".to_string(),
        problems: report.problems.iter().map(|p| p.path.clone()).collect(),
    })
}

fn load_effective_schema(
    source: &ResolvedCompositionSource,
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<Option<EffectiveSchema>, CompositionError> {
    let mut schemas = DarkmatterSchemas::new();
    if let Some(fallback) = file_ref_fallback_dir {
        schemas = schemas.with_file_ref_fallback_dir(fallback);
    }
    schemas.effective_for(&source.markdown).map_err(|err| {
        // A grammar/convert/shape error is a body-syntax problem (`SchemaParse`);
        // a missing file / remote URL / ambiguous reference stays `SchemaLoad`.
        // `effective_for` hands us the typed cause directly — no downcast needed.
        schema_error_to_composition_error(&source.resolved_path, err.to_string(), Some(&err))
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
/// the markdown's frontmatter. Returns the modified source plus a record
/// of every property that was actually elided.
fn source_with_dropped_optionals(
    source: &ResolvedCompositionSource,
    invalid_optional: &[ValidationProblem],
) -> (ResolvedCompositionSource, Vec<DroppedOptional>) {
    let mut clone = source.clone();
    let map = clone.markdown.frontmatter_mut().as_map_mut();
    let mut drops = Vec::new();
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
            drops.push(DroppedOptional {
                property: name,
                source: DroppedOptionalSource::Frontmatter,
                stage: DroppedOptionalStage::Composition,
                reason: problem.message.clone(),
            });
        }
    }
    (clone, drops)
}

/// Build a clone of `options` with each invalid-optional key removed from
/// the run's `set_overrides` JSON object. Returns the modified options
/// plus a record of every override that was elided.
///
/// CLI-supplied values (`key=value` setters and `--set` JSON) land in
/// `set_overrides` and are applied on top of frontmatter by Darkmatter's
/// compose stage. Dropping them on retry mirrors the source-level drop so
/// run-scoped invalid optionals follow the same rule as file-authored
/// invalid optionals: warn, elide, re-compose, re-validate.
fn options_with_dropped_optionals(
    mut options: PrepareOptions,
    invalid_optional: &[ValidationProblem],
) -> (PrepareOptions, Vec<DroppedOptional>) {
    let mut drops = Vec::new();
    let Some(serde_json::Value::Object(ref mut map)) = options.set_overrides else {
        return (options, drops);
    };
    for problem in invalid_optional {
        let Some(name) = top_level_pointer_segment(&problem.path) else {
            continue;
        };
        if map.remove(&name).is_some() {
            tracing::warn!(
                property = %name,
                pointer = %problem.path,
                message = %problem.message,
                "dropping optional schema override with invalid value",
            );
            drops.push(DroppedOptional {
                property: name,
                source: DroppedOptionalSource::Override,
                stage: DroppedOptionalStage::Composition,
                reason: problem.message.clone(),
            });
        }
    }
    (options, drops)
}

fn filter_droppable_invalid_optionals(
    invalid_optional: &[ValidationProblem],
    effective: Option<&EffectiveSchema>,
) -> Vec<ValidationProblem> {
    let shape: Option<&SchemaShape> = effective.and_then(|e| match e.simplified.as_ref() {
        Some(SimplifiedSchema::Single(s)) => Some(s),
        Some(SimplifiedSchema::Union(_)) | None => None,
    });
    invalid_optional
        .iter()
        .filter(|problem| !is_eager_file_problem(shape, problem))
        .cloned()
        .collect()
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

/// Classify a validation problem set as a
/// [`CompositionError::UnresolvedFileReference`] when a **provided** value for
/// a `file`/`file[]` property with non-empty `match(...)` patterns failed
/// existence resolution (Darkmatter's `NoMatch` case).
///
/// This is the read-side twin of the missing-property classification: instead
/// of the value being *absent*, the user supplied a value best interpreted as a
/// **partial** — a substring to match against the property's `match(...)` glob
/// candidates. The CLI catches this variant and drives a confirmation dialog
/// (single match) or chooser (multiple), mirroring the missing-property loop.
///
/// Returns `None` when no problem qualifies, so the caller falls back to the
/// generic [`CompositionError::SchemaValidation`]. Only the first qualifying
/// property is surfaced; the interactive retry re-runs validation and picks up
/// any remaining ones one at a time.
fn classify_unresolved_file_reference(
    source_path: &std::path::Path,
    problems: &[ValidationProblem],
    effective: Option<&EffectiveSchema>,
    instance: &serde_json::Value,
) -> Option<CompositionError> {
    let shape = match effective?.simplified.as_ref()? {
        SimplifiedSchema::Single(s) => s,
        SimplifiedSchema::Union(_) => return None,
    };
    for problem in problems {
        // Only Darkmatter's `NoMatch` ("no existing file matched reference")
        // is a resolvable partial — a parse/resolution error is a genuinely
        // bad value that a glob walk cannot rescue.
        if !problem.message.contains("no existing file matched reference") {
            continue;
        }
        let Some(name) = top_level_pointer_segment(&problem.path) else {
            continue;
        };
        let Some(atom) = atom_for_property(shape, &name) else {
            continue;
        };
        if !matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File)) {
            continue;
        }
        let patterns: Vec<String> = atom
            .constraints
            .iter()
            .find_map(|c| match c {
                Constraint::Match(p) => Some(p.clone()),
                _ => None,
            })
            .unwrap_or_default();
        // A bare `file` (no glob) has nothing to walk — leave it to the generic
        // validation path.
        if patterns.is_empty() {
            continue;
        }
        let Some(provided) = provided_partial_value(instance.get(&name)) else {
            continue;
        };
        return Some(CompositionError::UnresolvedFileReference {
            source_path: source_path.to_path_buf(),
            property: name,
            provided,
            patterns,
            is_array: atom.is_array,
            reason: problem.message.clone(),
        });
    }
    None
}

/// Extract the user-provided partial from a frontmatter/override value.
///
/// Returns the string for a scalar `file` value (the substring to match against
/// the `match(...)` glob candidates). For a `file[]` value, accepts an array of
/// strings and uses the first non-empty string as the partial; a scalar string
/// is treated as single-element intent for convenience. Non-string array
/// elements or empty arrays are rejected and left to the generic
/// schema-validation path.
fn provided_partial_value(value: Option<&serde_json::Value>) -> Option<String> {
    match value? {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        serde_json::Value::Array(arr) => arr
            .iter()
            .find_map(|v| match v {
                serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
                _ => None,
            }),
        _ => None,
    }
}

fn is_eager_file_problem(shape: Option<&SchemaShape>, problem: &ValidationProblem) -> bool {
    if !matches!(problem.kind, ValidationProblemKind::Invalid | ValidationProblemKind::Type) {
        return false;
    }
    let Some(name) = top_level_pointer_segment(&problem.path) else {
        return false;
    };
    let Some(shape) = shape else {
        return false;
    };
    let Some(def) = shape.properties.get(&name) else {
        return false;
    };
    let atoms: Vec<&PropertyAtom> = match def {
        PropertyDef::Single(atom) => vec![atom],
        PropertyDef::Union(items) => items.iter().collect(),
    };
    atoms.iter().any(|atom| {
        matches!(atom.ty, TypeExpr::Primitive(SimplifiedType::File))
            && atom
                .constraints
                .iter()
                .any(|constraint| matches!(constraint, Constraint::Eager))
    })
}

/// Map a [`PropertyAtom`] to an [`InteractiveShape`] for CLI prompting.
///
/// Returns `None` when the atom describes a shape that cannot be
/// collected via a single TUI widget (e.g. `object`, `any`).
fn interactive_shape_for_atom(atom: &PropertyAtom) -> Option<InteractiveShape> {
    match &atom.ty {
        TypeExpr::Primitive(SimplifiedType::Enum) => {
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
        TypeExpr::Primitive(SimplifiedType::Boolean | SimplifiedType::Boolish) => {
            Some(InteractiveShape::Boolean)
        }
        TypeExpr::Primitive(SimplifiedType::Number | SimplifiedType::NumberLike) => {
            let integer = atom
                .constraints
                .iter()
                .any(|c| matches!(c, Constraint::Integer));
            let (min, max) = min_max_constraints(atom);
            Some(InteractiveShape::Number { integer, min, max })
        }
        TypeExpr::Primitive(
            SimplifiedType::String | SimplifiedType::Yaml | SimplifiedType::Json,
        ) => {
            let (min_len, max_len) = string_length_constraints(atom);
            Some(InteractiveShape::Text {
                format: TextFormat::Plain,
                min_len,
                max_len,
            })
        }
        TypeExpr::Primitive(SimplifiedType::Date) => Some(InteractiveShape::Text {
            format: TextFormat::Date,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::DateTime) => Some(InteractiveShape::Text {
            format: TextFormat::DateTime,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::Time) => Some(InteractiveShape::Text {
            format: TextFormat::Time,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::Url) => Some(InteractiveShape::Text {
            format: TextFormat::Url,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::Email) => Some(InteractiveShape::Text {
            format: TextFormat::Email,
            min_len: None,
            max_len: None,
        }),
        TypeExpr::Primitive(SimplifiedType::File) => {
            let patterns = atom
                .constraints
                .iter()
                .find_map(|c| match c {
                    Constraint::Match(p) => Some(p.clone()),
                    _ => None,
                })
                .unwrap_or_default();
            Some(InteractiveShape::File {
                is_array: atom.is_array,
                patterns,
            })
        }
        TypeExpr::Primitive(SimplifiedType::Object | SimplifiedType::Any)
        | TypeExpr::InlineObject(_)
        | TypeExpr::Imported { .. } => None,
    }
}

fn min_max_constraints(atom: &PropertyAtom) -> (Option<f64>, Option<f64>) {
    let mut min: Option<f64> = None;
    let mut max: Option<f64> = None;
    for c in &atom.constraints {
        match c {
            Constraint::Min(v) => min = Some(*v),
            Constraint::Max(v) => max = Some(*v),
            _ => {}
        }
    }
    (min, max)
}

fn string_length_constraints(atom: &PropertyAtom) -> (Option<usize>, Option<usize>) {
    let mut min_len: Option<usize> = None;
    let mut max_len: Option<usize> = None;
    for c in &atom.constraints {
        match c {
            Constraint::MinLen(v) => min_len = Some(*v),
            Constraint::MaxLen(v) => max_len = Some(*v),
            _ => {}
        }
    }
    (min_len, max_len)
}

fn type_label_for_atom(atom: &PropertyAtom) -> String {
    let suffix = if atom.is_array { "[]" } else { "" };
    match &atom.ty {
        TypeExpr::Primitive(SimplifiedType::Enum) => {
            let members = atom.constraints.iter().find_map(|c| match c {
                Constraint::Members(m) => Some(m.join("|")),
                _ => None,
            });
            match members {
                Some(m) => format!("enum({m}){suffix}"),
                None => format!("enum(){suffix}"),
            }
        }
        TypeExpr::Primitive(ty) => format!("{base}{suffix}", base = ty.as_keyword()),
        TypeExpr::InlineObject(_) => format!("object{suffix}"),
        TypeExpr::Imported { name, reference } => format!("{name}{suffix}@{reference}"),
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
/// `file_ref_fallback_dir` is the captured launch area used as the
/// document-first / launch-area-fallback anchor for `file`-typed property
/// resolution, matching the expression path. Pass `None` only when no launch
/// area is known (e.g. unit tests).
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
    file_ref_fallback_dir: Option<&std::path::Path>,
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

    let effective = load_effective_schema(source, file_ref_fallback_dir)?;
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
    //
    // Composition-tolerant: Type/Invalid verdicts against a raw value that
    // contains Darkmatter template syntax (`{{ ... }}`) are deferred — the
    // status report mirrors what the preflight + prepare pipeline will
    // ultimately see, which is the *effective* frontmatter after
    // composition. Flagging `runtime_agent: '{{ env.AGENT }}'` as Invalid
    // here would contradict the (correct) successful execution that
    // follows. See `features/2026-05-15-schemas/review-4.md`.
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
                let Some(name) = top_level_pointer_segment(&problem.path) else {
                    continue;
                };
                if value_needs_composition(fm_map.get(&name)) {
                    continue;
                }
                invalid_by_name.insert(name);
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

/// Outcome of [`pre_validate_schema`].
///
/// The validation runs purely against the effective frontmatter — no
/// shell expansion, no transclusion — so it is safe to call before the
/// pre-flight phase. Invalid optional values are dropped in place; the
/// returned source and overrides reflect the cleaned state. All other
/// schema failures surface as typed [`CompositionError`] variants so the
/// CLI can render Claudine's standard error blocks (and so the
/// interactive collection loop can drive a retry on `MissingProperties`).
#[derive(Debug)]
pub struct PreValidatedSchema {
    /// Source with invalid optional keys removed from frontmatter.
    pub source: ResolvedCompositionSource,
    /// Override map with invalid optional keys removed.
    pub set_overrides: Option<serde_json::Value>,
    /// Optional properties whose value failed pre-validation and were
    /// elided before the run continued. The CLI renders one user-visible
    /// warning per entry so silently dropped values are surfaced.
    pub dropped_optionals: Vec<DroppedOptional>,
}

/// Pre-prepare schema validation. Detects missing required properties in
/// the raw frontmatter (plus `set_overrides`) BEFORE Darkmatter's compose
/// pipeline runs, so the CLI can drive interactive collection of those
/// values before the (potentially slow) compose phase begins.
///
/// **Composition-tolerant by design.** Templates (`{{ env.AGENT }}`),
/// transclusions, and sequence overlays can supply or transform values
/// during composition. Surfacing an `Invalid` verdict against a template
/// value would reject prompts whose effective frontmatter is perfectly
/// valid — see `features/2026-05-15-schemas/review-3.md`. Each Invalid
/// problem is checked against the raw value:
///
/// - If the raw value contains template syntax (`{{ ... }}`), the
///   verdict is **deferred** to [`prepare_direct_with_schema`] /
///   [`prepare_inline_with_schema`], which validate the *effective*
///   frontmatter produced by Darkmatter.
/// - Otherwise the verdict is final and surfaces here.
///
/// Invalid optional **non-template** values are dropped from the source
/// and overrides before pre-validation completes (with `tracing::warn!`),
/// so the downstream preflight + prepare pipeline does not re-encounter
/// the same problem.
///
/// Outcome rules:
///
/// - **No `$schema`** → returned unchanged.
/// - **Invalid optional** (non-template) → dropped from source frontmatter
///   and overrides.
/// - **Invalid optional** (template) → returned unchanged; deferred.
/// - **Invalid required** (non-template) → returned as
///   [`CompositionError::SchemaValidation`].
/// - **Invalid required** (template) → returned unchanged; deferred.
/// - **Missing required** → returned as
///   [`CompositionError::MissingProperties`].
/// - **Schema load failure** → returned as
///   [`CompositionError::SchemaLoad`].
///
/// ## Errors
///
/// Returns one of the typed `CompositionError` variants listed above when
/// pre-validation cannot proceed. The caller is expected to either render
/// the typed error or drive interactive collection on `MissingProperties`.
pub fn pre_validate_schema(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<PreValidatedSchema, CompositionError> {
    let no_schema = !source
        .markdown
        .frontmatter()
        .as_map()
        .contains_key("$schema");
    if no_schema {
        return Ok(PreValidatedSchema {
            source: source.clone(),
            set_overrides: set_overrides.cloned(),
            dropped_optionals: Vec::new(),
        });
    }

    // First pass: drop non-template invalid optionals so the prepare-time
    // pipeline (and the preflight Darkmatter pass) sees a clean slate.
    let (source, set_overrides, dropped_optionals) =
        drop_invalid_optionals(source.clone(), set_overrides.cloned(), file_ref_fallback_dir);

    let effective = match load_effective_schema(&source, file_ref_fallback_dir) {
        Ok(Some(e)) => e,
        Ok(None) => {
            // Raw JSON Schema (no SimplifiedSchema projection): we cannot
            // distinguish required from optional, so we cannot decide
            // which problems to surface. Defer all checks to the
            // prepare-time wrapper.
            return Ok(PreValidatedSchema {
                source,
                set_overrides,
                dropped_optionals,
            });
        }
        Err(err) => return Err(err),
    };

    let mut instance = build_effective_instance(&source, set_overrides.as_ref());
    normalize_file_array_values(&mut instance, Some(&effective));
    let report = effective.validate(&instance);
    if report.valid {
        return Ok(PreValidatedSchema {
            source,
            set_overrides,
            dropped_optionals,
        });
    }

    // Filter problems composition-tolerantly: drop Invalid/Type verdicts
    // whose raw value contains template syntax, because Darkmatter may
    // resolve them to valid values during composition. Missing verdicts
    // are composition-independent (no template can conjure a key that is
    // absent from both raw frontmatter and overrides).
    let instance_map = instance.as_object();
    let composition_independent: Vec<_> = report
        .problems
        .iter()
        .filter(|p| match p.kind {
            ValidationProblemKind::Missing => true,
            ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
                let raw = top_level_pointer_segment(&p.path)
                    .as_deref()
                    .and_then(|name| instance_map.and_then(|m| m.get(name)));
                !value_needs_composition(raw)
            }
        })
        .cloned()
        .collect();

    if composition_independent.is_empty() {
        return Ok(PreValidatedSchema {
            source,
            set_overrides,
            dropped_optionals,
        });
    }

    let categorized = categorize_problems(&composition_independent, Some(&effective));
    if !categorized.invalid_required.is_empty() {
        // A provided `file(match)` partial that failed existence resolution is
        // surfaced as the typed `UnresolvedFileReference` so the CLI can offer
        // a glob+substring confirmation/chooser, rather than the generic
        // wrong-type schema error.
        if let Some(err) = classify_unresolved_file_reference(
            &source.resolved_path,
            &categorized.invalid_required,
            Some(&effective),
            &instance,
        ) {
            return Err(err);
        }
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_required,
        ));
    }
    if !categorized.invalid_optional.is_empty() {
        // Eager-optional `file(match)` failures reach here too (they are kept,
        // not dropped); offer the same interactive resolution for a provided
        // partial before falling back to the generic schema error.
        if let Some(err) = classify_unresolved_file_reference(
            &source.resolved_path,
            &categorized.invalid_optional,
            Some(&effective),
            &instance,
        ) {
            return Err(err);
        }
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_optional,
        ));
    }
    if !categorized.missing_required.is_empty() {
        return Err(build_missing_properties_error(
            &source,
            &categorized.missing_required,
            &categorized.pointer_paths,
        ));
    }
    // Fall-through: only categories we couldn't act on remain. Surface a
    // generic SchemaValidation so the user still sees the failure.
    Err(CompositionError::SchemaValidation {
        source_path: source.resolved_path.clone(),
        message: "schema validation failed".to_string(),
        problems: composition_independent
            .iter()
            .map(|p| p.path.clone())
            .collect(),
    })
}

fn build_effective_instance(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
) -> serde_json::Value {
    let mut map: serde_json::Map<String, serde_json::Value> = source
        .markdown
        .frontmatter()
        .as_map()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if let Some(serde_json::Value::Object(overrides)) = set_overrides {
        for (k, v) in overrides {
            map.insert(k.clone(), v.clone());
        }
    }
    serde_json::Value::Object(map)
}

/// Normalize scalar string values into single-element arrays for `file[]`
/// schema properties.
///
/// This lets `attachments=everywhere` (parsed as a string) be treated as
/// single-element intent for a `file[]` property, matching the shorthand
/// behavior users expect and ensuring the value reaches the file-reference
/// classifier instead of failing with a type error.
fn normalize_file_array_values(
    instance: &mut serde_json::Value,
    effective: Option<&EffectiveSchema>,
) {
    let serde_json::Value::Object(map) = instance else {
        return;
    };
    let shape = match effective {
        Some(e) => match e.simplified.as_ref() {
            Some(SimplifiedSchema::Single(s)) => Some(s),
            _ => None,
        },
        _ => None,
    };
    let Some(shape) = shape else {
        return;
    };
    for (name, _atom) in shape.properties.iter().filter_map(|(n, def)| {
        match def {
            PropertyDef::Single(a) if a.is_array && matches!(a.ty, TypeExpr::Primitive(SimplifiedType::File)) => Some((n, a)),
            _ => None,
        }
    }) {
        let Some(value) = map.get_mut(name) else {
            continue;
        };
        if value.is_string() {
            *value = serde_json::Value::Array(vec![value.take()]);
        }
    }
}

/// Pre-scrub helper that drops invalid optional values from raw
/// frontmatter and overrides.
///
/// **Composition-tolerant.** Only drops values that are *definitely*
/// invalid: scalar literals whose runtime type does not match the schema
/// (e.g. `count: "not-a-number"` against `number`). Values containing
/// template syntax (`{{ ... }}`) are left in place, because Darkmatter's
/// compose pipeline can transform them into valid values during the
/// effective-frontmatter pass. Final validation of templated values
/// happens in [`prepare_direct_with_schema`] /
/// [`prepare_inline_with_schema`], which can still drop and retry against
/// the composed frontmatter.
///
/// Retained as a building block for [`pre_validate_schema`] and for
/// callers (notably `sequence`) that want to scrub doc-wide invalid
/// optionals before per-step validation.
pub fn drop_invalid_optionals(
    mut source: ResolvedCompositionSource,
    mut set_overrides: Option<serde_json::Value>,
    file_ref_fallback_dir: Option<&std::path::Path>,
) -> (
    ResolvedCompositionSource,
    Option<serde_json::Value>,
    Vec<DroppedOptional>,
) {
    let mut dropped: Vec<DroppedOptional> = Vec::new();
    if !source
        .markdown
        .frontmatter()
        .as_map()
        .contains_key("$schema")
    {
        return (source, set_overrides, dropped);
    }

    let effective = match load_effective_schema(&source, file_ref_fallback_dir) {
        Ok(Some(e)) => e,
        // No SimplifiedSchema projection (raw JSON Schema or schema load
        // failure) — let the prepare-time validator handle it.
        Ok(None) | Err(_) => return (source, set_overrides, dropped),
    };

    let mut instance: serde_json::Map<String, serde_json::Value> = source
        .markdown
        .frontmatter()
        .as_map()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    if let Some(serde_json::Value::Object(map)) = &set_overrides {
        for (k, v) in map {
            instance.insert(k.clone(), v.clone());
        }
    }

    let report = effective.validate(&serde_json::Value::Object(instance.clone()));
    if report.valid {
        return (source, set_overrides, dropped);
    }

    let shape: Option<&SchemaShape> = match effective.simplified.as_ref() {
        Some(SimplifiedSchema::Single(s)) => Some(s),
        _ => None,
    };

    // Property name → first reason seen, so we can attribute the warning
    // when the drop actually happens below.
    let mut to_drop: Vec<(String, String)> = Vec::new();
    for problem in &report.problems {
        match problem.kind {
            ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
                let Some(name) = top_level_pointer_segment(&problem.path) else {
                    continue;
                };
                if is_required(shape, &name) {
                    continue;
                }
                if is_eager_file_problem(shape, problem) {
                    continue;
                }
                // Composition-tolerant: skip values that look templated,
                // because Darkmatter may produce a valid value during
                // composition. The prepare-time validator will run
                // drop-and-retry against the composed value if it is
                // genuinely invalid.
                if value_needs_composition(instance.get(&name)) {
                    continue;
                }
                if !to_drop.iter().any(|(n, _)| n == &name) {
                    to_drop.push((name, problem.message.clone()));
                }
            }
            // Missing → required gap surfaced later. Invalid optional
            // missing is impossible by definition.
            ValidationProblemKind::Missing => {}
        }
    }

    if to_drop.is_empty() {
        return (source, set_overrides, dropped);
    }

    let fm = source.markdown.frontmatter_mut().as_map_mut();
    for (name, reason) in &to_drop {
        if fm.shift_remove(name).is_some() {
            tracing::warn!(
                property = %name,
                "dropping optional schema property with invalid value (pre-preflight)",
            );
            dropped.push(DroppedOptional {
                property: name.clone(),
                source: DroppedOptionalSource::Frontmatter,
                stage: DroppedOptionalStage::PreValidation,
                reason: reason.clone(),
            });
        }
    }

    if let Some(serde_json::Value::Object(map)) = &mut set_overrides {
        for (name, reason) in &to_drop {
            if map.remove(name).is_some() {
                tracing::warn!(
                    property = %name,
                    "dropping optional schema override with invalid value (pre-preflight)",
                );
                dropped.push(DroppedOptional {
                    property: name.clone(),
                    source: DroppedOptionalSource::Override,
                    stage: DroppedOptionalStage::PreValidation,
                    reason: reason.clone(),
                });
            }
        }
    }

    (source, set_overrides, dropped)
}

/// Returns `true` when `value` contains a Darkmatter template marker
/// (`{{ ... }}`) or a frontmatter shell expression (`$(...)`) somewhere
/// in any string descendant. Pre-validation must not drop or invalidate
/// such values, because composition (template interpolation and shell
/// expansion) may transform them into valid frontmatter.
fn value_needs_composition(value: Option<&serde_json::Value>) -> bool {
    let Some(value) = value else { return false };
    match value {
        serde_json::Value::String(s) => s.contains("{{") || s.contains("$("),
        serde_json::Value::Array(items) => items.iter().any(|v| value_needs_composition(Some(v))),
        serde_json::Value::Object(map) => map.values().any(|v| value_needs_composition(Some(v))),
        _ => false,
    }
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
mod tests;
