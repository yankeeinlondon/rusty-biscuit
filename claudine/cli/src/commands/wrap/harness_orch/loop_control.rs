// `CompositionError` is intentionally large (it carries frontmatter excerpts);
// the whole composition/wrap execution path returns it and opts out of the
// `result_large_err` lint the same way (see `wrap/composition/target.rs`,
// `commands/compose/`, `commands/sequence.rs`).
#![allow(clippy::result_large_err)]

use biscuit_terminal::terminal::Terminal;
use claudine::composition::lifecycle::LifecycleSignal;
use claudine::composition::lifecycle_context::{LifecycleCurrent, LifecycleErrorInfo, LifecycleTiming};
use claudine::composition::lifecycle_control::{ControlDispatch, control_budget_for};
use claudine::composition::lifecycle_executor::{
    LifecycleEventOutcome, StackControl, StackExecutionContext, SystemShellRunner,
};
use claudine::composition::{
    CompositionError, DocumentTransition, EvaluatedProxyRequest, IterationSummarySignals,
    LifecycleCatchExecution, LifecycleCatchProtocol, LifecycleCatchState, ProxyProvenance,
    LifecycleTransitionAbort, LifecycleTransitionDecision, LifecycleTransitionError,
    LifecycleTransitionInput, RunLedger, decide_lifecycle_transition,
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
    initial_transition: DocumentTransition,
    emit_prompt_timing: bool,
}

mod control_dispatch;
mod coordinator;
mod error_routing;
mod lifecycle_events;
mod proxy;
mod requeue;

use control_dispatch::*;
use coordinator::ActiveDocumentCoordinator;
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
    // The transition an upstream `initialize` route already decided:
    // `Continue` for an ordinary run, or `Proxy` when the launched document's
    // `initialize` stack handed off. The coordinator commits it exactly as it
    // commits a proxy raised by a terminal event — one commit point, one set
    // of resolution and cycle semantics, no second channel.
    initial_transition: DocumentTransition,
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
        initial_transition,
        emit_prompt_timing,
    };
    match run_harness_loop_inner(ctx)? {
        LoopStep::Return(result) => Ok(result),
        LoopStep::Abort { reason, code } => Err(eyre!("{reason} (exit code {code})")),
        LoopStep::NextAttempt => unreachable!("the harness loop owns attempt re-entry"),
    }
}

struct HarnessLoopState<'a, 'guard> {
    run: HarnessLoopCtx<'a, 'guard>,
    effect_engine: EffectEngine,
    harness_context: CachedHarnessLoopContext,
    attempt: u32,
    initial_materialized: Option<MaterializedHarnessPrompt>,
    harness_perf: Option<crate::perf::AgentExecutionPerf>,
    loop_start: std::time::Instant,
    control_budgets: ControlBudgets,
    coordinator: ActiveDocumentCoordinator,
}

impl<'a, 'guard> HarnessLoopState<'a, 'guard> {
    /// Open the loop's state, committing any transition the upstream
    /// `initialize` route already decided.
    ///
    /// ## Errors
    ///
    /// Returns the typed commit failure when an upstream `initialize` proxy
    /// names a target that cannot be resolved or that the ledger refuses. The
    /// same commit runs for a proxy raised mid-run, so both fail identically.
    fn new(mut run: HarnessLoopCtx<'a, 'guard>) -> Result<Self> {
        let mutation_root = run.repo_root.unwrap_or(run.child_cwd).to_path_buf();
        let effect_engine = EffectEngine::builder()
            .mutation_root(&mutation_root)
            .auto_rehash(false)
            .build();
        let harness_context = CachedHarnessLoopContext::with_shell_options(
            &run.prompt_state.source_path,
            run.repo_root,
            run.shell_options.clone(),
        );
        // The chain originates at the document whose lifecycle the guard was
        // built for — the router, when an upstream `initialize` proxy is being
        // committed below — so a hop back to it is a cycle from the first hop.
        let mut coordinator = ActiveDocumentCoordinator::new(
            run.lifecycle_guard.context().source_path.to_path_buf(),
            run.shell_options.approval_cache.clone(),
        );
        if let DocumentTransition::Proxy(request) =
            std::mem::replace(&mut run.initial_transition, DocumentTransition::Continue)
        {
            coordinator.adopt(
                request,
                run.repo_root,
                run.prompt_state,
                run.lifecycle_guard,
            )?;
        }
        let initial_materialized = run.initial_materialized.take();
        Ok(Self {
            run,
            effect_engine,
            harness_context,
            attempt: 1,
            initial_materialized,
            harness_perf: None,
            loop_start: std::time::Instant::now(),
            control_budgets: ControlBudgets::default(),
            coordinator,
        })
    }
}

struct PreparedHarnessAttempt {
    materialized: MaterializedHarnessPrompt,
    plan: claudine::harness::HarnessPlan,
}

struct AttemptPromptPreparation<'a> {
    prompt_state: &'a mut HarnessPromptState,
    harness_context: &'a mut CachedHarnessLoopContext,
    initial_materialized: &'a mut Option<MaterializedHarnessPrompt>,
    child_cwd: &'a Path,
    repo_root: Option<&'a Path>,
    effective_non_interactive: bool,
    show_checks: bool,
    detail_requested: bool,
    silent: bool,
}

struct AttemptLifecycleExecution<'a, 'guard> {
    guard: &'a mut claudine::composition::LifecycleRunGuard<'guard>,
    effect_engine: &'a EffectEngine,
    term: &'a Terminal,
    loop_start: std::time::Instant,
}

struct AttemptRetryProxyControl<'a> {
    attempt: &'a mut u32,
    budgets: &'a mut ControlBudgets,
    coordinator: &'a mut ActiveDocumentCoordinator,
    provider: Provider,
    profile: &'a dyn crate::commands::wrap::profile::WrapperProfile,
}

struct ExecutedHarnessAttempt {
    materialized: MaterializedHarnessPrompt,
    outcome: claudine::harness::AttemptOutcome,
    iteration_signals: Option<IterationSummarySignals>,
}

enum PhaseResult<T> {
    Ready(T),
    Transition(Box<LoopStep>),
}

fn run_harness_loop_inner(ctx: HarnessLoopCtx<'_, '_>) -> Result<LoopStep> {
    let mut state = HarnessLoopState::new(ctx)?;
    loop {
        let prepared = match prepare_attempt_phase(&mut state)? {
            PhaseResult::Ready(prepared) => prepared,
            PhaseResult::Transition(step) if matches!(*step, LoopStep::NextAttempt) => continue,
            PhaseResult::Transition(step) => return Ok(*step),
        };
        let executed = execute_attempt_phase(&mut state, prepared)?;
        match classify_attempt_phase(&mut state, executed)? {
            LoopStep::NextAttempt => continue,
            step => return Ok(step),
        }
    }
}

#[allow(unused_assignments)]
fn prepare_attempt_phase(
    state: &mut HarnessLoopState<'_, '_>,
) -> Result<PhaseResult<PreparedHarnessAttempt>> {
    let mut prompt = AttemptPromptPreparation {
        prompt_state: state.run.prompt_state,
        harness_context: &mut state.harness_context,
        initial_materialized: &mut state.initial_materialized,
        child_cwd: state.run.child_cwd,
        repo_root: state.run.repo_root,
        effective_non_interactive: state.run.effective_non_interactive,
        show_checks: state.run.show_checks,
        detail_requested: state.run.detail_requested,
        silent: state.run.silent,
    };
    let mut lifecycle = AttemptLifecycleExecution {
        guard: state.run.lifecycle_guard,
        effect_engine: &state.effect_engine,
        term: state.run.term,
        loop_start: state.loop_start,
    };
    let mut control = AttemptRetryProxyControl {
        attempt: &mut state.attempt,
        budgets: &mut state.control_budgets,
        coordinator: &mut state.coordinator,
        provider: state.run.provider,
        profile: state.run.profile,
    };
    let _attempt_cycle_span = info_span!(
        "harness_attempt_cycle",
        provider = %control.provider,
        attempt = *control.attempt,
        prompt_mode = harness_prompt_mode_label(prompt.prompt_state.mode),
        source_path = %prompt.prompt_state.source_path.display(),
    )
    .entered();
    prompt
        .harness_context
        .refresh(&prompt.prompt_state.source_path, prompt.repo_root);
    preflight_pending_proxy_phase(&mut prompt, &mut lifecycle, &control)?;
    let mut materialized =
        materialize_attempt_prompt_phase(&mut prompt, &mut lifecycle, &control)?;

    if let Some(step) = bootstrap_adopted_document_phase(
        &mut prompt,
        &mut lifecycle,
        &mut control,
        &mut materialized,
    )? {
        return Ok(PhaseResult::Transition(Box::new(step)));
    }

    let plan = prepare_harness_plan_phase(
        &mut prompt,
        &mut lifecycle,
        &control,
        &materialized,
    )?;

    if let Some(step) = start_lifecycle_phase(
        &mut prompt,
        &mut lifecycle,
        &mut control,
        &materialized,
    )? {
        return Ok(PhaseResult::Transition(Box::new(step)));
    }

    Ok(PhaseResult::Ready(PreparedHarnessAttempt { materialized, plan }))
}

fn start_lifecycle_phase(
    prompt: &mut AttemptPromptPreparation<'_>,
    lifecycle: &mut AttemptLifecycleExecution<'_, '_>,
    control: &mut AttemptRetryProxyControl<'_>,
    materialized: &MaterializedHarnessPrompt,
) -> Result<Option<LoopStep>> {
    let attempt = &mut *control.attempt;
    let profile = control.profile;
    let provider = control.provider;
    let control_budgets = &mut *control.budgets;
    let coordinator = &mut *control.coordinator;
    let prompt_state = &mut *prompt.prompt_state;
    let repo_root = prompt.repo_root;
    let show_checks = prompt.show_checks;
    let lifecycle_guard = &mut *lifecycle.guard;
    let effect_engine = lifecycle.effect_engine;
    let term = lifecycle.term;
    let loop_start = lifecycle.loop_start;
    let start_outcome = run_lifecycle_event(
        lifecycle_guard,
        LifecycleSignal::Start,
        materialized,
        &prompt_state.source_path,
        repo_root,
        term,
        effect_engine,
        None,
        loop_start,
    );
    let start_early = start_outcome.evaluation_error.as_ref().map(|info| {
        crate::output::error_walker::emit_lifecycle_evaluation_error_early(
            &prompt_state.source_path,
            "start",
            info,
            term,
        )
    });
    let catch_result = run_catch_protocol(
        lifecycle_guard,
        LifecycleSignal::Start,
        start_outcome.clone(),
        materialized,
        &prompt_state.source_path,
        repo_root,
        term,
        effect_engine,
        None,
        loop_start,
    );
    if catch_result.evaluation_error_signal.is_some() {
        return Err(surface_protocol_evaluation(
            &catch_result,
            LifecycleSignal::Start,
            &prompt_state.source_path,
            start_early,
            term,
        ));
    }
    if let Some(setup_error) = catch_result.setup_error.as_ref() {
        let message = if matches!(
            catch_result.control,
            Some(StackControl::Error { .. })
        ) {
            setup_error.msg.clone()
        } else {
            "lifecycle start failed".to_string()
        };
        return Err(eyre!(message));
    }
    if let Some(ref control) = catch_result.control {
        match control {
            StackControl::Error { .. } => {
                unreachable!("the catch protocol consumes start error control")
            }
            StackControl::Stop => {}
            _ => match dispatch_terminal_control(
                &start_outcome,
                *attempt,
                control_budgets,
                None,
                profile,
                provider,
                prompt_state,
                materialized,
                lifecycle_guard,
                coordinator.ledger(),
                term,
                show_checks,
            ) {
                TerminalControlAction::Continue { next_attempt } => {
                    *attempt = next_attempt;
                    return Ok(Some(LoopStep::NextAttempt));
                }
                TerminalControlAction::Proxy(request) => {
                    coordinator.adopt(request, repo_root, prompt_state, lifecycle_guard)?;
                    // Re-enter at attempt 1: the target gets a clean
                    // pre-flight / freeze cycle rather than inheriting the
                    // proxying document's attempt count.
                    *attempt = 1;
                    return Ok(Some(LoopStep::NextAttempt));
                }
                TerminalControlAction::Abort(err) => {
                    let finalize_outcome = run_lifecycle_event(
                        lifecycle_guard,
                        LifecycleSignal::Finalize,
                        materialized,
                        &prompt_state.source_path,
                        repo_root,
                        term,
                        effect_engine,
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
    Ok(None)
}

/// Run a newly adopted document's staged canonical boot.
///
/// The staging exists because of an ordering conflict: `initialize` may mutate
/// the document, and the full audit has to read the document it will actually
/// execute. Auditing everything first and then letting `initialize` rewrite the
/// file underneath the audit is the drift; running `initialize` first with
/// nothing approved is a hole. So the boot splits:
///
/// 1. the **bootstrap read** — the document as adopted, composed with the
///    caller's input layers, giving the lifecycle surface `initialize` needs;
/// 2. the **narrow safety gate** — approve only the shell commands
///    `initialize` could select, against that same read;
/// 3. `initialize` itself, through the normal evaluator, consuming
///    `skip`/`error`/`proxy` atomically;
/// 4. the **stabilized reread** — a fresh read, so an initialize-time file or
///    frontmatter mutation is visible, with the caller's layers reapplied
///    through the same assembly point;
/// 5. the **full audit** over every lifecycle surface, which reuses the gate's
///    approvals from the invocation-wide cache rather than prompting twice.
///
/// `initialize` fires exactly once across all five stages: only step 3 emits
/// it, and the reread re-points the guard's config without touching its
/// emission ledger.
///
/// ## Errors
///
/// The boundary is stage 2/3: a failure before the target's lifecycle config
/// is installed has no target catch events to route to, and the source's were
/// discarded by the clean handoff, so it surfaces as its own typed diagnostic.
/// From stage 3 on, the target owns the run and a stabilized-reread or audit
/// failure routes through its ordinary `blocked`/`finalize` — once, because
/// each stage either routes or propagates, never both.
fn bootstrap_adopted_document_phase(
    prompt: &mut AttemptPromptPreparation<'_>,
    lifecycle: &mut AttemptLifecycleExecution<'_, '_>,
    control: &mut AttemptRetryProxyControl<'_>,
    materialized: &mut MaterializedHarnessPrompt,
) -> Result<Option<LoopStep>> {
    if !control.coordinator.take_bootstrap_pending() {
        return Ok(None);
    }

    // Stage 1 — the bootstrap read is `materialized`, already composed against
    // the caller's input layers by the shared assembly point. Its lifecycle
    // came from canonical preparation, so its shell commands are C3-resolved:
    // the bytes the gate approves are the bytes the executor runs.
    let bootstrap_lifecycle = match materialized.lifecycle.clone() {
        Some(config) => config,
        None => {
            return Err(eyre!(
                "adopted document '{}' was prepared without a lifecycle surface",
                prompt.prompt_state.source_path.display()
            ));
        }
    };

    // Stage 2 — the narrow safety gate. Every `initialize` shell command the
    // evaluator could select is approved before the evaluator runs. Later
    // events are deliberately out of scope: their commands may not survive the
    // stabilized reread.
    claudine::composition::resolve_lifecycle_shell_approvals(
        &bootstrap_lifecycle,
        &prompt.prompt_state.source_path,
        &[LifecycleSignal::Initialize],
        prompt.harness_context.shell_options(),
    )?;
    lifecycle.guard.set_config(bootstrap_lifecycle);

    // Stage 3 — `initialize`, through the normal evaluator.
    let chain = control.coordinator.ledger().chain().to_vec();
    match run_target_initialize(
        lifecycle.guard,
        materialized,
        &prompt.prompt_state.source_path,
        prompt.repo_root,
        &chain,
        lifecycle.term,
        lifecycle.effect_engine,
        lifecycle.loop_start,
    ) {
        TargetInitializeAction::Proceed => {}
        TargetInitializeAction::ExitCleanly => {
            return Ok(Some(LoopStep::Return((0, None, None))));
        }
        TargetInitializeAction::Abort(error) => return Err(error),
        TargetInitializeAction::Reproxy(request) => {
            // A chained proxy re-enters this same staging for its own target,
            // so the chain stabilizes one atomic hop at a time before anything
            // commits to a provider launch or loop execution.
            control.coordinator.adopt(
                request,
                prompt.repo_root,
                prompt.prompt_state,
                lifecycle.guard,
            )?;
            *control.attempt = 1;
            return Ok(Some(LoopStep::NextAttempt));
        }
    }

    // Stage 4 — the stabilized reread. `initialize` may have rewritten the
    // document or its frontmatter; the run executes what is on disk now.
    if let Err(error) = preflight_proxy_target(
        prompt.prompt_state,
        prompt.harness_context.shell_options(),
        prompt.child_cwd,
    ) {
        return Err(bootstrap_blocked(prompt, lifecycle, materialized, &error));
    }
    *materialized = materialize_attempt_prompt_phase(prompt, lifecycle, control)?;
    let stabilized_lifecycle = match materialized.lifecycle.clone() {
        Some(config) => config,
        None => {
            return Err(eyre!(
                "adopted document '{}' was prepared without a lifecycle surface",
                prompt.prompt_state.source_path.display()
            ));
        }
    };

    // Stage 5 — the full audit. `set_config` re-points the guard at the
    // stabilized surface without resetting its emission ledger, so
    // `initialize` is not fired a second time.
    if let Err(error) = claudine::composition::resolve_lifecycle_shell_approvals(
        &stabilized_lifecycle,
        &prompt.prompt_state.source_path,
        &LifecycleSignal::ALL,
        prompt.harness_context.shell_options(),
    ) {
        return Err(bootstrap_blocked(prompt, lifecycle, materialized, &error));
    }
    lifecycle.guard.set_config(stabilized_lifecycle);

    if prompt.effective_non_interactive {
        crate::output::log_compose_prompt(
            &materialized.prompt,
            prompt.detail_requested,
            prompt.silent,
            false,
            lifecycle.term,
        );
    }
    Ok(None)
}


/// Route a stabilized-stage boot failure through the target's own
/// `blocked`/`finalize` stacks.
///
/// Only called after stage 3 installed the target's lifecycle config: before
/// that there is nothing to catch with.
fn bootstrap_blocked(
    prompt: &mut AttemptPromptPreparation<'_>,
    lifecycle: &mut AttemptLifecycleExecution<'_, '_>,
    materialized: &MaterializedHarnessPrompt,
    error: &dyn std::fmt::Display,
) -> color_eyre::eyre::Report {
    let err_info = LifecycleErrorInfo::from_action_failure("shell_approval", error.to_string());
    emit_blocked_finalize_with_err(
        lifecycle.guard,
        materialized,
        &prompt.prompt_state.source_path,
        prompt.repo_root,
        lifecycle.term,
        lifecycle.effect_engine,
        &err_info,
        lifecycle.loop_start,
    )
    .map(color_eyre::eyre::Report::from)
    .unwrap_or_else(|| eyre!("{error}"))
}

fn empty_materialized_prompt() -> MaterializedHarnessPrompt {
    MaterializedHarnessPrompt {
        frontmatter: serde_json::Value::Null,
        prompt: String::new(),
        env_overrides: Vec::new(),
        inline_closure_plan: None,
        lifecycle: None,
        live_frontmatter: MaterializedHarnessPrompt::live_cell_from(&serde_json::Value::Null),
    }
}

fn preflight_pending_proxy_phase(
    prompt: &mut AttemptPromptPreparation<'_>,
    lifecycle: &mut AttemptLifecycleExecution<'_, '_>,
    control: &AttemptRetryProxyControl<'_>,
) -> Result<()> {
    let attempt = *control.attempt;
    let proxy_pending = control.coordinator.bootstrap_pending();
    let has_seed = prompt.initial_materialized.is_some();
    let silent = prompt.silent;
    let prompt_state = &mut *prompt.prompt_state;
    let harness_context = &mut *prompt.harness_context;
    let child_cwd = prompt.child_cwd;
    let repo_root = prompt.repo_root;
    let lifecycle_guard = &mut *lifecycle.guard;
    let effect_engine = lifecycle.effect_engine;
    let term = lifecycle.term;
    let loop_start = lifecycle.loop_start;
    if proxy_pending && !silent {
        claudine::harness::report::report_proxy_handoff(&prompt_state.source_path, term);
    }
    if !proxy_pending || has_seed {
        return Ok(());
    }
    let result = info_span!(
        "harness_proxy_target_preflight",
        attempt,
        source_path = %prompt_state.source_path.display(),
    )
    .in_scope(|| {
        preflight_proxy_target(prompt_state, harness_context.shell_options(), child_cwd)
    });
    if let Err(error) = result {
        let err_info =
            LifecycleErrorInfo::from_action_failure("shell_approval", error.to_string());
        let empty = empty_materialized_prompt();
        return Err(emit_blocked_finalize_with_err(
            lifecycle_guard,
            &empty,
            &prompt_state.source_path,
            repo_root,
            term,
            effect_engine,
            &err_info,
            loop_start,
        )
        .map(color_eyre::eyre::Report::from)
        .unwrap_or_else(|| eyre!("{error}")));
    }
    Ok(())
}

fn materialize_attempt_prompt_phase(
    prompt: &mut AttemptPromptPreparation<'_>,
    lifecycle: &mut AttemptLifecycleExecution<'_, '_>,
    control: &AttemptRetryProxyControl<'_>,
) -> Result<MaterializedHarnessPrompt> {
    let attempt = *control.attempt;
    let initial_materialized = &mut *prompt.initial_materialized;
    let prompt_state = &mut *prompt.prompt_state;
    let child_cwd = prompt.child_cwd;
    let repo_root = prompt.repo_root;
    let lifecycle_guard = &mut *lifecycle.guard;
    let effect_engine = lifecycle.effect_engine;
    let term = lifecycle.term;
    let loop_start = lifecycle.loop_start;
    if let Some(seed) = initial_materialized.take() {
        return Ok(seed);
    }
    info_span!(
        "harness_materialize_prompt",
        attempt,
        source_path = %prompt_state.source_path.display(),
    )
    .in_scope(|| materialize_harness_prompt(prompt_state, repo_root, child_cwd))
    .map_err(|error| {
        // Canonical preparation returns concrete `CompositionError`s (a target's
        // malformed lifecycle stack, a schema failure, a shell denial). Keep the
        // typed identity rather than flattening it to a generic "materialize"
        // action failure: a target's parse error must present the same
        // `err.category`/`err.code` whether the target was invoked directly or
        // proxied to.
        let err_info = match error.downcast_ref::<claudine::composition::CompositionError>() {
            Some(composition) => LifecycleErrorInfo::from_composition_error(composition),
            None => LifecycleErrorInfo::from_action_failure("materialize", error.to_string()),
        };
        let empty = empty_materialized_prompt();
        match emit_blocked_finalize_with_err(
            lifecycle_guard,
            &empty,
            &prompt_state.source_path,
            repo_root,
            term,
            effect_engine,
            &err_info,
            loop_start,
        ) {
            Some(lifecycle_error) => lifecycle_error.into(),
            None => error,
        }
    })
}

fn prepare_harness_plan_phase(
    prompt: &mut AttemptPromptPreparation<'_>,
    lifecycle: &mut AttemptLifecycleExecution<'_, '_>,
    control: &AttemptRetryProxyControl<'_>,
    materialized: &MaterializedHarnessPrompt,
) -> Result<claudine::harness::HarnessPlan> {
    let attempt = *control.attempt;
    let show_checks = prompt.show_checks;
    let prompt_state = &*prompt.prompt_state;
    let harness_context = &mut *prompt.harness_context;
    let repo_root = prompt.repo_root;
    let lifecycle_guard = &mut *lifecycle.guard;
    let effect_engine = lifecycle.effect_engine;
    let term = lifecycle.term;
    let loop_start = lifecycle.loop_start;
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
    .map_err(|error| {
        let err_info = LifecycleErrorInfo::from_harness_error(&error);
        emit_blocked_finalize_with_err(
            lifecycle_guard,
            materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            effect_engine,
            &err_info,
            loop_start,
        )
        .map(color_eyre::eyre::Report::from)
        .unwrap_or_else(|| eyre!("{error}"))
    })?;

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
        let message = format!(
            "source file does not exist: {}",
            prompt_state.source_path.display()
        );
        let err_info = LifecycleErrorInfo::from_action_failure("missing_source", &message);
        if let Some(error) = emit_blocked_finalize_with_err(
            lifecycle_guard,
            materialized,
            &prompt_state.source_path,
            repo_root,
            term,
            effect_engine,
            &err_info,
            loop_start,
        ) {
            return Err(error.into());
        }
        return Err(eyre!(message));
    }

    if matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
        let source_text = std::fs::read_to_string(&prompt_state.source_path).ok();
        let auditable = claudine::harness::collect_auditable_commands(source_text.as_deref())?;
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
            let message = format!(
                "shell audit failed: {} denied directive(s) in source page",
                audit_report.failures().len()
            );
            if show_checks {
                claudine::harness::report::report_unhandled_failure(
                    "shell audit failed for source-page directives — cannot proceed",
                    term,
                );
            }
            let err_info = LifecycleErrorInfo::from_action_failure("shell_audit", &message);
            if let Some(error) = emit_blocked_finalize_with_err(
                lifecycle_guard,
                materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                effect_engine,
                &err_info,
                loop_start,
            ) {
                return Err(error.into());
            }
            return Err(eyre!(message));
        }
    }
    if attempt == 1 && !matches!(prompt_state.mode, HarnessPromptMode::Passthrough) {
        harness_context.freeze_shell_approvals();
    }
    Ok(plan)
}

fn execute_attempt_phase(
    state: &mut HarnessLoopState<'_, '_>,
    prepared: PreparedHarnessAttempt,
) -> Result<ExecutedHarnessAttempt> {
    let provider = state.run.provider;
    let profile = state.run.profile;
    let binary_path = state.run.binary_path;
    let child_cwd = state.run.child_cwd;
    let effective_non_interactive = state.run.effective_non_interactive;
    let cli_timeout = &state.run.cli_timeout;
    let cli_step_timeout = &state.run.cli_step_timeout;
    let cli_stall_timeout = &state.run.cli_stall_timeout;
    let base_args = state.run.base_args;
    let base_env = state.run.base_env;
    let use_structured = state.run.use_structured;
    let structured_codex_output = state.run.structured_codex_output;
    let stdout_noise = state.run.stdout_noise;
    let stderr_noise = state.run.stderr_noise;
    let suppress_stderr_on_success = state.run.suppress_stderr_on_success;
    let show_checks = state.run.show_checks;
    let stream_verbosity = state.run.stream_verbosity;
    let detail_requested = state.run.detail_requested;
    let env_context = state.run.env_context;
    let dispatch_context = state.run.dispatch_context;
    let repo_root = state.run.repo_root;
    let term = state.run.term;
    let emit_prompt_timing = state.run.emit_prompt_timing;
    let prompt_state = &mut *state.run.prompt_state;
    let lifecycle_guard = &mut *state.run.lifecycle_guard;
    let effect_engine = &state.effect_engine;
    let attempt = state.attempt;
    let loop_start = state.loop_start;
    let PreparedHarnessAttempt { materialized, plan } = prepared;
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
            effect_engine,
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
            effect_engine,
            &err_info,
            loop_start,
        ) {
            Some(ce) => ce.into(),
            None => eyre!("{e}"),
        }
    })?;
    if let Some(p) = perf {
        match state.harness_perf.as_mut() {
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
                state.harness_perf = Some(p);
            }
        }
    }


    Ok(ExecutedHarnessAttempt { materialized, outcome, iteration_signals })
}

fn classify_attempt_phase(
    state: &mut HarnessLoopState<'_, '_>,
    executed: ExecutedHarnessAttempt,
) -> Result<LoopStep> {
    let provider = state.run.provider;
    let profile = state.run.profile;
    let child_cwd = state.run.child_cwd;
    let repo_root = state.run.repo_root;
    let show_checks = state.run.show_checks;
    let term = state.run.term;
    let prompt_state = &mut *state.run.prompt_state;
    let lifecycle_guard = &mut *state.run.lifecycle_guard;
    let effect_engine = &state.effect_engine;
    let control_budgets = &mut state.control_budgets;
    let coordinator = &mut state.coordinator;
    let attempt = state.attempt;
    let loop_start = state.loop_start;
    let ExecutedHarnessAttempt { materialized, outcome, iteration_signals } = executed;
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
            effect_engine,
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
            effect_engine,
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
            effect_engine,
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
        return Ok(LoopStep::Return((
            outcome.exit_code,
            state.harness_perf.take(),
            iteration_signals,
        )));
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
            effect_engine,
            loop_start,
            attempt,
            control_budgets,
            outcome.session_id.as_deref(),
            profile,
            provider,
            prompt_state,
            coordinator.ledger(),
            show_checks,
        )? {
            TerminalRecovery::NextAttempt(next_attempt) => {
                state.attempt = next_attempt;
                return Ok(LoopStep::NextAttempt);
            }
            TerminalRecovery::Proxy(request) => {
                coordinator.adopt(request, repo_root, prompt_state, lifecycle_guard)?;
                state.attempt = 1;
                return Ok(LoopStep::NextAttempt);
            }
            TerminalRecovery::Completed => {}
        }
        // For provider-level failures, preserve the exit code at the
        // boundary rather than converting it into an `eyre` error. This
        // lets callers (e.g. `compose --loop`) inspect the terminal
        // attempt's iteration signals to build an honest
        // `LoopIterationFailed` cause.
        return Ok(LoopStep::Return((
            outcome.exit_code,
            state.harness_perf.take(),
            iteration_signals,
        )));
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
            effect_engine,
            loop_start,
            attempt,
            control_budgets,
            outcome.session_id.as_deref(),
            profile,
            provider,
            prompt_state,
            coordinator.ledger(),
            show_checks,
        )? {
            TerminalRecovery::NextAttempt(next_attempt) => {
                state.attempt = next_attempt;
                return Ok(LoopStep::NextAttempt);
            }
            TerminalRecovery::Proxy(request) => {
                coordinator.adopt(request, repo_root, prompt_state, lifecycle_guard)?;
                state.attempt = 1;
                return Ok(LoopStep::NextAttempt);
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
        effect_engine,
        loop_start,
        attempt,
        control_budgets,
        outcome.session_id.as_deref(),
        profile,
        provider,
        prompt_state,
        coordinator.ledger(),
        show_checks,
    )? {
        TerminalRecovery::NextAttempt(next_attempt) => {
            state.attempt = next_attempt;
            return Ok(LoopStep::NextAttempt);
        }
        TerminalRecovery::Proxy(request) => {
            coordinator.adopt(request, repo_root, prompt_state, lifecycle_guard)?;
            state.attempt = 1;
            return Ok(LoopStep::NextAttempt);
        }
        TerminalRecovery::Completed => {}
    }
    Ok(LoopStep::Return((
        outcome.exit_code,
        state.harness_perf.take(),
        iteration_signals,
    )))

}


#[cfg(test)]
mod tests;
