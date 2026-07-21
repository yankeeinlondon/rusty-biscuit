#![cfg(feature = "network")]

use biscuit_file::FetchPolicy;
use serial_test::serial;
use sniff::SniffError;
use sniff::filesystem::git::{branch_exists_on_remote_at, remote_vendor_at};
use test_toolkit::EnvGuard;
use wiremock::matchers::{header, method, path, query_param};
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
                "deploymentVersion": "Azure DevOps Server 2022.2"
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
async fn gitlab_discovery_uses_credentials_without_disclosing_them() {
    let _github = EnvGuard::remove_safe("GH_TOKEN");
    let _github_fallback = EnvGuard::remove_safe("GITHUB_TOKEN");
    let _gitlab_fallback = EnvGuard::remove_safe("GITLAB_PRIVATE_TOKEN");
    let _gitlab = EnvGuard::set_safe("GITLAB_TOKEN", "valid-discovery-secret");
    let valid = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .and(header("PRIVATE-TOKEN", "valid-discovery-secret"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "version": "17.2.0",
            "revision": "a1b2c3d4"
        })))
        .expect(1)
        .mount(&valid)
        .await;
    assert_eq!(discovered_vendor(&valid).await.unwrap(), "gitlab");

    drop(_gitlab);
    let _gitlab = EnvGuard::set_safe("GITLAB_TOKEN", "invalid-discovery-secret");
    let invalid = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .and(header("PRIVATE-TOKEN", "invalid-discovery-secret"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&invalid)
        .await;
    let error = discovered_vendor(&invalid).await.unwrap_err();
    assert!(matches!(error, SniffError::InvalidCredentials { .. }));
    let rendered = format!("{error:?}\n{error}");
    assert!(!rendered.contains("invalid-discovery-secret"));

    drop(_gitlab);
    let _gitlab = EnvGuard::remove_safe("GITLAB_TOKEN");
    let missing = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/v4/version"))
        .respond_with(ResponseTemplate::new(401))
        .expect(1)
        .mount(&missing)
        .await;
    let error = discovered_vendor(&missing).await.unwrap_err();
    assert!(matches!(
        error,
        SniffError::MissingCredentials { ref env_var, .. } if env_var == "GITLAB_TOKEN"
    ));
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
