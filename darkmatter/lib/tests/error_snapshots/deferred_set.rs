//! `DeferredSetError` variant snapshots.

use darkmatter::markdown::compose::transclusion::DeferredSetError;

use crate::helpers::{assert_contains_all, render};

#[test]
fn invalid_assignment_shows_raw_and_reason() {
    let err = DeferredSetError::InvalidAssignment {
        raw: "\"not json\"".into(),
        reason: "expected JSON5 object".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "DeferredSetError",
            "invalid set assignment",
            "\"not json\"",
            "expected JSON5 object",
            "set=",
        ],
    );
}

#[test]
fn reassigned_property_shows_property_name() {
    let err = DeferredSetError::ReassignedProperty {
        name: "title".into(),
    };
    let out = render(&err);
    assert_contains_all(
        &out,
        &[
            "DeferredSetError",
            "reassigned set property",
            "title",
            "set.NAME=",
        ],
    );
}
