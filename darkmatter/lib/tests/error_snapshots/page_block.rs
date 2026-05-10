//! `PageBlockError` variant snapshots.

use std::path::PathBuf;
use std::sync::Arc;

use biscuit_terminal::errors::SourceContext;
use darkmatter::markdown::compose::conditions::ConditionError;
use darkmatter::markdown::compose::page_blocks::PageBlockError;

use crate::helpers::{assert_contains_all, render, render_with_ansi, test_ctx_lines};

#[test]
fn parse_directive_shows_line_and_hint() {
    let err = PageBlockError::ParseDirective {
        ctx: Box::new(test_ctx_lines(15, "test.md")),
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
    let err = PageBlockError::UnmatchedEnd {
        ctx: Box::new(test_ctx_lines(25, "test.md")),
        line: 20,
    };
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
        ctx: Box::new(ctx),
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

/// Verifies the canonical variant emits the expected ANSI escape sequences.
///
/// Spec §3.5 requires:
/// - linked file path rendered as an OSC 8 hyperlink (`\x1b]8;;file://...`)
/// - the literal `::end-block` directive in the hint wrapped in SGR inverse
///   (`\x1b[7m`).
///
/// The plain-text [`render`] helper strips these so this test uses the
/// ANSI-preserving variant. Escape-sequence strings are matched via `contains`
/// rather than snapshotted: the byte-level form is noisy and drifts.
#[test]
fn unterminated_block_emits_ansi_styling() {
    let content = "---\ntitle: Sample\n---\n# Heading\n\nIntro paragraph.\n\n::block when=\"condition\"\nbody line one\nbody line two\n";
    let ctx = SourceContext::new(
        PathBuf::from("/tmp/sample.md"),
        PathBuf::from("sample.md"),
        Arc::from(content),
    );
    let err = PageBlockError::UnterminatedBlock {
        ctx: Box::new(ctx),
        opening_line: 8,
        opening_text: "::block when=\"condition\"".to_string(),
    };
    let out = render_with_ansi(&err);

    // OSC 8 hyperlink for the linked file path.
    assert!(
        out.contains("\x1b]8;;file://"),
        "expected OSC 8 hyperlink for linked path; got:\n{out:?}"
    );

    // SGR 7 (inverse) for the highlighted ::end-block token in the hint.
    assert!(
        out.contains("\x1b[7m"),
        "expected SGR inverse (\\x1b[7m) around ::end-block hint token; got:\n{out:?}"
    );
}

#[test]
fn condition_delegates_inner_block() {
    let err = PageBlockError::Condition(Box::new(ConditionError::Parse {
        ctx: Box::new(test_ctx_lines(8, "test.md")),
        expr: "a &&& b".into(),
        line: 4,
        message: "unexpected token".into(),
        span: 3..4,
    }));
    let out = render(&err);
    assert_contains_all(
        &out,
        &["ConditionError", "parse failed", "a &&& b", "4", "^"],
    );
    insta::assert_snapshot!("condition_delegates", out);
}
