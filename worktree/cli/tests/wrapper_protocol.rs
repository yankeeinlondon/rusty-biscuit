//! Exit codes, shell-wrapper detection, name resolution, and `create --from`
//! through the real `wt` binary.
//!
//! Every spawn removes `WT_SHELL_WRAPPER` and `CI` first, so a developer
//! running the suite from inside a wrapper or a CI shell sees the same
//! results; tests that need either set it explicitly.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use predicates::prelude::*;

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {repo:?}");
}

fn git_output(repo: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(repo)
        .args(args)
        .output()
        .expect("git should be installed");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

/// A repository on `main` with one commit.
fn temp_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path();
    run_git(path, &["init", "-b", "main"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    run_git(path, &["config", "commit.gpgsign", "false"]);
    fs::write(path.join("README.md"), "# test\n").unwrap();
    run_git(path, &["add", "README.md"]);
    run_git(path, &["commit", "-m", "initial"]);
    dir
}

fn wt(cwd: &Path) -> assert_cmd::Command {
    let mut cmd = assert_cmd::Command::cargo_bin("wt").unwrap();
    cmd.current_dir(cwd)
        .env_remove("WT_SHELL_WRAPPER")
        .env_remove("CI")
        .env_remove("COMPLETE")
        .write_stdin("");
    cmd
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

/// The path of the single `cd:` line on `stdout`.
fn cd_target(stdout: &[u8]) -> PathBuf {
    let text = String::from_utf8_lossy(stdout);
    let lines: Vec<&str> = text.lines().filter(|l| l.starts_with("cd:")).collect();
    assert_eq!(lines.len(), 1, "expected one cd: line in {text:?}");
    canonical(Path::new(&lines[0]["cd:".len()..]))
}

// --- Exit codes ------------------------------------------------------------

#[test]
fn exit_0_for_completions() {
    wt(Path::new("."))
        .args(["--completions", "zsh"])
        .assert()
        .code(0)
        .stdout(predicate::str::contains("WT_SHELL_WRAPPER=1 command wt"));
}

#[test]
fn exit_1_for_an_unknown_worktree() {
    let repo = temp_repo();
    wt(repo.path())
        .args(["go", "no-such-worktree"])
        .env("WT_SHELL_WRAPPER", "1")
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn exit_2_for_invalid_arguments() {
    wt(Path::new(".")).arg("--no-such-flag").assert().code(2);
    wt(Path::new("."))
        .args(["--completions", "elvish"])
        .assert()
        .code(2)
        .stderr(predicate::str::contains("bash, zsh, fish, powershell"));
}

#[test]
fn exit_3_when_removal_would_lose_files_and_nobody_can_confirm() {
    let repo = temp_repo();
    let wt_path = repo.path().join("wt-dirty");
    run_git(
        repo.path(),
        &["worktree", "add", wt_path.to_str().unwrap(), "-b", "feat/dirty"],
    );
    fs::write(wt_path.join("notes.txt"), "keep me\n").unwrap();

    for ci in [None, Some("true")] {
        let mut cmd = wt(repo.path());
        if let Some(ci) = ci {
            cmd.env("CI", ci);
        }
        cmd.args(["remove", "feat/dirty"])
            .assert()
            .code(3)
            .stderr(predicate::str::contains("Nothing was removed"));
        assert!(wt_path.join("notes.txt").exists(), "CI={ci:?}");
    }
}

#[test]
fn exit_4_for_go_without_the_shell_wrapper() {
    let repo = temp_repo();
    let wt_path = repo.path().join("feat-go");
    run_git(
        repo.path(),
        &["worktree", "add", wt_path.to_str().unwrap(), "-b", "feat/go"],
    );

    for value in [None, Some("0"), Some("")] {
        let mut cmd = wt(repo.path());
        if let Some(value) = value {
            cmd.env("WT_SHELL_WRAPPER", value);
        }
        let assert = cmd.args(["go", "feat/go"]).assert().code(4).stdout("");
        let stderr = String::from_utf8_lossy(&assert.get_output().stderr).into_owned();
        assert!(stderr.contains("Shell wrapper not active"), "{stderr}");
        for line in [
            "source <(wt --completions bash)",
            "source <(wt --completions zsh)",
            "wt --completions fish | source",
            "wt --completions powershell | Out-String | Invoke-Expression",
        ] {
            assert!(stderr.contains(line), "WT_SHELL_WRAPPER={value:?}: missing {line}");
        }
    }
}

// --- wt go and name resolution ----------------------------------------------

#[test]
fn go_prints_cd_only_with_the_wrapper_and_resolves_branch_and_basename() {
    let repo = temp_repo();
    // Outside the base checkout: a nested worktree also counts as "inside" base.
    let worktrees = tempfile::tempdir().unwrap();
    let wt_path = worktrees.path().join("theme-dir");
    run_git(
        repo.path(),
        &["worktree", "add", wt_path.to_str().unwrap(), "-b", "feat/theme"],
    );

    for name in ["feat/theme", "theme-dir"] {
        let assert = wt(repo.path())
            .env("WT_SHELL_WRAPPER", "1")
            .args(["go", name])
            .assert()
            .code(0);
        assert_eq!(cd_target(&assert.get_output().stdout), canonical(&wt_path), "{name}");
    }

    let assert = wt(&wt_path)
        .env("WT_SHELL_WRAPPER", "1")
        .args(["go", "base"])
        .assert()
        .code(0);
    assert_eq!(cd_target(&assert.get_output().stdout), canonical(repo.path()));
}

#[test]
fn go_with_an_ambiguous_name_lists_every_match_and_moves_nowhere() {
    let repo = temp_repo();
    let parser_work = repo.path().join("parser-work");
    let parser_dir = repo.path().join("parser");
    run_git(
        repo.path(),
        &["worktree", "add", parser_work.to_str().unwrap(), "-b", "parser"],
    );
    run_git(
        repo.path(),
        &["worktree", "add", parser_dir.to_str().unwrap(), "-b", "other"],
    );

    wt(repo.path())
        .env("WT_SHELL_WRAPPER", "1")
        .args(["go", "parser"])
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("matches more than one worktree"))
        .stderr(predicate::str::contains("branch parser, directory parser-work"))
        .stderr(predicate::str::contains("branch other, directory parser"));
}

#[test]
fn completions_offer_branch_and_directory_names() {
    let repo = temp_repo();
    let wt_path = repo.path().join("theme-dir");
    run_git(
        repo.path(),
        &["worktree", "add", wt_path.to_str().unwrap(), "-b", "feat/theme"],
    );

    let assert = wt(repo.path())
        .env("COMPLETE", "fish")
        .args(["--", "wt", "go", ""])
        .assert()
        .code(0);
    let offered: Vec<String> = String::from_utf8_lossy(&assert.get_output().stdout)
        .lines()
        .map(|line| line.split('\t').next().unwrap_or_default().to_string())
        .collect();
    for name in ["base", "main", "feat/theme", "theme-dir"] {
        assert!(offered.iter().any(|o| o == name), "{name} missing from {offered:?}");
    }
}

#[test]
fn remove_refuses_the_base_checkout_by_any_name_before_prompting() {
    let repo = temp_repo();
    for name in ["base", "main"] {
        wt(repo.path())
            .args(["remove", name])
            .assert()
            .code(1)
            .stderr(predicate::str::contains("main checkout"));
    }
    assert!(repo.path().join("README.md").exists());
}

// --- wt create ---------------------------------------------------------------

struct CreateFixture {
    repo: tempfile::TempDir,
    base: tempfile::TempDir,
    store: PathBuf,
}

impl CreateFixture {
    fn new() -> Self {
        let repo = temp_repo();
        run_git(repo.path(), &["checkout", "-b", "feat/theme"]);
        fs::write(repo.path().join("theme.txt"), "theme\n").unwrap();
        run_git(repo.path(), &["add", "theme.txt"]);
        run_git(repo.path(), &["commit", "-m", "theme"]);
        run_git(repo.path(), &["checkout", "main"]);
        let store = worktree::fork_origin::fork_origin_path(repo.path()).unwrap();
        let _ = fs::remove_file(&store);
        Self {
            repo,
            base: tempfile::tempdir().unwrap(),
            store,
        }
    }

    fn wt(&self) -> assert_cmd::Command {
        let mut cmd = wt(self.repo.path());
        cmd.env("WT", self.base.path());
        cmd
    }

    fn worktree_path(&self, dir: &str) -> PathBuf {
        let repo_name = self.repo.path().file_name().unwrap();
        self.base.path().join(repo_name).join(dir)
    }
}

impl Drop for CreateFixture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.store);
    }
}

#[test]
fn create_without_the_wrapper_creates_but_cannot_move() {
    let fixture = CreateFixture::new();
    fixture
        .wt()
        .args(["create", "fix/a"])
        .assert()
        .code(0)
        .stdout("")
        .stderr(predicate::str::contains("Created worktree"))
        .stderr(predicate::str::contains("Could not move your shell"))
        .stderr(predicate::str::contains("have been moved").not());
    assert!(fixture.worktree_path("fix-a").exists());
}

#[test]
fn create_with_the_wrapper_moves_and_stay_does_not() {
    let fixture = CreateFixture::new();
    let assert = fixture
        .wt()
        .env("WT_SHELL_WRAPPER", "1")
        .args(["create", "fix/b"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("have been moved"));
    assert_eq!(
        cd_target(&assert.get_output().stdout),
        canonical(&fixture.worktree_path("fix-b"))
    );

    fixture
        .wt()
        .args(["create", "fix/c", "--stay"])
        .assert()
        .code(0)
        .stdout("")
        .stderr(predicate::str::contains("Could not move").not());
    assert!(fixture.worktree_path("fix-c").exists());
}

#[test]
fn create_from_forks_the_named_base_and_records_it() {
    let fixture = CreateFixture::new();
    fixture
        .wt()
        .args(["create", "fix/d", "--from", "feat/theme", "--stay"])
        .assert()
        .code(0)
        .stderr(predicate::str::contains("forked from"))
        .stderr(predicate::str::contains("feat/theme"));

    let theme_tip = git_output(fixture.repo.path(), &["rev-parse", "feat/theme"]);
    assert_eq!(
        git_output(&fixture.worktree_path("fix-d"), &["rev-parse", "HEAD"]),
        theme_tip
    );
    let store = worktree::fork_origin::ForkOriginStore::load_from(&fixture.store);
    let origin = store.get("fix/d").expect("fork origin recorded");
    assert_eq!(origin.base_branch, "feat/theme");
    assert_eq!(origin.base_sha, theme_tip);
}

#[test]
fn create_from_errors_use_the_ruled_messages_and_create_nothing() {
    let fixture = CreateFixture::new();

    fixture
        .wt()
        .args(["create", "feat/theme", "--from", "main"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "`feat/theme` already exists, so `--from main` would be ignored. Drop `--from` to reuse it.",
        ));
    assert!(!fixture.worktree_path("feat-theme").exists());

    fixture
        .wt()
        .args(["create", "fix/e", "--from", "no-such-base"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains(
            "`--from no-such-base` does not name an existing local branch",
        ));
    assert!(!fixture.worktree_path("fix-e").exists());

    let head = git_output(fixture.repo.path(), &["rev-parse", "HEAD"]);
    run_git(fixture.repo.path(), &["checkout", "--detach", &head]);
    fixture
        .wt()
        .args(["create", "fix/f"])
        .assert()
        .code(1)
        .stderr(predicate::str::contains("HEAD is detached"))
        .stderr(predicate::str::contains("--from <branch>"));
    assert!(!fixture.worktree_path("fix-f").exists());
    assert!(!fixture.store.exists(), "no failed create may write a record");
}

#[test]
fn create_from_completes_local_branches() {
    let fixture = CreateFixture::new();
    let assert = fixture
        .wt()
        .env("COMPLETE", "fish")
        .args(["--", "wt", "create", "x", "--from", ""])
        .assert()
        .code(0);
    let offered = String::from_utf8_lossy(&assert.get_output().stdout).into_owned();
    assert!(offered.lines().any(|l| l == "main"), "{offered}");
    assert!(offered.lines().any(|l| l == "feat/theme"), "{offered}");
}

// --- Generated wrappers --------------------------------------------------------

#[test]
fn wrapper_snapshots() {
    use clap_complete::Shell;
    use worktree_cli::shell_integration::wrapper;

    let exe = "/opt/bin/wt";
    insta::assert_snapshot!("bash", wrapper(Shell::Bash, exe).unwrap());
    insta::assert_snapshot!("zsh", wrapper(Shell::Zsh, exe).unwrap());
    insta::assert_snapshot!("fish", wrapper(Shell::Fish, exe).unwrap());
    insta::assert_snapshot!("powershell", wrapper(Shell::PowerShell, exe).unwrap());
}
