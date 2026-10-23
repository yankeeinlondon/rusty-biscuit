//! The committed clients check their declared request value rules before
//! sending.
//!
//! OpenAI's `CreateSpeech` body is the imported `CreateSpeechRequest2`,
//! whose fields carry the OpenAPI document's `maxLength` on `input` and
//! `minimum`/`maximum` on `speed` as Schemars attributes. Calls are
//! anonymous so a host credential never reaches the mock.

use schematic_schema::openai::{CreateSpeechRequest, CreateSpeechRequest2};
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
