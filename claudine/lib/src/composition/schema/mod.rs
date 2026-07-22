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
    ValidationProblemCode, ValidationProblemKind,
};

use super::error::{
    CompositionError, DroppedOptional, DroppedOptionalSource, DroppedOptionalStage,
    InteractiveShape, MissingProperty, TextFormat,
};
use super::prepare::{PrepareOptions, PromptSource, prepare_direct_with_prompt, prepare_inline};
use super::types::{PreparedComposition, ResolvedCompositionSource};

pub mod classify;
pub mod translate;

pub use classify::*;
use translate::*;

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
    prepare_with_schema(
        source,
        options,
        PrepareMode::Direct(PromptSource::ComposedBody),
    )
}

/// [`prepare_direct_with_schema`] for a caller-supplied prompt.
///
/// The direct-wrapper passthrough case composes the document only for its
/// effective frontmatter, so it needs the schema layer's categorization
/// without the composed body becoming the prompt. Splitting this out is what
/// lets the harness route reach the same typed schema errors as the `compose`
/// and `sequence` routes; see [`prepare_document`][super::prepare_document].
///
/// ## Errors
///
/// See [`prepare_direct_with_schema`].
pub fn prepare_direct_with_schema_and_prompt(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    prompt_source: PromptSource,
) -> Result<PreparedComposition, CompositionError> {
    prepare_with_schema(source, options, PrepareMode::Direct(prompt_source))
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

#[derive(Clone)]
pub(super) enum PrepareMode {
    Direct(PromptSource),
    Inline,
}

fn prepare_with_schema(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode,
) -> Result<PreparedComposition, CompositionError> {
    let mut dropped: Vec<DroppedOptional> = Vec::new();
    let file_ref_fallback_dir = options.file_ref_fallback_dir.clone();
    let prepared = match run_prepare(source, options.clone(), &mode) {
        Ok(prepared) => prepared,
        Err(err) => handle_compose_error(source, options, &mode, err, &mut dropped)?,
    };
    // Re-validate the post-shell-expanded effective frontmatter and apply
    // the same typed error / drop-and-retry rules so values that became
    // invalid (or now satisfy the schema) after `$(...)` expansion are
    // judged on their final form.
    post_shell_validate(source, prepared, dropped, &mode, file_ref_fallback_dir.as_deref())
}

fn run_prepare(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: &PrepareMode,
) -> Result<PreparedComposition, CompositionError> {
    match mode {
        PrepareMode::Direct(prompt_source) => {
            prepare_direct_with_prompt(source, options, prompt_source.clone())
        }
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
fn post_shell_validate(
    source: &ResolvedCompositionSource,
    mut prepared: PreparedComposition,
    mut dropped: Vec<DroppedOptional>,
    _mode: &PrepareMode,
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

    let caller_resolved_eager_files: std::collections::HashSet<_> = file_ref_fallback_dir
        .zip(set_overrides.as_ref().and_then(serde_json::Value::as_object))
        .map(|(fallback, overrides)| {
            report
                .problems
                .iter()
                .filter(|problem| {
                    matches!(problem.code, ValidationProblemCode::InvalidFileReference)
                })
                .filter_map(|problem| top_level_pointer_segment(&problem.path))
                .filter(|property| {
                    overrides.get(property).is_some_and(|value| {
                        file_override_resolves_from(value, fallback)
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    // Filter problems composition-tolerantly: drop Invalid/Type verdicts
    // whose raw value contains template syntax, because Darkmatter may
    // resolve them to valid values during composition. A caller-originated
    // eager file likewise keeps its launch-area provenance only in canonical
    // preparation, but only after the literal reference resolves there. An
    // unresolved partial must remain visible so the CLI can offer completion.
    // Missing verdicts are composition-independent (no template or provenance
    // can conjure a key absent from both raw frontmatter and overrides).
    let instance_map = instance.as_object();
    let composition_independent: Vec<_> = report
        .problems
        .iter()
        .filter(|p| match p.kind {
            ValidationProblemKind::Missing => true,
            ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
                let property = top_level_pointer_segment(&p.path);
                if property
                    .as_ref()
                    .is_some_and(|name| caller_resolved_eager_files.contains(name))
                {
                    return false;
                }
                let raw = property
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

fn file_override_resolves_from(value: &serde_json::Value, base: &std::path::Path) -> bool {
    match value {
        serde_json::Value::String(raw) => biscuit_file::FileReference::new(raw)
            .and_then(|reference| reference.resolve_from(base))
            .is_ok_and(|resolved| resolved.is_some()),
        serde_json::Value::Array(values) => {
            !values.is_empty() && values.iter().all(|value| file_override_resolves_from(value, base))
        }
        _ => false,
    }
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
