//! Provider-neutral lifecycle runtime routing.
//!
//! The transition decision, the catch protocol, and the container-event engine
//! are Darkmatter's ([`darkmatter::lifecycle::runtime`]), re-exported here.
//! This module adds the constructor that classifies with Claudine's diagnostic
//! catalog ([`LifecycleCatchProtocolExt`]), names the container runner over
//! Claudine's run host, and keeps [`IterationSummarySignals`], the provider
//! facts Claudine lifts from a stream summary.

pub use darkmatter::lifecycle::runtime::{
    ContainerEventDecision, ContainerEventResult, EvaluationMode, LifecycleCatchExecution,
    LifecycleCatchProtocol, LifecycleCatchResult, LifecycleCatchState, LifecycleCatchStep,
    LifecycleTransitionAbort, LifecycleTransitionDecision, LifecycleTransitionError,
    LifecycleTransitionInput, decide_lifecycle_transition,
};

use super::LifecycleSignal;
use super::context::LifecycleErrorInfo;
use super::executor::LifecycleEventOutcome;
use crate::diagnostics::ClaudineDiagnostics;
use crate::stream::summary::{RateLimitInfo, StreamExecutionSummary};

#[cfg(test)]
use super::control::{ControlDispatch, MAX_PROXY_HOPS};
#[cfg(test)]
use super::executor::StackControl;
#[cfg(test)]
use crate::composition::LifecycleErrorInfoExt;

/// Shared lifecycle driver for sequence and group containers, over Claudine's
/// run host; see [`darkmatter::lifecycle::runtime::ContainerRunner`].
pub type ContainerRunner<'a> = darkmatter::lifecycle::runtime::ContainerRunner<'a, super::ClaudineRunHost<'a>>;

/// [`LifecycleCatchProtocol`]'s constructor under Claudine's diagnostic
/// catalog.
pub trait LifecycleCatchProtocolExt {
    /// Start routing after the originating event has executed; see
    /// [`LifecycleCatchProtocol::with_catalog`].
    fn new(
        origin: LifecycleSignal,
        state: LifecycleCatchState,
        prior_error: Option<LifecycleErrorInfo>,
        origin_outcome: LifecycleEventOutcome,
    ) -> Self;
}

impl LifecycleCatchProtocolExt for LifecycleCatchProtocol {
    fn new(
        origin: LifecycleSignal,
        state: LifecycleCatchState,
        prior_error: Option<LifecycleErrorInfo>,
        origin_outcome: LifecycleEventOutcome,
    ) -> Self {
        Self::with_catalog(&ClaudineDiagnostics, origin, state, prior_error, origin_outcome)
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
