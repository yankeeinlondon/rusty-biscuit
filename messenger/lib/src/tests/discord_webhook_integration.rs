use bytes::Bytes;
use secrecy::SecretString;
use wiremock::matchers::{
    body_json, body_string_contains, header, header_regex, method, path, query_param,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::attachment::{Attachment, AttachmentKind, AttachmentSource};
use crate::dispatch::Dispatch;
use crate::message::Message;
use crate::provider::discord_webhook::{DiscordWebhookConfig, DiscordWebhookProvider};
use crate::provider::{Messenger, Provider};
use crate::receipt::{MessageRef, ProviderKind};
use crate::target::Target;

/// A realistic numeric Discord snowflake so the URL passes the provider's
/// numeric-id validation.
const WEBHOOK_ID: &str = "123456789012345678";
const WEBHOOK_TOKEN: &str = "fake-token";

fn webhook_provider(base_uri: &str) -> DiscordWebhookProvider {
    DiscordWebhookProvider::try_new(DiscordWebhookConfig {
        webhook_url: SecretString::from(format!(
            "{base_uri}/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )),
    })
    .expect("test webhook URL must parse")
}

fn webhook_message_response() -> serde_json::Value {
    serde_json::json!({
        "id": "987654321098765432",
        "channel_id": "111222333444555666",
        "webhook_id": WEBHOOK_ID,
        "type": 0,
        "content": "hello",
        "timestamp": "2026-04-19T00:00:00.000000+00:00",
    })
}

fn webhook_message_response_without_webhook_id() -> serde_json::Value {
    serde_json::json!({
        "id": "987654321098765432",
        "channel_id": "111222333444555666",
        "type": 0,
        "content": "hello",
        "timestamp": "2026-04-19T00:00:00.000000+00:00",
    })
}

/// Capability set must match the table in `SKILL.md`: markdown, attachments,
/// and location (text fallback) are supported; replies, silent delivery, and
/// link-preview control are not.
#[tokio::test]
async fn capabilities_match_webhook_contract() {
    let provider = webhook_provider("https://discord.com");
    let caps = provider.capabilities();
    assert!(caps.supports_markdown_rendering);
    assert!(!caps.supports_reply);
    assert!(
        caps.supported_attachment_kinds
            .contains(&crate::AttachmentKind::Image)
    );
    assert!(
        caps.supported_attachment_kinds
            .contains(&crate::AttachmentKind::Document)
    );
    assert!(caps.supports_location);
    assert!(!caps.supports_silent_delivery);
    assert!(!caps.supports_link_preview_control);
}

#[tokio::test]
async fn kind_reports_discord_webhook() {
    let provider = webhook_provider("https://discord.com");
    assert_eq!(provider.kind(), ProviderKind::DiscordWebhook);
}

/// Plan-time hard-error: a webhook dispatch that carries `reply_to` must fail
/// *before* any HTTP call. A `MockServer` that expects zero requests verifies
/// the no-network contract — its `Drop` fails the test if any request lands.
#[tokio::test]
async fn plan_send_with_reply_to_errors_before_network_call() {
    let server = MockServer::start().await;
    // Any unexpected POST to the mock server will fail the `.expect(0)` check
    // when the mock guard drops at end of scope.
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let mut messenger = Messenger::new();
    messenger.register(Box::new(webhook_provider(&server.uri())));

    let dispatch = Dispatch::to(Target::discord_webhook()).reply_to(MessageRef::DiscordWebhook {
        webhook_id: WEBHOOK_ID.into(),
        channel_id: "111222333444555666".into(),
        message_id: "900".into(),
        thread_id: None,
    });
    let message = Message::text("hello");

    let err = messenger.plan_send(dispatch, &message).unwrap_err();
    assert!(
        matches!(
            err,
            crate::MessengerError::UnsupportedFeature {
                provider: ProviderKind::DiscordWebhook,
                feature: "replies"
            }
        ),
        "expected UnsupportedFeature(replies), got {err:?}"
    );
}

/// Even in `BestEffort` mode the webhook reply_to must hard-error — the guard
/// in `normalize_dispatch` is mode-independent. This protects against a future
/// refactor that forgets the unconditional branch.
#[tokio::test]
async fn plan_send_with_reply_hard_errors_in_best_effort_mode() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let mut messenger = Messenger::new();
    messenger.register(Box::new(webhook_provider(&server.uri())));

    // `Dispatch::to()` defaults to `CompatibilityMode::BestEffort`.
    let dispatch = Dispatch::to(Target::discord_webhook()).reply_to(MessageRef::DiscordWebhook {
        webhook_id: WEBHOOK_ID.into(),
        channel_id: "111222333444555666".into(),
        message_id: "900".into(),
        thread_id: None,
    });
    assert_eq!(
        dispatch.options.compatibility,
        crate::CompatibilityMode::BestEffort
    );

    let message = Message::text("hello");
    let err = messenger.plan_send(dispatch, &message).unwrap_err();
    assert!(matches!(
        err,
        crate::MessengerError::UnsupportedFeature {
            provider: ProviderKind::DiscordWebhook,
            feature: "replies"
        }
    ));
}

#[tokio::test]
async fn sends_text_message_with_expected_payload() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .and(query_param("wait", "true"))
        .and(header("Content-Type", "application/json"))
        .and(body_json(serde_json::json!({
            "content": "hello",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_message_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::DiscordWebhook);
    assert_eq!(receipt.raw_id, "987654321098765432");
    match receipt.message_ref {
        MessageRef::DiscordWebhook {
            webhook_id,
            channel_id,
            message_id,
            thread_id,
        } => {
            assert_eq!(webhook_id, WEBHOOK_ID);
            assert_eq!(channel_id, "111222333444555666");
            assert_eq!(message_id, "987654321098765432");
            assert!(thread_id.is_none());
        }
        other => panic!("expected DiscordWebhook message ref, got {other:?}"),
    }
}

#[tokio::test]
async fn sends_markdown_message_with_expected_payload() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .and(query_param("wait", "true"))
        .and(body_json(serde_json::json!({
            "content": "**bold**",
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_message_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::markdown("**bold**");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::DiscordWebhook);
    assert_eq!(receipt.raw_id, "987654321098765432");
}

#[tokio::test]
async fn sends_with_thread_id_query_parameter() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .and(query_param("wait", "true"))
        .and(query_param("thread_id", "555666777888999000"))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_message_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook_thread("555666777888999000"));
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    match receipt.message_ref {
        MessageRef::DiscordWebhook { thread_id, .. } => {
            assert_eq!(thread_id.as_deref(), Some("555666777888999000"));
        }
        other => panic!("expected DiscordWebhook message ref, got {other:?}"),
    }
}

#[tokio::test]
async fn invalid_thread_id_errors_before_network_call() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200))
        .expect(0)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook_thread("555666&wait=false"));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(matches!(
        err,
        crate::MessengerError::InvalidMessage(message)
        if message.contains("invalid Discord webhook thread ID")
    ));
}

#[tokio::test]
async fn sends_attachments_as_multipart_with_payload_json_and_file_part() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .and(query_param("wait", "true"))
        .and(header_regex("Content-Type", r"multipart/form-data;\s*boundary=.*"))
        .and(body_string_contains("name=\"payload_json\""))
        .and(body_string_contains("\"content\":\"hello\""))
        .and(body_string_contains(
            "\"attachments\":[{\"id\":0,\"filename\":\"report.txt\",\"description\":\"diagram alt\"}]",
        ))
        .and(body_string_contains("name=\"files[0]\""))
        .and(body_string_contains("filename=\"report.txt\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_message_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::text("hello").attachment(Attachment {
        kind: AttachmentKind::Document,
        source: AttachmentSource::Bytes {
            filename: "report.txt".into(),
            mime_type: "text/plain".into(),
            data: Bytes::from_static(b"attachment-bytes"),
        },
        caption: Some("diagram caption".into()),
        alt_text: Some("diagram alt".into()),
    });

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::DiscordWebhook);

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    let request = &requests[0];

    let content_type = request
        .headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap();
    assert!(
        content_type.starts_with("multipart/form-data; boundary="),
        "expected multipart content type, got: {content_type}"
    );
    assert!(
        request
            .body
            .windows(b"attachment-bytes".len())
            .any(|window| window == b"attachment-bytes"),
        "multipart body should contain the raw attachment bytes"
    );
}

#[tokio::test]
async fn rate_limit_maps_to_rate_limited_with_retry_after() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("Retry-After", "15")
                .set_body_json(serde_json::json!({
                    "message": "You are being rate limited.",
                    "retry_after": 15.0,
                    "global": false,
                })),
        )
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    match err {
        crate::MessengerError::RateLimited {
            provider,
            retry_after_ms,
        } => {
            assert_eq!(provider, ProviderKind::DiscordWebhook);
            assert_eq!(retry_after_ms, Some(15_000));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn generic_client_error_maps_to_provider_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "message": "Invalid Form Body",
            "code": 50006,
        })))
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(
            err,
            crate::MessengerError::Provider {
                provider: ProviderKind::DiscordWebhook,
                code: Some(ref code),
                ..
            } if code == "400"
        ),
        "expected Provider error with HTTP 400 code, got {err:?}"
    );
}

#[tokio::test]
async fn auth_error_maps_to_authentication() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "message": "401: Unauthorized",
            "code": 0,
        })))
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(
            err,
            crate::MessengerError::Authentication {
                provider: ProviderKind::DiscordWebhook,
                ..
            }
        ),
        "expected Authentication, got {err:?}"
    );
}

#[tokio::test]
async fn server_error_maps_to_transport() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(
            err,
            crate::MessengerError::Transport {
                provider: ProviderKind::DiscordWebhook,
                ..
            }
        ),
        "expected Transport, got {err:?}"
    );
}

#[tokio::test]
async fn falls_back_to_provider_webhook_id_when_response_omits_it() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(webhook_message_response_without_webhook_id()),
        )
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    match receipt.message_ref {
        MessageRef::DiscordWebhook { webhook_id, .. } => {
            assert_eq!(webhook_id, WEBHOOK_ID);
        }
        other => panic!("expected DiscordWebhook message ref, got {other:?}"),
    }
}

#[tokio::test]
async fn messenger_routes_to_discord_webhook_provider() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_message_response()))
        .mount(&server)
        .await;

    let mut messenger = Messenger::new();
    messenger.register(Box::new(webhook_provider(&server.uri())));

    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::text("hello");

    let receipt = messenger.send(dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::DiscordWebhook);
}

#[tokio::test]
async fn webhook_summarized_body_splits_into_content_and_embed() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .and(query_param("wait", "true"))
        .and(body_json(serde_json::json!({
            "content": "plain banner",
            "embeds": [{ "description": "**rich** body" }]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_message_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::summarized("plain banner", "**rich** body");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::DiscordWebhook);
    assert_eq!(receipt.raw_id, "987654321098765432");
}

#[tokio::test]
async fn webhook_markdown_body_does_not_include_embeds() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .and(query_param("wait", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_message_response()))
        .expect(1)
        .mount(&server)
        .await;

    let provider = webhook_provider(&server.uri());
    let dispatch = Dispatch::to(Target::discord_webhook());
    let message = Message::markdown("**bold**");

    provider.send(&dispatch, &message).await.unwrap();

    let received = server.received_requests().await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&received[0].body).unwrap();
    assert_eq!(
        body.get("content").and_then(|v| v.as_str()),
        Some("**bold**")
    );
    assert!(
        body.get("embeds").is_none(),
        "legacy Markdown body must not produce an embeds field"
    );
}
