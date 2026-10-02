//! `FileTreeError` variant snapshots.

use std::path::PathBuf;

use darkmatter::markdown::reference::file_tree::FileTreeError;

use super::helpers::{assert_contains_all, render};

#[test]
fn path_not_found_shows_path() {
    let err = FileTreeError::PathNotFound(PathBuf::from("./does/not/exist.md"));
    let out = render(&err);
    assert_contains_all(
        &out,
        &["FileTreeError", "path not found", "./does/not/exist.md"],
    );
}

#[test]
fn not_a_file_shows_path() {
    let err = FileTreeError::NotAFile(PathBuf::from("./some/dir"));
    let out = render(&err);
    assert_contains_all(&out, &["FileTreeError", "not a file", "./some/dir"]);
}

#[test]
fn markdown_delegates_inner_block() {
    let err = FileTreeError::Markdown(darkmatter::markdown::MarkdownError::Transform(
        "pipeline stalled".into(),
    ));
    let out = render(&err);
    // Delegating variant shows the inner MarkdownError block, not a FileTreeError wrapper.
    assert_contains_all(
        &out,
        &["MarkdownError", "transform failed", "pipeline stalled"],
    );
}

#[test]
fn reference_delegates_inner_block() {
    let err = FileTreeError::Reference(
        darkmatter::markdown::reference::ReferenceError::Validation("orphan node".into()),
    );
    let out = render(&err);
    assert_contains_all(
        &out,
        &["ReferenceError", "validation failed", "orphan node"],
    );
}

#[test]
fn not_a_file_resolved_path_has_no_verbatim_prefix() {
    let dir = tempfile::TempDir::new().unwrap();
    let err = FileTreeError::NotAFile(dir.path().to_path_buf());
    let out = render(&err);
    // A Windows `\\?\` prefix from a raw `canonicalize` must never reach the user.
    assert!(!out.contains(r"\\?\"), "verbatim path leaked: {out}");
    assert_contains_all(&out, &["not a file", "Resolved:"]);
}
