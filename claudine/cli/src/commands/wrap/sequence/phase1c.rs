//! Phase 1c — sequence-wide schema validation and shell approval.
//!
//! Every step is validated and approved here, before any step runs: `$schema`
//! required-property gaps are aggregated across all steps so the user fixes the
//! whole sequence in one edit, and every template and lifecycle shell command is
//! resolved and approved so no approval prompt can interrupt a running sequence.
//!
//! What this pass deliberately does **not** produce is a composed document per
//! step. Each step composes at its turn against the *live* source and the
//! accumulated runtime layers (see [`super::jit`]); a composition retained from
//! here would be stale the moment an earlier step wrote back a body or `set` a
//! value. The compose performed here is a validation probe whose result is
//! discarded — only its diagnostics and its approved command bytes survive.

use std::collections::{BTreeMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::sequence::build_step_overlay;
use claudine::composition::{
    self, CompositionError, LIFECYCLE_EVENT_KEYS, MissingProperty, PrepareOptions,
    PreparedComposition, ResolvedCompositionSource, SequenceMissingPropertiesStep, SequencePlan,
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
    pub(super) approved_commands: HashSet<String>,
    /// The user `--set` overrides after any interactive collection pass, so the
    /// values the user supplied here are the ones every step composes against.
    pub(super) resolved_overrides: Option<serde_json::Value>,
}

/// Run Phase 1c (sequence-wide validation) with schema validation and
/// aggregated missing-property handling.
///
/// On the first pass each step is composed via
/// [`composition::prepare_direct_with_schema`]. Missing-property errors
/// are collected per-step; all other failures short-circuit immediately
/// (including [`CompositionError::SchemaValidation`] for invalid required
/// values). When at least one step reports missing properties and
/// Interactive Mode is allowed, the deduplicated missing set is collected
/// via `biscuit-tui` prompts and the loop re-runs with the merged
/// overrides. When Interactive Mode is not allowed, an aggregated
/// [`CompositionError::SequenceMissingProperties`] is returned so the
/// user can fix the full sequence in one edit.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_phase_1c_with_schema(
    source: &ResolvedCompositionSource,
    plan: &SequencePlan,
    resolved_targets: &[Option<claudine::composition::ResolvedExecutionTarget>],
    user_set_overrides: &Option<serde_json::Value>,
    ctx: &StepComposeContext<'_>,
    effective_fail_fast: bool,
    initial_cumulative_approved: HashSet<String>,
    interrupted: &Arc<AtomicBool>,
    silent: bool,
) -> Result<Option<SequencePreflight>> {
    let mut overrides = user_set_overrides.clone();
    // At most one interactive collection pass — collected values are
    // shared across all steps that need the same property, and a second
    // attempt should always succeed (or fail with a different error).
    for attempt in 0..2 {
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
                if attempt > 0 {
                    // We already collected values once. If we're still
                    // seeing missing values, surface them.
                    return Err(into_sequence_missing(contexts).into());
                }
                // Honor `interactive.allowed()` first so non-TTY runs
                // produce the aggregated per-step report — matching the
                // direct `compose` path in
                // `pre_validate_with_interactive_collection`. Promoting
                // unsupported shapes ahead of this check would force
                // every non-TTY sequence with e.g. `object(required)` to
                // surface as `UnsupportedInteractiveSchema` instead of
                // the actionable aggregated `MissingProperties` report.
                let interactive = resolve_interactive_options(ctx.shared.silent);
                if !interactive.allowed() {
                    return Err(into_sequence_missing(contexts).into());
                }
                // Interactive Mode is allowed: now promote the first
                // missing property whose schema shape cannot be collected
                // via `biscuit-tui` (raw JSON Schema, `object`, `any`,
                // property-level union) so the user sees a targeted
                // `UnsupportedInteractiveSchema` error rather than a
                // generic aggregated report we can't satisfy.
                if let Some((source_path, property, shape)) =
                    find_first_unsupported(&failures_slice(&contexts))
                {
                    return Err(CompositionError::UnsupportedInteractiveSchema {
                        source_path,
                        property,
                        shape,
                    }
                    .into());
                }
                let collected = match collect_sequence_missing_values(
                    &contexts,
                    silent,
                    ctx.launch_area,
                    ctx.file_resolution_context,
                ) {
                    Ok(values) => values,
                    Err(_) => {
                        // Ctrl-C / Esc / unsupported shape — surface the
                        // aggregated error so the user sees the actionable
                        // non-TTY report.
                        return Err(into_sequence_missing(contexts).into());
                    }
                };
                if collected.is_empty() {
                    return Err(into_sequence_missing(contexts).into());
                }
                overrides = Some(merge_overrides(overrides.as_ref(), collected));
            }
        }
    }
    // Loop bound enforces two attempts; the inner match returns from
    // either branch so this point is unreachable.
    unreachable!("phase 1c attempt loop exited without resolving")
}

/// Per-step missing-properties context.
///
/// Pairs the typed `SequenceMissingPropertiesStep` (returned to the user
/// as part of `CompositionError::SequenceMissingProperties`) with the
/// post-overlay effective override map for that step. The effective
/// overrides are needed by [`build_schema_status_report`] so the
/// pre-prompt diagnostic reflects what the prepare pipeline will
/// validate against — values supplied by reserved per-step overlay keys
/// and CLI `--set` / shorthand setters must show as `Valid` (or omitted)
/// rather than as `Missing`.
struct StepMissingContext {
    failure: SequenceMissingPropertiesStep,
    effective_overrides: Option<serde_json::Value>,
}

enum Phase1cAttempt {
    Success(HashSet<String>),
    Missing(Vec<StepMissingContext>),
    Interrupted,
}

/// Borrow just the failure records out of a `[StepMissingContext]` slice
/// for helpers that already operate on `&[SequenceMissingPropertiesStep]`.
fn failures_slice(contexts: &[StepMissingContext]) -> Vec<SequenceMissingPropertiesStep> {
    contexts.iter().map(|c| c.failure.clone()).collect()
}

/// Convert per-step contexts into the typed aggregated error variant.
fn into_sequence_missing(contexts: Vec<StepMissingContext>) -> CompositionError {
    let failures: Vec<SequenceMissingPropertiesStep> =
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
    initial_cumulative_approved: HashSet<String>,
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

        // Validation, approval, and composition run through the very same path
        // execution uses at each step's turn, so a step that validates here
        // cannot fail differently there for want of a second implementation.
        match super::jit::compose_step(
            source,
            ctx,
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
                    failure: SequenceMissingPropertiesStep {
                        step: step_index + 1,
                        step_name: step.name.clone(),
                        source_path,
                        missing,
                        frontmatter_description,
                        pointer_paths,
                    },
                    effective_overrides: Some(step_set_overrides.clone()),
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
/// unique `(name, type_label, description)` exactly once.
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
                )
            {
                let status = Status::from_prose(format!(
                    "<b>Step {}:</b> <cyan>{}</cyan>",
                    failure.step, failure.step_name
                ))
                .state(StatusState::Info);
                log::message(&status.render(&term));
                render_status_report(&report, &term);
            }
        }
    }

    // Build a deduped list, keyed by (name, type_label, description).
    let mut seen_keys: HashSet<(String, Option<String>, Option<String>)> = HashSet::new();
    let mut unique: Vec<MissingProperty> = Vec::new();
    for ctx in contexts {
        for prop in &ctx.failure.missing {
            let key = (
                prop.name.clone(),
                prop.type_label.clone(),
                prop.description.clone(),
            );
            if seen_keys.insert(key) {
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
    failures: &[SequenceMissingPropertiesStep],
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
