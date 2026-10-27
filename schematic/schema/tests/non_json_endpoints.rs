//! Text, binary, and empty endpoints decode through endpoint methods exactly
//! as JSON ones do, and `send_raw`, which stays on request objects, returns a
//! reply without decoding it.
//!
//! Each test uses a real generated endpoint whose definition declares the
//! codec under test: GitHub `GetRepositoryContentRaw` (text), Bitbucket
//! `GetSource`'s `File` binary fallback, and Samsung Smart TV
//! `LaunchApplicationById` (empty).
//! Calls are anonymous so a host credential never reaches the mock.

use bytes::Bytes;
use schematic_schema::bitbucket::{GetSourceRequest, GetSourceResponse};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{Method, RawCompletion};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const SOURCE_PATH: &str = "/repositories/team/app/src/abc123/logo.png";
/// Bytes that are neither UTF-8 nor JSON, so any decoding would alter them.
const PNG_PREFIX: &[u8] = b"\x89PNG\r\n\x1a\n\x00\xff\xfe";

/// The text endpoint sends its own `Accept` header and returns the body as a
/// `String`, whatever content type the server declares.
#[tokio::test]
async fn text_endpoint_decodes_into_string() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/repos/octo/widgets/contents/docs/README.md"))
        .and(header("accept", "application/vnd.github.raw+json"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("# Widgets\n", "application/json"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = GitHub::new()
        .base_url(mock_server.uri())
        .build()
        .expect("client builds");
    let text: String = client
        .get_repository_content_raw("octo", "widgets", "docs/README.md")
        .anonymous()
        .await
        .expect("success");

    assert_eq!(text, "# Widgets\n");
}

/// A binary shape returns the body bytes unchanged as `Bytes`: `GetSource`
/// selects its `File` fallback for a media type no other shape declares.
#[tokio::test]
async fn binary_endpoint_decodes_into_bytes() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SOURCE_PATH))
        .and(query_param("pagelen", "10"))
        .respond_with(ResponseTemplate::new(200).set_body_raw(PNG_PREFIX, "image/png"))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = Bitbucket::new()
        .base_url(mock_server.uri())
        .build()
        .expect("client builds");
    let reply = client
        .get_source("team", "app", "abc123", "logo.png")
        .pagelen(10)
        .anonymous()
        .await
        .expect("success");

    let GetSourceResponse::File(body) = reply else { panic!("an image/png body is the file, not a listing: {reply:?}") };
    let body: Bytes = body;
    assert_eq!(body.as_ref(), PNG_PREFIX);
}

/// The empty endpoint succeeds with `()` and reads past any body it gets,
/// both awaited directly and through `.full()`.
#[tokio::test]
async fn empty_endpoint_decodes_into_unit() {
    let mock_server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/v2/applications/111299001912"))
        .respond_with(ResponseTemplate::new(200).set_body_raw("true", "application/json"))
        .expect(2)
        .mount(&mock_server)
        .await;

    let client = SamsungSmartTv::new()
        .base_url(mock_server.uri())
        .build()
        .expect("client builds");
    let () = client
        .launch_application_by_id("111299001912")
        .anonymous()
        .await
        .expect("success");
    let response = client
        .launch_application_by_id("111299001912")
        .anonymous()
        .full()
        .await
        .expect("a response head arrived");

    assert_eq!(response.meta.request.method, Method::POST);
    assert_eq!(response.meta.request.endpoint_id, "LaunchApplicationById");
    let () = response.into_result().expect("success");
}

/// `send_raw` returns status, headers, and body unchanged, for a success and
/// for an error status alike.
#[tokio::test]
async fn send_raw_returns_status_headers_and_body_unchanged() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(SOURCE_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-request-id", "raw-1")
                .set_body_raw(PNG_PREFIX, "image/png"),
        )
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path("/repositories/team/app/src/abc123/missing.txt"))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("x-request-id", "raw-2")
                .set_body_raw(r#"{"type":"error"}"#, "application/json"),
        )
        .mount(&mock_server)
        .await;

    let client = Bitbucket::new()
        .base_url(mock_server.uri())
        .build()
        .expect("client builds");
    let ok = client
        .send_raw(
            GetSourceRequest::new("team", "app", "abc123", "logo.png"),
            &CallOptions::anonymous(),
        )
        .await
        .expect("a response head arrived");
    let missing = client
        .send_raw(
            GetSourceRequest::new("team", "app", "abc123", "missing.txt"),
            &CallOptions::anonymous(),
        )
        .await
        .expect("a response head arrived");

    assert_eq!(ok.status().as_u16(), 200);
    assert_eq!(
        ok.headers().get("x-request-id").map(|v| v.as_bytes()),
        Some(b"raw-1".as_slice())
    );
    assert_eq!(
        ok.headers().get("content-type").map(|v| v.as_bytes()),
        Some(b"image/png".as_slice())
    );
    assert_eq!(ok.body.as_ref(), PNG_PREFIX);
    assert_eq!(ok.completion, RawCompletion::Complete);

    assert_eq!(missing.status().as_u16(), 404);
    assert_eq!(
        missing.headers().get("x-request-id").map(|v| v.as_bytes()),
        Some(b"raw-2".as_slice())
    );
    assert_eq!(missing.body.as_ref(), br#"{"type":"error"}"#.as_slice());
    assert!(missing.is_complete());
}
