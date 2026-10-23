//! Where a confirmed removal deletes the checkout: from its quarantine after
//! the move, or in place at its original path.
//!
//! The move is preferred because it frees the original path at once. It is
//! skipped, never forced, when it cannot be made without risking other data:
//!
//! - A filesystem without a no-replacement rename (WSL drvfs) is never
//!   served by a check followed by a plain rename, since another directory
//!   could appear between them.
//! - A Windows rename blocked by something in use inside the checkout would
//!   block everything; in place, unrelated files are deleted while the
//!   engine waits on the held ones.
//! - A filesystem mounted inside the checkout is not moved with its ancestor;
//!   the move is not attempted at all.
//! - A `.git` link verified to name this worktree's own record, but that
//!   cannot be rewritten for the move ([`MoveRefusal::LinkNotMovable`]).
//!
//! Every other failure stays an error, with nothing replaced: a collision at
//! the hidden name, a permission denial, another volume, a refusal (above
//! all a `.git` that names another worktree's record, which this approval
//! does not cover), and a move that was rolled back or stranded.

use std::path::Path;

use crate::remove::mounts::{Mount, mounts_within};

use super::association::{Faults, MoveError, MovePlan, MoveRefusal, move_with};
use super::rename::RenameFailure;

/// How the checkout is deleted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Placement {
    /// It is at the plan's quarantine, still registered with Git there.
    Moved,
    /// It is at its original path, with its registration, exactly as before.
    InPlace(InPlaceReason),
}

/// Why the checkout is deleted where it stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InPlaceReason {
    /// The filesystem cannot rename without replacing (R4).
    NoReplaceUnsupported,
    /// Windows refused the move because something inside is in use.
    InUse,
    /// Filesystems mounted inside the checkout, outermost first.
    NestedMounts(Vec<Mount>),
    /// The `.git` link names this worktree's record but cannot be kept
    /// across a move (R8).
    LinkNotMovable(String),
}

/// Moves the checkout to `plan.quarantine` unless it must be deleted in
/// place. `mounts` is the mount table; only mounts strictly inside the
/// checkout count, and any of them rules the move out before it is tried.
///
/// ## Errors
///
/// The [`MoveError`] of a failed move that does not map to deletion in
/// place; see the module documentation.
pub fn place(base: &Path, plan: &MovePlan, mounts: &[Mount]) -> Result<Placement, MoveError> {
    place_with(base, plan, mounts, &Faults::default())
}

pub(crate) fn place_with(base: &Path, plan: &MovePlan, mounts: &[Mount], faults: &Faults) -> Result<Placement, MoveError> {
    let nested = mounts_within(mounts, &plan.original);
    if !nested.is_empty() {
        return Ok(Placement::InPlace(InPlaceReason::NestedMounts(nested.into_iter().cloned().collect())));
    }
    let error = match move_with(base, plan, faults) {
        Ok(()) => return Ok(Placement::Moved),
        Err(error) => error,
    };
    // Both mapped kinds fail before anything moves.
    let in_place = match &error {
        MoveError::Rename(rename) => in_place_for_rename(rename.kind),
        MoveError::Refused(refusal) => in_place_for_refusal(refusal),
        MoveError::RolledBack { .. } | MoveError::Stranded { .. } => None,
    };
    in_place.map(Placement::InPlace).ok_or(error)
}

/// The in-place reason a failed no-replacement rename calls for, or `None`
/// when the failure stays an error.
pub fn in_place_for_rename(kind: RenameFailure) -> Option<InPlaceReason> {
    match kind {
        RenameFailure::Unsupported => Some(InPlaceReason::NoReplaceUnsupported),
        RenameFailure::Sharing => Some(InPlaceReason::InUse),
        RenameFailure::Collision | RenameFailure::CrossDevice | RenameFailure::Permission | RenameFailure::Other => None,
    }
}

/// The in-place reason a refused move calls for, or `None` when the refusal
/// stands. The caller applies this to [`plan_move`](super::association::plan_move)'s
/// refusal too, which comes before any [`MovePlan`] exists.
pub fn in_place_for_refusal(refusal: &MoveRefusal) -> Option<InPlaceReason> {
    match refusal {
        MoveRefusal::LinkNotMovable(why) => Some(InPlaceReason::LinkNotMovable(why.clone())),
        MoveRefusal::NotARealDirectory(_)
        | MoveRefusal::Locked { .. }
        | MoveRefusal::Unsupported(_)
        | MoveRefusal::Association(_)
        | MoveRefusal::Changed(_)
        | MoveRefusal::Uninspectable(_) => None,
    }
}

#[cfg(test)]
mod tests;
