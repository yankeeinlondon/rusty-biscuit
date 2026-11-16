//! Phase 1c — sequence-wide schema validation and shell approval.
//!
//! Every step is validated and approved here, before any step runs: `$schema`
//! required-property gaps are aggregated across all steps so the user fixes the
//! whole sequence in one edit, and every template and lifecycle shell command is
//! discovered and approved without executing document shell stages.
//!
//! Before any of that, [`prepare_root_plan`] inventories every owning
//! document's inputs (the root, a whole document the root references, and
//! every whole document a nested task references), collects the missing ones
//! in one pass, and only then fills the root's templates into its plan, so a
//! missing input is reported before a template that reads it can fail.
//! Collection is bounded by progress: a pass that asks no new property ends
//! with the combined report, and no property is ever asked twice.
//!
//! What this pass deliberately does **not** produce is a composed document per
//! step. Each step composes at its turn against the *live* source and the
//! accumulated runtime layers (see [`super::jit`]); a composition retained from
//! here would be stale the moment an earlier step wrote back a body or `set` a
//! value. The compose performed here is a validation probe whose result is
//! discarded — only its diagnostics and approved bytes/shapes survive.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::sequence::build_step_overlay;
use claudine::composition::{
    self, CompositionError, MissingProperty, PrepareOptions,
    PreparedComposition, ResolvedCompositionSource, SequenceMissingPropertiesEntry, SequenceMissingSubject, SequencePlan,
};
use claudine::harness::ShellApprovalOptions;
use color_eyre::eyre::{Result, eyre};

use crate::commands::compose::SharedComposeArgs;
use crate::commands::schema_interactive::{
    collect_missing_values, emit_dropped_optional_warnings, render_status_report,
    resolve_interactive_options,
};
use super::jit::StepComposeContext;
use crate::log;

/// What the sequence-wide validation pass leaves behind for execution.
pub(super) struct SequencePreflight {
    /// Every shell command approved across all steps, template and lifecycle
    /// alike. Handed to each just-in-time prepare as `pre_approved_commands`, so
    /// a running sequence never stops for an approval prompt.
    pub(super) approved_commands: darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals,
    /// The user `--set` overrides after any interactive collection pass, so the
    /// values the user supplied here are the ones every step composes against.
    pub(super) resolved_overrides: Option<serde_json::Value>,
}

/// Run Phase 1c (sequence-wide validation) with schema validation and
/// aggregated missing-property handling.
///
/// Each pass composes every step as a validation probe. Missing-property
/// errors are collected per step; all other failures short-circuit
/// immediately (including [`CompositionError::SchemaValidation`] for invalid
/// required values). When at least one step reports missing properties and
/// Interactive Mode is allowed, the deduplicated properties not already in
/// `asked` (including those input discovery asked) are collected via
/// `biscuit-tui` prompts and the pass re-runs with the merged overrides; a
/// pass that would ask nothing new reports instead. When Interactive Mode is
/// not allowed, an aggregated [`CompositionError::SequenceMissingProperties`]
/// is returned so the user can fix the full sequence in one edit.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_phase_1c_with_schema(
    source: &ResolvedCompositionSource,
    plan: &SequencePlan,
    resolved_targets: &[Option<claudine::composition::ResolvedExecutionTarget>],
    user_set_overrides: &Option<serde_json::Value>,
    ctx: &StepComposeContext<'_>,
    effective_fail_fast: bool,
    initial_cumulative_approved: darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals,
    interrupted: &Arc<AtomicBool>,
    silent: bool,
    asked: &mut HashSet<CollectionKey>,
) -> Result<Option<SequencePreflight>> {
    let mut overrides = user_set_overrides.clone();
    // Each pass either succeeds, or asks at least one property not asked
    // before (here or during input discovery); otherwise it reports.
    loop {
        let attempt_result = run_phase_1c_attempt(
            source,
            plan,
            resolved_targets,
            &overrides,
            ctx,
            effective_fail_fast,
            initial_cumulative_approved.clone(),
            interrupted,
        )?;

        match attempt_result {
            Phase1cAttempt::Interrupted => {
                return Ok(None);
            }
            Phase1cAttempt::Success(approved) => {
                return Ok(Some(SequencePreflight {
                    approved_commands: approved,
                    resolved_overrides: overrides,
                }));
            }
            Phase1cAttempt::Missing(contexts) => {
                let collected = collect_or_report(contexts, silent, ctx.launch_area, ctx.file_resolution_context, asked)?;
                overrides = Some(merge_overrides(overrides.as_ref(), collected));
            }
        }
    }
}

/// Collect the missing properties `contexts` report, or return their combined
/// report when they cannot be collected: outside Interactive Mode, for a
/// shape no prompt can collect, when the user cancels, or when every one of
/// them was already asked (`asked`), so no property is asked twice.
fn collect_or_report(
    contexts: Vec<StepMissingContext>,
    silent: bool,
    launch_area: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    asked: &mut HashSet<CollectionKey>,
) -> Result<serde_json::Map<String, serde_json::Value>> {
    // Honor `interactive.allowed()` first so non-TTY runs produce the
    // aggregated report — matching the direct `compose` path in
    // `pre_validate_with_interactive_collection`. Promoting unsupported
    // shapes ahead of this check would force every non-TTY sequence with
    // e.g. `object(required)` to surface as `UnsupportedInteractiveSchema`
    // instead of the actionable aggregated `MissingProperties` report.
    let interactive = resolve_interactive_options(silent);
    if !interactive.allowed() {
        return Err(into_sequence_missing(contexts).into());
    }
    // Interactive Mode is allowed: promote the first missing property whose
    // schema shape cannot be collected via `biscuit-tui` (raw JSON Schema,
    // `object`, `any`, property-level union) so the user sees a targeted
    // `UnsupportedInteractiveSchema` error rather than a generic aggregated
    // report we can't satisfy.
    if let Some((source_path, property, shape)) = find_first_unsupported(&failures_slice(&contexts)) {
        return Err(CompositionError::UnsupportedInteractiveSchema { source_path, property, shape }.into());
    }
    let fresh = contexts.iter()
        .flat_map(|context| context.failure.missing.iter())
        .any(|property| !asked.contains(&collection_key(property)));
    if !fresh {
        return Err(into_sequence_missing(contexts).into());
    }
    match collect_sequence_missing_values(&contexts, silent, launch_area, file_resolution_context, asked) {
        // Ctrl-C / Esc / unsupported shape — surface the aggregated error so
        // the user sees the actionable non-TTY report.
        Ok(values) if !values.is_empty() => Ok(values),
        _ => Err(into_sequence_missing(contexts).into()),
    }
}

/// One collection prompt per distinct `(name, type_label, description)`.
pub(super) type CollectionKey = (String, Option<String>, Option<String>);

fn collection_key(property: &MissingProperty) -> CollectionKey {
    (property.name.clone(), property.type_label.clone(), property.description.clone())
}

/// The root sequence before its plan exists: what [`prepare_root_plan`]
/// fills the plan from.
pub(crate) struct RootSequence<'a> {
    /// The invocation token, allocated before the root's `initialize` so its
    /// events and the plan share one `seq.run_id`.
    pub(crate) sequence_id: String,
    /// The root document as authored, with its `$state`/`template:` and its
    /// inline containers' declarations, which the composition view drops.
    pub(crate) authored: ResolvedCompositionSource,
    /// Expander for a `$( … )` source.
    pub(crate) shell_runner: claudine::composition::ShellSourceRunner<'a>,
}

/// The invocation inputs [`prepare_root_plan`] discovers and plans under.
pub(super) struct RootPlanRequest<'a> {
    pub(super) root: &'a RootSequence<'a>,
    /// Writes the root's `initialize` made, above the caller's setters.
    pub(super) runtime_writes: serde_json::Map<String, serde_json::Value>,
    pub(super) caller_input_records: &'a darkmatter::markdown::compose::CallerInputRecords,
    /// The context a caller's value without a record resolves from.
    pub(super) launch: &'a biscuit_file::FileResolutionContext,
    pub(super) compose_context: &'a darkmatter::markdown::compose::ComposeContext,
    pub(super) invocation: &'a claudine::invocation_context::InvocationContext,
    pub(super) source_context: &'a claudine::invocation_context::SourceContext,
    pub(super) file_resolution_context: &'a biscuit_file::FileResolutionContext,
    pub(super) launch_area: Option<&'a std::path::Path>,
    pub(super) silent: bool,
}

/// The root plan, with what preparing it found.
pub(super) struct RootPlan {
    pub(super) plan: SequencePlan,
    /// The inventory of the last discovery pass, whose parses the graph
    /// walk reuses.
    pub(super) inventory: claudine::composition::SequenceInputInventory,
    /// The caller inputs (and `initialize` writes) the plan was filled under.
    pub(super) layers: claudine::composition::CallerInputLayers,
    /// The caller's setters plus every value collected here.
    pub(super) overrides: Option<serde_json::Value>,
    /// Whether an owner's preparation failed in a way only
    /// [`RootPlanRequest::recheck_owners`] reports.
    pub(super) recheck_owners: bool,
}

impl RootPlanRequest<'_> {
    fn layers(&self, overrides: Option<&serde_json::Value>) -> claudine::composition::CallerInputLayers {
        let mut layers = claudine::composition::CallerInputLayers::from_caller_overrides(overrides.cloned(), self.launch.clone());
        layers.caller_input_records.extend(self.caller_input_records.iter().map(|(key, record)| (key.clone(), record.clone())));
        if !self.runtime_writes.is_empty() {
            let mut values = overrides.and_then(serde_json::Value::as_object).cloned().unwrap_or_default();
            values.extend(self.runtime_writes.clone());
            layers.set_overrides = Some(serde_json::Value::Object(values));
        }
        layers.file_ref_fallback_dir = self.launch_area.map(std::path::Path::to_path_buf);
        layers
    }

    /// Report the preparation failure discovery recorded for an owner, once
    /// the graph's structure and provider selection have been checked.
    pub(super) fn recheck_owners(&self, layers: &claudine::composition::CallerInputLayers) -> Result<()> {
        let options = composition::InputDiscoveryOptions::new(self.file_resolution_context, self.compose_context)
            .with_caller_layers(layers)
            .with_invocation(self.invocation, self.source_context);
        composition::discover_sequence_inputs(&self.root.authored, options)?;
        Ok(())
    }

    fn plan(&self, layers: &claudine::composition::CallerInputLayers) -> Result<SequencePlan> {
        let options = composition::SequenceSourceOptions::new(self.file_resolution_context)
            .with_shell_runner(self.root.shell_runner)
            .with_compose_context(self.compose_context)
            .with_caller_layers(layers)
            .with_invocation(self.invocation);
        let plan = composition::resolve_sequence_plan_with(&self.root.authored, options)?.ok_or_else(|| {
            eyre!(
                "file '{}' does not define a `sequence` frontmatter property",
                self.root.authored.resolved_path.display()
            )
        })?;
        Ok(plan.with_sequence_id(&self.root.sequence_id))
    }
}

/// Discover every owning document's inputs, collect the missing ones, and
/// fill the root's plan.
///
/// The root document's own missing inputs are left to Phase 1c when nothing
/// else is missing and the plan has steps: every root step composes the root
/// document, so Phase 1c reports (and collects) them per step, as it always
/// has. Any other owner's missing inputs, a reference waiting on one, or an
/// empty root that is missing one, are collected (or reported) here.
///
/// ## Errors
///
/// The combined [`CompositionError::SequenceMissingProperties`] report when
/// inputs are missing and cannot be collected, and any discovery or plan
/// error.
pub(super) fn prepare_root_plan(
    request: &RootPlanRequest<'_>,
    mut overrides: Option<serde_json::Value>,
    asked: &mut HashSet<CollectionKey>,
) -> Result<RootPlan> {
    loop {
        let layers = request.layers(overrides.as_ref());
        let options = composition::InputDiscoveryOptions::new(request.file_resolution_context, request.compose_context)
            .with_caller_layers(&layers)
            .with_invocation(request.invocation, request.source_context);
        // An owner's own verdict (an invalid value, a removed spelling) is
        // reported after the graph's structure and provider selection, as it
        // always was: the root's by Phase 1c's per-step probes, every other
        // owner's by `RootPlan::recheck_owners`.
        let inventory = composition::discover_sequence_inputs(&request.root.authored, options.leaving_verdicts())?;
        let mut missing = inventory.owners.iter().filter(|owner| owner.missing.is_some()).peekable();
        let plan = if missing.peek().is_none() {
            Some(request.plan(&layers)?)
        } else if inventory.pending.is_empty() && missing.all(|owner| owner.role == composition::InputOwnerRole::Root) {
            request.plan(&layers).ok().filter(|plan| !plan.steps.is_empty())
        } else {
            None
        };
        if let Some(plan) = plan {
            // No step probes an empty root, so its failure is rechecked too.
            let recheck_owners = inventory.failed_owners.iter()
                .any(|&owner| inventory.owners[owner].role != composition::InputOwnerRole::Root || plan.steps.is_empty());
            return Ok(RootPlan { plan, inventory, layers, overrides, recheck_owners });
        }
        let contexts = inventory.missing_entries().into_iter()
            .map(|failure| StepMissingContext { failure, effective_overrides: layers.set_overrides.clone() })
            .collect();
        let collected = collect_or_report(contexts, request.silent, request.launch_area, request.file_resolution_context, asked)?;
        overrides = Some(merge_overrides(overrides.as_ref(), collected));
    }
}

/// Per-step missing-properties context.
///
/// Pairs the typed `SequenceMissingPropertiesEntry` (returned to the user
/// as part of `CompositionError::SequenceMissingProperties`) with the
/// post-overlay effective override map for that step. The effective
/// overrides are needed by [`build_schema_status_report`] so the
/// pre-prompt diagnostic reflects what the prepare pipeline will
/// validate against — values supplied by reserved per-step overlay keys
/// and CLI `--set` / shorthand setters must show as `Valid` (or omitted)
/// rather than as `Missing`.
struct StepMissingContext {
    failure: SequenceMissingPropertiesEntry,
    effective_overrides: Option<serde_json::Value>,
}

enum Phase1cAttempt {
    Success(darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals),
    Missing(Vec<StepMissingContext>),
    Interrupted,
}

/// Borrow just the failure records out of a `[StepMissingContext]` slice
/// for helpers that already operate on `&[SequenceMissingPropertiesEntry]`.
fn failures_slice(contexts: &[StepMissingContext]) -> Vec<SequenceMissingPropertiesEntry> {
    contexts.iter().map(|c| c.failure.clone()).collect()
}

/// Convert per-step contexts into the typed aggregated error variant.
fn into_sequence_missing(contexts: Vec<StepMissingContext>) -> CompositionError {
    let failures: Vec<SequenceMissingPropertiesEntry> =
        contexts.into_iter().map(|c| c.failure).collect();
    CompositionError::SequenceMissingProperties {
        failure_count: failures.len(),
        failures,
    }
}

#[allow(clippy::too_many_arguments)]
fn run_phase_1c_attempt(
    source: &ResolvedCompositionSource,
    plan: &SequencePlan,
    resolved_targets: &[Option<claudine::composition::ResolvedExecutionTarget>],
    user_set_overrides: &Option<serde_json::Value>,
    ctx: &StepComposeContext<'_>,
    effective_fail_fast: bool,
    initial_cumulative_approved: darkmatter::markdown::compose::shell_expansion::template::ShellPreApprovals,
    interrupted: &Arc<AtomicBool>,
) -> Result<Phase1cAttempt> {
    let total_steps = plan.steps.len();
    let mut cumulative_approved = initial_cumulative_approved;
    let mut missing_contexts: Vec<StepMissingContext> = Vec::new();

    for step_index in 0..total_steps {
        if interrupted.load(Ordering::SeqCst) {
            return Ok(Phase1cAttempt::Interrupted);
        }
        // Validation layers against the *initial* runtime state — empty
        // `outputs`, no mutations — which is exactly what the first step sees
        // and what `--dry-run` composes against.
        let step_set_overrides = super::jit::step_set_overrides(
            plan,
            step_index,
            user_set_overrides.as_ref(),
            None,
        );

        let target = resolved_targets
            .get(step_index)
            .ok_or_else(|| eyre!("missing resolved target for step {}", step_index + 1))?;
        let env_overrides =
            super::jit::step_env_overrides(effective_fail_fast, ctx.shared, target.as_ref());

        // Share execution's option builder while keeping document shell stages
        // passive until the step reaches its turn.
        let step_context = StepComposeContext {
            lifecycle_group: None,
            loop_values: None,
            validation_probe: true,
            deferred_shell_roots: super::jit::sequence_shell_roots(step_index),
            ..ctx.clone()
        };
        match super::jit::compose_step(
            source,
            &step_context,
            &step_set_overrides,
            &env_overrides,
            std::mem::take(&mut cumulative_approved),
            plan.steps[step_index].executable.is_some(),
        ) {
            Ok(composed) => {
                cumulative_approved = composed.approved;
                // The composed document itself is discarded: execution
                // re-composes this step at its turn against the live file and
                // the accumulated runtime layers.
                drop(composed.prepared);
            }
            Err(CompositionError::MissingProperties {
                source_path,
                missing,
                frontmatter_description,
                pointer_paths,
            }) => {
                let step = &plan.steps[step_index];
                missing_contexts.push(StepMissingContext {
                    failure: SequenceMissingPropertiesEntry {
                        subject: SequenceMissingSubject::Step { number: step_index + 1, name: step.name.clone() },
                        source_path,
                        missing,
                        frontmatter_description,
                        pointer_paths,
                    },
                    effective_overrides: Some(step_set_overrides.to_value()),
                });
                // Continue so every step's missing properties are accumulated
                // and the user fixes the whole sequence in one edit.
                continue;
            }
            Err(other) => return Err(other.into()),
        }
    }

    if !missing_contexts.is_empty() {
        return Ok(Phase1cAttempt::Missing(missing_contexts));
    }
    Ok(Phase1cAttempt::Success(cumulative_approved))
}

/// Dedupe missing properties across all failed steps and prompt for each
/// unique `(name, type_label, description)` not already in `asked`, adding
/// each one asked.
///
/// Returns a JSON map of `{ property_name: collected_value }` suitable for
/// merging into the user `--set` overrides. The first failure that
/// declared the property supplies the metadata used for the prompt.
///
/// The per-step status report uses `StepMissingContext::effective_overrides`
/// so a required property already satisfied by a reserved per-step overlay
/// value or by a CLI `--set` / shorthand setter is not reported as
/// `Missing` while the user is being prompted for a different property
/// (matches the direct `compose` status-drift fix from review-5).
fn collect_sequence_missing_values(
    contexts: &[StepMissingContext],
    silent: bool,
    launch_area: Option<&std::path::Path>,
    file_resolution_context: &biscuit_file::FileResolutionContext,
    asked: &mut HashSet<CollectionKey>,
) -> std::io::Result<serde_json::Map<String, serde_json::Value>> {
    if !silent {
        let term = log::terminal();
        // Render one status report per step so the user sees the same
        // diagnostic they get for direct compose.
        for ctx in contexts {
            let failure = &ctx.failure;
            // Build a minimal source view from the recorded path for the
            // status report. The library helper expects a resolved source
            // so we re-resolve here; if that fails we fall through with
            // a header-only note.
            if let Ok(source) = composition::resolve_composition_source_in_context(
                &failure.source_path.display().to_string(),
                file_resolution_context,
            )
                && let Ok(Some(report)) = composition::build_schema_status_report(
                    &source,
                    ctx.effective_overrides.as_ref(),
                    launch_area,
                    file_resolution_context,
                    // Display only: the step's own pre-validation already
                    // recorded the triggers its discovery skipped.
                    &composition::TriggerFindings::default(),
                )
            {
                let heading = match &failure.subject {
                    SequenceMissingSubject::Step { number, name } => format!("<b>Step {number}:</b> <cyan>{name}</cyan>"),
                    SequenceMissingSubject::Document { property } if property.is_empty() => "<b>Document</b>".to_string(),
                    SequenceMissingSubject::Document { property } => format!("<b>Document</b> <cyan>`{property}`</cyan>"),
                };
                let status = Status::from_prose(heading)
                .state(StatusState::Info);
                log::message(&status.render(&term));
                render_status_report(&report, &term);
            }
        }
    }

    // A deduped list of properties not asked before, keyed by (name,
    // type_label, description).
    let mut unique: Vec<MissingProperty> = Vec::new();
    for ctx in contexts {
        for prop in &ctx.failure.missing {
            if asked.insert(collection_key(prop)) {
                unique.push(prop.clone());
            }
        }
    }

    collect_missing_values(&unique)
}

/// Locate the first missing property across all sequence step failures
/// whose schema shape cannot be collected interactively.
///
/// Returns `(source_path, property_name, shape_label)` so the caller can
/// build a typed [`CompositionError::UnsupportedInteractiveSchema`] that
/// matches the direct `compose` path. The shape label falls back to
/// `(unknown)` when the property has no recorded type label.
pub(super) fn find_first_unsupported(
    failures: &[SequenceMissingPropertiesEntry],
) -> Option<(std::path::PathBuf, String, String)> {
    for failure in failures {
        for prop in &failure.missing {
            if prop.interactive_shape.is_none() {
                return Some((
                    failure.source_path.clone(),
                    prop.name.clone(),
                    prop.type_label
                        .clone()
                        .unwrap_or_else(|| "(unknown)".to_string()),
                ));
            }
        }
    }
    None
}

fn merge_overrides(
    base: Option<&serde_json::Value>,
    collected: serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut out = match base {
        Some(serde_json::Value::Object(map)) => map.clone(),
        _ => serde_json::Map::new(),
    };
    for (k, v) in collected {
        out.insert(k, v);
    }
    serde_json::Value::Object(out)
}
