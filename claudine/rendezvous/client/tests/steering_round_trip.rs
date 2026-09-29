//! Steering control over the real local endpoint: an owner registers on a
//! `SteeringControl` stream, a separate client lists and routes, and the
//! owner's reply comes back. Runs unchanged over a Unix-domain socket and a
//! Windows named pipe, whose ownership rules the daemon's transport already
//! enforces (see `local_round_trip.rs` and the daemon's transport tests).
//!
//! It also pins the storage boundary: a control registration is not a
//! replicated presence entry, and no message text reaches the daemon's
//! durable data directory.

use std::time::Duration;

use rendezvous_client::connect;
use rendezvous_core::local_endpoint::test_support::private_endpoint;
use rendezvous_core::{
    ListActiveSessionsRequest, ListManagedTargetsRequest, ManagedTargetInfo, RouteOutcome, RouteSteeringRequest,
    SteeringControlUp, SteeringDelivery, SteeringReply, TargetBinding, steering_control_down, steering_control_up,
};
use rendezvous_daemon::local_transport::spawn_local_server;
use rendezvous_daemon::server::DaemonConfig;
use tempfile::TempDir;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;

/// Unique bytes that must never reach durable storage.
const MARKER: &str = "steer-marker-5b1f0c7e do not persist";

fn binding() -> TargetBinding {
    TargetBinding {
        execution_id: "00000000-0000-0000-0000-00000000c0de".into(),
        wrapper_pid: 4242,
        wrapper_start: "1700000000".into(),
        generation: 0,
        conversation: None,
    }
}

fn info() -> ManagedTargetInfo {
    ManagedTargetInfo {
        binding: Some(binding()),
        provider: "pi".into(),
        state: "working".into(),
        availability: "unavailable".into(),
        reason: Some("fixture".into()),
        ..ManagedTargetInfo::default()
    }
}

fn frame(kind: steering_control_up::Kind) -> SteeringControlUp {
    SteeringControlUp { kind: Some(kind) }
}

fn files_containing(dir: &std::path::Path, needle: &[u8]) -> Vec<std::path::PathBuf> {
    let mut hits = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(path) = stack.pop() {
        if path.is_dir() {
            stack.extend(std::fs::read_dir(&path).unwrap().map(|entry| entry.unwrap().path()));
        } else if std::fs::read(&path).unwrap_or_default().windows(needle.len()).any(|window| window == needle) {
            hits.push(path);
        }
    }
    hits
}

#[tokio::test]
async fn an_owner_receives_a_routed_request_and_its_reply_returns() {
    let tmp = TempDir::new().expect("tempdir");
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let data_dir = tmp.path().join("data");
    let config = DaemonConfig::with_data_dir(data_dir.clone()).with_in_memory_projection().without_networking();
    let handle = spawn_local_server(endpoint.clone(), config).expect("spawn daemon");

    // Owner: register over its own connection.
    let (up, up_rx) = mpsc::channel(4);
    up.send(frame(steering_control_up::Kind::Register(info()))).await.unwrap();
    let mut owner = connect(&endpoint).await.expect("owner connects");
    let mut down = owner.steering_control(ReceiverStream::new(up_rx)).await.expect("control stream").into_inner();
    let accepted = down.message().await.unwrap().unwrap();
    assert!(matches!(accepted.kind, Some(steering_control_down::Kind::Accepted(_))));

    // A separate requester sees exactly that managed target.
    let mut requester = connect(&endpoint).await.expect("requester connects");
    let targets = requester.list_managed_targets(ListManagedTargetsRequest {}).await.unwrap().into_inner().targets;
    assert_eq!(targets, vec![info()]);
    // Control registration is not replicated presence.
    let presence = requester.list_active_sessions(ListActiveSessionsRequest {}).await.unwrap().into_inner();
    assert!(
        presence.hosts.iter().all(|host| !host.sessions_json.contains("c0de")),
        "a control registration never enters the sessions-active register"
    );

    let owner_task = tokio::spawn(async move {
        let delivery = match down.message().await.unwrap().unwrap().kind {
            Some(steering_control_down::Kind::Delivery(delivery)) => *delivery,
            other => panic!("expected a delivery, got {other:?}"),
        };
        assert_eq!(delivery.message, MARKER, "the owner receives the original text");
        up.send(frame(steering_control_up::Kind::Reply(SteeringReply {
            request_id: delivery.request_id,
            outcome: "accepted".into(),
            mechanism: Some("rpc-steer".into()),
            ..SteeringReply::default()
        })))
        .await
        .unwrap();
        (up, down)
    });

    let routed = requester
        .route_steering(RouteSteeringRequest {
            delivery: Some(SteeringDelivery {
                request_id: "00000000-0000-0000-0000-000000000001".into(),
                expected: Some(binding()),
                origin: "manual".into(),
                operation: "steer_active_turn".into(),
                message: MARKER.into(),
                consented_operation: None,
            }),
            deadline_ms: 5_000,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(RouteOutcome::try_from(routed.outcome).unwrap(), RouteOutcome::OwnerReplied);
    assert_eq!(routed.reply.unwrap().outcome, "accepted");

    // Closing the owner's stream removes its registration.
    let (up, down) = owner_task.await.unwrap();
    drop(up);
    drop(down);
    drop(owner);
    let mut remaining = Vec::new();
    for _ in 0..100 {
        remaining = requester.list_managed_targets(ListManagedTargetsRequest {}).await.unwrap().into_inner().targets;
        if remaining.is_empty() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert!(remaining.is_empty(), "disconnect cleans up the registration");
    let gone = requester
        .route_steering(RouteSteeringRequest {
            delivery: Some(SteeringDelivery {
                request_id: "00000000-0000-0000-0000-000000000002".into(),
                expected: Some(binding()),
                origin: "manual".into(),
                operation: "steer_active_turn".into(),
                message: MARKER.into(),
                consented_operation: None,
            }),
            deadline_ms: 1_000,
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(RouteOutcome::try_from(gone.outcome).unwrap(), RouteOutcome::NoOwner);

    drop(requester);
    handle.shutdown().await.expect("shutdown");
    assert!(data_dir.is_dir(), "the daemon opened its durable data directory");
    assert_eq!(files_containing(&data_dir, MARKER.as_bytes()), Vec::<std::path::PathBuf>::new());
}

/// A control stream stays open for an owner's whole execution. Shutting the
/// daemon down must end it rather than wait on it.
#[tokio::test]
async fn shutdown_completes_while_an_owner_is_still_connected() {
    let tmp = TempDir::new().expect("tempdir");
    let endpoint = private_endpoint(tmp.path(), "daemon");
    let config = DaemonConfig::with_data_dir(tmp.path().join("data")).with_in_memory_projection().without_networking();
    let handle = spawn_local_server(endpoint.clone(), config).expect("spawn daemon");

    let (up, up_rx) = mpsc::channel(4);
    up.send(frame(steering_control_up::Kind::Register(info()))).await.unwrap();
    let mut owner = connect(&endpoint).await.expect("owner connects");
    let mut down = owner.steering_control(ReceiverStream::new(up_rx)).await.expect("control stream").into_inner();
    assert!(matches!(down.message().await.unwrap().unwrap().kind, Some(steering_control_down::Kind::Accepted(_))));

    tokio::time::timeout(Duration::from_secs(10), handle.shutdown())
        .await
        .expect("shutdown must not wait on an open control stream")
        .expect("shutdown");
    let ended = tokio::time::timeout(Duration::from_secs(5), down.message()).await.expect("the owner sees its stream end");
    assert!(!matches!(ended, Ok(Some(_))), "no frame arrives after shutdown");
    drop(up);
}
