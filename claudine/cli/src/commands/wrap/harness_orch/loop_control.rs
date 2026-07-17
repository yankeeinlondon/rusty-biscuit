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
    LifecycleTransitionInput, RunLedger, SharedRunLedger, SurfacedHandoff,
    commit_proxy, decide_lifecycle_transition,
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
    preflight_proxy_target, session_compat_key, HarnessPromptMode,
};

type HarnessLoopResult = (
    i32,
    Option<crate::perf::AgentExecutionPerf>,
    Option<IterationSummarySignals>,
    // A proxy handoff the harness surfaced (compose path): a terminal-recovery /
    // start-stack proxy the harness committed against the shared invocation
    // ledger, for the command coordinator to re-prepare through the full
    // canonical launch pipeline. `None` on an ordinary run and on the
    // sequence-contained path (which adopts in place instead).
    Option<SurfacedHandoff>,
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
    // Immutable invocation-level launch intent, retained so the R6 target launch
    // rebuild can re-resolve a proxied target's model while keeping explicit CLI
    // intent authoritative. `cli_model` is `--model` (wins over any target
    // frontmatter `model:`); `yolo` is the invocation's bypass intent.
    cli_model: Option<String>,
    yolo: bool,
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
    // The one invocation-wide ledger, shared with the command coordinator, when
    // this run has one (compose / inline-compose). A terminal-event proxy commits
    // against it and surfaces the committed handoff up rather than adopting in
    // place. `None` for a sequence step (its proxy stays contained) and for the
    // direct wrapper passthrough.
    handoff_ledger: Option<SharedRunLedger>,
    emit_prompt_timing: bool,
}

mod control_dispatch;
mod coordinator;
mod error_routing;
mod lifecycle_events;
mod proxy;
mod requeue;
mod target_launch;

use control_dispatch::*;
use coordinator::ActiveDocumentCoordinator;
use error_routing::*;
use lifecycle_events::*;
use proxy::*;
use target_launch::rebuild_target_launch;
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
    cli_model: Option<String>,
    yolo: bool,
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
    // The shared invocation ledger (compose/inline-compose), or `None` for a
    // sequence step / direct passthrough. See [`HarnessLoopCtx::handoff_ledger`].
    handoff_ledger: Option<SharedRunLedger>,
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
        cli_model,
        yolo,
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
        handoff_ledger,
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
    initial_materialized: Option<MaterializedHarnessPrompt>,
    harness_perf: Option<crate::perf::AgentExecutionPerf>,
    loop_start: std::time::Instant,
    /// The one owner of the active document's mutable execution state — the
    /// provider-attempt slice (number, live session, resume follow-up) and the
    /// enclosing iteration's retry/resume budgets. There is no parallel copy:
    /// the attempt number, the ceilings, and the resume session all read from
    /// here. A proxy discards it wholesale (see [`ActiveDocumentCoordinator::adopt`]).
    active: claudine::composition::ActiveDocumentState,
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
        // Established before the initial commit rather than in the struct
        // literal below: an upstream `initialize` proxy is committed here, and
        // `adopt` discards the active-document state of the document it replaces.
        let mut active = claudine::composition::ActiveDocumentState::initial();
        if let DocumentTransition::Proxy(request) =
            std::mem::replace(&mut run.initial_transition, DocumentTransition::Continue)
        {
            coordinator.adopt(
                request,
                run.repo_root,
                run.prompt_state,
                run.lifecycle_guard,
                &mut active,
            )?;
        }
        let initial_materialized = run.initial_materialized.take();
        Ok(Self {
            run,
            effect_engine,
            harness_context,
            initial_materialized,
            harness_perf: None,
            loop_start: std::time::Instant::now(),
            active,
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
    active: &'a mut claudine::composition::ActiveDocumentState,
    coordinator: &'a mut ActiveDocumentCoordinator,
    provider: Provider,
    profile: &'a dyn crate::commands::wrap::profile::WrapperProfile,
    /// Immutable invocation launch intent, read by the R6 target launch rebuild
    /// when a proxied target's launch identity is recomputed. Explicit `--model`
    /// stays authoritative over any target frontmatter `model:`.
    cli_model: Option<&'a str>,
    yolo: bool,
    /// The shared invocation ledger (compose path) a terminal-event proxy commits
    /// against before surfacing up, or `None` for the sequence-contained path
    /// (which adopts in place on the harness-local coordinator).
    handoff_ledger: Option<&'a SharedRunLedger>,
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
        active: &mut state.active,
        coordinator: &mut state.coordinator,
        provider: state.run.provider,
        profile: state.run.profile,
        cli_model: state.run.cli_model.as_deref(),
        yolo: state.run.yolo,
        handoff_ledger: state.run.handoff_ledger.as_ref(),
    };
    let _attempt_cycle_span = info_span!(
        "harness_attempt_cycle",
        provider = %control.provider,
        attempt = control.active.iteration().attempt().number(),
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
    let attempt = control.active.iteration().attempt().number();
    let profile = control.profile;
    let provider = control.provider;
    let handoff_ledger = control.handoff_ledger;
    let active = &mut *control.active;
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
            _ => {
                // Provenance and the hop/cycle check use the shared invocation
                // ledger on the compose path (so a hop back to an invocation
                // ancestor is checked against the complete chain), and the
                // harness-local coordinator ledger on the sequence path. The lock
                // guard is dropped before the surface/adopt commit re-locks.
                let dispatch = {
                    let shared_guard = handoff_ledger.map(|l| l.lock().unwrap());
                    let ledger_ref: &RunLedger =
                        shared_guard.as_deref().unwrap_or_else(|| coordinator.ledger());
                    dispatch_terminal_control(
                        &start_outcome,
                        attempt,
                        active.iteration_mut(),
                        None,
                        profile,
                        provider,
                        prompt_state,
                        materialized,
                        lifecycle_guard,
                        ledger_ref,
                        term,
                        show_checks,
                    )
                };
                match dispatch {
                TerminalControlAction::Continue => {
                    // The attempt slice was advanced on `active` by the dispatch;
                    // the next iteration reads its number from there.
                    return Ok(Some(LoopStep::NextAttempt));
                }
                TerminalControlAction::Proxy(request) => {
                    // Compose: commit against the shared ledger and surface up so
                    // the command coordinator re-prepares the target through the
                    // full canonical launch pipeline. Sequence: adopt in place. No
                    // attempt has run at `start`, so no perf/signals ride along.
                    return Ok(Some(surface_or_adopt_terminal_proxy(
                        handoff_ledger,
                        coordinator,
                        request,
                        repo_root,
                        prompt_state,
                        lifecycle_guard,
                        active,
                        materialized,
                        term,
                        effect_engine,
                        loop_start,
                        None,
                        None,
                    )?));
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
                }
            }
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
            return Err(target_bootstrap_failed(
                prompt,
                control.coordinator,
                "canonical preparation returned no lifecycle surface for the bootstrap read",
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
            return Ok(Some(LoopStep::Return((0, None, None, None))));
        }
        TargetInitializeAction::Abort(error) => return Err(error),
        TargetInitializeAction::Reproxy(request) => {
            // A chained proxy re-enters this same staging for its own target,
            // so the chain stabilizes one atomic hop at a time before anything
            // commits to a provider launch or loop execution.
            if let Err(error) = control.coordinator.adopt(
                request,
                prompt.repo_root,
                prompt.prompt_state,
                lifecycle.guard,
                control.active,
            ) {
                return Err(route_handoff_failure(
                    lifecycle.guard,
                    materialized,
                    &prompt.prompt_state.source_path,
                    prompt.repo_root,
                    lifecycle.term,
                    lifecycle.effect_engine,
                    error,
                    lifecycle.loop_start,
                ));
            }
            // `adopt` reset `active` to attempt 1 for the chained target.
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
            return Err(target_bootstrap_failed(
                prompt,
                control.coordinator,
                "canonical preparation returned no lifecycle surface for the stabilized reread",
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

    // R6 — rebuild the target's launch identity from its own frontmatter now that
    // its document identity has stabilized. Recomputes provider/model (honoring
    // explicit CLI precedence) into the `AGENT`/`MODEL`/`YOLO` env, then installs
    // it on both consumers: the target's lifecycle early-binding context (so its
    // stacks resolve `env.MODEL`/`ctx.model` to the target's own identity) and
    // the child environment (via `materialized.env_overrides`, retained on the
    // coordinator so every subsequent attempt carries it too). Without this a
    // proxied target keeps the router's launch state — a target pinned to a
    // different model would otherwise launch with the router's empty model.
    let launch_area = lifecycle
        .guard
        .context()
        .launch_area
        .map(Path::to_path_buf)
        .or_else(|| prompt.repo_root.map(Path::to_path_buf))
        .unwrap_or_else(|| prompt.child_cwd.to_path_buf());
    let rebuild = rebuild_target_launch(
        control.provider,
        control.cli_model,
        control.yolo,
        prompt.repo_root,
        &launch_area,
        materialized,
    );
    apply_target_env_overrides(materialized, &rebuild.env_overrides);
    control
        .coordinator
        .set_target_env_overrides(rebuild.env_overrides);
    lifecycle
        .guard
        .set_proxy_prepared_context(rebuild.prepared_context);

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

/// Resolve a terminal-event proxy request either by surfacing it up to the
/// command coordinator (compose path) or by adopting it in place (sequence path).
///
/// This is the one place a terminal-recovery / `start`-stack proxy is turned
/// into a loop step, so the two ownership models converge here:
///
/// - **Compose** (`handoff_ledger` is `Some`): the harness commits the request
///   against the *one* invocation-wide ledger while the source document's stacks
///   are still live to catch a refused hop, then surfaces the committed handoff
///   up as a [`LoopStep::Return`] carrying it. The harness never repoints its own
///   active document; the command coordinator re-prepares the resolved target
///   through the full canonical launch pipeline — the same rebuild a direct
///   invocation performs (R6).
/// - **Sequence** (`handoff_ledger` is `None`): the proxy stays contained within
///   the current step and adopts in place on the harness-local coordinator (R1).
///
/// A refused hop routes through the source's `blocked`/`finalize` with the typed
/// commit error in both models, since the source is still the active document.
#[allow(clippy::too_many_arguments)]
fn surface_or_adopt_terminal_proxy(
    handoff_ledger: Option<&SharedRunLedger>,
    coordinator: &mut ActiveDocumentCoordinator,
    request: EvaluatedProxyRequest,
    repo_root: Option<&Path>,
    prompt_state: &mut HarnessPromptState,
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    active: &mut claudine::composition::ActiveDocumentState,
    materialized: &MaterializedHarnessPrompt,
    term: &Terminal,
    effect_engine: &EffectEngine,
    loop_start: std::time::Instant,
    perf: Option<crate::perf::AgentExecutionPerf>,
    iteration_signals: Option<IterationSummarySignals>,
) -> Result<LoopStep> {
    match handoff_ledger {
        Some(shared) => match commit_proxy(&mut shared.lock().unwrap(), request, repo_root) {
            Ok(handoff) => Ok(LoopStep::Return((
                0,
                perf,
                iteration_signals,
                Some(SurfacedHandoff::Committed(Box::new(handoff))),
            ))),
            Err(error) => Err(route_handoff_failure(
                lifecycle_guard,
                materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                effect_engine,
                error,
                loop_start,
            )),
        },
        None => match coordinator.adopt(request, repo_root, prompt_state, lifecycle_guard, active) {
            Ok(()) => Ok(LoopStep::NextAttempt),
            Err(error) => Err(route_handoff_failure(
                lifecycle_guard,
                materialized,
                &prompt_state.source_path,
                repo_root,
                term,
                effect_engine,
                error,
                loop_start,
            )),
        },
    }
}

/// Overlay the rebuilt target launch-identity env onto a materialized prompt's
/// `env_overrides`, replacing any prior entry for the same key so a re-applied
/// rebuild is idempotent.
fn apply_target_env_overrides(
    materialized: &mut MaterializedHarnessPrompt,
    overrides: &[(String, String)],
) {
    for (key, value) in overrides {
        materialized.env_overrides.retain(|(k, _)| k != key);
        materialized
            .env_overrides
            .push((key.clone(), value.clone()));
    }
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
        selection_hints: claudine::composition::EffectiveSelectionHints::default(),
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
    let attempt = control.active.iteration().attempt().number();
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
    let attempt = control.active.iteration().attempt().number();
    // The resume follow-up, when this attempt is a resume, is the provider
    // input the model recorded on the attempt slice — it overrides the composed
    // prompt for exactly this attempt.
    let resume_followup = control
        .active
        .iteration()
        .attempt()
        .resume_followup()
        .map(str::to_string);
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
    // Re-apply the active target's rebuilt launch identity (R6). A directly
    // invoked document has none (empty), so this is a no-op there; a proxied
    // target's every attempt — including later loop iterations, which
    // re-materialize from disk — carries the target's own `AGENT`/`MODEL`/`YOLO`
    // in its child environment.
    let target_env_overrides = control.coordinator.target_env_overrides().to_vec();
    info_span!(
        "harness_materialize_prompt",
        attempt,
        source_path = %prompt_state.source_path.display(),
    )
    .in_scope(|| {
        materialize_harness_prompt(prompt_state, repo_root, child_cwd, resume_followup.as_deref())
    })
    .map(|mut materialized| {
        apply_target_env_overrides(&mut materialized, &target_env_overrides);
        materialized
    })
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
    let attempt = control.active.iteration().attempt().number();
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
    let attempt = state.active.iteration().attempt().number();
    // The live session to resume, when this attempt is a resume. Read from the
    // single active-document owner rather than a parallel prompt-state field.
    let resume_session = state
        .active
        .iteration()
        .attempt()
        .session_id()
        .map(str::to_string);
    let prompt_state = &mut *state.run.prompt_state;
    let lifecycle_guard = &mut *state.run.lifecycle_guard;
    let effect_engine = &state.effect_engine;
    let loop_start = state.loop_start;
    let PreparedHarnessAttempt { materialized, plan } = prepared;

    // R8 — compute the session-compatibility key for the launch this attempt is
    // about to run, from the resolved launch state the harness will execute
    // (provider, effective child env, resolved argv, working directory, mode).
    let launch_key = session_compat_key(
        provider,
        profile,
        binary_path,
        child_cwd,
        state.run.yolo,
        effective_non_interactive,
        use_structured,
        structured_codex_output.is_some(),
        base_args,
        base_env,
        &materialized,
    );
    // A resume carries the key of the session-producing attempt forward with the
    // live session. If the canonical refresh changed a launch property the
    // provider fixed when it opened the session, refuse the resume with a typed
    // diagnostic before launching the provider under the stale session — never
    // mix a live session with a newly prepared launch plan.
    if resume_session.is_some()
        && let Some(prior) = state.active.iteration().attempt().compat_key()
    {
        let facets = prior.incompatibilities(&launch_key);
        if !facets.is_empty() {
            return Err(CompositionError::LifecycleResumeIncompatible {
                source_path: prompt_state.source_path.clone(),
                facets,
            }
            .into());
        }
    }
    // Record the key of the launch this attempt runs with, so a subsequent
    // resume can compare its refreshed plan against it.
    state
        .active
        .iteration_mut()
        .attempt_mut()
        .set_compat_key(launch_key);

    let launch = build_harness_launch(
        provider,
        profile,
        base_args,
        base_env,
        resume_session.as_deref(),
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
    let handoff_ledger = state.run.handoff_ledger.clone();
    let attempt = state.active.iteration().attempt().number();
    let harness_perf = &mut state.harness_perf;
    let prompt_state = &mut *state.run.prompt_state;
    let lifecycle_guard = &mut *state.run.lifecycle_guard;
    let effect_engine = &state.effect_engine;
    let active = &mut state.active;
    let coordinator = &mut state.coordinator;
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
            harness_perf.take(),
            iteration_signals,
            None,
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
        let recovery = {
            let shared_guard = handoff_ledger.as_ref().map(|l| l.lock().unwrap());
            let ledger_ref: &RunLedger =
                shared_guard.as_deref().unwrap_or_else(|| coordinator.ledger());
            drive_terminal_recovery(
                lifecycle_guard,
                LifecycleSignal::Failure,
                Some(&err_info),
                &materialized,
                repo_root,
                term,
                effect_engine,
                loop_start,
                attempt,
                active.iteration_mut(),
                outcome.session_id.as_deref(),
                profile,
                provider,
                prompt_state,
                ledger_ref,
                show_checks,
            )?
        };
        match recovery {
            TerminalRecovery::NextAttempt => {
                // The attempt slice was advanced on `active` by the dispatch.
                return Ok(LoopStep::NextAttempt);
            }
            TerminalRecovery::Proxy(request) => {
                return surface_or_adopt_terminal_proxy(
                    handoff_ledger.as_ref(),
                    coordinator,
                    request,
                    repo_root,
                    prompt_state,
                    lifecycle_guard,
                    active,
                    &materialized,
                    term,
                    effect_engine,
                    loop_start,
                    harness_perf.take(),
                    iteration_signals,
                );
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
            harness_perf.take(),
            iteration_signals,
            None,
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
        let recovery = {
            let shared_guard = handoff_ledger.as_ref().map(|l| l.lock().unwrap());
            let ledger_ref: &RunLedger =
                shared_guard.as_deref().unwrap_or_else(|| coordinator.ledger());
            drive_terminal_recovery(
                lifecycle_guard,
                LifecycleSignal::Failure,
                Some(&err_info),
                &materialized,
                repo_root,
                term,
                effect_engine,
                loop_start,
                attempt,
                active.iteration_mut(),
                outcome.session_id.as_deref(),
                profile,
                provider,
                prompt_state,
                ledger_ref,
                show_checks,
            )?
        };
        match recovery {
            TerminalRecovery::NextAttempt => {
                // The attempt slice was advanced on `active` by the dispatch.
                return Ok(LoopStep::NextAttempt);
            }
            TerminalRecovery::Proxy(request) => {
                return surface_or_adopt_terminal_proxy(
                    handoff_ledger.as_ref(),
                    coordinator,
                    request,
                    repo_root,
                    prompt_state,
                    lifecycle_guard,
                    active,
                    &materialized,
                    term,
                    effect_engine,
                    loop_start,
                    harness_perf.take(),
                    iteration_signals,
                );
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
    let recovery = {
        let shared_guard = handoff_ledger.as_ref().map(|l| l.lock().unwrap());
        let ledger_ref: &RunLedger =
            shared_guard.as_deref().unwrap_or_else(|| coordinator.ledger());
        drive_terminal_recovery(
            lifecycle_guard,
            LifecycleSignal::Success,
            None,
            &materialized,
            repo_root,
            term,
            effect_engine,
            loop_start,
            attempt,
            active.iteration_mut(),
            outcome.session_id.as_deref(),
            profile,
            provider,
            prompt_state,
            ledger_ref,
            show_checks,
        )?
    };
    match recovery {
        TerminalRecovery::NextAttempt => {
            // The attempt slice was advanced on `active` by the dispatch.
            return Ok(LoopStep::NextAttempt);
        }
        TerminalRecovery::Proxy(request) => {
            return surface_or_adopt_terminal_proxy(
                handoff_ledger.as_ref(),
                coordinator,
                request,
                repo_root,
                prompt_state,
                lifecycle_guard,
                active,
                &materialized,
                term,
                effect_engine,
                loop_start,
                harness_perf.take(),
                iteration_signals,
            );
        }
        TerminalRecovery::Completed => {}
    }
    Ok(LoopStep::Return((
        outcome.exit_code,
        harness_perf.take(),
        iteration_signals,
        None,
    )))
}


#[cfg(test)]
mod tests;
