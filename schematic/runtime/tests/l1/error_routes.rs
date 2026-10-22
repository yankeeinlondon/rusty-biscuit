//! The effective error table at run time (R19): an exact status beats a
//! range and a narrower range beats a wider one whatever the table's order;
//! the matched entry's shape decodes as `Declared`, an undecodable body is
//! `MalformedError`, an unmatched status or an opted-out endpoint is
//! `Undeclared`, and the raw body is kept in every case. Also the runtime's
//! single-shape failure types (text, bytes, empty, any JSON).

use std::marker::PhantomData;

use bytes::Bytes;
use http::Method;
use schematic_runtime::{
    ApiDefinition, Client, EndpointSpec, ErrorBody, ErrorKind, ErrorRoute, InvalidArgument, Json, NoDeclaredError,
    RequestParts, Response, StatusMatch,
};
use serde::Deserialize;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{Call, Item, TestApi, builder};

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Problem {
    detail: String,
}

/// The failure enum of a table with three distinct shapes.
#[derive(Debug, Clone, PartialEq)]
enum Failure {
    Problem(Problem),
    Text(String),
    Throttled,
}

impl ErrorBody for Failure {
    fn decode_body(alternative: usize, body: &[u8]) -> Option<Self> {
        match alternative {
            0 => serde_json::from_slice(body).ok().map(Self::Problem),
            1 => Some(Self::Text(String::from_utf8_lossy(body).into_owned())),
            _ => Some(Self::Throttled),
        }
    }
}

/// An API-level exact 404 (JSON) overlaid by an endpoint's 4xx class (text)
/// and its narrower 420–429 range (empty), listed widest-first so order
/// cannot be what decides.
const TABLE: &[ErrorRoute] = &[
    ErrorRoute::new(StatusMatch::Range { lo: 400, hi: 499 }, 1),
    ErrorRoute::new(StatusMatch::Range { lo: 420, hi: 429 }, 2),
    ErrorRoute::new(StatusMatch::Exact(404), 0),
];

/// A GET of `/{path}` failing with `E` under `ERRORS`.
struct Get<E, T> {
    path: &'static str,
    table: PhantomData<fn() -> (E, T)>,
}

trait Table: 'static {
    const ERRORS: &'static [ErrorRoute];
}

struct Routed;
impl Table for Routed {
    const ERRORS: &'static [ErrorRoute] = TABLE;
}

/// An endpoint that opted out of declared errors.
struct OptedOut;
impl Table for OptedOut {
    const ERRORS: &'static [ErrorRoute] = &[];
}

impl<E: ErrorBody, T: Table> EndpointSpec for Get<E, T> {
    type Api = TestApi;
    type Success = Item;
    type Failure = E;
    type Decoder = Json<Item>;

    const ENDPOINT_ID: &'static str = "get";
    const METHOD: Method = Method::GET;
    const ERRORS: &'static [ErrorRoute] = T::ERRORS;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(Method::GET, self.path))
    }
}

async fn serve(routes: &[(&'static str, u16, &'static str)]) -> MockServer {
    let server = MockServer::start().await;
    for (at, status, body) in routes {
        Mock::given(path(*at))
            .respond_with(ResponseTemplate::new(*status).set_body_raw(*body, "application/json"))
            .mount(&server)
            .await;
    }
    server
}

fn client(server: &MockServer) -> Client<TestApi> {
    builder(ApiDefinition::new("https://api.example.invalid"), &server.uri())
}

async fn send<E: ErrorBody, T: Table>(client: &Client<TestApi>, at: &'static str) -> Response<Item, E> {
    client.send(Get::<E, T> { path: at, table: PhantomData }).full().await.expect("head")
}

fn failure<E: Clone>(response: &Response<Item, E>) -> (ErrorKind<E>, Bytes) {
    let error = response.error().expect("failure");
    (error.kind.clone(), error.body.clone())
}

#[test]
fn lookup_prefers_an_exact_status_then_the_narrowest_range() {
    assert_eq!(ErrorRoute::lookup(TABLE, 404).map(|route| route.alternative), Some(0));
    assert_eq!(ErrorRoute::lookup(TABLE, 425).map(|route| route.alternative), Some(2));
    assert_eq!(ErrorRoute::lookup(TABLE, 400).map(|route| route.alternative), Some(1));
    assert_eq!(ErrorRoute::lookup(TABLE, 499).map(|route| route.alternative), Some(1));
    assert_eq!(ErrorRoute::lookup(TABLE, 500), None);
    assert_eq!(ErrorRoute::lookup(TABLE, 200), None);
    assert_eq!(StatusMatch::Range { lo: 400, hi: 499 }.to_string(), "400-499");
    assert_eq!(StatusMatch::Exact(404).width(), 1);
}

#[tokio::test]
async fn the_matched_entry_decodes_its_own_shape_and_everything_else_keeps_its_body() {
    let server = serve(&[
        ("/missing", 404, r#"{"detail":"no such item"}"#),
        ("/garbled", 404, "not json"),
        ("/bad", 400, "bad request"),
        ("/slow", 429, ""),
        ("/down", 503, r#"{"detail":"down"}"#),
    ])
    .await;
    let client = client(&server);

    let missing = send::<Failure, Routed>(&client, "/missing").await;
    assert_eq!(
        failure(&missing),
        (ErrorKind::Declared(Failure::Problem(Problem { detail: "no such item".into() })), Bytes::from_static(br#"{"detail":"no such item"}"#)),
        "the API's exact 404 beats the endpoint's 4xx range"
    );
    assert_eq!(failure(&send::<Failure, Routed>(&client, "/garbled").await), (ErrorKind::MalformedError, Bytes::from_static(b"not json")));
    assert_eq!(
        failure(&send::<Failure, Routed>(&client, "/bad").await),
        (ErrorKind::Declared(Failure::Text("bad request".into())), Bytes::from_static(b"bad request"))
    );
    assert_eq!(failure(&send::<Failure, Routed>(&client, "/slow").await), (ErrorKind::Declared(Failure::Throttled), Bytes::new()));
    assert_eq!(
        failure(&send::<Failure, Routed>(&client, "/down").await),
        (ErrorKind::Undeclared, Bytes::from_static(br#"{"detail":"down"}"#)),
        "a status no entry matches"
    );

    for at in ["/missing", "/bad", "/down"] {
        let (kind, body) = failure(&send::<NoDeclaredError, OptedOut>(&client, at).await);
        assert_eq!(kind, ErrorKind::Undeclared, "an opted-out endpoint declares nothing: {at}");
        assert!(!body.is_empty(), "{at}");
    }
}

#[tokio::test]
async fn the_runtime_failure_types_decode_text_bytes_empty_and_any_json_under_the_default_table() {
    let server = serve(&[("/text", 500, "oops \u{e9}"), ("/json", 400, r#"{"a":1}"#), ("/not-json", 400, "{")]).await;
    let client = client(&server);
    async fn declared<E: ErrorBody + Clone>(client: &Client<TestApi>, at: &str) -> ErrorKind<E> {
        let response = client.send(Call::<Json<Item>, E>::get(at)).full().await.expect("head");
        failure(&response).0
    }
    assert_eq!(declared::<String>(&client, "/text").await, ErrorKind::Declared("oops \u{e9}".to_string()));
    assert_eq!(declared::<Bytes>(&client, "/text").await, ErrorKind::Declared(Bytes::from("oops \u{e9}")));
    assert_eq!(declared::<()>(&client, "/text").await, ErrorKind::Declared(()));
    assert_eq!(declared::<serde_json::Value>(&client, "/json").await, ErrorKind::Declared(serde_json::json!({"a": 1})));
    assert_eq!(declared::<serde_json::Value>(&client, "/not-json").await, ErrorKind::MalformedError);
}
