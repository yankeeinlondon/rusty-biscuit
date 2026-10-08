//! L1 unit tests for [`GitRepo::changed_paths`].
//!
//! Fixture identity: every repository is initialized on an explicit `main`
//! branch so no result depends on the host's `init.defaultBranch`, commits
//! are created through the `git2` dev-dependency (no signing, no subprocess),
//! and every fixture file name states what it is for. Paths are asserted as
//! exact sorted sets through the public method.

use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use tempfile::TempDir;

use super::types::{ChangedBase, GitRepo};
use crate::SniffError;

/// The branch every fixture repository is initialized on.
const MAIN_BRANCH: &str = "refs/heads/main";

fn signature() -> git2::Signature<'static> {
    git2::Signature::now("changed-paths fixture", "fixture@example.com").unwrap()
}

/// Initialize a repository on the explicit [`MAIN_BRANCH`] branch.
fn init_repo(dir: &Path) -> git2::Repository {
    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head(MAIN_BRANCH);
    git2::Repository::init_opts(dir, &opts).unwrap()
}

/// Initialize a bare repository.
fn init_bare_repo(dir: &Path) -> git2::Repository {
    let mut opts = git2::RepositoryInitOptions::new();
    opts.bare(true);
    opts.initial_head(MAIN_BRANCH);
    git2::Repository::init_opts(dir, &opts).unwrap()
}

fn discover(path: &Path) -> GitRepo {
    GitRepo::discover(path)
        .unwrap()
        .expect("fixture repository should be discoverable")
}

/// Stage `path` (`git add <path>`).
fn stage(repo_path: &Path, relative: &str) {
    let repo = git2::Repository::open(repo_path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

/// Stage a path spelled with raw index bytes, for names `add_path` cannot
/// represent on every platform. The content becomes a blob directly, so the
/// fixture also works when no worktree file with that name can exist.
#[cfg(unix)]
fn stage_bytes(repo_path: &Path, relative: &[u8]) {
    stage_bytes_with_content(repo_path, relative, b"raw bytes\n");
}

#[cfg(unix)]
fn stage_bytes_with_content(repo_path: &Path, relative: &[u8], content: &[u8]) {
    let repo = git2::Repository::open(repo_path).unwrap();
    let blob = repo.blob(content).unwrap();
    let mut index = repo.index().unwrap();
    index
        .add_frombuffer(
            &git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: content.len() as u32,
                id: blob,
                flags: 0,
                flags_extended: 0,
                path: relative.to_vec(),
            },
            content,
        )
        .unwrap();
    index.write().unwrap();
}

/// Stage a deletion (`git rm --cached <path>` plus nothing on disk changes).
fn stage_deletion(repo_path: &Path, relative: &str) {
    let repo = git2::Repository::open(repo_path).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

/// Write files and commit them on `reference` (`HEAD` keeps the current
/// branch; any other reference names or moves a branch and points HEAD at it
/// through the caller).
///
/// Every staging and committing helper opens a fresh handle: libgit2 caches
/// one index per repository handle, so a handle opened before another handle
/// staged a path can commit a tree that lacks it.
fn commit_files(
    repo_path: &Path,
    message: &str,
    reference: &str,
    files: &[(&str, &str)],
) -> git2::Oid {
    for (path, content) in files {
        let full = repo_path.join(path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(full, content).unwrap();
        stage(repo_path, path);
    }
    commit_index(repo_path, message, reference)
}

/// Commit the current on-disk index on `reference` with HEAD's commit as
/// parent.
fn commit_index(repo_path: &Path, message: &str, reference: &str) -> git2::Oid {
    let repo = git2::Repository::open(repo_path).unwrap();
    let sig = signature();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some(reference), &sig, &sig, message, &tree, &parents)
        .unwrap()
}

/// Commit on a named branch, stacking on the current HEAD tip, and check
/// that branch out.
fn commit_on_branch(
    repo_path: &Path,
    branch: &str,
    message: &str,
    files: &[(&str, &str)],
) -> git2::Oid {
    let reference = format!("refs/heads/{branch}");
    let repo = git2::Repository::open(repo_path).unwrap();
    let head_target = repo.head().unwrap().target().unwrap();
    drop(repo);
    let repo = git2::Repository::open(repo_path).unwrap();
    repo.reference(&reference, head_target, true, "branch fixture")
        .unwrap();
    repo.set_head(&reference).unwrap();
    drop(repo);
    commit_files(repo_path, message, &reference, files)
}

/// Lossy path strings of a changed-paths result, for readable assertions.
fn lossy(paths: &[PathBuf]) -> Vec<String> {
    paths.iter().map(|p| p.to_string_lossy().into_owned()).collect()
}

/// Reset the on-disk index to HEAD's tree.
///
/// Fixtures stage into a shared index, so a branch switch must drop what the
/// previous branch staged before building the next commit.
fn reset_index_to_head(repo_path: &Path) {
    let repo = git2::Repository::open(repo_path).unwrap();
    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = repo.find_tree(commit.tree_id()).unwrap();
    let mut index = repo.index().unwrap();
    index.read_tree(&tree).unwrap();
    index.write().unwrap();
}

/// One-commit repository with the given files, HEAD on `main`.
fn repo_with_commit(files: &[(&str, &str)]) -> (TempDir, git2::Repository) {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path());
    commit_files(dir.path(), "fixture commit", MAIN_BRANCH, files);
    (dir, repo)
}

fn changed(dir: &Path, base: ChangedBase) -> crate::Result<Vec<PathBuf>> {
    discover(dir).changed_paths(base)
}

fn omitted() -> ChangedBase {
    ChangedBase::Omitted
}

fn reference(token: &str) -> ChangedBase {
    ChangedBase::Reference(token.to_string())
}

// ─────────────────────────────────────────────────────────────────────────
// Working-tree status classification
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn staged_unstaged_both_and_untracked_paths_union_once() {
    let (dir, _repo) = repo_with_commit(&[
        ("staged.txt", "s1\n"),
        ("unstaged.txt", "u1\n"),
        ("both.txt", "b1\n"),
    ]);

    std::fs::write(dir.path().join("staged.txt"), "s2\n").unwrap();
    stage(dir.path(), "staged.txt");

    std::fs::write(dir.path().join("unstaged.txt"), "u2\n").unwrap();

    std::fs::write(dir.path().join("both.txt"), "b2-staged\n").unwrap();
    stage(dir.path(), "both.txt");
    std::fs::write(dir.path().join("both.txt"), "b2-worktree\n").unwrap();

    std::fs::write(dir.path().join("untracked.txt"), "new\n").unwrap();

    let paths = changed(dir.path(), omitted()).unwrap();
    assert_eq!(
        lossy(&paths),
        vec![
            "both.txt",
            "staged.txt",
            "unstaged.txt",
            "untracked.txt",
        ],
        "one path per file regardless of how many categories hold it, sorted by path bytes"
    );
}

#[test]
fn clean_repository_yields_empty_selection() {
    let (dir, _repo) = repo_with_commit(&[("clean.txt", "c\n")]);
    let paths = changed(dir.path(), omitted()).unwrap();
    assert!(paths.is_empty(), "clean worktree selects nothing: {paths:?}");

    let paths = changed(dir.path(), reference("main")).unwrap();
    assert!(
        paths.is_empty(),
        "base equal to HEAD contributes nothing: {paths:?}"
    );
}

#[test]
fn ignored_untracked_files_are_excluded() {
    let (dir, _repo) = repo_with_commit(&[("tracked.txt", "t\n")]);
    std::fs::write(dir.path().join(".gitignore"), "ignored.txt\n").unwrap();
    stage(dir.path(), ".gitignore");
    commit_index(dir.path(), "gitignore", MAIN_BRANCH);

    std::fs::write(dir.path().join("ignored.txt"), "never seen\n").unwrap();
    std::fs::write(dir.path().join("fresh.txt"), "seen\n").unwrap();

    let paths = changed(dir.path(), omitted()).unwrap();
    assert_eq!(
        lossy(&paths),
        vec!["fresh.txt"],
        "ignored untracked files never enter the changed set"
    );
}

#[test]
fn staged_edit_undone_on_disk_still_selects_the_path() {
    let (dir, _repo) = repo_with_commit(&[("restored.txt", "original\n")]);
    std::fs::write(dir.path().join("restored.txt"), "edited\n").unwrap();
    stage(dir.path(), "restored.txt");
    // Undo the edit on disk: the index still holds the staged edit.
    std::fs::write(dir.path().join("restored.txt"), "original\n").unwrap();

    let paths = changed(dir.path(), omitted()).unwrap();
    assert_eq!(lossy(&paths), vec!["restored.txt"]);
}

#[test]
fn committed_change_restored_to_base_content_still_selects_with_base() {
    let (dir, repo) = repo_with_commit(&[("file.txt", "one\n")]);
    let base_sha = repo.head().unwrap().target().unwrap().to_string();
    commit_files(dir.path(), "edit", MAIN_BRANCH, &[("file.txt", "two\n")]);

    // Restore the base content both on disk and in the index: the working
    // tree exactly matches the base, and only HEAD moved.
    std::fs::write(dir.path().join("file.txt"), "one\n").unwrap();
    stage(dir.path(), "file.txt");

    let paths = changed(dir.path(), reference(&base_sha)).unwrap();
    assert_eq!(
        lossy(&paths),
        vec!["file.txt"],
        "a committed change undone locally is still selected against its base"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Rename, deletion, copy, recreate
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn staged_rename_surfaces_as_delete_and_add() {
    let (dir, _repo) = repo_with_commit(&[("old-name.txt", "content\n")]);
    std::fs::rename(
        dir.path().join("old-name.txt"),
        dir.path().join("new-name.txt"),
    )
    .unwrap();
    let repo = git2::Repository::open(dir.path()).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("old-name.txt")).unwrap();
    index.add_path(Path::new("new-name.txt")).unwrap();
    index.write().unwrap();

    let paths = changed(dir.path(), omitted()).unwrap();
    assert_eq!(lossy(&paths), vec!["new-name.txt", "old-name.txt"]);
}

#[test]
fn staged_deletion_selects_the_deleted_path() {
    let (dir, _repo) = repo_with_commit(&[("gone.txt", "content\n")]);
    std::fs::remove_file(dir.path().join("gone.txt")).unwrap();
    stage_deletion(dir.path(), "gone.txt");

    let paths = changed(dir.path(), omitted()).unwrap();
    assert_eq!(
        lossy(&paths),
        vec!["gone.txt"],
        "whether the path still exists on disk is the caller's concern"
    );
}

#[test]
fn staged_deletion_then_recreate_selects_the_current_file() {
    let (dir, _repo) = repo_with_commit(&[("recreated.txt", "old\n")]);
    std::fs::remove_file(dir.path().join("recreated.txt")).unwrap();
    stage_deletion(dir.path(), "recreated.txt");
    std::fs::write(dir.path().join("recreated.txt"), "new content\n").unwrap();

    let paths = changed(dir.path(), omitted()).unwrap();
    assert_eq!(lossy(&paths), vec!["recreated.txt"]);
}

#[test]
fn copy_keeps_both_paths() {
    let (dir, _repo) = repo_with_commit(&[("source.txt", "shared\n")]);
    let content = std::fs::read(dir.path().join("source.txt")).unwrap();
    std::fs::write(dir.path().join("copy.txt"), content).unwrap();
    stage(dir.path(), "copy.txt");
    commit_index(dir.path(), "copy", MAIN_BRANCH);

    let paths = changed(dir.path(), omitted()).unwrap();
    assert!(paths.is_empty(), "a committed copy leaves a clean worktree: {paths:?}");

    // Against the parent commit, the identical-content copy is a plain
    // addition: no similarity detection pairs it with its source.
    let repo = git2::Repository::open(dir.path()).unwrap();
    let parent = repo.head().unwrap().peel_to_commit().unwrap().parent(0).unwrap();
    let paths = changed(dir.path(), reference(&parent.id().to_string())).unwrap();
    assert_eq!(
        lossy(&paths),
        vec!["copy.txt"],
        "no rename/copy similarity: only the actually-added path differs"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Committed comparison
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn divergent_base_compares_tips_not_merge_base() {
    let (dir, _repo) = repo_with_commit(&[("shared.txt", "base\n")]);

    // Diverge: feature edits feature-only.txt, main edits main-only.txt.
    commit_on_branch(dir.path(), "feature", "feature work", &[("feature-only.txt", "f\n")]);
    let repo = git2::Repository::open(dir.path()).unwrap();
    repo.set_head(MAIN_BRANCH).unwrap();
    drop(repo);
    reset_index_to_head(dir.path());
    commit_files(dir.path(), "main work", MAIN_BRANCH, &[("main-only.txt", "m\n")]);
    let repo = git2::Repository::open(dir.path()).unwrap();
    repo.set_head("refs/heads/feature").unwrap();

    let paths = changed(dir.path(), reference("main")).unwrap();
    assert_eq!(
        lossy(&paths),
        vec!["feature-only.txt", "main-only.txt"],
        "the main-side change must be selected too; a merge-base comparison would yield only feature-only.txt"
    );
}

#[test]
fn base_equal_to_head_contributes_no_committed_paths() {
    let (dir, repo) = repo_with_commit(&[("file.txt", "one\n")]);
    // A branch pointing at HEAD is the same tip.
    let head = repo.head().unwrap().target().unwrap();
    repo.reference("refs/heads/same-tip", head, true, "same tip")
        .unwrap();
    std::fs::write(dir.path().join("file.txt"), "dirty\n").unwrap();

    for token in ["HEAD", "main", "same-tip"] {
        let paths = changed(dir.path(), reference(token)).unwrap();
        assert_eq!(
            lossy(&paths),
            vec!["file.txt"],
            "base {token} equals HEAD, so only the working-tree change appears"
        );
    }
}

#[test]
fn commit_sha_abbreviated_sha_and_annotated_tag_bases_resolve() {
    let (dir, repo) = repo_with_commit(&[("v1.txt", "one\n")]);
    let first = repo.head().unwrap().target().unwrap();
    commit_files(dir.path(), "second", MAIN_BRANCH, &[("v2.txt", "two\n")]);

    // Annotated tag naming the first commit: peeling must reach the commit.
    let target = repo.find_commit(first).unwrap().into_object();
    repo.tag("release", &target, &signature(), "annotated", false)
        .unwrap();

    let full_sha = first.to_string();
    let abbreviated = &full_sha[..7];
    for token in [full_sha.as_str(), abbreviated, "release"] {
        let paths = changed(dir.path(), reference(token)).unwrap();
        assert_eq!(
            lossy(&paths),
            vec!["v2.txt"],
            "base {token} resolves to the first commit"
        );
    }
}

#[test]
fn empty_and_unknown_reference_tokens_are_unresolved() {
    let (dir, _repo) = repo_with_commit(&[("file.txt", "one\n")]);
    for token in ["", "does-not-exist", "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef"] {
        let err = changed(dir.path(), reference(token)).unwrap_err();
        match &err {
            SniffError::UnresolvedReference { reference, .. } => {
                assert_eq!(reference, token);
            }
            other => panic!("token {token:?}: expected UnresolvedReference, got {other:?}"),
        }
        assert!(
            err.to_string().contains(token),
            "message names the token: {err}"
        );
    }
}

#[test]
fn tree_and_blob_references_are_non_commit() {
    let (dir, repo) = repo_with_commit(&[("file.txt", "one\n")]);
    let commit = repo.head().unwrap().peel_to_commit().unwrap();

    // A reference naming a tree, and one naming a bare blob.
    let tree_oid = commit.tree_id();
    repo.reference("refs/tags/tree-tag", tree_oid, true, "tree target")
        .unwrap();
    let blob_oid = repo.blob(b"loose blob bytes\n").unwrap();
    repo.reference("refs/tags/blob-tag", blob_oid, true, "blob target")
        .unwrap();

    for (token, kind) in [("tree-tag", "tree"), ("blob-tag", "blob")] {
        let err = changed(dir.path(), reference(token)).unwrap_err();
        match &err {
            SniffError::NonCommitReference {
                reference, kind: k, ..
            } => {
                assert_eq!(reference, token);
                assert_eq!(*k, kind);
            }
            other => panic!("token {token:?}: expected NonCommitReference, got {other:?}"),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Unborn and detached HEAD
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn omitted_base_on_unborn_head_uses_working_tree_status() {
    let dir = TempDir::new().unwrap();
    let repo = init_repo(dir.path());
    assert!(repo.head().is_err(), "fixture must be unborn");

    std::fs::write(dir.path().join("staged.txt"), "staged\n").unwrap();
    stage(dir.path(), "staged.txt");
    std::fs::write(dir.path().join("untracked.txt"), "untracked\n").unwrap();

    let paths = changed(dir.path(), omitted()).unwrap();
    assert_eq!(lossy(&paths), vec!["staged.txt", "untracked.txt"]);
}

#[test]
fn explicit_head_on_unborn_head_is_unresolved() {
    let dir = TempDir::new().unwrap();
    let _repo = init_repo(dir.path());
    std::fs::write(dir.path().join("staged.txt"), "staged\n").unwrap();
    stage(dir.path(), "staged.txt");

    let err = changed(dir.path(), reference("HEAD")).unwrap_err();
    assert!(
        matches!(err, SniffError::UnresolvedReference { .. }),
        "explicit HEAD on an unborn repository must fail, not fall back to the empty base: {err:?}"
    );
}

#[test]
fn explicit_base_with_unborn_head_compares_against_the_empty_tree() {
    let (dir, repo) = repo_with_commit(&[("base-only.txt", "gone from the index\n")]);
    let base_sha = repo.head().unwrap().target().unwrap().to_string();

    // Switch to an unborn branch, then drop the file from both the index and
    // the worktree so no status category can produce the path: only the
    // base-vs-empty-tree comparison can.
    repo.set_head("refs/heads/orphan").unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new("base-only.txt")).unwrap();
    index.write().unwrap();
    std::fs::remove_file(dir.path().join("base-only.txt")).unwrap();

    let paths = changed(dir.path(), reference(&base_sha)).unwrap();
    assert_eq!(
        lossy(&paths),
        vec!["base-only.txt"],
        "the explicit base is compared against the empty current tree"
    );

    // The omitted base still contributes nothing here.
    let paths = changed(dir.path(), omitted()).unwrap();
    assert!(paths.is_empty(), "no working-tree change remains: {paths:?}");
}

#[test]
fn detached_head_uses_its_commit() {
    let (dir, repo) = repo_with_commit(&[("a.txt", "a\n")]);
    let first = repo.head().unwrap().target().unwrap();
    commit_files(dir.path(), "second", MAIN_BRANCH, &[("b.txt", "b\n")]);

    // Detach at the first commit: the comparison must use the detached
    // commit, not main's tip.
    repo.set_head_detached(first).unwrap();

    let paths = changed(dir.path(), reference("main")).unwrap();
    assert_eq!(lossy(&paths), vec!["b.txt"]);
}

// ─────────────────────────────────────────────────────────────────────────
// Conflicts, bare, repository boundaries
// ─────────────────────────────────────────────────────────────────────────

/// Write unmerged index entries at stages 1-3 for `path`.
fn conflict_at(repo: &git2::Repository, path: &str, blob: &str) {
    let oid = repo.blob(blob.as_bytes()).unwrap();
    let mut index = repo.index().unwrap();
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
fn index_conflicts_anywhere_fail_naming_every_path() {
    let (dir, repo) = repo_with_commit(&[("clean.txt", "c\n")]);
    conflict_at(&repo, "conflicted.txt", "ours\n");
    conflict_at(&repo, "nested/conflicted.txt", "theirs\n");
    // Ordinary dirt elsewhere must not change the verdict.
    std::fs::write(dir.path().join("clean.txt"), "edited\n").unwrap();

    let err = changed(dir.path(), omitted()).unwrap_err();
    match &err {
        SniffError::MergeConflicts { paths, .. } => {
            assert_eq!(
                lossy(paths),
                vec!["conflicted.txt", "nested/conflicted.txt"],
                "every conflicted path is named, deduplicated and sorted"
            );
        }
        other => panic!("expected MergeConflicts, got {other:?}"),
    }
    let message = err.to_string();
    assert!(message.contains("conflicted.txt"), "{message}");
    assert!(message.contains("nested/conflicted.txt"), "{message}");
}

#[test]
fn bare_repository_is_rejected() {
    let dir = TempDir::new().unwrap();
    let _repo = init_bare_repo(dir.path());
    let err = changed(dir.path(), omitted()).unwrap_err();
    assert!(
        matches!(err, SniffError::BareRepository { .. }),
        "bare repositories are rejected explicitly: {err:?}"
    );
}

#[test]
fn discovery_outside_any_repository_returns_none() {
    let dir = TempDir::new().unwrap();
    assert!(GitRepo::discover(dir.path()).unwrap().is_none());
}

#[test]
fn nested_repository_contents_are_not_selected() {
    let (dir, _repo) = repo_with_commit(&[("outer.txt", "outer\n")]);

    // An independent repository nested inside the outer one, holding its own
    // uncommitted file.
    let inner = dir.path().join("inner");
    std::fs::create_dir_all(&inner).unwrap();
    let inner_repo = init_repo(&inner);
    std::fs::write(inner.join("inner-file.txt"), "inner change\n").unwrap();
    stage(&inner, "inner-file.txt");
    let _ = inner_repo.head();

    let paths = changed(dir.path(), omitted()).unwrap();
    for path in &paths {
        let crosses_boundary = path.components().next()
            == Some(Component::Normal(OsStr::new("inner")))
            && path.components().count() >= 2;
        assert!(
            !crosses_boundary,
            "no recursive selection inside a nested repository: {path:?}"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────
// Path spellings
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn non_ascii_and_spaced_paths_survive() {
    let (dir, _repo) = repo_with_commit(&[("日本語/追跡.txt", "tracked\n")]);

    std::fs::create_dir_all(dir.path().join("dir with space")).unwrap();
    std::fs::write(dir.path().join("dir with space/file name.rs"), "x\n").unwrap();
    std::fs::write(dir.path().join("héllo-wörld.rs"), "y\n").unwrap();
    std::fs::write(dir.path().join("日本語/新規.txt"), "z\n").unwrap();

    let paths = changed(dir.path(), omitted()).unwrap();
    assert_eq!(
        lossy(&paths),
        vec![
            "dir with space/file name.rs",
            "héllo-wörld.rs",
            "日本語/新規.txt",
        ]
    );
}

#[cfg(unix)]
#[test]
fn non_utf8_paths_survive_byte_exact() {
    use std::os::unix::ffi::OsStrExt;

    let (dir, repo) = repo_with_commit(&[("plain.txt", "p\n")]);
    let base_sha = repo.head().unwrap().target().unwrap().to_string();
    let raw_path = b"bad-\xff-file.txt";
    let name = std::ffi::OsStr::from_bytes(raw_path);

    // macOS rejects non-UTF-8 filenames at the syscall level (EILSEQ), so a
    // worktree copy of the file exists only where the filesystem accepts
    // the name. The committed comparison below covers every Unix host: the
    // path lives in the index and tree without ever touching the disk.
    let worktree_accepts = std::fs::write(dir.path().join(name), "raw bytes\n").is_ok();
    if worktree_accepts {
        let paths = changed(dir.path(), omitted()).unwrap();
        assert_eq!(
            paths,
            vec![PathBuf::from(name)],
            "an uncommitted non-UTF-8 path is returned as native bytes, never lossy text"
        );
    }
    stage_bytes(dir.path(), raw_path);
    commit_index(dir.path(), "stage the raw-named file", MAIN_BRANCH);

    let paths = changed(dir.path(), reference(&base_sha)).unwrap();
    assert!(
        paths.contains(&PathBuf::from(name)),
        "the non-UTF-8 path crosses the tree diff as native bytes: {paths:?}"
    );
}

#[cfg(unix)]
#[test]
fn executable_mode_change_selects_the_path() {
    let (dir, repo) = repo_with_commit(&[("script.sh", "echo mode\n")]);
    let base_sha = repo.head().unwrap().target().unwrap().to_string();

    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(
        dir.path().join("script.sh"),
        std::fs::Permissions::from_mode(0o755),
    )
    .unwrap();
    stage(dir.path(), "script.sh");
    commit_index(dir.path(), "chmod +x", MAIN_BRANCH);

    let paths = changed(dir.path(), reference(&base_sha)).unwrap();
    assert_eq!(
        lossy(&paths),
        vec!["script.sh"],
        "a mode-only change is a committed difference"
    );
}

// ─────────────────────────────────────────────────────────────────────────
// Work assertion
// ─────────────────────────────────────────────────────────────────────────

#[test]
fn changed_paths_performs_one_status_walk_and_no_diff_work() {
    let (dir, repo) = repo_with_commit(&[("committed.txt", "one\n")]);
    let base_sha = repo.head().unwrap().target().unwrap().to_string();
    commit_files(dir.path(), "second", MAIN_BRANCH, &[("second.txt", "two\n")]);

    // Every working-tree category plus a committed difference, so both halves
    // of the request have real work to do.
    std::fs::write(dir.path().join("committed.txt"), "staged\n").unwrap();
    stage(dir.path(), "committed.txt");
    std::fs::write(dir.path().join("committed.txt"), "staged then edited\n").unwrap();
    std::fs::write(dir.path().join("untracked.txt"), "new\n").unwrap();

    let collector = crate::performance::PerformanceCollector::new_shared();
    let before = super::status::status_walk_count(dir.path());
    let paths = crate::performance::with_current_collector(Some(collector.clone()), || {
        discover(dir.path()).changed_paths(reference(&base_sha))
    })
    .unwrap();
    let counters = collector.snapshot(std::time::Duration::ZERO).counters;

    assert!(!paths.is_empty(), "fixture must produce paths");
    let walk_delta = super::status::status_walk_count(dir.path()) - before;
    assert_eq!(
        walk_delta, 1,
        "one status observation answers staged, unstaged, untracked, and conflict detection"
    );

    let count = |name: &str| counters.get(name).copied().unwrap_or(0);
    assert_eq!(
        count(crate::performance::counters::GIT_STATUS_WALKS),
        1,
        "counters: {counters:?}"
    );
    assert_eq!(
        count(crate::performance::counters::GIT_BLOB_LOADS),
        0,
        "no blob loads for line statistics: {counters:?}"
    );
    assert_eq!(
        count(crate::performance::counters::GIT_FILE_DIFFS),
        0,
        "no line-diff or patch work: {counters:?}"
    );
    assert_eq!(
        count(crate::performance::counters::GIT_REF_WALKS),
        0,
        "no ref-store enumeration: {counters:?}"
    );
    assert_eq!(
        count(crate::performance::counters::GIT_COMMIT_VISITS),
        0,
        "no history walk: {counters:?}"
    );
    assert_eq!(
        count(crate::performance::counters::GIT_WORKTREE_OPENS),
        0,
        "no linked-worktree enumeration: {counters:?}"
    );
}

fn symlink_file(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).expect("create file symlink");
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(target, link).expect("file symlink requires Developer Mode or symlink privilege");
}

#[test]
fn file_type_replacements_agree_across_status_readers_and_commit_phases() {
    use super::status::{get_repo_status_counts, get_repo_status_counts_detailed, is_repo_dirty};
    use super::types::{FileAction, FileStatus};
    use crate::request::GitRequest;

    for initially_symlink in [false, true] {
        let (dir, repo) = repo_with_commit(&[("leaf.rs", "fn original() {}\n"), ("target.rs", "fn target() {}\n")]);
        repo.config().unwrap().set_bool("core.symlinks", true).unwrap();
        if initially_symlink {
            std::fs::remove_file(dir.path().join("leaf.rs")).unwrap();
            symlink_file(Path::new("target.rs"), &dir.path().join("leaf.rs"));
            stage(dir.path(), "leaf.rs");
            commit_index(dir.path(), "symlink base", "HEAD");
        }
        let base = git2::Repository::open(dir.path()).unwrap().head().unwrap().target().unwrap();
        std::fs::remove_file(dir.path().join("leaf.rs")).unwrap();
        if initially_symlink {
            std::fs::write(dir.path().join("leaf.rs"), "fn restored() {}\n").unwrap();
        } else {
            symlink_file(Path::new("target.rs"), &dir.path().join("leaf.rs"));
        }
        for staged in [false, true] {
            if staged { stage(dir.path(), "leaf.rs"); }
            let observed = discover(dir.path());
            assert_eq!(observed.changed_paths(omitted()).unwrap(), vec![PathBuf::from("leaf.rs")]);
            let changes = observed.file_changes().unwrap();
            assert_eq!(changes.len(), 1, "no missing or extra changed files");
            assert_eq!(changes[0].path, PathBuf::from("leaf.rs"));
            assert_eq!(changes[0].action, FileAction::Modified);
            assert_eq!(changes[0].status, if staged { FileStatus::Staged } else { FileStatus::Modified });
            let status = observed.repo_status().unwrap();
            assert!(status.is_dirty);
            assert_eq!((status.staged_count, status.unstaged_count, status.untracked_count), if staged { (1, 0, 0) } else { (0, 1, 0) });
            let gix = gix::open(dir.path()).unwrap();
            assert!(is_repo_dirty(&gix).unwrap());
            assert_eq!(get_repo_status_counts(&gix).unwrap(), (true, 1));
            assert_eq!(get_repo_status_counts_detailed(&gix).unwrap(), if staged { (true, 1, 0, 0) } else { (true, 0, 1, 0) });
            for request in [GitRequest::minimal(), GitRequest::summary()] {
                assert!(observed.detect_with_request(&request).unwrap().status.unwrap().is_dirty);
            }
            let mut counts = GitRequest::minimal();
            counts.commit_count = 1;
            let status = observed.detect_with_request(&counts).unwrap().status.unwrap();
            assert!(status.is_dirty);
            assert_eq!((status.staged_count, status.unstaged_count, status.untracked_count), if staged { (1, 0, 0) } else { (0, 1, 0) });
        }
        commit_index(dir.path(), "type replacement", "HEAD");
        let observed = discover(dir.path());
        assert_eq!(observed.changed_paths(reference(&base.to_string())).unwrap(), vec![PathBuf::from("leaf.rs")]);
        assert!(observed.changed_paths(omitted()).unwrap().is_empty());
        assert!(observed.file_changes().unwrap().is_empty());
        assert!(!observed.repo_status().unwrap().is_dirty);
        let gix = gix::open(dir.path()).unwrap();
        assert!(!is_repo_dirty(&gix).unwrap());
        assert_eq!(get_repo_status_counts(&gix).unwrap(), (false, 0));
        assert_eq!(get_repo_status_counts_detailed(&gix).unwrap(), (false, 0, 0, 0));
    }
}

#[path = "index_reads_tests.rs"]
mod index_reads;
