//! `EditorError` variant snapshots.

use std::io;

use darkmatter::editor::EditorError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn no_editor_found_lists_probe_binaries() {
    let err = EditorError::NoEditorFound;
    let out = render(&err);
    assert_contains_all(&out, &["EditorError", "no editor found", "$EDITOR"]);
    assert!(
        out.contains("nvim") || out.contains("vim") || out.contains("code"),
        "expected editor probe list in output; got:\n{out}"
    );
}

#[test]
fn non_zero_exit_surfaces_status_code() {
    let err = EditorError::NonZeroExit(42);
    let out = render(&err);
    assert_contains_all(&out, &["EditorError", "exited with error", "42"]);
}

#[test]
fn missing_file_hints_at_restore() {
    let err = EditorError::Missing;
    let out = render(&err);
    assert_contains_all(&out, &["EditorError", "edited file missing"]);
}

#[test]
fn launch_failed_includes_editor_and_command() {
    let err = EditorError::LaunchFailed {
        editor: "nvim".into(),
        source: io::Error::new(io::ErrorKind::PermissionDenied, "permission denied"),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["EditorError", "launch failed", "nvim", "PermissionDenied"],
    );
}

#[test]
fn io_error_includes_kind() {
    let err = EditorError::Io(io::Error::new(io::ErrorKind::NotFound, "temp file vanished"));
    let out = render(&err);
    assert_contains_all(&out, &["EditorError", "NotFound"]);
}
