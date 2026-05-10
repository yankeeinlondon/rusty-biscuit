//! `ConditionError` variant snapshots.

use darkmatter::markdown::compose::conditions::ConditionError;

use crate::helpers::{assert_contains_all, render, test_ctx_lines};

#[test]
fn parse_lists_operators_and_helpers() {
    let err = ConditionError::Parse {
        ctx: Box::new(test_ctx_lines(8, "test.md")),
        expr: "a &&& b".into(),
        line: 4,
        message: "unexpected token".into(),
        span: 3..4,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ConditionError",
            "parse failed",
            "a &&& b",
            "4",
            "near position 3",
            "^",
            "&&",
            "HasKey",
        ],
    );
    insta::assert_snapshot!("parse", out);
}

#[test]
fn eval_points_at_state() {
    let err = ConditionError::Eval {
        ctx: Box::new(test_ctx_lines(15, "test.md")),
        expr: "length(items) > 0".into(),
        line: 10,
        message: "items not found".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "ConditionError",
            "evaluation failed",
            "length(items) > 0",
            "10",
            "items not found",
        ],
    );
    insta::assert_snapshot!("eval", out);
}
