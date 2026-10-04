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

#[test]
fn unchanged_included_copy_and_other_ignored_files_remove_without_force() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\ntarget/\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"SECRET=old\n").unwrap();
    fs::create_dir_all(fixture.root.path().join("wts")).unwrap();
    let created = fixture.wt(&fixture.repo()).args(["create", "included"])
        .env("WT", fixture.root.path().join("wts")).output().unwrap();
    assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));
    let target = fixture.root.path().join("wts/repo/included");
    let record = worktree::copy_record::record_path(&fixture.repo(), &target).unwrap();
    assert!(record.exists());
    fs::create_dir_all(target.join("target")).unwrap();
    fs::write(target.join("target/build"), b"generated").unwrap();
    fs::write(fixture.repo().join(".env"), b"SECRET=new\n").unwrap();
    let removed = fixture.wt(&fixture.repo()).args(["remove", "included"])
        .output().unwrap();
    assert!(removed.status.success(), "{}", String::from_utf8_lossy(&removed.stderr));
    assert!(!target.exists());
    assert!(!record.exists());
}

fn setup_included(fixture: &Fixture) -> PathBuf {
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"SECRET=old\n").unwrap();
    fs::create_dir_all(fixture.root.path().join("wts")).unwrap();
    let output = fixture.wt(&fixture.repo()).args(["create", "included"])
        .env("WT", fixture.root.path().join("wts")).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    fixture.root.path().join("wts/repo/included")
}

#[test]
fn changed_included_copy_requires_force_and_preserves_state_on_refusal() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    fs::write(target.join(".env"), b"SECRET=new\n").unwrap();
    let output = fixture.wt(&fixture.repo()).args(["remove", "included"]).output().unwrap();
    assert_eq!(output.status.code(), Some(3));
    let report = String::from_utf8_lossy(&output.stderr);
    assert!(report.contains(".env") && report.contains("changed"), "{report}");
    assert!(target.join(".env").exists());
    assert!(fixture.branch_exists("included"));
    fixture.wt(&fixture.repo()).args(["remove", "included", "--force-worktree"])
        .assert().code(0);
    assert!(!target.exists());
}

#[test]
fn same_size_included_edit_with_restored_mtime_requires_consent_at_both_sizes() {
    for size in [11, 2_000_000] {
        let fixture = Fixture::new();
        fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
        fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
        git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
        git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
        fs::write(fixture.repo().join(".env"), vec![b'a'; size]).unwrap();
        fs::create_dir_all(fixture.root.path().join("wts")).unwrap();
        let created = fixture.wt(&fixture.repo()).args(["create", "included"])
            .env("WT", fixture.root.path().join("wts")).output().unwrap();
        assert!(created.status.success(), "{}", String::from_utf8_lossy(&created.stderr));
        let target = fixture.root.path().join("wts/repo/included");
        let path = target.join(".env");
        let modified = fs::metadata(&path).unwrap().modified().unwrap();
        fs::write(&path, vec![b'b'; size]).unwrap();
        fs::File::options().write(true).open(&path).unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified)).unwrap();
        let output = fixture.wt(&fixture.repo()).args(["remove", "included"]).output().unwrap();
        assert_eq!(output.status.code(), Some(3), "size={size}: {}", String::from_utf8_lossy(&output.stderr));
        assert!(String::from_utf8_lossy(&output.stderr).contains("changed"));
        assert_eq!(fs::read(path).unwrap(), vec![b'b'; size]);
    }
}

#[test]
fn no_record_uses_distinct_source_and_new_file_needs_consent() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"SECRET=old\n").unwrap();
    let target = fixture.add_worktree("feat/x", "feat-x");
    fs::write(target.join(".env"), b"SECRET=old\n").unwrap();
    fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).assert().code(0);
    assert!(!target.exists());

    let target = fixture.add_worktree("feat/y", "feat-y");
    fs::write(target.join(".env"), b"SECRET=new\n").unwrap();
    fs::remove_file(fixture.repo().join(".env")).unwrap();
    fixture.wt(&fixture.repo()).args(["remove", "feat-y"]).assert().code(3)
        .stderr(predicate::str::contains(".env"))
        .stderr(predicate::str::contains("new"));
    assert!(target.join(".env").exists());
}

#[test]
fn no_record_source_change_refuses_handoff() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"SECRET=old\n").unwrap();
    let target = fixture.add_worktree("feat/x", "feat-x");
    fs::write(target.join(".env"), b"SECRET=old\n").unwrap();
    let (landing, token) = first_run(&fixture, &target, &[]);
    fs::write(fixture.repo().join(".env"), b"SECRET=new\n").unwrap();
    fixture.wt(&landing).args(["remove", "--handoff", &token]).assert().code(3)
        .stderr(predicate::str::contains("copy baseline"));
    assert!(target.join(".env").exists());
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn no_record_uses_the_checked_out_fork_parent_as_source() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    fs::write(fixture.repo().join(".env"), b"BASE=1\n").unwrap();
    let parent = fixture.add_worktree("feat/parent", "parent");
    fs::write(parent.join(".env"), b"PARENT=1\n").unwrap();
    let target = fixture.add_worktree("feat/child", "child");
    fs::write(target.join(".env"), b"PARENT=1\n").unwrap();
    let path = worktree::fork_origin::fork_origin_path(&fixture.repo()).unwrap();
    worktree::fork_origin::record(&path, "feat/child", worktree::fork_origin::ForkOrigin {
        base_branch: "feat/parent".into(), base_sha: git(&fixture.repo(), &["rev-parse", "HEAD"]), created_at: 0,
    }).unwrap();
    fixture.wt(&fixture.repo()).args(["remove", "child"]).assert().code(0);
    assert!(!target.exists());
    assert!(parent.join(".env").exists());
}

#[test]
fn changed_include_rules_refuse_handoff_even_when_file_leaves_set() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    let first = fixture.wt(&target).env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "included"]).assert().code(0).get_output().stdout.clone();
    let (landing, token) = protocol(&first);
    fs::write(target.join(".worktreeinclude"), b"other.env\n").unwrap();
    fixture.wt(&landing).args(["remove", "--handoff", &token]).assert().code(3)
        .stderr(predicate::str::contains("include rules"));
    assert!(target.join(".env").exists());
    assert!(fixture.branch_exists("included"));
}

#[test]
fn changed_copy_record_refuses_handoff_and_keeps_registration() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    let first = fixture.wt(&target).env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "included"]).assert().code(0).get_output().stdout.clone();
    let (landing, token) = protocol(&first);
    let path = worktree::copy_record::record_path(&fixture.repo(), &target).unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    value["source_label"] = serde_json::Value::String("changed source label".into());
    fs::write(path, serde_json::to_vec(&value).unwrap()).unwrap();
    fixture.wt(&landing).args(["remove", "--handoff", &token]).assert().code(3)
        .stderr(predicate::str::contains("copy baseline"));
    assert!(target.join(".env").exists());
    assert!(fixture.branch_exists("included"));
}

#[test]
fn changed_included_contents_refuse_handoff_even_when_classification_stays_changed() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    fs::write(target.join(".env"), b"SECRET=one\n").unwrap();
    let first = fixture.wt(&target).env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "included", "--force-worktree"])
        .assert().code(0).get_output().stdout.clone();
    let (landing, token) = protocol(&first);
    fs::write(target.join(".env"), b"SECRET=two\n").unwrap();
    fixture.wt(&landing).args(["remove", "--handoff", &token]).assert().code(3)
        .stderr(predicate::str::contains("included files"));
    assert_eq!(fs::read(target.join(".env")).unwrap(), b"SECRET=two\n");
    assert!(fixture.branch_exists("included"));
}

#[test]
fn corrupt_copy_record_falls_back_to_source_without_consent() {
    let fixture = Fixture::new();
    let target = setup_included(&fixture);
    let path = worktree::copy_record::record_path(&fixture.repo(), &target).unwrap();
    fs::write(&path, b"{not json").unwrap();
    let output = fixture.wt(&fixture.repo()).args(["remove", "included"]).output().unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(String::from_utf8_lossy(&output.stderr).contains("copy record is untrusted"));
    assert!(!target.exists());
    assert!(!path.exists());
}

#[test]
fn unavailable_distinct_source_marks_included_file_unknown() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b".env\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b".env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    let target = fixture.add_worktree("feat/x", "feat-x");
    fs::write(target.join(".env"), b"SECRET=old\n").unwrap();
    let path = worktree::fork_origin::fork_origin_path(&fixture.repo()).unwrap();
    worktree::fork_origin::record(&path, "feat/x", worktree::fork_origin::ForkOrigin {
        base_branch: "feat/x".into(), base_sha: git(&fixture.repo(), &["rev-parse", "HEAD"]), created_at: 0,
    }).unwrap();
    fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).assert().code(3)
        .stderr(predicate::str::contains("unknown"));
    assert!(target.join(".env").exists());
    fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force-worktree"])
        .assert().code(0);
    assert!(!target.exists());
}

#[test]
fn invalid_include_rules_fail_before_removal() {
    let fixture = Fixture::new();
    let target = fixture.add_worktree("feat/x", "feat-x");
    fs::create_dir(target.join(".worktreeinclude")).unwrap();
    fs::write(target.join(".worktreeinclude/entry"), b"secret").unwrap();
    fixture.wt(&fixture.repo()).args(["remove", "feat-x", "--force-worktree"])
        .assert().code(1);
    assert!(target.join(".worktreeinclude/entry").exists());
    assert!(fixture.branch_exists("feat/x"));
    assert!(git(&fixture.repo(), &["worktree", "list", "--porcelain"]).contains("branch refs/heads/feat/x"));
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
fn ordinary_ignored_entries_are_disposable() {
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
            .code(0)
            .stderr(predicate::str::contains("Also deletes ignored files"))
            .stderr(predicate::str::contains("deletes"))
            .stderr(predicate::str::contains(ignore));
        assert!(!wt.exists(), "{ignore}");
    }
}

#[test]
fn mixed_ignored_directory_report_names_disposable_files() {
    let fixture = Fixture::new();
    fs::write(fixture.repo().join(".gitignore"), b"config/\n").unwrap();
    fs::write(fixture.repo().join(".worktreeinclude"), b"config/.env\n").unwrap();
    git(&fixture.repo(), &["add", ".gitignore", ".worktreeinclude"]);
    git(&fixture.repo(), &["commit", "-q", "-m", "include rules"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::create_dir_all(wt.join("config")).unwrap();
    fs::write(wt.join("config/.env"), b"secret").unwrap();
    fs::write(wt.join("config/cache.bin"), b"cache").unwrap();

    fixture.wt(&fixture.repo()).args(["remove", "feat-x"])
        .assert().code(3)
        .stderr(predicate::str::contains("config/.env (new)"))
        .stderr(predicate::str::contains("Also deletes ignored files: config/cache.bin"));
    assert!(wt.join("config/.env").exists());
    assert!(wt.join("config/cache.bin").exists());
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
fn a_new_disposable_ignored_entry_between_the_runs_does_not_refuse() {
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
        .code(0);
    assert!(!wt.exists());
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
        .stderr(predicate::str::contains("uncommitted or included files"))
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
        .stderr(predicate::str::contains("uncommitted or included files"))
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

/// 1.5 MiB of varied bytes: past the 1 MiB size a metadata shortcut was once
/// proposed for.
fn large_contents() -> Vec<u8> {
    (0..1_572_864_u32).map(|index| (index % 251) as u8).collect()
}

/// Flips bytes at the start, middle, and end of `path` and then restores
/// its modification time, so neither its size nor its time shows the edit.
/// Returns the new contents.
fn edit_keeping_size_and_mtime(path: &Path) -> Vec<u8> {
    let before = fs::metadata(path).unwrap();
    let modified = before.modified().unwrap();
    let mut bytes = fs::read(path).unwrap();
    let last = bytes.len() - 1;
    for index in [0, bytes.len() / 2, last] {
        bytes[index] ^= 0x01;
    }
    fs::write(path, &bytes).unwrap();
    // A separate handle, so no later write through it can move the time.
    fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(modified))
        .unwrap();
    let after = fs::metadata(path).unwrap();
    assert_eq!(after.len(), before.len(), "{path:?} kept its size");
    assert_eq!(after.modified().unwrap(), modified, "{path:?} kept its modification time");
    bytes
}

/// The spec's fingerprint scenario: an edit that keeps both size and
/// modification time, to a large dirty file and to files inside an untracked
/// nested repository, refuses the handoff and keeps the edit. The consumed
/// token cannot then be replayed.
#[test]
fn a_same_size_same_mtime_edit_between_the_runs_refuses_and_keeps_the_edit() {
    for case in [
        "a large untracked file",
        "a large modified tracked file",
        "a file inside an untracked nested repo",
        "a large file inside an untracked nested repo",
    ] {
        let fixture = Fixture::new();
        let (wt, edited) = match case {
            "a large untracked file" => {
                let wt = fixture.add_worktree("feat/x", "feat-x");
                fs::write(wt.join("big.bin"), large_contents()).unwrap();
                (wt.clone(), wt.join("big.bin"))
            }
            "a large modified tracked file" => {
                fs::write(fixture.repo().join("big.bin"), large_contents()).unwrap();
                git(&fixture.repo(), &["add", "big.bin"]);
                git(&fixture.repo(), &["commit", "-q", "-m", "big"]);
                let wt = fixture.add_worktree("feat/x", "feat-x");
                let mut dirty = large_contents();
                dirty[10] ^= 0x02;
                fs::write(wt.join("big.bin"), dirty).unwrap();
                assert_eq!(git(&wt, &["status", "--porcelain=v1"]), "M big.bin");
                (wt.clone(), wt.join("big.bin"))
            }
            "a file inside an untracked nested repo" => {
                let wt = worktree_with_nested_repo(&fixture);
                (wt.clone(), wt.join("nested/notes"))
            }
            _ => {
                let wt = worktree_with_nested_repo(&fixture);
                fs::write(wt.join("nested/big.bin"), large_contents()).unwrap();
                (wt.clone(), wt.join("nested/big.bin"))
            }
        };
        let status = git(&wt, &["status", "--porcelain=v1", "-uall"]);
        let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

        let new_bytes = edit_keeping_size_and_mtime(&edited);
        assert_eq!(git(&wt, &["status", "--porcelain=v1", "-uall"]), status, "{case}: git sees no change");
        fixture
            .wt(&landing)
            .args(["remove", "--handoff", &token])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("uncommitted or included files"))
            .stderr(predicate::str::contains("Removed").not());
        assert!(fs::read(&edited).unwrap() == new_bytes, "{case}: the edit is kept");
        assert_nothing_removed(&fixture, &wt);

        fixture
            .wt(&landing)
            .args(["remove", "--handoff", &token])
            .assert()
            .code(4)
            .stderr(predicate::str::contains("no pending removal"));
        assert!(fs::read(&edited).unwrap() == new_bytes, "{case}: a replay removes nothing");
        assert_nothing_removed(&fixture, &wt);
    }
}

/// Keeps `path` unreadable while alive: mode 000 on Unix, an exclusive open
/// on Windows, where mode bits cannot deny a read.
struct Unreadable {
    #[cfg(unix)]
    path: PathBuf,
    #[cfg(windows)]
    _held: fs::File,
}

impl Unreadable {
    /// `None` when the file stays readable (root ignores mode bits).
    fn new(path: &Path) -> Option<Self> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(path, fs::Permissions::from_mode(0o000)).unwrap();
            let this = Self { path: path.to_path_buf() };
            fs::File::open(path).is_err().then_some(this)
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let held = fs::OpenOptions::new().read(true).share_mode(0).open(path).unwrap();
            assert!(fs::File::open(path).is_err(), "the exclusive open denies other readers");
            Some(Self { _held: held })
        }
        #[cfg(not(any(unix, windows)))]
        {
            let _ = path;
            None
        }
    }
}

#[cfg(unix)]
impl Drop for Unreadable {
    fn drop(&mut self) {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&self.path, fs::Permissions::from_mode(0o644));
    }
}

/// A dirty file the second run cannot read, at the top level or inside an
/// untracked nested repository, refuses the handoff (exit 4, as every
/// `WorktreeError::Io` does) and keeps the file.
#[test]
fn an_unreadable_dirty_file_refuses_the_handoff_with_exit_4_and_is_kept() {
    for case in ["an untracked file", "a file inside an untracked nested repo"] {
        let fixture = Fixture::new();
        let (wt, dirty, contents) = if case == "an untracked file" {
            let wt = fixture.add_worktree("feat/x", "feat-x");
            fs::write(wt.join("scratch.txt"), "approved scratch\n").unwrap();
            (wt.clone(), wt.join("scratch.txt"), "approved scratch\n")
        } else {
            let wt = worktree_with_nested_repo(&fixture);
            (wt.clone(), wt.join("nested/notes"), "approved content\n")
        };
        let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

        let Some(unreadable) = Unreadable::new(&dirty) else {
            eprintln!("skipping {case}: the file stays readable to this user");
            continue;
        };
        let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
        drop(unreadable);

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(4), "{case}: {stderr}");
        assert!(stderr.contains("Could not read everything"), "{case}: {stderr}");
        assert!(stderr.contains("Nothing was removed"), "{case}: {stderr}");
        assert!(!stderr.contains("Removed"), "{case}: {stderr}");
        assert_eq!(fs::read_to_string(&dirty).unwrap(), contents, "{case}");
        assert_nothing_removed(&fixture, &wt);
    }
}

/// Paths that are not UTF-8. `\xff` and `\xfe` both decode lossily to
/// U+FFFD, so only a byte-faithful fingerprint tells them apart.
#[cfg(unix)]
mod non_utf8_paths {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;
    use std::os::unix::fs::symlink;

    use super::*;

    const APPROVED: &[u8] = b"target-\xff";
    const CHANGED: &[u8] = b"target-\xfe";

    fn relink(link: &Path, target: &[u8]) {
        let _ = fs::remove_file(link);
        symlink(OsStr::from_bytes(target), link).unwrap();
    }

    fn target_of(link: &Path) -> Vec<u8> {
        fs::read_link(link).unwrap().as_os_str().as_bytes().to_vec()
    }

    /// A file named by `name`, or `None` (with a note) on a filesystem that
    /// refuses names that are not UTF-8, as macOS APFS does.
    fn create(dir: &Path, name: &[u8], content: &str) -> Option<PathBuf> {
        let path = dir.join(OsStr::from_bytes(name));
        match fs::write(&path, content) {
            Ok(()) => Some(path),
            Err(error) => {
                eprintln!("skipped: this filesystem refuses a non-UTF-8 file name ({error})");
                None
            }
        }
    }

    fn assert_refused_with_nothing_removed(fixture: &Fixture, landing: &Path, token: &str) {
        fixture
            .wt(landing)
            .args(["remove", "--handoff", token])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("uncommitted or included files"))
            .stderr(predicate::str::contains("Removed").not());
        assert!(git(&fixture.repo(), &["worktree", "list", "--porcelain"]).contains("branch refs/heads/feat/x"));
        assert!(fixture.branch_exists("feat/x"));
    }

    #[test]
    fn a_changed_symlink_target_between_the_runs_refuses_with_nothing_removed() {
        let fixture = Fixture::new();
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let link = wt.join("link");
        relink(&link, APPROVED);
        let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

        relink(&link, CHANGED);
        assert_refused_with_nothing_removed(&fixture, &landing, &token);
        assert_eq!(target_of(&link), CHANGED);
    }

    #[test]
    fn a_changed_symlink_target_inside_an_untracked_nested_repo_refuses_with_nothing_removed() {
        let fixture = Fixture::new();
        let wt = worktree_with_nested_repo(&fixture);
        let link = wt.join("nested/link");
        relink(&link, APPROVED);
        let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

        relink(&link, CHANGED);
        assert_eq!(git(&wt, &["status", "--porcelain=v1", "-uall"]), "?? nested/");
        assert_refused_with_nothing_removed(&fixture, &landing, &token);
        assert_eq!(target_of(&link), CHANGED);
    }

    #[test]
    fn an_unchanged_non_utf8_symlink_target_is_removed_by_the_handoff() {
        let fixture = Fixture::new();
        let wt = fixture.add_worktree("feat/x", "feat-x");
        relink(&wt.join("link"), APPROVED);
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

    #[test]
    fn an_edited_non_utf8_file_name_between_the_runs_refuses_with_nothing_removed() {
        let fixture = Fixture::new();
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let Some(file) = create(&wt, b"notes-\xff", "approved content\n") else {
            return;
        };
        let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

        fs::write(&file, "new work after approval\n").unwrap();
        assert_refused_with_nothing_removed(&fixture, &landing, &token);
        assert_eq!(fs::read_to_string(&file).unwrap(), "new work after approval\n");
    }

    #[test]
    fn a_nested_child_renamed_to_a_name_with_the_same_lossy_text_refuses_with_nothing_removed() {
        let fixture = Fixture::new();
        let wt = worktree_with_nested_repo(&fixture);
        let nested = wt.join("nested");
        let Some(approved) = create(&nested, b"child-\xff", "same content\n") else {
            return;
        };
        let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

        let renamed = nested.join(OsStr::from_bytes(b"child-\xfe"));
        fs::rename(&approved, &renamed).unwrap();
        assert_refused_with_nothing_removed(&fixture, &landing, &token);
        assert_eq!(fs::read_to_string(&renamed).unwrap(), "same content\n");
    }
}

/// A chmod keeps a modified tracked file's status (` M`), its bytes, and its
/// index entry, so only the working file's mode tells the states apart.
#[cfg(unix)]
mod executable_bit {
    use std::os::unix::fs::PermissionsExt;

    use super::*;

    fn set_mode(path: &Path, mode: u32) {
        fs::set_permissions(path, fs::Permissions::from_mode(mode)).unwrap();
    }

    fn mode_of(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    /// `run.sh` committed at `100644` with `core.filemode=true`, then edited
    /// without staging in `feat-x`.
    fn worktree_with_modified_script(fixture: &Fixture) -> (PathBuf, PathBuf) {
        let repo = fixture.repo();
        git(&repo, &["config", "core.filemode", "true"]);
        fs::write(repo.join("run.sh"), "echo one\n").unwrap();
        set_mode(&repo.join("run.sh"), 0o644);
        git(&repo, &["add", "run.sh"]);
        git(&repo, &["commit", "-q", "-m", "script"]);
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let script = wt.join("run.sh");
        fs::write(&script, "echo edited\n").unwrap();
        set_mode(&script, 0o644);
        assert_eq!(git(&wt, &["status", "--porcelain=v1"]), "M run.sh");
        (wt, script)
    }

    #[test]
    fn a_changed_executable_bit_between_the_runs_refuses_with_nothing_removed() {
        let fixture = Fixture::new();
        let (wt, script) = worktree_with_modified_script(&fixture);
        let (landing, token) = first_run(&fixture, &wt, &["--force-worktree"]);

        set_mode(&script, 0o755);
        assert_eq!(git(&wt, &["status", "--porcelain=v1"]), "M run.sh");
        assert!(git(&wt, &["diff", "--summary"]).contains("mode change 100644 => 100755"));
        fixture
            .wt(&landing)
            .args(["remove", "--handoff", &token])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("uncommitted or included files"))
            .stderr(predicate::str::contains("Removed").not());
        assert_eq!(fs::read_to_string(&script).unwrap(), "echo edited\n");
        assert_eq!(mode_of(&script), 0o755);
        assert!(git(&fixture.repo(), &["worktree", "list", "--porcelain"]).contains("branch refs/heads/feat/x"));
        assert!(fixture.branch_exists("feat/x"));
    }

    #[test]
    fn an_unchanged_executable_bit_is_removed_by_the_handoff() {
        let fixture = Fixture::new();
        let (wt, _script) = worktree_with_modified_script(&fixture);
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
    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x", "--force-worktree", "--force-branch"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("1 commit exists nowhere else"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);
    assert!(wt.join("scratch.txt").exists());

    // The first run named the lost commit and the caller approved it; the
    // second run does not reassess safety, so it claims no count.
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Deleted branch feat/x"))
        .stderr(predicate::str::contains("lost").not());
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

// --- What the second run proves again -----------------------------------------

/// The worktree directory, its registration, and the branch are all intact.
fn assert_nothing_removed(fixture: &Fixture, wt: &Path) {
    assert!(wt.exists(), "the worktree directory stays");
    assert!(
        git(&fixture.repo(), &["worktree", "list", "--porcelain"]).contains("branch refs/heads/feat/x"),
        "the worktree stays registered"
    );
    assert!(fixture.branch_exists("feat/x"), "the branch stays");
}

/// A clone of `origin` standing in for someone else's pushes.
fn pusher(fixture: &Fixture) -> PathBuf {
    let pusher = fixture.root.path().join("pusher");
    git(fixture.root.path(), &["clone", "-q", fixture.origin().to_str().unwrap(), "pusher"]);
    configure(&pusher);
    pusher
}

/// The first run trusts `origin/main` as of the last fetch; the second run
/// must ask origin, because the tracking ref cannot see a force-push made
/// after the approval.
#[test]
fn a_branch_proved_only_by_origin_default_refuses_once_origin_drops_the_tip() {
    let fixture = Fixture::with_origin();
    let old_main = git(&fixture.repo(), &["rev-parse", "main"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "only-here.txt");
    git(&wt, &["push", "-q", "origin", "feat/x:main"]);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/main"]), tip);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "main"]), old_main, "local main is older");

    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("its commits are on origin/main"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);

    let pusher = pusher(&fixture);
    git(&pusher, &["push", "-q", "--force", "origin", &format!("{old_main}:refs/heads/main")]);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/main"]), tip, "the tracking ref is unchanged");

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("no longer safe"))
        .stderr(predicate::str::contains("origin/main has moved"))
        .stderr(predicate::str::contains("Detached HEAD").not())
        .stderr(predicate::str::contains("Removed").not());
    assert_nothing_removed(&fixture, &wt);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip);
}

#[test]
fn a_branch_proved_only_by_origin_default_is_deleted_while_origin_still_has_it() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");
    git(&wt, &["push", "-q", "origin", "feat/x:main"]);
    let (landing, token) = first_run(&fixture, &wt, &[]);

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Deleted branch feat/x"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/x"));
}

/// `feat/x` pushed to origin at the worktree's tip (main's commit, so the
/// local branch is Safe on its own), and the handoff approved with
/// `--force-remote` while origin had it.
fn approved_with_origin_branch_present(fixture: &Fixture) -> (PathBuf, PathBuf, String) {
    let wt = fixture.add_worktree("feat/x", "feat-x");
    git(&wt, &["push", "-q", "-u", "origin", "feat/x"]);
    let (landing, token) = first_run(fixture, &wt, &["--force-remote"]);
    (wt, landing, token)
}

#[test]
fn an_origin_branch_deleted_between_the_runs_refuses_with_nothing_removed() {
    let fixture = Fixture::with_origin();
    let (wt, landing, token) = approved_with_origin_branch_present(&fixture);

    let pusher = pusher(&fixture);
    git(&pusher, &["push", "-q", "origin", "--delete", "feat/x"]);
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("branch on origin changed since you confirmed"))
        .stderr(predicate::str::contains("Detached HEAD").not())
        .stderr(predicate::str::contains("Removed").not());
    assert_nothing_removed(&fixture, &wt);
}

/// The push URL still reads the same, but nothing answers there: an
/// unavailable head is not proof that it did not change.
#[test]
fn an_unreachable_origin_in_the_second_run_refuses_force_remote_with_nothing_removed() {
    let fixture = Fixture::with_origin();
    let (wt, landing, token) = approved_with_origin_branch_present(&fixture);

    let moved = fixture.root.path().join("moved.git");
    fs::rename(fixture.origin(), &moved).unwrap();
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    fs::rename(&moved, fixture.origin()).unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("could not be reached"), "{stderr}");
    assert!(stderr.contains("Nothing was removed"), "{stderr}");
    assert!(!stderr.contains("Removed"), "{stderr}");
    assert_nothing_removed(&fixture, &wt);
    assert!(fixture.origin_has("feat/x"));
}

#[test]
fn an_origin_branch_moved_between_the_runs_refuses_with_nothing_removed() {
    let fixture = Fixture::with_origin();
    let (wt, landing, token) = approved_with_origin_branch_present(&fixture);

    let pusher = pusher(&fixture);
    git(&pusher, &["checkout", "-q", "-b", "feat/x", "origin/feat/x"]);
    let pushed = fixture.commit(&pusher, "their-work.txt");
    git(&pusher, &["push", "-q", "origin", "feat/x"]);
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("branch on origin changed since you confirmed"))
        .stderr(predicate::str::contains("Removed").not());
    assert_nothing_removed(&fixture, &wt);
    assert_eq!(TwoRemotes::head_in(&fixture.origin(), "feat/x"), Some(pushed));
}

#[test]
fn an_origin_branch_still_absent_in_the_second_run_leaves_nothing_to_delete() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &["--force-remote"]);

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed worktree"))
        .stderr(predicate::str::contains("Nothing to delete on origin"));
    assert!(!wt.exists());
    assert!(!fixture.origin_has("feat/x"));
}

#[test]
fn an_origin_branch_created_between_the_runs_refuses_with_nothing_removed() {
    let fixture = Fixture::with_origin();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let (landing, token) = first_run(&fixture, &wt, &["--force-remote"]);

    let pusher = pusher(&fixture);
    git(&pusher, &["push", "-q", "origin", "main:refs/heads/feat/x"]);
    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(3)
        .stderr(predicate::str::contains("branch on origin changed since you confirmed"))
        .stderr(predicate::str::contains("Removed").not());
    assert_nothing_removed(&fixture, &wt);
    assert!(fixture.origin_has("feat/x"));
}

/// A push that lands after the second run's preflight (here from a
/// `pre-push` hook, which runs inside the deletion's own push) fails the
/// approved lease. The local removal has happened by then, so the result is
/// partial.
#[test]
fn a_push_after_the_second_run_preflight_fails_the_lease_with_a_partial_result() {
    let fixture = Fixture::with_origin();
    let main = git(&fixture.repo(), &["rev-parse", "main"]);
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");
    git(&wt, &["push", "-q", "-u", "origin", "feat/x"]);
    let (landing, token) = first_run(&fixture, &wt, &["--force-remote"]);

    let hooks = fixture.root.path().join("hooks");
    fs::create_dir(&hooks).unwrap();
    let origin = fixture.origin().to_str().unwrap().replace('\\', "/");
    let hook = hooks.join("pre-push");
    fs::write(
        &hook,
        format!("#!/bin/sh\ngit --git-dir=\"{origin}\" update-ref refs/heads/feat/x {main}\n"),
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    }
    git(&fixture.repo(), &["config", "core.hooksPath", hooks.to_str().unwrap()]);

    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("Removed worktree"), "{stderr}");
    assert!(stderr.contains("but origin/feat/x was not deleted"), "{stderr}");
    assert!(stderr.contains("git push origin --delete feat/x"), "{stderr}");
    assert!(!wt.exists(), "the local removal has already happened");
    assert!(fixture.branch_exists("feat/x"), "not safe without origin's copy, so kept");
    assert_eq!(TwoRemotes::head_in(&fixture.origin(), "feat/x"), Some(main));
}

// --- Second runs that need no network -------------------------------------------

const PROXIED_ORIGIN: &str = "http://gitea.test/o/r.git";

/// Stands in for every HTTP host: origin is `http://gitea.test/o/r.git` and
/// the proxy variables point at a local port, so each request the PR lookup
/// or a git transport makes arrives here as one connection. It is closed
/// unanswered (origin is unreachable), except that a PR list request gets
/// [`CountingProxy::answer_pulls`]'s body when one is set.
struct CountingProxy {
    address: std::net::SocketAddr,
    requests: std::sync::mpsc::Receiver<(std::net::SocketAddr, String)>,
    pulls: std::sync::Arc<std::sync::Mutex<Option<String>>>,
}

impl CountingProxy {
    fn start() -> Self {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (sender, requests) = std::sync::mpsc::channel();
        let pulls = std::sync::Arc::new(std::sync::Mutex::new(None::<String>));
        let answer = std::sync::Arc::clone(&pulls);
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let Ok(peer) = stream.peer_addr() else { continue };
                // Every client here (reqwest, git's curl, the sentinel) sends
                // its request at once; the timeout only bounds a silent one.
                let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(5)));
                let mut head = Vec::new();
                let mut chunk = [0_u8; 4096];
                while !head.windows(4).any(|window| window == b"\r\n\r\n") && head.len() < 65_536 {
                    match stream.read(&mut chunk) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => head.extend_from_slice(&chunk[..read]),
                    }
                }
                let line = String::from_utf8_lossy(&head).lines().next().unwrap_or_default().to_string();
                if is_pr_lookup(&line)
                    && let Some(body) = answer.lock().unwrap().clone()
                {
                    let _ = write!(
                        stream,
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                }
                if sender.send((peer, line)).is_err() {
                    return;
                }
            }
        });
        Self {
            address,
            requests,
            pulls,
        }
    }

    fn apply(&self, cmd: &mut assert_cmd::Command) {
        let url = format!("http://{}", self.address);
        for name in ["HTTP_PROXY", "http_proxy", "HTTPS_PROXY", "https_proxy", "ALL_PROXY", "all_proxy"] {
            cmd.env(name, &url);
        }
        cmd.env_remove("NO_PROXY").env_remove("no_proxy");
        // A developer's real provider token never reaches the stand-in.
        for name in [
            "GITEA_TOKEN",
            "FORGEJO_TOKEN",
            "CODEBERG_TOKEN",
            "GH_TOKEN",
            "GITHUB_TOKEN",
            "GITLAB_TOKEN",
            "GITLAB_PRIVATE_TOKEN",
            "BITBUCKET_TOKEN",
        ] {
            cmd.env_remove(name);
        }
    }

    /// Serves `body` to every later PR list request; `None` leaves them
    /// unanswered, like every other request.
    fn answer_pulls(&self, body: Option<String>) {
        *self.pulls.lock().unwrap() = body;
    }

    /// The request line of each connection since the previous call. A
    /// sentinel connection is accepted after every earlier one, so none
    /// still in the backlog is missed.
    fn requests(&self) -> Vec<String> {
        use std::io::Write;

        let mut sentinel = std::net::TcpStream::connect(self.address).unwrap();
        sentinel.write_all(b"SENTINEL / HTTP/1.1\r\n\r\n").unwrap();
        let sentinel_address = sentinel.local_addr().unwrap();
        let mut lines = Vec::new();
        loop {
            let (peer, line) = self
                .requests
                .recv_timeout(std::time::Duration::from_secs(30))
                .expect("the proxy accepts the sentinel");
            if peer == sentinel_address {
                return lines;
            }
            lines.push(line);
        }
    }

    /// Connections since the previous call (see [`CountingProxy::requests`]).
    fn connections(&self) -> usize {
        self.requests().len()
    }
}

/// sniff's Gitea PR list (`GET http://gitea.test/api/v1/repos/o/r/pulls?…`);
/// every other request through the proxy is a git transport's.
fn is_pr_lookup(request_line: &str) -> bool {
    request_line.contains("/api/v1/")
}

/// Runs the first run (which asks origin about PRs) and the second run with
/// every HTTP request counted, and returns the second run's stderr after
/// asserting it made none.
fn second_run_without_network(fixture: &Fixture, wt: &Path, extra: &[&str]) -> String {
    git(&fixture.repo(), &["remote", "add", "origin", PROXIED_ORIGIN]);
    let proxy = CountingProxy::start();
    let mut first = fixture.wt(wt);
    proxy.apply(&mut first);
    let out = first
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .args(extra)
        .assert()
        .code(0)
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&out);
    assert!(proxy.connections() > 0, "the first run's PR lookup reaches the proxy");

    let mut second = fixture.wt(&landing);
    proxy.apply(&mut second);
    let output = second.args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert_eq!(proxy.connections(), 0, "the second run made a network request: {stderr}");
    assert!(!wt.exists(), "{stderr}");
    stderr
}

#[test]
fn keeping_the_branch_makes_no_network_request_in_the_second_run() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");

    let stderr = second_run_without_network(&fixture, &wt, &[]);
    assert!(stderr.contains("Kept branch feat/x"), "{stderr}");
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn an_explicitly_deleted_branch_makes_no_network_request_in_the_second_run() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fixture.commit(&wt, "only-here.txt");

    let stderr = second_run_without_network(&fixture, &wt, &["--force-branch"]);
    assert!(stderr.contains("Deleted branch feat/x"), "{stderr}");
    assert!(!stderr.contains("lost"), "safety was not reassessed: {stderr}");
    assert!(!fixture.branch_exists("feat/x"));
}

#[test]
fn automatic_deletion_with_local_proof_makes_no_network_request_in_the_second_run() {
    for proof in ["the local default branch", "another local branch", "a tag"] {
        let fixture = Fixture::new();
        let wt = fixture.add_worktree("feat/x", "feat-x");
        // The worktree starts at main's tip; the other proofs hold a commit
        // of its own.
        if proof != "the local default branch" {
            let tip = fixture.commit(&wt, "only-here.txt");
            let proving_ref = if proof == "a tag" { "refs/tags/v1" } else { "refs/heads/backup" };
            git(&fixture.repo(), &["update-ref", proving_ref, &tip]);
        }

        let stderr = second_run_without_network(&fixture, &wt, &[]);
        assert!(stderr.contains("Deleted branch feat/x"), "{proof}: {stderr}");
        assert!(!fixture.branch_exists("feat/x"), "{proof}");
    }
}

/// A merged Gitea PR from `o/r`'s own `feat/x` whose head is `head`, as
/// sniff's PR list reads it.
fn merged_pr_list(head: &str) -> String {
    serde_json::json!([{
        "number": 7,
        "title": "feat/x",
        "state": "closed",
        "user": {"login": "dev"},
        "head": {"ref": "feat/x", "label": "feat/x", "sha": head, "repo": {"full_name": "o/r"}},
        "base": {"ref": "main", "repo": {"full_name": "o/r"}},
        "created_at": "2026-01-01T00:00:00Z",
        "merged": true,
        "merged_at": "2026-01-02T00:00:00Z",
        "html_url": "http://gitea.test/o/r/pulls/7",
    }])
    .to_string()
}

/// Only a merged PR holds the tip, so the second run must ask the provider
/// again: it deletes while the PR still answers, and refuses before
/// removing anything once the PR is gone or the provider does not answer.
#[test]
fn automatic_deletion_proved_only_by_a_pr_asks_the_provider_again_in_the_second_run() {
    for (second_answer, expected_code) in [("the same PR", 0), ("no PR", 3), ("no answer", 3)] {
        let fixture = Fixture::new();
        git(&fixture.repo(), &["remote", "add", "origin", PROXIED_ORIGIN]);
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let tip = fixture.commit(&wt, "only-here.txt");
        let proxy = CountingProxy::start();
        proxy.answer_pulls(Some(merged_pr_list(&tip)));

        let mut first = fixture.wt(&wt);
        proxy.apply(&mut first);
        let out = first
            .env("WT_SHELL_WRAPPER", "1")
            .args(["remove", "feat-x"])
            .assert()
            .code(0)
            .stderr(predicate::str::contains("PR #7"))
            .get_output()
            .stdout
            .clone();
        let (landing, token) = protocol(&out);
        assert!(proxy.requests().iter().any(|line| is_pr_lookup(line)), "{second_answer}");

        proxy.answer_pulls(match second_answer {
            "the same PR" => Some(merged_pr_list(&tip)),
            "no PR" => Some("[]".to_string()),
            _ => None,
        });
        let mut second = fixture.wt(&landing);
        proxy.apply(&mut second);
        let output = second.args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let requests = proxy.requests();
        assert!(
            requests.iter().any(|line| is_pr_lookup(line)),
            "{second_answer}: the second run asked the provider: {requests:?}"
        );
        assert_eq!(output.status.code(), Some(expected_code), "{second_answer}: {stderr}");
        if expected_code == 0 {
            assert!(stderr.contains("Deleted branch feat/x"), "{second_answer}: {stderr}");
            assert!(!wt.exists());
            assert!(!fixture.branch_exists("feat/x"));
        } else {
            assert!(stderr.contains("no longer safe"), "{second_answer}: {stderr}");
            assert!(!stderr.contains("Removed"), "{second_answer}: {stderr}");
            assert_nothing_removed(&fixture, &wt);
            assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip);
        }
    }
}

/// `feat/x` has a commit of its own that only `origin/backup` holds (not the
/// default branch), and the first run approved automatic deletion on that
/// proof after checking origin live.
fn approved_on_another_origin_branch(fixture: &Fixture) -> (PathBuf, PathBuf, String, String) {
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let tip = fixture.commit(&wt, "only-here.txt");
    git(&wt, &["push", "-q", "origin", "feat/x:refs/heads/backup"]);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/backup"]), tip);
    let out = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("origin/backup"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&out);
    (wt, landing, token, tip)
}

/// The last remote protection is taken away on origin itself; the tracking
/// ref still holds the tip, so only a live check can notice.
#[test]
fn a_branch_proved_only_by_another_origin_branch_refuses_once_origin_drops_it() {
    for change in ["unchanged", "deleted on origin", "moved on origin"] {
        let fixture = Fixture::with_origin();
        let old_main = git(&fixture.repo(), &["rev-parse", "main"]);
        let (wt, landing, token, tip) = approved_on_another_origin_branch(&fixture);

        if change != "unchanged" {
            let pusher = pusher(&fixture);
            match change {
                "deleted on origin" => git(&pusher, &["push", "-q", "origin", "--delete", "backup"]),
                _ => git(&pusher, &["push", "-q", "--force", "origin", &format!("{old_main}:refs/heads/backup")]),
            };
            assert_eq!(git(&fixture.repo(), &["rev-parse", "origin/backup"]), tip, "{change}");
        }

        let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        if change == "unchanged" {
            assert_eq!(output.status.code(), Some(0), "{change}: {stderr}");
            assert!(stderr.contains("Deleted branch feat/x"), "{change}: {stderr}");
            assert!(!fixture.branch_exists("feat/x"), "{change}");
        } else {
            assert_eq!(output.status.code(), Some(3), "{change}: {stderr}");
            assert!(stderr.contains("no longer safe"), "{change}: {stderr}");
            assert!(!stderr.contains("Removed"), "{change}: {stderr}");
            assert_nothing_removed(&fixture, &wt);
            assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip, "{change}");
        }
    }
}

/// The same approval, but origin now answers only through the network,
/// where nothing answers: the second run asks rather than trusting the
/// first run's live check, and an unanswered question refuses.
#[test]
fn automatic_deletion_proved_only_by_another_origin_branch_asks_origin_again_in_the_second_run() {
    let fixture = Fixture::with_origin();
    let (wt, landing, token, tip) = approved_on_another_origin_branch(&fixture);
    git(&fixture.repo(), &["remote", "set-url", "origin", PROXIED_ORIGIN]);

    let proxy = CountingProxy::start();
    let mut second = fixture.wt(&landing);
    proxy.apply(&mut second);
    let output = second.args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let requests = proxy.requests();
    assert!(
        requests.iter().any(|line| !is_pr_lookup(line)),
        "the second run checked origin's head live: {requests:?}"
    );
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("no longer safe"), "{stderr}");
    assert!(!stderr.contains("Removed"), "{stderr}");
    assert_nothing_removed(&fixture, &wt);
    assert_eq!(git(&fixture.repo(), &["rev-parse", "feat/x"]), tip);
}

/// With `--force-remote` the second run makes one preflight of the
/// destination's live head, and nothing else: no PR lookup, whether the
/// branch is kept, explicitly deleted, or proved by a tag. Origin does not
/// answer, so every row refuses before removing anything. The preflight is
/// asserted as "at least one git request" because git's smart-HTTP client
/// may open more than one connection for one `ls-remote`.
#[test]
fn a_force_remote_second_run_makes_only_the_preflight_request() {
    for (approval, extra) in [
        ("keep", &["--force-remote"][..]),
        ("explicit delete", &["--force-remote", "--force-branch"][..]),
        ("tag proof", &["--force-remote"][..]),
    ] {
        let fixture = Fixture::new();
        git(&fixture.repo(), &["remote", "add", "origin", PROXIED_ORIGIN]);
        let wt = fixture.add_worktree("feat/x", "feat-x");
        let tip = fixture.commit(&wt, "only-here.txt");
        if approval == "tag proof" {
            git(&fixture.repo(), &["update-ref", "refs/tags/v1", &tip]);
        }
        let proxy = CountingProxy::start();

        let mut first = fixture.wt(&wt);
        proxy.apply(&mut first);
        let out = first
            .env("WT_SHELL_WRAPPER", "1")
            .args(["remove", "feat-x"])
            .args(extra)
            .assert()
            .code(0)
            .get_output()
            .stdout
            .clone();
        let (landing, token) = protocol(&out);
        assert!(proxy.connections() > 0, "{approval}: the first run reaches the proxy");

        let mut second = fixture.wt(&landing);
        proxy.apply(&mut second);
        let output = second.args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = String::from_utf8_lossy(&output.stderr);
        let requests = proxy.requests();
        assert!(
            !requests.iter().any(|line| is_pr_lookup(line)),
            "{approval}: no PR lookup in the second run: {requests:?}"
        );
        assert!(!requests.is_empty(), "{approval}: the preflight asked origin: {stderr}");
        assert_eq!(output.status.code(), Some(3), "{approval}: {stderr}");
        assert!(stderr.contains("could not be reached"), "{approval}: {stderr}");
        assert!(!stderr.contains("Removed"), "{approval}: {stderr}");
        assert_nothing_removed(&fixture, &wt);
    }
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

// --- Worktrees Git can no longer read -----------------------------------------

fn porcelain(fixture: &Fixture) -> String {
    git(&fixture.repo(), &["worktree", "list", "--porcelain"])
}

/// The porcelain block for the worktree at `path`, if Git still lists it.
fn listed_block(fixture: &Fixture, path: &Path) -> Option<String> {
    let wanted = canonical(path.parent().unwrap()).join(path.file_name().unwrap());
    porcelain(fixture).split("\n\n").find(|block| {
        block.lines().next().and_then(|line| line.strip_prefix("worktree ")).is_some_and(|listed| {
            let listed = Path::new(listed);
            canonical(listed.parent().unwrap()).join(listed.file_name().unwrap()) == wanted
        })
    }).map(str::to_string)
}

fn stderr_of(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

const ALL_FORCE_FLAGS: [&str; 3] = ["--force-worktree", "--force-branch", "--force-remote"];

/// A worktree inside the base checkout. Only there can a caller standing in
/// it with its `.git` gone still reach the repository: Git discovers the
/// enclosing base checkout.
fn add_nested_worktree(fixture: &Fixture, branch: &str, dir_name: &str) -> PathBuf {
    let path = fixture.repo().join(".nested").join(dir_name);
    git(&fixture.repo(), &["worktree", "add", "-q", "-b", branch, path.to_str().unwrap(), "main"]);
    path
}

#[test]
fn a_missing_directory_has_only_its_record_removed_and_its_safe_branch_deleted() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    let other = fixture.add_worktree("feat/other", "feat-other");
    fs::remove_dir_all(&gone).unwrap();
    assert!(listed_block(&fixture, &gone).unwrap().contains("prunable"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-gone"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(0), "{stderr}");
    assert!(stderr.contains("Its directory is already gone"), "{stderr}");
    assert!(stderr.contains("Removed the record of worktree feat-gone"), "{stderr}");
    assert!(stderr.contains("Deleted branch feat/gone"), "{stderr}");
    // Nothing claims files were checked: there were none.
    assert!(!stderr.contains("No uncommitted or ignored files"), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_none(), "{}", porcelain(&fixture));
    assert!(!fixture.branch_exists("feat/gone"));
    assert!(!gone.exists(), "nothing is recreated");
    assert!(listed_block(&fixture, &other).is_some() && other.join("README.md").exists());
    assert!(fixture.branch_exists("feat/other"));
}

#[test]
fn staged_work_left_in_a_missing_directory_record_needs_consent() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    fs::write(gone.join("staged.txt"), "staged\n").unwrap();
    git(&gone, &["add", "staged.txt"]);
    fs::remove_dir_all(&gone).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-gone"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("holds staged changes"), "{stderr}");
    assert!(stderr.contains("staged.txt"), "{stderr}");
    assert!(stderr.contains("--force-worktree"), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_some(), "the record stays");
    assert!(fixture.branch_exists("feat/gone"));

    fixture.wt(&fixture.repo()).args(["remove", "feat-gone", "--force-worktree"]).assert().code(0);
    assert!(listed_block(&fixture, &gone).is_none());
}

#[test]
fn a_missing_detached_directory_has_its_record_removed() {
    let fixture = Fixture::new();
    let gone = fixture.root.path().join("wts").join("detached-gone");
    git(&fixture.repo(), &["worktree", "add", "-q", "--detach", gone.to_str().unwrap(), "main"]);
    fs::remove_dir_all(&gone).unwrap();

    fixture
        .wt(&fixture.repo())
        .args(["remove", "detached-gone"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Detached HEAD"))
        .stderr(predicate::str::contains("Removed the record of worktree detached-gone"));
    assert!(listed_block(&fixture, &gone).is_none());
}

#[test]
fn a_missing_directory_keeps_an_unsafe_branch_without_a_terminal() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    fixture.commit(&gone, "unique.txt");
    fs::remove_dir_all(&gone).unwrap();

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-gone"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed the record"))
        .stderr(predicate::str::contains("kept branch"));
    assert!(listed_block(&fixture, &gone).is_none());
    assert!(fixture.branch_exists("feat/gone"), "its commits exist nowhere else");
}

#[test]
fn a_missing_directory_whose_index_is_gone_refuses_even_with_every_force_flag() {
    let fixture = Fixture::new();
    let gone = fixture.add_worktree("feat/gone", "feat-gone");
    let admin = PathBuf::from(git(&gone, &["rev-parse", "--path-format=absolute", "--git-dir"]));
    fs::remove_dir_all(&gone).unwrap();
    fs::remove_file(admin.join("index")).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-gone"]).args(ALL_FORCE_FLAGS).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Can't remove feat-gone"), "{stderr}");
    assert!(stderr.contains("staged work can't be ruled out"), "{stderr}");
    assert!(stderr.contains("No working files, branches, or worktree records were removed"), "{stderr}");
    assert!(!stderr.contains("prune"), "{stderr}");
    assert!(listed_block(&fixture, &gone).is_some());
    assert!(fixture.branch_exists("feat/gone"));
}

/// The defect's original case, shaped like the observed one: detached, its
/// `.git` file gone, a working file kept.
#[test]
fn an_unlinked_worktree_is_repaired_then_its_files_are_protected() {
    let fixture = Fixture::new();
    let wt = fixture.root.path().join("wts").join("lhg-before");
    git(&fixture.repo(), &["worktree", "add", "-q", "--detach", wt.to_str().unwrap(), "main"]);
    fs::write(wt.join("README.md"), "edited, never committed\n").unwrap();
    fs::remove_file(wt.join(".git")).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "lhg-before"]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Restored the link for lhg-before so its files could be checked"), "{stderr}");
    assert!(stderr.contains("Uncommitted files (1)"), "{stderr}");
    assert!(stderr.contains("Nothing was removed"), "{stderr}");
    assert!(stderr.contains("The .git link restored for lhg-before was left in place"), "{stderr}");
    assert!(!stderr.contains("not a git repository"), "{stderr}");
    assert_eq!(fs::read_to_string(wt.join("README.md")).unwrap(), "edited, never committed\n");
    assert!(wt.join(".git").is_file(), "the repaired link is not rolled back");
    let block = listed_block(&fixture, &wt).expect("still registered");
    assert!(!block.contains("prunable"), "{block}");

    fixture.wt(&fixture.repo()).args(["remove", "lhg-before", "--force-worktree"]).assert().code(0);
    assert!(!wt.exists());
    assert!(listed_block(&fixture, &wt).is_none());
}

#[test]
fn a_clean_unlinked_worktree_is_repaired_and_removed_with_its_safe_branch() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::remove_file(wt.join(".git")).unwrap();

    fixture
        .wt(&fixture.repo())
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Restored the link for feat-x"))
        .stderr(predicate::str::contains("No uncommitted or ignored files"))
        .stderr(predicate::str::contains("Removed worktree feat-x"))
        .stderr(predicate::str::contains("Deleted branch feat/x"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/x"));
}

/// A directory Git can't write into: `git worktree repair` cannot recreate
/// `.git`, so no postcondition holds.
#[cfg(unix)]
#[test]
fn a_repair_that_is_not_verified_refuses_even_with_every_force_flag() {
    use std::os::unix::fs::PermissionsExt;

    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::write(wt.join("notes.txt"), "keep me\n").unwrap();
    fs::remove_file(wt.join(".git")).unwrap();
    fs::set_permissions(&wt, fs::Permissions::from_mode(0o555)).unwrap();
    struct Writable<'a>(&'a Path);
    impl Drop for Writable<'_> {
        fn drop(&mut self) {
            let _ = fs::set_permissions(self.0, fs::Permissions::from_mode(0o755));
        }
    }
    let _restore = Writable(&wt);
    if fs::write(wt.join("probe"), "").is_ok() {
        eprintln!("skipped: this user can write into a read-only directory");
        return;
    }

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).args(ALL_FORCE_FLAGS).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(
        stderr.contains("Can't remove feat-x: its .git file was missing and Git couldn't restore a verified link."),
        "{stderr}"
    );
    assert!(stderr.contains("No working files, branches, or worktree records were removed."), "{stderr}");
    assert!(stderr.contains("The repair attempt may have changed Git metadata"), "{stderr}");
    assert!(stderr.contains("git worktree list --porcelain"), "{stderr}");
    // Git's spelling of the path, which `canonical` matches on macOS too.
    assert!(stderr.contains(&format!("git worktree repair {}", canonical(&wt).display())), "{stderr}");
    assert!(stderr.contains("retry wt remove feat-x"), "{stderr}");
    assert_eq!(stderr.matches("Not verified: Git couldn't resolve").count(), 1, "said once: {stderr}");
    assert!(stderr.contains("Repair output:"), "{stderr}");
    assert!(!stderr.contains("Restored the link"), "never claimed: {stderr}");
    assert!(!stderr.contains("prune"), "{stderr}");
    assert!(!stderr.contains("Nothing was changed"), "{stderr}");
    assert_eq!(fs::read_to_string(wt.join("notes.txt")).unwrap(), "keep me\n");
    assert!(listed_block(&fixture, &wt).is_some(), "the record stays");
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn a_worktree_replaced_by_a_file_refuses_even_with_every_force_flag() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::remove_dir_all(&wt).unwrap();
    fs::write(&wt, "not a checkout\n").unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).args(ALL_FORCE_FLAGS).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("Can't remove feat-x: Git can't read this worktree"), "{stderr}");
    assert!(stderr.contains("is not a directory"), "{stderr}");
    assert!(stderr.contains("No working files, branches, or worktree records were removed"), "{stderr}");
    assert_eq!(fs::read_to_string(&wt).unwrap(), "not a checkout\n");
    assert!(listed_block(&fixture, &wt).is_some());
    assert!(fixture.branch_exists("feat/x"));
}

#[cfg(unix)]
#[test]
fn a_worktree_replaced_by_a_link_is_never_followed() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    let elsewhere = fixture.root.path().join("elsewhere");
    fs::create_dir(&elsewhere).unwrap();
    fs::write(elsewhere.join("precious.txt"), "precious\n").unwrap();
    fs::remove_dir_all(&wt).unwrap();
    std::os::unix::fs::symlink(&elsewhere, &wt).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).args(ALL_FORCE_FLAGS).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("is a link"), "{stderr}");
    assert!(fs::symlink_metadata(&wt).unwrap().file_type().is_symlink());
    assert_eq!(fs::read_to_string(elsewhere.join("precious.txt")).unwrap(), "precious\n");
    assert!(!elsewhere.join(".git").exists(), "nothing was repaired through the link");
    assert!(fixture.branch_exists("feat/x"));
}

/// Not prunable, so nothing is prepared, but `git status` fails: the error
/// names the target and the operation, and keeps its exit code.
#[test]
fn a_status_failure_names_the_target_and_operation_and_removes_nothing() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    fs::write(wt.join(".git"), "garbage\n").unwrap();
    assert!(!listed_block(&fixture, &wt).unwrap().contains("prunable"));

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).args(ALL_FORCE_FLAGS).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("could not check the files of worktree feat-x at "), "{stderr}");
    assert!(stderr.contains("fatal:"), "Git's own reason is kept: {stderr}");
    assert!(!stderr.contains("Removed"), "{stderr}");
    assert!(!stderr.contains("left in place"), "nothing was repaired: {stderr}");
    assert_eq!(fs::read_to_string(wt.join(".git")).unwrap(), "garbage\n");
    assert!(wt.join("README.md").exists());
    assert!(fixture.branch_exists("feat/x"));
}

/// Git never marks a locked record `prunable`, so a locked worktree whose
/// directory is gone takes the ordinary path, where the status check fails.
#[test]
fn a_locked_worktree_whose_directory_is_gone_fails_with_context_and_keeps_everything() {
    let fixture = Fixture::new();
    let wt = fixture.add_worktree("feat/x", "feat-x");
    git(&fixture.repo(), &["worktree", "lock", wt.to_str().unwrap()]);
    fs::remove_dir_all(&wt).unwrap();

    let output = fixture.wt(&fixture.repo()).args(["remove", "feat-x"]).args(ALL_FORCE_FLAGS).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.contains("could not check the files of worktree feat-x"), "{stderr}");
    assert!(listed_block(&fixture, &wt).unwrap().contains("locked"));
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn a_repaired_worktree_hands_off_and_finishes_with_the_ordinary_checks() {
    let fixture = Fixture::new();
    let wt = add_nested_worktree(&fixture, "feat/x", "feat-x");
    fs::remove_file(wt.join(".git")).unwrap();

    let first = fixture
        .wt(&wt)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["remove", "feat-x"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Restored the link for feat-x"))
        .get_output()
        .stdout
        .clone();
    let (landing, token) = protocol(&first);
    assert!(wt.join(".git").is_file());

    fixture
        .wt(&landing)
        .args(["remove", "--handoff", &token])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("Removed worktree"));
    assert!(!wt.exists());
    assert!(!fixture.branch_exists("feat/x"));
}

#[test]
fn a_link_broken_between_the_runs_refuses_without_another_repair() {
    let fixture = Fixture::new();
    let wt = add_nested_worktree(&fixture, "feat/x", "feat-x");
    fs::remove_file(wt.join(".git")).unwrap();
    let (landing, token) = first_run(&fixture, &wt, &[]);

    fs::remove_file(wt.join(".git")).unwrap();
    let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
    let stderr = stderr_of(&output);
    assert_eq!(output.status.code(), Some(3), "{stderr}");
    assert!(stderr.contains("broke since you confirmed"), "{stderr}");
    assert!(stderr.contains("Nothing was removed"), "{stderr}");
    assert!(!wt.join(".git").exists(), "the second run never repairs");
    assert!(wt.join("README.md").exists());
    assert!(fixture.branch_exists("feat/x"));
}

#[test]
fn a_link_redirected_between_the_runs_refuses_with_nothing_removed() {
    for repaired_first in [true, false] {
        let fixture = Fixture::new();
        let wt = add_nested_worktree(&fixture, "feat/x", "feat-x");
        let other = fixture.add_worktree("feat/y", "feat-y");
        if repaired_first {
            fs::remove_file(wt.join(".git")).unwrap();
        }
        let (landing, token) = first_run(&fixture, &wt, &[]);

        let other_record = git(&other, &["rev-parse", "--path-format=absolute", "--git-dir"]);
        fs::write(wt.join(".git"), format!("gitdir: {other_record}\n")).unwrap();
        let output = fixture.wt(&landing).args(["remove", "--handoff", &token]).output().unwrap();
        let stderr = stderr_of(&output);
        assert_eq!(output.status.code(), Some(3), "repaired first: {repaired_first}: {stderr}");
        assert!(stderr.contains("worktree link"), "{stderr}");
        assert!(wt.join("README.md").exists());
        assert!(other.join("README.md").exists());
        assert!(fixture.branch_exists("feat/x") && fixture.branch_exists("feat/y"));
    }
}
