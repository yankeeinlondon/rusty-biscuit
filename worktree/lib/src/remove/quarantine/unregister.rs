//! The final step of a checked removal: unregistering the worktree once its
//! quarantine is verified gone.
//!
//! [`unregister_quarantine`] runs after
//! [`delete_content`](crate::remove::observation::deletion::delete_content)
//! returned without a stop. It drains the observer one last time, confirms
//! that neither the quarantine nor anything at the original path exists,
//! re-binds the worktree's index (its bytes, and Git's reading of every entry,
//! split index included) and HEAD to the state approved before the move, stops
//! watching the admin directory, and removes exactly this registration through
//! [`remove_missing_record`]: plain `git worktree remove`, never `--force`,
//! never `prune` or `repair`.

use std::io;
use std::path::Path;
use std::time::SystemTime;

use sniff::filesystem::query::NativeString;

use crate::git::git_from;
use crate::remove::admin_entry::same_location;
use crate::remove::missing::{IndexBinding, MissingRefusal, inspect_missing, remove_missing_record};
use crate::remove::observation::deletion::{DeletionOptions, own_move};
use crate::remove::observation::{CoverageLost, EventKind, Observation, Scope};
use crate::remove::outcome::{ActivityKind, ActivityLocation, ObservedActivity, RemovalOutcome};
use crate::worktree::parse_worktree_list;

/// Why the registration was kept. The checkout's content is already deleted.
#[derive(Debug, thiserror::Error)]
pub enum UnregisterError {
    #[error("activity was observed at the checkout's locations or its Git record")]
    Activity(Vec<ObservedActivity>),
    #[error("activity observation was lost: {0}")]
    CoverageLost(CoverageLost),
    /// The worktree's index or HEAD is not the approved one: new staged work
    /// that no consent covered.
    #[error("the worktree's index or HEAD changed after it was approved")]
    IndexChanged,
    /// Git no longer lists the worktree at the quarantine exactly once.
    #[error("Git's worktree list {0}")]
    Registration(String),
    #[error(transparent)]
    Refused(MissingRefusal),
}

impl UnregisterError {
    /// The typed outcome of an operation that stopped here.
    pub fn outcome(&self) -> RemovalOutcome {
        match self {
            Self::Activity(_) | Self::IndexChanged => RemovalOutcome::ActivityDetected,
            Self::CoverageLost(_) => RemovalOutcome::StateUnverified,
            Self::Registration(_) | Self::Refused(_) => RemovalOutcome::CleanupFailed,
        }
    }
}

/// Unregisters the worktree whose quarantine `observation` watched, from the
/// base checkout `base`, if its index and HEAD still match `approved`.
///
/// ## Returns
///
/// A warning when the copy record could not be deleted; the registration is
/// gone either way.
///
/// ## Errors
///
/// An [`UnregisterError`]; the registration is kept in every case except a
/// failed `git worktree remove`, whose state the caller re-reads.
pub fn unregister_quarantine(
    base: &Path,
    observation: &mut Observation,
    approved: &IndexBinding,
) -> Result<Option<String>, UnregisterError> {
    let targets = observation.targets().clone();
    let drain = observation.drain();
    if let Some(lost) = drain.lost {
        return Err(UnregisterError::CoverageLost(lost));
    }
    let original_kinds: Vec<EventKind> =
        drain.events.iter().filter(|event| event.scope == Scope::Original(Default::default())).map(|event| event.kind).collect();
    let moved_away = own_move(&original_kinds, DeletionOptions::default().cumulative_kinds) && absent(targets.original());
    let mut activities = Vec::new();
    for event in &drain.events {
        let (location, path) = match &event.scope {
            Scope::Admin(name) => (ActivityLocation::Admin, targets.admin().join(name)),
            Scope::Original(relative) if relative.as_os_str().is_empty() && moved_away => continue,
            Scope::Original(relative) => (ActivityLocation::Original, targets.original().join(relative)),
            // Late events of the quarantine's own removal; whatever they
            // describe is gone.
            Scope::Tree(_) | Scope::Quarantine(_) if absent(targets.quarantine()) => continue,
            Scope::Tree(relative) | Scope::Quarantine(relative) => (ActivityLocation::Quarantine, targets.quarantine().join(relative)),
        };
        activities.push(activity(location, &path, event.received));
    }
    for (location, path) in [(ActivityLocation::Quarantine, targets.quarantine()), (ActivityLocation::Original, targets.original())] {
        if !absent(path) {
            activities.push(activity(location, path, SystemTime::now()));
        }
    }
    if !activities.is_empty() {
        return Err(UnregisterError::Activity(activities));
    }

    let listing = git_from(base, base, &["worktree", "list", "--porcelain"])
        .map_err(|error| UnregisterError::Registration(format!("could not be read: {error}")))?;
    let listed: Vec<_> = parse_worktree_list(&listing)
        .into_iter()
        .filter(|entry| same_location(&entry.path, targets.quarantine()))
        .collect();
    let [entry] = listed.as_slice() else {
        return Err(UnregisterError::Registration(format!("shows the quarantine {} times", listed.len())));
    };
    let inspected = inspect_missing(base, entry).map_err(UnregisterError::Refused)?;
    if !approved.matches(&inspected) {
        return Err(UnregisterError::IndexChanged);
    }
    // After the last index check, before Git deletes the admin directory.
    observation.release_admin();
    remove_missing_record(base, entry, &inspected).map_err(UnregisterError::Refused)
}

fn absent(path: &Path) -> bool {
    matches!(std::fs::symlink_metadata(path), Err(error) if error.kind() == io::ErrorKind::NotFound)
}

fn activity(location: ActivityLocation, path: &Path, received: SystemTime) -> ObservedActivity {
    let received_at_ms = received.duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64);
    // An admin file is only ever changed; anything at a checkout location
    // after deletion was created.
    let kind = if location == ActivityLocation::Admin { ActivityKind::Modified } else { ActivityKind::Created };
    ObservedActivity { location, path: NativeString::from(path), kind, received_at_ms }
}

#[cfg(test)]
mod tests;
