//! `.build()` refuses exactly the arguments the send refuses: every
//! `InvalidArgument` the first-hop preparation can produce, for `Call::build`,
//! `FullCall::build`, and the await of `client.send(request)`, with no request
//! reaching the server.

use bytes::Bytes;
use http::Method;
use schematic_runtime::{
    ApiDefinition, AuthConfig, CallError, Credential, CredentialPlacement, InvalidArgument, Json, RequestBody,
    RequestParts, TransportError, TransportErrorKind,
};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Call, Item, builder};

type ItemCall = Call<Json<Item>>;

/// A server answering `/ok` with an item; it records every request.
async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(path("/ok"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(&br#"{"id":7}"#[..], "application/json"))
        .mount(&server)
        .await;
    server
}

async fn request_count(server: &MockServer) -> usize {
    server.received_requests().await.expect("recording is on").len()
}

fn accepting(placement: CredentialPlacement) -> ApiDefinition {
    ApiDefinition::new("https://unused.invalid").auth(AuthConfig::none().accept(placement))
}

fn bearer_api() -> ApiDefinition {
    accepting(CredentialPlacement::Bearer { header: None })
}

fn ok_parts(edit: impl FnOnce(&mut RequestParts)) -> ItemCall {
    let mut parts = RequestParts::new(Method::GET, "/ok");
    edit(&mut parts);
    ItemCall::new(parts)
}

/// Asserts that `Call::build`, `FullCall::build`, and the await of a
/// `client.send(request)` call all refuse `request` (with the per-call
/// `credential`, if any) as the same `InvalidArgument`, mentioning
/// `reason_part`, and that nothing reached the server.
async fn assert_refused_before_sending(
    definition: ApiDefinition,
    request: ItemCall,
    credential: Option<Credential>,
    reason_part: &str,
) {
    let server = server().await;
    let client = builder(definition, &server.uri()).build().expect("client");
    let call = || {
        let call = client.send(request.clone());
        match &credential {
            Some(credential) => call.credential(credential.clone()),
            None => call,
        }
    };

    let refused: InvalidArgument = call().build().expect_err("Call::build refuses");
    assert!(refused.reason().contains(reason_part), "unexpected reason {:?}", refused.reason());
    let refused_full = call().full().build().expect_err("FullCall::build refuses");
    assert_eq!(refused_full, refused);
    assert_eq!(request_count(&server).await, 0, "build does no network work");

    let error = call().await.expect_err("the send refuses");
    let CallError::Transport(TransportError {
        kind: TransportErrorKind::InvalidArgument(argument),
        ..
    }) = &error
    else {
        panic!("expected an invalid argument, got {error:?}");
    };
    assert_eq!(argument, &refused, "build and send refuse with the same argument");
    assert_eq!(request_count(&server).await, 0, "the send refused before any request");
}

#[tokio::test]
async fn build_refuses_a_per_call_credential_kind_the_api_does_not_accept() {
    let request = ok_parts(|_| {});

    assert_refused_before_sending(bearer_api(), request, Some(Credential::basic("user", "password")), "does not accept").await;
}

#[tokio::test]
async fn build_refuses_a_credential_token_ending_in_a_newline() {
    let request = ok_parts(|_| {});

    assert_refused_before_sending(bearer_api(), request, Some(Credential::access_token("token\n")), "not a valid header value").await;
}

#[tokio::test]
async fn build_refuses_an_invalid_request_header_name() {
    let request = ok_parts(|parts| parts.headers = vec![("bad name".into(), "value".into())]);

    assert_refused_before_sending(bearer_api(), request, None, "header `bad name`").await;
}

#[tokio::test]
async fn build_refuses_a_request_header_value_ending_in_a_newline() {
    let request = ok_parts(|parts| parts.headers = vec![("x-note".into(), "value\n".into())]);

    assert_refused_before_sending(bearer_api(), request, None, "header `x-note`").await;
}

#[tokio::test]
async fn build_refuses_a_raw_body_content_type_ending_in_a_newline() {
    let request = ok_parts(|parts| {
        parts.body = RequestBody::Raw {
            content_type: "text/plain\n".into(),
            bytes: Bytes::from_static(b"hello"),
        };
    });

    assert_refused_before_sending(bearer_api(), request, None, "content type").await;
}

#[tokio::test]
async fn build_refuses_a_bearer_credential_over_a_request_authorization_header() {
    let request = ok_parts(|parts| parts.headers = vec![("authorization".into(), "Bearer from-parts".into())]);

    assert_refused_before_sending(bearer_api(), request, Some(Credential::access_token("token")), "already sets the header").await;
}

#[tokio::test]
async fn build_refuses_a_query_credential_over_a_request_query_value_of_the_same_name() {
    let definition = accepting(CredentialPlacement::ApiKeyQuery { name: "key".into() });
    let request = ok_parts(|parts| parts.query = vec![("key".into(), "from-parts".into())]);

    assert_refused_before_sending(definition, request, Some(Credential::api_key("secret")), "query parameter `key`").await;
}

#[tokio::test]
async fn build_refuses_a_cookie_credential_over_a_request_cookie_of_the_same_name() {
    let definition = accepting(CredentialPlacement::ApiKeyCookie { name: "key".into() });
    let request = ok_parts(|parts| parts.headers = vec![("Cookie".into(), "a=1; key=from-parts".into())]);

    assert_refused_before_sending(definition, request, Some(Credential::api_key("secret")), "cookie `key`").await;
}

#[tokio::test]
async fn build_refuses_a_client_credential_over_a_request_authorization_header() {
    let server = server().await;
    let client = builder(bearer_api(), &server.uri())
        .credentials(Credential::access_token("client-token"))
        .build()
        .expect("client");
    let request = ok_parts(|parts| parts.headers = vec![("authorization".into(), "Bearer from-parts".into())]);

    client.send(request.clone()).build().expect_err("Call::build checks the client's credential too");
    client.send(request.clone()).full().build().expect_err("FullCall::build likewise");
    client.send(request).await.expect_err("the send refuses");

    assert_eq!(request_count(&server).await, 0);
}

#[tokio::test]
async fn build_still_refuses_a_request_that_cannot_be_encoded() {
    let request = ItemCall::invalid("`id` is not encodable");

    assert_refused_before_sending(bearer_api(), request, None, "`id` is not encodable").await;
}

#[tokio::test]
async fn build_accepts_a_conflicting_request_when_the_call_is_anonymous_as_the_send_does() {
    let server = server().await;
    let client = builder(bearer_api(), &server.uri())
        .credentials(Credential::access_token("client-token"))
        .build()
        .expect("client");
    let request = ok_parts(|parts| parts.headers = vec![("authorization".into(), "Bearer from-parts".into())]);

    let built = client
        .send(request.clone())
        .credential(Credential::access_token("call-token"))
        .anonymous()
        .build()
        .expect("anonymous strips the slot, so nothing conflicts");
    assert!(client.send(request).anonymous().full().build().is_ok(), "FullCall::build agrees");
    assert_eq!(request_count(&server).await, 0);

    assert_eq!(built.await.expect("data"), Item { id: 7 });
    let requests = server.received_requests().await.expect("recording is on");
    assert_eq!(requests.len(), 1);
    assert!(!requests[0].headers.contains_key("authorization"), "anonymous sends no authorization");
}

#[tokio::test]
async fn a_valid_call_builds_without_sending_and_then_sends_exactly_one_request() {
    let server = server().await;
    let client = builder(bearer_api(), &server.uri()).build().expect("client");

    let built = client.send(ok_parts(|_| {})).credential(Credential::access_token("token")).build().expect("valid");
    assert!(client.send(ok_parts(|_| {})).full().build().is_ok(), "FullCall::build agrees");
    assert_eq!(request_count(&server).await, 0, "a successful build sends nothing");

    assert_eq!(built.await.expect("data"), Item { id: 7 });
    assert_eq!(request_count(&server).await, 1);
}
