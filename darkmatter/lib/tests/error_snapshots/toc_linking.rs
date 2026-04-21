//! `TocLinkingError` variant snapshots.

use std::io;

use darkmatter::markdown::compose::TocLinkingError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn parse_directive_shows_syntax_hint() {
    let err = TocLinkingError::ParseDirective {
        line: 2,
        message: "unexpected token".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TocLinkingError",
            "directive parse failed",
            "2",
            "unexpected token",
            "::toc-linking",
        ],
    );
}

#[test]
fn invalid_cleanup_service_enumerates_valid_values() {
    let err = TocLinkingError::InvalidCleanupService {
        service: "bogus".into(),
        line: 4,
    };
    let out = render(&err);
    for name in [
        "emoji_leader",
        "emoji_trailing",
        "emoji",
        "number",
        "capitalize",
    ] {
        assert!(
            out.contains(name),
            "expected valid-values list to include `{name}`; got:\n{out}"
        );
    }
    assert_contains_all(
        &out,
        &["TocLinkingError", "invalid cleanup service", "bogus"],
    );
}

#[test]
fn invalid_level_shows_range() {
    let err = TocLinkingError::InvalidLevel {
        level: "9".into(),
        line: 3,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TocLinkingError", "invalid heading level", "9", "1", "6"],
    );
}

#[test]
fn file_not_found_hints_at_fallback_chain() {
    let err = TocLinkingError::FileNotFound {
        path: "./missing.md".into(),
        line: 6,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TocLinkingError",
            "file not found",
            "./missing.md",
            "fallback",
        ],
    );
}

#[test]
fn invalid_glob_shows_pattern_and_message() {
    let err = TocLinkingError::InvalidGlob {
        pattern: "**{".into(),
        line: 9,
        message: "unbalanced braces".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TocLinkingError",
            "invalid glob pattern",
            "**{",
            "unbalanced braces",
        ],
    );
}

#[test]
fn io_error_shows_kind() {
    let err = TocLinkingError::Io(io::Error::new(io::ErrorKind::NotFound, "nope"));
    let out = render(&err);
    assert_contains_all(&out, &["TocLinkingError", "I/O error", "NotFound"]);
}
