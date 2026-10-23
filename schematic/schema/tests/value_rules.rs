//! The committed clients check their declared request value rules before
//! sending, and decode their replies without checking those rules.
//!
//! OpenAI's `CreateSpeech` body is the imported `CreateSpeechRequest2`,
//! whose fields carry the OpenAPI document's `maxLength` on `input` and
//! `minimum`/`maximum` on `speed` as Schemars attributes. Calls are
//! anonymous so a host credential never reaches the mock.

use schematic_schema::openai::{
    CreateSpeechRequest, CreateSpeechRequest2, CreateVectorStoreRequest, CreateVectorStoreRequest2,
    VectorStoreExpirationAfter,
};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::TransportErrorKind;
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, ResponseTemplate};

const REFUSAL: &str = "2 constraint violations: \
    body.input violates maxLength: must be at most 4096 characters long; \
    body.speed violates maximum: must be at most 4";

#[tokio::test]
async fn an_openai_speech_request_outside_its_declared_rules_is_refused_before_sending() {
    let server = MockServer::start().await;
    Mock::given(any()).respond_with(ResponseTemplate::new(200)).mount(&server).await;
    let client = OpenAI::new().base_url(server.uri()).build().expect("client builds");

    // 4096 characters of four bytes each pass: the limit counts characters.
    let valid = CreateSpeechRequest2 { input: "😀".repeat(4096), speed: Some(4.0), ..CreateSpeechRequest2::default() };
    CreateSpeechRequest::new(valid).validate().expect("within every declared rule");

    let body = CreateSpeechRequest2 { input: "x".repeat(4097), speed: Some(4.5), ..CreateSpeechRequest2::default() };
    let violations = CreateSpeechRequest::new(body.clone()).validate().unwrap_err();
    assert_eq!(violations.to_string(), REFUSAL);

    let built = client.send(CreateSpeechRequest::new(body.clone())).anonymous().build().unwrap_err();
    assert_eq!(built.reason(), REFUSAL);
    let error = client.send(CreateSpeechRequest::new(body)).anonymous().full().await.unwrap_err();
    let TransportErrorKind::InvalidArgument(argument) = &error.kind else {
        panic!("expected an invalid argument, got {error:?}");
    };
    assert_eq!(argument.reason(), REFUSAL);
    assert!(server.received_requests().await.unwrap().is_empty(), "nothing is sent");
}

/// Value rules are request-side only. `VectorStoreExpirationAfter` declares
/// `days` within 1–365 and `anchor` as `last_active_at`, and the client
/// refuses a `CreateVectorStore` body that breaks them; a `GetVectorStore`
/// reply breaking the same rules, and the enums on `object` and `status`,
/// still decodes and keeps the server's values.
#[tokio::test]
async fn a_reply_outside_its_declared_value_rules_still_decodes() {
    let server = MockServer::start().await;
    let reply = serde_json::json!({
        "id": "vs_1",
        "object": "not_a_vector_store",
        "created_at": 0,
        "name": "",
        "usage_bytes": -1,
        "file_counts": {},
        "status": "archived",
        "expires_after": { "anchor": "created_at", "days": 4000 },
        "last_active_at": null,
        "metadata": {}
    });
    Mock::given(any()).respond_with(ResponseTemplate::new(200).set_body_json(&reply)).mount(&server).await;
    let client = OpenAI::new().base_url(server.uri()).build().expect("client builds");

    let expires_after = VectorStoreExpirationAfter { anchor: "created_at".into(), days: 4000 };
    let body = CreateVectorStoreRequest2 { expires_after: Some(expires_after), ..CreateVectorStoreRequest2::default() };
    let refused = CreateVectorStoreRequest::new(body).validate().unwrap_err();
    assert_eq!(
        refused.to_string(),
        "2 constraint violations: \
         body.expires_after.anchor violates enum: must equal the one allowed value; \
         body.expires_after.days violates maximum: must be at most 365"
    );
    assert!(server.received_requests().await.unwrap().is_empty(), "the refused request is not sent");

    let store = client.get_vector_store("vs_1").anonymous().await.expect("the reply decodes");
    assert_eq!(store.object, "not_a_vector_store");
    assert_eq!(store.status, "archived");
    assert_eq!(store.usage_bytes, -1);
    let decoded_expiry = store.expires_after.as_ref().expect("expires_after is kept");
    assert_eq!((decoded_expiry.anchor.as_str(), decoded_expiry.days), ("created_at", 4000));
    assert_eq!(serde_json::to_value(&store).unwrap(), reply, "nothing is dropped or clamped");
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}
