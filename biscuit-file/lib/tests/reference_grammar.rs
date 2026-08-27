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
    assert_eq!(kind("&docs/spec.md"), FileReferenceKind::RepositoryRoot);
    assert_eq!(kind("^docs/spec.md"), FileReferenceKind::RepositoryScoped);
    assert_eq!(kind("vault:notes/today.md"), FileReferenceKind::Vault);
    assert_eq!(kind("http://example.com/a.md"), FileReferenceKind::Url);
    assert_eq!(kind("https://example.com/a.md"), FileReferenceKind::Url);
}

#[test]
fn defensive_sigil_payloads_are_portable_and_relative() {
    for sigil in ['@', '&', '^'] {
        for suffix in [
            "",
            "/",
            "//etc/hosts",
            "///etc/hosts",
            r"\Windows\win.ini",
            r"/\Windows\win.ini",
            r"\\server\share\file.md",
            r"/\\server\share\file.md",
            r"C:\Windows\win.ini",
            r"/C:\Windows\win.ini",
            r"C:Windows\win.ini",
        ] {
            let raw = format!("{sigil}{suffix}");
            assert!(
                matches!(
                    FileReference::new(&raw),
                    Err(FileReferenceError::InvalidSyntax(_))
                ),
                "rooted or empty sigil payload must be rejected on every host: {raw:?}",
            );

            let recursive = format!("%{raw}");
            assert!(
                matches!(
                    FileReference::new(&recursive),
                    Err(FileReferenceError::InvalidSyntax(_))
                ),
                "recursive rooted or empty sigil payload must be rejected: {recursive:?}",
            );
        }
    }
}

#[test]
fn one_forward_slash_after_a_defensive_sigil_is_optional() {
    for (compact, separated, expected) in [
        ("@docs/spec.md", "@/docs/spec.md", FileReferenceKind::Magic),
        (
            "&docs/spec.md",
            "&/docs/spec.md",
            FileReferenceKind::RepositoryRoot,
        ),
        (
            "^docs/spec.md",
            "^/docs/spec.md",
            FileReferenceKind::RepositoryScoped,
        ),
    ] {
        let compact = FileReference::new(compact).unwrap();
        let separated = FileReference::new(separated).unwrap();
        assert_eq!(compact.class().kind, expected);
        assert_eq!(separated.class().kind, expected);
        assert_eq!(compact.class(), separated.class());
    }
}

#[test]
fn removed_package_sigil_has_a_migration_diagnostic() {
    let error = FileReference::new("!lib/src/lib.rs").unwrap_err();
    let FileReferenceError::InvalidSyntax(detail) = error else {
        panic!("removed `!` must be InvalidSyntax");
    };
    assert!(detail.contains("removed"));
    assert!(detail.contains('!'));
    assert!(detail.contains('^'));

    assert_eq!(
        kind("./!weird-name.md"),
        FileReferenceKind::ExplicitRelative
    );
}

#[test]
fn reserved_schemes_and_windows_device_prefixes_are_rejected() {
    for (raw, expected_scheme) in [
        ("C:path", "C"),
        ("file:", "file"),
        ("file:///tmp/a.md", "file"),
        ("htps://example.com/a.md", "htps"),
    ] {
        assert!(matches!(
            FileReference::new(raw),
            Err(FileReferenceError::UnsupportedScheme { ref scheme, ref reference })
                if scheme == expected_scheme && reference == raw
        ));
    }

    for raw in [r"\\?\C:\work\a.md", r"\\.\C:\work\a.md"] {
        assert!(
            matches!(
                FileReference::new(raw),
                Err(FileReferenceError::InvalidSyntax(_))
            ),
            "Windows device prefixes are reserved on every host: {raw:?}",
        );
    }
}

#[test]
fn supported_absolute_and_explicit_filename_escape_hatches_are_preserved() {
    assert_eq!(kind("C:"), FileReferenceKind::Absolute);
    assert_eq!(kind("C:/abs"), FileReferenceKind::Absolute);
    assert_eq!(kind(r"C:\abs"), FileReferenceKind::Absolute);
    assert_eq!(kind("./name:part"), FileReferenceKind::ExplicitRelative);
}

#[test]
fn a_second_recursive_marker_is_invalid() {
    for raw in ["%%", "%%x"] {
        assert!(matches!(
            FileReference::new(raw),
            Err(FileReferenceError::InvalidSyntax(_))
        ));
    }
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
fn repository_error_diagnostics_carry_only_the_required_context() {
    let outside = FileReferenceError::OutsideRepository {
        sigil: '&',
        reference_cwd: "/workspace/outside".into(),
    }
    .to_string();
    assert!(outside.contains('&'));
    assert!(outside.contains("/workspace/outside"));

    let escaped = FileReferenceError::RepositoryEscape {
        sigil: '^',
        reference: "^../outside.md".to_string(),
        repository_root: "/workspace/repo".into(),
        escaped_candidate: "/workspace/outside.md".into(),
    }
    .to_string();
    assert!(escaped.contains('^'));
    assert!(escaped.contains("^../outside.md"));
    assert!(escaped.contains("/workspace/repo"));
    assert!(escaped.contains("/workspace/outside.md"));
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
