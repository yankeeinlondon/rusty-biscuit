//! Multi-shape success selection through the managed send's envelope (R3,
//! R3a, R3b, R3c): 204 to the empty shape, media type with case and
//! parameters ignored, declared statuses, discriminator and array/object
//! tags, the fallback, `UnexpectedContentType` with its evidence, the
//! single-shape compatibility rule (R3, R4), and the hooks a multi-shape
//! codec runs (R12). The compile error for a mutator on mixed alternatives is
//! a trybuild fixture (`compile_fail.rs`).

use std::sync::{Arc, Mutex};

use bytes::Bytes;
use schematic_runtime::{
    Alternatives, ApiDefinition, Binary, Client, Decode, DecodeFailure, Empty, ErrorKind, HookError, ItemStream, Json,
    JsonAlternatives, JsonTag, LiveBody, NoDeclaredError, Response, Route, Scalar, ShapeKind, Shapes, Sse, Text,
    UntilClose,
};
use serde::Deserialize;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Call, Item, TestApi, builder};

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Job {
    job: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Delta {
    text: String,
}

/// JSON, an SSE stream, and an empty reply.
#[derive(Debug)]
enum Mixed {
    Record(Item),
    Events(ItemStream<Delta>),
    Nothing,
}

impl Alternatives for Mixed {
    const ROUTES: &'static [Route] = &[
        Route::new(ShapeKind::Json).media("application/json"),
        Route::new(ShapeKind::Stream).media("text/event-stream"),
        Route::new(ShapeKind::Empty),
    ];

    async fn decode_alternative(index: usize, body: LiveBody) -> Result<Self, DecodeFailure> {
        match index {
            0 => Json::<Item>::decode(body).await.map(Self::Record),
            1 => Sse::<Delta, UntilClose>::decode(body).await.map(Self::Events),
            _ => Empty::decode(body).await.map(|()| Self::Nothing),
        }
    }
}

/// A 200 record and a 202 job receipt, both JSON, and a status-less note.
#[derive(Debug, PartialEq)]
enum ByStatus {
    Record(Item),
    Accepted(Job),
    Note(String),
}

impl Alternatives for ByStatus {
    const ROUTES: &'static [Route] = &[
        Route::new(ShapeKind::Json).media("application/json").statuses(&[200]),
        Route::new(ShapeKind::Json).media("application/json").statuses(&[202]),
        Route::new(ShapeKind::Text).media("text/plain"),
    ];

    async fn decode_alternative(index: usize, body: LiveBody) -> Result<Self, DecodeFailure> {
        match index {
            0 => Json::<Item>::decode(body).await.map(Self::Record),
            1 => Json::<Job>::decode(body).await.map(Self::Accepted),
            _ => Text::decode(body).await.map(Self::Note),
        }
    }
}

impl JsonAlternatives for AllJson {}

/// Two JSON shapes chosen by status: hook-eligible.
#[derive(Debug, PartialEq)]
enum AllJson {
    Record(Item),
    Accepted(Job),
}

impl Alternatives for AllJson {
    const ROUTES: &'static [Route] = &[
        Route::new(ShapeKind::Json).media("application/json").statuses(&[200]),
        Route::new(ShapeKind::Json).media("application/json").statuses(&[202]),
    ];

    async fn decode_alternative(index: usize, body: LiveBody) -> Result<Self, DecodeFailure> {
        match index {
            0 => Json::<Item>::decode(body).await.map(Self::Record),
            _ => Json::<Job>::decode(body).await.map(Self::Accepted),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct FileEntry {
    path: String,
    size: u64,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct DirectoryEntry {
    path: String,
}

/// Bitbucket-style entries told apart by their `type` field.
#[derive(Debug, PartialEq)]
enum Discriminated {
    File(FileEntry),
    Directory(DirectoryEntry),
}

impl Alternatives for Discriminated {
    const ROUTES: &'static [Route] = &[
        Route::new(ShapeKind::Json).media("application/json").tag(JsonTag::Field { name: "type", value: Scalar::String("commit_file") }),
        Route::new(ShapeKind::Json).media("application/json").tag(JsonTag::Field { name: "type", value: Scalar::String("commit_directory") }),
    ];

    async fn decode_alternative(index: usize, body: LiveBody) -> Result<Self, DecodeFailure> {
        match index {
            0 => Json::<FileEntry>::decode(body).await.map(Self::File),
            _ => Json::<DirectoryEntry>::decode(body).await.map(Self::Directory),
        }
    }
}

/// GitHub-contents style: an array for a directory, an object for a file.
#[derive(Debug, PartialEq)]
enum Listing {
    Many(Vec<Item>),
    One(Item),
}

impl Alternatives for Listing {
    const ROUTES: &'static [Route] = &[
        Route::new(ShapeKind::Json).media("application/json").tag(JsonTag::Array),
        Route::new(ShapeKind::Json).media("application/json").tag(JsonTag::Object),
    ];

    async fn decode_alternative(index: usize, body: LiveBody) -> Result<Self, DecodeFailure> {
        match index {
            0 => Json::<Vec<Item>>::decode(body).await.map(Self::Many),
            _ => Json::<Item>::decode(body).await.map(Self::One),
        }
    }
}

/// JSON, or any other media type as bytes.
#[derive(Debug, PartialEq)]
enum WithFallback {
    Record(Item),
    File(Bytes),
}

impl Alternatives for WithFallback {
    const ROUTES: &'static [Route] = &[
        Route::new(ShapeKind::Json).media("application/json"),
        Route::new(ShapeKind::Binary).fallback(),
    ];

    async fn decode_alternative(index: usize, body: LiveBody) -> Result<Self, DecodeFailure> {
        match index {
            0 => Json::<Item>::decode(body).await.map(Self::Record),
            _ => Binary::decode(body).await.map(Self::File),
        }
    }
}

/// A route: path, status, body, and the `Content-Type` values to send (none
/// for an absent header; two for a repeated one).
type Reply = (&'static str, u16, &'static str, &'static [&'static str]);

async fn serve(routes: &[Reply]) -> MockServer {
    let server = MockServer::start().await;
    for (at, status, body, content_types) in routes {
        let mut template = ResponseTemplate::new(*status).set_body_bytes(body.as_bytes());
        for content_type in *content_types {
            template = template.append_header("content-type", *content_type);
        }
        Mock::given(path(*at)).respond_with(template).mount(&server).await;
    }
    server
}

fn client(server: &MockServer) -> Client<TestApi> {
    builder(ApiDefinition::new("https://api.example.invalid"), &server.uri())
}

async fn send<A: Alternatives>(client: &Client<TestApi>, at: &str) -> Response<A, NoDeclaredError> {
    client.send(Call::<Shapes<A>>::get(at)).full().await.expect("head")
}

/// Asserts `response` is `UnexpectedContentType` keeping `status` and `body`.
fn assert_unexpected<A: std::fmt::Debug>(response: &Response<A, NoDeclaredError>, status: u16, body: &str) {
    assert_eq!(response.meta.status, status, "{response:?}");
    let error = response.error().unwrap_or_else(|| panic!("expected a failure, got {:?}", response.data()));
    assert_eq!(error.kind, ErrorKind::UnexpectedContentType, "{body:?}");
    assert_eq!(error.body, Bytes::from(body.to_string()), "the raw body is kept");
}

#[tokio::test]
async fn json_and_a_stream_are_chosen_by_media_type_ignoring_case_and_parameters() {
    let server = serve(&[
        ("/json", 200, r#"{"id":1}"#, &["Application/JSON; charset=utf-8"]),
        ("/twice", 200, r#"{"id":2}"#, &["application/json", "application/json; charset=utf-8"]),
        ("/sse", 200, "data: {\"text\":\"a\"}\n\n", &["text/event-stream;charset=UTF-8"]),
        ("/none", 204, "", &[]),
    ])
    .await;
    let client = client(&server);

    let json = send::<Mixed>(&client, "/json").await;
    assert!(matches!(json.data(), Some(Mixed::Record(Item { id: 1 }))), "{json:?}");
    let twice = send::<Mixed>(&client, "/twice").await;
    assert!(matches!(twice.data(), Some(Mixed::Record(Item { id: 2 }))), "the same media type twice agrees: {twice:?}");

    let Ok(Mixed::Events(mut events)) = send::<Mixed>(&client, "/sse").await.into_result() else {
        panic!("expected the stream shape");
    };
    assert_eq!(events.next().await, Some(Ok(Delta { text: "a".into() })));
    assert_eq!(events.next().await, None);

    let none = send::<Mixed>(&client, "/none").await;
    assert!(matches!(none.data(), Some(Mixed::Nothing)), "204 without a Content-Type selects the empty shape: {none:?}");
}

#[tokio::test]
async fn missing_invalid_conflicting_or_unmatched_content_types_are_unexpected_with_evidence() {
    let body = r#"{"id":1}"#;
    let server = serve(&[
        ("/missing", 200, body, &[]),
        ("/invalid", 200, body, &["application"]),
        ("/wildcard", 200, body, &["*/*"]),
        ("/conflicting", 200, body, &["application/json", "text/event-stream"]),
        ("/unmatched", 200, body, &["text/html"]),
        ("/empty-on-200", 200, "", &[]),
        ("/json-on-204", 204, "", &["application/json"]),
    ])
    .await;
    let client = client(&server);
    for at in ["/missing", "/invalid", "/wildcard", "/conflicting", "/unmatched"] {
        let response = send::<Mixed>(&client, at).await;
        assert_unexpected(&response, 200, body);
        if at == "/unmatched" {
            assert_eq!(response.meta.headers["content-type"], "text/html", "the headers are kept");
        }
    }
    assert_unexpected(&send::<Mixed>(&client, "/empty-on-200").await, 200, "");
    // The empty shape is chosen by 204 whatever the header says.
    let json_on_204 = send::<Mixed>(&client, "/json-on-204").await;
    assert!(matches!(json_on_204.data(), Some(Mixed::Nothing)), "{json_on_204:?}");
    // With no empty shape, 204 matches nothing.
    assert_unexpected(&send::<ByStatus>(&client, "/json-on-204").await, 204, "");
}

#[tokio::test]
async fn declared_statuses_choose_between_json_shapes_and_others_fall_to_status_less_shapes() {
    let server = serve(&[
        ("/record", 200, r#"{"id":7}"#, &["application/json"]),
        ("/job", 202, r#"{"job":"j-1"}"#, &["application/json"]),
        ("/note", 201, "noted", &["text/plain"]),
        ("/json-201", 201, r#"{"id":7}"#, &["application/json"]),
        ("/text-200", 200, "noted", &["text/plain"]),
        ("/bad-job", 202, r#"{"id":7}"#, &["application/json"]),
    ])
    .await;
    let client = client(&server);
    assert_eq!(send::<ByStatus>(&client, "/record").await.into_result().ok(), Some(ByStatus::Record(Item { id: 7 })));
    assert_eq!(send::<ByStatus>(&client, "/job").await.into_result().ok(), Some(ByStatus::Accepted(Job { job: "j-1".into() })));
    assert_eq!(send::<ByStatus>(&client, "/note").await.into_result().ok(), Some(ByStatus::Note("noted".into())));
    assert_unexpected(&send::<ByStatus>(&client, "/json-201").await, 201, r#"{"id":7}"#);
    assert_unexpected(&send::<ByStatus>(&client, "/text-200").await, 200, "noted");

    let bad = send::<ByStatus>(&client, "/bad-job").await;
    let error = bad.error().expect("failure");
    assert_eq!(error.kind, ErrorKind::MalformedSuccess, "a chosen shape that does not decode is malformed");
    assert_eq!(error.body, Bytes::from_static(br#"{"id":7}"#));
}

#[tokio::test]
async fn json_tags_choose_by_discriminator_or_by_array_versus_object() {
    let server = serve(&[
        ("/file", 200, r#"{"type":"commit_file","path":"a.rs","size":3}"#, &["application/json"]),
        ("/dir", 200, r#"{"path":"src","type":"commit_directory"}"#, &["application/json"]),
        ("/other", 200, r#"{"type":"commit_link","path":"x"}"#, &["application/json"]),
        ("/untyped", 200, r#"{"path":"x"}"#, &["application/json"]),
        ("/not-json", 200, "<html>", &["application/json"]),
        ("/bad-file", 200, r#"{"type":"commit_file","path":"a.rs"}"#, &["application/json"]),
        ("/array", 200, r#"  [{"id":1},{"id":2}]"#, &["application/json"]),
        ("/object", 200, "\n{\"id\":3}", &["application/json"]),
        ("/scalar", 200, r#""text""#, &["application/json"]),
    ])
    .await;
    let client = client(&server);
    assert_eq!(
        send::<Discriminated>(&client, "/file").await.into_result().ok(),
        Some(Discriminated::File(FileEntry { path: "a.rs".into(), size: 3 }))
    );
    assert_eq!(
        send::<Discriminated>(&client, "/dir").await.into_result().ok(),
        Some(Discriminated::Directory(DirectoryEntry { path: "src".into() }))
    );
    assert_unexpected(&send::<Discriminated>(&client, "/other").await, 200, r#"{"type":"commit_link","path":"x"}"#);
    assert_unexpected(&send::<Discriminated>(&client, "/untyped").await, 200, r#"{"path":"x"}"#);
    assert_unexpected(&send::<Discriminated>(&client, "/not-json").await, 200, "<html>");
    let bad = send::<Discriminated>(&client, "/bad-file").await;
    assert_eq!(bad.error().map(|error| &error.kind), Some(&ErrorKind::MalformedSuccess), "{bad:?}");

    assert_eq!(
        send::<Listing>(&client, "/array").await.into_result().ok(),
        Some(Listing::Many(vec![Item { id: 1 }, Item { id: 2 }]))
    );
    assert_eq!(send::<Listing>(&client, "/object").await.into_result().ok(), Some(Listing::One(Item { id: 3 })));
    assert_unexpected(&send::<Listing>(&client, "/scalar").await, 200, r#""text""#);
}

#[tokio::test]
async fn a_fallback_takes_a_media_type_no_other_shape_matches() {
    let server = serve(&[
        ("/png", 200, "\u{89}PNG", &["image/png"]),
        ("/json", 200, r#"{"id":5}"#, &["application/json"]),
        ("/missing", 200, "bytes", &[]),
    ])
    .await;
    let client = client(&server);
    assert_eq!(
        send::<WithFallback>(&client, "/png").await.into_result().ok(),
        Some(WithFallback::File(Bytes::from("\u{89}PNG")))
    );
    assert_eq!(send::<WithFallback>(&client, "/json").await.into_result().ok(), Some(WithFallback::Record(Item { id: 5 })));
    // A missing Content-Type is not a media type, so not even the fallback takes it.
    assert_unexpected(&send::<WithFallback>(&client, "/missing").await, 200, "bytes");
}

#[tokio::test]
async fn a_single_shape_endpoint_never_checks_the_content_type() {
    let server = serve(&[
        ("/html-json", 200, r#"{"id":9}"#, &["text/html"]),
        ("/audio", 200, "ID3", &["application/json"]),
        ("/broken", 200, "{", &["text/html"]),
    ])
    .await;
    let client = client(&server);
    let json = client.send(Call::<Json<Item>>::get("/html-json")).full().await.expect("head");
    assert_eq!(json.data(), Some(&Item { id: 9 }));
    let audio = client.send(Call::<Binary>::get("/audio")).full().await.expect("head");
    assert_eq!(audio.data(), Some(&Bytes::from_static(b"ID3")), "a declared media type does not reject a different one");
    let broken = client.send(Call::<Json<Item>>::get("/broken")).full().await.expect("head");
    assert_eq!(broken.error().map(|error| &error.kind), Some(&ErrorKind::MalformedSuccess));
}

/// Records each global hook call and unwraps a `{"data": …}` wrapper.
fn unwrapping(log: &Arc<Mutex<Vec<String>>>) -> impl Fn(&schematic_runtime::ResponseMeta, serde_json::Value) -> Result<serde_json::Value, HookError> + use<> {
    let log = Arc::clone(log);
    move |meta, json| {
        log.lock().expect("log").push(format!("global {}", meta.status.as_u16()));
        Ok(json.get("data").cloned().unwrap_or(json))
    }
}

#[tokio::test]
async fn the_global_hook_runs_only_when_the_chosen_shape_is_json() {
    let server = serve(&[
        ("/json", 200, r#"{"data":{"id":1}}"#, &["application/json"]),
        ("/sse", 200, "data: {\"text\":\"a\"}\n\n", &["text/event-stream"]),
        ("/none", 204, "", &[]),
        ("/html", 200, r#"{"id":1}"#, &["text/html"]),
    ])
    .await;
    let log = Arc::default();
    let client = client(&server).pre_response_json(unwrapping(&log));
    let json = send::<Mixed>(&client, "/json").await;
    assert!(matches!(json.data(), Some(Mixed::Record(Item { id: 1 }))), "the hook's output was decoded: {json:?}");
    assert!(matches!(send::<Mixed>(&client, "/sse").await.data(), Some(Mixed::Events(_))));
    assert!(matches!(send::<Mixed>(&client, "/none").await.data(), Some(Mixed::Nothing)));
    assert_unexpected(&send::<Mixed>(&client, "/html").await, 200, r#"{"id":1}"#);
    assert_eq!(*log.lock().expect("log"), ["global 200"], "only the JSON shape ran the hook");
}

#[tokio::test]
async fn an_all_json_enum_runs_a_typed_mutator_after_the_global_hook_and_keeps_original_bytes_on_refusal() {
    let server = serve(&[
        ("/record", 200, r#"{"data":{"id":1}}"#, &["application/json"]),
        ("/job", 202, r#"{"job":"j"}"#, &["application/json"]),
        ("/refused", 202, r#"{"job":"refuse"}"#, &["application/json"]),
        ("/html", 200, r#"{"id":1}"#, &["text/html"]),
    ])
    .await;
    let log: Arc<Mutex<Vec<String>>> = Arc::default();
    let mutator_log = Arc::clone(&log);
    let client = client(&server).pre_response_json(unwrapping(&log)).mutate_response::<Call<Shapes<AllJson>>, _>(move |_, value| {
        mutator_log.lock().expect("log").push(format!("mutator {value:?}"));
        match value {
            AllJson::Record(item) => item.id += 10,
            AllJson::Accepted(job) if job.job == "refuse" => return Err(HookError::new()),
            AllJson::Accepted(job) => job.job.push('!'),
        }
        Ok(())
    });

    assert_eq!(send::<AllJson>(&client, "/record").await.into_result().ok(), Some(AllJson::Record(Item { id: 11 })));
    assert_eq!(send::<AllJson>(&client, "/job").await.into_result().ok(), Some(AllJson::Accepted(Job { job: "j!".into() })));
    let refused = send::<AllJson>(&client, "/refused").await;
    let error = refused.error().expect("failure");
    assert_eq!(error.kind, ErrorKind::Hook);
    assert_eq!(error.body, Bytes::from_static(br#"{"job":"refuse"}"#), "the original bytes");
    assert_unexpected(&send::<AllJson>(&client, "/html").await, 200, r#"{"id":1}"#);
    assert_eq!(
        *log.lock().expect("log"),
        [
            "global 200",
            "mutator Record(Item { id: 1 })",
            "global 202",
            "mutator Accepted(Job { job: \"j\" })",
            "global 202",
            "mutator Accepted(Job { job: \"refuse\" })",
        ],
        "no hook runs on a reply that matches no shape"
    );
}
