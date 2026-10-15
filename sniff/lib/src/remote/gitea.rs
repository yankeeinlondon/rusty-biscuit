//! Gitea remote repository provider.
//!
//! This module implements [`RemoteRepoProvider`] for Gitea using the
//! generated schematic-schema client.
//!
//! Gitea is a self-hosted Git service, so a base URL is always required.
//! The [`GiteaRemote::codeberg`] constructor provides a convenient way to
//! access [Codeberg](https://codeberg.org), a popular public Gitea instance.

use async_trait::async_trait;
use schematic_schema::gitea::*;

use super::{
    count_api_request,
    envelope::{Replies, Unauthorized},
    provider::RemoteRepoProvider,
    snapshot::{
        CONTINUATION_PREFIXES, RemoteRepoSnapshot, RemoteTree, RemoteTreeFile, documents_from_tree,
    },
    types::{
        CiCdInfo, DocumentRef, GitProvider, IssueInfo, KeyUrls, OrgInfo, OrgRepoRef,
        PullRequestInfo, PullRequestState, ReleaseInfo, RepoMetadata, TagInfo, TagsAndReleases,
    },
};
use crate::error::SniffError;

const REPLIES: Replies = Replies::new("Gitea", Unauthorized::Missing("GITEA_TOKEN"));

/// Codeberg's API root.
const CODEBERG_API_URL: &str = "https://codeberg.org/api/v1";

/// Gitea remote repository provider.
///
/// Uses the schematic-generated Gitea API client for all HTTP operations.
/// Each request sends a non-empty `GITEA_TOKEN`, and is sent anonymously when
/// it is not set.
///
/// Note: Gitea is self-hosted only, so a base URL is always required.
/// Use [`GiteaRemote::with_base_url`] for custom instances or
/// [`GiteaRemote::codeberg`] for Codeberg.org.
///
/// ## Authentication
///
/// Gitea uses `Authorization: token <pat>` format. The `GITEA_TOKEN`
/// environment variable should contain the full token value.
///
/// For Codeberg, a non-empty `CODEBERG_TOKEN` takes precedence over
/// `GITEA_TOKEN`.
///
/// ## Examples
///
/// ```ignore
/// use sniff::remote::{GiteaRemote, RemoteRepoProvider};
///
/// // Connect to a self-hosted Gitea instance
/// let provider = GiteaRemote::with_base_url("https://gitea.example.com/api/v1")?;
/// let metadata = provider.get_repo_metadata("owner", "repo").await?;
///
/// // Connect to Codeberg
/// let codeberg = GiteaRemote::codeberg()?;
/// let metadata = codeberg.get_repo_metadata("forgejo", "forgejo").await?;
/// ```
pub struct GiteaRemote {
    client: Gitea,
    base_url: String,
}

impl GiteaRemote {
    /// Create for a Gitea instance with a custom base URL.
    ///
    /// The base URL should include the API path (e.g., `https://gitea.example.com/api/v1`).
    /// If you provide a URL without `/api/v1`, it will be appended automatically.
    ///
    /// Uses `GITEA_TOKEN` from environment for authentication.
    ///
    /// ## Arguments
    ///
    /// * `base_url` - Base URL for the Gitea instance API
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized,
    /// including when `base_url` is not an HTTP(S) URL with a host.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let provider = GiteaRemote::with_base_url("https://gitea.example.com/api/v1")?;
    /// // Or without the /api/v1 suffix (it will be appended):
    /// let provider = GiteaRemote::with_base_url("https://gitea.example.com")?;
    /// ```
    pub fn with_base_url(base_url: &str) -> Result<Self, SniffError> {
        let normalized_url = normalize_base_url(base_url);
        Self::build(Gitea::new().base_url(&normalized_url), normalized_url)
    }

    /// Create for [Codeberg](https://codeberg.org), a popular public Gitea instance.
    ///
    /// Uses `CODEBERG_TOKEN` or `GITEA_TOKEN` from environment for authentication
    /// (in that order of precedence).
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let codeberg = GiteaRemote::codeberg()?;
    /// let metadata = codeberg.get_repo_metadata("forgejo", "forgejo").await?;
    /// ```
    pub fn codeberg() -> Result<Self, SniffError> {
        Self::codeberg_at(CODEBERG_API_URL)
    }

    /// Codeberg's credential order against `api_url`, so a test can point it
    /// at a local server.
    fn codeberg_at(api_url: &str) -> Result<Self, SniffError> {
        let builder = Gitea::new()
            .base_url(api_url)
            .env_order(["CODEBERG_TOKEN", "GITEA_TOKEN"]);
        Self::build(builder, api_url.to_string())
    }

    fn build(builder: Gitea, base_url: String) -> Result<Self, SniffError> {
        let client = builder.build().map_err(|error| REPLIES.init(&error))?;
        Ok(Self { client, base_url })
    }

    /// Returns the base URL for this Gitea instance.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Extracts the web URL from the API base URL.
    ///
    /// For example, `https://gitea.example.com/api/v1` becomes `https://gitea.example.com`.
    fn web_url(&self) -> String {
        self.base_url
            .trim_end_matches("/api/v1")
            .trim_end_matches("/api/v1/")
            .to_string()
    }

    /// Fetches the recursive root tree once, continuing if Gitea truncated it.
    ///
    /// See `GitHubRemote::fetch_tree` — Gitea mirrors the GitHub tree API,
    /// including the `truncated` flag that was previously ignored.
    async fn fetch_tree(
        &self,
        owner: &str,
        repo: &str,
        default_branch: &str,
    ) -> Result<RemoteTree, SniffError> {
        count_api_request("tree");
        let response = REPLIES.value(
            self.client
                .get_git_tree(owner, repo, default_branch).recursive(true)
                .await,
        )?;

        let mut tree = RemoteTree::observed(blobs_of(response.tree), response.truncated);
        if tree.truncated {
            let mut recovered = Vec::new();
            for prefix in CONTINUATION_PREFIXES {
                let call = self.client.get_git_tree(
                    owner,
                    repo,
                    format!("{default_branch}:{prefix}"),
                ).recursive(true);
                count_api_request("tree_continuation");
                let Ok(response) = call.await else {
                    continue;
                };
                recovered.extend(
                    blobs_of(response.tree)
                        .into_iter()
                        .map(|file| RemoteTreeFile {
                            path: format!("{prefix}/{}", file.path),
                            size: file.size,
                        }),
                );
            }
            tree.extend_from_continuation(recovered);
        }
        Ok(tree)
    }
}

/// Keeps blob entries and drops directories, normalizing to [`RemoteTreeFile`].
///
/// Gitea models both `entry_type` and `path` as optional, so an entry missing
/// either carries no usable evidence and is dropped.
fn blobs_of(entries: Vec<GitTreeEntry>) -> Vec<RemoteTreeFile> {
    entries
        .into_iter()
        .filter(|entry| entry.entry_type.as_deref() == Some("blob"))
        .filter_map(|entry| {
            Some(RemoteTreeFile {
                path: entry.path?,
                size: entry.size.map(|s| s as u64),
            })
        })
        .collect()
}

/// Normalize a base URL to ensure it ends with `/api/v1`.
fn normalize_base_url(url: &str) -> String {
    let url = url.trim_end_matches('/');

    if url.ends_with("/api/v1") {
        url.to_string()
    } else if url.ends_with("/api") {
        format!("{}/v1", url)
    } else {
        format!("{}/api/v1", url)
    }
}

#[async_trait]
impl RemoteRepoProvider for GiteaRemote {
    fn provider(&self) -> GitProvider {
        GitProvider::Gitea
    }

    async fn get_repo_metadata(&self, owner: &str, repo: &str) -> Result<RepoMetadata, SniffError> {
        count_api_request("metadata");
        let info = REPLIES.value(self.client.get_repository(owner, repo).await)?;

        // Gitea types are heavily optional - extract with defaults
        let name = info.name.unwrap_or_else(|| repo.to_string());
        let full_name = info
            .full_name
            .unwrap_or_else(|| format!("{}/{}", owner, repo));
        let default_branch = info.default_branch.unwrap_or_else(|| "main".to_string());
        let html_url = info
            .html_url
            .unwrap_or_else(|| format!("{}/{}/{}", self.web_url(), owner, repo));

        Ok(RepoMetadata {
            name,
            full_name,
            description: info.description,
            private: info.private,
            default_branch,
            language: info.language,
            stars: info.stars_count.map(|v| v as u64),
            forks: info.forks_count.map(|v| v as u64),
            open_issues: info.open_issues_count.map(|v| v as u64),
            archived: info.archived,
            created_at: info.created_at,
            updated_at: info.updated_at,
            pushed_at: None,    // Gitea doesn't have pushed_at in RepositoryInfo
            license: None,      // Gitea doesn't have license in RepositoryInfo
            topics: Vec::new(), // Gitea doesn't have topics in RepositoryInfo
            has_issues: None,   // Not in Gitea RepositoryInfo
            has_wiki: None,     // Not in Gitea RepositoryInfo
            homepage: None,
            html_url,
        })
    }

    async fn get_org_info(&self, _org: &str) -> Result<OrgInfo, SniffError> {
        // Gitea GetOrganization endpoint is not implemented in schematic-definitions
        // Return a placeholder error for now
        Err(SniffError::RemoteApi {
            provider: "Gitea".to_string(),
            status: 501,
            message: "Organization info endpoint not implemented".to_string(),
        })
    }

    async fn snapshot(&self, owner: &str, repo: &str) -> Result<RemoteRepoSnapshot, SniffError> {
        let metadata = self.get_repo_metadata(owner, repo).await?;
        // A tree failure degrades the projections that read it; it must not sink a
        // report whose required metadata already succeeded (R11.6).
        let tree = self
            .fetch_tree(owner, repo, &metadata.default_branch)
            .await
            .unwrap_or_else(|_| RemoteTree::unavailable());

        Ok(RemoteRepoSnapshot {
            owner: owner.to_string(),
            repo: repo.to_string(),
            metadata,
            tree,
        })
    }

    async fn list_documents(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<DocumentRef>, SniffError> {
        let snapshot = self.snapshot(owner, repo).await?;
        self.list_documents_with(&snapshot).await
    }

    async fn list_documents_with(
        &self,
        snapshot: &RemoteRepoSnapshot,
    ) -> Result<Vec<DocumentRef>, SniffError> {
        Ok(documents_from_tree(&snapshot.tree))
    }

    async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> Result<String, SniffError> {
        count_api_request("contents");
        REPLIES.value(
            self.client
                .get_repository_content_raw(owner, repo, path)
                .await,
        )
    }

    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: PullRequestState,
    ) -> Result<Vec<PullRequestInfo>, SniffError> {
        // Map PullRequestState to Gitea API state parameter
        let api_state = match state {
            PullRequestState::Open => "open",
            PullRequestState::Closed => "closed",
            PullRequestState::Merged | PullRequestState::Draft | PullRequestState::All => "all",
        };

        count_api_request("pulls");
        let prs = REPLIES.value(
            self.client
                .list_pull_requests(owner, repo)
                .state(api_state.to_string())
                .await,
        )?;

        let mut prs: Vec<PullRequestInfo> = prs
            .into_iter()
            .filter_map(|pr| {
                // Skip PRs without essential fields
                let number = pr.number? as u64;
                let title = pr.title?;
                let state = pr.state.unwrap_or_else(|| "unknown".to_string());
                let author = pr
                    .user
                    .and_then(|u| u.login_name().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown".to_string());
                let created_at = pr.created_at.unwrap_or_default();
                let html_url = pr.html_url.unwrap_or_default();
                let source_head_sha = pr
                    .head
                    .as_ref()
                    .and_then(|head| head.sha.clone())
                    .filter(|sha| !sha.is_empty());

                Some(PullRequestInfo {
                    number,
                    title,
                    state,
                    author,
                    draft: pr.draft.unwrap_or(false),
                    source_branch: pr.head.and_then(|h| h.ref_name),
                    target_branch: pr.base.and_then(|b| b.ref_name),
                    labels: pr
                        .labels
                        .unwrap_or_default()
                        .into_iter()
                        .filter_map(|l| l.name)
                        .collect(),
                    body: pr.body,
                    created_at,
                    updated_at: pr.updated_at,
                    merged_at: pr.merged_at,
                    html_url,
                    // The schematic `PullRef` does not deserialize `head.repo`,
                    // so this path cannot name the source repository.
                    source_repo: None,
                    source_repo_is_target: None,
                    source_head_sha,
                })
            })
            .collect();

        // Post-filter for states Gitea API doesn't support directly
        match state {
            PullRequestState::Merged => {
                prs.retain(|pr| pr.merged_at.is_some());
            }
            PullRequestState::Draft => {
                prs.retain(|pr| pr.draft);
            }
            _ => {}
        }

        Ok(prs)
    }

    async fn list_issues(&self, owner: &str, repo: &str) -> Result<Vec<IssueInfo>, SniffError> {
        count_api_request("issues");
        let issues = REPLIES.value(self.client.list_issues(owner, repo).await)?;

        // Gitea's ListIssues with type=issues already filters out PRs
        Ok(issues
            .into_iter()
            .filter_map(|issue| {
                // Skip issues without essential fields
                let number = issue.issue_number()? as u64;
                let title = issue.title?;
                let state = issue.state.unwrap_or_else(|| "unknown".to_string());
                let author = issue
                    .user
                    .and_then(|u| u.login_name().map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown".to_string());
                let created_at = issue.created_at.unwrap_or_default();
                let html_url = issue.html_url.unwrap_or_default();

                Some(IssueInfo {
                    number,
                    title,
                    state,
                    author,
                    comment_count: issue.comments.map(|c| c as u64),
                    labels: Vec::new(), // Gitea IssueSummary doesn't have labels field in our types
                    created_at,
                    updated_at: issue.updated_at,
                    closed_at: issue.closed_at,
                    html_url,
                })
            })
            .collect())
    }

    async fn get_tags_and_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<TagsAndReleases, SniffError> {
        // Fetch tags
        count_api_request("tags");
        let tags = REPLIES.value(self.client.list_tags(owner, repo).await)?;

        // Fetch releases
        count_api_request("releases");
        let releases = REPLIES.value(self.client.list_releases(owner, repo).await)?;

        // Convert tags - we can determine if annotated from the message field
        let tag_infos: Vec<TagInfo> = tags
            .into_iter()
            .filter_map(|tag| {
                let name = tag.name?;
                let commit_sha = tag.commit.and_then(|c| c.sha).unwrap_or_default();
                let has_message = tag.message.is_some();

                Some(TagInfo {
                    name,
                    commit_sha,
                    annotated: has_message,
                    message: tag.message,
                    tagger: None,
                    tagged_at: None,
                })
            })
            .collect();

        // Convert releases
        let release_infos: Vec<ReleaseInfo> = releases
            .into_iter()
            .filter_map(|release| {
                let tag_name = release.tag_name?;

                Some(ReleaseInfo {
                    name: release.name,
                    tag_name,
                    draft: release.draft.unwrap_or(false),
                    prerelease: release.prerelease.unwrap_or(false),
                    published_at: release.published_at,
                    html_url: release.html_url,
                })
            })
            .collect();

        Ok(TagsAndReleases {
            tags: tag_infos,
            releases: release_infos,
        })
    }

    async fn detect_cicd(&self, owner: &str, repo: &str) -> Result<Option<CiCdInfo>, SniffError> {
        let snapshot = self.snapshot(owner, repo).await?;
        self.detect_cicd_with(&snapshot).await
    }

    async fn detect_cicd_with(
        &self,
        snapshot: &RemoteRepoSnapshot,
    ) -> Result<Option<CiCdInfo>, SniffError> {
        let (owner, repo) = (snapshot.owner.as_str(), snapshot.repo.as_str());
        let tree = &snapshot.tree;
        let web_base = self.web_url();

        // Check for Gitea Actions (native CI)
        if tree.has_path_under(".gitea/workflows") {
            return Ok(Some(CiCdInfo {
                provider: "Gitea Actions".to_string(),
                config_path: Some(".gitea/workflows".to_string()),
                name: "Gitea Actions".to_string(),
                status: "detected".to_string(),
                conclusion: None,
                html_url: Some(format!("{}/{}/{}/actions", web_base, owner, repo)),
                started_at: None,
                head_branch: None,
                event: None,
            }));
        }

        // Check for Drone CI (common with Gitea)
        if tree.contains(".drone.yml") {
            return Ok(Some(CiCdInfo {
                provider: "Drone CI".to_string(),
                config_path: Some(".drone.yml".to_string()),
                name: "Drone CI".to_string(),
                status: "detected".to_string(),
                conclusion: None,
                html_url: None, // Drone runs on separate infrastructure
                started_at: None,
                head_branch: None,
                event: None,
            }));
        }

        // Check for Woodpecker CI (Drone fork, common with Gitea/Forgejo)
        if tree.contains(".woodpecker.yml") {
            return Ok(Some(CiCdInfo {
                provider: "Woodpecker CI".to_string(),
                config_path: Some(".woodpecker.yml".to_string()),
                name: "Woodpecker CI".to_string(),
                status: "detected".to_string(),
                conclusion: None,
                html_url: None, // Woodpecker runs on separate infrastructure
                started_at: None,
                head_branch: None,
                event: None,
            }));
        }

        // Check for Woodpecker CI directory format
        if tree.has_path_under(".woodpecker") {
            return Ok(Some(CiCdInfo {
                provider: "Woodpecker CI".to_string(),
                config_path: Some(".woodpecker/".to_string()),
                name: "Woodpecker CI".to_string(),
                status: "detected".to_string(),
                conclusion: None,
                html_url: None,
                started_at: None,
                head_branch: None,
                event: None,
            }));
        }

        Ok(None)
    }

    async fn list_org_repos(&self, _org: &str) -> Result<Vec<OrgRepoRef>, SniffError> {
        // The generated client has `list_org_repos`, but this provider does not call
        // it yet, so organization listings are always empty.
        Ok(Vec::new())
    }

    fn build_key_urls(&self, owner: &str, repo: &str) -> KeyUrls {
        let web_base = self.web_url();
        let base = format!("{}/{}/{}", web_base, owner, repo);

        KeyUrls {
            repo: base.clone(),
            homepage: None, // Would come from repo metadata
            docs: None,
            issues: Some(format!("{}/issues", base)),
            pull_requests: Some(format!("{}/pulls", base)),
            wiki: Some(format!("{}/wiki", base)),
            ci_cd: Some(format!("{}/actions", base)),
            insights: None, // Gitea doesn't have insights page
            releases: Some(format!("{}/releases", base)),
            settings: Some(format!("{}/settings", base)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_base_url_already_complete() {
        assert_eq!(
            normalize_base_url("https://gitea.example.com/api/v1"),
            "https://gitea.example.com/api/v1"
        );
    }

    #[test]
    fn test_normalize_base_url_without_api_path() {
        assert_eq!(
            normalize_base_url("https://gitea.example.com"),
            "https://gitea.example.com/api/v1"
        );
    }

    #[test]
    fn test_normalize_base_url_trailing_slash() {
        assert_eq!(
            normalize_base_url("https://gitea.example.com/"),
            "https://gitea.example.com/api/v1"
        );
    }

    #[test]
    fn test_normalize_base_url_api_only() {
        assert_eq!(
            normalize_base_url("https://gitea.example.com/api"),
            "https://gitea.example.com/api/v1"
        );
    }

    #[test]
    fn test_web_url_extraction() {
        let provider = GiteaRemote::with_base_url("https://gitea.example.com/api/v1").unwrap();
        assert_eq!(provider.web_url(), "https://gitea.example.com");
    }

    #[test]
    fn test_build_key_urls() {
        let provider = GiteaRemote::with_base_url("https://gitea.example.com/api/v1").unwrap();
        let urls = provider.build_key_urls("owner", "repo");

        assert_eq!(urls.repo, "https://gitea.example.com/owner/repo");
        assert_eq!(
            urls.issues,
            Some("https://gitea.example.com/owner/repo/issues".to_string())
        );
        assert_eq!(
            urls.pull_requests,
            Some("https://gitea.example.com/owner/repo/pulls".to_string())
        );
        assert_eq!(
            urls.ci_cd,
            Some("https://gitea.example.com/owner/repo/actions".to_string())
        );
        assert_eq!(
            urls.releases,
            Some("https://gitea.example.com/owner/repo/releases".to_string())
        );
        // Gitea doesn't have insights
        assert!(urls.insights.is_none());
    }

    #[test]
    fn test_build_key_urls_codeberg() {
        let provider = GiteaRemote::codeberg().unwrap();
        let urls = provider.build_key_urls("forgejo", "forgejo");

        assert_eq!(urls.repo, "https://codeberg.org/forgejo/forgejo");
        assert_eq!(
            urls.issues,
            Some("https://codeberg.org/forgejo/forgejo/issues".to_string())
        );
    }

    /// Codeberg reads `CODEBERG_TOKEN` before `GITEA_TOKEN`; an empty variable
    /// counts as unset. Each case asserts the header the server received.
    mod codeberg_credentials {
        use serial_test::serial;
        use test_toolkit::EnvGuard;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        use super::*;

        async fn sent_authorization(codeberg: Option<&str>, gitea: Option<&str>) -> String {
            let _guards: Vec<EnvGuard> = [("CODEBERG_TOKEN", codeberg), ("GITEA_TOKEN", gitea)]
                .into_iter()
                .map(|(name, value)| match value {
                    Some(value) => EnvGuard::set_safe(name, value),
                    None => EnvGuard::remove_safe(name),
                })
                .collect();
            let server = MockServer::start().await;
            Mock::given(method("GET"))
                .and(path("/repos/owner/repo"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "name": "repo",
                    "default_branch": "main"
                })))
                .expect(1)
                .mount(&server)
                .await;
            let provider = GiteaRemote::codeberg_at(&server.uri()).unwrap();

            provider.get_repo_metadata("owner", "repo").await.unwrap();

            let requests = server.received_requests().await.unwrap();
            requests[0]
                .headers
                .get("authorization")
                .map(|value| value.to_str().unwrap().to_string())
                .unwrap_or_default()
        }

        #[tokio::test]
        #[serial]
        async fn codeberg_token_wins_over_gitea_token() {
            let sent = sent_authorization(Some("codeberg-secret"), Some("gitea-secret")).await;

            assert_eq!(sent, "token codeberg-secret");
        }

        #[tokio::test]
        #[serial]
        async fn gitea_token_is_used_when_codeberg_token_is_absent() {
            let sent = sent_authorization(None, Some("gitea-secret")).await;

            assert_eq!(sent, "token gitea-secret");
        }

        #[tokio::test]
        #[serial]
        async fn gitea_token_is_used_when_codeberg_token_is_empty() {
            let sent = sent_authorization(Some(""), Some("gitea-secret")).await;

            assert_eq!(sent, "token gitea-secret");
        }

        #[tokio::test]
        #[serial]
        async fn no_token_sends_no_authorization() {
            let sent = sent_authorization(None, None).await;

            assert_eq!(sent, "");
        }

        #[test]
        fn codeberg_targets_the_codeberg_api() {
            assert_eq!(
                GiteaRemote::codeberg().unwrap().base_url(),
                "https://codeberg.org/api/v1"
            );
        }
    }
}
