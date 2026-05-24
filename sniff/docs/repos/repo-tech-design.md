# Repo Remotes — Technical Design

> Implements the functional requirements from [repo-remotes-functional-design.md](./repo-remotes-functional-design.md).

## Overview

This design adds remote repository inspection to the sniff library. It uses the generated API clients in `schematic/schema` to fetch repository metadata, PRs, issues, documentation, tags/releases, CI/CD, and more from GitHub, GitLab, Gitea, and Bitbucket.

The design follows a **trait-based provider abstraction** so that consumer code (both the library API and the CLI) can work uniformly across all four providers while still permitting provider-specific extensions.

## Scope

### Stage 1 (this design)

| Provider | API Client | Auth Env Vars |
|----------|-----------|---------------|
| GitHub | `schematic_schema::GitHub` | `GITHUB_TOKEN` (Bearer) |
| GitLab | `schematic_schema::GitLab` | `GITLAB_TOKEN` (Bearer via `PRIVATE-TOKEN` header) |
| Gitea | `schematic_schema::Gitea` | `GITEA_TOKEN` (ApiKey `Authorization` header) |
| Bitbucket | `schematic_schema::Bitbucket` | `BITBUCKET_USERNAME` + `BITBUCKET_APP_PASSWORD` (Basic) |

### Stage 2 (future)

- AWS CodeCommit
- Azure DevOps

---

## Available Schematic Endpoints

These are the generated API client endpoints available today. The design maps each functional requirement to specific endpoints.

### GitHub (14 endpoints)

| Endpoint | Returns |
|----------|---------|
| `GetRepository` | `RepositoryInfo` — stars, forks, license, topics, wiki flag, default branch |
| `GetGitTree` / `GetGitTreeRecursive` | `GitTreeResponse` — file tree traversal |
| `GetRepositoryContentRaw` | `String` — raw file content (README, LICENSE) |
| `ListPullRequests` | `Vec<PullRequestSummary>` |
| `ListPullRequestFiles` | `Vec<PullRequestFile>` |
| `ListIssues` | `Vec<IssueSummary>` |
| `GetIssue` | `IssueSummary` |
| `ListIssueComments` | `Vec<IssueComment>` |
| `ListIssueTimeline` | `Vec<TimelineEvent>` |
| `ListTags` | `Vec<RepoTag>` |
| `ListReleases` | `Vec<Release>` |
| `GetTagReference` | `Vec<GitRef>` |
| `GetAnnotatedTag` | `AnnotatedTagObject` |

### GitLab (15 endpoints)

| Endpoint | Returns |
|----------|---------|
| `ListRepositoryTree` | `Vec<TreeItem>` — file tree |
| `GetRepositoryFile` | `FileContent` — base64 encoded file |
| `ListMergeRequests` | `Vec<MergeRequest>` |
| `GetMergeRequest` | `MergeRequest` |
| `ListMergeRequestCommits` | `Vec<Commit>` |
| `ListMergeRequestChanges` | `MergeRequestChanges` |
| `ListIssues` | `Vec<Issue>` |
| `GetIssue` | `Issue` |
| `ListIssueNotes` | `Vec<Note>` |
| `ListIssueParticipants` | `Vec<User>` |
| `ListTags` | `Vec<Tag>` |
| `GetTag` | `Tag` |
| `ListReleases` | `Vec<Release>` |
| `GetRelease` | `Release` |
| `GetLatestRelease` | `Release` |

### Gitea (14 endpoints)

| Endpoint | Returns |
|----------|---------|
| `GetRepository` | `RepositoryInfo` — stars, forks, default branch |
| `GetGitTree` / `GetGitTreeRecursive` | `GitTreeResponse` — file tree |
| `GetRepositoryContentRaw` | `String` — raw file content |
| `ListPullRequests` | `Vec<PullRequestSummary>` |
| `ListPullRequestFiles` | `Vec<PullRequestFile>` |
| `ListIssues` | `Vec<IssueSummary>` |
| `GetIssue` | `IssueSummary` |
| `ListIssueComments` | `Vec<IssueComment>` |
| `ListIssueTimeline` | `Vec<TimelineEvent>` |
| `ListTags` | `Vec<RepoTag>` |
| `ListReleases` | `Vec<Release>` |
| `GetTagReference` | `Vec<GitRef>` |
| `GetAnnotatedTag` | `AnnotatedTagObject` |

### Bitbucket (14 endpoints)

| Endpoint | Returns |
|----------|---------|
| `GetRepository` | `Repository` — workspace, project, default branch |
| `ListDirectoryContents` | `PaginatedResponse<SourceEntry>` — file tree |
| `GetFileContentRaw` | `String` — raw file content |
| `ListPullRequests` | `PaginatedResponse<PullRequest>` |
| `GetPullRequest` | `PullRequest` |
| `ListPullRequestComments` | `PaginatedResponse<PullRequestComment>` |
| `ListIssues` | `PaginatedResponse<Issue>` |
| `GetIssue` | `Issue` |
| `ListIssueComments` | `PaginatedResponse<IssueComment>` |
| `ListIssueChanges` | `PaginatedResponse<IssueChange>` |
| `ListTags` | `PaginatedResponse<Tag>` |
| `GetTag` | `Tag` |
| `ListDownloads` | `PaginatedResponse<Download>` |
| `GetDownload` | `Bytes` (binary) |

---

## Module Architecture

### New File Layout

```
sniff/lib/src/
├── filesystem/
│   ├── git.rs                    # Existing — local git detection
│   ├── repo.rs                   # Existing — monorepo/package detection
│   └── ...
├── remote/                       # NEW — remote repo module
│   ├── mod.rs                    # Re-exports, RemoteRepoProvider trait
│   ├── types.rs                  # Normalized types (GitRemoteInfo enum, provider-agnostic structs)
│   ├── github.rs                 # GitHubRemote impl
│   ├── gitlab.rs                 # GitLabRemote impl
│   ├── gitea.rs                  # GiteaRemote impl
│   ├── bitbucket.rs              # BitbucketRemote impl
│   └── url_parser.rs             # Remote URL → (provider, owner, repo) extraction
└── lib.rs                        # Add `pub mod remote`
```

### Dependency Changes

```toml
# sniff/lib/Cargo.toml additions
[dependencies]
schematic-schema = { path = "../../schematic/schema", optional = true }

[features]
default = []
network = ["dep:reqwest", "dep:tokio", "dep:futures", "reqwest/rustls-tls"]
remote = ["network", "dep:schematic-schema"]
```

The `remote` feature gate keeps the new functionality opt-in and avoids pulling in schematic dependencies for consumers who only need local detection. The `remote` feature implies `network` since all remote operations require HTTP.

---

## Core Types

### `RemoteRepoProvider` Trait

The central abstraction. Each provider implements this trait using its schematic-generated client.

```rust
// sniff/lib/src/remote/mod.rs

use crate::error::SniffError;

/// Trait for querying remote git hosting providers.
///
/// Each method returns `Option`-wrapped results to support providers that
/// may not offer certain features (e.g., Bitbucket issues are optional).
#[async_trait::async_trait]
pub trait RemoteRepoProvider: Send + Sync {
    /// Provider name for display purposes (e.g., "GitHub", "GitLab").
    fn provider_name(&self) -> &str;

    /// Repository metadata (stars, forks, license, description, etc.).
    async fn repo_metadata(&self) -> Result<RepoMetadata, SniffError>;

    /// Organization or workspace information.
    async fn organization(&self) -> Result<OrgInfo, SniffError>;

    /// List documentation files (markdown/text) in the repository.
    async fn list_documents(&self) -> Result<Vec<DocumentRef>, SniffError>;

    /// Fetch raw content of a specific file path.
    async fn get_file_content(&self, path: &str) -> Result<String, SniffError>;

    /// List pull/merge requests with metadata.
    async fn list_pull_requests(&self) -> Result<Vec<PullRequestInfo>, SniffError>;

    /// List issues with metadata.
    async fn list_issues(&self) -> Result<Vec<IssueInfo>, SniffError>;

    /// List tags and releases.
    async fn list_tags_and_releases(&self) -> Result<TagsAndReleases, SniffError>;

    /// Check if the repository has a wiki.
    async fn has_wiki(&self) -> Result<bool, SniffError>;

    /// List CI/CD workflows or pipelines.
    async fn list_ci_cd(&self) -> Result<Vec<CiCdInfo>, SniffError>;

    /// List other repositories under the same organization/workspace.
    async fn list_org_repos(&self) -> Result<Vec<OrgRepoRef>, SniffError>;

    /// Key URLs for the repository.
    async fn key_urls(&self) -> Result<KeyUrls, SniffError>;
}
```

### `GitRemoteInfo` Enum

The top-level enum from the functional design, wrapping provider-specific implementations.

```rust
// sniff/lib/src/remote/types.rs

/// A remote repository info handle, dispatching to the appropriate provider.
///
/// Created via `GitRemoteInfo::from_remote()` using a `RemoteInfo` from
/// local git detection, or `GitRemoteInfo::from_url()` for standalone use.
pub enum GitRemoteInfo {
    GitHub(GitHubRemote),
    GitLab(GitLabRemote),
    Gitea(GiteaRemote),
    Bitbucket(BitbucketRemote),
}

impl GitRemoteInfo {
    /// Create from an existing `RemoteInfo` (as detected by local git inspection).
    ///
    /// Returns `None` if the provider is not supported (e.g., SourceHut, SelfHosted)
    /// or if the URL cannot be parsed into owner/repo.
    pub fn from_remote(remote: &RemoteInfo) -> Option<Self> { ... }

    /// Create from a raw URL string.
    ///
    /// Detects the provider and extracts owner/repo from the URL.
    pub fn from_url(url: &str) -> Option<Self> { ... }

    /// Create from explicit provider, owner, and repo.
    ///
    /// For Gitea/Forgejo, also requires a base URL since these are self-hosted.
    pub fn new(
        provider: HostingProvider,
        owner: String,
        repo: String,
        base_url: Option<String>,
    ) -> Option<Self> { ... }
}
```

`GitRemoteInfo` implements `RemoteRepoProvider` by delegating to the inner variant.

### Normalized Output Types

Provider-agnostic types that each provider maps its API responses into.

```rust
// sniff/lib/src/remote/types.rs

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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    /// Creation timestamp.
    pub created_at: String,
    /// Last update timestamp.
    pub updated_at: Option<String>,
    /// Merge timestamp (None if not merged).
    pub merged_at: Option<String>,
    /// HTML URL to the PR.
    pub html_url: String,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
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
}

/// A reference to another repository in the same organization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgRepoRef {
    /// Repository name.
    pub name: String,
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
```

---

## Provider Implementations

### Common Pattern

Each provider module follows the same structure:

```rust
// sniff/lib/src/remote/github.rs (pattern representative of all providers)

use schematic_schema::GitHub;
use super::{RemoteRepoProvider, types::*};

/// GitHub remote repository provider.
///
/// Uses the schematic-generated `GitHub` API client for all requests.
pub struct GitHubRemote {
    client: GitHub,
    owner: String,
    repo: String,
}

impl GitHubRemote {
    /// Create a new GitHub remote handle.
    ///
    /// The client reads `GITHUB_TOKEN` from the environment for authentication.
    /// Unauthenticated requests are supported but subject to lower rate limits.
    pub fn new(owner: String, repo: String) -> Result<Self, SniffError> {
        let client = GitHub::new().map_err(|e| SniffError::RemoteInit(e.to_string()))?;
        Ok(Self { client, owner, repo })
    }
}
```

### Provider-Specific Mapping Notes

#### GitHub

- `repo_metadata()` → `GetRepository` request, map `RepositoryInfo` → `RepoMetadata`
- `list_documents()` → `GetGitTreeRecursive` with the default branch, filter for `.md`/`.txt` files
- `get_file_content()` → `GetRepositoryContentRaw` request
- `list_pull_requests()` → `ListPullRequests` request, map `PullRequestSummary` → `PullRequestInfo`
- `list_issues()` → `ListIssues` request, filter out entries where `pull_request` field is `Some`, map `IssueSummary` → `IssueInfo`
- `list_tags_and_releases()` → `ListTags` + `ListReleases` requests in parallel, join by `tag_name`
- `has_wiki()` → Read `has_wiki` field from `GetRepository` response
- `list_ci_cd()` → Not currently covered by schematic endpoints; return empty vec with a note (future: add GitHub Actions endpoint to schematic)
- `key_urls()` → Construct from `html_url` pattern (`{html_url}/issues`, `{html_url}/pulls`, etc.)
- `organization()` → Parse from `owner` field of `RepositoryInfo`
- `list_org_repos()` → Not currently covered by schematic endpoints; return empty vec (future: add org repos endpoint to schematic)

#### GitLab

- `repo_metadata()` → No direct `GetRepository` endpoint currently; extract from `ListMergeRequests` or `ListIssues` responses. **Note: schematic needs a `GetProject` endpoint added.**
- `list_documents()` → `ListRepositoryTree` with `recursive=true`, filter for `.md`/`.txt`
- `get_file_content()` → `GetRepositoryFile`, decode from base64
- `list_pull_requests()` → `ListMergeRequests`, map `MergeRequest` → `PullRequestInfo`
- `list_issues()` → `ListIssues`, map `Issue` → `IssueInfo`
- `list_tags_and_releases()` → `ListTags` + `ListReleases` in parallel
- `has_wiki()` → Requires `GetProject` endpoint (see gap below)
- `list_ci_cd()` → Not currently covered; return empty vec (future: add pipelines endpoint)
- `key_urls()` → Construct from project URL pattern

#### Gitea

- `repo_metadata()` → `GetRepository`, map `RepositoryInfo` → `RepoMetadata`
- `list_documents()` → `GetGitTreeRecursive`, filter for docs
- `get_file_content()` → `GetRepositoryContentRaw`
- `list_pull_requests()` → `ListPullRequests`, map `PullRequestSummary` → `PullRequestInfo`
- `list_issues()` → `ListIssues` (already filtered by `type=issues` in endpoint path)
- `list_tags_and_releases()` → `ListTags` + `ListReleases` in parallel
- `has_wiki()` → Read from `GetRepository` response (Gitea `RepositoryInfo` includes `has_wiki`)
- `list_ci_cd()` → Not currently covered; return empty vec
- `key_urls()` → Construct from `html_url` pattern
- **Base URL consideration**: Gitea instances are self-hosted, so the client must be configured with `variant()` to set the correct base URL.

#### Bitbucket

- `repo_metadata()` → `GetRepository`, map `Repository` → `RepoMetadata`
- `list_documents()` → `ListDirectoryContents` recursively, filter for docs
- `get_file_content()` → `GetFileContentRaw`
- `list_pull_requests()` → `ListPullRequests`, map `PullRequest` → `PullRequestInfo`
- `list_issues()` → `ListIssues`, map `Issue` → `IssueInfo`
- `list_tags_and_releases()` → `ListTags` + `ListDownloads` (Bitbucket uses "downloads" instead of GitHub-style releases)
- `has_wiki()` → Read from `GetRepository` response (`has_wiki` field)
- `list_ci_cd()` → Not currently covered (Bitbucket Pipelines); return empty vec
- `key_urls()` → Construct from `links` fields in `Repository` response
- `organization()` → Use `workspace` from `Repository` response
- **Pagination**: Bitbucket uses cursor-based `PaginatedResponse<T>` with a `next` URL. Provider impl must handle pagination by following `next` URLs.

---


## Integration with Existing Sniff

### Library API

```rust
// sniff/lib/src/remote/mod.rs — public API

/// Detect and create a remote provider from a RemoteInfo.
///
/// Returns `None` if the provider is unsupported or URL can't be parsed.
#[cfg(feature = "remote")]
pub fn remote_provider(remote: &RemoteInfo) -> Option<GitRemoteInfo> {
    GitRemoteInfo::from_remote(remote)
}

/// Fetch a comprehensive remote report for a remote provider.
///
/// Runs repo_metadata, list_pull_requests, list_issues, list_tags_and_releases,
/// and key_urls in parallel using tokio::join!.
#[cfg(feature = "remote")]
pub async fn fetch_remote_report(
    provider: &dyn RemoteRepoProvider,
) -> Result<RemoteReport, SniffError> {
    let (metadata, prs, issues, tags_releases, urls) = tokio::join!(
        provider.repo_metadata(),
        provider.list_pull_requests(),
        provider.list_issues(),
        provider.list_tags_and_releases(),
        provider.key_urls(),
    );

    Ok(RemoteReport {
        provider: provider.provider_name().to_string(),
        metadata: metadata?,
        pull_requests: prs?,
        issues: issues?,
        tags_and_releases: tags_releases?,
        key_urls: urls?,
    })
}

/// Complete remote report containing all fetched data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteReport {
    /// Provider name (e.g., "GitHub").
    pub provider: String,
    /// Repository metadata.
    pub metadata: RepoMetadata,
    /// Pull/merge requests.
    pub pull_requests: Vec<PullRequestInfo>,
    /// Issues.
    pub issues: Vec<IssueInfo>,
    /// Tags and releases.
    pub tags_and_releases: TagsAndReleases,
    /// Key URLs.
    pub key_urls: KeyUrls,
}
```

### Extended `RemoteInfo`

The existing `RemoteInfo` struct gains an optional `remote_report` field:

```rust
// sniff/lib/src/filesystem/git.rs — extension

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInfo {
    pub name: String,
    pub url: Option<String>,
    pub provider: HostingProvider,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branches: Option<Vec<String>>,

    /// Remote repository report (populated when using remote feature + deep mode).
    #[cfg(feature = "remote")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote_report: Option<RemoteReport>,
}
```

Alternatively — and preferably for separation of concerns — the `RemoteReport` lives alongside `GitInfo` rather than nested inside `RemoteInfo`:

```rust
// sniff/lib/src/lib.rs — add to SniffResult

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SniffResult {
    pub os: Option<OsInfo>,
    pub hardware: Option<HardwareInfo>,
    pub network: Option<NetworkInfo>,
    pub filesystem: Option<FilesystemInfo>,

    /// Remote repository report (populated with --remote or --deep flag).
    #[cfg(feature = "remote")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remote: Option<RemoteReport>,
}
```

**Decision**: Use the `SniffResult`-level approach. This keeps `RemoteInfo` lightweight (it's a local-detection struct) and puts the remote report at the top level where it naturally belongs as a separate concern.

### CLI Changes

```rust
// sniff/cli/src/main.rs — additions

/// Show only repository/monorepo structure
Repo {
    /// Render an internal dependency diagram
    #[arg(long)]
    deps: bool,

    /// Query remote repository info from the hosting provider API.
    /// If specified, shows remote info instead of local repo structure.
    /// Value is the remote name (e.g., "origin") or a URL.
    #[arg(long)]
    remote: Option<String>,
},

/// Show remote repository info (standalone, without local repo context).
Remote {
    /// Repository URL or "owner/repo" shorthand (assumes GitHub).
    repo: String,

    /// Hosting provider override (github, gitlab, gitea, bitbucket).
    /// Auto-detected from URL if not specified.
    #[arg(long)]
    provider: Option<String>,

    /// Base URL for self-hosted instances (required for Gitea/Forgejo).
    #[arg(long)]
    base_url: Option<String>,
},
```

#### CLI Usage Examples

```bash
# Local repo, supplement with remote data
sniff git --deep

# Query remote info for the "origin" remote
sniff git origin

# Query remote info for a specific URL
sniff git https://github.com/rust-lang/cargo
# Query cloud git repo using just org/repo name
# note: this will query Github, Gitlab, Bitbucket, and Gitea (if Gitea_URL env is set) in parallel
# and in most cases just resolve to one provider. If we actually resolve to more than one provider
# we will list all that have the org/repo combo with a warning message at the end that more than
# one match was found.
sniff git rust-lang/cargo

# JSON output
sniff git origin --json
```

---

## Error Handling

### New Error Variants

```rust
// sniff/lib/src/error.rs — additions

#[derive(Debug, thiserror::Error)]
pub enum SniffError {
    // ... existing variants ...

    /// Failed to initialize a remote provider client.
    #[error("remote provider initialization failed: {0}")]
    RemoteInit(String),

    /// Remote API request failed.
    #[error("remote API request failed: {0}")]
    RemoteApi(String),

    /// Unsupported hosting provider for remote queries.
    #[error("unsupported hosting provider: {0}")]
    UnsupportedProvider(String),

    /// Authentication credentials not configured for provider.
    #[error("missing credentials for {provider}: set {env_vars}")]
    MissingCredentials {
        provider: String,
        env_vars: String,
    },

    /// Rate limited by the hosting provider API.
    #[error("rate limited by {provider} API (retry after {retry_after:?}s)")]
    RateLimited {
        provider: String,
        retry_after: Option<u64>,
    },
}
```

### Graceful Degradation

All remote operations should degrade gracefully:

1. **Missing credentials**: Return `Err(MissingCredentials { .. })` — the CLI can display which env vars to set.
2. **Rate limiting**: Return `Err(RateLimited { .. })` — the CLI can suggest waiting.
3. **404 Not Found**: Map to `Err(RemoteApi("repository not found or private"))` — could be auth or genuinely missing.
4. **Network failure**: Map to `Err(RemoteApi(...))` with the underlying error message.
5. **Partial data in `--deep` mode**: When `--deep` is used on the `repo` subcommand (not `--remote`), remote failures should log a warning and continue with local-only data. The `remote` field in `SniffResult` stays `None`.

---

## Concurrency Strategy

### Parallel Provider Queries

Within a single provider, multiple independent API calls are executed concurrently:

```rust
// Example: GitHub repo_metadata + list_pull_requests + list_issues
let (metadata_res, prs_res, issues_res, tags_res, releases_res) = tokio::join!(
    self.client.request::<RepositoryInfo>(GetRepositoryRequest { owner, repo }),
    self.client.request::<Vec<PullRequestSummary>>(ListPullRequestsRequest { owner, repo }),
    self.client.request::<Vec<IssueSummary>>(ListIssuesRequest { owner, repo }),
    self.client.request::<Vec<RepoTag>>(ListTagsRequest { owner, repo }),
    self.client.request::<Vec<Release>>(ListReleasesRequest { owner, repo }),
);
```

### Rate Limit Awareness

- GitHub: 5000 req/hr authenticated, 60 req/hr unauthenticated
- GitLab: 2000 req/hr with PAT
- Gitea: Configurable per instance
- Bitbucket: 1000 req/hr with app password

The parallel request pattern is safe because a single `fetch_remote_report` call makes approximately 5-8 API calls, well within all rate limits. No batching or throttling is needed for single-repo queries.

---

## Gitea Base URL Resolution

Gitea (and Forgejo) instances are self-hosted, so the base URL cannot be hardcoded. Resolution strategy:

1. **From remote URL**: Extract the host from the git remote URL (e.g., `https://gitea.example.com/org/repo` → `https://gitea.example.com/api/v1`)
2. **From CLI flag**: `--base-url` explicitly sets the API base
3. **Environment variable**: `GITEA_BASE_URL` as a fallback

```rust
impl GiteaRemote {
    pub fn new(owner: String, repo: String, base_url: String) -> Result<Self, SniffError> {
        let client = Gitea::new()
            .map_err(|e| SniffError::RemoteInit(e.to_string()))?
            .variant()
            .base_url(&base_url)
            .build();
        Ok(Self { client, owner, repo })
    }
}
```

The `url_parser` module handles extracting the base URL:

```rust
// sniff/lib/src/remote/url_parser.rs

/// Parse a git remote URL into (provider, owner, repo, base_url).
///
/// For well-known providers (GitHub, GitLab, Bitbucket), the base URL is
/// hardcoded. For Gitea/Forgejo, it's extracted from the remote URL host.
pub fn parse_remote_url(url: &str) -> Option<ParsedRemote> { ... }

pub struct ParsedRemote {
    pub provider: HostingProvider,
    pub owner: String,
    pub repo: String,
    /// API base URL (only relevant for self-hosted providers like Gitea).
    pub base_url: Option<String>,
}
```

---

## Bitbucket Pagination

Bitbucket's paginated responses differ from other providers. The implementation needs a pagination helper:

```rust
/// Fetch all pages of a Bitbucket paginated endpoint.
///
/// Follows the `next` URL until no more pages are available.
async fn fetch_all_pages<T: DeserializeOwned>(
    client: &Bitbucket,
    initial_response: PaginatedResponse<T>,
) -> Result<Vec<T>, SniffError> {
    let mut all_values = initial_response.values;
    let mut next_url = initial_response.next;

    while let Some(url) = next_url {
        // Use the client to fetch the next page URL directly
        // This requires raw URL support from schematic or a reqwest fallback
        let page: PaginatedResponse<T> = fetch_url(client, &url).await?;
        all_values.extend(page.values);
        next_url = page.next;
    }

    Ok(all_values)
}
```

**Note**: Schematic-generated clients currently call specific endpoint paths. Pagination follow-up requires either:
- A `request_url()` method on the client (preferred — should be added to schematic), or
- Direct `reqwest` usage with the same auth headers

This is a known gap to address when implementing the Bitbucket provider.

---

## Document Discovery

The `list_documents()` method categorizes files discovered via tree traversal:

```rust
fn categorize_document(path: &str) -> Option<DocumentCategory> {
    let lower = path.to_lowercase();
    let is_doc = lower.ends_with(".md")
        || lower.ends_with(".txt")
        || lower.ends_with(".rst")
        || lower.ends_with(".adoc");

    if !is_doc {
        return None;
    }

    let filename = path.rsplit('/').next().unwrap_or(path);
    if filename.eq_ignore_ascii_case("readme.md") {
        Some(DocumentCategory::Readme)
    } else if path.starts_with("src/") || path.contains("/src/") {
        Some(DocumentCategory::SourceDoc)
    } else if path.starts_with("docs/") || path.starts_with("doc/")
        || path.contains("/docs/") || path.contains("/doc/")
    {
        Some(DocumentCategory::DocsFolder)
    } else {
        Some(DocumentCategory::Other)
    }
}
```

---

## Testing Strategy

### Unit Tests

- **URL parsing**: Test `parse_remote_url()` for all provider URL formats (HTTPS, SSH, with/without `.git`)
- **Type mapping**: Test conversion from provider-specific types to normalized types
- **Document categorization**: Test `categorize_document()` with various paths

### Integration Tests (with wiremock)

Each provider gets a set of wiremock-based tests:

```rust
#[cfg(test)]
mod tests {
    use wiremock::{MockServer, Mock, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn github_repo_metadata() {
        let mock_server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/repos/rust-lang/cargo"))
            .respond_with(ResponseTemplate::new(200).set_body_json(/* fixture */))
            .mount(&mock_server)
            .await;

        let remote = GitHubRemote::with_base_url(
            "rust-lang".into(),
            "cargo".into(),
            mock_server.uri(),
        );

        let metadata = remote.repo_metadata().await.unwrap();
        assert_eq!(metadata.name, "cargo");
        assert_eq!(metadata.stars, Some(12000));
    }
}
```

Each provider needs a `with_base_url()` constructor (or use schematic's `variant()`) to point at the mock server.

### Test Fixtures

Store JSON fixtures in `sniff/lib/tests/fixtures/remote/`:

```
sniff/lib/tests/fixtures/remote/
├── github/
│   ├── get_repository.json
│   ├── list_pull_requests.json
│   ├── list_issues.json
│   ├── list_tags.json
│   ├── list_releases.json
│   └── get_git_tree_recursive.json
├── gitlab/
│   ├── list_merge_requests.json
│   ├── list_issues.json
│   └── ...
├── gitea/
│   └── ...
└── bitbucket/
    └── ...
```

---

## Implementation Order

### Phase 1: Foundation

1. Create `sniff/lib/src/remote/mod.rs` with `RemoteRepoProvider` trait
2. Create `sniff/lib/src/remote/types.rs` with all normalized types
3. Create `sniff/lib/src/remote/url_parser.rs` with URL parsing
4. Add `remote` feature to `Cargo.toml`
5. Create `GitRemoteInfo` enum with `from_remote()`, `from_url()`, `new()`

### Phase 2: GitHub Provider

6. Implement `GitHubRemote` in `sniff/lib/src/remote/github.rs`
7. Write unit tests for type mapping
8. Write wiremock integration tests
9. Add CLI `--remote` flag to `repo` subcommand

### Phase 3: Remaining Providers

10. Implement `GitLabRemote` (add `GetProject` endpoint to schematic first)
11. Implement `GiteaRemote` (including base URL resolution)
12. Implement `BitbucketRemote` (including pagination helper)
13. Wiremock tests for each

### Phase 4: CLI and Integration

14. Add standalone `Remote` subcommand to CLI
15. Integrate `RemoteReport` into `SniffResult` for `--deep` mode
16. Add output formatting for remote data in `output/filesystem.rs`
17. Update sniff skill docs

### Phase 5: Missing Endpoints

18. Add CI/CD endpoints to schematic (GitHub Actions, GitLab Pipelines)
19. Add org/workspace repos endpoints to schematic
20. Add GitLab `GetProject` endpoint to schematic
21. Wire up `list_ci_cd()` and `list_org_repos()` implementations
