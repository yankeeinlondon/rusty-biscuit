//! `wt remove` interrupted or raced by another process, and the retry by the
//! original name, through the real binary.
//!
//! A controlled writer is a `git` shim first on `wt`'s `PATH`: it runs the
//! real Git, then, once, writes where a stray process would. The shim fires
//! on a chosen Git call. In a first removal those calls are, in order: the
//! question's `git status` in the checkout, the branch checks, the
//! lifecycle's `git worktree list` calls around the move, its last `git
//! status` in the moved checkout just before deletion (R14), then `git
//! worktree remove` and the branch deletion. Deletion itself runs in
//! process, so a write fired at that last `git status` reaches the moved
//! checkout before deletion starts. `wt` would read the worktree listings
//! in-process, so every run here sets
//! `worktree::git::TEST_LIST_THROUGH_GIT_VAR` to make them Git calls again.
//! Unix only: on Windows `wt` resolves `git.exe`, so a script cannot stand
//! in; the library's lifecycle tests cover the same cases on every OS.
//!
//! The same shim delivers `SIGINT` to `wt` (its parent) at a chosen Git call
//! and then waits until `wt` acknowledges it on stderr (or has exited), so
//! the interrupt is known to have arrived before the removal goes on: no
//! timing is assumed.
//!
//! Every run has stdin redirected and `CI`/`WT_SHELL_WRAPPER` removed, so it
//! is non-interactive and wrapper-less, and sets `NO_COLOR`.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::os::unix::process::ExitStatusExt as _;
use std::process::{Command, ExitStatus, Output, Stdio};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = worktree::git::command().current_dir(dir).args(args).env("GIT_TERMINAL_PROMPT", "0").output().unwrap();
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
            .env(worktree::git::TEST_LIST_THROUGH_GIT_VAR, "1")
            .stdin(std::process::Stdio::null());
        if let Some(path) = path {
            command.env("PATH", path);
        }
        command.output().unwrap()
    }

    /// `wt remove <args>` like [`Fixture::remove`], with stderr written to
    /// [`Fixture::stderr_file`] so an [`Fixture::interrupter`] can read it.
    fn remove_to_file(&self, args: &[&str], path: &str) -> (ExitStatus, String) {
        let file = fs::File::create(self.stderr_file()).unwrap();
        let status = Command::new(assert_cmd::cargo::cargo_bin("wt"))
            .current_dir(self.repo())
            .arg("remove")
            .args(args)
            .env_remove("WT_SHELL_WRAPPER")
            .env_remove("CI")
            .env_remove("COMPLETE")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("NO_COLOR", "1")
            .env(worktree::git::TEST_LIST_THROUGH_GIT_VAR, "1")
            .env("PATH", path)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(file)
            .status()
            .unwrap();
        (status, fs::read_to_string(self.stderr_file()).unwrap())
    }

    fn stderr_file(&self) -> PathBuf {
        self.root.path().join("stderr.txt")
    }

    /// A `git` shim in `bin_name` that, the first time `when` holds after a
    /// Git call, sends `SIGINT` to `wt` and waits (at most 10 s) until `wt`'s
    /// stderr acknowledges it or `wt` has exited (an interrupt before `wt`
    /// handles them ends it). Returns the `PATH` to run `wt` with.
    fn interrupter(&self, bin_name: &str, when: &str) -> String {
        let acknowledge = format!(
            "kill -INT $PPID; i=0; until grep -q 'Interrupted: stopping' '{}' || ! kill -0 $PPID 2>/dev/null || [ $i -ge 1000 ]; \
            do sleep 0.01; i=$((i+1)); done",
            self.stderr_file().display()
        );
        self.shim(bin_name, when, &acknowledge)
    }

    /// A `git` shim on `PATH` that runs the real Git and then, the first time
    /// `when` (a shell condition, evaluated after the call) holds, runs
    /// `write`. Returns the `PATH` to run `wt` with.
    fn writer(&self, when: &str, write: &str) -> String {
        self.shim("bin", when, write)
    }

    fn shim(&self, bin_name: &str, when: &str, write: &str) -> String {
        use std::os::unix::fs::PermissionsExt;

        let real = String::from_utf8(Command::new("sh").args(["-c", "command -v git"]).output().unwrap().stdout).unwrap();
        let bin = self.root.path().join(bin_name);
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
        worktree::git::command()
            .current_dir(self.repo())
            .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
            .status()
            .unwrap()
            .success()
    }

    fn listed(&self) -> String {
        git(&self.repo(), &["worktree", "list", "--porcelain"])
    }

    /// Saved operations in the removal store: pending removals. The store's
    /// `locks` directory is not one.
    fn pending(&self) -> usize {
        let store = self.repo().join(".git").join(worktree::remove::recovery::store::STORE_DIR);
        let Ok(entries) = fs::read_dir(store) else { return 0 };
        entries.filter_map(Result::ok).filter(|entry| entry.file_name() != "locks").count()
    }
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The shell condition "this Git call is the lifecycle's last `git status`,
/// in the moved checkout": after the move, just before deletion.
const STATUS_BEFORE_DELETION: &str = r#"case "$*" in *"/.wt-remove-"*" status "*) true;; *) false;; esac"#;

/// The shell condition "this Git call is the first branch check": after the
/// question's `git status`, before the report is printed and before `wt`
/// handles interrupts.
const BRANCH_CHECK: &str = r#"case "$*" in *" for-each-ref "*) true;; *) false;; esac"#;

/// The shell condition "this Git call is the lifecycle's first worktree
/// listing": after the report and the decision, before the move. `wt`'s own
/// listing of the name runs from the base without `-C`.
const LIFECYCLE_LISTING: &str = r#"case "$*" in "-C "*" worktree list --porcelain") true;; *) false;; esac"#;

/// Text that never appears in a normal removal's output (R19, R26).
fn assert_no_technical_output(stderr: &str) {
    for technical in [".wt-remove-", "Retry:", "report.json", "Full report", "journal", "watch"] {
        assert!(!stderr.contains(technical), "{technical:?} in {stderr}");
    }
}

/// Ctrl-C after the checkout moved: `wt` is not killed. Deletion sees the
/// interrupt before its first entry, so the run ends with exit 1 and one
/// plain sentence, keeping the moved checkout, its record, and the branch. A
/// retry by the original name finishes it silently with the earlier
/// approval (R19).
#[test]
fn an_interrupt_after_the_move_cancels_and_a_retry_by_name_finishes_silently() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.interrupter("bin", STATUS_BEFORE_DELETION);

    let (status, stderr) = fixture.remove_to_file(&["feat-x"], &path);

    assert_eq!(status.signal(), None, "not killed by the signal: {stderr}");
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Interrupted: stopping at the next safe point"), "{stderr}");
    assert!(stderr.contains("The removal was cancelled; run wt remove again to finish it."), "{stderr}");
    assert_no_technical_output(&stderr);
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    assert_eq!(fs::read_to_string(quarantine.join("README.md")).unwrap(), "readme\n", "remaining content is kept");
    assert!(!wt.exists());
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 1);

    let finished = fixture.remove(&["feat-x"], None);

    let stderr = stderr_of(&finished);
    assert_eq!(finished.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x (") && stderr.contains(" and deleted branch feat/x."), "{stderr}");
    assert!(!stderr.contains("Delete it?") && !stderr.contains("--force"), "asks nothing: {stderr}");
    assert_no_technical_output(&stderr);
    assert!(finished.stdout.is_empty());
    assert!(!quarantine.exists());
    assert!(!fixture.listed().contains("feat-x"), "{}", fixture.listed());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0, "completion leaves no record");
}

/// The shell condition "this Git call is the lifecycle's `git worktree
/// unlock`": after the approval is saved, before anything is deleted.
const UNLOCK: &str = r#"case "$*" in *" worktree unlock "*) true;; *) false;; esac"#;

/// R9 and R19 for a link to the worktree's own checkout: its approval is
/// saved like a checkout's, so Ctrl-C after the approved unlock keeps the
/// link, its checkout, and the branch, and a retry by name finishes both
/// without asking.
#[test]
fn an_interrupted_own_link_removal_is_finished_by_a_retry_without_a_question() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    git(&fixture.repo(), &["worktree", "lock", wt.to_str().unwrap()]);
    let real = fixture.wts().join("real");
    fs::rename(&wt, &real).unwrap();
    std::os::unix::fs::symlink(&real, &wt).unwrap();
    let path = fixture.interrupter("bin", UNLOCK);

    let (status, stderr) = fixture.remove_to_file(&["feat-x", "--force", "--force-branch"], &path);

    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("The removal was cancelled; run wt remove again to finish it."), "{stderr}");
    assert!(real.join("README.md").is_file(), "the checkout is kept");
    assert!(fs::symlink_metadata(&wt).unwrap().file_type().is_symlink(), "the link is kept");
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 1);

    let finished = fixture.remove(&["feat-x"], None);

    let stderr = stderr_of(&finished);
    assert_eq!(finished.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x (") && stderr.contains("deleted branch feat/x"), "{stderr}");
    assert!(!stderr.contains("Delete it?") && !stderr.contains("--force"), "asks nothing: {stderr}");
    assert_no_technical_output(&stderr);
    assert!(fs::symlink_metadata(&wt).is_err() && !real.exists(), "both are deleted");
    assert!(!fixture.listed().contains("feat-x"), "{}", fixture.listed());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}

/// An interrupted removal whose saved record has since become unreadable is
/// explained by name, never resumed, and nothing more is deleted: the moved
/// checkout, the record, and the branch stay as they were (exit 1).
#[test]
fn an_unreadable_record_of_an_interrupted_removal_is_explained_and_nothing_is_deleted() {
    use worktree::remove::recovery::store::{RECORD_FILE, STORE_DIR};

    let (fixture, _wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.interrupter("bin", STATUS_BEFORE_DELETION);
    let (status, stderr) = fixture.remove_to_file(&["feat-x"], &path);
    assert_eq!(status.code(), Some(1), "{stderr}");
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    let store = fixture.repo().join(".git").join(STORE_DIR);
    let operation = fs::read_dir(&store).unwrap().filter_map(Result::ok).find(|entry| entry.file_name() != "locks").unwrap();
    let record = operation.path().join(RECORD_FILE);
    fs::write(&record, b"{\"record_version\": 2,").unwrap();

    let output = fixture.remove(&["feat-x"], None);

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Nothing was removed. The record of an earlier removal could not be read"), "{stderr}");
    assert!(stderr.contains(&*operation.file_name().to_string_lossy()) && stderr.contains("malformed"), "{stderr}");
    assert!(!stderr.contains("Delete it?") && !stderr.contains("not found"), "{stderr}");
    assert!(output.stdout.is_empty());
    assert_eq!(fs::read_to_string(quarantine.join("README.md")).unwrap(), "readme\n", "nothing more is deleted");
    assert_eq!(fs::read(&record).unwrap(), b"{\"record_version\": 2,", "the record is kept as it was");
    assert!(fixture.branch_exists("feat/x"));
}

/// Ctrl-C before `wt` starts removing (here, during the branch checks that
/// precede the report) keeps the platform default: `wt` ends by the signal
/// and nothing is saved, moved, or deleted.
#[test]
fn an_interrupt_before_the_removal_starts_ends_wt_and_changes_nothing() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.interrupter("bin", BRANCH_CHECK);

    let (status, stderr) = fixture.remove_to_file(&["feat-x"], &path);

    assert_eq!(status.signal(), Some(SIGINT), "ended by the signal: {status:?} {stderr}");
    assert!(!stderr.contains("Removed"), "{stderr}");
    assert!(wt.join("README.md").is_file());
    assert!(fixture.quarantine().is_none());
    assert!(fixture.listed().contains("feat-x"), "{}", fixture.listed());
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}

/// `SIGINT`'s number, 2 on every Unix.
const SIGINT: i32 = 2;

/// A writer still using the checkout's old directory after the move (a
/// retained working directory) writes into the moved checkout just before
/// deletion. The file is deleted with the checkout (R20): no stop, exit 0,
/// nothing left. It was written after the last `git status`, so the final
/// line does not name it (R14 never claims to be complete).
#[test]
fn a_write_into_the_moved_checkout_just_before_deletion_is_deleted() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.writer(
        STATUS_BEFORE_DELETION,
        &format!("for q in '{}'/.wt-remove-*; do echo late > \"$q/new-work.txt\"; done", fixture.wts().display()),
    );

    let output = fixture.remove(&["feat-x"], Some(&path));

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("Removed feat-x (") && stderr.contains(" and deleted branch feat/x."), "{stderr}");
    assert!(!stderr.contains("new-work.txt"), "{stderr}");
    assert_no_technical_output(&stderr);
    assert!(fs::read_dir(fixture.wts()).unwrap().next().is_none(), "nothing is left beside the worktree");
    assert!(!wt.exists());
    assert!(!fixture.listed().contains("feat-x"), "{}", fixture.listed());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}

/// R14: a file created in the checkout after the question's `git status`
/// (here at the lifecycle's first listing, after the report) is deleted with
/// it, and the final line names it because the lifecycle's last `git status`
/// lists it.
#[test]
fn a_file_created_after_the_report_is_deleted_and_named_in_the_final_line() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.writer(LIFECYCLE_LISTING, &format!("echo new > '{}'", wt.join("new.txt").display()));

    let output = fixture.remove(&["feat-x"], Some(&path));

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("uncommitted file"), "the question did not show it: {stderr}");
    assert!(
        stderr.contains("Removed feat-x (") && stderr.contains("; also deleted 1 file changed after you answered: new.txt."),
        "{stderr}"
    );
    assert!(!wt.exists());
    assert!(fixture.quarantine().is_none());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}

/// A process recreates the checkout's original path by absolute path right
/// after the move. The recreated content is never touched; the moved
/// checkout, its registration, and the branch are deleted, and the run says
/// what it kept.
#[test]
fn recreation_at_the_original_path_is_kept_and_the_moved_checkout_deleted() {
    let (fixture, wt) = Fixture::new("chore/refilled", "refilled");
    let leftover = wt.join("target").join("flycheck0");
    let path = fixture.writer(
        STATUS_BEFORE_DELETION,
        &format!("mkdir -p '{}' && echo built > '{}/x'", leftover.display(), leftover.display()),
    );

    let output = fixture.remove(&["refilled"], Some(&path));

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("Removed refilled (") && stderr.contains(" and deleted branch chore/refilled."), "{stderr}");
    assert!(stderr.contains("New content found at ") && stderr.contains("after the worktree was moved away was kept."), "{stderr}");
    assert_no_technical_output(&stderr);
    assert_eq!(fs::read_to_string(leftover.join("x")).unwrap(), "built\n", "never cleaned up");
    assert!(fixture.quarantine().is_none(), "the moved checkout is deleted");
    assert!(!fixture.listed().contains("chore/refilled"), "{}", fixture.listed());
    assert!(!fixture.branch_exists("chore/refilled"));
    assert_eq!(fixture.pending(), 0);
}

/// A new worktree at the original path while the old removal is pending:
/// the name matches both, so both are listed and neither is touched (exit 3).
#[test]
fn a_new_worktree_at_the_original_path_is_ambiguous_with_the_pending_removal() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.interrupter("bin", STATUS_BEFORE_DELETION);
    assert_eq!(fixture.remove_to_file(&["feat-x"], &path).0.code(), Some(1));
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    git(&fixture.repo(), &["worktree", "add", "-q", "-b", "feat/again", wt.to_str().unwrap(), "main"]);

    let output = fixture.remove(&["feat-x", "--force", "--force-branch"], None);

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Nothing was removed."), "{stderr}");
    assert!(stderr.contains("feat-x matches more than one worktree or unfinished removal:"), "{stderr}");
    assert!(stderr.contains("worktree at "), "{stderr}");
    assert!(stderr.contains("an unfinished removal of the worktree that was at "), "{stderr}");
    assert!(wt.join("README.md").exists(), "the new worktree is untouched");
    assert!(quarantine.join("README.md").exists(), "the moved checkout is untouched");
    assert!(fixture.branch_exists("feat/x") && fixture.branch_exists("feat/again"));
    assert_eq!(fixture.pending(), 1);
}

/// A forced removal of a checkout with uncommitted work, interrupted after
/// the move. The retry runs without `--force` and without a terminal, yet
/// asks nothing and refuses nothing: the earlier approval covers the
/// uncommitted file (R19).
#[test]
fn a_retry_of_an_interrupted_removal_needs_no_consent_for_uncommitted_files() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    fs::write(wt.join("notes.txt"), "uncommitted\n").unwrap();
    let path = fixture.interrupter("bin", STATUS_BEFORE_DELETION);
    let (status, stderr) = fixture.remove_to_file(&["feat-x", "--force"], &path);
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("1 uncommitted file will be lost: notes.txt"), "{stderr}");
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    assert!(quarantine.join("notes.txt").is_file());

    let finished = fixture.remove(&["feat-x"], None);

    let stderr = stderr_of(&finished);
    assert_eq!(finished.status.code(), Some(0), "{stderr}");
    assert!(!stderr.contains("notes.txt"), "not reported again: {stderr}");
    assert!(!stderr.contains("Nothing was removed") && !stderr.contains("--force"), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(!quarantine.exists());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}

/// New work written into the moved checkout between an interrupted run and
/// its retry is deleted by the retry, which compares no content (R19, R20).
#[test]
fn a_retry_deletes_new_work_written_into_the_moved_checkout_after_the_interrupt() {
    let (fixture, _) = Fixture::new("feat/x", "feat-x");
    let path = fixture.interrupter("bin", STATUS_BEFORE_DELETION);
    assert_eq!(fixture.remove_to_file(&["feat-x"], &path).0.code(), Some(1));
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    fs::write(quarantine.join("new-work.txt"), "written after the interrupt\n").unwrap();

    let finished = fixture.remove(&["feat-x"], None);

    let stderr = stderr_of(&finished);
    assert_eq!(finished.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed feat-x ("), "{stderr}");
    assert!(!stderr.contains("Nothing was removed"), "{stderr}");
    assert!(!quarantine.exists());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}
