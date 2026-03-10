use wiremock::matchers::{body_json, header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::dispatch::Dispatch;
use crate::message::Message;
use crate::provider::Provider;
use crate::provider::signal::{SignalConfig, SignalProvider};
use crate::receipt::{MessageRef, ProviderKind, SignalAuthor, SignalThreadKey};
use crate::target::{SignalAddress, Target};

fn signal_provider(rpc_url: &str) -> SignalProvider {
    SignalProvider::new(SignalConfig {
        rpc_url: rpc_url.to_string(),
        account: "+15551234567".to_string(),
    })
}

fn jsonrpc_ok_response(timestamp: i64) -> serde_json::Value {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "timestamp": timestamp
        }
    })
}

#[tokio::test]
async fn sends_direct_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "send",
            "id": 1,
            "params": {
                "account": "+15551234567",
                "recipient": ["+15559876543"],
                "message": "hello"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(jsonrpc_ok_response(1712345678000)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = signal_provider(&server.uri());
    let dispatch = Dispatch::to(Target::signal_user(SignalAddress::Phone(
        "+15559876543".into(),
    )));
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Signal);
    assert_eq!(receipt.raw_id, "1712345678000");
}

#[tokio::test]
async fn sends_group_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "sendGroupMessage",
            "id": 1,
            "params": {
                "account": "+15551234567",
                "groupId": "base64groupid",
                "message": "group message"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(jsonrpc_ok_response(1712345679000)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = signal_provider(&server.uri());
    let dispatch = Dispatch::to(Target::signal_group("base64groupid"));
    let message = Message::text("group message");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Signal);
}

#[tokio::test]
async fn sends_reply_with_quote() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "send",
            "id": 1,
            "params": {
                "account": "+15551234567",
                "recipient": ["+15559876543"],
                "message": "reply",
                "quoteAuthor": "+15559876543",
                "quoteTimestamp": 1712345670000_i64
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(jsonrpc_ok_response(1712345680000)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = signal_provider(&server.uri());
    let dispatch = Dispatch::to(Target::signal_user(SignalAddress::Phone(
        "+15559876543".into(),
    )))
    .reply_to(MessageRef::Signal {
        thread: SignalThreadKey::Direct("+15559876543".into()),
        author: SignalAuthor::Phone("+15559876543".into()),
        timestamp_ms: 1712345670000,
    });
    let message = Message::text("reply");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Signal);
}

#[tokio::test]
async fn jsonrpc_error_maps_to_provider_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "error": {
                "code": -32000,
                "message": "User not registered"
            }
        })))
        .mount(&server)
        .await;

    let provider = signal_provider(&server.uri());
    let dispatch = Dispatch::to(Target::signal_user(SignalAddress::Phone(
        "+15559876543".into(),
    )));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::Provider { .. }),
        "expected Provider, got {err:?}"
    );
}

#[tokio::test]
async fn server_error_maps_to_transport() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let provider = signal_provider(&server.uri());
    let dispatch = Dispatch::to(Target::signal_user(SignalAddress::Phone(
        "+15559876543".into(),
    )));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(matches!(err, crate::MessengerError::Transport { .. }));
}

#[tokio::test]
async fn markdown_rendered_as_plain_text() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(body_json(serde_json::json!({
            "jsonrpc": "2.0",
            "method": "send",
            "id": 1,
            "params": {
                "account": "+15551234567",
                "recipient": ["+15559876543"],
                "message": "bold text"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(jsonrpc_ok_response(1712345681000)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = signal_provider(&server.uri());
    let dispatch = Dispatch::to(Target::signal_user(SignalAddress::Phone(
        "+15559876543".into(),
    )));
    let message = Message::markdown("**bold** text");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Signal);
}
