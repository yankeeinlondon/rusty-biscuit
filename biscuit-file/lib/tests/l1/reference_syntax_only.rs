//! Reference and glob grammar under `file-reference-syntax` alone.
//!
//! This module is the one the `l1` binary keeps when built with
//! `--no-default-features --features file-reference-syntax,yaml`, so it
//! proves the passive grammar needs none of the resolution stack (`gix`,
//! `walkdir`).

use biscuit_file::{
    FileReference, FileReferenceError, FileReferenceKind, GlobReference, GlobReferenceError,
    ResolutionFailure,
};

#[test]
fn explicit_relative_reference_parses() {
    let reference = FileReference::new("./a.md").unwrap();

    assert_eq!(reference.raw(), "./a.md");
    assert_eq!(reference.class().kind, FileReferenceKind::ExplicitRelative);
    assert!(!reference.class().recursive);
}

#[test]
fn magic_reference_parses_with_its_payload() {
    let reference = FileReference::new("%@docs/spec.md").unwrap();

    assert_eq!(reference.class().kind, FileReferenceKind::Magic);
    assert!(reference.class().recursive);
    assert_eq!(reference.payload(), "docs/spec.md");
}

#[test]
fn url_reference_parses() {
    let reference = FileReference::new("https://example.com/docs/a.md").unwrap();

    assert_eq!(reference.class().kind, FileReferenceKind::Url);
}

#[test]
fn invalid_reference_is_a_typed_syntax_error() {
    let error = FileReference::new("").unwrap_err();

    assert!(matches!(error, FileReferenceError::InvalidSyntax(_)), "{error:?}");
    assert_eq!(error.resolution_failure(), ResolutionFailure::InvalidReference);
}

#[test]
fn glob_reference_parses() {
    let glob = GlobReference::new(["src/**/*.rs", "!src/**/generated/**"]).unwrap();

    assert_eq!(
        glob.patterns().collect::<Vec<_>>(),
        ["src/**/*.rs", "!src/**/generated/**"]
    );
}

#[test]
fn invalid_glob_is_a_typed_error() {
    let error = GlobReference::new(["src/[*.rs"]).unwrap_err();

    assert!(
        matches!(&error, GlobReferenceError::InvalidGlob { pattern, .. } if pattern == "src/[*.rs"),
        "{error:?}"
    );
    assert_eq!(error.resolution_failure(), ResolutionFailure::InvalidReference);
}
