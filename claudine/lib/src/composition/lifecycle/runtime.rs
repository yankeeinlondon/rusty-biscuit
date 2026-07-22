//! Provider-neutral lifecycle runtime routing.

use super::control::{ControlDispatch, MAX_PROXY_HOPS, decide_control};
use super::executor::{LifecycleEventOutcome, StackControl};
use super::LifecycleSignal;
use crate::stream::summary::{RateLimitInfo, StreamExecutionSummary};

/// Provider-neutral state needed to decide the next lifecycle transition.
///
/// The runtime deliberately carries facts rather than CLI resources: callers
/// report whether a provider launched, whether a session exists, and which
/// lifecycle slots have fired. Process handles, paths, renderers, and other
/// effectful adapters stay outside the library.
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
    /// Hand execution to another composition document.
    ProxyHandoff {
        /// Authored target reference; resolution remains a CLI concern.
        target: String,
    },
    /// Stop because the requested transition is invalid or unavailable.
    Abort(LifecycleTransitionAbort),
}

/// How a CLI adapter must execute a lifecycle event requested by the catch protocol.
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
    pub error: Option<super::context::LifecycleErrorInfo>,
    /// Guard operation required to preserve the single-terminal-slot invariant.
    pub execution: LifecycleCatchExecution,
}

/// Completed result of a provider-neutral catch/finalize sequence.
#[derive(Debug, Clone, PartialEq)]
pub struct LifecycleCatchResult {
    /// Highest-precedence event whose expression evaluation failed.
    pub evaluation_error_signal: Option<LifecycleSignal>,
    /// Error payload carried by that event.
    pub evaluation_error: Option<super::context::LifecycleErrorInfo>,
    /// Setup-phase error that caused the protocol to request `failure`.
    pub setup_error: Option<super::context::LifecycleErrorInfo>,
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
/// The adapter executes only the requested [`LifecycleCatchStep`] and feeds
/// its outcome back with [`Self::record`]. This keeps terminal-slot
/// redesignation, active-error threading, evaluation precedence, and finalize
/// ordering in the library while process handles and rendering remain in the CLI.
#[derive(Debug, Clone)]
pub struct LifecycleCatchProtocol {
    origin: LifecycleSignal,
    state: LifecycleCatchState,
    prior_error: Option<super::context::LifecycleErrorInfo>,
    origin_outcome: LifecycleEventOutcome,
    setup_error: Option<super::context::LifecycleErrorInfo>,
    failure_outcome: Option<LifecycleEventOutcome>,
    finalize_outcome: Option<LifecycleEventOutcome>,
    pending: Option<LifecycleCatchStep>,
}

impl LifecycleCatchProtocol {
    /// Start routing after the originating event has executed.
    pub fn new(
        origin: LifecycleSignal,
        state: LifecycleCatchState,
        prior_error: Option<super::context::LifecycleErrorInfo>,
        origin_outcome: LifecycleEventOutcome,
    ) -> Self {
        let setup_error = Self::setup_error(origin, &origin_outcome);
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
        prior_error: Option<&super::context::LifecycleErrorInfo>,
        setup_error: Option<&super::context::LifecycleErrorInfo>,
        outcome: &LifecycleEventOutcome,
    ) -> Option<LifecycleCatchStep> {
        let evaluation = outcome.evaluation_error.clone();
        if setup_error.is_some() {
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
            });
        }

        let needs_finalize = matches!(
            origin,
            LifecycleSignal::Success | LifecycleSignal::Blocked | LifecycleSignal::Failure
        ) || (origin == LifecycleSignal::Loop && evaluation.is_some());
        (needs_finalize && !state.finalize_emitted).then(|| LifecycleCatchStep {
            signal: LifecycleSignal::Finalize,
            error: evaluation.or_else(|| prior_error.cloned()),
            execution: LifecycleCatchExecution::Record,
        })
    }

    fn setup_error(
        origin: LifecycleSignal,
        outcome: &LifecycleEventOutcome,
    ) -> Option<super::context::LifecycleErrorInfo> {
        match setup_catch_error(origin, outcome)? {
            LifecycleTransitionError::Evaluation => outcome.evaluation_error.clone(),
            LifecycleTransitionError::Action => outcome.action_error.clone(),
            LifecycleTransitionError::ExplicitControl => {
                let Some(StackControl::Error { reason }) = outcome.control.as_ref() else {
                    unreachable!("explicit-control setup error requires error control");
                };
                Some(super::context::LifecycleErrorInfo::from_action_failure(
                    "error",
                    reason.clone().unwrap_or_else(|| {
                        format!("lifecycle {} error", origin.property_name())
                    }),
                ))
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
    /// match the request, allowing adapters to fail closed on protocol misuse.
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
        LifecycleSignal::Success | LifecycleSignal::Blocked | LifecycleSignal::Failure
            if !input.finalize_emitted =>
        {
            LifecycleTransitionDecision::Finalize { error }
        }
        LifecycleSignal::Success
        | LifecycleSignal::Blocked
        | LifecycleSignal::Failure
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

/// Iteration-level signals lifted from a structured stream summary.
#[derive(Debug, Default, Clone)]
pub struct IterationSummarySignals {
    /// Rate-limit trailer observed during the iteration.
    pub rate_limit: Option<RateLimitInfo>,
    /// Structured exit reason such as `step_timeout`.
    pub exit_reason: Option<String>,
    /// Human-readable failure detail from the iteration summary.
    pub error_message: Option<String>,
    /// Provider identifier from the iteration summary.
    pub provider_id: Option<String>,
    /// Model identifier from the iteration summary.
    pub model_id: Option<String>,
}

impl IterationSummarySignals {
    /// Extract loop-relevant fields from a completed stream summary.
    pub fn from_summary(summary: &StreamExecutionSummary) -> Self {
        Self {
            rate_limit: summary.rate_limit.clone(),
            exit_reason: summary.error_kind.clone(),
            error_message: summary.error_message.clone(),
            provider_id: Some(summary.provider.to_string()),
            model_id: summary.model.clone(),
        }
    }

    /// Prefer projected rate-limit fields while retaining parser fallbacks.
    pub fn apply_projected_rate_limit(&mut self, projected: Option<RateLimitInfo>) {
        let Some(projected) = projected else {
            return;
        };
        let parser = self.rate_limit.take().unwrap_or_default();
        self.rate_limit = Some(RateLimitInfo {
            is_throttled: projected.is_throttled.or(parser.is_throttled),
            retry_after_ms: projected.retry_after_ms.or(parser.retry_after_ms),
            message: projected.message.or(parser.message),
            reset_at: projected.reset_at.or(parser.reset_at),
        });
    }
}

#[cfg(test)]
mod tests;
