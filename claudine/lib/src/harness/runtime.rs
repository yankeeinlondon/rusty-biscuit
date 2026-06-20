//! Runtime helpers for the harness execution loop.

use crate::harness::model::{AttemptOutcome, ProcessTermination};
use crate::stream::summary::StreamExecutionSummary;
use tracing::info_span;

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
        error_kind: None,
        guard_context: None,
    }
}
