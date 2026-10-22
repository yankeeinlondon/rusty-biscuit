//! GraphQL decoding through the managed send's envelope (R37, R38, R41):
//! the wrapper is classified before typed data, so a valid non-empty
//! `errors` array is always a declared GraphQL failure; partial data is kept
//! only when it decodes; malformed wrappers, entries, and data are malformed
//! successes; every non-2xx reply takes the HTTP error table, and GraphQL
//! errors stay declared after an HTTP opt-out. Sniff's check-status fixtures
//! are read as sent. The global hook sees the wire wrapper once and a typed
//! mutator sees only failure-free data. Also the request body (R35, R36).

use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use bytes::Bytes;
use http::Method;
use schematic_runtime::graphql::request_body;
use schematic_runtime::{
    ApiDefinition, Client, EndpointSpec, ErrorBody, ErrorKind, ErrorRoute, GraphQl, GraphQlFailure,
    GraphQlOperationError, HookError, InvalidArgument, Location, NoDeclaredError, PathSegment, RequestBody,
    RequestParts, Response,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use wiremock::matchers::path;
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::support::{TestApi, builder};

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Viewer {
    login: String,
}

/// The data of sniff's check-status query.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct CheckData {
    repository: Option<Repository>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Repository {
    p0: Option<PullRequest>,
    p1: Option<PullRequest>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct PullRequest {
    state: String,
    commits: Commits,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Commits {
    nodes: Vec<CommitNode>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct CommitNode {
    commit: Commit,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Commit {
    oid: String,
    #[serde(rename = "statusCheckRollup")]
    status_check_rollup: Option<serde_json::Value>,
}

/// GitHub's REST-style error body.
#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Problem {
    message: String,
}

impl ErrorBody for Problem {
    fn decode_body(_: usize, body: &[u8]) -> Option<Self> {
        serde_json::from_slice(body).ok()
    }
}

/// A GraphQL operation POSTed to `path`, whose data is `D` and whose HTTP
/// error body is `H`; `H = NoDeclaredError` opts out of HTTP errors.
struct Operation<D, H> {
    path: &'static str,
    types: PhantomData<fn() -> (D, H)>,
}

fn operation<D, H>(path: &'static str) -> Operation<D, H> {
    Operation { path, types: PhantomData }
}

impl<D, H> EndpointSpec for Operation<D, H>
where
    D: serde::de::DeserializeOwned + Send + 'static,
    H: ErrorBody,
{
    type Api = TestApi;
    type Success = D;
    type Failure = GraphQlOperationError<D, H>;
    type Decoder = GraphQl<D>;

    const ENDPOINT_ID: &'static str = "check_status";
    const METHOD: Method = Method::POST;
    const ERRORS: &'static [ErrorRoute] = if H::DECLARED { &[ErrorRoute::new(schematic_runtime::StatusMatch::Range { lo: 400, hi: 599 }, 0)] } else { &[] };

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        let mut parts = RequestParts::new(Method::POST, self.path);
        parts.body = request_body::<()>("query { viewer { login } }", None, None)?;
        Ok(parts)
    }
}

type Reply<D, H> = Response<D, GraphQlOperationError<D, H>>;

async fn serve(routes: &[(&'static str, u16, &str)]) -> MockServer {
    let server = MockServer::start().await;
    for (at, status, body) in routes {
        Mock::given(path(*at))
            .respond_with(ResponseTemplate::new(*status).set_body_raw(body.as_bytes().to_vec(), "application/json"))
            .mount(&server)
            .await;
    }
    server
}

fn client(server: &MockServer) -> Client<TestApi> {
    builder(ApiDefinition::new("https://api.example.invalid"), &server.uri())
}

async fn send<D, H>(client: &Client<TestApi>, at: &'static str) -> Reply<D, H>
where
    D: serde::de::DeserializeOwned + Send + 'static,
    H: ErrorBody,
{
    client.send(operation::<D, H>(at)).full().await.expect("head")
}

/// The GraphQL failure of `response`, asserting its raw body is `body`.
fn graphql_failure<'r, D, H: std::fmt::Debug>(response: &'r Reply<D, H>, body: &str) -> &'r GraphQlFailure<D>
where
    D: std::fmt::Debug,
{
    let error = response.error().unwrap_or_else(|| panic!("expected a failure: {:?}", response.meta.status));
    assert_eq!(error.body, Bytes::from(body.to_string()), "the raw body is kept");
    match &error.kind {
        ErrorKind::Declared(GraphQlOperationError::GraphQl(failure)) => failure,
        other => panic!("expected a declared GraphQL failure, got {other:?}"),
    }
}

fn kind<D, H>(response: &Reply<D, H>) -> Option<&ErrorKind<GraphQlOperationError<D, H>>> {
    response.error().map(|error| &error.kind)
}

#[test]
fn the_request_body_carries_the_document_unchanged_and_variables_only_as_an_object() {
    #[derive(Serialize)]
    struct Variables {
        owner: String,
        after: Option<String>,
    }
    let typed = request_body(
        "query Check($owner: String!) { repository(owner: $owner) { id } }",
        Some(&Variables { owner: "acme".into(), after: None }),
        Some("Check"),
    )
    .expect("an object");
    let RequestBody::Json(text) = typed else { panic!("a JSON body") };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&text).expect("json"),
        json!({
            "query": "query Check($owner: String!) { repository(owner: $owner) { id } }",
            "variables": {"owner": "acme", "after": null},
            "operationName": "Check",
        })
    );

    let RequestBody::Json(unit) = request_body::<()>("{ not graphql ((", None, Some("")).expect("no variables") else {
        panic!("a JSON body")
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&unit).expect("json"),
        json!({"query": "{ not graphql ((", "operationName": ""}),
        "no variables member; an invalid document and an empty name are sent unchanged"
    );

    for refused in [request_body("q", Some(&vec![1, 2]), None), request_body("q", Some(&"text"), None), request_body("q", Some(&()), None)] {
        let error = refused.expect_err("not an object");
        assert!(error.reason().contains("JSON object"), "{error}");
    }
}

#[tokio::test]
async fn sniff_success_fixtures_decode_as_typed_data() {
    let fixtures = [
        ("/success", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/success.json")),
        ("/passed", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/passed.json")),
        ("/changed-head", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/changed_head.json")),
        ("/null-rollup", include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/null_rollup.json")),
    ];
    let server = serve(&fixtures.map(|(at, body)| (at, 200, body))).await;
    let client = client(&server);
    for (at, _) in fixtures {
        let response = send::<CheckData, Problem>(&client, at).await;
        let data = response.data().unwrap_or_else(|| panic!("{at}: {:?}", response.error()));
        let pull = data.repository.as_ref().and_then(|repository| repository.p0.as_ref()).expect("p0");
        assert_eq!(pull.commits.nodes[0].commit.oid.len(), 40, "{at}");
    }
    let null_rollup = send::<CheckData, Problem>(&client, "/null-rollup").await;
    let rollup = &null_rollup.data().and_then(|data| data.repository.as_ref()).and_then(|repository| repository.p0.as_ref()).expect("p0").commits.nodes[0]
        .commit
        .status_check_rollup;
    assert_eq!(*rollup, None, "a null check result is domain data, not a failure");
}

#[tokio::test]
async fn sniff_error_fixtures_are_declared_graphql_failures_keeping_github_type() {
    let partial = include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/partial_errors.json");
    let request_wide = include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/request_wide_errors.json");
    let rate_limit = include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/rate_limit.json");
    let server = serve(&[("/partial", 200, partial), ("/request-wide", 200, request_wide), ("/rate-limit", 200, rate_limit)]).await;
    let client = client(&server);

    let response = send::<CheckData, Problem>(&client, "/partial").await;
    let failure = graphql_failure(&response, partial);
    assert_eq!(failure.errors.len(), 1);
    let error = &failure.errors[0];
    assert_eq!(error.message, "Could not resolve to a PullRequest with the number of 8.");
    assert_eq!(error.path, Some(vec![PathSegment::Field("repository".into()), PathSegment::Field("p1".into())]));
    assert_eq!(error.other.get("type"), Some(&json!("NOT_FOUND")), "GitHub's top-level type is kept");
    let repository = failure.partial_data.as_ref().and_then(|data| data.repository.as_ref()).expect("usable partial data");
    assert_eq!(repository.p0.as_ref().map(|pull| pull.state.as_str()), Some("OPEN"));
    assert_eq!(repository.p1, None);

    let response = send::<CheckData, Problem>(&client, "/request-wide").await;
    let failure = graphql_failure(&response, request_wide);
    assert_eq!(failure.partial_data, Some(CheckData { repository: None }), "non-null data that decodes is kept");

    let response = send::<CheckData, Problem>(&client, "/rate-limit").await;
    let failure = graphql_failure(&response, rate_limit);
    assert_eq!(failure.partial_data, None, "null data is never partial data");
    assert_eq!(failure.errors[0].other.get("type"), Some(&json!("RATE_LIMITED")));
    assert!(response.clone().into_result().is_err(), "into_result fails for a GraphQL failure");
}

#[tokio::test]
async fn a_permission_failure_takes_the_http_error_path_not_the_wrapper() {
    let body = include_str!("../../../../sniff/lib/tests/fixtures/remote/graphql_check_status/permission_failure.json");
    let server = serve(&[("/forbidden", 403, body)]).await;
    let client = client(&server);

    let declared = send::<CheckData, Problem>(&client, "/forbidden").await;
    assert_eq!(declared.meta.status, 403);
    let error = declared.error().expect("failure");
    assert_eq!(
        error.kind,
        ErrorKind::Declared(GraphQlOperationError::Http(Problem { message: "Resource not accessible by personal access token".into() }))
    );
    assert_eq!(error.body, Bytes::from(body.to_string()));

    let opted_out = send::<CheckData, NoDeclaredError>(&client, "/forbidden").await;
    assert_eq!(kind(&opted_out), Some(&ErrorKind::Undeclared));
}

#[tokio::test]
async fn the_wrapper_is_classified_before_typed_data() {
    let failures = [
        ("/errors-bad-data", r#"{"errors":[{"message":"m"}],"data":{"viewer":7}}"#),
        ("/errors-null-data", r#"{"errors":[{"message":"m"}],"data":null}"#),
        ("/errors-only", r#"{"errors":[{"message":"m","locations":[{"line":1,"column":3}],"extensions":{"code":"X"}}]}"#),
    ];
    let malformed = [
        ("/null-data", r#"{"data":null}"#),
        ("/no-data", r#"{}"#),
        ("/bad-data", r#"{"data":{"login":7}}"#),
        ("/errors-null", r#"{"errors":null,"data":{"login":"a"}}"#),
        ("/errors-object", r#"{"errors":{"message":"m"},"data":{"login":"a"}}"#),
        ("/no-message", r#"{"errors":[{"path":["a"]}],"data":{"login":"a"}}"#),
        ("/number-message", r#"{"errors":[{"message":1}]}"#),
        ("/negative-index", r#"{"errors":[{"message":"m","path":["a",-1]}]}"#),
        ("/fractional-index", r#"{"errors":[{"message":"m","path":["a",1.5]}]}"#),
        ("/line-zero", r#"{"errors":[{"message":"m","locations":[{"line":0,"column":1}]}]}"#),
        ("/one-bad-entry", r#"{"errors":[{"message":"m"},{"oops":true}]}"#),
        ("/array", r#"[{"data":{"login":"a"}}]"#),
        ("/not-json", r#"{"data":"#),
    ];
    let mut routes: Vec<(&'static str, u16, &str)> = vec![
        ("/ok", 200, r#"{"data":{"login":"octocat"}}"#),
        ("/empty-errors", 200, r#"{"errors":[],"data":{"login":"octocat"}}"#),
        ("/created", 201, r#"{"data":{"login":"octocat"}}"#),
        ("/partial-206", 206, r#"{"errors":[{"message":"m","path":["viewer",0]}],"data":{"login":"a"}}"#),
    ];
    routes.extend(failures.iter().chain(&malformed).map(|(at, body)| (*at, 200, *body)));
    let server = serve(&routes).await;
    let client = client(&server);

    for at in ["/ok", "/empty-errors", "/created"] {
        let response = send::<Viewer, Problem>(&client, at).await;
        assert_eq!(response.data(), Some(&Viewer { login: "octocat".into() }), "{at}");
    }
    assert_eq!(send::<Viewer, Problem>(&client, "/created").await.meta.status, 201, "every 2xx keeps its status");

    for (at, body) in failures {
        let response = send::<Viewer, Problem>(&client, at).await;
        let failure = graphql_failure(&response, body);
        assert_eq!(failure.errors[0].message, "m", "{at}");
        assert_eq!(failure.partial_data, None, "{at}: data that is null, absent, or undecodable is not partial data");
    }
    let errors_only = send::<Viewer, Problem>(&client, "/errors-only").await;
    let error = &graphql_failure(&errors_only, failures[2].1).errors[0];
    assert_eq!(error.locations, Some(vec![Location { line: 1, column: 3 }]));
    assert_eq!(error.extensions, Some(json!({"code": "X"})));
    assert!(error.other.is_empty());

    let partial = send::<Viewer, Problem>(&client, "/partial-206").await;
    assert_eq!(partial.meta.status, 206);
    let failure = graphql_failure(&partial, r#"{"errors":[{"message":"m","path":["viewer",0]}],"data":{"login":"a"}}"#);
    assert_eq!(failure.errors[0].path, Some(vec![PathSegment::Field("viewer".into()), PathSegment::Index(0)]));
    assert_eq!(failure.partial_data, Some(Viewer { login: "a".into() }));

    for (at, body) in malformed {
        let response = send::<Viewer, Problem>(&client, at).await;
        let error = response.error().unwrap_or_else(|| panic!("{at}: expected a failure"));
        assert_eq!(error.kind, ErrorKind::MalformedSuccess, "{at}");
        assert_eq!(error.body, Bytes::from(body), "{at}");
    }
}

#[tokio::test]
async fn graphql_errors_stay_declared_after_an_http_opt_out() {
    let errors = r#"{"errors":[{"message":"m"}]}"#;
    let server = serve(&[("/errors", 200, errors), ("/down", 502, errors)]).await;
    let client = client(&server);
    let response = send::<Viewer, NoDeclaredError>(&client, "/errors").await;
    assert_eq!(graphql_failure(&response, errors).errors.len(), 1);
    let down = send::<Viewer, NoDeclaredError>(&client, "/down").await;
    assert_eq!(kind(&down), Some(&ErrorKind::Undeclared), "a non-2xx body is never read as a wrapper");
}

#[tokio::test]
async fn the_global_hook_sees_the_wrapper_once_and_the_mutator_only_failure_free_data() {
    let server = serve(&[
        ("/ok", 200, r#"{"data":{"login":"octocat"},"extensions":{"cost":1}}"#),
        ("/partial", 200, r#"{"errors":[{"message":"m"}],"data":{"login":"a"}}"#),
        ("/forbidden", 403, r#"{"message":"no"}"#),
        ("/refuse", 200, r#"{"data":{"login":"refuse"}}"#),
    ])
    .await;
    let log: Arc<Mutex<Vec<String>>> = Arc::default();
    let (global_log, mutator_log) = (Arc::clone(&log), Arc::clone(&log));
    let client = client(&server)
        .pre_response_json(move |_, wrapper| {
            let keys: Vec<&str> = wrapper.as_object().map(|map| map.keys().map(String::as_str).collect()).unwrap_or_default();
            global_log.lock().expect("log").push(format!("global {keys:?}"));
            Ok(wrapper)
        })
        .mutate_response::<Operation<Viewer, Problem>, _>(move |_, viewer: &mut Viewer| {
            mutator_log.lock().expect("log").push(format!("mutator {}", viewer.login));
            if viewer.login == "refuse" {
                return Err(HookError::new());
            }
            viewer.login.make_ascii_uppercase();
            Ok(())
        });

    assert_eq!(send::<Viewer, Problem>(&client, "/ok").await.data(), Some(&Viewer { login: "OCTOCAT".into() }));
    let partial = send::<Viewer, Problem>(&client, "/partial").await;
    assert_eq!(
        graphql_failure(&partial, r#"{"errors":[{"message":"m"}],"data":{"login":"a"}}"#).partial_data,
        Some(Viewer { login: "a".into() }),
        "partial data is never mutated"
    );
    let forbidden = send::<Viewer, Problem>(&client, "/forbidden").await;
    assert_eq!(kind(&forbidden), Some(&ErrorKind::Declared(GraphQlOperationError::Http(Problem { message: "no".into() }))));
    let refused = send::<Viewer, Problem>(&client, "/refuse").await;
    let error = refused.error().expect("failure");
    assert_eq!(error.kind, ErrorKind::Hook);
    assert_eq!(error.body, Bytes::from_static(br#"{"data":{"login":"refuse"}}"#), "the original bytes");

    assert_eq!(
        *log.lock().expect("log"),
        [
            r#"global ["data", "extensions"]"#,
            "mutator octocat",
            r#"global ["data", "errors"]"#,
            r#"global ["data"]"#,
            "mutator refuse",
        ],
        "the hook sees each 2xx wrapper once and no non-2xx body; the mutator sees only failure-free data"
    );
}
