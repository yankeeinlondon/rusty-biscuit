//! Normalized types for remote repository inspection.
//!
//! These types provide a provider-agnostic representation of repository
//! metadata, pull requests, issues, tags, releases, and other information
//! fetched from git hosting providers.

use serde::{Deserialize, Serialize};

/// Supported git hosting providers for remote inspection (Stage 1).
///
/// This enum covers the providers supported in the initial implementation.
/// Future stages will add AWS CodeCommit and Azure DevOps.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GitProvider {
    /// GitHub (github.com)
    GitHub,
    /// GitLab (gitlab.com or self-hosted)
    GitLab,
    /// Gitea or Forgejo (self-hosted)
    Gitea,
    /// Bitbucket (bitbucket.org)
    Bitbucket,
}

impl GitProvider {
    /// Returns the display name of the provider.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::GitHub => "GitHub",
            Self::GitLab => "GitLab",
            Self::Gitea => "Gitea",
            Self::Bitbucket => "Bitbucket",
        }
    }
}

impl std::fmt::Display for GitProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// High-level repository metadata, normalized across providers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoMetadata {
    /// Repository name.
    pub name: String,
    /// Full name (e.g., "owner/repo").
    pub full_name: String,
    /// Repository description.
    pub description: Option<String>,
    /// Whether the repository is private.
    pub private: bool,
    /// Default branch name.
    pub default_branch: String,
    /// Primary programming language.
    pub language: Option<String>,
    /// Star/favorite count.
    pub stars: Option<u64>,
    /// Fork count.
    pub forks: Option<u64>,
    /// Open issue count.
    pub open_issues: Option<u64>,
    /// Whether the repository is archived.
    pub archived: bool,
    /// Creation timestamp (ISO 8601).
    pub created_at: Option<String>,
    /// Last update timestamp (ISO 8601).
    pub updated_at: Option<String>,
    /// Last push timestamp (ISO 8601).
    pub pushed_at: Option<String>,
    /// License information.
    pub license: Option<LicenseRef>,
    /// Topics/tags on the repository.
    pub topics: Vec<String>,
    /// Whether issues are enabled.
    pub has_issues: Option<bool>,
    /// Whether the wiki is enabled.
    pub has_wiki: Option<bool>,
    /// Homepage URL (repo-configured, not hosting provider URL).
    pub homepage: Option<String>,
    /// HTML URL to the repository on the hosting provider.
    pub html_url: String,
}

/// Normalized license reference.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseRef {
    /// SPDX identifier (e.g., "MIT", "Apache-2.0").
    pub spdx_id: Option<String>,
    /// Human-readable license name.
    pub name: String,
}

/// Organization/workspace information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgInfo {
    /// Organization/workspace name or slug.
    pub name: String,
    /// Display name (may differ from slug).
    pub display_name: Option<String>,
    /// Organization description.
    pub description: Option<String>,
    /// Avatar URL.
    pub avatar_url: Option<String>,
    /// URL to the organization page.
    pub html_url: Option<String>,
}

/// A reference to a document file in the repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentRef {
    /// File path relative to repo root.
    pub path: String,
    /// Document category.
    pub category: DocumentCategory,
    /// File size in bytes (if available).
    pub size: Option<u64>,
}

/// Categories for repository documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DocumentCategory {
    /// README files (case-insensitive match on README.md).
    Readme,
    /// Documents inside `src/` directories.
    SourceDoc,
    /// Documents inside `docs/` or `doc/` directories.
    DocsFolder,
    /// Other markdown/text files at repo root or elsewhere.
    Other,
}

/// Normalized pull request state for filtering and display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PullRequestState {
    /// Open pull requests.
    Open,
    /// Closed pull requests (not merged).
    Closed,
    /// Merged pull requests.
    Merged,
    /// Draft pull requests.
    Draft,
    /// All pull requests regardless of state.
    All,
}

/// A canonical query field accepting one value or an array of values.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryValues<T> {
    One(T),
    Many(Vec<T>),
}

impl<T> QueryValues<T> {
    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::One(value) => std::slice::from_ref(value),
            Self::Many(values) => values,
        }
    }
}

impl PullRequestState {
    /// Returns the lowercase string representation used for CLI input and JSON.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Merged => "merged",
            Self::Draft => "draft",
            Self::All => "all",
        }
    }
}

impl std::fmt::Display for PullRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for PullRequestState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "merged" => Ok(Self::Merged),
            "draft" => Ok(Self::Draft),
            "all" => Ok(Self::All),
            _ => Err(format!(
                "invalid PR state '{}'. Expected one of: open, closed, merged, draft, all",
                s
            )),
        }
    }
}

/// Normalized pull/merge request information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestInfo {
    /// PR number/ID.
    pub number: u64,
    /// PR title.
    pub title: String,
    /// State: "open", "closed", "merged".
    pub state: String,
    /// Author login/username.
    pub author: String,
    /// Whether this is a draft PR.
    pub draft: bool,
    /// Source branch name.
    pub source_branch: Option<String>,
    /// Target branch name.
    pub target_branch: Option<String>,
    /// Labels attached to the PR.
    pub labels: Vec<String>,
    /// PR description/body (if available).
    pub body: Option<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: Option<String>,
    /// Merge timestamp (None if not merged).
    pub merged_at: Option<String>,
    /// HTML URL to the PR.
    pub html_url: String,
}

/// Repository-qualified identity of a pull or merge request.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PullRequestReference {
    pub provider: GitProvider,
    pub api_flavor: String,
    pub host: String,
    pub namespace: String,
    pub repository: String,
    pub native_id: String,
    pub display_id: String,
    pub number: Option<u64>,
    pub web_url: Option<String>,
    pub api_url: Option<String>,
    pub original_url: Option<String>,
}

/// Structured pull-request result retaining identity and normalized details.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestRecord {
    pub identity: PullRequestReference,
    pub details: PullRequestInfo,
}

/// Provider-neutral pull-request query vocabulary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PullRequestQuery {
    pub state: Option<QueryValues<PullRequestState>>,
    pub source_branch: Option<String>,
    pub target_branch: Option<String>,
    pub author: Option<String>,
    pub assignee: Option<String>,
    pub reviewer: Option<String>,
    pub labels: Vec<String>,
    pub milestone: Option<String>,
    pub search: Option<String>,
    pub commit: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub updated_after: Option<String>,
    pub updated_before: Option<String>,
    pub draft: Option<bool>,
    pub sort: Option<String>,
    pub descending: bool,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

/// One cursor-addressable page of pull requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullRequestPage {
    pub items: Vec<PullRequestRecord>,
    pub next: Option<String>,
    pub total: Option<usize>,
    pub warnings: Vec<String>,
}

/// Normalized issue information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IssueInfo {
    /// Issue number/ID.
    pub number: u64,
    /// Issue title.
    pub title: String,
    /// State: "open", "closed".
    pub state: String,
    /// Author login/username.
    pub author: String,
    /// Number of comments.
    pub comment_count: Option<u64>,
    /// Labels.
    pub labels: Vec<String>,
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: Option<String>,
    /// Close timestamp (None if still open).
    pub closed_at: Option<String>,
    /// HTML URL to the issue.
    pub html_url: String,
}

/// Tags and releases for a repository.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TagsAndReleases {
    /// All tags.
    pub tags: Vec<TagInfo>,
    /// All releases (subset of tags that have release metadata).
    pub releases: Vec<ReleaseInfo>,
}

/// Normalized tag information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TagInfo {
    /// Tag name (e.g., "v1.0.0").
    pub name: String,
    /// Commit SHA the tag points to.
    pub commit_sha: String,
    /// Whether this is an annotated tag (vs lightweight).
    pub annotated: bool,
    /// Tag message (annotated tags only).
    pub message: Option<String>,
    /// Tagger name (annotated tags only).
    pub tagger: Option<String>,
    /// Tagger date (annotated tags only).
    pub tagged_at: Option<String>,
}

/// Normalized release information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseInfo {
    /// Release name/title.
    pub name: Option<String>,
    /// Associated tag name.
    pub tag_name: String,
    /// Whether this is a draft release.
    pub draft: bool,
    /// Whether this is a pre-release.
    pub prerelease: bool,
    /// Publication timestamp.
    pub published_at: Option<String>,
    /// HTML URL to the release page.
    pub html_url: Option<String>,
}

/// CI/CD pipeline or workflow information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiCdInfo {
    /// CI/CD provider name (e.g., "GitHub Actions", "GitLab CI").
    pub provider: String,
    /// Path to the CI/CD configuration file.
    pub config_path: Option<String>,
    /// Workflow/pipeline name.
    pub name: String,
    /// Current status: "running", "completed", "failed", etc.
    pub status: String,
    /// Conclusion for completed runs.
    pub conclusion: Option<String>,
    /// HTML URL to the run/pipeline.
    pub html_url: Option<String>,
    /// When this run started.
    pub started_at: Option<String>,
    /// Branch this run was triggered from.
    pub head_branch: Option<String>,
    /// Event that triggered this run (e.g., "push", "pull_request").
    pub event: Option<String>,
}

/// Identity of the workflow run or pipeline containing a CI/CD job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CiCdParentExecution {
    pub native_id: String,
    pub display_id: String,
    pub name: Option<String>,
    pub web_url: Option<String>,
}

/// Repository-qualified identity of one provider-addressable CI/CD job.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CiCdJobReference {
    pub provider: GitProvider,
    pub api_flavor: String,
    pub host: String,
    pub namespace: String,
    pub repository: String,
    pub native_id: String,
    pub display_id: String,
    pub original_url: Option<String>,
}

/// One normalized CI/CD job, never a configuration or bare parent execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiCdJob {
    pub reference: CiCdJobReference,
    pub parent: CiCdParentExecution,
    pub name: String,
    pub stage: Option<String>,
    pub normalized_status: String,
    pub native_status: String,
    pub conclusion: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub actor: Option<String>,
    pub trigger: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub web_url: Option<String>,
    pub api_url: Option<String>,
    pub runner: Option<String>,
}

/// Provider-neutral CI/CD job query vocabulary.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CiCdJobQuery {
    pub statuses: Option<QueryValues<String>>,
    pub name: Option<String>,
    pub stage: Option<String>,
    pub workflow: Option<String>,
    pub parent: Option<String>,
    pub branch: Option<String>,
    pub commit: Option<String>,
    pub actor: Option<String>,
    pub trigger: Option<String>,
    pub created_after: Option<String>,
    pub created_before: Option<String>,
    pub updated_after: Option<String>,
    pub updated_before: Option<String>,
    pub descending: bool,
    pub limit: Option<usize>,
    pub cursor: Option<String>,
}

/// One cursor-addressable page of CI/CD jobs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CiCdJobPage {
    pub items: Vec<CiCdJob>,
    pub next: Option<String>,
    pub total: Option<usize>,
    pub warnings: Vec<String>,
}

/// Capabilities exposed by the normalized provider contract.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderCapabilities {
    pub pull_requests: bool,
    pub cicd_jobs: bool,
    pub pagination: bool,
    pub direct_job_listing: bool,
    pub bounded_parent_traversal: bool,
    pub logs: bool,
    pub artifacts: bool,
    pub test_reports: bool,
    pub pull_request_filters: Vec<String>,
    pub cicd_job_filters: Vec<String>,
}

/// A reference to another repository in the same organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgRepoRef {
    /// Organization name.
    pub org: String,
    /// Repository name.
    pub repo: String,
    /// Full name ("org/repo").
    pub full_name: String,
    /// Description.
    pub description: Option<String>,
    /// Primary language.
    pub language: Option<String>,
    /// Star count.
    pub stars: Option<u64>,
    /// HTML URL.
    pub html_url: String,
}

/// Key URLs for a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyUrls {
    /// Repository homepage on the hosting provider.
    pub repo: String,
    /// Homepage URL (user-configured, e.g., project website).
    pub homepage: Option<String>,
    /// Documentation URL.
    pub docs: Option<String>,
    /// Issues page URL.
    pub issues: Option<String>,
    /// Pull/merge requests page URL.
    pub pull_requests: Option<String>,
    /// Wiki URL (if wiki is enabled).
    pub wiki: Option<String>,
    /// CI/CD or Actions page URL.
    pub ci_cd: Option<String>,
    /// Insights/analytics page URL.
    pub insights: Option<String>,
    /// Releases page URL.
    pub releases: Option<String>,
    /// Settings page URL.
    pub settings: Option<String>,
}

/// Complete remote report containing all fetched data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteReport {
    /// Provider name (e.g., "GitHub").
    pub provider: GitProvider,
    /// Repository metadata.
    pub metadata: RepoMetadata,
    /// Organization/workspace info.
    pub org_info: Option<OrgInfo>,
    /// Documentation files discovered in the repository.
    pub documents: Vec<DocumentRef>,
    /// Pull/merge requests.
    pub pull_requests: Vec<PullRequestInfo>,
    /// Issues.
    pub issues: Vec<IssueInfo>,
    /// Tags and releases.
    pub tags_and_releases: TagsAndReleases,
    /// CI/CD pipelines/workflows.
    pub ci_cd: Vec<CiCdInfo>,
    /// Other repos in the same org.
    pub org_repos: Vec<OrgRepoRef>,
    /// Key URLs.
    pub key_urls: KeyUrls,
}
