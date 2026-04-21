//! `PageBlockError` variant snapshots.

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
}

#[test]
fn unmatched_end_hints_at_block() {
    let err = PageBlockError::UnmatchedEnd { line: 20 };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "PageBlockError",
            "unmatched ::end-block",
            "20",
            "::block",
        ],
    );
}

#[test]
fn unterminated_block_shows_opening_line() {
    let err = PageBlockError::UnterminatedBlock {
        line: 7,
        opening_text: "::block when=\"condition\"".to_string(),
        file_ends_at_line: 42,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "PageBlockError",
            "unterminated ::block",
            "7",
            "::block when=\"condition\"",
            "42",
            "::end-block",
        ],
    );
}

#[test]
fn condition_delegates_inner_block() {
    let err = PageBlockError::Condition(ConditionError::Parse {
        expr: "a &&& b".into(),
        line: 4,
        message: "unexpected token".into(),
    });
    let out = render(&err);
    assert_contains_all(
        &out,
        &["ConditionError", "parse failed", "a &&& b", "4"],
    );
}
