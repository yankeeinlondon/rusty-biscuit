use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::LifecycleSignal;
use claudine::composition::lifecycle_context::{LifecycleCurrent, LifecycleErrorInfo, LifecycleTiming};
use claudine::composition::lifecycle_control::{ControlDispatch, control_budget_for, decide_control};
use claudine::composition::lifecycle_executor::{
    LifecycleEventOutcome, StackControl, StackExecutionContext, SystemShellRunner,
};
use claudine::composition::{
    CompositionError, IterationSummarySignals, route_blocked_finalize, route_failure_finalize,
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

use super::{
    CachedHarnessLoopContext, HarnessPromptState, MaterializedHarnessPrompt, build_harness_launch,
    execute_harness_attempt, harness_prompt_mode_label, materialize_harness_prompt,
    preflight_proxy_target, HarnessPromptMode,
};

type HarnessLoopResult = (
    i32,
    Option<crate::perf::AgentExecutionPerf>,
    Option<IterationSummarySignals>,
);

#[allow(dead_code)]
enum LoopStep {
    NextAttempt,
    Return(HarnessLoopResult),
    Abort { reason: String, code: i32 },
}

struct HarnessLoopCtx<'a, 'guard> {
    provider: Provider,
    profile: &'a dyn super::super::profile::WrapperProfile,
    binary_path: &'a Path,
    child_cwd: &'a Path,
    effective_non_interactive: bool,
    cli_timeout: Option<String>,
    cli_step_timeout: Option<String>,
    cli_stall_timeout: Option<String>,
    base_args: &'a [String],
    base_env: &'a HashMap<OsString, OsString>,
    prompt_state: &'a mut HarnessPromptState,
    repo_root: Option<&'a Path>,
    shell_options: claudine::harness::ShellApprovalOptions,
    use_structured: bool,
    structured_codex_output: Option<&'a super::super::policy::StructuredCodexOutput>,
    stdout_noise: &'a [&'a str],
    stderr_noise: &'a [&'a str],
    suppress_stderr_on_success: bool,
    show_checks: bool,
    stream_verbosity: Verbosity,
    detail_requested: bool,
    // Suppresses the flow-control proxy INFO line and the proxy-target agent
    // prompt preview when the caller requested a silent run.
    silent: bool,
    env_context: &'a EnvironmentContext,
    dispatch_context: &'a HashMap<String, serde_json::Value>,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    term: &'a Terminal,
    lifecycle_guard: &'a mut claudine::composition::LifecycleRunGuard<'guard>,
    initial_proxy_target: Option<&'a Path>,
    emit_prompt_timing: bool,
}

mod control_dispatch;
mod error_routing;
mod lifecycle_events;
mod proxy;
mod requeue;

use control_dispatch::*;
use error_routing::*;
use lifecycle_events::*;
use proxy::*;
#[allow(unused_imports)] // entirely dead_code until the rendezvous backend lands
use requeue::*;

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
    cli_stall_timeout: Option<String>,
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
    silent: bool,
    env_context: &EnvironmentContext,
    dispatch_context: &HashMap<String, serde_json::Value>,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    term: &Terminal,
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    // Set when an `initialize`-stack `proxy(...)` already redirected to this
    // target document upstream. Seeds the proxy chain (so a proxy back to the
    // original is caught as a cycle) and triggers the loop-top lifecycle
    // re-parse so the guard adopts the target's lifecycle.
    initial_proxy_target: Option<&Path>,
    // When `true`, every structured-stream attempt in the harness loop
    // emits the prompt-scoped timing header and — if the parsed plan
    // carries `timeout_warn` / `step_timeout_warn` — their fire-once
    // warning lines. Wrapper passthrough callers with no prompt file
    // pass `false` to suppress the header entirely; composition callers
    // pass `true`.
    emit_prompt_timing: bool,
) -> Result<HarnessLoopResult> {
    let ctx = HarnessLoopCtx {
        provider,
        profile,
        binary_path,
        child_cwd,
        effective_non_interactive,
        cli_timeout,
        cli_step_timeout,
        cli_stall_timeout,
        base_args,
        base_env,
        prompt_state,
        repo_root,
        shell_options,
        use_structured,
        structured_codex_output,
        stdout_noise,
        stderr_noise,
        suppress_stderr_on_success,
        show_checks,
        stream_verbosity,
        detail_requested,
        silent,
        env_context,
        dispatch_context,
        initial_materialized,
        term,
        lifecycle_guard,
        initial_proxy_target,
        emit_prompt_timing,
    };
    match run_harness_loop_inner(ctx)? {
        LoopStep::Return(result) => Ok(result),
        LoopStep::Abort { reason, code } => Err(eyre!("{reason} (exit code {code})")),
        LoopStep::NextAttempt => unreachable!("the harness loop owns attempt re-entry"),
    }
}

#[allow(unused_assignments)]
#[allow(
    clippy::result_large_err,
    reason = "proxy preflight preserves the shared typed CompositionError until lifecycle routing"
)]
fn run_harness_loop_inner(ctx: HarnessLoopCtx<'_, '_>) -> Result<LoopStep> {
    let HarnessLoopCtx {
        provider,
        profile,
        binary_path,
        child_cwd,
        effective_non_interactive,
        cli_timeout,
        cli_step_timeout,
        cli_stall_timeout,
        base_args,
        base_env,
        prompt_state,
        repo_root,
        shell_options,
        use_structured,
        structured_codex_output,
        stdout_noise,
        stderr_noise,
        suppress_stderr_on_success,
        show_checks,
        stream_verbosity,
        detail_requested,
        silent,
        env_context,
        dispatch_context,
        initial_materialized,
        term,
        lifecycle_guard,
        initial_proxy_target,
        emit_prompt_timing,
    } = ctx;
    let mutation_root = repo_root.unwrap_or(child_cwd).to_path_buf();
    let effect_engine = EffectEngine::builder()
        .mutation_root(&mutation_root)
        .auto_rehash(false)
        .build();
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
    // Run-level wall-clock anchor for lifecycle `timing.{document_ms,total_ms}`
    // emitted at each event. Captured before the first attempt so it spans the
    // whole harness loop (all retry/resume iterations).
    let loop_start = std::time::Instant::now();
    // Per-control retry/resume ceilings established on first firing of a
    // lifecycle `retry`/`resume` control in a terminal stack.
    let mut control_budgets = ControlBudgets::default();
    // Proxy hand-off chain + pending flag. A `Some(target)` initial proxy
    // (an `initialize`-stack `proxy(...)`) already swapped `source_path`
    // upstream, so seed the chain with it and flag a re-parse: the guard was
    // built against the *original* document's lifecycle and must adopt the
    // target's before its events fire.
    let mut proxy_tracking = ProxyTracking::default();
    if let Some(initial_target) = initial_proxy_target {
        if !proxy_tracking
            .chain
            .iter()
            .any(|p| p == lifecycle_guard.context().source_path)
        {
            proxy_tracking
                .chain
                .push(lifecycle_guard.context().source_path.to_path_buf());
        }
        proxy_tracking.chain.push(initial_target.to_path_buf());
        proxy_tracking.pending = true;
    }

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
        // Announce a pending proxy hand-off before the target is materialized,
        // so the redirect is reported even if the target then fails to compose
        // (e.g. a bad transclusion). Fires for every proxy — `initialize` or
        // recovery — because both re-enter here with `pending` set and
        // `source_path` already swapped to the target.
        if proxy_tracking.pending && !silent {
            claudine::harness::report::report_proxy_handoff(&prompt_state.source_path, term);
        }
        // A `proxy` hand-off is "a fresh prompt run, including pre-flight"
        // (lifecycle spec). The proxying document's pre-flight never saw the
        // target's own frontmatter `$(...)` / `::shell` commands, so audit them
        // now and fold the approvals into the carried pre-approved set before the
        // re-materialize compose below expands them — otherwise a whitelisted
        // command still trips `NotPreApproved`. Skipped when a seed prompt is
        // already staged (the original prepared prompt was preflighted upstream).
        if proxy_tracking.pending
            && initial_materialized.is_none()
            && let Err(e) = info_span!(
                "harness_proxy_target_preflight",
                attempt,
                source_path = %prompt_state.source_path.display(),
            )
            .in_scope(|| {
                preflight_proxy_target(
                    prompt_state,
                    harness_context.shell_options(),
                    child_cwd,
                )
            })
        {
            // A denial is a composition-preflight blocked path: route through
            // the stack-aware runner so `blocked`/`finalize` fire, matching the
            // primary pre-flight's shell-audit handling.
            let err_info =
                LifecycleErrorInfo::from_action_failure("shell_approval", e.to_string());
            let empty = MaterializedHarnessPrompt {
                frontmatter: serde_json::Value::Null,
                prompt: String::new(),
                env_overrides: Vec::new(),
                inline_closure_plan: None,
                live_frontmatter: MaterializedHarnessPrompt::live_cell_from(
                    &serde_json::Value::Null,
                ),
            };
            return Err(emit_blocked_finalize_with_err(
                lifecycle_guard,
                &empty,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            )
            .map(color_eyre::eyre::Report::from)
            .unwrap_or_else(|| eyre!("{e}")));
        }
        let materialized = if let Some(seed) = initial_materialized.take() {
            seed
        } else {
            info_span!(
                "harness_materialize_prompt",
                attempt,
                source_path = %prompt_state.source_path.display(),
            )
            .in_scope(|| materialize_harness_prompt(prompt_state, repo_root, child_cwd))
            .map_err(|e| {
                let err_info =
                    LifecycleErrorInfo::from_action_failure("materialize", e.to_string());
                // Materialization failed, so there is no prompt to carry into the
                // stack context. Synthesize an empty one: the guard still holds the
                // (proxying/original) document's parsed lifecycle, so its
                // blocked/finalize stacks fire. `frontmatter: Null` makes the
                // stack-context builder fall back to an empty frontmatter map, so
                // any `when:` referencing frontmatter resolves against {} — correct,
                // because the real frontmatter never materialized.
                let empty = MaterializedHarnessPrompt {
                    frontmatter: serde_json::Value::Null,
                    prompt: String::new(),
                    env_overrides: Vec::new(),
                    inline_closure_plan: None,
                    live_frontmatter: MaterializedHarnessPrompt::live_cell_from(
                        &serde_json::Value::Null,
                    ),
                };
                // A lifecycle evaluation error raised by the synthesized empty
                // prompt's blocked/finalize stack takes precedence over the
                // original materialize error — the lifecycle raise is the more
                // actionable diagnosis and must halt the run.
                match emit_blocked_finalize_with_err(
                    lifecycle_guard,
                    &empty,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    &err_info,
                    loop_start,
                ) {
                    Some(ce) => ce.into(),
                    // Return the original report unflattened: it wraps the typed
                    // Darkmatter compose/transclusion `BlockError`, so the CLI's
                    // top-level walker renders it as a styled block instead of a
                    // crude `Error: <string>`. `eyre!("{e}")` would discard the
                    // type and defeat that path.
                    None => e,
                }
            })?
        };

        // A proxy hand-off (from `initialize`, `blocked`, or `failure`)
        // swapped `source_path` to the target. The guard still holds the
        // proxying document's lifecycle, so repoint it at the target's —
        // parsed from the freshly materialized target frontmatter — before any
        // of the target's events fire. Without this the target's own
        // `start`/`success`/`finalize` never run and the proxying document's
        // `failure`/`proxy` stack re-fires, looping forever.
        if proxy_tracking.pending {
            proxy_tracking.pending = false;
            match claudine::composition::parse_lifecycle_config(
                &materialized.frontmatter,
                &prompt_state.source_path,
            ) {
                Ok(target_lifecycle) => lifecycle_guard.set_config(target_lifecycle),
                Err(e) => {
                    let err_info = LifecycleErrorInfo::from_composition_error(&e);
                    // A lifecycle evaluation error raised by the target's
                    // blocked/finalize stack takes precedence over the original
                    // target-lifecycle parse error — the lifecycle raise is the
                    // more actionable diagnosis and must halt the run.
                    return Err(emit_blocked_finalize_with_err(
                        lifecycle_guard,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        &err_info,
                        loop_start,
                    )
                    .map(color_eyre::eyre::Report::from)
                    .unwrap_or_else(|| eyre!("{e}")));
                }
            }
            // The proxied document enters at its own `initialize` — a fresh
            // prompt run. Reset the guard and emit the target's `initialize`
            // before pre-flight checks, honoring target-side `Skip`/`Proxy`/
            // `Error` logic.
            match run_target_initialize(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                loop_start,
            ) {
                TargetInitializeAction::Proceed => {}
                TargetInitializeAction::ExitCleanly => {
                    return Ok(LoopStep::Return((0, None, None)));
                }
                TargetInitializeAction::Abort(e) => return Err(e),
                TargetInitializeAction::Repoint { resolved } => {
                    if !proxy_tracking
                        .chain
                        .iter()
                        .any(|p| p == &prompt_state.source_path)
                    {
                        proxy_tracking.chain.push(prompt_state.source_path.clone());
                    }
                    if !claudine::composition::proxy_handoff_allowed(
                        &proxy_tracking.chain,
                        &resolved,
                    ) {
                        return Err(CompositionError::LifecycleProxyCycle {
                            source_path: prompt_state.source_path.clone(),
                            target: resolved.display().to_string(),
                            chain: proxy_tracking
                                .chain
                                .iter()
                                .map(|p| p.display().to_string())
                                .collect(),
                            limit: claudine::composition::MAX_PROXY_HOPS,
                        }
                        .into());
                    }
                    prompt_state.source_path = resolved.clone();
                    prompt_state.original_ref = resolved.display().to_string();
                    prompt_state.prompt_tail.clear();
                    prompt_state.next_prompt_override = None;
                    prompt_state.next_resume_session_id = None;
                    proxy_tracking.chain.push(resolved.clone());
                    proxy_tracking.pending = true;
                    // The next iteration's pending-adoption point announces the
                    // hand-off via `report_proxy_handoff`, so no message here.
                    // Re-enter at attempt 1 so the target document gets a clean
                    // pre-flight / freeze cycle rather than inheriting the
                    // proxying document's attempt count.
                    attempt = 1;
                    continue;
                }
            }
            // Reached only when the target's `initialize` returned `Proceed`
            // (the other arms return/continue above), so this is the settled
            // document that actually runs. Its composed body — not the proxying
            // source's — is the agent prompt; the pre-loop preview was
            // suppressed for an `initialize` proxy, so emit it here. Gated to
            // non-interactive runs to mirror the pre-loop preview.
            if effective_non_interactive {
                crate::output::log_compose_prompt(
                    &materialized.prompt,
                    detail_requested,
                    silent,
                    false,
                    term,
                );
            }
        }

        let plan = info_span!(
            "harness_plan_parse",
            attempt,
            source_path = %prompt_state.source_path.display(),
        )
        .in_scope(|| {
            claudine::harness::parse_harness_plan(
                &materialized.frontmatter,
                &prompt_state.source_path,
            )
        })
        .map_err(|e| {
            let err_info = LifecycleErrorInfo::from_harness_error(&e);
            // A lifecycle evaluation error raised by the blocked/finalize stack
            // takes precedence over the original harness-plan parse error —
            // the lifecycle raise is the more actionable diagnosis and must
            // halt the run.
            match emit_blocked_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            ) {
                Some(ce) => ce.into(),
                None => eyre!("{e}"),
            }
        })?;

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
            let err_info = LifecycleErrorInfo::from_action_failure(
                "missing_source",
                format!(
                    "source file does not exist: {}",
                    prompt_state.source_path.display()
                ),
            );
            // A lifecycle evaluation error raised by the blocked/finalize stack
            // takes precedence over the original missing-source error — the
            // lifecycle raise is the more actionable diagnosis and must halt
            // the run non-zero instead of being swallowed.
            if let Some(ce) = emit_blocked_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            ) {
                return Err(ce.into());
            }
            return Err(eyre!(
                "source file does not exist: {}",
                prompt_state.source_path.display()
            ));
        }

        // The parsed harness plan is used for shell audit and timeout
        // configuration. Pre/post validation checks have been removed.

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
                claudine::harness::collect_auditable_commands(source_text.as_deref())?;

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
                let msg = format!(
                    "shell audit failed: {} denied directive(s) in source page",
                    failed.len()
                );
                if show_checks {
                    claudine::harness::report::report_unhandled_failure(
                        "shell audit failed for source-page directives — cannot proceed",
                        term,
                    );
                }
                let err_info = LifecycleErrorInfo::from_action_failure("shell_audit", &msg);
                // A lifecycle evaluation error raised by the blocked/finalize
                // stack takes precedence over the original shell-audit error —
                // the lifecycle raise is the more actionable diagnosis and must
                // halt the run non-zero instead of being swallowed.
                if let Some(ce) = emit_blocked_finalize_with_err(
                    lifecycle_guard,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    &err_info,
                    loop_start,
                ) {
                    return Err(ce.into());
                }
                return Err(eyre!(msg));
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

        // Pre-check validation has been removed. Shell audit still runs above
        // for Passthrough mode; composition flows audit during preflight.

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
            loop_start,
        );
        // A late-binding evaluation error on `start` (a crashed `when:` guard or
        // interpolation) routes through `failure` → `finalize` like any other
        // setup failure and halts non-zero (Decision #5). Checked before the
        // control match because an evaluation raise leaves `control` `None`.
        if let Some(err) = handle_setup_evaluation_error(
            &start_outcome,
            "start",
            lifecycle_guard,
            &materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            &effect_engine,
            loop_start,
        ) {
            return Err(err);
        }
        if let Some(ref control) = start_outcome.control {
            match control {
                StackControl::Error { reason } => {
                    let msg = reason
                        .clone()
                        .unwrap_or_else(|| "lifecycle start error".to_string());
                    let err_info =
                        LifecycleErrorInfo::from_action_failure("error", msg.as_str());
                    if let Some(ce) = emit_failure_finalize_with_err(
                        lifecycle_guard,
                        &materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        &effect_engine,
                        &err_info,
                        loop_start,
                    ) {
                        return Err(ce.into());
                    }
                    return Err(eyre!(msg));
                }
                StackControl::Stop => {}
                // retry/resume/proxy/requeue at `start` dispatch through the
                // uniform path. The provider has not launched yet, so `resume`
                // surfaces `ResumeWithoutSession` and `retry` re-enters before
                // the agent runs.
                _ => match dispatch_terminal_control(
                    &start_outcome,
                    attempt,
                    &mut control_budgets,
                    None,
                    profile,
                    provider,
                    prompt_state,
                    &materialized,
                    repo_root,
                    lifecycle_guard,
                    &mut proxy_tracking,
                    term,
                    show_checks,
                ) {
                    TerminalControlAction::Continue { next_attempt } => {
                        attempt = next_attempt;
                        continue;
                    }
                    TerminalControlAction::Abort(err) => {
                        // No error info is available at the start-control-abort
                        // point, so `finalize` runs with `None` `err`; but its
                        // own stack may still raise an evaluation error, which
                        // must surface and halt rather than being swallowed by
                        // the abort `return`.
                        let finalize_outcome = run_lifecycle_event(
                            lifecycle_guard,
                            LifecycleSignal::Finalize,
                            &materialized,
                            &prompt_state.source_path,
                            repo_root,
                            term,
                            &effect_engine,
                            None,
                            loop_start,
                        );
                        if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                            return Err(
                                crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                                    CompositionError::lifecycle_evaluation(
                                        "finalize",
                                        &prompt_state.source_path,
                                        eval_info,
                                    ),
                                    term,
                                )
                                .into(),
                            );
                        }
                        return Err(err);
                    }
                    TerminalControlAction::Fallthrough => {}
                },
            }
        }
        if start_outcome.routes_to_failure(LifecycleSignal::Start) {
            // Record the `Failure` terminal signal FIRST, while we still hold
            // `&mut guard`, so the subsequent `Finalize` actually fires. The
            // error-carrying context built below immutably borrows
            // `guard.emitter()`/`guard.context()`, so recording must happen
            // before the borrow split. Skipping this (calling `run_event_stack`
            // directly) would leave `terminal_emitted` false and silently
            // suppress `finalize`.
            if lifecycle_guard.record_event_emission(LifecycleSignal::Failure) {
                let (timing, current) = capture_lifecycle_globals(
                    &prompt_state.source_path,
                    repo_root,
                    lifecycle_guard.context().launch_area,
                    loop_start,
                );
                let ctx = build_lifecycle_stack_context_for_materialized(
                    LifecycleSignal::Failure,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    lifecycle_guard.context().launch_area,
                    lifecycle_guard.effective_prepared_context(),
                    term,
                    lifecycle_guard.emitter(),
                    lifecycle_guard.context().settings,
                    lifecycle_guard.context().messaging,
                    &effect_engine,
                    start_outcome.action_error.as_ref(),
                    Some(&timing),
                    Some(&current),
                );
                let failure_outcome =
                    lifecycle_guard.run_event_stack(LifecycleSignal::Failure, &ctx);
                // If `failure` raised, thread its error (not the original) into
                // finalize so a `finalize.stack` can branch on the failure raise.
                let synthetic =
                    LifecycleErrorInfo::from_action_failure("error", "lifecycle start failed");
                let active_err = failure_outcome
                    .evaluation_error
                    .as_ref()
                    .or(start_outcome.action_error.as_ref())
                    .unwrap_or(&synthetic);
                let finalize_outcome = run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Finalize,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    Some(active_err),
                    loop_start,
                );
                if failure_outcome.evaluation_error.is_some()
                    || finalize_outcome.evaluation_error.is_some()
                {
                    let info = start_outcome
                        .action_error
                        .as_ref()
                        .unwrap_or(&synthetic);
                    // The original `start` failure was not an evaluation error;
                    // a catch event raised. Emit the surfaced evaluation error
                    // now — no further lifecycle events fire (Decision #2).
                    return Err(
                        crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                            CompositionError::catch_evaluation_error(
                                &prompt_state.source_path,
                                "start",
                                info,
                                Some(&failure_outcome),
                                Some(&finalize_outcome),
                            ),
                            term,
                        )
                        .into(),
                    );
                }
            } else {
                run_lifecycle_event(
                    lifecycle_guard,
                    LifecycleSignal::Finalize,
                    &materialized,
                    &prompt_state.source_path,
                    repo_root,
                    term,
                    &effect_engine,
                    start_outcome.action_error.as_ref(),
                    loop_start,
                );
            }
            return Err(eyre!("lifecycle start failed"));
        }

        // Pre-run snapshot capture for post-check comparisons has been
        // removed along with post-check validation.

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
            cli_stall_timeout.clone(),
        )
        .map_err(|e| {
            let err_info = LifecycleErrorInfo::from_action_failure("harness_launch", e.to_string());
            // A lifecycle evaluation error raised by the failure/finalize
            // stack takes precedence over the original harness-launch error —
            // the lifecycle raise is the more actionable diagnosis and must
            // halt the run.
            match emit_failure_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            ) {
                Some(ce) => ce.into(),
                None => eyre!("{e}"),
            }
        })?;
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
        // `execute_harness_attempt` can fail before spawning a child (e.g. a
        // malformed runaway `exit_expressions` regex in
        // `resolve_guard_inputs`/`compile_for_model`) or while delivering the
        // prompt. This is still post-`start`, so route through the typed
        // failure + finalize stacks (with `err`) before propagating.
        let (outcome, perf, iteration_signals) = attempt_result
        .map_err(|e| {
            let err_info =
                LifecycleErrorInfo::from_action_failure("harness_attempt", e.to_string());
            // A lifecycle evaluation error raised by the failure/finalize
            // stack takes precedence over the original harness-attempt error —
            // the lifecycle raise is the more actionable diagnosis and must
            // halt the run.
            match emit_failure_finalize_with_err(
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                &err_info,
                loop_start,
            ) {
                Some(ce) => ce.into(),
                None => eyre!("{e}"),
            }
        })?;
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
            let err_info = LifecycleErrorInfo::from_action_failure(
                "interrupted",
                "user interrupted the run",
            );
            let failure_outcome = execute_terminal_event(
                lifecycle_guard,
                LifecycleSignal::Failure,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
            )
            .outcome;
            // A late-binding evaluation error raised *by the failure stack*
            // halts the run: the helper runs `finalize` once with the
            // evaluation error as `err` and returns the typed catch error, so
            // do not also run a separate finalize when it returns `Some`.
            if let Some(err) = handle_terminal_evaluation_error(
                &failure_outcome,
                "failure",
                lifecycle_guard,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                loop_start,
            ) {
                return Err(err);
            }
            // `failure` did not raise: run `finalize` once, but a raise inside
            // `finalize` itself still halts the run non-zero rather than being
            // swallowed by the interrupt's `Ok` return.
            let finalize_outcome = run_lifecycle_event(
                lifecycle_guard,
                LifecycleSignal::Finalize,
                &materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                &effect_engine,
                Some(&err_info),
                loop_start,
            );
            if let Some(eval_info) = finalize_outcome.evaluation_error.as_ref() {
                return Err(
                    crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                        CompositionError::lifecycle_evaluation(
                            "finalize",
                            &prompt_state.source_path,
                            eval_info,
                        ),
                        term,
                    )
                    .into(),
                );
            }
            terminal_signals = iteration_signals;
            return Ok(LoopStep::Return((outcome.exit_code, harness_perf, terminal_signals)));
        }

        if claudine::harness::classify_failure(&outcome).is_some() {
            let message = claudine::harness::failure_message(&outcome, attempt);
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&message, term);
            }
            // Attribute the failure honestly: prefer the per-guard label the
            // outcome already carries (e.g. `step_timeout`, `runaway_repetition`)
            // so a `failure.stack` referencing `err.variant` branches correctly;
            // fall back to `agent_failure` when no structured label exists.
            let err_info = LifecycleErrorInfo::from_action_failure(
                outcome.error_kind.as_deref().unwrap_or("agent_failure"),
                message.as_str(),
            );
            match drive_terminal_recovery(
                lifecycle_guard,
                LifecycleSignal::Failure,
                Some(&err_info),
                &materialized,
                repo_root,
                term,
                &effect_engine,
                loop_start,
                attempt,
                &mut control_budgets,
                outcome.session_id.as_deref(),
                profile,
                provider,
                prompt_state,
                &mut proxy_tracking,
                show_checks,
            )? {
                TerminalRecovery::NextAttempt(next_attempt) => {
                    attempt = next_attempt;
                    continue;
                }
                TerminalRecovery::Completed => {}
            }
            // For provider-level failures, preserve the exit code at the
            // boundary rather than converting it into an `eyre` error. This
            // lets callers (e.g. `compose --loop`) inspect the terminal
            // attempt's iteration signals to build an honest
            // `LoopIterationFailed` cause.
            terminal_signals = iteration_signals;
            return Ok(LoopStep::Return((outcome.exit_code, harness_perf, terminal_signals)));
        }

        // For inline mode, apply closure after a successful provider run.
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
            let fail_msg = format!(
                "inline closure failed ({} {}): {}",
                failures.len(),
                if failures.len() == 1 { "failure" } else { "failures" },
                failures.join("; "),
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(&fail_msg, term);
            }
            let err_info =
                LifecycleErrorInfo::from_action_failure("inline_closure", fail_msg.as_str());
            match drive_terminal_recovery(
                lifecycle_guard,
                LifecycleSignal::Failure,
                Some(&err_info),
                &materialized,
                repo_root,
                term,
                &effect_engine,
                loop_start,
                attempt,
                &mut control_budgets,
                outcome.session_id.as_deref(),
                profile,
                provider,
                prompt_state,
                &mut proxy_tracking,
                show_checks,
            )? {
                TerminalRecovery::NextAttempt(next_attempt) => {
                    attempt = next_attempt;
                    continue;
                }
                TerminalRecovery::Completed => {}
            }
            return Err(eyre!("{fail_msg}"));
        }

        // A successful provider run proceeds to the success lifecycle event.
        // The `success.stack` may end in a flow-control action — either a direct
        // `resume`/`retry`/`proxy`/`requeue` (e.g. the agent finished but an
        // expected artifact is missing, so `resume` it), or an `error()` that
        // downgrades the run to failure (handled inside `execute_terminal_event`,
        // which then carries an `err` into `finalize`). Both surface as
        // `success.outcome.control`, so dispatch it uniformly.
        match drive_terminal_recovery(
            lifecycle_guard,
            LifecycleSignal::Success,
            None,
            &materialized,
            repo_root,
            term,
            &effect_engine,
            loop_start,
            attempt,
            &mut control_budgets,
            outcome.session_id.as_deref(),
            profile,
            provider,
            prompt_state,
            &mut proxy_tracking,
            show_checks,
        )? {
            TerminalRecovery::NextAttempt(next_attempt) => {
                attempt = next_attempt;
                continue;
            }
            TerminalRecovery::Completed => {}
        }
        terminal_signals = iteration_signals;
        return Ok(LoopStep::Return((outcome.exit_code, harness_perf, terminal_signals)));
    }
}


#[cfg(test)]
mod tests;
