//! The facts and primitives behind `wt remove`: what removing a worktree
//! would delete ([`inventory`]), whether deleting its branch would lose
//! commits ([`safety`]), the live remote checks ([`remote`], over the shared
//! [`crate::live_remote`] transport), and the move-first handoff record
//! ([`handoff`]). A target Git can't read is made checkable first by
//! [`prepare`]: its worktree record is found ([`admin_entry`]), then a missing
//! `.git` file is repaired and verified ([`repair`]) or a missing directory's
//! surviving index is checked ([`missing`]).
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
use crate::worktree::{WorktreeEntry, parse_worktree_list};

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

/// Removes the worktree at `path` (`git worktree remove`, with `--force` when
/// its files may be discarded).
///
/// `path` itself must be a real directory, read without following links:
/// Git would delete the files a replacement link leads to and then fail on
/// the link. Removal is then refused while another process works in the
/// folder ([`check_no_processes`]): on Windows the delete would fail
/// outright, and elsewhere the process can write into the tree while git
/// deletes it. On Windows the folder is then also probed for open files
/// ([`check_not_in_use`]), because `git worktree remove` on a held directory
/// deletes every file and unregisters the worktree before it fails.
///
/// Git's recursive delete stops at the first entry it cannot remove and
/// then unregisters the worktree anyway. When that happens the rest of the
/// folder is deleted here (see [`finish_unregistered`]).
///
/// ## Errors
///
/// [`WorktreeError::NotARealDirectory`] when `path` is a link, a file, or
/// can't be inspected; [`WorktreeError::DirectoryInUse`] or
/// [`WorktreeError::LockProbeRenameBack`] before anything is deleted;
/// [`WorktreeError::FolderNotFullyRemoved`] when git unregistered the
/// worktree but its folder could not be deleted; otherwise git's failure.
pub fn remove_worktree(base: &Path, path: &Path, force: bool) -> Result<(), WorktreeError> {
    if !matches!(availability::inspect(path), Ok(availability::EntryKind::Directory)) {
        return Err(WorktreeError::NotARealDirectory(path.to_path_buf()));
    }
    check_no_processes(path)?;
    #[cfg(windows)]
    check_not_in_use(path)?;
    let path_str = path.display().to_string();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path_str);
    // Only a worktree registered before git ran can have been unregistered by
    // it; any other folder is never deleted here.
    let registered = is_registered(base, path)?;
    match git_from(base, base, &args) {
        Ok(_) => Ok(()),
        Err(error) if !registered || is_registered(base, path)? => Err(error),
        Err(_) => finish_unregistered(path),
    }
}

/// Refuses with [`WorktreeError::DirectoryInUse`] when any process other
/// than the caller and its ancestors has its current directory in `path`.
///
/// Open files are not visible this way; see `sniff::os::processes_working_in`.
pub fn check_no_processes(path: &Path) -> Result<(), WorktreeError> {
    let processes = sniff::os::processes_working_in(path);
    if processes.is_empty() {
        Ok(())
    } else {
        Err(WorktreeError::DirectoryInUse {
            path: path.to_path_buf(),
            processes,
        })
    }
}

/// Whether git still lists a worktree at `path`, compared canonical, since a
/// wrong "no" would delete a folder git chose to keep.
fn is_registered(base: &Path, path: &Path) -> Result<bool, WorktreeError> {
    let porcelain = git_from(base, base, &["worktree", "list", "--porcelain"])?;
    let target = handoff::canonical(path);
    Ok(parse_worktree_list(&porcelain)
        .iter()
        .any(|entry| handoff::canonical(&entry.path) == target))
}

/// Deletes what is left of a worktree folder git has already unregistered:
/// up to three attempts, 200 ms apart, since what refilled it is usually a
/// moment's write by another program.
///
/// ## Errors
///
/// [`WorktreeError::FolderNotFullyRemoved`] when the folder survives every
/// attempt.
pub fn finish_unregistered(path: &Path) -> Result<(), WorktreeError> {
    let mut reason = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            std::thread::sleep(std::time::Duration::from_millis(200));
        }
        match std::fs::remove_dir_all(path) {
            Ok(()) => return Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
            Err(error) => reason = error.to_string(),
        }
    }
    Err(WorktreeError::FolderNotFullyRemoved {
        path: path.to_path_buf(),
        reason,
    })
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
    let tip = git_from(base, base, &["rev-parse", "--verify", &format!("refs/heads/{branch}^{{commit}}")])?;
    if tip != expected_tip {
        return Err(WorktreeError::BranchMoved { branch: branch.to_string(), expected: expected_tip.to_string(), found: tip });
    }
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
        return Err(WorktreeError::DirectoryInUse {
            path: path.to_path_buf(),
            processes: Vec::new(),
        });
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

    /// Git reads the checkout through the link and does not mark it
    /// `prunable`; preparation and the removal primitive both refuse anyway.
    #[test]
    fn a_readable_replacement_link_refuses_preparation_and_removal() {
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
        assert!(matches!(remove_worktree(&base, &target, true), Err(WorktreeError::NotARealDirectory(_))));

        assert!(saved.join("README.md").is_file(), "nothing was deleted through the link");
        assert_eq!(availability::inspect(&target).unwrap(), availability::EntryKind::Link);
        assert_eq!(listed(&repo, &target).branch.as_deref(), Some("feat/r"));
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
        assert!(matches!(check_not_in_use(&missing), Err(WorktreeError::DirectoryInUse { .. })));
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
        let tip = repo.sha("refs/heads/feat/dirty");
        remove_local_branch(&base, "feat/dirty", &tip).unwrap();
        assert!(repo.try_git(&["rev-parse", "--verify", "refs/heads/feat/dirty"]).is_err());
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

    /// A process standing in a worktree subdirectory blocks removal on every
    /// OS; on Windows the lock probe would catch it too.
    #[test]
    fn a_held_directory_is_in_use_and_nothing_is_removed() {
        let repo = TestRepo::new();
        let base = repo.path();
        let wt = repo.add_worktree("feat/held", "held", "main");
        let inside = wt.join("sub");
        std::fs::create_dir(&inside).unwrap();
        // Spawned directly, not through `cmd /C`: killing `cmd` would leave its
        // `ping` child holding the directory and the test's output pipe.
        #[cfg(windows)]
        let mut command = std::process::Command::new("ping");
        #[cfg(windows)]
        command.args(["-n", "60", "127.0.0.1"]);
        #[cfg(unix)]
        let mut command = std::process::Command::new("sleep");
        #[cfg(unix)]
        command.arg("60");
        let mut holder = command
            .current_dir(&inside)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(300));

        let result = remove_worktree(&base, &wt, true);

        let _ = holder.kill();
        let _ = holder.wait();
        match &result {
            Err(WorktreeError::DirectoryInUse { processes, .. }) => {
                #[cfg(unix)]
                assert!(processes.iter().any(|p| p.pid == holder.id()), "{processes:?}");
                let _ = processes;
            }
            other => panic!("expected DirectoryInUse, got {other:?}"),
        }
        assert!(wt.join("README.md").exists(), "files survive");
        let listed = repo.git(&["worktree", "list", "--porcelain"]);
        assert!(listed.contains("feat/held"), "still registered:\n{listed}");
    }

    /// Git stops deleting at an entry it cannot remove and unregisters the
    /// worktree anyway; a read-only subdirectory reproduces that.
    #[cfg(unix)]
    #[test]
    fn a_folder_git_unregistered_but_left_behind_is_reported_then_finished() {
        use std::os::unix::fs::PermissionsExt;

        let repo = TestRepo::new();
        let base = repo.path();
        let wt = repo.add_worktree("feat/stuck", "stuck", "main");
        let locked = wt.join("locked");
        std::fs::create_dir(&locked).unwrap();
        std::fs::write(locked.join("file"), "x").unwrap();
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o555)).unwrap();
        if std::fs::write(locked.join("probe"), "x").is_ok() {
            // Running as root: permissions cannot make git fail.
            std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
            return;
        }

        let result = remove_worktree(&base, &wt, true);

        assert!(matches!(&result, Err(WorktreeError::FolderNotFullyRemoved { .. })), "{result:?}");
        let listed = repo.git(&["worktree", "list", "--porcelain"]);
        assert!(!listed.contains("feat/stuck"), "git unregistered it:\n{listed}");
        assert!(locked.join("file").exists());
        assert!(repo.try_git(&["rev-parse", "--verify", "refs/heads/feat/stuck"]).is_ok(), "branch kept");

        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
        finish_unregistered(&wt).unwrap();
        assert!(!wt.exists());
    }

    #[test]
    fn an_unregistered_folder_is_never_deleted() {
        let repo = TestRepo::new();
        let base = repo.path();
        let stray = base.parent().unwrap().join("stray");
        std::fs::create_dir(&stray).unwrap();
        std::fs::write(stray.join("work.txt"), "x").unwrap();

        assert!(remove_worktree(&base, &stray, true).is_err());
        assert!(stray.join("work.txt").exists());
    }
}
