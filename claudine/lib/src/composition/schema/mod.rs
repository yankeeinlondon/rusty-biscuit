//! Schema-aware composition preparation.
//!
//! The launch verdict judges a document by its effective schema: the
//! matching trigger schemas of its five schema locations, with Claudine's
//! embedded bundle ([`bundle`]) as the last-resort location, under its
//! authored `$schema` (see [`load_effective_schema`]). Darkmatter's compose
//! pipeline also runs effective-schema validation between frontmatter
//! interpolation and shell expansion (see
//! `darkmatter::markdown::compose::schema_validation`). When the
//! post-override frontmatter does not satisfy that schema,
//! compose aborts with `MarkdownError::SchemaValidationFailed`, which the
//! prepare layer surfaces as `CompositionError::ComposeFailed`.
//!
//! This module wraps [`prepare_direct`] / [`prepare_inline`] so callers get
//! claudine's typed schema errors instead of a generic compose failure:
//!
//! - [`CompositionError::SchemaLoad`] when the `$schema` reference or a
//!   discovered trigger schema could not be resolved or compiled.
//! - [`CompositionError::SchemaValidation`] when one or more **required**
//!   properties or Claudine controls have the wrong type.
//! - [`CompositionError::MissingProperties`] when one or more **required**
//!   properties are missing. Carries the property metadata Phase 3 needs
//!   to drive the interactive TUI.
//!
//! Invalid **optional** values are dropped from the markdown's frontmatter
//! and composition is retried once, except a Claudine control (a property
//! whose winning declaration is Claudine's trigger payload), which fails the
//! launch instead: see [`bundle::is_claudine_owned`]. After composition succeeds, the
//! post-shell-expanded effective frontmatter is re-validated against the
//! same schema so values produced by frontmatter `$(...)` expressions are
//! judged on their final form. Dropped properties are captured as
//! [`super::error::DroppedOptional`] entries on the returned
//! [`PreparedComposition`] / [`PreValidatedSchema`] so the CLI can render a
//! user-visible warning to stderr; `tracing::warn!` events still fire for
//! anyone running with structured tracing enabled.
//!
//! The current launch contract is documented in
//! [Composition](../../../../docs/topics/composition.md).

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, EffectiveSchema, PropertyAtom, PropertyDef, SchemaError,
    SchemaOriginKind, SchemaShape, SimplifiedSchema, SimplifiedType, TypeExpr, ValidationProblem,
    ValidationProblemCode, ValidationProblemKind,
};

use super::error::{
    CompositionError, DroppedOptional, DroppedOptionalSource, DroppedOptionalStage,
    InteractiveShape, MissingProperty, TextFormat,
};
use super::prepare::{
    PrepareOptions, PromptSource, ResolvedIdentitySelector, prepare_direct_with_prompt_selecting,
    prepare_inline_selecting,
};
use super::types::{CompositionMode, PreparedComposition, ResolvedCompositionSource};
use darkmatter::markdown::schemas::{PositionMap, SchemaPhase, ValidationReport};

pub mod bundle;
pub mod classify;
pub mod globals;
mod status_render;
pub mod supplied;
pub mod translate;

pub use classify::*;
pub use status_render::{
    description_suffix, escape_schema_prose, render_optional_line, render_required_line,
    schema_status_report_prose,
};
pub(in crate::composition) use translate::schema_error_to_composition_error;
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
        PrepareMode::Direct(PromptSource::ComposedBody, None),
    )
}

/// [`prepare_direct_with_schema`], choosing the launch identity the body
/// renders with from the document's resolved frontmatter.
///
/// `select` is called once the frontmatter is resolved, every frontmatter
/// `$( … )` included, and before the body renders, with the hints that
/// frontmatter names and the composition environment to edit; see
/// [`ResolvedIdentitySelector`]. The single compose runs each frontmatter and
/// body command once.
///
/// ## Errors
///
/// As [`prepare_direct_with_schema`], plus `select`'s, which ends the
/// preparation before the body renders.
pub fn prepare_direct_with_schema_selecting(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    select: &ResolvedIdentitySelector<'_>,
) -> Result<PreparedComposition, CompositionError> {
    prepare_with_schema(source, options, PrepareMode::Direct(PromptSource::ComposedBody, Some(select)))
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
    prepare_with_schema(source, options, PrepareMode::Direct(prompt_source, None))
}

pub(super) fn prepare_direct_with_schema_and_prompt_selecting(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    prompt_source: PromptSource,
    select: Option<&ResolvedIdentitySelector<'_>>,
) -> Result<PreparedComposition, CompositionError> {
    prepare_with_schema(source, options, PrepareMode::Direct(prompt_source, select))
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
    prepare_with_schema(source, options, PrepareMode::Inline(None))
}

/// [`prepare_inline_with_schema`], choosing the launch identity as
/// [`prepare_direct_with_schema_selecting`] does.
///
/// ## Errors
///
/// As [`prepare_inline_with_schema`], plus `select`'s.
pub fn prepare_inline_with_schema_selecting(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    select: &ResolvedIdentitySelector<'_>,
) -> Result<PreparedComposition, CompositionError> {
    prepare_with_schema(source, options, PrepareMode::Inline(Some(select)))
}

/// The preparation a schema-aware entry runs (and reruns after dropping an
/// invalid optional), with its identity selector, if any.
#[derive(Clone)]
pub(super) enum PrepareMode<'s> {
    Direct(PromptSource, Option<&'s ResolvedIdentitySelector<'s>>),
    Inline(Option<&'s ResolvedIdentitySelector<'s>>),
}

fn prepare_with_schema(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode<'_>,
) -> Result<PreparedComposition, CompositionError> {
    let set_overrides = options.set_overrides.clone();
    prepare_with_schema_verdict(source, options, mode).map_err(|err| {
        prefer_removed_grammar(source, &build_effective_instance(source, set_overrides.as_ref()), err)
    })
}

fn prepare_with_schema_verdict(
    source: &ResolvedCompositionSource,
    options: PrepareOptions,
    mode: PrepareMode<'_>,
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
    mode: &PrepareMode<'_>,
) -> Result<PreparedComposition, CompositionError> {
    match mode {
        PrepareMode::Direct(prompt_source, select) => {
            prepare_direct_with_prompt_selecting(source, options, prompt_source.clone(), *select)
        }
        PrepareMode::Inline(select) => prepare_inline_selecting(source, options, *select),
    }
}

/// Re-validate the post-shell-expanded effective frontmatter against the
/// schema retained on the prepared composition, at that preparation's launch
/// phase, and record the launch report.
///
/// The categorization mirrors the compose-time translator: invalid required
/// values fail, droppable invalid optionals are removed and re-judged, and
/// missing launch-required values surface as [`CompositionError::MissingProperties`].
fn post_shell_validate(
    source: &ResolvedCompositionSource,
    mut prepared: PreparedComposition,
    mut dropped: Vec<DroppedOptional>,
    _mode: &PrepareMode<'_>,
    _file_ref_fallback_dir: Option<&std::path::Path>,
) -> Result<PreparedComposition, CompositionError> {
    // No `$schema` (or a deferred read that could not prepare it) → no
    // validation work to do. The retained launch schema is the one resolved
    // during preparation; nothing here resolves the declaration again.
    let Some((effective, phase)) = prepared
        .launch_schema
        .as_ref()
        .map(|launch| (launch.effective.clone(), launch.phase))
    else {
        prepared.dropped_optionals = dropped;
        return Ok(prepared);
    };
    // An authored raw JSON Schema has no per-property metadata to categorize,
    // and Darkmatter's compose-time stage already applied its `required`
    // fail-fast; only the properties a trigger declares are judged here.
    let trigger_declared_only = effective.simplified.is_none()
        && source.markdown.frontmatter().as_map().contains_key("$schema");

    let judge = |instance: &serde_json::Value| -> Result<ValidationReport, CompositionError> {
        let mut report = match phase {
            Some(phase) => effective.validate_for_phase(instance, phase).map_err(|error| {
                schema_error_to_composition_error(
                    &source.resolved_path,
                    error.to_string(),
                    Some(&error),
                )
            })?,
            None => effective.validate(instance),
        };
        if trigger_declared_only {
            report.problems.retain(|problem| {
                top_level_pointer_segment(&problem.path).is_some_and(|name| {
                    effective
                        .origins
                        .get(&name)
                        .is_some_and(|origin| origin.kind == SchemaOriginKind::Trigger)
                })
            });
            report.valid = report.problems.is_empty();
        }
        Ok(report)
    };
    let finish = |mut prepared: PreparedComposition,
                  dropped: Vec<DroppedOptional>,
                  validation: &ValidationReport| {
        let report = status_report_from_validation(
            &effective,
            phase,
            &source.resolved_path,
            instance_map(&prepared.effective_frontmatter),
            validation,
        );
        if let Some(launch) = prepared.launch_schema.as_mut() {
            launch.report = report;
        }
        prepared.dropped_optionals = dropped;
        prepared
    };

    let report = judge(&prepared.effective_frontmatter)?;
    if report.valid {
        return Ok(finish(prepared, dropped, &report));
    }

    let mut categorized = categorize_problems(&report.problems, Some(&effective), phase);
    if phase.is_none() {
        // Direct compose judges the authored contract: a required property whose
        // own expression resolved to `null` is a gap the caller fills at launch
        // (interactive collection), not a wrong-type value.
        promote_null_required_to_missing(
            &mut categorized,
            Some(&effective),
            Some(&prepared.effective_frontmatter),
        );
    }

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
            None => return Ok(finish(prepared, dropped, &report)),
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
                    reason: drop_reason(problem),
                });
            }
        }

        // Re-validate after dropping invalid optionals. If a required
        // value is still missing or invalid, surface it.
        let report2 = judge(&prepared.effective_frontmatter)?;
        if !report2.valid {
            let mut categorized2 =
                categorize_problems(&report2.problems, Some(&effective), phase);
            if phase.is_none() {
                promote_null_required_to_missing(
                    &mut categorized2,
                    Some(&effective),
                    Some(&prepared.effective_frontmatter),
                );
            }
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

        return Ok(finish(prepared, dropped, &report2));
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
        failures: translate::resolution_failures(&report.problems),
    })
}

fn instance_map(instance: &serde_json::Value) -> serde_json::Map<String, serde_json::Value> {
    instance.as_object().cloned().unwrap_or_default()
}

/// The runtime phase a composition mode judges its launch verdict at.
///
/// `inline-compose` launches at [`SchemaPhase::Launch`], where a
/// required-but-not-eager property may still be absent for the agent to
/// supply. Direct compose has no later producing actor for frontmatter, so its
/// launch verdict is the unphased authoring contract (`None`), which keeps the
/// existing `generated` exemption.
pub fn launch_phase_for_mode(mode: CompositionMode) -> Option<SchemaPhase> {
    match mode {
        CompositionMode::InlineFrontmatterPrompt => Some(SchemaPhase::Launch),
        CompositionMode::ChainedDocument => None,
    }
}

/// What the document's authored `$schema` establishes about parameter names,
/// for composition token ownership ([`super::ownership::own_arguments`]).
///
/// Reads the literal `$schema` through the same loader the composer uses (no
/// Darkmatter baseline or trigger layers, no templates, shell, or network), so
/// a schema file resolves relative to the source document. A SimplifiedSchema
/// contributes the names in every union arm; a raw JSON Schema its statically
/// declared top-level names. A `$schema` reference written as a template is
/// [`SchemaParameters::Unestablished`]: ownership never evaluates it.
///
/// ## Errors
///
/// The loader's [`CompositionError`] when the schema cannot be read; ownership
/// must not guess past it.
pub fn authored_schema_parameters(
    source: &ResolvedCompositionSource,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<super::ownership::SchemaParameters, CompositionError> {
    use super::ownership::SchemaParameters;

    let frontmatter = source.markdown.frontmatter().as_map();
    let Some(reference) = frontmatter.get("$schema") else {
        return Ok(SchemaParameters::NoSchema);
    };
    let templated = |text: &str| text.contains("{{") || text.contains("$(");
    let reference_is_templated = match reference {
        serde_json::Value::String(text) => templated(text),
        serde_json::Value::Array(arms) => arms
            .iter()
            .any(|arm| arm.as_str().is_some_and(templated)),
        _ => false,
    };
    if reference_is_templated {
        return Ok(SchemaParameters::Unestablished);
    }
    Ok(
        match load_authored_schema(
            source,
            file_ref_fallback_dir,
            file_resolution_context,
            &darkmatter::markdown::compose::CallerInputRecords::new(),
        )? {
            None => SchemaParameters::NoSchema,
            Some(schema) => SchemaParameters::Declared(std::sync::Arc::new(move |key| {
                schema.declares_top_level_property(key)
            })),
        },
    )
}

/// Resolve the effective schema a launch verdict judges the document by: the
/// matching trigger schemas of the document's five schema locations, with
/// Claudine's embedded bundle as the last-resort location (see [`bundle`]),
/// merged under the authored `$schema`.
///
/// Triggers match `snapshot`, the values the verdict is about to judge, so a
/// caller overlay or a composed value selects them as it would in the editor;
/// `None` matches the source's own frontmatter. Discovery runs from the
/// document's own context, while values resolve under the same launch context
/// the composer resolves them with, so the retained launch schema and the
/// compose-time verdict agree on every file reference. `callers` are the
/// properties a caller supplied, whose `match()` globs are judged from each
/// record's origin, as composition judges them.
///
/// Returns `Ok(None)` when no schema applies: no authored `$schema` and no
/// matching trigger.
pub(super) fn load_effective_schema(
    source: &ResolvedCompositionSource,
    snapshot: Option<&serde_json::Value>,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    callers: &darkmatter::markdown::compose::CallerInputRecords,
) -> Result<Option<EffectiveSchema>, CompositionError> {
    let schema_error = |err: SchemaError| {
        // A grammar/convert/shape error is a body-syntax problem (`SchemaParse`);
        // a missing file / remote URL / ambiguous reference / malformed trigger
        // stays `SchemaLoad`. Darkmatter hands us the typed cause directly.
        schema_error_to_composition_error(&source.resolved_path, err.to_string(), Some(&err))
    };
    let document_context = file_resolution_context.for_source(&source.resolved_path);
    let mut schemas = DarkmatterSchemas::new(document_context)
        .with_embedded_trigger_bundle(bundle::claudine_schema_bundle())
        .and_then(DarkmatterSchemas::with_trigger_discovery)
        .map_err(schema_error)?
        .with_file_resolution_context(file_resolution_context.clone())
        .with_caller_input_records(callers);
    if let Some(fallback) = file_ref_fallback_dir {
        schemas = schemas.with_file_ref_fallback_dir(fallback);
    }
    match snapshot.and_then(serde_json::Value::as_object) {
        Some(values) => schemas.effective_for(&matching_markdown(source, values)),
        None => schemas.effective_for(&source.markdown),
    }
    .map_err(schema_error)
}

/// The authored `$schema` alone, with no trigger layers: what the document
/// itself declares, for decisions about the author's own parameters: which
/// argument tokens are setters, and which caller values are eager files.
pub(super) fn load_authored_schema(
    source: &ResolvedCompositionSource,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    callers: &darkmatter::markdown::compose::CallerInputRecords,
) -> Result<Option<EffectiveSchema>, CompositionError> {
    let mut schemas =
        DarkmatterSchemas::new(file_resolution_context.clone()).with_caller_input_records(callers);
    if let Some(fallback) = file_ref_fallback_dir {
        schemas = schemas.with_file_ref_fallback_dir(fallback);
    }
    schemas.effective_for(&source.markdown).map_err(|err| {
        schema_error_to_composition_error(&source.resolved_path, err.to_string(), Some(&err))
    })
}

/// The source document carrying `values` as its frontmatter, for trigger
/// matching. The document keeps its file (so `$path` conditions judge it) and
/// its own `$schema`, which a value overlay never replaces.
fn matching_markdown(
    source: &ResolvedCompositionSource,
    values: &serde_json::Map<String, serde_json::Value>,
) -> darkmatter::markdown::Markdown {
    let mut markdown = source.markdown.clone();
    let authored_schema = source.markdown.frontmatter().as_map().get("$schema").cloned();
    let frontmatter = markdown.frontmatter_mut().as_map_mut();
    frontmatter.clear();
    if let Some(schema) = authored_schema {
        frontmatter.insert("$schema".to_string(), schema);
    }
    for (key, value) in values {
        if key != "$schema" {
            frontmatter.insert(key.clone(), value.clone());
        }
    }
    markdown
}

/// The caller records of raw `set_overrides`, by the rule composition
/// applies when no explicit records were captured.
pub(super) fn override_records(
    set_overrides: Option<&serde_json::Value>,
    context: &biscuit_file::FileResolutionContext,
    launch_area: Option<&std::path::Path>,
) -> darkmatter::markdown::compose::CallerInputRecords {
    darkmatter::markdown::compose::caller_input_records_for_overrides(set_overrides, context, launch_area)
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
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> Result<PreValidatedSchema, CompositionError> {
    pre_validate_schema_for_mode(
        source,
        set_overrides,
        file_ref_fallback_dir,
        file_resolution_context,
        CompositionMode::ChainedDocument,
    )
}

/// [`pre_validate_schema`] judged at the launch phase of `mode`.
///
/// `inline-compose` validates at [`SchemaPhase::Launch`]: a missing
/// `required; eager` property is reported (and collected) exactly as for
/// direct compose, while eager-only properties remain optional and a missing
/// required-but-not-eager property is deferred to completion. Present eager
/// values are type-checked at launch. Direct compose keeps the unphased
/// authoring verdict.
///
/// ## Errors
///
/// See [`pre_validate_schema`].
pub fn pre_validate_schema_for_mode(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    mode: CompositionMode,
) -> Result<PreValidatedSchema, CompositionError> {
    pre_validate_with_origin(
        source,
        set_overrides,
        &std::collections::BTreeSet::new(),
        file_ref_fallback_dir,
        file_resolution_context,
        mode,
    )
}

/// [`pre_validate_schema_for_mode`] for overrides that carry their origin.
///
/// A data override (a runtime value, never typed by a person) is final, so a
/// value holding `{{` or `$(` is judged rather than deferred to composition.
///
/// ## Errors
///
/// See [`pre_validate_schema`].
pub fn pre_validate_layered_for_mode(
    source: &ResolvedCompositionSource,
    overrides: &crate::composition::LayeredOverrides,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    mode: CompositionMode,
) -> Result<PreValidatedSchema, CompositionError> {
    pre_validate_with_origin(
        source,
        Some(&overrides.to_value()),
        overrides.data_keys(),
        file_ref_fallback_dir,
        file_resolution_context,
        mode,
    )
}

fn pre_validate_with_origin(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    data_keys: &std::collections::BTreeSet<String>,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    mode: CompositionMode,
) -> Result<PreValidatedSchema, CompositionError> {
    pre_validate_verdict(
        source,
        set_overrides,
        data_keys,
        file_ref_fallback_dir,
        file_resolution_context,
        mode,
    )
    .map_err(|err| prefer_removed_grammar(source, &build_effective_instance(source, set_overrides), err))
}

/// A retired lifecycle spelling is reported by the lifecycle parser, with its
/// replacement, rather than as the schema mismatch it also is: the schema
/// knows only the current grammar, so its verdict on a retired spelling names
/// no migration.
fn prefer_removed_grammar(
    source: &ResolvedCompositionSource,
    instance: &serde_json::Value,
    err: CompositionError,
) -> CompositionError {
    if !matches!(err, CompositionError::SchemaValidation { .. }) {
        return err;
    }
    match super::lifecycle::parse_lifecycle_config(instance, &source.resolved_path) {
        Err(removed @ CompositionError::RemovedGrammar { .. }) => removed,
        _ => err,
    }
}

fn pre_validate_verdict(
    source: &ResolvedCompositionSource,
    set_overrides: Option<&serde_json::Value>,
    data_keys: &std::collections::BTreeSet<String>,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    mode: CompositionMode,
) -> Result<PreValidatedSchema, CompositionError> {
    let phase = launch_phase_for_mode(mode);

    // First pass: drop non-template invalid optionals so the prepare-time
    // pipeline (and the preflight Darkmatter pass) sees a clean slate.
    let (source, set_overrides, dropped_optionals) = drop_invalid_optionals_with_origin(
        source.clone(),
        set_overrides.cloned(),
        data_keys,
        file_ref_fallback_dir,
        file_resolution_context,
    );

    // Deferral is decided on the raw values, where a stored literal token is
    // visibly data; validation judges the text composition will read.
    let raw_instance = build_effective_instance(&source, set_overrides.as_ref());
    let callers = override_records(set_overrides.as_ref(), file_resolution_context, file_ref_fallback_dir);
    let effective = match load_effective_schema(
        &source,
        Some(&raw_instance),
        file_ref_fallback_dir,
        file_resolution_context,
        &callers,
    ) {
        Ok(Some(e)) => e,
        // No authored `$schema` and no matching trigger: nothing to judge.
        Ok(None) => {
            return Ok(PreValidatedSchema {
                source,
                set_overrides,
                dropped_optionals,
            });
        }
        Err(err) => return Err(err),
    };
    // Darkmatter decodes the stored tokens itself, so it judges their text
    // without ever deferring it as a template.
    let mut instance = raw_instance.clone();
    normalize_file_array_values(&mut instance, Some(&effective));
    let report = effective
        .validate_stored(&instance, &PositionMap::new(), phase)
        .map_err(|error| {
            schema_error_to_composition_error(
                &source.resolved_path,
                error.to_string(),
                Some(&error),
            )
        })?;
    if report.valid {
        return Ok(PreValidatedSchema {
            source,
            set_overrides,
            dropped_optionals,
        });
    }

    let caller_resolved_eager_files =
        caller_resolved_file_properties(
            &report.problems,
            set_overrides.as_ref(),
            file_ref_fallback_dir,
            file_resolution_context,
        );

    // Filter problems composition-tolerantly: drop Invalid/Type verdicts
    // whose raw value at the problem's path contains template syntax, because
    // Darkmatter may resolve them to valid values during composition. A caller-originated
    // eager file likewise keeps its launch-area provenance only in canonical
    // preparation, but only after the literal reference resolves there. An
    // unresolved partial must remain visible so the CLI can offer completion.
    // Missing verdicts are composition-independent (no template or provenance
    // can conjure a key absent from both raw frontmatter and overrides).
    let composition_independent: Vec<_> = report
        .problems
        .iter()
        .filter(|p| {
            let caller_resolved = !matches!(p.kind, ValidationProblemKind::Missing)
                && top_level_pointer_segment(&p.path)
                    .is_some_and(|name| caller_resolved_eager_files.contains(&name));
            !caller_resolved && is_composition_independent(p, &raw_instance, data_keys, &report.deferred)
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

    let categorized = categorize_problems(&composition_independent, Some(&effective), phase);
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
        // Offer interactive resolution for a provided optional file partial
        // before falling back to the generic schema error.
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
        failures: translate::resolution_failures(&composition_independent),
    })
}

pub(super) fn build_effective_instance(
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
    source: ResolvedCompositionSource,
    set_overrides: Option<serde_json::Value>,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> (
    ResolvedCompositionSource,
    Option<serde_json::Value>,
    Vec<DroppedOptional>,
) {
    drop_invalid_optionals_with_origin(
        source,
        set_overrides,
        &std::collections::BTreeSet::new(),
        file_ref_fallback_dir,
        file_resolution_context,
    )
}

/// [`drop_invalid_optionals`] where the overrides named in `data_keys` are
/// data: final values, dropped when invalid whatever text they hold.
fn drop_invalid_optionals_with_origin(
    mut source: ResolvedCompositionSource,
    mut set_overrides: Option<serde_json::Value>,
    data_keys: &std::collections::BTreeSet<String>,
    file_ref_fallback_dir: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> (
    ResolvedCompositionSource,
    Option<serde_json::Value>,
    Vec<DroppedOptional>,
) {
    let mut dropped: Vec<DroppedOptional> = Vec::new();
    let serde_json::Value::Object(instance) = build_effective_instance(&source, set_overrides.as_ref())
    else {
        return (source, set_overrides, dropped);
    };
    let callers = override_records(set_overrides.as_ref(), file_resolution_context, file_ref_fallback_dir);
    let effective = match load_effective_schema(
        &source,
        Some(&serde_json::Value::Object(instance.clone())),
        file_ref_fallback_dir,
        file_resolution_context,
        &callers,
    ) {
        Ok(Some(e)) => e,
        // No schema applies, or it cannot be loaded: the prepare-time
        // validator reports a load failure.
        Ok(None) | Err(_) => return (source, set_overrides, dropped),
    };

    // Judged on the stored tokens' text, never deferred as a template; only
    // a judgment is made here, and the source handed on keeps its tokens.
    let report = match effective.validate_stored(
        &serde_json::Value::Object(instance.clone()),
        &PositionMap::new(),
        None,
    ) {
        Ok(report) => report,
        // Unphased validation projects nothing, so it cannot fail.
        Err(_) => return (source, set_overrides, dropped),
    };
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
                // A Claudine control is never cleaned away: an invalid one
                // blocks the launch instead of running without it.
                if bundle::is_claudine_owned(&effective, &name) {
                    continue;
                }
                if is_required(shape, &name) {
                    continue;
                }
                if is_eager(shape, &name) {
                    continue;
                }
                // Composition-tolerant: skip values that look templated,
                // because Darkmatter may produce a valid value during
                // composition. The prepare-time validator will run
                // drop-and-retry against the composed value if it is
                // genuinely invalid.
                if !data_keys.contains(&name) && value_needs_composition(instance.get(&name)) {
                    continue;
                }
                if !to_drop.iter().any(|(n, _)| n == &name) {
                    to_drop.push((name, drop_reason(problem)));
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

/// The top-level properties whose file-reference verdict is on a caller
/// override that resolves from the caller's launch area.
///
/// The validator judges every value from the document's directory; a caller
/// value resolves from where the caller launched, so such a verdict is about
/// the wrong base and must not be shown or enforced as invalid.
pub(super) fn caller_resolved_file_properties(
    problems: &[ValidationProblem],
    overrides: Option<&serde_json::Value>,
    launch_area: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
) -> std::collections::HashSet<String> {
    let Some((launch_area, overrides)) =
        launch_area.zip(overrides.and_then(serde_json::Value::as_object))
    else {
        return std::collections::HashSet::new();
    };
    // The caller launched there, so the launch area is trusted even when the
    // document's own tree does not contain it.
    let launch_context = file_resolution_context.for_trusted_external_cwd(launch_area);
    problems
        .iter()
        .filter(|problem| matches!(problem.code, ValidationProblemCode::InvalidFileReference))
        .filter_map(|problem| top_level_pointer_segment(&problem.path))
        .filter(|property| {
            overrides
                .get(property)
                .is_some_and(|value| file_override_resolves_from(value, &launch_context))
        })
        .collect()
}

fn file_override_resolves_from(
    value: &serde_json::Value,
    context: &biscuit_file::FileResolutionContext,
) -> bool {
    match value {
        serde_json::Value::String(raw) => biscuit_file::FileReference::new(raw)
            .and_then(|reference| reference.resolve_in_context(context))
            .is_ok_and(|resolved| resolved.is_some()),
        serde_json::Value::Array(values) => {
            !values.is_empty() && values.iter().all(|value| file_override_resolves_from(value, context))
        }
        _ => false,
    }
}

/// Whether a validation problem stands regardless of composition.
///
/// `Missing` always stands: no template can conjure a key absent from both raw
/// frontmatter and overrides. `Type`/`Invalid` stand unless Darkmatter's
/// [`awaits_rendering`](darkmatter::markdown::schemas::awaits_rendering) says
/// rendering could settle them: the value at the problem's own path (its
/// nearest existing ancestor when absent) holds template or shell syntax the
/// schema did not already accept as `deferred` (`deferred`, from the report)
/// and no malformed template, and the problem is not an unknown or missing
/// key on a collection. A
/// literal sibling of a template always stands. A property named in `data_keys`
/// is data and always stands. Shared by the pre-validator and root-union arm
/// selection so both judge arms by the same rule.
pub(super) fn is_composition_independent(
    problem: &ValidationProblem,
    instance: &serde_json::Value,
    data_keys: &std::collections::BTreeSet<String>,
    deferred: &[String],
) -> bool {
    match problem.kind {
        ValidationProblemKind::Missing => true,
        ValidationProblemKind::Type | ValidationProblemKind::Invalid => {
            let Some(name) = top_level_pointer_segment(&problem.path) else {
                return true;
            };
            // A malformed `{{ … }}` span can never render into a valid
            // value, so its problem stands, as plain validation reports it.
            let holds = |text: &str| {
                darkmatter::markdown::literal_token::holds_pending_syntax(text)
                    && !darkmatter::markdown::compose::expression::is_malformed_template(text)
            };
            data_keys.contains(&name)
                || !darkmatter::markdown::schemas::authored_node_at(instance, &problem.path)
                    .is_some_and(|(pointer, node)| {
                        darkmatter::markdown::schemas::awaits_rendering(problem, node, &pointer, deferred, &holds)
                    })
        }
    }
}

/// Returns `true` when `value` contains a Darkmatter template marker
/// (`{{ ... }}`) or a frontmatter shell expression (`$(...)`) somewhere
/// in any string descendant. Pre-validation must not drop or invalidate
/// such values, because composition (template interpolation and shell
/// expansion) may transform them into valid frontmatter. A stored literal
/// token is data, never pending.
fn value_needs_composition(value: Option<&serde_json::Value>) -> bool {
    let Some(value) = value else { return false };
    match value {
        serde_json::Value::String(s) => darkmatter::markdown::literal_token::holds_pending_syntax(s),
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
