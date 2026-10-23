//! The facts and primitives behind `wt remove`: what removing a worktree
//! would delete ([`inventory`]), whether deleting its branch would lose
//! commits ([`safety`]), the live remote checks ([`remote`], over the shared
//! [`crate::live_remote`] transport), and the move-first handoff record
//! ([`handoff`]). A target Git can't read is made checkable first by
//! [`prepare`]: its worktree record is found ([`admin_entry`]), then a missing
//! `.git` file is repaired and verified ([`repair`]) or a missing directory's
//! surviving index is checked ([`missing`]). A contention-aware removal
//! reports a typed [`outcome`] and keeps durable [`recovery`] state; [`usage`]
//! is its decisive check that no other process is using the checkout,
//! [`quarantine`] moves the checkout aside without replacing anything (and
//! unregisters it once it is gone), and [`observation`] watches both
//! locations while that happens and deletes the moved checkout entry by
//! entry against its operation-start inventory. [`delete`] is the plain
//! engine that replaces that checked deletion: it deletes whatever the
//! approved checkout holds, never crossing a link or a mount, and waits out
//! only temporary contention. [`lifecycle`] runs those steps in order as one
//! removal, and resumes one that stopped.
//! Its waits, mount boundaries, and filesystem failures are injected through
//! [`clock`], [`mounts`], and [`faults`], so tests never sleep, mount, or need
//! privileges.
//!
//! Every git call addresses the repository with `git -C` and runs from the
//! base checkout, never from inside the worktree being removed. The rules are
//! item 3 of `2026-09-24-ux-improvements`.

pub mod admin_entry;
pub mod clock;
pub mod delete;
pub mod faults;
pub mod handoff;
pub mod included;
pub mod inventory;
pub mod lifecycle;
pub mod missing;
pub mod mounts;
pub mod observation;
pub mod outcome;
pub mod quarantine;
pub mod recovery;
pub mod remote;
pub mod repair;
pub mod safety;
pub mod usage;

#[cfg(test)]
pub(crate) mod test_support;

use std::path::Path;

use crate::availability::{self, Availability, OtherCondition};
use crate::error::WorktreeError;
use crate::git::git_from;
use crate::worktree::WorktreeEntry;

pub use inventory::{DirtyEntry, Inventory, collect_inventory};
use missing::{MissingCheckout, MissingRefusal, inspect_missing};
use repair::{RepairGit, RepairRefusal, RepairReport, repair_unlinked};

/// The removal target's checkout as prepared for removal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckoutState {
    /// Git can read the checkout; nothing was done to it.
    Healthy,
    /// The directory is gone; its surviving record was inspected instead.
    Missing(MissingCheckout),
    /// The `.git` file was missing and its link was restored and verified.
    Repaired(RepairReport),
}

/// Why the target could not be prepared. Nothing was removed; only
/// [`PrepareRefusal::Repair`] with an attempted repair may have changed Git
/// metadata (see [`RepairRefusal::repair_attempted`]).
#[derive(Debug, thiserror::Error)]
pub enum PrepareRefusal {
    /// Git can't read the checkout and it is neither missing nor unlinked, or
    /// its path is a link.
    #[error("Git can't read this worktree, or its path is a link")]
    Unavailable(OtherCondition),
    #[error(transparent)]
    Missing(MissingRefusal),
    #[error(transparent)]
    Repair(RepairRefusal),
}

/// Classifies `entry` from a fresh look at the filesystem and readies it for
/// the removal checks: a healthy checkout is left alone, a missing one has
/// its record inspected, an unlinked one is repaired and verified, and any
/// other state refuses. A recorded path that is itself a link refuses as
/// [`OtherCondition::Link`] even when Git reads the checkout through it.
pub fn prepare(base: &Path, entry: &WorktreeEntry, git: &dyn RepairGit) -> Result<CheckoutState, PrepareRefusal> {
    match availability::classify(entry) {
        Availability::Healthy => Ok(CheckoutState::Healthy),
        Availability::Missing => inspect_missing(base, entry).map(CheckoutState::Missing).map_err(PrepareRefusal::Missing),
        Availability::Unlinked => repair_unlinked(base, entry, git).map(CheckoutState::Repaired).map_err(PrepareRefusal::Repair),
        Availability::Other(condition) => Err(PrepareRefusal::Unavailable(condition)),
    }
}

/// The commit `refs/heads/<branch>` points at, as `git rev-parse --verify
/// refs/heads/<branch>^{commit}` prints it. Read in-process and fresh on
/// every call, so a recheck still sees a concurrent update; Git runs only
/// when that read has no commit to give, so a missing branch fails with
/// Git's own error.
///
/// ## Errors
///
/// Git's failure to resolve the branch to a commit.
pub fn branch_tip(base: &Path, branch: &str) -> Result<String, WorktreeError> {
    let refname = format!("refs/heads/{branch}");
    if let Some(Some(tip)) = crate::git_metadata::commit_of(base, &refname) {
        return Ok(tip);
    }
    git_from(base, base, &["rev-parse", "--verify", &format!("{refname}^{{commit}}")])
}

/// Deletes the local `branch` with `git branch -D`, only while its tip is
/// still `expected_tip`. Whether it may be deleted is the caller's decision,
/// made from the safety tiers for that tip; commits added since were never
/// assessed.
///
/// ## Errors
///
/// [`WorktreeError::BranchMoved`] when the tip is no longer `expected_tip`
/// (the branch is kept); otherwise git's failure.
pub fn remove_local_branch(base: &Path, branch: &str, expected_tip: &str) -> Result<(), WorktreeError> {
    let tip = branch_tip(base, branch)?;
    if tip != expected_tip {
        return Err(WorktreeError::BranchMoved { branch: branch.to_string(), expected: expected_tip.to_string(), found: tip });
    }
    git_from(base, base, &["branch", "-D", branch])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree::parse_worktree_list;
    use test_support::TestRepo;

    fn listed(repo: &TestRepo, path: &Path) -> WorktreeEntry {
        parse_worktree_list(&repo.git(&["worktree", "list", "--porcelain"]))
            .into_iter()
            .find(|entry| admin_entry::same_location(&entry.path, path))
            .expect("listed")
    }

    #[test]
    fn preparation_follows_the_checkout_state() {
        let repo = TestRepo::new();
        let base = repo.path();
        let healthy = repo.add_worktree("feat/h", "h", "main");
        let missing = repo.add_worktree("feat/m", "m", "main");
        let unlinked = repo.add_worktree("feat/u", "u", "main");
        let damaged = repo.add_worktree("feat/d", "d", "main");
        std::fs::remove_dir_all(&missing).unwrap();
        std::fs::remove_file(unlinked.join(".git")).unwrap();
        // Prunable, yet not unlinked: the directory was replaced by a file.
        std::fs::remove_dir_all(&damaged).unwrap();
        std::fs::write(&damaged, "not a checkout").unwrap();

        assert_eq!(prepare(&base, &listed(&repo, &healthy), &repair::Git).unwrap(), CheckoutState::Healthy);
        let prepared = prepare(&base, &listed(&repo, &missing), &repair::Git);
        assert!(matches!(&prepared, Ok(CheckoutState::Missing(m)) if !m.needs_consent()), "{prepared:?}");
        assert!(matches!(prepare(&base, &listed(&repo, &unlinked), &repair::Git), Ok(CheckoutState::Repaired(_))));
        assert!(unlinked.join(".git").is_file(), "repaired before any inventory");
        assert!(matches!(
            prepare(&base, &listed(&repo, &damaged), &repair::Git),
            Err(PrepareRefusal::Unavailable(OtherCondition::NotADirectory))
        ));
        assert_eq!(std::fs::read_to_string(&damaged).unwrap(), "not a checkout");
    }

    /// Git reads the checkout through the link and does not mark it
    /// `prunable`; preparation refuses anyway.
    #[test]
    fn a_readable_replacement_link_refuses_preparation() {
        let repo = TestRepo::new();
        let base = repo.path();
        let target = repo.add_worktree("feat/r", "r", "main");
        let saved = test_support::replace_with_link(&target, "saved");
        let entry = listed(&repo, &target);
        assert_eq!(entry.prunable, None);

        assert!(matches!(
            prepare(&base, &entry, &repair::Git),
            Err(PrepareRefusal::Unavailable(OtherCondition::Link))
        ));

        assert!(saved.join("README.md").is_file(), "nothing was deleted through the link");
        assert_eq!(availability::inspect(&target).unwrap(), availability::EntryKind::Link);
        assert_eq!(listed(&repo, &target).branch.as_deref(), Some("feat/r"));
    }

    /// `-D` deletes an unmerged branch; `-d` would have refused.
    #[test]
    fn force_deletes_an_unmerged_branch_at_its_assessed_tip() {
        let repo = TestRepo::new();
        let base = repo.path();
        let wt = repo.add_worktree("feat/unmerged", "unmerged", "main");
        let tip = repo.commit_in(&wt, "unique.txt");
        repo.git(&["worktree", "remove", wt.to_str().unwrap()]);

        remove_local_branch(&base, "feat/unmerged", &tip).unwrap();

        assert!(repo.try_git(&["rev-parse", "--verify", "refs/heads/feat/unmerged"]).is_err());
    }

    /// The branch was judged safe (or its loss approved) at one tip; a commit
    /// added since was never assessed.
    #[test]
    fn a_branch_whose_tip_moved_since_it_was_assessed_is_kept() {
        let repo = TestRepo::new();
        let base = repo.path();
        let wt = repo.add_worktree("feat/moved", "moved", "main");
        let assessed = repo.sha("refs/heads/feat/moved");
        let added = repo.commit_in(&wt, "later.txt");

        let result = remove_local_branch(&base, "feat/moved", &assessed);

        assert!(
            matches!(&result, Err(WorktreeError::BranchMoved { expected, found, .. }) if *expected == assessed && *found == added),
            "{result:?}"
        );
        assert_eq!(repo.sha("refs/heads/feat/moved"), added);
    }
}
