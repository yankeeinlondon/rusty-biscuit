//! GitLab remote repository provider.
//!
//! This module implements [`RemoteRepoProvider`] for GitLab using the
//! generated schematic-schema client.
//!
//! ## Limitations
//!
//! The generated client has a `get_project` endpoint, but this provider does not
//! call it yet. Repository metadata is a stub instead:
//! - Existence is verified via `ListRepositoryTree` (404 if not found)
//! - Every field a project lookup would supply is `None` or a default

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use schematic_schema::gitlab::*;

use super::{
    count_api_request,
    envelope::{Replies, Unauthorized},
    provider::RemoteRepoProvider,
    snapshot::{RemoteRepoSnapshot, RemoteTree, RemoteTreeFile, documents_from_tree},
    types::{
        CiCdInfo, DocumentRef, GitProvider, IssueInfo, KeyUrls, OrgInfo, OrgRepoRef,
        PullRequestInfo, PullRequestState, ReleaseInfo, RepoMetadata, TagInfo, TagsAndReleases,
    },
};
use crate::error::SniffError;

const REPLIES: Replies = Replies::new(
    "GitLab",
    Unauthorized::Missing("GITLAB_TOKEN or GITLAB_PRIVATE_TOKEN"),
);

/// GitLab remote repository provider.
///
/// Uses the schematic-generated GitLab API client for all HTTP operations.
/// Each request sends the first non-empty `GITLAB_TOKEN` or
/// `GITLAB_PRIVATE_TOKEN` in the `PRIVATE-TOKEN` header, and is sent
/// anonymously when neither is set.
///
/// ## Project ID Encoding
///
/// GitLab requires project paths to be URL-encoded. For example, `owner/repo`
/// becomes `owner%2Frepo`. Nested groups like `group/subgroup/repo` become
/// `group%2Fsubgroup%2Frepo`. The generated client encodes each path
/// parameter once, so callers pass the plain `owner/repo` path.
///
/// ## Examples
///
/// ```ignore
/// use sniff::remote::{GitLabRemote, RemoteRepoProvider};
///
/// let provider = GitLabRemote::new()?;
/// let metadata = provider.get_repo_metadata("gitlab-org", "gitlab").await?;
/// println!("Stars: {:?}", metadata.stars);
/// ```
pub struct GitLabRemote {
    client: GitLab,
    base_url: String,
}

impl GitLabRemote {
    /// Create a new GitLab provider for gitlab.com.
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    pub fn new() -> Result<Self, SniffError> {
        Self::build(GitLab::new(), "https://gitlab.com")
    }

    /// Create a GitLab provider for a self-hosted instance.
    ///
    /// ## Arguments
    ///
    /// * `base_url` - Base URL for the GitLab instance (e.g., `https://gitlab.example.com`)
    ///
    /// ## Notes
    ///
    /// The provided `base_url` should be the root URL without `/api/v4`.
    /// The API path is appended automatically.
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized,
    /// including when `base_url` is not an HTTP(S) URL with a host.
    pub fn with_base_url(base_url: &str) -> Result<Self, SniffError> {
        let clean_url = base_url.trim_end_matches('/');
        let api_url = format!("{}/api/v4", clean_url);
        Self::build(GitLab::new().base_url(api_url), clean_url)
    }

    fn build(builder: GitLab, base_url: &str) -> Result<Self, SniffError> {
        let client = builder.build().map_err(|error| REPLIES.init(&error))?;
        Ok(Self {
            client,
            base_url: base_url.to_string(),
        })
    }
}

impl Default for GitLabRemote {
    /// ## Panics
    ///
    /// Panics if the HTTP client cannot be initialized; use
    /// [`GitLabRemote::new`] to handle that as an error.
    fn default() -> Self {
        Self::new().expect("the GitLab client builds from its default definition")
    }
}

impl GitLabRemote {
    /// Fetches the root tree listing.
    ///
    /// Counted as `tree`, not `metadata`: without a `GetProject` call, the
    /// metadata path pays for a full tree fetch, and the counter names the work
    /// done.
    async fn fetch_tree_items(&self, owner: &str, repo: &str) -> Result<Vec<TreeItem>, SniffError> {
        count_api_request("tree");
        REPLIES.value(
            self.client
                .list_repository_tree(project_path(owner, repo))
                .await,
        )
    }

    /// Builds the minimal metadata GitLab can report without `GetProject`.
    ///
    /// Pure — the caller has already proven the project exists by fetching its
    /// tree. Calling the generated `get_project` is what would let these
    /// `None`s become real values.
    fn stub_metadata(&self, owner: &str, repo: &str) -> RepoMetadata {
        let full_name = format!("{}/{}", owner, repo);
        let html_url = format!("{}/{}", self.base_url, full_name);

        RepoMetadata {
            name: repo.to_string(),
            full_name,
            description: None,                  // Requires GetProject
            private: false,                     // Requires GetProject (assume public if accessible)
            default_branch: "main".to_string(), // Requires GetProject; assume "main"
            language: None,                     // Requires GetProject
            stars: None,                        // Requires GetProject
            forks: None,                        // Requires GetProject
            open_issues: None,                  // Requires GetProject
            archived: false,                    // Requires GetProject
            created_at: None,                   // Requires GetProject
            updated_at: None,                   // Requires GetProject
            pushed_at: None,                    // Requires GetProject
            license: None,                      // Requires GetProject
            topics: Vec::new(),                 // Requires GetProject
            has_issues: None,                   // Requires GetProject
            has_wiki: None,                     // Requires GetProject
            homepage: None,                     // Requires GetProject
            html_url,
        }
    }
}

/// Keeps blob entries and drops directories, normalizing to [`RemoteTreeFile`].
fn blobs_of(items: Vec<TreeItem>) -> Vec<RemoteTreeFile> {
    items
        .into_iter()
        .filter(|entry| entry.item_type == "blob")
        .map(|entry| RemoteTreeFile {
            path: entry.path,
            // GitLab's tree listing does not include size.
            size: None,
        })
        .collect()
}

/// The project path GitLab accepts as a project ID.
///
/// It is not encoded here: the generated client percent-encodes every path
/// parameter, so `owner/repo` reaches the wire as `owner%2Frepo`.
fn project_path(owner: &str, repo: &str) -> String {
    format!("{owner}/{repo}")
}

#[async_trait]
impl RemoteRepoProvider for GitLabRemote {
    fn provider(&self) -> GitProvider {
        GitProvider::GitLab
    }

    async fn get_repo_metadata(&self, owner: &str, repo: &str) -> Result<RepoMetadata, SniffError> {
        // `get_project` is not called yet; the tree fetch proves the
        // project exists and the metadata is a stub.
        self.fetch_tree_items(owner, repo).await?;
        Ok(self.stub_metadata(owner, repo))
    }

    /// Resolves the one tree that serves as both existence probe and evidence.
    ///
    /// Without a `GetProject` call, GitLab's "metadata" call *is* a tree fetch whose result
    /// was thrown away — so a report used to pay for the identical tree three
    /// times (metadata, documents, CI/CD). Keeping the result collapses that to one.
    async fn snapshot(&self, owner: &str, repo: &str) -> Result<RemoteRepoSnapshot, SniffError> {
        let items = self.fetch_tree_items(owner, repo).await?;
        Ok(RemoteRepoSnapshot {
            owner: owner.to_string(),
            repo: repo.to_string(),
            metadata: self.stub_metadata(owner, repo),
            // GitLab's listing is flat and carries no truncation signal, so a
            // subtree continuation has nothing to key off. Root-only document and
            // CI/CD detection is the pre-existing contract, not a regression here.
            tree: RemoteTree::observed(blobs_of(items), false),
        })
    }

    async fn get_org_info(&self, _org: &str) -> Result<OrgInfo, SniffError> {
        // GitLab GetGroup endpoint is not implemented in schematic-definitions
        Err(SniffError::RemoteApi {
            provider: "GitLab".to_string(),
            status: 501,
            message: "Group info endpoint not implemented".to_string(),
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
        // "HEAD" is the default branch; GitLab accepts it as a ref.
        count_api_request("contents");
        let file = REPLIES.value(
            self.client
                .get_repository_file(project_path(owner, repo), path, "HEAD")
                .await,
        )?;

        // GitLab returns Base64-encoded content
        let decoded = BASE64
            .decode(&file.content)
            .map_err(|e| SniffError::RemoteApi {
                provider: "GitLab".to_string(),
                status: 0,
                message: format!("Base64 decode error: {}", e),
            })?;

        String::from_utf8(decoded).map_err(|e| SniffError::RemoteApi {
            provider: "GitLab".to_string(),
            status: 0,
            message: format!("UTF-8 decode error: {}", e),
        })
    }

    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: PullRequestState,
    ) -> Result<Vec<PullRequestInfo>, SniffError> {
        // Map PullRequestState to GitLab API state parameter
        let api_state = match state {
            PullRequestState::Open => ListMergeRequestsRequestState::Opened,
            PullRequestState::Closed => ListMergeRequestsRequestState::Closed,
            PullRequestState::Merged => ListMergeRequestsRequestState::Merged,
            PullRequestState::Draft | PullRequestState::All => ListMergeRequestsRequestState::All,
        };

        count_api_request("pulls");
        let mrs = REPLIES.value(
            self.client
                .list_merge_requests(project_path(owner, repo))
                .state(api_state)
                .await,
        )?;

        let mut mrs: Vec<PullRequestInfo> = mrs
            .into_iter()
            .map(|mr| {
                let source_repo_is_target = mr
                    .source_project_id
                    .zip(mr.target_project_id)
                    .map(|(source, target)| source == target);
                PullRequestInfo {
                    number: mr.iid,
                    title: mr.title,
                    state: mr.state,
                    author: mr.author.username,
                    draft: mr.draft || mr.work_in_progress,
                    source_branch: Some(mr.source_branch),
                    target_branch: Some(mr.target_branch),
                    labels: mr.labels,
                    body: mr.description,
                    created_at: mr.created_at,
                    updated_at: Some(mr.updated_at),
                    merged_at: mr.merged_at,
                    html_url: mr.web_url.unwrap_or_default(),
                    // The payload names the source project only by ID, so its path
                    // is known without another request only when it is this project.
                    source_repo: (source_repo_is_target == Some(true))
                        .then(|| format!("{owner}/{repo}")),
                    source_repo_is_target,
                    source_head_sha: mr.sha.filter(|sha| !sha.is_empty()),
                }
            })
            .collect();

        // Post-filter for Draft since GitLab API doesn't have a direct draft filter
        if state == PullRequestState::Draft {
            mrs.retain(|mr| mr.draft);
        }

        Ok(mrs)
    }

    async fn list_issues(&self, owner: &str, repo: &str) -> Result<Vec<IssueInfo>, SniffError> {
        count_api_request("issues");
        let issues = REPLIES.value(self.client.list_issues(project_path(owner, repo)).await)?;

        // GitLab issues API doesn't include merge requests (unlike GitHub)
        Ok(issues
            .into_iter()
            .map(|issue| IssueInfo {
                number: issue.iid,
                title: issue.title,
                state: issue.state,
                author: issue.author.username,
                comment_count: Some(issue.user_notes_count),
                labels: issue.labels,
                created_at: issue.created_at,
                updated_at: Some(issue.updated_at),
                closed_at: issue.closed_at,
                html_url: issue.web_url.unwrap_or_default(),
            })
            .collect())
    }

    async fn get_tags_and_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<TagsAndReleases, SniffError> {
        let project_id = project_path(owner, repo);

        // Fetch tags
        count_api_request("tags");
        let tags = REPLIES.value(self.client.list_tags(&project_id).await)?;

        // Fetch releases
        count_api_request("releases");
        let releases = REPLIES.value(self.client.list_releases(&project_id).await)?;

        // Convert tags - GitLab provides message for annotated tags
        let tag_infos: Vec<TagInfo> = tags
            .into_iter()
            .map(|tag| TagInfo {
                name: tag.name,
                commit_sha: tag.target,
                annotated: tag.message.is_some(),
                message: tag.message,
                tagger: tag.commit.author_name,
                tagged_at: tag.created_at.or(tag.commit.created_at),
            })
            .collect();

        // Convert releases - GitLab releases don't have draft/prerelease flags
        let release_infos: Vec<ReleaseInfo> = releases
            .into_iter()
            .map(|release| {
                // Build release URL from base_url and project path
                let html_url = release.links.map(|l| l.self_url);

                ReleaseInfo {
                    name: Some(release.name),
                    tag_name: release.tag_name,
                    draft: false,      // GitLab releases are always published
                    prerelease: false, // GitLab doesn't have prerelease concept
                    published_at: Some(release.released_at),
                    html_url,
                }
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
        if !snapshot.tree.contains(".gitlab-ci.yml") {
            return Ok(None);
        }

        Ok(Some(CiCdInfo {
            provider: "GitLab CI".to_string(),
            config_path: Some(".gitlab-ci.yml".to_string()),
            name: "GitLab CI".to_string(),
            status: "detected".to_string(),
            conclusion: None,
            html_url: Some(format!(
                "{}/{}/{}/-/pipelines",
                self.base_url, snapshot.owner, snapshot.repo
            )),
            started_at: None,
            head_branch: None,
            event: None,
        }))
    }

    async fn list_org_repos(&self, _org: &str) -> Result<Vec<OrgRepoRef>, SniffError> {
        // The generated client has `list_group_projects`, but this provider
        // does not call it yet, so group listings are always empty.
        Ok(Vec::new())
    }

    fn build_key_urls(&self, owner: &str, repo: &str) -> KeyUrls {
        let base = format!("{}/{}/{}", self.base_url, owner, repo);

        KeyUrls {
            repo: base.clone(),
            homepage: None, // Would come from project metadata
            docs: None,
            issues: Some(format!("{}/-/issues", base)),
            pull_requests: Some(format!("{}/-/merge_requests", base)),
            wiki: Some(format!("{}/-/wikis/home", base)),
            ci_cd: Some(format!("{}/-/pipelines", base)),
            insights: Some(format!("{}/-/graphs/main", base)),
            releases: Some(format!("{}/-/releases", base)),
            settings: Some(format!("{}/-/settings/repository", base)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_key_urls() {
        let provider = GitLabRemote::default();
        let urls = provider.build_key_urls("gitlab-org", "gitlab");

        assert_eq!(urls.repo, "https://gitlab.com/gitlab-org/gitlab");
        assert_eq!(
            urls.issues,
            Some("https://gitlab.com/gitlab-org/gitlab/-/issues".to_string())
        );
        assert_eq!(
            urls.pull_requests,
            Some("https://gitlab.com/gitlab-org/gitlab/-/merge_requests".to_string())
        );
        assert_eq!(
            urls.ci_cd,
            Some("https://gitlab.com/gitlab-org/gitlab/-/pipelines".to_string())
        );
    }

    #[test]
    fn test_build_key_urls_self_hosted() {
        let provider = GitLabRemote::with_base_url("https://gitlab.example.com").unwrap();
        let urls = provider.build_key_urls("team", "project");

        assert_eq!(urls.repo, "https://gitlab.example.com/team/project");
        assert_eq!(
            urls.issues,
            Some("https://gitlab.example.com/team/project/-/issues".to_string())
        );
    }
}
