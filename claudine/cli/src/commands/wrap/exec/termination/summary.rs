//! Projection of an [`EarlyTermination`] onto the synthesized stream summary
//! and the structured guard context carried on [`ProcessResult`].
//!
//! [`ProcessResult`]: super::super::ProcessResult

use claudine::stream::logs::EarlyTermination;
use claudine::stream::summary::StreamExecutionSummary;

use super::message::{
    render_exit_expression_message, render_repeated_stream_error_message,
    render_runaway_repetition_message, render_runaway_volume_message,
    render_stalled_generation_message,
};

/// Overwrite the synthesized summary fields when the stderr bridge or the
/// OpenCode wait loop signals an early-exit condition.
///
/// Preserves any parser-provided `rate_limit` fields field-by-field:
/// `is_throttled` is forced to `Some(true)`, `message` is only set when
/// absent, and `reset_at` keeps the first non-`None` value.
pub(crate) fn apply_early_termination_to_summary(
    summary: &mut StreamExecutionSummary,
    termination: &EarlyTermination,
) {
    match termination {
        EarlyTermination::RateLimit { message, reset_at } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("usage_limit_reached".into());
            summary.error_message = Some(message.clone());

            let mut rate_limit = summary.rate_limit.clone().unwrap_or_default();
            rate_limit.is_throttled = Some(true);
            if rate_limit.message.is_none() {
                rate_limit.message = Some(message.clone());
            }
            if rate_limit.reset_at.is_none()
                && let Some(reset) = reset_at
            {
                rate_limit.reset_at = Some(*reset);
            }
            summary.rate_limit = Some(rate_limit);
        }
        EarlyTermination::Timeout { message } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("timeout".into());
            summary.error_message = Some(message.clone());
        }
        EarlyTermination::StepTimeout { message, .. } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("step_timeout".into());
            summary.error_message = Some(message.clone());
        }
        EarlyTermination::ExitExpression { pattern, scope } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("exit_expression".into());
            summary.error_message = Some(render_exit_expression_message(pattern, scope.as_deref()));
        }
        EarlyTermination::RunawayRepetition {
            cycle_len, repeats, ..
        } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("runaway_repetition".into());
            summary.error_message = Some(render_runaway_repetition_message(*cycle_len, *repeats));
        }
        EarlyTermination::RunawayVolume { lines, bytes } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("runaway_volume".into());
            summary.error_message = Some(render_runaway_volume_message(*lines, *bytes));
        }
        EarlyTermination::RepeatedStreamError { count } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("repeated_stream_error".into());
            summary.error_message = Some(render_repeated_stream_error_message(*count));
        }
        EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            context,
        } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("stalled_generation".into());
            summary.error_message = Some(render_stalled_generation_message(
                *generation_count,
                *stall_duration,
                context,
            ));
        }
    }
}

/// Extract the structured [`GuardContext`] for a content-guard
/// [`EarlyTermination`], or `None` for the non-content variants
/// (rate-limit / timeout / step-timeout, which carry no guard context).
///
/// Only the cluster relevant to the trip is populated; every other field
/// stays `None`. Carried onto [`ProcessResult`](super::super::ProcessResult) so
/// the attempt outcome (and, in Phase 7, the failure-handler payload) can read
/// the guard detail without re-parsing the prose `error_message`.
///
/// [`GuardContext`]: claudine::harness::GuardContext
pub(crate) fn early_termination_guard_context(
    termination: &EarlyTermination,
) -> Option<claudine::harness::GuardContext> {
    use claudine::harness::GuardContext;
    match termination {
        EarlyTermination::ExitExpression { pattern, scope } => Some(GuardContext {
            pattern: Some(pattern.clone()),
            scope: scope.clone(),
            ..GuardContext::default()
        }),
        EarlyTermination::RunawayRepetition { cycle_len, repeats } => Some(GuardContext {
            cycle_len: Some(*cycle_len),
            repeats: Some(*repeats),
            ..GuardContext::default()
        }),
        EarlyTermination::RunawayVolume { lines, bytes } => Some(GuardContext {
            lines: Some(*lines),
            bytes: Some(*bytes),
            ..GuardContext::default()
        }),
        EarlyTermination::StalledGeneration {
            generation_count,
            stall_duration,
            context,
        } => Some(GuardContext {
            generation_count: Some(*generation_count),
            stall_duration_ms: Some(stall_duration.as_millis() as u64),
            // Carry only the safe identity metadata the detector captured;
            // each stays `None` when OpenCode never tagged it.
            session_id: context.session_id.clone(),
            step: context.step,
            agent: context.agent.clone(),
            provider_id: context.provider_id.clone(),
            model_id: context.model_id.clone(),
            mode: context.mode.clone(),
            ..GuardContext::default()
        }),
        EarlyTermination::RateLimit { .. }
        | EarlyTermination::Timeout { .. }
        | EarlyTermination::StepTimeout { .. }
        | EarlyTermination::RepeatedStreamError { .. } => None,
    }
}
