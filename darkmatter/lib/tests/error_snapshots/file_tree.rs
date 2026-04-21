//! `FileTreeError` variant snapshots.

use std::path::PathBuf;

use darkmatter::markdown::reference::file_tree::FileTreeError;

use crate::helpers::{assert_contains_all, render};

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
    assert_contains_all(&out, &["MarkdownError", "transform failed", "pipeline stalled"]);
}

#[test]
fn reference_delegates_inner_block() {
    let err = FileTreeError::Reference(darkmatter::markdown::reference::ReferenceError::Validation(
        "orphan node".into(),
    ));
    let out = render(&err);
    assert_contains_all(&out, &["ReferenceError", "validation failed", "orphan node"]);
}
