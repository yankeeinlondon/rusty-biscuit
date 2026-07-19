#![cfg(feature = "remote")]

use biscuit_file::FetchPolicy;
use sniff::SniffError;
use sniff::filesystem::git::{ApiFlavor, ResolvedRemote};
use sniff::remote::{
    CiCdJobQuery, CiCdJobReference, FocusedProviderClient, GitProvider, PullRequestQuery,
};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn remote(flavor: ApiFlavor) -> ResolvedRemote {
    ResolvedRemote {
        name: "origin".to_string(),
        fetch_url: "git@127.0.0.1:acme/project.git".to_string(),
        push_url: "git@127.0.0.1:acme/project.git".to_string(),
        host: Some("127.0.0.1".to_string()),
        namespace: Some("acme".to_string()),
        repository: Some("project".to_string()),
        api_flavor: flavor,
    }
}

fn provider(flavor: ApiFlavor) -> GitProvider {
    match flavor {
        ApiFlavor::GitHub => GitProvider::GitHub,
        ApiFlavor::GitLab => GitProvider::GitLab,
        ApiFlavor::Gitea | ApiFlavor::Forgejo => GitProvider::Gitea,
        ApiFlavor::Bitbucket => GitProvider::Bitbucket,
        _ => unreachable!(),
    }
}

fn client(server: &MockServer, flavor: ApiFlavor) -> FocusedProviderClient {
    FocusedProviderClient::with_api_base(
        remote(flavor),
        FetchPolicy::deny_all().allow_host("127.0.0.1"),
        &format!("{}/api", server.uri()),
    ).unwrap()
}

fn reference(flavor: ApiFlavor, id: &str) -> CiCdJobReference {
    CiCdJobReference {
        provider: provider(flavor),
        api_flavor: format!("{flavor:?}"),
        host: "example.com".to_string(),
        namespace: "acme".to_string(),
        repository: "project".to_string(),
        native_id: id.to_string(),
        display_id: id.to_string(),
        original_url: None,
    }
}

#[test]
fn canonical_pr_and_job_urls_preserve_repository_scoped_identity() {
    for (url, id) in [
        ("https://github.com/acme/project/pull/7", "7"),
        ("https://gitlab.com/group/sub/project/-/merge_requests/8", "8"),
        ("https://forgejo.example/acme/project/pulls/9", "9"),
        ("https://bitbucket.org/acme/project/pull-requests/10", "10"),
    ] {
        let (_, parsed) = FocusedProviderClient::from_pull_request_url(
            url,
            FetchPolicy::deny_all(),
        )
        .unwrap();
        assert_eq!(parsed, id);
    }

    for (url, native_id, namespace) in [
        ("https://github.com/acme/project/actions/runs/20/job/21", "21", "acme"),
        ("https://gitlab.com/group/sub/project/-/jobs/22", "22", "group/sub"),
        (
            "https://bitbucket.org/acme/project/pipelines/results/p1/steps/s1",
            "p1/s1",
            "acme",
        ),
    ] {
        let (remote, reference) = FocusedProviderClient::job_reference_from_url(url).unwrap();
        assert_eq!(reference.native_id, native_id);
        assert_eq!(reference.original_url.as_deref(), Some(url));
        assert_eq!(remote.namespace.as_deref(), Some(namespace));
        assert_eq!(remote.repository.as_deref(), Some("project"));
    }

    assert!(FocusedProviderClient::from_pull_request_url(
        "https://github.com/acme/project/issues/7",
        FetchPolicy::deny_all(),
    )
    .is_err());
}

#[tokio::test]
async fn exact_pull_requests_preserve_identity_and_authoritative_not_found() {
    let cases = [
        (ApiFlavor::GitHub, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://github.example/pr/7", "url": "https://api.example/pr/7"})),
        (ApiFlavor::GitLab, "/api/projects/acme%2Fproject/merge_requests/7", "/api/projects/acme%2Fproject/merge_requests/8", serde_json::json!({"iid": 7, "title": "Fix", "state": "opened", "author": {"username": "alice"}, "created_at": "2024-01-01", "web_url": "https://gitlab.example/mr/7"})),
        (ApiFlavor::Gitea, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://gitea.example/pr/7"})),
        (ApiFlavor::Forgejo, "/api/repos/acme/project/pulls/7", "/api/repos/acme/project/pulls/8", serde_json::json!({"number": 7, "title": "Fix", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-01", "html_url": "https://forgejo.example/pr/7"})),
        (ApiFlavor::Bitbucket, "/api/repositories/acme/project/pullrequests/7", "/api/repositories/acme/project/pullrequests/8", serde_json::json!({"id": 7, "title": "Fix", "state": "OPEN", "author": {"display_name": "alice"}, "created_on": "2024-01-01", "links": {"html": {"href": "https://bitbucket.example/pr/7"}}})),
    ];
    for (flavor, found_path, missing_path, body) in cases {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path(found_path)).respond_with(ResponseTemplate::new(200).set_body_json(body)).expect(1).mount(&server).await;
        Mock::given(method("GET")).and(path(missing_path)).respond_with(ResponseTemplate::new(404)).expect(1).mount(&server).await;
        let adapter = client(&server, flavor);
        let record = adapter.get_pull_request("7").await.unwrap().unwrap();
        assert_eq!(record.identity.native_id, "7");
        assert_eq!(record.identity.namespace, "acme");
        assert_eq!(record.identity.repository, "project");
        assert_eq!(record.details.author, "alice");
        assert!(adapter.get_pull_request("8").await.unwrap().is_none());
    }
}

#[tokio::test]
async fn exact_jobs_are_normalized_for_every_initial_flavor() {
    let cases = [
        (ApiFlavor::GitHub, "10", "/api/repos/acme/project/actions/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "completed", "conclusion": "success", "run_id": 1})),
        (ApiFlavor::GitLab, "10", "/api/projects/acme%2Fproject/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "success", "pipeline_id": 1})),
        (ApiFlavor::Gitea, "10", "/api/repos/acme/project/actions/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "success", "run_id": 1})),
        (ApiFlavor::Forgejo, "10", "/api/repos/acme/project/actions/jobs/10", serde_json::json!({"id": 10, "name": "test", "status": "success", "run_id": 1})),
        (ApiFlavor::Bitbucket, "parent/step", "/api/repositories/acme/project/pipelines/parent/steps/step", serde_json::json!({"uuid": "step", "name": "test", "state": "success"})),
    ];
    for (flavor, id, endpoint, body) in cases {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(endpoint))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;
        let job = client(&server, flavor).get_cicd_job(&reference(flavor, id)).await.unwrap().unwrap();
        assert_eq!(job.name, "test");
        assert_eq!(job.normalized_status, "success");
        assert!(!job.parent.native_id.is_empty());
    }
}

#[tokio::test]
async fn job_listing_uses_direct_or_bounded_parent_strategy_for_every_flavor() {
    for flavor in [ApiFlavor::GitHub, ApiFlavor::GitLab, ApiFlavor::Gitea, ApiFlavor::Forgejo, ApiFlavor::Bitbucket] {
        let server = MockServer::start().await;
        if flavor == ApiFlavor::GitLab {
            Mock::given(method("GET"))
                .and(path("/api/projects/acme%2Fproject/jobs"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                    {"id": 10, "name": "test", "status": "success", "pipeline_id": 1}
                ])))
                .expect(1)
                .mount(&server)
                .await;
        } else {
            let (parents_path, jobs_path, parent_body, jobs_body) = if flavor == ApiFlavor::Bitbucket {
                ("/api/repositories/acme/project/pipelines", "/api/repositories/acme/project/pipelines/p1/steps", serde_json::json!({"values": [{"uuid": "p1"}]}), serde_json::json!({"values": [{"uuid": "s1", "name": "test", "state": "success"}]}))
            } else {
                ("/api/repos/acme/project/actions/runs", "/api/repos/acme/project/actions/runs/1/jobs", serde_json::json!({"workflow_runs": [{"id": 1, "name": "CI"}]}), serde_json::json!({"jobs": [{"id": 10, "name": "test", "status": "success"}]}))
            };
            Mock::given(method("GET")).and(path(parents_path)).respond_with(ResponseTemplate::new(200).set_body_json(parent_body)).expect(1).mount(&server).await;
            Mock::given(method("GET")).and(path(jobs_path)).respond_with(ResponseTemplate::new(200).set_body_json(jobs_body)).expect(1).mount(&server).await;
        }
        let page = client(&server, flavor).query_cicd_jobs(CiCdJobQuery { limit: Some(2), ..Default::default() }).await.unwrap();
        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].name, "test");
    }
}

#[tokio::test]
async fn pull_request_query_paginates_until_filtered_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .and(query_param("page", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"number": 1, "title": "skip", "state": "open", "user": {"login": "bob"}, "created_at": "2024-01-01", "html_url": "https://example/pr/1"},
            {"number": 2, "title": "keep", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-02", "html_url": "https://example/pr/2"}
        ])))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .and(query_param("page", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"number": 3, "title": "keep again", "state": "open", "user": {"login": "alice"}, "created_at": "2024-01-03", "html_url": "https://example/pr/3"}
        ])))
        .expect(1)
        .mount(&server)
        .await;
    let page = client(&server, ApiFlavor::GitHub).query_pull_requests(PullRequestQuery {
        author: Some("alice".to_string()),
        limit: Some(2),
        ..Default::default()
    }).await.unwrap();
    assert_eq!(page.items.iter().map(|item| item.identity.native_id.as_str()).collect::<Vec<_>>(), ["2", "3"]);
}

/// Builds a query exercising exactly one canonical field.
///
/// `direction` is the catalog-facing name for the struct's `descending` flag,
/// so it has no serde key of its own.
fn single_filter_query(field: &str) -> PullRequestQuery {
    if field == "direction" {
        return PullRequestQuery { descending: true, limit: Some(5), ..Default::default() };
    }
    let value = match field {
        "state" => serde_json::json!(["open"]),
        "labels" => serde_json::json!(["bug"]),
        "draft" => serde_json::json!(true),
        "limit" => serde_json::json!(5),
        "sort" => serde_json::json!("created"),
        "created_after" | "created_before" | "updated_after" | "updated_before" => {
            serde_json::json!("2024-01-01T00:00:00Z")
        }
        _ => serde_json::json!("x"),
    };
    serde_json::from_value(serde_json::json!({ field: value, "limit": 5 })).unwrap()
}

/// AC22: `capabilities()` is a public promise, not documentation. Every filter
/// it declares must actually be honored, and every filter it omits from the
/// canonical query vocabulary must be refused rather than silently dropped.
#[tokio::test]
async fn declared_filters_match_the_filters_the_client_actually_honors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([])))
        .mount(&server)
        .await;
    let adapter = client(&server, ApiFlavor::GitHub);
    let declared = adapter.capabilities().pull_request_filters;

    for field in &declared {
        adapter
            .query_pull_requests(single_filter_query(field))
            .await
            .unwrap_or_else(|error| panic!("declared filter {field} was not honored: {error}"));
    }

    for field in ["assignee", "reviewer", "milestone", "commit"] {
        assert!(
            !declared.iter().any(|declared| declared == field),
            "{field} is rejected at runtime but still advertised"
        );
        assert!(
            matches!(
                adapter.query_pull_requests(single_filter_query(field)).await,
                Err(SniffError::UnsupportedRemoteFilter { .. })
            ),
            "{field} was silently ignored instead of refused"
        );
    }

    let capabilities = adapter.capabilities();
    assert!(!capabilities.logs);
    assert!(!capabilities.artifacts);
    assert!(!capabilities.test_reports);
}

#[tokio::test]
async fn denial_validation_and_provider_failures_remain_distinct() {
    let server = MockServer::start().await;
    let denied = FocusedProviderClient::with_api_base(remote(ApiFlavor::GitHub), FetchPolicy::deny_all(), &format!("{}/api", server.uri())).unwrap();
    assert!(matches!(denied.get_pull_request("1").await, Err(SniffError::RemotePolicyDenied { .. })));
    assert!(server.received_requests().await.unwrap().is_empty());

    let mut mismatched_remote = remote(ApiFlavor::GitHub);
    mismatched_remote.host = Some("provider.example".to_string());
    let mismatched = FocusedProviderClient::with_api_base(
        mismatched_remote,
        FetchPolicy::deny_all().allow_host("provider.example"),
        &format!("{}/api", server.uri()),
    )
    .unwrap();
    assert!(matches!(
        mismatched.get_pull_request("1").await,
        Err(SniffError::RemotePolicyDenied { .. })
    ));
    assert!(server.received_requests().await.unwrap().is_empty());

    let unsupported = client(&server, ApiFlavor::GitHub).query_pull_requests(PullRequestQuery {
        reviewer: Some("alice".to_string()),
        ..Default::default()
    }).await.unwrap_err();
    assert!(matches!(unsupported, SniffError::UnsupportedRemoteFilter { field: "reviewer", .. }));
    assert!(server.received_requests().await.unwrap().is_empty());

    for (status, expected) in [(401, "auth"), (403, "forbidden"), (429, "rate") ] {
        let server = MockServer::start().await;
        Mock::given(method("GET")).and(path("/api/repos/acme/project/pulls/1")).respond_with(ResponseTemplate::new(status)).expect(1).mount(&server).await;
        let error = client(&server, ApiFlavor::GitHub).get_pull_request("1").await.unwrap_err();
        match expected {
            "auth" => assert!(matches!(error, SniffError::MissingCredentials { .. } | SniffError::InvalidCredentials { .. })),
            "forbidden" => assert!(matches!(error, SniffError::RemoteForbidden { .. })),
            _ => assert!(matches!(error, SniffError::RateLimited { .. })),
        }
    }

    let server = MockServer::start().await;
    Mock::given(method("GET")).and(path("/api/repos/acme/project/pulls/1")).respond_with(ResponseTemplate::new(200).set_body_string("not json")).expect(1).mount(&server).await;
    assert!(matches!(client(&server, ApiFlavor::GitHub).get_pull_request("1").await, Err(SniffError::RemoteApi { status: 200, .. })));

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/repos/acme/project/pulls/1"))
        .respond_with(
            ResponseTemplate::new(302).insert_header("location", "https://evil.example/leak"),
        )
        .expect(1)
        .mount(&server)
        .await;
    assert!(matches!(
        client(&server, ApiFlavor::GitHub).get_pull_request("1").await,
        Err(SniffError::RemoteUnreachable { .. })
    ));

    let unavailable = FocusedProviderClient::with_api_base(remote(ApiFlavor::GitHub), FetchPolicy::deny_all().allow_host("127.0.0.1"), "http://127.0.0.1:9/api").unwrap();
    assert!(matches!(unavailable.get_pull_request("1").await, Err(SniffError::RemoteUnreachable { .. })));
}
