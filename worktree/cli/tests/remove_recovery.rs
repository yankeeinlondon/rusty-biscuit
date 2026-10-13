//! `wt remove` stopping after the checkout moved, and the retry by the
//! original name, through the real binary.
//!
//! A controlled writer is a `git` shim first on `wt`'s `PATH`: it runs the
//! real Git, then, once, writes where a stray process would. The shim fires
//! on a Git call the removal makes after the move, so the write reaches the
//! deletion race rather than the preflight usage check. Unix only: on Windows
//! `wt` resolves `git.exe`, so a script cannot stand in; the library's
//! lifecycle tests cover the same stops on every OS.
//!
//! Every run has stdin redirected and `CI`/`WT_SHELL_WRAPPER` removed, so it
//! is non-interactive and wrapper-less, and sets `NO_COLOR`.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git").current_dir(dir).args(args).env("GIT_TERMINAL_PROMPT", "0").output().unwrap();
    assert!(out.status.success(), "git {args:?} in {dir:?}: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    /// `root/repo` on `main` with one commit, and a worktree on `branch` at
    /// `root/wts/<dir_name>` holding one tracked file. The repository's Git
    /// config is written directly rather than through `git config` calls.
    fn new(branch: &str, dir_name: &str) -> (Self, PathBuf) {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        let config = repo.join(".git").join("config");
        let mut text = fs::read_to_string(&config).unwrap();
        text.push_str("[user]\n\temail = test@example.com\n\tname = Test User\n[commit]\n\tgpgsign = false\n[gc]\n\tauto = 0\n");
        fs::write(&config, text).unwrap();
        fs::write(repo.join("README.md"), "readme\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-q", "-m", "init"]);
        let this = Self { root };
        let wt = this.wts().join(dir_name);
        git(&repo, &["worktree", "add", "-q", "-b", branch, wt.to_str().unwrap(), "main"]);
        (this, wt)
    }

    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn wts(&self) -> PathBuf {
        self.root.path().join("wts")
    }

    /// `wt remove <args>` from the base checkout, with `path` as `PATH` when
    /// given.
    fn remove(&self, args: &[&str], path: Option<&str>) -> Output {
        let mut command = Command::new(assert_cmd::cargo::cargo_bin("wt"));
        command
            .current_dir(self.repo())
            .arg("remove")
            .args(args)
            .env_remove("WT_SHELL_WRAPPER")
            .env_remove("CI")
            .env_remove("COMPLETE")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("NO_COLOR", "1")
            .stdin(std::process::Stdio::null());
        if let Some(path) = path {
            command.env("PATH", path);
        }
        command.output().unwrap()
    }

    /// A `git` shim on `PATH` that runs the real Git and then, the first time
    /// `when` (a shell condition, evaluated after the call) holds, runs
    /// `write`. Returns the `PATH` to run `wt` with.
    fn writer(&self, when: &str, write: &str) -> String {
        use std::os::unix::fs::PermissionsExt;

        let real = String::from_utf8(Command::new("sh").args(["-c", "command -v git"]).output().unwrap().stdout).unwrap();
        let bin = self.root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let fired = bin.join("fired");
        let shim = bin.join("git");
        fs::write(
            &shim,
            format!(
                "#!/bin/sh\n'{real}' \"$@\"\nstatus=$?\n\
                if [ ! -e '{fired}' ] && {when}; then : > '{fired}'; {write}; fi\n\
                exit $status\n",
                real = real.trim(),
                fired = fired.display(),
            ),
        )
        .unwrap();
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
        format!("{}:{}", bin.display(), std::env::var("PATH").unwrap_or_default())
    }

    /// The hidden sibling the checkout was moved to, if one exists.
    fn quarantine(&self) -> Option<PathBuf> {
        fs::read_dir(self.wts())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .find(|path| path.file_name().unwrap().to_string_lossy().starts_with(".wt-remove-"))
    }

    fn branch_exists(&self, branch: &str) -> bool {
        Command::new("git")
            .current_dir(self.repo())
            .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
            .status()
            .unwrap()
            .success()
    }

    fn listed(&self) -> String {
        git(&self.repo(), &["worktree", "list", "--porcelain"])
    }

    /// Operation directories that still hold a journal: pending removals.
    fn pending(&self) -> usize {
        let Ok(entries) = fs::read_dir(self.repo().join(".git").join("wt-removal")) else { return 0 };
        entries.filter_map(Result::ok).filter(|entry| entry.path().join("journal.jsonl").exists()).count()
    }
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The saved report a stop names on its `Full report:` line, parsed.
fn saved_report(stderr: &str) -> serde_json::Value {
    let path = stderr
        .lines()
        .find_map(|line| line.strip_prefix("Full report: "))
        .unwrap_or_else(|| panic!("no Full report line in {stderr}"));
    serde_json::from_str(&fs::read_to_string(path.trim()).unwrap()).unwrap()
}

/// The shell condition "the checkout has been moved aside": a hidden
/// `.wt-remove-*` sibling exists.
fn moved_condition(wts: &Path) -> String {
    format!("ls -d '{}'/.wt-remove-* >/dev/null 2>&1", wts.display())
}

/// A writer still using the checkout's old directory after the move (a
/// retained working directory) writes into the moved checkout. Removal stops
/// with exit 1 and both locations; the new file and the checkout's registration
/// at the moved path are kept and no branch is touched. A retry by the
/// original name finds the pending removal, lists the new file as work that
/// needs fresh consent, and refuses without it; with `--force-worktree` it
/// finishes the removal and the branch.
#[test]
fn a_write_into_the_moved_checkout_stops_then_a_retry_needs_fresh_consent() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.writer(
        &moved_condition(&fixture.wts()),
        &format!("for q in '{}'/.wt-remove-*; do echo late > \"$q/new-work.txt\"; done", fixture.wts().display()),
    );

    let output = fixture.remove(&["feat-x"], Some(&path));

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty(), "no protocol lines after a stop");
    assert!(stderr.contains("Removal stopped:"), "{stderr}");
    assert!(stderr.contains(&format!("Original: {}", fs::canonicalize(fixture.wts()).unwrap().join("feat-x").display()))
        || stderr.contains(&format!("Original: {}", wt.display())), "{stderr}");
    assert!(stderr.contains("Moved to: "), "{stderr}");
    assert!(stderr.contains("Remaining: "), "{stderr}");
    assert!(stderr.contains("Git registration: now points to the moved directory"), "{stderr}");
    assert!(stderr.contains("Branch feat/x: kept"), "{stderr}");
    assert!(stderr.contains("Stop any tasks using either path."), "{stderr}");
    assert!(stderr.contains("Retry: wt remove feat-x"), "{stderr}");
    let report = saved_report(&stderr);
    assert_eq!(report["outcome"], "activity_detected", "{report}");
    assert_eq!(report["registration"], "at_quarantine", "{report}");
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    assert_eq!(fs::read_to_string(quarantine.join("new-work.txt")).unwrap(), "late\n");
    assert!(!wt.exists(), "nothing is moved back");
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 1);

    // Without consent for the new file, the retry refuses and keeps it.
    let refused = fixture.remove(&["feat-x"], None);
    let stderr = stderr_of(&refused);
    assert_eq!(refused.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Continuing an earlier removal of feat-x"), "{stderr}");
    assert!(stderr.contains("new-work.txt"), "the new file is reported: {stderr}");
    assert!(stderr.contains("Nothing was removed."), "{stderr}");
    assert!(quarantine.join("new-work.txt").exists());
    assert_eq!(fixture.pending(), 1);

    let finished = fixture.remove(&["feat-x", "--force-worktree"], None);
    let stderr = stderr_of(&finished);
    assert_eq!(finished.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed worktree feat-x"), "{stderr}");
    assert!(stderr.contains("Deleted branch feat/x"), "{stderr}");
    assert!(finished.stdout.is_empty());
    assert!(!quarantine.exists());
    assert!(!fixture.listed().contains("feat/x"), "{}", fixture.listed());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0, "completion closes the record");
}

/// A process recreates the checkout's original path by absolute path right
/// after the move. Removal stops with exit 1: the recreated content is never
/// touched, the moved checkout is kept, and the branch is kept. A retry
/// refuses (exit 3) while something is at the original path; once the user
/// moves it away, the retry finishes the removal and the branch step.
#[test]
fn recreation_at_the_original_path_stops_and_is_never_cleaned_up() {
    let (fixture, wt) = Fixture::new("chore/refilled", "refilled");
    let leftover = wt.join("target").join("flycheck0");
    let path = fixture.writer(
        &moved_condition(&fixture.wts()),
        &format!("mkdir -p '{}' && echo built > '{}/x'", leftover.display(), leftover.display()),
    );

    let output = fixture.remove(&["refilled"], Some(&path));

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("Removal stopped:"), "{stderr}");
    assert!(stderr.contains("(something new is here; it was not touched)"), "{stderr}");
    assert!(stderr.contains("Moved to: "), "{stderr}");
    assert!(stderr.contains("Branch chore/refilled: kept"), "{stderr}");
    assert!(stderr.contains("Retry: wt remove refilled"), "{stderr}");
    assert!(stderr.contains("Run it from the base checkout at "), "{stderr}");
    let report = saved_report(&stderr);
    assert_eq!(report["original_state"], "recreated", "{report}");
    assert_eq!(fs::read_to_string(leftover.join("x")).unwrap(), "built\n");
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    assert!(quarantine.join("README.md").exists());
    assert!(fixture.branch_exists("chore/refilled"));
    assert_eq!(fixture.pending(), 1);

    let refused = fixture.remove(&["refilled", "--force-worktree", "--force-branch"], None);
    let stderr = stderr_of(&refused);
    assert_eq!(refused.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Can't continue the removal of"), "{stderr}");
    assert!(stderr.contains("something new is at the worktree's original path"), "{stderr}");
    assert!(stderr.contains("Nothing was deleted or repaired by this run."), "{stderr}");
    assert_eq!(fs::read_to_string(leftover.join("x")).unwrap(), "built\n", "never cleaned up");
    assert!(quarantine.join("README.md").exists(), "neither tree is cleaned up");
    assert!(fixture.branch_exists("chore/refilled"));

    let saved = fixture.root.path().join("saved");
    fs::rename(&wt, &saved).unwrap();
    let finished = fixture.remove(&["refilled"], None);
    let stderr = stderr_of(&finished);
    assert_eq!(finished.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Continuing an earlier removal of refilled"), "{stderr}");
    assert!(stderr.contains("Removed worktree refilled"), "{stderr}");
    assert!(stderr.contains("Deleted branch chore/refilled"), "{stderr}");
    assert!(!quarantine.exists());
    assert!(!fixture.branch_exists("chore/refilled"));
    assert_eq!(fixture.pending(), 0);
    assert_eq!(fs::read_to_string(saved.join("target/flycheck0/x")).unwrap(), "built\n");
}

/// A new worktree at the original path while the old removal is pending:
/// the name matches both, so both are listed and neither is touched (exit 3).
#[test]
fn a_new_worktree_at_the_original_path_is_ambiguous_with_the_pending_removal() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.writer(
        &moved_condition(&fixture.wts()),
        &format!("for q in '{}'/.wt-remove-*; do echo late > \"$q/new-work.txt\"; done", fixture.wts().display()),
    );
    assert_eq!(fixture.remove(&["feat-x"], Some(&path)).status.code(), Some(1));
    git(&fixture.repo(), &["worktree", "add", "-q", "-b", "feat/again", wt.to_str().unwrap(), "main"]);

    let output = fixture.remove(&["feat-x", "--force-worktree", "--force-branch"], None);

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("feat-x matches a pending removal and another worktree"), "{stderr}");
    assert!(stderr.contains("worktree at ") && stderr.contains("(branch feat/again)"), "{stderr}");
    assert!(stderr.contains("pending removal of "), "{stderr}");
    assert!(wt.join("README.md").exists(), "the new worktree is untouched");
    assert!(fixture.quarantine().unwrap().join("new-work.txt").exists(), "the moved checkout is untouched");
    assert!(fixture.branch_exists("feat/x") && fixture.branch_exists("feat/again"));
}
