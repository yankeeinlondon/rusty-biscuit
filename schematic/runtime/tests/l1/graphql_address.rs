//! The GraphQL operation address (R39, D-11): derivation from the client's
//! effective base URL by the API's ordered rules, the caller's full-address
//! override, its validation at build, its cross-origin check, and the
//! pre-send refusal when a client has no address. Also the generic
//! `ErrorBody<Api>` form generated code uses for a type of another module.

use http::Method;
use schematic_runtime::graphql::{AddressError, derive_address, request_body};
use schematic_runtime::pager::spec::{IndexBase, Input, Kind, MoreSignal, Source};
use schematic_runtime::pager::{ItemSelector, PageNumberKind, Paginated, Strategy};
use schematic_runtime::{
    Address, AddressRule, Api, ApiDefinition, BuildError, CallOptions, Client, EndpointSpec, ErrorBody, ErrorKind,
    GraphQl, GraphQlOperationError, InvalidArgument, Json, NoDeclaredError, OriginPolicy, RequestParts,
    TransportErrorKind, Url,
};
use serde::Deserialize;
use serde_json::json;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::pager_support::Def;
use crate::support::Item;

/// GitHub's rules: Enterprise `/api/v3` becomes `/api/graphql`; anything
/// else gains `/graphql`.
const GITHUB: &[AddressRule] = &[
    AddressRule::ReplaceSuffix { suffix: "/api/v3", with: "/api/graphql" },
    AddressRule::Append("/graphql"),
];

struct GitHub;

impl Api for GitHub {
    const NAME: &'static str = "GitHub";
}

/// The definition's base is never served; each test points the client's
/// base URL at a mock, as a caller override does.
fn definition() -> ApiDefinition {
    ApiDefinition::new("https://api.github.invalid").graphql(GITHUB)
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Viewer {
    login: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
struct Data {
    viewer: Viewer,
}

/// A REST GET of `/repos`.
struct Repos;

impl EndpointSpec for Repos {
    type Api = GitHub;
    type Success = serde_json::Value;
    type Failure = NoDeclaredError;
    type Decoder = Json<serde_json::Value>;

    const ENDPOINT_ID: &'static str = "repos";
    const METHOD: Method = Method::GET;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        Ok(RequestParts::new(Method::GET, "/repos"))
    }
}

/// A GraphQL operation sent to the client's GraphQL address. Its path is
/// deliberately not empty, to prove it is ignored; its query pair is kept.
struct Viewing;

const DOCUMENT: &str = "query Viewing { viewer { login } }";

impl EndpointSpec for Viewing {
    type Api = GitHub;
    type Success = Data;
    type Failure = GraphQlOperationError<Data, registered::Problem>;
    type Decoder = GraphQl<Data>;

    const ENDPOINT_ID: &'static str = "viewing";
    const METHOD: Method = Method::POST;
    const ADDRESS: Address = Address::GraphQl;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        let mut parts = RequestParts::new(Method::POST, "/ignored");
        parts.query.push(("trace".to_owned(), "1".into()));
        parts.headers.push(("Accept".to_owned(), "application/vnd.github+json".to_owned()));
        parts.body = request_body::<()>(DOCUMENT, None, Some("Viewing"))?;
        Ok(parts)
    }
}

/// Stands in for `schematic-definitions`: a registered JSON type defined
/// outside the module that declares the API marker.
mod registered {
    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Deserialize)]
    pub struct Problem {
        pub message: String,
    }
}

// The form generated code writes once per API module: the marker is local,
// so the impl is legal even when the type lives in another crate.
impl ErrorBody<GitHub> for registered::Problem {
    fn decode_body(_: usize, body: &[u8]) -> Option<Self> {
        serde_json::from_slice(body).ok()
    }
}

const REPLY: &str = r#"{"data":{"viewer":{"login":"octocat"}}}"#;

/// A mock answering every request with [`REPLY`].
async fn server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(wiremock::matchers::any())
        .respond_with(ResponseTemplate::new(200).set_body_raw(REPLY, "application/json"))
        .mount(&server)
        .await;
    server
}

fn client(definition: ApiDefinition, base_url: &str) -> Client<GitHub> {
    Client::<GitHub>::new(definition).base_url(base_url)
}

/// The paths (with query) every request to `server` reached, in order.
async fn reached(server: &MockServer) -> Vec<String> {
    let requests = server.received_requests().await.expect("recording enabled");
    requests
        .iter()
        .map(|request| match request.url.query() {
            Some(query) => format!("{}?{query}", request.url.path()),
            None => request.url.path().to_owned(),
        })
        .collect()
}

#[test]
fn derivation_takes_the_first_matching_rule_over_path_segments() {
    let derive = |base: &str, rules: &[AddressRule]| {
        derive_address(&Url::parse(base).expect("base parses"), rules).map(String::from)
    };
    let cases = [
        ("https://api.github.com", "https://api.github.com/graphql"),
        ("https://api.github.com/", "https://api.github.com/graphql"),
        ("https://ghe.example/api/v3", "https://ghe.example/api/graphql"),
        ("https://ghe.example/api/v3/", "https://ghe.example/api/graphql"),
        ("https://h.example/prefix/api/v3/", "https://h.example/prefix/api/graphql"),
        ("http://h.example:8443/api/v3", "http://h.example:8443/api/graphql"),
        ("http://h.example:8443/base", "http://h.example:8443/base/graphql"),
        // A segment that only ends in `v3` is not the suffix.
        ("https://h.example/api/xv3", "https://h.example/api/xv3/graphql"),
        ("https://h.example/v3", "https://h.example/v3/graphql"),
    ];
    for (base, expected) in cases {
        assert_eq!(derive(base, GITHUB), Ok(expected.to_owned()), "base {base}");
    }

    for base in ["https://h.example/api/v3?x=1", "https://h.example/api/v3#top", "https://h.example/?"] {
        assert_eq!(derive(base, GITHUB), Err(AddressError::QueryOrFragment), "base {base}");
    }
    assert_eq!(derive("data:text/plain,x", GITHUB), Err(AddressError::NotHierarchical));
    assert_eq!(derive("https://h.example/", &[]), Err(AddressError::NoRuleMatched));
    assert_eq!(
        derive("https://h.example/rest", &[AddressRule::ReplaceSuffix { suffix: "v3", with: "graphql" }]),
        Err(AddressError::NoRuleMatched)
    );

    // Order decides: an `Append` first shadows the replacement.
    let append_first = [GITHUB[1], GITHUB[0]];
    assert_eq!(derive("https://h.example/api/v3", &append_first), Ok("https://h.example/api/v3/graphql".to_owned()));
}

#[tokio::test]
async fn rest_and_graphql_endpoints_reach_their_addresses_under_both_bases() {
    // Arrange: an Enterprise-style base and a GitHub.com-style base on one
    // mock, each set as the caller's base URL over the definition's.
    let server = server().await;
    let enterprise = client(definition(), &format!("{}/api/v3", server.uri()));
    let dotcom = client(definition(), &server.uri());

    // Act
    for client in [&enterprise, &dotcom] {
        client.send(Repos).full().await.expect("head").into_result().expect("rest reply");
        let data = client.send(Viewing).full().await.expect("head").into_result().expect("graphql reply");
        assert_eq!(data.viewer.login, "octocat");
        client.send_raw(Viewing, &CallOptions::default()).await.expect("raw head");
    }

    // Assert: REST joins its path; GraphQL ignores it and keeps the query.
    assert_eq!(
        reached(&server).await,
        [
            "/api/v3/repos",
            "/api/graphql?trace=1",
            "/api/graphql?trace=1",
            "/repos",
            "/graphql?trace=1",
            "/graphql?trace=1",
        ]
    );
    let requests = server.received_requests().await.expect("recording enabled");
    let operation = &requests[1];
    assert_eq!(operation.method, Method::POST);
    assert_eq!(operation.headers.get("accept").expect("accept"), "application/vnd.github+json");
    assert_eq!(operation.headers.get("content-type").expect("content type"), "application/json");
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&operation.body).expect("JSON body"),
        json!({ "query": DOCUMENT, "operationName": "Viewing" })
    );
}

#[tokio::test]
async fn a_graphql_url_override_beats_the_rules() {
    let server = server().await;
    let client = client(definition(), &format!("{}/api/v3", server.uri())).graphql_url(format!("{}/custom/gql?v=2", server.uri()));

    client.send(Viewing).full().await.expect("head").into_result().expect("graphql reply");

    assert_eq!(reached(&server).await, ["/custom/gql?v=2&trace=1"]);
}

#[test]
fn an_invalid_override_or_an_underivable_base_fails_at_build() {
    let invalid = |reason| Err(BuildError::InvalidGraphQlUrl { reason });
    let build = |client: Client<GitHub>| client.build().map(drop);
    let base = "https://ghe.example/api/v3";

    let overrides = [
        ("not a url", "not an absolute URL"),
        ("ftp://ghe.example/graphql", "scheme is not http or https"),
        ("https://ghe.example/graphql#frag", "carries a fragment"),
        ("https://user:SENTINEL@ghe.example/graphql", "carries user information"),
    ];
    for (url, reason) in overrides {
        let result = build(client(definition(), base).graphql_url(url));
        assert_eq!(result, invalid(reason), "override {url}");
        let text = result.expect_err("refused").to_string();
        assert!(!text.contains("SENTINEL"), "{text}");
    }

    // A base carrying a query cannot derive under rules, unless an override
    // makes derivation unnecessary; without rules nothing is derived.
    let queried = "https://ghe.example/api/v3?tenant=a";
    assert_eq!(build(client(definition(), queried)), invalid("the base URL carries a query or fragment"));
    assert_eq!(build(client(definition(), queried).graphql_url("https://ghe.example/api/graphql")), Ok(()));
    assert_eq!(build(client(ApiDefinition::new(queried), queried)), Ok(()));
}

#[tokio::test]
async fn a_foreign_override_is_refused_under_same_origin_and_sent_when_its_endpoint_allows_it() {
    // Arrange: the base on one origin, the override on another.
    let base = server().await;
    let foreign = server().await;
    let override_url = format!("{}/graphql", foreign.uri());

    // Act + Assert: the default same-origin policy refuses before sending.
    let refused = client(definition(), &base.uri()).graphql_url(&override_url);
    let error = match refused.send(Viewing).full().await {
        Ok(response) => panic!("expected a refusal, got {response}"),
        Err(error) => error,
    };
    assert!(matches!(error.kind, TransportErrorKind::CrossOriginRejected { .. }), "{:?}", error.kind);
    assert!(reached(&foreign).await.is_empty());

    // An allow list for this endpoint, by its id, lets it through.
    let allowed = definition().endpoint_cross_origin(Viewing::ENDPOINT_ID, OriginPolicy::allow_origins([foreign.uri()]));
    let client = client(allowed, &base.uri()).graphql_url(&override_url);
    client.send(Viewing).full().await.expect("head").into_result().expect("graphql reply");
    assert_eq!(reached(&foreign).await, ["/graphql?trace=1"]);
    assert!(reached(&base).await.is_empty());
}

#[tokio::test]
async fn a_graphql_send_with_neither_rules_nor_override_fails_before_sending() {
    let server = server().await;
    let client = client(ApiDefinition::new("https://api.github.invalid"), &server.uri());

    let error = match client.send(Viewing).full().await {
        Ok(response) => panic!("expected a refusal, got {response}"),
        Err(error) => error,
    };
    let raw = client.send_raw(Viewing, &CallOptions::default()).await.map(drop).expect_err("raw refused");

    assert_eq!(error.kind, TransportErrorKind::InvalidUrl);
    assert_eq!(raw.kind, TransportErrorKind::InvalidUrl);
    // REST endpoints of the same client are unaffected.
    client.send(Repos).full().await.expect("head").into_result().expect("rest reply");
    assert_eq!(reached(&server).await, ["/repos"]);
}

#[tokio::test]
async fn a_registered_type_of_another_module_decodes_a_declared_http_error() {
    let server = MockServer::start().await;
    Mock::given(path("/graphql"))
        .respond_with(ResponseTemplate::new(403).set_body_raw(r#"{"message":"Resource not accessible"}"#, "application/json"))
        .mount(&server)
        .await;
    let client = client(definition(), &server.uri());

    let response = client.send(Viewing).full().await.expect("head");

    let error = response.error().expect("failure");
    assert_eq!(
        error.kind,
        ErrorKind::Declared(GraphQlOperationError::Http(registered::Problem {
            message: "Resource not accessible".to_owned()
        }))
    );
}

/// A page-numbered list sent to the GraphQL address, to prove the pager's
/// requests take it too.
struct Listing;

impl EndpointSpec for Listing {
    type Api = GitHub;
    type Success = Vec<Item>;
    type Failure = NoDeclaredError;
    type Decoder = Json<Vec<Item>>;

    const ENDPOINT_ID: &'static str = "listing";
    const METHOD: Method = Method::POST;
    const ADDRESS: Address = Address::GraphQl;

    fn into_parts(self) -> Result<RequestParts, InvalidArgument> {
        let mut parts = RequestParts::new(Method::POST, "");
        parts.body = request_body::<()>("query { items }", None, None)?;
        Ok(parts)
    }
}

impl Paginated for Listing {
    type Item = Item;
    type Kind = PageNumberKind;
    type Session = schematic_runtime::pager::NoSession;

    const STRATEGY: &'static Strategy = &Def::new(Kind::PageNumber {
        page: Input::Query("page"),
        base: IndexBase::OneBased,
        more: MoreSignal::Present { source: Source::Header("X-Next-Page") },
    })
    .done();
    const ITEMS: &'static ItemSelector = &ItemSelector::ROOT;
    const PATH: &'static str = "";
}

#[tokio::test]
async fn every_page_of_a_graphql_address_endpoint_reaches_that_address() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/graphql"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).insert_header("X-Next-Page", "2").set_body_json(json!([{ "id": 1 }])))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/api/graphql"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!([{ "id": 2 }])))
        .mount(&server)
        .await;
    let client = client(definition(), &format!("{}/api/v3", server.uri()));

    let items = client.send(Listing).pager().collect_all().await.expect("both pages");

    assert_eq!(items, [Item { id: 1 }, Item { id: 2 }]);
    assert_eq!(reached(&server).await, ["/api/graphql?page=1", "/api/graphql?page=2"]);
}
