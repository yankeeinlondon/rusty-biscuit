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

/// Decide the next lifecycle transition without performing side effects.
///
/// Evaluation errors take precedence over action and prior errors. Controls
/// are then resolved using the shared attempt/session/launch policy. Normal
/// terminal completion requests finalize exactly once; callers execute that
/// event and feed its outcome back through this function.
pub fn decide_lifecycle_transition(
    input: &LifecycleTransitionInput<'_>,
) -> LifecycleTransitionDecision {
    if input.outcome.evaluation_error.is_some() {
        if input.event == LifecycleSignal::Finalize || input.finalize_emitted {
            return LifecycleTransitionDecision::Abort(
                LifecycleTransitionAbort::EvaluationAfterFinalize,
            );
        }
        return if matches!(
            input.event,
            LifecycleSignal::Initialize | LifecycleSignal::Start | LifecycleSignal::Blocked
        ) {
            LifecycleTransitionDecision::CatchFailure {
                error: LifecycleTransitionError::Evaluation,
            }
        } else {
            LifecycleTransitionDecision::Finalize {
                error: Some(LifecycleTransitionError::Evaluation),
            }
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

/// Pure result of routing a terminal lifecycle event through its catch events.
#[derive(Debug, Clone, PartialEq)]
pub struct TerminalRoutingDecision {
    /// Highest-precedence event whose expression evaluation failed.
    pub evaluation_error_signal: Option<LifecycleSignal>,
    /// Flow control surfaced by the originating event when no evaluation failed.
    pub control: Option<StackControl>,
}

impl TerminalRoutingDecision {
    fn new(
        origin: LifecycleSignal,
        origin_outcome: &LifecycleEventOutcome,
        failure_outcome: Option<&LifecycleEventOutcome>,
        finalize_outcome: Option<&LifecycleEventOutcome>,
    ) -> Self {
        let evaluation_error_signal = if finalize_outcome
            .and_then(|outcome| outcome.evaluation_error.as_ref())
            .is_some()
        {
            Some(LifecycleSignal::Finalize)
        } else if failure_outcome
            .and_then(|outcome| outcome.evaluation_error.as_ref())
            .is_some()
        {
            Some(LifecycleSignal::Failure)
        } else if origin_outcome.evaluation_error.is_some() {
            Some(origin)
        } else {
            None
        };

        let control = if evaluation_error_signal.is_none() {
            origin_outcome.control.clone()
        } else {
            None
        };

        Self {
            evaluation_error_signal,
            control,
        }
    }
}

/// Route a blocked event and its optional failure/finalize catch outcomes.
pub fn route_blocked_finalize(
    blocked_outcome: &LifecycleEventOutcome,
    failure_outcome: Option<&LifecycleEventOutcome>,
    finalize_outcome: Option<&LifecycleEventOutcome>,
) -> TerminalRoutingDecision {
    TerminalRoutingDecision::new(
        LifecycleSignal::Blocked,
        blocked_outcome,
        failure_outcome,
        finalize_outcome,
    )
}

/// Route a failure event and its finalize catch outcome.
pub fn route_failure_finalize(
    failure_outcome: &LifecycleEventOutcome,
    finalize_outcome: Option<&LifecycleEventOutcome>,
) -> TerminalRoutingDecision {
    TerminalRoutingDecision::new(
        LifecycleSignal::Failure,
        failure_outcome,
        None,
        finalize_outcome,
    )
}

/// Route a post-finalize loop gate and its finalize catch outcome.
pub fn route_loop_gate(
    loop_outcome: &LifecycleEventOutcome,
    finalize_outcome: Option<&LifecycleEventOutcome>,
) -> TerminalRoutingDecision {
    TerminalRoutingDecision::new(
        LifecycleSignal::Loop,
        loop_outcome,
        None,
        finalize_outcome,
    )
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
