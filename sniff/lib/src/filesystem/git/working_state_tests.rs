//! L1 unit tests for [`GitRepo::working_state`].
//!
//! Fixture repositories are created through the `git2` dev-dependency on an
//! explicit `main` branch, with no subprocess and no signing.

use std::path::Path;

use tempfile::TempDir;

use super::types::GitRepo;
use super::working_state::PathContent;

fn init_repo(dir: &Path) -> git2::Repository {
    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head("refs/heads/main");
    git2::Repository::init_opts(dir, &opts).unwrap()
}

/// Write `files`, stage them, and commit on `HEAD`.
fn commit(dir: &Path, files: &[(&str, &str)]) {
    let repo = git2::Repository::open(dir).unwrap();
    let mut index = repo.index().unwrap();
    for (name, content) in files {
        std::fs::write(dir.join(name), content).unwrap();
        index.add_path(Path::new(name)).unwrap();
    }
    index.write().unwrap();
    let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git2::Signature::now("working-state fixture", "fixture@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(Some("HEAD"), &signature, &signature, "fixture", &tree, &parents)
        .unwrap();
}

/// A committed repository with a tracked file and an ignored `target/`.
fn fixture() -> TempDir {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());
    commit(
        dir.path(),
        &[("tracked.txt", "original\n"), (".gitignore", "target/\n")],
    );
    dir
}

fn state_id(dir: &Path) -> String {
    GitRepo::discover(dir)
        .unwrap()
        .expect("fixture repository")
        .working_state()
        .unwrap()
        .id
}

#[test]
fn clean_tree_is_head_alone_and_stable() {
    let dir = fixture();
    let state = GitRepo::discover(dir.path()).unwrap().unwrap().working_state().unwrap();
    assert!(state.changed.is_empty());
    assert_eq!(state.head.as_deref().map(str::len), Some(40));
    assert_eq!(state.id.len(), 16);
    assert_eq!(state_id(dir.path()), state.id, "a repeat observation agrees");
}

#[test]
fn same_content_rewritten_keeps_the_id() {
    let dir = fixture();
    std::fs::write(dir.path().join("tracked.txt"), "edited\n").unwrap();
    let before = state_id(dir.path());
    // Rewriting identical bytes changes timestamps, not content.
    std::fs::write(dir.path().join("tracked.txt"), "edited\n").unwrap();
    assert_eq!(state_id(dir.path()), before);
}

#[test]
fn an_edit_changes_the_id_and_reverting_restores_it() {
    let dir = fixture();
    let clean = state_id(dir.path());
    std::fs::write(dir.path().join("tracked.txt"), "edited\n").unwrap();
    let edited = state_id(dir.path());
    assert_ne!(edited, clean);
    std::fs::write(dir.path().join("tracked.txt"), "edited again\n").unwrap();
    assert_ne!(state_id(dir.path()), edited, "a second edit is another state");
    std::fs::write(dir.path().join("tracked.txt"), "original\n").unwrap();
    assert_eq!(state_id(dir.path()), clean, "restored content is the clean state");
}

#[test]
fn an_untracked_file_changes_the_id() {
    let dir = fixture();
    let clean = state_id(dir.path());
    std::fs::create_dir(dir.path().join("new")).unwrap();
    std::fs::write(dir.path().join("new").join("file.txt"), "new\n").unwrap();
    let added = state_id(dir.path());
    assert_ne!(added, clean);
    std::fs::write(dir.path().join("new").join("file.txt"), "changed\n").unwrap();
    assert_ne!(
        state_id(dir.path()),
        added,
        "content inside an untracked directory counts"
    );
}

#[test]
fn a_deletion_changes_the_id_and_reads_as_absent() {
    let dir = fixture();
    let clean = state_id(dir.path());
    std::fs::remove_file(dir.path().join("tracked.txt")).unwrap();
    let state = GitRepo::discover(dir.path()).unwrap().unwrap().working_state().unwrap();
    assert_ne!(state.id, clean);
    assert_eq!(state.changed.len(), 1);
    assert_eq!(state.changed[0].content, PathContent::Absent);
}

#[test]
fn an_ignored_file_keeps_the_id() {
    let dir = fixture();
    let clean = state_id(dir.path());
    std::fs::create_dir(dir.path().join("target")).unwrap();
    std::fs::write(dir.path().join("target").join("out.bin"), "build output\n").unwrap();
    assert_eq!(state_id(dir.path()), clean);
}

#[test]
fn a_new_commit_changes_the_id() {
    let dir = fixture();
    let clean = state_id(dir.path());
    commit(dir.path(), &[("second.txt", "two\n")]);
    assert_ne!(state_id(dir.path()), clean);
}

#[test]
fn an_unborn_head_is_valid() {
    let dir = TempDir::new().unwrap();
    init_repo(dir.path());
    std::fs::write(dir.path().join("first.txt"), "first\n").unwrap();
    let state = GitRepo::discover(dir.path()).unwrap().unwrap().working_state().unwrap();
    assert_eq!(state.head, None);
    assert_eq!(state.changed.len(), 1);
    assert!(matches!(state.changed[0].content, PathContent::File(_)));
}

#[test]
fn one_status_walk_and_one_read_per_changed_path() {
    use crate::performance::{self, PerformanceCollector, counters};

    let dir = fixture();
    std::fs::write(dir.path().join("tracked.txt"), "edited\n").unwrap();
    std::fs::write(dir.path().join("untracked.txt"), "new\n").unwrap();
    let repo = GitRepo::discover(dir.path()).unwrap().unwrap();

    let collector = PerformanceCollector::new_shared();
    let state =
        performance::with_current_collector(Some(collector.clone()), || repo.working_state())
            .unwrap();
    let counters_seen = collector.snapshot(std::time::Duration::ZERO).counters;
    let count = |name: &str| counters_seen.get(name).copied().unwrap_or(0);

    assert_eq!(state.changed.len(), 2);
    assert_eq!(count(counters::GIT_STATUS_WALKS), 1, "{counters_seen:?}");
    assert_eq!(count(counters::GIT_WORKING_STATE_PATHS_HASHED), 2, "{counters_seen:?}");
    assert_eq!(count(counters::GIT_FILE_DIFFS), 0, "{counters_seen:?}");
    assert_eq!(count(counters::GIT_COMMIT_VISITS), 0, "{counters_seen:?}");
}
