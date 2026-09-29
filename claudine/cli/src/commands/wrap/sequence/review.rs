//! Which steps get a row on the sequence review screen, and how the reviewed
//! rows map back onto the full, one-entry-per-step target vector.
//!
//! Rows are keyed by each draft's original `step_index`, never by step name
//! (names may repeat) or by row number (hidden steps shift it).

// Wired into `execute_sequence` by the change that implements these stubs;
// until then only the tests call them.
#![cfg_attr(
    not(test),
    allow(dead_code, reason = "stubs are wired into execute_sequence in phase 2")
)]

use std::io;

use claudine::composition::{
    ProviderResolutionReason, ResolvedExecutionTarget, SequenceStep, SequenceStepDraft,
};

/// Whether a step's planned provider/model is offered for review.
///
/// Reads only the normalized step's outer executable: `prompt`, `task`,
/// `group`, and a body step (no executable) are eligible; `shell` and
/// `side_effect` are not. Never composes files or runs commands.
pub(super) fn is_review_eligible(_step: &SequenceStep) -> bool {
    unimplemented!("phase 2: review eligibility")
}

/// One target per draft, in draft order, built exactly as the review-bypassed
/// branch builds them. `provider_reason` is shared by every step.
pub(super) fn baseline_targets(
    _drafts: &[SequenceStepDraft],
    _provider_reason: ProviderResolutionReason,
) -> Vec<ResolvedExecutionTarget> {
    unimplemented!("phase 2: baseline targets")
}

/// The submitted row count differed from the number of rows shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReviewRowCountMismatch {
    /// Rows shown (eligible drafts).
    pub expected: usize,
    /// Targets the review returned.
    pub found: usize,
}

/// Writes `submitted[i]` over `baseline[reviewed_step_indices[i]]`.
///
/// ## Errors
///
/// [`ReviewRowCountMismatch`] when the two slices differ in length; the
/// baseline is not modified.
pub(super) fn merge_reviewed_targets(
    _baseline: Vec<ResolvedExecutionTarget>,
    _reviewed_step_indices: &[usize],
    _submitted: Vec<ResolvedExecutionTarget>,
) -> Result<Vec<ResolvedExecutionTarget>, ReviewRowCountMismatch> {
    unimplemented!("phase 2: merge reviewed targets")
}

/// Reviews the eligible drafts and returns one target per original step.
///
/// `drafts` and `baseline` are index-aligned with `steps`. When no draft is
/// eligible, `review` is never called and `baseline` is returned unchanged.
///
/// ## Errors
///
/// Propagates `review`'s error unchanged (so cancellation keeps its kind),
/// and returns [`io::ErrorKind::InvalidData`] when `review` returns a
/// different number of targets than the rows it was given.
pub(super) fn review_targets(
    _steps: &[SequenceStep],
    _drafts: Vec<SequenceStepDraft>,
    _baseline: Vec<ResolvedExecutionTarget>,
    _review: impl FnOnce(Vec<SequenceStepDraft>) -> io::Result<Vec<ResolvedExecutionTarget>>,
) -> io::Result<Vec<ResolvedExecutionTarget>> {
    unimplemented!("phase 2: review targets")
}

#[cfg(test)]
mod tests;
