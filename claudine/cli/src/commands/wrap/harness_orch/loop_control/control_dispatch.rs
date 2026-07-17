//! Terminal-control dispatch: translating a lifecycle stack's control action
//! into a loop action (retry/resume/proxy budgets included) and the shared
//! terminal-recovery sequence every terminal event runs through.

use super::*;

/// Per-control retry/resume budget tracking for one `run_harness_loop` call.
///
/// A lifecycle `retry`/`resume` control declares `max_attempts` relative to
/// the attempt at which it first fires. The budget (the absolute attempt
/// ceiling) is computed once on first firing and reused so the ceiling does
/// not drift as the attempt counter advances.
#[derive(Default)]
pub(super) struct ControlBudgets {
    pub(super) retry: Option<u32>,
    pub(super) resume: Option<u32>,
}

impl ControlBudgets {
    /// Return (and lazily establish) the budget for a control firing at
    /// `attempt`. `max_attempts` is the additional-attempts parameter.
    fn budget_for(slot: &mut Option<u32>, attempt: u32, max_attempts: u32) -> u32 {
        *slot.get_or_insert_with(|| control_budget_for(attempt, max_attempts))
    }
}

/// What the loop should do after dispatching a terminal-event control.
#[derive(Debug)]
pub(super) enum TerminalControlAction {
    /// No actionable control (Stop/Skip/None) — fall through to the
    /// loop's normal terminal handling (finalize + return).
    Fallthrough,
    /// Re-enter the loop for another attempt at `next_attempt`.
    Continue { next_attempt: u32 },
    /// A control could not be honored; abort the run with this error.
    Abort(color_eyre::eyre::Report),
}

/// Translate **any** lifecycle event's stack [`StackControl`] into a loop
/// action, applying the retry/resume/proxy/requeue runtime effect.
///
/// This is the single, **event-agnostic** handler-dispatch path shared by every
/// event whose stack can carry a recovery handler (`blocked`/`failure`/
/// `finalize`). Which control is *valid* in which event is the parse-time
/// pre-scan's job, so this function does not branch on the event; it derives
/// `Retry` re-entry from the guard's `provider_launched()` state, not the
/// signal.
///
/// Reuses the existing redirect/resume substrate: a retry bumps the attempt
/// and `continue`s; a resume seeds `next_resume_session_id` +
/// `next_prompt_override`; a proxy swaps `source_path`/`original_ref` and
/// resets the guard for a fresh `initialize`; a requeue records the
/// materialized prompt in rendezvous and exits the current run.
#[allow(clippy::too_many_arguments)]
pub(super) fn dispatch_terminal_control(
    outcome: &LifecycleEventOutcome,
    attempt: u32,
    budgets: &mut ControlBudgets,
    session_id: Option<&str>,
    profile: &dyn crate::commands::wrap::profile::WrapperProfile,
    // `_provider` / `_materialized` are retained on the signature for when
    // `defer` is wired to the rendezvous deferred-execution backend (it will
    // need them to enqueue); unused while `defer` returns not-implemented.
    _provider: Provider,
    prompt_state: &mut HarnessPromptState,
    _materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    proxy: &mut ProxyTracking,
    term: &Terminal,
    show_checks: bool,
) -> TerminalControlAction {
    let Some(control) = outcome.control.as_ref() else {
        return TerminalControlAction::Fallthrough;
    };

    // Compute the control budget (only retry/resume consume one).
    let budget = match control {
        StackControl::Retry { max_attempts, .. } => {
            ControlBudgets::budget_for(&mut budgets.retry, attempt, *max_attempts)
        }
        StackControl::Resume { max_attempts, .. } => {
            ControlBudgets::budget_for(&mut budgets.resume, attempt, *max_attempts)
        }
        _ => 0,
    };

    let proxy_hops_used = proxy.chain.len()
        + usize::from(!proxy.chain.iter().any(|path| path == &prompt_state.source_path));
    let decision = decide_lifecycle_transition(&LifecycleTransitionInput {
        event: lifecycle_guard
            .terminal_signal()
            .unwrap_or(LifecycleSignal::Finalize),
        terminal_slot: lifecycle_guard.terminal_signal(),
        provider_launched: lifecycle_guard.provider_launched(),
        has_prior_error: false,
        outcome,
        has_session: session_id.is_some(),
        attempt,
        control_budget: budget,
        proxy_hops_used,
        proxy_target_seen: false,
        finalize_emitted: lifecycle_guard.finalize_emitted(),
    });

    match decision {
        LifecycleTransitionDecision::TerminalFailure {
            error: LifecycleTransitionError::ExplicitControl,
        } => match control {
            StackControl::Error { reason } => TerminalControlAction::Abort(eyre!(
                "{}",
                reason
                    .as_deref()
                    .unwrap_or("lifecycle stack marked the run as failed")
            )),
            _ => TerminalControlAction::Fallthrough,
        },
        LifecycleTransitionDecision::Reenter(ControlDispatch::Retry {
            delay,
            reenter_preflight,
        }) => {
            if show_checks {
                let what = if reenter_preflight { "pre-flight" } else { "the agent" };
                claudine::harness::report::report_lifecycle_recovery(
                    &format!("lifecycle retry: re-running {what} (attempt {})", attempt + 1),
                    term,
                );
            }
            if !delay.is_zero() {
                std::thread::sleep(delay);
            }
            // The terminal event already fired for this iteration; reset the
            // guard's per-iteration state so the retried attempt can emit its
            // own start/terminal/finalize without the terminal slot being
            // suppressed as already-taken.
            lifecycle_guard.reset_for_next_iteration();
            TerminalControlAction::Continue {
                next_attempt: attempt + 1,
            }
        }
        LifecycleTransitionDecision::Reenter(ControlDispatch::Resume { message }) => {
            // Honor the provider's resume capability. The CLI-side resume gate
            // surfaces a clear error when the provider cannot resume or the
            // session id is missing.
            if let Err(e) = crate::commands::wrap::resume::check_resume_support(
                &profile.provider().to_string(),
                profile.supports_resume(),
                session_id,
            ) {
                return TerminalControlAction::Abort(e);
            }
            prompt_state.next_resume_session_id = session_id.map(|id| id.to_string());
            prompt_state.next_prompt_override = Some(message);
            prompt_state.prompt_tail.clear();
            if show_checks {
                claudine::harness::report::report_lifecycle_recovery(
                    &format!("lifecycle resume: resuming session (attempt {})", attempt + 1),
                    term,
                );
            }
            // Reset per-iteration guard state (the failure terminal already
            // fired) so the resumed attempt emits its own lifecycle events.
            lifecycle_guard.reset_for_next_iteration();
            TerminalControlAction::Continue {
                next_attempt: attempt + 1,
            }
        }
        LifecycleTransitionDecision::Abort(LifecycleTransitionAbort::ResumeWithoutSession) => {
            TerminalControlAction::Abort(
                CompositionError::LifecycleResumeWithoutSession {
                    source_path: prompt_state.source_path.clone(),
                }
                .into(),
            )
        }
        LifecycleTransitionDecision::ProxyHandoff { target } => {
            // Every proxy route funnels through `resolve_proxy_target` (D5/D6):
            // the shared existence-checking resolver anchored on the current
            // source document. This route previously called
            // `resolve_harness_path` directly and swapped `source_path` to a
            // path it never probed, so a `failure`-stack proxy to a missing
            // file only failed later in pre-flight.
            let resolved = match claudine::composition::resolve_proxy_target(
                &target,
                &prompt_state.source_path,
                repo_root,
            ) {
                Ok(path) => path,
                Err(e) => {
                    return TerminalControlAction::Abort(
                        CompositionError::InvalidFileReference {
                            context: Box::new(claudine::composition::FileReferenceContext {
                                source_path: prompt_state.source_path.clone(),
                                event: None,
                                property: "proxy".to_string(),
                                reference: target.clone(),
                                hint: crate::commands::wrap::composition::PROXY_TARGET_HINT
                                    .to_string(),
                            }),
                            source: e,
                        }
                        .into(),
                    );
                }
            };
            // Cycle / hop-limit guard: a `failure` stack that proxies back to a
            // document whose own `failure` stack proxies again would loop
            // forever. Reject a self-proxy, an A->B->A cycle, or an
            // over-long chain with a typed error rather than hanging.
            if !proxy.chain.iter().any(|p| p == &prompt_state.source_path) {
                proxy.chain.push(prompt_state.source_path.clone());
            }
            if !claudine::composition::proxy_handoff_allowed(&proxy.chain, &resolved) {
                return TerminalControlAction::Abort(
                    CompositionError::LifecycleProxyCycle {
                        source_path: prompt_state.source_path.clone(),
                        target: target.clone(),
                        chain: proxy
                            .chain
                            .iter()
                            .map(|p| p.display().to_string())
                            .collect(),
                        limit: claudine::composition::MAX_PROXY_HOPS,
                    }
                    .into(),
                );
            }
            // Swap the running document for the target and reset per-iteration
            // guard state so the target runs a fresh `initialize`/pre-flight.
            prompt_state.source_path = resolved.clone();
            prompt_state.original_ref = target.clone();
            prompt_state.prompt_tail.clear();
            prompt_state.next_prompt_override = None;
            prompt_state.next_resume_session_id = None;
            lifecycle_guard.reset_for_proxy();
            // Record the hop and flag that the loop top must re-parse the
            // guard's lifecycle config from the target's frontmatter — without
            // this the target's events would run against the proxying
            // document's lifecycle (and the original `failure`/`proxy` stack
            // would re-fire, looping forever).
            proxy.chain.push(resolved.clone());
            proxy.pending = true;
            // The loop's pending-adoption point announces the hand-off via
            // `report_proxy_handoff` on re-entry, so no message here.
            // Re-enter at attempt 1 so the target document gets a clean
            // pre-flight / freeze cycle rather than inheriting the proxying
            // document's attempt count.
            TerminalControlAction::Continue { next_attempt: 1 }
        }
        LifecycleTransitionDecision::Abort(
            LifecycleTransitionAbort::DeferredExecutionUnsupported,
        ) => {
            // `defer` (deferred re-execution) is accepted in every event, but its
            // runtime home — the rendezvous deferred-execution scheduler — is not
            // ready to receive prompts yet, so surface a clear "not implemented"
            // error rather than enqueuing. The rendezvous enqueue machinery
            // (`enqueue_requeue_entry`) is retained for when it lands.
            TerminalControlAction::Abort(
                CompositionError::LifecycleDeferNotImplemented {
                    source_path: prompt_state.source_path.clone(),
                }
                .into(),
            )
        }
        LifecycleTransitionDecision::Abort(LifecycleTransitionAbort::ProxyBudgetExhausted) => {
            let mut chain = proxy.chain.clone();
            if !chain.iter().any(|path| path == &prompt_state.source_path) {
                chain.push(prompt_state.source_path.clone());
            }
            TerminalControlAction::Abort(CompositionError::LifecycleProxyCycle {
                source_path: prompt_state.source_path.clone(),
                target: match control {
                    StackControl::Proxy { target } => target.clone(),
                    _ => String::new(),
                },
                chain: chain.iter().map(|path| path.display().to_string()).collect(),
                limit: claudine::composition::MAX_PROXY_HOPS,
            }
            .into())
        }
        LifecycleTransitionDecision::Continue
        | LifecycleTransitionDecision::CatchFailure { .. }
        | LifecycleTransitionDecision::Finalize { .. }
        | LifecycleTransitionDecision::TerminalSuccess
        | LifecycleTransitionDecision::TerminalFailure { .. } => {
            TerminalControlAction::Fallthrough
        }
        LifecycleTransitionDecision::Abort(LifecycleTransitionAbort::EvaluationAfterFinalize) => {
            unreachable!("evaluation errors are handled before control dispatch")
        }
        LifecycleTransitionDecision::Reenter(_) => {
            unreachable!("only retry and resume controls re-enter")
        }
    }
}

/// Run the `finalize` event, then dispatch any recovery control action its
/// stack ended in (`retry`/`resume`/`requeue`/`proxy`).
///
/// `finalize` is the optional-error terminal event, so it doubles as a
/// last-chance recovery surface: a `finalize.stack` that decides the work was
/// not actually done (typically guarded by `when: "err"`) can recover exactly
/// as the `failure` event can. The returned [`TerminalControlAction`] tells the
/// caller whether to re-enter the loop (`Continue`), propagate a hard error
/// (`Abort`), or proceed to its normal terminal return (`Fallthrough`).
///
/// On a recovery re-entry the dispatch resets the guard's per-iteration state,
/// so the next attempt emits its own `start`/terminal/`finalize` signals.
#[allow(clippy::too_many_arguments)]
pub(super) fn run_finalize_with_recovery(
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    err: Option<&LifecycleErrorInfo>,
    loop_start: std::time::Instant,
    attempt: u32,
    budgets: &mut ControlBudgets,
    session_id: Option<&str>,
    profile: &dyn crate::commands::wrap::profile::WrapperProfile,
    provider: Provider,
    prompt_state: &mut HarnessPromptState,
    proxy: &mut ProxyTracking,
    show_checks: bool,
) -> TerminalControlAction {
    let finalize_outcome = run_lifecycle_event(
        lifecycle_guard,
        LifecycleSignal::Finalize,
        materialized,
        &prompt_state.source_path,
        repo_root,
        term,
        effect_engine,
        err,
        loop_start,
    );
    // An evaluation error raised *inside* `finalize` halts the run, but must
    // not re-enter `finalize` (Decision #3 / re-entry guard): surface it as a
    // hard abort directly rather than recursing through the recovery path.
    if let Some(info) = finalize_outcome.evaluation_error.as_ref() {
        // A raise inside `finalize` halts; no further events fire. Emit the
        // styled block now (Decision #2) and mark it already-emitted.
        return TerminalControlAction::Abort(
            crate::output::error_walker::emit_lifecycle_evaluation_error_block(
                CompositionError::lifecycle_evaluation(
                    "finalize",
                    &prompt_state.source_path,
                    info,
                ),
                term,
            )
            .into(),
        );
    }
    dispatch_terminal_control(
        &finalize_outcome,
        attempt,
        budgets,
        session_id,
        profile,
        provider,
        prompt_state,
        materialized,
        repo_root,
        lifecycle_guard,
        proxy,
        term,
        show_checks,
    )
}

/// Outcome of [`drive_terminal_recovery`] for the caller's loop control.
pub(super) enum TerminalRecovery {
    /// A recovery control (`retry`/`resume`/`proxy`) re-entered the loop:
    /// the caller sets `attempt` and `continue`s.
    NextAttempt(u32),
    /// The terminal event and `finalize` completed without recovery: the
    /// caller proceeds to its own terminal return (exit code or typed error).
    Completed,
}

/// Run one terminal lifecycle event through the full recovery sequence:
/// execute the (possibly downgrading) event, halt on a late-binding
/// evaluation error (Decision #3), dispatch the event stack's control
/// action, then give `finalize` its last-chance recovery pass.
///
/// The `err` threaded into `finalize` is `event_err` when the caller
/// supplied one (the failure paths), otherwise a success/blocked stack's
/// explicit `error()` downgrade. On a control `Abort`, `finalize` still
/// fires once carrying that `err` before the abort propagates — and a raise
/// *inside* that finalize halts with the evaluation error instead
/// (Decision #2's single styled emission is preserved by the callees).
///
/// `Err` means halt the run now; the caller propagates it unchanged.
#[allow(clippy::too_many_arguments)]
pub(super) fn drive_terminal_recovery(
    lifecycle_guard: &mut claudine::composition::LifecycleRunGuard<'_>,
    signal: LifecycleSignal,
    event_err: Option<&LifecycleErrorInfo>,
    materialized: &MaterializedHarnessPrompt,
    repo_root: Option<&Path>,
    term: &Terminal,
    effect_engine: &EffectEngine,
    loop_start: std::time::Instant,
    attempt: u32,
    budgets: &mut ControlBudgets,
    session_id: Option<&str>,
    profile: &dyn crate::commands::wrap::profile::WrapperProfile,
    provider: Provider,
    prompt_state: &mut HarnessPromptState,
    proxy: &mut ProxyTracking,
    show_checks: bool,
) -> Result<TerminalRecovery> {
    let event = execute_terminal_event(
        lifecycle_guard,
        signal,
        materialized,
        &prompt_state.source_path,
        repo_root,
        term,
        effect_engine,
        event_err,
        loop_start,
    );
    // A late-binding evaluation error raised by the event's stack halts the
    // run: the helper runs `finalize` once with the evaluation error as
    // `err` and returns the typed catch error, so no separate finalize runs
    // when it returns `Some`.
    if let Some(err) = handle_terminal_evaluation_error(
        &event.outcome,
        event.effective_event,
        lifecycle_guard,
        materialized,
        &prompt_state.source_path,
        repo_root,
        term,
        effect_engine,
        loop_start,
    ) {
        return Err(err);
    }
    let finalize_err = event_err.or(event.downgrade_err.as_ref());
    // The event's stack may end in a lifecycle control action
    // (retry/resume/requeue/proxy). Dispatch it before finalizing so a
    // re-entry skips finalize for this iteration.
    match dispatch_terminal_control(
        &event.outcome,
        attempt,
        budgets,
        session_id,
        profile,
        provider,
        prompt_state,
        materialized,
        repo_root,
        lifecycle_guard,
        proxy,
        term,
        show_checks,
    ) {
        TerminalControlAction::Continue { next_attempt } => {
            return Ok(TerminalRecovery::NextAttempt(next_attempt));
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
                finalize_err,
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
    // `finalize` is a last-chance recovery surface: its stack may recover
    // (retry/resume/requeue/proxy) when the terminal event did not.
    match run_finalize_with_recovery(
        lifecycle_guard,
        materialized,
        repo_root,
        term,
        effect_engine,
        finalize_err,
        loop_start,
        attempt,
        budgets,
        session_id,
        profile,
        provider,
        prompt_state,
        proxy,
        show_checks,
    ) {
        TerminalControlAction::Continue { next_attempt } => {
            Ok(TerminalRecovery::NextAttempt(next_attempt))
        }
        TerminalControlAction::Abort(err) => Err(err),
        TerminalControlAction::Fallthrough => Ok(TerminalRecovery::Completed),
    }
}
