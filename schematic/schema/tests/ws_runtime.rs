//! Runtime integration tests for generated WebSocket clients.

#![allow(clippy::result_large_err)]

mod common;
mod ws_mock;

use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use schematic_schema::elevenlabs_ws::{ElevenLabsTTSWs, TextToSpeechConnectionParams};
use schematic_schema::unfolded_circle_core_ws::UnfoldedCircleCoreWs;
use schematic_schema::ws_shared::{ReconnectPolicy, WsClientOptions, WsConnectionState, WsError};
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
    client.headers = client.headers.clone().use_api_key("test-key", "xi-api-key");

    let params = TextToSpeechConnectionParams {
        model_id: Some("eleven turbo".to_string()),
        output_format: Some("mp3_44100_128".to_string()),
        ..Default::default()
    };

    let ws = client
        .connect_text_to_speech("voice 42", params, WsClientOptions::default())
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
    client.headers = client.headers.clone().use_api_key("test-key", "API-KEY");

    let options = WsClientOptions::builder()
        .request_timeout(Duration::from_millis(100))
        .build();
    let ws = client.connect_core_ws(options).await.unwrap();

    let err = ws
        .request(serde_json::json!({"cmd": "no_response"}))
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
    client.headers = client.headers.clone().use_api_key("test-key", "API-KEY");

    let options = WsClientOptions::builder()
        .request_timeout(Duration::from_secs(2))
        .build();
    let ws = client.connect_core_ws(options).await.unwrap();

    let err = ws
        .request(serde_json::json!({"cmd": "disconnect_me"}))
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
    client.headers = client.headers.clone().use_api_key("test-key", "API-KEY");

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
    let ws = client.connect_core_ws(options).await.unwrap();

    let first_err = ws
        .request(serde_json::json!({"cmd": "first"}))
        .await
        .unwrap_err();
    assert!(matches!(first_err, WsError::Disconnected));

    let started = Instant::now();
    while ws.state() != WsConnectionState::Ready && started.elapsed() < Duration::from_secs(3) {
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(ws.state(), WsConnectionState::Ready);

    let response = ws
        .request(serde_json::json!({"cmd": "second"}))
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

    client.connect_core_ws(WsClientOptions::default()).await.unwrap().close().await.unwrap();
    client
        .connect_core_integrations(WsClientOptions::default())
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
    client
        .connect_core_profiles(WsClientOptions::default())
        .await
        .unwrap()
        .close()
        .await
        .unwrap();
    client
        .connect_core_events(WsClientOptions::default())
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
        .connect_text_to_speech("a/b c?d", TextToSpeechConnectionParams::default(), WsClientOptions::default())
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
        .connect_text_to_speech("v", params, WsClientOptions::default())
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
