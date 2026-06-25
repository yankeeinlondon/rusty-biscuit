//! Runtime helpers for the harness execution loop.

use crate::harness::model::{AttemptOutcome, FailureEvent, ProcessTermination};
use crate::stream::summary::StreamExecutionSummary;
use tracing::info_span;

/// Map `ProcessTermination` and `AttemptOutcome` to a `FailureEvent`.
///
/// ## Notes
///
/// `ProcessTermination::Aborted` (a claudine content-guard trip —
/// exit-expression, runaway-repetition, or volume-cap) deliberately maps
/// to [`FailureEvent::AgentFailure`] rather than [`FailureEvent::Timeout`]:
/// the timeout-retry path would re-run the provider and reproduce the
/// runaway. It also deliberately does **not** map to `None` the way
/// [`ProcessTermination::Interrupted`] does, because a guard trip is a
/// genuine failure the operator's lifecycle recovery must observe —
/// suppressing it would silently swallow a runaway kill.
pub fn classify_failure(outcome: &AttemptOutcome) -> Option<FailureEvent> {
    let _span = info_span!(
        "harness_classify_failure",
        termination = %outcome.termination,
        exit_code = outcome.exit_code,
        attempt = outcome.attempt,
    )
    .entered();
    match outcome.termination {
        ProcessTermination::TimedOut => Some(FailureEvent::Timeout),
        ProcessTermination::Interrupted => None, // User canceled, no recovery
        ProcessTermination::LaunchFailed => Some(FailureEvent::AgentFailure),
        ProcessTermination::Aborted => Some(FailureEvent::AgentFailure),
        ProcessTermination::Completed => {
            if outcome.exit_code != 0 {
                Some(FailureEvent::AgentFailure)
            } else {
                None // Success, no failure event
            }
        }
    }
}

pub fn build_attempt_outcome(
    attempt: u32,
    summary: &StreamExecutionSummary,
    termination: ProcessTermination,
) -> AttemptOutcome {
    let _span = info_span!(
        "harness_attempt_outcome",
        attempt,
        termination = %termination,
        exit_code = summary.exit_code,
        has_session_id = summary.session_id.is_some(),
    )
    .entered();
    AttemptOutcome {
        attempt,
        session_id: summary.session_id.clone(),
        final_response: summary.assistant_text.clone(),
        exit_code: summary.exit_code,
        termination,
        stderr_text: summary.stderr_text.clone(),
        // Preserve the synthesized per-guard label so the failure-handler
        // payload can read it. The summary carries no structured guard
        // detail, so `guard_context` stays `None` here; the wrapper attempt
        // path sets it directly from `ProcessResult.guard_context`.
        error_kind: summary.error_kind.clone(),
        guard_context: None,
    }
}
