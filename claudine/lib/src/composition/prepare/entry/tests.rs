//! The stage matrix is the thing this phase exists to make checkable, so it is
//! asserted as a table rather than trusted as prose.

use super::*;

/// The `expect` in [`DocumentEntryReason::stages`] is only unreachable if every
/// variant has a row. A variant added without one would panic at runtime, on the
/// entry path, in production.
#[test]
fn stage_matrix_covers_every_entry_reason() {
    assert_eq!(
        STAGE_MATRIX.len(),
        DocumentEntryReason::ALL.len(),
        "the matrix has exactly one row per entry reason"
    );
    for reason in DocumentEntryReason::ALL {
        let rows = STAGE_MATRIX.iter().filter(|(r, _)| r == reason).count();
        assert_eq!(
            rows, 1,
            "{:?} must have exactly one row — no entry reason may fall through \
             to another's policy",
            reason
        );
        // Does not panic.
        let _ = reason.stages();
    }
}

/// R3's equivalence contract, stated as a table row: a proxy target runs the
/// same stages as a direct document. The *only* difference is that the target
/// must be read from disk, because the handoff commits to a document the
/// source may never have loaded.
#[test]
fn proxy_target_runs_the_same_stages_as_a_direct_document() {
    let direct = DocumentEntryReason::Direct.stages();
    let proxied = DocumentEntryReason::ProxyTarget.stages();

    assert_eq!(direct.emits_initialize, proxied.emits_initialize);
    assert_eq!(direct.full_validation, proxied.full_validation);
    assert_eq!(
        direct.loop_ownership, proxied.loop_ownership,
        "a proxied target recognizes its own `loop:` exactly as a direct \
         invocation does — this is the motivating bug"
    );
    assert_eq!(direct.source_basis, SourceBasis::CallerResolved);
    assert_eq!(
        proxied.source_basis,
        SourceBasis::FreshRead,
        "the one sanctioned difference"
    );
}

/// `initialize` runs once per active *document*, not once per attempt. A retry
/// or resume re-enters a document that has already initialized; only entering a
/// new active document emits it.
#[test]
fn only_a_new_active_document_emits_initialize() {
    let emitting: Vec<_> = DocumentEntryReason::ALL
        .iter()
        .filter(|r| r.stages().emits_initialize)
        .collect();
    assert_eq!(
        emitting,
        vec![
            &DocumentEntryReason::Direct,
            &DocumentEntryReason::ProxyTarget
        ],
        "exactly the two entries that make a document newly active"
    );
}

/// Retry and resume re-read and re-validate: the document may have changed on
/// disk since the last attempt, so its schema and its shell surfaces get a
/// fresh look. A loop iteration does not — it re-materializes against the
/// stamped structural plan and so cannot introduce command bytes the audit
/// never saw.
#[test]
fn retry_and_resume_fully_validate_but_a_loop_iteration_reuses_its_plan() {
    for reason in [DocumentEntryReason::Retry, DocumentEntryReason::Resume] {
        let stages = reason.stages();
        assert!(stages.full_validation, "{reason:?} re-validates");
        assert_eq!(stages.source_basis, SourceBasis::FreshRead);
        assert_eq!(
            stages.loop_ownership,
            LoopOwnership::InheritActive,
            "{reason:?} keeps the loop the active document already owns"
        );
    }

    let looped = DocumentEntryReason::LoopIteration.stages();
    assert!(!looped.full_validation);
    assert_eq!(looped.source_basis, SourceBasis::StampedStructuralPlan);
    assert_eq!(looped.loop_ownership, LoopOwnership::ReuseStamped);
}

#[test]
fn labels_are_stable_and_distinct() {
    let mut labels: Vec<_> = DocumentEntryReason::ALL.iter().map(|r| r.label()).collect();
    labels.sort_unstable();
    labels.dedup();
    assert_eq!(labels.len(), DocumentEntryReason::ALL.len());
}
