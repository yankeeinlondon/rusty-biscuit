//! `wt remove` through the real binary: the flag surface, the spec's Examples
//! table, exit codes, and the move-first handoff.
//!
//! Every run has stdin redirected and `CI`/`WT_SHELL_WRAPPER` removed, so it
//! is non-interactive and wrapper-less unless a test says otherwise, and sets
//! `NO_COLOR`. Remotes
//! are local bare repositories: the PR lookup answers "unsupported host" at
//! once and the live checks run `git ls-remote` against the bare repository.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use predicates::prelude::*;

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git should be installed");
    assert!(
        out.status.success(),
        "git {args:?} in {dir:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn configure(dir: &Path) {
    for (key, value) in [
        ("user.email", "test@example.com"),
        ("user.name", "Test User"),
        ("commit.gpgsign", "false"),
        ("gc.auto", "0"),
    ] {
        git(dir, &["config", key, value]);
    }
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

struct Fixture {
    root: tempfile::TempDir,
}

impl Fixture {
    /// `root/repo` on `main` with one commit; worktrees go in `root/wts`.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        configure(&repo);
        let this = Self { root };
        this.commit(&this.repo(), "README.md");
        this
    }

    /// [`Fixture::new`] plus a bare `origin` holding `main`.
    fn with_origin() -> Self {
        let this = Self::new();
        let origin = this.origin();
        git(this.root.path(), &["init", "-q", "--bare", "-b", "main", origin.to_str().unwrap()]);
        git(&this.repo(), &["remote", "add", "origin", origin.to_str().unwrap()]);
        git(&this.repo(), &["push", "-q", "-u", "origin", "main"]);
        this
    }

    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn origin(&self) -> PathBuf {
        self.root.path().join("origin.git")
    }

    fn commit(&self, dir: &Path, file: &str) -> String {
        fs::write(dir.join(file), format!("{file}\n")).unwrap();
        git(dir, &["add", file]);
        git(dir, &["commit", "-q", "-m", &format!("add {file}")]);
        git(dir, &["rev-parse", "HEAD"])
    }

    fn add_worktree(&self, branch: &str, dir_name: &str) -> PathBuf {
        let path = self.root.path().join("wts").join(dir_name);
        git(&self.repo(), &["worktree", "add", "-q", "-b", branch, path.to_str().unwrap(), "main"]);
        path
    }

    fn wt(&self, cwd: &Path) -> assert_cmd::Command {
        let mut cmd = assert_cmd::Command::cargo_bin("wt").unwrap();
        cmd.current_dir(cwd)
            .env_remove("WT_SHELL_WRAPPER")
            .env_remove("CI")
            .env_remove("COMPLETE")
            .env("GIT_TERMINAL_PROMPT", "0")
            // Plain text, so needles never straddle an SGR sequence.
            .env("NO_COLOR", "1")
            .write_stdin("");
        cmd
    }

    fn branch_exists(&self, branch: &str) -> bool {
        Command::new("git")
            .current_dir(self.repo())
            .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
            .status()
            .unwrap()
            .success()
    }

    fn origin_has(&self, branch: &str) -> bool {
        !git(&self.repo(), &["ls-remote", "origin", &format!("refs/heads/{branch}")]).is_empty()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Ok(path) = worktree::fork_origin::fork_origin_path(&self.repo()) {
            let _ = fs::remove_file(path);
        }
    }
}

// --- Flag surface -----------------------------------------------------------

#[test]
fn retired_flags_are_clap_errors() {
    let fixture = Fixture::new();
    fixture.add_worktree("feat/x", "feat-x");
    for args in [
        &["remove", "feat/x", "-b"][..],
        &["remove", "feat/x", "--branch"],
        &["remove", "feat/x", "-f"],
        &["remove", "feat/x", "-ff"],
        &["remove", "feat/x", "--force"],
        &["remove", "feat/x", "--remove-branch"],
        &["remove", "feat/x", "--remove-remote"],
    ] {
        fixture
            .wt(&fixture.repo())
            .args(args)
            .assert()
            .code(2)
            .stderr(predicate::str::contains("unexpected argument"));
    }
    assert!(fixture.root.path().join("wts/feat-x").exists());
}

#[test]
fn help_lists_the_three_force_flags_and_hides_handoff() {
    assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .args(["remove", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force-worktree"))
        .stdout(predicate::str::contains("--force-branch"))
        .stdout(predicate::str::contains("--force-remote"))
        .stdout(predicate::str::contains("--handoff").not())
        .stdout(predicate::str::contains("-f,").not());
}

#[test]
fn handoff_conflicts_with_a_name_and_every_force_flag() {
    let token = "a".repeat(32);
    for extra in [
        &["feat/x"][..],
        &["--force-worktree"],
        &["--force-branch"],
        &["--force-remote"],
    ] {
        assert_cmd::Command::cargo_bin("wt")
            .unwrap()
            .args(["remove", "--handoff", &token])
            .args(extra)
            .assert()
            .code(2);
    }
    assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .args(["remove"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("required"));
}

#[test]
fn an_unknown_name_fails_and_the_base_checkout_is_refused() {
    let fixture = Fixture::new();
    fixture
        .wt(&fixture.repo())
        .args(["remove", "no-such-worktree"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("not found"));
    fixture
        .wt(&fixture.repo())
        .args(["remove", "base", "--force-worktree", "--force-branch"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("main checkout"));
}

// --- The spec's Examples table ----------------------------------------------

#[test]
fn example_clean_and_merged_removes_worktree_and_branch() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("chore/old-cleanup", "old-cleanup");
    fixture.commit(&wt, "cleanup.txt");
    git(&fixture.repo(), &["merge", "-q", "--no-ff", "-m", "merge", "chore/old-cleanup"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "old-cleanup"])
        .assert()
        .code(0)
        .stdout("")
        .stderr(predicate::str::contains("Safe"))
        .stderr(predicate::str::contains("its commits are on main"))
        .stderr(predicate::str::contains("Removed worktree"))
        .stderr(predicate::str::contains("Deleted branch"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("chore/old-cleanup"));
}

#[test]
fn example_pushed_without_a_pr_is_pretty_safe_and_names_the_origin_copy() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/dark-fixes", "feat-dark-fixes");
    fixture.commit(&wt, "dark.txt");
    git(&wt, &["push", "-q", "-u", "origin", "feat/dark-fixes"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-dark-fixes"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Pretty safe"))
        .stderr(predicate::str::contains("origin/feat/dark-fixes"))
        .stderr(predicate::str::contains("PR status unavailable"))
        .stderr(predicate::str::contains("Deleted branch"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/dark-fixes"));
    assert!(fixture.origin_has("feat/dark-fixes"), "origin is never touched without --force-remote");
}

#[test]
fn example_unique_commits_without_a_terminal_keep_the_branch_with_a_warning() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("spike/parser", "spike-parser");
    for file in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        fixture.commit(&wt, file);
    }

    fixture
        .wt(&fixture.repo())
        .args(["remove", "spike-parser"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Not safe"))
        .stderr(predicate::str::contains("4 commits exist nowhere else"))
        .stderr(predicate::str::contains("Warning:"))
        .stderr(predicate::str::contains("kept branch spike/parser"));
    assert!(!wt.exists(), "the worktree is still removed");
    assert!(fixture.branch_exists("spike/parser"));
}

#[test]
fn example_force_branch_deletes_unique_commits() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("spike/parser", "spike-parser");
    for file in ["a.txt", "b.txt", "c.txt", "d.txt"] {
        fixture.commit(&wt, file);
    }

    fixture
        .wt(&fixture.repo())
        .args(["remove", "spike-parser", "--force-branch"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Deleted branch"))
        .stderr(predicate::str::contains("4 commits lost"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("spike/parser"));
}

#[test]
fn example_dirty_files_without_a_terminal_refuse_with_nothing_removed() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("fix/wt-ux", "fix-wt-ux");
    fs::write(wt.join("README.md"), "changed\n").unwrap();
    fs::write(wt.join("lib.rs"), "fn a() {}\n").unwrap();
    fs::write(wt.join("notes.txt"), "keep\n").unwrap();

    for ci in [None, Some("true")] {
        let mut cmd = fixture.wt(&fixture.repo());
        if let Some(ci) = ci {
            cmd.env("CI", ci);
        }
        cmd.args(["remove", "fix-wt-ux"])
            .assert()
            .code(3)
            .stdout("")
            .stderr(predicate::str::contains("Uncommitted files (3)"))
            .stderr(predicate::str::contains("lib.rs"))
            .stderr(predicate::str::contains("Nothing was removed"))
            .stderr(predicate::str::contains("--force-worktree"));
    }
    assert!(wt.join("notes.txt").exists());
    assert!(fixture.branch_exists("fix/wt-ux"));
}

#[test]
fn example_all_three_flags_remove_worktree_branch_and_origin_branch() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("fix/wt-ux", "fix-wt-ux");
    fixture.commit(&wt, "fix.txt");
    git(&wt, &["push", "-q", "-u", "origin", "fix/wt-ux"]);
    fs::write(wt.join("scratch.txt"), "x\n").unwrap();

    fixture
        .wt(&fixture.repo())
        .args([
            "remove",
            "fix-wt-ux",
            "--force-worktree",
            "--force-branch",
            "--force-remote",
        ])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("origin/fix/wt-ux will be deleted"))
        .stderr(predicate::str::contains("Deleted origin/fix/wt-ux"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("fix/wt-ux"));
    assert!(!fixture.origin_has("fix/wt-ux"));
}

// --- Ignored entries, conflicts, and remote failures -------------------------

#[test]
fn ignored_entries_need_consent_like_dirty_files() {
    for (ignore, make) in [(".env", ".env"), ("target/", "target/debug/app")] {
        let fixture = Fixture::new();
        fs::write(fixture.repo().join(".gitignore"), format!("{ignore}\n")).unwrap();
        git(&fixture.repo(), &["add", ".gitignore"]);
        git(&fixture.repo(), &["commit", "-q", "-m", "ignore"]);
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let file = wt.join(make);
        fs::create_dir_all(file.parent().unwrap()).unwrap();
        fs::write(&file, "local only\n").unwrap();

        fixture
            .wt(&fixture.repo())
            .args(["remove", "feat-x"])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("Ignored files"))
            .stderr(predicate::str::contains("deletes"))
            .stderr(predicate::str::contains(ignore));
        assert!(file.exists(), "{ignore}");

        fixture
            .wt(&fixture.repo())
            .args(["remove", "feat-x", "--force-worktree"])
            .assert()
            .code(0);
        assert!(!wt.exists(), "{ignore}");
    }
}

#[test]
fn force_branch_on_a_dirty_worktree_without_force_worktree_is_a_conflict() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "unique.txt");
    fs::write(wt.join("scratch.txt"), "x\n").unwrap();

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-branch"])
        .assert()
        .code(3)
        .stderr(predicate::str::contains(
            "--force-branch needs the worktree removed first",
        ));
    assert!(wt.join("scratch.txt").exists());
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn force_remote_with_an_unreachable_origin_removes_locally_then_exits_1() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    git(&wt, &["push", "-q", "-u", "origin", "feat/x"]);
    git(&fixture.repo(), &["remote", "set-url", "origin", "/nonexistent/origin.git"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("could not be reached"))
        .stderr(predicate::str::contains("Removed worktree"))
        .stderr(predicate::str::contains("git push origin --delete feat/x"));
    assert!(!wt.exists(), "the local removal has already happened");
}

#[test]
fn force_remote_without_a_remote_branch_says_so() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("no matching remote branch"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/x"), "on main, so Safe");
}

/// A branch that was Pretty safe only because it was pushed becomes Not safe
/// under `--force-remote`, so the local copy survives.
#[test]
fn force_remote_keeps_a_branch_whose_only_other_copy_it_deletes() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");
    git(&wt, &["push", "-q", "-u", "origin", "feat/x"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Not safe"))
        .stderr(predicate::str::contains("kept branch feat/x"))
        .stderr(predicate::str::contains("Deleted origin/feat/x"));
    assert!(fixture.branch_exists("feat/x"));
    assert!(!fixture.origin_has("feat/x"));
}

/// `feat/x` has one unique commit, pushed only as origin's `main`, and
/// tracks `origin/main`, so `--force-remote` deletes `main` on origin. Local
/// `main` is behind.
fn feature_tracking_origin_main(fixture: &Fixture) -> (PathBuf, String) {
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "only-here.txt");
    git(&wt, &["push", "-q", "origin", "feat/x:main"]);
    git(&wt, &["branch", "-q", "--set-upstream-to=origin/main", "feat/x"]);
    git(&fixture.repo(), &["remote", "set-head", "origin", "main"]);
    // A bare repository refuses to delete its HEAD branch by default.
    git(&fixture.origin(), &["config", "receive.denyDeleteCurrent", "ignore"]);
    (wt, tip)
}

fn assert_kept_after_deleting_origin_main(fixture: &Fixture, wt: &Path, tip: &str) {
    assert!(!wt.exists());
    assert!(!fixture.origin_has("main"));
    assert!(fixture.branch_exists("feat/x"), "the only remaining copy");
    assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip);
}

/// The destination about to be deleted is `origin/main`, the default
/// branch's copy; it must not make the branch Safe.
#[test]
fn force_remote_of_the_upstream_default_branch_keeps_the_local_branch() {
    let fixture = Fixture::with_origin();
    let (wt, tip) = feature_tracking_origin_main(&fixture);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Not safe"))
        .stderr(predicate::str::contains("kept branch feat/x"))
        .stderr(predicate::str::contains("Deleted origin/main"));
    assert_kept_after_deleting_origin_main(&fixture, &wt, &tip);
}

#[test]
fn force_remote_of_the_upstream_default_branch_keeps_the_local_branch_after_a_handoff() {
    let fixture = Fixture::with_origin();
    let (wt, tip) = feature_tracking_origin_main(&fixture);
    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Not safe"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);

    fixture
        .wt(&landing)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Kept branch feat/x"))
        .stderr(predicate::str::contains("Deleted origin/main"));
    assert_kept_after_deleting_origin_main(&fixture, &wt, &tip);
}

#[test]
fn force_remote_of_the_upstream_default_branch_accepts_an_independent_tag() {
    let fixture = Fixture::with_origin();
    let (wt, tip) = feature_tracking_origin_main(&fixture);
    // Lightweight, whatever the host's `tag.gpgSign`.
    git(&fixture.repo(), &["update-ref", "refs/tags/v1", &tip]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Pretty safe"))
        .stderr(predicate::str::contains("Deleted origin/main"));
    assert!(!wt.exists());
    assert!(!fixture.origin_has("main"));
    assert!(!fixture.branch_exists("feat/x"));
}

#[test]
fn a_detached_worktree_depends_only_on_its_files() {
    let fixture = Fixture::new();
    let path = fixture.root.path().join("wts").join("bisect");
    git(&fixture.repo(), &["worktree", "add", "-q", "--detach", path.to_str().unwrap(), "main"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "bisect"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("no branch to delete"));
    assert!(!path.exists());
}

#[test]
fn the_report_names_ahead_behind_against_the_selected_target() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "one.txt");
    fixture.commit(&wt, "two.txt");
    // origin/main moves ahead of local main: it becomes the target.
    git(&fixture.repo(), &["push", "-q", "origin", "feat/x:refs/heads/elsewhere"]);
    let other = fixture.root.path().join("other");
    git(fixture.root.path(), &["clone", "-q", fixture.origin().to_str().unwrap(), "other"]);
    configure(&other);
    fixture.commit(&other, "upstream.txt");
    git(&other, &["push", "-q", "origin", "main"]);
    git(&fixture.repo(), &["fetch", "-q", "origin"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("2 ahead, 1 behind origin/main"))
        .stderr(predicate::str::contains("(as of your last fetch)"))
        .stderr(predicate::str::contains("origin/elsewhere"));
}

/// Item 4: merged into HEAD but not into its upstream. `git branch -d`
/// refused this with "not fully merged" and the old `-b` reported it as
/// preserved; the tiers say Safe, so it is deleted.
#[test]
fn a_branch_merged_into_head_but_not_its_upstream_is_deleted() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("fix/ci-build", "ci-build");
    fixture.commit(&wt, "one.txt");
    git(&wt, &["push", "-q", "-u", "origin", "fix/ci-build"]);
    fixture.commit(&wt, "two.txt");
    git(&fixture.repo(), &["merge", "-q", "--no-ff", "-m", "merge", "fix/ci-build"]);

    fixture
        .wt(&fixture.repo())
        .args(["remove", "ci-build"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Deleted branch"))
        .stderr(predicate::str::contains("preserved").not());
    assert!(!fixture.branch_exists("fix/ci-build"));
}

// --- Move-first removal ------------------------------------------------------

fn protocol(stdout: &[u8]) -> (PathBuf, String) {
    let text = String::from_utf8_lossy(stdout);
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "expected cd: then remove-handoff: in {text:?}");
    let cd = lines[0].strip_prefix("cd:").expect("cd: first");
    let token = lines[1].strip_prefix("remove-handoff:").expect("handoff second");
    (PathBuf::from(cd), token.to_string())
}

#[test]
fn standing_inside_without_the_wrapper_exits_4_and_removes_nothing() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");

    fixture
        .wt(&wt)
        .args(["remove", "feat-x"])
        .assert()
        .code(4)
        .stdout("")
        .stderr(predicate::str::contains("Nothing was removed"))
        .stderr(predicate::str::contains("wt --completions zsh"))
        .stderr(predicate::str::contains("from another directory"));
    assert!(wt.exists());
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn standing_inside_with_the_wrapper_hands_off_then_finishes_from_the_landing() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.repo().join("docs")).unwrap();
    fixture.commit(&fixture.repo().join("docs"), ".keep");
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let inside = wt.join("docs");

    let first = fixture
        .wt(&inside)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Moving you to the base repo"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);
    // The subdirectory is kept because it exists in the base repo too.
    assert_eq!(canonical(&landing), canonical(&fixture.repo().join("docs")));
    assert!(wt.exists(), "the first run removes nothing");

    fixture
        .wt(&landing)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stdout("")
        .stderr(predicate::str::contains("Removed worktree"))
        .stderr(predicate::str::contains("Deleted branch"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/x"));

    // The token was consumed.
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("no pending removal"));
}

#[test]
fn the_handoff_lands_in_the_fork_parent_worktree() {
    let fixture = Fixture::new();
    let parent = fixture.add_worktree("feat/theme", "feat-theme");
    let child = fixture.add_worktree("feat/dark-fixes", "feat-dark-fixes");
    let store = worktree::fork_origin::fork_origin_path(&fixture.repo()).unwrap();
    worktree::fork_origin::record(
        &store,
        "feat/dark-fixes",
        worktree::fork_origin::ForkOrigin {
            base_branch: "feat/theme".into(),
            base_sha: git(&fixture.repo(), &["rev-parse", "feat/theme"]),
            created_at: 0,
        },
    )
    .unwrap();

    let first = fixture
        .wt(&child)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-dark-fixes"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("feat/theme worktree"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);
    assert_eq!(canonical(&landing), canonical(&parent));

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0);
    assert!(!child.exists());
    assert!(parent.exists());
}

fn first_run(fixture: &Fixture, wt: &Path, extra: &[&str]) -> (PathBuf, String) {
    let out = fixture
        .wt(wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .args(extra)
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    protocol(&out)
}

#[test]
fn a_changed_branch_tip_between_the_runs_refuses_with_nothing_removed() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &[]);

    fixture.commit(&wt, "late.txt");
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("checked-out commit"))
        .stderr(predicate::str::contains("start over"));
    assert!(wt.join("late.txt").exists());
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn a_new_ignored_entry_between_the_runs_refuses() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), ".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "ignore"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &[]);

    fs::write(wt.join(".env"), "SECRET=1\n").unwrap();
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("uncommitted or ignored files"));
    assert!(wt.join(".env").exists());
}

/// Restaging keeps the status (`MM`) and the working bytes, even with
/// `--force-worktree` approved; the staged version alone must refuse.
#[test]
fn a_restaged_version_between_the_runs_refuses_with_nothing_removed() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let stage_then_edit = |staged: &str| {
        fs::write(wt.join("README.md"), staged).unwrap();
        git(&wt, &["add", "README.md"]);
        fs::write(wt.join("README.md"), "working copy\n").unwrap();
        assert_eq!(git(&wt, &["status", "--porcelain=v1"]), "MM README.md");
    };
    stage_then_edit("staged before\n");
    let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

    stage_then_edit("new staged work\n");
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("uncommitted or ignored files"))
        .stderr(predicate::str::contains("Removed").not());
    assert_eq!(fs::read_to_string(wt.join("README.md")).unwrap(), "working copy\n");
    assert_eq!(git(&wt, &["show", ":README.md"]), "new staged work");
    assert!(git(&fixture.repo(), &["worktree", "list", "--porcelain"]).contains("branch refs/heads/feat/x"));
    assert!(fixture.branch_exists("feat/x"));
}

/// Git lists an untracked nested repository as one `?? nested/` entry, so
/// only the handoff fingerprint's walk of that directory sees its contents.
fn worktree_with_nested_repo(fixture: &Fixture) -> PathBuf {
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let nested = wt.join("nested");
    fs::create_dir(&nested).unwrap();
    git(&nested, &["init", "-q", "-b", "main"]);
    fs::write(nested.join("notes"), "approved content\n").unwrap();
    assert_eq!(git(&wt, &["status", "--porcelain=v1", "-uall"]), "?? nested/");
    wt
}

#[test]
fn changes_inside_an_untracked_nested_repo_between_the_runs_refuse_with_nothing_removed() {
    let fixture = Fixture::new();
    let wt = worktree_with_nested_repo(&fixture);
    let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

    fs::write(wt.join("nested/notes"), "new work after approval\n").unwrap();
    fs::write(wt.join("nested/new-file"), "new work\n").unwrap();
    assert_eq!(git(&wt, &["status", "--porcelain=v1", "-uall"]), "?? nested/");
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("uncommitted or ignored files"))
        .stderr(predicate::str::contains("Removed").not());
    assert_eq!(fs::read_to_string(wt.join("nested/notes")).unwrap(), "new work after approval\n");
    assert_eq!(fs::read_to_string(wt.join("nested/new-file")).unwrap(), "new work\n");
    assert!(git(&fixture.repo(), &["worktree", "list", "--porcelain"]).contains("branch refs/heads/feat/x"));
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn an_unchanged_untracked_nested_repo_is_removed_by_the_handoff() {
    let fixture = Fixture::new();
    let wt = worktree_with_nested_repo(&fixture);
    let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed worktree"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/x"));
}

#[cfg(unix)]
#[test]
fn an_unreadable_file_inside_a_nested_repo_refuses_the_handoff_with_exit_4() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let wt = worktree_with_nested_repo(&fixture);
    let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

    let notes = wt.join("nested/notes");
    fs::set_permissions(&notes, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(&notes).is_ok() {
        // Root reads it regardless of the mode bits.
        return;
    }
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    fs::set_permissions(&notes, fs::Permissions::from_mode(0o644)).unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(4), "{stderr}");
    assert!(stderr.contains("Could not read everything"), "{stderr}");
    assert!(stderr.contains("Nothing was removed"), "{stderr}");
    assert_eq!(fs::read_to_string(&notes).unwrap(), "approved content\n");
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn an_expired_token_refuses_with_exit_4() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &[]);

    let record = worktree::remove::handoff::handoff_path(&fixture.repo(), &token).unwrap();
    let text = fs::read_to_string(&record).unwrap();
    let mut json: serde_json::Value = serde_json::from_str(&text).unwrap();
    json["created_at"] = serde_json::json!(1);
    fs::write(&record, serde_json::to_vec(&json).unwrap()).unwrap();

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("expired"));
    assert!(wt.exists());
    assert!(!record.exists(), "an expired record is still consumed");
}

#[test]
fn a_caller_still_inside_the_target_is_refused_with_exit_4() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (_landing, token) = first_run(&fixture, &wt, &[]);

    fixture
        .wt(&wt)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(4)
        .stderr(predicate::str::contains("still inside the worktree"));
    assert!(wt.exists());
}

#[test]
fn a_missing_or_malformed_token_refuses_with_exit_4() {
    let fixture = Fixture::new();
    for token in ["0".repeat(32), "../../escape".to_string()] {
        fixture
            .wt(&fixture.repo())
            .args(["remove", "--handoff", &token])
            .assert()
            .code(4)
            .stderr(predicate::str::contains("no pending removal"));
    }
}

#[test]
fn the_handoff_carries_force_flags_to_the_second_run() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "unique.txt");
    fs::write(wt.join("scratch.txt"), "x\n").unwrap();
    let (landing, token) = first_run(&fixture, &wt, &["--force-worktree", "--force-branch"]);
    assert!(wt.join("scratch.txt").exists());

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("(1 commit lost)"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/x"));
}

#[test]
fn a_branch_that_stops_being_safe_between_the_runs_refuses() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "unique.txt");
    git(&fixture.repo(), &["branch", "backup", &tip]);
    let (landing, token) = first_run(&fixture, &wt, &[]);

    // The only other copy disappears.
    git(&fixture.repo(), &["branch", "-D", "backup"]);
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("no longer safe"));
    assert!(wt.exists());
    assert!(fixture.branch_exists("feat/x"));
}

/// Two origin branches at one commit: the lease cannot tell them apart, so
/// the second run must compare the destination itself.
#[test]
fn a_changed_remote_destination_between_the_runs_refuses_with_nothing_removed() {
    let fixture = Fixture::with_origin();
    git(&fixture.repo(), &["push", "-q", "origin", "main:approved", "main:unapproved"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    git(&fixture.repo(), &["branch", "--set-upstream-to=origin/approved", "feat/x"]);
    let (landing, token) = first_run(&fixture, &wt, &["--force-remote"]);

    git(&fixture.repo(), &["branch", "--set-upstream-to=origin/unapproved", "feat/x"]);
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("branch on origin to delete changed"))
        .stderr(predicate::str::contains("Deleted").not());
    assert!(fixture.origin_has("approved"));
    assert!(fixture.origin_has("unapproved"));
    assert!(wt.exists());
    assert!(fixture.branch_exists("feat/x"));
}

// --- The repository `--force-remote` deletes from -----------------------------

/// `approved.git` and `other.git`, each holding `main` and `feat/x` at the same
/// commit, with `origin` pointing at `approved.git`; `feat/x` is also a
/// worktree.
struct TwoRemotes {
    fixture: Fixture,
    wt: PathBuf,
    approved: PathBuf,
    other: PathBuf,
}

impl TwoRemotes {
    fn new() -> Self {
        let fixture = Fixture::new();
        let approved = fixture.root.path().join("approved.git");
        let other = fixture.root.path().join("other.git");
        let origin = Self::url(&approved).to_string();
        Self::build(fixture, approved, other, &origin)
    }

    /// The bare repositories are `approved` and `other` inside the base
    /// checkout, and origin's URL is the relative path `approved`.
    fn relative() -> Self {
        let fixture = Fixture::new();
        let approved = fixture.repo().join("approved");
        let other = fixture.repo().join("other");
        let this = Self::build(fixture, approved, other, "approved");
        assert_eq!(git(&this.fixture.repo(), &["remote", "get-url", "--push", "origin"]), "approved");
        this
    }

    fn build(fixture: Fixture, approved: PathBuf, other: PathBuf, origin: &str) -> Self {
        let wt = fixture.add_worktree("feat/x", "feat-x");
        for bare in [&approved, &other] {
            let bare = bare.to_str().unwrap();
            git(fixture.root.path(), &["init", "-q", "--bare", "-b", "main", bare]);
            git(&fixture.repo(), &["push", "-q", bare, "main", "feat/x"]);
        }
        git(&fixture.repo(), &["remote", "add", "origin", origin]);
        Self {
            fixture,
            wt,
            approved,
            other,
        }
    }

    fn url(path: &Path) -> &str {
        path.to_str().unwrap()
    }

    fn head_in(bare: &Path, branch: &str) -> Option<String> {
        let out = git(bare, &["for-each-ref", "--format=%(objectname)", &format!("refs/heads/{branch}")]);
        (!out.is_empty()).then_some(out)
    }

    /// Nothing was removed anywhere.
    fn assert_untouched(&self) {
        assert!(Self::head_in(&self.approved, "feat/x").is_some(), "approved.git keeps feat/x");
        assert!(Self::head_in(&self.other, "feat/x").is_some(), "other.git keeps feat/x");
        assert!(self.wt.exists(), "the worktree stays");
        assert!(self.fixture.branch_exists("feat/x"), "the local branch stays");
    }

    fn handoff_after(&self, change: &[&str]) {
        let (landing, token) = first_run(&self.fixture, &self.wt, &["--force-remote"]);
        git(&self.fixture.repo(), change);
        self.fixture
            .wt(&landing)
            .args(["remove", "--handoff", &token])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("repository origin pushes to changed"))
            .stderr(predicate::str::contains("Deleted").not());
        self.assert_untouched();
    }
}

#[test]
fn a_changed_origin_url_between_the_runs_refuses_with_nothing_removed() {
    let remotes = TwoRemotes::new();
    remotes.handoff_after(&["remote", "set-url", "origin", TwoRemotes::url(&remotes.other)]);
}

#[test]
fn a_changed_origin_push_url_between_the_runs_refuses_with_nothing_removed() {
    let remotes = TwoRemotes::new();
    remotes.handoff_after(&["remote", "set-url", "--push", "origin", TwoRemotes::url(&remotes.other)]);
}

/// Fetch and push URLs differ, and `approved.git`'s `feat/x` sits at another
/// commit: observing the fetch URL would take the lease against the wrong
/// head (or miss the branch), so a deletion that succeeds proves the report,
/// the lease, and the push all used the push URL.
#[test]
fn a_separate_push_url_is_both_observed_and_deleted_from() {
    let remotes = TwoRemotes::new();
    let repo = remotes.fixture.repo();
    git(&repo, &["checkout", "-q", "-b", "side"]);
    let side = remotes.fixture.commit(&repo, "side.txt");
    git(&repo, &["checkout", "-q", "main"]);
    git(&repo, &["push", "-q", "--force", TwoRemotes::url(&remotes.approved), "side:feat/x"]);
    git(&repo, &["remote", "set-url", "--push", "origin", TwoRemotes::url(&remotes.other)]);

    let out = remotes
        .fixture
        .wt(&remotes.wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("other.git"))
        .stderr(predicate::str::contains("origin/feat/x will be deleted; it has no commits"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&out);

    remotes
        .fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Deleted origin/feat/x"));
    assert_eq!(TwoRemotes::head_in(&remotes.other, "feat/x"), None);
    assert_eq!(TwoRemotes::head_in(&remotes.approved, "feat/x"), Some(side));
    assert!(!remotes.wt.exists());
}

/// Deleting from several repositories would act on copies the report never
/// showed, so the run refuses before removing anything (exit 3, since
/// leaving out `--force-remote` is the remedy).
#[test]
fn several_push_urls_refuse_force_remote_with_nothing_removed() {
    let remotes = TwoRemotes::new();
    let repo = remotes.fixture.repo();
    for bare in [&remotes.approved, &remotes.other] {
        git(&repo, &["remote", "set-url", "--add", "--push", "origin", TwoRemotes::url(bare)]);
    }

    remotes
        .fixture
        .wt(&repo)
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("pushes to 2 repositories"))
        .stderr(predicate::str::contains("Nothing was removed"));
    remotes.assert_untouched();
}

/// [`TwoRemotes`] plus `fetch.git` (same branches) as origin's only URL, and
/// `url.<approved>.pushInsteadOf=<fetch>`, so origin's resolved push URL is
/// `approved.git`.
fn rewritten_to_approved() -> (TwoRemotes, String) {
    let remotes = TwoRemotes::new();
    let repo = remotes.fixture.repo();
    let fetch = remotes.fixture.root.path().join("fetch.git");
    git(remotes.fixture.root.path(), &["init", "-q", "--bare", "-b", "main", TwoRemotes::url(&fetch)]);
    git(&repo, &["push", "-q", TwoRemotes::url(&fetch), "main", "feat/x"]);
    git(&repo, &["remote", "set-url", "origin", TwoRemotes::url(&fetch)]);
    let approved_rule = format!("url.{}.pushInsteadOf", TwoRemotes::url(&remotes.approved));
    git(&repo, &["config", &approved_rule, TwoRemotes::url(&fetch)]);
    assert_eq!(
        git(&repo, &["remote", "get-url", "--push", "origin"]),
        TwoRemotes::url(&remotes.approved)
    );
    let redirect = format!("url.{}.pushInsteadOf", TwoRemotes::url(&remotes.other));
    (remotes, redirect)
}

fn refs_of(bare: &Path) -> String {
    git(bare, &["for-each-ref", "--format=%(objectname) %(refname)"])
}

/// A second rule maps the resolved push URL (`approved.git`) on to
/// `other.git`; `git push <approved>` would delete from `other.git`.
#[test]
fn a_rewrite_rule_matching_the_push_url_refuses_force_remote_with_nothing_removed() {
    let (remotes, redirect) = rewritten_to_approved();
    let repo = remotes.fixture.repo();
    git(&repo, &["config", &redirect, TwoRemotes::url(&remotes.approved)]);
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&repo)
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("Nothing was removed"))
        .stderr(predicate::str::contains("would rewrite origin's push URL"))
        .stderr(predicate::str::contains("pushinsteadof="))
        .stderr(predicate::str::contains("Deleted").not());
    remotes.assert_untouched();
    assert_eq!(refs_of(&remotes.other), other_before, "other.git is never modified");
}

/// The rule appears between the runs while `git remote get-url --push`
/// still prints the approved URL, so only the rewrite check can see it.
#[test]
fn a_rewrite_rule_added_between_the_runs_refuses_with_nothing_removed() {
    let (remotes, redirect) = rewritten_to_approved();
    let repo = remotes.fixture.repo();
    let (landing, token) = first_run(&remotes.fixture, &remotes.wt, &["--force-remote"]);
    git(&repo, &["config", &redirect, TwoRemotes::url(&remotes.approved)]);
    assert_eq!(
        git(&repo, &["remote", "get-url", "--push", "origin"]),
        TwoRemotes::url(&remotes.approved)
    );
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Nothing was removed"))
        .stderr(predicate::str::contains("would rewrite origin's push URL"))
        .stderr(predicate::str::contains("Deleted").not());
    remotes.assert_untouched();
    assert_eq!(refs_of(&remotes.other), other_before, "other.git is never modified");
}

/// Remote `approved` fetches from `approved` but pushes to `other`, so
/// `ls-remote approved` and `push approved` would each read it as that remote
/// rather than as origin's relative path.
fn name_the_endpoint_as_a_remote(remotes: &TwoRemotes) {
    let repo = remotes.fixture.repo();
    git(&repo, &["remote", "add", "approved", TwoRemotes::url(&remotes.approved)]);
    git(&repo, &["remote", "set-url", "--push", "approved", TwoRemotes::url(&remotes.other)]);
    assert_eq!(git(&repo, &["remote", "get-url", "--push", "origin"]), "approved");
}

#[test]
fn an_endpoint_that_names_another_remote_refuses_force_remote_with_nothing_removed() {
    let remotes = TwoRemotes::relative();
    name_the_endpoint_as_a_remote(&remotes);
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&remotes.fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("Nothing was removed"))
        .stderr(predicate::str::contains("as the name of the remote defined by remote.approved."))
        .stderr(predicate::str::contains("Deleted").not());
    remotes.assert_untouched();
    assert_eq!(refs_of(&remotes.other), other_before, "other is never modified");
}

/// `git remote get-url approved` reports no such remote for a remote given
/// only by `-c`, yet `git push approved` uses it.
#[test]
fn an_endpoint_naming_a_remote_from_command_line_config_refuses_with_nothing_removed() {
    let remotes = TwoRemotes::relative();
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&remotes.fixture.repo())
        .env("GIT_CONFIG_COUNT", "1")
        .env("GIT_CONFIG_KEY_0", "remote.approved.pushurl")
        .env("GIT_CONFIG_VALUE_0", TwoRemotes::url(&remotes.other))
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(3)
        .stdout("")
        .stderr(predicate::str::contains("remote defined by remote.approved.pushurl"))
        .stderr(predicate::str::contains("Deleted").not());
    remotes.assert_untouched();
    assert_eq!(refs_of(&remotes.other), other_before, "other is never modified");
}

/// The remote appears between the runs while origin's push URL still prints
/// `approved`, so the endpoint and head comparisons both pass.
#[test]
fn a_remote_named_like_the_endpoint_added_between_the_runs_refuses_with_nothing_removed() {
    let remotes = TwoRemotes::relative();
    let (landing, token) = first_run(&remotes.fixture, &remotes.wt, &["--force-remote"]);
    name_the_endpoint_as_a_remote(&remotes);
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("Nothing was removed"))
        .stderr(predicate::str::contains("as the name of the remote defined by remote.approved."))
        .stderr(predicate::str::contains("Deleted").not());
    remotes.assert_untouched();
    assert_eq!(refs_of(&remotes.other), other_before, "other is never modified");
}

#[test]
fn an_unambiguous_relative_endpoint_is_observed_and_deleted_from() {
    let remotes = TwoRemotes::relative();
    let other_before = refs_of(&remotes.other);

    remotes
        .fixture
        .wt(&remotes.fixture.repo())
        .args(["remove", "feat-x", "--force-remote"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Origin (approved)"))
        .stderr(predicate::str::contains("Deleted origin/feat/x"));
    assert_eq!(TwoRemotes::head_in(&remotes.approved, "feat/x"), None);
    assert_eq!(refs_of(&remotes.other), other_before, "other is never modified");
    assert!(!remotes.wt.exists());
}
