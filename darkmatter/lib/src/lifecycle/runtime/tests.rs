//! The catch protocol names how each requested event was entered.

use std::error::Error as StdError;

use super::*;
use crate::lifecycle::envelope::DiagnosticSnapshot;

struct NoCodes;

impl DiagnosticCatalog for NoCodes {
    fn select(&self, _: &(dyn StdError + 'static)) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_code(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_error_kind(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
}

fn raised() -> LifecycleEventOutcome {
    LifecycleEventOutcome {
        evaluation_error: Some(LifecycleErrorInfo::from_action_failure_with(&NoCodes, "evaluation", "boom")),
        ..LifecycleEventOutcome::default()
    }
}

fn step_mode(protocol: &LifecycleCatchProtocol) -> Option<(LifecycleSignal, EvaluationMode)> {
    protocol.next_step().map(|step| (step.signal, step.mode))
}

#[test]
fn an_explicit_setup_error_is_refusal_cleanup_through_finalize() {
    let origin = LifecycleEventOutcome {
        control: Some(StackControl::Error { reason: None }),
        ..LifecycleEventOutcome::default()
    };
    let mut protocol = LifecycleCatchProtocol::with_catalog(
        &NoCodes,
        LifecycleSignal::Initialize,
        LifecycleCatchState::default(),
        None,
        origin,
    );

    assert_eq!(step_mode(&protocol), Some((LifecycleSignal::Failure, EvaluationMode::RefusalCleanup)));
    assert_eq!(
        protocol.next_step().and_then(|step| step.error.as_ref()).map(|error| error.msg.as_str()),
        Some("lifecycle initialize error")
    );
    assert!(protocol.record(LifecycleSignal::Failure, LifecycleEventOutcome::default()));
    assert_eq!(step_mode(&protocol), Some((LifecycleSignal::Finalize, EvaluationMode::RefusalCleanup)));
}

#[test]
fn a_cleanup_evaluation_error_makes_finalize_fatal_evaluation_cleanup() {
    let start_failure = LifecycleEventOutcome {
        action_error: Some(LifecycleErrorInfo::from_action_failure_with(&NoCodes, "stdout", "closed")),
        ..LifecycleEventOutcome::default()
    };
    let mut protocol = LifecycleCatchProtocol::with_catalog(
        &NoCodes,
        LifecycleSignal::Start,
        LifecycleCatchState::default(),
        None,
        start_failure,
    );
    assert!(protocol.record(LifecycleSignal::Failure, raised()));

    assert_eq!(step_mode(&protocol), Some((LifecycleSignal::Finalize, EvaluationMode::FatalEvaluationCleanup)));
}

#[test]
fn a_terminal_event_requests_a_normal_finalize() {
    let protocol = LifecycleCatchProtocol::with_catalog(
        &NoCodes,
        LifecycleSignal::Success,
        LifecycleCatchState { terminal_slot: Some(LifecycleSignal::Success), finalize_emitted: false },
        None,
        LifecycleEventOutcome::default(),
    );

    assert_eq!(step_mode(&protocol), Some((LifecycleSignal::Finalize, EvaluationMode::NormalRecovery)));
}
