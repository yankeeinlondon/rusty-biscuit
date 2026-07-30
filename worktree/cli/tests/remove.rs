use predicates::prelude::*;
use std::fs;

use std::process::Command;

/// Returns a temporary git repository initialised with an initial commit.
fn temp_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path();

    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    run_git(path, &["config", "commit.gpgsign", "false"]);

    fs::write(path.join("README.md"), "# test\n").unwrap();
    run_git(path, &["add", "README.md"]);
    run_git(path, &["commit", "-m", "initial"]);

    dir
}

fn run_git(repo: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {:?} failed in {:?}", args, repo);
}

fn add_worktree(repo: &std::path::Path, branch: &str, path: &str) {
    run_git(repo, &["worktree", "add", path, "-b", branch]);
}

// =============================================================================
//                              CLI PARSING TESTS
// =============================================================================

#[test]
fn remove_help_shows_usage() {
    assert_cmd::Command::cargo_bin("wt").unwrap()
        .args(["remove", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Remove a worktree"))
        .stdout(predicate::str::contains("--force"))
        .stdout(predicate::str::contains("--branch"));
}

#[test]
fn remove_missing_name_fails() {
    assert_cmd::Command::cargo_bin("wt").unwrap()
        .args(["remove"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn remove_nonexistent_worktree_fails() {
    let repo = temp_repo();
    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["remove", "no-such-worktree"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found").or(predicate::str::contains("WorktreeNotFound")));
}

// =============================================================================
//                              FUNCTIONAL TESTS
// =============================================================================

#[test]
fn remove_clean_worktree_with_force() {
    let repo = temp_repo();
    let wt_path = repo.path().join("wt-test");
    add_worktree(repo.path(), "feat/test", wt_path.to_str().unwrap());

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["remove", "feat/test", "-f"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed worktree"));

    assert!(!wt_path.exists(), "worktree directory should be removed");
}

#[test]
fn remove_clean_worktree_with_ff() {
    let repo = temp_repo();
    let wt_path = repo.path().join("wt-test");
    add_worktree(repo.path(), "feat/ff", wt_path.to_str().unwrap());

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["remove", "feat/ff", "-ff"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed worktree"));

    assert!(!wt_path.exists(), "worktree directory should be removed");
}

#[test]
fn remove_dirty_worktree_with_ff() {
    let repo = temp_repo();
    let wt_path = repo.path().join("wt-dirty");
    add_worktree(repo.path(), "feat/dirty", wt_path.to_str().unwrap());

    // Create an uncommitted file inside the worktree
    fs::write(wt_path.join("dirty.txt"), "dirty content\n").unwrap();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["remove", "feat/dirty", "-ff"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed worktree"));

    assert!(!wt_path.exists(), "worktree directory should be removed");
}

#[test]
fn remove_with_branch_flag_deletes_branch() {
    let repo = temp_repo();
    let wt_path = repo.path().join("wt-branch");
    add_worktree(repo.path(), "feat/branch", wt_path.to_str().unwrap());

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["remove", "feat/branch", "-ff", "-b"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Deleted branch").or(predicate::str::contains("was preserved")));

    assert!(!wt_path.exists(), "worktree directory should be removed");
}

#[test]
fn remove_main_worktree_is_rejected() {
    let repo = temp_repo();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["remove", "base", "-ff"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("main checkout").or(predicate::str::contains("cannot remove the main worktree")));
}

#[test]
fn remove_dirty_worktree_with_f_non_source_bypasses() {
    let repo = temp_repo();
    let wt_path = repo.path().join("wt-bypass");
    add_worktree(repo.path(), "feat/bypass", wt_path.to_str().unwrap());

    // Create a single non-source dirty file (< 10 files, no source)
    fs::write(wt_path.join("notes.txt"), "notes\n").unwrap();

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["remove", "feat/bypass", "-f"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Removed worktree"));

    assert!(!wt_path.exists(), "worktree directory should be removed");
}

#[test]
fn remove_preserves_unmerged_branch() {
    let repo = temp_repo();
    let wt_path = repo.path().join("wt-unmerged");
    add_worktree(repo.path(), "feat/unmerged", wt_path.to_str().unwrap());

    // Create a unique commit in the worktree so the branch is not merged
    fs::write(wt_path.join("unique.md"), "unique content\n").unwrap();
    run_git(&wt_path, &["add", "unique.md"]);
    run_git(&wt_path, &["commit", "-m", "diverge"]);

    assert_cmd::Command::cargo_bin("wt").unwrap()
        .current_dir(repo.path())
        .args(["remove", "feat/unmerged", "-ff", "-b"])
        .assert()
        .success()
        .stderr(predicate::str::contains("Warning:").and(predicate::str::contains("preserved")));

    assert!(!wt_path.exists(), "worktree directory should be removed");

    // Branch should still exist
    let branch_list = String::from_utf8(
        Command::new("git")
            .current_dir(repo.path())
            .args(["branch", "--list", "feat/unmerged"])
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    assert!(branch_list.contains("feat/unmerged"), "unmerged branch should be preserved");
}
