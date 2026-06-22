use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::LifecycleSignal;
use claudine::composition::lifecycle_context::{LifecycleCurrent, LifecycleErrorInfo, LifecycleTiming};
use claudine::composition::lifecycle_executor::{
    LifecycleEventOutcome, StackControl, StackExecutionContext, SystemShellRunner,
};
use claudine::events::EnvironmentContext;
use claudine::provider::Provider;
use claudine::stream::stderr::Verbosity;
use color_eyre::eyre::{Result, eyre};
use darkmatter::effects::EffectEngine;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::Path;
use tracing::info_span;

use super::super::composition::IterationSummarySignals;
use super::{
    CachedHarnessLoopContext, HarnessPromptState, MaterializedHarnessPrompt, build_harness_launch,
    execute_harness_attempt, harness_prompt_mode_label, materialize_harness_prompt, HarnessPromptMode,
};

/// Execute a terminal lifecycle event, converting an explicit `Error` control
/// action into `Failure` for events that would otherwise record a successful
/// or blocked outcome.
///
/// `Success` and `Blocked` run their stack **exactly once** (stack-only, before
/// any top-level communication). If that stack terminates with
/// `StackControl::Error`, the run is routed to the `Failure` event — its
/// top-level communication and stack fire — and the guard records `Failure` as
/// the terminal signal; the success/blocked top-level communication never
/// fires. Otherwise the success/blocked top-level communication fires after the
/// already-run stack and that signal is recorded. This preserves the spec rule
/// that an explicit `error()` action in a success/blocked stack downgrades the
/// run, without running the stack twice.
#[allow(clippy::too_many_arguments)]
fn execute_terminal_event(
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
) -> LifecycleEventOutcome {
    if matches!(signal, LifecycleSignal::Success | LifecycleSignal::Blocked) {
        // Run the success/blocked stack exactly once (no top-level comm yet).
        let outcome = run_lifecycle_stack_only(
            guard,
            signal,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            err,
        );
        if matches!(outcome.control, Some(StackControl::Error { .. })) {
            // The stack downgraded the run: record `Failure` as the terminal
            // signal and fire the failure event (top-level comm + failure
            // stack). The success/blocked top-level comm is intentionally
            // skipped.
            return run_lifecycle_event(
                guard,
                LifecycleSignal::Failure,
                materialized,
                source_path,
                repo_root,
                term,
                effect_engine,
                err,
            );
        }
        // Stack stayed success/blocked: emit only the top-level comm (the
        // stack already ran) and record this signal as terminal.
        emit_lifecycle_top_level_only(
            guard,
            signal,
            materialized,
            source_path,
            repo_root,
            term,
            effect_engine,
            err,
        );
        return outcome;
    }
    run_lifecycle_event(
        guard,
        signal,
        materialized,
        source_path,
        repo_root,
        term,
        effect_engine,
        err,
    )
}

/// Record `signal` as the terminal signal and emit only its top-level
/// communication properties (no stack).
///
/// Used by [`execute_terminal_event`] after a success/blocked stack has already
/// run once: this fires the communication surface without re-running the stack.
/// If the terminal slot was already taken (another terminal signal won),
/// nothing is emitted.
#[allow(clippy::too_many_arguments)]
fn emit_lifecycle_top_level_only(
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
) {
    if !guard.record_event_emission(signal) {
        return;
    }
    let ctx = build_lifecycle_stack_context_for_materialized(
        signal,
        materialized,
        source_path,
        repo_root,
        term,
        guard.emitter(),
        guard.context().settings,
        guard.context().messaging,
        effect_engine,
        err,
    );
    ctx.emit_top_level_for_signal(guard.config());
}

/// Run one lifecycle event (top-level + stack), recording emission state in
/// `guard` and returning the event outcome.
///
/// The helper is careful to release the mutable borrow used for state recording
/// before building the [`StackExecutionContext`] that immutably borrows the
/// guard's emitter.
#[allow(clippy::too_many_arguments)]
fn run_lifecycle_event(
    guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
) -> LifecycleEventOutcome {
    if !guard.record_event_emission(signal) {
        return LifecycleEventOutcome::default();
    }
    let ctx = build_lifecycle_stack_context_for_materialized(
        signal,
        materialized,
        source_path,
        repo_root,
        term,
        guard.emitter(),
        guard.context().settings,
        guard.context().messaging,
        effect_engine,
        err,
    );
    guard.run_event_stack(signal, &ctx)
}

/// Run only the stack for `signal` (no top-level communication).
///
/// Used to preview success/blocked stacks for explicit `Error` control actions
/// before committing to the terminal signal.
#[allow(clippy::too_many_arguments)]
fn run_lifecycle_stack_only(
    guard: &claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    materialized: &MaterializedHarnessPrompt,
    source_path: &Path,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
) -> LifecycleEventOutcome {
    let ctx = build_lifecycle_stack_context_for_materialized(
        signal,
        materialized,
        source_path,
        repo_root,
        term,
        guard.emitter(),
        guard.context().settings,
        guard.context().messaging,
        effect_engine,
        err,
    );
    ctx.execute_stack_for_signal(guard.config())
}

/// Build a stack context from a materialized prompt and guard-derived routes.
#[allow(clippy::too_many_arguments)]
fn build_lifecycle_stack_context_for_materialized<'a>(
    signal: LifecycleSignal,
    materialized: &'a MaterializedHarnessPrompt,
    source_path: &'a Path,
    repo_root: Option<&'a Path>,
    term: &'a Terminal,
    emitter: &'a dyn claudine::composition::LifecycleEmitter,
    settings: &'a claudine::events::GlobalSettings,
    messaging: &'a claudine::messaging::RuntimeMessagingSettings,
    effect_engine: &'a EffectEngine,
    err: Option<&'a LifecycleErrorInfo>,
) -> StackExecutionContext<'a> {
    static EMPTY_FRONTMATTER: std::sync::OnceLock<serde_json::Map<String, serde_json::Value>> =
        std::sync::OnceLock::new();
    let fm_map = materialized
        .frontmatter
        .as_object()
        .unwrap_or_else(|| EMPTY_FRONTMATTER.get_or_init(serde_json::Map::new));
    let base_dir = source_path.parent().or(repo_root);
    StackExecutionContext {
        signal,
        frontmatter: fm_map,
        err,
        timing: None,
        current: None,
        base_dir,
        effect_engine,
        shell_runner: &SystemShellRunner,
        emitter,
        term,
        source_path,
        repo_root,
        messaging,
        settings,
    }
}

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
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    // When `true`, every structured-stream attempt in the harness loop
    // emits the prompt-scoped timing header and — if the parsed plan
    // carries `timeout_warn` / `step_timeout_warn` — their fire-once
    // warning lines. Wrapper passthrough callers with no prompt file
    // pass `false` to suppress the header entirely; composition callers
    // pass `true`.
    emit_prompt_timing: bool,
) -> Result<(i32, Option<crate::perf::AgentExecutionPerf>, Option<IterationSummarySignals>)> {
    const DEFAULT_MAX_RETRIES: u32 = 3;
    let mutation_root = repo_root.unwrap_or(child_cwd).to_path_buf();
    let effect_engine = EffectEngine::builder()
        .mutation_root(&mutation_root)
        .auto_rehash(false)
        .build();
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
            .map_err(|e| lifecycle_guard.emit_blocked_or_err(e))?
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
        .map_err(|e| lifecycle_guard.emit_blocked_or_err(e))?;

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
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Blocked,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
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
                    run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Blocked,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        None,
                    );
                    run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Finalize,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        None,
                    );
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
                    run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Blocked,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        None,
                    );
                    run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Finalize,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        None,
                    );
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
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Blocked,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
            return Err(eyre!("{fail_msg}"));
        }

        // Emit start lifecycle signal before the first provider launch.
        let start_outcome = run_lifecycle_event(
            lifecycle_guard,
            LifecycleSignal::Start,
            &materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            &effect_engine,
            None,
        );
        if let Some(ref control) = start_outcome.control {
            match control {
                StackControl::Error { reason } => {
                    run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Failure,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        None,
                    );
                    run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Finalize,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        None,
                    );
                    let msg = reason
                        .clone()
                        .unwrap_or_else(|| "lifecycle start error".to_string());
                    return Err(eyre!(msg));
                }
                StackControl::Stop => {}
                _ => {
                    return Err(eyre!(
                        "lifecycle control action {control:?} is not valid at start"
                    ));
                }
            }
        }
        if start_outcome.routes_to_failure(LifecycleSignal::Start) {
            let failure_ctx = start_outcome
                .action_error
                .as_ref()
                .map(|e| build_lifecycle_stack_context_for_materialized(
                    LifecycleSignal::Failure,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    lifecycle_guard.emitter(),
                    lifecycle_guard.context().settings,
                    lifecycle_guard.context().messaging,
                    &effect_engine,
                    Some(e),
                ));
            if let Some(ctx) = failure_ctx.as_ref() {
                lifecycle_guard.run_event_stack(LifecycleSignal::Failure, ctx);
            } else {
                run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Failure,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    None,
                );
            }
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                start_outcome.action_error.as_ref(),
            );
            return Err(eyre!("lifecycle start failed"));
        }

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
            lifecycle_guard.mark_provider_launched();
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
            execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Failure,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
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
            execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Failure,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
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
            execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Failure,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
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
            execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Success,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
            run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                None,
            );
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
        execute_terminal_event(
            lifecycle_guard,
            LifecycleSignal::Failure,
            &materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            &effect_engine,
            None,
        );
        run_lifecycle_event(
            lifecycle_guard,
            LifecycleSignal::Finalize,
            &materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            &effect_engine,
            None,
        );
        return Err(eyre!("{fail_msg}"));
    }
}

#[cfg(test)]
mod terminal_event_tests {
    use super::*;
    use claudine::composition::{
        LifecycleConfig, LifecycleEmitter, LifecycleRunGuard, LifecycleRuntimeContext,
        parse_lifecycle_config,
    };
    use claudine::events::GlobalSettings;
    use claudine::messaging::RuntimeMessagingSettings;
    use std::sync::Mutex;

    /// One emitted top-level communication, recorded by [`RecordingEmitter`].
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Emitted {
        Stderr(LifecycleSignal, String),
        Message(String),
        Speech(String),
    }

    /// Lifecycle emitter test double that records every emission.
    #[derive(Default)]
    struct RecordingEmitter {
        events: Mutex<Vec<Emitted>>,
    }

    impl RecordingEmitter {
        fn events(&self) -> Vec<Emitted> {
            self.events.lock().unwrap().clone()
        }
    }

    impl LifecycleEmitter for RecordingEmitter {
        fn emit_stderr(&self, signal: LifecycleSignal, text: &str, _term: &Terminal) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Stderr(signal, text.to_string()));
        }
        fn emit_message(
            &self,
            text: &str,
            _source_path: &Path,
            _repo_root: Option<&Path>,
            _messaging: &RuntimeMessagingSettings,
        ) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Message(text.to_string()));
        }
        fn emit_speech(&self, text: &str, _config: biscuit_speaks::TtsConfig) {
            self.events
                .lock()
                .unwrap()
                .push(Emitted::Speech(text.to_string()));
        }
        fn emit_effect(&self, _name: &str) {}
        fn emit_notification(&self, _title: &str) {}
    }

    fn materialized(frontmatter: serde_json::Value) -> MaterializedHarnessPrompt {
        MaterializedHarnessPrompt {
            frontmatter,
            prompt: String::new(),
            env_overrides: Vec::new(),
            inline_closure_plan: None,
        }
    }

    /// Number of lines a stack's `append_line` side effect wrote — i.e. the
    /// number of times the stack actually executed its side effects.
    fn line_count(path: &Path) -> usize {
        std::fs::read_to_string(path)
            .map(|s| s.lines().count())
            .unwrap_or(0)
    }

    struct Fixture {
        _dir: tempfile::TempDir,
        log_path: PathBuf,
        config: LifecycleConfig,
        settings: GlobalSettings,
        messaging: RuntimeMessagingSettings,
        term: Terminal,
        source_path: PathBuf,
        materialized: MaterializedHarnessPrompt,
    }

    use std::path::PathBuf;

    /// Build a fixture whose `success` and `blocked` stacks each append one
    /// line to `events.log` (a side-effect counter) and carry a top-level
    /// `stderr` communication. When `with_error` is set, the named event's
    /// stack ends in `error('downgraded')` so it routes to `failure`.
    fn fixture(frontmatter: serde_json::Value) -> Fixture {
        let dir = tempfile::tempdir().unwrap();
        let source_path = dir.path().join("prompt.md");
        let log_path = dir.path().join("events.log");
        let config = parse_lifecycle_config(&frontmatter, &source_path).unwrap();
        Fixture {
            _dir: dir,
            log_path,
            config,
            settings: GlobalSettings::default(),
            messaging: RuntimeMessagingSettings {
                user: None,
                repo: None,
            },
            term: Terminal::default(),
            source_path,
            materialized: materialized(frontmatter),
        }
    }

    fn engine(root: &Path) -> EffectEngine {
        EffectEngine::builder()
            .mutation_root(root)
            .auto_rehash(false)
            .build()
    }

    #[test]
    fn success_stack_side_effects_run_exactly_once() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "succeeded",
                "stack": [{"action": "append_line('events.log', 'ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let outcome = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
        );

        assert!(outcome.control.is_none());
        // The stack's side effect fired exactly once (was twice before the fix).
        assert_eq!(line_count(&fx.log_path), 1, "stack ran exactly once");
        // Top-level success communication fired (the stack stayed success).
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Success, "succeeded".to_string())]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Success));
    }

    #[test]
    fn success_stack_error_routes_to_failure_runs_once_and_skips_success_comm() {
        let fx = fixture(serde_json::json!({
            "success": {
                "stderr": "succeeded",
                "stack": [{"action": ["append_line('events.log', 'ran')", "error('downgraded')"]}]
            },
            "failure": {
                "stderr": "failed",
                "stack": [{"action": "append_line('events.log', 'failure-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        guard.mark_provider_launched();
        let eng = engine(fx._dir.path());

        let outcome = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Success,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
        );

        // Outcome reflects the failure event's run (no Error control surviving).
        assert!(outcome.control.is_none());
        // Success stack ran once (append + error), failure stack ran once.
        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["ran", "failure-ran"],
            "success stack and failure stack each ran exactly once"
        );
        // Failure top-level comm fired; success top-level comm did NOT.
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string())]
        );
        // Guard recorded the downgraded terminal signal as Failure.
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    }

    #[test]
    fn blocked_stack_side_effects_run_exactly_once() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stderr": "blocked",
                "stack": [{"action": "append_line('events.log', 'ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        let outcome = execute_terminal_event(
            &mut guard,
            LifecycleSignal::Blocked,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
        );

        assert!(outcome.control.is_none());
        assert_eq!(line_count(&fx.log_path), 1, "blocked stack ran exactly once");
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Blocked, "blocked".to_string())]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Blocked));
    }

    #[test]
    fn blocked_stack_error_routes_to_failure_runs_once_and_skips_blocked_comm() {
        let fx = fixture(serde_json::json!({
            "blocked": {
                "stderr": "blocked",
                "stack": [{"action": ["append_line('events.log', 'ran')", "error('downgraded')"]}]
            },
            "failure": {
                "stderr": "failed",
                "stack": [{"action": "append_line('events.log', 'failure-ran')"}]
            }
        }));
        let emitter = RecordingEmitter::default();
        let ctx = LifecycleRuntimeContext {
            settings: &fx.settings,
            messaging: &fx.messaging,
            term: &fx.term,
            source_path: &fx.source_path,
            repo_root: Some(fx._dir.path()),
        };
        let mut guard = LifecycleRunGuard::new(&fx.config, &ctx, &emitter);
        let eng = engine(fx._dir.path());

        execute_terminal_event(
            &mut guard,
            LifecycleSignal::Blocked,
            &fx.materialized,
            &fx.source_path,
            Some(fx._dir.path()),
            &fx.term,
            &eng,
            None,
        );

        let lines = std::fs::read_to_string(&fx.log_path).unwrap();
        assert_eq!(
            lines.lines().collect::<Vec<_>>(),
            vec!["ran", "failure-ran"],
            "blocked stack and failure stack each ran exactly once"
        );
        assert_eq!(
            emitter.events(),
            vec![Emitted::Stderr(LifecycleSignal::Failure, "failed".to_string())]
        );
        assert_eq!(guard.terminal_signal(), Some(LifecycleSignal::Failure));
    }
}
