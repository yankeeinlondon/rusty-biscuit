//! Provider-neutral lifecycle runtime routing.

use super::executor::{LifecycleEventOutcome, StackControl};
use super::LifecycleSignal;
use crate::stream::summary::{RateLimitInfo, StreamExecutionSummary};

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
mod tests {
    use super::*;
    use crate::composition::LifecycleErrorInfo;

    fn raised(message: &str) -> LifecycleEventOutcome {
        LifecycleEventOutcome {
            evaluation_error: Some(LifecycleErrorInfo::from_action_failure(
                "evaluation",
                message,
            )),
            ..LifecycleEventOutcome::default()
        }
    }

    #[test]
    fn blocked_routing_uses_finalize_failure_origin_precedence() {
        let blocked = raised("blocked");
        let failure = raised("failure");
        let finalize = raised("finalize");
        assert_eq!(
            route_blocked_finalize(&blocked, Some(&failure), Some(&finalize))
                .evaluation_error_signal,
            Some(LifecycleSignal::Finalize)
        );
        assert_eq!(
            route_blocked_finalize(&blocked, Some(&failure), None).evaluation_error_signal,
            Some(LifecycleSignal::Failure)
        );
        assert_eq!(
            route_blocked_finalize(&blocked, None, None).evaluation_error_signal,
            Some(LifecycleSignal::Blocked)
        );
    }
}
