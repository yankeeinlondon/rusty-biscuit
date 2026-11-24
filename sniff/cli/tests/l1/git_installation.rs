//! `sniff` names the Git for Windows installation in `EXEPATH` before it opens
//! a repository, so `gix` finds Git's system configuration without starting
//! `git` (with a hidden console) to ask where it is installed.
//!
//! gix-path's fallbacks derive the same prefix as the hint, so comparing
//! prefixes cannot show which route was taken; these tests count the Git
//! processes a run starts instead (`GIT_TRACE2_EVENT=<dir>` writes one file
//! per process).

#![cfg(windows)]

use std::fs;
use std::path::Path;
use std::process::Output;

use crate::common::SniffCliFixture;

/// `sniff repo git-status --json` in a fresh one-commit repository, with
/// `exepath` as the caller's `EXEPATH` (removed when `None`), returning the
/// run and the number of Git processes it started.
fn git_status(exepath: Option<&Path>) -> (Output, usize) {
    let fixture = SniffCliFixture::named("sniff-git-installation");
    let repo_dir = fixture.cwd().join("repo");
    fs::create_dir_all(&repo_dir).unwrap();
    let repo = git2::Repository::init(&repo_dir).unwrap();
    let sig = git2::Signature::now("Test", "test@test.com").unwrap();
    let tree_id = repo.index().unwrap().write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[]).unwrap();

    let traces = tempfile::tempdir().unwrap();
    let builder = fixture.command_builder();
    // The host's Git for Windows installation is the tool under observation:
    // the hint names it, and without the hint gix would start it.
    let builder = builder.host_path();
    let mut command = builder.ambient_context(&repo_dir).build_std();
    // The fixture skips the system configuration; reading it is what makes
    // gix look for the installation.
    command
        .env_remove("GIT_CONFIG_NOSYSTEM")
        .env_remove("GIT_EXEC_PATH")
        .env("GIT_TRACE2_EVENT", traces.path())
        .args(["repo", "git-status", "--json"]);
    match exepath {
        Some(dir) => command.env("EXEPATH", dir),
        None => command.env_remove("EXEPATH"),
    };
    let output = command.output().expect("sniff should run");
    let started = fs::read_dir(traces.path()).unwrap().count();
    (output, started)
}

#[test]
fn a_repository_run_without_an_installation_hint_starts_no_git_process() {
    let (output, started) = git_status(None);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(started, 0, "sniff started Git to find the installation");
}

/// The control for the test above: a caller's `EXEPATH` that names no
/// installation is kept, gix ignores it, and the same run asks Git. Without
/// this, a run that never reached gix's installation lookup would pass too.
#[test]
fn a_repository_run_with_an_unusable_caller_hint_asks_git() {
    let empty = tempfile::tempdir().unwrap();
    let (output, started) = git_status(Some(empty.path()));
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(started > 0, "gix should have started Git to find the installation");
}
