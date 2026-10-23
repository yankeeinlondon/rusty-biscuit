//! The client supervisor's inbound idle timeout (`receive_timeout`) and its
//! responsiveness while the inbound event queue is full or writes are
//! stalled, observed from a loopback mock server.

#![allow(clippy::result_large_err)]

mod ws_mock;

use std::time::{Duration, Instant};

use schematic_schema::unfolded_circle_core_ws::{CoreWsClient, UnfoldedCircleCoreWs};
use schematic_schema::ws_shared::{
    ReconnectPolicy, WsClientOptions, WsClientOptionsBuilder, WsConnectionState, WsError,
};
use serde_json::{Value, json};
use ws_mock::{Barrier, MockWsServer, Script, Step};

/// Generous against a loaded runner; every wait ends on an event well before it.
const WAIT: Duration = Duration::from_secs(5);
/// The idle timeout under test.
const IDLE: Duration = Duration::from_millis(400);
/// A window in which a live connection must stay quiet.
const QUIET: Duration = Duration::from_millis(600);
/// A send not accepted within this long is waiting on a full outbound queue.
const STALL: Duration = Duration::from_millis(500);
/// Upper bound on filler frames; stalling needs a few MiB of kernel buffers.
const MAX_FILL: usize = 2048;

fn builder() -> WsClientOptionsBuilder {
    WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(2))
        .request_timeout(Duration::from_secs(5))
}

fn reconnect(backoff: Duration) -> ReconnectPolicy {
    ReconnectPolicy {
        initial_backoff: backoff,
        max_backoff: backoff,
        multiplier: 1.0,
        jitter: 0.0,
        max_attempts: Some(5),
    }
}

async fn connect(origin: &str, options: WsClientOptions) -> CoreWsClient {
    UnfoldedCircleCoreWs::with_base_url(origin)
        .api_key("test-key")
        .connect_core_ws(options)
        .await
        .unwrap()
}

async fn wait_for_state(ws: &CoreWsClient, expected: WsConnectionState) {
    let deadline = Instant::now() + WAIT;
    while ws.state() != expected {
        assert!(Instant::now() < deadline, "state stayed {} instead of {expected}", ws.state());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

async fn next_event(ws: &mut CoreWsClient) -> Option<Result<Value, WsError>> {
    tokio::time::timeout(WAIT, ws.next_value_event())
        .await
        .expect("no event within the wait bound")
}

/// Asserts `elapsed` is a plausible firing of an idle limit `limit`
/// measured from the last inbound frame.
fn assert_fired_near(elapsed: Duration, limit: Duration) {
    assert!(elapsed >= limit / 2, "fired early: {elapsed:?} for {limit:?}");
    assert!(elapsed <= limit * 3, "fired late: {elapsed:?} for {limit:?}");
}

/// Sends ~64 KiB frames to a server that does not read until a send is not
/// accepted within [`STALL`]: the socket buffers, the frame being written,
/// and the outbound queue are then full. The waiting send is cancelled, so
/// nothing is left waiting for room. Returns the frames accepted.
async fn fill_until_stalled(ws: &CoreWsClient) -> usize {
    let filler = json!({"kind": "event", "msg": "filler", "pad": "x".repeat(64 * 1024)});
    for accepted in 0..MAX_FILL {
        match tokio::time::timeout(STALL, ws.send_value(filler.clone())).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("send failed while filling after {accepted} frames: {error:?}"),
            Err(_) => return accepted,
        }
    }
    panic!("writes never stalled after {MAX_FILL} frames");
}

/// Data frames the server recorded on `connection`, as JSON.
fn frames_on(server: &MockWsServer, connection: usize) -> Vec<Value> {
    server
        .frames()
        .into_iter()
        .filter(|(index, _)| *index == connection)
        .map(|(_, message)| serde_json::from_str(message.to_text().unwrap()).unwrap())
        .collect()
}

// R8: after one frame and then silence on an open socket, the client reports
// `ReceiveTimeout`, redials under its policy, and is ready again.
#[tokio::test]
async fn silence_after_a_frame_times_out_and_redials() {
    let server = MockWsServer::start(vec![Script::Accept(vec![
        Step::text(json!({"kind": "event", "msg": "hello"}).to_string()),
        Step::Stall,
    ])])
    .await;
    let mut ws = connect(
        &server.ws_origin(),
        builder().receive_timeout(Some(IDLE)).reconnect(Some(reconnect(Duration::from_millis(20)))).build(),
    )
    .await;

    let hello = next_event(&mut ws).await.unwrap().unwrap();
    assert_eq!(hello["msg"], "hello");
    let last_frame = Instant::now();
    match next_event(&mut ws).await {
        Some(Err(WsError::ReceiveTimeout(limit))) => assert_eq!(limit, IDLE),
        other => panic!("expected ReceiveTimeout, got {other:?}"),
    }
    assert_fired_near(last_frame.elapsed(), IDLE);

    server.wait_for_connections(2, WAIT).await;
    wait_for_state(&ws, WsConnectionState::Ready).await;
    let again = next_event(&mut ws).await.unwrap().unwrap();
    assert_eq!(again["msg"], "hello", "the replacement connection serves events");
    ws.close().await.unwrap();
}

// R8: with `receive_timeout: None` a silent server never loses the connection.
#[tokio::test]
async fn no_receive_timeout_keeps_a_silent_connection() {
    let server = MockWsServer::start(vec![Script::Accept(vec![
        Step::text(json!({"kind": "event", "msg": "hello"}).to_string()),
        Step::Stall,
    ])])
    .await;
    let mut ws = connect(
        &server.ws_origin(),
        builder().receive_timeout(None).reconnect(Some(reconnect(Duration::from_millis(20)))).build(),
    )
    .await;

    assert_eq!(next_event(&mut ws).await.unwrap().unwrap()["msg"], "hello");
    let quiet = tokio::time::timeout(QUIET.max(IDLE * 2), ws.next_value_event()).await;
    assert!(quiet.is_err(), "no event on a silent connection: {quiet:?}");
    assert_eq!(ws.state(), WsConnectionState::Ready);
    assert_eq!(server.connections(), 1);
    ws.close().await.unwrap();
}

// R8: server pings every quarter of the limit keep the connection alive past
// twice the limit, and the server receives the client's pongs, which the
// client flushes without writing anything of its own.
#[tokio::test]
async fn server_pings_keep_the_connection_alive_and_are_ponged() {
    let until = Barrier::new();
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Heartbeat {
        every: IDLE / 4,
        until: until.clone(),
        read: true,
    }])])
    .await;
    let mut ws = connect(
        &server.ws_origin(),
        builder().receive_timeout(Some(IDLE)).reconnect(Some(reconnect(Duration::from_millis(20)))).build(),
    )
    .await;

    // Ten pings at a quarter of the limit span 2.25 limits.
    server.wait_for_pongs(10, WAIT).await;
    let quiet = tokio::time::timeout(Duration::ZERO, ws.next_value_event()).await;
    assert!(quiet.is_err(), "pings must not time the connection out: {quiet:?}");
    assert_eq!(ws.state(), WsConnectionState::Ready);
    assert_eq!(server.connections(), 1);
    let pongs = server.pongs();
    for (n, (connection, payload)) in pongs.iter().enumerate() {
        assert_eq!(*connection, 0);
        assert_eq!(*payload, (n as u64 + 1).to_be_bytes(), "each pong echoes its ping");
    }

    // Once the pings stop, the limit applies again.
    until.release();
    let stopped = Instant::now();
    assert!(matches!(next_event(&mut ws).await, Some(Err(WsError::ReceiveTimeout(_)))));
    assert!(stopped.elapsed() <= IDLE * 3, "fired late: {:?}", stopped.elapsed());
    ws.close().await.unwrap();
}

// R8: outbound traffic does not reset the idle timer.
#[tokio::test]
async fn outbound_sends_do_not_reset_the_idle_timer() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Receive(1000)])]).await;
    let mut ws = connect(&server.ws_origin(), builder().receive_timeout(Some(IDLE)).build()).await;

    let connected = Instant::now();
    let mut sent = 0;
    let fired = loop {
        assert!(connected.elapsed() < WAIT, "no ReceiveTimeout while sending");
        if ws.send_value(json!({"kind": "event", "msg": "tick", "n": sent})).await.is_ok() {
            sent += 1;
        }
        if let Ok(event) = tokio::time::timeout(IDLE / 4, ws.next_value_event()).await {
            break event;
        }
    };
    let elapsed = connected.elapsed();

    assert!(matches!(fired, Some(Err(WsError::ReceiveTimeout(_)))), "{fired:?}");
    assert_fired_near(elapsed, IDLE);
    server.wait_for_frames(3, WAIT).await;
    let ticks = frames_on(&server, 0);
    assert!(ticks.len() >= 3, "the server received the sends: {ticks:?}");
    assert!(ticks.iter().all(|tick| tick["msg"] == "tick"));
    wait_for_state(&ws, WsConnectionState::Closed).await;
}

// A full inbound queue pauses reading, which counts as silence: the timeout
// fires and state changes publish while no event is consumed, and close
// returns promptly.
#[tokio::test]
async fn a_full_event_queue_still_times_out_publishes_state_and_closes() {
    let mut steps: Vec<Step> = (0..10)
        .map(|n| Step::text(json!({"kind": "event", "msg": "burst", "n": n}).to_string()))
        .collect();
    steps.push(Step::Stall);
    let server = MockWsServer::start(vec![Script::Accept(steps)]).await;
    let mut ws = connect(
        &server.ws_origin(),
        builder()
            .inbound_capacity(1)
            .receive_timeout(Some(IDLE))
            // Long enough that `Disconnected` is observable by polling.
            .reconnect(Some(reconnect(Duration::from_millis(300))))
            .build(),
    )
    .await;

    wait_for_state(&ws, WsConnectionState::Disconnected).await;
    server.wait_for_connections(2, WAIT).await;
    wait_for_state(&ws, WsConnectionState::Ready).await;

    let closing = Instant::now();
    ws.close().await.unwrap();
    assert!(closing.elapsed() < Duration::from_secs(2), "close took {:?}", closing.elapsed());
    assert_eq!(ws.state(), WsConnectionState::Closed);

    let first = next_event(&mut ws).await.unwrap().unwrap();
    assert_eq!(first["n"], 0, "the queued event survives");
    while next_event(&mut ws).await.is_some() {}
}

// Stalled writes (the server stops reading) do not block a manual close, and
// a send waiting for room in the outbound queue fails `Disconnected`.
#[tokio::test]
async fn stalled_writes_do_not_block_close() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Stall])]).await;
    let ws = connect(&server.ws_origin(), builder().outbound_capacity(2).build()).await;
    let accepted = fill_until_stalled(&ws).await;
    assert!(accepted > 0);

    let closing = Instant::now();
    let (waiting, closed) = tokio::join!(ws.send_value(json!({"kind": "event", "msg": "late"})), async {
        ws.close().await.unwrap();
        closing.elapsed()
    });
    assert!(closed < Duration::from_secs(2), "close took {closed:?}");
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert!(matches!(waiting, Err(WsError::Disconnected)), "{waiting:?}");
}

// Stalled writes do not block the idle timeout. The request is registered
// as pending and then waits for room in the full outbound queue; the
// request timeout only starts once it is queued, so the specified outcome
// is `Disconnected` (pending requests fail on loss), never `RequestTimeout`.
#[tokio::test]
async fn stalled_writes_do_not_block_the_idle_timeout_and_pending_request_fails() {
    let until = Barrier::new();
    let server = MockWsServer::start(vec![Script::Accept(vec![
        // Keeps the connection alive without reading while the client fills.
        Step::Heartbeat { every: Duration::from_millis(100), until: until.clone(), read: false },
        Step::Stall,
    ])])
    .await;
    let idle = Duration::from_millis(500);
    let mut ws = connect(
        &server.ws_origin(),
        builder().outbound_capacity(2).receive_timeout(Some(idle)).build(),
    )
    .await;
    fill_until_stalled(&ws).await;
    assert_eq!(ws.state(), WsConnectionState::Ready, "pings kept the connection alive");

    let (request, released) = tokio::join!(ws.request_value(json!({"kind": "req", "msg": "x"})), async {
        until.release();
        Instant::now()
    });
    let failed_after = released.elapsed();

    assert!(matches!(request, Err(WsError::Disconnected)), "{request:?}");
    assert!(failed_after <= idle * 3, "the timeout fired late: {failed_after:?}");
    match next_event(&mut ws).await {
        Some(Err(WsError::ReceiveTimeout(limit))) => assert_eq!(limit, idle),
        other => panic!("expected ReceiveTimeout, got {other:?}"),
    }
    wait_for_state(&ws, WsConnectionState::Closed).await;
}

// On loss, a request in flight fails `Disconnected` and the commands queued
// for the lost connection are discarded: the replacement connection's first
// frames are the ones sent after it became ready.
#[tokio::test]
async fn disconnect_fails_pending_and_discards_queued_commands() {
    let gate = Barrier::new();
    let server = MockWsServer::start(vec![
        // Read the request, stop reading while the client fills, then drop.
        Script::Accept(vec![Step::Receive(1), Step::Wait(gate.clone()), Step::Drop]),
        Script::Accept(vec![Step::Receive(2)]),
    ])
    .await;
    let ws = connect(
        &server.ws_origin(),
        builder().outbound_capacity(4).reconnect(Some(reconnect(Duration::from_millis(20)))).build(),
    )
    .await;

    let (request, ()) = tokio::join!(ws.request_value(json!({"kind": "req", "msg": "x"})), async {
        server.wait_for_frames(1, WAIT).await;
        fill_until_stalled(&ws).await;
        gate.release();
    });
    assert!(matches!(request, Err(WsError::Disconnected)), "{request:?}");

    server.wait_for_connections(2, WAIT).await;
    wait_for_state(&ws, WsConnectionState::Ready).await;
    for tag in ["fresh-1", "fresh-2"] {
        ws.send_value(json!({"kind": "event", "msg": tag})).await.unwrap();
    }
    server.wait_for_frames(3, WAIT).await;

    let first = frames_on(&server, 0);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0]["msg"], "x");
    let replacement: Vec<Value> = frames_on(&server, 1).into_iter().map(|f| f["msg"].clone()).collect();
    assert_eq!(replacement, ["fresh-1", "fresh-2"], "no old command reached the new connection");
    ws.close().await.unwrap();
}

// A send that was waiting for room in the outbound queue when the connection
// was lost belongs to the lost connection: it is not written to the
// replacement.
#[tokio::test]
async fn a_send_waiting_for_room_at_disconnect_is_not_written_to_the_replacement() {
    let gate = Barrier::new();
    let server = MockWsServer::start(vec![
        Script::Accept(vec![Step::Wait(gate.clone()), Step::Drop]),
        Script::Accept(vec![Step::Receive(2)]),
    ])
    .await;
    let ws = connect(
        &server.ws_origin(),
        builder().outbound_capacity(2).reconnect(Some(reconnect(Duration::from_millis(20)))).build(),
    )
    .await;
    fill_until_stalled(&ws).await;

    let (late, ()) = tokio::join!(ws.send_value(json!({"kind": "event", "msg": "late"})), async {
        gate.release();
    });
    server.wait_for_connections(2, WAIT).await;
    wait_for_state(&ws, WsConnectionState::Ready).await;
    for tag in ["fresh-1", "fresh-2"] {
        ws.send_value(json!({"kind": "event", "msg": tag})).await.unwrap();
    }
    server.wait_for_frames(2, WAIT).await;

    let replacement: Vec<Value> = frames_on(&server, 1).into_iter().map(|f| f["msg"].clone()).collect();
    assert_eq!(
        replacement,
        ["fresh-1", "fresh-2"],
        "the waiting send (result {late:?}) reached the replacement connection"
    );
    ws.close().await.unwrap();
}

// A correlated request that was waiting for room in the outbound queue when
// the connection was lost fails `Disconnected` and is never re-sent: it is
// not written to the replacement connection either.
#[tokio::test]
async fn a_request_waiting_for_room_at_disconnect_is_not_replayed() {
    let gate = Barrier::new();
    let server = MockWsServer::start(vec![
        Script::Accept(vec![Step::Wait(gate.clone()), Step::Drop]),
        Script::Accept(vec![Step::Receive(2)]),
    ])
    .await;
    let ws = connect(
        &server.ws_origin(),
        builder().outbound_capacity(2).reconnect(Some(reconnect(Duration::from_millis(20)))).build(),
    )
    .await;
    fill_until_stalled(&ws).await;

    let (request, ()) = tokio::join!(ws.request_value(json!({"kind": "req", "msg": "x"})), async {
        gate.release();
    });
    assert!(matches!(request, Err(WsError::Disconnected)), "{request:?}");
    server.wait_for_connections(2, WAIT).await;
    wait_for_state(&ws, WsConnectionState::Ready).await;
    for tag in ["fresh-1", "fresh-2"] {
        ws.send_value(json!({"kind": "event", "msg": tag})).await.unwrap();
    }
    server.wait_for_frames(2, WAIT).await;

    let replacement: Vec<Value> = frames_on(&server, 1).into_iter().map(|f| f["msg"].clone()).collect();
    assert_eq!(replacement, ["fresh-1", "fresh-2"], "the failed request was replayed");
    ws.close().await.unwrap();
}
