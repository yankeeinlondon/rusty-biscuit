//! Public-API tests for `GitRepo::committed_tree` and `CommittedTreeSnapshot`.
//!
//! Fixtures are written through git2 (one temp repository per test, no
//! subprocess); the parity test compares the snapshot with read-only `git`
//! plumbing (`ls-tree`, `rev-parse`) as the independent oracle. Special
//! entries (symbolic link, executable, submodule) are written straight into
//! the index, so no filesystem symlink is needed and the test runs unchanged
//! on Windows.

use std::path::Path;
use std::process::Command;

use git2::{IndexEntry, IndexTime, Oid, Repository, Signature};
use tempfile::TempDir;

use sniff::SniffError;
use sniff::filesystem::git::{CommittedEntry, CommittedTreeSnapshot, GitRepo};

use crate::builder;

const MODE_FILE: u32 = 0o100_644;
const MODE_EXECUTABLE: u32 = 0o100_755;
const MODE_SYMLINK: u32 = 0o120_000;
const MODE_GITLINK: u32 = 0o160_000;

/// An empty repository whose unborn `HEAD` names `main`.
fn init(root: &Path) -> Repository {
    let repo = builder::init_repo(root);
    repo.set_head("refs/heads/main")
        .expect("point HEAD at main");
    repo
}

fn entry(path: &str, mode: u32, id: Oid) -> IndexEntry {
    IndexEntry {
        ctime: IndexTime::new(0, 0),
        mtime: IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode,
        uid: 0,
        gid: 0,
        file_size: 0,
        id,
        flags: 0,
        flags_extended: 0,
        path: path.as_bytes().to_vec(),
    }
}

/// Stage `bytes` at `path` with `mode` in the index only (the worktree is
/// not touched).
fn stage_blob(repo: &Repository, path: &str, mode: u32, bytes: &[u8]) {
    let mut index = repo.index().expect("index");
    index
        .add_frombuffer(&entry(path, mode, Oid::zero()), bytes)
        .expect("stage blob");
    index.write().expect("write index");
}

/// Write `bytes` to `path` in the worktree and stage it.
fn write_and_stage(repo: &Repository, root: &Path, path: &str, bytes: &str) {
    builder::write_file(&root.join(path), bytes);
    let mut index = repo.index().expect("index");
    index.add_path(Path::new(path)).expect("stage file");
    index.write().expect("write index");
}

/// Commit the index onto `update_ref` (its current tip, if any, is the
/// parent) and return the new commit id.
fn commit_index(repo: &Repository, update_ref: &str, message: &str) -> Oid {
    let tree_id = repo
        .index()
        .expect("index")
        .write_tree()
        .expect("write tree");
    let tree = repo.find_tree(tree_id).expect("tree");
    let signature = Signature::new("Committed Tree", "tree@sniff.test", &git2::Time::new(0, 0))
        .expect("signature");
    let parent = repo
        .find_reference(update_ref)
        .ok()
        .and_then(|reference| reference.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repo.commit(
        Some(update_ref),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
    )
    .expect("commit")
}

fn capture(path: &Path) -> CommittedTreeSnapshot {
    GitRepo::discover(path)
        .expect("discover")
        .expect("fixture repository")
        .committed_tree()
        .expect("capture committed tree")
}

fn blob(bytes: &str) -> CommittedEntry {
    CommittedEntry::Blob {
        bytes: bytes.as_bytes().to_vec(),
        executable: false,
    }
}

fn read(snapshot: &CommittedTreeSnapshot, path: &str) -> CommittedEntry {
    snapshot.read_file(path).expect("read committed path")
}

/// The loose-object file holding `id`.
fn loose_object(repo: &Repository, id: Oid) -> std::path::PathBuf {
    let hex = id.to_string();
    repo.path().join("objects").join(&hex[..2]).join(&hex[2..])
}

/// Delete a loose object. Git writes them read-only, which blocks deletion
/// on Windows, so the bit is cleared first.
#[allow(clippy::permissions_set_readonly_false)]
fn delete_object(repo: &Repository, id: Oid) {
    let path = loose_object(repo, id);
    let mut permissions = std::fs::metadata(&path)
        .expect("object exists")
        .permissions();
    permissions.set_readonly(false);
    std::fs::set_permissions(&path, permissions).expect("make object writable");
    std::fs::remove_file(&path).expect("delete object");
}

fn assert_git_error<T: std::fmt::Debug>(result: sniff::Result<T>) {
    match result {
        Err(SniffError::Git { .. }) => {}
        other => panic!("expected SniffError::Git, got {other:?}"),
    }
}

/// Run a read-only `git` command in `root`, isolated from any ambient
/// repository selection a parent process (such as a hook) may export.
fn git(root: &Path, args: &[&str]) -> Vec<u8> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env_remove("GIT_OBJECT_DIRECTORY")
        .output()
        .expect("`git` is on PATH");
    assert!(
        output.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

#[test]
fn committed_tree_unborn_head_is_an_empty_view() {
    let dir = TempDir::new().unwrap();
    init(dir.path());

    let snapshot = capture(dir.path());

    assert_eq!(snapshot.commit_id(), None);
    assert_eq!(snapshot.tree_id(), None);
    assert_eq!(snapshot.paths().unwrap(), Vec::<String>::new());
    assert_eq!(read(&snapshot, "anything.txt"), CommittedEntry::Absent);
    assert!(
        matches!(
            snapshot.read_file("../escape"),
            Err(SniffError::InvalidRepositoryPath { .. })
        ),
        "path validation does not depend on HEAD being born"
    );
}

#[test]
fn committed_tree_matches_git_ls_tree_and_rev_parse() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let repo = init(root);
    // Names chosen so tree order differs from naive per-level sorting:
    // `a.txt` < `a/` < `a0` in byte order.
    for (path, mode, bytes) in [
        ("a.txt", MODE_FILE, "dot\n"),
        ("a/b.txt", MODE_FILE, "nested\n"),
        ("a/c/d.txt", MODE_FILE, "deeper\n"),
        ("a0", MODE_FILE, "zero\n"),
        ("B.md", MODE_FILE, "upper\n"),
        ("bin/run.sh", MODE_EXECUTABLE, "#!/bin/sh\n"),
        ("link", MODE_SYMLINK, "a/b.txt"),
        ("z/y/x.txt", MODE_FILE, "last\n"),
    ] {
        stage_blob(&repo, path, mode, bytes.as_bytes());
    }
    let pinned_submodule = Oid::from_str("0123456789abcdef0123456789abcdef01234567").unwrap();
    let mut index = repo.index().unwrap();
    index
        .add(&entry("vendor/sub", MODE_GITLINK, pinned_submodule))
        .unwrap();
    index.write().unwrap();
    commit_index(&repo, "HEAD", "parity fixture");

    let snapshot = capture(root);

    let ids = String::from_utf8(git(root, &["rev-parse", "HEAD", "HEAD^{tree}"])).unwrap();
    let ids: Vec<&str> = ids.lines().collect();
    assert_eq!(snapshot.commit_id().as_deref(), Some(ids[0]));
    assert_eq!(snapshot.tree_id().as_deref(), Some(ids[1]));

    // `<mode> SP <type> SP <oid> TAB <path> NUL`, in tree order.
    let listing = git(root, &["ls-tree", "-r", "-z", "--full-tree", "HEAD"]);
    let mut expected_paths = Vec::new();
    for record in listing
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty())
    {
        let record = std::str::from_utf8(record).unwrap();
        let (meta, path) = record.split_once('\t').unwrap();
        let fields: Vec<&str> = meta.split(' ').collect();
        let (mode, oid) = (fields[0], fields[2]);
        let entry = read(&snapshot, path);
        let stored = match (mode, &entry) {
            (
                "100644",
                CommittedEntry::Blob {
                    bytes,
                    executable: false,
                },
            )
            | (
                "100755",
                CommittedEntry::Blob {
                    bytes,
                    executable: true,
                },
            )
            | ("120000", CommittedEntry::Symlink { target: bytes }) => bytes.clone(),
            ("160000", CommittedEntry::Submodule { commit_id }) => {
                assert_eq!(commit_id, oid, "{path}");
                continue;
            }
            _ => panic!("{path}: git reports mode {mode}, snapshot reports {entry:?}"),
        };
        // Equal blob ids mean byte-for-byte what `git cat-file -p` prints.
        assert_eq!(
            Oid::hash_object(git2::ObjectType::Blob, &stored)
                .unwrap()
                .to_string(),
            oid,
            "{path}"
        );
        expected_paths.push(path.to_owned());
    }
    assert_eq!(snapshot.paths().unwrap(), expected_paths);
    assert_eq!(expected_paths.len(), 8, "every blob and symlink was listed");

    assert_eq!(read(&snapshot, "a"), CommittedEntry::Directory);
    assert_eq!(read(&snapshot, "a/c"), CommittedEntry::Directory);
    assert_eq!(
        read(&snapshot, "link"),
        CommittedEntry::Symlink {
            target: b"a/b.txt".to_vec()
        }
    );
    assert_eq!(
        read(&snapshot, "a.txt/below-a-file"),
        CommittedEntry::Absent
    );
    assert_eq!(
        read(&snapshot, "link/below-a-symlink"),
        CommittedEntry::Absent
    );
    assert_eq!(read(&snapshot, "vendor/sub/inside"), CommittedEntry::Absent);
    assert_eq!(read(&snapshot, "a/missing.txt"), CommittedEntry::Absent);
}

#[test]
fn committed_tree_reads_committed_bytes_not_working_copy_or_index() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let repo = init(root);
    for path in ["staged.txt", "unstaged.txt", "deleted.txt"] {
        write_and_stage(&repo, root, path, "committed\n");
    }
    commit_index(&repo, "HEAD", "base");

    write_and_stage(&repo, root, "staged.txt", "staged edit\n");
    builder::write_file(&root.join("unstaged.txt"), "unstaged edit\n");
    std::fs::remove_file(root.join("deleted.txt")).unwrap();
    write_and_stage(&repo, root, "added.txt", "new\n");

    let snapshot = capture(root);

    assert_eq!(read(&snapshot, "staged.txt"), blob("committed\n"));
    assert_eq!(read(&snapshot, "unstaged.txt"), blob("committed\n"));
    assert_eq!(read(&snapshot, "deleted.txt"), blob("committed\n"));
    assert_eq!(read(&snapshot, "added.txt"), CommittedEntry::Absent);
    assert_eq!(
        snapshot.paths().unwrap(),
        ["deleted.txt", "staged.txt", "unstaged.txt"]
    );
}

#[test]
fn committed_tree_keeps_reading_the_original_tree_after_refs_move() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let repo = init(root);
    write_and_stage(&repo, root, "policy.yaml", "v1\n");
    let original = commit_index(&repo, "HEAD", "v1");

    let snapshot = capture(root);

    // A new commit on the current branch.
    write_and_stage(&repo, root, "policy.yaml", "v2\n");
    write_and_stage(&repo, root, "later.txt", "later\n");
    commit_index(&repo, "HEAD", "v2");
    // Then a switch to another branch with different content.
    stage_blob(&repo, "policy.yaml", MODE_FILE, b"other\n");
    let other = commit_index(&repo, "refs/heads/other", "other");
    repo.set_head("refs/heads/other").unwrap();

    assert_eq!(snapshot.commit_id(), Some(original.to_string()));
    assert_eq!(read(&snapshot, "policy.yaml"), blob("v1\n"));
    assert_eq!(read(&snapshot, "later.txt"), CommittedEntry::Absent);
    assert_eq!(snapshot.paths().unwrap(), ["policy.yaml"]);

    let fresh = capture(root);
    assert_eq!(
        fresh.commit_id(),
        Some(other.to_string()),
        "control: HEAD did move"
    );
    assert_eq!(read(&fresh, "policy.yaml"), blob("other\n"));
}

#[test]
fn committed_tree_in_a_linked_worktree_pins_that_worktrees_head() {
    let dir = TempDir::new().unwrap();
    let root = dir.path().join("main");
    std::fs::create_dir(&root).unwrap();
    let repo = init(&root);
    write_and_stage(&repo, &root, "policy.yaml", "main\n");
    let main_tip = commit_index(&repo, "HEAD", "main");
    stage_blob(&repo, "policy.yaml", MODE_FILE, b"feature\n");
    let feature_tip = commit_index(&repo, "refs/heads/feature", "feature");

    let linked = dir.path().join("linked");
    let feature = repo.find_reference("refs/heads/feature").unwrap();
    let mut options = git2::WorktreeAddOptions::new();
    options.reference(Some(&feature));
    repo.worktree("linked", &linked, Some(&options)).unwrap();

    let linked_snapshot = capture(&linked);
    let main_snapshot = capture(&root);

    assert_eq!(linked_snapshot.commit_id(), Some(feature_tip.to_string()));
    assert_eq!(read(&linked_snapshot, "policy.yaml"), blob("feature\n"));
    assert_eq!(main_snapshot.commit_id(), Some(main_tip.to_string()));
    assert_eq!(read(&main_snapshot, "policy.yaml"), blob("main\n"));
}

#[test]
fn committed_tree_missing_objects_are_errors_not_absence() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let repo = init(root);
    stage_blob(&repo, "kept.txt", MODE_FILE, b"kept\n");
    stage_blob(&repo, "lost.txt", MODE_FILE, b"lost\n");
    stage_blob(&repo, "dir/inner.txt", MODE_FILE, b"inner\n");
    commit_index(&repo, "HEAD", "fixture");
    let head_tree = repo.head().unwrap().peel_to_tree().unwrap();
    let lost_blob = head_tree.get_path(Path::new("lost.txt")).unwrap().id();
    let subtree = head_tree.get_path(Path::new("dir")).unwrap().id();
    let root_tree = head_tree.id();

    let snapshot = capture(root);
    delete_object(&repo, lost_blob);

    assert_git_error(snapshot.read_file("lost.txt"));
    assert_eq!(
        read(&snapshot, "kept.txt"),
        blob("kept\n"),
        "control: other blobs read"
    );
    assert_eq!(read(&snapshot, "nope.txt"), CommittedEntry::Absent);
    assert_eq!(
        snapshot.paths().unwrap(),
        ["dir/inner.txt", "kept.txt", "lost.txt"]
    );

    delete_object(&repo, subtree);
    assert_git_error(snapshot.read_file("dir/inner.txt"));
    assert_git_error(snapshot.paths());

    delete_object(&repo, root_tree);
    assert_git_error(GitRepo::discover(root).unwrap().unwrap().committed_tree());
}

#[test]
fn committed_tree_rejects_paths_that_are_not_repository_relative() {
    let dir = TempDir::new().unwrap();
    let repo = init(dir.path());
    stage_blob(&repo, "a/b.txt", MODE_FILE, b"b\n");
    commit_index(&repo, "HEAD", "fixture");
    let snapshot = capture(dir.path());

    for path in [
        "",
        "/a/b.txt",
        "a/../a/b.txt",
        "..",
        "./a/b.txt",
        "a//b.txt",
        "a/",
        "a\\b.txt",
        "C:/a/b.txt",
        "c:",
        "a/b\0.txt",
    ] {
        match snapshot.read_file(path) {
            Err(SniffError::InvalidRepositoryPath { path: reported, .. }) => {
                assert_eq!(reported, path);
            }
            other => panic!("{path:?} should be rejected, got {other:?}"),
        }
    }
    assert_eq!(read(&snapshot, "a/b.txt"), blob("b\n"));
}

#[test]
fn committed_tree_snapshot_reads_from_other_threads() {
    let dir = TempDir::new().unwrap();
    let repo = init(dir.path());
    stage_blob(&repo, "shared.txt", MODE_FILE, b"shared\n");
    commit_index(&repo, "HEAD", "fixture");
    let snapshot = std::sync::Arc::new(capture(dir.path()));

    let readers: Vec<_> = (0..4)
        .map(|_| {
            let snapshot = std::sync::Arc::clone(&snapshot);
            std::thread::spawn(move || snapshot.read_file("shared.txt").unwrap())
        })
        .collect();
    for reader in readers {
        assert_eq!(reader.join().unwrap(), blob("shared\n"));
    }
}
