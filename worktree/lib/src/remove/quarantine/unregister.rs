//! The final step of a checked removal: unregistering the worktree once its
//! quarantine is verified gone.
//!
//! [`unregister_quarantine`] runs after
//! [`delete_content`](crate::remove::observation::deletion::delete_content)
//! returned without a stop. It drains the observer one last time, re-lists
//! the worktree once, stops watching the admin directory, compares the
//! worktree's index (its bytes, and Git's reading of every entry, split
//! index included) and HEAD with the state approved before the move, and
//! removes exactly this registration as
//! [`remove_missing_record`](crate::remove::missing::remove_missing_record)
//! does: plain `git worktree remove`, never `--force`, never `prune` or
//! `repair`.
//!
//! The user agreed to the removal, so what the last drain shows, lost
//! watching, something at the original path (never touched), and a changed
//! index or HEAD are returned as warnings ([`Unregistered::activity`]), not
//! refusals. Something at the quarantine path is not the moved checkout and
//! keeps the registration, since `git worktree remove` would delete it. An
//! interrupt that ends either wait for delivery keeps the registration.

use std::io;
use std::path::Path;
use std::time::SystemTime;

use sniff::filesystem::query::NativeString;

use crate::remove::admin_entry::same_location;
use crate::availability::{self, Availability};
use crate::remove::admin_entry::AdminEntry;
use crate::remove::missing::{Changes, IndexBinding, MissingRefusal, remove_listed_record};
use crate::remove::observation::deletion::{DeletionOptions, own_move};
use crate::remove::observation::{EventKind, Observation, ObservationStage, Scope};
use crate::remove::outcome::{ActivityKind, ActivityLocation, ObservedActivity, RemovalOutcome};
use crate::worktree::parse_worktree_list;

/// Why the registration was kept. The checkout's content is already deleted.
#[derive(Debug, thiserror::Error)]
pub enum UnregisterError {
    /// Git no longer lists the worktree at the quarantine exactly once.
    #[error("Git's worktree list {0}")]
    Registration(String),
    #[error(transparent)]
    Refused(MissingRefusal),
    /// The interrupt ended a wait for change delivery before Git's step.
    #[error("the removal was interrupted")]
    Interrupted,
}

impl UnregisterError {
    /// The typed outcome of an operation that stopped here.
    pub fn outcome(&self) -> RemovalOutcome {
        match self {
            Self::Registration(_) | Self::Refused(_) | Self::Interrupted => RemovalOutcome::CleanupFailed,
        }
    }
}

/// The registration is gone.
#[derive(Debug, Default)]
pub struct Unregistered {
    /// Set when the copy record could not be deleted.
    pub copy_warning: Option<String>,
    /// What changed after the user agreed: the last drain's events, lost
    /// watching, something at the original path, and a changed index or
    /// HEAD (as a change to the admin directory's `index` or `HEAD`).
    pub activity: Vec<ObservedActivity>,
}

/// Unregisters the worktree whose quarantine `observation` watched, from the
/// base checkout `base`. `approved` is the index and HEAD the user agreed
/// to; a difference is reported, not refused. `interrupted` is polled by the
/// observer's waits.
///
/// ## Errors
///
/// An [`UnregisterError`]; the registration is kept in every case except a
/// failed `git worktree remove`, whose state the caller re-reads.
pub fn unregister_quarantine(
    base: &Path,
    observation: &mut Observation,
    approved: &IndexBinding,
    interrupted: &dyn Fn() -> bool,
) -> Result<Unregistered, UnregisterError> {
    let targets = observation.targets().clone();
    observation.enter_stage(ObservationStage::Unregistration);
    let drain = observation.drain(interrupted);
    if drain.interrupted {
        return Err(UnregisterError::Interrupted);
    }
    let mut activities = Vec::new();
    if drain.lost.is_some() {
        activities.push(ObservedActivity {
            location: ActivityLocation::Quarantine,
            path: NativeString::from(targets.quarantine()),
            kind: ActivityKind::CoverageLost,
            received_at_ms: ms(SystemTime::now()),
        });
    }
    let original_kinds: Vec<EventKind> =
        drain.events.iter().filter(|event| event.scope == Scope::Original(Default::default())).map(|event| event.kind).collect();
    let moved_away = own_move(&original_kinds, DeletionOptions::default().cumulative_kinds) && absent(targets.original());
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
    // Outside what the user agreed to; never touched.
    if !absent(targets.original()) {
        activities.push(activity(ActivityLocation::Original, targets.original(), SystemTime::now()));
    }

    let listing = crate::git::worktree_list_from(base, base)
        .map_err(|error| UnregisterError::Registration(format!("could not be read: {error}")))?;
    let listed: Vec<_> = parse_worktree_list(&listing)
        .into_iter()
        .filter(|entry| same_location(&entry.path, targets.quarantine()))
        .collect();
    let [entry] = listed.as_slice() else {
        return Err(UnregisterError::Registration(format!("shows the quarantine {} times", listed.len())));
    };
    let availability = availability::classify(entry);
    if availability != Availability::Missing {
        return Err(UnregisterError::Refused(MissingRefusal::NotMissing(availability)));
    }
    // Before Git deletes the admin directory. The index and HEAD are checked
    // after this, once, against the approval; a write to the index in
    // between changes its bytes, so it still refuses.
    observation.release_admin(interrupted).map_err(|_| UnregisterError::Interrupted)?;
    let admin = AdminEntry { dir: targets.admin().to_path_buf() };
    let (copy_warning, changed) =
        remove_listed_record(base, entry, entry, &admin, approved, Changes::Allow).map_err(UnregisterError::Refused)?;
    if changed {
        activities.push(activity(ActivityLocation::Admin, &targets.admin().join("index"), SystemTime::now()));
    }
    Ok(Unregistered { copy_warning, activity: activities })
}

fn ms(at: SystemTime) -> u64 {
    at.duration_since(SystemTime::UNIX_EPOCH).map_or(0, |since| since.as_millis() as u64)
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
