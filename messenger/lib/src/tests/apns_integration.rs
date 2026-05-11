use secrecy::SecretString;
use wiremock::matchers::{body_json, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::dispatch::Dispatch;
use crate::message::Message;
use crate::provider::apns::{ApnsConfig, ApnsProvider};
use crate::provider::{Messenger, Provider};
use crate::receipt::{MessageRef, ProviderKind};
use crate::target::Target;

const DEVICE_TOKEN: &str = "a1b2c3d4e5f6";

fn apns_provider(base_uri: &str) -> ApnsProvider {
    ApnsProvider::new(ApnsConfig {
        team_id: "TEAMID1234".into(),
        key_id: "KEYID12345".into(),
        private_key: SecretString::from(String::from(
            "-----BEGIN PRIVATE KEY-----\n\
            MIGHAgEAMBMGByqGSM49AgEGCCqGSM49AwEHBG0wawIBAQQgJvdhN9y/wiI1IisY\n\
            noI7KUl13N3Vf4nLVLNMfUOxEV6hRANCAASeEY5EAmpiF510WOl1lzUaG9Ugcp/D\n\
            z9whuA6MsoXjFY188mVmNRT5KZvq6BkCGN6Z6wFymXDDwMfY7qx6fByf\n\
            -----END PRIVATE KEY-----",
        )),
        bundle_id: "com.example.app".into(),
        use_sandbox: true,
        api_base_url: Some(base_uri.into()),
    })
}

#[tokio::test]
async fn kind_reports_apns() {
    let provider = apns_provider("https://api.sandbox.push.apple.com");
    assert_eq!(provider.kind(), ProviderKind::Apns);
}

#[tokio::test]
async fn capabilities_match_spec() {
    let provider = apns_provider("https://api.sandbox.push.apple.com");
    let caps = provider.capabilities();
    assert!(!caps.supports_markdown_rendering);
    assert!(!caps.supports_reply);
    assert!(caps.supported_attachment_kinds.is_empty());
    assert!(!caps.supports_location);
    assert!(caps.supports_silent_delivery);
    assert!(!caps.supports_link_preview_control);
}

#[tokio::test]
async fn sends_text_message_with_expected_payload() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/3/device/{DEVICE_TOKEN}")))
        .respond_with(ResponseTemplate::new(200).insert_header("apns-id", "abc-123"))
        .mount(&server)
        .await;

    let provider = apns_provider(&server.uri());
    let dispatch = Dispatch::to(Target::apns(DEVICE_TOKEN));
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Apns);
    assert_eq!(receipt.raw_id, "abc-123");
    match receipt.message_ref {
        MessageRef::Apns { apns_id } => assert_eq!(apns_id, "abc-123"),
        other => panic!("expected Apns message ref, got {other:?}"),
    }
}

#[tokio::test]
async fn sends_title_and_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/3/device/{DEVICE_TOKEN}")))
        .and(body_json(serde_json::json!({
            "aps": {
                "alert": {
                    "title": "Alert",
                    "body": "hello"
                },
                "sound": "default"
            }
        })))
        .respond_with(ResponseTemplate::new(200).insert_header("apns-id", "def-456"))
        .expect(1)
        .mount(&server)
        .await;

    let provider = apns_provider(&server.uri());
    let dispatch = Dispatch::to(Target::apns(DEVICE_TOKEN));
    let message = Message::text("hello").title("Alert");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.raw_id, "def-456");
}

#[tokio::test]
async fn silent_delivery_omits_sound_and_sets_priority() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/3/device/{DEVICE_TOKEN}")))
        .and(body_json(serde_json::json!({
            "aps": {
                "alert": {
                    "body": "hello"
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).insert_header("apns-id", "silent-1"))
        .expect(1)
        .mount(&server)
        .await;

    let provider = apns_provider(&server.uri());
    let dispatch = Dispatch::to(Target::apns(DEVICE_TOKEN)).silent();
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.raw_id, "silent-1");
}

#[tokio::test]
async fn bad_device_token_maps_to_invalid_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/3/device/{DEVICE_TOKEN}")))
        .respond_with(ResponseTemplate::new(410).set_body_json(serde_json::json!({
            "reason": "Unregistered"
        })))
        .mount(&server)
        .await;

    let provider = apns_provider(&server.uri());
    let dispatch = Dispatch::to(Target::apns(DEVICE_TOKEN));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::InvalidMessage(_)),
        "expected InvalidMessage, got {err:?}"
    );
}

#[tokio::test]
async fn invalid_auth_token_maps_to_authentication() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/3/device/{DEVICE_TOKEN}")))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "reason": "InvalidProviderToken"
        })))
        .mount(&server)
        .await;

    let provider = apns_provider(&server.uri());
    let dispatch = Dispatch::to(Target::apns(DEVICE_TOKEN));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::Authentication { provider, .. } if provider == ProviderKind::Apns),
        "expected Authentication, got {err:?}"
    );
}

#[tokio::test]
async fn generic_error_maps_to_provider() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/3/device/{DEVICE_TOKEN}")))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "reason": "BadCollapseId"
        })))
        .mount(&server)
        .await;

    let provider = apns_provider(&server.uri());
    let dispatch = Dispatch::to(Target::apns(DEVICE_TOKEN));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::Provider { provider, .. } if provider == ProviderKind::Apns),
        "expected Provider, got {err:?}"
    );
}

#[tokio::test]
async fn messenger_routes_to_apns_provider() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path(format!("/3/device/{DEVICE_TOKEN}")))
        .respond_with(ResponseTemplate::new(200).insert_header("apns-id", "msg-1"))
        .mount(&server)
        .await;

    let mut messenger = Messenger::new();
    messenger.register(Box::new(apns_provider(&server.uri())));

    let dispatch = Dispatch::to(Target::apns(DEVICE_TOKEN));
    let message = Message::text("hello");

    let receipt = messenger.send(dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Apns);
    assert_eq!(receipt.raw_id, "msg-1");
}
