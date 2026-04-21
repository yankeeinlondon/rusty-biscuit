//! `ReferenceError` variant snapshots.

use std::io;

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::reference::ReferenceError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn parse_directive_shows_line_and_syntax_hint() {
    let err = ReferenceError::ParseDirective {
        line: 5,
        message: "unexpected end".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ReferenceError",
            "directive parse failed",
            "5",
            "unexpected end",
            "::file",
        ],
    );
}

#[test]
fn missing_source_context_shows_reference_and_line() {
    let err = ReferenceError::MissingSourceContext {
        reference: "./sibling.md".into(),
        line: 3,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ReferenceError",
            "missing source context",
            "./sibling.md",
            "3",
        ],
    );
}

#[test]
fn validation_shows_message() {
    let err = ReferenceError::Validation("orphan node".into());
    let out = render(&err);
    assert_contains_all(&out, &["ReferenceError", "validation failed", "orphan node"]);
}

#[test]
fn compose_delegates_inner_block() {
    let err = ReferenceError::Compose(Box::new(MarkdownError::Transform("stalled".into())));
    let out = render(&err);
    assert_contains_all(&out, &["MarkdownError", "transform failed", "stalled"]);
}

#[test]
fn io_error_shows_kind() {
    let err = ReferenceError::Io(io::Error::new(io::ErrorKind::NotFound, "file gone"));
    let out = render(&err);
    assert_contains_all(&out, &["ReferenceError", "I/O error", "NotFound"]);
}

#[test]
fn url_error_has_scheme_hint() {
    let parse_error = url::Url::parse("not a url").unwrap_err();
    let err = ReferenceError::Url(parse_error);
    let out = render(&err);
    assert_contains_all(&out, &["ReferenceError", "URL parse error", "https://"]);
}
