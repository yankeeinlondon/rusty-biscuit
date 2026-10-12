//! Typed sends against a local mock: envelope placement, metadata, failure
//! classification, codecs, request assembly, and redaction of what is
//! stored (R3–R6, R8–R10, R12, R13, R28).

use bytes::Bytes;
use http::Method;
use schematic_runtime::redact::REDACTED;
use schematic_runtime::{
    ApiDefinition, AuthConfig, AuthMethod, Binary, CallOptions, Credential, CredentialPlacement, Empty, ErrorKind,
    FormPart, Json, NoDeclaredError, Outcome, RequestBody, RequestParts, Text, TransportErrorKind,
};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Call, Item, ProviderError, SENTINEL, builder, closed_port_url, plain_client};

async fn serve(status: u16, body: &'static [u8], content_type: &str) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/item"))
        .respond_with(ResponseTemplate::new(status).set_body_raw(body, content_type))
        .mount(&server)
        .await;
    server
}

async fn request_count(server: &MockServer) -> usize {
    server.received_requests().await.expect("recording is on").len()
}

#[tokio::test]
async fn a_success_reaches_the_caller_with_its_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/item"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_raw(r#"{"id":7}"#, "application/json")
                .append_header("x-request-id", "req-1")
                .append_header("x-multi", "a")
                .append_header("x-multi", "b"),
        )
        .mount(&server)
        .await;
    let client = builder(ApiDefinition::new("https://unused.invalid").request_id_header("X-Request-Id"), &server.uri())
        .build()
        .expect("client");

    let mut parts = RequestParts::new(Method::GET, "/item");
    parts.query = vec![("page".into(), "2".into())];
    parts.headers = vec![("x-trace".into(), "t1".into())];
    let response = client.send(Call::<Json<Item>>::new(parts), &CallOptions::new()).await.expect("head arrived");

    assert_eq!(response.data(), Some(&Item { id: 7 }));
    assert!(response.error().is_none());
    let meta = &response.meta;
    assert_eq!(meta.status, 200);
    assert_eq!(meta.request.endpoint_id, "call");
    assert_eq!(meta.request.method, Method::GET);
    assert_eq!(meta.request.url.as_str(), format!("{}/item?page=2", server.uri()));
    assert_eq!(meta.request.query, vec![("page".to_string(), "2".to_string())]);
    assert_eq!(meta.request.headers["x-trace"], "t1");
    assert_eq!(meta.request.auth, AuthMethod::Anonymous);
    assert_eq!(meta.request.page, None);
    assert_eq!(meta.request_id.as_deref(), Some("req-1"));
    let multi: Vec<_> = meta.headers.get_all("x-multi").iter().collect();
    assert_eq!(multi, ["a", "b"], "repeated response headers stay repeated");

    let (kept, outcome) = response.into_parts();
    assert_eq!(kept.status, 200);
    assert_eq!(outcome, Outcome::Success(Item { id: 7 }));
}

#[tokio::test]
async fn the_request_id_is_none_without_the_header_or_the_definition_field() {
    let server = serve(200, br#"{"id":1}"#, "application/json").await;
    let named = builder(ApiDefinition::new("https://unused.invalid").request_id_header("x-request-id"), &server.uri())
        .build()
        .expect("client");
    let response = named.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head");
    assert_eq!(response.meta.request_id, None, "the response does not carry the header");

    let server = MockServer::start().await;
    Mock::given(path("/item"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(r#"{"id":1}"#, "application/json").append_header("x-request-id", "r"))
        .mount(&server)
        .await;
    let unnamed = plain_client(&server.uri());
    let response = unnamed.send(Call::<Json<Item>>::get("/item"), &CallOptions::new()).await.expect("head");
    assert_eq!(response.meta.request_id, None, "the API names no request-id header");
}

#[tokio::test]
async fn a_declared_error_is_decoded_on_arrival_and_keeps_its_body() {
    let body: &[u8] = br#"{"message":"quota exceeded"}"#;
    let server = serve(429, body, "application/json").await;
    let client = plain_client(&server.uri());

    let response = client
        .send(Call::<Json<Item>, ProviderError>::get("/item"), &CallOptions::new())
        .await
        .expect("head arrived");

    assert_eq!(response.meta.status, 429);
    assert!(response.data().is_none());
    let error = response.error().expect("failure");
    assert_eq!(
        error.declared(),
        Some(&ProviderError {
            message: "quota exceeded".into()
        })
    );
    assert_eq!(error.body, Bytes::from_static(body));
    assert_eq!(response.meta.headers["content-type"], "application/json");
    assert!(!response.is_decode_failure() && !response.is_interrupted());
    assert!(response.into_result().is_err());
}

#[tokio::test]
async fn an_undecodable_declared_error_is_malformed_with_its_body() {
    let server = serve(400, b"<html>bad</html>", "text/html").await;
    let client = plain_client(&server.uri());
    let response = client
        .send(Call::<Json<Item>, ProviderError>::get("/item"), &CallOptions::new())
        .await
        .expect("head arrived");
    let error = response.error().expect("failure");
    assert_eq!(error.kind, ErrorKind::MalformedError);
    assert_eq!(error.body, Bytes::from_static(b"<html>bad</html>"));
    assert!(!error.is_decode_failure(), "a malformed error body is not a decode failure of a success");
}

#[tokio::test]
async fn statuses_outside_the_declared_range_and_endpoints_without_a_declared_error_are_undeclared() {
    for (status, body) in [(500_u16, &b""[..]), (404, &br#"{"message":"gone"}"#[..])] {
        let server = serve(status, body, "application/json").await;
        let client = plain_client(&server.uri());
        let response = client
            .send(Call::<Json<Item>, NoDeclaredError>::get("/item"), &CallOptions::new())
            .await
            .expect("a head arrived, so an envelope");
        let error = response.error().expect("failure");
        assert_eq!(error.kind, ErrorKind::Undeclared, "status {status}");
        assert_eq!(error.body, Bytes::from_static(body));
        assert_eq!(response.meta.status, status);
    }

    // A 3xx is not in the declared range even when the endpoint declares an error.
    let server = serve(304, b"", "application/json").await;
    let response = plain_client(&server.uri())
        .send(Call::<Json<Item>, ProviderError>::get("/item"), &CallOptions::new())
        .await
        .expect("head");
    assert_eq!(response.error().expect("failure").kind, ErrorKind::Undeclared);
}

#[tokio::test]
async fn html_on_a_json_endpoint_is_a_malformed_success_with_its_evidence() {
    let html: &[u8] = b"<!doctype html><title>Sign in</title>";
    let server = serve(200, html, "text/html").await;
    let response = plain_client(&server.uri())
        .send(Call::<Json<Item>, ProviderError>::get("/item"), &CallOptions::new())
        .await
        .expect("head arrived");

    assert_eq!(response.meta.status, 200);
    assert_eq!(response.meta.headers["content-type"], "text/html");
    assert!(response.is_decode_failure());
    assert!(!response.is_interrupted());
    let error = response.error().expect("failure");
    assert_eq!(error.kind, ErrorKind::MalformedSuccess);
    assert_eq!(error.body, Bytes::from_static(html));
    assert!(response.into_result().is_err());
}

#[tokio::test]
async fn codecs_judge_the_body_not_the_content_type() {
    let server = MockServer::start().await;
    for (route, body, content_type) in [
        ("/json", &br#"{"id":3}"#[..], "text/plain"),
        ("/text", &b"caf\xc3\xa9 \xff"[..], "application/octet-stream"),
        ("/binary", &b"\x00\x01{"[..], "application/json"),
        ("/empty", &b"ignored bytes"[..], "text/plain"),
    ] {
        Mock::given(path(route))
            .respond_with(ResponseTemplate::new(200).set_body_raw(body, content_type))
            .mount(&server)
            .await;
    }
    let client = plain_client(&server.uri());
    let options = CallOptions::new();

    let json = client.send(Call::<Json<Item>>::get("/json"), &options).await.expect("head");
    assert_eq!(json.data(), Some(&Item { id: 3 }), "valid JSON under a misleading content type decodes");
    let text = client.send(Call::<Text>::get("/text"), &options).await.expect("head");
    assert_eq!(text.data().map(String::as_str), Some("café \u{fffd}"), "invalid UTF-8 is replaced");
    let binary = client.send(Call::<Binary>::get("/binary"), &options).await.expect("head");
    assert_eq!(binary.data(), Some(&Bytes::from_static(b"\x00\x01{")));
    let empty = client.send(Call::<Empty>::get("/empty"), &options).await.expect("head");
    assert_eq!(empty.data(), Some(&()), "an unexpected body is drained and ignored");
}

#[tokio::test]
async fn a_refused_connection_is_a_transport_error_with_request_metadata() {
    let base = closed_port_url().await;
    let mut parts = RequestParts::new(Method::GET, "/item");
    parts.query = vec![("api_key".into(), SENTINEL.into()), ("page".into(), "2".into())];
    let error = plain_client(&base)
        .send(Call::<Json<Item>>::new(parts), &CallOptions::new())
        .await
        .expect_err("no head can arrive");

    assert_eq!(error.kind, TransportErrorKind::Connect);
    assert_eq!(error.request.url.as_str(), format!("{base}/item?api_key={REDACTED}&page=2"));
    for text in [error.to_string(), format!("{error:?}")] {
        assert!(!text.contains(SENTINEL), "{text}");
        assert!(text.contains("page=2"), "{text}");
        assert!(text.contains(&format!("api_key={REDACTED}")), "{text}");
    }
}

#[tokio::test]
async fn an_invalid_argument_is_refused_before_any_request() {
    let server = serve(200, br#"{"id":1}"#, "application/json").await;
    let client = plain_client(&server.uri());

    let error = client
        .send(Call::<Json<Item>>::invalid("page_size above the maximum"), &CallOptions::new())
        .await
        .expect_err("refused before sending");
    match &error.kind {
        TransportErrorKind::InvalidArgument(argument) => assert_eq!(argument.reason(), "page_size above the maximum"),
        other => panic!("expected InvalidArgument, got {other:?}"),
    }
    assert_eq!(error.request.method, Method::GET, "the endpoint's declared method");
    assert_eq!(error.request.url.as_str(), format!("{}/", server.uri()));
    assert_eq!(error.request.endpoint_id, "call");

    let mut parts = RequestParts::new(Method::POST, "/item");
    parts.headers = vec![("bad header".into(), "v".into())];
    let error = client.send(Call::<Json<Item>>::new(parts), &CallOptions::new()).await.expect_err("refused");
    assert!(matches!(error.kind, TransportErrorKind::InvalidArgument(_)), "{:?}", error.kind);
    assert_eq!(error.request.method, Method::POST);

    assert_eq!(request_count(&server).await, 0, "nothing reached the server");
}

#[tokio::test]
async fn query_order_duplicates_and_a_template_query_reach_the_server() {
    let server = serve(200, br#"{"id":1}"#, "application/json").await;
    let mut parts = RequestParts::new(Method::GET, "/item?fixed=1#fragment");
    parts.query = vec![("tag".into(), "a".into()), ("tag".into(), "b".into()), ("q".into(), "x y&z+~é".into())];
    let response = plain_client(&server.uri())
        .send(Call::<Json<Item>>::new(parts), &CallOptions::new())
        .await
        .expect("head");

    let received = server.received_requests().await.expect("recorded");
    assert_eq!(received[0].url.query(), Some("fixed=1&tag=a&tag=b&q=x%20y%26z%2B~%C3%A9"));
    assert_eq!(received[0].url.fragment(), None, "fragments are never sent");
    let names: Vec<_> = response.meta.request.query.iter().map(|(name, value)| format!("{name}={value}")).collect();
    assert_eq!(names, ["fixed=1", "tag=a", "tag=b", "q=x y&z+~é"]);
}

#[tokio::test]
async fn every_body_form_is_sent_with_its_content_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(204))
        .mount(&server)
        .await;
    let client = plain_client(&server.uri());
    let bodies = [
        RequestBody::Json(r#"{"a":1}"#.into()),
        RequestBody::UrlEncoded(vec![("a".into(), "1 2".into()), ("a".into(), "&".into())]),
        RequestBody::Multipart(vec![
            FormPart::Text {
                name: "model".into(),
                value: "m1".into(),
            },
            FormPart::File {
                name: "file".into(),
                file_name: "clip.wav".into(),
                mime: Some("audio/wav".into()),
                bytes: Bytes::from_static(b"RIFF"),
            },
        ]),
        RequestBody::Raw {
            content_type: "application/octet-stream".into(),
            bytes: Bytes::from_static(b"\x00raw"),
        },
    ];
    for body in bodies {
        let mut parts = RequestParts::new(Method::POST, "/upload");
        parts.body = body;
        let response = client.send(Call::<Empty>::new(parts), &CallOptions::new()).await.expect("head");
        assert_eq!(response.data(), Some(&()));
    }

    let received = server.received_requests().await.expect("recorded");
    let content_type = |index: usize| received[index].headers["content-type"].to_str().expect("ascii").to_string();
    assert_eq!(content_type(0), "application/json");
    assert_eq!(received[0].body, br#"{"a":1}"#);
    assert_eq!(content_type(1), "application/x-www-form-urlencoded");
    assert_eq!(received[1].body, b"a=1+2&a=%26");
    let boundary = content_type(2).strip_prefix("multipart/form-data; boundary=").expect("boundary").to_string();
    let multipart = String::from_utf8_lossy(&received[2].body).into_owned();
    assert!(multipart.contains("name=\"model\"\r\n\r\nm1\r\n"), "{multipart}");
    assert!(multipart.contains("filename=\"clip.wav\"\r\nContent-Type: audio/wav\r\n\r\nRIFF\r\n"), "{multipart}");
    assert!(multipart.ends_with(&format!("--{boundary}--\r\n")), "{multipart}");
    assert_eq!(content_type(3), "application/octet-stream");
    assert_eq!(received[3].body, b"\x00raw");
}

/// Sentinel placements: a built-in secret header and query name, an
/// API-marked secret name, an API-key query credential, a secret response
/// header, and a secret query in a `Location` header. `page=2` is not secret.
fn secret_parts() -> RequestParts {
    let mut parts = RequestParts::new(Method::GET, "/item");
    parts.query = vec![("api_key".into(), SENTINEL.into()), ("page".into(), "2".into())];
    parts.headers = vec![("X-Api-Key".into(), SENTINEL.into()), ("X-Vendor-Secret".into(), SENTINEL.into())];
    parts
}

fn secret_definition() -> ApiDefinition {
    ApiDefinition::new("https://unused.invalid")
        .secret_names(["X-Vendor-Secret"])
        .auth(AuthConfig::none().accept(CredentialPlacement::ApiKeyQuery { name: "sig_key".into() }))
}

#[tokio::test]
async fn no_stored_field_or_display_text_holds_a_secret_in_any_outcome() {
    let server = MockServer::start().await;
    for (route, status, body, content_type) in [
        ("/ok", 200_u16, &br#"{"id":1}"#[..], "application/json"),
        ("/declared", 403, &br#"{"message":"denied"}"#[..], "application/json"),
        ("/malformed", 200, &b"<html>"[..], "text/html"),
    ] {
        Mock::given(path(route))
            .respond_with(
                ResponseTemplate::new(status)
                    .set_body_raw(body, content_type)
                    .append_header("set-cookie", format!("session={SENTINEL}").as_str())
                    .append_header("location", format!("/next?token={SENTINEL}&page=3").as_str()),
            )
            .mount(&server)
            .await;
    }
    let client = builder(secret_definition(), &server.uri()).build().expect("client");
    let options = CallOptions::new().credential(Credential::api_key(SENTINEL));

    for route in ["/ok", "/declared", "/malformed"] {
        let mut parts = secret_parts();
        parts.path = route.into();
        let response = client
            .send(Call::<Json<Item>, ProviderError>::new(parts), &options)
            .await
            .expect("head");
        let stored = format!("{:?}", response.meta);
        let shown = response.to_string();
        for text in [&stored, &shown] {
            assert!(!text.contains(SENTINEL), "{route}: {text}");
        }
        if let Some(error) = response.error() {
            assert!(!error.to_string().contains(SENTINEL), "{route}");
        }
        assert!(shown.contains("page=2"), "{route}: {shown}");
        let request = &response.meta.request;
        assert_eq!(request.headers["x-api-key"], REDACTED);
        assert_eq!(request.headers["x-vendor-secret"], REDACTED);
        assert!(request.query.contains(&("sig_key".into(), REDACTED.into())), "{:?}", request.query);
        assert!(request.query.contains(&("api_key".into(), REDACTED.into())));
        assert!(request.query.contains(&("page".into(), "2".into())));
        assert_eq!(request.auth, AuthMethod::PerCall);
        assert_eq!(response.meta.headers["set-cookie"], REDACTED);
        assert_eq!(response.meta.headers["location"], format!("/next?token={REDACTED}&page=3"));
    }

    // The server received the real values: redaction applies to what is stored.
    let received = server.received_requests().await.expect("recorded");
    assert_eq!(received[0].headers["x-api-key"], SENTINEL);
    assert!(received[0].url.query_pairs().any(|(name, value)| name == "sig_key" && value == SENTINEL));

    let error = builder(secret_definition(), &closed_port_url().await)
        .build()
        .expect("client")
        .send(Call::<Json<Item>, ProviderError>::new(secret_parts()), &options)
        .await
        .expect_err("transport failure");
    for text in [format!("{:?}", error.request), error.to_string(), format!("{error:?}")] {
        assert!(!text.contains(SENTINEL), "{text}");
    }
}

#[tokio::test]
async fn a_provider_echo_stays_in_the_raw_body_and_out_of_metadata_and_display() {
    let echo = format!(r#"{{"message":"invalid key {SENTINEL}"}}"#);
    let server = MockServer::start().await;
    Mock::given(path("/item"))
        .respond_with(ResponseTemplate::new(401).set_body_raw(echo.clone(), "application/json"))
        .mount(&server)
        .await;
    let client = builder(secret_definition(), &server.uri()).build().expect("client");
    let response = client
        .send(
            Call::<Json<Item>, ProviderError>::get("/item"),
            &CallOptions::new().credential(Credential::api_key(SENTINEL)),
        )
        .await
        .expect("head");

    let error = response.error().expect("failure");
    assert_eq!(error.body, Bytes::from(echo), "evidence is kept unchanged");
    assert!(!format!("{:?}", response.meta).contains(SENTINEL));
    assert!(!response.to_string().contains(SENTINEL));
    assert!(!error.to_string().contains(SENTINEL));
}
