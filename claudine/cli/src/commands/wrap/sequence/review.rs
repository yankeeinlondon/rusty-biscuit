//! When the sequence review screen opens, and how its rows map back onto the
//! per-task selections.
//!
//! Every provider-launching task gets a row, keyed by its graph identity
//! (never by name, which may repeat across nested containers, or by row
//! number). Only a task whose provider nothing resolved is editable; every
//! other task is a read-only row showing its provider (or that its launch
//! decides it), so the screen never offers a choice the launch would discard.

use std::io;

use biscuit_tui::prelude::{ABORTED_KIND, CANCELLED_KIND};
use claudine::composition::{ResolvedExecutionTarget, SequenceStepDraft};

use super::providers::{TaskSelections, review_drafts};

/// The submitted row count differed from the number of editable rows shown.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("sequence review returned {found} targets for {expected} editable rows")]
pub(super) struct ReviewRowCountMismatch {
    /// Editable rows shown.
    pub expected: usize,
    /// Targets the review returned.
    pub found: usize,
}

/// Whether the review screen opens: on a terminal, when some task is missing
/// a provider (or names an ambiguous list) and nothing else resolved it.
pub(super) fn needs_review(is_tty: bool, selections: &TaskSelections) -> bool {
    is_tty && selections.first_unresolved().is_some()
}

/// Review `selections` and store each editable row's choice as its task's
/// fallback. Returns `false` when the user left the screen with `Esc` or
/// `Ctrl+C`, in which case no task may start.
///
/// `review` receives one draft per task, read-only rows included, and returns
/// one target per editable row, in row order.
///
/// ## Errors
///
/// Propagates `review`'s error unchanged except cancellation, and returns
/// [`io::ErrorKind::InvalidData`] when `review` returns a different number of
/// targets than there were editable rows.
pub(super) fn review_selections(
    selections: &mut TaskSelections,
    review: impl FnOnce(Vec<SequenceStepDraft>) -> io::Result<Vec<ResolvedExecutionTarget>>,
) -> io::Result<bool> {
    let drafts = review_drafts(selections);
    let identities: Vec<_> = selections.iter().map(|selection| selection.site.identity.clone()).collect();
    let editable: Vec<_> = drafts
        .iter()
        .zip(&identities)
        .filter(|(draft, _)| draft.read_only.is_none())
        .map(|(_, identity)| identity.clone())
        .collect();
    let submitted = match review(drafts) {
        Ok(submitted) => submitted,
        Err(error) if matches!(error.kind(), CANCELLED_KIND | ABORTED_KIND) => return Ok(false),
        Err(error) => return Err(error),
    };
    if submitted.len() != editable.len() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            ReviewRowCountMismatch { expected: editable.len(), found: submitted.len() },
        ));
    }
    selections.apply_review(editable.into_iter().zip(submitted).collect());
    Ok(true)
}

#[cfg(test)]
mod tests;
