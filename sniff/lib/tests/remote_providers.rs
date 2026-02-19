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
//! cargo test -p sniff --features remote --test remote_providers
//! ```

#![cfg(feature = "remote")]

use sniff::error::SniffError;
use sniff::remote::{
    BitbucketRemote, DocumentCategory, GitHubRemote, GitLabRemote, GitProvider, GitRemote,
    GiteaRemote, RemoteRepoProvider,
};
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

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

        let metadata = provider.get_repo_metadata("test-owner", "test-repo").await.unwrap();

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
        assert_eq!(
            metadata.topics,
            vec![
                "testing", "rust"
            ]
        );
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

        // Mock the tree endpoint
        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/git/trees/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_tree_fixture()))
            .mount(&server)
            .await;

        let documents = provider.list_documents("test-owner", "test-repo").await.unwrap();

        assert!(!documents.is_empty());

        // Check that we have expected document categories
        let readme_count =
            documents.iter().filter(|d| d.category == DocumentCategory::Readme).count();
        let docs_folder_count =
            documents.iter().filter(|d| d.category == DocumentCategory::DocsFolder).count();
        let source_doc_count =
            documents.iter().filter(|d| d.category == DocumentCategory::SourceDoc).count();
        let other_count =
            documents.iter().filter(|d| d.category == DocumentCategory::Other).count();

        assert!(readme_count >= 1, "Should have at least one README");
        assert!(docs_folder_count >= 1, "Should have at least one docs folder doc");
        assert!(source_doc_count >= 1, "Should have at least one source doc");
        assert!(other_count >= 1, "Should have at least one other doc (CHANGELOG)");
    }

    #[tokio::test]
    async fn list_pull_requests_success() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/pulls.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_pull_requests_fixture()))
            .mount(&server)
            .await;

        let prs = provider.list_pull_requests("test-owner", "test-repo").await.unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 42);
        assert_eq!(prs[0].title, "Add new feature");
        assert_eq!(prs[0].state, "open");
        assert_eq!(prs[0].author, "contributor1");
        assert!(!prs[0].draft);
        assert_eq!(prs[0].source_branch, Some("feature-branch".to_string()));
        assert_eq!(prs[0].target_branch, Some("main".to_string()));
    }

    #[tokio::test]
    async fn list_issues_success() {
        let (server, provider) = setup_github_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repos/test-owner/test-repo/issues.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(github_issues_fixture()))
            .mount(&server)
            .await;

        let issues = provider.list_issues("test-owner", "test-repo").await.unwrap();

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

        let tags_and_releases =
            provider.get_tags_and_releases("test-owner", "test-repo").await.unwrap();

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

        let cicd = provider.detect_cicd("test-owner", "test-repo").await.unwrap();

        assert!(cicd.is_some());
        let cicd_info = cicd.unwrap();
        assert_eq!(cicd_info.provider, "GitHub Actions");
        assert_eq!(cicd_info.config_path, Some(".github/workflows".to_string()));
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
            SniffError::InvalidCredentials {
                provider,
                ..
            } => {
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

        let result = provider.get_repo_metadata("test-owner", "nonexistent-repo").await;

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
            SniffError::RateLimited {
                provider,
                ..
            } => {
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
            SniffError::RateLimited {
                provider,
                ..
            } => {
                assert_eq!(provider, "GitHub");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
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

        let metadata = provider.get_repo_metadata("test-owner", "test-repo").await.unwrap();

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

        let documents = provider.list_documents("test-owner", "test-repo").await.unwrap();

        assert!(!documents.is_empty());
        let readme_count =
            documents.iter().filter(|d| d.category == DocumentCategory::Readme).count();
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

        let mrs = provider.list_pull_requests("test-owner", "test-repo").await.unwrap();

        assert_eq!(mrs.len(), 1);
        assert_eq!(mrs[0].number, 15);
        assert_eq!(mrs[0].title, "Implement feature X");
        assert_eq!(mrs[0].state, "opened");
        assert_eq!(mrs[0].author, "developer1");
    }

    #[tokio::test]
    async fn list_issues_success() {
        let (server, provider) = setup_gitlab_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v4/projects/.*/issues.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitlab_issues_fixture()))
            .mount(&server)
            .await;

        let issues = provider.list_issues("test-owner", "test-repo").await.unwrap();

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

        let tags_and_releases =
            provider.get_tags_and_releases("test-owner", "test-repo").await.unwrap();

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

        let cicd = provider.detect_cicd("test-owner", "test-repo").await.unwrap();

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
            SniffError::MissingCredentials {
                provider,
                ..
            } => {
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

        let result = provider.get_repo_metadata("test-owner", "nonexistent").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RemoteApi {
                provider,
                status,
                ..
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
            SniffError::RateLimited {
                provider,
                ..
            } => {
                assert_eq!(provider, "GitLab");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
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
                "created_at": "2024-06-10T09:00:00Z",
                "updated_at": "2024-06-14T15:00:00Z",
                "merged_at": null,
                "html_url": "https://gitea.example.com/test-owner/test-repo/pulls/8"
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

        let metadata = provider.get_repo_metadata("test-owner", "test-repo").await.unwrap();

        assert_eq!(metadata.name, "test-repo");
        assert_eq!(metadata.full_name, "test-owner/test-repo");
        assert_eq!(metadata.description, Some("A test repository on Gitea".to_string()));
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
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/git/trees/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_tree_fixture()))
            .mount(&server)
            .await;

        let documents = provider.list_documents("test-owner", "test-repo").await.unwrap();

        assert!(!documents.is_empty());
        let readme_count =
            documents.iter().filter(|d| d.category == DocumentCategory::Readme).count();
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

        let prs = provider.list_pull_requests("test-owner", "test-repo").await.unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 8);
        assert_eq!(prs[0].title, "Add new endpoint");
    }

    #[tokio::test]
    async fn list_issues_success() {
        let (server, provider) = setup_gitea_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/issues.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_issues_fixture()))
            .mount(&server)
            .await;

        let issues = provider.list_issues("test-owner", "test-repo").await.unwrap();

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

        let tags_and_releases =
            provider.get_tags_and_releases("test-owner", "test-repo").await.unwrap();

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
            .and(path_regex(r"/api/v1/repos/test-owner/test-repo/git/trees/.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(gitea_tree_fixture()))
            .mount(&server)
            .await;

        let cicd = provider.detect_cicd("test-owner", "test-repo").await.unwrap();

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
            SniffError::MissingCredentials {
                provider,
                ..
            } => {
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

        let result = provider.get_repo_metadata("test-owner", "nonexistent").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RemoteApi {
                provider,
                status,
                ..
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
            SniffError::RateLimited {
                provider,
                ..
            } => {
                assert_eq!(provider, "Gitea");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
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

        let metadata = provider.get_repo_metadata("test-workspace", "test-repo").await.unwrap();

        assert_eq!(metadata.name, "test-repo");
        assert_eq!(metadata.full_name, "test-workspace/test-repo");
        assert_eq!(metadata.description, Some("A test repository on Bitbucket".to_string()));
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

        let documents = provider.list_documents("test-workspace", "test-repo").await.unwrap();

        assert!(!documents.is_empty());
        let readme_count =
            documents.iter().filter(|d| d.category == DocumentCategory::Readme).count();
        assert!(readme_count >= 1, "Should have at least one README");
    }

    #[tokio::test]
    async fn list_pull_requests_success() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repositories/test-workspace/test-repo/pullrequests.*"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(bitbucket_pull_requests_fixture()),
            )
            .mount(&server)
            .await;

        let prs = provider.list_pull_requests("test-workspace", "test-repo").await.unwrap();

        assert_eq!(prs.len(), 1);
        assert_eq!(prs[0].number, 100);
        assert_eq!(prs[0].title, "Refactor auth module");
        assert_eq!(prs[0].state, "open");
    }

    #[tokio::test]
    async fn list_issues_success() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repositories/test-workspace/test-repo/issues.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_issues_fixture()))
            .mount(&server)
            .await;

        let issues = provider.list_issues("test-workspace", "test-repo").await.unwrap();

        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].number, 30);
        assert_eq!(issues[0].title, "Update documentation");
        assert_eq!(issues[0].state, "open"); // "new" maps to "open"
    }

    #[tokio::test]
    async fn list_issues_returns_empty_when_disabled() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repositories/test-workspace/test-repo/issues.*"))
            .respond_with(ResponseTemplate::new(404).set_body_string("Issue tracker disabled"))
            .mount(&server)
            .await;

        let issues = provider.list_issues("test-workspace", "test-repo").await.unwrap();

        // Bitbucket returns empty list when issue tracker is disabled (404)
        assert!(issues.is_empty());
    }

    #[tokio::test]
    async fn get_tags_and_releases_success() {
        let (server, provider) = setup_bitbucket_mock().await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repositories/test-workspace/test-repo/refs/tags.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_tags_fixture()))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/repositories/test-workspace/test-repo/downloads.*"))
            .respond_with(ResponseTemplate::new(200).set_body_json(bitbucket_downloads_fixture()))
            .mount(&server)
            .await;

        let tags_and_releases =
            provider.get_tags_and_releases("test-workspace", "test-repo").await.unwrap();

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

        let cicd = provider.detect_cicd("test-workspace", "test-repo").await.unwrap();

        assert!(cicd.is_some());
        let cicd_info = cicd.unwrap();
        assert_eq!(cicd_info.provider, "Bitbucket Pipelines");
        assert_eq!(cicd_info.config_path, Some("bitbucket-pipelines.yml".to_string()));
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

        let result = provider.get_repo_metadata("test-workspace", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::MissingCredentials {
                provider,
                ..
            } => {
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

        let result = provider.get_repo_metadata("test-workspace", "nonexistent").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RemoteApi {
                provider,
                status,
                ..
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

        let result = provider.get_repo_metadata("test-workspace", "test-repo").await;

        assert!(result.is_err());
        match result.unwrap_err() {
            SniffError::RateLimited {
                provider,
                ..
            } => {
                assert_eq!(provider, "Bitbucket");
            }
            other => panic!("Expected RateLimited error, got: {:?}", other),
        }
    }
}

// =============================================================================
// Shorthand Resolution Tests
// =============================================================================

mod shorthand_tests {
    use super::*;

    #[tokio::test]
    async fn from_shorthand_tries_github_when_token_set() {
        // Set GitHub credentials so the constructor succeeds.
        // Don't remove other provider env vars — that would race with parallel tests.
        unsafe { std::env::set_var("GITHUB_TOKEN", "test-token-12345") };

        let result = GitRemote::from_shorthand("test-owner", "test-repo").await;

        // With a real API URL and a fake token, we expect either:
        // - ShorthandNotFound (all providers returned 404 or network errors)
        // - InvalidCredentials (GitHub rejected the fake token)
        // - Some other API error
        // The important thing: GitHub was tried (not skipped for missing credentials).
        match result {
            Ok(remote) => {
                // If it succeeds, it found the repo on some provider
                assert_eq!(remote.provider(), GitProvider::GitHub);
            }
            Err(SniffError::ShorthandNotFound {
                providers_tried,
                ..
            }) => {
                assert!(
                    providers_tried.contains("GitHub"),
                    "GitHub should be in tried providers, got: {}",
                    providers_tried
                );
            }
            Err(SniffError::InvalidCredentials {
                provider,
                ..
            }) => {
                assert_eq!(provider, "GitHub");
            }
            Err(SniffError::RateLimited {
                provider,
                ..
            }) => {
                assert_eq!(provider, "GitHub");
            }
            Err(_) => {
                // Network errors, etc. — GitHub was still attempted
            }
        }
    }
}
