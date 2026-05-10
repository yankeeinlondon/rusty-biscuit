//! `PageBlockError` variant snapshots.

use std::path::PathBuf;
use std::sync::Arc;

use biscuit_terminal::errors::SourceContext;
use darkmatter::markdown::compose::conditions::ConditionError;
use darkmatter::markdown::compose::page_blocks::PageBlockError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn parse_directive_shows_line_and_hint() {
    let err = PageBlockError::ParseDirective {
        line: 12,
        message: "unexpected token".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "PageBlockError",
            "directive parse failed",
            "12",
            "unexpected token",
            "::block",
            "::end-block",
        ],
    );
    insta::assert_snapshot!("parse_directive", out);
}

#[test]
fn unmatched_end_hints_at_block() {
    let err = PageBlockError::UnmatchedEnd { line: 20 };
    let out = render(&err);
    assert_contains_all(
        &out,
        &["PageBlockError", "unmatched ::end-block", "20", "::block"],
    );
    insta::assert_snapshot!("unmatched_end", out);
}

#[test]
fn unterminated_block_shows_opening_line() {
    let content = "---\ntitle: Sample\n---\n# Heading\n\nIntro paragraph.\n\n::block when=\"condition\"\nbody line one\nbody line two\n";
    let ctx = SourceContext::new(
        PathBuf::from("/tmp/sample.md"),
        PathBuf::from("sample.md"),
        Arc::from(content),
    );
    let err = PageBlockError::UnterminatedBlock {
        ctx,
        opening_line: 8,
        opening_text: "::block when=\"condition\"".to_string(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "PageBlockError",
            "unterminated",
            "sample.md",
            "::block",
            "::end-block",
        ],
    );
    insta::assert_snapshot!("unterminated_block", out);
}

#[test]
fn condition_delegates_inner_block() {
    let err = PageBlockError::Condition(ConditionError::Parse {
        expr: "a &&& b".into(),
        line: 4,
        message: "unexpected token".into(),
        span: 3..4,
    });
    let out = render(&err);
    assert_contains_all(
        &out,
        &["ConditionError", "parse failed", "a &&& b", "4", "^"],
    );
    insta::assert_snapshot!("condition_delegates", out);
}
