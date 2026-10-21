//! Provider-neutral lifecycle runtime routing: the transition decision, the
//! catch protocol, and the container-event engine.
//!
//! Everything here is a decision over facts the host reports — whether a
//! provider launched, whether a session exists, which slots have fired — or a
//! [`ContainerRunner`] driving one [`LifecycleRunGuard`]. Process handles,
//! paths, renderers, and scheduling stay with the host.

use std::time::Duration;

use indexmap::IndexMap;
use serde_json::Value;

use super::cancellation::CancellationCause;
use super::context::LifecycleErrorInfo;
use super::control::{ControlDispatch, MAX_PROXY_HOPS, decide_control};
use super::envelope::DiagnosticCatalog;
use super::executor::{LifecycleEventOutcome, StackControl};
use super::run_guard::{GuardedEventStack, LifecycleRunGuard, LifecycleRunHost};
use super::LifecycleSignal;

/// How an event was entered, which decides whether its recovery directives
/// count.
///
/// The engine selects the mode; a host never infers it from the event name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvaluationMode {
    /// Terminal events and a normal `finalize` may request recovery.
    NormalRecovery,
    /// Cleanup after an expression evaluation failed.
    FatalEvaluationCleanup,
    /// Cleanup after the run refused to continue: a setup-phase action failure
    /// or an explicit `error(...)`.
    RefusalCleanup,
    /// `cancelled` and `finalize` after cancellation; a recovery directive is
    /// refused and reported, and the cancellation keeps the terminal outcome.
    CancellationCleanup,
}

/// Provider-neutral state needed to decide the next lifecycle transition.
///
/// The runtime deliberately carries facts rather than host resources: callers
/// report whether a provider launched, whether a session exists, and which
/// lifecycle slots have fired. Process handles, paths, renderers, and other
/// effectful adapters stay with the host.
#[derive(Debug, Clone)]
pub struct LifecycleTransitionInput<'a> {
    /// Event whose stack outcome is being routed.
    pub event: LifecycleSignal,
    /// Event currently occupying the single terminal slot, if any.
    pub terminal_slot: Option<LifecycleSignal>,
    /// Whether provider execution began for this attempt.
    pub provider_launched: bool,
    /// Whether an error existed before this event ran.
    pub has_prior_error: bool,
    /// Outcome produced by this event's stack.
    pub outcome: &'a LifecycleEventOutcome,
    /// Whether a resumable provider session is available.
    pub has_session: bool,
    /// Current 1-based attempt number.
    pub attempt: u32,
    /// Absolute retry/resume ceiling, or zero before the first control firing.
    pub control_budget: u32,
    /// Number of proxy handoffs already committed to the current chain.
    pub proxy_hops_used: usize,
    /// Whether the requested proxy target already occurs in the chain.
    pub proxy_target_seen: bool,
    /// Whether the finalize slot has already fired for this attempt.
    pub finalize_emitted: bool,
}

/// Error channel that determines a terminal transition.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LifecycleTransitionError {
    /// An error existed before the routed event.
    Prior,
    /// Event-time expression evaluation failed.
    Evaluation,
    /// A lifecycle side effect failed during dispatch.
    Action,
    /// The stack explicitly selected `error(...)`.
    ExplicitControl,
}

/// Pure reasons a lifecycle transition cannot continue.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LifecycleTransitionAbort {
    /// Evaluation failed in `finalize`, or after finalize had already fired.
    EvaluationAfterFinalize,
    /// `resume(...)` was selected without an available provider session.
    ResumeWithoutSession,
    /// The proxy chain repeated a target or reached its hop ceiling.
    ProxyBudgetExhausted,
    /// Deferred execution has no provider-neutral runtime implementation yet.
    DeferredExecutionUnsupported,
}

/// Provider-neutral decision returned after one lifecycle event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LifecycleTransitionDecision {
    /// Continue within the current attempt.
    Continue,
    /// Re-enter provider execution or preflight using the supplied dispatch.
    Reenter(ControlDispatch),
    /// Route a setup-phase error through the failure catch event.
    CatchFailure {
        /// Error to expose to the failure stack.
        error: LifecycleTransitionError,
    },
    /// Execute finalize before selecting the terminal result.
    Finalize {
        /// Error to expose to finalize, when one exists.
        error: Option<LifecycleTransitionError>,
    },
    /// End the attempt successfully.
    TerminalSuccess,
    /// End the attempt as a failure.
    TerminalFailure {
        /// Error channel responsible for the failure.
        error: LifecycleTransitionError,
    },
    /// Hand execution to another document.
    ProxyHandoff {
        /// Authored target reference; resolution remains the host's.
        target: String,
    },
    /// Stop because the requested transition is invalid or unavailable.
    Abort(LifecycleTransitionAbort),
}

/// How the host must execute a lifecycle event requested by the catch protocol.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum LifecycleCatchExecution {
    /// Execute through the guard's normal exactly-once event recorder.
    Record,
    /// Re-designate an occupied blocked terminal slot as failure, then run the
    /// failure event directly without recording a second terminal event.
    RedesignateBlockedAsFailure,
}

/// One provider-neutral effect requested by [`LifecycleCatchProtocol`].
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleCatchStep {
    /// Lifecycle event the adapter must execute next.
    pub signal: LifecycleSignal,
    /// Error exposed to the event as the late-bound `err` global.
    pub error: Option<LifecycleErrorInfo>,
    /// Guard operation required to preserve the single-terminal-slot invariant.
    pub execution: LifecycleCatchExecution,
    /// How the event was entered: a normal `finalize`, or cleanup after an
    /// evaluation failure or a setup-phase refusal.
    pub mode: EvaluationMode,
}

/// Completed result of a provider-neutral catch/finalize sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleCatchResult {
    /// Highest-precedence event whose expression evaluation failed.
    pub evaluation_error_signal: Option<LifecycleSignal>,
    /// Error payload carried by that event.
    pub evaluation_error: Option<LifecycleErrorInfo>,
    /// Setup-phase error that caused the protocol to request `failure`.
    pub setup_error: Option<LifecycleErrorInfo>,
    /// Flow control from the originating event when no evaluation failed.
    pub control: Option<StackControl>,
}

/// Guard facts that constrain catch/finalize eligibility.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq)]
pub struct LifecycleCatchState {
    /// Event currently occupying the single terminal slot, if any.
    pub terminal_slot: Option<LifecycleSignal>,
    /// Whether finalize already fired for the current attempt.
    pub finalize_emitted: bool,
}

/// Provider-neutral owner of setup catch routing and exactly-once finalize ordering.
///
/// The host executes only the requested [`LifecycleCatchStep`] and feeds
/// its outcome back with [`Self::record`]. This keeps terminal-slot
/// redesignation, active-error threading, evaluation precedence, and finalize
/// ordering in the engine while process handles and rendering remain the
/// host's.
#[derive(Debug, Clone)]
pub struct LifecycleCatchProtocol {
    origin: LifecycleSignal,
    state: LifecycleCatchState,
    prior_error: Option<LifecycleErrorInfo>,
    origin_outcome: LifecycleEventOutcome,
    setup_error: Option<LifecycleErrorInfo>,
    failure_outcome: Option<LifecycleEventOutcome>,
    finalize_outcome: Option<LifecycleEventOutcome>,
    pending: Option<LifecycleCatchStep>,
}

impl LifecycleCatchProtocol {
    /// Start routing after the originating event has executed.
    ///
    /// `catalog` classifies the error an explicit `error(...)` in a setup
    /// event raises; nothing else is classified.
    pub fn with_catalog(
        catalog: &dyn DiagnosticCatalog,
        origin: LifecycleSignal,
        state: LifecycleCatchState,
        prior_error: Option<LifecycleErrorInfo>,
        origin_outcome: LifecycleEventOutcome,
    ) -> Self {
        let setup_error = Self::setup_error(catalog, origin, &origin_outcome);
        let pending = Self::after_origin(
            origin,
            state,
            prior_error.as_ref(),
            setup_error.as_ref(),
            &origin_outcome,
        );
        Self {
            origin,
            state,
            prior_error,
            origin_outcome,
            setup_error,
            failure_outcome: None,
            finalize_outcome: None,
            pending,
        }
    }

    fn after_origin(
        origin: LifecycleSignal,
        state: LifecycleCatchState,
        prior_error: Option<&LifecycleErrorInfo>,
        setup_error: Option<&LifecycleErrorInfo>,
        outcome: &LifecycleEventOutcome,
    ) -> Option<LifecycleCatchStep> {
        let evaluation = outcome.evaluation_error.clone();
        if setup_error.is_some() {
            let mode = match setup_catch_error(origin, outcome) {
                Some(LifecycleTransitionError::Evaluation) => EvaluationMode::FatalEvaluationCleanup,
                _ => EvaluationMode::RefusalCleanup,
            };
            return Some(LifecycleCatchStep {
                signal: LifecycleSignal::Failure,
                error: setup_error
                    .cloned()
                    .or_else(|| prior_error.cloned()),
                execution: if origin == LifecycleSignal::Blocked
                    && state.terminal_slot == Some(LifecycleSignal::Blocked)
                {
                    LifecycleCatchExecution::RedesignateBlockedAsFailure
                } else {
                    LifecycleCatchExecution::Record
                },
                mode,
            });
        }

        let needs_finalize = matches!(
            origin,
            LifecycleSignal::Success | LifecycleSignal::Blocked | LifecycleSignal::Failure | LifecycleSignal::Cancelled
        ) || (origin == LifecycleSignal::Loop && evaluation.is_some());
        (needs_finalize && !state.finalize_emitted).then(|| LifecycleCatchStep {
            signal: LifecycleSignal::Finalize,
            mode: match evaluation {
                Some(_) => EvaluationMode::FatalEvaluationCleanup,
                None => EvaluationMode::NormalRecovery,
            },
            error: evaluation.or_else(|| prior_error.cloned()),
            execution: LifecycleCatchExecution::Record,
        })
    }

    fn setup_error(
        catalog: &dyn DiagnosticCatalog,
        origin: LifecycleSignal,
        outcome: &LifecycleEventOutcome,
    ) -> Option<LifecycleErrorInfo> {
        match setup_catch_error(origin, outcome)? {
            LifecycleTransitionError::Evaluation => outcome.evaluation_error.clone(),
            LifecycleTransitionError::Action => outcome.action_error.clone(),
            LifecycleTransitionError::ExplicitControl => {
                let Some(StackControl::Error { reason }) = outcome.control.as_ref() else {
                    unreachable!("explicit-control setup error requires error control");
                };
                Some(explicit_error(catalog, origin, reason.as_deref()))
            }
            LifecycleTransitionError::Prior => {
                unreachable!("prior errors do not start setup catch routing")
            }
        }
    }

    /// Return the next event request, if the protocol is not complete.
    pub fn next_step(&self) -> Option<&LifecycleCatchStep> {
        self.pending.as_ref()
    }

    /// Record the outcome of the currently requested event and advance.
    ///
    /// Returns `false` when no step was pending or the supplied signal did not
    /// match the request, allowing hosts to fail closed on protocol misuse.
    pub fn record(
        &mut self,
        signal: LifecycleSignal,
        outcome: LifecycleEventOutcome,
    ) -> bool {
        let Some(step) = self.pending.take() else {
            return false;
        };
        if step.signal != signal {
            self.pending = Some(step);
            return false;
        }
        match signal {
            LifecycleSignal::Failure => {
                // A later evaluation error in cleanup replaces the earlier one.
                let mode = match outcome.evaluation_error {
                    Some(_) => EvaluationMode::FatalEvaluationCleanup,
                    None => step.mode,
                };
                let active_error = outcome
                    .evaluation_error
                    .clone()
                    .or(step.error)
                    .or_else(|| self.prior_error.clone());
                self.failure_outcome = Some(outcome);
                if !self.state.finalize_emitted {
                    self.pending = Some(LifecycleCatchStep {
                        signal: LifecycleSignal::Finalize,
                        error: active_error,
                        execution: LifecycleCatchExecution::Record,
                        mode,
                    });
                }
            }
            LifecycleSignal::Finalize => {
                self.finalize_outcome = Some(outcome);
            }
            _ => return false,
        }
        true
    }

    /// Finish the sequence once no event remains to execute.
    pub fn finish(self) -> Option<LifecycleCatchResult> {
        if self.pending.is_some() {
            return None;
        }
        let (evaluation_error_signal, evaluation_error) = if let Some(error) = self
            .finalize_outcome
            .as_ref()
            .and_then(|outcome| outcome.evaluation_error.clone())
        {
            (Some(LifecycleSignal::Finalize), Some(error))
        } else if let Some(error) = self
            .failure_outcome
            .as_ref()
            .and_then(|outcome| outcome.evaluation_error.clone())
        {
            (Some(LifecycleSignal::Failure), Some(error))
        } else if let Some(error) = self.origin_outcome.evaluation_error.clone() {
            (Some(self.origin), Some(error))
        } else {
            (None, None)
        };
        let control = evaluation_error_signal
            .is_none()
            .then_some(self.origin_outcome.control)
            .flatten();
        Some(LifecycleCatchResult {
            evaluation_error_signal,
            evaluation_error,
            setup_error: self.setup_error,
            control,
        })
    }
}

/// The error an explicit `error(...)` in `event`'s stack selects; a
/// reasonless one reads `lifecycle {event} error`.
pub(crate) fn explicit_error(
    catalog: &dyn DiagnosticCatalog,
    event: LifecycleSignal,
    reason: Option<&str>,
) -> LifecycleErrorInfo {
    LifecycleErrorInfo::from_action_failure_with(
        catalog,
        "error",
        reason.map_or_else(|| format!("lifecycle {} error", event.property_name()), str::to_string),
    )
}

fn setup_catch_error(
    event: LifecycleSignal,
    outcome: &LifecycleEventOutcome,
) -> Option<LifecycleTransitionError> {
    if !matches!(
        event,
        LifecycleSignal::Initialize | LifecycleSignal::Start | LifecycleSignal::Blocked
    ) {
        return None;
    }
    if outcome.evaluation_error.is_some() {
        Some(LifecycleTransitionError::Evaluation)
    } else if outcome.routes_to_failure(event) {
        Some(LifecycleTransitionError::Action)
    } else if matches!(outcome.control, Some(StackControl::Error { .. })) {
        Some(LifecycleTransitionError::ExplicitControl)
    } else {
        None
    }
}

/// Decide the next lifecycle transition without performing side effects.
///
/// Evaluation errors take precedence over action and prior errors. Controls
/// are then resolved using the shared attempt/session/launch policy. Normal
/// terminal completion requests finalize exactly once; callers execute that
/// event and feed its outcome back through this function.
pub fn decide_lifecycle_transition(
    input: &LifecycleTransitionInput<'_>,
) -> LifecycleTransitionDecision {
    if let Some(error) = setup_catch_error(input.event, input.outcome) {
        return LifecycleTransitionDecision::CatchFailure { error };
    }
    if input.outcome.evaluation_error.is_some() {
        if input.event == LifecycleSignal::Finalize || input.finalize_emitted {
            return LifecycleTransitionDecision::Abort(
                LifecycleTransitionAbort::EvaluationAfterFinalize,
            );
        }
        return LifecycleTransitionDecision::Finalize {
            error: Some(LifecycleTransitionError::Evaluation),
        };
    }

    if let Some(control) = input.outcome.control.as_ref() {
        let dispatch = decide_control(
            control,
            input.attempt,
            input.control_budget,
            input.has_session,
            input.provider_launched,
        );
        return match dispatch {
            ControlDispatch::Retry { .. } | ControlDispatch::Resume { .. } => {
                LifecycleTransitionDecision::Reenter(dispatch)
            }
            ControlDispatch::ResumeWithoutSession => LifecycleTransitionDecision::Abort(
                LifecycleTransitionAbort::ResumeWithoutSession,
            ),
            ControlDispatch::Proxy { target } => {
                if input.proxy_target_seen || input.proxy_hops_used >= MAX_PROXY_HOPS {
                    LifecycleTransitionDecision::Abort(
                        LifecycleTransitionAbort::ProxyBudgetExhausted,
                    )
                } else {
                    LifecycleTransitionDecision::ProxyHandoff { target }
                }
            }
            ControlDispatch::Defer { .. } => LifecycleTransitionDecision::Abort(
                LifecycleTransitionAbort::DeferredExecutionUnsupported,
            ),
            ControlDispatch::Exhausted => terminal_or_finalize(input, None),
            ControlDispatch::Stop => {
                if matches!(control, StackControl::Error { .. }) {
                    LifecycleTransitionDecision::TerminalFailure {
                        error: LifecycleTransitionError::ExplicitControl,
                    }
                } else {
                    terminal_or_finalize(input, None)
                }
            }
        };
    }

    let error = if input.outcome.routes_to_failure(input.event) {
        Some(LifecycleTransitionError::Action)
    } else if input.has_prior_error {
        Some(LifecycleTransitionError::Prior)
    } else {
        None
    };
    terminal_or_finalize(input, error)
}

fn terminal_or_finalize(
    input: &LifecycleTransitionInput<'_>,
    error: Option<LifecycleTransitionError>,
) -> LifecycleTransitionDecision {
    match input.event {
        LifecycleSignal::Initialize | LifecycleSignal::Start => {
            if let Some(error) = error {
                LifecycleTransitionDecision::CatchFailure { error }
            } else {
                LifecycleTransitionDecision::Continue
            }
        }
        LifecycleSignal::Loop => LifecycleTransitionDecision::Continue,
        LifecycleSignal::Success | LifecycleSignal::Blocked | LifecycleSignal::Failure | LifecycleSignal::Cancelled
            if !input.finalize_emitted =>
        {
            LifecycleTransitionDecision::Finalize { error }
        }
        LifecycleSignal::Success
        | LifecycleSignal::Blocked
        | LifecycleSignal::Failure
        | LifecycleSignal::Cancelled
        | LifecycleSignal::Finalize => {
            let terminal = input.terminal_slot.unwrap_or(input.event);
            let error = error.or_else(|| {
                matches!(terminal, LifecycleSignal::Blocked | LifecycleSignal::Failure)
                    .then_some(LifecycleTransitionError::Prior)
            });
            if let Some(error) = error {
                LifecycleTransitionDecision::TerminalFailure { error }
            } else {
                LifecycleTransitionDecision::TerminalSuccess
            }
        }
    }
}

/// The next effect requested by a container's lifecycle driver.
#[derive(Debug, Clone, PartialEq)]
pub enum ContainerEventDecision {
    /// Preparation or child execution may proceed.
    Continue,
    /// Initialize opted out; no other event or result is owed.
    Skip,
    /// Execute the failure stack with this error before finalizing.
    Failure { error: Option<LifecycleErrorInfo> },
    /// Execute finalize before publishing any result.
    Finalize { failed: bool, error: Option<LifecycleErrorInfo> },
    /// Finalize settled. Publication also requires that the runner is not cancelled.
    Finished { failed: bool },
    /// Re-enter preparation with these evaluated runtime writes.
    Retry { delay: Duration, overlay: IndexMap<String, Value> },
    /// Transfer this slot without finalizing the source.
    Proxy { target: String, overlay: IndexMap<String, Value> },
    /// An unsupported recovery must become a typed caller error.
    Abort(LifecycleTransitionAbort),
    /// Recovery before successful preparation is unavailable.
    SetupRecoveryUnsupported { event: LifecycleSignal, action: &'static str },
}

/// One guarded event and the next effect its outcome requests.
#[derive(Debug, Clone, PartialEq)]
pub struct ContainerEventResult {
    /// Stack evidence, retained for typed diagnostics.
    pub outcome: LifecycleEventOutcome,
    /// The next lifecycle effect; the caller supplies its lexical context.
    pub decision: ContainerEventDecision,
}

/// Shared lifecycle driver for sequence and group containers.
///
/// Callers perform preparation and children between events and publish native
/// results only after `Finished { failed: false }` when [`Self::is_cancelled`]
/// is false. A retry preserves the
/// guard's initialize slot and its attempt ceiling; it does not finalize.
/// Keeping event execution separate from preparation lets each caller rebuild
/// lexical `seq`, `group`, and `err` values at every requested event.
pub struct ContainerRunner<'a, H: LifecycleRunHost> {
    guard: LifecycleRunGuard<'a, H>,
    attempt: u32,
    budget: u32,
    failed: bool,
    error: Option<LifecycleErrorInfo>,
    secondary_errors: Vec<LifecycleErrorInfo>,
    cancellation: Option<CancellationCause>,
}

impl<'a, H: LifecycleRunHost> ContainerRunner<'a, H> {
    /// Adopt a fresh guard for one container invocation.
    pub fn new(guard: LifecycleRunGuard<'a, H>) -> Self {
        Self { guard, attempt: 1, budget: 0, failed: false, error: None, secondary_errors: Vec::new(), cancellation: None }
    }

    /// Access the guard when adapting an existing event executor.
    pub fn guard_mut(&mut self) -> &mut LifecycleRunGuard<'a, H> {
        &mut self.guard
    }

    /// Current one-based attempt, including the initial attempt.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// Re-enter `start` for the next pass of a looping container.
    ///
    /// The pass that just finished settled through `finalize`; the next one is
    /// a fresh attempt with its own retry budget and terminal outcome, while
    /// `initialize` stays fired for the invocation.
    pub fn begin_next_pass(&mut self) {
        self.attempt = 1;
        self.budget = 0;
        self.failed = false;
        self.error = None;
        self.secondary_errors.clear();
        self.guard.reset_for_next_iteration();
    }

    /// Execute exactly one guarded event with the caller's current bindings.
    pub fn execute_event<S: GuardedEventStack + ?Sized>(
        &mut self,
        signal: LifecycleSignal,
        stack: &S,
    ) -> ContainerEventResult {
        let outcome = if signal == LifecycleSignal::Failure
            && self.guard.redesignate_terminal_to_failure()
        {
            self.guard.run_event_stack(signal, stack)
        } else {
            self.guard.execute_event(signal, stack)
        };
        let decision = self.record_outcome(signal, &outcome);
        ContainerEventResult { outcome, decision }
    }

    /// Preserve a child/preparation failure as the error exposed to cleanup.
    pub fn set_error(&mut self, error: LifecycleErrorInfo) {
        self.failed = true;
        self.error = Some(error);
    }

    /// Cancel an entered primitive after its descendants have settled.
    /// Finalized primitives keep their original terminal outcome.
    pub fn cancel(&mut self, cause: CancellationCause) -> bool {
        if self.guard.finalize_emitted() || self.cancellation.is_some() { return false; }
        self.error = Some(cause.error_with(self.guard.host().diagnostics()));
        self.cancellation = Some(cause);
        true
    }

    /// Whether cancellation owns this primitive's terminal outcome.
    pub fn is_cancelled(&self) -> bool { self.cancellation.is_some() }

    /// How the next event is evaluated: cancellation cleanup once the
    /// primitive is cancelled, normal recovery otherwise.
    pub fn mode(&self) -> EvaluationMode {
        match self.cancellation {
            Some(_) => EvaluationMode::CancellationCleanup,
            None => EvaluationMode::NormalRecovery,
        }
    }

    /// The failure or cancellation cause exposed to terminal cleanup.
    pub fn primary_error(&self) -> Option<&LifecycleErrorInfo> {
        self.error.as_ref()
    }

    /// Cleanup diagnostics that leave the primary terminal outcome unchanged.
    pub fn secondary_errors(&self) -> &[LifecycleErrorInfo] {
        &self.secondary_errors
    }

    /// Route an event the host already executed through the guard.
    pub fn record_outcome(
        &mut self,
        signal: LifecycleSignal,
        outcome: &LifecycleEventOutcome,
    ) -> ContainerEventDecision {
        use ContainerEventDecision as Decision;
        let mode = self.mode();
        if mode == EvaluationMode::CancellationCleanup {
            let warnings = self.guard.cleanup_warnings(mode, outcome);
            self.secondary_errors.extend(warnings);
            return if signal == LifecycleSignal::Finalize { Decision::Finished { failed: false } }
                else { Decision::Finalize { failed: false, error: self.error.clone() } };
        }
        if signal == LifecycleSignal::Failure || signal == LifecycleSignal::Blocked {
            self.failed = true;
        }
        let raised = outcome.evaluation_error.clone().or_else(|| outcome.action_error.clone());
        let explicit_error = match &outcome.control {
            Some(StackControl::Error { reason }) => Some(LifecycleErrorInfo::from_action_failure_with(
                self.guard.host().diagnostics(),
                "error", reason.clone().unwrap_or_else(|| "container lifecycle error".into()),
            )),
            _ => None,
        };
        if let Some(error) = raised.or(explicit_error) {
            self.failed = true;
            if self.error.is_none() {
                self.error = Some(error);
            } else if self.error.as_ref() != Some(&error) {
                self.secondary_errors.push(error);
            }
            if matches!(signal, LifecycleSignal::Initialize | LifecycleSignal::Start | LifecycleSignal::Success | LifecycleSignal::Blocked) {
                return Decision::Failure { error: self.error.clone() };
            }
        } else if signal != LifecycleSignal::Finalize {
            if let Some(control) = &outcome.control {
                match control {
                    StackControl::Skip => {
                        self.guard.defuse();
                        return Decision::Skip;
                    }
                    // A `break` before the children ran: no terminal event
                    // fires, and `finalize` still runs once.
                    StackControl::Break { .. }
                        if matches!(signal, LifecycleSignal::Initialize | LifecycleSignal::Start) =>
                    {
                        self.guard.defuse();
                        return Decision::Finalize { failed: self.failed, error: self.error.clone() };
                    }
                    StackControl::Resume { .. } => {
                        return Decision::Abort(LifecycleTransitionAbort::ResumeWithoutSession);
                    }
                    StackControl::Retry { .. } if signal == LifecycleSignal::Initialize || signal == LifecycleSignal::Blocked => {
                        return Decision::SetupRecoveryUnsupported { event: signal, action: "retry" };
                    }
                    StackControl::Proxy { .. } if signal == LifecycleSignal::Blocked => {
                        return Decision::SetupRecoveryUnsupported { event: signal, action: "proxy" };
                    }
                    StackControl::Retry { max_attempts, overlay, .. } => {
                        if self.budget == 0 {
                            self.budget = self.attempt.saturating_add(*max_attempts);
                        }
                        if let ControlDispatch::Retry { delay, .. } = decide_control(control, self.attempt, self.budget, false, false) {
                            self.attempt = self.attempt.saturating_add(1);
                            self.failed = false;
                            self.error = None;
                            self.secondary_errors.clear();
                            self.guard.reset_for_next_iteration();
                            return Decision::Retry { delay, overlay: overlay.clone() };
                        }
                    }
                    StackControl::Proxy { target, overlay, .. } => {
                        self.guard.defuse();
                        return Decision::Proxy { target: target.clone(), overlay: overlay.clone() };
                    }
                    StackControl::Defer { .. } => {
                        return Decision::Abort(LifecycleTransitionAbort::DeferredExecutionUnsupported);
                    }
                    _ => {}
                }
            }
        }
        match signal {
            LifecycleSignal::Initialize | LifecycleSignal::Start | LifecycleSignal::Loop => Decision::Continue,
            LifecycleSignal::Finalize => Decision::Finished { failed: self.failed },
            _ => Decision::Finalize { failed: self.failed, error: self.error.clone() },
        }
    }
}

#[cfg(test)]
mod tests;
