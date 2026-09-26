//! The facts and primitives behind `wt remove`: what removing a worktree
//! would delete ([`inventory`]), whether deleting its branch would lose
//! commits ([`safety`]), the live remote checks ([`live_remote`], [`remote`]),
//! and the move-first handoff record ([`handoff`]).
//!
//! Every git call addresses the repository with `git -C` and runs from the
//! base checkout, never from inside the worktree being removed. The rules are
//! item 3 of `2026-09-24-ux-improvements`.

pub mod handoff;
pub mod inventory;
pub mod live_remote;
pub mod remote;
pub mod safety;

#[cfg(test)]
pub(crate) mod test_support;

use std::path::Path;

use crate::error::WorktreeError;
use crate::git::git_from;

pub use inventory::{DirtyEntry, IgnoredGroup, Inventory, collect_inventory};

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
    use test_support::TestRepo;

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
