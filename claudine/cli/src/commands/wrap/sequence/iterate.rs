//! Per-step execution loop (Phase 2) for the sequence orchestrator.
//!
//! Iterates the prepared step contexts produced by Phase 1c, dispatches each
//! step through the wrapper-grade composition executor, and builds the
//! [`SequenceRunSummary`] consumed by the final report.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use biscuit_terminal::components::horizontal_rule::{
    HorizontalRule, RuleAlignment, RuleStyle, RuleWeight,
};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, ResolvedCompositionSource, SequencePlan,
    SequenceRunSummary, SequenceStepResult,
};
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::Result;
use tracing::{debug, info_span};

use crate::commands::compose::SharedComposeArgs;
use crate::commands::wrap::composition::{execute_composition_request_inner, CompositionPrepContext};
use crate::log;

use super::phase1c::StepContext;
use super::SEQUENCE_INTERRUPT_EXIT_CODE;

/// Run the prepared sequence steps through the wrapper-grade executor.
///
/// Returns the populated [`SequenceRunSummary`] and a flag indicating whether
/// an interrupt was observed during iteration (so the orchestrator can emit
/// the standard `130` exit code).
#[allow(deprecated)]
#[allow(clippy::too_many_arguments)]
pub(super) fn run_sequence_steps(
    plan: &SequencePlan,
    step_contexts: &[StepContext],
    resolved_targets: &[Option<claudine::composition::ResolvedExecutionTarget>],
    source: &ResolvedCompositionSource,
    prep_context: &CompositionPrepContext,
    shared: &SharedComposeArgs,
    shared_approval_cache: &composition::SharedApprovalCache,
    interrupted: &Arc<AtomicBool>,
    inline_mode: bool,
    effective_fail_fast: bool,
    silent: bool,
    verbose: u8,
    perf_enabled: bool,
    perf_accumulator: &mut Option<crate::perf::SequencePerfAccumulator>,
) -> Result<(SequenceRunSummary, bool)> {
    let total_steps = plan.steps.len();
    let mut summary = SequenceRunSummary {
        total_steps,
        succeeded: 0,
        failed: 0,
        steps: Vec::with_capacity(total_steps),
    };

    // One cell for the whole sequence: `outputs` accumulates across steps and a
    // `set` from step 1 is still visible in step 5.
    let runtime_state = std::rc::Rc::new(claudine::composition::RuntimeState::new());

    let mut interrupt_observed = false;
    for (step_index, step_ctx) in step_contexts.iter().enumerate() {
        if interrupted.load(Ordering::SeqCst) {
            interrupt_observed = true;
            if let Some(acc) = perf_accumulator.as_mut() {
                acc.set_partial();
            }
            break;
        }
        let step = &plan.steps[step_index];

        // Dry-run: separate each document's rendered metadata block with a
        // full-width horizontal rule on stderr. The per-document render
        // (frontmatter + table → stderr, body → stdout) happens inside
        // `execute_composition_request_inner`; the rule sits *between*
        // documents, so it precedes docs 2..M. `--quiet` / `--silent` have
        // no effect on dry-run output, so this emits unconditionally.
        if shared.dry_run && step_index > 0 {
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

        if !silent {
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
        let prepared = step_ctx.prepared.clone();
        // `resolved_targets[step_index]` is itself an `Option` (a `--dry-run`
        // step with an unresolved agent state has no target); flatten so the
        // dry-run seam in `execute_composition_request_inner` sees `None` and
        // renders the classified state.
        let resolved_target = resolved_targets.get(step_index).cloned().flatten();
        // Captured before `resolved_target` moves into the request: a dry-run
        // step with no target is an unresolved agent state whose provider is a
        // placeholder, so the success status must not attribute a provider.
        let target_unresolved = resolved_target.is_none();

        let system_prompt_args = SystemPromptArgs {
            append_file: shared.append_system_prompt.clone(),
            replace_file: shared.replace_system_prompt.clone(),
        };

        let request = {
            let resolved = shared.resolve_session_interactivity(prepared.selection_hints.interactive);
            CompositionExecutionRequest {
                mode: if inline_mode {
                    CompositionMode::InlineFrontmatterPrompt
                } else {
                    CompositionMode::ChainedDocument
                },
                file_ref: source.original_ref.clone(),
                prepared,
                resolved_target,
                explicit_provider: shared.explicit_provider(),
                excluded: shared.excluded(),
                sequence: true,
                yolo: shared.yolo,
                include: shared.include.clone(),
                model: shared.model.clone(),
                output: shared.output,
                system_prompt_args,
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
                env_overrides: step_ctx.env_overrides.clone(),
                shared_approval_cache: Some(Arc::clone(shared_approval_cache)),
                installed_snapshot: Some(prep_context.installed_snapshot.clone()),
                prep_launch_workspace: Some(prep_context.launch_workspace.clone()),
                prep_launch_context: Some(prep_context.launch_context.clone()),
                prep_env_context: Some(prep_context.env_context.clone()),
                prep_launch_detection_error: prep_context.launch_detection_error.clone(),
                // Sequence steps render their header in-pipeline: per-step
                // prep already ran up front, so the executor's emit is timely.
                header_emitted: false,
                // The same forwarded tail applies to every sequence step,
                // regardless of the provider each step resolves to.
                provider_args: shared.provider_args.clone(),
                provider_args_explicit: shared.provider_args_explicit,
                runtime_state: Some(std::rc::Rc::clone(&runtime_state)),
            }
        };

        let step_result = execute_composition_request_inner(
            request,
            verbose,
            None,
            perf_enabled,
        );

        let duration = start.elapsed();

        match step_result {
            Ok(outcome) if outcome.exit_code == 0 => {
                if let Some(acc) = perf_accumulator.as_mut() {
                    acc.add_step(crate::perf::SequenceStepPerf {
                        step_index,
                        step_name: step.name.clone(),
                        wall_clock: duration,
                        compose_perf: step_ctx.prepared.compose_perf.clone(),
                        agent_perf: outcome.agent_perf,
                    });
                }
                summary.succeeded += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: true,
                    error: None,
                    duration,
                });
                debug!(step_index = step_index + 1, step_name = %step.name, provider = %outcome.provider, exit_code = 0, "sequence step succeeded");
                if !silent {
                    // A `--dry-run` step with an unresolved agent state has no
                    // real provider (`outcome.provider` is a placeholder), so
                    // drop the misleading "via {provider}" attribution; the
                    // rendered dry-run table already reports the agent state.
                    let prose = if shared.dry_run && target_unresolved {
                        format!(
                            "step <b><yellow>{}/{}</yellow></b> rendered (<dim><i>dry-run</i></dim>)",
                            step_index + 1,
                            total_steps,
                        )
                    } else {
                        format!(
                            "step <b><yellow>{}/{}</yellow></b> succeeded (<dim><i>via {}</i></dim>)",
                            step_index + 1,
                            total_steps,
                            outcome.provider
                        )
                    };
                    let status = Status::from_prose(prose).state(StatusState::Success);
                    log::message(&status.render(&log::terminal()));
                }
            }
            Ok(outcome)
                if outcome.exit_code == SEQUENCE_INTERRUPT_EXIT_CODE
                    || interrupted.load(Ordering::SeqCst) =>
            {
                if let Some(acc) = perf_accumulator.as_mut() {
                    acc.add_step(crate::perf::SequenceStepPerf {
                        step_index,
                        step_name: step.name.clone(),
                        wall_clock: duration,
                        compose_perf: step_ctx.prepared.compose_perf.clone(),
                        agent_perf: outcome.agent_perf,
                    });
                    acc.set_partial();
                }
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some("interrupted by SIGINT".into()),
                    duration,
                });
                debug!(step_index = step_index + 1, step_name = %step.name, "sequence step interrupted");
                if !silent {
                    let status = Status::from_prose(format!(
                        "step <b><yellow>{}/{}</yellow></b> interrupted by Ctrl+C",
                        step_index + 1,
                        total_steps,
                    ))
                    .state(StatusState::Failure);
                    log::message(&status.render(&log::terminal()));
                }
                interrupt_observed = true;
                break;
            }
            Ok(outcome) => {
                if let Some(acc) = perf_accumulator.as_mut() {
                    acc.add_step(crate::perf::SequenceStepPerf {
                        step_index,
                        step_name: step.name.clone(),
                        wall_clock: duration,
                        compose_perf: step_ctx.prepared.compose_perf.clone(),
                        agent_perf: outcome.agent_perf,
                    });
                }
                let error_msg = format!(
                    "provider {} exited with code {}",
                    outcome.provider, outcome.exit_code
                );
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration,
                });
                debug!(step_index = step_index + 1, step_name = %step.name, provider = %outcome.provider, exit_code = outcome.exit_code, error = %error_msg, "sequence step failed");
                if !silent {
                    let status = Status::from_prose(format!(
                        "step <b><yellow>{}/{}</yellow></b> failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ))
                    .state(StatusState::Failure);
                    log::message(&status.render(&log::terminal()));
                }
                if effective_fail_fast {
                    if let Some(acc) = perf_accumulator.as_mut() {
                        acc.set_partial();
                    }
                    debug!(step_index = step_index + 1, step_name = %step.name, fail_fast = %effective_fail_fast, "sequence fail-fast triggered");
                    break;
                }
            }
            Err(e) => {
                let error_msg = e.to_string();
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration,
                });
                debug!(step_index = step_index + 1, step_name = %step.name, error = %error_msg, "sequence step error");
                if !silent {
                    let status = Status::from_prose(format!(
                        "step <b><yellow>{}/{}</yellow></b> failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ))
                    .state(StatusState::Failure);
                    log::message(&status.render(&log::terminal()));
                }
                if effective_fail_fast {
                    if let Some(acc) = perf_accumulator.as_mut() {
                        acc.set_partial();
                    }
                    debug!(step_index = step_index + 1, step_name = %step.name, fail_fast = %effective_fail_fast, "sequence fail-fast triggered");
                    break;
                }
            }
        }
    }

    Ok((summary, interrupt_observed))
}
