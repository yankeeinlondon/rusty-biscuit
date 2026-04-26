//! Remote repository provider trait.
//!
//! The [`RemoteRepoProvider`] trait defines the interface for querying remote
//! git hosting providers. Each provider (GitHub, GitLab, Gitea, Bitbucket)
//! implements this trait using its schematic-generated API client.

use async_trait::async_trait;

use super::types::{
    CiCdInfo, DocumentRef, GitProvider, IssueInfo, KeyUrls, OrgInfo, OrgRepoRef,
    PullRequestInfo, PullRequestState, RemoteReport, RepoMetadata, TagsAndReleases,
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
        // The metadata call is required - if it fails, the whole report fails
        let metadata = self.get_repo_metadata(owner, repo).await?;

        // All other calls are optional - failures produce empty/None values
        let org_info = self.get_org_info(owner).await.ok();
        let documents = self.list_documents(owner, repo).await.unwrap_or_default();
        let pull_requests = self
            .list_pull_requests(owner, repo, PullRequestState::Open)
            .await
            .unwrap_or_default();
        let issues = self.list_issues(owner, repo).await.unwrap_or_default();
        let tags_and_releases = self
            .get_tags_and_releases(owner, repo)
            .await
            .unwrap_or_default();
        let cicd = self.detect_cicd(owner, repo).await.unwrap_or(None);
        let org_repos = self.list_org_repos(owner).await.unwrap_or_default();
        let key_urls = self.build_key_urls(owner, repo);

        Ok(RemoteReport {
            provider: self.provider(),
            metadata,
            org_info,
            documents,
            pull_requests,
            issues,
            tags_and_releases,
            ci_cd: cicd.into_iter().collect(),
            org_repos,
            key_urls,
        })
    }
}

// Compile-time assertion: the trait is object-safe
#[cfg(test)]
const _: () = {
    const fn _assert_object_safe<T: ?Sized>() {}
    _assert_object_safe::<dyn RemoteRepoProvider>();
};
