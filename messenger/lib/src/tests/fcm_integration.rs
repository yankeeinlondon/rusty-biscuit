use secrecy::SecretString;
use wiremock::matchers::{body_json, method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::dispatch::Dispatch;
use crate::message::Message;
use crate::provider::fcm::{FcmConfig, FcmProvider};
use crate::provider::{Messenger, Provider};
use crate::receipt::{MessageRef, ProviderKind};
use crate::target::Target;

const DEVICE_TOKEN: &str = "fcm-device-token-123";
const PROJECT_ID: &str = "test-project-12345";

fn fcm_provider(base_uri: &str) -> FcmProvider {
    FcmProvider::new(FcmConfig {
        project_id: PROJECT_ID.into(),
        access_token: SecretString::from(String::from("test-access-token")),
        api_base_url: Some(base_uri.into()),
    })
}

#[tokio::test]
async fn kind_reports_fcm() {
    let provider = fcm_provider("https://fcm.googleapis.com");
    assert_eq!(provider.kind(), ProviderKind::Fcm);
}

#[tokio::test]
async fn capabilities_match_spec() {
    let provider = fcm_provider("https://fcm.googleapis.com");
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
        .and(path_regex(format!(
            r"^/v1/projects/{PROJECT_ID}/messages:send$"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "projects/test-project-12345/messages/0:1234567890"
        })))
        .mount(&server)
        .await;

    let provider = fcm_provider(&server.uri());
    let dispatch = Dispatch::to(Target::fcm(DEVICE_TOKEN));
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Fcm);
    assert_eq!(
        receipt.raw_id,
        "projects/test-project-12345/messages/0:1234567890"
    );
    match receipt.message_ref {
        MessageRef::Fcm {
            message_id,
            project_id,
        } => {
            assert_eq!(
                message_id,
                "projects/test-project-12345/messages/0:1234567890"
            );
            assert_eq!(project_id, PROJECT_ID);
        }
        other => panic!("expected Fcm message ref, got {other:?}"),
    }
}

#[tokio::test]
async fn sends_title_and_body() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(format!(
            r"^/v1/projects/{PROJECT_ID}/messages.*"
        )))
        .and(body_json(serde_json::json!({
            "message": {
                "token": DEVICE_TOKEN,
                "notification": {
                    "title": "Alert",
                    "body": "hello"
                },
                "android": {
                    "priority": "high"
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "projects/test-project-12345/messages/0:1234567891"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = fcm_provider(&server.uri());
    let dispatch = Dispatch::to(Target::fcm(DEVICE_TOKEN));
    let message = Message::text("hello").title("Alert");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(
        receipt.raw_id,
        "projects/test-project-12345/messages/0:1234567891"
    );
}

#[tokio::test]
async fn silent_delivery_sets_normal_priority() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(format!(
            r"^/v1/projects/{PROJECT_ID}/messages.*"
        )))
        .and(body_json(serde_json::json!({
            "message": {
                "token": DEVICE_TOKEN,
                "notification": {
                    "body": "hello"
                },
                "android": {
                    "priority": "normal"
                }
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "projects/test-project-12345/messages/0:1234567892"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let provider = fcm_provider(&server.uri());
    let dispatch = Dispatch::to(Target::fcm(DEVICE_TOKEN)).silent();
    let message = Message::text("hello");

    let receipt = provider.send(&dispatch, &message).await.unwrap();
    assert_eq!(
        receipt.raw_id,
        "projects/test-project-12345/messages/0:1234567892"
    );
}

#[tokio::test]
async fn invalid_registration_maps_to_invalid_message() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(format!(
            r"^/v1/projects/{PROJECT_ID}/messages.*"
        )))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "error": {
                "code": 404,
                "message": "Requested entity was not found. registration-token-not-registered",
                "status": "NOT_FOUND"
            }
        })))
        .mount(&server)
        .await;

    let provider = fcm_provider(&server.uri());
    let dispatch = Dispatch::to(Target::fcm(DEVICE_TOKEN));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::InvalidMessage(_)),
        "expected InvalidMessage, got {err:?}"
    );
}

#[tokio::test]
async fn auth_error_maps_to_authentication() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(format!(
            r"^/v1/projects/{PROJECT_ID}/messages.*"
        )))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": {
                "code": 401,
                "message": "Request had invalid authentication credentials.",
                "status": "UNAUTHENTICATED"
            }
        })))
        .mount(&server)
        .await;

    let provider = fcm_provider(&server.uri());
    let dispatch = Dispatch::to(Target::fcm(DEVICE_TOKEN));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::Authentication { provider, .. } if provider == ProviderKind::Fcm),
        "expected Authentication, got {err:?}"
    );
}

#[tokio::test]
async fn generic_error_maps_to_provider() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(format!(
            r"^/v1/projects/{PROJECT_ID}/messages.*"
        )))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": {
                "code": 400,
                "message": "Invalid JSON payload received.",
                "status": "INVALID_ARGUMENT"
            }
        })))
        .mount(&server)
        .await;

    let provider = fcm_provider(&server.uri());
    let dispatch = Dispatch::to(Target::fcm(DEVICE_TOKEN));
    let message = Message::text("hello");

    let err = provider.send(&dispatch, &message).await.unwrap_err();
    assert!(
        matches!(err, crate::MessengerError::Provider { provider, .. } if provider == ProviderKind::Fcm),
        "expected Provider, got {err:?}"
    );
}

#[tokio::test]
async fn messenger_routes_to_fcm_provider() {
    let server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path_regex(format!(
            r"^/v1/projects/{PROJECT_ID}/messages.*"
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "projects/test-project-12345/messages/0:1234567893"
        })))
        .mount(&server)
        .await;

    let mut messenger = Messenger::new();
    messenger.register(Box::new(fcm_provider(&server.uri())));

    let dispatch = Dispatch::to(Target::fcm(DEVICE_TOKEN));
    let message = Message::text("hello");

    let receipt = messenger.send(dispatch, &message).await.unwrap();
    assert_eq!(receipt.provider, ProviderKind::Fcm);
    assert_eq!(
        receipt.raw_id,
        "projects/test-project-12345/messages/0:1234567893"
    );
}
