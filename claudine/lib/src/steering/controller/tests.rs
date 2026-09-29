//! Controller behavior through its public API with a fake executor: routing,
//! serialization, bounds, deadlines, stale and duplicate rejection, and
//! shutdown. Deadline tests run on a paused clock.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;
use tokio::sync::Notify;

use super::*;
use crate::steering::contract::{InterruptionConsent, SteeringMessage};
use crate::steering::eligibility::evaluate_with;
use crate::steering::identity::RequestId;
use crate::steering::vocabulary::{
    AccessStatus, ActivationGrant, AdapterRef, CaseSupport, ConversationEffect as E, DeliveryBoundary as B,
    ExecutionSelection, GuaranteeLevel as G, OperationIntent as O, ProviderSteering, ReceiptGuarantees,
    ReceiptStrength, ReceiptTiming, SteeringAccess, SteeringAvailability, SteeringCase, SteeringMechanism,
    SteeringTransport,
};

const SECRET_MESSAGE: &str = "Recheck the failing test; the key is ghp_0123456789abcdefghij0123 ✓";

const fn mechanism(id: &'static str, intent: O, effect: E) -> SteeringMechanism {
    SteeringMechanism {
        id,
        transport: SteeringTransport::Stdio,
        request_format: "JSON line",
        operation_intent: intent,
        conversation_effect: effect,
        delivery_boundary: B::EndOfToolBatch,
        receipt_timing: ReceiptTiming::Early,
        receipts: ReceiptGuarantees {
            request_acceptance: G::Confirmed,
            persistence: G::Unknown,
            scheduling: G::Unknown,
            conversation_delivery: G::Unknown,
        },
        delivery_states: &[],
    }
}

const fn case(state: ExecutionState, support: CaseSupport, ids: &'static [&'static str]) -> SteeringCase {
    SteeringCase {
        profile_id: "managed",
        os: HostOs::Macos,
        launch_mode: LaunchMode::NonInteractive,
        origin: LaunchOrigin::Claudine,
        state,
        support,
        mechanism_ids: ids,
        reason: "fixture",
    }
}

const fn access(mechanism_id: &'static str) -> SteeringAccess {
    SteeringAccess { mechanism_id, profile_id: "managed", os: HostOs::Macos, status: AccessStatus::Available, prerequisite: "" }
}

static FACTS: ProviderSteering = ProviderSteering {
    discovery: &[],
    mechanisms: &[
        mechanism("steer", O::SteerActiveTurn, E::PreserveRunningTurn),
        mechanism("idle", O::StartIdleTurn, E::ResumeSameConversation),
        mechanism("abort", O::InterruptThenSubmit, E::CancelTurnSameConversation),
    ],
    cases: &[
        case(ExecutionState::Working, CaseSupport::NonInterrupting, &["steer"]),
        case(ExecutionState::Idle, CaseSupport::NonInterrupting, &["idle"]),
    ],
    access: &[access("steer"), access("idle"), access("abort")],
    verification: &[],
    execution_interfaces: &[],
    execution_selection: Some(ExecutionSelection { preferred: "rpc", fallback: None }),
};

/// Only interruption is available while working.
static INTERRUPT_ONLY: ProviderSteering = ProviderSteering {
    cases: &[case(ExecutionState::Working, CaseSupport::InterruptionRequired, &["abort"])],
    ..FACTS
};

const ADAPTER: AdapterRef = AdapterRef { id: "fixture", revision: 1 };

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

fn eligibility_over(facts: &'static ProviderSteering) -> EligibilityFn {
    let grants = vec![
        grant("steer", O::SteerActiveTurn, ExecutionState::Working),
        grant("idle", O::StartIdleTurn, ExecutionState::Idle),
        grant("abort", O::InterruptThenSubmit, ExecutionState::Working),
    ];
    Arc::new(move |session: &SessionFacts<'_>| evaluate_with(session, facts, &grants, &|adapter| adapter == ADAPTER))
}

fn facts(profile_id: Option<&str>) -> ExecutionFacts {
    ExecutionFacts {
        provider: Provider::Pi,
        profile_id: profile_id.map(str::to_string),
        os: HostOs::Macos,
        launch_mode: LaunchMode::NonInteractive,
        provider_version: Some("0.84.4".into()),
        cwd: Some("/work/project".into()),
        name: None,
    }
}

#[derive(Clone, Copy)]
enum Behavior {
    /// Report `outcome` after `delay`.
    Reply(SendOutcome, Duration),
    /// Wait until the gate is opened, then report accepted.
    Gated,
    /// Never resolve.
    Hang,
}

#[derive(Default)]
struct FakeExecutor {
    script: Mutex<VecDeque<Behavior>>,
    delivered: Mutex<Vec<(RequestId, String, &'static str)>>,
    in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    gate: Notify,
}

impl FakeExecutor {
    fn scripted(script: impl IntoIterator<Item = Behavior>) -> Arc<Self> {
        Arc::new(Self { script: Mutex::new(script.into_iter().collect()), ..Self::default() })
    }

    fn delivered(&self) -> Vec<(RequestId, String, &'static str)> {
        self.delivered.lock().unwrap().clone()
    }
}

struct Handle(Arc<FakeExecutor>);

impl SteeringExecutor for Handle {
    fn deliver(&self, request: SteeringRequest, route: Route, _deadlines: DeliveryDeadlines) -> DeliveryFuture {
        let fake = Arc::clone(&self.0);
        Box::pin(async move {
            let now = fake.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            fake.max_in_flight.fetch_max(now, Ordering::SeqCst);
            fake.delivered.lock().unwrap().push((request.id, request.message.as_str().to_string(), route.mechanism.id));
            let behavior = fake.script.lock().unwrap().pop_front().unwrap_or(Behavior::Reply(SendOutcome::Accepted, Duration::ZERO));
            let outcome = match behavior {
                Behavior::Reply(outcome, delay) => {
                    tokio::time::sleep(delay).await;
                    outcome
                }
                Behavior::Gated => {
                    fake.gate.notified().await;
                    SendOutcome::Accepted
                }
                Behavior::Hang => std::future::pending().await,
            };
            fake.in_flight.fetch_sub(1, Ordering::SeqCst);
            DeliveryReport::new(SteeringResult::submitted(&request, Some(route.mechanism.id), outcome))
        })
    }
}

struct Fixture {
    controller: SteeringController,
    executor: Arc<FakeExecutor>,
    audit_dir: tempfile::TempDir,
}

fn start_with(facts_profile: Option<&str>, eligibility: EligibilityFn, executor: Arc<FakeExecutor>) -> Fixture {
    let audit_dir = tempfile::tempdir().unwrap();
    let config = ControllerConfig {
        execution: ExecutionId::from_u128(0xE0),
        wrapper: ProcessStartIdentity::new(4242, "1700000000").unwrap(),
        facts: facts(facts_profile),
        initial_state: ExecutionState::Working,
        eligibility,
        audit: SteeringAuditLog::at(audit_dir.path()),
    };
    let controller = SteeringController::spawn(config, Arc::new(Handle(Arc::clone(&executor))));
    Fixture { controller, executor, audit_dir }
}

fn start(executor: Arc<FakeExecutor>) -> Fixture {
    start_with(Some("managed"), eligibility_over(&FACTS), executor)
}

fn request(fixture: &Fixture, id: u128, origin: SteeringOrigin, operation: O) -> SteeringRequest {
    SteeringRequest {
        id: RequestId::from_u128(id),
        target: fixture.controller.snapshot().target.id(),
        origin,
        operation,
        message: SteeringMessage::new(SECRET_MESSAGE).unwrap(),
        consent: None,
    }
}

fn manual(fixture: &Fixture, id: u128) -> Submission {
    Submission {
        request: request(fixture, id, SteeringOrigin::Manual, O::SteerActiveTurn),
        expected: Some(fixture.controller.snapshot().target),
        opportunity: None,
    }
}

fn automatic(fixture: &Fixture, id: u128) -> Submission {
    Submission {
        request: request(fixture, id, SteeringOrigin::Automatic, O::SteerActiveTurn),
        expected: None,
        opportunity: Some(OpportunityId::from_u128(id)),
    }
}

fn audit_records(dir: &std::path::Path) -> Vec<Value> {
    let mut records = Vec::new();
    for entry in std::fs::read_dir(dir).unwrap() {
        for line in std::fs::read_to_string(entry.unwrap().path()).unwrap().lines() {
            records.push(serde_json::from_str(line).unwrap());
        }
    }
    records
}

async fn wait_for_deliveries(executor: &FakeExecutor, count: usize) {
    while executor.delivered().len() < count {
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn routes_original_text_once_and_audits_only_masked_text() {
    let fixture = start(FakeExecutor::scripted([]));
    let reply = fixture.controller.submit(manual(&fixture, 1)).await;

    assert_eq!(reply.result.outcome, SendOutcome::Accepted);
    assert_eq!(reply.result.receipt, ReceiptStrength::Accepted);
    assert_eq!(reply.result.mechanism, Some("steer"));
    assert_eq!(fixture.executor.delivered(), vec![(RequestId::from_u128(1), SECRET_MESSAGE.to_string(), "steer")]);

    let records = audit_records(fixture.audit_dir.path());
    let kinds: Vec<&str> = records.iter().map(|record| record["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["request", "result"]);
    assert_eq!(records[0]["message"], "Recheck the failing test; the key is **** ✓");
    assert_eq!(records[0]["provider"], "pi");
    assert_eq!(records[0]["generation"], 0);
    assert_eq!(records[1]["mechanism"], "steer");
    assert!(!serde_json::to_string(&records).unwrap().contains("ghp_"));
}

#[tokio::test]
async fn a_receipt_stronger_than_the_mechanism_can_prove_is_lowered() {
    let fixture = start(FakeExecutor::scripted([Behavior::Reply(SendOutcome::Delivered, Duration::ZERO)]));
    let reply = fixture.controller.submit(manual(&fixture, 1)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted, "the fixture mechanism only confirms acceptance");
    assert_eq!(reply.result.receipt, ReceiptStrength::Accepted);
}

#[tokio::test]
async fn concurrent_senders_are_serialized_in_arrival_order() {
    let script = (0..MAX_PENDING_PER_EXECUTION).map(|_| Behavior::Reply(SendOutcome::Accepted, Duration::from_millis(5)));
    let fixture = start(FakeExecutor::scripted(script));
    let mut sends = Vec::new();
    for id in 0..MAX_PENDING_PER_EXECUTION as u128 {
        let controller = fixture.controller.clone();
        let submission = manual(&fixture, id);
        sends.push(tokio::spawn(async move { controller.submit(submission).await }));
        // Admission order is dispatch order.
        tokio::task::yield_now().await;
    }
    for send in sends {
        assert_eq!(send.await.unwrap().result.outcome, SendOutcome::Accepted);
    }
    assert_eq!(fixture.executor.max_in_flight.load(Ordering::SeqCst), 1, "provider mutations never overlap");
    let order: Vec<RequestId> = fixture.executor.delivered().into_iter().map(|(id, ..)| id).collect();
    assert_eq!(order, (0..MAX_PENDING_PER_EXECUTION as u128).map(RequestId::from_u128).collect::<Vec<_>>());
}

#[tokio::test]
async fn a_full_queue_refuses_as_busy_without_evicting_accepted_requests() {
    let script = (0..MAX_PENDING_PER_EXECUTION).map(|_| Behavior::Gated);
    let fixture = start(FakeExecutor::scripted(script));
    let mut sends = Vec::new();
    for id in 0..MAX_PENDING_PER_EXECUTION as u128 {
        let controller = fixture.controller.clone();
        let submission = manual(&fixture, id);
        sends.push(tokio::spawn(async move { controller.submit(submission).await }));
    }
    wait_for_deliveries(&fixture.executor, 1).await;
    for _ in 0..64 {
        tokio::task::yield_now().await;
    }

    let overflow = fixture.controller.submit(manual(&fixture, 99)).await;
    assert_eq!(overflow.result.outcome, SendOutcome::Busy);
    assert_eq!(overflow.detail.unwrap().as_str(), "this execution's steering queue is full");

    for _ in 0..MAX_PENDING_PER_EXECUTION {
        fixture.executor.gate.notify_one();
        tokio::task::yield_now().await;
    }
    for send in sends {
        assert_eq!(send.await.unwrap().result.outcome, SendOutcome::Accepted, "every admitted request is kept");
    }
    assert_eq!(fixture.executor.delivered().len(), MAX_PENDING_PER_EXECUTION, "the busy request was never submitted");
    // Capacity returns once requests complete.
    assert_eq!(fixture.controller.submit(manual(&fixture, 100)).await.result.outcome, SendOutcome::Accepted);
}

#[tokio::test]
async fn only_one_automatic_request_may_be_pending() {
    let fixture = start(FakeExecutor::scripted([Behavior::Gated]));
    let controller = fixture.controller.clone();
    let first = automatic(&fixture, 1);
    let pending = tokio::spawn(async move { controller.submit(first).await });
    wait_for_deliveries(&fixture.executor, 1).await;

    let second = fixture.controller.submit(automatic(&fixture, 2)).await;
    assert_eq!(second.result.outcome, SendOutcome::Busy);
    assert_eq!(second.detail.unwrap().as_str(), "an automatic request is already pending for this execution");

    // A manual request is still admitted behind it.
    let controller = fixture.controller.clone();
    let manual_submission = manual(&fixture, 3);
    let queued = tokio::spawn(async move { controller.submit(manual_submission).await });
    fixture.executor.gate.notify_one();
    assert_eq!(pending.await.unwrap().result.outcome, SendOutcome::Accepted);
    assert_eq!(queued.await.unwrap().result.outcome, SendOutcome::Accepted);
    // The automatic slot is free again.
    assert_eq!(fixture.controller.submit(automatic(&fixture, 4)).await.result.outcome, SendOutcome::Accepted);
}

#[test]
fn deadlines_follow_origin_and_consent() {
    let fixture_ids = SteeringTargetId::Managed { execution: ExecutionId::from_u128(1) };
    let base = SteeringRequest {
        id: RequestId::from_u128(1),
        target: fixture_ids.clone(),
        origin: SteeringOrigin::Manual,
        operation: O::SteerActiveTurn,
        message: SteeringMessage::new("x").unwrap(),
        consent: None,
    };
    let at = Instant::now();
    assert_eq!(DeliveryDeadlines::for_request(&base, at), DeliveryDeadlines { cancellation: None, acceptance: at + Duration::from_secs(10) });
    let automatic = SteeringRequest { origin: SteeringOrigin::Automatic, ..base.clone() };
    assert_eq!(DeliveryDeadlines::for_request(&automatic, at).acceptance, at + Duration::from_secs(2));
    let consented = SteeringRequest {
        operation: O::InterruptThenSubmit,
        consent: Some(InterruptionConsent { target: fixture_ids, operation: O::InterruptThenSubmit }),
        ..base
    };
    assert_eq!(
        DeliveryDeadlines::for_request(&consented, at),
        DeliveryDeadlines { cancellation: Some(at + Duration::from_secs(10)), acceptance: at + Duration::from_secs(20) }
    );
}

#[tokio::test(start_paused = true)]
async fn an_expired_unsent_request_is_discarded_and_a_submitted_one_is_unknown() {
    let fixture = start(FakeExecutor::scripted([Behavior::Hang]));
    let controller = fixture.controller.clone();
    let first = manual(&fixture, 1);
    let first = tokio::spawn(async move { controller.submit(first).await });
    wait_for_deliveries(&fixture.executor, 1).await;
    let second = fixture.controller.submit(manual(&fixture, 2)).await;
    let first = first.await.unwrap();

    assert_eq!(first.result.outcome, SendOutcome::Unknown, "a submission without acceptance is ambiguous");
    assert!(first.detail.unwrap().as_str().contains("never retried"));
    assert_eq!(second.result.outcome, SendOutcome::Busy, "the queued request expired unsent");
    assert_eq!(second.detail.unwrap().as_str(), "the request expired before submission; nothing was sent");
    assert_eq!(fixture.executor.delivered().len(), 1, "the expired request never reached the provider");
}

#[tokio::test(start_paused = true)]
async fn a_late_acceptance_is_logged_as_an_update_and_never_resent() {
    let fixture = start(FakeExecutor::scripted([Behavior::Reply(SendOutcome::Accepted, Duration::from_secs(12))]));
    let reply = fixture.controller.submit(manual(&fixture, 1)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unknown);

    tokio::time::sleep(Duration::from_secs(5)).await;
    let records = audit_records(fixture.audit_dir.path());
    let kinds: Vec<&str> = records.iter().map(|record| record["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["request", "result", "late_result"]);
    assert_eq!(records[1]["outcome"], "unknown");
    assert_eq!(records[2]["outcome"], "accepted");
    assert_eq!(records[2]["request_id"], records[0]["request_id"]);
    assert_eq!(fixture.executor.delivered().len(), 1);
}

#[tokio::test(start_paused = true)]
async fn automatic_requests_get_two_seconds() {
    let fixture = start(FakeExecutor::scripted([
        Behavior::Reply(SendOutcome::Accepted, Duration::from_millis(1_900)),
        Behavior::Reply(SendOutcome::Accepted, Duration::from_millis(2_100)),
    ]));
    assert_eq!(fixture.controller.submit(automatic(&fixture, 1)).await.result.outcome, SendOutcome::Accepted);
    assert_eq!(fixture.controller.submit(automatic(&fixture, 2)).await.result.outcome, SendOutcome::Unknown);
}

#[tokio::test]
async fn a_stale_target_is_rejected_instead_of_retargeted() {
    let fixture = start(FakeExecutor::scripted([]));
    let listed = fixture.controller.snapshot().target;

    let generation = fixture.controller.set_conversation(Some("conversation-b".into()));
    assert_eq!(generation, ConversationGeneration(1));
    let stale = Submission { expected: Some(listed.clone()), ..manual(&fixture, 1) };
    let reply = fixture.controller.submit(stale).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert_eq!(reply.detail.unwrap().as_str(), "the selected target changed: the provider conversation was replaced");

    // A reused wrapper PID with a new start marker is a different owner.
    let reused_pid = ManagedTarget {
        wrapper: ProcessStartIdentity::new(4242, "1800000000").unwrap(),
        ..fixture.controller.snapshot().target
    };
    let reply = fixture.controller.submit(Submission { expected: Some(reused_pid), ..manual(&fixture, 2) }).await;
    assert_eq!(reply.detail.unwrap().as_str(), "the selected target changed: the owning wrapper process changed");

    assert!(fixture.executor.delivered().is_empty(), "stale requests never reach the provider");
    // The fresh binding still works.
    assert_eq!(fixture.controller.submit(manual(&fixture, 3)).await.result.outcome, SendOutcome::Accepted);
}

#[tokio::test]
async fn a_repeated_request_id_is_refused_and_not_replayed() {
    let fixture = start(FakeExecutor::scripted([Behavior::Reply(SendOutcome::Unknown, Duration::ZERO)]));
    assert_eq!(fixture.controller.submit(manual(&fixture, 7)).await.result.outcome, SendOutcome::Unknown);
    let again = fixture.controller.submit(manual(&fixture, 7)).await;
    assert_eq!(again.result.outcome, SendOutcome::Refused);
    assert!(again.detail.unwrap().as_str().contains("never replayed"));
    assert_eq!(fixture.executor.delivered().len(), 1);
}

#[tokio::test]
async fn requests_for_another_target_or_without_consent_are_refused() {
    let fixture = start(FakeExecutor::scripted([]));

    let mut other = manual(&fixture, 1);
    other.request.target = SteeringTargetId::Managed { execution: ExecutionId::from_u128(0xE1) };
    assert_eq!(fixture.controller.submit(other).await.result.outcome, SendOutcome::Refused);

    let unconsented = Submission {
        request: request(&fixture, 2, SteeringOrigin::Manual, O::InterruptThenSubmit),
        ..manual(&fixture, 2)
    };
    let reply = fixture.controller.submit(unconsented).await;
    assert_eq!(reply.result.outcome, SendOutcome::Refused);
    assert!(reply.detail.unwrap().as_str().contains("explicit manual consent"));

    let automatic_interrupt = Submission {
        request: request(&fixture, 3, SteeringOrigin::Automatic, O::InterruptThenSubmit),
        ..automatic(&fixture, 3)
    };
    assert_eq!(fixture.controller.submit(automatic_interrupt).await.result.outcome, SendOutcome::Refused);
    assert!(fixture.executor.delivered().is_empty());
}

#[tokio::test]
async fn lost_non_interrupting_delivery_never_switches_to_interruption() {
    let fixture = start_with(Some("managed"), eligibility_over(&INTERRUPT_ONLY), FakeExecutor::scripted([]));
    assert_eq!(fixture.controller.snapshot().availability.availability, SteeringAvailability::InterruptionRequired);

    let reply = fixture.controller.submit(manual(&fixture, 1)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert_eq!(
        reply.detail.unwrap().as_str(),
        "the requested `steer_active_turn` operation is no longer available; this session now offers `interrupt_then_submit`"
    );
    assert!(fixture.executor.delivered().is_empty());

    // An explicitly consented interruption is admitted and routed.
    let target = fixture.controller.snapshot().target.id();
    let consented = Submission {
        request: SteeringRequest {
            operation: O::InterruptThenSubmit,
            consent: Some(InterruptionConsent { target, operation: O::InterruptThenSubmit }),
            ..request(&fixture, 2, SteeringOrigin::Manual, O::InterruptThenSubmit)
        },
        ..manual(&fixture, 2)
    };
    let reply = fixture.controller.submit(consented).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted);
    assert_eq!(reply.result.mechanism, Some("abort"));
}

#[tokio::test]
async fn state_changes_update_availability_and_route_selection() {
    let fixture = start(FakeExecutor::scripted([]));
    let mut updates = fixture.controller.subscribe();
    fixture.controller.set_state(ExecutionState::Idle);
    assert!(updates.has_changed().unwrap());
    assert_eq!(updates.borrow_and_update().state, ExecutionState::Idle);

    // The working-turn operation no longer fits an idle session.
    let reply = fixture.controller.submit(manual(&fixture, 1)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    let idle = Submission { request: request(&fixture, 2, SteeringOrigin::Manual, O::StartIdleTurn), ..manual(&fixture, 2) };
    let reply = fixture.controller.submit(idle).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted);
    assert_eq!(reply.result.mechanism, Some("idle"));

    // Automatic help never starts an idle turn.
    let reply = fixture.controller.submit(automatic(&fixture, 3)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert_eq!(reply.detail.unwrap().as_str(), "automatic help needs a working session; state is idle");
}

#[tokio::test]
async fn an_unmapped_or_ungranted_launch_is_unavailable_with_a_reason() {
    let unmapped = start_with(None, eligibility_over(&FACTS), FakeExecutor::scripted([]));
    let summary = unmapped.controller.snapshot().availability;
    assert_eq!(summary.availability, SteeringAvailability::Unavailable);
    assert_eq!(summary.reason.as_deref(), Some(UNMAPPED_PROFILE_REASON));
    let reply = unmapped.controller.submit(manual(&unmapped, 1)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert_eq!(reply.detail.unwrap().as_str(), UNMAPPED_PROFILE_REASON);

    // The shipped policy blocks Pi's managed RPC profile, so a researched Pi
    // RPC launch reports the reviewed reason instead of routing.
    let shipped = start_with(Some("retained-rpc"), Arc::new(crate::steering::eligibility::evaluate), FakeExecutor::scripted([]));
    let summary = shipped.controller.snapshot().availability;
    assert_eq!(summary.availability, SteeringAvailability::Unavailable);
    assert!(summary.reason.unwrap().contains("steering is blocked for this launch profile"));
    assert!(summary.setup_requirements.is_empty());
    assert_eq!(shipped.controller.submit(automatic(&shipped, 1)).await.result.outcome, SendOutcome::Unavailable);
    assert!(unmapped.executor.delivered().is_empty() && shipped.executor.delivered().is_empty());
}

#[tokio::test]
async fn shutdown_answers_queued_requests_unavailable_and_the_in_flight_one_unknown() {
    let fixture = start(FakeExecutor::scripted([Behavior::Hang]));
    let controller = fixture.controller.clone();
    let first = manual(&fixture, 1);
    let first = tokio::spawn(async move { controller.submit(first).await });
    wait_for_deliveries(&fixture.executor, 1).await;
    let controller = fixture.controller.clone();
    let second = manual(&fixture, 2);
    let second = tokio::spawn(async move { controller.submit(second).await });
    for _ in 0..16 {
        tokio::task::yield_now().await;
    }

    fixture.controller.shutdown();
    assert_eq!(first.await.unwrap().result.outcome, SendOutcome::Unknown);
    let second = second.await.unwrap();
    assert_eq!(second.result.outcome, SendOutcome::Unavailable);
    assert_eq!(second.detail.unwrap().as_str(), "the execution ended before the request was submitted");
    let after = fixture.controller.submit(manual(&fixture, 3)).await;
    assert_eq!(after.result.outcome, SendOutcome::Unavailable);
    assert_eq!(fixture.executor.delivered().len(), 1);
}

#[tokio::test]
async fn an_audit_write_failure_leaves_the_delivery_result_unchanged() {
    let blocker = tempfile::NamedTempFile::new().unwrap();
    let executor = FakeExecutor::scripted([]);
    let config = ControllerConfig {
        audit: SteeringAuditLog::at(blocker.path().join("steering")),
        eligibility: eligibility_over(&FACTS),
        initial_state: ExecutionState::Working,
        ..ControllerConfig::new(ProcessStartIdentity::new(1, "1").unwrap(), facts(Some("managed")))
    };
    let controller = SteeringController::spawn(config, Arc::new(Handle(Arc::clone(&executor))));
    let submission = Submission {
        request: SteeringRequest {
            id: RequestId::random(),
            target: controller.snapshot().target.id(),
            origin: SteeringOrigin::Manual,
            operation: O::SteerActiveTurn,
            message: SteeringMessage::new(SECRET_MESSAGE).unwrap(),
            consent: None,
        },
        expected: Some(controller.snapshot().target),
        opportunity: None,
    };
    let reply = controller.submit(submission).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted);
    assert_eq!(reply.audit_failures.len(), 2, "request and result records both failed");
    assert_eq!(executor.delivered().len(), 1, "exactly one delivery");
}

#[test]
fn random_identifiers_are_canonical_and_distinct() {
    let first = ExecutionId::random();
    let second = ExecutionId::random();
    assert_ne!(first, second);
    assert_eq!(first.to_string().parse::<ExecutionId>().unwrap(), first);
    assert_eq!(RequestId::random().to_string().len(), 36);
}
