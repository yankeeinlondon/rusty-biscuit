//! HTTP mock server integration tests for the fetch primitive.

use biscuit_file::file_reference::fetch::policy_client;
use biscuit_file::{Conditional, FetchError, FetchPolicy, HostPattern, fetch, fetch_blocking};
use reqwest::Client;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn setup_server() -> (MockServer, Client, FetchPolicy) {
    let server = MockServer::start().await;
    let client = Client::new();
    let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
    (server, client, policy)
}

#[tokio::test]
async fn fetch_200_returns_body_and_headers() {
    let (server, client, policy) = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/doc.md"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string("hello world")
                .insert_header("etag", "\"abc123\"")
                .insert_header("last-modified", "Mon, 02 Jun 2026 00:00:00 GMT")
                .insert_header("cache-control", "max-age=3600"),
        )
        .mount(&server)
        .await;

    let url = url::Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
    let resp = fetch(&client, &url, &policy, &Conditional::default())
        .await
        .unwrap();

    assert_eq!(resp.status, 200);
    assert_eq!(resp.body.as_ref(), b"hello world");
    assert_eq!(resp.etag.as_deref(), Some("\"abc123\""));
    assert_eq!(
        resp.last_modified.as_deref(),
        Some("Mon, 02 Jun 2026 00:00:00 GMT")
    );
    assert_eq!(resp.cache_control.as_deref(), Some("max-age=3600"));
    assert!(resp.is_success());
    assert!(!resp.is_not_modified());
}

#[tokio::test]
async fn fetch_304_not_modified_preserves_headers() {
    let (server, client, policy) = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/cached"))
        .and(header("If-None-Match", "\"old\""))
        .respond_with(
            ResponseTemplate::new(304)
                .insert_header("etag", "\"new\"")
                .insert_header("cache-control", "max-age=7200"),
        )
        .mount(&server)
        .await;

    let url = url::Url::parse(&format!("{}/cached", server.uri())).unwrap();
    let conditional = Conditional {
        if_none_match: Some("\"old\"".to_string()),
        if_modified_since: None,
    };
    let resp = fetch(&client, &url, &policy, &conditional)
        .await
        .unwrap();

    assert_eq!(resp.status, 304);
    assert!(resp.is_not_modified());
    assert!(!resp.is_success());
    assert_eq!(resp.etag.as_deref(), Some("\"new\""));
    assert!(resp.body.is_empty());
}

#[tokio::test]
async fn fetch_404_returns_http_error() {
    let (server, client, policy) = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/missing"))
        .respond_with(ResponseTemplate::new(404))
        .mount(&server)
        .await;

    let url = url::Url::parse(&format!("{}/missing", server.uri())).unwrap();
    let err = fetch(&client, &url, &policy, &Conditional::default())
        .await
        .unwrap_err();

    match err {
        FetchError::HttpError { status, url: _ } => assert_eq!(status, 404),
        other => panic!("expected HttpError, got: {other}"),
    }
}

#[tokio::test]
async fn fetch_500_returns_http_error() {
    let (server, client, policy) = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/error"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&server)
        .await;

    let url = url::Url::parse(&format!("{}/error", server.uri())).unwrap();
    let err = fetch(&client, &url, &policy, &Conditional::default())
        .await
        .unwrap_err();

    match err {
        FetchError::HttpError { status, url: _ } => assert_eq!(status, 500),
        other => panic!("expected HttpError, got: {other}"),
    }
}

#[tokio::test]
async fn fetch_policy_denied_host() {
    let server = MockServer::start().await;
    let client = Client::new();
    let policy = FetchPolicy::deny_all();

    let url = url::Url::parse(&format!("{}/doc.md", server.uri())).unwrap();
    let err = fetch(&client, &url, &policy, &Conditional::default())
        .await
        .unwrap_err();

    match err {
        FetchError::PolicyDenied { host } => assert_eq!(host, "127.0.0.1"),
        other => panic!("expected PolicyDenied, got: {other}"),
    }
}

#[tokio::test]
async fn fetch_unsupported_scheme() {
    let client = Client::new();
    let policy = FetchPolicy::deny_all().allow_host("example.com");
    let url = url::Url::parse("ftp://example.com/file.txt").unwrap();

    let err = fetch(&client, &url, &policy, &Conditional::default())
        .await
        .unwrap_err();

    match err {
        FetchError::UnsupportedScheme(s) => assert_eq!(s, "ftp"),
        other => panic!("expected UnsupportedScheme, got: {other}"),
    }
}

#[tokio::test]
async fn fetch_conditional_if_modified_since() {
    let (server, client, policy) = setup_server().await;

    Mock::given(method("GET"))
        .and(path("/stale"))
        .respond_with(ResponseTemplate::new(200).set_body_string("fresh content"))
        .mount(&server)
        .await;

    let url = url::Url::parse(&format!("{}/stale", server.uri())).unwrap();
    let conditional = Conditional {
        if_none_match: None,
        if_modified_since: Some("Tue, 01 Jan 2025 00:00:00 GMT".to_string()),
    };
    let resp = fetch(&client, &url, &policy, &conditional)
        .await
        .unwrap();

    assert_eq!(resp.status, 200);
    assert_eq!(resp.body.as_ref(), b"fresh content");
}

#[tokio::test]
async fn fetch_wildcard_does_not_match_bare_parent_host() {
    // The mock server is reached at the bare host `127.0.0.1`. A wildcard over
    // that host delegates subdomains only, so the request must be policy-denied
    // before any network access — exact-host coverage lives in the other tests
    // via `setup_server`.
    let server = MockServer::start().await;
    let client = Client::new();
    let policy = FetchPolicy::deny_all().allow(HostPattern::Wildcard("127.0.0.1".to_string()));

    let url = url::Url::parse(&format!("{}/ok", server.uri())).unwrap();
    let err = fetch(&client, &url, &policy, &Conditional::default())
        .await
        .unwrap_err();

    assert!(matches!(err, FetchError::PolicyDenied { .. }));
}

#[tokio::test]
async fn policy_client_blocks_redirect_to_unallowed_host() {
    // The allowed host serves a 302 pointing at a host the policy never
    // authorized. `policy_client()` does not follow redirects, so the primitive
    // surfaces the 302 as `RedirectBlocked` instead of fetching the blocked
    // host's body — the SSRF bypass the redirect would otherwise open.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/redirect"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "http://blocked-host.example/secret"),
        )
        .mount(&server)
        .await;

    let client = policy_client();
    let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
    let url = url::Url::parse(&format!("{}/redirect", server.uri())).unwrap();

    let err = fetch(&client, &url, &policy, &Conditional::default())
        .await
        .unwrap_err();

    match err {
        FetchError::RedirectBlocked { status, location } => {
            assert_eq!(status, 302);
            assert_eq!(location, "http://blocked-host.example/secret");
        }
        other => panic!("expected RedirectBlocked, got: {other}"),
    }
}

#[tokio::test]
async fn policy_client_blocks_redirect_to_unsupported_scheme() {
    // A redirect to a non-HTTP(S) scheme is blocked at the redirect itself,
    // before any scheme handling — the request is never re-issued.
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/to-ftp"))
        .respond_with(
            ResponseTemplate::new(301).insert_header("location", "ftp://blocked-host.example/file"),
        )
        .mount(&server)
        .await;

    let client = policy_client();
    let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
    let url = url::Url::parse(&format!("{}/to-ftp", server.uri())).unwrap();

    let err = fetch(&client, &url, &policy, &Conditional::default())
        .await
        .unwrap_err();

    match err {
        FetchError::RedirectBlocked { status, location } => {
            assert_eq!(status, 301);
            assert_eq!(location, "ftp://blocked-host.example/file");
        }
        other => panic!("expected RedirectBlocked, got: {other}"),
    }
}

#[test]
fn fetch_blocking_works() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let server = rt.block_on(async {
        let server = wiremock::MockServer::start().await;
        wiremock::Mock::given(wiremock::matchers::method("GET"))
            .and(wiremock::matchers::path("/sync"))
            .respond_with(ResponseTemplate::new(200).set_body_string("sync body"))
            .mount(&server)
            .await;
        server
    });

    let client = Client::new();
    let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");
    let url = url::Url::parse(&format!("{}/sync", server.uri())).unwrap();

    let resp = fetch_blocking(&client, &url, &policy, &Conditional::default()).unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body.as_ref(), b"sync body");
}
