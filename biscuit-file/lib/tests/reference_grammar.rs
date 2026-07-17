//! Public-API tests for reference classification (`FileReference::class`).
//!
//! These exercise the observable grammar with no filesystem access: they
//! assert the public [`FileReferenceKind`]/[`FileReferenceClass`] surface and
//! the cross-platform (Windows/POSIX) and home-pinned grammar added in the
//! file-resolution work. Classification is host-independent, so Windows drive
//! and UNC forms classify as absolute even on this POSIX host.

use biscuit_file::{FileReference, FileReferenceError, FileReferenceKind};

fn kind(raw: &str) -> FileReferenceKind {
    FileReference::new(raw)
        .unwrap_or_else(|e| panic!("`{raw}` failed to parse: {e}"))
        .class()
        .kind
}

#[test]
fn explicit_vs_implicit_relative_are_distinguished_without_fs() {
    assert_eq!(kind("./foo.md"), FileReferenceKind::ExplicitRelative);
    assert_eq!(kind("../foo.md"), FileReferenceKind::ExplicitRelative);
    assert_eq!(kind("."), FileReferenceKind::ExplicitRelative);
    assert_eq!(kind(".."), FileReferenceKind::ExplicitRelative);

    assert_eq!(kind("foo.md"), FileReferenceKind::ImplicitRelative);
    assert_eq!(kind("path/to/foo.md"), FileReferenceKind::ImplicitRelative);
}

#[test]
fn special_kinds_classify() {
    assert_eq!(kind("/tmp/a.md"), FileReferenceKind::Absolute);
    assert_eq!(kind("@docs/spec.md"), FileReferenceKind::Magic);
    assert_eq!(kind("!lib/src/lib.rs"), FileReferenceKind::Package);
    assert_eq!(kind("vault:notes/today.md"), FileReferenceKind::Vault);
    assert_eq!(kind("http://example.com/a.md"), FileReferenceKind::Url);
    assert_eq!(kind("https://example.com/a.md"), FileReferenceKind::Url);
}

#[test]
fn home_kind_is_observable() {
    assert_eq!(kind("~"), FileReferenceKind::Home);
    assert_eq!(kind("~/notes.md"), FileReferenceKind::Home);
    assert_eq!(kind(r"~\notes.md"), FileReferenceKind::Home);
}

#[test]
fn tilde_user_is_rejected_at_construction() {
    let err = FileReference::new("~alice/notes.md").unwrap_err();
    assert!(
        matches!(err, FileReferenceError::UnsupportedUserHome(_)),
        "expected UnsupportedUserHome, got {err}"
    );
}

#[test]
fn recursive_is_a_modifier_over_a_kind() {
    let class = FileReference::new("%docs/spec.md").unwrap().class();
    assert_eq!(class.kind, FileReferenceKind::ImplicitRelative);
    assert!(class.recursive, "`%` sets the recursive modifier");

    let class = FileReference::new("%@README.md").unwrap().class();
    assert_eq!(class.kind, FileReferenceKind::Magic);
    assert!(class.recursive);

    let class = FileReference::new("docs/spec.md").unwrap().class();
    assert!(!class.recursive);
}

#[test]
fn windows_absolute_and_unc_classify_absolute_on_any_host() {
    assert_eq!(kind(r"C:\work\a.md"), FileReferenceKind::Absolute);
    assert_eq!(kind("C:/work/a.md"), FileReferenceKind::Absolute);
    assert_eq!(kind(r"\\server\share\a.md"), FileReferenceKind::Absolute);
}

#[test]
fn windows_drive_relative_is_not_absolute() {
    // `C:foo.md` is drive-relative; it must not be mistaken for absolute.
    assert_eq!(kind("C:foo.md"), FileReferenceKind::ImplicitRelative);
}

#[test]
fn windows_backslash_explicit_relative() {
    assert_eq!(kind(r".\foo.md"), FileReferenceKind::ExplicitRelative);
    assert_eq!(kind(r"..\foo.md"), FileReferenceKind::ExplicitRelative);
}

#[test]
fn url_scheme_recognition_is_case_insensitive_and_beats_drive_classifier() {
    assert_eq!(kind("HTTP://example.com/a.md"), FileReferenceKind::Url);
    assert_eq!(kind("HttpS://example.com/a.md"), FileReferenceKind::Url);
}
