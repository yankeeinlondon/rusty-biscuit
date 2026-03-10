use secrecy::SecretString;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::dispatch::Dispatch;
use crate::message::Message;
use crate::provider::slack::{SlackConfig, SlackProvider};
use crate::provider::{Messenger, Provider};
use crate::receipt::{MessageRef, ProviderKind};
use crate::target::Target;

fn slack_provider(base_url: &str) -> SlackProvider {
    SlackProvider::new(SlackConfig {
        bot_token: SecretString::from("xoxb-test-token"),
        api_base_url: Some(base_url.to_string()),
    })
}

fn slack_ok_response() -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "channel": "C012345",
        "ts": "1712345678.000100",
        "message": {
            "text": "hello",
            "ts": "1712345678.000100"
        }
    })
}

#[tokio::test]
async fn sends_text_message_with_correct_payload() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat.postMessage"))
        .and(header("Authorization", "Bearer xoxb-test-token"))
        .and(header("Content-Type", "application/json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(slack_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = slack_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_channel("C012345"));
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Slack);
    assert_eq!(receipt.raw_id, "1712345678.000100");
}

#[tokio::test]
async fn sends_markdown_as_mrkdwn() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat.postMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(slack_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = slack_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_channel("C012345"));
    let message = Message::markdown("**bold** text");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Slack);
}

#[tokio::test]
async fn includes_thread_ts_for_reply() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat.postMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(slack_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = slack_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_channel("C012345")).reply_to(MessageRef::Slack {
        channel_id: "C012345".into(),
        thread_ts: "1712345678.000100".into(),
    });
    let message = Message::text("reply");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Slack);
}

#[tokio::test]
async fn auth_error_maps_to_authentication() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat.postMessage"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "ok": false,
                "error": "invalid_auth"
            })),
        )
        .mount(&server)
        .await;

    let provider = slack_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_channel("C012345"));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::Authentication { .. }),
        "expected Authentication, got {err:?}"
    );
}

#[tokio::test]
async fn rate_limit_maps_to_rate_limited() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat.postMessage"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "30")
                .set_body_json(serde_json::json!({"ok": false, "error": "ratelimited"})),
        )
        .mount(&server)
        .await;

    let provider = slack_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_channel("C012345"));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    match err {
        crate::MessengerError::RateLimited {
            retry_after_ms, ..
        } => {
            assert_eq!(retry_after_ms, Some(30_000));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn server_error_maps_to_transport() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat.postMessage"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let provider = slack_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_channel("C012345"));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::Transport { .. }),
        "expected Transport, got {err:?}"
    );
}

#[tokio::test]
async fn messenger_routes_to_slack_provider() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/chat.postMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(slack_ok_response()))
        .mount(&server)
        .await;

    let mut messenger = Messenger::new();
    messenger.register(Box::new(slack_provider(&server.uri())));

    let dispatch = Dispatch::to(Target::slack_channel("C012345"));
    let message = Message::text("hello");

    let receipt = messenger.send(dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Slack);
}
