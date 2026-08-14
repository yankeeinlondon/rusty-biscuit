#![cfg(feature = "network")]

use biscuit_file::FetchPolicy;
use base64::Engine;
use serial_test::serial;
use sniff::SniffError;
use sniff::filesystem::git::{branch_exists_on_remote_at, remote_vendor_at};
use test_toolkit::EnvGuard;
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn repository(remote_url: &str) -> (tempfile::TempDir, git2::Repository) {
    let directory = tempfile::tempdir().unwrap();
    let repository = git2::Repository::init(directory.path()).unwrap();
    repository.remote("origin", remote_url).unwrap();
    (directory, repository)
}

fn advertisement(refs: &[&str]) -> Vec<u8> {
    let mut body = Vec::new();
    for (index, reference) in refs.iter().enumerate() {
        let capabilities = if index == 0 { "\0multi_ack" } else { "" };
        let packet = format!("0000000000000000000000000000000000000000 {reference}{capabilities}\n");
        body.extend_from_slice(format!("{:04x}", packet.len() + 4).as_bytes());
        body.extend_from_slice(packet.as_bytes());
    }
    body.extend_from_slice(b"0000");
    body
}

async fn discovered_vendor(server: &MockServer) -> Result<String, SniffError> {
    let (directory, _) = repository(&format!("{}/acme/project.git", server.uri()));
    let root = directory.path().to_path_buf();
    tokio::task::spawn_blocking(move || {
        remote_vendor_at(
            &root,
            None,
            &FetchPolicy::deny_all().allow_host("127.0.0.1"),
        )
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn branch_observation_reads_live_advertisement_without_local_mutation() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/acme/project.git/info/refs"))
        .and(query_param("service", "git-upload-pack"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(advertisement(&[
            "refs/heads/main",
            "refs/heads/release",
        ])))
        .expect(3)
        .mount(&server)
        .await;
    let (directory, repository) = repository(&format!("{}/acme/project.git", server.uri()));
    let config_before = std::fs::read_to_string(repository.path().join("config")).unwrap();
    let refs_before = repository.references().unwrap().count();
    let policy = FetchPolicy::deny_all().allow_host("127.0.0.1");

    let root = directory.path().to_path_buf();
    let policy_copy = policy.clone();
    assert!(tokio::task::spawn_blocking(move || {
        branch_exists_on_remote_at(&root, Some("release"), None, &policy_copy)
    }).await.unwrap().unwrap());
    let root = directory.path().to_path_buf();
    assert!(!tokio::task::spawn_blocking(move || {
        branch_exists_on_remote_at(&root, Some("missing"), None, &policy)
    }).await.unwrap().unwrap());
    let root = directory.path().to_path_buf();
    assert!(tokio::task::spawn_blocking(move || {
        branch_exists_on_remote_at(
            &root,
            Some("refs/heads/release"),
            None,
            &FetchPolicy::deny_all().allow_host("127.0.0.1"),
        )
    }).await.unwrap().unwrap());

    assert_eq!(std::fs::read_to_string(repository.path().join("config")).unwrap(), config_before);
    assert_eq!(repository.references().unwrap().count(), refs_before);
}

#[tokio::test]
async fn invalid_branch_is_rejected_before_any_request() {
    let server = MockServer::start().await;
    let (directory, _) = repository(&format!("{}/acme/project.git", server.uri()));
    let root = directory.path().to_path_buf();
    let error = tokio::task::spawn_blocking(move || {
        branch_exists_on_remote_at(
            &root,
            Some("refs/tags/v1"),
            None,
            &FetchPolicy::deny_all().allow_host("127.0.0.1"),
        )
    }).await.unwrap().unwrap_err();
    assert!(matches!(error, SniffError::Git { operation: "remote_branch_name", .. }));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn deny_policy_rejects_before_any_request() {
    let server = MockServer::start().await;
    let (directory, _) = repository(&format!("{}/acme/project.git", server.uri()));
    let root = directory.path().to_path_buf();
    let error = tokio::task::spawn_blocking(move || {
        branch_exists_on_remote_at(&root, Some("main"), None, &FetchPolicy::deny_all())
    }).await.unwrap().unwrap_err();
    assert!(matches!(error, SniffError::RemotePolicyDenied { .. }));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[test]
fn unknown_ssh_provider_is_reported_as_an_explicit_capability_gap() {
    let (directory, _) = repository("git@example.com:acme/project.git");
    assert!(matches!(
        branch_exists_on_remote_at(
            directory.path(),
            Some("main"),
            None,
            &FetchPolicy::deny_all().allow_host("example.com"),
        ),
        Err(SniffError::UnsupportedRemoteCapability {
            capability: "provider branch lookup",
            ..
        })
    ));
}

#[tokio::test]
async fn redirect_and_rate_limit_errors_remain_distinct() {
    for (status, rate_limited) in [(302, false), (429, true)] {
        let server = MockServer::start().await;
        let mut response = ResponseTemplate::new(status);
        if status == 302 {
            response = response.insert_header("location", "https://elsewhere.invalid/repo");
        }
        Mock::given(method("GET"))
            .and(path("/acme/project.git/info/refs"))
            .respond_with(response)
            .expect(1)
            .mount(&server)
            .await;
        let (directory, _) = repository(&format!("{}/acme/project.git", server.uri()));
        let root = directory.path().to_path_buf();
        let error = tokio::task::spawn_blocking(move || {
            branch_exists_on_remote_at(
                &root,
                Some("main"),
                None,
                &FetchPolicy::deny_all().allow_host("127.0.0.1"),
            )
        }).await.unwrap().unwrap_err();
        if rate_limited {
            assert!(matches!(error, SniffError::RateLimited { .. }));
        } else {
            assert!(matches!(error, SniffError::RemoteUnreachable { ref message, .. } if message.contains("redirect")));
        }
    }
}

#[tokio::test]
async fn vendor_detection_is_local_when_unambiguous_and_allowlisted_when_probed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "version": "Forgejo 9.0"
        })))
        .expect(1)
        .mount(&server)
        .await;
    let (directory, _) = repository(&format!("{}/acme/project.git", server.uri()));
    let root = directory.path().to_path_buf();
    let vendor = tokio::task::spawn_blocking(move || {
        remote_vendor_at(
            &root,
            None,
            &FetchPolicy::deny_all().allow_host("127.0.0.1"),
        )
    }).await.unwrap().unwrap();
    assert_eq!(vendor, "forgejo");

    let (github_directory, _) = repository("https://github.com/acme/project.git");
    assert_eq!(
        remote_vendor_at(github_directory.path(), None, &FetchPolicy::deny_all()).unwrap(),
        "github"
    );
}

#[tokio::test]
async fn ambiguous_discovery_distinguishes_all_six_server_flavors() {
    let cases = [
        ("/api/v3/meta", serde_json::json!({ "installed_version": "3.16.1" }), "github"),
        (
            "/api/v4/version",
            serde_json::json!({ "version": "17.2.0", "revision": "a1b2c3d4" }),
            "gitlab",
        ),
        ("/api/v1/version", serde_json::json!({ "version": "1.25.0" }), "gitea"),
        ("/api/v1/version", serde_json::json!({ "version": "Forgejo 14.0.0" }), "forgejo"),
        (
            "/rest/api/1.0/application-properties",
            serde_json::json!({ "displayName": "Bitbucket", "version": "9.4.1" }),
            "bitbucket",
        ),
        (
            "/_apis/connectionData",
            serde_json::json!({
                "instanceId": "9e3f8106-1d6b-4ff8-9a7e-10e1fd9ab06f",
                "deploymentId": "880d3b0c-9d90-4c7f-bb08-a2d1e159b4b2",
                "deploymentType": "OnPremises"
            }),
            "azure_devops",
        ),
    ];

    for (signature_path, body, expected) in cases {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(signature_path))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&server)
            .await;

        assert_eq!(discovered_vendor(&server).await.unwrap(), expected);
        assert_eq!(server.received_requests().await.unwrap().len(), 5);
    }
}

#[tokio::test]
async fn azure_discovery_requires_documented_on_premises_identity() {
    let hosted = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/_apis/connectionData"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "instanceId": "9e3f8106-1d6b-4ff8-9a7e-10e1fd9ab06f",
            "deploymentId": "880d3b0c-9d90-4c7f-bb08-a2d1e159b4b2",
            "deploymentType": "Hosted"
        })))
        .expect(1)
        .mount(&hosted)
        .await;
    assert!(matches!(
        discovered_vendor(&hosted).await,
        Err(SniffError::UnsupportedProvider { .. })
    ));

    for body in [
        serde_json::json!({
            "deploymentId": "880d3b0c-9d90-4c7f-bb08-a2d1e159b4b2",
            "deploymentType": "OnPremises"
        }),
        serde_json::json!({
            "instanceId": "",
            "deploymentId": "880d3b0c-9d90-4c7f-bb08-a2d1e159b4b2",
            "deploymentType": "OnPremises"
        }),
        serde_json::json!({
            "instanceId": "9e3f8106-1d6b-4ff8-9a7e-10e1fd9ab06f",
            "deploymentType": "OnPremises"
        }),
        serde_json::json!({
            "instanceId": "9e3f8106-1d6b-4ff8-9a7e-10e1fd9ab06f",
            "deploymentId": 42,
            "deploymentType": "OnPremises"
        }),
        serde_json::json!({
            "instanceId": "9e3f8106-1d6b-4ff8-9a7e-10e1fd9ab06f",
            "deploymentId": "880d3b0c-9d90-4c7f-bb08-a2d1e159b4b2",
            "deploymentType": 2
        }),
        serde_json::json!({
            "instanceId": "9e3f8106-1d6b-4ff8-9a7e-10e1fd9ab06f",
            "deploymentId": "880d3b0c-9d90-4c7f-bb08-a2d1e159b4b2",
            "deploymentType": "Unknown"
        }),
    ] {
        let malformed = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/_apis/connectionData"))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .expect(1)
            .mount(&malformed)
            .await;
        assert!(matches!(
            discovered_vendor(&malformed).await,
            Err(SniffError::UnsupportedProvider { .. })
        ));
    }
}

#[tokio::test]
async fn ambiguous_discovery_rejects_conflicting_and_unidentified_signatures() {
    let conflict = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "version": "17.2.0",
            "revision": "a1b2c3d4"
        })))
        .mount(&conflict)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/v1/version"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "version": "1.25.0"
        })))
        .mount(&conflict)
        .await;
    let error = discovered_vendor(&conflict).await.unwrap_err();
    assert!(matches!(
        error,
        SniffError::RemoteApi { ref message, .. }
            if message.contains("conflicting provider signatures")
    ));

    let unidentified = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "message": "generic service response"
        })))
        .expect(5)
        .mount(&unidentified)
        .await;
    let error = discovered_vendor(&unidentified).await.unwrap_err();
    assert!(matches!(error, SniffError::UnsupportedProvider { .. }));
    assert_eq!(unidentified.received_requests().await.unwrap().len(), 5);
}

#[tokio::test]
#[serial]
async fn ambiguous_discovery_keeps_credentials_host_and_provider_bound() {
    let global_tokens = [
        ("GH_TOKEN", "global-github-secret"),
        ("GITHUB_TOKEN", "global-github-fallback-secret"),
        ("GITLAB_TOKEN", "global-gitlab-secret"),
        ("GITLAB_PRIVATE_TOKEN", "global-gitlab-fallback-secret"),
        ("GITEA_TOKEN", "global-gitea-secret"),
        ("FORGEJO_TOKEN", "global-forgejo-secret"),
        ("CODEBERG_TOKEN", "global-codeberg-secret"),
        ("BITBUCKET_TOKEN", "global-bitbucket-secret"),
        ("AZURE_DEVOPS_TOKEN", "global-azure-secret"),
    ];
    let _global_tokens = global_tokens
        .iter()
        .map(|(name, value)| EnvGuard::set_safe(name, value))
        .collect::<Vec<_>>();
    let host_variable = "SNIFF_GITLAB_127_2E_0_2E_0_2E_1_TOKEN";
    let host_token = EnvGuard::set_safe(host_variable, "host-gitlab-secret");
    let valid = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(|request: &wiremock::Request| {
            let status = if request.headers.contains_key("private-token") {
                200
            } else {
                401
            };
            ResponseTemplate::new(status).set_body_json(serde_json::json!({
                "version": "17.2.0",
                "revision": "a1b2c3d4"
            }))
        })
        .expect(2)
        .mount(&valid)
        .await;
    assert_eq!(discovered_vendor(&valid).await.unwrap(), "gitlab");
    let requests = valid.received_requests().await.unwrap();
    assert_eq!(requests.len(), 6);
    let authenticated = requests
        .iter()
        .filter(|request| request.headers.contains_key("private-token"))
        .collect::<Vec<_>>();
    assert_eq!(authenticated.len(), 1);
    assert_eq!(authenticated[0].url.path(), "/api/v4/version");
    assert_eq!(
        authenticated[0].headers["private-token"].to_str().unwrap(),
        "host-gitlab-secret"
    );
    for request in &requests {
        assert!(!request.headers.contains_key("authorization"));
        if request.url.path() != "/api/v4/version"
            || !request.headers.contains_key("private-token")
        {
            assert!(!request.headers.contains_key("private-token"));
        }
        let rendered = format!("{:?}", request.headers);
        for (_, secret) in global_tokens {
            assert!(!rendered.contains(secret), "global credential reached {}", request.url);
        }
    }

    drop(host_token);
    let host_token = EnvGuard::set_safe(host_variable, "invalid-host-secret");
    let invalid = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "version": "17.2.0",
            "revision": "a1b2c3d4"
        })))
        .expect(2)
        .mount(&invalid)
        .await;
    let error = discovered_vendor(&invalid).await.unwrap_err();
    assert!(matches!(
        error,
        SniffError::InvalidCredentials { ref provider, .. } if provider == "GitLab"
    ));
    let rendered = format!("{error:?}\n{error}");
    assert!(!rendered.contains("invalid-host-secret"));

    drop(host_token);
    let _host_token = EnvGuard::remove_safe(host_variable);
    let missing = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "version": "17.2.0",
            "revision": "a1b2c3d4"
        })))
        .expect(1)
        .mount(&missing)
        .await;
    let error = discovered_vendor(&missing).await.unwrap_err();
    assert!(matches!(
        error,
        SniffError::MissingCredentials { ref provider, ref env_var }
            if provider == "GitLab" && env_var == host_variable
    ));

    let generic_proxy = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "message": "authentication required"
        })))
        .expect(5)
        .mount(&generic_proxy)
        .await;
    let error = discovered_vendor(&generic_proxy).await.unwrap_err();
    assert!(matches!(error, SniffError::UnsupportedProvider { .. }));
    assert_eq!(generic_proxy.received_requests().await.unwrap().len(), 5);
}

#[tokio::test]
#[serial]
async fn unsigned_authentication_challenges_use_only_one_host_bound_provider_credential() {
    let host_variables = [
        "SNIFF_GITHUB_127_2E_0_2E_0_2E_1_TOKEN",
        "SNIFF_GITLAB_127_2E_0_2E_0_2E_1_TOKEN",
        "SNIFF_GITEA_127_2E_0_2E_0_2E_1_TOKEN",
        "SNIFF_FORGEJO_127_2E_0_2E_0_2E_1_TOKEN",
        "SNIFF_BITBUCKET_127_2E_0_2E_0_2E_1_TOKEN",
        "SNIFF_AZURE_DEVOPS_127_2E_0_2E_0_2E_1_TOKEN",
    ];
    let _clean_host_variables = host_variables
        .iter()
        .map(EnvGuard::remove_safe)
        .collect::<Vec<_>>();
    let global_tokens = [
        ("GH_TOKEN", "unsigned-global-github-secret"),
        ("GITHUB_TOKEN", "unsigned-global-github-fallback-secret"),
        ("GITLAB_TOKEN", "unsigned-global-gitlab-secret"),
        ("GITLAB_PRIVATE_TOKEN", "unsigned-global-gitlab-fallback-secret"),
        ("GITEA_TOKEN", "unsigned-global-gitea-secret"),
        ("FORGEJO_TOKEN", "unsigned-global-forgejo-secret"),
        ("CODEBERG_TOKEN", "unsigned-global-codeberg-secret"),
        ("BITBUCKET_TOKEN", "unsigned-global-bitbucket-secret"),
        ("AZURE_DEVOPS_TOKEN", "unsigned-global-azure-secret"),
    ];
    let _global_tokens = global_tokens
        .iter()
        .map(|(name, value)| EnvGuard::set_safe(name, value))
        .collect::<Vec<_>>();
    let gitlab_variable = host_variables[1];

    let host_token = EnvGuard::set_safe(gitlab_variable, "unsigned-host-gitlab-secret");
    let success = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(|request: &wiremock::Request| {
            if request.url.path() == "/api/v4/version"
                && request.headers.contains_key("private-token")
            {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "version": "17.2.0",
                    "revision": "a1b2c3d4"
                }))
            } else {
                ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "message": "authentication required"
                }))
            }
        })
        .expect(6)
        .mount(&success)
        .await;
    assert_eq!(discovered_vendor(&success).await.unwrap(), "gitlab");
    let requests = success.received_requests().await.unwrap();
    assert_eq!(requests.len(), 6);
    let authenticated = requests
        .iter()
        .filter(|request| {
            request.headers.contains_key("authorization")
                || request.headers.contains_key("private-token")
        })
        .collect::<Vec<_>>();
    assert_eq!(authenticated.len(), 1);
    assert_eq!(authenticated[0].url.path(), "/api/v4/version");
    assert_eq!(
        authenticated[0].headers["private-token"].to_str().unwrap(),
        "unsigned-host-gitlab-secret"
    );
    let rendered_requests = format!("{requests:?}");
    for (_, secret) in global_tokens {
        assert!(!rendered_requests.contains(secret));
    }

    drop(host_token);
    let host_token = EnvGuard::set_safe(gitlab_variable, "unsigned-invalid-host-secret");
    let invalid = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "message": "authentication required"
        })))
        .expect(6)
        .mount(&invalid)
        .await;
    let error = discovered_vendor(&invalid).await.unwrap_err();
    assert!(matches!(
        error,
        SniffError::InvalidCredentials { ref provider, .. } if provider == "GitLab"
    ));
    assert!(!format!("{error:?}\n{error}").contains("unsigned-invalid-host-secret"));

    drop(host_token);
    let host_token = EnvGuard::set_safe(gitlab_variable, "unsigned-forbidden-host-secret");
    let forbidden = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(|request: &wiremock::Request| {
            let status = if request.headers.contains_key("private-token") {
                403
            } else {
                401
            };
            ResponseTemplate::new(status).set_body_json(serde_json::json!({
                "message": "authentication required"
            }))
        })
        .expect(6)
        .mount(&forbidden)
        .await;
    let error = discovered_vendor(&forbidden).await.unwrap_err();
    assert!(matches!(
        error,
        SniffError::RemoteForbidden { ref provider, .. } if provider == "GitLab"
    ));
    assert!(!format!("{error:?}\n{error}").contains("unsigned-forbidden-host-secret"));

    drop(host_token);
    let _missing_host_token = EnvGuard::remove_safe(gitlab_variable);
    let missing = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "message": "authentication required"
        })))
        .expect(5)
        .mount(&missing)
        .await;
    let error = discovered_vendor(&missing).await.unwrap_err();
    assert!(matches!(error, SniffError::UnsupportedProvider { .. }));
    let requests = missing.received_requests().await.unwrap();
    assert_eq!(requests.len(), 5);
    let rendered_requests = format!("{requests:?}");
    for (_, secret) in global_tokens {
        assert!(!rendered_requests.contains(secret));
    }

    let _github_token = EnvGuard::set_safe(host_variables[0], "unsigned-host-github-secret");
    let _gitlab_token = EnvGuard::set_safe(gitlab_variable, "unsigned-host-gitlab-secret");
    let multiple = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "message": "authentication required"
        })))
        .expect(5)
        .mount(&multiple)
        .await;
    let error = discovered_vendor(&multiple).await.unwrap_err();
    assert!(matches!(
        error,
        SniffError::RemoteApi { ref message, .. }
            if message.contains("multiple host-bound provider credentials")
    ));
    let rendered = format!("{error:?}\n{error}");
    assert!(!rendered.contains("unsigned-host-github-secret"));
    assert!(!rendered.contains("unsigned-host-gitlab-secret"));
    let requests = multiple.received_requests().await.unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests.iter().all(|request| {
        !request.headers.contains_key("authorization")
            && !request.headers.contains_key("private-token")
    }));
}

#[tokio::test]
#[serial]
async fn host_bound_discovery_uses_each_providers_exact_authentication_header() {
    let token = "host-auth-secret";
    let basic = format!(
        "Basic {}",
        base64::engine::general_purpose::STANDARD.encode(format!(":{token}"))
    );
    let cases = [
        (
            "SNIFF_GITHUB_127_2E_0_2E_0_2E_1_TOKEN",
            "/api/v3/meta",
            serde_json::json!({"installed_version": "3.16.1"}),
            "authorization",
            format!("Bearer {token}"),
            "github",
        ),
        (
            "SNIFF_GITLAB_127_2E_0_2E_0_2E_1_TOKEN",
            "/api/v4/version",
            serde_json::json!({"version": "17.2.0", "revision": "a1b2c3d4"}),
            "private-token",
            token.to_string(),
            "gitlab",
        ),
        (
            "SNIFF_GITEA_127_2E_0_2E_0_2E_1_TOKEN",
            "/api/v1/version",
            serde_json::json!({"version": "1.25.0"}),
            "authorization",
            format!("token {token}"),
            "gitea",
        ),
        (
            "SNIFF_FORGEJO_127_2E_0_2E_0_2E_1_TOKEN",
            "/api/v1/version",
            serde_json::json!({"version": "Forgejo 14.0.0"}),
            "authorization",
            format!("token {token}"),
            "forgejo",
        ),
        (
            "SNIFF_BITBUCKET_127_2E_0_2E_0_2E_1_TOKEN",
            "/rest/api/1.0/application-properties",
            serde_json::json!({"displayName": "Bitbucket", "version": "9.4.1"}),
            "authorization",
            format!("Bearer {token}"),
            "bitbucket",
        ),
        (
            "SNIFF_AZURE_DEVOPS_127_2E_0_2E_0_2E_1_TOKEN",
            "/_apis/connectionData",
            serde_json::json!({
                "instanceId": "9e3f8106-1d6b-4ff8-9a7e-10e1fd9ab06f",
                "deploymentId": "880d3b0c-9d90-4c7f-bb08-a2d1e159b4b2",
                "deploymentType": "OnPremises"
            }),
            "authorization",
            basic,
            "azure_devops",
        ),
    ];
    let host_variables = cases.iter().map(|case| case.0).collect::<Vec<_>>();
    let _clean_host_variables = host_variables
        .iter()
        .map(EnvGuard::remove_safe)
        .collect::<Vec<_>>();
    let global_tokens = [
        ("GH_TOKEN", "matrix-global-github-secret"),
        ("GITHUB_TOKEN", "matrix-global-github-fallback-secret"),
        ("GITLAB_TOKEN", "matrix-global-gitlab-secret"),
        ("GITLAB_PRIVATE_TOKEN", "matrix-global-gitlab-fallback-secret"),
        ("GITEA_TOKEN", "matrix-global-gitea-secret"),
        ("FORGEJO_TOKEN", "matrix-global-forgejo-secret"),
        ("CODEBERG_TOKEN", "matrix-global-codeberg-secret"),
        ("BITBUCKET_TOKEN", "matrix-global-bitbucket-secret"),
        ("AZURE_DEVOPS_TOKEN", "matrix-global-azure-secret"),
    ];
    let _global_tokens = global_tokens
        .iter()
        .map(|(name, value)| EnvGuard::set_safe(name, value))
        .collect::<Vec<_>>();

    for (variable, signature_path, signature, header_name, header_value, expected) in cases {
        for signed_challenge in [true, false] {
            let _host_token = EnvGuard::set_safe(variable, token);
            let server = MockServer::start().await;
            let body = signature.clone();
            let expected_value = header_value.clone();
            Mock::given(method("GET"))
                .and(path(signature_path))
                .respond_with(move |request: &wiremock::Request| {
                    let authenticated = request
                        .headers
                        .get(header_name)
                        .and_then(|value| value.to_str().ok())
                        .is_some_and(|value| value == expected_value);
                    let response_body = if authenticated || signed_challenge {
                        body.clone()
                    } else {
                        serde_json::json!({"message": "authentication required"})
                    };
                    ResponseTemplate::new(if authenticated { 200 } else { 401 })
                        .set_body_json(response_body)
                })
                .expect(2)
                .mount(&server)
                .await;

            assert_eq!(discovered_vendor(&server).await.unwrap(), expected);
            let requests = server.received_requests().await.unwrap();
            let authenticated = requests
                .iter()
                .filter(|request| {
                    request.headers.contains_key("authorization")
                        || request.headers.contains_key("private-token")
                })
                .collect::<Vec<_>>();
            assert_eq!(authenticated.len(), 1, "{expected} signed={signed_challenge}");
            assert_eq!(
                authenticated[0].headers[header_name].to_str().unwrap(),
                header_value,
                "{expected} signed={signed_challenge}"
            );
            let rendered = format!("{requests:?}");
            for (_, secret) in global_tokens {
                assert!(!rendered.contains(secret), "global credential reached {expected}");
            }
        }

        let invalid_token = format!("invalid-{expected}-secret");
        let _host_token = EnvGuard::set_safe(variable, &invalid_token);
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path(signature_path))
            .respond_with(ResponseTemplate::new(401).set_body_json(signature))
            .expect(2)
            .mount(&server)
            .await;
        let error = discovered_vendor(&server).await.unwrap_err();
        assert!(matches!(error, SniffError::InvalidCredentials { .. }));
        assert!(!format!("{error:?}\n{error}").contains(&invalid_token));
        let rendered = format!("{:?}", server.received_requests().await.unwrap());
        for (_, secret) in global_tokens {
            assert!(!rendered.contains(secret), "global credential reached {expected}");
        }
    }
}

#[tokio::test]
async fn ambiguous_ssh_and_scp_vendor_discovery_reaches_the_https_host_policy() {
    for remote_url in [
        "ssh://git@git.example:2222/acme/project.git",
        "git@git.example:acme/project.git",
    ] {
        let (directory, _) = repository(remote_url);
        let root = directory.path().to_path_buf();
        let error = tokio::task::spawn_blocking(move || {
            remote_vendor_at(&root, None, &FetchPolicy::deny_all())
        })
        .await
        .unwrap()
        .unwrap_err();

        assert!(matches!(
            error,
            SniffError::RemotePolicyDenied { ref host } if host == "git.example"
        ));
    }
}

#[test]
fn ssh_and_scp_vendor_detection_succeeds_for_local_classifications() {
    for (remote_url, expected) in [
        ("ssh://git@gitlab.com:2222/acme/project.git", "gitlab"),
        ("git@gitlab.com:acme/project.git", "gitlab"),
        ("ssh://git@gitea.example:2222/acme/project.git", "gitea"),
        ("git@gitea.example:acme/project.git", "gitea"),
        ("ssh://git@codeberg.org:2222/acme/project.git", "forgejo"),
        ("git@codeberg.org:acme/project.git", "forgejo"),
    ] {
        let (directory, _) = repository(remote_url);
        assert_eq!(
            remote_vendor_at(directory.path(), None, &FetchPolicy::deny_all()).unwrap(),
            expected,
            "{remote_url}"
        );
    }
}
