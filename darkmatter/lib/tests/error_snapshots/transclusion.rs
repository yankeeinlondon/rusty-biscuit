//! `TransclusionError` variant snapshots.

use std::io;
use std::path::PathBuf;

use darkmatter::markdown::compose::TransclusionError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn parse_directive_has_syntax_hint() {
    let err = TransclusionError::ParseDirective {
        line: 3,
        message: "unexpected token".into(),
        caret_col: Some(8),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "directive parse failed",
            "3",
            "unexpected token",
            "::file",
        ],
    );
}

#[test]
fn invalid_reference_shows_reference() {
    use darkmatter::markdown::compose::transclusion::DirectiveKind;
    let err = TransclusionError::InvalidReference {
        reference: "//weird".into(),
        line: 2,
        source_file: PathBuf::from("doc.md"),
        directive_kind: DirectiveKind::File,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TransclusionError", "invalid reference", "//weird", "2", "doc.md", "::file"],
    );
}

#[test]
fn missing_source_context_has_hint() {
    let err = TransclusionError::MissingSourceContext {
        reference: "./sibling.md".into(),
        line: 4,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "missing source context",
            "./sibling.md",
            "4",
        ],
    );
}

#[test]
fn unsupported_reference_type_shows_value() {
    let err = TransclusionError::UnsupportedReferenceType {
        reference: "ftp://host/file".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "unsupported reference type",
            "ftp://host/file",
        ],
    );
}

#[test]
fn unsupported_file_type_shows_path() {
    let err = TransclusionError::UnsupportedFileType {
        path: PathBuf::from("./thing.xyz"),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TransclusionError", "unsupported file type", "./thing.xyz"],
    );
}

#[test]
fn non_text_code_source_shows_path() {
    let err = TransclusionError::NonTextCodeSource {
        path: PathBuf::from("./binary.bin"),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TransclusionError", "non-text code source", "./binary.bin"],
    );
}

#[test]
fn cycle_detected_lists_chain() {
    let err = TransclusionError::CycleDetected {
        chain: vec![
            (PathBuf::from("a.md"), 3),
            (PathBuf::from("b.md"), 7),
            (PathBuf::from("a.md"), 3),
        ],
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TransclusionError", "cycle detected", "a.md", "b.md", ":line 3", ":line 7"],
    );
}

#[test]
fn max_depth_exceeded_shows_limit() {
    let err = TransclusionError::MaxDepthExceeded { max_depth: 8 };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TransclusionError", "max recursion depth exceeded", "8"],
    );
}

#[test]
fn condition_eval_shows_expr_and_state_hint() {
    let err = TransclusionError::ConditionEval {
        expr: "length(items) > 0".into(),
        line: 10,
        message: "items not found".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "condition evaluation failed",
            "length(items) > 0",
            "10",
            "items not found",
        ],
    );
}

#[test]
fn condition_parse_lists_operators_and_helpers() {
    let err = TransclusionError::ConditionParse {
        expr: "a &&& b".into(),
        line: 4,
        message: "unexpected token".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "condition parse failed",
            "a &&& b",
            "4",
            "&&",
            "HasKey",
        ],
    );
}

#[test]
fn relevel_shows_message() {
    let err = TransclusionError::Relevel("would push past H6".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &["TransclusionError", "re-leveling failed", "would push past H6"],
    );
}

#[test]
fn url_execution_disabled_shows_url() {
    let err = TransclusionError::UrlExecutionDisabled {
        url: "https://example.com/doc.md".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "remote transclusion disabled",
            "https://example.com/doc.md",
        ],
    );
}

#[test]
fn invalid_frontmatter_assignment_shows_cta() {
    let err = TransclusionError::InvalidFrontmatterAssignment {
        line: 5,
        raw: "bogus".into(),
        reason: "expected JSON5 object".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "invalid frontmatter assignment",
            "5",
            "bogus",
            "expected JSON5 object",
            "--allow-invalid-frontmatter-assignment",
        ],
    );
}

#[test]
fn invalid_reassigned_frontmatter_property_shows_cta() {
    let err = TransclusionError::InvalidReassignedFrontmatterProperty {
        line: 6,
        name: "title".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "reassigned frontmatter property",
            "6",
            "title",
            "--allow-reassigned-frontmatter-property",
        ],
    );
}

#[test]
fn io_error_shows_kind() {
    let err = TransclusionError::Io(io::Error::new(io::ErrorKind::NotFound, "file gone"));
    let out = render(&err);
    assert_contains_all(&out, &["TransclusionError", "I/O error", "NotFound"]);
}

#[test]
fn url_parse_error_has_scheme_hint() {
    let parse_error = url::Url::parse("not a url").unwrap_err();
    let err = TransclusionError::UrlParse(parse_error);
    let out = render(&err);
    assert_contains_all(&out, &["TransclusionError", "URL parse error", "https://"]);
}

#[test]
fn json_error_includes_position() {
    let parse_error = serde_json::from_str::<serde_json::Value>("{ bogus }").unwrap_err();
    let err = TransclusionError::Json(parse_error);
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "TransclusionError",
            "directive option parse error",
            "line",
            "column",
        ],
    );
}
