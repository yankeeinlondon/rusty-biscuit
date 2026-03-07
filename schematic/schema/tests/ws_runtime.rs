//! Runtime integration tests for generated WebSocket clients.

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
