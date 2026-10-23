//! Session setup and recovery of generated WebSocket clients: the
//! consecutive-failure reconnect budget (R2), the retained opening message
//! and setup callback re-run on every connection (R3), and the supervisor's
//! close, drop, and timeout rules, observed from loopback servers.

#![allow(clippy::result_large_err)]

mod common;
mod ws_mock;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use common::EnvGuard;
use futures_util::StreamExt;
use schematic_schema::elevenlabs_ws::{
    ElevenLabsTTSWs, MultiContextInitMessage, MultiContextTextToSpeechConnectionParams,
    TextToSpeechClient, TextToSpeechConnectionParams, TtsInitMessage,
};
use schematic_schema::unfolded_circle_core_ws::{
    CoreWsAuthMessage, CoreWsClient, CoreWsClientMessage, CoreWsEnvelopeKind, CoreWsKnownMessage,
    CoreWsServerMessage, CoreWsSetup, UnfoldedCircleCoreWs,
};
use schematic_schema::ws_shared::{
    ReconnectPolicy, WsClientOptions, WsConnectionState, WsError, WsSetupError,
};
use serde_json::value::RawValue;
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;
use ws_mock::{Barrier, MockWsServer, Script, Step};

/// Bounds a hang only; every wait ends on an observed condition well before.
const WAIT: Duration = Duration::from_secs(5);
/// How long a test watches to show that something did NOT happen.
const QUIET: Duration = Duration::from_millis(400);
/// Bound on `close()` returning while a backoff, dial, or setup is pending.
const PROMPT: Duration = Duration::from_secs(2);
/// Every ElevenLabs resolver candidate, in resolver order.
const ELEVENLABS_ENV: [&str; 2] = ["ELEVEN_LABS_API_KEY", "ELEVENLABS_API_KEY"];
const KEY: &str = "sentinel-key";

fn policy(max_attempts: Option<u32>) -> ReconnectPolicy {
    ReconnectPolicy {
        initial_backoff: Duration::from_millis(20),
        max_backoff: Duration::from_millis(20),
        multiplier: 1.0,
        jitter: 0.0,
        max_attempts,
    }
}

fn slow_policy(backoff: Duration) -> ReconnectPolicy {
    ReconnectPolicy {
        initial_backoff: backoff,
        max_backoff: backoff,
        ..policy(None)
    }
}

fn options(reconnect: Option<ReconnectPolicy>) -> WsClientOptions {
    WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(2))
        .request_timeout(Duration::from_secs(2))
        .reconnect(reconnect)
        .build()
}

/// A connection the server upgrades and immediately closes. A clean close
/// is a loss without an error event, so the event queue carries only the
/// failures under test.
fn accept_then_close() -> Script {
    Script::Accept(vec![Step::Close])
}

/// Polls `condition` until it holds; panics after [`WAIT`].
async fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
}

/// Watches the server for [`QUIET`] and asserts its connection count stays
/// `expected`.
async fn assert_no_more_connections(server: &MockWsServer, expected: usize) {
    assert_eq!(server.connections(), expected);
    tokio::time::sleep(QUIET).await;
    assert_eq!(server.connections(), expected, "the client dialed again");
}

/// The data frames connection `connection` delivered to the server, as JSON.
fn frames_of(server: &MockWsServer, connection: usize) -> Vec<Value> {
    server
        .frames()
        .into_iter()
        .filter(|(index, _)| *index == connection)
        .map(|(_, message)| match message {
            Message::Text(text) => serde_json::from_str(text.as_ref()).unwrap(),
            other => panic!("expected a text frame, got {other:?}"),
        })
        .collect()
}

// ---------------------------------------------------------------- Core clients

async fn core_connect(server: &MockWsServer, options: WsClientOptions) -> CoreWsClient {
    UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .api_key("test-key")
        .connect_core_ws(options)
        .await
        .unwrap()
}

/// The next event within [`WAIT`]; `None` once the event stream has ended.
async fn next_core_event(ws: &mut CoreWsClient) -> Option<Result<Value, WsError>> {
    tokio::time::timeout(WAIT, ws.next_value_event())
        .await
        .expect("an event or the end of the stream arrives")
}

async fn expect_core_error(ws: &mut CoreWsClient) -> WsError {
    match next_core_event(ws).await {
        Some(Err(error)) => error,
        other => panic!("expected an error event, got {other:?}"),
    }
}

/// A server challenge, the client's auth message, and the server's answer
/// with `code`.
fn auth_steps(code: u16) -> Vec<Step> {
    vec![
        Step::text(r#"{"kind":"event","msg":"auth_required"}"#),
        Step::Receive(1),
        Step::text(
            json!({"kind": "resp", "msg": "authentication", "req_id": 1, "code": code})
                .to_string(),
        ),
    ]
}

/// [`auth_steps`] followed by `then`.
fn authed(code: u16, then: Vec<Step>) -> Script {
    let mut steps = auth_steps(code);
    steps.extend(then);
    Script::Accept(steps)
}

/// Core's message-based authentication: answers the challenge with the
/// handshake credential. A 401 is a rejection; any other non-200 answer is
/// transient.
async fn authenticate(mut session: CoreWsSetup) -> Result<(), WsSetupError> {
    match session.next_message().await? {
        CoreWsServerMessage::AuthRequired(_) => {}
        other => return Err(WsSetupError::Transient(format!("no challenge: {other:?}"))),
    }
    let token = session.credential().expose_secret().unwrap_or_default().to_string();
    let msg_data = RawValue::from_string(json!({ "token": token }).to_string()).unwrap();
    session
        .send(CoreWsClientMessage::Auth(CoreWsAuthMessage {
            kind: CoreWsEnvelopeKind::Req,
            msg: CoreWsKnownMessage::Auth,
            msg_data,
        }))
        .await?;
    let answer = session.next_value().await?;
    match answer["code"].as_u64() {
        Some(200) => Ok(()),
        Some(401) => Err(WsSetupError::Rejected("authentication refused".to_string())),
        _ => Err(WsSetupError::Transient(format!("authentication answered {answer}"))),
    }
}

/// Connects Core with [`authenticate`] as setup; `runs` counts completed
/// setups.
async fn core_connect_with_auth(
    server: &MockWsServer,
    options: WsClientOptions,
    runs: &Arc<AtomicUsize>,
) -> Result<CoreWsClient, WsError> {
    let runs = Arc::clone(runs);
    UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .api_key("k")
        .connect_core_ws_with_setup(options, move |session| {
            let runs = Arc::clone(&runs);
            async move {
                let result = authenticate(session).await;
                if result.is_ok() {
                    runs.fetch_add(1, Ordering::SeqCst);
                }
                result
            }
        })
        .await
}

fn auth_frame(token: &str) -> Value {
    json!({"kind": "req", "msg": "auth", "msg_data": {"token": token}})
}

// ------------------------------------------------------------ R2: the budget

// R2: the budget counts consecutive failures, so five losses in a row, each
// followed by a successful redial, never exhaust `Some(2)`.
#[tokio::test]
async fn repeated_losses_with_successful_redials_never_exhaust_the_budget() {
    let mut scripts = vec![accept_then_close(); 5];
    scripts.push(Script::accept());
    let server = MockWsServer::start(scripts).await;
    let ws = core_connect(&server, options(Some(policy(Some(2))))).await;

    server.wait_for_connections(6, WAIT).await;
    wait_until("Ready on the sixth connection", || ws.state() == WsConnectionState::Ready).await;
    assert_no_more_connections(&server, 6).await;
    assert_eq!(ws.state(), WsConnectionState::Ready);
    let _ = ws.close().await;
}

// R2: two consecutive refused redials exhaust `Some(2)`: both errors are
// delivered, the client is Closed, the event stream ends, no third redial.
#[tokio::test]
async fn two_consecutive_refused_redials_exhaust_a_budget_of_two() {
    let server = MockWsServer::start(vec![
        accept_then_close(),
        Script::Refuse(503),
        Script::Refuse(503),
        Script::accept(),
    ])
    .await;
    let mut ws = core_connect(&server, options(Some(policy(Some(2))))).await;

    for _ in 0..2 {
        let error = expect_core_error(&mut ws).await;
        assert!(
            matches!(error, WsError::HandshakeRejected { status: 503, .. }),
            "{error:?}"
        );
    }
    assert!(next_core_event(&mut ws).await.is_none(), "the event stream ends");
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert_no_more_connections(&server, 3).await;
}

// R2: `Some(0)` never redials.
#[tokio::test]
async fn zero_max_attempts_closes_on_loss_without_redialing() {
    let server = MockWsServer::start(vec![accept_then_close(), Script::accept()]).await;
    let mut ws = core_connect(&server, options(Some(policy(Some(0))))).await;

    assert!(next_core_event(&mut ws).await.is_none(), "the event stream ends");
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert_no_more_connections(&server, 1).await;
}

// R2: `None` imposes no attempt limit.
#[tokio::test]
async fn unlimited_max_attempts_survives_consecutive_failures() {
    let mut scripts = vec![accept_then_close()];
    scripts.extend(vec![Script::Refuse(503); 4]);
    scripts.push(Script::accept());
    let server = MockWsServer::start(scripts).await;
    let mut ws = core_connect(&server, options(Some(policy(None)))).await;

    for _ in 0..4 {
        let error = expect_core_error(&mut ws).await;
        assert!(
            matches!(error, WsError::HandshakeRejected { status: 503, .. }),
            "{error:?}"
        );
    }
    server.wait_for_connections(6, WAIT).await;
    wait_until("Ready after four failures", || ws.state() == WsConnectionState::Ready).await;
    assert_no_more_connections(&server, 6).await;
    let _ = ws.close().await;
}

// R2: a redial that completes resets the count. Three failed redials in
// total, never two in a row, leave a `Some(2)` client alive.
#[tokio::test]
async fn budget_resets_after_each_successful_redial() {
    let server = MockWsServer::start(vec![
        accept_then_close(),
        Script::Refuse(503),
        accept_then_close(),
        Script::Refuse(503),
        accept_then_close(),
        Script::Refuse(503),
        Script::accept(),
    ])
    .await;
    let mut ws = core_connect(&server, options(Some(policy(Some(2))))).await;

    for _ in 0..3 {
        let error = expect_core_error(&mut ws).await;
        assert!(
            matches!(error, WsError::HandshakeRejected { status: 503, .. }),
            "{error:?}"
        );
    }
    server.wait_for_connections(7, WAIT).await;
    wait_until("Ready after the last redial", || ws.state() == WsConnectionState::Ready).await;
    assert_no_more_connections(&server, 7).await;
    let _ = ws.close().await;
}

// No reconnect policy: a lost connection closes the client.
#[tokio::test]
async fn no_reconnect_policy_closes_on_loss() {
    let server = MockWsServer::start(vec![accept_then_close(), Script::accept()]).await;
    let mut ws = core_connect(&server, options(None)).await;

    assert!(next_core_event(&mut ws).await.is_none(), "the event stream ends");
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert_no_more_connections(&server, 1).await;
}

// ---------------------------------------------------- R3: the opening message

fn tts(server: &MockWsServer) -> ElevenLabsTTSWs {
    ElevenLabsTTSWs::with_base_url(server.ws_origin())
}

fn hello(key: Option<&str>) -> TtsInitMessage {
    TtsInitMessage {
        text: "hello".into(),
        xi_api_key: key.map(str::to_string),
        ..Default::default()
    }
}

async fn tts_connect(
    client: ElevenLabsTTSWs,
    opening: TtsInitMessage,
    options: WsClientOptions,
) -> Result<TextToSpeechClient, WsError> {
    client
        .connect_text_to_speech("voice", TextToSpeechConnectionParams::default(), opening, options)
        .await
}

/// The first text frame the server received, kept raw.
async fn first_raw_frame(server: &MockWsServer) -> String {
    server.wait_for_frames(1, WAIT).await;
    match &server.frames()[0].1 {
        Message::Text(text) => text.to_string(),
        other => panic!("expected a text frame, got {other:?}"),
    }
}

// R3: the retained opening, with the handshake key filled in, is the first
// frame of every connection, and is on the wire before the client is Ready:
// a frame sent as soon as Ready is observed always follows it.
#[tokio::test]
async fn opening_message_is_written_first_on_every_connection() {
    let gates = [Barrier::new(), Barrier::new()];
    let held = |gate: &Barrier| {
        Script::Accept(vec![Step::Receive(2), Step::Wait(gate.clone()), Step::Drop])
    };
    let server = MockWsServer::start(vec![
        held(&gates[0]),
        held(&gates[1]),
        Script::Accept(vec![Step::Receive(2)]),
    ])
    .await;
    let ws = tts_connect(tts(&server).api_key(KEY), hello(None), options(Some(policy(Some(2)))))
        .await
        .unwrap();
    let opening = json!({"text": "hello", "xi-api-key": KEY});

    for connection in 0..3 {
        if connection > 0 {
            gates[connection - 1].release();
            server.wait_for_connections(connection + 1, WAIT).await;
        }
        wait_until("Ready", || ws.state() == WsConnectionState::Ready).await;
        ws.send_value(json!({"marker": connection})).await.unwrap();
        server.wait_for_frames(2 * (connection + 1), WAIT).await;
        assert_eq!(
            frames_of(&server, connection),
            [opening.clone(), json!({"marker": connection})],
            "connection {connection}"
        );
    }
    let _ = ws.close().await;
}

// R3 binding cell: an absent `xi-api-key` is filled from the handshake key.
#[tokio::test]
async fn opening_without_a_key_field_is_filled_from_the_credential() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Receive(1)])]).await;
    let ws = tts_connect(tts(&server).api_key(KEY), hello(None), options(None))
        .await
        .unwrap();

    let frame: Value = serde_json::from_str(&first_raw_frame(&server).await).unwrap();
    assert_eq!(frame, json!({"text": "hello", "xi-api-key": KEY}));
    let _ = ws.close().await;
}

// R3 binding cell: an equal value is kept, written once.
#[tokio::test]
async fn opening_with_the_same_key_keeps_one_field() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Receive(1)])]).await;
    let ws = tts_connect(tts(&server).api_key(KEY), hello(Some(KEY)), options(None))
        .await
        .unwrap();

    let raw = first_raw_frame(&server).await;
    assert_eq!(raw.matches("xi-api-key").count(), 1, "{raw}");
    assert_eq!(raw.matches(KEY).count(), 1, "{raw}");
    let frame: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(frame, json!({"text": "hello", "xi-api-key": KEY}));
    let _ = ws.close().await;
}

// R3 binding cell: a different value is a second credential source and is
// refused before any request, without showing either value.
#[tokio::test]
async fn opening_with_a_different_key_is_refused_before_dialing() {
    let server = MockWsServer::start(vec![Script::accept()]).await;
    let error = tts_connect(
        tts(&server).api_key(KEY),
        hello(Some("sentinel-other")),
        options(Some(policy(Some(2)))),
    )
    .await
    .err()
    .expect("the conflicting key is refused");

    assert!(matches!(error, WsError::InvalidCredential(_)), "{error:?}");
    for text in [error.to_string(), format!("{error:?}")] {
        assert!(!text.contains(KEY), "{text}");
        assert!(!text.contains("sentinel-other"), "{text}");
    }
    assert_no_more_connections(&server, 0).await;
    assert!(server.requests().is_empty());
}

// R3 binding cell: with no credential selected, the caller's value is kept
// and the handshake carries none.
#[tokio::test]
#[serial_test::serial]
async fn anonymous_opening_keeps_the_callers_key() {
    let _env = EnvGuard::clear(&ELEVENLABS_ENV);
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Receive(1)])]).await;
    let ws = tts_connect(tts(&server), hello(Some("caller-key")), options(None))
        .await
        .unwrap();

    let frame: Value = serde_json::from_str(&first_raw_frame(&server).await).unwrap();
    assert_eq!(frame, json!({"text": "hello", "xi-api-key": "caller-key"}));
    assert_eq!(server.requests()[0].header("xi-api-key"), None);
    let _ = ws.close().await;
}

// R3 binding cell: an opening with no declared credential field is sent as
// given, even with a key selected.
#[tokio::test]
async fn multi_context_opening_is_sent_unchanged() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Receive(1)])]).await;
    let opening = MultiContextInitMessage {
        text: "hi".into(),
        context_id: "c1".into(),
        voice_settings: None,
        generation_config: None,
    };
    let ws = tts(&server)
        .api_key(KEY)
        .connect_multi_context_text_to_speech(
            "voice",
            MultiContextTextToSpeechConnectionParams::default(),
            opening,
            options(None),
        )
        .await
        .unwrap();

    let frame: Value = serde_json::from_str(&first_raw_frame(&server).await).unwrap();
    assert_eq!(frame, json!({"text": "hi", "context_id": "c1"}));
    assert_eq!(server.requests()[0].header("xi-api-key"), Some(KEY));
    let _ = ws.close().await;
}

// R3: an opening that cannot be written fails the explicit connect; nothing
// reaches the server and nothing redials. The write fails deterministically
// because the frame exceeds the transport's maximum write buffer.
#[tokio::test]
async fn opening_write_failure_fails_the_connect_without_redialing() {
    let server = MockWsServer::start(vec![Script::Accept(vec![Step::Receive(1)])]).await;
    let options = WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(2))
        .reconnect(Some(policy(Some(2))))
        .websocket_config(Some(
            WebSocketConfig::default()
                .write_buffer_size(0)
                .max_write_buffer_size(16),
        ))
        .build();

    let error = tts_connect(tts(&server).api_key(KEY), hello(None), options)
        .await
        .err()
        .expect("the opening does not fit the write buffer");
    assert!(matches!(error, WsError::OpeningWrite(_)), "{error:?}");
    assert!(!error.to_string().contains(KEY), "{error}");
    assert_no_more_connections(&server, 1).await;
    assert!(server.frames().is_empty(), "{:?}", server.frames());
}

// ---------------------------------------------------- R3: the setup callback

// R3: setup runs on the initial connection and again on a redial, before
// Ready, with the handshake credential; a frame setup does not read becomes
// an event.
#[tokio::test]
async fn setup_runs_before_ready_on_every_connection() {
    let drop_gate = Barrier::new();
    let server = MockWsServer::start(vec![
        authed(200, vec![Step::Wait(drop_gate.clone()), Step::Close]),
        authed(
            200,
            vec![Step::text(r#"{"kind":"event","msg":"entity_change","msg_data":{"id":7}}"#)],
        ),
    ])
    .await;
    let runs = Arc::new(AtomicUsize::new(0));
    let mut ws = core_connect_with_auth(&server, options(Some(policy(Some(2)))), &runs)
        .await
        .unwrap();
    assert_eq!(ws.state(), WsConnectionState::Ready);
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    assert_eq!(frames_of(&server, 0), [auth_frame("k")]);

    drop_gate.release();
    server.wait_for_connections(2, WAIT).await;
    wait_until("Ready after the redial", || ws.state() == WsConnectionState::Ready).await;
    assert_eq!(runs.load(Ordering::SeqCst), 2, "Ready before setup completed");
    assert_eq!(frames_of(&server, 1), [auth_frame("k")]);

    let event = next_core_event(&mut ws).await;
    assert_eq!(
        event.unwrap().unwrap(),
        json!({"kind": "event", "msg": "entity_change", "msg_data": {"id": 7}})
    );
    let _ = ws.close().await;
}

// R3: a rejection from setup on the explicit connect is returned, with no
// second attempt of any kind.
#[tokio::test]
async fn setup_rejection_on_connect_is_returned_without_a_second_attempt() {
    let server = MockWsServer::start(vec![authed(401, vec![]), authed(200, vec![])]).await;
    let runs = Arc::new(AtomicUsize::new(0));
    let error = core_connect_with_auth(&server, options(Some(policy(Some(2)))), &runs)
        .await
        .err()
        .expect("setup rejects");

    assert!(matches!(error, WsError::SetupRejected(_)), "{error:?}");
    assert!(error.is_terminal());
    assert_no_more_connections(&server, 1).await;
    assert_eq!(runs.load(Ordering::SeqCst), 0);
}

// R3: a rejection from setup on a redial is terminal: delivered as the last
// event, the client is Closed, and nothing redials.
#[tokio::test]
async fn setup_rejection_on_redial_closes_the_client() {
    let server = MockWsServer::start(vec![
        authed(200, vec![Step::Close]),
        authed(401, vec![]),
        authed(200, vec![]),
    ])
    .await;
    let runs = Arc::new(AtomicUsize::new(0));
    let mut ws = core_connect_with_auth(&server, options(Some(policy(Some(2)))), &runs)
        .await
        .unwrap();

    let error = expect_core_error(&mut ws).await;
    assert!(matches!(error, WsError::SetupRejected(_)), "{error:?}");
    assert!(next_core_event(&mut ws).await.is_none(), "the event stream ends");
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert_no_more_connections(&server, 2).await;
}

// R2/R3: a transient setup failure on a redial uses the budget.
#[tokio::test]
async fn transient_setup_failures_on_redials_use_the_budget() {
    let server = MockWsServer::start(vec![
        authed(200, vec![Step::Close]),
        authed(500, vec![]),
        authed(500, vec![]),
        authed(200, vec![]),
    ])
    .await;
    let runs = Arc::new(AtomicUsize::new(0));
    let mut ws = core_connect_with_auth(&server, options(Some(policy(Some(2)))), &runs)
        .await
        .unwrap();

    for _ in 0..2 {
        let error = expect_core_error(&mut ws).await;
        assert!(matches!(error, WsError::Setup(_)), "{error:?}");
    }
    assert!(next_core_event(&mut ws).await.is_none(), "the event stream ends");
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert_no_more_connections(&server, 3).await;
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

// R3: setup past the attempt timeout fails the connect, drops the
// connection (the server reads EOF), and revokes the session, even one moved
// into another task: its pending and later operations return
// `SessionInvalid`.
#[tokio::test]
async fn setup_cancellation_drops_stream_and_invalidates_session() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let origin = format!("ws://{}", listener.local_addr().unwrap());
    let accepted = Arc::new(AtomicUsize::new(0));
    let (eof_tx, mut eof_rx) = mpsc::unbounded_channel::<usize>();
    let server = {
        let accepted = Arc::clone(&accepted);
        tokio::spawn(async move {
            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let index = accepted.fetch_add(1, Ordering::SeqCst);
                let eof_tx = eof_tx.clone();
                tokio::spawn(async move {
                    let mut ws = tokio_tungstenite::accept_async(stream).await.unwrap();
                    while let Some(Ok(message)) = ws.next().await {
                        if message.is_close() {
                            break;
                        }
                    }
                    let _ = eof_tx.send(index);
                });
            }
        })
    };

    let (outcome_tx, mut outcome_rx) = mpsc::unbounded_channel();
    let options = WsClientOptions::builder()
        .handshake_timeout(Duration::from_millis(300))
        .reconnect(Some(policy(Some(2))))
        .build();
    let started = Instant::now();
    let error = UnfoldedCircleCoreWs::with_base_url(&origin)
        .api_key("k")
        .connect_core_ws_with_setup(options, move |mut session: CoreWsSetup| {
            let outcome_tx = outcome_tx.clone();
            async move {
                tokio::spawn(async move {
                    let pending = session.next_value().await;
                    let later = session.send_value(json!({"late": true})).await;
                    let _ = outcome_tx.send((pending, later));
                });
                std::future::pending::<Result<(), WsSetupError>>().await
            }
        })
        .await
        .err()
        .expect("setup stalls past the attempt timeout");
    assert!(
        matches!(error, WsError::HandshakeTimeout(limit) if limit == Duration::from_millis(300)),
        "{error:?}"
    );
    assert!(started.elapsed() < PROMPT, "{:?}", started.elapsed());

    let eof = tokio::time::timeout(WAIT, eof_rx.recv()).await.expect("the server reads EOF");
    assert_eq!(eof, Some(0));
    let (pending, later) = tokio::time::timeout(WAIT, outcome_rx.recv())
        .await
        .expect("the moved session reports")
        .unwrap();
    assert_eq!(pending.err(), Some(WsSetupError::SessionInvalid));
    assert_eq!(later.err(), Some(WsSetupError::SessionInvalid));

    tokio::time::sleep(QUIET).await;
    assert_eq!(accepted.load(Ordering::SeqCst), 1, "the client dialed again");
    server.abort();
}

// --------------------------------------------------------- close and drop

// A manual close never redials.
#[tokio::test]
async fn manual_close_never_redials() {
    let server = MockWsServer::start(vec![Script::accept()]).await;
    let ws = core_connect(&server, options(Some(policy(None)))).await;

    ws.close().await.unwrap();
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert_no_more_connections(&server, 1).await;
}

// Close interrupts a 60 s reconnect backoff.
#[tokio::test]
async fn close_during_backoff_returns_promptly() {
    let server = MockWsServer::start(vec![accept_then_close(), Script::accept()]).await;
    let ws = core_connect(&server, options(Some(slow_policy(Duration::from_secs(60))))).await;
    wait_until("Disconnected", || ws.state() == WsConnectionState::Disconnected).await;

    let started = Instant::now();
    ws.close().await.unwrap();
    assert!(started.elapsed() < PROMPT, "{:?}", started.elapsed());
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert_no_more_connections(&server, 1).await;
}

// Close interrupts a redial whose setup has stalled.
#[tokio::test]
async fn close_during_stalled_redial_setup_returns_promptly() {
    let server = MockWsServer::start(vec![
        authed(200, vec![Step::Close]),
        Script::Accept(vec![
            Step::text(r#"{"kind":"event","msg":"auth_required"}"#),
            Step::Receive(1),
            Step::Stall,
        ]),
    ])
    .await;
    let options = WsClientOptions::builder()
        .handshake_timeout(Duration::from_secs(4))
        .reconnect(Some(policy(Some(2))))
        .build();
    let runs = Arc::new(AtomicUsize::new(0));
    let ws = core_connect_with_auth(&server, options, &runs).await.unwrap();
    server.wait_for_frames(2, WAIT).await;
    assert_eq!(ws.state(), WsConnectionState::Authenticating);

    let started = Instant::now();
    ws.close().await.unwrap();
    assert!(started.elapsed() < PROMPT, "{:?}", started.elapsed());
    assert_eq!(ws.state(), WsConnectionState::Closed);
    assert_no_more_connections(&server, 2).await;
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

// Dropping the client during a backoff shorter than the watch window stops
// the redial.
#[tokio::test]
async fn dropping_the_client_during_backoff_never_redials() {
    let server = MockWsServer::start(vec![accept_then_close(), Script::accept()]).await;
    let ws = core_connect(&server, options(Some(slow_policy(Duration::from_millis(300))))).await;
    wait_until("Disconnected", || ws.state() == WsConnectionState::Disconnected).await;

    drop(ws);
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert_no_more_connections(&server, 1).await;
}

// ------------------------------------------------------------- deadlines

// An explicit connect makes one attempt: an upgrade that never completes
// times out and is not retried.
#[tokio::test]
async fn stalled_initial_upgrade_times_out_after_one_attempt() {
    let server = MockWsServer::start(vec![Script::Stall, Script::accept()]).await;
    let options = WsClientOptions::builder()
        .handshake_timeout(Duration::from_millis(300))
        .reconnect(Some(policy(None)))
        .build();

    let started = Instant::now();
    let error = UnfoldedCircleCoreWs::with_base_url(server.ws_origin())
        .api_key("test-key")
        .connect_core_ws(options)
        .await
        .err()
        .expect("the upgrade stalls");
    assert!(matches!(error, WsError::HandshakeTimeout(_)), "{error:?}");
    assert!(started.elapsed() < PROMPT, "{:?}", started.elapsed());
    assert_no_more_connections(&server, 1).await;
}

// A ready connection has no lifetime deadline: it outlives the attempt
// timeout, and a redial long after it still gets a full attempt for setup.
#[tokio::test]
async fn ready_connection_and_later_redials_have_no_inherited_deadline() {
    let drop_gate = Barrier::new();
    let server = MockWsServer::start(vec![
        authed(200, vec![Step::Wait(drop_gate.clone()), Step::Close]),
        authed(200, vec![]),
    ])
    .await;
    let options = WsClientOptions::builder()
        .handshake_timeout(Duration::from_millis(300))
        .reconnect(Some(policy(Some(1))))
        .build();
    let runs = Arc::new(AtomicUsize::new(0));
    let connected = Instant::now();
    let ws = core_connect_with_auth(&server, options, &runs).await.unwrap();

    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(ws.state(), WsConnectionState::Ready);
    assert_eq!(server.connections(), 1);

    drop_gate.release();
    server.wait_for_connections(2, WAIT).await;
    wait_until("Ready after the redial", || ws.state() == WsConnectionState::Ready).await;
    assert!(connected.elapsed() > Duration::from_millis(600));
    assert_eq!(runs.load(Ordering::SeqCst), 2);
    let _ = ws.close().await;
}

// ------------------------------------------------- sending while disconnected

// Sending while not Ready fails at once instead of waiting for a redial.
#[tokio::test]
async fn send_while_disconnected_fails_immediately() {
    let server = MockWsServer::start(vec![accept_then_close(), Script::accept()]).await;
    let ws = core_connect(&server, options(Some(slow_policy(Duration::from_secs(60))))).await;
    wait_until("Disconnected", || ws.state() == WsConnectionState::Disconnected).await;

    let started = Instant::now();
    let sent = ws.send_value(json!({"during": "backoff"})).await;
    assert!(matches!(sent, Err(WsError::Disconnected)), "{sent:?}");
    assert!(started.elapsed() < Duration::from_secs(1), "{:?}", started.elapsed());
    let _ = ws.close().await;
}

// Messages sent on a lost connection, and attempts to send while
// disconnected, never reach the replacement connection: its frames are the
// opening followed by what was sent after Ready.
#[tokio::test]
async fn messages_from_a_lost_connection_are_not_replayed() {
    let drop_gate = Barrier::new();
    let server = MockWsServer::start(vec![
        // Reads only the opening, so later frames stay unread in transit.
        Script::Accept(vec![Step::Receive(1), Step::Wait(drop_gate.clone()), Step::Drop]),
        Script::Accept(vec![Step::Receive(2)]),
    ])
    .await;
    let ws = tts_connect(
        tts(&server).api_key(KEY),
        hello(None),
        options(Some(slow_policy(Duration::from_millis(300)))),
    )
    .await
    .unwrap();
    for stale in 0..3 {
        ws.send_value(json!({"stale": stale})).await.unwrap();
    }

    drop_gate.release();
    wait_until("the loss", || ws.state() != WsConnectionState::Ready).await;
    let sent = ws.send_value(json!({"stale": "while disconnected"})).await;
    assert!(matches!(sent, Err(WsError::Disconnected)), "{sent:?}");

    server.wait_for_connections(2, WAIT).await;
    wait_until("Ready after the redial", || ws.state() == WsConnectionState::Ready).await;
    ws.send_value(json!({"fresh": true})).await.unwrap();
    server.wait_for_frames(3, WAIT).await;
    assert_eq!(
        frames_of(&server, 1),
        [json!({"text": "hello", "xi-api-key": KEY}), json!({"fresh": true})]
    );
    let _ = ws.close().await;
}
