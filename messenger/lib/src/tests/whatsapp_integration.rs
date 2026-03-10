use secrecy::SecretString;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::dispatch::Dispatch;
use crate::message::Message;
use crate::provider::Provider;
use crate::provider::whatsapp::{WhatsAppConfig, WhatsAppProvider};
use crate::receipt::{MessageRef, ProviderKind};
use crate::target::Target;

fn whatsapp_provider(base_url: &str) -> WhatsAppProvider {
    WhatsAppProvider::new(WhatsAppConfig {
        access_token: SecretString::from("test-access-token"),
        phone_number_id: "123456".to_string(),
        api_version: None,
        api_base_url: Some(base_url.to_string()),
    })
}

fn whatsapp_ok_response(message_id: &str) -> serde_json::Value {
    serde_json::json!({
        "messaging_product": "whatsapp",
        "contacts": [{ "wa_id": "15551234567" }],
        "messages": [{ "id": message_id }]
    })
}

#[tokio::test]
async fn sends_text_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("Authorization", "Bearer test-access-token"))
        .and(body_json(serde_json::json!({
            "messaging_product": "whatsapp",
            "to": "+15551234567",
            "type": "text",
            "text": {
                "body": "hello"
            }
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(whatsapp_ok_response("wamid.abc123")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = whatsapp_provider(&server.uri());
    let dispatch = Dispatch::to(Target::whatsapp_recipient("+15551234567"));
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::WhatsApp);
    assert_eq!(receipt.raw_id, "wamid.abc123");
}

#[tokio::test]
async fn sends_location() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(body_json(serde_json::json!({
            "messaging_product": "whatsapp",
            "to": "+15551234567",
            "type": "location",
            "location": {
                "latitude": 34.05,
                "longitude": -118.24
            }
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(whatsapp_ok_response("wamid.loc456")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = whatsapp_provider(&server.uri());
    let dispatch = Dispatch::to(Target::whatsapp_recipient("+15551234567"));
    let message = Message::location(34.05, -118.24);

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.raw_id, "wamid.loc456");
}

#[tokio::test]
async fn sends_reply() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(body_json(serde_json::json!({
            "messaging_product": "whatsapp",
            "to": "+15551234567",
            "type": "text",
            "text": {
                "body": "reply"
            },
            "context": {
                "message_id": "wamid.original123"
            }
        })))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(whatsapp_ok_response("wamid.reply789")),
        )
        .expect(1)
        .mount(&server)
        .await;

    let provider = whatsapp_provider(&server.uri());
    let dispatch =
        Dispatch::to(Target::whatsapp_recipient("+15551234567")).reply_to(MessageRef::WhatsApp {
            message_id: "wamid.original123".into(),
        });
    let message = Message::text("reply");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::WhatsApp);
}

#[tokio::test]
async fn auth_error_maps_correctly() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "error": {
                "code": 190,
                "message": "Invalid OAuth access token"
            }
        })))
        .mount(&server)
        .await;

    let provider = whatsapp_provider(&server.uri());
    let dispatch = Dispatch::to(Target::whatsapp_recipient("+15551234567"));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(matches!(err, crate::MessengerError::Authentication { .. }));
}

#[tokio::test]
async fn rate_limit_returns_error() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(429))
        .mount(&server)
        .await;

    let provider = whatsapp_provider(&server.uri());
    let dispatch = Dispatch::to(Target::whatsapp_recipient("+15551234567"));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(matches!(err, crate::MessengerError::RateLimited { .. }));
}

#[tokio::test]
async fn server_error_maps_to_transport() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let provider = whatsapp_provider(&server.uri());
    let dispatch = Dispatch::to(Target::whatsapp_recipient("+15551234567"));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(matches!(err, crate::MessengerError::Transport { .. }));
}

#[tokio::test]
async fn markdown_rendered_as_plain_text() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(body_json(serde_json::json!({
            "messaging_product": "whatsapp",
            "to": "+15551234567",
            "type": "text",
            "text": {
                "body": "bold text"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(whatsapp_ok_response("wamid.md123")))
        .expect(1)
        .mount(&server)
        .await;

    let provider = whatsapp_provider(&server.uri());
    let dispatch = Dispatch::to(Target::whatsapp_recipient("+15551234567"));
    let message = Message::markdown("**bold** text");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::WhatsApp);
}
