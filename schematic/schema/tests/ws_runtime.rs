//! Runtime integration tests for generated WebSocket clients.

#![allow(clippy::result_large_err)]

mod common;
mod ws_mock;

use ws_mock::no_setup;

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use schematic_schema::elevenlabs_ws::{
    ElevenLabsTTSWs, MultiContextInitMessage, MultiContextTextToSpeechConnectionParams,
    MultiContextTextToSpeechServerMessage, TextToSpeechConnectionParams, TextToSpeechServerMessage,
    TtsInitMessage,
};
use schematic_schema::samsung_smart_tv_remote_ws::{
    ArtModeConnectionParams, ArtModeServerMessage, RemoteControlConnectionParams,
    RemoteControlServerMessage, SamsungRemoteEventName, SamsungRemoteKnownEvent, SamsungSmartTvRemoteWs,
};
use schematic_schema::unfolded_circle_core_ws::{
    CoreWsClientMessage, CoreWsEnvelopeKind, CoreWsEventCategory, CoreWsKnownCategory,
    CoreWsKnownMessage, CoreWsMessageName, CoreWsPingMessage, CoreWsServerMessage, UnfoldedCircleCoreWs,
};
use schematic_schema::unfolded_circle_dock_ws::{
    DockRootServerMessage, DockWsAuthRequired, DockWsEnvelopeType, DockWsKnownMessage, DockWsMessageName,
    DockWsResponseEnvelope, UnfoldedCircleDockWs,
};
use schematic_schema::unfolded_circle_integration_ws::{
    IntegrationServerMessage, IntegrationWsKnownMessage, IntegrationWsMessageName,
    UnfoldedCircleIntegrationWs,
};
use schematic_schema::ws_shared::{
    ReconnectPolicy, WsClientOptions, WsConnectionState, WsError, WsRawFrame,
};
use serde_json::json;
use serde_json::value::RawValue;
use tokio::net::TcpListener;
use tokio_tungstenite::accept_async;
use tokio_tungstenite::accept_hdr_async;
use tokio_tungstenite::tungstenite::Message;

fn parse_json_message(msg: Message) -> serde_json::Value {
    match msg {
        Message::Text(text) => serde_json::from_str(text.as_ref()).expect("valid json text"),
        Message::Binary(bytes) => serde_json::from_slice(bytes.as_ref()).expect("valid json bytes"),
        other => panic!("unexpected message type: {other:?}"),
    }
}

#[tokio::test]
async fn ws_connect_substitutes_path_and_query_params() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (uri_tx, uri_rx) = tokio::sync::oneshot::channel::<String>();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let _ws = accept_hdr_async(
            stream,
            move |request: &tokio_tungstenite::tungstenite::handshake::server::Request,
                  response| {
                let _ = uri_tx.send(request.uri().to_string());
                Ok(response)
            },
        )
        .await
        .unwrap();
    });

    let mut client = ElevenLabsTTSWs::with_base_url(format!("ws://{addr}"));
    client = client.api_key("test-key");

    let params = TextToSpeechConnectionParams {
        model_id: Some("eleven turbo".to_string()),
        output_format: Some("mp3_44100_128".to_string()),
        ..Default::default()
    };

    let ws = client
        .connect_text_to_speech("voice 42", params, TtsInitMessage::default(), WsClientOptions::default())
        .await
        .unwrap();
    let _ = ws.close().await;

    let request_uri = tokio::time::timeout(Duration::from_secs(2), uri_rx)
        .await
        .expect("server should observe handshake path")
        .expect("uri channel should carry value");

    assert!(
        request_uri.starts_with("/v1/text-to-speech/voice%2042/stream-input?"),
        "unexpected handshake path: {request_uri}"
    );
    assert!(request_uri.contains("model_id=eleven%20turbo"));
    assert!(request_uri.contains("output_format=mp3_44100_128"));

    server.await.unwrap();
}

#[tokio::test]
async fn ws_correlated_request_uses_options_timeout() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        let _ = ws.next().await;
        tokio::time::sleep(Duration::from_millis(300)).await;
    });

    let mut client = UnfoldedCircleCoreWs::with_base_url(format!("ws://{addr}"));
    client = client.api_key("test-key");

    let options = WsClientOptions::builder()
        .request_timeout(Duration::from_millis(100))
        .build();
    let ws = client.connect_core_ws_with_setup(options, no_setup).await.unwrap();

    let err = ws
        .request_value(serde_json::json!({"cmd": "no_response"}))
        .await
        .unwrap_err();
    assert!(matches!(err, WsError::RequestTimeout(_)));

    let _ = ws.close().await;
    server.await.unwrap();
}

#[tokio::test]
async fn ws_disconnect_fails_pending_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut ws = accept_async(stream).await.unwrap();
        let _ = ws.next().await;
        let _ = ws.close(None).await;
    });

    let mut client = UnfoldedCircleCoreWs::with_base_url(format!("ws://{addr}"));
    client = client.api_key("test-key");

    let options = WsClientOptions::builder()
        .request_timeout(Duration::from_secs(2))
        .build();
    let ws = client.connect_core_ws_with_setup(options, no_setup).await.unwrap();

    let err = ws
        .request_value(serde_json::json!({"cmd": "disconnect_me"}))
        .await
        .unwrap_err();
    assert!(matches!(err, WsError::Disconnected));

    server.await.unwrap();
}

#[tokio::test]
async fn ws_reconnect_policy_redials_after_disconnect() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server = tokio::spawn(async move {
        let (stream1, _) = listener.accept().await.unwrap();
        let mut ws1 = accept_async(stream1).await.unwrap();
        let _ = ws1.next().await;
        let _ = ws1.close(None).await;

        let (stream2, _) = listener.accept().await.unwrap();
        let mut ws2 = accept_async(stream2).await.unwrap();
        if let Some(Ok(msg)) = ws2.next().await {
            let request = parse_json_message(msg);
            let req_id = request.get("id").and_then(|v| v.as_u64()).unwrap_or(0);
            let response = serde_json::json!({"req_id": req_id, "ok": true});
            ws2.send(Message::Text(response.to_string().into()))
                .await
                .unwrap();
        }
    });

    let mut client = UnfoldedCircleCoreWs::with_base_url(format!("ws://{addr}"));
    client = client.api_key("test-key");

    let reconnect = ReconnectPolicy {
        initial_backoff: Duration::from_millis(25),
        max_backoff: Duration::from_millis(100),
        multiplier: 1.5,
        jitter: 0.0,
        max_attempts: Some(5),
    };
    let options = WsClientOptions::builder()
        .request_timeout(Duration::from_secs(2))
        .reconnect(Some(reconnect))
        .build();
    let ws = client.connect_core_ws_with_setup(options, no_setup).await.unwrap();

    let first_err = ws
        .request_value(serde_json::json!({"cmd": "first"}))
        .await
        .unwrap_err();
    assert!(matches!(first_err, WsError::Disconnected));

    let started = Instant::now();
    while ws.state() != WsConnectionState::Ready && started.elapsed() < Duration::from_secs(3) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(ws.state(), WsConnectionState::Ready);

    let response = ws
        .request_value(serde_json::json!({"cmd": "second"}))
        .await
        .unwrap();
    assert_eq!(response.get("ok").and_then(|v| v.as_bool()), Some(true));

    let _ = ws.close().await;
    server.await.unwrap();
}

/// R1: each Core endpoint, dialed from the generated client's default base
/// URL (only its host replaced by the mock's), reaches its documented
/// channel path once — not `/ws/ws`.
#[tokio::test]
#[serial_test::serial]
async fn core_endpoints_dial_their_documented_channel_paths() {
    let _env = common::EnvGuard::clear(&["UCR_CORE_API_KEY", "UNFOLDED_CIRCLE_API_KEY"]);
    let server = ws_mock::MockWsServer::start(vec![ws_mock::Script::accept()]).await;

    let default_base = UnfoldedCircleCoreWs::new().base_url().to_string();
    assert_eq!(default_base, "ws://remote.local");
    let (_, after_scheme) = default_base.split_once("://").unwrap();
    let base_path = after_scheme.find('/').map_or("", |index| &after_scheme[index..]);
    let client = UnfoldedCircleCoreWs::with_base_url(format!("{}{base_path}", server.ws_origin()));

    client.connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap().close().await.unwrap();
    client
        .connect_core_integrations_with_setup(WsClientOptions::default(), no_setup)
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
    client
        .connect_core_profiles_with_setup(WsClientOptions::default(), no_setup)
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
    client
        .connect_core_events_with_setup(WsClientOptions::default(), no_setup)
        .await
        .unwrap()
        .close()
        .await
        .unwrap();

    server.wait_for_requests(4, Duration::from_secs(5)).await;
    let requests = server.requests();
    let targets: Vec<String> = requests.iter().map(ws_mock::RecordedRequest::target).collect();
    assert_eq!(targets, ["/ws", "/intg", "/profiles", "/events"]);
    assert!(
        requests.iter().all(|request| request.header("api-key").is_none()),
        "no credential variable is set, so no API key is sent"
    );
    server.shutdown().await;
}

#[tokio::test]
async fn elevenlabs_handshake_encodes_one_path_segment_and_omits_unset_queries() {
    let server = ws_mock::MockWsServer::start(vec![ws_mock::Script::accept()]).await;
    // A base path is a prefix the endpoint path extends, never replaces.
    let client = ElevenLabsTTSWs::with_base_url(format!("{}/proxy", server.ws_origin()));

    client
        .connect_text_to_speech(
            "a/b c?d",
            TextToSpeechConnectionParams::default(),
            TtsInitMessage::default(),
            WsClientOptions::default(),
        )
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
    let params = TextToSpeechConnectionParams {
        seed: Some(7),
        enable_logging: Some(false),
        ..Default::default()
    };
    client
        .connect_text_to_speech("v", params, TtsInitMessage::default(), WsClientOptions::default())
        .await
        .unwrap()
        .close()
        .await
        .unwrap();

    server.wait_for_requests(2, Duration::from_secs(5)).await;
    let targets: Vec<String> =
        server.requests().iter().map(ws_mock::RecordedRequest::target).collect();
    assert_eq!(
        targets,
        [
            "/proxy/v1/text-to-speech/a%2Fb%20c%3Fd/stream-input",
            "/proxy/v1/text-to-speech/v/stream-input?enable_logging=false&seed=7",
        ]
    );
    server.shutdown().await;
}

// ─── R10: typed inbound selection over the loopback mock ────────────────────

/// Generous bound on one inbound event; a passing test takes milliseconds.
const EVENT_WAIT: Duration = Duration::from_secs(5);

/// A script that sends each frame as a text frame, then stays open.
fn sends_text(frames: &[&str]) -> ws_mock::Script {
    ws_mock::Script::Accept(frames.iter().map(|frame| ws_mock::Step::text(*frame)).collect())
}

/// Awaits one receive call, panicking if it takes longer than [`EVENT_WAIT`]
/// or the event stream ended.
async fn received<T>(next: impl std::future::Future<Output = Option<Result<T, WsError>>>) -> Result<T, WsError> {
    tokio::time::timeout(EVENT_WAIT, next)
        .await
        .expect("an inbound event within the wait")
        .expect("the connection is still open")
}

/// Like [`received`], but the event must have decoded.
async fn decoded<T>(next: impl std::future::Future<Output = Option<Result<T, WsError>>>) -> T {
    received(next).await.unwrap_or_else(|error| panic!("expected a decoded event, got {error}"))
}

fn raw_json(raw: &RawValue) -> serde_json::Value {
    serde_json::from_str(raw.get()).expect("msg_data is JSON")
}

/// A Core client for the mock, with an explicit key so no credential
/// variable is ever read.
fn core_client(server: &ws_mock::MockWsServer) -> UnfoldedCircleCoreWs {
    let mut client = UnfoldedCircleCoreWs::with_base_url(server.ws_origin());
    client = client.api_key("test-key");
    client
}

/// R10: every `CoreWsServerMessage` variant, each from a frame shaped as the
/// official UC Core AsyncAPI documents it
/// (`schematic/specs/unfolded-circle/UCR-core-asyncapi.yaml`). The four Core
/// endpoints (`/ws`, `/intg`, `/profiles`, `/events`) declare the same
/// message list and selectors, so `/ws` stands for all four.
///
/// Every `req_id` here matches no pending request, so correlation passes each
/// frame through to the event queue.
#[tokio::test]
async fn core_ws_decodes_every_server_message_variant() {
    let frames = [
        // `authRequiredEvent`
        r#"{"kind":"event","msg":"auth_required","msg_data":{"api":{"name":"UC Core API","version":"0.20.0"}}}"#,
        // `authMsg`
        r#"{"kind":"resp","req_id":901,"code":200,"msg":"authentication","msg_data":{"core":"0.40.3"}}"#,
        // `pong`: `msg_data` echoes the ping's payload
        r#"{"kind":"resp","req_id":902,"code":200,"msg":"pong","msg_data":{"ts":1700000000}}"#,
        // `versionInfoMsg`
        r#"{"kind":"resp","req_id":903,"code":200,"msg":"version_info","msg_data":{"model":"UCR3","device_name":"Remote 3","hostname":"Remote3-A1B2C3","address":"AA:BB:CC:DD:EE:FF","api":"0.20.0","core":"0.40.3","ui":"0.50.1","os":"2.0.18","integrations":{"uc.main":"0.1.0"}}}"#,
        // `systemInfoMsg`
        r#"{"kind":"resp","req_id":904,"code":200,"msg":"system_info","msg_data":{"model_name":"Remote 3","model_number":"ucr3-02","serial_number":"8C1F64012345","hw_revision":"rev3"}}"#,
        // `result`: the spec's second example, verbatim
        r#"{"kind":"resp","req_id":124,"msg":"result","code":400,"msg_data":{"code":"INV_ARGUMENT","message":"Invalid argument. Foo must be a positive number."}}"#,
        // `commonEvent`: an entity change on the ENTITY category
        r#"{"kind":"event","msg":"entity_change","cat":"ENTITY","ts":"2024-05-01T12:00:00.000Z","msg_data":{"entity_id":"uc.main.light.1","entity_type":"light","new_state":{"attributes":{"state":"ON"}}}}"#,
        // `factoryResetTokenMsg`: a documented response with no message of its
        // own in the definition, so it reaches the `ResponseEnvelope` fallback
        r#"{"kind":"resp","req_id":905,"code":200,"msg":"factory_reset_token","msg_data":{"token":"8d1f2c"}}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let mut ws = core_client(&server).connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::AuthRequired(auth_required) = event else { panic!("expected AuthRequired, got {event:?}") };
    assert_eq!(auth_required.kind, CoreWsEnvelopeKind::Event);
    assert_eq!(auth_required.msg, CoreWsKnownMessage::AuthRequired);
    assert_eq!(raw_json(auth_required.msg_data.as_deref().unwrap())["api"]["name"], "UC Core API");

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::Authentication(authentication) = event else { panic!("expected Authentication, got {event:?}") };
    assert_eq!(raw_json(authentication.msg_data.as_deref().unwrap()), json!({"core": "0.40.3"}));

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::Pong(pong) = event else { panic!("expected Pong, got {event:?}") };
    assert_eq!(pong.req_id, 902);
    assert_eq!(raw_json(pong.msg_data.as_deref().unwrap()), json!({"ts": 1_700_000_000}));

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::VersionInfo(version_info) = event else { panic!("expected VersionInfo, got {event:?}") };
    assert_eq!(version_info.req_id, 903);
    assert_eq!(raw_json(version_info.msg_data.as_deref().unwrap())["model"], "UCR3");

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::SystemInfo(system_info) = event else { panic!("expected SystemInfo, got {event:?}") };
    assert_eq!(raw_json(system_info.msg_data.as_deref().unwrap())["model_number"], "ucr3-02");

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::Result(result) = event else { panic!("expected Result, got {event:?}") };
    assert_eq!(result.req_id, 124);
    assert_eq!(raw_json(result.msg_data.as_deref().unwrap())["code"], "INV_ARGUMENT");

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::EventEnvelope(entity_event) = event else { panic!("expected EventEnvelope, got {event:?}") };
    assert_eq!(entity_event.msg, CoreWsMessageName::Known(CoreWsKnownMessage::EntityChange));
    assert_eq!(entity_event.cat, Some(CoreWsEventCategory::Known(CoreWsKnownCategory::Entity)));
    assert_eq!(raw_json(&entity_event.msg_data)["entity_id"], "uc.main.light.1");

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::ResponseEnvelope(response) = event else { panic!("expected ResponseEnvelope, got {event:?}") };
    assert_eq!(response.req_id, 905);
    assert_eq!(response.msg, CoreWsMessageName::Other("factory_reset_token".to_string()));
    assert_eq!(response.code, 200);
    assert_eq!(raw_json(response.msg_data.as_deref().unwrap()), json!({"token": "8d1f2c"}));

    server.shutdown().await;
}

/// R10: a frame that two declared messages would both decode is selected by
/// the most specific selector, and a frame no specific selector names reaches
/// the direction's fallback.
#[tokio::test]
async fn core_ws_selects_the_most_specific_message_before_the_fallback() {
    let frames = [
        // Also a valid `ResponseEnvelope` (it has `kind`, `req_id`, `msg`, `code`).
        r#"{"kind":"resp","req_id":911,"code":200,"msg":"pong","msg_data":{}}"#,
        r#"{"kind":"resp","req_id":912,"code":200,"msg":"something_undocumented","msg_data":{"x":1}}"#,
        // Also a valid `EventEnvelope` (it carries the required `msg_data`).
        r#"{"kind":"event","msg":"auth_required","msg_data":{}}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let mut ws = core_client(&server).connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::Pong(pong) = event else { panic!("expected Pong, got {event:?}") };
    assert_eq!(pong.req_id, 911);

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::ResponseEnvelope(response) = event else { panic!("expected ResponseEnvelope, got {event:?}") };
    assert_eq!(response.msg, CoreWsMessageName::Other("something_undocumented".to_string()));
    assert_eq!(response.req_id, 912);

    let event = decoded(ws.next_event()).await;
    assert!(matches!(event, CoreWsServerMessage::AuthRequired(_)), "expected AuthRequired, got {event:?}");

    server.shutdown().await;
}

/// R10: a text frame that is not JSON is reported with the exact text and
/// the connection stays usable.
#[tokio::test]
async fn core_ws_reports_a_non_json_text_frame_and_keeps_decoding() {
    let malformed = "{\"kind\":\"resp\", not json at all";
    let frames = [malformed, r#"{"kind":"resp","req_id":921,"code":200,"msg":"pong","msg_data":{}}"#];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let mut ws = core_client(&server).connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    match received(ws.next_event()).await {
        Err(WsError::Decode { frame: WsRawFrame::Text(text), reason }) => {
            assert_eq!(text, malformed);
            assert!(reason.starts_with("not JSON"), "unexpected reason: {reason}");
        }
        other => panic!("expected a text Decode error, got {other:?}"),
    }

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::Pong(pong) = event else { panic!("expected Pong, got {event:?}") };
    assert_eq!(pong.req_id, 921);
    assert_eq!(ws.state(), WsConnectionState::Ready);

    server.shutdown().await;
}

/// R10: a binary frame that is not JSON is reported with the exact bytes.
#[tokio::test]
async fn core_ws_reports_a_non_json_binary_frame_with_its_bytes() {
    let malformed: Vec<u8> = vec![0x00, 0x9f, 0x92, 0x96, b'{', 0xff];
    let script = ws_mock::Script::Accept(vec![
        ws_mock::Step::Send(Message::Binary(malformed.clone().into())),
        ws_mock::Step::text(r#"{"kind":"resp","req_id":931,"code":200,"msg":"pong","msg_data":{}}"#),
    ]);
    let server = ws_mock::MockWsServer::start(vec![script]).await;
    let mut ws = core_client(&server).connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    match received(ws.next_event()).await {
        Err(WsError::Decode { frame: WsRawFrame::Binary(bytes), .. }) => assert_eq!(bytes, malformed),
        other => panic!("expected a binary Decode error, got {other:?}"),
    }

    let event = decoded(ws.next_event()).await;
    assert!(matches!(event, CoreWsServerMessage::Pong(_)), "expected Pong, got {event:?}");
    assert_eq!(ws.state(), WsConnectionState::Ready);

    server.shutdown().await;
}

/// R10: a frame whose selectors name a message but whose payload does not
/// decode as it is reported with the raw text and the message's name; it
/// does not fall through to the fallback.
#[tokio::test]
async fn core_ws_reports_a_selected_payload_that_does_not_decode() {
    // Selects `Pong`, but lacks the required `req_id`.
    let incomplete = r#"{"kind":"resp","msg":"pong"}"#;
    let server = ws_mock::MockWsServer::start(vec![sends_text(&[incomplete])]).await;
    let mut ws = core_client(&server).connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    match received(ws.next_event()).await {
        Err(WsError::Decode { frame: WsRawFrame::Text(text), reason }) => {
            assert_eq!(text, incomplete);
            assert!(reason.contains("`Pong`"), "reason should name the selected message: {reason}");
        }
        other => panic!("expected a Decode error, got {other:?}"),
    }
    assert_eq!(ws.state(), WsConnectionState::Ready);

    server.shutdown().await;
}

/// R10: typed and value receivers read one queue, so each frame reaches
/// exactly one of them, in arrival order.
#[tokio::test]
async fn core_ws_typed_and_value_receivers_share_one_queue() {
    let frames = [
        r#"{"kind":"resp","req_id":941,"code":200,"msg":"pong","msg_data":{}}"#,
        r#"{"kind":"resp","req_id":942,"code":200,"msg":"pong","msg_data":{}}"#,
        r#"{"kind":"resp","req_id":943,"code":200,"msg":"pong","msg_data":{}}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let mut ws = core_client(&server).connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    let first = decoded(ws.next_value_event()).await;
    assert_eq!(first, serde_json::from_str::<serde_json::Value>(frames[0]).unwrap());

    let event = decoded(ws.next_event()).await;
    let CoreWsServerMessage::Pong(second) = event else { panic!("expected Pong, got {event:?}") };
    assert_eq!(second.req_id, 942);

    // Neither earlier frame is delivered again: the third is next.
    let third = decoded(ws.next_value_event()).await;
    assert_eq!(third["req_id"], 943);

    server.shutdown().await;
}

/// R10: `events()` consumes the client and keeps its connection open; a frame
/// the server sends only after the stream has yielded still arrives.
#[tokio::test]
async fn core_ws_events_stream_keeps_the_connection_open() {
    let barrier = ws_mock::Barrier::new();
    let script = ws_mock::Script::Accept(vec![
        ws_mock::Step::text(r#"{"kind":"resp","req_id":951,"code":200,"msg":"pong","msg_data":{}}"#),
        ws_mock::Step::Wait(barrier.clone()),
        ws_mock::Step::text(r#"{"kind":"event","msg":"entity_change","cat":"ENTITY","msg_data":{"entity_id":"uc.main.light.1"}}"#),
    ]);
    let server = ws_mock::MockWsServer::start(vec![script]).await;
    let ws = core_client(&server).connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap();
    let mut events = ws.events();

    let event = decoded(events.next()).await;
    let CoreWsServerMessage::Pong(pong) = event else { panic!("expected Pong, got {event:?}") };
    assert_eq!(pong.req_id, 951);

    barrier.release();
    let event = decoded(events.next()).await;
    let CoreWsServerMessage::EventEnvelope(entity_event) = event else { panic!("expected EventEnvelope, got {event:?}") };
    assert_eq!(entity_event.msg, CoreWsMessageName::Known(CoreWsKnownMessage::EntityChange));

    drop(events);
    server.shutdown().await;
}

/// R10: a typed send writes the payload alone — no variant tag or wrapper —
/// and `send_value` writes its value as given.
#[tokio::test]
async fn core_ws_typed_send_writes_the_payload_alone() {
    let script = ws_mock::Script::Accept(vec![ws_mock::Step::Receive(2)]);
    let server = ws_mock::MockWsServer::start(vec![script]).await;
    let ws = core_client(&server).connect_core_ws_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    let ping = CoreWsPingMessage {
        kind: CoreWsEnvelopeKind::Req,
        id: 7,
        msg: CoreWsKnownMessage::Ping,
        msg_data: Some(RawValue::from_string(r#"{"ts":1700000000}"#.to_string()).unwrap()),
    };
    ws.send(CoreWsClientMessage::Ping(ping)).await.unwrap();
    let untyped = json!({"kind": "req", "id": 8, "msg": "version"});
    ws.send_value(untyped.clone()).await.unwrap();

    server.wait_for_frames(2, EVENT_WAIT).await;
    let frames = server.frames();
    let texts: Vec<&str> = frames
        .iter()
        .map(|(_, message)| match message {
            Message::Text(text) => text.as_str(),
            other => panic!("expected a text frame, got {other:?}"),
        })
        .collect();
    assert_eq!(texts[0], r#"{"kind":"req","id":7,"msg":"ping","msg_data":{"ts":1700000000}}"#);
    assert_eq!(serde_json::from_str::<serde_json::Value>(texts[1]).unwrap(), untyped);

    server.shutdown().await;
}

// Recorded Dock frames, verbatim from the UCD2 AsyncAPI examples
// (`schematic/specs/unfolded-circle/UCD2-asyncapi.yaml`).
const DOCK_AUTH_REQUIRED: &str = r#"{"type":"auth_required","model":"UCD2","revision":"5.4","version":"0.6.0"}"#;
const DOCK_SYSINFO_REPLY: &str = r#"{"type":"dock","req_id":2,"msg":"get_sysinfo","code":200,"name":"My dock","hostname":"UC-Dock-1234","model":"UCD2","revision":"5.4","version":"0.4.0","serial":"0042","ir_learning":false}"#;
const DOCK_IR_RECEIVE: &str = r#"{"type":"event","msg":"ir_receive","ir_code":"4;0x10;12;0"}"#;

/// Asserts `response` is the recorded `get_sysinfo` reply, result fields included.
fn assert_dock_sysinfo_reply(response: &DockWsResponseEnvelope) {
    assert_eq!(response.type_name, DockWsEnvelopeType::Dock);
    assert_eq!(response.req_id, Some(2));
    assert_eq!(response.msg, DockWsMessageName::Known(DockWsKnownMessage::GetSysinfo));
    assert_eq!(response.code, Some(200));
    assert_eq!(response.data["name"], "My dock");
    assert_eq!(response.data["model"], "UCD2");
    assert_eq!(response.data["version"], "0.4.0");
    assert_eq!(response.data["ir_learning"], false);
}

/// R10: every `DockRootServerMessage` variant, each from an unmodified
/// recorded frame (the `authResponseMsg` and `pong` schemas have no example,
/// so those two are their required fields alone).
#[tokio::test]
async fn dock_root_decodes_every_server_message_variant() {
    let frames = [
        DOCK_AUTH_REQUIRED,
        r#"{"type":"auth","msg":"authentication","code":200}"#,
        DOCK_IR_RECEIVE,
        DOCK_SYSINFO_REPLY,
        r#"{"type":"dock","msg":"pong"}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let client = UnfoldedCircleDockWs::with_base_url(server.ws_origin());
    let mut ws = client.connect_dock_root_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    let event = decoded(ws.next_event()).await;
    let DockRootServerMessage::AuthRequired(auth_required) = event else { panic!("expected AuthRequired, got {event:?}") };
    assert_eq!(
        auth_required,
        DockWsAuthRequired {
            type_name: DockWsEnvelopeType::AuthRequired,
            model: Some("UCD2".to_string()),
            revision: Some("5.4".to_string()),
            version: Some("0.6.0".to_string()),
        }
    );

    let event = decoded(ws.next_event()).await;
    let DockRootServerMessage::Authentication(authentication) = event else { panic!("expected Authentication, got {event:?}") };
    assert_eq!(authentication.msg, DockWsKnownMessage::Authentication);
    assert_eq!(authentication.code, 200);

    let event = decoded(ws.next_event()).await;
    let DockRootServerMessage::EventEnvelope(ir_event) = event else { panic!("expected EventEnvelope, got {event:?}") };
    assert_eq!(ir_event.msg, DockWsMessageName::Known(DockWsKnownMessage::IrReceive));
    assert_eq!(ir_event.data["ir_code"], "4;0x10;12;0");

    let event = decoded(ws.next_event()).await;
    let DockRootServerMessage::ResponseEnvelope(response) = event else { panic!("expected ResponseEnvelope, got {event:?}") };
    assert_dock_sysinfo_reply(&response);

    let event = decoded(ws.next_event()).await;
    let DockRootServerMessage::ResponseEnvelope(pong) = event else { panic!("expected ResponseEnvelope, got {event:?}") };
    assert_eq!(pong.msg, DockWsMessageName::Known(DockWsKnownMessage::Pong));
    assert_eq!((pong.req_id, pong.code), (None, None));

    server.shutdown().await;
}

/// R10: `next_value_event()` returns recorded Dock frames exactly as sent,
/// and the typed `events()` stream decodes the same recorded reply as
/// `next_event()` does.
#[tokio::test]
async fn dock_root_value_and_typed_streams_keep_recorded_frames() {
    let frames = [DOCK_SYSINFO_REPLY, DOCK_IR_RECEIVE, DOCK_SYSINFO_REPLY, DOCK_IR_RECEIVE];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let client = UnfoldedCircleDockWs::with_base_url(server.ws_origin());
    let mut ws = client.connect_dock_root_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    let reply: serde_json::Value = serde_json::from_str(DOCK_SYSINFO_REPLY).unwrap();
    let ir_event: serde_json::Value = serde_json::from_str(DOCK_IR_RECEIVE).unwrap();
    assert_eq!(decoded(ws.next_value_event()).await, reply);
    assert_eq!(decoded(ws.next_value_event()).await, ir_event);

    let mut events = ws.events();
    let event = decoded(events.next()).await;
    let DockRootServerMessage::ResponseEnvelope(response) = event else { panic!("expected ResponseEnvelope, got {event:?}") };
    assert_dock_sysinfo_reply(&response);
    let event = decoded(events.next()).await;
    let DockRootServerMessage::EventEnvelope(ir) = event else { panic!("expected EventEnvelope, got {event:?}") };
    assert_eq!(ir.data["ir_code"], "4;0x10;12;0");

    drop(events);
    server.shutdown().await;
}

/// R10: every `IntegrationServerMessage` variant (the driver's frames, as the
/// remote's client receives them), shaped as the UC Integration AsyncAPI
/// documents them (`schematic/gen/tests/fixtures/asyncapi/UCR-integration-asyncapi.json`).
#[tokio::test]
async fn integration_decodes_every_server_message_variant() {
    let frames = [
        // `authRequiredEvent`, with the optional `driverVersion` payload
        r#"{"kind":"event","msg":"auth_required","msg_data":{"name":"Demo driver","version":{"api":"0.12.1","driver":"1.0.0"}}}"#,
        // `authMsg`: a driver without auth still sends this with `req_id: 0`
        r#"{"kind":"resp","req_id":0,"code":200,"msg":"authentication","msg_data":{"name":"Demo driver","version":{"api":"0.12.1","driver":"1.0.0"}}}"#,
        // `deviceStateEventMsg`
        r#"{"kind":"event","msg":"device_state","cat":"DEVICE","msg_data":{"state":"CONNECTED"}}"#,
        // `driverVersionMsg`: no message of its own, so the fallback
        r#"{"kind":"resp","req_id":3,"code":200,"msg":"driver_version","msg_data":{"name":"Demo driver","version":{"api":"0.12.1","driver":"1.0.0"}}}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let mut client = UnfoldedCircleIntegrationWs::with_base_url(server.ws_origin());
    client = client.api_key("test-token");
    let mut ws = client.connect_integration_with_setup(WsClientOptions::default(), no_setup).await.unwrap();

    let event = decoded(ws.next_event()).await;
    let IntegrationServerMessage::AuthRequired(auth_required) = event else { panic!("expected AuthRequired, got {event:?}") };
    assert_eq!(auth_required.msg, IntegrationWsKnownMessage::AuthRequired);
    assert_eq!(raw_json(auth_required.msg_data.as_deref().unwrap())["name"], "Demo driver");

    let event = decoded(ws.next_event()).await;
    let IntegrationServerMessage::Authentication(authentication) = event else { panic!("expected Authentication, got {event:?}") };
    assert_eq!(raw_json(authentication.msg_data.as_deref().unwrap())["name"], "Demo driver");

    let event = decoded(ws.next_event()).await;
    let IntegrationServerMessage::EventEnvelope(device_state) = event else { panic!("expected EventEnvelope, got {event:?}") };
    assert_eq!(device_state.msg, IntegrationWsMessageName::Known(IntegrationWsKnownMessage::DeviceState));
    assert_eq!(device_state.cat.as_deref(), Some("DEVICE"));
    assert_eq!(raw_json(&device_state.msg_data), json!({"state": "CONNECTED"}));

    let event = decoded(ws.next_event()).await;
    let IntegrationServerMessage::ResponseEnvelope(response) = event else { panic!("expected ResponseEnvelope, got {event:?}") };
    assert_eq!(response.req_id, 3);
    assert_eq!(response.msg, IntegrationWsMessageName::Known(IntegrationWsKnownMessage::DriverVersion));
    assert_eq!(raw_json(response.msg_data.as_deref().unwrap())["version"]["driver"], "1.0.0");

    server.shutdown().await;
}

/// R10: every `RemoteControlServerMessage` variant. The connect frame is the
/// one recorded in `homelab/docs/samsung/samsung-s95-tcp-api.md` (§4.1.1); the
/// repository records no unauthorized, error, or installed-apps frame, so
/// those follow the shapes the definition documents
/// (`schematic/definitions/src/samsung_smart_tv/remote_ws/`).
#[tokio::test]
async fn samsung_remote_control_decodes_every_server_message_variant() {
    let frames = [
        r#"{"event":"ms.channel.connect","data":{"token":"83746291","id":"abc123"}}"#,
        r#"{"event":"ms.channel.unauthorized"}"#,
        r#"{"event":"ms.error","data":{"message":"unrecognized method value : ms.remote.contro"}}"#,
        // An `ms.channel.emit` reply with no message of its own: the fallback
        r#"{"event":"ed.installedApp.get","from":"host","data":{"data":[{"appId":"111299001912","app_type":2,"name":"YouTube"}]}}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let client = SamsungSmartTvRemoteWs::with_base_url(server.ws_origin());
    let params = RemoteControlConnectionParams {
        name: "TXlSZW1vdGVBcHA=".to_string(),
        token: None,
    };
    let mut ws = client.connect_remote_control(params, WsClientOptions::default()).await.unwrap();

    let event = decoded(ws.next_event()).await;
    let RemoteControlServerMessage::ChannelConnectEvent(connect) = event else { panic!("expected ChannelConnectEvent, got {event:?}") };
    let data = connect.data.expect("connect data");
    assert_eq!(data.token.as_deref(), Some("83746291"));
    assert_eq!(data.id.as_deref(), Some("abc123"));

    let event = decoded(ws.next_event()).await;
    let RemoteControlServerMessage::ChannelUnauthorizedEvent(unauthorized) = event else { panic!("expected ChannelUnauthorizedEvent, got {event:?}") };
    assert_eq!(unauthorized.event, SamsungRemoteEventName::Known(SamsungRemoteKnownEvent::MsChannelUnauthorized));

    let event = decoded(ws.next_event()).await;
    let RemoteControlServerMessage::ChannelErrorEvent(error) = event else { panic!("expected ChannelErrorEvent, got {event:?}") };
    assert_eq!(raw_json(error.data.as_deref().unwrap())["message"], "unrecognized method value : ms.remote.contro");

    let event = decoded(ws.next_event()).await;
    let RemoteControlServerMessage::ChannelEnvelope(envelope) = event else { panic!("expected ChannelEnvelope, got {event:?}") };
    assert_eq!(envelope.event, Some(SamsungRemoteEventName::Known(SamsungRemoteKnownEvent::EdInstalledAppGet)));
    assert_eq!(raw_json(envelope.data.as_deref().unwrap())["data"][0]["name"], "YouTube");

    server.shutdown().await;
}

/// R10: every `ArtModeServerMessage` variant. The `d2d_service_message` reply
/// carries its result as a JSON string in `data`, as the Art Mode client in
/// `homelab/docs/samsung/samsung-s95-tcp-api.md` reads it.
#[tokio::test]
async fn samsung_art_mode_decodes_every_server_message_variant() {
    let frames = [
        r#"{"event":"d2d_service_message","data":"{\"id\":\"5f0c\",\"event\":\"artmode_status\",\"value\":\"on\"}"}"#,
        r#"{"event":"ms.channel.connect","data":{"id":"abc123","clients":[{"id":"abc123","deviceName":"TXlSZW1vdGVBcHA=","isHost":false}]}}"#,
        // Any other channel event reaches the fallback.
        r#"{"event":"ms.channel.ready","data":{}}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let client = SamsungSmartTvRemoteWs::with_base_url(server.ws_origin());
    let params = ArtModeConnectionParams {
        name: "TXlSZW1vdGVBcHA=".to_string(),
        token: None,
    };
    let mut ws = client.connect_art_mode(params, WsClientOptions::default()).await.unwrap();

    let event = decoded(ws.next_event()).await;
    let ArtModeServerMessage::ArtModeEvent(art) = event else { panic!("expected ArtModeEvent, got {event:?}") };
    let inner: serde_json::Value = serde_json::from_str(art.data.as_deref().unwrap()).unwrap();
    assert_eq!(inner["event"], "artmode_status");
    assert_eq!(inner["value"], "on");

    let event = decoded(ws.next_event()).await;
    let ArtModeServerMessage::ChannelConnectEvent(connect) = event else { panic!("expected ChannelConnectEvent, got {event:?}") };
    let clients = connect.data.and_then(|data| data.clients).expect("connected clients");
    assert_eq!(clients[0].is_host, Some(false));

    let event = decoded(ws.next_event()).await;
    let ArtModeServerMessage::ChannelEnvelope(envelope) = event else { panic!("expected ChannelEnvelope, got {event:?}") };
    assert_eq!(envelope.event, Some(SamsungRemoteEventName::Other("ms.channel.ready".to_string())));
    assert_eq!(raw_json(envelope.data.as_deref().unwrap()), json!({}));

    server.shutdown().await;
}

/// R10: `TextToSpeechServerMessage` has one message and no selectors, so it
/// takes every frame — an audio chunk and the bare final marker alike.
/// Frames from `schematic/docs/research/elevenlabs-api.md`.
#[tokio::test]
async fn elevenlabs_text_to_speech_decodes_every_server_message_variant() {
    let frames = [
        r#"{"audio":"base64_encoded_audio_chunk","alignment":{"charStartTimesMs":[0,3,7,12],"charDurationsMs":[3,4,5,3],"chars":["H","e","l","l"]},"normalizedAlignment":{"charStartTimesMs":[0,3,7,12],"charDurationsMs":[3,4,5,3],"chars":["H","e","l","l"]}}"#,
        r#"{"isFinal":true}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let mut client = ElevenLabsTTSWs::with_base_url(server.ws_origin());
    client = client.api_key("test-key");
    let mut ws = client
        .connect_text_to_speech(
            "voice",
            TextToSpeechConnectionParams::default(),
            TtsInitMessage::default(),
            WsClientOptions::default(),
        )
        .await
        .unwrap();

    let event = decoded(ws.next_event()).await;
    let TextToSpeechServerMessage::AudioChunk(chunk) = event;
    assert_eq!(chunk.audio.as_deref(), Some("base64_encoded_audio_chunk"));
    assert_eq!(chunk.alignment.expect("alignment").char_start_times_ms, [0, 3, 7, 12]);

    let event = decoded(ws.next_event()).await;
    let TextToSpeechServerMessage::AudioChunk(last) = event;
    assert_eq!(last.is_final, Some(true));
    assert!(last.audio.is_none());

    server.shutdown().await;
}

/// R10: as above for the multi-context endpoint, whose frames carry a
/// `contextId`. Frames from `schematic/docs/research/elevenlabs-api.md`.
#[tokio::test]
async fn elevenlabs_multi_context_decodes_every_server_message_variant() {
    let frames = [
        r#"{"audio":"base64_encoded_audio","contextId":"context_1","alignment":{"charStartTimesMs":[0,3,7,12],"charDurationsMs":[3,4,5,3],"chars":["H","e","l","l"]},"normalizedAlignment":{"charStartTimesMs":[0,3,7,12],"charDurationsMs":[3,4,5,3],"chars":["H","e","l","l"]}}"#,
        r#"{"isFinal":true,"contextId":"context_1"}"#,
    ];
    let server = ws_mock::MockWsServer::start(vec![sends_text(&frames)]).await;
    let mut client = ElevenLabsTTSWs::with_base_url(server.ws_origin());
    client = client.api_key("test-key");
    let mut ws = client
        .connect_multi_context_text_to_speech(
            "voice",
            MultiContextTextToSpeechConnectionParams::default(),
            MultiContextInitMessage {
                text: " ".to_string(),
                context_id: "context_1".to_string(),
                voice_settings: None,
                generation_config: None,
            },
            WsClientOptions::default(),
        )
        .await
        .unwrap();

    let event = decoded(ws.next_event()).await;
    let MultiContextTextToSpeechServerMessage::AudioChunk(chunk) = event;
    assert_eq!(chunk.context_id, "context_1");
    assert_eq!(chunk.alignment.expect("alignment").chars, ["H", "e", "l", "l"]);

    let event = decoded(ws.next_event()).await;
    let MultiContextTextToSpeechServerMessage::AudioChunk(last) = event;
    assert_eq!(last.context_id, "context_1");
    assert_eq!(last.is_final, Some(true));

    server.shutdown().await;
}
