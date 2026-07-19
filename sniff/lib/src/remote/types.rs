//! Normalized types for remote repository inspection.
//!
//! These types provide a provider-agnostic representation of repository
//! metadata, pull requests, issues, tags, releases, and other information
//! fetched from git hosting providers.

use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};
use serde::{Deserialize, Serialize};

use crate::SniffError;

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

/// The `state` vocabulary authored canonical queries are closed to.
///
/// This is deliberately narrower than [`PullRequestState`], which also carries
/// the `draft` and `all` selectors the Stage-1 report API and the `sniff repo`
/// CLI accept. Canonical queries express "draft" through the independent
/// `draft` boolean and "any state" by omitting `state`, so admitting those two
/// tokens here would create filter arms with no authored spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CanonicalPullRequestState {
    /// Open pull requests.
    Open,
    /// Closed pull requests that were not merged.
    Closed,
    /// Merged pull requests.
    Merged,
}

impl CanonicalPullRequestState {
    /// Returns the lowercase token authors write and Sniff serializes.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Closed => "closed",
            Self::Merged => "merged",
        }
    }
}

impl std::fmt::Display for CanonicalPullRequestState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl std::str::FromStr for CanonicalPullRequestState {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "open" => Ok(Self::Open),
            "closed" => Ok(Self::Closed),
            "merged" => Ok(Self::Merged),
            _ => Err(format!(
                "invalid PR state '{s}'. Expected one of: open, closed, merged"
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
///
/// `Default` is hand-written rather than derived because the ratified contract
/// makes newest-first the default order; a derived `descending: false` would
/// silently invert every unordered query, including the `#[serde(default)]`
/// fill-in used for absent keys.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct PullRequestQuery {
    pub state: Option<QueryValues<CanonicalPullRequestState>>,
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

impl Default for PullRequestQuery {
    fn default() -> Self {
        Self {
            state: None,
            source_branch: None,
            target_branch: None,
            author: None,
            assignee: None,
            reviewer: None,
            labels: Vec::new(),
            milestone: None,
            search: None,
            commit: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            draft: None,
            sort: None,
            descending: true,
            limit: None,
            cursor: None,
        }
    }
}

impl PullRequestQuery {
    /// Validates the flavor-independent canonical vocabulary.
    ///
    /// Darkmatter calls this before it resolves a repository or builds a
    /// client, and [`crate::remote::FocusedProviderClient::query_pull_requests`]
    /// calls it again before its first request, so no invalid canonical query
    /// can reach the network from either entry point.
    ///
    /// ## Errors
    ///
    /// Returns [`SniffError::InvalidRemoteQuery`] naming the offending field
    /// for an out-of-range limit, an empty `state` set, an unknown `sort`, a
    /// datetime that is not RFC 3339 / ISO 8601, or an inverted time range.
    pub fn validate_canonical(&self) -> Result<(), SniffError> {
        validate_limit(self.limit)?;
        if self.state.as_ref().is_some_and(|states| states.as_slice().is_empty()) {
            return Err(SniffError::InvalidRemoteQuery {
                field: "state",
                message: "must not be empty".to_string(),
            });
        }
        if self
            .sort
            .as_deref()
            .is_some_and(|sort| !matches!(sort, "created" | "updated" | "provider-default"))
        {
            return Err(SniffError::InvalidRemoteQuery {
                field: "sort",
                message: "expected created, updated, or provider-default".to_string(),
            });
        }
        validate_time_window(
            ("created_after", self.created_after.as_deref()),
            ("created_before", self.created_before.as_deref()),
        )?;
        validate_time_window(
            ("updated_after", self.updated_after.as_deref()),
            ("updated_before", self.updated_before.as_deref()),
        )
    }
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
    /// When the runner picked the job up, distinct from when it was enqueued.
    ///
    /// Every supported provider reports queueing and execution start as separate
    /// instants (`created_at`/`started_at`, `created_on`/`started_on`), so
    /// collapsing them would hide the queue wait the spec's timestamp set exists
    /// to expose.
    pub started_at: Option<String>,
    /// When the job reached a terminal state (`finished_at`/`completed_at`/`completed_on`).
    pub finished_at: Option<String>,
    /// Last-modification instant, or the terminal instant on providers that
    /// expose no separate one. This is what the `updated_*` filters compare.
    pub updated_at: Option<String>,
    pub web_url: Option<String>,
    pub api_url: Option<String>,
    pub runner: Option<String>,
}

/// Normalized CI/CD lifecycle states a canonical query may select.
pub const CICD_JOB_STATUSES: &[&str] = &[
    "queued", "running", "success", "failed", "cancelled", "skipped", "manual",
];

/// Provider-neutral CI/CD job query vocabulary.
///
/// `Default` is hand-written for the same reason as [`PullRequestQuery`]:
/// newest-first is the ratified default order.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CiCdJobQuery {
    pub statuses: Option<QueryValues<String>>,
    pub name: Option<String>,
    pub stage: Option<String>,
    pub workflow: Option<String>,
    /// Parent workflow-run/pipeline identity, normalized to its string form.
    ///
    /// Providers spell this as an integer (GitHub, GitLab, Gitea) or an opaque
    /// UUID (Bitbucket), so authors may write either and both land here as the
    /// string the job matcher compares against.
    #[serde(default, deserialize_with = "deserialize_parent_identity")]
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

impl Default for CiCdJobQuery {
    fn default() -> Self {
        Self {
            statuses: None,
            name: None,
            stage: None,
            workflow: None,
            parent: None,
            branch: None,
            commit: None,
            actor: None,
            trigger: None,
            created_after: None,
            created_before: None,
            updated_after: None,
            updated_before: None,
            descending: true,
            limit: None,
            cursor: None,
        }
    }
}

impl CiCdJobQuery {
    /// Validates the flavor-independent canonical vocabulary.
    ///
    /// See [`PullRequestQuery::validate_canonical`] for why both Darkmatter and
    /// the focused client call this.
    ///
    /// ## Errors
    ///
    /// Returns [`SniffError::InvalidRemoteQuery`] naming the offending field
    /// for an out-of-range limit, an empty or unrecognized `statuses` set, a
    /// datetime that is not RFC 3339 / ISO 8601, or an inverted time range.
    pub fn validate_canonical(&self) -> Result<(), SniffError> {
        validate_limit(self.limit)?;
        if let Some(statuses) = &self.statuses
            && (statuses.as_slice().is_empty()
                || statuses
                    .as_slice()
                    .iter()
                    .any(|status| !CICD_JOB_STATUSES.contains(&status.as_str())))
        {
            return Err(SniffError::InvalidRemoteQuery {
                field: "statuses",
                message: format!("expected one or more of: {}", CICD_JOB_STATUSES.join(", ")),
            });
        }
        validate_time_window(
            ("created_after", self.created_after.as_deref()),
            ("created_before", self.created_before.as_deref()),
        )?;
        validate_time_window(
            ("updated_after", self.updated_after.as_deref()),
            ("updated_before", self.updated_before.as_deref()),
        )
    }
}

fn deserialize_parent_identity<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum ParentIdentity {
        Text(String),
        Number(u64),
    }

    Ok(
        Option::<ParentIdentity>::deserialize(deserializer)?.map(|identity| match identity {
            ParentIdentity::Text(text) => text,
            ParentIdentity::Number(number) => number.to_string(),
        }),
    )
}

fn validate_limit(limit: Option<usize>) -> Result<(), SniffError> {
    if matches!(limit, Some(0 | 101..)) {
        Err(SniffError::InvalidRemoteQuery {
            field: "limit",
            message: "must be between 1 and 100".to_string(),
        })
    } else {
        Ok(())
    }
}

fn validate_time_window(
    after: (&'static str, Option<&str>),
    before: (&'static str, Option<&str>),
) -> Result<(), SniffError> {
    let after_instant = validate_timestamp(after.0, after.1)?;
    let before_instant = validate_timestamp(before.0, before.1)?;
    if after_instant
        .zip(before_instant)
        .is_some_and(|(after, before)| after > before)
    {
        return Err(SniffError::InvalidRemoteQuery {
            field: after.0,
            message: "range is inverted".to_string(),
        });
    }
    Ok(())
}

fn validate_timestamp(
    field: &'static str,
    value: Option<&str>,
) -> Result<Option<DateTime<FixedOffset>>, SniffError> {
    value
        .map(|value| {
            parse_query_timestamp(value).ok_or_else(|| SniffError::InvalidRemoteQuery {
                field,
                message: format!(
                    "`{value}` is not an RFC 3339 timestamp, `YYYY-MM-DDTHH:MM:SS`, or `YYYY-MM-DD`"
                ),
            })
        })
        .transpose()
}

/// Parses an authored or provider-supplied timestamp to an absolute instant.
///
/// Offset-bearing RFC 3339 is the canonical spelling; a bare datetime or date
/// is accepted and read as UTC so that an authored `2026-07-01` window bound
/// does not have to carry a zone.
///
/// ## Notes
///
/// Comparing the parsed instants rather than the source strings is what makes
/// `2026-07-01T00:00:00Z` correctly later than `2026-06-30T20:00:00-04:00`;
/// those two spellings order the wrong way lexically.
fn parse_query_timestamp(value: &str) -> Option<DateTime<FixedOffset>> {
    if let Ok(instant) = DateTime::parse_from_rfc3339(value) {
        return Some(instant);
    }
    let naive = NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| {
            NaiveDate::parse_from_str(value, "%Y-%m-%d").map(|date| date.and_hms_opt(0, 0, 0).expect("midnight is always valid"))
        })
        .ok()?;
    Some(Utc.from_utc_datetime(&naive).fixed_offset())
}

/// Orders an item timestamp against a validated query bound.
///
/// Query bounds are already known to parse, but provider payloads are not, so
/// an unparseable pair falls back to byte order rather than silently dropping
/// the row from an otherwise complete result domain.
pub(crate) fn timestamp_order(item: &str, bound: &str) -> std::cmp::Ordering {
    match (parse_query_timestamp(item), parse_query_timestamp(bound)) {
        (Some(item), Some(bound)) => item.cmp(&bound),
        _ => item.cmp(bound),
    }
}

/// Orders two optional timestamps, ranking an absent one first.
pub(crate) fn optional_timestamp_order(
    left: Option<&str>,
    right: Option<&str>,
) -> std::cmp::Ordering {
    match (left, right) {
        (Some(left), Some(right)) => timestamp_order(left, right),
        (left, right) => left.is_some().cmp(&right.is_some()),
    }
}

pub(crate) fn at_or_after(item: &str, bound: &str) -> bool {
    timestamp_order(item, bound).is_ge()
}

pub(crate) fn at_or_before(item: &str, bound: &str) -> bool {
    timestamp_order(item, bound).is_le()
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
