//! Integration tests for the `wt list --perf` runtime performance report.

use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::process::Command;

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

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {:?} failed in {:?}", args, repo);
}

#[test]
fn list_perf_emits_report_on_stderr_and_empty_stdout() {
    let repo = temp_repo();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["list", "--perf"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Performance"))
        .stderr(predicate::str::contains("pre-dispatch"))
        .stderr(predicate::str::contains("list gather"))
        .stderr(predicate::str::contains("table render"));
}

#[test]
fn list_without_perf_emits_no_report() {
    let repo = temp_repo();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["list"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Performance").not())
        .stderr(predicate::str::contains("pre-dispatch").not())
        .stderr(predicate::str::contains("list gather").not());
}

fn temp_repo_with_feature_branch() -> tempfile::TempDir {
    let dir = temp_repo();
    let path = dir.path();

    fs::write(path.join("file.txt"), "2\n").unwrap();
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "commit 2"]);

    run_git(path, &["checkout", "-b", "feature-a"]);
    fs::write(path.join("a.txt"), "a\n").unwrap();
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-m", "feature a"]);
    run_git(path, &["checkout", "main"]);

    dir
}

#[test]
fn list_perf_non_image_terminal_omits_graph_stages() {
    let repo = temp_repo();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .args(["list", "--perf"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("pre-dispatch"))
        .stderr(predicate::str::contains("list gather"))
        .stderr(predicate::str::contains("table render"))
        .stderr(predicate::str::contains("graph gather").not())
        .stderr(predicate::str::contains("graph image render").not());
}

#[test]
fn list_perf_non_image_verbose_includes_verbose_gather() {
    let repo = temp_repo_with_feature_branch();
    let repo_path = repo.path();
    let feature_path = repo_path
        .parent()
        .expect("temp dir has a parent")
        .join(format!(
            "{}-feature",
            repo_path.file_name().unwrap().to_string_lossy()
        ));
    run_git(repo_path, &["worktree", "add", feature_path.to_str().unwrap(), "feature-a"]);

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(&feature_path)
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .args(["list", "-v", "--perf"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("pre-dispatch"))
        .stderr(predicate::str::contains("list gather"))
        .stderr(predicate::str::contains("table render"))
        .stderr(predicate::str::contains("verbose gather"))
        .stderr(predicate::str::contains("verbose render"))
        .stderr(predicate::str::contains("graph gather").not())
        .stderr(predicate::str::contains("graph image render").not());
}

#[test]
fn list_perf_error_path_emits_no_report() {
    let dir = tempfile::tempdir().expect("create temp dir");

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(dir.path())
        .args(["list", "--perf"])
        .assert()
        .failure()
        .stdout(predicate::str::is_empty())
        .stderr(predicate::str::contains("Performance").not());
}
