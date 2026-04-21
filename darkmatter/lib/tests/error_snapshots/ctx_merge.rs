//! `CtxMergeError` variant snapshots.

use darkmatter::markdown::compose::context::merge::CtxMergeError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn invalid_user_ctx_renders_expected_kind_and_hint() {
    let err = CtxMergeError::InvalidUserCtx {
        kind: "string".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "CtxMergeError",
            "invalid user ctx",
            "string",
            "JSON object",
            "--allow-override",
        ],
    );
}
