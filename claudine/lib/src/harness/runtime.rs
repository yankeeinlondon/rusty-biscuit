//! Runtime helpers for the harness execution loop.

use crate::harness::model::{AttemptOutcome, ProcessTermination};
use crate::stream::summary::StreamExecutionSummary;

/// Build an [`AttemptOutcome`] from a stream execution summary and termination info.
pub fn build_attempt_outcome(
    attempt: u32,
    summary: &StreamExecutionSummary,
    termination: ProcessTermination,
) -> AttemptOutcome {
    AttemptOutcome {
        attempt,
        session_id: summary.session_id.clone(),
        final_response: summary.assistant_text.clone(),
        exit_code: summary.exit_code,
        termination,
        stderr_text: summary.stderr_text.clone(),
    }
}
