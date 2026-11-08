//! L1 unit tests for [`GitRepo::tracked_state`] and
//! [`GitRepo::worktree_blob_id`].
//!
//! Fixture repositories are created through the `git2` dev-dependency on an
//! explicit `main` branch, with no subprocess and no signing. Every path
//! handed to the queries is canonical, as the caller contract requires.

use std::path::{Path, PathBuf};

use tempfile::TempDir;

use super::tracked_state::TrackedState;
use super::types::GitRepo;
use crate::SniffError;

const COMMITTED: &str = "committed\n";

fn init_repo(dir: &Path) -> git2::Repository {
    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head("refs/heads/main");
    git2::Repository::init_opts(dir, &opts).unwrap()
}

/// Write `content` to `relative` and stage it.
fn stage(dir: &Path, relative: &str, content: &[u8]) {
    let full = dir.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, content).unwrap();
    let repo = git2::Repository::open(dir).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

/// Commit the on-disk index on `HEAD`.
fn commit(dir: &Path) {
    let repo = git2::Repository::open(dir).unwrap();
    let tree = repo
        .find_tree(repo.index().unwrap().write_tree().unwrap())
        .unwrap();
    let signature = git2::Signature::now("tracked-state fixture", "fixture@example.com").unwrap();
    let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        "fixture",
        &tree,
        &parents,
    )
    .unwrap();
}

/// A committed repository with a tracked file, ignore rules, and an
/// LFS-filtered file. Returns the directory guard and its canonical root.
fn fixture() -> (TempDir, PathBuf) {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    init_repo(&root);
    stage(&root, "tracked.txt", COMMITTED.as_bytes());
    stage(&root, "nested/deep.txt", b"deep\n");
    stage(&root, ".gitignore", b"*.log\nignored-dir/\n");
    stage(&root, ".gitattributes", b"data.bin filter=lfs\n");
    stage(&root, "data.bin", b"binary payload\n");
    commit(&root);
    (dir, root)
}

fn discover(root: &Path) -> GitRepo {
    GitRepo::discover(root)
        .unwrap()
        .expect("fixture repository")
}

fn state_of(root: &Path, relative: &str) -> TrackedState {
    let states = discover(root)
        .tracked_state(&[root.join(relative)])
        .unwrap();
    assert_eq!(states.len(), 1);
    states.into_iter().next().unwrap()
}

fn head_blob(root: &Path, relative: &str) -> String {
    let repo = git2::Repository::open(root).unwrap();
    let tree = repo.head().unwrap().peel_to_tree().unwrap();
    tree.get_path(Path::new(relative)).unwrap().id().to_string()
}

fn index_blob(root: &Path, relative: &str) -> String {
    let repo = git2::Repository::open(root).unwrap();
    let index = repo.index().unwrap();
    index
        .get_path(Path::new(relative), 0)
        .unwrap()
        .id
        .to_string()
}

#[test]
fn tracked_state_clean_file_reports_its_real_blob_ids() {
    let (_dir, root) = fixture();
    let expected = head_blob(&root, "tracked.txt");
    assert_eq!(index_blob(&root, "tracked.txt"), expected);

    assert_eq!(
        state_of(&root, "tracked.txt"),
        TrackedState::Clean {
            index_blob: expected.clone(),
            head_blob: expected.clone(),
        }
    );
    let bytes = std::fs::read(root.join("tracked.txt")).unwrap();
    let repo = discover(&root);
    assert_eq!(
        repo.worktree_blob_id(&root.join("tracked.txt"), &bytes)
            .unwrap(),
        expected
    );
    assert!(
        matches!(
            state_of(&root, "nested/deep.txt"),
            TrackedState::Clean { .. }
        ),
        "a file below a subdirectory is clean too"
    );
}

#[test]
fn tracked_state_staged_modification_is_dirty() {
    let (_dir, root) = fixture();
    stage(&root, "tracked.txt", b"staged edit\n");
    assert_eq!(state_of(&root, "tracked.txt"), TrackedState::Dirty);
}

#[test]
fn tracked_state_worktree_modification_is_dirty() {
    let (_dir, root) = fixture();
    std::fs::write(root.join("tracked.txt"), "a longer worktree edit\n").unwrap();
    assert_eq!(state_of(&root, "tracked.txt"), TrackedState::Dirty);
}

#[test]
fn tracked_state_same_size_worktree_edit_is_dirty() {
    let (_dir, root) = fixture();
    let same_size = "COMMITTED\n";
    assert_eq!(same_size.len(), COMMITTED.len());
    std::fs::write(root.join("tracked.txt"), same_size).unwrap();
    assert_eq!(
        state_of(&root, "tracked.txt"),
        TrackedState::Dirty,
        "content is hashed when the recorded size matches"
    );
}

#[test]
fn tracked_state_added_but_not_committed_is_dirty() {
    let (_dir, root) = fixture();
    stage(&root, "added.txt", b"added\n");
    assert_eq!(state_of(&root, "added.txt"), TrackedState::Dirty);
}

#[test]
fn tracked_state_deleted_from_worktree_is_dirty() {
    let (_dir, root) = fixture();
    std::fs::remove_file(root.join("tracked.txt")).unwrap();
    assert_eq!(state_of(&root, "tracked.txt"), TrackedState::Dirty);
}

#[test]
fn tracked_state_untracked_existing_and_missing_paths_are_untracked() {
    let (_dir, root) = fixture();
    std::fs::write(root.join("new.txt"), "new\n").unwrap();
    assert_eq!(state_of(&root, "new.txt"), TrackedState::Untracked);
    assert_eq!(
        state_of(&root, "does/not/exist.txt"),
        TrackedState::Untracked,
        "a nonexistent path outside the index is untracked"
    );
}

#[test]
fn tracked_state_ignored_paths_are_ignored() {
    let (_dir, root) = fixture();
    std::fs::write(root.join("build.log"), "log\n").unwrap();
    std::fs::create_dir(root.join("ignored-dir")).unwrap();
    std::fs::write(root.join("ignored-dir").join("out.txt"), "out\n").unwrap();

    assert_eq!(state_of(&root, "build.log"), TrackedState::Ignored);
    assert_eq!(
        state_of(&root, "ignored-dir/out.txt"),
        TrackedState::Ignored,
        "a file inside an ignored directory is ignored"
    );
    assert_eq!(state_of(&root, "ignored-dir"), TrackedState::Ignored);
}

/// Write unmerged index entries at stages 1-3 for `path`.
fn conflict_at(root: &Path, path: &str) {
    let repo = git2::Repository::open(root).unwrap();
    let oid = repo.blob(b"conflict\n").unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new(path)).unwrap();
    for stage in 1..=3u16 {
        index
            .add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: 0,
                id: oid,
                flags: stage << 12,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            })
            .unwrap();
    }
    index.write().unwrap();
}

#[test]
fn tracked_state_unmerged_entries_are_conflicted() {
    let (_dir, root) = fixture();
    conflict_at(&root, "tracked.txt");
    assert_eq!(state_of(&root, "tracked.txt"), TrackedState::Conflicted);
}

#[test]
fn tracked_state_filter_attribute_is_filtered() {
    let (_dir, root) = fixture();
    assert_eq!(state_of(&root, "data.bin"), TrackedState::Filtered);

    let err = discover(&root)
        .worktree_blob_id(&root.join("data.bin"), b"binary payload\n")
        .unwrap_err();
    assert!(
        matches!(
            err,
            SniffError::Git {
                operation: "worktree_blob_id",
                ..
            }
        ),
        "a filtered path has no driver-free blob id: {err:?}"
    );
}

#[test]
fn tracked_state_paths_beyond_the_working_tree_are_outside() {
    let (_dir, root) = fixture();
    let elsewhere = TempDir::new().unwrap();
    let elsewhere = biscuit_file::canonicalize_simplified(elsewhere.path()).unwrap();
    std::fs::write(elsewhere.join("foreign.txt"), "foreign\n").unwrap();

    let inner = root.join("inner");
    std::fs::create_dir(&inner).unwrap();
    init_repo(&inner);
    std::fs::write(inner.join("inner.txt"), "inner\n").unwrap();

    let states = discover(&root)
        .tracked_state(&[
            elsewhere.join("foreign.txt"),
            root.join(".git").join("config"),
            inner.join("inner.txt"),
        ])
        .unwrap();
    assert_eq!(states, vec![TrackedState::Outside; 3]);

    let err = discover(&root)
        .worktree_blob_id(&elsewhere.join("foreign.txt"), b"foreign\n")
        .unwrap_err();
    assert!(
        matches!(
            err,
            SniffError::Git {
                operation: "worktree_blob_id",
                ..
            }
        ),
        "{err:?}"
    );
}

#[test]
fn tracked_state_batched_call_preserves_input_order() {
    let (_dir, root) = fixture();
    std::fs::write(root.join("new.txt"), "new\n").unwrap();
    std::fs::write(root.join("build.log"), "log\n").unwrap();
    std::fs::write(root.join("nested/deep.txt"), "edited\n").unwrap();

    let states = discover(&root)
        .tracked_state(&[
            root.join("build.log"),
            root.join("data.bin"),
            root.join("tracked.txt"),
            root.join("new.txt"),
            root.join("nested/deep.txt"),
            root.join("build.log"),
        ])
        .unwrap();
    let blob = head_blob(&root, "tracked.txt");
    assert_eq!(
        states,
        vec![
            TrackedState::Ignored,
            TrackedState::Filtered,
            TrackedState::Clean {
                index_blob: blob.clone(),
                head_blob: blob,
            },
            TrackedState::Untracked,
            TrackedState::Dirty,
            TrackedState::Ignored,
        ]
    );
    assert!(discover(&root).tracked_state(&[]).unwrap().is_empty());
}

#[test]
fn tracked_state_reads_one_file_and_walks_no_status() {
    use crate::performance::{self, PerformanceCollector, counters};

    let (_dir, root) = fixture();
    std::fs::write(root.join("new.txt"), "new\n").unwrap();
    let repo = discover(&root);
    let paths = [
        root.join("tracked.txt"),
        root.join("new.txt"),
        root.join("data.bin"),
    ];

    let collector = PerformanceCollector::new_shared();
    performance::with_current_collector(Some(collector.clone()), || repo.tracked_state(&paths))
        .unwrap();
    let seen = collector.snapshot(std::time::Duration::ZERO).counters;
    let count = |name: &str| seen.get(name).copied().unwrap_or(0);

    assert_eq!(count(counters::GIT_STATUS_WALKS), 0, "{seen:?}");
    assert_eq!(count(counters::FS_WALK_STARTS), 0, "{seen:?}");
    assert_eq!(
        count(counters::FS_FILE_OPENS),
        1,
        "only the tracked candidate's bytes are read: {seen:?}"
    );
    assert_eq!(
        count(counters::FS_BYTES_READ),
        COMMITTED.len() as u64,
        "{seen:?}"
    );
}

#[test]
fn worktree_blob_id_applies_autocrlf_to_a_crlf_checkout() {
    let dir = TempDir::new().unwrap();
    let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
    init_repo(&root);
    let config_path = root.join(".git").join("config");
    let mut config = std::fs::read_to_string(&config_path).unwrap();
    config.push_str("[core]\n\tautocrlf = true\n");
    std::fs::write(&config_path, config).unwrap();

    let crlf = b"first line\r\nsecond line\r\n";
    stage(&root, "windows.txt", crlf);
    commit(&root);
    let lf_blob = git2::Oid::hash_object(git2::ObjectType::Blob, b"first line\nsecond line\n")
        .unwrap()
        .to_string();
    assert_eq!(
        head_blob(&root, "windows.txt"),
        lf_blob,
        "committed blob is LF"
    );

    let repo = discover(&root);
    assert_eq!(
        repo.worktree_blob_id(&root.join("windows.txt"), crlf)
            .unwrap(),
        lf_blob
    );
    assert_eq!(
        state_of(&root, "windows.txt"),
        TrackedState::Clean {
            index_blob: lf_blob.clone(),
            head_blob: lf_blob,
        }
    );
}

#[test]
fn tracked_state_and_worktree_blob_id_reject_a_bare_repository() {
    let dir = TempDir::new().unwrap();
    let mut opts = git2::RepositoryInitOptions::new();
    opts.bare(true);
    git2::Repository::init_opts(dir.path(), &opts).unwrap();
    let repo = discover(dir.path());

    let err = repo
        .tracked_state(&[dir.path().join("file.txt")])
        .unwrap_err();
    assert!(matches!(err, SniffError::BareRepository { .. }), "{err:?}");
    let err = repo
        .worktree_blob_id(&dir.path().join("file.txt"), b"x")
        .unwrap_err();
    assert!(matches!(err, SniffError::BareRepository { .. }), "{err:?}");
}

#[cfg(unix)]
#[test]
fn tracked_state_executable_bit_change_is_dirty() {
    use std::os::unix::fs::PermissionsExt as _;

    let (_dir, root) = fixture();
    let path = root.join("tracked.txt");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(state_of(&root, "tracked.txt"), TrackedState::Dirty);
}

#[cfg(unix)]
#[test]
fn tracked_state_tracked_symlink_is_dirty_and_never_followed() {
    let (_dir, root) = fixture();
    std::os::unix::fs::symlink("tracked.txt", root.join("link.txt")).unwrap();
    stage_existing(&root, "link.txt");
    commit(&root);
    assert_eq!(state_of(&root, "link.txt"), TrackedState::Dirty);
}

#[cfg(unix)]
fn stage_existing(dir: &Path, relative: &str) {
    let repo = git2::Repository::open(dir).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}
