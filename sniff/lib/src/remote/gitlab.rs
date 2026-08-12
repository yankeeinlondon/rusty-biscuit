//! GitLab remote repository provider.
//!
//! This module implements [`RemoteRepoProvider`] for GitLab using the
//! generated schematic-schema client.
//!
//! ## Limitations
//!
//! GitLab's schematic client does not include a `GetProject` endpoint.
//! Repository metadata is inferred from other API responses:
//! - Existence verified via `ListRepositoryTree` (returns 404 if not found)
//! - Full metadata requires the `GetProject` endpoint (future iteration)

use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD as BASE64};
use schematic_schema::gitlab::*;
use schematic_schema::shared::{AuthStrategy, SchematicError, UpdateStrategy};

use super::{
    count_api_request,
    provider::RemoteRepoProvider,
    snapshot::{documents_from_tree, RemoteRepoSnapshot, RemoteTree, RemoteTreeFile},
    types::{
        CiCdInfo, DocumentRef, GitProvider, IssueInfo, KeyUrls, OrgInfo,
        OrgRepoRef, PullRequestInfo, PullRequestState, ReleaseInfo, RepoMetadata, TagInfo,
        TagsAndReleases,
    },
};
use crate::error::SniffError;

/// GitLab remote repository provider.
///
/// Uses the schematic-generated GitLab API client for all HTTP operations.
/// Authentication is via the `GITLAB_TOKEN` or `GITLAB_PRIVATE_TOKEN`
/// environment variable.
///
/// ## Project ID Encoding
///
/// GitLab requires project paths to be URL-encoded. For example, `owner/repo`
/// becomes `owner%2Frepo`. Nested groups like `group/subgroup/repo` become
/// `group%2Fsubgroup%2Frepo`. This encoding is handled automatically.
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
    /// Create a new GitLab provider for gitlab.com using GITLAB_TOKEN from environment.
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    pub fn new() -> Result<Self, SniffError> {
        Ok(Self {
            client: GitLab::new(),
            base_url: "https://gitlab.com".to_string(),
        })
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
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    pub fn with_base_url(base_url: &str) -> Result<Self, SniffError> {
        // Strip trailing slash if present
        let clean_url = base_url.trim_end_matches('/');
        let api_url = format!("{}/api/v4", clean_url);

        Ok(Self {
            client: GitLab::with_base_url(&api_url),
            base_url: clean_url.to_string(),
        })
    }
}

impl Default for GitLabRemote {
    fn default() -> Self {
        Self {
            client: GitLab::new(),
            base_url: "https://gitlab.com".to_string(),
        }
    }
}

impl GitLabRemote {
    /// Build a GitLab API client variant that performs no authentication.
    ///
    /// The client is constructed by overriding both the env-fallback list
    /// (so no credentials can be picked up from `GITLAB_TOKEN`,
    /// `GITLAB_PRIVATE_TOKEN`, etc.) **and** the auth strategy itself (so
    /// the schematic runtime does not pre-flight-reject the request for
    /// missing credentials). The combination produces a truly anonymous
    /// client whose request passes through to the wire and is judged by
    /// the API alone.
    ///
    /// This supports the "attempt unauthenticated" fallback: when an
    /// authenticated request fails because no credentials are configured,
    /// we retry once with this anonymous client before surfacing a
    /// `MissingCredentials` error to the user.
    fn unauthenticated_client(&self) -> GitLab {
        self.client
            .variant()
            .env_auth(Vec::new())
            .auth_update(UpdateStrategy::ChangeTo(AuthStrategy::None))
            .build()
    }

    /// Fetches the root tree listing.
    ///
    /// Counted as `tree`, not `metadata`: absent `GetProject`, the metadata path
    /// pays for a full tree fetch, and the counter names the work done.
    async fn fetch_tree_items(&self, owner: &str, repo: &str) -> Result<Vec<TreeItem>, SniffError> {
        let project_id = encode_project_id(owner, repo);
        let request = ListRepositoryTreeRequest::new(&project_id);
        count_api_request("tree");
        self.client
            .request(request)
            .await
            .map_err(map_schematic_error)
    }

    /// Builds the minimal metadata GitLab can report without `GetProject`.
    ///
    /// Pure — the caller has already proven the project exists by fetching its
    /// tree. Adding `GetProject` to schematic-definitions is what would let these
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

/// Encode a project path for GitLab API.
///
/// GitLab requires project paths to be URL-encoded in API requests.
/// For example: `owner/repo` -> `owner%2Frepo`
fn encode_project_id(owner: &str, repo: &str) -> String {
    let path = format!("{}/{}", owner, repo);
    urlencoding::encode(&path).to_string()
}

/// Map a [`SchematicError`] to a [`SniffError`].
///
/// Handles special cases:
/// - `MissingCredential` / `AuthenticationRequired` -> `MissingCredentials`
/// - 401 -> `MissingCredentials` (anonymous request rejected)
/// - 403 with rate limit info -> `RateLimited`
/// - 404 -> `RemoteApi` with "Not found"
/// - Other HTTP errors -> `RemoteApi`
///
/// ## Notes
///
/// `MissingCredentials` is only emitted *after* the anonymous retry has
/// failed (see [`GitLabRemote::unauthenticated_client`]) or when the API
/// explicitly demands credentials for a private resource (a real 401
/// response). Call sites that want the "attempt unauthenticated"
/// behaviour must wrap the initial request in a
/// `MissingCredential`/`AuthenticationRequired` retry rather than mapping
/// the first failure directly through this function.
fn map_schematic_error(err: SchematicError) -> SniffError {
    match err {
        SchematicError::MissingCredential { env_vars } => SniffError::MissingCredentials {
            provider: "GitLab".to_string(),
            env_var: env_vars.join(" or "),
        },
        SchematicError::AuthenticationRequired {
            env_fallback_vars, ..
        } => SniffError::MissingCredentials {
            provider: "GitLab".to_string(),
            env_var: env_fallback_vars.join(" or "),
        },
        SchematicError::ApiError { status: 401, body } => SniffError::MissingCredentials {
            provider: "GitLab".to_string(),
            env_var: format!(
                "GITLAB_TOKEN or GITLAB_PRIVATE_TOKEN (API returned 401: {})",
                body
            ),
        },
        SchematicError::ApiError { status: 403, body } => {
            // Check if this is a rate limit error
            if body.to_lowercase().contains("rate limit") {
                SniffError::RateLimited {
                    provider: "GitLab".to_string(),
                    retry_after: None,
                }
            } else {
                SniffError::RemoteApi {
                    provider: "GitLab".to_string(),
                    status: 403,
                    message: body,
                }
            }
        }
        SchematicError::ApiError { status: 429, .. } => SniffError::RateLimited {
            provider: "GitLab".to_string(),
            retry_after: None,
        },
        SchematicError::ApiError { status: 404, .. } => SniffError::RemoteApi {
            provider: "GitLab".to_string(),
            status: 404,
            message: "Not found".to_string(),
        },
        SchematicError::ApiError { status, body } => SniffError::RemoteApi {
            provider: "GitLab".to_string(),
            status,
            message: body,
        },
        SchematicError::Http(e) => SniffError::RemoteApi {
            provider: "GitLab".to_string(),
            status: 0,
            message: e.to_string(),
        },
        SchematicError::Json(e) => SniffError::RemoteApi {
            provider: "GitLab".to_string(),
            status: 0,
            message: format!("JSON parse error: {}", e),
        },
        other => SniffError::RemoteApi {
            provider: "GitLab".to_string(),
            status: 0,
            message: other.to_string(),
        },
    }
}


#[async_trait]
impl RemoteRepoProvider for GitLabRemote {
    fn provider(&self) -> GitProvider {
        GitProvider::GitLab
    }

    async fn get_repo_metadata(&self, owner: &str, repo: &str) -> Result<RepoMetadata, SniffError> {
        // GitLab's schematic client does not include GetProject.
        // Use ListRepositoryTree to verify project existence and extract basic info.
        // This is a mitigation documented in phase0-audit.md.
        self.fetch_tree_items(owner, repo).await?;
        Ok(self.stub_metadata(owner, repo))
    }

    /// Resolves the one tree that serves as both existence probe and evidence.
    ///
    /// Absent `GetProject`, GitLab's "metadata" call *is* a tree fetch whose result
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
        let project_id = encode_project_id(owner, repo);
        // URL-encode the file path for nested paths
        let encoded_path = urlencoding::encode(path);
        // Use "HEAD" as default ref (GitLab API accepts this for default branch)
        let request = GetRepositoryFileRequest::new(&project_id, &*encoded_path, "HEAD");
        count_api_request("contents");
        let file: FileContent = self
            .client
            .request(request)
            .await
            .map_err(map_schematic_error)?;

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
        let project_id = encode_project_id(owner, repo);
        let mut request = ListMergeRequestsRequest::new(&project_id);

        // Map PullRequestState to GitLab API state parameter
        match state {
            PullRequestState::Open => {
                request = request.with_state("opened".to_string());
            }
            PullRequestState::Closed => {
                request = request.with_state("closed".to_string());
            }
            PullRequestState::Merged => {
                request = request.with_state("merged".to_string());
            }
            PullRequestState::Draft | PullRequestState::All => {
                request = request.with_state("all".to_string());
            }
        }

        // Attempt the request with the configured client first. If it fails
        // because no credentials are available, retry once with an explicitly
        // unauthenticated client. Only after the anonymous retry also fails
        // (or on a real 401/403) do we surface a credentials error.
        count_api_request("pulls");
        let mrs: Vec<MergeRequest> = match self.client.request(request.clone()).await {
            Ok(mrs) => mrs,
            Err(SchematicError::MissingCredential { .. })
            | Err(SchematicError::AuthenticationRequired { .. }) => {
                count_api_request("pulls");
                self.unauthenticated_client()
                    .request(request)
                    .await
                    .map_err(map_schematic_error)?
            }
            Err(err) => return Err(map_schematic_error(err)),
        };

        let mut mrs: Vec<PullRequestInfo> = mrs
            .into_iter()
            .map(|mr| PullRequestInfo {
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
            })
            .collect();

        // Post-filter for Draft since GitLab API doesn't have a direct draft filter
        if state == PullRequestState::Draft {
            mrs.retain(|mr| mr.draft);
        }

        Ok(mrs)
    }

    async fn list_issues(&self, owner: &str, repo: &str) -> Result<Vec<IssueInfo>, SniffError> {
        let project_id = encode_project_id(owner, repo);
        let request = ListIssuesRequest::new(&project_id);
        count_api_request("issues");
        let issues: Vec<Issue> = self
            .client
            .request(request)
            .await
            .map_err(map_schematic_error)?;

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
        let project_id = encode_project_id(owner, repo);

        // Fetch tags
        let tags_request = ListTagsRequest::new(&project_id);
        count_api_request("tags");
        let tags: Vec<Tag> = self
            .client
            .request(tags_request)
            .await
            .map_err(map_schematic_error)?;

        // Fetch releases
        let releases_request = ListReleasesRequest::new(&project_id);
        count_api_request("releases");
        let releases: Vec<Release> = self
            .client
            .request(releases_request)
            .await
            .map_err(map_schematic_error)?;

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
        // GitLab ListGroupProjects endpoint is not implemented in schematic-definitions
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
    fn test_encode_project_id() {
        assert_eq!(encode_project_id("owner", "repo"), "owner%2Frepo");
        assert_eq!(
            encode_project_id("group", "subgroup/repo"),
            "group%2Fsubgroup%2Frepo"
        );
    }


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

    #[test]
    fn test_map_schematic_error_missing_credentials() {
        let err = SchematicError::MissingCredential {
            env_vars: vec![
                "GITLAB_TOKEN".to_string(),
                "GITLAB_PRIVATE_TOKEN".to_string(),
            ],
        };
        match map_schematic_error(err) {
            SniffError::MissingCredentials { provider, env_var } => {
                assert_eq!(provider, "GitLab");
                assert!(env_var.contains("GITLAB_TOKEN"));
            }
            _ => panic!("Expected MissingCredentials error"),
        }
    }

    #[test]
    fn test_map_schematic_error_rate_limited() {
        let err = SchematicError::ApiError {
            status: 403,
            body: "Rate limit exceeded".to_string(),
        };
        match map_schematic_error(err) {
            SniffError::RateLimited { provider, .. } => {
                assert_eq!(provider, "GitLab");
            }
            _ => panic!("Expected RateLimited error"),
        }
    }

    #[test]
    fn test_map_schematic_error_not_found() {
        let err = SchematicError::ApiError {
            status: 404,
            body: "Not Found".to_string(),
        };
        match map_schematic_error(err) {
            SniffError::RemoteApi {
                provider,
                status,
                message,
            } => {
                assert_eq!(provider, "GitLab");
                assert_eq!(status, 404);
                assert_eq!(message, "Not found");
            }
            _ => panic!("Expected RemoteApi error"),
        }
    }
}
