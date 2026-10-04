//! The facts and primitives behind `wt remove`: what removing a worktree
//! would delete ([`inventory`]), whether deleting its branch would lose
//! commits ([`safety`]), the live remote checks ([`remote`], over the shared
//! [`crate::live_remote`] transport), and the move-first handoff record
//! ([`handoff`]).
//!
//! Every git call addresses the repository with `git -C` and runs from the
//! base checkout, never from inside the worktree being removed. The rules are
//! item 3 of `2026-09-24-ux-improvements`.

pub mod admin_entry;
pub mod handoff;
pub mod included;
pub mod inventory;
pub mod missing;
pub mod remote;
pub mod repair;
pub mod safety;

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
    /// Git can't read the checkout, and it is neither missing nor unlinked.
    #[error("Git can't read this worktree")]
    Unavailable(OtherCondition),
    #[error(transparent)]
    Missing(MissingRefusal),
    #[error(transparent)]
    Repair(RepairRefusal),
}

/// Classifies `entry` from a fresh look at the filesystem and readies it for
/// the removal checks: a healthy checkout is left alone, a missing one has
/// its record inspected, an unlinked one is repaired and verified, and any
/// other state refuses.
pub fn prepare(base: &Path, entry: &WorktreeEntry, git: &dyn RepairGit) -> Result<CheckoutState, PrepareRefusal> {
    match availability::classify(entry) {
        Availability::Healthy => Ok(CheckoutState::Healthy),
        Availability::Missing => inspect_missing(base, entry).map(CheckoutState::Missing).map_err(PrepareRefusal::Missing),
        Availability::Unlinked => repair_unlinked(base, entry, git).map(CheckoutState::Repaired).map_err(PrepareRefusal::Repair),
        Availability::Other(condition) => Err(PrepareRefusal::Unavailable(condition)),
    }
}

/// Removes the worktree at `path` (`git worktree remove`, with `--force` when
/// its files may be discarded).
///
/// On Windows the directory is first checked for another program's lock (see
/// [`check_not_in_use`]), because `git worktree remove` on a held directory
/// deletes every file and unregisters the worktree before it fails.
///
/// ## Errors
///
/// [`WorktreeError::DirectoryInUse`] or [`WorktreeError::LockProbeRenameBack`]
/// on Windows; otherwise git's failure.
pub fn remove_worktree(base: &Path, path: &Path, force: bool) -> Result<(), WorktreeError> {
    #[cfg(windows)]
    check_not_in_use(path)?;
    let path_str = path.display().to_string();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path_str);
    git_from(base, base, &args)?;
    Ok(())
}

/// Deletes the local `branch` with `git branch -D`. Whether it may be deleted
/// is the caller's decision, made from the safety tiers.
pub fn remove_local_branch(base: &Path, branch: &str) -> Result<(), WorktreeError> {
    git_from(base, base, &["branch", "-D", branch])?;
    Ok(())
}

/// Checks that no program holds `path` by renaming it to a sibling name and
/// straight back.
///
/// On Windows the first rename fails for a directory that is some process's
/// current directory or holds an open file; elsewhere it always succeeds.
/// A failed rename back is retried three times, 50 ms apart.
///
/// ## Errors
///
/// [`WorktreeError::DirectoryInUse`] when the first rename fails (nothing
/// changed); [`WorktreeError::LockProbeRenameBack`] when the directory could
/// not be renamed back.
pub fn check_not_in_use(path: &Path) -> Result<(), WorktreeError> {
    let Some(name) = path.file_name() else {
        return Ok(());
    };
    let temporary = path.with_file_name(format!(
        "{}.wt-lock-check-{}",
        name.to_string_lossy(),
        std::process::id()
    ));
    if std::fs::rename(path, &temporary).is_err() {
        return Err(WorktreeError::DirectoryInUse(path.to_path_buf()));
    }
    for attempt in 0..4 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        if std::fs::rename(&temporary, path).is_ok() {
            return Ok(());
        }
    }
    Err(WorktreeError::LockProbeRenameBack {
        original: path.to_path_buf(),
        temporary,
    })
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

    #[test]
    fn an_unheld_directory_passes_the_lock_check_untouched() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("feat-x");
        std::fs::create_dir(&target).unwrap();
        std::fs::write(target.join("file.txt"), "x").unwrap();

        check_not_in_use(&target).unwrap();

        assert_eq!(std::fs::read_to_string(target.join("file.txt")).unwrap(), "x");
        let names: Vec<_> = std::fs::read_dir(dir.path()).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(names, ["feat-x"], "no probe name left behind");
    }

    #[test]
    fn a_missing_directory_is_reported_as_in_use_before_git_runs() {
        let dir = tempfile::tempdir().unwrap();
        let missing = dir.path().join("gone");
        assert!(matches!(check_not_in_use(&missing), Err(WorktreeError::DirectoryInUse(_))));
    }

    #[test]
    fn removes_clean_and_forced_dirty_worktrees_and_force_deletes_branches() {
        let repo = TestRepo::new();
        let base = repo.path();
        let clean = repo.add_worktree("feat/clean", "clean", "main");
        remove_worktree(&base, &clean, false).unwrap();
        assert!(!clean.exists());

        let dirty = repo.add_worktree("feat/dirty", "dirty", "main");
        repo.commit_in(&dirty, "unique.txt");
        std::fs::write(dirty.join("scratch.txt"), "x").unwrap();
        assert!(remove_worktree(&base, &dirty, false).is_err(), "git refuses dirty files");
        assert!(dirty.join("scratch.txt").exists());
        remove_worktree(&base, &dirty, true).unwrap();
        assert!(!dirty.exists());

        // `-D` deletes an unmerged branch; `-d` would have refused.
        remove_local_branch(&base, "feat/dirty").unwrap();
        assert!(repo.try_git(&["rev-parse", "--verify", "refs/heads/feat/dirty"]).is_err());
    }

    /// A process whose current directory is the worktree holds it on Windows.
    #[cfg(windows)]
    #[test]
    fn a_held_directory_is_in_use_and_nothing_is_removed() {
        let repo = TestRepo::new();
        let base = repo.path();
        let wt = repo.add_worktree("feat/held", "held", "main");
        // Spawned directly, not through `cmd /C`: killing `cmd` would leave its
        // `ping` child holding the directory and the test's output pipe.
        let mut holder = std::process::Command::new("ping")
            .args(["-n", "60", "127.0.0.1"])
            .current_dir(&wt)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));

        let result = remove_worktree(&base, &wt, true);

        let _ = holder.kill();
        let _ = holder.wait();
        assert!(matches!(result, Err(WorktreeError::DirectoryInUse(_))), "{result:?}");
        assert!(wt.join("README.md").exists(), "files survive");
        let listed = repo.git(&["worktree", "list", "--porcelain"]);
        assert!(listed.contains("feat/held"), "still registered:\n{listed}");
    }
}
