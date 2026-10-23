//! `wt remove` stopping after the checkout moved, and the retry by the
//! original name, through the real binary.
//!
//! A controlled writer is a `git` shim first on `wt`'s `PATH`: it runs the
//! real Git, then, once, writes where a stray process would. The shim fires
//! on a Git call the removal makes after the move, so the write reaches the
//! deletion race rather than the preflight usage check. `wt` would read that
//! call's answer (the worktree listing) in-process, so every run here sets
//! `worktree::git::TEST_LIST_THROUGH_GIT_VAR` to make it a Git call again. Unix only: on Windows
//! `wt` resolves `git.exe`, so a script cannot stand in; the library's
//! lifecycle tests cover the same stops on every OS.
//!
//! The same shim delivers `SIGINT` to `wt` (its parent) at a chosen Git call
//! and then waits until `wt` acknowledges it on stderr, so the interrupt is
//! known to have arrived before the removal goes on: no timing is assumed.
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
    /// stderr acknowledges it. Returns the `PATH` to run `wt` with.
    fn interrupter(&self, bin_name: &str, when: &str) -> String {
        let acknowledge = format!(
            "kill -INT $PPID; i=0; until grep -q 'Interrupted: stopping' '{}' || [ $i -ge 1000 ]; do sleep 0.01; i=$((i+1)); done",
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

/// The shell condition "this Git call reads an index for the lifecycle's
/// binding": the last Git call before the move in a first removal, and
/// before deletion in a retry. Nothing outside the lifecycle runs it.
const BINDING_CALL: &str = r#"case "$*" in *"ls-files --stage --debug"*) true;; *) false;; esac"#;

/// Ctrl-C after the checkout moved: `wt` is not killed. It stops at the next
/// entry boundary with exit 1, prints the stopped report with its saved
/// report and retry guidance, and keeps the moved checkout and the branch. A
/// retry by the original name finishes the removal.
#[test]
fn an_interrupt_after_the_move_stops_with_a_report_and_a_retry_finishes() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.interrupter("bin", &moved_condition(&fixture.wts()));

    let (status, stderr) = fixture.remove_to_file(&["feat-x"], &path);

    assert_eq!(status.signal(), None, "not killed by the signal: {stderr}");
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Interrupted: stopping at the next safe point"), "{stderr}");
    assert!(stderr.contains("Removal stopped:") && stderr.contains("the removal was interrupted"), "{stderr}");
    assert!(stderr.contains("Moved to: ") && stderr.contains("Remaining: "), "{stderr}");
    assert!(stderr.contains("Branch feat/x: kept"), "{stderr}");
    assert!(stderr.contains("Retry: wt remove feat-x"), "{stderr}");
    let report = saved_report(&stderr);
    assert_eq!(report["outcome"], "cleanup_failed", "{report}");
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    assert_eq!(fs::read_to_string(quarantine.join("README.md")).unwrap(), "readme\n", "remaining content is kept");
    assert!(!wt.exists());
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 1);

    let finished = fixture.remove(&["feat-x"], None);
    let stderr = stderr_of(&finished);
    assert_eq!(finished.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Continuing an earlier removal of feat-x"), "{stderr}");
    assert!(stderr.contains("Removed worktree feat-x"), "{stderr}");
    assert!(!quarantine.exists());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}

/// Ctrl-C during a retry of a moved checkout stops it the same way, and the
/// next retry finishes.
#[test]
fn an_interrupt_during_a_quarantine_retry_stops_with_a_report() {
    let (fixture, _) = Fixture::new("feat/x", "feat-x");
    let first = fixture.interrupter("bin", &moved_condition(&fixture.wts()));
    assert_eq!(fixture.remove_to_file(&["feat-x"], &first).0.code(), Some(1));
    let quarantine = fixture.quarantine().expect("the moved checkout is kept");
    let retry = fixture.interrupter("bin-retry", BINDING_CALL);

    let (status, stderr) = fixture.remove_to_file(&["feat-x"], &retry);

    assert_eq!(status.signal(), None, "not killed by the signal: {stderr}");
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Continuing an earlier removal of feat-x"), "{stderr}");
    assert!(stderr.contains("Removal stopped:") && stderr.contains("the removal was interrupted"), "{stderr}");
    assert!(stderr.contains("Full report: "), "{stderr}");
    assert!(quarantine.join("README.md").is_file(), "remaining content is kept");
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 1);

    let finished = fixture.remove(&["feat-x"], None);
    assert_eq!(finished.status.code(), Some(0), "{}", stderr_of(&finished));
    assert!(!quarantine.exists());
    assert_eq!(fixture.pending(), 0);
}

/// Ctrl-C before the move (here, while the lifecycle binds the index) stops
/// with nothing moved and no pending removal: exit 1, the checkout and branch
/// in place.
#[test]
fn an_interrupt_before_the_move_removes_nothing() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.interrupter("bin", BINDING_CALL);

    let (status, stderr) = fixture.remove_to_file(&["feat-x"], &path);

    assert_eq!(status.signal(), None, "not killed by the signal: {stderr}");
    assert_eq!(status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Nothing was removed."), "{stderr}");
    assert!(stderr.contains("interrupted before it changed anything"), "{stderr}");
    assert!(wt.join("README.md").is_file());
    assert!(fixture.quarantine().is_none());
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}

/// A writer still using the checkout's old directory after the move (a
/// retained working directory) writes into the moved checkout. The user
/// agreed to the removal, so the new file is deleted with the rest, the
/// removal and its branch step finish (exit 0), and a warning names it.
#[test]
fn a_write_into_the_moved_checkout_is_deleted_and_reported() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.writer(
        &moved_condition(&fixture.wts()),
        &format!("for q in '{}'/.wt-remove-*; do echo late > \"$q/new-work.txt\"; done", fixture.wts().display()),
    );

    let output = fixture.remove(&["feat-x"], Some(&path));

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed worktree feat-x"), "{stderr}");
    assert!(stderr.contains("after you agreed; wt removed the worktree anyway."), "{stderr}");
    assert!(stderr.contains("new-work.txt"), "the new file is named: {stderr}");
    assert!(stderr.contains("Full report: "), "{stderr}");
    assert!(fixture.quarantine().is_none(), "the moved checkout is gone");
    assert!(!wt.exists());
    assert!(!fixture.listed().contains("feat/x"), "{}", fixture.listed());
    assert!(!fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0, "completion closes the record");
}

/// A process recreates the checkout's original path by absolute path right
/// after the move. That is outside what the user agreed to: it is never
/// touched, a warning names it, and the moved checkout, its record, and the
/// branch are removed (exit 0).
#[test]
fn recreation_at_the_original_path_is_reported_and_never_cleaned_up() {
    let (fixture, wt) = Fixture::new("chore/refilled", "refilled");
    let leftover = wt.join("target").join("flycheck0");
    let path = fixture.writer(
        &moved_condition(&fixture.wts()),
        &format!("mkdir -p '{}' && echo built > '{}/x'", leftover.display(), leftover.display()),
    );

    let output = fixture.remove(&["refilled"], Some(&path));

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Removed worktree refilled"), "{stderr}");
    assert!(stderr.contains("(outside the worktree; not touched)"), "{stderr}");
    assert!(stderr.contains("Deleted branch chore/refilled"), "{stderr}");
    assert_eq!(fs::read_to_string(leftover.join("x")).unwrap(), "built\n", "never cleaned up");
    assert!(fixture.quarantine().is_none());
    assert!(!fixture.branch_exists("chore/refilled"));
    assert_eq!(fixture.pending(), 0);
}

/// A new worktree at the original path while the old removal is pending:
/// the name matches both, so both are listed and neither is touched (exit 3).
#[test]
fn a_new_worktree_at_the_original_path_is_ambiguous_with_the_pending_removal() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let path = fixture.interrupter("bin", &moved_condition(&fixture.wts()));
    let (status, stderr) = fixture.remove_to_file(&["feat-x"], &path);
    assert_eq!(status.code(), Some(1), "an interrupt leaves the removal pending: {stderr}");
    git(&fixture.repo(), &["worktree", "add", "-q", "-b", "feat/again", wt.to_str().unwrap(), "main"]);

    let output = fixture.remove(&["feat-x", "--force-worktree", "--force-branch"], None);

    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("feat-x matches a pending removal and another worktree"), "{stderr}");
    assert!(stderr.contains("worktree at ") && stderr.contains("(branch feat/again)"), "{stderr}");
    assert!(stderr.contains("pending removal of "), "{stderr}");
    assert!(wt.join("README.md").exists(), "the new worktree is untouched");
    assert!(fixture.quarantine().unwrap().join("README.md").exists(), "the moved checkout is untouched");
    assert!(fixture.branch_exists("feat/x") && fixture.branch_exists("feat/again"));
}

/// A first removal of `feat-x` that deleted the clean tracked file
/// `deep/sub/tracked.txt`, then stopped one level up on a directory it
/// cannot write into (`locked/`, made writable again afterwards). Returns the
/// moved checkout, or `None` when this user can write into a read-only
/// directory and no such stop can be made.
fn stopped_after_deleting_a_tracked_file(fixture: &Fixture, wt: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt;

    for file in ["deep/sub/tracked.txt", "locked/kept.txt"] {
        fs::create_dir_all(wt.join(file).parent().unwrap()).unwrap();
        fs::write(wt.join(file), "committed\n").unwrap();
    }
    git(wt, &["add", "deep", "locked"]);
    git(wt, &["commit", "-q", "-m", "nested files"]);
    let locked = wt.join("locked");
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o555)).unwrap();
    if fs::write(locked.join("probe"), "").is_ok() {
        fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
        eprintln!("skipped: this user can write into a read-only directory");
        return None;
    }

    let output = fixture.remove(&["feat-x"], None);

    let quarantine = fixture.quarantine();
    for dir in [Some(wt.to_path_buf()), quarantine.clone()].into_iter().flatten() {
        let _ = fs::set_permissions(dir.join("locked"), fs::Permissions::from_mode(0o755));
    }
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removal stopped:"), "{stderr}");
    let quarantine = quarantine.expect("the moved checkout is kept");
    assert!(!quarantine.join("deep/sub/tracked.txt").exists(), "deleted before the stop");
    assert!(quarantine.join("locked/kept.txt").is_file(), "kept by the stop");
    assert_eq!(fixture.pending(), 1);
    Some(quarantine)
}

/// The retry after a stop that deleted a tracked file: Git lists the file as
/// an unstaged deletion, but this removal's own confirmed deletion explains
/// it, so the noninteractive retry needs no `--force-worktree` and says how
/// many files it did not ask about.
#[test]
fn a_retry_needs_no_consent_for_files_the_removal_already_deleted() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let Some(quarantine) = stopped_after_deleting_a_tracked_file(&fixture, &wt) else { return };
    // An unstaged deletion and nothing staged: porcelain ` D`.
    assert_eq!(git(&quarantine, &["diff", "--name-status"]), "D\tdeep/sub/tracked.txt");
    assert_eq!(git(&quarantine, &["diff", "--cached", "--name-status"]), "");

    let finished = fixture.remove(&["feat-x"], None);

    let stderr = stderr_of(&finished);
    assert_eq!(finished.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Continuing an earlier removal of feat-x"), "{stderr}");
    assert!(stderr.contains("1 tracked file this removal already deleted is not asked about again."), "{stderr}");
    assert!(!stderr.contains("tracked.txt"), "not listed as work to discard: {stderr}");
    assert!(stderr.contains("Removed worktree feat-x"), "{stderr}");
    assert!(!quarantine.exists());
    // The fixture's commit is not merged, so the branch is kept without
    // `--force-branch`; only the checkout's consent changed.
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 0);
}

/// The same retry with new work in the moved checkout still asks for it:
/// without `--force-worktree` it refuses (exit 3) and keeps everything.
#[test]
fn a_retry_with_new_work_beside_files_the_removal_already_deleted_refuses() {
    let (fixture, wt) = Fixture::new("feat/x", "feat-x");
    let Some(quarantine) = stopped_after_deleting_a_tracked_file(&fixture, &wt) else { return };
    fs::write(quarantine.join("new-work.txt"), "written after the stop\n").unwrap();

    let refused = fixture.remove(&["feat-x"], None);

    let stderr = stderr_of(&refused);
    assert_eq!(refused.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("new-work.txt"), "the new file is reported: {stderr}");
    assert!(!stderr.contains("tracked.txt"), "{stderr}");
    assert!(stderr.contains("Nothing was removed."), "{stderr}");
    assert_eq!(fs::read_to_string(quarantine.join("new-work.txt")).unwrap(), "written after the stop\n");
    assert!(quarantine.join("locked/kept.txt").is_file());
    assert!(fixture.branch_exists("feat/x"));
    assert_eq!(fixture.pending(), 1);
}
