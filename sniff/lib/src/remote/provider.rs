//! Remote repository provider trait.
//!
//! The [`RemoteRepoProvider`] trait defines the interface for querying remote
//! git hosting providers. Each provider (GitHub, GitLab, Gitea, Bitbucket)
//! implements this trait using its schematic-generated API client.

use async_trait::async_trait;

use super::snapshot::RemoteRepoSnapshot;
use super::types::{
    CiCdInfo, DocumentRef, GitProvider, IssueInfo, KeyUrls, OrgInfo, OrgRepoRef, PullRequestInfo,
    PullRequestState, RemoteReport, RepoMetadata, TagsAndReleases,
};
use crate::error::SniffError;

/// Trait for querying remote git hosting providers.
///
/// Each method returns `Result`-wrapped values to handle API errors, rate
/// limiting, and missing credentials. Providers that don't support certain
/// features (e.g., Bitbucket issues are optional) return empty collections
/// rather than errors.
///
/// ## Object Safety
///
/// This trait is object-safe and can be used as `dyn RemoteRepoProvider`.
/// The `async_trait` attribute handles the async method transformation.
///
/// ## Default Implementation
///
/// The [`RemoteRepoProvider::fetch_report`] method has a default implementation that calls all
/// other methods and aggregates results. Individual method failures are
/// handled gracefully using `unwrap_or_default()` or `ok()` so partial
/// results are still returned.
///
/// ## Examples
///
/// ```ignore
/// use sniff::remote::{RemoteRepoProvider, GitHubRemote};
///
/// let remote = GitHubRemote::new("rust-lang", "cargo")?;
/// let report = remote.fetch_report("rust-lang", "cargo").await?;
/// println!("Stars: {:?}", report.metadata.stars);
/// ```
#[async_trait]
pub trait RemoteRepoProvider: Send + Sync {
    /// Returns the provider type (GitHub, GitLab, etc.).
    fn provider(&self) -> GitProvider;

    /// Get repository metadata (stars, forks, license, description, etc.).
    async fn get_repo_metadata(&self, owner: &str, repo: &str) -> Result<RepoMetadata, SniffError>;

    /// Get organization/group information.
    ///
    /// For GitHub, this queries the org API. For GitLab, this queries the
    /// group API. For Bitbucket, this returns workspace information.
    async fn get_org_info(&self, org: &str) -> Result<OrgInfo, SniffError>;

    /// List documentation files (README, CHANGELOG, etc.) in the repository.
    ///
    /// Discovers files by traversing the repository tree and categorizing
    /// markdown, text, and reStructuredText files.
    async fn list_documents(&self, owner: &str, repo: &str)
    -> Result<Vec<DocumentRef>, SniffError>;

    /// Get file content by path.
    ///
    /// Returns the raw text content of a file at the given path in the
    /// repository's default branch.
    async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> Result<String, SniffError>;

    /// List pull requests / merge requests filtered by state.
    ///
    /// For GitLab, these are called "merge requests". The provider
    /// implementation normalizes the terminology.
    ///
    /// The `state` parameter filters which PRs to return. Providers that
    /// don't support a particular state natively will post-filter the results.
    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: PullRequestState,
    ) -> Result<Vec<PullRequestInfo>, SniffError>;

    /// List open issues.
    ///
    /// Note: GitHub's issues API returns both issues and pull requests.
    /// The provider implementation filters out pull requests.
    async fn list_issues(&self, owner: &str, repo: &str) -> Result<Vec<IssueInfo>, SniffError>;

    /// Get tags and releases.
    ///
    /// Returns all tags and any associated release metadata. Tags without
    /// release information are still included in the `tags` field.
    async fn get_tags_and_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<TagsAndReleases, SniffError>;

    /// Detect CI/CD configuration.
    ///
    /// Looks for CI/CD configuration files (`.github/workflows`, `.gitlab-ci.yml`,
    /// etc.) and returns information about detected CI/CD providers.
    ///
    /// Returns `None` if no CI/CD configuration is detected.
    async fn detect_cicd(&self, owner: &str, repo: &str) -> Result<Option<CiCdInfo>, SniffError>;

    /// List recent workflow runs.
    ///
    /// Fetches actual CI/CD execution runs from the provider's API.
    /// Returns up to `limit` recent runs. Providers that don't support
    /// workflow runs return an empty vector.
    ///
    /// The default implementation returns an empty vector.
    async fn list_workflow_runs(
        &self,
        _owner: &str,
        _repo: &str,
        _limit: usize,
    ) -> Result<Vec<CiCdInfo>, SniffError> {
        Ok(Vec::new())
    }

    /// Resolve the shared per-report evidence: metadata, default branch, and tree.
    ///
    /// The default implementation resolves metadata only and reports the tree as
    /// unavailable, so a provider that has not adopted this hook keeps working —
    /// its `list_documents`/`detect_cicd` are still called through the defaults
    /// below (R11.4).
    ///
    /// ## Errors
    ///
    /// Metadata is required; propagate its failure. A *tree* failure must not be
    /// an error here — degrade to [`super::RemoteTree::unavailable`] so the report still
    /// carries metadata (R11.6).
    async fn snapshot(&self, owner: &str, repo: &str) -> Result<RemoteRepoSnapshot, SniffError> {
        let metadata = self.get_repo_metadata(owner, repo).await?;
        Ok(RemoteRepoSnapshot::metadata_only(owner, repo, metadata))
    }

    /// List documents from already-resolved evidence.
    ///
    /// The default re-enters [`RemoteRepoProvider::list_documents`], which is why
    /// a provider adopting this hook must override **both** — and must not
    /// implement `list_documents` by calling this method unless it does.
    async fn list_documents_with(
        &self,
        snapshot: &RemoteRepoSnapshot,
    ) -> Result<Vec<DocumentRef>, SniffError> {
        self.list_documents(&snapshot.owner, &snapshot.repo).await
    }

    /// Detect CI/CD from already-resolved evidence.
    ///
    /// See [`RemoteRepoProvider::list_documents_with`] for the override contract.
    async fn detect_cicd_with(
        &self,
        snapshot: &RemoteRepoSnapshot,
    ) -> Result<Option<CiCdInfo>, SniffError> {
        self.detect_cicd(&snapshot.owner, &snapshot.repo).await
    }

    /// List other repositories in the same org/group.
    ///
    /// Useful for discovering related projects in the same organization.
    async fn list_org_repos(&self, org: &str) -> Result<Vec<OrgRepoRef>, SniffError>;

    /// Build key URLs for the repository.
    ///
    /// Constructs URLs to important repository pages (issues, pull requests,
    /// wiki, CI/CD, releases, etc.) based on the provider's URL patterns.
    ///
    /// This method is synchronous because it doesn't require API calls.
    fn build_key_urls(&self, owner: &str, repo: &str) -> KeyUrls;

    /// Fetch complete remote report.
    ///
    /// This is the primary entry point for fetching all repository information.
    /// It calls all other trait methods and aggregates the results.
    ///
    /// ## Graceful Degradation
    ///
    /// Individual method failures do not cause the entire report to fail.
    /// Optional fields are set to `None` and collections default to empty
    /// when their respective API calls fail.
    ///
    /// ## Required vs Optional
    ///
    /// - **Required**: `get_repo_metadata` - if this fails, the entire report fails
    /// - **Optional**: All other methods - failures result in empty/None values
    async fn fetch_report(&self, owner: &str, repo: &str) -> Result<RemoteReport, SniffError> {
        // Resolve metadata, default branch, and tree exactly once for the whole
        // report. The document and CI/CD projections below read this instead of
        // re-fetching their own copies, which is what made a single report cost up
        // to three metadata calls and three identical tree calls.
        let snapshot = self.snapshot(owner, repo).await?;
        let metadata = snapshot.metadata.clone();

        // All other calls are optional and independent, so run them concurrently
        // to collapse a chain of API round-trips into a single parallel batch.
        // `join!` (not `try_join!`) is used deliberately: each branch swallows
        // its own failure into an empty/None value, so a partial failure must
        // not short-circuit the others.
        let (org_info, documents, pull_requests, issues, tags_and_releases, cicd, org_repos) = tokio::join!(
            async { self.get_org_info(owner).await.ok() },
            async { self.list_documents_with(&snapshot).await.unwrap_or_default() },
            async {
                self.list_pull_requests(owner, repo, PullRequestState::Open)
                    .await
                    .unwrap_or_default()
            },
            async { self.list_issues(owner, repo).await.unwrap_or_default() },
            async {
                self.get_tags_and_releases(owner, repo)
                    .await
                    .unwrap_or_default()
            },
            // Try to fetch actual workflow runs first; fall back to presence detection
            async {
                match self.list_workflow_runs(owner, repo, 5).await {
                    Ok(runs) if !runs.is_empty() => runs,
                    _ => self
                        .detect_cicd_with(&snapshot)
                        .await
                        .unwrap_or(None)
                        .into_iter()
                        .collect(),
                }
            },
            async { self.list_org_repos(owner).await.unwrap_or_default() },
        );
        let key_urls = self.build_key_urls(owner, repo);

        Ok(RemoteReport {
            provider: self.provider(),
            metadata,
            org_info,
            documents,
            pull_requests,
            issues,
            tags_and_releases,
            ci_cd: cicd,
            org_repos,
            key_urls,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Compile-time assertion: the trait is object-safe.
    const _: () = {
        const fn _assert_object_safe<T: ?Sized>() {}
        _assert_object_safe::<dyn RemoteRepoProvider>();
    };

    /// Minimal provider for exercising the `fetch_report` CI/CD fallback logic.
    ///
    /// `workflow_runs` is what `list_workflow_runs` yields (or a `RemoteApi`
    /// error when `runs_fail` is set); `detected` is the presence-detection
    /// fallback returned by `detect_cicd`.
    struct FakeProvider {
        workflow_runs: Vec<CiCdInfo>,
        runs_fail: bool,
        detected: Option<CiCdInfo>,
    }

    fn run_entry(name: &str) -> CiCdInfo {
        CiCdInfo {
            provider: "GitHub Actions".to_string(),
            config_path: None,
            name: name.to_string(),
            status: "completed".to_string(),
            conclusion: Some("success".to_string()),
            html_url: None,
            started_at: Some("2024-01-15T10:30:00Z".to_string()),
            head_branch: Some("main".to_string()),
            event: Some("push".to_string()),
        }
    }

    fn presence_entry() -> CiCdInfo {
        CiCdInfo {
            provider: "GitHub Actions".to_string(),
            config_path: Some(".github/workflows".to_string()),
            name: "GitHub Actions".to_string(),
            status: "detected".to_string(),
            conclusion: None,
            html_url: None,
            started_at: None,
            head_branch: None,
            event: None,
        }
    }

    fn api_error() -> SniffError {
        SniffError::RemoteApi {
            provider: "Fake".to_string(),
            status: 500,
            message: "boom".to_string(),
        }
    }

    #[async_trait]
    impl RemoteRepoProvider for FakeProvider {
        fn provider(&self) -> GitProvider {
            GitProvider::GitHub
        }

        async fn get_repo_metadata(
            &self,
            _owner: &str,
            _repo: &str,
        ) -> Result<RepoMetadata, SniffError> {
            Ok(RepoMetadata {
                name: "repo".to_string(),
                full_name: "owner/repo".to_string(),
                description: None,
                private: false,
                default_branch: "main".to_string(),
                language: None,
                stars: None,
                forks: None,
                open_issues: None,
                archived: false,
                created_at: None,
                updated_at: None,
                pushed_at: None,
                license: None,
                topics: vec![],
                has_issues: None,
                has_wiki: None,
                homepage: None,
                html_url: "https://github.com/owner/repo".to_string(),
            })
        }

        async fn get_org_info(&self, _org: &str) -> Result<OrgInfo, SniffError> {
            Err(api_error())
        }

        async fn list_documents(
            &self,
            _owner: &str,
            _repo: &str,
        ) -> Result<Vec<DocumentRef>, SniffError> {
            Ok(Vec::new())
        }

        async fn get_file_content(
            &self,
            _owner: &str,
            _repo: &str,
            _path: &str,
        ) -> Result<String, SniffError> {
            Ok(String::new())
        }

        async fn list_pull_requests(
            &self,
            _owner: &str,
            _repo: &str,
            _state: PullRequestState,
        ) -> Result<Vec<PullRequestInfo>, SniffError> {
            Ok(Vec::new())
        }

        async fn list_issues(
            &self,
            _owner: &str,
            _repo: &str,
        ) -> Result<Vec<IssueInfo>, SniffError> {
            Ok(Vec::new())
        }

        async fn get_tags_and_releases(
            &self,
            _owner: &str,
            _repo: &str,
        ) -> Result<TagsAndReleases, SniffError> {
            Ok(TagsAndReleases::default())
        }

        async fn detect_cicd(
            &self,
            _owner: &str,
            _repo: &str,
        ) -> Result<Option<CiCdInfo>, SniffError> {
            Ok(self.detected.clone())
        }

        async fn list_workflow_runs(
            &self,
            _owner: &str,
            _repo: &str,
            _limit: usize,
        ) -> Result<Vec<CiCdInfo>, SniffError> {
            if self.runs_fail {
                Err(api_error())
            } else {
                Ok(self.workflow_runs.clone())
            }
        }

        async fn list_org_repos(&self, _org: &str) -> Result<Vec<OrgRepoRef>, SniffError> {
            Ok(Vec::new())
        }

        fn build_key_urls(&self, _owner: &str, _repo: &str) -> KeyUrls {
            KeyUrls {
                repo: "https://github.com/owner/repo".to_string(),
                homepage: None,
                docs: None,
                issues: None,
                pull_requests: None,
                wiki: None,
                ci_cd: None,
                insights: None,
                releases: None,
                settings: None,
            }
        }
    }

    #[tokio::test]
    async fn test_fetch_report_uses_workflow_runs_when_present() {
        let provider = FakeProvider {
            workflow_runs: vec![run_entry("CI")],
            runs_fail: false,
            detected: Some(presence_entry()),
        };
        let report = provider.fetch_report("owner", "repo").await.unwrap();
        // Run-shaped entries win over the presence-detection fallback.
        assert_eq!(report.ci_cd.len(), 1);
        assert_eq!(report.ci_cd[0].name, "CI");
        assert!(report.ci_cd[0].started_at.is_some());
    }

    #[tokio::test]
    async fn test_fetch_report_falls_back_to_detection_when_no_runs() {
        let provider = FakeProvider {
            workflow_runs: vec![],
            runs_fail: false,
            detected: Some(presence_entry()),
        };
        let report = provider.fetch_report("owner", "repo").await.unwrap();
        assert_eq!(report.ci_cd.len(), 1);
        assert_eq!(report.ci_cd[0].status, "detected");
        assert!(report.ci_cd[0].started_at.is_none());
    }

    #[tokio::test]
    async fn test_fetch_report_falls_back_when_runs_fail() {
        let provider = FakeProvider {
            workflow_runs: vec![],
            runs_fail: true,
            detected: Some(presence_entry()),
        };
        let report = provider.fetch_report("owner", "repo").await.unwrap();
        assert_eq!(report.ci_cd.len(), 1);
        assert_eq!(report.ci_cd[0].status, "detected");
    }

    #[tokio::test]
    async fn test_fetch_report_empty_cicd_when_runs_fail_and_no_detection() {
        let provider = FakeProvider {
            workflow_runs: vec![],
            runs_fail: true,
            detected: None,
        };
        let report = provider.fetch_report("owner", "repo").await.unwrap();
        assert!(report.ci_cd.is_empty());
    }
}
