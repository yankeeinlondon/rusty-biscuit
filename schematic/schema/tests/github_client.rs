//! Envelope, credential, and cross-origin behavior of the generated GitHub
//! client against a local wiremock server.
//!
//! GitHub is used because its definition names a request-id header
//! (`x-github-request-id`), reads `GITHUB_TOKEN` then `GH_TOKEN` as its
//! credential fallback, and keeps the default `same_origin` cross-origin
//! policy. Tests that touch those variables hold an [`EnvGuard`] and are
//! `#[serial]`; the others build clients with an empty env-var order so a
//! host token never reaches the mock.

mod common;

use common::EnvGuard;
use schematic_schema::github::{
    GetIssueRequest, GetRepositoryContentRawRequest, GetRepositoryRequest, GetTagReferenceRequest,
    ListOrgReposRequest, ListPullRequestsRequest,
};
use schematic_schema::prelude::*;
use schematic_schema::schematic_runtime::{
    AuthMethod, EndpointSpec, ErrorKind, Method, Outcome, RedirectRefusal, TransportErrorKind, Url,
};
use serial_test::serial;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

const TOKEN_VARS: [&str; 2] = ["GITHUB_TOKEN", "GH_TOKEN"];
const REPO_PATH: &str = "/repos/octo/widgets";

fn repository_json() -> serde_json::Value {
    serde_json::json!({
        "id": 7,
        "name": "widgets",
        "full_name": "octo/widgets",
        "owner": { "login": "octo" },
        "html_url": "https://github.com/octo/widgets",
        "url": "https://api.github.com/repos/octo/widgets",
        "default_branch": "main"
    })
}

/// A client that ignores the host's `GITHUB_TOKEN`/`GH_TOKEN`.
fn client_without_env(base_url: &str) -> GitHub {
    GitHub::new()
        .base_url(base_url)
        .env_order(Vec::<String>::new())
        .build()
        .expect("client builds")
}

fn authorization(request: &Request) -> Option<String> {
    request
        .headers
        .get("authorization")
        .map(|value| value.to_str().expect("ASCII header").to_string())
}

/// Every generated GitHub request names `GitHubApi` as its API, which is what
/// lets `GitHub::send` accept it; a request of another API sent through
/// this client is rejected at compile time (proven over real generated output
/// by schematic-gen's `slow_generated_types_reject_misuse_at_compile_time`).
#[test]
fn generated_requests_belong_to_their_own_api() {
    fn assert_api<R: EndpointSpec<Api = schematic_schema::github::GitHubApi>>() {}
    assert_api::<GetRepositoryRequest>();
    assert_api::<GetRepositoryContentRawRequest>();
    assert_api::<GetIssueRequest>();
    assert_api::<ListPullRequestsRequest>();
    assert_api::<ListOrgReposRequest>();
    assert_api::<GetTagReferenceRequest>();
}

/// A success exposes the request id, status, response headers, endpoint id,
/// method, and URL.
#[tokio::test]
async fn success_metadata_carries_request_id_status_and_headers() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(REPO_PATH))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-github-request-id", "ABCD:1234")
                .insert_header("x-ratelimit-remaining", "59")
                .set_body_json(repository_json()),
        )
        .mount(&mock_server)
        .await;

    let response = client_without_env(&mock_server.uri())
        .send(
            GetRepositoryRequest::new("octo", "widgets"),
            &CallOptions::default(),
        )
        .await
        .expect("a response head arrived");

    let meta = &response.meta;
    assert_eq!(meta.request_id.as_deref(), Some("ABCD:1234"));
    assert_eq!(meta.status.as_u16(), 200);
    assert_eq!(
        meta.headers
            .get("x-ratelimit-remaining")
            .map(|v| v.as_bytes()),
        Some(b"59".as_slice())
    );
    assert_eq!(meta.request.endpoint_id, GetRepositoryRequest::ENDPOINT_ID);
    assert_eq!(meta.request.endpoint_id, "GetRepository");
    assert_eq!(meta.request.method, Method::GET);
    assert!(
        meta.request.url.to_string().ends_with(REPO_PATH),
        "url: {}",
        meta.request.url
    );
    assert_eq!(response.data().expect("success").full_name, "octo/widgets");
}

/// A failure exposes the same metadata as a success, and no request id when
/// the reply carries none.
#[tokio::test]
async fn failure_metadata_carries_request_id_status_and_headers() {
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(REPO_PATH))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("x-github-request-id", "EEEE:404")
                .insert_header("x-ratelimit-remaining", "58")
                .set_body_json(serde_json::json!({ "message": "Not Found" })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;
    Mock::given(method("GET"))
        .and(path(REPO_PATH))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let client = client_without_env(&mock_server.uri());
    let response = client
        .send(
            GetRepositoryRequest::new("octo", "widgets"),
            &CallOptions::default(),
        )
        .await
        .expect("a response head arrived");

    assert_eq!(response.meta.request_id.as_deref(), Some("EEEE:404"));
    assert_eq!(response.meta.status.as_u16(), 404);
    assert_eq!(
        response
            .meta
            .headers
            .get("x-ratelimit-remaining")
            .map(|v| v.as_bytes()),
        Some(b"58".as_slice())
    );
    assert_eq!(response.meta.request.endpoint_id, "GetRepository");
    assert_eq!(response.meta.request.method, Method::GET);
    assert!(matches!(
        response.error().expect("failure").kind,
        ErrorKind::Undeclared
    ));

    let without_id = client
        .send(
            GetRepositoryRequest::new("octo", "widgets"),
            &CallOptions::default(),
        )
        .await
        .expect("a response head arrived");
    assert_eq!(without_id.meta.status.as_u16(), 500);
    assert_eq!(without_id.meta.request_id, None);
}

/// Per-call beats client beats `GITHUB_TOKEN` beats `GH_TOKEN` beats
/// anonymous: removing each winner lets the next one win, and each request
/// carries exactly the winner's credential.
#[tokio::test]
#[serial]
async fn credential_precedence_per_call_client_env_anonymous() {
    let env = EnvGuard::clear(&TOKEN_VARS);
    env.set("GITHUB_TOKEN", "env-primary");
    env.set("GH_TOKEN", "env-secondary");
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(REPO_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(repository_json()))
        .mount(&mock_server)
        .await;

    let with_client_credential = GitHub::new()
        .base_url(mock_server.uri())
        .credentials(Credential::access_token("client-token"))
        .build()
        .expect("client builds");
    let without_client_credential = GitHub::new()
        .base_url(mock_server.uri())
        .build()
        .expect("client builds");
    let per_call = CallOptions::new().credential(Credential::access_token("call-token"));
    let request = || GetRepositoryRequest::new("octo", "widgets");

    let mut sources = Vec::new();
    for (client, options) in [
        (&with_client_credential, &per_call),
        (&with_client_credential, &CallOptions::default()),
        (&without_client_credential, &CallOptions::default()),
    ] {
        let response = client
            .send(request(), options)
            .await
            .expect("a response head arrived");
        sources.push(response.meta.request.auth.clone());
    }
    env.remove("GITHUB_TOKEN");
    let response = without_client_credential
        .send(request(), &CallOptions::default())
        .await
        .expect("a response head arrived");
    sources.push(response.meta.request.auth.clone());
    env.remove("GH_TOKEN");
    let response = without_client_credential
        .send(request(), &CallOptions::default())
        .await
        .expect("a response head arrived");
    sources.push(response.meta.request.auth.clone());

    assert_eq!(
        sources,
        [
            AuthMethod::PerCall,
            AuthMethod::ClientLevel,
            AuthMethod::EnvVar("GITHUB_TOKEN".to_string()),
            AuthMethod::EnvVar("GH_TOKEN".to_string()),
            AuthMethod::Anonymous,
        ]
    );
    let received = mock_server
        .received_requests()
        .await
        .expect("recording is on");
    let sent: Vec<Option<String>> = received.iter().map(authorization).collect();
    assert_eq!(
        sent,
        [
            Some("Bearer call-token".to_string()),
            Some("Bearer client-token".to_string()),
            Some("Bearer env-primary".to_string()),
            Some("Bearer env-secondary".to_string()),
            None,
        ]
    );
}

/// With no credential anywhere the request is sent without one, and the 401
/// arrives as an envelope failure with its status, headers, and raw body.
#[tokio::test]
#[serial]
async fn no_credential_sends_anonymously_and_returns_the_401() {
    let _env = EnvGuard::clear(&TOKEN_VARS);
    let mock_server = MockServer::start().await;
    let rejection =
        br#"{"message":"Requires authentication","documentation_url":"https://docs.github.com"}"#;
    Mock::given(method("GET"))
        .and(path(REPO_PATH))
        .respond_with(
            ResponseTemplate::new(401)
                .insert_header("www-authenticate", "Bearer realm=\"GitHub\"")
                .set_body_raw(rejection.as_slice(), "application/json"),
        )
        .expect(1)
        .mount(&mock_server)
        .await;

    let response = GitHub::new()
        .base_url(mock_server.uri())
        .build()
        .expect("client builds")
        .send(
            GetRepositoryRequest::new("octo", "widgets"),
            &CallOptions::default(),
        )
        .await
        .expect("a 401 is a response, not a transport error");

    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    assert_eq!(response.meta.status.as_u16(), 401);
    assert!(response.meta.headers.contains_key("www-authenticate"));
    let error = response.error().expect("failure outcome");
    assert!(
        matches!(error.kind, ErrorKind::Undeclared),
        "kind: {}",
        error.kind
    );
    assert_eq!(error.body.as_ref(), rejection.as_slice());
    let received = mock_server
        .received_requests()
        .await
        .expect("recording is on");
    assert_eq!(authorization(&received[0]), None);
}

/// `CallOptions::anonymous()` sends no credential although both a client
/// credential and the env fallback are available.
#[tokio::test]
#[serial]
async fn anonymous_call_options_send_no_credential_despite_client_and_env() {
    let env = EnvGuard::clear(&TOKEN_VARS);
    env.set("GITHUB_TOKEN", "env-primary");
    let mock_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(REPO_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(repository_json()))
        .expect(1)
        .mount(&mock_server)
        .await;

    let response = GitHub::new()
        .base_url(mock_server.uri())
        .credentials(Credential::access_token("client-token"))
        .build()
        .expect("client builds")
        .send(
            GetRepositoryRequest::new("octo", "widgets"),
            &CallOptions::anonymous(),
        )
        .await
        .expect("a response head arrived");

    assert_eq!(response.meta.request.auth, AuthMethod::Anonymous);
    let received = mock_server
        .received_requests()
        .await
        .expect("recording is on");
    assert_eq!(authorization(&received[0]), None);
}

/// Under GitHub's `same_origin` policy, an absolute link on the base origin
/// passes and one on another origin is refused before any request to it.
#[tokio::test]
async fn check_link_accepts_same_origin_and_rejects_a_foreign_origin() {
    let base = MockServer::start().await;
    let foreign = MockServer::start().await;
    let client = client_without_env(&base.uri());

    let same_origin =
        Url::parse(&format!("{}/repos/octo/widgets/pulls?page=2", base.uri())).expect("url");
    let elsewhere = Url::parse(&format!(
        "{}/repos/octo/widgets/pulls?page=2",
        foreign.uri()
    ))
    .expect("url");

    client
        .check_link::<ListPullRequestsRequest>(&same_origin)
        .expect("a same-origin link passes");
    let refused = client
        .check_link::<ListPullRequestsRequest>(&elsewhere)
        .expect_err("a foreign origin is refused");

    match &refused.kind {
        TransportErrorKind::CrossOriginRejected { target } => {
            assert!(
                target.to_string().starts_with(&foreign.uri()),
                "target: {target}"
            );
        }
        other => panic!("expected CrossOriginRejected, got {other}"),
    }
    assert_eq!(refused.request.endpoint_id, "ListPullRequests");
    assert!(
        foreign
            .received_requests()
            .await
            .expect("recording is on")
            .is_empty()
    );
}

/// Under `same_origin`, a redirect to the base origin is followed with the
/// credential, and a redirect to another origin is not followed at all: the
/// foreign server sees no request and the envelope shows the refused 302.
#[tokio::test]
async fn redirects_follow_the_same_origin_policy() {
    let base = MockServer::start().await;
    let foreign = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(REPO_PATH))
        .respond_with(ResponseTemplate::new(302).insert_header("location", "/repos/octo/renamed"))
        .mount(&base)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/octo/renamed"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repository_json()))
        .mount(&base)
        .await;
    Mock::given(method("GET"))
        .and(path("/repos/octo/moved"))
        .respond_with(ResponseTemplate::new(302).insert_header(
            "location",
            format!("{}/repos/octo/widgets", foreign.uri()).as_str(),
        ))
        .mount(&base)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(repository_json()))
        .mount(&foreign)
        .await;

    let client = GitHub::new()
        .base_url(base.uri())
        .env_order(Vec::<String>::new())
        .credentials(Credential::access_token("client-token"))
        .build()
        .expect("client builds");

    let followed = client
        .send(
            GetRepositoryRequest::new("octo", "widgets"),
            &CallOptions::default(),
        )
        .await
        .expect("a response head arrived");
    assert_eq!(followed.meta.status.as_u16(), 200);
    assert!(
        followed
            .meta
            .request
            .url
            .to_string()
            .ends_with("/repos/octo/renamed")
    );
    let base_requests = base.received_requests().await.expect("recording is on");
    assert_eq!(base_requests.len(), 2);
    assert_eq!(
        authorization(&base_requests[1]).as_deref(),
        Some("Bearer client-token")
    );

    let refused = client
        .send(
            GetRepositoryRequest::new("octo", "moved"),
            &CallOptions::default(),
        )
        .await
        .expect("the refused redirect is a response");
    assert_eq!(refused.meta.status.as_u16(), 302);
    match &refused.outcome {
        Outcome::Failure(error) => assert!(
            matches!(
                error.kind,
                ErrorKind::RedirectRefused {
                    reason: RedirectRefusal::CrossOrigin
                }
            ),
            "kind: {}",
            error.kind
        ),
        Outcome::Success(_) => panic!("a cross-origin redirect must not be followed"),
    }
    assert!(
        foreign
            .received_requests()
            .await
            .expect("recording is on")
            .is_empty()
    );
}

/// A 2xx whose body does not decode is a `MalformedSuccess` failure with the
/// exact body bytes and the 200 status kept; a JSON endpoint receiving
/// non-JSON content lands here too.
#[tokio::test]
async fn undecodable_success_body_is_malformed_success_with_raw_bytes() {
    let mock_server = MockServer::start().await;
    let body = b"<html>rate limited \xff</html>";
    Mock::given(method("GET"))
        .and(path(REPO_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_raw(body.as_slice(), "text/html"))
        .mount(&mock_server)
        .await;

    let response = client_without_env(&mock_server.uri())
        .send(
            GetRepositoryRequest::new("octo", "widgets"),
            &CallOptions::default(),
        )
        .await
        .expect("a response head arrived");

    assert!(response.is_decode_failure());
    assert!(!response.is_interrupted());
    assert_eq!(response.meta.status.as_u16(), 200);
    assert_eq!(
        response
            .meta
            .headers
            .get("content-type")
            .map(|v| v.as_bytes()),
        Some(b"text/html".as_slice())
    );
    let error = response.into_result().expect_err("not a success");
    assert!(
        matches!(error.kind, ErrorKind::MalformedSuccess),
        "kind: {}",
        error.kind
    );
    assert!(error.is_decode_failure());
    assert_eq!(error.body.as_ref(), body.as_slice());
}
