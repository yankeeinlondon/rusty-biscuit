//! Which steps get a row on the sequence review screen, and how the reviewed
//! rows map back onto the full, one-entry-per-step target vector.
//!
//! Rows are keyed by each draft's original `step_index`, never by step name
//! (names may repeat) or by row number (hidden steps shift it).

use std::io;

use biscuit_tui::prelude::{ABORTED_KIND, CANCELLED_KIND};
use claudine::composition::{
    AgentResolutionState, ExecutableField, ProviderResolutionReason, ResolvedExecutionTarget,
    SequenceStep, SequenceStepDraft,
};
use claudine::provider::Provider;

use super::resolve::is_auto_selectable_state;

/// Whether a step's planned provider/model is offered for review.
///
/// Reads only the normalized step's outer executable: `prompt`, `task`,
/// `group`, and a body step (no executable) are eligible; `shell` and
/// `side_effect` are not. Never composes files or runs commands.
pub(super) fn is_review_eligible(step: &SequenceStep) -> bool {
    match step.executable.as_ref().map(|executable| executable.field) {
        None | Some(ExecutableField::Prompt | ExecutableField::Task | ExecutableField::Group) => {
            true
        }
        Some(ExecutableField::Shell | ExecutableField::SideEffect) => false,
    }
}

/// One target per draft, in draft order, built exactly as the review-bypassed
/// branch builds them. `provider_reason` is shared by every step; a draft
/// with no resolved provider falls back to Claude.
pub(super) fn baseline_targets(
    drafts: &[SequenceStepDraft],
    provider_reason: ProviderResolutionReason,
) -> Vec<ResolvedExecutionTarget> {
    drafts
        .iter()
        .map(|draft| ResolvedExecutionTarget {
            provider: draft.resolved_provider.unwrap_or(Provider::Claude),
            provider_reason,
            model: draft.proposed_model.clone(),
            model_reason: draft.model_reason.clone(),
        })
        .collect()
}

/// The submitted row count differed from the number of rows shown.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("sequence review returned {found} targets for {expected} rows")]
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
    mut baseline: Vec<ResolvedExecutionTarget>,
    reviewed_step_indices: &[usize],
    submitted: Vec<ResolvedExecutionTarget>,
) -> Result<Vec<ResolvedExecutionTarget>, ReviewRowCountMismatch> {
    if submitted.len() != reviewed_step_indices.len() {
        return Err(ReviewRowCountMismatch {
            expected: reviewed_step_indices.len(),
            found: submitted.len(),
        });
    }
    for (&step_index, target) in reviewed_step_indices.iter().zip(submitted) {
        baseline[step_index] = target;
    }
    Ok(baseline)
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
    steps: &[SequenceStep],
    drafts: Vec<SequenceStepDraft>,
    baseline: Vec<ResolvedExecutionTarget>,
    review: impl FnOnce(Vec<SequenceStepDraft>) -> io::Result<Vec<ResolvedExecutionTarget>>,
) -> io::Result<Vec<ResolvedExecutionTarget>> {
    let eligible: Vec<SequenceStepDraft> = drafts
        .into_iter()
        .filter(|draft| is_review_eligible(&steps[draft.step_index]))
        .collect();
    if eligible.is_empty() {
        return Ok(baseline);
    }
    let reviewed_step_indices: Vec<usize> =
        eligible.iter().map(|draft| draft.step_index).collect();
    let submitted = review(eligible)?;
    merge_reviewed_targets(baseline, &reviewed_step_indices, submitted)
        .map_err(|mismatch| io::Error::new(io::ErrorKind::InvalidData, mismatch))
}

/// Whether the review screen opens: only a prompting agent state on a
/// terminal. `shared_state` is `None` when an explicit `--<provider>` flag
/// decided the provider, which never prompts.
pub(super) fn needs_review(is_tty: bool, shared_state: Option<&AgentResolutionState>) -> bool {
    is_tty && shared_state.is_some_and(|state| !is_auto_selectable_state(state))
}

/// One target per step for a live run, or `None` when the user left the
/// review screen with `Esc` or `Ctrl+C`, in which case no step may start.
///
/// `review` is called only when `needs_review` holds and at least one step is
/// eligible; otherwise every step keeps its [`baseline_targets`] entry.
///
/// ## Errors
///
/// As [`review_targets`], except that cancellation is `Ok(None)`.
pub(super) fn live_targets(
    needs_review: bool,
    steps: &[SequenceStep],
    drafts: Vec<SequenceStepDraft>,
    provider_reason: ProviderResolutionReason,
    review: impl FnOnce(Vec<SequenceStepDraft>) -> io::Result<Vec<ResolvedExecutionTarget>>,
) -> io::Result<Option<Vec<ResolvedExecutionTarget>>> {
    let baseline = baseline_targets(&drafts, provider_reason);
    if !needs_review {
        return Ok(Some(baseline));
    }
    match review_targets(steps, drafts, baseline, review) {
        Ok(targets) => Ok(Some(targets)),
        Err(error) if matches!(error.kind(), CANCELLED_KIND | ABORTED_KIND) => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests;
