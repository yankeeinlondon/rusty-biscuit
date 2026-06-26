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
    Constraint, DarkmatterSchemas, EffectiveSchema, PropertyAtom, PropertyDef, SchemaShape,
    SimplifiedSchema, SimplifiedType, TypeExpr, ValidationProblem, ValidationProblemKind,
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
    let prepared = match run_prepare(source, options.clone(), mode) {
        Ok(prepared) => prepared,
        Err(err) => handle_compose_error(source, options, mode, err, &mut dropped)?,
    };
    // Re-validate the post-shell-expanded effective frontmatter and apply
    // the same typed error / drop-and-retry rules so values that became
    // invalid (or now satisfy the schema) after `$(...)` expansion are
    // judged on their final form.
    post_shell_validate(source, prepared, dropped, mode)
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
    dropped: &mut Vec<DroppedOptional>,
) -> Result<PreparedComposition, CompositionError> {
    let CompositionError::ComposeFailed(markdown_err) = err else {
        return Err(err);
    };

    match markdown_err {
        MarkdownError::SchemaValidationFailed {
            problems, summary, ..
        } => translate_schema_failure(source, options, mode, problems, summary, dropped),
        other => Err(CompositionError::ComposeFailed(other)),
    }
}

fn translate_schema_failure(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode,
    problems: Vec<ValidationProblem>,
    summary: String,
    dropped: &mut Vec<DroppedOptional>,
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
        // Drop invalid optionals from a clone of the source AND from the
        // run's `set_overrides` map. Source-only removal is not enough:
        // overrides land on top of frontmatter during compose, so a bad
        // `key=value` or `--set` value would otherwise re-fail validation.
        // Retry the prepare step exactly once. If composition still fails,
        // fall through to surface the residual problem (likely a missing
        // required property).
        let (retry_source, source_drops) =
            source_with_dropped_optionals(source, &categorized.invalid_optional);
        let (retry_options, override_drops) =
            options_with_dropped_optionals(options.clone(), &categorized.invalid_optional);
        dropped.extend(source_drops);
        dropped.extend(override_drops);
        return match run_prepare(&retry_source, retry_options, mode) {
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

    let Some(effective) = load_effective_schema(source)? else {
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
        for problem in &categorized.invalid_optional {
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
        TypeExpr::Primitive(SimplifiedType::String) => {
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
        | TypeExpr::InlineObject(_) => None,
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
        drop_invalid_optionals(source.clone(), set_overrides.cloned());

    let effective = match load_effective_schema(&source) {
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

    let instance = build_effective_instance(&source, set_overrides.as_ref());
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
        return Err(build_schema_validation_error(
            &source.resolved_path,
            &categorized.invalid_required,
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

    let effective = match load_effective_schema(&source) {
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
    fn invalid_optional_setter_is_dropped_and_retried() {
        // `count` is optional in the schema. When the user supplies a bad
        // value via `key=value` / `--set`, the override map must be
        // scrubbed alongside the source frontmatter so the retry succeeds.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n  count: 'number'\n---\nbody\n",
        );

        let options = PrepareOptions {
            set_overrides: Some(serde_json::json!({
                "title": "Plan",
                "count": "not-a-number",
            })),
            ..Default::default()
        };
        let prepared = prepare_direct_with_schema(&source, options).unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();
        assert_eq!(fm.get("title").and_then(|v| v.as_str()), Some("Plan"));
        assert!(
            !fm.contains_key("count"),
            "invalid optional override `count` should have been dropped"
        );
    }

    /// Phase 7 (acceptance criteria 10 + the reproduction fixture): a prompt
    /// with a user `$schema` *and* a lifecycle `failure.message: "{{err.msg}}"`
    /// validates its ordinary schema inputs exactly as today (DM1b: deferred
    /// lifecycle keys are excluded from user schema value validation) and still
    /// reaches lifecycle parsing with the late-binding span deferred raw.
    #[test]
    fn schema_validates_while_lifecycle_err_span_is_deferred() {
        let dir = TempDir::new().unwrap();
        // Mirrors `prompts/implement-plan.md`: required numeric schema inputs
        // alongside a `failure` block whose message references the late-binding
        // `err` global. The `{{err.msg}}` span must not be validated against the
        // user schema, must not fail composition, and must survive raw.
        let source = make_source(
            &dir,
            "---\n$schema:\n  phase: 'number(required)'\n  total_phases: 'number(required)'\nphase: 1\ntotal_phases: 3\nfailure:\n  message: \"❌️ phase {{phase}} failed: {{err.msg}}\"\n---\nbody\n",
        );

        let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();

        // Ordinary schema inputs validated and present.
        assert_eq!(fm.get("phase").and_then(|v| v.as_i64()), Some(1));
        assert_eq!(fm.get("total_phases").and_then(|v| v.as_i64()), Some(3));

        // The lifecycle key is deferred (DM1) and its span survives raw.
        assert!(
            prepared
                .deferred_lifecycle_keys
                .iter()
                .any(|k| k == "failure"),
            "failure should be reported as a deferred lifecycle key"
        );
        assert_eq!(
            prepared
                .lifecycle
                .failure
                .as_ref()
                .unwrap()
                .message
                .as_deref(),
            Some("❌️ phase {{phase}} failed: {{err.msg}}"),
            "lifecycle parsing sees the raw late-binding span after schema validation"
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

    // -- optional null acceptance (Phase 3) -------------------------------

    #[test]
    fn optional_string_resolved_to_null_passes_direct() {
        // Regression for the optional-schema-properties incident: an optional
        // `string` whose frontmatter ternary resolves to `null` must validate
        // successfully, and the resolved `null` must be retained in the
        // effective frontmatter rather than silently dropped.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            concat!(
                "---\n",
                "$schema:\n",
                "  design: 'string'\n",
                "design: \"{{ file_exists('design.md') ? 'design.md' : null }}\"\n",
                "---\nbody\n",
            ),
        );

        let prepared = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();
        assert!(
            fm.contains_key("design"),
            "optional property resolved to null must be retained"
        );
        assert_eq!(fm.get("design"), Some(&serde_json::Value::Null));
    }

    #[test]
    fn optional_string_resolved_to_null_passes_inline() {
        // Same null-retention contract on the inline-compose path, which also
        // requires a `prompt` property.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            concat!(
                "---\n",
                "$schema:\n",
                "  prompt: 'string(required)'\n",
                "  design: 'string'\n",
                "prompt: List three colors\n",
                "design: \"{{ file_exists('design.md') ? 'design.md' : null }}\"\n",
                "---\nbody\n",
            ),
        );

        let prepared = prepare_inline_with_schema(&source, PrepareOptions::default()).unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();
        assert!(
            fm.contains_key("design"),
            "optional property resolved to null must be retained in inline compose"
        );
        assert_eq!(fm.get("design"), Some(&serde_json::Value::Null));
    }

    #[test]
    fn required_string_resolved_to_null_fails_schema_validation() {
        // A required `string` whose ternary resolves to `null` must still be
        // classified as an invalid required value (Type problem), producing
        // `SchemaValidation`. If categorization read requiredness from the JSON
        // Schema instead of the `PropertyAtom`, the null could be treated as
        // "absent" and surface as `MissingProperties` instead.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            concat!(
                "---\n",
                "$schema:\n",
                "  design: 'string(required)'\n",
                "design: \"{{ file_exists('design.md') ? 'design.md' : null }}\"\n",
                "---\nbody\n",
            ),
        );

        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        assert!(
            matches!(err, CompositionError::SchemaValidation { .. }),
            "required property resolved to null must fail with SchemaValidation, got: {err:?}"
        );
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
                        format: TextFormat::Plain,
                        min_len: None,
                        max_len: None,
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
                    Some(InteractiveShape::Number {
                        integer: true,
                        min: None,
                        max: None,
                    })
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
    fn missing_file_property_maps_to_file_shape() {
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
                    Some(InteractiveShape::File {
                        is_array: false,
                        patterns: Vec::new(),
                    })
                );
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_file_array_property_maps_to_file_array_shape() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  attachments: 'file[](required)'\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => {
                assert_eq!(
                    missing[0].interactive_shape,
                    Some(InteractiveShape::File {
                        is_array: true,
                        patterns: Vec::new(),
                    })
                );
            }
            other => panic!("expected MissingProperties, got {other:?}"),
        }
    }

    #[test]
    fn missing_file_property_preserves_match_patterns() {
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  cover: \"file(match('*.png', '*.jpg'); required)\"\n---\nbody\n",
        );
        let err = prepare_direct_with_schema(&source, PrepareOptions::default()).unwrap_err();
        match err {
            CompositionError::MissingProperties { missing, .. } => match &missing[0]
                .interactive_shape
            {
                Some(InteractiveShape::File { patterns, is_array }) => {
                    assert!(!is_array);
                    assert_eq!(patterns, &["*.png", "*.jpg"]);
                }
                other => panic!("expected File shape, got {other:?}"),
            },
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
    fn status_report_does_not_mark_templated_required_as_invalid() {
        // Regression test for review-4 medium finding. The status report
        // runs against the *raw* frontmatter (no composition), so a
        // schema-constrained value supplied as a template expression
        // (`{{ env.AGENT }}`) would otherwise be flagged Invalid. The
        // preflight + prepare pipeline that executes immediately after
        // composes the frontmatter and finds the value valid — the
        // status report must agree.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            concat!(
                "---\n",
                "$schema:\n",
                "  runtime_agent: 'enum(goose; required)'\n",
                "  topic: 'string(required)'\n",
                "runtime_agent: '{{ env.AGENT }}'\n",
                "---\nbody\n",
            ),
        );
        let report = build_schema_status_report(&source, None).unwrap().unwrap();
        let runtime = report
            .required
            .iter()
            .find(|s| s.name == "runtime_agent")
            .expect("runtime_agent listed");
        assert_ne!(
            runtime.state,
            PropertyState::Invalid,
            "templated required value must not appear Invalid in the status report: {runtime:?}",
        );
        // The companion missing required must still appear Missing so the
        // user sees what they need to supply.
        let topic = report
            .required
            .iter()
            .find(|s| s.name == "topic")
            .expect("topic listed");
        assert_eq!(topic.state, PropertyState::Missing);
    }

    #[test]
    fn status_report_does_not_mark_templated_optional_as_invalid() {
        // Same composition-tolerance, applied to optional properties:
        // a templated optional value must not contribute to
        // `has_invalid_optional`, because the prepare pipeline will not
        // drop it.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            concat!(
                "---\n",
                "$schema:\n",
                "  title: 'string(required)'\n",
                "  variant: 'enum(small, medium, large)'\n",
                "title: Plan\n",
                "variant: '{{ env.SIZE }}'\n",
                "---\nbody\n",
            ),
        );
        let report = build_schema_status_report(&source, None).unwrap().unwrap();
        assert!(
            !report.has_invalid_optional,
            "templated optional must not be flagged invalid: {:?}",
            report.optional,
        );
        let variant = report
            .optional
            .iter()
            .find(|s| s.name == "variant")
            .expect("variant listed");
        assert_ne!(variant.state, PropertyState::Invalid);
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

    // -- composition-tolerant pre-validation ------------------------------

    #[test]
    fn pre_validate_does_not_reject_template_bearing_value() {
        // Regression test for review-3 high finding. A schema-constrained
        // value supplied as a template expression must NOT fail
        // pre-validation, because Darkmatter's compose pipeline can
        // resolve `{{ env.AGENT }}` into a valid enum member before the
        // prepare-time validator runs.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  runtime_agent: 'enum(goose; required)'\nruntime_agent: '{{ env.AGENT }}'\n---\nbody\n",
        );

        let pre = pre_validate_schema(&source, None)
            .expect("template-bearing required value must pass pre-validation");
        // Source/overrides are returned unchanged.
        assert!(pre.set_overrides.is_none());
        let raw = pre
            .source
            .markdown
            .frontmatter()
            .as_map()
            .get("runtime_agent")
            .and_then(|v| v.as_str())
            .unwrap();
        assert!(raw.contains("{{"), "value must not be dropped or scrubbed");
    }

    #[test]
    fn pre_validate_defers_template_invalid_required_to_prepare_time() {
        // A required field with a template value used to fail at
        // pre-validation against the raw frontmatter. Composition may
        // resolve the template to a valid value, so the verdict is
        // deferred. Prepare-time still validates the composed result.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  runtime_agent: 'enum(goose; required)'\nruntime_agent: '{{ env.AGENT }}'\n---\nbody\n",
        );

        let pre = pre_validate_schema(&source, None);
        assert!(
            pre.is_ok(),
            "pre-validation must defer template-bearing invalid-required to prepare-time"
        );
    }

    #[test]
    fn pre_validate_still_surfaces_literal_invalid_required() {
        // A required field with a literal (non-template) value that
        // doesn't satisfy the schema is definitively bad and surfaces
        // here as `SchemaValidation` so users see the error early.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  count: 'number(required)'\ncount: not-a-number\n---\nbody\n",
        );

        let err = pre_validate_schema(&source, None).unwrap_err();
        assert!(
            matches!(err, CompositionError::SchemaValidation { .. }),
            "expected SchemaValidation for literal invalid-required, got: {err:?}"
        );
    }

    #[test]
    fn pre_validate_still_surfaces_genuinely_missing_required() {
        // Missing-required is composition-independent: no template can
        // conjure a key that isn't present anywhere. This case must still
        // produce `MissingProperties` so the CLI can drive interactive
        // collection.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  title: 'string(required)'\n---\nbody\n",
        );

        let err = pre_validate_schema(&source, None).unwrap_err();
        assert!(
            matches!(err, CompositionError::MissingProperties { .. }),
            "expected MissingProperties, got: {err:?}"
        );
    }

    #[test]
    fn drop_invalid_optionals_skips_template_bearing_values() {
        // `count: '{{ env.COUNT }}'` is optional and currently looks like a
        // string (invalid for `number`). The pre-preflight scrub must NOT
        // drop it — composition can produce a numeric value.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  count: 'number'\ncount: '{{ env.COUNT }}'\n---\nbody\n",
        );

        let (scrubbed, _, _) = drop_invalid_optionals(source, None);
        let value = scrubbed
            .markdown
            .frontmatter()
            .as_map()
            .get("count")
            .and_then(|v| v.as_str());
        assert_eq!(value, Some("{{ env.COUNT }}"));
    }

    #[test]
    fn drop_invalid_optionals_still_drops_literal_invalid_values() {
        // Non-template invalid optional values are still dropped early as
        // before (preserves the existing UX for hardcoded mistakes).
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  count: 'number'\ncount: nope\n---\nbody\n",
        );

        let (scrubbed, _, _) = drop_invalid_optionals(source, None);
        assert!(
            !scrubbed
                .markdown
                .frontmatter()
                .as_map()
                .contains_key("count"),
            "literal invalid optional should still be dropped pre-preflight",
        );
    }

    #[test]
    fn value_needs_composition_detects_nested_templates() {
        assert!(value_needs_composition(Some(&serde_json::json!(
            "{{ env.X }}"
        ))));
        assert!(value_needs_composition(Some(&serde_json::json!([
            "a", "{{ x }}"
        ]))));
        assert!(value_needs_composition(Some(&serde_json::json!({
            "nested": "{{ x }}"
        }))));
        assert!(!value_needs_composition(Some(&serde_json::json!("plain"))));
        assert!(!value_needs_composition(Some(&serde_json::json!(42))));
        assert!(!value_needs_composition(None));
        // Frontmatter shell expressions (`$(...)`) must also defer.
        assert!(value_needs_composition(Some(&serde_json::json!(
            "$(echo small)"
        ))));
        assert!(value_needs_composition(Some(&serde_json::json!([
            "a",
            "$(echo b)"
        ]))));
    }

    // -- post-shell validation -------------------------------------------

    fn approve_echo() -> std::collections::HashSet<String> {
        let mut set = std::collections::HashSet::new();
        set.insert("echo small".to_string());
        set.insert("echo medium".to_string());
        set.insert("echo large".to_string());
        set.insert("echo huge".to_string());
        set
    }

    #[cfg(unix)]
    #[test]
    fn post_shell_valid_value_passes() {
        // `$(echo small)` resolves to a valid enum member during shell
        // expansion. Post-shell validation must accept the composed value.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            concat!(
                "---\n",
                "$schema:\n",
                "  tier: 'enum(small, medium, large; required)'\n",
                "tier: $(echo small)\n",
                "---\nbody\n",
            ),
        );

        let opts = PrepareOptions {
            pre_approved_commands: Some(approve_echo()),
            ..Default::default()
        };
        let prepared = prepare_direct_with_schema(&source, opts).unwrap();
        let tier = prepared
            .effective_frontmatter
            .as_object()
            .unwrap()
            .get("tier")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(tier, "small");
        assert!(prepared.dropped_optionals.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn post_shell_invalid_required_returns_schema_validation_error() {
        // `$(echo huge)` produces a value that is NOT a member of the
        // enum. Post-shell validation must surface a SchemaValidation
        // error so the provider is never launched on bad final input.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            concat!(
                "---\n",
                "$schema:\n",
                "  tier: 'enum(small, medium, large; required)'\n",
                "tier: $(echo huge)\n",
                "---\nbody\n",
            ),
        );

        let opts = PrepareOptions {
            pre_approved_commands: Some(approve_echo()),
            ..Default::default()
        };
        let err = prepare_direct_with_schema(&source, opts).unwrap_err();
        assert!(
            matches!(err, CompositionError::SchemaValidation { .. }),
            "expected post-shell SchemaValidation, got: {err:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn post_shell_invalid_optional_is_dropped_with_diagnostic() {
        // Optional `tier` becomes invalid after shell expansion. Drop it
        // from the effective frontmatter, track the drop, and let the run
        // continue.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            concat!(
                "---\n",
                "$schema:\n",
                "  title: 'string(required)'\n",
                "  tier: 'enum(small, medium, large)'\n",
                "title: Plan\n",
                "tier: $(echo huge)\n",
                "---\nbody\n",
            ),
        );

        let opts = PrepareOptions {
            pre_approved_commands: Some(approve_echo()),
            ..Default::default()
        };
        let prepared = prepare_direct_with_schema(&source, opts).unwrap();
        let fm = prepared.effective_frontmatter.as_object().unwrap();
        assert!(
            !fm.contains_key("tier"),
            "invalid optional `tier` should have been dropped post-shell"
        );
        let drops: Vec<_> = prepared
            .dropped_optionals
            .iter()
            .filter(|d| d.property == "tier")
            .collect();
        assert_eq!(drops.len(), 1, "expected one post-shell drop diagnostic");
        assert_eq!(drops[0].stage, DroppedOptionalStage::PostShellExpansion);
    }

    #[test]
    fn pre_validation_drop_surfaces_diagnostic() {
        // A file-authored invalid optional value should produce a
        // DroppedOptional diagnostic from pre-validation.
        let dir = TempDir::new().unwrap();
        let source = make_source(
            &dir,
            "---\n$schema:\n  count: 'number'\ncount: nope\n---\nbody\n",
        );

        let pre = pre_validate_schema(&source, None).unwrap();
        assert_eq!(pre.dropped_optionals.len(), 1);
        assert_eq!(pre.dropped_optionals[0].property, "count");
        assert_eq!(
            pre.dropped_optionals[0].source,
            DroppedOptionalSource::Frontmatter
        );
        assert_eq!(
            pre.dropped_optionals[0].stage,
            DroppedOptionalStage::PreValidation
        );
    }
}
