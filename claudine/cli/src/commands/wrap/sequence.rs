//! Serial sequence orchestrator.

use std::collections::HashSet;

use claudine::composition::{
    self, CompositionExecutionRequest, CompositionMode, PrepareOptions,
    ResolvedCompositionSource, SequenceExecutionOptions, SequencePlan, SequenceRunSummary,
    SequenceStepResult, SystemPromptInput,
};
use claudine::composition::sequence::build_step_overlay;
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
        log::info(&format!(
            "Sequence: {} step(s), fail_fast={}",
            total_steps, effective_fail_fast
        ));
    }

    let mut summary = SequenceRunSummary {
        total_steps,
        succeeded: 0,
        failed: 0,
        steps: Vec::with_capacity(total_steps),
    };

    let mut cumulative_approved: HashSet<String> = HashSet::new();

    for step_index in 0..total_steps {
        let step = &plan.steps[step_index];
        let overlay = build_step_overlay(&plan, step_index);

        if !silent {
            log::info(&format!(
                "[{}/{}] {}",
                step_index + 1,
                total_steps,
                step.name
            ));
        }

        let start = std::time::Instant::now();

        let step_set_overrides = overlay.as_set_overrides(user_set_overrides.clone());

        let mut env_overrides = std::collections::BTreeMap::new();
        env_overrides.insert("FAIL_FAST".to_string(), effective_fail_fast.to_string());

        // Pre-flight shell approval for this step
        let compose_options = {
            let mut opts = darkmatter::markdown::compose::ComposeOptions::new()
                .with_source_file(&source.resolved_path);
            opts = opts.with_set_overrides(step_set_overrides.clone());
            opts
        };

        let approval_options = super::build_harness_shell_options(
            &source.resolved_path,
            None,
            shared.interactive,
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
            env_overrides,
        };

        let prepared = match composition::prepare_direct(source, prepare_options) {
            Ok(p) => p,
            Err(e) => {
                let duration = start.elapsed();
                let error_msg = e.to_string();
                if !silent {
                    log::error(&format!(
                        "step {}/{} failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ));
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

        let system_prompt = shared
            .system_prompt
            .as_ref()
            .map(|prompt| SystemPromptInput::Inline {
                prompt: prompt.clone(),
            })
            .or_else(|| {
                shared
                    .system_prompt_file
                    .as_ref()
                    .map(|path| SystemPromptInput::File { path: path.clone() })
            });

        let request = CompositionExecutionRequest {
            mode: CompositionMode::ChainedDocument,
            file_ref: source.original_ref.clone(),
            prepared,
            explicit_provider: shared.explicit_provider(),
            excluded: shared.excluded(),
            yolo: shared.yolo,
            include: shared.include.clone(),
            model: shared.model.clone(),
            output: shared.output,
            system_prompt,
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
        };

        let step_result = super::composition::execute_composition_request(request, verbose);

        let duration = start.elapsed();

        match step_result {
            Ok(exit_code) if exit_code == 0 => {
                summary.succeeded += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: true,
                    error: None,
                    duration,
                });
                if !silent {
                    log::info(&format!(
                        "step {}/{} succeeded",
                        step_index + 1,
                        total_steps
                    ));
                }
            }
            Ok(exit_code) => {
                let error_msg = format!("provider exited with code {exit_code}");
                summary.failed += 1;
                summary.steps.push(SequenceStepResult {
                    step: step_index + 1,
                    name: step.name.clone(),
                    success: false,
                    error: Some(error_msg.clone()),
                    duration,
                });
                if !silent {
                    log::error(&format!(
                        "step {}/{} failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ));
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
                    log::error(&format!(
                        "step {}/{} failed: {}",
                        step_index + 1,
                        total_steps,
                        error_msg
                    ));
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
            log::info(&format!(
                "Sequence finished: {} succeeded, 0 failed",
                summary.succeeded
            ));
        } else {
            log::error(&format!(
                "Sequence finished: {} succeeded, {} failed",
                summary.succeeded, summary.failed
            ));
        }
    }

    if summary.failed > 0 { Ok(1) } else { Ok(0) }
}
