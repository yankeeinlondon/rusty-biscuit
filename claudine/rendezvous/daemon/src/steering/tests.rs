//! Router behavior with in-process channels standing in for owner streams:
//! registration, listing, exact routing, stale and duplicate refusal, busy,
//! disconnect, deadline, and reconnection without replay.

use super::*;
use rendezvous_core::{SteeringDelivery, steering_control_down::Kind};

const EXECUTION: &str = "00000000-0000-0000-0000-0000000000e0";

fn binding(generation: u64, start: &str) -> TargetBinding {
    TargetBinding {
        execution_id: EXECUTION.into(),
        wrapper_pid: 4242,
        wrapper_start: start.into(),
        generation,
        conversation: Some("conversation-a".into()),
    }
}

fn info(generation: u64) -> ManagedTargetInfo {
    ManagedTargetInfo {
        binding: Some(binding(generation, "1700000000")),
        provider: "pi".into(),
        state: "working".into(),
        availability: "unavailable".into(),
        ..ManagedTargetInfo::default()
    }
}

fn route_request(request_id: &str, expected: TargetBinding, deadline_ms: u32) -> RouteSteeringRequest {
    RouteSteeringRequest {
        delivery: Some(SteeringDelivery {
            request_id: request_id.into(),
            expected: Some(expected),
            origin: "manual".into(),
            operation: "steer_active_turn".into(),
            message: "recheck the failing test".into(),
            consented_operation: None,
        }),
        deadline_ms,
    }
}

fn reply(request_id: &str, outcome: &str) -> SteeringReply {
    SteeringReply { request_id: request_id.into(), outcome: outcome.into(), mechanism: Some("rpc-steer".into()), ..SteeringReply::default() }
}

type Frames = mpsc::Receiver<Result<SteeringControlDown, Status>>;

fn owner(router: &SteeringRouter, capacity: usize) -> (Registration, Frames) {
    let (tx, rx) = mpsc::channel(capacity);
    (router.register(info(0), tx).expect("register"), rx)
}

fn delivery_id(frame: Result<SteeringControlDown, Status>) -> String {
    match frame.unwrap().kind {
        Some(Kind::Delivery(delivery)) => delivery.request_id,
        other => panic!("expected a delivery, got {other:?}"),
    }
}

fn outcome(response: &RouteSteeringResponse) -> RouteOutcome {
    RouteOutcome::try_from(response.outcome).unwrap()
}

#[tokio::test]
async fn a_request_reaches_exactly_its_owner_and_returns_its_reply() {
    let router = SteeringRouter::default();
    let (registration, mut frames) = owner(&router, OWNER_CHANNEL_CAPACITY);
    let other_router_owner = {
        let (tx, rx) = mpsc::channel(OWNER_CHANNEL_CAPACITY);
        let mut other = info(0);
        other.binding.as_mut().unwrap().execution_id = "00000000-0000-0000-0000-0000000000e1".into();
        (router.register(other, tx).unwrap(), rx)
    };
    assert_eq!(router.list().len(), 2);

    let responder = {
        let router = router.clone();
        tokio::spawn(async move {
            let id = delivery_id(frames.recv().await.unwrap());
            router.complete(&registration, reply(&id, "accepted"));
            frames
        })
    };
    let response = router.route(route_request("r-1", binding(0, "1700000000"), 1_000)).await.unwrap();
    assert_eq!(outcome(&response), RouteOutcome::OwnerReplied);
    let owner_reply = response.reply.unwrap();
    assert_eq!(owner_reply.outcome, "accepted", "the owner's verdict passes through unchanged");
    assert_eq!(owner_reply.mechanism.as_deref(), Some("rpc-steer"));

    let (_, mut other_frames) = other_router_owner;
    assert!(other_frames.try_recv().is_err(), "no other owner saw the request");
    drop(responder.await.unwrap());
}

#[tokio::test]
async fn no_owner_stale_binding_and_duplicates_send_nothing() {
    let router = SteeringRouter::default();
    let missing = router.route(route_request("r-0", binding(0, "1700000000"), 1_000)).await.unwrap();
    assert_eq!(outcome(&missing), RouteOutcome::NoOwner);

    let (registration, mut frames) = owner(&router, OWNER_CHANNEL_CAPACITY);
    let replaced = router.route(route_request("r-1", binding(1, "1700000000"), 1_000)).await.unwrap();
    assert_eq!(outcome(&replaced), RouteOutcome::StaleTarget);
    assert_eq!(replaced.detail, "the provider conversation was replaced");
    // Same PID, new start marker: the PID was reused by another wrapper.
    let reused = router.route(route_request("r-2", binding(0, "1800000000"), 1_000)).await.unwrap();
    assert_eq!(reused.detail, "the owning wrapper process changed");
    assert!(frames.try_recv().is_err(), "refused requests never reach the owner");

    // Route once; the same request ID is refused afterwards.
    let responder = {
        let router = router.clone();
        tokio::spawn(async move {
            let id = delivery_id(frames.recv().await.unwrap());
            router.complete(&registration, reply(&id, "unknown"));
            frames
        })
    };
    let first = router.route(route_request("r-3", binding(0, "1700000000"), 1_000)).await.unwrap();
    assert_eq!(outcome(&first), RouteOutcome::OwnerReplied);
    let mut frames = responder.await.unwrap();
    let again = router.route(route_request("r-3", binding(0, "1700000000"), 1_000)).await.unwrap();
    assert_eq!(outcome(&again), RouteOutcome::DuplicateRequest);
    assert!(frames.try_recv().is_err(), "a duplicate correlation ID is never replayed");
}

#[tokio::test]
async fn a_full_owner_channel_is_busy() {
    let router = SteeringRouter::default();
    let (_registration, _frames) = owner(&router, 1);
    let blocked = {
        let router = router.clone();
        tokio::spawn(async move { router.route(route_request("r-1", binding(0, "1700000000"), 5_000)).await })
    };
    tokio::task::yield_now().await;
    let busy = router.route(route_request("r-2", binding(0, "1700000000"), 5_000)).await.unwrap();
    assert_eq!(outcome(&busy), RouteOutcome::Busy);
    blocked.abort();
}

#[tokio::test]
async fn an_owner_disconnect_resolves_its_in_flight_requests_as_unknown() {
    let router = SteeringRouter::default();
    let (registration, mut frames) = owner(&router, OWNER_CHANNEL_CAPACITY);
    let pending = {
        let router = router.clone();
        tokio::spawn(async move { router.route(route_request("r-1", binding(0, "1700000000"), 5_000)).await })
    };
    delivery_id(frames.recv().await.unwrap());
    router.unregister(&registration);

    let response = pending.await.unwrap().unwrap();
    assert_eq!(outcome(&response), RouteOutcome::Unknown);
    assert!(response.detail.contains("may have been submitted"));
    assert!(router.list().is_empty(), "disconnect removes the registration");
}

#[tokio::test]
async fn no_reply_before_the_deadline_is_unknown_and_a_late_reply_goes_nowhere() {
    let router = SteeringRouter::default();
    let (registration, mut frames) = owner(&router, OWNER_CHANNEL_CAPACITY);
    let response = router.route(route_request("r-1", binding(0, "1700000000"), 50)).await.unwrap();
    assert_eq!(outcome(&response), RouteOutcome::Unknown);
    assert!(response.detail.contains("before the deadline"));

    // The owner answers after the requester gave up; nothing is waiting.
    let id = delivery_id(frames.recv().await.unwrap());
    router.complete(&registration, reply(&id, "accepted"));
    assert!(frames.try_recv().is_err());
}

#[tokio::test]
async fn a_reconnected_owner_never_receives_the_previous_connections_requests() {
    let router = SteeringRouter::default();
    let (first, mut first_frames) = owner(&router, OWNER_CHANNEL_CAPACITY);
    let pending = {
        let router = router.clone();
        tokio::spawn(async move { router.route(route_request("r-1", binding(0, "1700000000"), 5_000)).await })
    };
    delivery_id(first_frames.recv().await.unwrap());
    router.unregister(&first);
    assert_eq!(outcome(&pending.await.unwrap().unwrap()), RouteOutcome::Unknown);

    let (second, mut second_frames) = owner(&router, OWNER_CHANNEL_CAPACITY);
    assert_ne!(first, second);
    assert!(second_frames.try_recv().is_err(), "nothing is replayed on reconnection");
    // A late reply on the old connection cannot resolve anything now.
    router.complete(&first, reply("r-1", "accepted"));
    router.unregister(&first);
    assert_eq!(router.list().len(), 1, "the stale connection cannot remove its successor");
}

#[tokio::test]
async fn registrations_and_updates_are_validated() {
    let router = SteeringRouter::default();
    let (tx, _rx) = mpsc::channel(1);
    let mut missing = info(0);
    missing.binding = None;
    assert_eq!(router.register(missing, tx.clone()).unwrap_err(), RegisterError::Missing("binding"));
    let mut no_start = info(0);
    no_start.binding.as_mut().unwrap().wrapper_start.clear();
    assert_eq!(router.register(no_start, tx.clone()).unwrap_err(), RegisterError::Missing("binding.wrapper_start"));

    let registration = router.register(info(3), tx.clone()).unwrap();
    assert_eq!(router.register(info(3), tx).unwrap_err(), RegisterError::AlreadyRegistered);
    assert_eq!(router.update(&registration, info(2)).unwrap_err(), RegisterError::GenerationRegressed);
    let mut moved = info(4);
    moved.binding.as_mut().unwrap().wrapper_start = "1800000000".into();
    assert_eq!(router.update(&registration, moved).unwrap_err(), RegisterError::IdentityChanged);

    let mut idle = info(4);
    idle.state = "idle".into();
    router.update(&registration, idle).unwrap();
    let listed = router.list();
    assert_eq!(listed[0].state, "idle");
    assert_eq!(listed[0].binding.as_ref().unwrap().generation, 4);
}

#[tokio::test]
async fn malformed_route_requests_are_rejected() {
    let router = SteeringRouter::default();
    for deadline in [0, MAX_ROUTE_DEADLINE_MS + 1] {
        let error = router.route(route_request("r", binding(0, "1"), deadline)).await.unwrap_err();
        assert_eq!(error.code(), tonic::Code::InvalidArgument);
    }
    let error = router.route(RouteSteeringRequest { delivery: None, deadline_ms: 10 }).await.unwrap_err();
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
    let error = router.route(route_request("", binding(0, "1"), 10)).await.unwrap_err();
    assert_eq!(error.code(), tonic::Code::InvalidArgument);
}

#[tokio::test]
async fn closing_ends_every_owner_stream_and_refuses_new_registrations() {
    let router = SteeringRouter::default();
    let (_registration, mut frames) = owner(&router, OWNER_CHANNEL_CAPACITY);
    let pending = {
        let router = router.clone();
        tokio::spawn(async move { router.route(route_request("r-1", binding(0, "1700000000"), 5_000)).await })
    };
    delivery_id(frames.recv().await.unwrap());

    router.close();
    assert!(frames.recv().await.is_none(), "the owner's stream ends");
    assert_eq!(outcome(&pending.await.unwrap().unwrap()), RouteOutcome::Unknown);
    let (tx, _rx) = mpsc::channel(1);
    assert_eq!(router.register(info(0), tx).unwrap_err(), RegisterError::Closed);
    assert!(router.list().is_empty());
}
