//! The call returned by `send`: what an await returns, what `.full()`
//! returns, `.build()`'s early check, re-sending a built call, the per-call
//! settings, and the combined `CallError`.

use std::error::Error as _;
use std::time::Duration;

use bytes::Bytes;
use schematic_runtime::{
    ApiDefinition, ApiError, AuthConfig, AuthMethod, CallError, Credential, CredentialPlacement, Deadline, ErrorKind,
    Json, NoDeclaredError, Outcome, TransportError, TransportErrorKind,
};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{
    AbortOnDrop, Call, Item, ProviderError, SENTINEL, builder, closed_port_url, listener, plain_client, read_head,
    write_all,
};

type ItemCall = Call<Json<Item>, ProviderError>;

/// A server answering `/ok` with an item, `/declared` with a declared 404,
/// and `/malformed` with a 2xx body that is not JSON; every reply carries a
/// request id.
async fn server() -> MockServer {
    let server = MockServer::start().await;
    for (route, status, body, content_type) in [
        ("/ok", 200_u16, &br#"{"id":7}"#[..], "application/json"),
        ("/declared", 404, &br#"{"message":"no such item"}"#[..], "application/json"),
        ("/malformed", 200, &b"<html>"[..], "text/html"),
    ] {
        Mock::given(path(route))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_raw(body, content_type)
                    .append_header("x-request-id", format!("req-{route}").as_str()),
            )
            .mount(&server)
            .await;
    }
    server
}

fn definition() -> ApiDefinition {
    ApiDefinition::new("https://unused.invalid")
        .request_id_header("x-request-id")
        .auth(AuthConfig::none().accept(CredentialPlacement::Bearer { header: None }))
}

async fn request_count(server: &MockServer) -> usize {
    server.received_requests().await.expect("recording is on").len()
}

fn api_parts(error: CallError<ProviderError>) -> (ApiError<ProviderError>, u16) {
    match error {
        CallError::Api { error, meta } => (error, meta.status.as_u16()),
        other => panic!("expected an API failure, got {other:?}"),
    }
}

#[tokio::test]
async fn awaiting_a_call_returns_the_data_of_a_2xx_reply() {
    let server = server().await;
    let client = builder(definition(), &server.uri()).build().expect("client");

    let item = client.send(ItemCall::get("/ok")).await.expect("data");

    assert_eq!(item, Item { id: 7 });
}

#[tokio::test]
async fn a_declared_error_or_an_undecodable_body_is_an_api_error_with_its_response_kept() {
    let server = server().await;
    let client = builder(definition(), &server.uri()).build().expect("client");

    let declared = client.send(ItemCall::get("/declared")).await.expect_err("declared error");
    assert_eq!(declared.request_meta().endpoint_id, "call");
    assert_eq!(declared.request_meta().url.as_str(), format!("{}/declared", server.uri()));
    let CallError::Api { error, meta } = declared else {
        panic!("expected an API failure");
    };
    assert_eq!(meta.status, 404);
    assert_eq!(meta.request_id.as_deref(), Some("req-/declared"));
    assert_eq!(
        error.declared(),
        Some(&ProviderError {
            message: "no such item".into()
        })
    );
    assert_eq!(error.body, Bytes::from_static(br#"{"message":"no such item"}"#));

    let malformed = client.send(ItemCall::get("/malformed")).await.expect_err("undecodable");
    let (error, status) = api_parts(malformed);
    assert_eq!(status, 200);
    assert_eq!(error.kind, ErrorKind::MalformedSuccess);
    assert_eq!(error.body, Bytes::from_static(b"<html>"));
}

#[tokio::test]
async fn a_body_cut_off_after_the_head_is_an_api_error_with_the_bytes_that_arrived() {
    let (listener, url) = listener().await;
    let _server = AbortOnDrop(tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        read_head(&mut stream).await;
        write_all(&mut stream, b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 100\r\n\r\n{\"id\"").await;
    }));
    let client = plain_client(&url);

    let error = client.send(ItemCall::get("/item")).await.expect_err("interrupted");

    assert_eq!(error.request_meta().url.as_str(), format!("{url}/item"));
    let (error, status) = api_parts(error);
    assert_eq!(status, 200);
    assert_eq!(error.kind, ErrorKind::BodyInterrupted);
    assert_eq!(error.body, Bytes::from_static(b"{\"id\""));
}

#[tokio::test]
async fn a_connection_failure_is_a_transport_error_with_the_request_kept() {
    let url = closed_port_url().await;
    let client = plain_client(&url);

    let error = client.send(ItemCall::get("/item")).await.expect_err("no head");

    assert_eq!(error.request_meta().url.as_str(), format!("{url}/item"));
    let CallError::Transport(transport) = error else {
        panic!("expected a transport failure");
    };
    assert_eq!(transport.kind, TransportErrorKind::Connect);
}

#[tokio::test]
async fn full_returns_the_envelope_for_a_success_and_for_an_error_reply() {
    let server = server().await;
    let client = builder(definition(), &server.uri()).build().expect("client");

    let success = client.send(ItemCall::get("/ok")).full().await.expect("head");
    assert_eq!(success.meta.status, 200);
    assert_eq!(success.meta.headers["content-type"], "application/json");
    assert_eq!(success.meta.request_id.as_deref(), Some("req-/ok"));
    assert_eq!(success.outcome, Outcome::Success(Item { id: 7 }));

    let failure = client.send(ItemCall::get("/declared")).full().await.expect("head");
    assert_eq!(failure.meta.status, 404);
    assert_eq!(failure.meta.request_id.as_deref(), Some("req-/declared"));
    assert_eq!(failure.error().and_then(ApiError::declared).map(|e| e.message.as_str()), Some("no such item"));

    let url = closed_port_url().await;
    let error = plain_client(&url).send(ItemCall::get("/item")).full().await.expect_err("no head");
    assert_eq!(error.kind, TransportErrorKind::Connect);
}

#[tokio::test]
async fn build_refuses_an_invalid_argument_before_any_request_is_sent() {
    let server = server().await;
    let client = builder(definition(), &server.uri()).build().expect("client");

    let refused = client.send(ItemCall::invalid("`id` is not encodable")).build().expect_err("refused");
    assert_eq!(refused.reason(), "`id` is not encodable");
    let refused_full = client.send(ItemCall::invalid("`id` is not encodable")).full().build().expect_err("refused");
    assert_eq!(refused_full, refused);
    assert_eq!(request_count(&server).await, 0, "build does no network work");

    // Without `build`, the same check runs at the await, still before sending.
    let error = client.send(ItemCall::invalid("`id` is not encodable")).await.expect_err("refused");
    let CallError::Transport(TransportError {
        kind: TransportErrorKind::InvalidArgument(argument),
        ..
    }) = &error
    else {
        panic!("expected an invalid argument, got {error:?}");
    };
    assert_eq!(argument, &refused);
    assert_eq!(request_count(&server).await, 0);

    let built = client.send(ItemCall::get("/ok")).build().expect("valid");
    assert_eq!(request_count(&server).await, 0, "a successful build sends nothing either");
    assert_eq!(built.await.expect("data"), Item { id: 7 });
}

#[tokio::test]
async fn a_built_call_sent_twice_makes_two_requests_and_two_results() {
    let server = server().await;
    let client = builder(definition(), &server.uri()).build().expect("client");

    let built = client.send(ItemCall::get("/ok")).credential(Credential::access_token("t")).build().expect("valid");
    let first = built.clone().await.expect("first");
    let second = built.await.expect("second");
    assert_eq!((first, second), (Item { id: 7 }, Item { id: 7 }));

    let full = client.send(ItemCall::get("/declared")).full().build().expect("valid");
    let (first, second) = (full.clone().await.expect("head"), full.await.expect("head"));
    assert_eq!((first.meta.status.as_u16(), second.meta.status.as_u16()), (404, 404));

    let received = server.received_requests().await.expect("recorded");
    assert_eq!(received.len(), 4);
    for request in &received[..2] {
        assert_eq!(request.headers["authorization"], "Bearer t", "a clone keeps the per-call settings");
    }
}

#[tokio::test]
async fn per_call_settings_behave_on_call_and_full_alike_and_the_last_credential_mode_wins() {
    let server = server().await;
    let client = builder(definition(), &server.uri())
        .credentials(Credential::access_token("client"))
        .build()
        .expect("client");
    let call = || client.send(ItemCall::get("/ok"));

    // Each case: the call as configured, and the authorization header sent.
    let cases = [
        (call().full(), Some("Bearer client"), AuthMethod::ClientLevel),
        (call().anonymous().full(), None, AuthMethod::Anonymous),
        (call().full().anonymous(), None, AuthMethod::Anonymous),
        (call().credential(Credential::access_token("call")).full(), Some("Bearer call"), AuthMethod::PerCall),
        (call().full().credential(Credential::access_token("call")), Some("Bearer call"), AuthMethod::PerCall),
        (
            call().credential(Credential::access_token("call")).anonymous().full(),
            None,
            AuthMethod::Anonymous,
        ),
        (
            call().anonymous().full().credential(Credential::access_token("last")),
            Some("Bearer last"),
            AuthMethod::PerCall,
        ),
    ];
    let count = cases.len();
    for (index, (configured, header, auth)) in cases.into_iter().enumerate() {
        let response = configured.await.expect("head");
        assert_eq!(response.meta.request.auth, auth, "case {index}");
        let received = server.received_requests().await.expect("recorded");
        let sent = received[index].headers.get("authorization").map(|value| value.to_str().expect("ascii"));
        assert_eq!(sent, header, "case {index}");
    }
    assert_eq!(request_count(&server).await, count);

    // The data path reports the same credential source.
    let error = client
        .send(ItemCall::get("/declared"))
        .credential(Credential::access_token("call"))
        .await
        .expect_err("declared");
    assert_eq!(error.request_meta().auth, AuthMethod::PerCall);
}

#[tokio::test]
async fn an_expired_deadline_is_a_transport_error_on_call_and_full_and_nothing_is_sent() {
    let server = server().await;
    let client = builder(definition(), &server.uri()).build().expect("client");
    let spent = Deadline::after(Duration::ZERO);

    let error = client.send(ItemCall::get("/ok")).deadline(&spent).await.expect_err("expired");
    let CallError::Transport(transport) = error else {
        panic!("expected a transport failure");
    };
    assert!(matches!(transport.kind, TransportErrorKind::DeadlineExceeded { .. }), "{:?}", transport.kind);

    let error = client.send(ItemCall::get("/ok")).full().deadline(&spent).await.expect_err("expired");
    assert!(matches!(error.kind, TransportErrorKind::DeadlineExceeded { .. }), "{:?}", error.kind);
    assert_eq!(request_count(&server).await, 0);

    // A live deadline counts each completed call, through either await.
    let live = Deadline::after(Duration::from_secs(30));
    client.send(ItemCall::get("/ok")).deadline(&live).await.expect("data");
    client.send(ItemCall::get("/ok")).deadline(&live).full().await.expect("head");
    assert_eq!(live.completed_requests(), 2);
}

#[tokio::test]
async fn map_request_replaces_the_request_and_keeps_the_settings() {
    let server = server().await;
    let client = builder(definition(), &server.uri()).build().expect("client");

    let response = client
        .send(ItemCall::get("/declared"))
        .anonymous()
        .map_request(|_| ItemCall::get("/ok"))
        .full()
        .await
        .expect("head");

    assert_eq!(response.outcome, Outcome::Success(Item { id: 7 }));
    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
}

#[test]
fn a_call_can_move_to_another_thread_before_it_is_awaited() {
    fn assert_send<T: Send>(_: &T) {}
    let client = plain_client("http://127.0.0.1:9");
    let call = client.send(ItemCall::get("/item"));
    assert_send(&call);
    assert_send(&call.clone().full());
}

#[tokio::test]
async fn call_error_display_names_the_call_without_secrets_and_its_source_is_the_inner_error() {
    let server = MockServer::start().await;
    Mock::given(path("/item"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(format!(r#"{{"message":"bad key {SENTINEL}"}}"#), "application/json"))
        .mount(&server)
        .await;
    let secret = ApiDefinition::new("https://unused.invalid")
        .auth(AuthConfig::none().accept(CredentialPlacement::ApiKeyQuery { name: "key".into() }));
    let client = builder(secret.clone(), &server.uri()).build().expect("client");

    let api = client.send(ItemCall::get("/item")).credential(Credential::api_key(SENTINEL)).await.expect_err("401");
    let shown = api.to_string();
    assert!(shown.starts_with("call call failed: 401 Unauthorized from GET "), "{shown}");
    assert!(!shown.contains(SENTINEL), "{shown}");
    assert!(!format!("{:?}", api.request_meta()).contains(SENTINEL));
    let source = api.source().expect("a source");
    let inner = source.downcast_ref::<ApiError<ProviderError>>().expect("the API error");
    assert_eq!(inner.kind, ErrorKind::Declared(ProviderError { message: format!("bad key {SENTINEL}") }));
    assert!(!source.to_string().contains(SENTINEL));

    let unreachable = builder(secret, &closed_port_url().await).build().expect("client");
    let transport =
        unreachable.send(ItemCall::get("/item")).credential(Credential::api_key(SENTINEL)).await.expect_err("no head");
    let shown = transport.to_string();
    assert_eq!(shown, "call call failed before a response arrived");
    let source = transport.source().expect("a source");
    let inner = source.downcast_ref::<TransportError>().expect("the transport error");
    assert_eq!(inner.kind, TransportErrorKind::Connect);
    assert!(!source.to_string().contains(SENTINEL), "{source}");
    assert!(!format!("{:?}", transport.request_meta()).contains(SENTINEL));
}

#[tokio::test]
async fn call_error_is_matched_with_a_wildcard_arm_and_converts_from_a_transport_error() {
    let client = plain_client("http://127.0.0.1:9");
    let error: CallError<NoDeclaredError> = client.send(Call::<Json<Item>>::invalid("bad")).await.expect_err("refused");

    // `CallError` is non-exhaustive, so a downstream match needs this arm.
    #[allow(unreachable_patterns)]
    let described = match &error {
        CallError::Transport(transport) => transport.kind.to_string(),
        CallError::Api { error, .. } => error.to_string(),
        _ => unreachable!(),
    };
    assert_eq!(described, "invalid argument: bad");

    let CallError::Transport(transport) = error.clone() else {
        panic!("expected a transport failure");
    };
    assert_eq!(CallError::<NoDeclaredError>::from(transport), error);
}
