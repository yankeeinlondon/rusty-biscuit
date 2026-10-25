//! Finishing a confirmed removal that an earlier run saved and did not
//! finish (R19), a run that waited for another removal of the same worktree
//! (R16), and the shell wrapper's second run (R18).
//!
//! The saved record is the approval: nothing is asked again, and the steps
//! left are run as [`remove`](super::remove) runs them. Where to start is read
//! from the paths and Git's registration as they are now, never from the
//! progress marker alone, and nothing that is no longer the approved object
//! is touched:
//!
//! - a different directory at the deletion path, or Git's record of the
//!   worktree reused for another worktree, refuses with everything kept;
//! - a checkout deleted in place whose path now holds something else is
//!   outside the approval, so the operation is closed and that is kept;
//! - new content at the original path after a move is kept and reported,
//!   while the moved checkout is still deleted;
//! - for a target that is not a checkout directory (an own link and its
//!   checkout, a file, a link to anything else, nothing), each object is
//!   removed only while it has its recorded identity, and one that changed
//!   closes the operation with it kept.
//!
//! A legacy (version 1) record is migrated by the store on the way in; its
//! branch steps that cannot be rebuilt are reported and never run.

use std::io;
use std::path::{Path, PathBuf};

use sniff::filesystem::query::FileIdentity;

use super::{
    Crashed, Env, Finished, Operation, Refusal, Removal, Seams, absent, take_lock,
};
use crate::availability::{self, EntryKind};
use crate::remove::admin_entry::{BackReferenceError, common_git_dir, read_back_reference, same_entry, same_location};
use crate::remove::handoff::{OperationHandoff, canonical, verify_operation};
use crate::remove::quarantine::association::{MovePlan, plan_move};
use crate::remove::quarantine::placement::in_place_for_refusal;
use crate::remove::quarantine::rename;
use crate::remove::recovery::OperationId;
use crate::remove::recovery::compact::{ApprovedTarget, CompactRecord, DeletionMode, Progress};
use crate::remove::recovery::record::RepositoryBinding;
use crate::remove::recovery::store::{RecoveryStore, StoreError, Stored};
use crate::worktree::{WorktreeEntry, parse_worktree_list};

/// Finishes pending operation `id` of the repository at `base` with its
/// saved approval, asking nothing.
pub fn finish_pending(base: &Path, id: &OperationId, env: &Env<'_>) -> Removal {
    // No question's list survives an interruption, so no late change is
    // named (R14 covers only the run that asked or its handoff).
    match resume_with(base, id, env, &mut Seams::none(), None, None) {
        Ok(removal) => removal,
        Err(Crashed) => unreachable!("only a test hook stops the lifecycle"),
    }
}

/// The shell wrapper's second run (R18): the first run saved its approval as
/// `handoff.operation` and printed the token. Checks that the caller left the
/// worktree and runs in the same repository, then finishes the operation as
/// [`finish_pending`] does, with every branch protection rechecked. Nothing
/// about the checkout's content is checked; the dirty paths the first run's
/// question showed only let the final line name the ones it did not (R14).
pub fn finish_handoff(base: &Path, handoff: &OperationHandoff, cwd: &Path, env: &Env<'_>) -> Removal {
    if let Err(refusal) = verify_operation(handoff, base, cwd) {
        return Removal::Refused(Box::new(Refusal::Replaced(match refusal {
            crate::remove::handoff::HandoffRefusal::InsideTarget => {
                "the shell is still inside the worktree, so it was not removed".into()
            }
            crate::remove::handoff::HandoffRefusal::Changed(what) => format!("the {} changed after the question", what.join(", ")),
        })));
    }
    let shown = handoff.shown();
    match resume_with(base, &handoff.operation, env, &mut Seams::none(), Some(&handoff.target), shown.as_deref()) {
        Ok(removal) => removal,
        Err(Crashed) => unreachable!("only a test hook stops the lifecycle"),
    }
}

pub(crate) fn resume_with(
    base: &Path,
    id: &OperationId,
    env: &Env<'_>,
    seams: &mut Seams<'_>,
    expected_target: Option<&Path>,
    shown: Option<&[PathBuf]>,
) -> Result<Removal, Crashed> {
    let refuse = |refusal: Refusal| Ok(Removal::Refused(Box::new(refusal)));
    let store = match common_git_dir(base).map_err(Refusal::Git).and_then(|common| RecoveryStore::open(&common).map_err(Refusal::Store)) {
        Ok(store) => store,
        Err(refusal) => return refuse(refusal),
    };
    let stored = match store.read_stored(id) {
        Ok(stored) => stored,
        Err(error) => return refuse(Refusal::Store(error)),
    };
    let original = match &stored {
        Stored::Compact(record) => record.original.as_path().to_path_buf(),
        Stored::Legacy(legacy) => legacy.record.original.as_path().to_path_buf(),
    };
    if let Some(target) = expected_target
        && canonical(target) != canonical(&original)
    {
        return refuse(Refusal::Replaced(format!("the saved removal is of {}, not {}", original.display(), target.display())));
    }
    let binding = match current_binding(stored.repository()) {
        Ok(binding) => binding,
        Err(refusal) => return refuse(refusal),
    };
    let lock = match take_lock(&store, &binding, env) {
        Ok(lock) => lock,
        Err(refusal) => return refuse(refusal),
    };
    // Whoever held the lock may have finished the operation meanwhile.
    if absent(&store.operation_dir(id)) {
        return Ok(Removal::Finished(Box::new(Finished::new(id.clone(), &original))));
    }
    let pending = match store.resume(lock, id, &binding) {
        Ok(pending) => pending,
        Err(StoreError::Completed(_)) => return Ok(Removal::Finished(Box::new(Finished::new(id.clone(), &original)))),
        Err(error) => return refuse(Refusal::Store(error)),
    };
    let start = match inspect(base, pending.record()) {
        Ok(start) => start,
        Err(Inspected::Refused(refusal)) => return refuse(refusal),
        Err(Inspected::Closed(refusal)) => {
            let _ = pending.finish();
            return refuse(refusal);
        }
    };
    let mut operation = Operation { base, env, pending, finished: Finished::new(id.clone(), &original), fresh: false };
    if start.in_place && let Err(error) = operation.pending.mark_in_place() {
        return refuse(Refusal::Store(error));
    }
    operation.run(start.from, start.plan, shown, seams)
}

/// Where a resumed operation starts, read from what is there now.
struct Start {
    /// The first step to run is the one after `from`.
    from: Progress,
    /// The move to make, for a checkout still at its original path.
    plan: Option<MovePlan>,
    /// The checkout cannot be moved now and is deleted where it stands.
    in_place: bool,
}

enum Inspected {
    /// Everything is kept, the record included; the reason names the path.
    Refused(Refusal),
    /// The checkout to delete is not the approved one any more; the record
    /// is deleted so a later run starts from a fresh question.
    Closed(Refusal),
}

/// What is at a path, read without following a link.
#[derive(Debug, PartialEq, Eq)]
enum At {
    Absent,
    Checkout,
    Other,
}

fn at(path: &Path, identity: &FileIdentity) -> Result<At, Inspected> {
    match availability::inspect(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(At::Absent),
        Err(error) => Err(Inspected::Refused(Refusal::Replaced(format!("{} cannot be inspected: {error}", path.display())))),
        Ok(EntryKind::Directory) => match rename::identity(path) {
            Ok(found) if found == *identity => Ok(At::Checkout),
            Ok(_) => Ok(At::Other),
            Err(error) => Err(Inspected::Refused(Refusal::Replaced(format!("{} cannot be inspected: {error}", path.display())))),
        },
        Ok(_) => Ok(At::Other),
    }
}

/// Where Git's record of the worktree says the checkout is.
#[derive(Debug, PartialEq, Eq)]
enum Registered {
    Original,
    Deletion,
    Removed,
    Elsewhere(String),
}

fn registered(record: &CompactRecord) -> Registered {
    let admin = record.repository.admin_dir.as_path();
    // Only a checkout directory is followed: any other target's path may be
    // a link or a file, and is compared as itself.
    let names = |named: &Path, checkout: &Path| match record.target {
        ApprovedTarget::Checkout { .. } => same_location(named, &checkout.join(".git")),
        _ => named.file_name() == Some(".git".as_ref()) && named.parent().is_some_and(|parent| same_entry(parent, checkout)),
    };
    match read_back_reference(admin) {
        Ok(named) if names(&named, record.deletion_path.as_path()) => Registered::Deletion,
        Ok(named) if names(&named, record.original.as_path()) => Registered::Original,
        Ok(named) => Registered::Elsewhere(format!("Git's record of the worktree now names {}", named.display())),
        Err(BackReferenceError::Missing) if absent(admin) => Registered::Removed,
        Err(error) => Registered::Elsewhere(format!("Git's record of the worktree cannot be read: {error}")),
    }
}

fn inspect(base: &Path, record: &CompactRecord) -> Result<Start, Inspected> {
    let original = record.original.as_path();
    let deletion = record.deletion_path.as_path();
    let kept = |why: String| Err(Inspected::Refused(Refusal::Replaced(why)));
    let done = |from: Progress| Ok(Start { from, plan: None, in_place: false });
    let not_placed = record.progress.min(Progress::Unlocked);
    let registration = registered(record);
    if let Registered::Elsewhere(why) = &registration {
        return kept(format!("{why}, so the removal of {} was not continued", original.display()));
    }
    let identity = &match record.target {
        ApprovedTarget::Checkout { identity } => identity,
        // Nothing is placed, and the content step checks each object's
        // identity where it stands, so it runs again until Git's record
        // is gone. An own link's record names the link, never its checkout.
        _ => {
            return match registration {
                Registered::Removed => done(Progress::Unregistered),
                Registered::Deletion if deletion != original => kept(format!(
                    "Git's record of the worktree now names {}, so the removal of {} was not continued",
                    deletion.display(),
                    original.display()
                )),
                _ => done(not_placed),
            };
        }
    };

    if record.deletion_mode == DeletionMode::InPlace {
        return match (at(original, identity)?, registration) {
            (At::Checkout, _) => done(record.progress.min(Progress::Placed)),
            (At::Other, _) => Err(Inspected::Closed(Refusal::Replaced(format!(
                "{} is no longer the worktree that was approved for removal, so it was kept",
                original.display()
            )))),
            (At::Absent, Registered::Removed) => done(Progress::Unregistered),
            (At::Absent, _) => done(record.progress.max(Progress::ContentRemoved)),
        };
    }

    match (at(original, identity)?, at(deletion, identity)?, registration) {
        (_, At::Other, _) => kept(format!(
            "{} is no longer the worktree that was moved there, so nothing in it was deleted; Git's record of it was kept",
            deletion.display()
        )),
        // The process stopped between the rename and Git's record: move it
        // back without replacing anything, then place it again.
        (At::Absent, At::Checkout, Registered::Original) => {
            if let Err(error) = rename::rename_no_replace(deletion, original) {
                return kept(format!("{} could not be moved back to {}: {error}", deletion.display(), original.display()));
            }
            plan_again(base, record, not_placed)
        }
        (_, At::Checkout, Registered::Deletion) => done(record.progress.max(Progress::Placed)),
        (_, At::Checkout, _) => kept(format!("Git no longer records the worktree at {}, so it was kept", deletion.display())),
        (At::Checkout, At::Absent, Registered::Original) if record.progress < Progress::Placed => plan_again(base, record, not_placed),
        (_, At::Absent, Registered::Deletion) => done(record.progress.max(Progress::ContentRemoved)),
        (_, At::Absent, Registered::Removed) => done(Progress::Unregistered),
        // The checkout vanished before it was moved (R17): nothing is left
        // to delete, and its record is removed where Git keeps it.
        (At::Absent, At::Absent, Registered::Original) if record.progress < Progress::Placed => {
            Ok(Start { from: Progress::ContentRemoved, plan: None, in_place: true })
        }
        (original_at, _, registration) => {
            let found = match original_at {
                At::Absent => "nothing is",
                At::Checkout => "the worktree is",
                At::Other => "something other than the worktree is",
            };
            let recorded = match &registration {
                Registered::Original => "at that path".to_string(),
                Registered::Deletion => format!("at {}", deletion.display()),
                Registered::Removed => "nowhere".to_string(),
                // Returned early above.
                Registered::Elsewhere(_) => "elsewhere".to_string(),
            };
            kept(format!(
                "{found} at {} and Git records the worktree {recorded}, which no step of its removal left, so everything was kept",
                original.display()
            ))
        }
    }
}

/// Plans the move again for a checkout back at its original path. A link
/// that cannot be kept across the move means deletion in place, as on the
/// first run.
fn plan_again(base: &Path, record: &CompactRecord, from: Progress) -> Result<Start, Inspected> {
    let original = record.original.as_path();
    let entry = listed_at(base, original).map_err(|why| Inspected::Refused(Refusal::Replaced(why)))?;
    match plan_move(base, &entry, &record.operation_id) {
        Ok(plan) if record.target == (ApprovedTarget::Checkout { identity: plan.identity }) => Ok(Start { from, plan: Some(plan), in_place: false }),
        Ok(_) => Err(Inspected::Closed(Refusal::Replaced(format!("{} is no longer the approved worktree", original.display())))),
        Err(refusal) if in_place_for_refusal(&refusal).is_some() => Ok(Start { from, plan: None, in_place: true }),
        Err(refusal) => Err(Inspected::Refused(Refusal::Ineligible(refusal))),
    }
}

fn listed_at(base: &Path, path: &Path) -> Result<WorktreeEntry, String> {
    let listing = crate::git::worktree_list_from(base, base).map_err(|error| format!("Git's worktree list could not be read: {error}"))?;
    parse_worktree_list(&listing)
        .into_iter()
        .find(|entry| same_location(&entry.path, path))
        .ok_or_else(|| format!("Git no longer lists {}", path.display()))
}

/// The binding to check the record against, read fresh. Git's record of the
/// worktree is gone once the operation unregistered it, so then the saved
/// identity stands in; a record of that name with another identity was
/// reused for another worktree and refuses.
fn current_binding(saved: &RepositoryBinding) -> Result<RepositoryBinding, Refusal> {
    let common_dir_identity = rename::identity(saved.common_dir.as_path()).map_err(|error| Refusal::Git(error.into()))?;
    let admin_dir_identity = match rename::identity(saved.admin_dir.as_path()) {
        Ok(identity) if identity == saved.admin_dir_identity => identity,
        Ok(_) => {
            return Err(Refusal::Replaced(format!(
                "Git's record {} now belongs to another worktree, so the earlier removal was not continued",
                saved.admin_dir.as_path().display()
            )));
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => saved.admin_dir_identity,
        Err(error) => return Err(Refusal::Git(error.into())),
    };
    Ok(RepositoryBinding {
        common_dir: saved.common_dir.clone(),
        common_dir_identity,
        admin_dir: saved.admin_dir.clone(),
        admin_dir_identity,
    })
}

