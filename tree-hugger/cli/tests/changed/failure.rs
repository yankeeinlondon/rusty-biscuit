//! The Git failure rows: repository absence, a bare repository, unresolved
//! and non-commit references, conflicts outside the selected package, and
//! corrupt Git data — each exits 1 with an actionable stderr message and
//! never prints a report on stdout.

use crate::{Repo, conflict_at, directory_outside_any_repository, git_init_bare, head_tree_id};

/// Asserts `output` is an exit-1 failure whose stderr names `needle`, with
/// nothing (no success JSON) on stdout.
fn assert_failure(output: &std::process::Output, args: &[&str], needle: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(1),
        "hug {args:?} must exit 1: {stderr}"
    );
    assert!(stderr.contains(needle), "hug {args:?}: `{needle}` in\n{stderr}");
    assert!(
        output.stdout.is_empty(),
        "hug {args:?} prints no report on stdout, got:\n{}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn outside_any_repository_the_run_fails_naming_the_directory() {
    let dir = directory_outside_any_repository();
    let repo = Repo::unborn();
    assert_failure(
        &repo.hug_in(dir.path(), &["--json", "symbols", "--changed"]),
        &["--json", "symbols", "--changed"],
        "needs a git repository",
    );
}

#[test]
fn a_bare_repository_is_rejected() {
    let bare = tempfile::TempDir::new().unwrap();
    git_init_bare(bare.path());
    let repo = Repo::unborn();
    assert_failure(
        &repo.hug_in(bare.path(), &["--json", "symbols", "--changed"]),
        &["--json", "symbols", "--changed"],
        "bare repository has no working tree",
    );
}

#[test]
fn an_unresolved_reference_fails_naming_the_reference() {
    let repo = Repo::committed(&[("lib.rs", "fn one() {}\n")]);
    let output = repo.hug(&["--json", "symbols", "--changed=nope"]);
    assert_failure(&output, &["--json", "symbols", "--changed=nope"], "--changed=nope");
    assert_failure(&output, &["--json", "symbols", "--changed=nope"], "'nope' does not resolve to any object");
}

#[test]
fn a_non_commit_reference_fails_naming_the_kind() {
    let repo = Repo::committed(&[("lib.rs", "fn one() {}\n")]);
    let tree = head_tree_id(repo.dir.path());
    let flag = format!("--changed={tree}");
    let args = ["--json", "symbols", flag.as_str()];
    let output = repo.hug(&args);
    assert_failure(&output, &args, "resolves to a tree, not a commit");
    assert_failure(&output, &args, &tree);
}

#[test]
fn conflicts_outside_the_package_still_fail() {
    let repo = Repo::committed(&[
        ("Cargo.toml", "[workspace]\n"),
        ("pkg/Cargo.toml", "[package]\nname = \"p\"\nversion = \"0.1.0\"\n"),
        ("pkg/lib.rs", "fn in_package() {}\n"),
    ]);
    // Conflicted entries far outside the selected package.
    let handle = git2::Repository::open(repo.dir.path()).unwrap();
    conflict_at(&handle, "other/conflicted.txt", "ours\n");
    drop(handle);

    let pkg = repo.dir.path().join("pkg");
    let output = repo.hug_in(&pkg, &["--json", "symbols", "--changed"]);
    assert_failure(&output, &["--json", "symbols", "--changed"], "unresolved merge conflicts");
    assert_failure(&output, &["--json", "symbols", "--changed"], "other/conflicted.txt");
}

#[test]
fn corrupt_git_data_fails_without_a_report() {
    let repo = Repo::committed(&[("lib.rs", "fn one() {}\n")]);
    std::fs::write(repo.dir.path().join(".git").join("HEAD"), "zzz").unwrap();
    let output = repo.hug(&["--json", "symbols", "--changed"]);
    assert_failure(&output, &["--json", "symbols", "--changed"], "could not read the repository");
}

#[test]
fn a_blob_reference_is_not_a_commit() {
    let repo = Repo::committed(&[("lib.rs", "fn one() {}\n")]);
    let handle = git2::Repository::open(repo.dir.path()).unwrap();
    let blob = handle.blob(b"not a commit").unwrap().to_string();
    let flag = format!("--changed={blob}");
    let args = ["--json", "symbols", flag.as_str()];
    let output = repo.hug(&args);
    assert_failure(&output, &args, "resolves to a blob, not a commit");
    assert_failure(&output, &args, &blob);
}
