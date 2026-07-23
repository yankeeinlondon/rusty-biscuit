//! Local GitHub report fixture for request-count benchmarks.

#![allow(dead_code)]

use sniff::remote::GitHubRemote;
use tokio::runtime::Runtime;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Runtime, mock server, and provider kept alive for a benchmark group.
pub struct RemoteReportFixture {
    _server: MockServer,
    pub provider: GitHubRemote,
    pub runtime: Runtime,
}

/// Start a deterministic provider fixture with one metadata and one tree route.
pub fn github() -> RemoteReportFixture {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("build remote report runtime");
    let server = runtime.block_on(async {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/bench-owner/bench-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": 1,
                "name": "bench-repo",
                "full_name": "bench-owner/bench-repo",
                "private": false,
                "owner": { "login": "bench-owner", "id": 2 },
                "html_url": "https://example.test/bench-owner/bench-repo",
                "url": "https://example.test/repos/bench-owner/bench-repo",
                "default_branch": "main",
                "stargazers_count": 1,
                "forks_count": 0,
                "open_issues_count": 0,
                "archived": false,
                "disabled": false
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"/repos/bench-owner/bench-repo/git/trees/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "abc123",
                "tree": [
                    {"path": "README.md", "type": "blob", "mode": "100644", "sha": "aaa", "size": 128},
                    {"path": ".github/workflows/ci.yml", "type": "blob", "mode": "100644", "sha": "bbb", "size": 128}
                ],
                "truncated": false
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;
        server
    });

    // SAFETY: Criterion registers this fixture before it begins sampling. No
    // benchmark worker concurrently mutates the credential environment.
    unsafe { std::env::set_var("GITHUB_TOKEN", "benchmark-token") };
    let provider = GitHubRemote::with_base_url(&server.uri()).expect("build fixture provider");
    RemoteReportFixture {
        _server: server,
        provider,
        runtime,
    }
}
