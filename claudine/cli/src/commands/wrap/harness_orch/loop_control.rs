use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::LifecycleSignal;
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use tracing::info_span;

use super::super::composition::IterationSummarySignals;
use super::{
    CachedHarnessLoopContext, HarnessPromptState, MaterializedHarnessPrompt, build_harness_launch,
    execute_harness_attempt, harness_prompt_mode_label, materialize_harness_prompt, HarnessPromptMode,
};

#[allow(clippy::too_many_arguments)]
#[allow(unused_assignments)]
pub(crate) fn run_harness_loop(
    provider: Provider,
    profile: &dyn super::super::profile::WrapperProfile,
    binary_path: &Path,
    child_cwd: &Path,
    effective_non_interactive: bool,
    cli_timeout: Option<String>,
    cli_step_timeout: Option<String>,
    base_args: &[String],
    base_env: &HashMap<OsString, OsString>,
    prompt_state: &mut HarnessPromptState,
    repo_root: Option<&Path>,
    shell_options: claudine::harness::ShellApprovalOptions,
    use_structured: bool,
    structured_codex_output: Option<&super::super::policy::StructuredCodexOutput>,
    stdout_noise: &[&str],
    stderr_noise: &[&str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    detail_requested: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    term: &Terminal,
    lifecycle: &claudine::composition::LifecycleConfig,
    lifecycle_ctx: &claudine::composition::LifecycleRuntimeContext<'_>,
    lifecycle_emitter: &dyn claudine::composition::LifecycleEmitter,
    // When `true`, every structured-stream attempt in the harness loop
    // emits the prompt-scoped timing header and — if the parsed plan
    // carries `timeout_warn` / `step_timeout_warn` — their fire-once
    // warning lines. Wrapper passthrough callers with no prompt file
    // pass `false` to suppress the header entirely; composition callers
    // pass `true`.
    emit_prompt_timing: bool,
) -> Result<(i32, Option<crate::perf::AgentExecutionPerf>, Option<IterationSummarySignals>)> {
    const DEFAULT_MAX_RETRIES: u32 = 3;
    let mut guard =
        claudine::composition::LifecycleRunGuard::new(lifecycle, lifecycle_ctx, lifecycle_emitter);
    let permission_probe = super::super::policy::WrapperHarnessPermissionProbe::new(
        provider,
        base_args.to_vec(),
        repo_root,
    );
    let mut harness_context = CachedHarnessLoopContext::with_shell_options(
        &prompt_state.source_path,
        repo_root,
        shell_options,
    );
    let mut attempt = 1u32;
    let mut initial_materialized = initial_materialized;
    let mut harness_perf: Option<crate::perf::AgentExecutionPerf> = None;
    let mut terminal_signals: Option<IterationSummarySignals> = None;
    let mut _harness_attempts: usize = 0;

    loop {
        let _attempt_cycle_span = info_span!(
            "harness_attempt_cycle",
            provider = %provider,
            attempt,
            prompt_mode = harness_prompt_mode_label(prompt_state.mode),
            source_path = %prompt_state.source_path.display(),
        )
        .entered();
        harness_context.refresh(&prompt_state.source_path, repo_root);
        let materialized = if let Some(seed) = initial_materialized.take() {
            seed
        } else {
            info_span!(
                "harness_materialize_prompt",
                attempt,
                source_path = %prompt_state.source_path.display(),
            )
            .in_scope(|| materialize_harness_prompt(prompt_state, repo_root, child_cwd))
            .map_err(|e| guard.emit_blocked_or_err(e))?
        };
        let resolve_ctx = harness_context.resolve_context();
        let plan = info_span!(
            "harness_plan_parse",
            attempt,
            source_path = %prompt_state.source_path.display(),
        )
        .in_scope(|| {
            claudine::harness::parse_harness_plan(
                &materialized.frontmatter,
                &prompt_state.source_path,
                &resolve_ctx,
            )
        })
        .map_err(|e| guard.emit_blocked_or_err(e))?;

        // Source-file existence reporting
        if show_checks {
            claudine::harness::report::report_source_file(
                &prompt_state.original_ref,
                &prompt_state.source_path,
                term,
            );
        }
        if !prompt_state.source_path.exists() {
            if show_checks {
                claudine::harness::report::report_unhandled_failure(
                    "source file does not exist — cannot proceed",
                    term,
                );
            }
            guard.emit_blocked_or_failure();
            return Err(eyre!(
                "source file does not exist: {}",
                prompt_state.source_path.display()
            ));
        }

        // Finalize the parsed plan into the effective plan. For inline
        // composition this prepends a system-owned writability pre-check so
        // handler recovery paths can respond to permission failures.
        let plan = claudine::harness::finalize_effective_plan(
            plan,
            if matches!(prompt_state.mode, HarnessPromptMode::Inline) {
                claudine::harness::EffectivePlanMode::Inline
            } else {
                claudine::harness::EffectivePlanMode::Direct
            },
            &prompt_state.source_path,
        );

        // Shell audit preflight.
        //
        // Composition flows (Compose/Inline) preflight all shell commands
        // before the provider starts — template directives during composition
        // and harness commands in execute_composition_request.  The per-
        // attempt audit below is redundant for those modes because:
        //
        //   1. source_text is None, so source-page ::shell directives are
        //      excluded (they were discovered via Darkmatter's graph walker
        //      during composition, which respects ::block when="false").
        //   2. Harness commands were approved and cached during the
        //      composition preflight pass.
        //   3. The approval handler is frozen after attempt 1, so no new
        //      interactive prompts are possible.
        //
        // Only Passthrough mode needs the per-attempt audit because it reads
        // raw source text and the source file may change between
        // redirect/retry iterations.
        if matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            let source_text = std::fs::read_to_string(&prompt_state.source_path).ok();

            let auditable =
                claudine::harness::collect_auditable_commands(&plan, source_text.as_deref())?;

            let audit_report = info_span!(
                "harness_shell_audit",
                attempt,
                command_count = auditable.len(),
            )
            .in_scope(|| {
                claudine::harness::audit_shell_commands(&auditable, harness_context.shell_options())
            });

            if show_checks {
                claudine::harness::report::report_shell_audit_header(
                    audit_report.outcomes.len(),
                    term,
                );
                claudine::harness::report::report_shell_audit_outcomes(&audit_report, term);
            }

            if !audit_report.all_passed() {
                let failed = audit_report.failures();
                let (source_failures, harness_failures): (Vec<_>, Vec<_>) =
                    failed.into_iter().partition(|o| {
                        matches!(
                            o.command.source,
                            claudine::harness::AuditedCommandSource::ComposeSourceLine { .. }
                        )
                    });

                // Source-page ::shell failures are terminal in v1 — no recovery.
                if !source_failures.is_empty() {
                    if show_checks {
                        claudine::harness::report::report_unhandled_failure(
                            "shell audit failed for source-page directives — cannot proceed",
                            term,
                        );
                    }
                    guard.emit_blocked_or_failure();
                    return Err(eyre!(
                        "shell audit failed: {} denied directive(s) in source page",
                        source_failures.len()
                    ));
                }

                // Non-source failures flow through handler resolution.
                if !harness_failures.is_empty() {
                    let contexts = claudine::harness::build_audit_failure_context(
                        &harness_failures,
                        provider.as_slug(),
                        plan.source_path.as_path(),
                        attempt,
                    );
                    if let Some(next_plan) = super::super::resume::try_resolve_handler(
                        &contexts,
                        &plan,
                        attempt,
                        DEFAULT_MAX_RETRIES,
                        profile,
                        None,
                        &prompt_state.source_path,
                        repo_root,
                        show_checks,
                        term,
                    )? {
                        attempt = next_plan.next_attempt;
                        continue;
                    }

                    let msg = format!(
                        "shell audit failed: {} command(s) denied. \
                         No handler available to resolve.",
                        harness_failures.len()
                    );
                    if show_checks {
                        claudine::harness::report::report_unhandled_failure(&msg, term);
                    }
                    guard.emit_blocked_or_failure();
                    return Err(eyre!("shell audit failed"));
                }
            }
        }

        // Composition flows resolved all shell approvals during preflight.
        // Freeze the approval set so redirect/retry iterations cannot
        // trigger new interactive prompts — only cached/whitelisted
        // commands pass; new uncached commands are denied.  Passthrough
        // mode has no prior preflight so its handler stays active.
        if attempt == 1 && !matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
            harness_context.freeze_shell_approvals();
        }

        let pre_report = info_span!(
            "harness_pre_validation",
            attempt,
            rule_count = plan.pre_checks.len(),
        )
        .in_scope(|| claudine::harness::evaluate_pre_checks(&plan, Some(&permission_probe)));

        if show_checks {
            claudine::harness::report::report_phase_discovery(
                claudine::harness::FailurePhase::PreCheck,
                pre_report.count(),
                term,
            );
            claudine::harness::report::report_check_outcomes(&pre_report, term);
        }

        if !pre_report.all_passed() {
            let failures = pre_report.failures();
            let contexts = claudine::harness::build_validation_failure_context(
                &failures,
                provider.as_slug(),
                plan.source_path.as_path(),
                attempt,
                None,
                None,
            );
            if let Some(next_plan) = super::super::resume::try_resolve_handler(
                &contexts,
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                None,
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                super::super::resume::apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            let fail_msg = format!(
                "pre-check validation failed ({} {})",
                failures.len(),
                if failures.len() == 1 {
                    "failure"
                } else {
                    "failures"
                }
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&fail_msg, term);
            }
            guard.emit_blocked_or_failure();
            return Err(eyre!("{fail_msg}"));
        }

        // Emit start lifecycle signal before the first provider launch.
        guard.emit_start_once();

        let snapshot = info_span!(
            "harness_pre_snapshot",
            attempt,
            rule_count = plan.post_checks.len(),
        )
        .in_scope(|| claudine::harness::capture_pre_run_snapshot(&plan))
        .map_err(|e| eyre!("harness snapshot: {e}"))?;
        let launch = build_harness_launch(
            provider,
            profile,
            base_args,
            base_env,
            prompt_state,
            &materialized,
            effective_non_interactive,
            cli_timeout.clone(),
            plan.timeout,
            cli_step_timeout.clone(),
            plan.step_timeout,
        )?;
        let _launch_span = info_span!(
            "harness_launch_plan",
            attempt,
            timeout_secs = launch
                .timeout_config
                .timeout
                .map(|d| d.as_secs())
                .unwrap_or(0),
            step_timeout_secs = launch
                .timeout_config
                .step_timeout
                .map(|d| d.as_secs())
                .unwrap_or(0),
        )
        .entered();

        // Build the prompt-scoped timing context for this attempt. The
        // warn thresholds are re-read from each parsed plan so a
        // handler that redirects to a different source document picks
        // up the replacement document's warn values, not the original's.
        let prompt_timing = if emit_prompt_timing {
            Some(super::super::composition::build_prompt_timing_context(
                &prompt_state.source_path,
                repo_root,
                plan.timeout_warn,
                plan.step_timeout_warn,
            ))
        } else {
            None
        };

        let mut child_spawned = false;
        let attempt_result = execute_harness_attempt(
            attempt,
            provider,
            profile,
            binary_path,
            child_cwd,
            &launch,
            prompt_state.mode,
            prompt_state,
            &materialized,
            effective_non_interactive,
            use_structured,
            structured_codex_output,
            stdout_noise,
            stderr_noise,
            suppress_stderr_on_success,
            show_checks,
            stream_verbosity,
            detail_requested,
            env_context,
            dispatch_context,
            term,
            &mut child_spawned,
            prompt_timing,
        );

        // Mark launched as soon as spawn succeeded — before propagating
        // any post-spawn error — so the guard correctly classifies
        // subsequent failures as `Failure` rather than `Blocked`.
        if child_spawned {
            guard.mark_provider_launched();
        }
        let (outcome, perf, iteration_signals) = attempt_result?;
        if let Some(p) = perf {
            _harness_attempts += 1;
            match harness_perf.as_mut() {
                Some(acc) => {
                    acc.launches += p.launches;
                    acc.total_elapsed += p.total_elapsed;
                    if acc.first_response_latency.is_none() && p.first_response_latency.is_some() {
                        acc.first_response_latency = p.first_response_latency;
                    }
                    if let Some(api) = p.provider_api_duration {
                        acc.provider_api_duration = Some(
                            acc.provider_api_duration
                                .unwrap_or(std::time::Duration::ZERO)
                                + api,
                        );
                    }
                }
                None => {
                    harness_perf = Some(p);
                }
            }
        }

        if outcome.termination == claudine::harness::ProcessTermination::Interrupted {
            // Surface the interrupt to the user before we let the guard
            // close: without this the wrapper would silently return 130
            // and the operator has no feedback that Claudine noticed.
            eprintln!("{}", crate::output::format_user_interrupt_status());
            guard.emit_terminal(LifecycleSignal::Failure);
            terminal_signals = iteration_signals;
            return Ok((outcome.exit_code, harness_perf, terminal_signals));
        }

        if let Some(failure_event) = claudine::harness::classify_failure(&outcome) {
            let message = match failure_event {
                claudine::harness::FailureEvent::Timeout => {
                    format!("provider timed out (attempt {attempt})")
                }
                claudine::harness::FailureEvent::AgentFailure => {
                    format!(
                        "agent exited with error code {} (attempt {attempt})",
                        outcome.exit_code
                    )
                }
                _ => format!("failure on attempt {attempt}"),
            };
            let ctx = claudine::harness::build_agent_failure_context(
                provider.as_slug(),
                plan.source_path.as_path(),
                failure_event,
                message.clone(),
                attempt,
                outcome.session_id.clone(),
                Some(outcome.clone()),
                // Forward the honest per-guard label + structured detail the
                // attempt outcome already carries (content-guard trips) so
                // the programmatic handler payload exposes them without
                // parsing the human message string.
                outcome.error_kind.clone(),
                outcome.guard_context.as_ref(),
            );
            if let Some(next_plan) = super::super::resume::try_resolve_handler(
                &[ctx],
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                outcome.session_id.as_deref(),
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                super::super::resume::apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&message, term);
            }
            guard.emit_terminal(LifecycleSignal::Failure);
            // For provider-level failures, preserve the exit code at the
            // boundary rather than converting it into an `eyre` error. This
            // lets callers (e.g. `compose --loop`) inspect the terminal
            // attempt's iteration signals to build an honest
            // `LoopIterationFailed` cause.
            terminal_signals = iteration_signals;
            return Ok((outcome.exit_code, harness_perf, terminal_signals));
        }

        // For inline mode, apply closure BEFORE post-checks so that
        // file-state checks (file_changed, frontmatter comparisons, etc.)
        // observe the final rewritten document rather than the pre-closure
        // source file.
        if let Some(closure_plan) = materialized.inline_closure_plan.as_ref()
            && outcome.exit_code == 0
            && let Err(failures) = super::super::inline::try_inline_closure(
                closure_plan,
                &outcome.final_response,
                &prompt_state.source_path,
                child_cwd,
                show_checks,
                term,
            )
        {
            let contexts = claudine::harness::build_validation_failure_context(
                &failures,
                provider.as_slug(),
                plan.source_path.as_path(),
                attempt,
                outcome.session_id.clone(),
                Some(outcome.clone()),
            );
            if let Some(next_plan) = super::super::resume::try_resolve_handler(
                &contexts,
                &plan,
                attempt,
                DEFAULT_MAX_RETRIES,
                profile,
                outcome.session_id.as_deref(),
                &prompt_state.source_path,
                repo_root,
                show_checks,
                term,
            )? {
                attempt = next_plan.next_attempt;
                super::super::resume::apply_next_attempt_plan(prompt_state, &next_plan);
                continue;
            }
            let fail_msg = format!(
                "inline closure validation failed ({} {})",
                failures.len(),
                if failures.len() == 1 {
                    "failure"
                } else {
                    "failures"
                }
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&fail_msg, term);
            }
            guard.emit_terminal(LifecycleSignal::Failure);
            return Err(eyre!("{fail_msg}"));
        }

        // Evaluate post-checks. In inline mode this now runs against the
        // post-closure document so file-state checks see the final artifact.
        let post_report = info_span!(
            "harness_post_validation",
            attempt,
            rule_count = plan.post_checks.len(),
        )
        .in_scope(|| {
            claudine::harness::evaluate_post_checks(
                &plan,
                &snapshot,
                &outcome,
                Some(&permission_probe),
            )
        });

        if show_checks {
            claudine::harness::report::report_phase_discovery(
                claudine::harness::FailurePhase::PostCheck,
                post_report.count(),
                term,
            );
            claudine::harness::report::report_check_outcomes(&post_report, term);
        }

        if post_report.all_passed() {
            guard.emit_terminal(LifecycleSignal::Success);
            terminal_signals = iteration_signals;
            return Ok((outcome.exit_code, harness_perf, terminal_signals));
        }

        let failures = post_report.failures();
        let contexts = claudine::harness::build_validation_failure_context(
            &failures,
            provider.as_slug(),
            plan.source_path.as_path(),
            attempt,
            outcome.session_id.clone(),
            Some(outcome.clone()),
        );
        if let Some(next_plan) = super::super::resume::try_resolve_handler(
            &contexts,
            &plan,
            attempt,
            DEFAULT_MAX_RETRIES,
            profile,
            outcome.session_id.as_deref(),
            &prompt_state.source_path,
            repo_root,
            show_checks,
            term,
        )? {
            attempt = next_plan.next_attempt;
            super::super::resume::apply_next_attempt_plan(prompt_state, &next_plan);
            continue;
        }
        let fail_msg = format!(
            "post-check validation failed ({} {})",
            failures.len(),
            if failures.len() == 1 {
                "failure"
            } else {
                "failures"
            }
        );
        if show_checks {
            claudine::harness::report::report_unhandled_failure(&fail_msg, term);
        }
        guard.emit_terminal(LifecycleSignal::Failure);
        return Err(eyre!("{fail_msg}"));
    }
}
