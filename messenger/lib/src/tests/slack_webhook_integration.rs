use secrecy::SecretString;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::dispatch::Dispatch;
use crate::message::Message;
use crate::provider::slack_webhook::SlackWebhookProvider;
use crate::provider::{Messenger, Provider};
use crate::receipt::{MessageRef, ProviderKind};
use crate::target::Target;

/// Build a provider wired to the wiremock server. The production `try_new`
/// validator rejects any host other than `hooks.slack.com`, so tests use the
/// crate-private `new_unchecked` seam to target a local mock URL.
fn webhook_provider(base_uri: &str) -> SlackWebhookProvider {
    SlackWebhookProvider::new_unchecked(SecretString::from(base_uri.to_string()))
}

fn webhook_ok_response() -> serde_json::Value {
    serde_json::json!({ "ok": true })
}

// ---------------------------------------------------------------------------
// 3.2 Success cases
// ---------------------------------------------------------------------------

#[tokio::test]
async fn sends_text_message_with_expected_payload() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(serde_json::json!({ "text": "hello" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::SlackWebhook);
    assert_eq!(receipt.raw_id, "");
    assert!(matches!(
        receipt.message_ref,
        MessageRef::SlackWebhook { thread_ts: None }
    ));
    assert_eq!(
        receipt
            .metadata
            .get("delivery_confirmed")
            .map(String::as_str),
        Some("true")
    );
}

#[tokio::test]
async fn sends_markdown_as_mrkdwn() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(serde_json::json!({ "text": "*bold* text" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::markdown("**bold** text");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::SlackWebhook);
}

#[tokio::test]
async fn includes_thread_ts_for_reply() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(serde_json::json!({
            "text": "reply",
            "thread_ts": "1712345678.000100"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook()).reply_to(MessageRef::SlackWebhook {
        thread_ts: Some("1712345678.000100".into()),
    });
    let message = Message::text("reply");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    // The webhook response carries no `ts`, so a fresh receipt always shows
    // `thread_ts: None` regardless of the inbound reply target.
    assert!(matches!(
        receipt.message_ref,
        MessageRef::SlackWebhook { thread_ts: None }
    ));
}

#[tokio::test]
async fn omits_thread_ts_when_reply_ref_has_no_timestamp() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(serde_json::json!({ "text": "hello" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook())
        .reply_to(MessageRef::SlackWebhook { thread_ts: None });
    let message = Message::text("hello");

    provider.send(&dispatch, &message).await.unwrap();
}

#[tokio::test]
async fn disables_link_preview_when_requested() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(serde_json::json!({
            "text": "https://example.com",
            "unfurl_links": false,
            "unfurl_media": false
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook()).disable_link_preview();
    let message = Message::text("https://example.com");

    provider.send(&dispatch, &message).await.unwrap();
}

#[tokio::test]
async fn default_dispatch_omits_unfurl_fields() {
    let server = MockServer::start().await;

    // Use `body_json` to assert the exact payload shape — if `unfurl_links` or
    // `unfurl_media` were serialized, the json comparison would fail.
    Mock::given(method("POST"))
        .and(path("/"))
        .and(body_json(serde_json::json!({
            "text": "https://example.com"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::text("https://example.com");

    provider.send(&dispatch, &message).await.unwrap();
}

// ---------------------------------------------------------------------------
// 3.3 Error mapping
// ---------------------------------------------------------------------------

async fn send_with_webhook_error(error_code: &str) -> crate::MessengerError {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": false,
            "error": error_code,
        })))
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::text("hello");

    provider.send(&dispatch, &message).await.unwrap_err()
}

#[tokio::test]
async fn invalid_token_maps_to_authentication() {
    let err = send_with_webhook_error("invalid_token").await;
    assert!(
        matches!(
            err,
            crate::MessengerError::Authentication {
                provider: ProviderKind::SlackWebhook,
                ..
            }
        ),
        "expected Authentication, got {err:?}"
    );
}

#[tokio::test]
async fn action_prohibited_maps_to_authentication() {
    let err = send_with_webhook_error("action_prohibited").await;
    assert!(
        matches!(
            err,
            crate::MessengerError::Authentication {
                provider: ProviderKind::SlackWebhook,
                ..
            }
        ),
        "expected Authentication, got {err:?}"
    );
}

#[tokio::test]
async fn invalid_payload_maps_to_invalid_message() {
    let err = send_with_webhook_error("invalid_payload").await;
    assert!(
        matches!(err, crate::MessengerError::InvalidMessage(_)),
        "expected InvalidMessage, got {err:?}"
    );
}

#[tokio::test]
async fn channel_is_archived_maps_to_invalid_message() {
    let err = send_with_webhook_error("channel_is_archived").await;
    assert!(
        matches!(err, crate::MessengerError::InvalidMessage(_)),
        "expected InvalidMessage, got {err:?}"
    );
}

#[tokio::test]
async fn unknown_error_maps_to_provider() {
    let err = send_with_webhook_error("mystery_error").await;
    assert!(
        matches!(
            err,
            crate::MessengerError::Provider {
                provider: ProviderKind::SlackWebhook,
                ..
            }
        ),
        "expected Provider, got {err:?}"
    );
}

#[tokio::test]
async fn rate_limit_maps_to_rate_limited_with_retry_after() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "30")
                .set_body_json(serde_json::json!({ "ok": false, "error": "rate_limited" })),
        )
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    match err {
        crate::MessengerError::RateLimited {
            provider,
            retry_after_ms,
        } => {
            assert_eq!(provider, ProviderKind::SlackWebhook);
            assert_eq!(retry_after_ms, Some(30_000));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn http_500_maps_to_transport() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(
            err,
            crate::MessengerError::Transport {
                provider: ProviderKind::SlackWebhook,
                ..
            }
        ),
        "expected Transport, got {err:?}"
    );
}

#[tokio::test]
async fn http_502_maps_to_transport() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(matches!(
        err,
        crate::MessengerError::Transport {
            provider: ProviderKind::SlackWebhook,
            ..
        }
    ));
}

#[tokio::test]
async fn http_503_maps_to_transport() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(matches!(
        err,
        crate::MessengerError::Transport {
            provider: ProviderKind::SlackWebhook,
            ..
        }
    ));
}

// ---------------------------------------------------------------------------
// 3.4 Plan-time mismatch: provider-mismatch check fires before transport.
// ---------------------------------------------------------------------------

#[tokio::test]
async fn plan_send_with_slack_bot_reply_errors_before_network_call() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let mut messenger = Messenger::new();
    messenger.register(Box::new(webhook_provider(&server.uri())));

    let dispatch = Dispatch::to(Target::slack_webhook()).reply_to(MessageRef::Slack {
        channel_id: "C012345".into(),
        thread_ts: "1712345678.000100".into(),
    });
    let message = Message::text("hello");

    let err = messenger.plan_send(dispatch, &message).unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::InvalidMessage(_)),
        "expected InvalidMessage for provider mismatch, got {err:?}"
    );
}

#[tokio::test]
async fn messenger_routes_to_slack_webhook_provider() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_ok_response()))
        .expect(1)
        .mount(&server)
        .await;

    let mut messenger = Messenger::new();
    messenger.register(Box::new(webhook_provider(&server.uri())));

    let dispatch = Dispatch::to(Target::slack_webhook());
    let message = Message::text("hello");

    let receipt = messenger.send(dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::SlackWebhook);
    assert_eq!(receipt.raw_id, "");
    assert_eq!(
        receipt
            .metadata
            .get("delivery_confirmed")
            .map(String::as_str),
        Some("true")
    );
}
