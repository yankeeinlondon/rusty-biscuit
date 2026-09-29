//! `FileReference::validate_contained_candidate`: containment of a candidate
//! of any reference kind in a caller-chosen boundary, lexically and after
//! following symlinks.

use std::fs;
use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileReferenceError};
use tempfile::TempDir;

/// A boundary directory holding `docs/note.md` and `src/config.rs`, plus a
/// sibling directory outside it holding `secret.md`.
struct Layout {
    _root: TempDir,
    boundary: PathBuf,
    outside: PathBuf,
}

fn layout() -> Layout {
    let root = TempDir::new().expect("temp dir");
    let boundary = root.path().join("boundary");
    let outside = root.path().join("outside");
    fs::create_dir_all(boundary.join("docs")).unwrap();
    fs::create_dir_all(boundary.join("src")).unwrap();
    fs::create_dir_all(&outside).unwrap();
    fs::write(boundary.join("docs/note.md"), "note").unwrap();
    fs::write(boundary.join("src/config.rs"), "config").unwrap();
    fs::write(outside.join("secret.md"), "secret").unwrap();
    Layout {
        _root: root,
        boundary,
        outside,
    }
}

fn check(reference: &str, candidate: &Path, boundary: &Path) -> Result<(), FileReferenceError> {
    FileReference::new(reference)
        .expect("valid reference")
        .validate_contained_candidate(candidate, boundary)
}

fn assert_escape(result: Result<(), FileReferenceError>, reference: &str) {
    match result {
        Err(FileReferenceError::BoundaryEscape {
            reference: reported,
            ..
        }) => assert_eq!(reported, reference),
        other => panic!("{reference}: expected a boundary escape, got {other:?}"),
    }
}

#[test]
fn candidates_inside_the_boundary_pass() {
    let layout = layout();
    let docs = layout.boundary.join("docs");
    for (reference, candidate) in [
        ("./note.md", docs.join("./note.md")),
        ("note.md", docs.join("note.md")),
        ("../src/config.rs", docs.join("../src/config.rs")),
        // A missing file is judged by the directory that would hold it.
        ("./missing.md", docs.join("./missing.md")),
        ("../new/dir/file.md", docs.join("../new/dir/file.md")),
    ] {
        check(reference, &candidate, &layout.boundary)
            .unwrap_or_else(|error| panic!("{reference}: {error}"));
    }
}

#[test]
fn a_parent_path_escaping_the_boundary_is_rejected() {
    let layout = layout();
    let docs = layout.boundary.join("docs");
    // Existing and missing targets escape alike: the lexical check comes first.
    assert_escape(
        check("../../outside/secret.md", &docs.join("../../outside/secret.md"), &layout.boundary),
        "../../outside/secret.md",
    );
    assert_escape(
        check("../../elsewhere.rs", &docs.join("../../elsewhere.rs"), &layout.boundary),
        "../../elsewhere.rs",
    );
    // A sibling whose name extends the boundary's is outside it.
    let sibling = layout.boundary.with_file_name("boundary-extra");
    fs::create_dir_all(&sibling).unwrap();
    assert_escape(
        check("../../boundary-extra/x.md", &sibling.join("x.md"), &layout.boundary),
        "../../boundary-extra/x.md",
    );
    let Err(FileReferenceError::BoundaryEscape {
        boundary,
        escaped_candidate,
        ..
    }) = check("../../elsewhere.rs", &docs.join("../../elsewhere.rs"), &layout.boundary)
    else {
        panic!("expected an escape");
    };
    assert_eq!(boundary, layout.boundary);
    assert_eq!(escaped_candidate, layout.boundary.parent().unwrap().join("elsewhere.rs"));
}

#[cfg(unix)]
#[test]
fn a_symlink_escaping_the_boundary_is_rejected() {
    use std::os::unix::fs::symlink;

    let layout = layout();
    symlink(&layout.outside, layout.boundary.join("linked-dir")).unwrap();
    symlink(layout.outside.join("secret.md"), layout.boundary.join("linked-file.md")).unwrap();
    // Lexically inside, resolved outside.
    assert_escape(
        check("linked-file.md", &layout.boundary.join("linked-file.md"), &layout.boundary),
        "linked-file.md",
    );
    assert_escape(
        check("linked-dir/secret.md", &layout.boundary.join("linked-dir/secret.md"), &layout.boundary),
        "linked-dir/secret.md",
    );
    // A missing file under a symlinked directory is judged by that directory.
    assert_escape(
        check("linked-dir/missing.md", &layout.boundary.join("linked-dir/missing.md"), &layout.boundary),
        "linked-dir/missing.md",
    );
    // A symlink that stays inside passes.
    symlink(layout.boundary.join("src/config.rs"), layout.boundary.join("inner.rs")).unwrap();
    check("inner.rs", &layout.boundary.join("inner.rs"), &layout.boundary).unwrap();
}

#[cfg(unix)]
#[test]
fn a_broken_symlink_is_judged_by_its_directory() {
    use std::os::unix::fs::symlink;

    let layout = layout();
    symlink(layout.boundary.join("gone.md"), layout.boundary.join("dangling.md")).unwrap();
    check("dangling.md", &layout.boundary.join("dangling.md"), &layout.boundary).unwrap();
    // Its dangling target is outside, but nothing there exists to read.
    symlink(layout.outside.join("gone.md"), layout.boundary.join("dangling-out.md")).unwrap();
    check("dangling-out.md", &layout.boundary.join("dangling-out.md"), &layout.boundary).unwrap();
}

#[test]
fn a_missing_boundary_is_an_io_error() {
    let layout = layout();
    let boundary = layout.boundary.join("not-there");
    match check("x.md", &boundary.join("x.md"), &boundary) {
        Err(FileReferenceError::Io { path, source }) => {
            assert_eq!(path, boundary);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected an I/O error, got {other:?}"),
    }
}
