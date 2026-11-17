//! How a terminal restoration failure combines with the launch's own result.
//! The end-to-end handoff runs in a PTY in the CLI's `editor_launcher` tests.

use std::io;

use super::{EditorError, with_restoration};

fn restore_failure() -> io::Result<()> {
    Err(io::Error::from(io::ErrorKind::PermissionDenied))
}

#[test]
fn a_restoration_failure_fails_an_otherwise_successful_edit() {
    let error = with_restoration(Ok(()), restore_failure()).unwrap_err();
    let EditorError::Io { operation, source } = &error else {
        panic!("{error:?}");
    };
    assert_eq!(*operation, "restore the terminal");
    assert_eq!(source.kind(), io::ErrorKind::PermissionDenied);
}

#[test]
fn an_earlier_failure_stays_primary_over_a_restoration_failure() {
    let path = std::path::PathBuf::from("/notes/a.md");
    let earlier = [
        EditorError::Cancelled { path: path.clone() },
        EditorError::NonZeroExit { code: 3, editor: "vi".into(), path: path.clone() },
        EditorError::Io { operation: "wait for the editor", source: io::Error::from(io::ErrorKind::Interrupted) },
    ];
    for primary in earlier {
        let expected = primary.to_string();
        let error = with_restoration(Err(primary), restore_failure()).unwrap_err();
        assert_eq!(error.to_string(), expected);
    }
}

#[test]
fn a_successful_restoration_leaves_the_result_unchanged() {
    assert!(with_restoration(Ok(()), Ok(())).is_ok());
    let error = with_restoration(Err(EditorError::NonZeroExit { code: 7, editor: "vi".into(), path: "/a.md".into() }), Ok(()))
        .unwrap_err();
    assert!(matches!(error, EditorError::NonZeroExit { code: 7, .. }), "{error:?}");
}
