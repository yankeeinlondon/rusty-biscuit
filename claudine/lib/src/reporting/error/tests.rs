//! The D9 persistence-boundary contract for [`SyncFailure`].
//!
//! Every test here is about *identity survival*: what the report record still
//! knows about a failure after the concrete Rust error is gone.

use super::*;
use crate::diagnostics::{DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, next_registered_cause};

fn sqlite_ingest_error() -> IngestError {
    IngestError {
        source_file: "/logs/2026-07-20.jsonl".to_string(),
        line_number: 0,
        // The shape the retired allow entry called double-flattened: a
        // `ClaudineError` that already wraps a lower `rusqlite::Error`.
        source: ClaudineError::Sqlite(rusqlite::Error::QueryReturnedNoRows),
    }
}

/// The defect the review named. Before the fix both levels collapsed into one
/// `message` string; now the outer classification and the inner typed cause are
/// separately readable off the persisted record.
#[test]
fn a_wrapped_sqlite_failure_persists_both_levels_not_one_string() {
    let failure = SyncFailure::from(sqlite_ingest_error());
    let snapshot = failure.diagnostic.expect("a new record carries a snapshot");

    // Outer: the reporting boundary's own classification.
    assert_eq!(snapshot.code, "io.read_failed");
    assert_eq!(snapshot.category, "io");
    assert_eq!(snapshot.detail["path"], "/logs/2026-07-20.jsonl");

    // Inner: the typed `ClaudineError` survived to the projection point.
    let cause = snapshot.cause.expect("the typed cause is projected");
    assert_eq!(cause.code, "io.read_failed");
    assert!(
        cause.message.contains("SQLite"),
        "the cause kept its own identity, got: {}",
        cause.message
    );
}

/// The wrapper is `Semantic`, so it — not its cause — is what the selection
/// walk returns for itself. If it ever became `Transparent` the snapshot would
/// silently start speaking with the cause's identity.
#[test]
fn the_ingest_wrapper_is_the_selected_diagnostic_and_its_cause_is_reachable() {
    let error = sqlite_ingest_error();
    let selected = crate::diagnostics::select_effective_diagnostic(&error)
        .expect("the wrapper is in the discovery registry");
    let diagnostic = selected.diagnostic().expect("a Claudine diagnostic");

    assert_eq!(
        diagnostic as *const dyn Diagnostic as *const (),
        &error as *const IngestError as *const (),
        "selection returned something other than the wrapper itself"
    );
    assert!(next_registered_cause(diagnostic).is_some());
}

/// A per-line JSON failure keeps its own catalog identity as the cause, so the
/// record distinguishes "the file is unreadable" from "line 12 is malformed".
#[test]
fn a_malformed_line_persists_its_parse_identity_as_the_cause() {
    let parse_error = serde_json::from_str::<serde_json::Value>("{oops").unwrap_err();
    let failure = SyncFailure::from(IngestError {
        source_file: "/logs/a.jsonl".to_string(),
        line_number: 12,
        source: ClaudineError::from(parse_error),
    });

    assert_eq!(failure.line_number, 12);
    let snapshot = failure.diagnostic.expect("a snapshot");
    assert_eq!(snapshot.code, "io.read_failed");
    assert_eq!(
        snapshot.cause.expect("the parse cause is projected").code,
        "config.invalid"
    );
}

/// D9: the snapshot is versioned, and the human `message` sits *beside* the
/// facets rather than replacing them.
#[test]
fn the_record_carries_a_versioned_snapshot_alongside_its_prose() {
    let failure = SyncFailure::from(sqlite_ingest_error());
    assert!(
        failure.message.contains("SQLite"),
        "the human line is still populated: {}",
        failure.message
    );
    assert_eq!(
        failure.diagnostic.unwrap().schema_version,
        DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION
    );
}
