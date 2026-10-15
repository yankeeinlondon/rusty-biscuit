//! Elapsed time covers the whole logical send (R4, R27): a buffered reply's
//! `meta.elapsed` includes the wait for its body, through decoding, error
//! classification, hooks, interruption, and stopped or followed redirects,
//! while a streaming output still returns at about the time of the head.
//!
//! Every reply's head arrives at once and its body [`DELAY`] later, so an
//! `elapsed` stopped at the head reads well under [`FLOOR`].

use std::time::Duration;

use schematic_runtime::{
    ApiDefinition, Binary, CallOptions, Client, Decode, Empty, ErrorBody, ErrorKind, HookError, Json, NoDeclaredError, RawCompletion,
    RawResponse, RedirectRefusal, Response, Text,
};
use tokio::net::TcpStream;
use tokio::time::Instant;

use crate::support::{AbortOnDrop, Call, Item, ProviderError, TestApi, builder, listener, read_head, write_all};
use crate::timing::Lines;

const DELAY: Duration = Duration::from_millis(80);
/// The least `elapsed` that proves the body wait was counted, with margin
/// for timer granularity.
const FLOOR: Duration = Duration::from_millis(70);

/// How a route answers: `status` and extra header lines at once, then
/// `body` after [`DELAY`] unless `immediate`, advertising 100 more bytes
/// than it sends when `cut_off`.
struct Route {
    path: &'static str,
    status: &'static str,
    headers: &'static str,
    body: &'static str,
    cut_off: bool,
    immediate: bool,
}

const fn route(path: &'static str, status: &'static str, headers: &'static str, body: &'static str) -> Route {
    Route {
        path,
        status,
        headers,
        body,
        cut_off: false,
        immediate: false,
    }
}

const ROUTES: &[Route] = &[
    route("/json", "200 OK", "", r#"{"id":1}"#),
    route("/text", "200 OK", "", "hello"),
    route("/drain", "200 OK", "", "drain me"),
    route("/declared", "400 Bad Request", "", r#"{"message":"no"}"#),
    route("/server-error", "500 Internal Server Error", "", "oops"),
    route("/not-json", "200 OK", "", "not json"),
    route("/declared-not-json", "400 Bad Request", "", "not json"),
    route("/lines", "200 OK", "", "a\nb\n"),
    Route {
        cut_off: true,
        ..route("/cut-off", "200 OK", "", r#"{"id":1"#)
    },
    route("/cross-origin", "302 Found", "Location: http://192.0.2.1/elsewhere\r\n", "moved"),
    route("/no-location", "302 Found", "", "moved"),
    route("/unsafe-location", "302 Found", "Location: ftp://example.invalid/file\r\n", "moved"),
    Route {
        cut_off: true,
        ..route("/redirect-cut-off", "302 Found", "Location: /json\r\n", "mov")
    },
    Route {
        immediate: true,
        ..route("/follow-json", "302 Found", "Location: /json\r\n", "")
    },
];

/// Serves [`ROUTES`] on loopback, each connection on its own task, and a
/// client for it with `configure` applied.
async fn serve_with(configure: impl FnOnce(Client<TestApi>) -> Client<TestApi>) -> (Client<TestApi>, AbortOnDrop) {
    let (listener, url) = listener().await;
    let task = tokio::spawn(async move {
        while let Ok((stream, _)) = listener.accept().await {
            tokio::spawn(answer(stream));
        }
    });
    let client = configure(builder(ApiDefinition::new("https://api.example.invalid"), &url).timeout(Duration::from_secs(5)))
        .build()
        .expect("client builds");
    (client, AbortOnDrop(task))
}

async fn serve() -> (Client<TestApi>, AbortOnDrop) {
    serve_with(|client| client).await
}

async fn answer(mut stream: TcpStream) {
    let head = read_head(&mut stream).await;
    let path = head.split_whitespace().nth(1).unwrap_or_default();
    let route = ROUTES.iter().find(|route| route.path == path).expect("a known route");
    let advertised = route.body.len() + if route.cut_off { 100 } else { 0 };
    let response_head = format!(
        "HTTP/1.1 {}\r\n{}Content-Length: {advertised}\r\nConnection: close\r\n\r\n",
        route.status, route.headers
    );
    write_all(&mut stream, response_head.as_bytes()).await;
    if !route.immediate {
        tokio::time::sleep(DELAY).await;
    }
    write_all(&mut stream, route.body.as_bytes()).await;
}

async fn typed<D: Decode, E: ErrorBody>(client: &Client<TestApi>, path: &str) -> Response<D::Output, E> {
    client.send(Call::<D, E>::get(path)).full().await.expect("a head arrived")
}

async fn raw(client: &Client<TestApi>, path: &str) -> RawResponse {
    client.send_raw(Call::<Binary>::get(path), &CallOptions::new()).await.expect("a head arrived")
}

#[track_caller]
fn assert_body_wait_counted(elapsed: Duration) {
    assert!(elapsed >= FLOOR, "elapsed {elapsed:?} stopped before the body arrived");
    assert!(elapsed < Duration::from_secs(5), "{elapsed:?}");
}

fn refusal<T, E>(response: &Response<T, E>) -> Option<RedirectRefusal> {
    match response.error()?.kind {
        ErrorKind::RedirectRefused { reason } => Some(reason),
        _ => None,
    }
}

#[tokio::test]
async fn typed_buffered_successes_count_the_body_read() {
    let (client, _server) = serve().await;

    let json = typed::<Json<Item>, ProviderError>(&client, "/json").await;
    assert_eq!(json.data(), Some(&Item { id: 1 }));
    assert_body_wait_counted(json.meta.elapsed);

    let text = typed::<Text, ProviderError>(&client, "/text").await;
    assert_eq!(text.data().map(String::as_str), Some("hello"));
    assert_body_wait_counted(text.meta.elapsed);

    let binary = typed::<Binary, ProviderError>(&client, "/text").await;
    assert_eq!(binary.data().map(|bytes| &bytes[..]), Some(&b"hello"[..]));
    assert_body_wait_counted(binary.meta.elapsed);

    let empty = typed::<Empty, ProviderError>(&client, "/drain").await;
    assert_eq!(empty.data(), Some(&()));
    assert_body_wait_counted(empty.meta.elapsed);
}

#[tokio::test]
async fn typed_failures_count_the_body_read_and_classification() {
    let (client, _server) = serve().await;

    let declared = typed::<Json<Item>, ProviderError>(&client, "/declared").await;
    assert_eq!(
        declared.error().map(|error| &error.kind),
        Some(&ErrorKind::Declared(ProviderError { message: "no".into() }))
    );
    assert_body_wait_counted(declared.meta.elapsed);

    let undeclared = typed::<Json<Item>, NoDeclaredError>(&client, "/server-error").await;
    assert_eq!(undeclared.error().map(|error| &error.kind), Some(&ErrorKind::Undeclared));
    assert_body_wait_counted(undeclared.meta.elapsed);

    let malformed = typed::<Json<Item>, ProviderError>(&client, "/not-json").await;
    assert_eq!(malformed.error().map(|error| &error.kind), Some(&ErrorKind::MalformedSuccess));
    assert_body_wait_counted(malformed.meta.elapsed);

    let malformed_error = typed::<Json<Item>, ProviderError>(&client, "/declared-not-json").await;
    assert_eq!(malformed_error.error().map(|error| &error.kind), Some(&ErrorKind::MalformedError));
    assert_body_wait_counted(malformed_error.meta.elapsed);

    let interrupted = typed::<Json<Item>, ProviderError>(&client, "/cut-off").await;
    assert!(interrupted.is_interrupted());
    assert_body_wait_counted(interrupted.meta.elapsed);
}

#[tokio::test]
async fn a_hook_rejection_counts_the_body_read() {
    let (client, _server) = serve_with(|client| client.pre_response_json(|_, _| Err(HookError::new()))).await;

    let response = typed::<Json<Item>, ProviderError>(&client, "/json").await;
    assert_eq!(response.error().map(|error| &error.kind), Some(&ErrorKind::Hook));
    assert_body_wait_counted(response.meta.elapsed);
}

#[tokio::test]
async fn stopped_redirects_count_the_redirect_body_read() {
    let (client, _server) = serve().await;

    for (path, reason) in [
        ("/cross-origin", RedirectRefusal::CrossOrigin),
        ("/no-location", RedirectRefusal::MissingLocation),
        ("/unsafe-location", RedirectRefusal::UnsafeLocation),
    ] {
        let response = typed::<Json<Item>, ProviderError>(&client, path).await;
        assert_eq!(refusal(&response), Some(reason), "{path}");
        assert_body_wait_counted(response.meta.elapsed);
    }

    let interrupted = typed::<Json<Item>, ProviderError>(&client, "/redirect-cut-off").await;
    assert_eq!(interrupted.meta.status, 302);
    assert!(interrupted.is_interrupted());
    assert_body_wait_counted(interrupted.meta.elapsed);
}

#[tokio::test]
async fn raw_replies_count_the_body_read() {
    let (client, _server) = serve().await;

    for (path, status, completion) in [
        ("/json", 200, RawCompletion::Complete),
        ("/server-error", 500, RawCompletion::Complete),
        ("/cut-off", 200, RawCompletion::BodyInterrupted),
        (
            "/cross-origin",
            302,
            RawCompletion::RedirectRefused {
                reason: RedirectRefusal::CrossOrigin,
            },
        ),
    ] {
        let response = raw(&client, path).await;
        assert_eq!(response.status(), status, "{path}");
        assert_eq!(response.completion, completion, "{path}");
        assert_body_wait_counted(response.meta.elapsed);
    }
}

#[tokio::test]
async fn a_followed_redirect_counts_the_final_body_read() {
    let (client, _server) = serve().await;

    let typed = typed::<Json<Item>, ProviderError>(&client, "/follow-json").await;
    assert_eq!(typed.data(), Some(&Item { id: 1 }));
    assert_eq!(typed.meta.request.url.as_str().rsplit('/').next(), Some("json"));
    assert_body_wait_counted(typed.meta.elapsed);

    let raw = raw(&client, "/follow-json").await;
    assert_eq!(raw.status(), 200);
    assert_eq!(&raw.body[..], br#"{"id":1}"#);
    assert_body_wait_counted(raw.meta.elapsed);
}

#[tokio::test]
async fn a_streaming_output_returns_with_about_the_time_to_the_head() {
    let (client, _server) = serve().await;

    let response = typed::<Lines, ProviderError>(&client, "/lines").await;
    assert!(
        response.meta.elapsed < FLOOR,
        "a stream's elapsed {:?} waited for its body",
        response.meta.elapsed
    );

    let reading = Instant::now();
    let mut lines = response.into_result().unwrap_or_else(|error| panic!("expected a stream, got {error}"));
    let first = lines.next_line().await.expect("a line").expect("read");
    assert_eq!(first, "a");
    assert!(reading.elapsed() >= FLOOR, "the body was still on its way: {:?}", reading.elapsed());
}
