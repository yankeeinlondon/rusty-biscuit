//! Just-in-time step composition.
//!
//! One step composes at its turn, not up front. That is the whole point of this
//! module: [`compose_step`] re-reads nothing itself, but it is handed a *live*
//! source by the executor and layers the runtime state as it stands at that
//! moment, so an earlier step's inline-compose write-back, an agent's mid-run
//! frontmatter edit, and every accumulated `set` are all visible.
//!
//! The sequence-wide validation pass ([`super::phase1c`]) calls the same
//! function against the initial source and an empty runtime cell. Sharing one
//! path is what makes "validated == executed" true: there is no second prepare
//! implementation that could drift.
//!
//! What is *not* re-derived at a step boundary: the step list itself. Dynamic
//! sources snapshot once during static preflight (spec → *Phase 1 — Static
//! Preflight*), so a `sequence: $(ls)` that would enumerate differently mid-run
//! does not change the plan under execution.

use std::collections::{BTreeMap, HashSet};
use std::path::Path;
use std::sync::Arc;

use claudine::composition::{
    self, CompositionError, LayeredOverrides, PrepareOptions, PreparedComposition,
    ResolvedCompositionSource, ResolvedExecutionTarget, SequencePlan,
};
use serde_json::Value;

use crate::commands::compose::SharedComposeArgs;
use crate::commands::schema_interactive::emit_dropped_optional_warnings;

/// The per-invocation inputs every step composition shares.
#[derive(Clone)]
pub(super) struct StepComposeContext<'a> {
    /// Repo root of the sequence source, for shell policy resolution.
    pub(super) source_repo_root: Option<&'a Path>,
    /// Working directory `::shell` expansions run in.
    pub(super) child_cwd: &'a Path,
    /// Launch-area metadata retained for file-reference diagnostics.
    pub(super) launch_area: Option<&'a Path>,
    pub(super) shared: &'a SharedComposeArgs,
    /// Shared across steps so "allow once" holds for the whole run.
    pub(super) approval_cache: composition::SharedApprovalCache,
    /// `true` when the sequence document carries a string `prompt:`, making
    /// every step an inline composition that rewrites the body on disk.
    pub(super) inline_mode: bool,
    /// Immutable request-scoped file-resolution snapshot.
    pub(super) file_resolution_context: &'a biscuit_file::FileResolutionContext,
    /// Immutable CLI caller values, kept separate from task and runtime layers.
    pub(super) caller_input_records: &'a darkmatter::markdown::compose::CallerInputRecords,
    pub(super) invocation: &'a claudine::invocation_context::InvocationContext,
    /// The composition run a step's documents join: the sequence document
    /// composed for the step, its task, and the task's prompt document are
    /// one run. `None` opens a new run per preparation.
    pub(super) run_evidence: Option<&'a claudine::invocation_context::RunEvidence>,
}

impl<'ctx> StepComposeContext<'ctx> {
    /// The same context for a referenced document, whose own frontmatter — not
    /// the sequence's — decides whether composing it rewrites its body.
    pub(super) fn for_referenced_document<'a>(
        &'a self,
        inline_mode: bool,
        source_context: &'a claudine::invocation_context::SourceContext,
    ) -> StepComposeContext<'a>
    where
        'ctx: 'a,
    {
        StepComposeContext {
            inline_mode,
            source_repo_root: source_context.repository_root(),
            file_resolution_context: source_context.file_resolution_context(),
            ..self.clone()
        }
    }
}

/// One step's composed document, plus the approvals its composition contributed.
pub(super) struct StepComposition {
    pub(super) prepared: PreparedComposition,
    /// `approved` plus every command this step's template and lifecycle
    /// discovery added, so the next step inherits them.
    pub(super) approved: HashSet<String>,
}

/// Fold this step's overrides from the four just-in-time layers.
///
/// Lowest precedence first: user setters, accumulated runtime mutations, then
/// the reserved per-step overlay. The live document's own frontmatter is
/// Darkmatter's base and sits below all of them. Only the user setters are
/// authored; the runtime layers and the overlay are data. Passing
/// `runtime: None` yields the initial view — empty `outputs`, no mutations —
/// which is what the validation pass and `--dry-run` compose against.
pub(super) fn step_set_overrides(
    plan: &SequencePlan,
    step_index: usize,
    user_setters: Option<&Value>,
    runtime: Option<&composition::RuntimeSnapshot>,
) -> LayeredOverrides {
    let overlay = reserved_overlay(plan, step_index);
    composition::layered_set_overrides(
        LayeredOverrides::authored(user_setters),
        runtime,
        Some(&overlay),
    )
}

/// This step's reserved overlay alone — `state`, `previous`, `next`,
/// `sequence_id` — with no lower layer folded in.
///
/// A task executor needs it unmixed: it folds its own layers (`params` <
/// user setters < mutations < overlay), and handing it the already-folded object
/// would raise user setters above the runtime mutations that must outrank them.
pub(super) fn reserved_overlay(plan: &SequencePlan, step_index: usize) -> Value {
    composition::sequence::build_step_overlay(plan, step_index).as_set_overrides(None)
}

/// The environment a step's composition and child process see.
pub(super) fn step_env_overrides(
    effective_fail_fast: bool,
    shared: &SharedComposeArgs,
    target: Option<&ResolvedExecutionTarget>,
) -> BTreeMap<String, String> {
    let mut env = step_owned_env(effective_fail_fast);
    // A `--dry-run` step with an unresolved agent state has no target; leaving
    // `AGENT` unset makes `{{env.AGENT}}` resolve empty exactly as the direct
    // compose `--dry-run` path does.
    if let Some(target) = target {
        env.insert("AGENT".to_string(), target.provider.as_slug().to_string());
        if let Some(model) = &target.model {
            env.insert("MODEL".to_string(), model.clone());
        }
    }
    env.insert("YOLO".to_string(), shared.yolo.to_string());
    env
}

/// The part of [`step_env_overrides`] that belongs to the step rather than to
/// the document it runs, so a proxy target inside the step keeps it while
/// installing its own `AGENT`/`MODEL`/`YOLO`.
pub(super) fn step_owned_env(effective_fail_fast: bool) -> BTreeMap<String, String> {
    BTreeMap::from([(
        "CLAUDINE_FAIL_FAST".to_string(),
        effective_fail_fast.to_string(),
    )])
}

/// Validate, approve, and compose one step against `source`.
///
/// ## Errors
///
/// Every failure is a typed [`CompositionError`]. The caller decides what it
/// means: the sequence-wide validation pass aggregates
/// [`CompositionError::MissingProperties`] across steps and aborts on anything
/// else, while execution treats *any* failure here as a failure of that step,
/// governed by `fail_fast` (spec → *Mid-run failure policy*).
#[allow(clippy::result_large_err)]
///
/// `allow_empty_body` is set by a step that declares an executable: it runs its
/// task instead of the document body, so a bodyless source — the shape a
/// directly invoked `kind: sequence` YAML file always has — is legal there and
/// only there.
pub(super) fn compose_step(
    source: &ResolvedCompositionSource,
    ctx: &StepComposeContext<'_>,
    set_overrides: &LayeredOverrides,
    env_overrides: &BTreeMap<String, String>,
    approved: HashSet<String>,
    allow_empty_body: bool,
) -> Result<StepComposition, CompositionError> {
    // Schema pre-validation before the shell pass. A step whose effective
    // frontmatter is missing required values must report that rather than let
    // Darkmatter's compose surface a raw `SchemaValidationFailed`, which would
    // hide the property names the caller needs to collect or report.
    let pre = composition::pre_validate_layered_for_mode(
        source,
        set_overrides,
        ctx.launch_area,
        ctx.file_resolution_context,
        if ctx.inline_mode {
            composition::CompositionMode::InlineFrontmatterPrompt
        } else {
            composition::CompositionMode::ChainedDocument
        },
    )?;
    emit_dropped_optional_warnings(&pre.dropped_optionals);
    let step_source = pre.source;
    // Pre-validation may drop invalid optional keys; the rest keep their origin.
    let step_overrides =
        LayeredOverrides::from_parts(pre.set_overrides.as_ref(), set_overrides.data_keys());

    let approval_options = super::super::apply_composition_shell_overrides(
        super::super::build_harness_shell_options_for_source_with_cache(
            &step_source.resolved_path,
            ctx.source_repo_root,
            Some(Arc::clone(&ctx.approval_cache)),
        ),
        ctx.shared.dry_run,
        ctx.shared.yolo,
    );

    // Discovery and preparation share one option set, so the template audit
    // approves exactly the bytes the step's compose executes.
    let mut prepare_options =
        step_prepare_options(&step_source, ctx, step_overrides, env_overrides, allow_empty_body);
    let mut approved = approved;
    let template_preflight =
        composition::approve_document_shell(&step_source, &prepare_options, &approval_options)?;
    approved.extend(template_preflight.approved_commands.iter().cloned());
    prepare_options.pre_approved_commands = Some(approved.clone());

    // Inline steps prepare via `prepare_inline_with_schema` so the composed
    // `prompt` frontmatter becomes the agent prompt and the prepared closure is
    // `Inline`, which drives body write-back after the provider run.
    let prepared = if ctx.inline_mode {
        composition::prepare_inline_with_schema(&step_source, prepare_options)?
    } else {
        composition::prepare_direct_with_schema(&step_source, prepare_options)?
    };
    emit_dropped_optional_warnings(&prepared.dropped_optionals);

    // Audit the resolved lifecycle commands too. At validation time this is
    // what front-loads approval for the whole run; at execution time the cache
    // already holds them, so nothing prompts.
    let lifecycle_preflight = composition::resolve_shell_approvals(
        None,
        None,
        &approval_options,
        Some(&prepared.lifecycle),
        Some(&prepared.resolved_path),
    )?;
    approved.extend(lifecycle_preflight.approved_commands);

    Ok(StepComposition { prepared, approved })
}

/// The options one step both discovers its template shell commands with and
/// prepares with.
///
/// Each call opens the step's own document epoch: one launch-anchored
/// snapshot, constructed through the invocation owner and never from the step
/// document's location, so moving the document cannot change launch-facing
/// `ctx.*`, while its `SourceContext` (`ctx.file_resolution_context`) drives
/// file resolution. The epoch joins `ctx.run_evidence` when the step supplies
/// one, so every document of one step observes one view of Git state. `{{state}}`/`{{previous}}`/`{{next}}` render their `name`
/// in string context; whole-value and dotted access keep the typed object.
pub(super) fn step_prepare_options(
    source: &ResolvedCompositionSource,
    ctx: &StepComposeContext<'_>,
    overrides: LayeredOverrides,
    env_overrides: &BTreeMap<String, String>,
    allow_empty_body: bool,
) -> PrepareOptions {
    let document_epoch = match ctx.run_evidence {
        Some(run) => ctx.invocation.begin_document_epoch_in(run),
        None => ctx.invocation.begin_document_epoch(),
    };
    let requirements =
        darkmatter::markdown::compose::ContextRequirements::for_document(&source.markdown);
    let mut prepared_context = document_epoch.capture_launch_context(&requirements);
    for (key, value) in env_overrides {
        prepared_context.env_mut().insert(key.clone(), value.clone());
    }
    PrepareOptions {
        pre_approved_commands: None,
        env_overrides: env_overrides.clone(),
        perf_enabled: ctx.shared.perf,
        source_repo_root: ctx.source_repo_root.map(Path::to_path_buf),
        shell_working_directory: Some(ctx.child_cwd.to_path_buf()),
        prepared_context: Some(prepared_context),
        file_ref_fallback_dir: ctx.launch_area.map(Path::to_path_buf),
        caller_input_records: ctx.caller_input_records.clone(),
        name_coercion_keys: composition::sequence::reserved::NAME_COERCION_KEYS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        allow_empty_body,
        defer_schema_verdict: false,
        invocation_context: Some(ctx.invocation.clone()),
        document_epoch: Some(document_epoch),
        ..PrepareOptions::new(ctx.file_resolution_context.clone())
    }
    .with_layered_overrides(overrides)
}

#[cfg(test)]
mod tests;
