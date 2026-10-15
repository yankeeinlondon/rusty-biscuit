//! Integration tests for remote repository providers using wiremock.
//!
//! These tests verify that each provider correctly:
//! - Parses API responses
//! - Handles error status codes (401, 404, 429)
//! - Maps errors to appropriate SniffError variants
//!
//! ## Running Tests
//!
//! ```bash
//! cargo test -p sniff --features remote --test l1 remote_providers::
//! ```

use sniff::error::SniffError;
use sniff::remote::{
    BitbucketRemote, DocumentCategory, GitHubRemote, GitLabRemote, GitProvider, GitRemote,
    GiteaRemote, PullRequestState, RemoteRepoProvider,
};
use test_toolkit::EnvGuard;
use wiremock::matchers::{header_exists, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Every credential variable the four providers read.
const PROVIDER_CREDENTIAL_VARIABLES: [&str; 8] = [
    "GITHUB_TOKEN",
    "GH_TOKEN",
    "GITLAB_TOKEN",
    "GITLAB_PRIVATE_TOKEN",
    "GITEA_TOKEN",
    "CODEBERG_TOKEN",
    "BITBUCKET_USERNAME",
    "BITBUCKET_APP_PASSWORD",
];

// =============================================================================
// Test Fixtures
// =============================================================================

fn github_repo_fixture() -> serde_json::Value {
    serde_json::json!({
        "id": 12345678,
        "name": "test-repo",
        "full_name": "test-owner/test-repo",
        "description": "A test repository",
        "private": false,
        "owner": {
            "login": "test-owner",
            "id": 1234
        },
        "html_url": "https://github.com/test-owner/test-repo",
        "url": "https://api.github.com/repos/test-owner/test-repo",
        "default_branch": "main",
        "language": "Rust",
        "stargazers_count": 42,
        "forks_count": 5,
        "open_issues_count": 3,
        "created_at": "2024-01-15T10:30:00Z",
        "updated_at": "2024-06-20T15:45:00Z",
        "pushed_at": "2024-06-19T12:00:00Z",
        "has_issues": true,
        "has_wiki": true,
        "archived": false,
        "disabled": false,
        "license": {
            "key": "mit",
            "name": "MIT License",
            "spdx_id": "MIT"
        },
        "topics": ["testing", "rust"]
    })
}

fn github_tree_fixture() -> serde_json::Value {
    serde_json::json!({
        "sha": "abc123def456",
        "url": "https://api.github.com/repos/test-owner/test-repo/git/trees/abc123",
        "tree": [
            {"path": "README.md", "type": "blob", "mode": "100644", "sha": "aaa111", "size": 1024},
            {"path": "src/main.rs", "type": "blob", "mode": "100644", "sha": "ccc333", "size": 512},
            {"path": "src/README.md", "type": "blob", "mode": "100644", "sha": "ddd444", "size": 256},
            {"path": "docs/guide.md", "type": "blob", "mode": "100644", "sha": "fff666", "size": 2048},
            {"path": "CHANGELOG.md", "type": "blob", "mode": "100644", "sha": "ggg777", "size": 4096},
            {"path": ".github/workflows/ci.yml", "type": "blob", "mode": "100644", "sha": "jjj000", "size": 128}
        ],
        "truncated": false
    })
}

fn github_pull_requests_fixture() -> serde_json::Value {
    serde_json::json!([
        {
            "number": 42,
            "title": "Add new feature",
            "state": "open",
            "user": {"login": "contributor1", "id": 5678},
            "draft": false,
            "head": {"ref": "feature-branch", "sha": "abc123"},
            "base": {"ref": "main", "sha": "def456"},
            "labels": [{"name": "enhancement"}, {"name": "help wanted"}],
            "body": "This PR adds a new feature.\n\n## Changes\n- Added feature X",
            "created_at": "2024-06-15T10:00:00Z",
            "updated_at": "2024-06-18T14:30:00Z",
            "merged_at": null,
            "html_url": "https://github.com/test-owner/test-repo/pull/42",
            "url": "https://api.github.com/repos/test-owner/test-repo/pulls/42"
        }
    ])
}

fn github_issues_fixture() -> serde_json::Value {
    serde_json::json!([
        {
            "id": 1000,
            "number": 10,
            "title": "Document the API",
            "state": "open",
            "user": {"login": "reporter1", "id": 1111},
            "comments": 5,
            "labels": [{"name": "documentation"}],
            "created_at": "2024-06-01T09:00:00Z",
            "updated_at": "2024-06-15T11:00:00Z",
            "closed_at": null,
            "html_url": "https://github.com/test-owner/test-repo/issues/10",
            "url": "https://api.github.com/repos/test-owner/test-repo/issues/10"
        }
    ])
}

fn github_tags_fixture() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "v1.0.0",
            "commit": {
                "sha": "abc123def456789",
                "url": "https://api.github.com/repos/test-owner/test-repo/commits/abc123def456789"
            },
            "zipball_url": "https://api.github.com/repos/test-owner/test-repo/zipball/v1.0.0",
            "tarball_url": "https://api.github.com/repos/test-owner/test-repo/tarball/v1.0.0",
            "node_id": "MDM6UmVmMTIzNDU2Nzg6dmVycy8xLjAuMA=="
        }
    ])
}

fn github_releases_fixture() -> serde_json::Value {
    serde_json::json!([
        {
            "id": 100,
            "tag_name": "v1.0.0",
            "name": "Version 1.0.0",
            "draft": false,
            "prerelease": false,
            "created_at": "2024-06-01T10:00:00Z",
            "published_at": "2024-06-01T12:00:00Z",
            "html_url": "https://github.com/test-owner/test-repo/releases/tag/v1.0.0"
        }
    ])
}

fn github_workflow_runs_fixture() -> serde_json::Value {
    serde_json::json!({
        "total_count": 2,
        "workflow_runs": [
            {
                "id": 111,
                "name": "CI",
                "status": "completed",
                "conclusion": "success",
                "event": "push",
                "head_branch": "main",
                "html_url": "https://github.com/test-owner/test-repo/actions/runs/111",
                "created_at": "2024-06-20T10:00:00Z"
            },
            {
                "id": 112,
                "name": "Lint",
                "status": "in_progress",
                "conclusion": null,
                "event": "pull_request",
                "head_branch": "feature",
                "html_url": "https://github.com/test-owner/test-repo/actions/runs/112",
                "created_at": "2024-06-20T11:00:00Z"
            }
        ]
    })
}

// =============================================================================
// GitHub Provider Tests
// =============================================================================

mod github_tests {
    use super::*;

    async fn setup_github_mock() -> (MockServer, GitHubRemote) {
        let server = MockServer::start().await;
        // Set a mock token in the environment for this test
        // SAFETY: Tests run sequentially and this env var is only used by this test
        unsafe { std::env::set_var("GITHUB_TOKEN", "test-token-12345") };
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();
        (server, provider)
    }

    #[tokio::test]
    async fn get_repo_metadata_success() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_repo_fixture()))
            .mount(&server)
            .await;

        let metadata = provider
            .get_repo_metadata("test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(metadata.name, "test-repo");
        assert_eq!(metadata.full_name, "test-owner/test-repo");
        assert_eq!(metadata.description, Some("A test repository".to_string()));
        assert!(!metadata.private);
        assert_eq!(metadata.default_branch, "main");
        assert_eq!(metadata.language, Some("Rust".to_string()));
        assert_eq!(metadata.stars, Some(42));
        assert_eq!(metadata.forks, Some(5));
        assert!(!metadata.archived);
        assert!(metadata.license.is_some());
        assert_eq!(metadata.topics, vec!["testing", "rust"]);
    }

    #[tokio::test]
    async fn list_documents_success() {
        let (server, provider) = setup_github_mock().await;

        // Mock the repo endpoint (needed to get default branch)
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_repo_fixture()))
            .mount(&server)
            .await;

        // Without `recursive=true` GitHub lists only the top level of the tree.
        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/git/trees/.*"))
            .and(query_param("recursive", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_tree_fixture()))
            .mount(&server)
            .await;

        let documents = provider
            .list_documents("test-owner", "test-repo")
            .await
            .unwrap();

        assert!(!documents.is_empty());

        // Check that we have expected document categories
        let readme_count = documents
            .iter()
            .filter(|d| d.category == DocumentCategory::Readme)
            .count();
        let docs_folder_count = documents
            .iter()
            .filter(|d| d.category == DocumentCategory::DocsFolder)
            .count();
        let source_doc_count = documents
            .iter()
            .filter(|d| d.category == DocumentCategory::SourceDoc)
            .count();
        let other_count = documents
            .iter()
            .filter(|d| d.category == DocumentCategory::Other)
            .count();

        assert!(readme_count >= 1, "Should have at least one README");
        assert!(
            docs_folder_count >= 1,
            "Should have at least one docs folder doc"
        );
        assert!(source_doc_count >= 1, "Should have at least one source doc");
        assert!(
            other_count >= 1,
            "Should have at least one other doc (CHANGELOG)"
        );
    }

    #[tokio::test]
    async fn list_pull_requests_success() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_pull_requests_fixture()))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 42);
        assert_eq!(prs[0].title, "Add new feature");
        assert_eq!(prs[0].state, "open");
        assert_eq!(prs[0].author, "contributor1");
        assert!(!prs[0].draft);
        assert_eq!(prs[0].source_branch, Some("feature-branch".to_string()));
        assert_eq!(prs[0].target_branch, Some("main".to_string()));
        assert_eq!(
            prs[0].labels,
            vec!["enhancement".to_string(), "help wanted".to_string()]
        );
        assert!(prs[0].body.as_ref().unwrap().contains("This PR adds"));
    }

    #[tokio::test]
    async fn list_pull_requests_no_labels() {
        let (server, provider) = setup_github_mock().await;

        // Fixture with explicitly empty labels array.
        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "number": 7,
                    "title": "PR with no labels",
                    "state": "open",
                    "user": {"login": "contributor1", "id": 5678},
                    "draft": false,
                    "head": {"ref": "feature-branch", "sha": "abc123"},
                    "base": {"ref": "main", "sha": "def456"},
                    "labels": [],
                    "body": "Body without labels.",
                    "created_at": "2024-06-15T10:00:00Z",
                    "updated_at": "2024-06-18T14:30:00Z",
                    "merged_at": null,
                    "html_url": "https://github.com/test-owner/test-repo/pull/7",
                    "url": "https://api.github.com/repos/test-owner/test-repo/pulls/7"
                }
            ])))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 7);
        assert!(
            prs[0].labels.is_empty(),
            "Expected empty labels, got: {:?}",
            prs[0].labels
        );
    }

    #[tokio::test]
    async fn list_pull_requests_omitted_labels() {
        let (server, provider) = setup_github_mock().await;

        // Fixture that omits the `labels` field entirely. This proves the
        // schema's `#[serde(default)]` lets the response deserialize without
        // requiring the labels key, so older API responses (or partial
        // payloads) still parse cleanly.
        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "number": 9,
                    "title": "PR with omitted labels field",
                    "state": "open",
                    "user": {"login": "contributor2", "id": 9999},
                    "draft": false,
                    "head": {"ref": "feature-branch", "sha": "abc123"},
                    "base": {"ref": "main", "sha": "def456"},
                    "body": "No labels key in fixture.",
                    "created_at": "2024-06-15T10:00:00Z",
                    "updated_at": "2024-06-18T14:30:00Z",
                    "merged_at": null,
                    "html_url": "https://github.com/test-owner/test-repo/pull/9",
                    "url": "https://api.github.com/repos/test-owner/test-repo/pulls/9"
                }
            ])))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 9);
        assert!(
            prs[0].labels.is_empty(),
            "Expected empty labels (omitted field), got: {:?}",
            prs[0].labels
        );
    }

    #[tokio::test]
    async fn list_pull_requests_merged_filter() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "number": 1,
                    "title": "Merged PR",
                    "state": "closed",
                    "user": {"login": "user1", "id": 1},
                    "draft": false,
                    "head": {"ref": "branch1", "sha": "abc123"},
                    "base": {"ref": "main", "sha": "def456"},
                    "labels": [],
                    "body": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "merged_at": "2024-01-02T00:00:00Z",
                    "html_url": "https://github.com/test-owner/test-repo/pull/1",
                    "url": "https://api.github.com/repos/test-owner/test-repo/pulls/1"
                },
                {
                    "number": 2,
                    "title": "Closed PR",
                    "state": "closed",
                    "user": {"login": "user2", "id": 2},
                    "draft": false,
                    "head": {"ref": "branch2", "sha": "abc123"},
                    "base": {"ref": "main", "sha": "def456"},
                    "labels": [],
                    "body": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "merged_at": null,
                    "html_url": "https://github.com/test-owner/test-repo/pull/2",
                    "url": "https://api.github.com/repos/test-owner/test-repo/pulls/2"
                }
            ])))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Merged)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 1);
        assert_eq!(prs[0].title, "Merged PR");
    }

    #[tokio::test]
    async fn list_pull_requests_draft_filter() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "number": 1,
                    "title": "Draft PR",
                    "state": "open",
                    "user": {"login": "user1", "id": 1},
                    "draft": true,
                    "head": {"ref": "branch1", "sha": "abc123"},
                    "base": {"ref": "main", "sha": "def456"},
                    "labels": [],
                    "body": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "merged_at": null,
                    "html_url": "https://github.com/test-owner/test-repo/pull/1",
                    "url": "https://api.github.com/repos/test-owner/test-repo/pulls/1"
                },
                {
                    "number": 2,
                    "title": "Open PR",
                    "state": "open",
                    "user": {"login": "user2", "id": 2},
                    "draft": false,
                    "head": {"ref": "branch2", "sha": "abc123"},
                    "base": {"ref": "main", "sha": "def456"},
                    "labels": [],
                    "body": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "merged_at": null,
                    "html_url": "https://github.com/test-owner/test-repo/pull/2",
                    "url": "https://api.github.com/repos/test-owner/test-repo/pulls/2"
                }
            ])))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Draft)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 1);
        assert_eq!(prs[0].title, "Draft PR");
        assert!(prs[0].draft);
    }

    #[tokio::test]
    async fn list_issues_success() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/issues.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_issues_fixture()))
            .mount(&server)
            .await;

        let issues = provider
            .list_issues("test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 10);
        assert_eq!(issues[0].title, "Document the API");
        assert_eq!(issues[0].state, "open");
        assert_eq!(issues[0].author, "reporter1");
        assert_eq!(issues[0].comment_count, Some(5));
        assert!(issues[0].labels.contains(&"documentation".to_string()));
    }

    #[tokio::test]
    async fn get_tags_and_releases_success() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/tags.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_tags_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/releases.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_releases_fixture()))
            .mount(&server)
            .await;

        let tags_and_releases = provider
            .get_tags_and_releases("test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(tags_and_releases.tags.len(), 1);
        assert_eq!(tags_and_releases.tags[0].name, "v1.0.0");

        assert_eq!(tags_and_releases.releases.len(), 1);
        assert_eq!(tags_and_releases.releases[0].tag_name, "v1.0.0");
        assert!(!tags_and_releases.releases[0].prerelease);
    }

    #[tokio::test]
    async fn detect_cicd_github_actions() {
        let (server, provider) = setup_github_mock().await;

        // Mock the repo endpoint
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_repo_fixture()))
            .mount(&server)
            .await;

        // Mock the tree endpoint with GitHub Actions workflow
        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/git/trees/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_tree_fixture()))
            .mount(&server)
            .await;

        let cicd = provider
            .detect_cicd("test-owner", "test-repo")
            .await
            .unwrap();

        assert!(cicd.is_some());
        let cicd_info = cicd.unwrap();
        assert_eq!(cicd_info.provider, "GitHub Actions");
        assert_eq!(cicd_info.config_path, Some(".github/workflows".to_string()));
    }

    #[tokio::test]
    async fn list_workflow_runs_success() {
        let (server, provider) = setup_github_mock().await;

        // The `.and(query_param(...))` matcher fails the request unless
        // `per_page=5` is on the wire, so the `.expect(1)` (verified on drop)
        // proves the limit is threaded into the request as `per_page`.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/runs"))
            .and(query_param("per_page", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_workflow_runs_fixture()))
            .expect(1)
            .mount(&server)
            .await;

        let runs = provider
            .list_workflow_runs("test-owner", "test-repo", 5)
            .await
            .unwrap();

        assert_eq!(runs.len(), 2);

        // Completed run maps every field through `map_workflow_run`.
        assert_eq!(runs[0].provider, "GitHub Actions");
        assert_eq!(runs[0].name, "CI");
        assert_eq!(runs[0].status, "completed");
        assert_eq!(runs[0].conclusion, Some("success".to_string()));
        assert_eq!(runs[0].head_branch, Some("main".to_string()));
        assert_eq!(runs[0].event, Some("push".to_string()));
        assert_eq!(runs[0].started_at, Some("2024-06-20T10:00:00Z".to_string()));
        assert_eq!(
            runs[0].html_url,
            Some("https://github.com/test-owner/test-repo/actions/runs/111".to_string())
        );

        // In-flight run carries its status with no conclusion.
        assert_eq!(runs[1].name, "Lint");
        assert_eq!(runs[1].status, "in_progress");
        assert_eq!(runs[1].conclusion, None);
        assert_eq!(runs[1].event, Some("pull_request".to_string()));
    }

    /// The page size is pager-managed and bounded to GitHub's 1..=100, so a
    /// limit outside that range is clamped rather than refused before sending.
    #[tokio::test]
    async fn list_workflow_runs_clamps_the_page_size_to_githubs_bounds() {
        let (server, provider) = setup_github_mock().await;

        for (limit, per_page, expected_runs) in [(150, "100", 2), (0, "1", 0)] {
            server.reset().await;
            Mock::given(method("GET"))
                .and(path("/repos/test-owner/test-repo/actions/runs"))
                .and(query_param("per_page", per_page))
                .respond_with(
                    ResponseTemplate::new(200).set_body_json(github_workflow_runs_fixture()),
                )
                .expect(1)
                .mount(&server)
                .await;

            let runs = provider
                .list_workflow_runs("test-owner", "test-repo", limit)
                .await
                .unwrap_or_else(|error| panic!("limit {limit}: {error}"));

            assert_eq!(runs.len(), expected_runs, "limit {limit}");
            server.verify().await;
        }
    }

    #[tokio::test]
    async fn list_workflow_runs_caps_results_at_limit() {
        let (server, provider) = setup_github_mock().await;

        // GitHub honors `per_page`, but a server that over-delivers (or ignores
        // the page size) must not blow past `limit`. Respond with five runs to a
        // `limit=2` request and assert `take(limit)` clamps the mapped vec to 2.
        let over_limit = serde_json::json!({
            "total_count": 5,
            "workflow_runs": (0..5).map(|i| serde_json::json!({
                "id": 200 + i,
                "name": "CI",
                "status": "completed",
                "conclusion": "success",
                "event": "push",
                "head_branch": "main",
                "html_url": format!("https://github.com/test-owner/test-repo/actions/runs/{}", 200 + i),
                "created_at": "2024-06-20T10:00:00Z"
            })).collect::<Vec<_>>()
        });

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/runs"))
            .and(query_param("per_page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(over_limit))
            .mount(&server)
            .await;

        let runs = provider
            .list_workflow_runs("test-owner", "test-repo", 2)
            .await
            .unwrap();

        assert_eq!(runs.len(), 2);
    }

    #[tokio::test]
    async fn fetch_report_via_enum_includes_workflow_runs() {
        // Regression: the production CLI dispatches through `GitRemote`, not
        // `GitHubRemote` directly. If the enum fails to forward
        // `list_workflow_runs`, `fetch_report` silently uses the trait default
        // (empty vec) and falls back to presence detection — so the run-shaped
        // CI/CD table never renders. This asserts the enum threads runs through.
        let (server, provider) = setup_github_mock().await;
        let remote = GitRemote::GitHub(provider);

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_repo_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/runs"))
            .and(query_param("per_page", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_workflow_runs_fixture()))
            .expect(1)
            .mount(&server)
            .await;

        let report = remote
            .fetch_report("test-owner", "test-repo")
            .await
            .unwrap();

        // Run-shaped entries (started_at populated) rather than presence detection.
        assert_eq!(report.ci_cd.len(), 2);
        assert_eq!(report.ci_cd[0].name, "CI");
        assert_eq!(report.ci_cd[0].status, "completed");
        assert!(report.ci_cd[0].started_at.is_some());
    }

    #[tokio::test]
    async fn provider_returns_github() {
        let (_, provider) = setup_github_mock().await;
        assert_eq!(provider.provider(), GitProvider::GitHub);
    }

    // Error case tests
    #[tokio::test]
    async fn error_401_unauthorized() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Bad credentials"))
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::InvalidCredentials { provider, .. } => {
                assert_eq!(provider, "GitHub");
            }
            other => panic!("Expected InvalidCredentials error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_404_not_found() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/nonexistent-repo"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let result = provider
            .get_repo_metadata("test-owner", "nonexistent-repo")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RemoteApi {
                provider,
                status,
                message,
            } => {
                assert_eq!(provider, "GitHub");
                assert_eq!(status, 404);
                assert_eq!(message, "Not found");
            }
            other => panic!("Expected RemoteApi error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_429_rate_limited() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RateLimited { provider, .. } => {
                assert_eq!(provider, "GitHub");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_403_rate_limited_in_body() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(403).set_body_string("API rate limit exceeded"))
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RateLimited { provider, .. } => {
                assert_eq!(provider, "GitHub");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_file_content_returns_the_raw_text() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/contents/docs/guide.md"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/plain; charset=utf-8")
                    .set_body_string("# Guide\n\nHello.\n"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let content = provider
            .get_file_content("test-owner", "test-repo", "docs/guide.md")
            .await
            .unwrap();

        assert_eq!(content, "# Guide\n\nHello.\n");
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests[0].headers.get("accept").unwrap(),
            "application/vnd.github.raw+json"
        );
    }

    #[tokio::test]
    async fn get_file_content_maps_a_missing_file_to_not_found() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/contents/missing.md"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let result = provider
            .get_file_content("test-owner", "test-repo", "missing.md")
            .await;

        assert!(
            matches!(result, Err(SniffError::RemoteApi { status: 404, .. })),
            "{result:?}"
        );
    }

    /// `Retry-After` in delta-seconds on a 429 becomes `retry_after`; an absent or
    /// HTTP-date value is `None`.
    #[tokio::test]
    async fn error_429_reads_retry_after_seconds() {
        let (server, provider) = setup_github_mock().await;

        for (repo, retry_after, expected) in [
            ("with-seconds", Some("30"), Some(30)),
            ("without-header", None, None),
            ("with-date", Some("Wed, 21 Oct 2015 07:28:00 GMT"), None),
        ] {
            let mut reply = ResponseTemplate::new(429).set_body_string("slow down");
            if let Some(value) = retry_after {
                reply = reply.insert_header("retry-after", value);
            }
            Mock::given(method("GET"))
                .and(path(format!("/repos/test-owner/{repo}")))
                .respond_with(reply)
                .mount(&server)
                .await;

            let result = provider.get_repo_metadata("test-owner", repo).await;

            match result {
                Err(SniffError::RateLimited { retry_after, .. }) => {
                    assert_eq!(retry_after, expected, "{repo}");
                }
                other => panic!("Expected RateLimited for {repo}, got: {other:?}"),
            }
        }
    }

    #[tokio::test]
    async fn error_403_with_a_spent_quota_is_rate_limited() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-ratelimit-remaining", "0")
                    .insert_header("retry-after", "60")
                    .set_body_string("Forbidden"),
            )
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        match result {
            Err(SniffError::RateLimited { retry_after, .. }) => {
                assert_eq!(retry_after, Some(60));
            }
            other => panic!("Expected RateLimited error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn error_403_without_rate_limit_evidence_keeps_the_body() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(
                ResponseTemplate::new(403)
                    .insert_header("x-ratelimit-remaining", "4999")
                    .set_body_string("Resource not accessible by integration"),
            )
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        match result {
            Err(SniffError::RemoteApi {
                status, message, ..
            }) => {
                assert_eq!(status, 403);
                assert_eq!(message, "Resource not accessible by integration");
            }
            other => panic!("Expected RemoteApi error, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn error_5xx_keeps_its_status_and_body() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(502).set_body_string("bad gateway"))
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        match result {
            Err(SniffError::RemoteApi {
                status, message, ..
            }) => {
                assert_eq!(status, 502);
                assert_eq!(message, "bad gateway");
            }
            other => panic!("Expected RemoteApi error, got: {other:?}"),
        }
    }

    /// A 2xx body that does not decode and a 2xx body that is cut off are both
    /// status 0, and their messages tell them apart.
    #[tokio::test]
    async fn an_undecodable_body_and_an_interrupted_body_are_reported_distinctly() {
        let (server, provider) = setup_github_mock().await;
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_string("{not json"))
            .mount(&server)
            .await;
        let interrupted = GitHubRemote::with_base_url(&truncated_body_server()).unwrap();

        let undecodable = provider
            .get_repo_metadata("test-owner", "test-repo")
            .await
            .unwrap_err();
        let cut_off = interrupted
            .get_repo_metadata("test-owner", "test-repo")
            .await
            .unwrap_err();

        match (&undecodable, &cut_off) {
            (
                SniffError::RemoteApi {
                    status: 0,
                    message: undecodable,
                    ..
                },
                SniffError::RemoteApi {
                    status: 0,
                    message: cut_off,
                    ..
                },
            ) => {
                assert!(
                    undecodable.contains("could not be decoded"),
                    "{undecodable}"
                );
                assert!(cut_off.contains("interrupted"), "{cut_off}");
            }
            other => panic!("Expected two status-0 RemoteApi errors, got: {other:?}"),
        }
    }

    /// Serves one 200 reply that promises more body bytes than it sends, then
    /// closes the connection.
    fn truncated_body_server() -> String {
        use std::io::{Read, Write};

        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buffer = [0u8; 1024];
            while !request.windows(4).any(|window| window == b"\r\n\r\n") {
                let read = stream.read(&mut buffer).unwrap();
                if read == 0 {
                    return;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let _ = stream.write_all(
                b"HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: 100\r\n\r\n{\"name\":",
            );
        });
        format!("http://{address}")
    }
}

// =============================================================================
// GitLab Provider Tests
// =============================================================================

mod gitlab_tests {
    use super::*;

    async fn setup_gitlab_mock() -> (MockServer, GitLabRemote) {
        let server = MockServer::start().await;
        // SAFETY: Tests run sequentially and this env var is only used by this test
        unsafe { std::env::set_var("GITLAB_TOKEN", "test-gitlab-token") };
        let provider = GitLabRemote::with_base_url(&server.uri()).unwrap();
        (server, provider)
    }

    fn gitlab_tree_fixture() -> serde_json::Value {
        serde_json::json!([
            {"id": "aaa111", "name": "README.md", "type": "blob", "path": "README.md", "mode": "100644"},
            {"id": "ccc333", "name": ".gitlab-ci.yml", "type": "blob", "path": ".gitlab-ci.yml", "mode": "100644"},
            {"id": "eee555", "name": "guide.md", "type": "blob", "path": "docs/guide.md", "mode": "100644"}
        ])
    }

    fn gitlab_merge_requests_fixture() -> serde_json::Value {
        serde_json::json!([
            {
                "id": 1000,
                "iid": 15,
                "project_id": 5000,
                "title": "Implement feature X",
                "state": "opened",
                "author": {"username": "developer1", "id": 100},
                "draft": false,
                "work_in_progress": false,
                "source_branch": "feature-x",
                "target_branch": "main",
                "labels": ["testing", "feature"],
                "description": "Implements feature X as described in #123.",
                "created_at": "2024-06-10T08:00:00Z",
                "updated_at": "2024-06-15T12:00:00Z",
                "merged_at": null,
                "web_url": "https://gitlab.com/test-owner/test-repo/-/merge_requests/15"
            }
        ])
    }

    fn gitlab_issues_fixture() -> serde_json::Value {
        serde_json::json!([
            {
                "id": 2000,
                "iid": 25,
                "project_id": 5000,
                "title": "Add more tests",
                "state": "opened",
                "author": {"username": "reporter1", "id": 200},
                "user_notes_count": 3,
                "labels": ["testing", "enhancement"],
                "created_at": "2024-06-12T09:00:00Z",
                "updated_at": "2024-06-18T14:00:00Z",
                "closed_at": null,
                "web_url": "https://gitlab.com/test-owner/test-repo/-/issues/25"
            }
        ])
    }

    fn gitlab_tags_fixture() -> serde_json::Value {
        serde_json::json!([
            {
                "name": "v2.0.0",
                "target": "abc123def456",
                "message": "Release version 2.0.0",
                "commit": {
                    "id": "abc123def456",
                    "short_id": "abc123",
                    "title": "Release 2.0.0",
                    "author_name": "Release Bot",
                    "created_at": "2024-06-01T10:00:00Z"
                },
                "created_at": "2024-06-01T10:00:00Z"
            }
        ])
    }

    fn gitlab_releases_fixture() -> serde_json::Value {
        serde_json::json!([
            {
                "name": "Version 2.0.0",
                "tag_name": "v2.0.0",
                "description": "Major release",
                "created_at": "2024-06-01T10:00:00Z",
                "released_at": "2024-06-01T12:00:00Z",
                "author": {"id": 100, "username": "releaser"},
                "commit": {
                    "id": "abc123def456",
                    "short_id": "abc123",
                    "title": "Release 2.0.0",
                    "created_at": "2024-06-01T10:00:00Z"
                },
                "assets": {"count": 0, "sources": [], "links": []},
                "_links": {"self": "https://gitlab.com/test-owner/test-repo/-/releases/v2.0.0"}
            }
        ])
    }

    #[tokio::test]
    async fn get_repo_metadata_success() {
        let (server, provider) = setup_gitlab_mock().await;

        // GitLab uses ListRepositoryTree to verify project exists
        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/repository/tree.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_tree_fixture()))
            .mount(&server)
            .await;

        let metadata = provider
            .get_repo_metadata("test-owner", "test-repo")
            .await
            .unwrap();

        // GitLab provider returns minimal metadata without GetProject endpoint
        assert_eq!(metadata.name, "test-repo");
        assert_eq!(metadata.full_name, "test-owner/test-repo");
    }

    #[tokio::test]
    async fn list_documents_success() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/repository/tree.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_tree_fixture()))
            .mount(&server)
            .await;

        let documents = provider
            .list_documents("test-owner", "test-repo")
            .await
            .unwrap();

        assert!(!documents.is_empty());
        let readme_count = documents
            .iter()
            .filter(|d| d.category == DocumentCategory::Readme)
            .count();
        assert!(readme_count >= 1, "Should have at least one README");
    }

    #[tokio::test]
    async fn list_pull_requests_success() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/merge_requests.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_merge_requests_fixture()))
            .mount(&server)
            .await;

        let mrs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .unwrap();

        assert_eq!(mrs.len(), 1);
        assert_eq!(mrs[0].number, 15);
        assert_eq!(mrs[0].title, "Implement feature X");
        assert_eq!(mrs[0].state, "opened");
        assert_eq!(mrs[0].author, "developer1");
        assert_eq!(mrs[0].labels, vec!["testing", "feature"]);
        assert_eq!(
            mrs[0].body,
            Some("Implements feature X as described in #123.".to_string())
        );
    }

    #[tokio::test]
    async fn list_pull_requests_draft_filter() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/merge_requests.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 1000,
                    "iid": 15,
                    "project_id": 5000,
                    "title": "Draft MR",
                    "state": "opened",
                    "author": {"username": "developer1", "id": 100},
                    "draft": true,
                    "work_in_progress": true,
                    "source_branch": "feature-x",
                    "target_branch": "main",
                    "labels": ["testing"],
                    "description": "WIP: draft work",
                    "created_at": "2024-06-10T08:00:00Z",
                    "updated_at": "2024-06-15T12:00:00Z",
                    "merged_at": null,
                    "web_url": "https://gitlab.com/test-owner/test-repo/-/merge_requests/15"
                },
                {
                    "id": 1001,
                    "iid": 16,
                    "project_id": 5000,
                    "title": "Open MR",
                    "state": "opened",
                    "author": {"username": "developer2", "id": 101},
                    "draft": false,
                    "work_in_progress": false,
                    "source_branch": "feature-y",
                    "target_branch": "main",
                    "labels": ["feature"],
                    "description": "Ready for review",
                    "created_at": "2024-06-10T08:00:00Z",
                    "updated_at": "2024-06-15T12:00:00Z",
                    "merged_at": null,
                    "web_url": "https://gitlab.com/test-owner/test-repo/-/merge_requests/16"
                }
            ])))
            .mount(&server)
            .await;

        let mrs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Draft)
            .await
            .unwrap();

        assert_eq!(mrs.len(), 1);
        assert_eq!(mrs[0].number, 15);
        assert_eq!(mrs[0].title, "Draft MR");
        assert!(mrs[0].draft);
    }

    #[tokio::test]
    async fn list_issues_success() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/issues.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_issues_fixture()))
            .mount(&server)
            .await;

        let issues = provider
            .list_issues("test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 25);
        assert_eq!(issues[0].title, "Add more tests");
    }

    #[tokio::test]
    async fn get_tags_and_releases_success() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/repository/tags.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_tags_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/releases.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_releases_fixture()))
            .mount(&server)
            .await;

        let tags_and_releases = provider
            .get_tags_and_releases("test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(tags_and_releases.tags.len(), 1);
        assert_eq!(tags_and_releases.tags[0].name, "v2.0.0");
        assert!(tags_and_releases.tags[0].annotated); // Has message
    }

    #[tokio::test]
    async fn detect_cicd_gitlab_ci() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/repository/tree.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_tree_fixture()))
            .mount(&server)
            .await;

        let cicd = provider
            .detect_cicd("test-owner", "test-repo")
            .await
            .unwrap();

        assert!(cicd.is_some());
        let cicd_info = cicd.unwrap();
        assert_eq!(cicd_info.provider, "GitLab CI");
        assert_eq!(cicd_info.config_path, Some(".gitlab-ci.yml".to_string()));
    }

    #[tokio::test]
    async fn provider_returns_gitlab() {
        let (_, provider) = setup_gitlab_mock().await;
        assert_eq!(provider.provider(), GitProvider::GitLab);
    }

    #[tokio::test]
    async fn error_401_unauthorized() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/repository/tree.*"))
            .respond_with(ResponseTemplate::new(401).set_body_string("401 Unauthorized"))
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::MissingCredentials { provider, .. } => {
                assert_eq!(provider, "GitLab");
            }
            other => panic!("Expected MissingCredentials error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_404_not_found() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/repository/tree.*"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let result = provider
            .get_repo_metadata("test-owner", "nonexistent")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RemoteApi {
                provider, status, ..
            } => {
                assert_eq!(provider, "GitLab");
                assert_eq!(status, 404);
            }
            other => panic!("Expected RemoteApi error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_429_rate_limited() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/repository/tree.*"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RateLimited { provider, .. } => {
                assert_eq!(provider, "GitLab");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
    }

    /// The project path and the file path are each percent-encoded once, and the
    /// base64 `content` is decoded to text.
    #[tokio::test]
    async fn get_file_content_decodes_the_base64_content() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path(
                "/api/v4/projects/test-owner%2Ftest-repo/repository/files/docs%2Fguide.md",
            ))
            .and(query_param("ref", "HEAD"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "file_name": "guide.md",
                "file_path": "docs/guide.md",
                "size": 9,
                "encoding": "base64",
                "content": "IyBHdWlkZQo=",
                "ref": "HEAD",
                "blob_id": "b",
                "commit_id": "c",
                "last_commit_id": "c"
            })))
            .expect(1)
            .mount(&server)
            .await;

        let content = provider
            .get_file_content("test-owner", "test-repo", "docs/guide.md")
            .await
            .unwrap();

        assert_eq!(content, "# Guide\n");
    }
}

// =============================================================================
// Gitea Provider Tests
// =============================================================================

mod gitea_tests {
    use super::*;

    async fn setup_gitea_mock() -> (MockServer, GiteaRemote) {
        let server = MockServer::start().await;
        // SAFETY: Tests run sequentially and this env var is only used by this test
        unsafe { std::env::set_var("GITEA_TOKEN", "test-gitea-token") };
        let provider = GiteaRemote::with_base_url(&server.uri()).unwrap();
        (server, provider)
    }

    fn gitea_repo_fixture() -> serde_json::Value {
        serde_json::json!({
            "id": 54321,
            "name": "test-repo",
            "full_name": "test-owner/test-repo",
            "description": "A test repository on Gitea",
            "private": false,
            "html_url": "https://gitea.example.com/test-owner/test-repo",
            "default_branch": "main",
            "language": "Go",
            "stars_count": 15,
            "forks_count": 3,
            "open_issues_count": 2,
            "archived": false,
            "created_at": "2024-01-20T08:00:00Z",
            "updated_at": "2024-06-15T10:00:00Z"
        })
    }

    fn gitea_tree_fixture() -> serde_json::Value {
        serde_json::json!({
            "sha": "abc123def456",
            "tree": [
                {"path": "README.md", "type": "blob", "mode": "100644", "sha": "aaa111", "size": 512},
                {"path": "docs/api.md", "type": "blob", "mode": "100644", "sha": "ggg777", "size": 2048},
                {"path": ".gitea/workflows/ci.yml", "type": "blob", "mode": "100644", "sha": "eee555", "size": 256}
            ],
            "truncated": false
        })
    }

    fn gitea_pull_requests_fixture() -> serde_json::Value {
        serde_json::json!([
            {
                "number": 8,
                "title": "Add new endpoint",
                "state": "open",
                "user": {"login": "developer1", "id": 600},
                "draft": false,
                "head": {"ref": "new-endpoint"},
                "base": {"ref": "main"},
                "labels": [],
                "body": "Adds a new API endpoint.",
                "created_at": "2024-06-10T09:00:00Z",
                "updated_at": "2024-06-14T15:00:00Z",
                "merged_at": null,
                "html_url": "https://gitea.example.com/test-owner/test-repo/pulls/8"
            }
        ])
    }

    fn gitea_pull_requests_with_labels_fixture() -> serde_json::Value {
        serde_json::json!([
            {
                "number": 11,
                "title": "Labeled PR",
                "state": "open",
                "user": {"login": "developer2", "id": 601},
                "draft": false,
                "head": {"ref": "labeled-branch"},
                "base": {"ref": "main"},
                "labels": [
                    {"name": "feature"},
                    {"name": "good first issue"}
                ],
                "body": "PR with labels populated.",
                "created_at": "2024-06-11T09:00:00Z",
                "updated_at": "2024-06-15T15:00:00Z",
                "merged_at": null,
                "html_url": "https://gitea.example.com/test-owner/test-repo/pulls/11"
            }
        ])
    }

    fn gitea_issues_fixture() -> serde_json::Value {
        serde_json::json!([
            {
                "number": 20,
                "title": "Improve error messages",
                "state": "open",
                "user": {"login": "user1", "id": 700},
                "comments": 2,
                "created_at": "2024-06-08T11:00:00Z",
                "updated_at": "2024-06-12T13:00:00Z",
                "closed_at": null,
                "html_url": "https://gitea.example.com/test-owner/test-repo/issues/20"
            }
        ])
    }

    fn gitea_tags_fixture() -> serde_json::Value {
        serde_json::json!([
            {"name": "v1.5.0", "commit": {"sha": "abc123"}, "message": "Release 1.5.0"}
        ])
    }

    fn gitea_releases_fixture() -> serde_json::Value {
        serde_json::json!([
            {
                "id": 50,
                "tag_name": "v1.5.0",
                "name": "Release 1.5.0",
                "draft": false,
                "prerelease": false,
                "published_at": "2024-06-01T10:00:00Z",
                "html_url": "https://gitea.example.com/test-owner/test-repo/releases/tag/v1.5.0"
            }
        ])
    }

    #[tokio::test]
    async fn get_repo_metadata_success() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_repo_fixture()))
            .mount(&server)
            .await;

        let metadata = provider
            .get_repo_metadata("test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(metadata.name, "test-repo");
        assert_eq!(metadata.full_name, "test-owner/test-repo");
        assert_eq!(
            metadata.description,
            Some("A test repository on Gitea".to_string())
        );
        assert!(!metadata.private);
        assert_eq!(metadata.default_branch, "main");
        assert_eq!(metadata.language, Some("Go".to_string()));
        assert_eq!(metadata.stars, Some(15));
        assert_eq!(metadata.forks, Some(3));
    }

    #[tokio::test]
    async fn list_documents_success() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_repo_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/api/v1/repos/test-owner/test-repo/git/trees/.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_tree_fixture()))
            .mount(&server)
            .await;

        let documents = provider
            .list_documents("test-owner", "test-repo")
            .await
            .unwrap();

        assert!(!documents.is_empty());
        let readme_count = documents
            .iter()
            .filter(|d| d.category == DocumentCategory::Readme)
            .count();
        assert!(readme_count >= 1, "Should have at least one README");
    }

    #[tokio::test]
    async fn list_pull_requests_success() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_pull_requests_fixture()))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 8);
        assert_eq!(prs[0].title, "Add new endpoint");
        assert!(prs[0].labels.is_empty());
        assert_eq!(prs[0].body, Some("Adds a new API endpoint.".to_string()));
    }

    #[tokio::test]
    async fn list_pull_requests_with_labels() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/pulls.*"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(gitea_pull_requests_with_labels_fixture()),
            )
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 11);
        assert_eq!(prs[0].title, "Labeled PR");
        assert_eq!(
            prs[0].labels,
            vec!["feature".to_string(), "good first issue".to_string()],
            "Expected labels to round-trip through PullRequestInfo"
        );
    }

    #[tokio::test]
    async fn list_pull_requests_merged_filter() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "number": 1,
                    "title": "Merged PR",
                    "state": "closed",
                    "user": {"login": "user1", "id": 1},
                    "draft": false,
                    "head": {"ref": "branch1"},
                    "base": {"ref": "main"},
                    "labels": [],
                    "body": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "merged_at": "2024-01-02T00:00:00Z",
                    "html_url": "https://gitea.example.com/test-owner/test-repo/pulls/1"
                },
                {
                    "number": 2,
                    "title": "Closed PR",
                    "state": "closed",
                    "user": {"login": "user2", "id": 2},
                    "draft": false,
                    "head": {"ref": "branch2"},
                    "base": {"ref": "main"},
                    "labels": [],
                    "body": null,
                    "created_at": "2024-01-01T00:00:00Z",
                    "updated_at": "2024-01-02T00:00:00Z",
                    "merged_at": null,
                    "html_url": "https://gitea.example.com/test-owner/test-repo/pulls/2"
                }
            ])))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Merged)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 1);
        assert_eq!(prs[0].title, "Merged PR");
    }

    #[tokio::test]
    async fn list_issues_success() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/issues.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_issues_fixture()))
            .mount(&server)
            .await;

        let issues = provider
            .list_issues("test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 20);
        assert_eq!(issues[0].title, "Improve error messages");
    }

    #[tokio::test]
    async fn get_tags_and_releases_success() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/tags.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_tags_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/releases.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_releases_fixture()))
            .mount(&server)
            .await;

        let tags_and_releases = provider
            .get_tags_and_releases("test-owner", "test-repo")
            .await
            .unwrap();

        assert_eq!(tags_and_releases.tags.len(), 1);
        assert_eq!(tags_and_releases.tags[0].name, "v1.5.0");
    }

    #[tokio::test]
    async fn detect_cicd_gitea_actions() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_repo_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/api/v1/repos/test-owner/test-repo/git/trees/.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_tree_fixture()))
            .mount(&server)
            .await;

        let cicd = provider
            .detect_cicd("test-owner", "test-repo")
            .await
            .unwrap();

        assert!(cicd.is_some());
        let cicd_info = cicd.unwrap();
        assert_eq!(cicd_info.provider, "Gitea Actions");
    }

    #[tokio::test]
    async fn provider_returns_gitea() {
        let (_, provider) = setup_gitea_mock().await;
        assert_eq!(provider.provider(), GitProvider::Gitea);
    }

    #[tokio::test]
    async fn error_401_unauthorized() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::MissingCredentials { provider, .. } => {
                assert_eq!(provider, "Gitea");
            }
            other => panic!("Expected MissingCredentials error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_404_not_found() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/nonexistent"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let result = provider
            .get_repo_metadata("test-owner", "nonexistent")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RemoteApi {
                provider, status, ..
            } => {
                assert_eq!(provider, "Gitea");
                assert_eq!(status, 404);
            }
            other => panic!("Expected RemoteApi error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_429_rate_limited() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit"))
            .mount(&server)
            .await;

        let result = provider.get_repo_metadata("test-owner", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RateLimited { provider, .. } => {
                assert_eq!(provider, "Gitea");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn get_file_content_returns_the_raw_text() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo/raw/docs/guide.md"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/plain; charset=utf-8")
                    .set_body_string("# Guide\n"),
            )
            .expect(1)
            .mount(&server)
            .await;

        let content = provider
            .get_file_content("test-owner", "test-repo", "docs/guide.md")
            .await
            .unwrap();

        assert_eq!(content, "# Guide\n");
    }

    /// A truncated Gitea tree continues each document/CI prefix with one
    /// recursive subtree request; a prefix that fails is skipped.
    #[tokio::test]
    async fn a_truncated_tree_continues_for_document_and_ci_prefixes() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_repo_fixture()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo/git/trees/main"))
            .and(query_param("recursive", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "root",
                "tree": [{"path": "README.md", "type": "blob", "size": 1}],
                "truncated": true
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/api/v1/repos/test-owner/test-repo/git/trees/main%3Adocs",
            ))
            .and(query_param("recursive", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "d",
                "tree": [{"path": "guide.md", "type": "blob", "size": 2}],
                "truncated": false
            })))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let documents = provider
            .list_documents("test-owner", "test-repo")
            .await
            .unwrap();

        let paths: Vec<&str> = documents.iter().map(|d| d.path.as_str()).collect();
        assert!(paths.contains(&"docs/guide.md"), "{paths:?}");
        assert!(paths.contains(&"README.md"), "{paths:?}");
    }
}

// =============================================================================
// Bitbucket Provider Tests
// =============================================================================

mod bitbucket_tests {
    use super::*;

    async fn setup_bitbucket_mock() -> (MockServer, BitbucketRemote) {
        let server = MockServer::start().await;
        // SAFETY: Tests run sequentially and these env vars are only used by this test
        unsafe {
            std::env::set_var("BITBUCKET_USERNAME", "test-user");
            std::env::set_var("BITBUCKET_APP_PASSWORD", "test-app-password");
        }
        let provider = BitbucketRemote::with_base_url(&server.uri()).unwrap();
        (server, provider)
    }

    fn bitbucket_repo_fixture() -> serde_json::Value {
        serde_json::json!({
            "uuid": "{abc12345}",
            "name": "test-repo",
            "full_name": "test-workspace/test-repo",
            "description": "A test repository on Bitbucket",
            "is_private": false,
            "links": {
                "html": {"href": "https://bitbucket.org/test-workspace/test-repo"}
            },
            "mainbranch": {"name": "main", "type": "branch"},
            "language": "python",
            "created_on": "2024-02-01T12:00:00Z",
            "updated_on": "2024-06-18T08:00:00Z",
            "has_issues": true,
            "has_wiki": true
        })
    }

    fn bitbucket_directory_fixture() -> serde_json::Value {
        serde_json::json!({
            "values": [
                {"path": "README.md", "type": "commit_file", "size": 1024},
                {"path": "src", "type": "commit_directory"},
                {"path": "docs", "type": "commit_directory"},
                {"path": "CHANGELOG.md", "type": "commit_file", "size": 2048},
                {"path": "bitbucket-pipelines.yml", "type": "commit_file", "size": 512}
            ],
            "size": 5,
            "pagelen": 10,
            "page": 1
        })
    }

    fn bitbucket_pull_requests_fixture() -> serde_json::Value {
        serde_json::json!({
            "values": [
                {
                    "id": 100,
                    "title": "Refactor auth module",
                    "state": "OPEN",
                    "author": {"type": "user", "display_name": "Alice Developer"},
                    "source": {"branch": {"name": "feature/auth-refactor"}},
                    "destination": {"branch": {"name": "main"}},
                    "created_on": "2024-06-12T14:00:00Z",
                    "updated_on": "2024-06-16T09:00:00Z",
                    "links": {"html": {"href": "https://bitbucket.org/test-workspace/test-repo/pull-requests/100"}}
                }
            ],
            "size": 1,
            "pagelen": 10,
            "page": 1
        })
    }

    fn bitbucket_issues_fixture() -> serde_json::Value {
        serde_json::json!({
            "values": [
                {
                    "id": 30,
                    "title": "Update documentation",
                    "state": "new",
                    "reporter": {"type": "user", "display_name": "Doc Writer"},
                    "comment_count": 1,
                    "created_on": "2024-06-10T11:00:00Z",
                    "updated_on": "2024-06-15T14:00:00Z",
                    "links": {"html": {"href": "https://bitbucket.org/test-workspace/test-repo/issues/30"}}
                }
            ],
            "size": 1,
            "pagelen": 10,
            "page": 1
        })
    }

    fn bitbucket_tags_fixture() -> serde_json::Value {
        serde_json::json!({
            "values": [
                {
                    "name": "v3.0.0",
                    "target": {"hash": "abc123def456789"},
                    "message": "Release 3.0.0",
                    "tagger": {"raw": "Release Bot <release@example.com>"},
                    "date": "2024-06-01T12:00:00Z"
                }
            ],
            "size": 1,
            "pagelen": 10,
            "page": 1
        })
    }

    fn bitbucket_downloads_fixture() -> serde_json::Value {
        serde_json::json!({
            "values": [
                {
                    "name": "release-3.0.0.zip",
                    "links": {"self": {"href": "https://bitbucket.org/test-workspace/test-repo/downloads/release-3.0.0.zip"}},
                    "created_on": "2024-06-01T12:30:00Z",
                    "size": 1048576
                }
            ],
            "size": 1,
            "pagelen": 10,
            "page": 1
        })
    }

    #[tokio::test]
    async fn get_repo_metadata_success() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path("/repositories/test-workspace/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_repo_fixture()))
            .mount(&server)
            .await;

        let metadata = provider
            .get_repo_metadata("test-workspace", "test-repo")
            .await
            .unwrap();

        assert_eq!(metadata.name, "test-repo");
        assert_eq!(metadata.full_name, "test-workspace/test-repo");
        assert_eq!(
            metadata.description,
            Some("A test repository on Bitbucket".to_string())
        );
        assert!(!metadata.private);
        assert_eq!(metadata.default_branch, "main");
        assert_eq!(metadata.language, Some("python".to_string()));
    }

    #[tokio::test]
    async fn list_documents_success() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path("/repositories/test-workspace/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_repo_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repositories/test-workspace/test-repo/src/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_directory_fixture()))
            .mount(&server)
            .await;

        let documents = provider
            .list_documents("test-workspace", "test-repo")
            .await
            .unwrap();

        assert!(!documents.is_empty());
        let readme_count = documents
            .iter()
            .filter(|d| d.category == DocumentCategory::Readme)
            .count();
        assert!(readme_count >= 1, "Should have at least one README");
    }

    #[tokio::test]
    async fn a_malformed_directory_listing_leaves_the_tree_unavailable_not_empty() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path("/repositories/test-workspace/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_repo_fixture()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"/repositories/test-workspace/test-repo/src/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not a listing"))
            .mount(&server)
            .await;

        let snapshot = provider
            .snapshot("test-workspace", "test-repo")
            .await
            .expect("metadata succeeded, so the snapshot must not fail");

        // An undecodable listing degrades the tree to unavailable; an observed
        // empty tree would claim the repository has no documents or CI.
        assert!(!snapshot.tree.available);
        assert!(snapshot.tree.files.is_empty());
    }

    #[tokio::test]
    async fn get_file_content_returns_the_raw_bytes_as_text() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path("/repositories/test-workspace/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_repo_fixture()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(
                "/repositories/test-workspace/test-repo/src/main/README.md",
            ))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_raw(b"# Title\n\xFF caf\xC3\xA9\n".to_vec(), "text/plain"),
            )
            .mount(&server)
            .await;

        let content = provider
            .get_file_content("test-workspace", "test-repo", "README.md")
            .await
            .unwrap();

        assert_eq!(content, "# Title\n\u{FFFD} caf\u{E9}\n");
    }

    #[tokio::test]
    async fn list_pull_requests_success() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/pullrequests.*",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(bitbucket_pull_requests_fixture()),
            )
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-workspace", "test-repo", PullRequestState::Open)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 100);
        assert_eq!(prs[0].title, "Refactor auth module");
        assert_eq!(prs[0].state, "open");
        assert!(prs[0].labels.is_empty());
        assert!(prs[0].body.is_none());
    }

    #[tokio::test]
    async fn list_pull_requests_body_is_none() {
        // Bitbucket Cloud's list-pull-requests endpoint does NOT include the
        // PR description in the response. The provider therefore documents
        // this limitation by always mapping `body` to `None` in the list
        // response. Populating `body` would require a follow-up
        // `GET /pullrequests/{id}` call per PR. This test pins that
        // contract so a future schema change cannot silently regress it.
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/pullrequests.*",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(bitbucket_pull_requests_fixture()),
            )
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-workspace", "test-repo", PullRequestState::Open)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert!(
            prs[0].body.is_none(),
            "Bitbucket list endpoint does not return PR descriptions; body must be None"
        );
        assert!(
            prs[0].labels.is_empty(),
            "Bitbucket Cloud's PR API does not expose labels"
        );
    }

    #[tokio::test]
    async fn list_pull_requests_merged_filter() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/pullrequests.*",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "values": [
                        {
                            "id": 1,
                            "title": "Merged PR",
                            "state": "MERGED",
                            "author": {"type": "user", "display_name": "Alice Developer"},
                            "source": {"branch": {"name": "feature/merged"}},
                            "destination": {"branch": {"name": "main"}},
                            "created_on": "2024-06-12T14:00:00Z",
                            "updated_on": "2024-06-16T09:00:00Z",
                            "links": {"html": {"href": "https://bitbucket.org/test-workspace/test-repo/pull-requests/1"}}
                        },
                        {
                            "id": 2,
                            "title": "Declined PR",
                            "state": "DECLINED",
                            "author": {"type": "user", "display_name": "Bob Developer"},
                            "source": {"branch": {"name": "feature/declined"}},
                            "destination": {"branch": {"name": "main"}},
                            "created_on": "2024-06-12T14:00:00Z",
                            "updated_on": "2024-06-16T09:00:00Z",
                            "links": {"html": {"href": "https://bitbucket.org/test-workspace/test-repo/pull-requests/2"}}
                        }
                    ],
                    "size": 2,
                    "pagelen": 10,
                    "page": 1
                })),
            )
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-workspace", "test-repo", PullRequestState::Merged)
            .await
            .unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 1);
        assert_eq!(prs[0].title, "Merged PR");
    }

    #[tokio::test]
    async fn list_pull_requests_draft_returns_empty() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/pullrequests.*",
            ))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(bitbucket_pull_requests_fixture()),
            )
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-workspace", "test-repo", PullRequestState::Draft)
            .await
            .unwrap();

        // Bitbucket doesn't have draft PRs
        assert!(prs.is_empty());
    }

    #[tokio::test]
    async fn list_issues_success() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/issues.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_issues_fixture()))
            .mount(&server)
            .await;

        let issues = provider
            .list_issues("test-workspace", "test-repo")
            .await
            .unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 30);
        assert_eq!(issues[0].title, "Update documentation");
        assert_eq!(issues[0].state, "open"); // "new" maps to "open"
    }

    #[tokio::test]
    async fn list_issues_returns_empty_when_disabled() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/issues.*",
            ))
            .respond_with(ResponseTemplate::new(404).set_body_string("Issue tracker disabled"))
            .mount(&server)
            .await;

        let issues = provider
            .list_issues("test-workspace", "test-repo")
            .await
            .unwrap();

        // Bitbucket returns empty list when issue tracker is disabled (404)
        assert!(issues.is_empty());
    }

    #[tokio::test]
    async fn get_tags_and_releases_success() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/refs/tags.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_tags_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/downloads.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_downloads_fixture()))
            .mount(&server)
            .await;

        let tags_and_releases = provider
            .get_tags_and_releases("test-workspace", "test-repo")
            .await
            .unwrap();

        assert_eq!(tags_and_releases.tags.len(), 1);
        assert_eq!(tags_and_releases.tags[0].name, "v3.0.0");
        assert!(tags_and_releases.tags[0].annotated); // Has message
    }

    #[tokio::test]
    async fn detect_cicd_bitbucket_pipelines() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path("/repositories/test-workspace/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_repo_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repositories/test-workspace/test-repo/src/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_directory_fixture()))
            .mount(&server)
            .await;

        let cicd = provider
            .detect_cicd("test-workspace", "test-repo")
            .await
            .unwrap();

        assert!(cicd.is_some());
        let cicd_info = cicd.unwrap();
        assert_eq!(cicd_info.provider, "Bitbucket Pipelines");
        assert_eq!(
            cicd_info.config_path,
            Some("bitbucket-pipelines.yml".to_string())
        );
    }

    #[tokio::test]
    async fn provider_returns_bitbucket() {
        let (_, provider) = setup_bitbucket_mock().await;
        assert_eq!(provider.provider(), GitProvider::Bitbucket);
    }

    #[tokio::test]
    async fn error_401_unauthorized() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path("/repositories/test-workspace/test-repo"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let result = provider
            .get_repo_metadata("test-workspace", "test-repo")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::MissingCredentials { provider, .. } => {
                assert_eq!(provider, "Bitbucket");
            }
            other => panic!("Expected MissingCredentials error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_404_not_found() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path("/repositories/test-workspace/nonexistent"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Not Found"))
            .mount(&server)
            .await;

        let result = provider
            .get_repo_metadata("test-workspace", "nonexistent")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RemoteApi {
                provider, status, ..
            } => {
                assert_eq!(provider, "Bitbucket");
                assert_eq!(status, 404);
            }
            other => panic!("Expected RemoteApi error, got: {:?}", other),
        }
    }

    #[tokio::test]
    async fn error_429_rate_limited() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path("/repositories/test-workspace/test-repo"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Rate limit exceeded"))
            .mount(&server)
            .await;

        let result = provider
            .get_repo_metadata("test-workspace", "test-repo")
            .await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RateLimited { provider, .. } => {
                assert_eq!(provider, "Bitbucket");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
    }

    /// Downloads are optional evidence: a failed listing leaves the releases
    /// empty without failing the tags.
    #[tokio::test]
    async fn get_tags_and_releases_treats_a_failed_downloads_listing_as_empty() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/refs/tags.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_tags_fixture()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/downloads.*",
            ))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .expect(1)
            .mount(&server)
            .await;

        let tags_and_releases = provider
            .get_tags_and_releases("test-workspace", "test-repo")
            .await
            .unwrap();

        assert_eq!(tags_and_releases.tags.len(), 1);
        assert!(tags_and_releases.releases.is_empty());
    }
}

// =============================================================================
// Anonymous Request Tests
// =============================================================================
//
// A missing credential sends the request anonymously: the generated client no
// longer refuses it, so there is nothing to retry. Per provider:
//
// 1. `<provider>_list_pull_requests_sends_anonymously_when_token_missing`
//    - With NO credentials in the environment, exactly one request reaches the
//      server, it carries no `Authorization` header, and its result is
//      returned.
//
// 2. `<provider>_list_pull_requests_returns_*_credentials_when_anonymous_rejected`
//    - With NO credentials, a 401 is classified after exactly one request.
//      GitHub maps 401 → `InvalidCredentials`; the others map 401 →
//      `MissingCredentials`.
//
// GitHub `list_workflow_runs` keeps one consumer retry: a 401/403 to a request
// that carried a credential is resent once with no credential at all.
//
// Every test clears the host's provider credentials through `EnvGuard` and is
// `#[serial]`, so a developer's real tokens never reach the mock server.

mod anonymous_request_tests {
    use super::*;
    use serial_test::serial;

    /// Clears every credential variable the four providers read.
    fn without_credentials() -> Vec<EnvGuard> {
        PROVIDER_CREDENTIAL_VARIABLES
            .into_iter()
            .map(EnvGuard::remove_safe)
            .collect()
    }

    /// The requests the server received, asserting that none carried a
    /// credential.
    async fn anonymous_requests(server: &MockServer) -> usize {
        let requests = server.received_requests().await.unwrap();
        for request in &requests {
            assert!(
                request.headers.get("authorization").is_none(),
                "an anonymous request carried Authorization"
            );
        }
        requests.len()
    }

    // -----------------------------------------------------------------------
    // GitHub
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn github_list_pull_requests_sends_anonymously_when_token_missing() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "number": 42,
                    "title": "Anonymous PR",
                    "state": "open",
                    "user": {"login": "anon", "id": 1},
                    "draft": false,
                    "head": {"ref": "branch", "sha": "abc"},
                    "base": {"ref": "main", "sha": "def"},
                    "labels": [],
                    "body": null,
                    "created_at": "2024-06-15T10:00:00Z",
                    "updated_at": "2024-06-18T14:30:00Z",
                    "merged_at": null,
                    "html_url": "https://github.com/test-owner/test-repo/pull/42",
                    "url": "https://api.github.com/repos/test-owner/test-repo/pulls/42"
                }
            ])))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .expect("an anonymous request should succeed when the remote returns 200");

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 42);
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    #[tokio::test]
    #[serial]
    async fn github_list_pull_requests_returns_invalid_credentials_when_anonymous_rejected() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Bad credentials"))
            .mount(&server)
            .await;

        let result = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await;

        match result {
            Err(SniffError::InvalidCredentials { provider, .. }) => {
                assert_eq!(provider, "GitHub");
            }
            other => panic!(
                "Expected InvalidCredentials error from GitHub, got: {:?}",
                other
            ),
        }
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    #[tokio::test]
    #[serial]
    async fn github_list_workflow_runs_sends_anonymously_when_token_missing() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_workflow_runs_fixture()))
            .mount(&server)
            .await;

        let runs = provider
            .list_workflow_runs("test-owner", "test-repo", 5)
            .await
            .expect("an anonymous request should succeed when the remote returns 200");

        assert_eq!(runs.len(), 2);
        assert_eq!(runs[0].name, "CI");
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    #[tokio::test]
    #[serial]
    async fn github_list_workflow_runs_is_not_retried_when_an_anonymous_request_is_rejected() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/runs"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Bad credentials"))
            .mount(&server)
            .await;

        let result = provider
            .list_workflow_runs("test-owner", "test-repo", 5)
            .await;

        assert!(
            matches!(result, Err(SniffError::InvalidCredentials { .. })),
            "{result:?}"
        );
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    /// A 401/403 to a request carrying `GITHUB_TOKEN` is retried once with no
    /// credential; the rejected token is not resent.
    async fn assert_rejected_token_is_retried_anonymously(rejection: u16) {
        let _env = without_credentials();
        let _token = EnvGuard::set_safe("GITHUB_TOKEN", "workflow-token-6d2a");
        let server = MockServer::start().await;
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        // Mounted first, so it answers every request that carries a credential.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/runs"))
            .and(header_exists("authorization"))
            .respond_with(ResponseTemplate::new(rejection).set_body_string("Bad credentials"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/actions/runs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_workflow_runs_fixture()))
            .mount(&server)
            .await;

        let runs = provider
            .list_workflow_runs("test-owner", "test-repo", 5)
            .await
            .expect("the anonymous retry should succeed");

        assert_eq!(runs.len(), 2);
        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests.len(),
            2,
            "one rejected request, one anonymous retry"
        );
        assert_eq!(
            requests[0].headers.get("authorization").unwrap(),
            "Bearer workflow-token-6d2a"
        );
        assert!(
            requests[1].headers.get("authorization").is_none(),
            "the retry must not resend the rejected credential"
        );
    }

    #[tokio::test]
    #[serial]
    async fn github_list_workflow_runs_retries_anonymously_when_token_gets_401() {
        assert_rejected_token_is_retried_anonymously(401).await;
    }

    #[tokio::test]
    #[serial]
    async fn github_list_workflow_runs_retries_anonymously_when_token_gets_403() {
        assert_rejected_token_is_retried_anonymously(403).await;
    }

    // -----------------------------------------------------------------------
    // Gitea
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn gitea_list_pull_requests_sends_anonymously_when_token_missing() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = GiteaRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "number": 8,
                    "title": "Anonymous Gitea PR",
                    "state": "open",
                    "user": {"login": "anon", "id": 1},
                    "draft": false,
                    "head": {"ref": "branch"},
                    "base": {"ref": "main"},
                    "labels": [],
                    "body": "Body",
                    "created_at": "2024-06-10T09:00:00Z",
                    "updated_at": "2024-06-14T15:00:00Z",
                    "merged_at": null,
                    "html_url": "https://gitea.example.com/test-owner/test-repo/pulls/8"
                }
            ])))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .expect("an anonymous request should succeed when Gitea returns 200");

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 8);
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    #[tokio::test]
    #[serial]
    async fn gitea_list_pull_requests_returns_missing_credentials_when_anonymous_rejected() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = GiteaRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let result = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await;

        match result {
            Err(SniffError::MissingCredentials { provider, .. }) => {
                assert_eq!(provider, "Gitea");
            }
            other => panic!(
                "Expected MissingCredentials error from Gitea, got: {:?}",
                other
            ),
        }
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    // -----------------------------------------------------------------------
    // GitLab
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn gitlab_list_pull_requests_sends_anonymously_when_token_missing() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = GitLabRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/merge_requests.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {
                    "id": 1000,
                    "iid": 15,
                    "project_id": 5000,
                    "title": "Anonymous MR",
                    "state": "opened",
                    "author": {"username": "anon", "id": 1},
                    "draft": false,
                    "work_in_progress": false,
                    "source_branch": "feature",
                    "target_branch": "main",
                    "labels": [],
                    "description": "Body",
                    "created_at": "2024-06-10T08:00:00Z",
                    "updated_at": "2024-06-15T12:00:00Z",
                    "merged_at": null,
                    "web_url": "https://gitlab.com/test-owner/test-repo/-/merge_requests/15"
                }
            ])))
            .mount(&server)
            .await;

        let mrs = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await
            .expect("an anonymous request should succeed when GitLab returns 200");

        assert_eq!(mrs.len(), 1);
        assert_eq!(mrs[0].number, 15);
        let requests = server.received_requests().await.unwrap();
        assert!(requests[0].headers.get("private-token").is_none());
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    #[tokio::test]
    #[serial]
    async fn gitlab_list_pull_requests_returns_missing_credentials_when_anonymous_rejected() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = GitLabRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/merge_requests.*"))
            .respond_with(ResponseTemplate::new(401).set_body_string("401 Unauthorized"))
            .mount(&server)
            .await;

        let result = provider
            .list_pull_requests("test-owner", "test-repo", PullRequestState::Open)
            .await;

        match result {
            Err(SniffError::MissingCredentials { provider, .. }) => {
                assert_eq!(provider, "GitLab");
            }
            other => panic!(
                "Expected MissingCredentials error from GitLab, got: {:?}",
                other
            ),
        }
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    // -----------------------------------------------------------------------
    // Bitbucket
    // -----------------------------------------------------------------------

    #[tokio::test]
    #[serial]
    async fn bitbucket_list_pull_requests_sends_anonymously_when_token_missing() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = BitbucketRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/pullrequests.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "values": [
                    {
                        "id": 100,
                        "title": "Anonymous Bitbucket PR",
                        "state": "OPEN",
                        "author": {"type": "user", "display_name": "Anon"},
                        "source": {"branch": {"name": "feature"}},
                        "destination": {"branch": {"name": "main"}},
                        "created_on": "2024-06-12T14:00:00Z",
                        "updated_on": "2024-06-16T09:00:00Z",
                        "links": {"html": {"href": "https://bitbucket.org/test-workspace/test-repo/pull-requests/100"}}
                    }
                ],
                "size": 1,
                "pagelen": 10,
                "page": 1
            })))
            .mount(&server)
            .await;

        let prs = provider
            .list_pull_requests("test-workspace", "test-repo", PullRequestState::Open)
            .await
            .expect("an anonymous request should succeed when Bitbucket returns 200");

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 100);
        assert_eq!(anonymous_requests(&server).await, 1);
    }

    #[tokio::test]
    #[serial]
    async fn bitbucket_list_pull_requests_returns_missing_credentials_when_anonymous_rejected() {
        let _env = without_credentials();
        let server = MockServer::start().await;
        let provider = BitbucketRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(
                r"/repositories/test-workspace/test-repo/pullrequests.*",
            ))
            .respond_with(ResponseTemplate::new(401).set_body_string("Unauthorized"))
            .mount(&server)
            .await;

        let result = provider
            .list_pull_requests("test-workspace", "test-repo", PullRequestState::Open)
            .await;

        match result {
            Err(SniffError::MissingCredentials { provider, .. }) => {
                assert_eq!(provider, "Bitbucket");
            }
            other => panic!(
                "Expected MissingCredentials error from Bitbucket, got: {:?}",
                other
            ),
        }
        assert_eq!(anonymous_requests(&server).await, 1);
    }
}

// =============================================================================
// Secret Redaction Tests
// =============================================================================
//
// A credential sent with a request must never appear in the `SniffError` a
// failure produces, through either `Display` or `Debug`. The provider bodies
// here do not echo it, so any appearance would be sniff's own doing.

mod secret_redaction_tests {
    use super::*;
    use serial_test::serial;

    const SENTINEL: &str = "sniff-sentinel-token-9b7e31";

    fn assert_no_sentinel(error: &SniffError) {
        let shown = format!("{error} {error:?}");
        assert!(
            !shown.contains(SENTINEL),
            "the credential leaked into the error: {shown}"
        );
    }

    #[tokio::test]
    #[serial]
    async fn a_sent_token_never_appears_in_a_classified_error() {
        let _token = EnvGuard::set_safe("GITHUB_TOKEN", SENTINEL);
        let server = MockServer::start().await;
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/broken"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal error"))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/private"))
            .respond_with(
                ResponseTemplate::new(401)
                    .set_body_json(serde_json::json!({"message": "Bad credentials"})),
            )
            .mount(&server)
            .await;

        let failed = provider
            .get_repo_metadata("test-owner", "broken")
            .await
            .unwrap_err();
        let rejected = provider
            .get_repo_metadata("test-owner", "private")
            .await
            .unwrap_err();

        let requests = server.received_requests().await.unwrap();
        assert_eq!(
            requests[0].headers.get("authorization").unwrap(),
            &format!("Bearer {SENTINEL}"),
            "the sentinel must actually be sent for this test to prove anything"
        );
        assert!(matches!(failed, SniffError::RemoteApi { status: 500, .. }));
        assert!(matches!(rejected, SniffError::InvalidCredentials { .. }));
        assert_no_sentinel(&failed);
        assert_no_sentinel(&rejected);
    }

    #[tokio::test]
    #[serial]
    async fn a_sent_token_never_appears_in_a_transport_error() {
        let _token = EnvGuard::set_safe("GITHUB_TOKEN", SENTINEL);
        // Bound and released, so nothing listens on the port.
        let address = std::net::TcpListener::bind("127.0.0.1:0")
            .unwrap()
            .local_addr()
            .unwrap();
        let provider = GitHubRemote::with_base_url(&format!("http://{address}")).unwrap();

        let error = provider
            .get_repo_metadata("test-owner", "test-repo")
            .await
            .unwrap_err();

        assert!(
            matches!(error, SniffError::RemoteApi { status: 0, .. }),
            "{error:?}"
        );
        assert_no_sentinel(&error);
    }
}

// =============================================================================
// Shared-snapshot request bounds (R11)
// =============================================================================
//
// One `fetch_report` used to resolve the same evidence up to three times:
// `get_repo_metadata`, `list_documents`, and `detect_cicd` each fetched their own
// metadata (for the default branch) and then their own identical recursive tree.
// These tests pin the bound with wiremock `expect(..)`, which is verified when the
// `MockServer` drops.

mod snapshot_reuse_tests {
    use super::*;

    // The per-provider modules keep their fixtures private; these mirror the
    // minimum shape each provider's snapshot reads.

    fn gitea_repo_fixture() -> serde_json::Value {
        serde_json::json!({
            "id": 54321,
            "name": "test-repo",
            "full_name": "test-owner/test-repo",
            "private": false,
            "html_url": "https://gitea.example.com/test-owner/test-repo",
            "default_branch": "main",
            "archived": false
        })
    }

    fn gitea_tree_fixture() -> serde_json::Value {
        serde_json::json!({
            "sha": "abc123def456",
            "tree": [
                {"path": "README.md", "type": "blob", "mode": "100644", "sha": "aaa111", "size": 512},
                {"path": ".gitea/workflows/ci.yml", "type": "blob", "mode": "100644", "sha": "eee555", "size": 256}
            ],
            "truncated": false
        })
    }

    fn gitlab_tree_fixture() -> serde_json::Value {
        serde_json::json!([
            {"id": "aaa111", "name": "README.md", "type": "blob", "path": "README.md", "mode": "100644"},
            {"id": "ccc333", "name": ".gitlab-ci.yml", "type": "blob", "path": ".gitlab-ci.yml", "mode": "100644"}
        ])
    }

    /// GitHub: one metadata call and one root-tree call per report.
    ///
    /// The report also asks for workflow runs; when those come back non-empty the
    /// CI/CD ladder short-circuits before `detect_cicd_with`, but that must not
    /// change the *evidence* count — the snapshot is resolved either way.
    #[tokio::test]
    async fn github_fetch_report_resolves_metadata_and_tree_once() {
        let server = MockServer::start().await;
        unsafe { std::env::set_var("GITHUB_TOKEN", "test-token-12345") };
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_repo_fixture()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/git/trees/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_tree_fixture()))
            .expect(1)
            .mount(&server)
            .await;
        // Everything else the report fans out to; unbounded, not under test here.
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let report = provider
            .fetch_report("test-owner", "test-repo")
            .await
            .unwrap();

        // The projections still see the shared evidence.
        assert!(report.documents.iter().any(|d| d.path == "README.md"));
        assert!(
            report
                .documents
                .iter()
                .any(|d| d.category == DocumentCategory::DocsFolder)
        );
        assert!(
            report.ci_cd.iter().any(|c| c.provider == "GitHub Actions"),
            "the .github/workflows blob in the shared tree must still be detected"
        );

        drop(server);
    }

    /// Gitea: the ladder always falls through to `detect_cicd` (no workflow-run
    /// override), which is exactly the case that used to pay 3 metadata + 2 tree.
    #[tokio::test]
    async fn gitea_fetch_report_resolves_metadata_and_tree_once() {
        let server = MockServer::start().await;
        unsafe { std::env::set_var("GITEA_TOKEN", "test-token-12345") };
        let provider = GiteaRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/api/v1/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_repo_fixture()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(
                r"/api/v1/repos/test-owner/test-repo/git/trees/.*",
            ))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_tree_fixture()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let report = provider
            .fetch_report("test-owner", "test-repo")
            .await
            .unwrap();
        assert!(report.documents.iter().any(|d| d.path == "README.md"));

        drop(server);
    }

    /// GitLab: absent `GetProject`, its metadata call *is* a tree fetch. Keeping
    /// that result collapses three identical tree requests into one.
    #[tokio::test]
    async fn gitlab_fetch_report_fetches_the_tree_once() {
        let server = MockServer::start().await;
        unsafe { std::env::set_var("GITLAB_TOKEN", "test-token-12345") };
        let provider = GitLabRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/repository/tree.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_tree_fixture()))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let report = provider
            .fetch_report("test-owner", "test-repo")
            .await
            .unwrap();
        assert_eq!(report.metadata.full_name, "test-owner/test-repo");

        drop(server);
    }

    /// A failed tree must not sink a report whose required metadata succeeded:
    /// metadata survives, documents degrade to empty, CI/CD to `None` (R11.6).
    #[tokio::test]
    async fn a_failed_tree_preserves_metadata_and_degrades_optional_sections() {
        let server = MockServer::start().await;
        unsafe { std::env::set_var("GITHUB_TOKEN", "test-token-12345") };
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_repo_fixture()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/git/trees/.*"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let report = provider
            .fetch_report("test-owner", "test-repo")
            .await
            .expect("metadata succeeded, so the report must not fail");

        assert_eq!(report.metadata.name, "test-repo");
        assert!(report.documents.is_empty());
        assert!(report.ci_cd.is_empty());
    }

    /// A provider-reported truncated tree is not complete: bounded subtree
    /// continuations recover the document/CI prefixes the projections read (R11.5).
    #[tokio::test]
    async fn a_truncated_github_tree_continues_for_document_and_ci_prefixes() {
        let server = MockServer::start().await;
        unsafe { std::env::set_var("GITHUB_TOKEN", "test-token-12345") };
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_repo_fixture()))
            .mount(&server)
            .await;

        // The root tree truncated before reaching docs/ or .github/workflows/.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/git/trees/main"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "root",
                "url": "https://api.github.com/x",
                "tree": [
                    {"path": "README.md", "type": "blob", "mode": "100644", "sha": "a", "size": 1}
                ],
                "truncated": true
            })))
            .mount(&server)
            .await;

        // `branch:path` subtree continuations. Paths are subtree-relative.
        // The client percent-encodes the whole `branch:prefix` tree_sha into one
        // path segment, so the subtree address arrives as `main%3Adocs`.
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/git/trees/main%3Adocs"))
            .and(query_param("recursive", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "d", "url": "https://api.github.com/x",
                "tree": [
                    {"path": "guide.md", "type": "blob", "mode": "100644", "sha": "b", "size": 2}
                ],
                "truncated": false
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/git/trees/main%3A.github"))
            .and(query_param("recursive", "true"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "w", "url": "https://api.github.com/x",
                "tree": [
                    {"path": "workflows/ci.yml", "type": "blob", "mode": "100644", "sha": "c", "size": 3}
                ],
                "truncated": false
            })))
            .mount(&server)
            .await;
        // Prefixes this repo does not have 404, and must be skipped, not fatal.
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let documents = provider
            .list_documents("test-owner", "test-repo")
            .await
            .unwrap();
        assert!(
            documents.iter().any(|d| d.path == "docs/guide.md"),
            "a continued subtree entry must be re-prefixed to a repo-root path"
        );
        assert!(documents.iter().any(|d| d.path == "README.md"));

        let cicd = provider
            .detect_cicd("test-owner", "test-repo")
            .await
            .unwrap();
        assert!(
            cicd.is_some(),
            "workflows recovered by continuation must still detect CI"
        );
    }

    /// Without continuation a truncated tree reports "no docs, no CI" with full
    /// confidence — the defect R11.5 names. This pins that the flag is read at all.
    #[tokio::test]
    async fn a_truncated_tree_with_no_recoverable_prefixes_still_returns_root_entries() {
        let server = MockServer::start().await;
        unsafe { std::env::set_var("GITHUB_TOKEN", "test-token-12345") };
        let provider = GitHubRemote::with_base_url(&server.uri()).unwrap();

        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_repo_fixture()))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/repos/test-owner/test-repo/git/trees/main"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sha": "root", "url": "https://api.github.com/x",
                "tree": [
                    {"path": "README.md", "type": "blob", "mode": "100644", "sha": "a", "size": 1}
                ],
                "truncated": true
            })))
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(404))
            .mount(&server)
            .await;

        let documents = provider
            .list_documents("test-owner", "test-repo")
            .await
            .unwrap();
        assert_eq!(
            documents.len(),
            1,
            "a failed continuation must not discard the root entries"
        );
        assert_eq!(documents[0].path, "README.md");
    }
}
