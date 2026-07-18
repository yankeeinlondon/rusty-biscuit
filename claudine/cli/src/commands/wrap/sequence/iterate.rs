//! Per-step execution loop (Phase 2) for the sequence orchestrator.
//!
//! Iterates the prepared step contexts produced by Phase 1c, dispatches each
//! step through the wrapper-grade composition executor, and builds the
//! [`SequenceRunSummary`] consumed by the final report.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::horizontal_rule::{
    HorizontalRule, RuleAlignment, RuleStyle, RuleWeight,
};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, ProxyHandoff, ResolvedCompositionSource,
    RunLedger, SequencePlan, SequenceRunSummary, SequenceStepResult, SharedRunLedger,
    SurfacedHandoff, commit_proxy,
};
use claudine::system_prompt::SystemPromptArgs;
use color_eyre::eyre::Result;
use tracing::{debug, info_span};

use crate::commands::compose::CompositionKind;
use crate::commands::compose::SharedComposeArgs;
use crate::commands::compose::prep::{
    ActiveDocumentOutcome, prepare_and_run_active_document, resolve_composition_source,
};
use crate::commands::wrap::composition::{execute_composition_request_inner, CompositionPrepContext};
use crate::commands::wrap::overlay::merge_frontmatter_overlay;
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
    user_set_overrides: &Option<serde_json::Value>,
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

    // A sequence step's proxy target is re-prepared through the same canonical
    // pipeline the top-level `compose`/`inline-compose` commands use, so the
    // step's inline-vs-chained mode maps onto the shared `CompositionKind`. The
    // launch-area anchor lets that pipeline restore the launch CWD before
    // preparing the target (R5), exactly as the compose coordinator does.
    let step_kind = if inline_mode {
        CompositionKind::Inline
    } else {
        CompositionKind::Direct
    };
    let launch_area_fallback: Option<PathBuf> =
        Some(prep_context.launch_workspace.launch_cwd.clone());

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

        // Each step owns its own coordinator scope (R1). Hop/cycle accounting is
        // per-step: a fresh ledger, seeded with the sequence document so a
        // self-proxy is a first-hop cycle, means one step's proxy chain never
        // shares a hop counter with another step's. The invocation-wide approval
        // cache is still shared across every step (R9) — only hop/cycle isolation
        // is per-step.
        let step_ledger: SharedRunLedger = Arc::new(Mutex::new(RunLedger::new(
            source.resolved_path.clone(),
            Arc::clone(shared_approval_cache),
        )));

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
                system_prompt_args: system_prompt_args.clone(),
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
                // A sequence step's proxy stays contained within the step (R1),
                // but it is surfaced to the step's own command-owned coordinator
                // (via `handoff_ledger`) rather than adopted in-harness, so the
                // target is re-prepared through canonical preparation (R6/R7).
                // The caller's document is the first hop; `with:` overlays arrive
                // per-handoff and are applied by the contained loop below.
                proxy_overlay: indexmap::IndexMap::new(),
                handoff_ledger: Some(Arc::clone(&step_ledger)),
                // The step's own document is invoked directly, not through a
                // proxy; a surfaced handoff is adopted by the target below.
                adopted_handoff: None,
            }
        };

        let step_result = execute_composition_request_inner(
            request,
            verbose,
            None,
            perf_enabled,
        );

        // Resolve any surfaced proxy INSIDE this step (R1). The step remains in
        // its current iteration until the target completes, fails, or proxies
        // again; the sequence neither advances nor restarts. The target is
        // committed against the per-step ledger and re-prepared through the same
        // canonical launch pipeline a direct invocation uses (R6/R7), so it
        // acquires its own launch bundle and document loop rather than being
        // adopted as a reduced in-harness attempt. The contained loop's final
        // exit code becomes the step's exit code and flows into the existing
        // success/fail/fail-fast handling unchanged. `--dry-run` never surfaces a
        // handoff, so a dry-run step skips the loop and behaves exactly as before.
        let step_result = step_result.and_then(|mut outcome| {
            if let Some(surfaced) = outcome.initialize_handoff.take() {
                outcome.exit_code = run_step_proxy_loop(
                    surfaced,
                    &step_ledger,
                    prep_context.source_repo_root.as_deref(),
                    shared,
                    step_kind,
                    &system_prompt_args,
                    user_set_overrides.clone(),
                    &launch_area_fallback,
                    shared_approval_cache,
                    verbose,
                )?;
            }
            Ok(outcome)
        });

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

/// Drive a sequence step's contained handoff chain to completion (R1).
///
/// A proxy surfaced by the step's harness (via the per-step ledger) is committed
/// here and its target is re-prepared through the shared canonical pipeline
/// [`prepare_and_run_active_document`] — the same one the top-level
/// `compose`/`inline-compose` coordinator uses — so the target rebuilds its full
/// launch bundle and acquires its own document loop (R6/R7). The loop stays
/// inside the current sequence step: it never advances or restarts the sequence,
/// and every commit is scoped to the step's own `ledger`, so hop/cycle accounting
/// is per-step. Returns the final target's exit code, which becomes the step's
/// exit code.
///
/// The first commit resolves `@repo/…` targets against the step document's
/// repository root; each subsequent hop uses the freshly-prepared target's own
/// root (returned alongside its outcome), matching the compose coordinator.
#[allow(clippy::too_many_arguments)]
fn run_step_proxy_loop(
    initial: SurfacedHandoff,
    ledger: &SharedRunLedger,
    step_repo_root: Option<&Path>,
    shared: &SharedComposeArgs,
    kind: CompositionKind,
    system_prompt_args: &SystemPromptArgs,
    user_set_overrides: Option<serde_json::Value>,
    launch_area_fallback: &Option<PathBuf>,
    shared_approval_cache: &composition::SharedApprovalCache,
    verbose: u8,
) -> Result<i32> {
    let mut surfaced = initial;
    let mut commit_repo_root: Option<PathBuf> = step_repo_root.map(Path::to_path_buf);
    loop {
        // An `initialize`-route proxy arrives as a request awaiting commit by
        // this step's ledger; a terminal-route proxy was already committed by
        // the harness against the same ledger and is used directly.
        let handoff: ProxyHandoff = match surfaced {
            SurfacedHandoff::Request(request) => {
                commit_proxy(&mut ledger.lock().unwrap(), request, commit_repo_root.as_deref())
                    .map_err(color_eyre::eyre::Report::from)?
            }
            SurfacedHandoff::Committed(handoff) => *handoff,
        };
        let target_ref = handoff.resolved_target().to_string_lossy().into_owned();
        let mut target_source = resolve_composition_source(&target_ref, kind, shared)?;
        // Precedence, low→high: target-authored frontmatter < the proxy's `with:`
        // overlay < caller `--set`. The overlay lands in the authored map before
        // composition; the caller's overrides ride on the compose options on top.
        merge_frontmatter_overlay(
            target_source.markdown.frontmatter_mut().as_map_mut(),
            handoff.overlay(),
        );
        let overlay = handoff.overlay().clone();
        let target_file = handoff.authored_target().to_string();
        // A proxied target is never the caller's document, so it never re-runs the
        // inline deferred body report — pass an empty inline state and `first=false`.
        let mut inline_state = None;
        // The step's target adopts the committed handoff for its staged bootstrap
        // (R4), so its own `initialize`/reread/audit run rather than the setup
        // pipeline routing `initialize` a second time.
        let adopted_handoff = Some(Box::new(handoff));

        let (outcome, next_repo_root) = prepare_and_run_active_document(
            shared,
            kind,
            target_source,
            &target_file,
            &overlay,
            adopted_handoff,
            false,
            &mut inline_state,
            system_prompt_args,
            user_set_overrides.clone(),
            launch_area_fallback,
            shared_approval_cache,
            ledger,
            verbose,
            None,
            Vec::new(),
            std::time::Instant::now(),
        )?;

        match outcome {
            ActiveDocumentOutcome::Done(code) => return Ok(code),
            ActiveDocumentOutcome::Handoff(next) => {
                surfaced = next;
                commit_repo_root = next_repo_root;
            }
        }
    }
}
