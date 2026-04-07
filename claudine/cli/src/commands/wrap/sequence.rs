//! Serial sequence orchestrator.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, Mutex};

use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::sequence::build_step_overlay;
use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, PrepareOptions, ResolvedCompositionSource,
    SequenceExecutionOptions, SequencePlan, SequenceRunSummary, SequenceStepResult,
};
use color_eyre::eyre::{Result, eyre};

use crate::commands::compose::SharedComposeArgs;
use crate::log;

/// Execute a full sequence: iterate steps, compose each, and report results.
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

    let mut cumulative_approved: HashSet<String> = HashSet::new();

    // Shared approval cache lives for the whole sequence run so that
    // "allow once" decisions from earlier steps survive into later
    // steps for both template `::shell` directives and harness shell
    // commands.
    let shared_approval_cache: composition::SharedApprovalCache =
        Arc::new(Mutex::new(HashMap::new()));

    for step_index in 0..total_steps {
        let step = &plan.steps[step_index];
        let overlay = build_step_overlay(&plan, step_index);

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

        let step_set_overrides = overlay.as_set_overrides(user_set_overrides.clone());

        let mut env_overrides: BTreeMap<String, String> = BTreeMap::new();
        env_overrides.insert("FAIL_FAST".to_string(), effective_fail_fast.to_string());

        // Pre-flight shell approval for this step.
        //
        // The compose context used for ::shell discovery must see the same
        // `FAIL_FAST` value the child process will see, otherwise the
        // template interpolation used for pre-flight may diverge from
        // runtime. Build a context explicitly and inject env overrides.
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
            shared.interactive,
            Some(Arc::clone(&shared_approval_cache)),
        );

        let preflight = composition::resolve_shell_approvals(
            Some(&source.markdown),
            Some(&compose_options),
            None,
            &approval_options,
        )
        .map_err(|e| eyre!("{e}"))?;

        cumulative_approved.extend(preflight.approved_commands.iter().cloned());

        let prepare_options = PrepareOptions {
            set_overrides: Some(step_set_overrides),
            pre_approved_commands: Some(cumulative_approved.clone()),
            env_overrides: env_overrides.clone(),
        };

        let prepared = match composition::prepare_direct(source, prepare_options) {
            Ok(p) => p,
            Err(e) => {
                let duration = start.elapsed();
                let error_msg = e.to_string();
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
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg),
                    duration,
                });
                if effective_fail_fast {
                    break;
                }
                continue;
            }
        };

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
            env_overrides: env_overrides.clone(),
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

    if summary.failed > 0 { Ok(1) } else { Ok(0) }
}
