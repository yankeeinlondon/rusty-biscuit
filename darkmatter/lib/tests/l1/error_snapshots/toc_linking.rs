//! `TocLinkingError` variant snapshots.

use std::io;

use darkmatter::markdown::compose::TocLinkingError;

use super::helpers::{assert_contains_all, render};

#[test]
fn parse_directive_shows_syntax_hint() {
    let err = TocLinkingError::ParseDirective {
        line: 2,
        message: "unexpected token".into(),
        document: None,
        authored: None,
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
        document: None,
        authored: None,
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
        &[
            "TocLinkingError",
            "invalid cleanup service",
            "bogus",
            "Strip leading emoji",
        ],
    );
}

#[test]
fn invalid_level_shows_range() {
    let err = TocLinkingError::InvalidLevel {
        level: "9".into(),
        line: 3,
        document: None,
        authored: None,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TocLinkingError", "invalid heading level", "9", "1", "6"],
    );
}

#[test]
fn unresolved_target_names_its_failure_and_hints_at_fallback_chain() {
    let err = TocLinkingError::Unresolved {
        path: "./missing.md".into(),
        line: 6,
        failure: Some(biscuit_file::ResolutionFailure::NoMatch),
        glob_hint: None,
        document: None,
        authored: None,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TocLinkingError",
            "file reference failure",
            "./missing.md",
            "failure: no-match",
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
        document: None,
        authored: None,
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

/// R11-F2: a directive whose authored line is unproven carries line `0`; its
/// status block omits the `Line:` row rather than printing `Line: 0`, for
/// every line-carrying `::toc-linking` and `::file-links` variant, while a
/// known line keeps its row.
#[test]
fn an_unknown_line_shows_no_line_row() {
    use darkmatter::markdown::compose::FileLinksError;
    let rendered = |line: usize| -> Vec<String> {
        let toc = [
            TocLinkingError::ParseDirective { line, message: "bad".into(), document: None, authored: None },
            TocLinkingError::InvalidCleanupService { service: "bogus".into(), line, document: None, authored: None },
            TocLinkingError::InvalidLevel { level: "9".into(), line, document: None, authored: None },
            TocLinkingError::Unresolved {
                path: "./missing.md".into(),
                line,
                failure: None,
                glob_hint: None,
                document: None,
                authored: None,
            },
            TocLinkingError::InvalidGlob { pattern: "[".into(), line, message: "bad".into(), document: None, authored: None },
        ];
        let links = [
            FileLinksError::ParseDirective { line, message: "bad".into(), document: None, authored: None },
            FileLinksError::InvalidDepth { value: "two".into(), line, document: None, authored: None },
            FileLinksError::MissingSourceContext { line, document: None, authored: None },
            FileLinksError::TargetNotFound { path: "./gone".into(), line, document: None, authored: None },
            FileLinksError::TargetNotDirectory { path: "./a.md".into(), line, document: None, authored: None },
            FileLinksError::Unreadable {
                path: "./locked".into(),
                line,
                source: io::Error::other("denied"),
                document: None,
                authored: None,
            },
        ];
        toc.iter().map(|err| render(err)).chain(links.iter().map(|err| render(err))).collect()
    };
    for out in rendered(0) {
        assert!(!out.contains("Line:"), "an unknown line has no row:\n{out}");
    }
    for out in rendered(7) {
        assert!(out.contains("Line: 7"), "a known line keeps its row:\n{out}");
    }
}
