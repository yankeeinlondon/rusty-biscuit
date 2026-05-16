//! `LinkError` variant snapshots.

use darkmatter::render::link::LinkError;
use darkmatter::render::stylesheet::{StylesheetBlockError, StylesheetError};

use crate::helpers::{assert_contains_all, render, test_ctx};

#[test]
fn empty_href_has_actionable_hint() {
    let err = LinkError::EmptyHref;
    let out = render(&err);
    assert_contains_all(&out, &["LinkError", "empty href", "Link::new"]);
    insta::assert_snapshot!("empty_href", out);
}

#[test]
fn unrecognized_format_mentions_html_and_markdown() {
    let err = LinkError::UnrecognizedFormat;
    let out = render(&err);
    assert_contains_all(&out, &["LinkError", "unrecognized link format"]);
    insta::assert_snapshot!("unrecognized_format", out);
}

#[test]
fn malformed_html_includes_message() {
    let err = LinkError::MalformedHtml("missing </a>".into());
    let out = render(&err);
    assert_contains_all(&out, &["LinkError", "malformed HTML link", "missing </a>"]);
    insta::assert_snapshot!("malformed_html", out);
}

#[test]
fn malformed_markdown_includes_message_input_and_caret() {
    let err = LinkError::MalformedMarkdown {
        ctx: Box::new(test_ctx("[label] target\n", "doc.md")),
        message: "expected ( after display text".into(),
        caret: Some(8),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "LinkError",
            "malformed markdown link",
            "expected ( after display text",
            "[label] target",
            "^",
        ],
    );
    insta::assert_snapshot!("malformed_markdown_with_context", out);
}

#[test]
fn malformed_markdown_without_context_still_renders_message() {
    let err = LinkError::from("missing closing bracket");
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "LinkError",
            "malformed markdown link",
            "missing closing bracket",
        ],
    );
    insta::assert_snapshot!("malformed_markdown_without_context", out);
}

#[test]
fn missing_href_has_href_hint() {
    let err = LinkError::MissingHref;
    let out = render(&err);
    assert_contains_all(&out, &["LinkError", "missing href", "href"]);
    insta::assert_snapshot!("missing_href", out);
}

#[test]
fn invalid_style_delegates_stylesheet_block() {
    let inner = StylesheetError::InvalidInteger {
        value: "abc".into(),
    };
    let err = LinkError::InvalidStyle(StylesheetBlockError(inner));
    let out = render(&err);
    assert_contains_all(&out, &["StylesheetError", "invalid integer value", "abc"]);
    insta::assert_snapshot!("invalid_style_delegates", out);
}

#[test]
fn invalid_target_lists_accepted_values() {
    let err = LinkError::InvalidTarget {
        value: "_parent_".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "LinkError",
            "invalid target",
            "_parent_",
            "_self",
            "_blank",
            "_parent",
            "_top",
        ],
    );
    insta::assert_snapshot!("invalid_target", out);
}
