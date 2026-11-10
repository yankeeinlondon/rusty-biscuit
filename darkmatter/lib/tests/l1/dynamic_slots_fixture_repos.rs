//! The dynamic-slot repository fixture produces each state it names, and its
//! copied schema's slot path reaches the data file. Later approval tests rely
//! on these states differing from the clean control in exactly one way.

use std::path::Path;
use std::process::Command;

use crate::dynamic_slots_fixtures::{DATA_FILE, DIRTY_ENTRY, DOCUMENT_FILE, RepoState, SCHEMA_FILE, SlotRepo};

/// `git status --porcelain --ignored` for the whole repository.
fn porcelain(root: &Path) -> String {
    let output = Command::new("git")
        .args(["status", "--porcelain", "--ignored", "--untracked-files=all"])
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git status");
    assert!(output.status.success(), "git status failed: {output:?}");
    String::from_utf8(output.stdout).expect("utf-8 status")
}

#[test]
fn clean_state_tracks_and_commits_every_file() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::Clean);
    assert_eq!(porcelain(repo.root()), "");
    let schema = std::fs::read_to_string(repo.path(SCHEMA_FILE)).expect("schema copied");
    assert!(schema.contains(r#"load_data("../../../os.yaml", "string[]")"#), "{schema}");
    let slot_target = repo.path(SCHEMA_FILE).parent().unwrap().join("../../../os.yaml");
    assert_eq!(
        biscuit_file::canonicalize_simplified(&slot_target).expect("slot target exists"),
        repo.path(DATA_FILE),
        "the schema-relative slot path names the data file",
    );
    assert!(repo.path(DOCUMENT_FILE).is_file());
    assert_eq!(std::fs::read_to_string(repo.path(DATA_FILE)).unwrap(), "- macOS\n- Linux\n- Windows\n");
}

#[test]
fn dirty_state_modifies_only_the_data_file() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::DirtyData);
    assert_eq!(porcelain(repo.root()), format!(" M {DATA_FILE}\n"));
    let data = std::fs::read_to_string(repo.path(DATA_FILE)).unwrap();
    assert!(data.ends_with(DIRTY_ENTRY), "{data}");
}

#[test]
fn untracked_state_leaves_only_the_data_file_unadded() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::UntrackedData);
    assert_eq!(porcelain(repo.root()), format!("?? {DATA_FILE}\n"));
}

#[test]
fn ignored_state_matches_the_data_file_with_a_committed_gitignore() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::IgnoredData);
    assert_eq!(porcelain(repo.root()), format!("!! {DATA_FILE}\n"));
}

#[test]
fn no_repository_state_copies_the_files_without_git() {
    let source = biscuit_test_harness::manifest_dir!().join("tests/fixtures/dynamic_slots/claudine");
    let repo = SlotRepo::build(&source, RepoState::NoRepository);
    assert!(!repo.root().join(".git").exists());
    assert!(repo.path(DATA_FILE).is_file());
    assert!(repo.path(SCHEMA_FILE).is_file());
}
