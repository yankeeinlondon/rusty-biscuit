//! Serial sequence orchestrator.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::sequence::build_step_overlay;
use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, PrepareOptions, PreparedComposition,
    ResolvedCompositionSource, SequenceExecutionOptions, SequencePlan, SequenceRunSummary,
    SequenceStepResult,
};
use claudine::harness::{HarnessResolutionContext, has_harness_properties, parse_harness_plan};
use color_eyre::eyre::{Result, eyre};

use crate::commands::compose::SharedComposeArgs;
use crate::log;

/// Exit code emitted when Ctrl+C is observed during a sequence run.
/// Matches the standard `128 + SIGINT(2)` convention used by shells.
const SEQUENCE_INTERRUPT_EXIT_CODE: i32 = 130;

/// Context data for a single step in the sequence.
///
/// Captures the per-step environment overrides and the prepared composition
/// so that Phase 2 execution can reuse the work done during Phase 1 pre-flight.
struct StepContext {
    env_overrides: BTreeMap<String, String>,
    prepared: PreparedComposition,
}

/// Execute a full sequence: iterate steps, compose each, and report results.
#[allow(deprecated)]
pub(crate) fn execute_sequence(
    source: &ResolvedCompositionSource,
    plan: SequencePlan,
    shared: &SharedComposeArgs,
    user_set_overrides: Option<serde_json::Value>,
    execution_options: SequenceExecutionOptions,
    verbose: u8,
) -> Result<i32> {
    let silent = shared.silent;

    let effective_fail_fast = execution_options
        .fail_fast_override
        .unwrap_or(plan.document_fail_fast);

    // `--step-timeout` applies to every step in the sequence. Parse once so
    // the CLI flag error grammar is raised at sequence entry, not inside the
    // hot per-step loop.
    let cli_step_timeout_secs = shared.step_timeout_secs()?;

    let total_steps = plan.steps.len();

    if !silent {
        let term = log::terminal();
        let status = Status::from_prose(format!(
            "<b>Sequence:</b> <yellow>{}</yellow> step(s), <i>fail_fast</i> is set to <blue>{}</blue>",
            total_steps, effective_fail_fast
        ))
        .state(StatusState::Info);
        log::message(&status.render(&term));
    }

    let mut summary = SequenceRunSummary {
        total_steps,
        succeeded: 0,
        failed: 0,
        steps: Vec::with_capacity(total_steps),
    };

    // Persistent SIGINT tracker for the duration of the sequence run.
    //
    // The provider children installed by `wait_with_signal_handling` have
    // their own SIGINT handler scoped to each invocation; once a child
    // exits, that handler no longer updates any flag the sequence can
    // observe. We also need to catch Ctrl+C that arrives outside a child
    // (Phase 1 prep, between steps, or while finalizing summaries). This
    // flag flips on the first SIGINT and is checked before each step and
    // after each step completes so the loop aborts promptly.
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_handler = interrupted.clone();
    let _sigint_guard = {
        #[cfg(unix)]
        {
            unsafe {
                signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
                    interrupted_handler.store(true, Ordering::SeqCst);
                })
            }
            .ok()
        }
        #[cfg(not(unix))]
        {
            // No-op on non-Unix; sequence is Unix-only in practice.
            let _ = interrupted_handler;
            Option::<()>::None
        }
    };

    let mut cumulative_approved: HashSet<String> = HashSet::new();

    // Shared approval cache lives for the whole sequence run so that
    // "allow once" decisions from earlier steps survive into later
    // steps for both template `::shell` directives and harness shell
    // commands.
    let shared_approval_cache: composition::SharedApprovalCache =
        Arc::new(Mutex::new(HashMap::new()));

    if !silent {
        let term = log::terminal();
        let status = Status::from_prose("Starting pre-flight checks".to_string())
            .state(StatusState::Info)
            .theme(biscuit_terminal::components::status::StatusTheme::Circular);
        log::message(&status.render(&term));
    }

    // ── Phase 1: run pre-flight shell discovery for every step ─────────
    //
    // Template `::shell` directives can reference per-step state
    // (`{{state.name}}`, `{{previous_state.foo}}`, etc.) so each step's
    // discovery pass needs its own compose context. Running the whole
    // pass up-front means any required approval prompts fire BEFORE
    // the first agent launches — the operator reviews all shell commands
    // (both template and harness) once and then walks away.
    let mut step_contexts: Vec<StepContext> = Vec::with_capacity(total_steps);
    for step_index in 0..total_steps {
        if interrupted.load(Ordering::SeqCst) {
            return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
        }
        let overlay = build_step_overlay(&plan, step_index);
        let step_set_overrides = overlay.as_set_overrides(user_set_overrides.clone());

        let mut env_overrides: BTreeMap<String, String> = BTreeMap::new();
        env_overrides.insert("FAIL_FAST".to_string(), effective_fail_fast.to_string());

        // The compose context used for ::shell discovery must see the
        // same `FAIL_FAST` value the child process will see, otherwise
        // the template interpolation used for pre-flight may diverge
        // from runtime.
        let compose_options = {
            let mut ctx = darkmatter::markdown::compose::ComposeContext::capture();
            for (key, value) in &env_overrides {
                ctx.env_mut().insert(key.clone(), value.clone());
            }
            let mut opts = darkmatter::markdown::compose::ComposeOptions::new_with_context(ctx)
                .with_source_file(&source.resolved_path);
            opts = opts.with_set_overrides(step_set_overrides.clone());
            opts
        };

        let approval_options = super::build_harness_shell_options_with_cache(
            &source.resolved_path,
            None,
            Some(Arc::clone(&shared_approval_cache)),
        );

        // ── Template pre-flight ───────────────────────────────────────
        let template_preflight = composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )
        .map_err(|e| eyre!("{e}"))?;

        cumulative_approved.extend(template_preflight.approved_commands.iter().cloned());

        // ── Prepare composition ───────────────────────────────────────
        let prepare_options = PrepareOptions {
            set_overrides: Some(step_set_overrides.clone()),
            pre_approved_commands: Some(cumulative_approved.clone()),
            env_overrides: env_overrides.clone(),
        };

        let prepared = composition::prepare_direct(source, prepare_options)
            .map_err(crate::output::shell_expansion_error::pretty_or_report)?;

        // ── Harness pre-flight ────────────────────────────────────────
        if has_harness_properties(&prepared.effective_frontmatter) {
            let effective_repo_root = prepared.source_repo_root.as_deref();
            let resolve_ctx = HarnessResolutionContext {
                source_path: &prepared.resolved_path,
                repo_root: effective_repo_root,
            };
            let plan = parse_harness_plan(
                &prepared.effective_frontmatter,
                &prepared.resolved_path,
                &resolve_ctx,
            )
            .map_err(|e| eyre!("{e}"))?;

            let harness_preflight =
                composition::resolve_shell_approvals(None, None, Some(&plan), &approval_options)
                    .map_err(|e| eyre!("{e}"))?;

            cumulative_approved.extend(harness_preflight.approved_commands.iter().cloned());
        }

        step_contexts.push(StepContext {
            env_overrides,
            prepared,
        });
    }

    if !silent {
        let status = Status::from_prose(format!(
            "<b>Preflight:</b> shell commands approved for all \
             <yellow>{}</yellow> step(s) in the sequence",
            total_steps
        ))
        .state(StatusState::Info);
        log::message(&status.render(&log::terminal()));
    }

    // ── Phase 2: execute each step ─────────────────────────────────────
    let mut interrupt_observed = false;
    for (step_index, step_ctx) in step_contexts.iter().enumerate() {
        if interrupted.load(Ordering::SeqCst) {
            interrupt_observed = true;
            break;
        }
        let step = &plan.steps[step_index];

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

        // Use the prepared composition from Phase 1 instead of re-preparing
        let prepared = step_ctx.prepared.clone();

        let system_prompt_args = claudine::system_prompt::SystemPromptArgs {
            append_file: shared.append_system_prompt.clone(),
            replace_file: shared.replace_system_prompt.clone(),
        };

        let request = CompositionExecutionRequest {
            mode: CompositionMode::ChainedDocument,
            file_ref: source.original_ref.clone(),
            prepared,
            explicit_provider: shared.explicit_provider(),
            excluded: shared.excluded(),
            sequence: true,
            yolo: shared.yolo,
            include: shared.include.clone(),
            model: shared.model.clone(),
            output: shared.output,
            system_prompt_args,
            timeout: shared.timeout,
            step_timeout: cli_step_timeout_secs,
            operation: shared.operation.clone(),
            sandbox: shared.sandbox,
            repo: shared.repo,
            dry_run: shared.dry_run,
            mcp: shared.mcp,
            mcp_use: shared.mcp_use.clone(),
            strict: shared.strict,
            session_interactive: shared.interactive,
            quiet: shared.quiet,
            silent: shared.silent,
            env_overrides: step_ctx.env_overrides.clone(),
            shared_approval_cache: Some(Arc::clone(&shared_approval_cache)),
        };

        let step_result = super::composition::execute_composition_request_inner(request, verbose);

        let duration = start.elapsed();

        match step_result {
            Ok(outcome) if outcome.exit_code == 0 => {
                summary.succeeded += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: true,
                    error: None,
                    duration,
                });
                if !silent {
                    let status = Status::from_prose(format!(
                        "step <b><yellow>{}/{}</yellow></b> succeeded (<dim><i>via {}</i></dim>)",
                        step_index + 1,
                        total_steps,
                        outcome.provider
                    ))
                    .state(StatusState::Success);
                    log::message(&status.render(&log::terminal()));
                }
            }
            Ok(outcome)
                if outcome.exit_code == SEQUENCE_INTERRUPT_EXIT_CODE
                    || interrupted.load(Ordering::SeqCst) =>
            {
                // Ctrl+C was observed either by the in-child signal handler
                // (exit 130) or by our persistent process-level handler.
                // Record the failure, stop the loop unconditionally, and
                // propagate the interrupt exit code to the caller regardless
                // of `fail_fast`.
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some("interrupted by SIGINT".into()),
                    duration,
                });
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
                    break;
                }
            }
        }
    }

    // Final summary
    if !silent {
        eprintln!();
        if summary.failed == 0 {
            let status = Status::from_prose(format!(
                "Sequence finished: <green>{}</green> succeeded, 0 failed",
                summary.succeeded
            ))
            .state(StatusState::Success);
            log::message(&status.render(&log::terminal()));
        } else {
            let status = Status::from_prose(format!(
                "Sequence finished: <green>{}</green> succeeded, <red>{}</red> failed",
                summary.succeeded, summary.failed
            ))
            .state(StatusState::Failure);
            log::message(&status.render(&log::terminal()));
        }
    }

    if interrupt_observed || interrupted.load(Ordering::SeqCst) {
        return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
    }
    if summary.failed > 0 { Ok(1) } else { Ok(0) }
}
