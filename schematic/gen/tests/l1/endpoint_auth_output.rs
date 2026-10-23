//! Endpoint authentication in generated clients.
//!
//! One `slow_` test generates a fresh client for a bearer fixture API whose
//! endpoints are public, inherit the API policy, or replace it (Basic from
//! their own variables, a query key, a cookie key), builds it offline
//! through [`crate::common::fresh_output`], and runs child tests that send
//! to a local wiremock server and assert exactly what it received. `slow_`
//! because it invokes Cargo (`just test-fresh-output`).

use schematic_define::{
    ApiKeyLocation, AuthMethod, AuthPolicy, AuthStrategy, DefinitionContext, Endpoint,
    EnvAuthStrategy, EnvList, EnvMapping, ParamDef, ParamStyle, QueryParamType, RestApi,
};

use crate::common::fresh_output::{Definitions, FreshCrate};

const MODULE: &str = "fixture_endpoint_auth";

fn param(name: &str) -> ParamDef {
    ParamDef { constraints: None,
        deprecated: false,
        rust_name: None,
        secret: false,
        pager_managed: false,
        name: name.to_string(),
        required: false,
        description: None,
        param_type: QueryParamType::String,
        explode: false,
        style: ParamStyle::Form,
    }
}

fn key_param(name: &str, location: ApiKeyLocation) -> AuthPolicy {
    AuthPolicy {
        explicit: vec![AuthMethod::ApiKeyParam {
            name: name.to_string(),
            location,
        }],
        env_fallback: None,
    }
}

/// A bearer API whose `GetStatus` is public (and lets the caller set raw
/// values in every auth location), `GetAccount` inherits, `GetReport` takes
/// Basic from its own variables, and `Search` and `GetSession` take an
/// explicit query or cookie key.
fn fixture_api() -> RestApi {
    let ctx = DefinitionContext::types_in(format!("::schematic_definitions::{MODULE}"));
    let status = Endpoint::get("GetStatus", "/status")
        .describes("Public status")
        .returns_empty()
        .public()
        .header_param(param("Authorization"))
        .header_param(param("X-Trace"))
        .query(param("key"))
        .query(param("page"))
        .cookie(param("sid"))
        .cookie(param("theme"))
        .build()
        .expect("public endpoint");
    let account = Endpoint::get("GetAccount", "/account")
        .describes("The caller's account")
        .returns_empty()
        .build()
        .expect("inheriting endpoint");
    let report = Endpoint::get("GetReport", "/report")
        .describes("A report behind Basic credentials")
        .returns_empty()
        .auth_with_env_mapping(
            AuthPolicy {
                explicit: vec![AuthMethod::Basic],
                env_fallback: Some(EnvAuthStrategy::Basic),
            },
            EnvMapping {
                basic_user: Some(EnvList::from_strs(&["FIXTURE_EA_REPORT_USER"])),
                basic_pass: Some(EnvList::from_strs(&["FIXTURE_EA_REPORT_PASS"])),
                ..EnvMapping::default()
            },
        )
        .build()
        .expect("Basic replacement");
    let search = Endpoint::get("Search", "/search")
        .describes("Search behind a query key")
        .returns_empty()
        .auth(key_param("key", ApiKeyLocation::Query))
        .build()
        .expect("query-key replacement");
    let session = Endpoint::get("GetSession", "/session")
        .describes("Session behind a cookie key")
        .returns_empty()
        .auth(key_param("sid", ApiKeyLocation::Cookie))
        .build()
        .expect("cookie-key replacement");
    RestApi::builder(&ctx, "FixtureEndpointAuth", "http://127.0.0.1:1")
        .describes("Endpoint authentication fixture")
        .authentication(AuthStrategy::BearerToken { header: None }, ["FIXTURE_EA_TOKEN"])
        .module_path(MODULE)
        .endpoints([status, account, report, search, session])
        .build()
        .expect("fixture API")
}

const CHILD_ENV: &[(&str, &str)] = &[
    ("FIXTURE_EA_TOKEN", "env-token"),
    ("FIXTURE_EA_REPORT_USER", "reporter"),
    ("FIXTURE_EA_REPORT_PASS", "report-pass"),
];

/// The child tests [`CHILD_TEST`] must report as passed.
const CHILD_TESTS: &[&str] = &[
    "a_public_call_carries_no_credential_from_any_source_and_keeps_ordinary_values",
    "an_authenticated_sibling_after_a_public_call_carries_its_credential",
    "a_basic_replacement_reads_its_own_variables_and_never_reuses_the_bearer_token",
    "query_and_cookie_replacements_send_the_per_call_key_in_their_own_location",
    "a_replacement_with_no_source_or_an_unaccepted_kind_fails_before_send",
];

const CHILD_TEST: &str = r##"
use schematic_schema::fixture_endpoint_auth::{
    FixtureEndpointAuth, GetAccountRequest, GetReportRequest, GetSessionRequest, GetStatusRequest, SearchRequest,
};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{AuthMethod, TransportErrorKind};
use wiremock::matchers::any;
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const SECRET: &str = "secret-5e1f";

async fn start_server() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(any()).respond_with(ResponseTemplate::new(200)).mount(&server).await;
    server
}

async fn received(server: &MockServer) -> Vec<Request> {
    server.received_requests().await.expect("recording is on")
}

fn header(request: &Request, name: &str) -> Vec<String> {
    request
        .headers
        .get_all(name)
        .iter()
        .map(|value| String::from_utf8_lossy(value.as_bytes()).into_owned())
        .collect()
}

fn client(server: &MockServer) -> FixtureEndpointAuth {
    FixtureEndpointAuth::new().base_url(server.uri())
}

#[tokio::test]
async fn a_public_call_carries_no_credential_from_any_source_and_keeps_ordinary_values() {
    let server = start_server().await;
    // FIXTURE_EA_TOKEN is set too, so the environment is a source as well.
    let with_token = client(&server).credentials(Credential::access_token(SECRET));
    let with_key = client(&server).api_key(SECRET);
    let raw = || {
        GetStatusRequest::default()
            .with_authorization(format!("Bearer {SECRET}"))
            .with_x_trace("t1".to_string())
            .with_key(SECRET.to_string())
            .with_page("2".to_string())
            .with_sid(SECRET.to_string())
            .with_theme("dark".to_string())
    };

    for client in [&with_token, &with_key] {
        let plain = client.send(raw()).full().await.expect("head");
        assert_eq!(plain.meta.request.auth, AuthMethod::Anonymous);
        let per_call = client
            .send(raw())
            .credential(Credential::access_token(SECRET))
            .full()
            .await
            .expect("a per-call credential is dropped on a public endpoint");
        assert_eq!(per_call.meta.request.auth, AuthMethod::Anonymous);
    }

    let requests = received(&server).await;
    assert_eq!(requests.len(), 4);
    for request in &requests {
        assert!(header(request, "authorization").is_empty(), "{request:?}");
        assert_eq!(header(request, "x-trace"), ["t1"]);
        assert_eq!(header(request, "cookie"), ["theme=dark"]);
        assert_eq!(request.url.query(), Some("page=2"));
    }
}

#[tokio::test]
async fn an_authenticated_sibling_after_a_public_call_carries_its_credential() {
    let server = start_server().await;
    let from_env = client(&server);
    let from_client = client(&server).credentials(Credential::access_token("client-token"));

    from_env.send(GetStatusRequest::default()).full().await.expect("head");
    let env_sibling = from_env.send(GetAccountRequest::default()).full().await.expect("head");
    from_client.send(GetStatusRequest::default()).full().await.expect("head");
    let client_sibling = from_client.send(GetAccountRequest::default()).full().await.expect("head");

    assert_eq!(env_sibling.meta.request.auth, AuthMethod::EnvVar("FIXTURE_EA_TOKEN".into()));
    assert_eq!(client_sibling.meta.request.auth, AuthMethod::ClientLevel);
    let sent: Vec<Vec<String>> = received(&server).await.iter().map(|request| header(request, "authorization")).collect();
    assert_eq!(
        sent,
        [vec![], vec!["Bearer env-token".to_string()], vec![], vec!["Bearer client-token".to_string()]]
    );
}

#[tokio::test]
async fn a_basic_replacement_reads_its_own_variables_and_never_reuses_the_bearer_token() {
    let server = start_server().await;
    let with_token = client(&server).credentials(Credential::access_token(SECRET));
    let with_basic = client(&server).basic_auth("alice", "client-pass");

    let from_env = with_token.send(GetReportRequest::default()).full().await.expect("head");
    let from_client = with_basic.send(GetReportRequest::default()).full().await.expect("head");
    let inherited = with_basic.send(GetAccountRequest::default()).full().await.expect("head");

    assert_eq!(from_env.meta.request.auth, AuthMethod::EnvVar("FIXTURE_EA_REPORT_PASS".into()));
    assert_eq!(from_client.meta.request.auth, AuthMethod::ClientLevel);
    assert_eq!(inherited.meta.request.auth, AuthMethod::EnvVar("FIXTURE_EA_TOKEN".into()));
    let requests = received(&server).await;
    assert_eq!(header(&requests[0], "authorization"), ["Basic cmVwb3J0ZXI6cmVwb3J0LXBhc3M="], "reporter:report-pass");
    assert_eq!(header(&requests[1], "authorization"), ["Basic YWxpY2U6Y2xpZW50LXBhc3M="], "alice:client-pass");
    assert_eq!(header(&requests[2], "authorization"), ["Bearer env-token"], "Basic is never sent as a bearer token");
}

#[tokio::test]
async fn query_and_cookie_replacements_send_the_per_call_key_in_their_own_location() {
    let server = start_server().await;
    let client = client(&server);

    let search = client.send(SearchRequest::default()).credential(Credential::api_key("query-key")).full().await.expect("head");
    let session = client.send(GetSessionRequest::default()).credential(Credential::api_key("cookie-key")).full().await.expect("head");

    assert_eq!(search.meta.request.auth, AuthMethod::PerCall);
    assert_eq!(session.meta.request.auth, AuthMethod::PerCall);
    assert!(!format!("{:?}", search.meta).contains("query-key"), "the replacement's query name is secret");
    let requests = received(&server).await;
    assert_eq!(requests[0].url.query(), Some("key=query-key"));
    assert!(header(&requests[0], "authorization").is_empty(), "the API's bearer token is not merged in");
    assert_eq!(header(&requests[1], "cookie"), ["sid=cookie-key"]);
    assert!(header(&requests[1], "authorization").is_empty());
}

#[tokio::test]
async fn a_replacement_with_no_source_or_an_unaccepted_kind_fails_before_send() {
    let server = start_server().await;
    // The client token and the API's environment variable are bearer
    // sources; the query-key endpoint accepts neither and has no fallback.
    let with_token = client(&server).credentials(Credential::access_token(SECRET));

    let missing = with_token.send(SearchRequest::default()).full().await.expect_err("no key source");
    let TransportErrorKind::InvalidArgument(argument) = &missing.kind else {
        panic!("expected an invalid argument, got {:?}", missing.kind);
    };
    assert!(argument.to_string().contains("`Search`"), "{argument}");
    let unaccepted = with_token
        .send(GetReportRequest::default())
        .credential(Credential::access_token(SECRET))
        .full()
        .await
        .expect_err("a bearer token on a Basic endpoint");
    assert!(matches!(unaccepted.kind, TransportErrorKind::InvalidArgument(_)), "{:?}", unaccepted.kind);
    assert!(!format!("{missing:?}{unaccepted:?}").contains(SECRET));

    assert!(received(&server).await.is_empty(), "nothing is sent");
}
"##;

#[test]
fn slow_generated_clients_honor_public_and_replacement_endpoint_auth() {
    let lockfile = biscuit_test_harness::manifest_dir!().join("../schema/Cargo.lock");
    let api = fixture_api();
    let fresh = FreshCrate::generate(&[&api], Definitions::Fixture(&[(MODULE, "")]), &lockfile);
    let generated = std::fs::read_to_string(fresh.src_dir.join(format!("{MODULE}.rs"))).expect("generated module");
    for expected in [".endpoint_public(\"GetStatus\")", ".endpoint_auth(", "pub fn basic_auth("] {
        assert!(generated.contains(expected), "missing `{expected}` in:\n{generated}");
    }

    fresh.add_test_file("endpoint_auth", CHILD_TEST);
    let run = fresh.nextest_run(CHILD_ENV);
    run.assert_success();
    let report = format!("{}{}", run.stdout, run.stderr);
    for test in CHILD_TESTS {
        assert!(report.contains(test), "the child run did not report {test}:\n{report}");
    }
}
