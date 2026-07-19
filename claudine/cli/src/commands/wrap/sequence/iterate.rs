//! Just-in-time serial execution (Phase 2) for the sequence orchestrator.
//!
//! Each step is composed **at its turn**, against the live file on disk and the
//! runtime layers as they stand at that moment. That ordering is the contract:
//! step 3 sees step 2's inline-compose write-back, step 2's `set` mutations, and
//! step 2's committed `outputs` entry, none of which exist when the sequence
//! starts. Nothing here re-resolves the *step list* — dynamic sources snapshot
//! once during static preflight.
//!
//! A step either runs the source document's own body (the default) or the one
//! executable its declaration named, which replaces the body entirely. Both
//! paths report the same [`SequenceStepResult`], so `fail_fast`, the summary,
//! and the interrupt story do not branch on task kind.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use biscuit_terminal::components::horizontal_rule::{
    HorizontalRule, RuleAlignment, RuleStyle, RuleWeight,
};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::{
    self, CompositionError, CompositionExecutionRequest, CompositionMode, PreflightGraph,
    ResolvedCompositionSource, RuntimeState, SequencePlan, SequenceRunSummary, SequenceStepResult,
    SequenceTaskResult,
};
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::Result;
use serde_json::Value;
use tracing::{debug, info_span};

use crate::commands::compose::SharedComposeArgs;
use crate::commands::wrap::composition::{CompositionPrepContext, execute_composition_request_inner};
use crate::commands::schema_interactive::{collect_missing_values, resolve_interactive_options};
use crate::log;

use super::jit::{self, StepComposeContext};
use super::task_run;

/// Everything the serial loop needs that does not vary per step.
pub(super) struct SequenceRunContext<'a> {
    pub(super) plan: &'a SequencePlan,
    pub(super) graph: &'a PreflightGraph,
    pub(super) resolved_targets: &'a [Option<claudine::composition::ResolvedExecutionTarget>],
    pub(super) source: &'a ResolvedCompositionSource,
    pub(super) prep_context: &'a CompositionPrepContext,
    pub(super) shared: &'a SharedComposeArgs,
    pub(super) compose: &'a StepComposeContext<'a>,
    /// Every command approved during the sequence-wide validation pass, so a
    /// step's just-in-time prepare never stops to prompt.
    pub(super) approved: HashSet<String>,
    /// User `--set` overrides after the single interactive collection pass.
    pub(super) user_set_overrides: Option<Value>,
    pub(super) interrupted: &'a Arc<AtomicBool>,
    pub(super) effective_fail_fast: bool,
    pub(super) silent: bool,
    pub(super) verbose: u8,
    pub(super) perf_enabled: bool,
}

/// How one step finished, before it becomes a [`SequenceStepResult`].
pub(super) enum StepOutcome {
    Succeeded {
        /// The provider that ran, when one did and can be named.
        provider: Option<claudine::provider::Provider>,
        agent_perf: Option<crate::perf::AgentExecutionPerf>,
        /// This step's own composition cost. It is metered *inside* the step's
        /// wall clock now, which is what lets the report nest it under the step.
        compose_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
        /// Group member outcomes, when the step ran a group.
        tasks: Vec<SequenceTaskResult>,
    },
    Failed {
        message: String,
        agent_perf: Option<crate::perf::AgentExecutionPerf>,
        compose_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
        tasks: Vec<SequenceTaskResult>,
    },
    Interrupted {
        agent_perf: Option<crate::perf::AgentExecutionPerf>,
        compose_perf: Option<darkmatter::markdown::compose::ComposePerfReport>,
        tasks: Vec<SequenceTaskResult>,
    },
}

/// Run every step in order, composing each at its turn.
///
/// Returns the populated [`SequenceRunSummary`] and a flag indicating whether an
/// interrupt was observed (so the orchestrator can emit the standard `130`).
pub(super) fn run_sequence_steps(
    run: &SequenceRunContext<'_>,
    perf_accumulator: &mut Option<crate::perf::SequencePerfAccumulator>,
) -> Result<(SequenceRunSummary, bool)> {
    let total_steps = run.plan.steps.len();
    let mut summary = SequenceRunSummary {
        total_steps,
        succeeded: 0,
        failed: 0,
        steps: Vec::with_capacity(total_steps),
    };

    // One cell for the whole sequence: `outputs` accumulates across steps and a
    // `set` from step 1 is still visible in step 5.
    let runtime_state = std::sync::Arc::new(RuntimeState::new());

    let mut interrupt_observed = false;
    for step_index in 0..total_steps {
        if run.interrupted.load(Ordering::SeqCst) {
            interrupt_observed = true;
            if let Some(acc) = perf_accumulator.as_mut() {
                acc.set_partial();
            }
            break;
        }
        let step = &run.plan.steps[step_index];

        // Dry-run: separate each document's rendered metadata block with a
        // full-width horizontal rule on stderr. The per-document render happens
        // inside the executor; the rule sits *between* documents, so it precedes
        // docs 2..M. `--quiet` / `--silent` have no effect on dry-run output.
        if run.shared.dry_run && step_index > 0 {
            let hr = HorizontalRule::new()
                .style(RuleStyle::Dashes)
                .alignment(RuleAlignment::Full)
                .weight(RuleWeight::Medium)
                .render(&log::terminal());
            log::message(&hr);
        }

        let _step_span = info_span!(
            "sequence_step",
            step_index = step_index + 1,
            step_name = %step.name
        )
        .entered();

        if !run.silent {
            let status = Status::from_prose(format!(
                "[<yellow>{}/{}</yellow>] <i>starting</i> <b>{}</b>",
                step_index + 1,
                total_steps,
                step.name
            ))
            .state(StatusState::Info);
            log::message(&status.render(&log::terminal()));
        }

        let start = std::time::Instant::now();
        let outcome = run_one_step(run, step_index, &runtime_state);
        let duration = start.elapsed();

        let (success, error, provider, agent_perf, compose_perf, tasks) = match outcome {
            StepOutcome::Succeeded {
                provider,
                agent_perf,
                compose_perf,
                tasks,
            } => (true, None, provider, agent_perf, compose_perf, tasks),
            StepOutcome::Failed {
                message,
                agent_perf,
                compose_perf,
                tasks,
            } => (false, Some(message), None, agent_perf, compose_perf, tasks),
            StepOutcome::Interrupted {
                agent_perf,
                compose_perf,
                tasks,
            } => (
                false,
                Some("interrupted by SIGINT".to_string()),
                None,
                agent_perf,
                compose_perf,
                tasks,
            ),
        };
        let interrupted_step = error.as_deref() == Some("interrupted by SIGINT");

        if let Some(acc) = perf_accumulator.as_mut() {
            acc.add_step(crate::perf::SequenceStepPerf {
                step_index,
                step_name: step.name.clone(),
                wall_clock: duration,
                compose_perf,
                agent_perf,
                group_tasks: tasks
                    .iter()
                    .map(|task| crate::perf::SequenceTaskPerf {
                        name: task.name.clone(),
                        duration: task.duration,
                    })
                    .collect(),
            });
        }

        if success {
            summary.succeeded += 1;
        } else {
            summary.failed += 1;
        }
        summary.steps.push(SequenceStepResult {
            step: step_index + 1,
            name: step.name.clone(),
            success,
            error: error.clone(),
            duration,
            tasks: tasks.clone(),
        });

        if !run.silent {
            emit_step_status(run, step_index, total_steps, success, &error, provider);
            emit_group_task_breakdown(&tasks);
        }

        if interrupted_step {
            debug!(step_index = step_index + 1, step_name = %step.name, "sequence step interrupted");
            if let Some(acc) = perf_accumulator.as_mut() {
                acc.set_partial();
            }
            interrupt_observed = true;
            break;
        }
        if !success {
            debug!(step_index = step_index + 1, step_name = %step.name, error = ?error, "sequence step failed");
            if run.effective_fail_fast {
                if let Some(acc) = perf_accumulator.as_mut() {
                    acc.set_partial();
                }
                debug!(step_index = step_index + 1, fail_fast = true, "sequence fail-fast triggered");
                break;
            }
        }
    }

    Ok((summary, interrupt_observed))
}

/// Compose and run one step against the live file and current runtime layers.
fn run_one_step(
    run: &SequenceRunContext<'_>,
    step_index: usize,
    runtime_state: &std::sync::Arc<RuntimeState>,
) -> StepOutcome {
    // The live re-read is the boundary the spec ratifies: everything an earlier
    // step wrote to this file is visible from here on.
    let live = match composition::reload_composition_source(run.source) {
        Ok(live) => live,
        Err(error) => return StepOutcome::failed(&error),
    };

    let target = run.resolved_targets.get(step_index).cloned().flatten();
    let env_overrides =
        jit::step_env_overrides(run.effective_fail_fast, run.shared, target.as_ref());
    // Dry-run composes against the *initial* state so late-binding references
    // render exactly as a first-step composition would see them.
    let snapshot = (!run.shared.dry_run).then(|| runtime_state.snapshot());
    let set_overrides = jit::step_set_overrides(
        run.plan,
        step_index,
        run.user_set_overrides.as_ref(),
        snapshot.as_ref(),
    );

    // A step declaring an executable never sends the document body, so a
    // bodyless source is legal for it — that is the shape a directly invoked
    // `kind: sequence` YAML file always has.
    let has_executable = run
        .graph
        .steps
        .get(step_index)
        .is_some_and(|step| step.task.is_some());
    let composed = match compose_with_late_collection(
        run,
        &live,
        &set_overrides,
        &env_overrides,
        has_executable,
    ) {
        Ok(composed) => composed,
        Err(error) => return StepOutcome::failed(&error),
    };

    // A step that declared an executable runs it *instead of* the document
    // body; the composition above still happened, because the task's `params`,
    // `timeout`, and action stacks bind against this step's effective state.
    if let Some(task) = run.graph.steps.get(step_index).and_then(|s| s.task.as_ref()) {
        return task_run::run_step_task(
            run,
            task,
            &composed.prepared,
            &jit::reserved_overlay(run.plan, step_index),
            &env_overrides,
            target.as_ref(),
            runtime_state,
            composed.prepared.compose_perf.clone(),
        );
    }

    if composed.prepared.prompt.trim().is_empty() {
        return StepOutcome::failed(&CompositionError::SequenceStepNotExecutable {
            step: step_index + 1,
            name: run.plan.steps[step_index].name.clone(),
            path: live.resolved_path.clone(),
        });
    }

    let compose_perf = composed.prepared.compose_perf.clone();
    let request =
        build_body_request(run, &live, composed.prepared, target, env_overrides, runtime_state);
    match execute_composition_request_inner(request, run.verbose, None, run.perf_enabled) {
        Ok(outcome) if super::run_was_interrupted(outcome.exit_code, run.interrupted) => {
            StepOutcome::Interrupted {
                agent_perf: outcome.agent_perf,
                compose_perf,
                tasks: Vec::new(),
            }
        }
        Ok(outcome) if outcome.exit_code == 0 => StepOutcome::Succeeded {
            // A `--dry-run` step with an unresolved agent state has no real
            // provider, so it must not be attributed to the placeholder.
            provider: (!(run.shared.dry_run && target_was_unresolved(run, step_index)))
                .then_some(outcome.provider),
            agent_perf: outcome.agent_perf,
            compose_perf,
            tasks: Vec::new(),
        },
        Ok(outcome) => StepOutcome::Failed {
            message: format!(
                "provider {} exited with code {}",
                outcome.provider, outcome.exit_code
            ),
            agent_perf: outcome.agent_perf,
            compose_perf,
            tasks: Vec::new(),
        },
        Err(error) => StepOutcome::Failed {
            message: error.to_string(),
            agent_perf: None,
            compose_perf,
            tasks: Vec::new(),
        },
    }
}

/// Compose a step, prompting once for required properties an earlier step was
/// expected to `set` but did not.
///
/// This is the mid-sequence human-in-the-loop case the spec warns authors to
/// design away, and it behaves exactly like direct compose: interactive
/// collection on a TTY, a typed failure otherwise. The failure is a failure of
/// *this step* — preflight already passed, so `fail_fast` governs what happens
/// next rather than the abort-all rule.
#[allow(clippy::result_large_err)]
fn compose_with_late_collection(
    run: &SequenceRunContext<'_>,
    live: &ResolvedCompositionSource,
    set_overrides: &Value,
    env_overrides: &std::collections::BTreeMap<String, String>,
    allow_empty_body: bool,
) -> Result<jit::StepComposition, CompositionError> {
    let first = jit::compose_step(
        live,
        run.compose,
        set_overrides,
        env_overrides,
        run.approved.clone(),
        allow_empty_body,
    );
    let Err(CompositionError::MissingProperties { ref missing, .. }) = first else {
        return first;
    };
    if !resolve_interactive_options(run.shared.silent).allowed()
        || missing.iter().any(|p| p.interactive_shape.is_none())
    {
        return first;
    }
    let Ok(collected) = collect_missing_values(missing) else {
        return first;
    };
    if collected.is_empty() {
        return first;
    }

    let mut merged = match set_overrides {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    merged.extend(collected);
    jit::compose_step(
        live,
        run.compose,
        &Value::Object(merged),
        env_overrides,
        run.approved.clone(),
        allow_empty_body,
    )
}

fn target_was_unresolved(run: &SequenceRunContext<'_>, step_index: usize) -> bool {
    run.resolved_targets
        .get(step_index)
        .is_none_or(Option::is_none)
}

/// Build the execution request for a step that runs the source document body.
fn build_body_request(
    run: &SequenceRunContext<'_>,
    live: &ResolvedCompositionSource,
    prepared: claudine::composition::PreparedComposition,
    resolved_target: Option<claudine::composition::ResolvedExecutionTarget>,
    env_overrides: std::collections::BTreeMap<String, String>,
    runtime_state: &std::sync::Arc<RuntimeState>,
) -> CompositionExecutionRequest {
    let shared = run.shared;
    let resolved = shared.resolve_session_interactivity(prepared.selection_hints.interactive);
    CompositionExecutionRequest {
        mode: if run.compose.inline_mode {
            CompositionMode::InlineFrontmatterPrompt
        } else {
            CompositionMode::ChainedDocument
        },
        file_ref: live.original_ref.clone(),
        prepared,
        resolved_target,
        explicit_provider: shared.explicit_provider(),
        excluded: shared.excluded(),
        sequence: true,
        yolo: shared.yolo,
        include: shared.include.clone(),
        model: shared.model.clone(),
        output: shared.output,
        system_prompt_args: SystemPromptArgs {
            append_file: shared.append_system_prompt.clone(),
            replace_file: shared.replace_system_prompt.clone(),
        },
        timeout: shared.timeout.clone(),
        step_timeout: shared.step_timeout.clone(),
        stall_timeout: shared.stall_timeout.clone(),
        operation: shared.operation.clone(),
        sandbox: shared.sandbox,
        repo: shared.repo,
        dry_run: shared.dry_run,
        mcp: shared.mcp,
        mcp_use: shared.mcp_use.clone(),
        strict: shared.strict,
        session_interactive: resolved.value,
        session_interactive_source: resolved.source,
        quiet: shared.quiet,
        silent: shared.silent,
        env_overrides,
        shared_approval_cache: Some(Arc::clone(&run.compose.approval_cache)),
        installed_snapshot: Some(run.prep_context.installed_snapshot.clone()),
        prep_launch_workspace: Some(run.prep_context.launch_workspace.clone()),
        prep_launch_context: Some(run.prep_context.launch_context.clone()),
        prep_env_context: Some(run.prep_context.env_context.clone()),
        prep_launch_detection_error: run.prep_context.launch_detection_error.clone(),
        // Composition now happens at the step's turn, so the executor's own
        // header emit is timely.
        header_emitted: false,
        provider_args: shared.provider_args.clone(),
        provider_args_explicit: shared.provider_args_explicit,
        runtime_state: Some(std::sync::Arc::clone(runtime_state)),
        // The default body *is* the step: the executor owns its output commit.
        suppress_output_commit: false,
        // A bodyless sequence step renders under the step header, not a task bar.
        task_frame_writer: None,
    }
}

impl StepOutcome {
    /// A step that failed before anything launched.
    fn failed(error: &CompositionError) -> Self {
        StepOutcome::Failed {
            message: error.to_string(),
            agent_perf: None,
            compose_perf: None,
            tasks: Vec::new(),
        }
    }
}

/// List a group step's member tasks with their outcome and timing.
///
/// Names and timings only: `outputs` is the sole output accumulator, so nothing
/// a task produced is echoed here.
fn emit_group_task_breakdown(tasks: &[SequenceTaskResult]) {
    if tasks.is_empty() {
        return;
    }
    let mut list = UnorderedList::empty();
    for task in tasks {
        let outcome = match (task.success, task.interrupted) {
            (true, _) => "<green>succeeded</green>",
            (false, true) => "<yellow>interrupted</yellow>",
            (false, false) => "<red>failed</red>",
        };
        list.add(Prose::new(format!(
            "<b>{}</b> — {outcome} <dim><i>({:.1}s)</i></dim>",
            task.name,
            task.duration.as_secs_f64()
        )));
    }
    log::message(&list.render(&log::terminal()));
}

fn emit_step_status(
    run: &SequenceRunContext<'_>,
    step_index: usize,
    total_steps: usize,
    success: bool,
    error: &Option<String>,
    provider: Option<claudine::provider::Provider>,
) {
    let prose = match (success, provider) {
        (true, Some(provider)) => format!(
            "step <b><yellow>{}/{}</yellow></b> succeeded (<dim><i>via {}</i></dim>)",
            step_index + 1,
            total_steps,
            provider
        ),
        (true, None) if run.shared.dry_run => format!(
            "step <b><yellow>{}/{}</yellow></b> rendered (<dim><i>dry-run</i></dim>)",
            step_index + 1,
            total_steps,
        ),
        (true, None) => format!(
            "step <b><yellow>{}/{}</yellow></b> succeeded",
            step_index + 1,
            total_steps,
        ),
        (false, _) if error.as_deref() == Some("interrupted by SIGINT") => format!(
            "step <b><yellow>{}/{}</yellow></b> interrupted by Ctrl+C",
            step_index + 1,
            total_steps,
        ),
        (false, _) => format!(
            "step <b><yellow>{}/{}</yellow></b> failed: {}",
            step_index + 1,
            total_steps,
            error.as_deref().unwrap_or("unknown error")
        ),
    };
    let state = if success {
        StatusState::Success
    } else {
        StatusState::Error
    };
    log::message(&Status::from_prose(prose).state(state).render(&log::terminal()));
}
