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
    self, CompositionError, LIFECYCLE_EVENT_KEYS, PrepareOptions, PreparedComposition,
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
    pub(super) invocation: &'a claudine::invocation_context::InvocationContext,
    /// Invocation-anchored graph snapshot cloned into each step epoch.
    pub(super) launch_context: &'a darkmatter::markdown::compose::ComposeContext,
    /// Groups already present in `launch_context`.
    pub(super) launch_requirements:
        &'a darkmatter::markdown::compose::ContextRequirements,
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

/// Context ownership and resolution inputs for template shell preflight.
pub(super) struct TemplatePreflightContext<'a> {
    pub(super) launch_area: Option<&'a Path>,
    pub(super) file_resolution_context: Option<&'a biscuit_file::FileResolutionContext>,
    pub(super) invocation: Option<&'a claudine::invocation_context::InvocationContext>,
    pub(super) launch_context: Option<&'a darkmatter::markdown::compose::ComposeContext>,
    pub(super) launch_requirements:
        Option<&'a darkmatter::markdown::compose::ContextRequirements>,
}

/// One step's composed document, plus the approvals its composition contributed.
pub(super) struct StepComposition {
    pub(super) prepared: PreparedComposition,
    /// `approved` plus every command this step's template and lifecycle
    /// discovery added, so the next step inherits them.
    pub(super) approved: HashSet<String>,
}

/// Fold this step's `set_overrides` from the four just-in-time layers.
///
/// Lowest precedence first: user setters, accumulated runtime mutations, then
/// the reserved per-step overlay. The live document's own frontmatter is
/// Darkmatter's base and sits below all of them. Passing `runtime: None` yields
/// the initial view — empty `outputs`, no mutations — which is what the
/// validation pass and `--dry-run` compose against.
pub(super) fn step_set_overrides(
    plan: &SequencePlan,
    step_index: usize,
    user_setters: Option<&Value>,
    runtime: Option<&composition::RuntimeSnapshot>,
) -> Value {
    let overlay = reserved_overlay(plan, step_index);
    composition::layered_set_overrides(user_setters, runtime, Some(&overlay))
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
    let mut env: BTreeMap<String, String> = BTreeMap::new();
    env.insert(
        "CLAUDINE_FAIL_FAST".to_string(),
        effective_fail_fast.to_string(),
    );
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
    set_overrides: &Value,
    env_overrides: &BTreeMap<String, String>,
    approved: HashSet<String>,
    allow_empty_body: bool,
) -> Result<StepComposition, CompositionError> {
    // Schema pre-validation before the shell pass. A step whose effective
    // frontmatter is missing required values must report that rather than let
    // Darkmatter's compose surface a raw `SchemaValidationFailed`, which would
    // hide the property names the caller needs to collect or report.
    let pre = composition::pre_validate_schema(source, Some(set_overrides), ctx.launch_area)?;
    emit_dropped_optional_warnings(&pre.dropped_optionals);
    let step_source = pre.source;
    let step_overrides = pre
        .set_overrides
        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));

    let (compose_options, prepared_context) = build_template_preflight_options(
        env_overrides,
        &step_source.resolved_path,
        &step_source.markdown,
        &step_overrides,
        TemplatePreflightContext {
            launch_area: ctx.launch_area,
            file_resolution_context: Some(ctx.file_resolution_context),
            invocation: Some(ctx.invocation),
            launch_context: Some(ctx.launch_context),
            launch_requirements: Some(ctx.launch_requirements),
        },
    );

    let approval_options = super::super::apply_composition_shell_overrides(
        super::super::build_harness_shell_options_for_source_with_cache(
            &step_source.resolved_path,
            ctx.source_repo_root,
            Some(Arc::clone(&ctx.approval_cache)),
        ),
        ctx.shared.dry_run,
        ctx.shared.yolo,
    );

    let mut approved = approved;
    let template_preflight = composition::resolve_shell_approvals(
        Some(&step_source.markdown),
        Some(&compose_options),
        &approval_options,
        None,
        None,
    )?;
    approved.extend(template_preflight.approved_commands.iter().cloned());

    let prepare_options = PrepareOptions {
        set_overrides: Some(step_overrides),
        pre_approved_commands: Some(approved.clone()),
        env_overrides: env_overrides.clone(),
        perf_enabled: ctx.shared.perf,
        source_repo_root: ctx.source_repo_root.map(Path::to_path_buf),
        shell_working_directory: Some(ctx.child_cwd.to_path_buf()),
        // The shell audit and canonical preparation must expand against the
        // same captured snapshot even if the process CWD changes between them.
        prepared_context: Some(prepared_context),
        file_ref_fallback_dir: ctx.launch_area.map(Path::to_path_buf),
        file_resolution_context: Some(ctx.file_resolution_context.clone()),
        // `{{state}}`/`{{previous}}`/`{{next}}` render their `name` in string
        // context; whole-value/dotted access keeps the typed object.
        name_coercion_keys: composition::sequence::reserved::NAME_COERCION_KEYS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
        allow_empty_body,
        defer_schema_verdict: false,
        invocation_context: Some(ctx.invocation.clone()),
    };

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

/// Build the Darkmatter `ComposeOptions` for a step's template SHELL preflight.
///
/// The document path and request snapshot determine file-reference resolution.
/// `launch_area` is retained as diagnostic metadata and is not a candidate for
/// references authored by the template. `None` preserves the library-only
/// compatibility boundary.
pub(super) fn build_template_preflight_options(
    env_overrides: &BTreeMap<String, String>,
    source_path: &Path,
    markdown: &darkmatter::markdown::Markdown,
    set_overrides: &Value,
    runtime: TemplatePreflightContext<'_>,
) -> (
    darkmatter::markdown::compose::ComposeOptions,
    darkmatter::markdown::compose::ComposeContext,
) {
    let anchor = runtime
        .launch_area
        .or_else(|| source_path.parent())
        .unwrap_or_else(|| Path::new("."));
    let derived_source_context = runtime.invocation.map(|invocation| {
        invocation
            .derive_source(source_path)
            .expect("resolved sequence document always has a parent directory")
    });
    let mut ctx = match (
        runtime.invocation,
        runtime.launch_context,
        runtime.launch_requirements,
    ) {
        (Some(invocation), Some(launch_context), Some(launch_requirements)) => {
            let requirements =
                darkmatter::markdown::compose::ContextRequirements::for_document(markdown);
            let mut context = launch_context.clone();
            let mut captured = launch_requirements.clone();
            invocation.extend_launch_context(
                &mut context,
                &mut captured,
                &requirements,
            );
            context
        }
        _ => claudine::composition::capture_compatibility_context_for_document(anchor, markdown),
    };
    for (key, value) in env_overrides {
        ctx.env_mut().insert(key.clone(), value.clone());
    }
    let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(ctx.clone())
        .with_source_file(source_path)
        // Defer the lifecycle event keys (DM1), matching the main prepare pass.
        // The preflight compose exists only to discover template `::shell`
        // directives; without the exclusion it resolves the deferred lifecycle
        // subtree at compose time, so a `success`/`failure` read-side file
        // reference (a file a later event creates) trips the fatal file-ref
        // check before that event fires. Lifecycle shell commands are audited
        // separately via `collect_lifecycle_shell_commands`.
        .with_exclude_keys(LIFECYCLE_EVENT_KEYS.iter().copied());
    if let Some(source_context) = derived_source_context.as_ref() {
        opts = opts.with_file_resolution_context(source_context.file_resolution_context().clone());
    } else if let Some(context) = runtime.file_resolution_context {
        opts = opts.with_file_resolution_context(context.clone());
    }
    if let Some(launch_area) = runtime.launch_area {
        opts = opts.with_file_ref_fallback_dir(launch_area.to_path_buf());
    }
    opts = opts.with_set_overrides(set_overrides.clone());
    // Coerce `{{state}}`/`{{previous}}`/`{{next}}` to their `name` in string
    // context so a shell command's approved bytes match its executed bytes.
    opts = opts.with_name_coercion_keys(
        composition::sequence::reserved::NAME_COERCION_KEYS
            .iter()
            .map(|s| (*s).to_string())
            .collect(),
    );
    (opts, ctx)
}

#[cfg(test)]
mod tests;
