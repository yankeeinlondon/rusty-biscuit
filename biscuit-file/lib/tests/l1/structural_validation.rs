//! Public-API tests for passive structural validation (`FileReference::new`).
//!
//! `FileReference::new` takes no resolution context, so it has no filesystem,
//! environment, configuration, or network to read: its verdict comes from
//! the text alone. The corpus carries no `cfg`, so every host OS asserts the
//! same verdict for every row, Windows spellings included. Templates name
//! variables that are never set, which proves a well-formed template is
//! accepted without reading the environment.

use biscuit_file::{FileReference, FileReferenceError};

/// Well-formed references of every supported kind.
const WELL_FORMED: &[&str] = &[
    "./docs/spec.md",
    "../spec.md",
    ".",
    "/tmp/spec.md",
    r"\Users\ken\data\foobar.json",
    r"C:\x\y.md",
    "C:/x/y.md",
    r"\\server\share\x.md",
    "//server/share/x.md",
    r".\docs\spec.md",
    "@docs/spec.md",
    "@/docs/spec.md",
    "@@name.md",
    "%@spec.md",
    "%./docs",
    "&docs/spec.md",
    "^docs/spec.md",
    "~",
    "~/spec.md",
    r"~\spec.md",
    "vault:notes/spec.md",
    "vault::notes/spec.md",
    "https://example.com/spec.md",
    "http://example.com/spec.md",
    "HTTPS://example.com/spec.md",
    "https://example.com",
    "https://example.com:8080/a?b=c#d",
    "https://{{BISCUIT_FILE_UNSET_HOST}}/spec.md",
    "https://example.com/{{BISCUIT_FILE_UNSET_PATH}}.md",
    "{{BISCUIT_FILE_UNSET_ROOT}}/x.md",
    "vault:{{BISCUIT_FILE_UNSET_NOTE}}.md",
    "${HOME}/x",
    "$HOME/x",
    "42",
    "hello world",
];

/// Malformed references: each is rejected before any resolution.
const MALFORMED: &[&str] = &[
    "",
    "a\0b.md",
    "./docs/\0",
    "https://",
    "http://",
    "https:///x",
    "https://?q",
    "https://#frag",
    "https://exa mple.com/x",
    "%https://example.com/x",
    "%http://example.com/x",
    "vault:",
    "vault::",
    "vault:/etc/hosts",
    r"vault:\Windows\win.ini",
    r"vault:C:\x",
    "vault::/etc/hosts",
    "{{HOME/x",
    "{{}}",
    "{{home}}",
    "{{env.X}}",
    "ftp://x/y",
    "mailto:a@b",
    "s3://b/k",
    "file://foo/bar",
    "file://",
    "C:x",
    r"\\?\C:\x",
    "@",
    "&",
    "^",
    "@/",
    "%",
    "%%x",
    "!x",
    "~bob/x",
];

#[test]
fn every_supported_kind_is_well_formed_on_every_host() {
    for raw in WELL_FORMED {
        if let Err(error) = FileReference::new(raw) {
            panic!("`{raw:?}` should be well-formed: {error}");
        }
    }
}

#[test]
fn malformed_references_are_rejected_by_construction() {
    for raw in MALFORMED {
        assert!(FileReference::new(raw).is_err(), "`{raw:?}` should be malformed");
    }
}

#[test]
fn the_new_structural_checks_are_syntax_errors() {
    for raw in [
        "a\0b.md",
        "https://",
        "https:///x",
        "https://exa mple.com/x",
        "%https://example.com/x",
        "vault:",
        "vault::",
        "vault:/etc/hosts",
    ] {
        assert!(
            matches!(FileReference::new(raw), Err(FileReferenceError::InvalidSyntax(_))),
            "`{raw:?}` should be an InvalidSyntax error"
        );
    }
}

#[test]
fn a_url_error_names_the_missing_host() {
    let message = FileReference::new("https://").unwrap_err().to_string();
    assert!(message.contains("needs a host"), "{message}");
    let message = FileReference::new("vault:/etc/hosts").unwrap_err().to_string();
    assert!(message.contains("must be relative"), "{message}");
}
