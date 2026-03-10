use secrecy::SecretString;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::dispatch::Dispatch;
use crate::message::Message;
use crate::provider::Provider;
use crate::provider::telegram::{TelegramConfig, TelegramProvider};
use crate::receipt::{MessageRef, ProviderKind, TelegramChatRef};
use crate::target::{Target, TelegramChatId};

fn telegram_provider(base_url: &str) -> TelegramProvider {
    TelegramProvider::new(TelegramConfig {
        bot_token: SecretString::from("test-bot-token"),
        api_base_url: Some(base_url.to_string()),
    })
}

fn telegram_ok_response(message_id: i64) -> serde_json::Value {
    serde_json::json!({
        "ok": true,
        "result": {
            "message_id": message_id,
            "chat": { "id": 12345 },
            "date": 1712345678,
            "text": "hello"
        }
    })
}

#[tokio::test]
async fn sends_text_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/sendMessage"))
        .and(body_json(serde_json::json!({
            "chat_id": "12345",
            "text": "hello"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(telegram_ok_response(42)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = telegram_provider(&server.uri());
    let dispatch = Dispatch::to(Target::telegram_chat(TelegramChatId::Id(12345)));
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Telegram);
    assert_eq!(receipt.raw_id, "42");
}

#[tokio::test]
async fn sends_markdown_as_html() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/sendMessage"))
        .and(body_json(serde_json::json!({
            "chat_id": "12345",
            "text": "<b>bold</b> text",
            "parse_mode": "HTML"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(telegram_ok_response(43)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = telegram_provider(&server.uri());
    let dispatch = Dispatch::to(Target::telegram_chat(TelegramChatId::Id(12345)));
    let message = Message::markdown("**bold** text");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Telegram);
}

#[tokio::test]
async fn sends_location() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/sendLocation"))
        .and(body_json(serde_json::json!({
            "chat_id": "12345",
            "latitude": 34.05,
            "longitude": -118.24
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(telegram_ok_response(44)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = telegram_provider(&server.uri());
    let dispatch = Dispatch::to(Target::telegram_chat(TelegramChatId::Id(12345)));
    let message = Message::location(34.05, -118.24);

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.raw_id, "44");
}

#[tokio::test]
async fn includes_reply_parameters() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/sendMessage"))
        .and(body_json(serde_json::json!({
            "chat_id": "12345",
            "text": "reply",
            "reply_parameters": {
                "message_id": 10
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(telegram_ok_response(45)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = telegram_provider(&server.uri());
    let dispatch = Dispatch::to(Target::telegram_chat(TelegramChatId::Id(12345))).reply_to(
        MessageRef::Telegram {
            chat_id: TelegramChatRef::Id(12345),
            message_id: 10,
            thread_id: None,
        },
    );
    let message = Message::text("reply");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Telegram);
}

#[tokio::test]
async fn auth_error_maps_correctly() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/sendMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": false,
            "description": "Unauthorized"
        })))
        .mount(&server)
        .await;

    let provider = telegram_provider(&server.uri());
    let dispatch = Dispatch::to(Target::telegram_chat(TelegramChatId::Id(12345)));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(matches!(err, crate::MessengerError::Authentication { .. }));
}

#[tokio::test]
async fn rate_limit_maps_correctly() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/sendMessage"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "ok": false,
            "description": "Too Many Requests",
            "parameters": { "retry_after": 30 }
        })))
        .mount(&server)
        .await;

    let provider = telegram_provider(&server.uri());
    let dispatch = Dispatch::to(Target::telegram_chat(TelegramChatId::Id(12345)));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    match err {
        crate::MessengerError::RateLimited { retry_after_ms, .. } => {
            assert_eq!(retry_after_ms, Some(30_000));
        }
        other => panic!("expected RateLimited, got {other:?}"),
    }
}

#[tokio::test]
async fn silent_delivery() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/sendMessage"))
        .and(body_json(serde_json::json!({
            "chat_id": "12345",
            "text": "quiet",
            "disable_notification": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(telegram_ok_response(46)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = telegram_provider(&server.uri());
    let dispatch = Dispatch::to(Target::telegram_chat(TelegramChatId::Id(12345))).silent();
    let message = Message::text("quiet");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Telegram);
}

#[tokio::test]
async fn disables_link_preview_in_payload() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/sendMessage"))
        .and(body_json(serde_json::json!({
            "chat_id": "12345",
            "text": "https://example.com",
            "disable_web_page_preview": true
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(telegram_ok_response(47)))
        .expect(1)
        .mount(&server)
        .await;

    let provider = telegram_provider(&server.uri());
    let dispatch =
        Dispatch::to(Target::telegram_chat(TelegramChatId::Id(12345))).disable_link_preview();
    let message = Message::text("https://example.com");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.raw_id, "47");
}
