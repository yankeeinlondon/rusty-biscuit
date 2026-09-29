use super::*;
use crate::provider_id::PROVIDERS_DISPLAY_ORDER;
use crate::steering::vocabulary::{
    ConversationEffect as E, DeliveryBoundary as B, ExecutionSelection, GuaranteeLevel as G,
    OperationIntent as O, ReceiptGuarantees, SteeringAccess, SteeringCase, SteeringTransport,
};

const RECEIPTS: ReceiptGuarantees = ReceiptGuarantees {
    request_acceptance: G::Confirmed,
    persistence: G::Unknown,
    scheduling: G::Unknown,
    conversation_delivery: G::Unknown,
};

const fn mechanism(id: &'static str, intent: O, effect: E, boundary: B, timing: ReceiptTiming) -> SteeringMechanism {
    SteeringMechanism {
        id,
        transport: SteeringTransport::Stdio,
        request_format: "JSON line",
        operation_intent: intent,
        conversation_effect: effect,
        delivery_boundary: boundary,
        receipt_timing: timing,
        receipts: RECEIPTS,
        delivery_states: &[],
    }
}

const fn case(state: ExecutionState, support: CaseSupport, mechanism_ids: &'static [&'static str]) -> SteeringCase {
    SteeringCase {
        profile_id: "managed",
        os: HostOs::Macos,
        launch_mode: LaunchMode::NonInteractive,
        origin: LaunchOrigin::Claudine,
        state,
        support,
        mechanism_ids,
        reason: "fixture",
    }
}

const fn access(mechanism_id: &'static str, status: AccessStatus) -> SteeringAccess {
    SteeringAccess { mechanism_id, profile_id: "managed", os: HostOs::Macos, status, prerequisite: "start with --rpc" }
}

static FACTS: ProviderSteering = ProviderSteering {
    discovery: &[],
    mechanisms: &[
        mechanism("steer", O::SteerActiveTurn, E::PreserveRunningTurn, B::EndOfToolBatch, ReceiptTiming::Early),
        mechanism("follow", O::QueueFollowUp, E::PreserveRunningTurn, B::NextTurn, ReceiptTiming::Early),
        mechanism("idle", O::StartIdleTurn, E::ResumeSameConversation, B::IdleTurnStart, ReceiptTiming::MultiPhase),
        mechanism("abort", O::InterruptThenSubmit, E::CancelTurnSameConversation, B::IdleTurnStart, ReceiptTiming::Early),
        mechanism("late", O::SteerActiveTurn, E::PreserveRunningTurn, B::NextToolBoundary, ReceiptTiming::Terminal),
        mechanism("setup", O::SteerActiveTurn, E::PreserveRunningTurn, B::NextToolBoundary, ReceiptTiming::Early),
    ],
    cases: &[
        case(ExecutionState::Working, CaseSupport::NonInterrupting, &["steer", "abort"]),
        case(ExecutionState::Idle, CaseSupport::NonInterrupting, &["idle"]),
    ],
    access: &[
        access("steer", AccessStatus::Available),
        access("follow", AccessStatus::Available),
        access("idle", AccessStatus::Available),
        access("abort", AccessStatus::Available),
        access("late", AccessStatus::Available),
        access("setup", AccessStatus::SetupRequired),
    ],
    verification: &[],
    execution_interfaces: &[],
    execution_selection: Some(ExecutionSelection { preferred: "rpc", fallback: None }),
};

const ADAPTER: AdapterRef = AdapterRef { id: "fixture-rpc", revision: 1 };

fn grant(mechanism_id: &'static str, operation: O, state: ExecutionState) -> ActivationGrant {
    ActivationGrant {
        provider: "pi",
        mechanism_id,
        operation,
        adapter: ADAPTER,
        profile_id: "managed",
        os: HostOs::Macos,
        provider_version: "0.84.4",
        launch_mode: LaunchMode::NonInteractive,
        origin: LaunchOrigin::Claudine,
        state,
        verification_ids: &["verified"],
    }
}

fn session(state: ExecutionState) -> SessionFacts<'static> {
    SessionFacts {
        provider: Provider::Pi,
        profile_id: "managed",
        os: HostOs::Macos,
        launch_mode: LaunchMode::NonInteractive,
        origin: LaunchOrigin::Claudine,
        state,
        provider_version: Some("0.84.4"),
    }
}

fn implemented(adapter: AdapterRef) -> bool {
    adapter == ADAPTER
}

fn all_grants() -> Vec<ActivationGrant> {
    vec![
        grant("steer", O::SteerActiveTurn, ExecutionState::Working),
        grant("abort", O::InterruptThenSubmit, ExecutionState::Working),
        grant("idle", O::StartIdleTurn, ExecutionState::Idle),
    ]
}

/// A variant of [`FACTS`] whose working case lists different mechanisms.
fn with_working_mechanisms(ids: &'static [&'static str]) -> &'static ProviderSteering {
    let cases: &'static [SteeringCase] = Box::leak(Box::new([case(ExecutionState::Working, CaseSupport::NonInterrupting, ids)]));
    Box::leak(Box::new(ProviderSteering { cases, ..FACTS }))
}

#[test]
fn verified_implemented_steering_is_non_interrupting_and_rescues() {
    let result = evaluate_with(&session(ExecutionState::Working), &FACTS, &all_grants(), &implemented);
    assert_eq!(result.manual.availability, SteeringAvailability::NonInterrupting);
    let route = result.manual.route.unwrap();
    assert_eq!(route.mechanism.id, "steer", "non-interrupting preferred over interruption");
    assert_eq!(route.max_receipt(), ReceiptStrength::Accepted);
    assert!(matches!(result.automatic, AutomaticEligibility::Eligible(route) if route.mechanism.id == "steer"));
    assert!(result.manual.blockers.is_empty());
}

#[test]
fn exact_applicability_rejects_every_mismatched_dimension() {
    let grants = all_grants();
    let base = session(ExecutionState::Working);
    let wrong_version = SessionFacts { provider_version: Some("0.84.5"), ..base };
    let wrong_provider = SessionFacts { provider: Provider::Codex, ..base };
    for mismatched in [wrong_version, wrong_provider] {
        let result = evaluate_with(&mismatched, &FACTS, &grants, &implemented);
        assert_eq!(result.manual.availability, SteeringAvailability::Unavailable);
        assert_eq!(
            result.manual.blockers,
            [Blocker::NotActivated { mechanism: "steer", setup: None }, Blocker::NotActivated { mechanism: "abort", setup: None }]
        );
    }
    for mismatched in [
        SessionFacts { os: HostOs::Windows, ..base },
        SessionFacts { profile_id: "ordinary", ..base },
        SessionFacts { origin: LaunchOrigin::Native, ..base },
        SessionFacts { launch_mode: LaunchMode::Interactive, ..base },
    ] {
        let result = evaluate_with(&mismatched, &FACTS, &grants, &implemented);
        assert_eq!(result.manual.blockers, [Blocker::NoResearchCase], "{mismatched:?}");
        assert_eq!(result.automatic, AutomaticEligibility::Unavailable(AutomaticBlocker::NoNonInterruptingRoute));
    }
}

#[test]
fn a_grant_for_another_state_does_not_apply() {
    let grants = vec![grant("steer", O::SteerActiveTurn, ExecutionState::Idle)];
    let result = evaluate_with(&session(ExecutionState::Working), with_working_mechanisms(&["steer"]), &grants, &implemented);
    assert_eq!(result.manual.blockers, [Blocker::NotActivated { mechanism: "steer", setup: None }]);
}

#[test]
fn unimplemented_or_unreviewed_adapter_revision_blocks_delivery() {
    let result = evaluate_with(&session(ExecutionState::Working), with_working_mechanisms(&["steer"]), &all_grants(), &|_| false);
    assert_eq!(result.manual.availability, SteeringAvailability::Unavailable);
    assert_eq!(result.manual.blockers, [Blocker::AdapterNotImplemented { mechanism: "steer", adapter: ADAPTER }]);
    assert!(result.manual.blockers[0].to_string().contains("revision 1"));
}

#[test]
fn next_turn_follow_up_is_manual_only() {
    let grants = vec![grant("follow", O::QueueFollowUp, ExecutionState::Working)];
    let result = evaluate_with(&session(ExecutionState::Working), with_working_mechanisms(&["follow"]), &grants, &implemented);
    assert_eq!(result.manual.availability, SteeringAvailability::NonInterrupting);
    assert_eq!(
        result.automatic,
        AutomaticEligibility::Unavailable(AutomaticBlocker::CannotRescueActiveTurn { mechanisms: vec!["follow"] })
    );
}

#[test]
fn terminal_only_acknowledgment_cannot_return_after_acceptance() {
    let grants = vec![grant("late", O::SteerActiveTurn, ExecutionState::Working)];
    let result = evaluate_with(&session(ExecutionState::Working), with_working_mechanisms(&["late"]), &grants, &implemented);
    assert_eq!(result.manual.availability, SteeringAvailability::Unavailable);
    assert_eq!(result.manual.blockers, [Blocker::NoEarlyAcceptance { mechanism: "late", timing: ReceiptTiming::Terminal }]);
}

#[test]
fn setup_required_access_needs_a_reviewed_grant_and_reports_its_prerequisite() {
    let facts = with_working_mechanisms(&["setup"]);
    let result = evaluate_with(&session(ExecutionState::Working), facts, &[], &implemented);
    let [blocker] = result.manual.blockers.as_slice() else { panic!("{:?}", result.manual.blockers) };
    assert_eq!(*blocker, Blocker::NotActivated { mechanism: "setup", setup: Some("start with --rpc") });
    assert!(blocker.to_string().contains("requires setup: start with --rpc"));

    let grants = vec![grant("setup", O::SteerActiveTurn, ExecutionState::Working)];
    let result = evaluate_with(&session(ExecutionState::Working), facts, &grants, &implemented);
    assert_eq!(result.manual.availability, SteeringAvailability::NonInterrupting);
}

#[test]
fn blocked_or_unknown_access_blocks_even_with_a_grant() {
    for status in [AccessStatus::Blocked, AccessStatus::Unknown] {
        let access: &'static [SteeringAccess] = Box::leak(Box::new([access("steer", status)]));
        let facts: &'static ProviderSteering =
            Box::leak(Box::new(ProviderSteering { access, ..*with_working_mechanisms(&["steer"]) }));
        let result = evaluate_with(&session(ExecutionState::Working), facts, &all_grants(), &implemented);
        assert_eq!(
            result.manual.blockers,
            [Blocker::AccessNotAvailable { mechanism: "steer", status, prerequisite: "start with --rpc" }]
        );
    }
    let facts: &'static ProviderSteering =
        Box::leak(Box::new(ProviderSteering { access: &[], ..*with_working_mechanisms(&["steer"]) }));
    let result = evaluate_with(&session(ExecutionState::Working), facts, &all_grants(), &implemented);
    assert!(matches!(
        result.manual.blockers.as_slice(),
        [Blocker::AccessNotAvailable { status: AccessStatus::Unknown, .. }]
    ));
}

#[test]
fn interruption_only_route_is_selectable_manually_but_never_automatic() {
    let result = evaluate_with(&session(ExecutionState::Working), with_working_mechanisms(&["abort"]), &all_grants(), &implemented);
    assert_eq!(result.manual.availability, SteeringAvailability::InterruptionRequired);
    assert_eq!(result.manual.route.unwrap().mechanism.id, "abort");
    assert_eq!(result.automatic, AutomaticEligibility::Unavailable(AutomaticBlocker::NoNonInterruptingRoute));
}

#[test]
fn idle_start_is_manual_and_never_automatic() {
    let result = evaluate_with(&session(ExecutionState::Idle), &FACTS, &all_grants(), &implemented);
    assert_eq!(result.manual.availability, SteeringAvailability::NonInterrupting);
    assert_eq!(result.manual.route.unwrap().mechanism.id, "idle");
    assert_eq!(result.automatic, AutomaticEligibility::Unavailable(AutomaticBlocker::NotWorking(ExecutionState::Idle)));
}

#[test]
fn a_mechanism_whose_operation_cannot_serve_the_state_is_blocked() {
    // An idle-start mechanism listed on a working case is a research error,
    // but runtime still refuses it rather than starting a new turn.
    let grants = vec![grant("idle", O::StartIdleTurn, ExecutionState::Working)];
    let result = evaluate_with(&session(ExecutionState::Working), with_working_mechanisms(&["idle"]), &grants, &implemented);
    assert_eq!(result.manual.blockers, [Blocker::OperationMismatch { mechanism: "idle" }]);
}

#[test]
fn unknown_state_or_version_is_never_guessed() {
    let unknown_state = evaluate_with(&session(ExecutionState::Unknown), &FACTS, &all_grants(), &implemented);
    assert_eq!(unknown_state.manual.blockers, [Blocker::StateUnknown]);
    assert_eq!(unknown_state.automatic, AutomaticEligibility::Unavailable(AutomaticBlocker::NotWorking(ExecutionState::Unknown)));

    let unknown_version = SessionFacts { provider_version: None, ..session(ExecutionState::Working) };
    let result = evaluate_with(&unknown_version, &FACTS, &all_grants(), &implemented);
    assert_eq!(result.manual.blockers, [Blocker::VersionUnknown]);
}

#[test]
fn unsupported_research_case_reports_its_reason() {
    let cases: &'static [SteeringCase] =
        Box::leak(Box::new([case(ExecutionState::Working, CaseSupport::Unknown, &[])]));
    let facts: &'static ProviderSteering = Box::leak(Box::new(ProviderSteering { cases, ..FACTS }));
    let result = evaluate_with(&session(ExecutionState::Working), facts, &all_grants(), &implemented);
    assert_eq!(result.manual.blockers, [Blocker::CaseNotSupported { support: CaseSupport::Unknown, reason: "fixture" }]);
}

/// Passive corpus check over the shipped generated catalog: with no
/// reviewed grants and no implemented adapters, no researched case of any
/// provider is selectable, and each reports a specific blocker.
#[test]
fn shipped_catalog_activates_nothing_without_reviewed_grants() {
    for provider in PROVIDERS_DISPLAY_ORDER {
        let facts = crate::steering::facts(provider);
        assert!(!facts.cases.is_empty(), "{provider:?} has no researched cases");
        for case in facts.cases {
            let result = evaluate(&SessionFacts {
                provider,
                profile_id: case.profile_id,
                os: case.os,
                launch_mode: case.launch_mode,
                origin: case.origin,
                state: case.state,
                provider_version: Some("0.84.4"),
            });
            assert_eq!(result.manual.availability, SteeringAvailability::Unavailable, "{provider:?} {case:?}");
            assert!(!result.manual.blockers.is_empty(), "{provider:?} {case:?}");
            assert!(matches!(result.automatic, AutomaticEligibility::Unavailable(_)));
        }
    }
    assert!(crate::steering::activation_grants().is_empty());
}

/// The existing Pi verification records are evidence only: even the passing
/// active-steering fixture grants nothing without a reviewed grant.
#[test]
fn pi_passing_fixture_records_are_not_activation_grants() {
    let pi = crate::steering::facts(Provider::Pi);
    let active = pi.verification.iter().find(|record| record.id == "pi-rpc-steer-active-0844").unwrap();
    let result = evaluate(&SessionFacts {
        provider: Provider::Pi,
        profile_id: active.profile_id,
        os: active.os,
        launch_mode: active.launch_mode,
        origin: active.origin,
        state: active.state,
        provider_version: Some(active.provider_version),
    });
    assert_eq!(result.manual.availability, SteeringAvailability::Unavailable);
    assert!(
        result.manual.blockers.iter().any(|blocker| matches!(blocker, Blocker::NotActivated { mechanism: "rpc-steer", setup: Some(_) })),
        "{:?}",
        result.manual.blockers
    );
}
