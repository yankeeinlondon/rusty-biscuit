//! `LinkError` variant snapshots.

use darkmatter::render::link::LinkError;
use darkmatter::render::stylesheet::StylesheetError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn empty_href_has_actionable_hint() {
    let err = LinkError::EmptyHref;
    let out = render(&err);
    assert_contains_all(&out, &["LinkError", "empty href", "Link::new"]);
}

#[test]
fn unrecognized_format_mentions_html_and_markdown() {
    let err = LinkError::UnrecognizedFormat;
    let out = render(&err);
    assert_contains_all(&out, &["LinkError", "unrecognized link format"]);
}

#[test]
fn malformed_html_includes_message() {
    let err = LinkError::MalformedHtml("missing </a>".into());
    let out = render(&err);
    assert_contains_all(&out, &["LinkError", "malformed HTML link", "missing </a>"]);
}

#[test]
fn malformed_markdown_includes_message() {
    let err = LinkError::MalformedMarkdown("expected ( after display text".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "LinkError",
            "malformed markdown link",
            "expected ( after display text",
        ],
    );
}

#[test]
fn missing_href_has_href_hint() {
    let err = LinkError::MissingHref;
    let out = render(&err);
    assert_contains_all(&out, &["LinkError", "missing href", "href"]);
}

#[test]
fn invalid_style_delegates_stylesheet_block() {
    let inner = StylesheetError::InvalidInteger {
        value: "abc".into(),
    };
    let err = LinkError::InvalidStyle(inner);
    let out = render(&err);
    assert_contains_all(&out, &["StylesheetError", "invalid integer value", "abc"]);
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
}
