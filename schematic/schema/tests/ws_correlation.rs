//! Request/response correlation of generated WebSocket clients (Core, and
//! one Dock request on the recorded wire shape).

#![allow(clippy::result_large_err)]

mod ws_mock;

use ws_mock::no_setup;

use std::future::Future;
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use schematic_schema::unfolded_circle_core_ws::{
    CoreWsClient, CoreWsEnvelopeKind, CoreWsKnownMessage, CoreWsPingMessage, CoreWsServerMessage,
    UnfoldedCircleCoreWs,
};
use schematic_schema::unfolded_circle_dock_ws::{
    DockWsEnvelopeType, DockWsKnownMessage, DockWsMessageName, DockWsRequestEnvelope,
    UnfoldedCircleDockWs,
};
use schematic_schema::ws_shared::{
    ReconnectPolicy, WsClientOptions, WsClientOptionsBuilder, WsConnectionState, WsError,
};
use serde_json::{Value, json};
use serde_json::value::RawValue;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use tokio_tungstenite::tungstenite::Message;
use ws_mock::{MockWsServer, Script, Step};

/// Generous against a loaded runner; every wait ends on an event well before it.
const WAIT: Duration = Duration::from_secs(3);

type PeerStream = tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>;

/// The server side of one accepted connection.
struct Peer(PeerStream);

impl Peer {
    /// The next data frame as JSON; `None` once the client leaves.
    async fn recv(&mut self) -> Option<Value> {
        while let Some(message) = self.0.next().await {
            match message.ok()? {
                Message::Text(text) => return Some(serde_json::from_str(text.as_ref()).unwrap()),
                Message::Binary(bytes) => return Some(serde_json::from_slice(bytes.as_ref()).unwrap()),
                Message::Close(_) => return None,
                _ => {}
            }
        }
        None
    }

    async fn send(&mut self, value: Value) {
        self.0.send(Message::Text(value.to_string().into())).await.unwrap();
    }
}

/// Serves one connection on a fresh loopback port with `handler`.
async fn spawn_peer<F, Fut>(handler: F) -> (String, JoinHandle<Fut::Output>)
where
    F: FnOnce(Peer) -> Fut + Send + 'static,
    Fut: Future + Send + 'static,
    Fut::Output: Send + 'static,
{
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("ws://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let ws = tokio_tungstenite::accept_async(stream).await.unwrap();
        handler(Peer(ws)).await
    });
    (origin, task)
}

fn builder() -> WsClientOptionsBuilder {
    WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(2))
        .request_timeout(Duration::from_secs(2))
}

async fn connect(origin: &str, options: WsClientOptions) -> CoreWsClient {
    UnfoldedCircleCoreWs::with_base_url(origin)
        .api_key("test-key")
        .connect_core_ws_with_setup(options, no_setup)
        .await
        .unwrap()
}

/// A ping whose `msg_data` is `data`; the client overwrites `id`.
fn ping(data: Value) -> CoreWsPingMessage {
    CoreWsPingMessage {
        kind: CoreWsEnvelopeKind::Req,
        id: 0,
        msg: CoreWsKnownMessage::Ping,
        msg_data: Some(RawValue::from_string(data.to_string()).unwrap()),
    }
}

fn pong(req_id: &Value, data: Value) -> Value {
    json!({"kind": "resp", "req_id": req_id, "msg": "pong", "code": 200, "msg_data": data})
}

fn data_of(pong: &schematic_schema::unfolded_circle_core_ws::CoreWsPongMessage) -> Value {
    serde_json::from_str(pong.msg_data.as_ref().expect("pong carries data").get()).unwrap()
}

// R11: a typed request writes a numeric id and completes with the typed answer.
#[tokio::test]
async fn request_ping_sends_a_numeric_id_and_returns_the_pong() {
    let (origin, peer) = spawn_peer(|mut peer| async move {
        let request = peer.recv().await.unwrap();
        peer.send(pong(&request["id"], json!({"echo": "hello"}))).await;
        request
    })
    .await;
    let ws = connect(&origin, builder().build()).await;

    let answer = ws.request_ping(ping(json!({"say": "hello"}))).await.unwrap();
    let request = peer.await.unwrap();
    assert!(request["id"].is_u64(), "id is not a JSON number: {request}");
    assert_eq!(request["id"], json!(1));
    assert_eq!(request["msg"], "ping");
    assert_eq!(request["msg_data"], json!({"say": "hello"}));
    assert_eq!(answer.req_id, 1);
    assert_eq!(answer.msg, CoreWsKnownMessage::Pong);
    assert_eq!(data_of(&answer), json!({"echo": "hello"}));
}

// R11: another message carrying the request's id stays an ordinary event.
#[tokio::test]
async fn same_id_with_another_message_is_an_event_not_the_answer() {
    let (origin, peer) = spawn_peer(|mut peer| async move {
        let request = peer.recv().await.unwrap();
        let id = request["id"].clone();
        peer.send(json!({"kind": "resp", "req_id": id, "msg": "result", "code": 200})).await;
        peer.send(json!({"kind": "event", "req_id": id, "msg": "entity_change", "msg_data": {}}))
            .await;
        peer.send(pong(&id, json!({"n": 1}))).await;
        while peer.recv().await.is_some() {}
    })
    .await;
    let mut ws = connect(&origin, builder().build()).await;

    let answer = ws.request_ping(ping(json!({}))).await.unwrap();
    assert_eq!(data_of(&answer), json!({"n": 1}));
    match ws.next_event().await {
        Some(Ok(CoreWsServerMessage::Result(result))) => assert_eq!(result.req_id, answer.req_id),
        other => panic!("expected the result frame as an event, got {other:?}"),
    }
    assert!(
        matches!(ws.next_event().await, Some(Ok(CoreWsServerMessage::EventEnvelope(_)))),
        "the event frame is delivered as an event"
    );
    let _ = ws.close().await;
    peer.await.unwrap();
}

// R11: with only a non-answering frame for the id, the request times out
// and the frame is still delivered as an event.
#[tokio::test]
async fn same_id_with_another_message_alone_times_the_request_out() {
    let (origin, peer) = spawn_peer(|mut peer| async move {
        let request = peer.recv().await.unwrap();
        peer.send(json!({"kind": "resp", "req_id": request["id"], "msg": "result", "code": 200}))
            .await;
        while peer.recv().await.is_some() {}
    })
    .await;
    let mut ws = connect(&origin, builder().request_timeout(Duration::from_millis(300)).build()).await;

    let started = Instant::now();
    let error = ws.request_ping(ping(json!({}))).await.unwrap_err();
    assert!(matches!(error, WsError::RequestTimeout(_)), "{error:?}");
    assert!(started.elapsed() >= Duration::from_millis(250));
    assert!(matches!(
        ws.next_event().await,
        Some(Ok(CoreWsServerMessage::Result(result))) if result.req_id == 1
    ));
    let _ = ws.close().await;
    peer.await.unwrap();
}

// R11: concurrent requests get distinct ids and their own answers, even
// when the answers arrive out of order.
#[tokio::test]
async fn concurrent_requests_receive_their_own_answers_out_of_order() {
    let (origin, peer) = spawn_peer(|mut peer| async move {
        let first = peer.recv().await.unwrap();
        let second = peer.recv().await.unwrap();
        for request in [&second, &first] {
            peer.send(pong(&request["id"], request["msg_data"].clone())).await;
        }
        while peer.recv().await.is_some() {}
        (first, second)
    })
    .await;
    let ws = connect(&origin, builder().build()).await;

    let (a, b) = tokio::join!(
        ws.request_ping(ping(json!({"tag": "a"}))),
        ws.request_ping(ping(json!({"tag": "b"}))),
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_eq!(data_of(&a), json!({"tag": "a"}));
    assert_eq!(data_of(&b), json!({"tag": "b"}));
    assert_ne!(a.req_id, b.req_id);

    let _ = ws.close().await;
    let (first, second) = peer.await.unwrap();
    let mut ids = [first["id"].as_u64().unwrap(), second["id"].as_u64().unwrap()];
    ids.sort_unstable();
    assert_eq!(ids, [1, 2]);
    let mut answered = [a.req_id, b.req_id];
    answered.sort_unstable();
    assert_eq!(answered, ids);
}

/// Answers every ping whose `msg_data.answer` is `true`; ignores the rest.
async fn selective_peer() -> (String, JoinHandle<Vec<Value>>) {
    spawn_peer(|mut peer| async move {
        let mut seen = Vec::new();
        while let Some(request) = peer.recv().await {
            if request.pointer("/msg_data/answer") == Some(&json!(true)) {
                peer.send(pong(&request["id"], json!({"ok": true}))).await;
            }
            seen.push(request);
        }
        seen
    })
    .await
}

fn silent() -> CoreWsPingMessage {
    ping(json!({"answer": false}))
}

fn answered() -> CoreWsPingMessage {
    ping(json!({"answer": true}))
}

// R11: under `max_pending_requests(1)` a timed-out, cancelled, or
// unserializable request frees its slot; a genuinely pending one does not.
#[tokio::test]
async fn pending_capacity_is_reclaimed_after_timeout_cancel_and_serialize_failure() {
    let (origin, peer) = selective_peer().await;
    let ws = connect(
        &origin,
        builder()
            .max_pending_requests(1)
            .request_timeout(Duration::from_millis(300))
            .build(),
    )
    .await;

    // Control: while one request is pending, a second is refused.
    let (held, refused) = tokio::join!(ws.request_ping(silent()), async {
        tokio::time::sleep(Duration::from_millis(50)).await;
        ws.request_ping(answered()).await
    });
    assert!(matches!(held, Err(WsError::RequestTimeout(_))), "{held:?}");
    assert!(matches!(refused, Err(WsError::BackpressureFull(1))), "{refused:?}");

    // (a) after a timeout.
    let error = ws.request_ping(silent()).await.unwrap_err();
    assert!(matches!(error, WsError::RequestTimeout(_)), "{error:?}");
    assert_eq!(data_of(&ws.request_ping(answered()).await.unwrap()), json!({"ok": true}));

    // (b) after cancellation: the future is dropped while still pending.
    let cancelled = tokio::time::timeout(Duration::from_millis(50), ws.request_ping(silent())).await;
    assert!(cancelled.is_err(), "the silent request cannot complete");
    assert_eq!(data_of(&ws.request_ping(answered()).await.unwrap()), json!({"ok": true}));

    // (c) after a payload that is not a JSON object.
    let error = ws.request_value(json!(5)).await.unwrap_err();
    assert!(matches!(error, WsError::Protocol(_)), "{error:?}");
    let value = ws
        .request_value(json!({"kind": "req", "msg": "ping", "msg_data": {"answer": true}}))
        .await
        .unwrap();
    assert_eq!(value["msg"], "pong");

    let _ = ws.close().await;
    let seen = peer.await.unwrap();
    // The refused and unserializable requests never reached the wire.
    assert_eq!(seen.len(), 6, "{seen:?}");
}

// R11: a request in flight when the connection drops fails with
// `Disconnected`, is not re-sent on the replacement connection, and ids keep
// counting up across the redial.
#[tokio::test]
async fn disconnect_fails_in_flight_request_without_resending_it() {
    let server = MockWsServer::start(vec![
        Script::Accept(vec![Step::Receive(1), Step::Drop]),
        Script::Accept(vec![
            Step::Receive(1),
            Step::text(json!({"kind": "resp", "req_id": 2, "msg": "pong", "code": 200}).to_string()),
        ]),
    ])
    .await;
    let reconnect = ReconnectPolicy {
        initial_backoff: Duration::from_millis(20),
        max_backoff: Duration::from_millis(40),
        multiplier: 1.0,
        jitter: 0.0,
        max_attempts: Some(5),
    };
    let ws = connect(&server.ws_origin(), builder().reconnect(Some(reconnect)).build()).await;

    let error = ws.request_ping(ping(json!({"first": true}))).await.unwrap_err();
    assert!(matches!(error, WsError::Disconnected), "{error:?}");

    server.wait_for_requests(2, WAIT).await;
    let deadline = Instant::now() + WAIT;
    while ws.state() != WsConnectionState::Ready {
        assert!(Instant::now() < deadline, "no replacement connection: {}", ws.state());
        tokio::time::sleep(Duration::from_millis(5)).await;
    }

    let answer = ws.request_ping(ping(json!({"second": true}))).await.unwrap();
    assert_eq!(answer.req_id, 2);

    let frames: Vec<(usize, Value)> = server
        .frames()
        .into_iter()
        .map(|(connection, message)| {
            (connection, serde_json::from_str(message.to_text().unwrap()).unwrap())
        })
        .collect();
    assert_eq!(frames.len(), 2, "{frames:?}");
    assert_eq!(frames[0].0, 0);
    assert_eq!(frames[0].1["id"], json!(1));
    assert_eq!(frames[0].1["msg_data"], json!({"first": true}));
    let on_replacement: Vec<&Value> =
        frames.iter().filter(|(connection, _)| *connection == 1).map(|(_, f)| f).collect();
    assert_eq!(on_replacement.len(), 1);
    assert_eq!(on_replacement[0]["id"], json!(2));
    assert_eq!(on_replacement[0]["msg_data"], json!({"second": true}));
    let _ = ws.close().await;
}

// R11: `request_value` completes with the first frame carrying its id,
// whatever message it is; frames with other ids stay events.
#[tokio::test]
async fn request_value_returns_the_first_frame_with_its_id() {
    let (origin, peer) = spawn_peer(|mut peer| async move {
        let request = peer.recv().await.unwrap();
        let id = request["id"].as_u64().unwrap();
        peer.send(json!({"kind": "resp", "req_id": id + 100, "msg": "result", "code": 200}))
            .await;
        peer.send(json!({"kind": "resp", "req_id": id, "msg": "result", "code": 200, "tag": "first"}))
            .await;
        peer.send(pong(&json!(id), json!({}))).await;
        while peer.recv().await.is_some() {}
        request
    })
    .await;
    let mut ws = connect(&origin, builder().build()).await;

    let value = ws
        .request_value(json!({"kind": "req", "msg": "version", "id": "ignored"}))
        .await
        .unwrap();
    assert_eq!(value["msg"], "result");
    assert_eq!(value["tag"], "first");
    assert_eq!(value["req_id"], json!(1));

    let stray = ws.next_value_event().await.unwrap().unwrap();
    assert_eq!(stray["req_id"], json!(101));
    let late_pong = ws.next_value_event().await.unwrap().unwrap();
    assert_eq!(late_pong["msg"], "pong");

    let _ = ws.close().await;
    let request = peer.await.unwrap();
    assert_eq!(request["id"], json!(1), "the client's id replaces the caller's");
}

// R10/R11: a typed Dock command is written as the UCD2 AsyncAPI records it
// (`type=dock`, `command`, the allocated `id`), and the recorded
// `systemInfoMsg` reply, with only `req_id` set to that id, returns typed
// result fields.
#[tokio::test]
async fn dock_get_sysinfo_request_uses_the_recorded_wire_shape() {
    let (origin, peer) = spawn_peer(|mut peer| async move {
        let request = peer.recv().await.unwrap();
        let mut reply: Value = serde_json::from_str(
            r#"{"type":"dock","req_id":2,"msg":"get_sysinfo","code":200,"name":"My dock","hostname":"UC-Dock-1234","model":"UCD2","revision":"5.4","version":"0.4.0","serial":"0042","ir_learning":false}"#,
        )
        .unwrap();
        reply["req_id"] = request["id"].clone();
        peer.send(reply).await;
        while peer.recv().await.is_some() {}
        request
    })
    .await;
    let ws = UnfoldedCircleDockWs::with_base_url(&origin)
        .connect_dock_root_with_setup(builder().build(), no_setup)
        .await
        .unwrap();

    let request = DockWsRequestEnvelope {
        type_name: DockWsEnvelopeType::Dock,
        id: 0,
        command: DockWsMessageName::Known(DockWsKnownMessage::GetSysinfo),
        params: serde_json::Map::new(),
    };
    let answer = ws.request_request_envelope(request).await.unwrap();
    assert_eq!(answer.req_id, Some(1));
    assert_eq!(answer.msg, DockWsMessageName::Known(DockWsKnownMessage::GetSysinfo));
    assert_eq!(answer.code, Some(200));
    assert_eq!(answer.data["name"], "My dock");
    assert_eq!(answer.data["model"], "UCD2");
    assert_eq!(answer.data["version"], "0.4.0");

    let _ = ws.close().await;
    let wire = peer.await.unwrap();
    assert_eq!(wire, json!({"type": "dock", "id": 1, "command": "get_sysinfo"}));
}
