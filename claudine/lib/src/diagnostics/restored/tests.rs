use std::error::Error as StdError;

use serde_json::json;

use super::*;
use crate::composition::CompositionError;
use crate::diagnostics::{DiagnosticRole, as_diagnostic, select_effective_diagnostic};

fn erase<'a>(error: &'a (dyn StdError + 'static)) -> &'a (dyn StdError + 'static) {
    error
}

/// A restored snapshot must re-project to the snapshot it came from. This is
/// the property that makes restoration lossless: whatever the early boundary
/// captured is what the late boundary hands to `err.*` and machine output.
#[test]
fn restoring_a_snapshot_is_a_fixed_point() {
    let snapshot =
        DiagnosticSnapshot::from_diagnostic(&CompositionError::FileNotFound("gone.md".into()));
    let restored = RestoredDiagnostic::new(snapshot.clone());

    assert_eq!(DiagnosticSnapshot::from_diagnostic(&restored), snapshot);
}

/// Every facet the snapshot carried is readable off the restored value — the
/// point of restoring rather than lifting `snapshot.message` into a report.
#[test]
fn restored_facets_match_the_snapshot() {
    let snapshot =
        DiagnosticSnapshot::from_diagnostic(&CompositionError::FileNotFound("gone.md".into()));
    let restored = RestoredDiagnostic::new(snapshot.clone());

    assert_eq!(restored.code(), snapshot.code);
    assert_eq!(restored.category().as_str(), snapshot.category);
    assert_eq!(restored.disposition().as_str(), snapshot.disposition);
    assert_eq!(restored.origin().as_str(), snapshot.origin);
    assert_eq!(Diagnostic::severity(&restored).as_str(), snapshot.severity);
    assert_eq!(restored.detail(), snapshot.detail);
    assert_eq!(restored.role(), DiagnosticRole::Semantic);
}

/// A snapshot whose facet strings are the catalog's own values for `code`.
///
/// Restoration re-derives the facets from the catalog row rather than trusting
/// the owned strings, so a fixture that disagrees with the catalog would be
/// testing the disagreement rather than the round trip.
fn catalog_snapshot(code: &str, message: &str, detail: serde_json::Value) -> DiagnosticSnapshot {
    let spec = code_spec(code).expect("fixture code is in the locked catalog");
    DiagnosticSnapshot {
        schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: spec.category.as_str().to_string(),
        code: spec.code.to_string(),
        disposition: spec.disposition.as_str().to_string(),
        origin: spec.origin.as_str().to_string(),
        severity: spec.severity().as_str().to_string(),
        detail,
        message: message.to_string(),
        cause: None,
    }
}

/// The one-level cause is restored too, so a re-projection carries the same
/// `cause` rather than dropping to `None`.
#[test]
fn the_one_level_cause_survives_restoration() {
    let inner = catalog_snapshot("io.read_failed", "inner", json!({ "path": "/tmp/x" }));
    let snapshot = DiagnosticSnapshot {
        cause: Some(DiagnosticCause {
            category: inner.category,
            code: inner.code,
            disposition: inner.disposition,
            origin: inner.origin,
            severity: inner.severity,
            detail: inner.detail,
            message: inner.message,
        }),
        ..catalog_snapshot("composition.failed", "outer", json!({ "message": "outer" }))
    };

    let restored = RestoredDiagnostic::new(snapshot.clone());
    let reprojected = DiagnosticSnapshot::from_diagnostic(&restored);

    assert_eq!(reprojected, snapshot);
    assert_eq!(
        reprojected.cause.expect("cause survives").code,
        "io.read_failed"
    );
}

/// The restoring boundary's framing reaches the human message without touching
/// any facet — the reason `with_context` exists instead of a second code.
#[test]
fn context_prefixes_the_message_but_not_the_facets() {
    let snapshot =
        DiagnosticSnapshot::from_diagnostic(&CompositionError::FileNotFound("gone.md".into()));
    let restored = RestoredDiagnostic::new(snapshot.clone()).with_context("cannot continue");

    assert_eq!(restored.to_string(), format!("cannot continue: {}", snapshot.message));
    assert_eq!(restored.code(), snapshot.code);
    assert_eq!(restored.detail(), snapshot.detail);
}

/// The whole reason restoration exists: the value must be discoverable by the
/// walk the CLI renders through and `err.*` classifies through.
#[test]
fn a_restored_diagnostic_is_discoverable_and_selectable() {
    let snapshot =
        DiagnosticSnapshot::from_diagnostic(&CompositionError::FileNotFound("gone.md".into()));
    let restored = RestoredDiagnostic::new(snapshot.clone());

    assert_eq!(
        as_diagnostic(erase(&restored)).map(|d| d.code()),
        Some("composition.invalid_file_reference")
    );

    let selected = select_effective_diagnostic(erase(&restored)).expect("a diagnostic is selected");
    assert_eq!(selected.diagnostic().map(|d| d.code()), Some(snapshot.code.as_str()));
    assert!(
        selected
            .block_error()
            .report_block_error_optimistic(Some(100))
            .contains("gone.md"),
        "the restored block must render the snapshot's message"
    );
}

/// A code this build's catalog does not know still restores — facets degrade to
/// the fallback row while `detail` and `message` carry through untouched.
#[test]
fn an_unknown_code_degrades_instead_of_failing() {
    let snapshot = DiagnosticSnapshot {
        schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: "future".to_string(),
        code: "future.unheard_of".to_string(),
        disposition: "correctable".to_string(),
        origin: "author".to_string(),
        severity: "error".to_string(),
        detail: json!({ "anything": 1 }),
        message: "from a newer producer".to_string(),
        cause: None,
    };

    let restored = RestoredDiagnostic::new(snapshot.clone());

    assert_eq!(restored.code(), UNKNOWN_CODE_FALLBACK);
    assert_eq!(restored.detail(), snapshot.detail);
    assert_eq!(restored.to_string(), snapshot.message);
}
