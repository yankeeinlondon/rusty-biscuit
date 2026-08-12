//! Final summary reporting and `--perf` emission for the sequence orchestrator.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::status::{Status, StatusState};
use claudine::composition::SequenceRunSummary;
use color_eyre::eyre::Result;

use crate::log;

use super::SEQUENCE_INTERRUPT_EXIT_CODE;

/// Emit the final sequence summary to the terminal and, when requested,
/// the performance report.
///
/// Returns the appropriate process exit code: `130` if an interrupt was
/// observed, `1` if any step failed, or `0` on full success.
#[allow(deprecated)]
pub(super) fn emit_sequence_summary(
    summary: &SequenceRunSummary,
    perf_accumulator: Option<crate::perf::SequencePerfAccumulator>,
    interrupted: &Arc<AtomicBool>,
    interrupt_observed: bool,
    silent: bool,
) -> Result<i32> {
    if !silent {
        eprintln!();
        if summary.failed == 0 {
            let status = Status::from_prose(format!(
                "Sequence finished: <green>{}</green> succeeded, 0 failed",
                summary.succeeded
            ))
            .state(StatusState::Success);
            log::message(&status.render(&log::terminal()));
        } else {
            let status = Status::from_prose(format!(
                "Sequence finished: <green>{}</green> succeeded, <red>{}</red> failed",
                summary.succeeded, summary.failed
            ))
            .state(StatusState::Failure);
            log::message(&status.render(&log::terminal()));
        }
    }

    // `--perf` is an explicit opt-in and overrides `--silent`/`--quiet`.
    // The perf report is always emitted to stderr when requested.
    if let Some(acc) = perf_accumulator {
        crate::perf::emit_report(&acc.into_report());
    }

    if interrupt_observed || interrupted.load(Ordering::SeqCst) {
        return Ok(SEQUENCE_INTERRUPT_EXIT_CODE);
    }
    if summary.failed > 0 { Ok(1) } else { Ok(0) }
}
