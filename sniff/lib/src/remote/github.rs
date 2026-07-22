//! GitHub remote repository provider.
//!
//! This module implements [`RemoteRepoProvider`] for GitHub using the
//! generated schematic-schema client.

use async_trait::async_trait;
use schematic_schema::github::*;
use schematic_schema::shared::{AuthStrategy, SchematicError, UpdateStrategy};

use super::{
    count_api_request,
    provider::RemoteRepoProvider,
    snapshot::{documents_from_tree, RemoteRepoSnapshot, RemoteTree, RemoteTreeFile, CONTINUATION_PREFIXES},
    types::{
        CiCdInfo, DocumentRef, GitProvider, IssueInfo, KeyUrls, LicenseRef,
        OrgInfo, OrgRepoRef, PullRequestInfo, PullRequestState, ReleaseInfo, RepoMetadata, TagInfo,
        TagsAndReleases,
    },
};
use crate::error::SniffError;

/// GitHub remote repository provider.
///
/// Uses the schematic-generated GitHub API client for all HTTP operations.
/// Authentication is via the `GITHUB_TOKEN` or `GH_TOKEN` environment variable.
///
/// ## Examples
///
/// ```ignore
/// use sniff::remote::{GitHubRemote, RemoteRepoProvider};
///
/// let provider = GitHubRemote::new()?;
/// let metadata = provider.get_repo_metadata("rust-lang", "cargo").await?;
/// println!("Stars: {:?}", metadata.stars);
/// ```
pub struct GitHubRemote {
    client: GitHub,
}

impl GitHubRemote {
    /// Create a new GitHub provider using GITHUB_TOKEN from environment.
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    pub fn new() -> Result<Self, SniffError> {
        Ok(Self {
            client: GitHub::new(),
        })
    }

    /// Create with a custom base URL (for GitHub Enterprise).
    ///
    /// ## Arguments
    ///
    /// * `base_url` - Base URL for the GitHub Enterprise API (e.g., `https://github.example.com/api/v3`)
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    pub fn with_base_url(base_url: &str) -> Result<Self, SniffError> {
        Ok(Self {
            client: GitHub::with_base_url(base_url),
        })
    }
}

impl Default for GitHubRemote {
    fn default() -> Self {
        Self {
            client: GitHub::new(),
        }
    }
}

impl GitHubRemote {
    /// Build a GitHub API client variant that performs no authentication.
    ///
    /// The client is constructed by overriding both the env-fallback list
    /// (so no credentials can be picked up from `GITHUB_TOKEN`, `GH_TOKEN`,
    /// etc.) **and** the auth strategy itself (so the schematic runtime
    /// does not pre-flight-reject the request for missing credentials).
    /// The combination produces a truly anonymous client whose request
    /// passes through to the wire and is judged by the API alone.
    ///
    /// This is used for the "attempt unauthenticated" fallback mandated by
    /// the `sniff repo pr` spec: when an authenticated request fails because
    /// no credentials are configured, we retry once with this anonymous
    /// client before surfacing a `MissingCredentials` error to the user.
    fn unauthenticated_client(&self) -> GitHub {
        self.client
            .variant()
            .env_auth(Vec::new())
            .auth_update(UpdateStrategy::ChangeTo(AuthStrategy::None))
            .build()
    }

    /// Fetches the recursive root tree once, continuing if GitHub truncated it.
    ///
    /// ## Notes
    ///
    /// GitHub truncates a recursive tree past ~100k entries or 7 MB and reports it
    /// in `truncated`. That flag was previously ignored, so a large repository
    /// reported "no docs, no CI" with full confidence. Completing such a tree would
    /// cost an unbounded walk; instead this continues only the prefixes the
    /// document and CI/CD projections actually read (R11.5).
    async fn fetch_tree(
        &self,
        owner: &str,
        repo: &str,
        default_branch: &str,
    ) -> Result<RemoteTree, SniffError> {
        let request = GetGitTreeRecursiveRequest::new(owner, repo, default_branch);
        count_api_request("tree");
        let response: GitTreeResponse = self
            .client
            .request(request)
            .await
            .map_err(map_schematic_error)?;

        let mut tree = RemoteTree::observed(blobs_of(response.tree), response.truncated);
        if tree.truncated {
            self.continue_truncated_tree(owner, repo, default_branch, &mut tree)
                .await;
        }
        Ok(tree)
    }

    /// Re-requests the document/CI prefixes of a truncated tree, one subtree each.
    ///
    /// Failures are swallowed per prefix: a continuation is a best-effort repair of
    /// evidence that is already known incomplete, so failing one must not discard
    /// the entries the root tree did return.
    async fn continue_truncated_tree(
        &self,
        owner: &str,
        repo: &str,
        default_branch: &str,
        tree: &mut RemoteTree,
    ) {
        let mut recovered = Vec::new();

        for prefix in CONTINUATION_PREFIXES {
            // `branch:path` addresses a subtree directly, so each continuation is
            // one bounded request rather than a walk.
            let request = GetGitTreeRecursiveRequest::new(
                owner,
                repo,
                format!("{default_branch}:{prefix}"),
            );
            // Counted under its own slug: a continuation preserves correctness and
            // must stay distinguishable from the duplicate root-tree requests this
            // phase removed.
            count_api_request("tree_continuation");
            let Ok(response) = self.client.request::<GitTreeResponse>(request).await else {
                // Almost always a 404 for a prefix this repository does not have.
                continue;
            };

            // Subtree paths are relative to the subtree root; re-prefix them so the
            // projections see repository-root-relative paths like every other entry.
            recovered.extend(blobs_of(response.tree).into_iter().map(|file| RemoteTreeFile {
                path: format!("{prefix}/{}", file.path),
                size: file.size,
            }));
        }

        tree.extend_from_continuation(recovered);
    }
}

/// Keeps blob entries and drops directories, normalizing to [`RemoteTreeFile`].
fn blobs_of(entries: Vec<GitTreeEntry>) -> Vec<RemoteTreeFile> {
    entries
        .into_iter()
        .filter(|entry| entry.entry_type == "blob")
        .map(|entry| RemoteTreeFile {
            path: entry.path,
            size: entry.size,
        })
        .collect()
}


/// Map a [`SchematicError`] to a [`SniffError`].
///
/// Handles special cases:
/// - `MissingCredential` / `AuthenticationRequired` -> `MissingCredentials`
/// - 401 -> `InvalidCredentials`
/// - 403 with rate limit info -> `RateLimited`
/// - 404 -> `RemoteApi` with "Not found"
/// - Other HTTP errors -> `RemoteApi`
///
/// ## Notes
///
/// `MissingCredentials` is only emitted *after* the anonymous retry has
/// failed (see [`GitHubRemote::unauthenticated_client`]) or when the API
/// explicitly demands credentials for a private resource. Call sites that
/// want the "attempt unauthenticated" behaviour must use the
/// `try_with_anonymous_fallback`-style wrapper rather than mapping the
/// first failure directly through this function.
fn map_schematic_error(err: SchematicError) -> SniffError {
    match err {
        SchematicError::MissingCredential { env_vars } => SniffError::MissingCredentials {
            provider: "GitHub".to_string(),
            env_var: env_vars.join(" or "),
        },
        SchematicError::AuthenticationRequired {
            env_fallback_vars, ..
        } => SniffError::MissingCredentials {
            provider: "GitHub".to_string(),
            env_var: env_fallback_vars.join(" or "),
        },
        SchematicError::ApiError { status: 401, body } => {
            let message = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("message")?.as_str().map(String::from))
                .unwrap_or_else(|| "bad credentials".to_string());
            SniffError::InvalidCredentials {
                provider: "GitHub".to_string(),
                message,
            }
        }
        SchematicError::ApiError { status: 403, body } => {
            // Check if this is a rate limit error
            if body.to_lowercase().contains("rate limit") {
                SniffError::RateLimited {
                    provider: "GitHub".to_string(),
                    retry_after: None, // Would need to parse X-RateLimit-Reset header
                }
            } else {
                SniffError::RemoteApi {
                    provider: "GitHub".to_string(),
                    status: 403,
                    message: body,
                }
            }
        }
        SchematicError::ApiError { status: 429, .. } => SniffError::RateLimited {
            provider: "GitHub".to_string(),
            retry_after: None,
        },
        SchematicError::ApiError { status: 404, .. } => SniffError::RemoteApi {
            provider: "GitHub".to_string(),
            status: 404,
            message: "Not found".to_string(),
        },
        SchematicError::ApiError { status, body } => SniffError::RemoteApi {
            provider: "GitHub".to_string(),
            status,
            message: body,
        },
        SchematicError::Http(e) => SniffError::RemoteApi {
            provider: "GitHub".to_string(),
            status: 0,
            message: e.to_string(),
        },
        SchematicError::Json(e) => SniffError::RemoteApi {
            provider: "GitHub".to_string(),
            status: 0,
            message: format!("JSON parse error: {}", e),
        },
        other => SniffError::RemoteApi {
            provider: "GitHub".to_string(),
            status: 0,
            message: other.to_string(),
        },
    }
}

/// Map a GitHub [`WorkflowRun`] to normalized [`CiCdInfo`].
fn map_workflow_run(run: WorkflowRun) -> CiCdInfo {
    CiCdInfo {
        provider: "GitHub Actions".to_string(),
        config_path: None,
        name: run.name.unwrap_or_else(|| "workflow".to_string()),
        status: run.status.unwrap_or_default(),
        conclusion: run.conclusion,
        html_url: run.html_url,
        started_at: run.created_at,
        head_branch: run.head_branch,
        event: run.event,
    }
}

#[async_trait]
impl RemoteRepoProvider for GitHubRemote {
    fn provider(&self) -> GitProvider {
        GitProvider::GitHub
    }

    async fn get_repo_metadata(&self, owner: &str, repo: &str) -> Result<RepoMetadata, SniffError> {
        let request = GetRepositoryRequest::new(owner, repo);
        count_api_request("metadata");
        let info: RepositoryInfo = self
            .client
            .request(request)
            .await
            .map_err(map_schematic_error)?;

        Ok(RepoMetadata {
            name: info.name,
            full_name: info.full_name,
            description: info.description,
            private: info.private,
            default_branch: info.default_branch,
            language: info.language,
            stars: info.stargazers_count,
            forks: info.forks_count,
            open_issues: info.open_issues_count,
            archived: info.archived,
            created_at: info.created_at,
            updated_at: info.updated_at,
            pushed_at: info.pushed_at,
            license: info.license.map(|l| LicenseRef {
                spdx_id: l.spdx_id,
                name: l.name,
            }),
            topics: info.topics,
            has_issues: info.has_issues,
            has_wiki: info.has_wiki,
            homepage: None, // GitHub API returns this in a separate field we don't have
            html_url: info.html_url,
        })
    }

    async fn get_org_info(&self, _org: &str) -> Result<OrgInfo, SniffError> {
        // GitHub GetOrganization endpoint is not implemented in schematic-definitions
        // Return a placeholder error for now
        Err(SniffError::RemoteApi {
            provider: "GitHub".to_string(),
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
        let request = GetRepositoryContentRawRequest::new(owner, repo, path);
        count_api_request("contents");
        self.client
            .get_repository_content_raw(request)
            .await
            .map_err(map_schematic_error)
    }

    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: PullRequestState,
    ) -> Result<Vec<PullRequestInfo>, SniffError> {
        let mut request = ListPullRequestsRequest::new(owner, repo);

        // Map PullRequestState to GitHub API state parameter
        match state {
            PullRequestState::Open => {
                request = request.with_state("open".to_string());
            }
            PullRequestState::Closed => {
                request = request.with_state("closed".to_string());
            }
            PullRequestState::Merged | PullRequestState::Draft | PullRequestState::All => {
                request = request.with_state("all".to_string());
            }
        }

        // Attempt the request with the configured client first. If it fails
        // because no credentials are available, retry once with an explicitly
        // unauthenticated client. Only after the anonymous retry also fails
        // (or on a real 401/403) do we surface a credentials error.
        count_api_request("pulls");
        let prs: Vec<PullRequestSummary> = match self.client.request(request.clone()).await {
            Ok(prs) => prs,
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

        let mut prs: Vec<PullRequestInfo> = prs
            .into_iter()
            .map(|pr| PullRequestInfo {
                number: pr.number,
                title: pr.title,
                state: pr.state,
                author: pr.user.login,
                draft: pr.draft.unwrap_or(false),
                source_branch: Some(pr.head.ref_name),
                target_branch: Some(pr.base.ref_name),
                labels: pr.labels.into_iter().map(|l| l.name).collect(),
                body: pr.body,
                created_at: pr.created_at,
                updated_at: Some(pr.updated_at),
                merged_at: pr.merged_at,
                html_url: pr.html_url,
            })
            .collect();

        // Post-filter for states GitHub API doesn't support directly
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
        let request = ListIssuesRequest::new(owner, repo);
        count_api_request("issues");
        let issues: Vec<IssueSummary> = self
            .client
            .request(request)
            .await
            .map_err(map_schematic_error)?;

        // Filter out pull requests (GitHub issues API includes PRs)
        Ok(issues
            .into_iter()
            .filter(|issue| !issue.is_pull_request())
            .map(|issue| IssueInfo {
                number: issue.number,
                title: issue.title,
                state: issue.state,
                author: issue.user.login,
                comment_count: Some(issue.comments),
                labels: issue.labels.into_iter().map(|l| l.name).collect(),
                created_at: issue.created_at,
                updated_at: Some(issue.updated_at),
                closed_at: issue.closed_at,
                html_url: issue.html_url,
            })
            .collect())
    }

    async fn get_tags_and_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<TagsAndReleases, SniffError> {
        // Fetch tags
        let tags_request = ListTagsRequest::new(owner, repo);
        count_api_request("tags");
        let tags: Vec<RepoTag> = self
            .client
            .request(tags_request)
            .await
            .map_err(map_schematic_error)?;

        // Fetch releases
        let releases_request = ListReleasesRequest::new(owner, repo);
        count_api_request("releases");
        let releases: Vec<Release> = self
            .client
            .request(releases_request)
            .await
            .map_err(map_schematic_error)?;

        // Convert tags - we can't easily determine if they're annotated without
        // making additional API calls per tag, so we default to false
        let tag_infos: Vec<TagInfo> = tags
            .into_iter()
            .map(|tag| TagInfo {
                name: tag.name,
                commit_sha: tag.commit.sha,
                annotated: false, // Would need GetTagReference + GetAnnotatedTag to determine
                message: None,
                tagger: None,
                tagged_at: None,
            })
            .collect();

        // Convert releases
        let release_infos: Vec<ReleaseInfo> = releases
            .into_iter()
            .map(|release| ReleaseInfo {
                name: release.name,
                tag_name: release.tag_name,
                draft: release.draft,
                prerelease: release.prerelease,
                published_at: release.published_at,
                html_url: Some(release.html_url),
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
        let has_workflows = snapshot.tree.has_path_under(".github/workflows");

        if has_workflows {
            Ok(Some(CiCdInfo {
                provider: "GitHub Actions".to_string(),
                config_path: Some(".github/workflows".to_string()),
                name: "GitHub Actions".to_string(),
                status: "detected".to_string(),
                conclusion: None,
                html_url: Some(format!("https://github.com/{}/{}/actions", owner, repo)),
                started_at: None,
                head_branch: None,
                event: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        limit: usize,
    ) -> Result<Vec<CiCdInfo>, SniffError> {
        let request = ListWorkflowRunsRequest::new(owner, repo).with_per_page(limit as i64);

        // Attempt the request with the configured client first. If it fails
        // because no credentials are available, or with a 401/403 response,
        // retry once with an explicitly unauthenticated client.
        count_api_request("workflow_runs");
        let response: WorkflowRunsResponse = match self.client.request(request.clone()).await {
            Ok(runs) => runs,
            Err(SchematicError::MissingCredential { .. })
            | Err(SchematicError::AuthenticationRequired { .. })
            | Err(SchematicError::ApiError { status: 401, .. })
            | Err(SchematicError::ApiError { status: 403, .. }) => {
                count_api_request("workflow_runs");
                self.unauthenticated_client()
                    .request(request)
                    .await
                    .map_err(map_schematic_error)?
            }
            Err(err) => return Err(map_schematic_error(err)),
        };

        // `per_page` bounds the server-side page, but a generous or non-honoring
        // response could still exceed `limit`; `take(limit)` pins the invariant.
        Ok(response
            .workflow_runs
            .into_iter()
            .map(map_workflow_run)
            .take(limit)
            .collect())
    }

    async fn list_org_repos(&self, _org: &str) -> Result<Vec<OrgRepoRef>, SniffError> {
        // GitHub ListOrgRepos endpoint is not implemented in schematic-definitions
        // Return empty for now
        Ok(Vec::new())
    }

    fn build_key_urls(&self, owner: &str, repo: &str) -> KeyUrls {
        let base = format!("https://github.com/{}/{}", owner, repo);

        KeyUrls {
            repo: base.clone(),
            homepage: None, // Would come from repo metadata
            docs: None,
            issues: Some(format!("{}/issues", base)),
            pull_requests: Some(format!("{}/pulls", base)),
            wiki: Some(format!("{}/wiki", base)),
            ci_cd: Some(format!("{}/actions", base)),
            insights: Some(format!("{}/pulse", base)),
            releases: Some(format!("{}/releases", base)),
            settings: Some(format!("{}/settings", base)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[test]
    fn test_build_key_urls() {
        let provider = GitHubRemote::default();
        let urls = provider.build_key_urls("rust-lang", "cargo");

        assert_eq!(urls.repo, "https://github.com/rust-lang/cargo");
        assert_eq!(
            urls.issues,
            Some("https://github.com/rust-lang/cargo/issues".to_string())
        );
        assert_eq!(
            urls.pull_requests,
            Some("https://github.com/rust-lang/cargo/pulls".to_string())
        );
        assert_eq!(
            urls.ci_cd,
            Some("https://github.com/rust-lang/cargo/actions".to_string())
        );
    }

    #[test]
    fn test_map_schematic_error_missing_credentials() {
        let err = SchematicError::MissingCredential {
            env_vars: vec!["GITHUB_TOKEN".to_string(), "GH_TOKEN".to_string()],
        };
        match map_schematic_error(err) {
            SniffError::MissingCredentials { provider, env_var } => {
                assert_eq!(provider, "GitHub");
                assert!(env_var.contains("GITHUB_TOKEN"));
            }
            _ => panic!("Expected MissingCredentials error"),
        }
    }

    #[test]
    fn test_map_schematic_error_rate_limited() {
        let err = SchematicError::ApiError {
            status: 403,
            body: "API rate limit exceeded".to_string(),
        };
        match map_schematic_error(err) {
            SniffError::RateLimited { provider, .. } => {
                assert_eq!(provider, "GitHub");
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
                assert_eq!(provider, "GitHub");
                assert_eq!(status, 404);
                assert_eq!(message, "Not found");
            }
            _ => panic!("Expected RemoteApi error"),
        }
    }

    #[test]
    fn test_map_workflow_run_full() {
        let run = WorkflowRun {
            id: 12345,
            name: Some("CI".to_string()),
            run_number: Some(42),
            run_attempt: Some(1),
            status: Some("completed".to_string()),
            conclusion: Some("success".to_string()),
            workflow_id: Some(101),
            event: Some("push".to_string()),
            head_branch: Some("main".to_string()),
            head_sha: Some("abc123".to_string()),
            html_url: Some("https://github.com/owner/repo/actions/runs/12345".to_string()),
            url: None,
            created_at: Some("2024-01-15T10:30:00Z".to_string()),
            updated_at: None,
            actor: None,
            triggering_actor: None,
        };

        let info = map_workflow_run(run);

        assert_eq!(info.provider, "GitHub Actions");
        assert_eq!(info.name, "CI");
        assert_eq!(info.status, "completed");
        assert_eq!(info.conclusion, Some("success".to_string()));
        assert_eq!(
            info.html_url,
            Some("https://github.com/owner/repo/actions/runs/12345".to_string())
        );
        assert_eq!(info.started_at, Some("2024-01-15T10:30:00Z".to_string()));
        assert_eq!(info.head_branch, Some("main".to_string()));
        assert_eq!(info.event, Some("push".to_string()));
        assert_eq!(info.config_path, None);
    }

    #[test]
    fn test_map_workflow_run_minimal() {
        let run = WorkflowRun {
            id: 0,
            name: None,
            run_number: None,
            run_attempt: None,
            status: None,
            conclusion: None,
            workflow_id: None,
            event: None,
            head_branch: None,
            head_sha: None,
            html_url: None,
            url: None,
            created_at: None,
            updated_at: None,
            actor: None,
            triggering_actor: None,
        };

        let info = map_workflow_run(run);

        assert_eq!(info.provider, "GitHub Actions");
        assert_eq!(info.name, "workflow");
        assert_eq!(info.status, "");
        assert_eq!(info.conclusion, None);
        assert_eq!(info.html_url, None);
        assert_eq!(info.started_at, None);
        assert_eq!(info.head_branch, None);
        assert_eq!(info.event, None);
    }
}
