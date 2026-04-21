//! `NormalizationError` variant snapshots.

use darkmatter::markdown::normalize::{HeadingLevel, NormalizationError};

use crate::helpers::{assert_contains_all, render};

#[test]
fn level_overflow_shows_would_become_and_safe_target() {
    let err = NormalizationError::LevelOverflow {
        target: HeadingLevel::H4,
        affected_count: 3,
        deepest_title: "Deep Section".into(),
        would_become: 8,
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "NormalizationError",
            "heading level overflow",
            "H4",
            "3",
            "Deep Section",
            "H8",
            "H2",
        ],
    );
}

#[test]
fn validation_failed_shows_message_and_hint() {
    let err = NormalizationError::ValidationFailed("multiple H1 headings found".into());
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "NormalizationError",
            "validation failed",
            "multiple H1 headings found",
        ],
    );
}
