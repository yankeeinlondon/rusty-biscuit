//! Integration tests for the `ImplicitRelative` reference kind.
//!
//! These tests build a real git repository in a temp directory so the
//! `find_git_root` logic in `biscuit-file` can discover it the same way it
//! would in production.

use std::fs;
use std::path::Path;

use biscuit_file::FileReference;
use tempfile::TempDir;

/// Initialise a fresh git repository at `path`.
fn git_init(path: &Path) {
    // Use git2 directly so we don't depend on a system git binary.
    git2::Repository::init(path).expect("git init failed");
}

#[test]
fn resolves_file_in_git_root_when_absent_from_cwd() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Create `file_in_root.md` at the repo root.
    fs::write(repo_root.join("file_in_root.md"), b"root").unwrap();

    // Create a nested subdir that will act as the "CWD".
    let subdir = repo_root.join("sub/dir");
    fs::create_dir_all(&subdir).unwrap();

    // Canonicalize to match what the resolver does internally.
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("file_in_root.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("file_in_root.md").as_path()),
        "implicit relative path should fall back to git root"
    );
}

#[test]
fn prefers_cwd_over_git_root_on_name_collision() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Same filename exists in both git root and a subdirectory.
    fs::write(repo_root.join("notes.md"), b"root").unwrap();
    let subdir = repo_root.join("pkg");
    fs::create_dir_all(&subdir).unwrap();
    fs::write(subdir.join("notes.md"), b"subdir").unwrap();

    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("notes.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(subdir.join("notes.md").as_path()),
        "CWD should take priority over git root for implicit relative refs"
    );
}

#[test]
fn explicit_relative_does_not_fall_back_to_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    fs::write(repo_root.join("file_in_root.md"), b"root").unwrap();
    let subdir = repo_root.join("sub");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("./file_in_root.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert!(
        resolved.is_none(),
        "./ prefix should pin lookup to CWD only; got {resolved:?}"
    );
}

#[test]
fn subdir_path_resolves_against_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    fs::create_dir_all(repo_root.join("foo/bar")).unwrap();
    fs::write(repo_root.join("foo/bar/doc.md"), b"nested").unwrap();

    // CWD is the repo root's sibling subdir with no `foo/bar/doc.md`.
    let subdir = repo_root.join("other");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("foo/bar/doc.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("foo/bar/doc.md").as_path()),
    );
}

#[test]
fn returns_none_when_neither_cwd_nor_git_root_has_file() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    let subdir = repo_root.join("pkg");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();

    let resolved = FileReference::new("does_not_exist.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert!(resolved.is_none());
}

#[test]
fn recursive_implicit_relative_finds_file_under_git_root() {
    let tmp = TempDir::new().unwrap();
    let repo_root = tmp.path();
    git_init(repo_root);

    // Deeply nested file.
    fs::create_dir_all(repo_root.join("a/b/c")).unwrap();
    fs::write(repo_root.join("a/b/c/deep.md"), b"deep").unwrap();

    // CWD is a sibling that won't see `deep.md` via its own walk.
    let subdir = repo_root.join("other");
    fs::create_dir_all(&subdir).unwrap();
    let subdir = subdir.canonicalize().unwrap();
    let repo_root_canon = repo_root.canonicalize().unwrap();

    let resolved = FileReference::new("%deep.md")
        .unwrap()
        .resolve_from(&subdir)
        .unwrap();

    assert_eq!(
        resolved.as_deref(),
        Some(repo_root_canon.join("a/b/c/deep.md").as_path()),
        "recursive search should include git root as a traversal start"
    );
}
