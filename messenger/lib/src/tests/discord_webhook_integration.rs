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

/// Answers each webhook post with a distinct, sequential message id so tests
/// can tell posts apart in the receipt.
struct SequentialIds(std::sync::atomic::AtomicUsize);

impl wiremock::Respond for SequentialIds {
    fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
        let n = self.0.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": format!("90000000000000000{n}"),
            "channel_id": "111222333444555666",
            "webhook_id": WEBHOOK_ID,
        }))
    }
}

async fn sequential_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(format!(
            "/api/v10/webhooks/{WEBHOOK_ID}/{WEBHOOK_TOKEN}"
        )))
        .and(query_param("wait", "true"))
        .respond_with(SequentialIds(std::sync::atomic::AtomicUsize::new(0)))
        .mount(&server)
        .await;
    server
}

/// Decoded JSON bodies of every post the server received, in arrival order.
async fn posted_bodies(server: &MockServer) -> Vec<serde_json::Value> {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .map(|request| serde_json::from_slice(&request.body).unwrap())
        .collect()
}

fn content_of(body: &serde_json::Value) -> &str {
    body["content"].as_str().expect("post carries content")
}

#[tokio::test]
async fn content_of_exactly_2000_characters_is_one_post() {
    let server = sequential_server().await;
    let provider = webhook_provider(&server.uri());
    let text = "a".repeat(2000);

    let receipt = provider
        .send(
            &Dispatch::to(Target::discord_webhook()),
            &Message::text(&text),
        )
        .await
        .unwrap();

    let bodies = posted_bodies(&server).await;
    assert_eq!(bodies.len(), 1);
    assert_eq!(content_of(&bodies[0]), text);
    assert_eq!(receipt.raw_id, "900000000000000000");
    assert!(receipt.metadata.is_empty(), "single post adds no metadata");
}

#[tokio::test]
async fn content_of_2001_characters_is_two_posts_with_first_post_receipt() {
    let server = sequential_server().await;
    let provider = webhook_provider(&server.uri());
    let text = format!("{}b", "a".repeat(2000));

    let receipt = provider
        .send(
            &Dispatch::to(Target::discord_webhook()),
            &Message::text(&text),
        )
        .await
        .unwrap();

    let bodies = posted_bodies(&server).await;
    assert_eq!(bodies.len(), 2);
    assert_eq!(content_of(&bodies[0]), "a".repeat(2000));
    assert_eq!(content_of(&bodies[1]), "b");

    assert_eq!(receipt.raw_id, "900000000000000000");
    match &receipt.message_ref {
        MessageRef::DiscordWebhook { message_id, .. } => {
            assert_eq!(message_id, "900000000000000000");
        }
        other => panic!("expected DiscordWebhook message ref, got {other:?}"),
    }
    assert_eq!(
        receipt.metadata.get("post_count").map(String::as_str),
        Some("2")
    );
    assert_eq!(
        receipt.metadata.get("post_message_ids").map(String::as_str),
        Some("900000000000000000,900000000000000001")
    );
}

#[tokio::test]
async fn multibyte_characters_at_the_boundary_are_not_split() {
    let server = sequential_server().await;
    let provider = webhook_provider(&server.uri());
    // 1999 ASCII chars then two multi-byte chars straddling the 2000 limit.
    let text = format!("{}é😀tail", "a".repeat(1999));

    provider
        .send(
            &Dispatch::to(Target::discord_webhook()),
            &Message::text(&text),
        )
        .await
        .unwrap();

    let bodies = posted_bodies(&server).await;
    assert_eq!(bodies.len(), 2);
    let first = content_of(&bodies[0]);
    assert_eq!(first.chars().count(), 2000);
    assert!(first.ends_with('é'));
    assert_eq!(content_of(&bodies[1]), "😀tail");
}

#[tokio::test]
async fn long_markdown_with_code_block_arrives_in_order_with_valid_fences() {
    let server = sequential_server().await;
    let provider = webhook_provider(&server.uri());
    let items: String = (0..40)
        .map(|i| format!("- review item {i}: confirm the change is intended\n"))
        .collect();
    let code: String = (0..120)
        .map(|i| format!("let value_{i} = {i};\n"))
        .collect();
    let markdown = format!("Lifecycle summary.\n\n{items}\n```rust\n{code}```\n\nDone.");

    provider
        .send(
            &Dispatch::to(Target::discord_webhook()),
            &Message::markdown(&markdown),
        )
        .await
        .unwrap();

    let bodies = posted_bodies(&server).await;
    assert!(
        bodies.len() >= 3,
        "expected several posts, got {}",
        bodies.len()
    );
    for body in &bodies {
        let content = content_of(body);
        assert!(content.chars().count() <= 2000);
        let fences = content
            .lines()
            .filter(|l| l.trim_start().starts_with("```"))
            .count();
        assert_eq!(fences % 2, 0, "unbalanced fence in post:\n{content}");
    }

    // Every source line arrives exactly once and in order, ignoring the
    // fence lines added to close and reopen the block between posts.
    let delivered: Vec<&str> = bodies
        .iter()
        .flat_map(|body| content_of(body).lines())
        .filter(|line| !line.starts_with("```") && !line.is_empty())
        .collect();
    let expected_lines: Vec<String> = (0..40)
        .map(|i| format!("review item {i}: confirm the change is intended"))
        .chain((0..120).map(|i| format!("let value_{i} = {i};")))
        .collect();
    let delivered_items: Vec<&str> = delivered
        .iter()
        .map(|line| line.trim_start_matches(['-', '*', '•', ' ']))
        .filter(|line| expected_lines.iter().any(|e| e == line))
        .collect();
    assert_eq!(delivered_items, expected_lines);
    assert_eq!(delivered.first().copied(), Some("Lifecycle summary."));
    assert_eq!(delivered.last().copied(), Some("Done."));
}

#[tokio::test]
async fn embed_rides_on_the_final_post_of_a_long_summary() {
    let server = sequential_server().await;
    let provider = webhook_provider(&server.uri());
    let summary = "s".repeat(2500);

    provider
        .send(
            &Dispatch::to(Target::discord_webhook()),
            &Message::summarized(&summary, "**rich** body"),
        )
        .await
        .unwrap();

    let bodies = posted_bodies(&server).await;
    assert_eq!(bodies.len(), 2);
    assert!(bodies[0].get("embeds").is_none());
    assert_eq!(
        bodies[1]["embeds"],
        serde_json::json!([{ "description": "**rich** body" }])
    );
}

#[tokio::test]
async fn failure_on_a_later_post_returns_the_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(200).set_body_json(webhook_message_response()))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad"))
        .mount(&server)
        .await;
    let provider = webhook_provider(&server.uri());

    let err = provider
        .send(
            &Dispatch::to(Target::discord_webhook()),
            &Message::text("a".repeat(4500)),
        )
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        crate::MessengerError::Provider {
            provider: ProviderKind::DiscordWebhook,
            ..
        }
    ));
    // The third post is never attempted once the second fails.
    assert_eq!(server.received_requests().await.unwrap().len(), 2);
}
