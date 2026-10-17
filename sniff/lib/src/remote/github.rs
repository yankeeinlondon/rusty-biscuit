//! GitHub remote repository provider.
//!
//! This module implements [`RemoteRepoProvider`] for GitHub using the
//! generated schematic-schema client.

use async_trait::async_trait;
use schematic_schema::github::*;
use schematic_schema::schematic_runtime::AuthMethod;
use schematic_schema::schematic_runtime::pager::PagerError;

use super::{
    count_api_request,
    envelope::{Replies, Unauthorized},
    provider::RemoteRepoProvider,
    snapshot::{
        CONTINUATION_PREFIXES, RemoteRepoSnapshot, RemoteTree, RemoteTreeFile, documents_from_tree,
    },
    types::{
        CiCdInfo, DocumentRef, GitProvider, IssueInfo, KeyUrls, LicenseRef, OrgInfo, OrgRepoRef,
        PullRequestInfo, PullRequestState, ReleaseInfo, RepoMetadata, TagInfo, TagsAndReleases,
    },
};
use crate::error::SniffError;

const REPLIES: Replies = Replies::new("GitHub", Unauthorized::Rejected);

/// GitHub remote repository provider.
///
/// Uses the schematic-generated GitHub API client for all HTTP operations.
/// Each request sends the first non-empty `GITHUB_TOKEN` or `GH_TOKEN` as a
/// bearer token, and is sent anonymously when neither is set.
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
    /// Create a new GitHub provider for `api.github.com`.
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    pub fn new() -> Result<Self, SniffError> {
        Self::build(GitHub::new())
    }

    /// Create with a custom base URL (for GitHub Enterprise).
    ///
    /// ## Arguments
    ///
    /// * `base_url` - Base URL for the GitHub Enterprise API (e.g., `https://github.example.com/api/v3`)
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized,
    /// including when `base_url` is not an HTTP(S) URL with a host.
    pub fn with_base_url(base_url: &str) -> Result<Self, SniffError> {
        Self::build(GitHub::new().base_url(base_url))
    }

    fn build(builder: GitHub) -> Result<Self, SniffError> {
        let client = builder.build().map_err(|error| REPLIES.init(&error))?;
        Ok(Self { client })
    }
}

impl Default for GitHubRemote {
    /// ## Panics
    ///
    /// Panics if the HTTP client cannot be initialized; use
    /// [`GitHubRemote::new`] to handle that as an error.
    fn default() -> Self {
        Self::new().expect("the GitHub client builds from its default definition")
    }
}

impl GitHubRemote {
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
        count_api_request("tree");
        let response = REPLIES.value(
            self.client
                .get_git_tree(owner, repo, default_branch)
                .recursive(true)
                .await,
        )?;

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
            let call = self
                .client
                .get_git_tree(owner, repo, format!("{default_branch}:{prefix}"))
                .recursive(true);
            // Counted under its own slug: a continuation preserves correctness and
            // must stay distinguishable from the duplicate root-tree requests this
            // phase removed.
            count_api_request("tree_continuation");
            let Ok(response) = call.await else {
                // Almost always a 404 for a prefix this repository does not have.
                continue;
            };

            // Subtree paths are relative to the subtree root; re-prefix them so the
            // projections see repository-root-relative paths like every other entry.
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
        count_api_request("metadata");
        let info = REPLIES.value(self.client.get_repository(owner, repo).await)?;

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
        // Map PullRequestState to GitHub API state parameter
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
            .map(|pr| {
                let source_repo = pr.head.repo.as_ref().map(|repo| repo.full_name.clone());
                let source_repo_is_target = source_repo
                    .as_ref()
                    .zip(pr.base.repo.as_ref())
                    .map(|(source, target)| source.eq_ignore_ascii_case(&target.full_name));
                PullRequestInfo {
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
                    source_repo,
                    source_repo_is_target,
                    source_head_sha: Some(pr.head.sha).filter(|sha| !sha.is_empty()),
                }
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
        count_api_request("issues");
        let issues = REPLIES.value(self.client.list_issues(owner, repo).await)?;

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
        count_api_request("tags");
        let tags = REPLIES.value(self.client.list_tags(owner, repo).await)?;

        // Fetch releases
        count_api_request("releases");
        let releases = REPLIES.value(self.client.list_releases(owner, repo).await)?;

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
        let call = self.client.list_workflow_runs(owner, repo);
        // `per_page` is pager-managed: one page is fetched at this size. The
        // definition bounds it to 1..=100, and a size outside the bounds is
        // refused before sending, so the limit is clamped into them.
        let page_size = u32::try_from(limit.clamp(1, 100)).unwrap_or(100);

        count_api_request("workflow_runs");
        let first = call.clone().page_size(page_size).fetch_page().await;
        // Public workflow runs are readable anonymously, so a credential the API
        // rejects is retried once without one. A reply to an anonymous request is
        // final: the runtime never retries, and resending it would change nothing.
        let rejected_credential = matches!(
            &first,
            Err(PagerError::Api { meta, .. })
                if matches!(meta.status.as_u16(), 401 | 403)
                    && meta.request.auth != AuthMethod::Anonymous
        );
        let reply = if rejected_credential {
            count_api_request("workflow_runs");
            call.anonymous().page_size(page_size).fetch_page().await
        } else {
            first
        };
        let page = REPLIES.page(reply)?;

        // The page size bounds the server-side page, but a generous or
        // non-honoring response could still exceed `limit`; `take(limit)` pins
        // the invariant.
        Ok(page
            .items
            .into_iter()
            .map(map_workflow_run)
            .take(limit)
            .collect())
    }

    async fn list_org_repos(&self, _org: &str) -> Result<Vec<OrgRepoRef>, SniffError> {
        // The generated client has `list_org_repos`, but this provider does not call
        // it yet, so organization listings are always empty.
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
