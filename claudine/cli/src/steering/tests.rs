//! Managed steering through the wrapper's owner link and the requester.
//!
//! - Wire matrices: every load-bearing registration, delivery, and reply
//!   field is edited one cell at a time from a control value that reads.
//! - Without a daemon: routing is unavailable, listing fails fast, and the
//!   owner still delivers in-owner automatic help.
//! - With a daemon (`daemon-tests`): exact routing, stale selection,
//!   disconnect cleanup, restart without replay, and serialized senders over
//!   the real local endpoint.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use claudine::provider::Provider;
use claudine::steering::audit::{DeliveryReport, SteeringAuditLog};
use claudine::steering::contract::{InterruptionConsent, SendOutcome, SteeringMessage, SteeringOrigin, SteeringRequest, SteeringResult};
use claudine::steering::controller::{
    ControllerConfig, ControllerReply, DeliveryDeadlines, DeliveryFuture, EligibilityFn, ExecutionFacts, SteeringExecutor,
    Submission, UNMAPPED_PROFILE_REASON,
};
use claudine::steering::eligibility::{AutomaticEligibility, Eligibility, ManualEligibility, Route};
use claudine::steering::identity::{ExecutionId, ManagedTarget, OpportunityId, ProcessStartIdentity, RequestId};
use claudine::steering::vocabulary::{
    AdapterRef, CaseSupport, ExecutionState, LaunchMode, OperationIntent, ReceiptStrength, SteeringAvailability,
    SteeringMechanism,
};
use rendezvous_core::local_endpoint::test_support::{endpoint_env_value, private_endpoint};
use rendezvous_core::local_endpoint::{ENDPOINT_ENV_VAR, LocalEndpoint};
use rendezvous_core::{ManagedTargetInfo, SteeringDelivery, SteeringReply};

use super::owner::ExecutionSteering;
use super::{requester, wire};

const MESSAGE: &str = "Recheck the failing test before editing; token=ghp_0123456789abcdefghij0123";

fn rpc_steer() -> &'static SteeringMechanism {
    claudine::steering::facts(Provider::Pi).mechanisms.iter().find(|m| m.id == "rpc-steer").expect("researched mechanism")
}

/// Eligibility that always offers Pi's researched `rpc-steer`, so routing is
/// exercised without an activation grant.
fn always_routable() -> EligibilityFn {
    Arc::new(|_| {
        let route = Route { mechanism: rpc_steer(), adapter: AdapterRef { id: "fixture", revision: 1 }, support: CaseSupport::NonInterrupting };
        Eligibility {
            manual: ManualEligibility { availability: SteeringAvailability::NonInterrupting, route: Some(route), blockers: Vec::new() },
            automatic: AutomaticEligibility::Eligible(route),
        }
    })
}

#[derive(Default)]
struct Recording {
    delivered: Mutex<Vec<(RequestId, String)>>,
    in_flight: AtomicUsize,
    max_in_flight: AtomicUsize,
    delay: Duration,
}

impl Recording {
    fn count(&self) -> usize {
        self.delivered.lock().unwrap().len()
    }
}

struct Executor(Arc<Recording>);

impl SteeringExecutor for Executor {
    fn deliver(&self, request: SteeringRequest, route: Route, _deadlines: DeliveryDeadlines) -> DeliveryFuture {
        let recording = Arc::clone(&self.0);
        Box::pin(async move {
            let now = recording.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
            recording.max_in_flight.fetch_max(now, Ordering::SeqCst);
            recording.delivered.lock().unwrap().push((request.id, request.message.as_str().to_string()));
            tokio::time::sleep(recording.delay).await;
            recording.in_flight.fetch_sub(1, Ordering::SeqCst);
            DeliveryReport::new(SteeringResult::submitted(&request, Some(route.mechanism.id), SendOutcome::Accepted))
        })
    }
}

fn facts() -> ExecutionFacts {
    ExecutionFacts {
        provider: Provider::Pi,
        profile_id: Some("retained-rpc".into()),
        os: claudine::steering::host_os(),
        launch_mode: LaunchMode::NonInteractive,
        provider_version: Some("0.84.4".into()),
        cwd: Some("/work/project".into()),
        name: Some("fixture".into()),
    }
}

fn owner(audit: &tempfile::TempDir, recording: &Arc<Recording>) -> ExecutionSteering {
    let config = ControllerConfig {
        execution: ExecutionId::random(),
        wrapper: ProcessStartIdentity::new(std::process::id(), "fixture-start").unwrap(),
        facts: facts(),
        initial_state: ExecutionState::Working,
        eligibility: always_routable(),
        audit: SteeringAuditLog::at(audit.path()),
    };
    ExecutionSteering::start_with(config, Arc::new(Executor(Arc::clone(recording))))
}

fn manual_request(target: &ManagedTarget) -> SteeringRequest {
    SteeringRequest {
        id: RequestId::random(),
        target: target.id(),
        origin: SteeringOrigin::Manual,
        operation: OperationIntent::SteerActiveTurn,
        message: SteeringMessage::new(MESSAGE).unwrap(),
        consent: None,
    }
}

/// Points this test process's endpoint resolution at `endpoint`. nextest
/// runs each test in its own process, so this cannot race another test.
fn point_endpoint_at(endpoint: &LocalEndpoint) {
    // SAFETY: see above — this process is this test's alone.
    unsafe { std::env::set_var(ENDPOINT_ENV_VAR, endpoint_env_value(endpoint)) };
}

// ---------------------------------------------------------------------------
// Wire matrices
// ---------------------------------------------------------------------------

/// One matrix cell: its name and the single edit it makes to the control.
type Cell<T> = (&'static str, Box<dyn Fn(&mut T)>);

fn control_info() -> ManagedTargetInfo {
    let target = ManagedTarget {
        execution: ExecutionId::from_u128(0xC0DE),
        wrapper: ProcessStartIdentity::new(10, "100").unwrap(),
        generation: claudine::steering::identity::ConversationGeneration(2),
        conversation: Some("conversation:with:colons".into()),
    };
    ManagedTargetInfo {
        binding: Some(wire::binding_to_wire(&target)),
        provider: "kimi".into(),
        cwd: Some("/work".into()),
        name: None,
        state: "idle".into(),
        launch_profile: Some("managed".into()),
        provider_version: None,
        availability: "interruption_required".into(),
        reason: Some("delivery requires interrupting the running turn first".into()),
        setup_requirements: vec!["start with --rpc".into()],
        provider_pid: Some(77),
        provider_start: Some("700".into()),
        observed_at_unix_ms: 1_760_000_000_000,
    }
}

#[test]
fn registration_matrix_reads_every_field_strictly() {
    let listing = wire::info_to_listing(control_info()).expect("control registration reads");
    assert_eq!(listing.id.to_string(), "managed:00000000-0000-0000-0000-00000000c0de");
    assert_eq!(listing.provider, Provider::KimiCode);
    assert_eq!(listing.state, ExecutionState::Idle);
    assert_eq!(listing.availability.availability, SteeringAvailability::InterruptionRequired);
    assert_eq!(listing.availability.setup_requirements, ["start with --rpc"]);
    assert_eq!(
        listing.session_key,
        Some((ProcessStartIdentity::new(77, "700").unwrap(), "conversation:with:colons".to_string()))
    );

    let cells: Vec<Cell<ManagedTargetInfo>> = vec![
        ("binding absent", Box::new(|i| i.binding = None)),
        ("execution id empty", Box::new(|i| i.binding.as_mut().unwrap().execution_id.clear())),
        ("execution id uppercase", Box::new(|i| i.binding.as_mut().unwrap().execution_id.make_ascii_uppercase())),
        ("wrapper start empty", Box::new(|i| i.binding.as_mut().unwrap().wrapper_start.clear())),
        ("wrapper start with colon", Box::new(|i| i.binding.as_mut().unwrap().wrapper_start = "1:2".into())),
        ("provider empty", Box::new(|i| i.provider.clear())),
        ("provider not a slug", Box::new(|i| i.provider = "kimi_code".into())),
        ("state empty", Box::new(|i| i.state.clear())),
        ("state unknown value", Box::new(|i| i.state = "asleep".into())),
        ("availability empty", Box::new(|i| i.availability.clear())),
        ("availability unknown value", Box::new(|i| i.availability = "maybe".into())),
        ("provider pid without start", Box::new(|i| i.provider_start = None)),
        ("provider start without pid", Box::new(|i| i.provider_pid = None)),
        ("observed time out of range", Box::new(|i| i.observed_at_unix_ms = i64::MAX)),
    ];
    for (cell, edit) in cells {
        let mut info = control_info();
        edit(&mut info);
        assert!(wire::info_to_listing(info).is_err(), "{cell} must be rejected, never defaulted");
    }
    // Absent optional facts stay explicit unknowns.
    let sparse = ManagedTargetInfo { provider_pid: None, provider_start: None, cwd: None, ..control_info() };
    let listing = wire::info_to_listing(sparse).unwrap();
    assert_eq!((listing.cwd, listing.session_key), (None, None));
}

fn control_delivery() -> SteeringDelivery {
    let target = wire::binding_from_wire(control_info().binding.as_ref().unwrap()).unwrap();
    let request = SteeringRequest {
        consent: Some(InterruptionConsent { target: target.id(), operation: OperationIntent::InterruptThenSubmit }),
        operation: OperationIntent::InterruptThenSubmit,
        ..manual_request(&target)
    };
    wire::request_to_delivery(&request, &target)
}

#[test]
fn delivery_matrix_refuses_anything_it_cannot_read_exactly() {
    let submission = wire::delivery_to_submission(control_delivery()).expect("control delivery reads");
    assert_eq!(submission.request.message.as_str(), MESSAGE, "the original bytes reach the owner");
    assert!(submission.request.may_interrupt(), "consent stays bound to the selected target and operation");
    assert_eq!(submission.expected.unwrap().generation.0, 2);

    let cells: Vec<Cell<SteeringDelivery>> = vec![
        ("request id empty", Box::new(|d| d.request_id.clear())),
        ("request id prefix", Box::new(|d| d.request_id.truncate(8))),
        ("expected absent", Box::new(|d| d.expected = None)),
        ("origin automatic", Box::new(|d| d.origin = "automatic".into())),
        ("origin unknown", Box::new(|d| d.origin = "robot".into())),
        ("operation unknown", Box::new(|d| d.operation = "shout".into())),
        ("operation empty", Box::new(|d| d.operation.clear())),
        ("message whitespace", Box::new(|d| d.message = " \n".into())),
        ("message with NUL", Box::new(|d| d.message = "a\0b".into())),
        ("message too long", Box::new(|d| d.message = "x".repeat(64 * 1024 + 1))),
        ("consent unknown", Box::new(|d| d.consented_operation = Some("everything".into()))),
    ];
    for (cell, edit) in cells {
        let mut delivery = control_delivery();
        edit(&mut delivery);
        let error = wire::delivery_to_submission(delivery).expect_err(cell);
        let refusal = wire::refusal("r", &error);
        assert_eq!(refusal.outcome, "refused", "{cell}: an unreadable request is refused, never submitted");
        assert!(!refusal.detail.unwrap().contains("ghp_"), "{cell}: refusals never echo the message");
    }
}

#[test]
fn reply_matrix_never_upgrades_or_misattributes_a_result() {
    let target = wire::binding_from_wire(control_info().binding.as_ref().unwrap()).unwrap();
    let request = manual_request(&target);
    let owner_reply = ControllerReply {
        result: SteeringResult::submitted(&request, Some("rpc-steer"), SendOutcome::Accepted),
        detail: None,
        audit_failures: Vec::new(),
    };
    let control = wire::reply_to_wire(&request.id.to_string(), &owner_reply);
    let result = wire::reply_from_wire(&request, Provider::Pi, &control).expect("control reply reads");
    assert_eq!(result, owner_reply.result);
    assert_eq!(result.receipt, ReceiptStrength::Accepted);

    let cells: Vec<Cell<SteeringReply>> = vec![
        ("another request", Box::new(|r| r.request_id = RequestId::random().to_string())),
        ("outcome unknown", Box::new(|r| r.outcome = "maybe".into())),
        ("outcome empty", Box::new(|r| r.outcome.clear())),
        ("mechanism not researched", Box::new(|r| r.mechanism = Some("telepathy".into()))),
        ("replacement without cancellation", Box::new(|r| r.replacement = Some("accepted".into()))),
        ("cancellation unknown", Box::new(|r| r.cancellation = Some("sort_of".into()))),
    ];
    for (cell, edit) in cells {
        let mut reply = control.clone();
        edit(&mut reply);
        assert!(wire::reply_from_wire(&request, Provider::Pi, &reply).is_err(), "{cell} must not be trusted");
    }
}

// ---------------------------------------------------------------------------
// No daemon
// ---------------------------------------------------------------------------

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_daemon_routing_is_unavailable_and_listing_fails_fast() {
    let tmp = tempfile::tempdir().unwrap();
    point_endpoint_at(&private_endpoint(tmp.path(), "absent"));
    let target = ManagedTarget {
        execution: ExecutionId::random(),
        wrapper: ProcessStartIdentity::new(1, "1").unwrap(),
        generation: claudine::steering::identity::ConversationGeneration(0),
        conversation: None,
    };
    let started = std::time::Instant::now();
    let request = manual_request(&target);
    let routed = requester::route_to_owner(&request, &target, Provider::Pi).await;
    assert_eq!(routed.result.outcome, SendOutcome::Unavailable);
    assert!(matches!(routed.failure, Some(requester::RouteFailure::NoRoute(super::DaemonAccessError::Unreachable(_)))));
    assert!(routed.detail(&request).unwrap().as_str().starts_with("no local steering route: the local Rendezvous daemon is not reachable"));

    let managed: Arc<dyn claudine::steering::discovery::ManagedSource> = Arc::new(requester::DaemonManagedSource);
    let failure = claudine::steering::discovery::discover(Some(managed), &[]).await.unwrap_err();
    assert_eq!(failure.errors[0].source, "managed");
    assert!(failure.errors[0].cause().downcast_ref::<super::DaemonAccessError>().is_some(), "the typed cause is kept");
    assert!(started.elapsed() < Duration::from_secs(3), "a missing daemon fails fast");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn without_a_daemon_the_owner_still_delivers_automatic_help_directly() {
    let tmp = tempfile::tempdir().unwrap();
    point_endpoint_at(&private_endpoint(tmp.path(), "absent"));
    let recording = Arc::new(Recording::default());
    let steering = owner(&tmp, &recording);
    let controller = steering.controller();
    let submission = Submission {
        request: SteeringRequest {
            origin: SteeringOrigin::Automatic,
            ..manual_request(&controller.snapshot().target)
        },
        expected: None,
        opportunity: Some(OpportunityId::random()),
    };
    let reply = controller.submit(submission).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted);
    assert_eq!(recording.count(), 1);
}

#[tokio::test(start_paused = true)]
async fn without_a_daemon_the_link_gives_up_after_bounded_attempts() {
    let tmp = tempfile::tempdir().unwrap();
    point_endpoint_at(&private_endpoint(tmp.path(), "absent"));
    let recording = Arc::new(Recording::default());
    let steering = owner(&tmp, &recording);
    let started = tokio::time::Instant::now();
    while !steering.link_is_finished() {
        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(started.elapsed() < Duration::from_secs(60), "the link must stop retrying");
    }
    // 1 + 2 + 4 + 8 seconds of backoff between five attempts.
    assert!(started.elapsed() >= Duration::from_secs(15));
    let request = manual_request(&steering.controller().snapshot().target);
    let reply = steering.controller().submit(Submission { expected: None, opportunity: None, request }).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted, "the owner keeps working without a route");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_wrapped_child_registers_as_unavailable_until_its_profile_is_mapped() {
    let tmp = tempfile::tempdir().unwrap();
    point_endpoint_at(&private_endpoint(tmp.path(), "absent"));
    let steering = ExecutionSteering::for_wrapped_child(Provider::Codex, false, tmp.path()).expect("wrapper identity");
    let snapshot = steering.controller().snapshot();
    assert_eq!(snapshot.availability.availability, SteeringAvailability::Unavailable);
    assert_eq!(snapshot.availability.reason.as_deref(), Some(UNMAPPED_PROFILE_REASON));
    assert_eq!(snapshot.target.wrapper.pid, std::process::id(), "bound to this wrapper process");
    assert_eq!(snapshot.facts.launch_mode, LaunchMode::NonInteractive);
    assert_eq!(snapshot.facts.cwd.as_deref(), Some(biscuit_file::to_portable_string(tmp.path()).as_str()));
}

// ---------------------------------------------------------------------------
// With a daemon
// ---------------------------------------------------------------------------

#[cfg(feature = "daemon-tests")]
mod with_daemon {
    use super::*;
    use claudine::steering::discovery::{ManagedSource, ObservationSource, SessionListing};

    async fn boot(tmp: &tempfile::TempDir, endpoint: &LocalEndpoint) -> rendezvous_daemon::server::ServerHandle {
        let mut config = rendezvous_daemon::server::DaemonConfig::with_data_dir(tmp.path().join("data")).with_in_memory_projection();
        config.networking = None;
        let handle = rendezvous_daemon::local_transport::spawn_local_server(endpoint.clone(), config).expect("spawn daemon");
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while rendezvous_client::connect(endpoint).await.is_err() {
            assert!(std::time::Instant::now() < deadline, "daemon never accepted a connection");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
        handle
    }

    /// Waits until the daemon lists a managed row satisfying `ready`.
    async fn listed(ready: impl Fn(&SessionListing) -> bool) -> SessionListing {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        loop {
            if let Ok(listing) = requester::DaemonManagedSource.list().await
                && let Some(row) = listing.sessions.into_iter().find(|row| ready(row))
            {
                return row;
            }
            assert!(std::time::Instant::now() < deadline, "the owner never appeared in the listing");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    async fn unlisted() {
        let deadline = std::time::Instant::now() + Duration::from_secs(10);
        while !requester::DaemonManagedSource.list().await.unwrap().sessions.is_empty() {
            assert!(std::time::Instant::now() < deadline, "the registration was never removed");
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    }

    fn expected(row: &SessionListing, owner: &ExecutionSteering) -> ManagedTarget {
        let target = owner.controller().snapshot().target;
        assert_eq!(row.id, target.id());
        target
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_routed_request_reaches_exactly_its_owner_and_its_receipt_returns() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        let recording = Arc::new(Recording::default());
        let other_recording = Arc::new(Recording::default());
        let steering = owner(&tmp, &recording);
        let _other = owner(&tmp, &other_recording);

        let own_id = steering.controller().snapshot().target.id();
        let row = listed(|row| row.id == own_id).await;
        assert_eq!(row.origins, [ObservationSource::Managed]);
        assert_eq!(row.availability.availability, SteeringAvailability::NonInterrupting, "the owner's verdict is listed");
        assert_eq!(row.provider, Provider::Pi);

        let target = expected(&row, &steering);
        let request = manual_request(&target);
        let routed = requester::route_to_owner(&request, &target, Provider::Pi).await;
        assert_eq!(routed.result.outcome, SendOutcome::Accepted);
        assert_eq!(routed.result.receipt, ReceiptStrength::Accepted);
        assert_eq!(routed.result.mechanism, Some("rpc-steer"));
        assert_eq!(recording.delivered.lock().unwrap().clone(), [(request.id, MESSAGE.to_string())]);
        assert_eq!(other_recording.count(), 0, "no other owner saw the request");
        daemon.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_stale_selection_is_rejected_and_never_reaches_the_provider() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        let recording = Arc::new(Recording::default());
        let steering = owner(&tmp, &recording);
        let own_id = steering.controller().snapshot().target.id();
        let target = expected(&listed(|row| row.id == own_id).await, &steering);

        steering.controller().set_conversation(Some("replacement".into()));
        let request = manual_request(&target);
        let routed = requester::route_to_owner(&request, &target, Provider::Pi).await;
        assert_eq!(routed.result.outcome, SendOutcome::Unavailable);
        assert_eq!(
            routed.detail(&request).unwrap().as_str(),
            "the selected target changed: the provider conversation was replaced",
            "stale either at the daemon or at the owner, never retargeted"
        );
        assert_eq!(recording.count(), 0);
        daemon.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn dropping_the_owner_removes_its_route() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        let recording = Arc::new(Recording::default());
        let steering = owner(&tmp, &recording);
        let own_id = steering.controller().snapshot().target.id();
        let target = expected(&listed(|row| row.id == own_id).await, &steering);

        drop(steering);
        unlisted().await;
        let request = manual_request(&target);
        let routed = requester::route_to_owner(&request, &target, Provider::Pi).await;
        assert_eq!(routed.result.outcome, SendOutcome::Unavailable);
        assert_eq!(routed.detail(&request).unwrap().as_str(), "no live owner is registered for this execution");
        daemon.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_restarted_daemon_gets_a_fresh_registration_and_nothing_is_replayed() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        let recording = Arc::new(Recording::default());
        let steering = owner(&tmp, &recording);
        let own_id = steering.controller().snapshot().target.id();
        let target = expected(&listed(|row| row.id == own_id).await, &steering);
        let first = manual_request(&target);
        assert_eq!(requester::route_to_owner(&first, &target, Provider::Pi).await.result.outcome, SendOutcome::Accepted);

        daemon.shutdown().await.unwrap();
        let routed = requester::route_to_owner(&manual_request(&target), &target, Provider::Pi).await;
        assert_eq!(routed.result.outcome, SendOutcome::Unavailable, "daemon failure is a missing route, not a task failure");

        let restarted = boot(&tmp, &endpoint).await;
        listed(|row| row.id == own_id).await;
        assert_eq!(recording.count(), 1, "reconnection replays nothing");
        // Repeating the old correlation ID is refused, not redelivered.
        let replayed = requester::route_to_owner(&first, &target, Provider::Pi).await;
        assert_eq!(replayed.result.outcome, SendOutcome::Refused);
        assert_eq!(recording.count(), 1);
        let fresh = requester::route_to_owner(&manual_request(&target), &target, Provider::Pi).await;
        assert_eq!(fresh.result.outcome, SendOutcome::Accepted);
        assert_eq!(recording.count(), 2);
        restarted.shutdown().await.unwrap();
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn concurrent_senders_through_the_daemon_are_serialized() {
        let tmp = tempfile::tempdir().unwrap();
        let endpoint = private_endpoint(tmp.path(), "daemon");
        point_endpoint_at(&endpoint);
        let daemon = boot(&tmp, &endpoint).await;
        let recording = Arc::new(Recording { delay: Duration::from_millis(20), ..Recording::default() });
        let steering = owner(&tmp, &recording);
        let own_id = steering.controller().snapshot().target.id();
        let target = expected(&listed(|row| row.id == own_id).await, &steering);

        let sends: Vec<_> = (0..8)
            .map(|_| {
                let target = target.clone();
                tokio::spawn(async move { requester::route_to_owner(&manual_request(&target), &target, Provider::Pi).await })
            })
            .collect();
        for send in sends {
            assert_eq!(send.await.unwrap().result.outcome, SendOutcome::Accepted);
        }
        assert_eq!(recording.count(), 8);
        assert_eq!(recording.max_in_flight.load(Ordering::SeqCst), 1, "provider mutations never overlap");
        daemon.shutdown().await.unwrap();
    }
}
