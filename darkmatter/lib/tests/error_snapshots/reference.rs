//! `ReferenceError` variant snapshots.

use std::io;

use darkmatter::markdown::MarkdownError;
use darkmatter::markdown::reference::ReferenceError;

use crate::helpers::{assert_contains_all, render, test_ctx_lines};

#[test]
fn parse_directive_shows_line_and_syntax_hint() {
    let err = ReferenceError::ParseDirective {
        ctx: Box::new(test_ctx_lines(8, "docs/root.md")),
        line: 5,
        message: "unexpected end".into(),
        directive_text: "::file ./broken.md when=".into(),
        caret_col: Some(27),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ReferenceError",
            "directive parse failed",
            "5",
            "unexpected end",
            "docs/root.md",
            "::file ./broken.md when=",
            "Column 27",
            "::file",
        ],
    );
    insta::assert_snapshot!("parse_directive", out);
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
    insta::assert_snapshot!("missing_source_context", out);
}

#[test]
fn validation_shows_message() {
    let err = ReferenceError::Validation("orphan node".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &["ReferenceError", "validation failed", "orphan node"],
    );
    insta::assert_snapshot!("validation", out);
}

#[test]
fn compose_delegates_inner_block() {
    let err = ReferenceError::Compose(Box::new(MarkdownError::Transform("stalled".into())));
    let out = render(&err);
    assert_contains_all(&out, &["MarkdownError", "transform failed", "stalled"]);
    insta::assert_snapshot!("compose_delegates", out);
}

#[test]
fn io_error_shows_kind() {
    let err = ReferenceError::Io(io::Error::new(io::ErrorKind::NotFound, "file gone"));
    let out = render(&err);
    assert_contains_all(&out, &["ReferenceError", "I/O failure", "file gone"]);
    insta::assert_snapshot!("io_error", out);
}

#[test]
fn url_error_has_scheme_hint() {
    let parse_error = url::Url::parse("not a url").unwrap_err();
    let err = ReferenceError::Url(parse_error);
    let out = render(&err);
    assert_contains_all(&out, &["ReferenceError", "URL parse failure", "https://"]);
    insta::assert_snapshot!("url_error", out);
}
