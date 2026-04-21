//! `NormalizationError` variant snapshots.

use darkmatter::markdown::normalize::{
    HeadingLevel, NormalizationError, StructureIssue, StructureIssueKind,
};

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
    let err = NormalizationError::ValidationFailed(vec![
        StructureIssue::new(
            StructureIssueKind::MultipleH1,
            String::new(),
            0,
            "multiple H1 headings found".into(),
        ),
        StructureIssue::new(
            StructureIssueKind::SkippedLevel,
            "Details".into(),
            12,
            "Heading 'Details' skips from H2 to H4 (skipped 1 level(s))".into(),
        )
        .with_suggestion("Add an H3 heading before `Details`.".into()),
    ]);
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "NormalizationError",
            "validation failed",
            "2 issue(s)",
            "multiple H1 headings found",
            "skipped level",
            "Suggestion:",
        ],
    );
}
