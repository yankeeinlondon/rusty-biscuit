//! `ImageRefError` variant snapshots.

use darkmatter::render::image_ref::ImageRefError;
use darkmatter::render::stylesheet::StylesheetError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn empty_source_has_actionable_hint() {
    let err = ImageRefError::EmptySource;
    let out = render(&err);
    assert_contains_all(&out, &["ImageRefError", "empty source", "ImageRef::new"]);
    insta::assert_snapshot!("empty_source", out);
}

#[test]
fn missing_source_mentions_src_and_srcset() {
    let err = ImageRefError::MissingSource;
    let out = render(&err);
    assert_contains_all(&out, &["ImageRefError", "missing source", "src", "srcset"]);
    insta::assert_snapshot!("missing_source", out);
}

#[test]
fn unrecognized_format_hints_at_markdown_and_html() {
    let err = ImageRefError::UnrecognizedFormat;
    let out = render(&err);
    assert_contains_all(&out, &["ImageRefError", "unrecognized image format"]);
    insta::assert_snapshot!("unrecognized_format", out);
}

#[test]
fn malformed_html_includes_message() {
    let err = ImageRefError::MalformedHtml("missing end of tag".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ImageRefError",
            "malformed HTML image",
            "missing end of tag",
        ],
    );
    insta::assert_snapshot!("malformed_html", out);
}

#[test]
fn malformed_markdown_includes_message_input_and_caret() {
    let err = ImageRefError::MalformedMarkdown {
        message: "expected ( after alt".into(),
        input: Some("![alt] image.png".into()),
        caret: Some(7),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ImageRefError",
            "malformed markdown image",
            "expected ( after alt",
        ],
    );
    assert_contains_all(&out, &["![alt] image.png", "^"]);
    insta::assert_snapshot!("malformed_markdown_with_context", out);
}

#[test]
fn malformed_markdown_without_context_still_renders_message() {
    let err = ImageRefError::from("missing closing bracket");
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ImageRefError",
            "malformed markdown image",
            "missing closing bracket",
        ],
    );
    insta::assert_snapshot!("malformed_markdown_without_context", out);
}

#[test]
fn invalid_style_delegates_stylesheet_block() {
    let inner = StylesheetError::InvalidColor {
        value: "notacolor".into(),
    };
    let err = ImageRefError::InvalidStyle(inner);
    let out = render(&err);
    // InvalidStyle delegates through to StylesheetError.
    assert_contains_all(
        &out,
        &["StylesheetError", "invalid color value", "notacolor"],
    );
    insta::assert_snapshot!("invalid_style_delegates", out);
}

#[test]
fn invalid_decoding_lists_accepted_values() {
    let err = ImageRefError::InvalidDecoding {
        value: "eager".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ImageRefError",
            "invalid decoding",
            "eager",
            "sync",
            "async",
            "auto",
        ],
    );
    insta::assert_snapshot!("invalid_decoding", out);
}

#[test]
fn invalid_fetch_priority_lists_accepted_values() {
    let err = ImageRefError::InvalidFetchPriority {
        value: "mid".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ImageRefError",
            "invalid fetch priority",
            "mid",
            "high",
            "low",
            "auto",
        ],
    );
    insta::assert_snapshot!("invalid_fetch_priority", out);
}

#[test]
fn invalid_loading_lists_accepted_values() {
    let err = ImageRefError::InvalidLoading {
        value: "never".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["ImageRefError", "invalid loading", "never", "eager", "lazy"],
    );
    insta::assert_snapshot!("invalid_loading", out);
}

#[test]
fn invalid_referrer_policy_suggests_close_match() {
    let err = ImageRefError::InvalidReferrerPolicy {
        value: "noreferer".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ImageRefError",
            "invalid referrer policy",
            "noreferer",
            "Did you mean:",
            "no-referrer",
        ],
    );
    insta::assert_snapshot!("invalid_referrer_policy_suggests", out);
}

#[test]
fn invalid_referrer_policy_lists_all_accepted_values_when_no_match() {
    let err = ImageRefError::InvalidReferrerPolicy {
        value: "xyzzy".into(),
    };
    let out = render(&err);
    // With no close match the did-you-mean line is omitted but the full list
    // is still present in the hint.
    for accepted in [
        "no-referrer",
        "no-referrer-when-downgrade",
        "origin",
        "same-origin",
        "unsafe-url",
    ] {
        assert!(
            out.contains(accepted),
            "expected accepted value `{accepted}` in output; got:\n{out}"
        );
    }
    insta::assert_snapshot!("invalid_referrer_policy_no_match", out);
}
