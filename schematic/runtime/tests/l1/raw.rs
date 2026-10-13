//! Raw sends (R27, PD14): the reply's metadata and bytes with a completion
//! disposition, through the same credentials, origin policy, redirects,
//! deadline, and redaction as a typed send. The last test runs the same
//! endings through both sends to show they share one execution core.

use std::time::Duration;

use bytes::Bytes;
use http::Method;
use schematic_runtime::redact::REDACTED;
use schematic_runtime::{
    ApiDefinition, AuthConfig, AuthMethod, CallOptions, Credential, CredentialPlacement, Deadline, ErrorKind,
    FailureCategory, Json, RawCompletion, RawJsonError, RawResponse, RedirectRefusal, RequestParts, Response,
    ResponseMeta, SecretNames, Text, TransportErrorKind,
};
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Call, Item, ProviderError, SENTINEL, builder, closed_port_url, listener, plain_client, read_head, write_all};

async fn mount(server: &MockServer, at: &str, response: ResponseTemplate) {
    Mock::given(path(at)).respond_with(response).mount(server).await;
}

async fn request_count(server: &MockServer) -> usize {
    server.received_requests().await.expect("recording is on").len()
}

fn json(status: u16, body: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).set_body_raw(body.to_owned(), "application/json")
}

fn redirect(status: u16, location: &str) -> ResponseTemplate {
    ResponseTemplate::new(status).insert_header("Location", location).set_body_string("moved")
}

/// A one-connection server that sends a head promising 100 bytes, then
/// `partial` (which is valid JSON on its own), then closes.
async fn cut_off_server(partial: &'static [u8]) -> String {
    let (listener, url) = listener().await;
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        read_head(&mut stream).await;
        let mut reply = b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 100\r\n\r\n".to_vec();
        reply.extend_from_slice(partial);
        write_all(&mut stream, &reply).await;
    });
    url
}

#[tokio::test]
async fn a_complete_reply_exposes_status_headers_and_body_from_one_record() {
    let server = MockServer::start().await;
    mount(
        &server,
        "/item",
        json(200, r#"{"id":7}"#).append_header("x-multi", "a").append_header("x-multi", "b"),
    )
    .await;

    let raw = plain_client(&server.uri())
        .send_raw(Call::<Json<Item>>::get("/item"), &CallOptions::new())
        .await
        .expect("head arrived");

    assert_eq!(raw.completion, RawCompletion::Complete);
    assert!(raw.is_complete());
    assert_eq!(raw.status(), 200);
    assert_eq!(raw.status(), raw.meta.status);
    assert!(std::ptr::eq(raw.headers(), &raw.meta.headers), "accessors read the one metadata record");
    let multi: Vec<_> = raw.headers().get_all("x-multi").iter().collect();
    assert_eq!(multi, ["a", "b"]);
    assert_eq!(raw.body, Bytes::from_static(br#"{"id":7}"#));
    assert_eq!(raw.json::<Item>(), Ok(Item { id: 7 }));
    assert_eq!(raw.meta.request.endpoint_id, "call");
    assert_eq!(raw.meta.request.method, Method::GET);

    let text = raw.to_string();
    assert!(text.starts_with("200 OK from GET "), "{text}");
    assert!(text.ends_with(": complete (8 body bytes)"), "{text}");
    assert!(!text.contains(r#""id""#), "Display never prints the body: {text}");
}

#[tokio::test]
async fn http_error_statuses_are_complete_raw_responses_with_no_classification() {
    let server = MockServer::start().await;
    mount(&server, "/missing", json(404, r#"{"message":"no such item"}"#)).await;
    mount(&server, "/broken", ResponseTemplate::new(503).set_body_string("<html>down</html>")).await;

    // The endpoint declares an error body; a raw send still does not decode it.
    let client = plain_client(&server.uri());
    let missing = client
        .send_raw(Call::<Json<Item>, ProviderError>::get("/missing"), &CallOptions::new())
        .await
        .expect("head arrived");
    assert_eq!(missing.status(), 404);
    assert_eq!(missing.completion, RawCompletion::Complete);
    assert_eq!(missing.json::<ProviderError>(), Ok(ProviderError { message: "no such item".into() }));

    let broken = client.send_raw(Call::<Json<Item>>::get("/broken"), &CallOptions::new()).await.expect("head arrived");
    assert_eq!(broken.status(), 503);
    assert_eq!(broken.completion, RawCompletion::Complete);
    assert_eq!(broken.body, Bytes::from_static(b"<html>down</html>"));
    assert_eq!(broken.json::<Item>(), Err(RawJsonError::Malformed), "a complete non-JSON body");
    assert_eq!(broken.json::<serde_json::Value>(), Err(RawJsonError::Malformed));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_cut_off_body_is_interrupted_and_json_refuses_it_even_when_the_bytes_parse() {
    const PARTIAL: &[u8] = br#"{"id":1}"#;
    let url = cut_off_server(PARTIAL).await;

    let raw = plain_client(&url)
        .send_raw(Call::<Json<Item>>::get("/item"), &CallOptions::new())
        .await
        .expect("a head arrived, so not a transport error");

    assert_eq!(raw.completion, RawCompletion::BodyInterrupted);
    assert!(!raw.is_complete());
    assert_eq!(raw.status(), 200);
    assert_eq!(raw.headers()["content-length"], "100");
    assert_eq!(raw.body, Bytes::from_static(PARTIAL), "the bytes that arrived are kept");
    assert!(serde_json::from_slice::<Item>(&raw.body).is_ok(), "control: the partial bytes alone parse");
    assert_eq!(raw.json::<Item>(), Err(RawJsonError::Incomplete));
    assert!(raw.to_string().ends_with(": response body was interrupted (8 body bytes)"), "{raw}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_body_read_still_waiting_at_the_expiration_is_interrupted_not_a_transport_error() {
    let (listener, url) = listener().await;
    tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.expect("accept");
        read_head(&mut stream).await;
        write_all(&mut stream, b"HTTP/1.1 200 OK\r\nContent-Length: 100\r\n\r\n{\"id\"").await;
        std::future::pending::<()>().await;
    });

    let raw = builder(ApiDefinition::new("https://api.example.invalid"), &url)
        .timeout(Duration::from_millis(300))
        .build()
        .expect("client")
        .send_raw(Call::<Json<Item>>::get("/item"), &CallOptions::new())
        .await
        .expect("the head arrived");

    assert_eq!(raw.completion, RawCompletion::BodyInterrupted);
    assert_eq!(raw.body, Bytes::from_static(br#"{"id""#));
    assert_eq!(raw.json::<serde_json::Value>(), Err(RawJsonError::Incomplete));
}

#[tokio::test]
async fn a_cross_origin_redirect_is_refused_with_the_redirects_evidence_and_never_reaches_the_target() {
    let a = MockServer::start().await;
    let b = MockServer::start().await;
    mount(&b, "/elsewhere", json(200, r#"{"id":1}"#)).await;
    mount(&a, "/item", redirect(302, &format!("{}/elsewhere?token={SENTINEL}", b.uri()))).await;

    let raw = plain_client(&a.uri())
        .send_raw(Call::<Json<Item>>::get("/item"), &CallOptions::new())
        .await
        .expect("head arrived");

    assert_eq!(raw.completion, RawCompletion::RedirectRefused { reason: RedirectRefusal::CrossOrigin });
    assert_eq!(request_count(&b).await, 0, "a refused origin receives no request");
    assert_eq!(raw.status(), 302);
    assert_eq!(raw.body, Bytes::from_static(b"moved"));
    let location = raw.headers()["location"].to_str().expect("text");
    assert!(location.starts_with(&b.uri()), "the destination stays visible: {location}");
    assert!(location.contains(&format!("token={REDACTED}")), "{location}");
    assert!(!format!("{:?}{raw}", raw.meta).contains(SENTINEL));
    assert_eq!(raw.json::<serde_json::Value>(), Err(RawJsonError::Incomplete));
}

#[tokio::test]
async fn a_failed_hop_after_a_followed_redirect_keeps_the_redirect_and_names_the_target() {
    let a = MockServer::start().await;
    let closed = closed_port_url().await;
    // Under `danger_allow_any` the closed port is followed, so the failure is
    // the connection, not the policy.
    mount(&a, "/item", redirect(307, &format!("{closed}/gone"))).await;
    let client = builder(
        ApiDefinition::new("https://api.example.invalid").cross_origin(schematic_runtime::OriginPolicy::danger_allow_any()),
        &a.uri(),
    )
    .build()
    .expect("client");
    let deadline = Deadline::after(Duration::from_secs(30));

    let raw = client
        .send_raw(Call::<Json<Item>>::get("/item"), &CallOptions::new().deadline(&deadline))
        .await
        .expect("a head arrived before the failed hop");

    let RawCompletion::RedirectFailed { target, category } = &raw.completion else {
        panic!("expected a failed redirect, got {:?}", raw.completion);
    };
    assert_eq!(*category, FailureCategory::Connect);
    assert_eq!(target.as_str(), format!("{closed}/gone"));
    assert_eq!(raw.status(), 307);
    assert_eq!(raw.body, Bytes::from_static(b"moved"));
    assert_eq!(deadline.completed_requests(), 1, "the logical send counts once");
    assert_eq!(raw.json::<serde_json::Value>(), Err(RawJsonError::Incomplete));
}

#[tokio::test]
async fn credentials_follow_the_same_precedence_and_never_reach_stored_metadata() {
    let server = MockServer::start().await;
    mount(&server, "/start", redirect(302, "/item")).await;
    mount(&server, "/item", json(200, r#"{"id":1}"#)).await;
    let definition = ApiDefinition::new("https://api.example.invalid")
        .auth(AuthConfig::none().accept(CredentialPlacement::Bearer { header: None }))
        .secret_names(["x-session"]);
    let client = builder(definition, &server.uri())
        .credentials(Credential::access_token("client-token"))
        .build()
        .expect("client");

    let mut parts = RequestParts::new(Method::GET, "/start");
    parts.headers = vec![("x-session".into(), SENTINEL.into())];
    parts.query = vec![("access_token".into(), SENTINEL.into())];
    let options = CallOptions::new().credential(Credential::access_token(SENTINEL));
    let raw = client.send_raw(Call::<Json<Item>>::new(parts), &options).await.expect("head arrived");

    assert_eq!(raw.completion, RawCompletion::Complete);
    assert_eq!(raw.meta.request.auth, AuthMethod::PerCall, "a per-call credential beats the client's");
    assert_eq!(raw.meta.request.url.as_str(), format!("{}/item", server.uri()), "metadata describes the final hop");
    let received = server.received_requests().await.expect("recording");
    assert_eq!(received.len(), 2);
    for request in &received {
        assert_eq!(request.headers["authorization"], format!("Bearer {SENTINEL}"), "{}", request.url);
    }
    let stored = format!("{:?} {raw}", raw.meta);
    assert!(!stored.contains(SENTINEL), "{stored}");
    assert_eq!(raw.meta.request.headers["authorization"], REDACTED);
    assert_eq!(raw.meta.request.headers["x-session"], REDACTED);

    let anonymous = client
        .send_raw(Call::<Json<Item>>::get("/item"), &CallOptions::anonymous())
        .await
        .expect("head arrived");
    assert_eq!(anonymous.meta.request.auth, AuthMethod::Anonymous);
    let last = server.received_requests().await.expect("recording").pop().expect("a request");
    assert!(!last.headers.contains_key("authorization"));
}

#[tokio::test]
async fn refusals_before_any_head_are_transport_errors_with_no_request() {
    let server = MockServer::start().await;
    let foreign = MockServer::start().await;
    mount(&server, "/item", json(200, r#"{"id":1}"#)).await;
    let client = plain_client(&server.uri());

    let invalid = client
        .send_raw(Call::<Json<Item>>::invalid("bad parameter"), &CallOptions::new())
        .await
        .expect_err("refused before sending");
    assert!(matches!(invalid.kind, TransportErrorKind::InvalidArgument(_)), "{invalid}");

    let elsewhere = client
        .send_raw(Call::<Json<Item>>::get(&format!("{}/item", foreign.uri())), &CallOptions::new())
        .await
        .expect_err("a foreign absolute URL is refused before sending");
    assert!(matches!(elsewhere.kind, TransportErrorKind::CrossOriginRejected { .. }), "{elsewhere}");

    let spent = Deadline::after(Duration::ZERO);
    let expired = client
        .send_raw(Call::<Json<Item>>::get("/item"), &CallOptions::new().deadline(&spent))
        .await
        .expect_err("an exhausted deadline sends nothing");
    assert!(
        matches!(expired.kind, TransportErrorKind::DeadlineExceeded { completed_requests: 0, .. }),
        "{expired}"
    );

    assert_eq!(request_count(&server).await, 0);
    assert_eq!(request_count(&foreign).await, 0);
    assert_eq!(spent.completed_requests(), 0, "a send with no head is not counted");
}

#[tokio::test]
async fn a_deadline_shorter_than_the_reply_ends_the_raw_send_before_the_head() {
    let server = MockServer::start().await;
    mount(&server, "/slow", json(200, r#"{"id":1}"#).set_delay(Duration::from_millis(800))).await;
    let deadline = Deadline::after(Duration::from_millis(200));

    let error = plain_client(&server.uri())
        .send_raw(Call::<Json<Item>>::get("/slow"), &CallOptions::new().deadline(&deadline))
        .await
        .expect_err("no head arrived in time");

    assert!(matches!(error.kind, TransportErrorKind::DeadlineExceeded { .. }), "{error}");
    assert!(error.elapsed < Duration::from_millis(700), "{:?}", error.elapsed);
    assert_eq!(deadline.completed_requests(), 0);
}

/// The fields a typed and a raw send must agree on; `elapsed` and the
/// server's `Date` header differ between two sends.
fn evidence(meta: &ResponseMeta) -> (u16, String, AuthMethod, http::HeaderMap, Option<String>) {
    let mut headers = meta.headers.clone();
    headers.remove(http::header::DATE);
    (
        meta.status.as_u16(),
        meta.request.url.as_str().to_owned(),
        meta.request.auth.clone(),
        headers,
        meta.request_id.clone(),
    )
}

/// A server that accepts every connection and closes it before any head, so
/// a hop to it fails at once on every OS (a refused loopback connection takes
/// about 2 s on Windows).
async fn closing_server() -> String {
    let (listener, url) = listener().await;
    tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            drop(stream);
        }
    });
    url
}

/// The raw completion a typed outcome corresponds to.
fn typed_disposition<T>(response: &Response<T, schematic_runtime::NoDeclaredError>) -> RawCompletion {
    match response.error().map(|error| &error.kind) {
        None | Some(ErrorKind::Undeclared | ErrorKind::MalformedSuccess) => RawCompletion::Complete,
        Some(ErrorKind::BodyInterrupted) => RawCompletion::BodyInterrupted,
        Some(ErrorKind::RedirectRefused { reason }) => RawCompletion::RedirectRefused { reason: *reason },
        Some(ErrorKind::RedirectFailed { target, category }) => RawCompletion::RedirectFailed {
            target: target.clone(),
            category: *category,
        },
        Some(other) => panic!("unexpected kind {other:?}"),
    }
}

fn typed_body<T>(response: &Response<T, schematic_runtime::NoDeclaredError>) -> Option<Bytes> {
    response.error().map(|error| error.body.clone())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn typed_and_raw_sends_report_the_same_evidence_for_every_ending() {
    let a = MockServer::start().await;
    let b = MockServer::start().await;
    let closing = closing_server().await;
    mount(&a, "/ok", json(200, r#"{"id":1}"#).insert_header("x-request-id", "r-1")).await;
    mount(&a, "/teapot", json(418, r#"{"message":"short"}"#)).await;
    mount(&a, "/html", ResponseTemplate::new(200).set_body_raw("<html/>", "application/json")).await;
    mount(&a, "/hop", redirect(301, "/ok")).await;
    mount(&a, "/away", redirect(302, &format!("{}/ok", b.uri()))).await;
    mount(&a, "/dead", redirect(308, &format!("{closing}/x"))).await;
    let definition = ApiDefinition::new("https://api.example.invalid")
        .request_id_header("x-request-id")
        .auth(AuthConfig::none().accept(CredentialPlacement::Bearer { header: None }));
    let same_origin = builder(definition.clone(), &a.uri())
        .credentials(Credential::access_token(SENTINEL))
        .build()
        .expect("client");
    // `/dead` needs a policy that follows the closing server's port.
    let any = builder(definition.cross_origin(schematic_runtime::OriginPolicy::danger_allow_any()), &a.uri())
        .credentials(Credential::access_token(SENTINEL))
        .build()
        .expect("client");

    let dead_target = SecretNames::builtin().redact_url(&url::Url::parse(&format!("{closing}/x")).expect("url"));
    for (client, at, expected) in [
        (&same_origin, "/ok", Some(RawCompletion::Complete)),
        (&same_origin, "/teapot", Some(RawCompletion::Complete)),
        (&same_origin, "/html", Some(RawCompletion::Complete)),
        (&same_origin, "/hop", Some(RawCompletion::Complete)),
        (&same_origin, "/away", Some(RawCompletion::RedirectRefused { reason: RedirectRefusal::CrossOrigin })),
        // How a closed connection is categorized can vary by OS, so only
        // the failed target is fixed here.
        (&any, "/dead", None),
    ] {
        let typed = client.send(Call::<Json<Item>>::get(at)).full().await.expect("typed head");
        let raw = client.send_raw(Call::<Json<Item>>::get(at), &CallOptions::new()).await.expect("raw head");

        match expected {
            Some(expected) => assert_eq!(raw.completion, expected, "{at}"),
            None => assert!(
                matches!(&raw.completion, RawCompletion::RedirectFailed { target, .. } if *target == dead_target),
                "{at}: {:?}",
                raw.completion
            ),
        }
        assert_eq!(typed_disposition(&typed), raw.completion, "{at}");
        assert_eq!(evidence(&typed.meta), evidence(&raw.meta), "{at}");
        if let Some(body) = typed_body(&typed) {
            assert_eq!(body, raw.body, "{at}: both keep the same bytes");
        }
        assert!(!format!("{:?} {raw}", raw.meta).contains(SENTINEL), "{at}");
    }
    assert_eq!(request_count(&b).await, 0, "neither send reached the refused origin");

    // A cut-off body is interrupted for both.
    for send_raw in [false, true] {
        let url = cut_off_server(br#"{"id":2}"#).await;
        let client = plain_client(&url);
        let (completion, body) = if send_raw {
            let raw: RawResponse = client.send_raw(Call::<Text>::get("/x"), &CallOptions::new()).await.expect("head");
            (raw.completion, raw.body)
        } else {
            let typed = client.send(Call::<Json<Item>>::get("/x")).full().await.expect("head");
            (typed_disposition(&typed), typed_body(&typed).expect("failure body"))
        };
        assert_eq!(completion, RawCompletion::BodyInterrupted, "raw: {send_raw}");
        assert_eq!(body, Bytes::from_static(br#"{"id":2}"#), "raw: {send_raw}");
    }
}
