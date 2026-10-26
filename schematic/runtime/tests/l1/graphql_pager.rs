//! Relay pagination of a GraphQL operation (R40): the pager writes its cursor
//! and page size into the serialized `variables`, reads items, `endCursor`,
//! and `hasNextPage` relative to the reply's `data`, and treats a reply with
//! GraphQL errors as a declared failure that stops collection without adding
//! its partial data. The operation below is what `schematic-gen` emits for
//! `Relay::forward("repository.issues").max_page_size(50)`.

use std::time::Duration;

use http::Method;
use schematic_runtime::graphql::{page_data, request_body};
use schematic_runtime::pager::spec::{Boundary, CursorLeg, CursorPart, CursorSource, Input, Kind, MoreSignal, PageSize, Polarity, Source, Step};
use schematic_runtime::pager::{CursorKind, IncompleteReason, ItemSelector, NoSession, PageRootError, PageSizeKnob, Paginated, PagerError, ResumeAfter, Strategy};
use schematic_runtime::{
    ApiDefinition, Client, Deadline, EndpointSpec, ErrorKind, ErrorRoute, GraphQl, GraphQlOperationError, InvalidArgument,
    NoDeclaredError, RequestParts,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

use crate::pager_support::Def;
use crate::support::{TestApi, builder};

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Issue {
    number: u64,
}

/// `data` of the issues query; only its shape matters to the pager.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct IssuesData {
    repository: Option<Value>,
}

/// Pager-managed variables are optional. `after` serializes `null` until
/// the pager writes it; `first` is omitted unless set.
#[derive(Debug, Clone, Serialize)]
struct IssuesVars {
    owner: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    first: Option<u32>,
    after: Option<String>,
}

#[derive(Debug, Clone)]
struct Issues {
    variables: Value,
}

impl Issues {
    fn new(variables: impl Serialize) -> Self {
        Self { variables: serde_json::to_value(variables).unwrap() }
    }
}

const DOCUMENT: &str = "query Issues($owner: String!, $first: Int, $after: String) { repository(owner: $owner) { issues(first: $first, after: $after) { nodes { number } pageInfo { endCursor hasNextPage } } } }";

impl EndpointSpec for Issues {
    type Api = TestApi;
    type Success = IssuesData;
    type Failure = GraphQlOperationError<IssuesData, NoDeclaredError>;
    type Decoder = GraphQl<IssuesData>;

    const ENDPOINT_ID: &'static str = "Issues";
    const METHOD: Method = Method::POST;
    const ERRORS: &'static [ErrorRoute] = &[];

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        let mut parts = RequestParts::new(Method::POST, "/graphql");
        parts.body = request_body(DOCUMENT, Some(&self.variables), None)?;
        Ok(parts)
    }
}

const ROOT: &[&str] = &["repository", "issues"];

impl Paginated for Issues {
    type Item = Issue;
    type Kind = CursorKind;
    type Session = NoSession;

    const STRATEGY: &'static Strategy = &Def::new(Kind::Cursor {
        forward: CursorLeg {
            parts: &[CursorPart {
                param: Input::Body(&["variables", "after"]),
                source: CursorSource::Field(Source::Body(&["repository", "issues", "pageInfo", "endCursor"])),
            }],
            boundary: Boundary::Exclusive,
        },
        backward: None,
        more: Some(MoreSignal::Flag {
            source: Source::Body(&["repository", "issues", "pageInfo", "hasNextPage"]),
            polarity: Polarity::TrueMeansMore,
        }),
        resumable: true,
    })
    .size(PageSize { param: Input::Body(&["variables", "first"]), server_default: None, sent_default: None, min: 1, max: Some(50), adjustable: false })
    .done();
    const ITEMS: &'static ItemSelector = &ItemSelector {
        record: &[Step::Key(ROOT[0]), Step::Key(ROOT[1]), Step::Key("nodes"), Step::Each],
        projection: &[],
    };
    const PATH: &'static str = "";

    fn page_root(body: Value) -> Result<Value, PageRootError<Self::Failure>> {
        page_data(body)
    }
}

impl PageSizeKnob for Issues {}
impl ResumeAfter for Issues {
    type Cursor = String;
}

fn vars() -> IssuesVars {
    IssuesVars { owner: "acme", first: None, after: None }
}

fn page(numbers: &[u64], end: Value, more: Value) -> Value {
    let nodes: Vec<Value> = numbers.iter().map(|number| json!({"number": number})).collect();
    json!({"data": {"repository": {"issues": {"nodes": nodes, "pageInfo": {"endCursor": end, "hasNextPage": more}}}}})
}

fn reply(body: Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(body)
}

/// A server answering by the request's `variables.after`: `pages` maps a
/// cursor (`null` or absent for the first page) to a reply.
async fn serve(pages: impl Fn(Option<&str>) -> ResponseTemplate + Send + Sync + 'static) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::method("POST"))
        .and(wiremock::matchers::path("/graphql"))
        .respond_with(move |request: &Request| {
            let body: Value = serde_json::from_slice(&request.body).unwrap();
            pages(body["variables"].get("after").and_then(Value::as_str))
        })
        .mount(&server)
        .await;
    server
}

fn client(server: &MockServer) -> Client<TestApi> {
    builder(ApiDefinition::new("https://api.example.invalid"), &server.uri())
}

async fn sent_variables(server: &MockServer) -> Vec<Value> {
    let requests = server.received_requests().await.unwrap();
    requests.iter().map(|request| serde_json::from_slice::<Value>(&request.body).unwrap()["variables"].clone()).collect()
}

fn ids(items: &[Issue]) -> Vec<u64> {
    items.iter().map(|issue| issue.number).collect()
}

#[tokio::test]
async fn two_pages_are_collected_from_data_with_the_cursor_and_size_written_into_variables() {
    let server = serve(|after| match after {
        None => reply(page(&[1, 2], json!("c1"), json!(true))),
        Some("c1") => reply(page(&[3], json!("c2"), json!(false))),
        other => panic!("unexpected cursor {other:?}"),
    })
    .await;

    let items = client(&server).send(Issues::new(vars())).pager().page_size(2).collect_all().await.expect("both pages");

    assert_eq!(ids(&items), [1, 2, 3]);
    let sent = sent_variables(&server).await;
    assert_eq!(sent, [json!({"owner": "acme", "first": 2, "after": null}), json!({"owner": "acme", "first": 2, "after": "c1"})],
        "the null initial cursor is sent as serialized; the pager writes its inputs over the variables");
    let bodies = server.received_requests().await.unwrap();
    let first: Value = serde_json::from_slice(&bodies[0].body).unwrap();
    assert_eq!(first["query"], DOCUMENT, "the document is sent unchanged on every page");
}

#[tokio::test]
async fn an_omitted_initial_cursor_and_size_stay_omitted_until_the_pager_has_them() {
    #[derive(Serialize)]
    struct Bare {
        owner: &'static str,
    }
    let server = serve(|after| match after {
        None => reply(page(&[1], json!("c1"), json!(true))),
        Some("c1") => reply(page(&[2], json!(null), json!(false))),
        other => panic!("unexpected cursor {other:?}"),
    })
    .await;

    let items = client(&server).send(Issues::new(Bare { owner: "acme" })).pager().collect_all().await.expect("both pages");

    assert_eq!(ids(&items), [1, 2]);
    assert_eq!(sent_variables(&server).await, [json!({"owner": "acme"}), json!({"owner": "acme", "after": "c1"})]);
}

#[tokio::test]
async fn a_resumed_cursor_is_written_into_variables_of_the_first_request() {
    let server = serve(|after| match after {
        Some("kept") => reply(page(&[9], json!(null), json!(false))),
        other => panic!("unexpected cursor {other:?}"),
    })
    .await;

    let items = client(&server).send(Issues::new(vars())).pager().after("kept".to_owned()).collect_all().await.expect("one page");

    assert_eq!(ids(&items), [9]);
    assert_eq!(sent_variables(&server).await, [json!({"owner": "acme", "after": "kept"})]);
}

#[tokio::test]
async fn a_graphql_error_stops_collection_and_its_partial_data_joins_no_items() {
    let server = serve(|after| match after {
        None => reply(page(&[1, 2], json!("c1"), json!(true))),
        Some("c1") => {
            let mut body = page(&[3], json!("c2"), json!(true));
            body["errors"] = json!([{"message": "rate limited", "type": "RATE_LIMITED"}]);
            reply(body)
        }
        other => panic!("unexpected cursor {other:?}"),
    })
    .await;

    let incomplete = client(&server).send(Issues::new(vars())).pager().collect_all().await.expect_err("page two fails");

    assert_eq!(incomplete.reason, IncompleteReason::HttpError);
    assert_eq!(ids(&incomplete.items), [1, 2], "the failed page's partial data is not collected");
    let Some(PagerError::Api { error, meta }) = &incomplete.error else { panic!("{incomplete:?}") };
    assert_eq!(meta.status.as_u16(), 200);
    let ErrorKind::Declared(GraphQlOperationError::GraphQl(failure)) = &error.kind else { panic!("{error:?}") };
    assert_eq!(failure.errors[0].message, "rate limited");
    assert_eq!(failure.errors[0].other["type"], "RATE_LIMITED");
    assert!(failure.partial_data.is_some(), "the partial data stays in the error");
    assert!(std::str::from_utf8(&error.body).unwrap().contains("RATE_LIMITED"), "the raw body is kept");
    assert_eq!(server.received_requests().await.unwrap().len(), 2, "nothing is sent after the failure");
}

#[tokio::test]
async fn a_wrapper_without_data_or_with_undecodable_data_is_a_malformed_page() {
    for (label, body) in [
        ("null data", json!({"data": null})),
        ("data that does not decode", json!({"data": "text"})),
        ("data without the connection", json!({"data": {"repository": 5}})),
        ("not a wrapper", json!([1, 2])),
        ("errors not an array", json!({"data": {"repository": null}, "errors": {}})),
    ] {
        let server = serve(move |_| reply(body.clone())).await;
        let error = client(&server).send(Issues::new(vars())).pager().fetch_page().await.expect_err(label);
        let PagerError::Api { error, .. } = &error else { panic!("{label}: {error:?}") };
        assert!(matches!(error.kind, ErrorKind::MalformedSuccess), "{label}: {error:?}");
    }
}

#[tokio::test]
async fn a_missing_or_invalid_continuation_follows_the_incomplete_rules() {
    let missing = serve(|_| reply(page(&[1], json!(null), json!(true)))).await;
    let incomplete = client(&missing).send(Issues::new(vars())).pager().collect_all().await.expect_err("no cursor");
    assert_eq!(incomplete.reason, IncompleteReason::InvalidContinuation);
    assert!(matches!(incomplete.error, Some(PagerError::MissingContinuation { .. })), "{incomplete:?}");
    assert_eq!(ids(&incomplete.items), [1], "the page that lacked the cursor is still delivered");

    let invalid = serve(|_| reply(page(&[1], json!("c1"), json!("yes")))).await;
    let incomplete = client(&invalid).send(Issues::new(vars())).pager().collect_all().await.expect_err("bad signal");
    assert_eq!(incomplete.reason, IncompleteReason::InvalidContinuation);
    assert!(matches!(incomplete.error, Some(PagerError::InvalidContinuation { .. })), "{incomplete:?}");
}

#[tokio::test]
async fn a_repeated_cursor_and_the_page_limit_stop_collection() {
    let looping = serve(|_| reply(page(&[1], json!("same"), json!(true)))).await;
    let incomplete = client(&looping).send(Issues::new(vars())).pager().collect_all().await.expect_err("loops");
    assert_eq!(incomplete.reason, IncompleteReason::RepeatedContinuation);
    assert_eq!(looping.received_requests().await.unwrap().len(), 2);

    let endless = serve(|after| {
        let next = after.map_or(1, |cursor| cursor.parse::<u64>().unwrap() + 1);
        reply(page(&[next], json!(next.to_string()), json!(true)))
    })
    .await;
    let incomplete = client(&endless).send(Issues::new(vars())).pager().page_limit(3).collect_all().await.expect_err("limit");
    assert_eq!(incomplete.reason, IncompleteReason::PageLimitReached);
    assert_eq!(ids(&incomplete.items), [1, 2, 3]);
}

#[tokio::test]
async fn a_page_size_above_the_declared_maximum_is_refused_before_sending() {
    let server = serve(|_| reply(page(&[1], json!(null), json!(false)))).await;
    let error = client(&server).send(Issues::new(vars())).pager().page_size(51).fetch_page().await.expect_err("too large");
    assert!(matches!(error, PagerError::Transport(_)), "{error:?}");
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn the_shared_deadline_bounds_every_page() {
    let server = serve(|after| match after {
        None => reply(page(&[1], json!("c1"), json!(true))),
        _ => reply(page(&[2], json!(null), json!(false))).set_delay(Duration::from_secs(5)),
    })
    .await;
    let deadline = Deadline::after(Duration::from_millis(300));

    let incomplete = client(&server).send(Issues::new(vars())).pager().deadline(&deadline).collect_all().await.expect_err("deadline");

    assert_eq!(incomplete.reason, IncompleteReason::DeadlineExceeded);
    assert_eq!(ids(&incomplete.items), [1]);
    assert!(matches!(incomplete.error, Some(PagerError::DeadlineExceeded { completed_pages: 1, .. })), "{incomplete:?}");
}

#[test]
fn page_data_reads_data_and_classifies_the_wrapper_as_one_send_does() {
    type Failure = GraphQlOperationError<IssuesData, NoDeclaredError>;
    let root = |body: Value| page_data::<IssuesData, NoDeclaredError>(body);
    assert_eq!(root(json!({"data": {"repository": null}, "errors": []})).unwrap(), json!({"repository": null}));
    assert!(matches!(root(json!({"errors": [{"message": "x"}]})), Err(PageRootError::Declared(Failure::GraphQl(f))) if f.partial_data.is_none()));
    assert!(matches!(root(json!({"errors": [{"message": "x"}], "data": "text"})),
        Err(PageRootError::Declared(Failure::GraphQl(f))) if f.partial_data.is_none()), "undecodable partial data is dropped");
    assert!(matches!(root(json!({"data": "text"})), Err(PageRootError::Malformed)), "data must decode as the operation's type");
    assert!(matches!(root(json!({"errors": [{"path": ["a"]}], "data": {"repository": null}})), Err(PageRootError::Malformed)),
        "an error entry without a message is malformed");
    assert!(matches!(root(json!({})), Err(PageRootError::Malformed)));
}
