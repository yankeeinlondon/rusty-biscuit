//! Remote repository inspection via git hosting providers.
//!
//! This module provides normalized types and traits for querying remote git
//! hosting providers (GitHub, GitLab, Gitea, Bitbucket). It uses the generated
//! API clients in `schematic-schema` for all HTTP operations.
//!
//! ## Feature Flag
//!
//! This module is gated behind the `remote` feature flag in `Cargo.toml`.
//! Enable it with:
//!
//! ```toml
//! [dependencies]
//! sniff = { version = "...", features = ["remote"] }
//! ```
//!
//! ## Usage
//!
//! The primary interface is the [`RemoteRepoProvider`] trait. Each provider
//! (GitHub, GitLab, Gitea, Bitbucket) implements this trait.
//!
//! ### Direct Provider Construction
//!
//! ```ignore
//! use sniff::remote::{RemoteRepoProvider, GitHubRemote};
//!
//! let provider = GitHubRemote::new()?;
//! let report = provider.fetch_report("rust-lang", "cargo").await?;
//! println!("Stars: {:?}", report.metadata.stars);
//! ```
//!
//! ### Automatic Provider Detection
//!
//! Use [`GitRemote::from_url`] to automatically detect the provider from a URL:
//!
//! ```ignore
//! use sniff::remote::{GitRemote, RemoteRepoProvider};
//!
//! let remote = GitRemote::from_url("https://github.com/rust-lang/cargo")?;
//! let parsed = GitRemote::parse_url("https://github.com/rust-lang/cargo")?;
//! let report = remote.fetch_report(&parsed.owner, &parsed.repo).await?;
//! ```

mod bitbucket;
mod focused;
mod gitea;
mod github;
mod gitlab;
mod provider;
mod types;
mod url_parser;

use async_trait::async_trait;

pub use bitbucket::*;
pub use focused::*;
pub use gitea::*;
pub use github::*;
pub use gitlab::*;
pub use provider::*;
pub use types::*;
pub use url_parser::*;

use crate::error::SniffError;

type RemoteCandidate = (&'static str, fn() -> Result<GitRemote, SniffError>);

/// Unified remote repository provider.
///
/// Use [`GitRemote::from_url`] to automatically detect the provider from a URL,
/// or construct directly for explicit provider selection.
///
/// ## Examples
///
/// ```ignore
/// use sniff::remote::{GitRemote, RemoteRepoProvider};
///
/// // Auto-detect provider from URL
/// let remote = GitRemote::from_url("https://github.com/rust-lang/cargo")?;
/// let parsed = GitRemote::parse_url("https://github.com/rust-lang/cargo")?;
/// let report = remote.fetch_report(&parsed.owner, &parsed.repo).await?;
///
/// // Or construct explicitly
/// let github = GitRemote::GitHub(GitHubRemote::new()?);
/// ```
pub enum GitRemote {
    /// GitHub (github.com or GitHub Enterprise).
    GitHub(GitHubRemote),
    /// GitLab (gitlab.com or self-hosted).
    GitLab(GitLabRemote),
    /// Gitea or Forgejo (self-hosted).
    Gitea(GiteaRemote),
    /// Bitbucket (bitbucket.org or Bitbucket Server).
    Bitbucket(BitbucketRemote),
}

impl GitRemote {
    /// Create a provider from a git remote URL.
    ///
    /// Automatically detects the provider from the URL host.
    /// For self-hosted instances, the base URL is extracted from the parsed URL.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use sniff::remote::GitRemote;
    ///
    /// // GitHub
    /// let remote = GitRemote::from_url("https://github.com/rust-lang/cargo")?;
    ///
    /// // GitLab (self-hosted)
    /// let remote = GitRemote::from_url("https://gitlab.example.com/team/project")?;
    ///
    /// // Gitea/Codeberg
    /// let remote = GitRemote::from_url("https://codeberg.org/forgejo/forgejo")?;
    ///
    /// // SSH URLs also work
    /// let remote = GitRemote::from_url("git@github.com:rust-lang/cargo.git")?;
    /// ```
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::UnsupportedProvider` if the URL cannot be parsed.
    /// Returns `SniffError::RemoteInit` if the provider cannot be initialized.
    pub fn from_url(url: &str) -> Result<Self, SniffError> {
        let parsed = parse_remote_url(url)?;
        match parsed.provider {
            GitProvider::GitHub => {
                if let Some(base_url) = parsed.base_url {
                    Ok(Self::GitHub(GitHubRemote::with_base_url(&base_url)?))
                } else {
                    Ok(Self::GitHub(GitHubRemote::new()?))
                }
            }
            GitProvider::GitLab => {
                if let Some(base_url) = parsed.base_url {
                    // GitLab base_url from parser is already the API URL
                    // Extract the host part for with_base_url which expects the root URL
                    let root_url = base_url
                        .strip_suffix("/api/v4")
                        .unwrap_or(&base_url)
                        .to_string();
                    Ok(Self::GitLab(GitLabRemote::with_base_url(&root_url)?))
                } else {
                    Ok(Self::GitLab(GitLabRemote::new()?))
                }
            }
            GitProvider::Gitea => {
                let base_url = parsed.base_url.ok_or_else(|| SniffError::RemoteInit {
                    provider: "Gitea".to_string(),
                    message: "Gitea requires a base URL".to_string(),
                })?;
                Ok(Self::Gitea(GiteaRemote::with_base_url(&base_url)?))
            }
            GitProvider::Bitbucket => {
                if let Some(base_url) = parsed.base_url {
                    Ok(Self::Bitbucket(BitbucketRemote::with_base_url(&base_url)?))
                } else {
                    Ok(Self::Bitbucket(BitbucketRemote::new()?))
                }
            }
        }
    }

    /// Resolve an `owner/repo` shorthand by probing configured providers.
    ///
    /// Checks which providers have credentials configured, then tries each
    /// sequentially (GitHub → GitLab → Bitbucket) using `get_repo_metadata`
    /// as a lightweight existence check.
    ///
    /// Gitea is excluded because it is self-hosted and has no default base URL.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// use sniff::remote::GitRemote;
    ///
    /// let (remote, owner, repo) = GitRemote::from_shorthand("rust-lang", "cargo").await?;
    /// let report = remote.fetch_report(&owner, &repo).await?;
    /// ```
    ///
    /// ## Errors
    ///
    /// - `MissingCredentials` if no providers have credentials configured.
    /// - `ShorthandNotFound` if all providers with credentials returned 404.
    /// - `InvalidCredentials` or `RateLimited` if a provider rejects immediately.
    pub async fn from_shorthand(owner: &str, repo: &str) -> Result<Self, SniffError> {
        let candidates: Vec<RemoteCandidate> = vec![
            ("GitHub", || GitHubRemote::new().map(GitRemote::GitHub)),
            ("GitLab", || GitLabRemote::new().map(GitRemote::GitLab)),
            ("Bitbucket", || {
                BitbucketRemote::new().map(GitRemote::Bitbucket)
            }),
        ];

        let mut tried: Vec<&str> = Vec::new();

        for (name, constructor) in &candidates {
            // Skip providers whose constructor fails (missing credentials)
            let remote = match constructor() {
                Ok(r) => r,
                Err(SniffError::MissingCredentials { .. }) => continue,
                Err(e) => return Err(e),
            };

            tried.push(name);

            match remote.get_repo_metadata(owner, repo).await {
                Ok(_) => return Ok(remote),
                Err(SniffError::RemoteApi { status: 404, .. }) => {
                    // Not found on this provider, try next
                    continue;
                }
                Err(e @ SniffError::InvalidCredentials { .. })
                | Err(e @ SniffError::RateLimited { .. }) => {
                    return Err(e);
                }
                Err(_) => {
                    // Other errors (network, etc.) — skip to next provider
                    continue;
                }
            }
        }

        if tried.is_empty() {
            return Err(SniffError::MissingCredentials {
                provider: "any".to_string(),
                env_var: "GITHUB_TOKEN/GH_TOKEN, GITLAB_TOKEN/GITLAB_PRIVATE_TOKEN, or BITBUCKET_USERNAME+BITBUCKET_APP_PASSWORD".to_string(),
            });
        }

        Err(SniffError::ShorthandNotFound {
            owner: owner.to_string(),
            repo: repo.to_string(),
            providers_tried: tried.join(", "),
        })
    }

    /// Parse a git remote URL without constructing a provider.
    ///
    /// Returns the parsed URL info including owner, repo, and provider.
    /// Use this when you need the URL components but want to construct
    /// the provider separately.
    ///
    /// ## Examples
    ///
    /// ```
    /// use sniff::remote::GitRemote;
    ///
    /// let parsed = GitRemote::parse_url("https://github.com/rust-lang/cargo").unwrap();
    /// assert_eq!(parsed.owner, "rust-lang");
    /// assert_eq!(parsed.repo, "cargo");
    /// ```
    pub fn parse_url(url: &str) -> Result<ParsedRemoteUrl, SniffError> {
        parse_remote_url(url)
    }
}

#[async_trait]
impl RemoteRepoProvider for GitRemote {
    fn provider(&self) -> GitProvider {
        match self {
            Self::GitHub(p) => p.provider(),
            Self::GitLab(p) => p.provider(),
            Self::Gitea(p) => p.provider(),
            Self::Bitbucket(p) => p.provider(),
        }
    }

    async fn get_repo_metadata(&self, owner: &str, repo: &str) -> Result<RepoMetadata, SniffError> {
        match self {
            Self::GitHub(p) => p.get_repo_metadata(owner, repo).await,
            Self::GitLab(p) => p.get_repo_metadata(owner, repo).await,
            Self::Gitea(p) => p.get_repo_metadata(owner, repo).await,
            Self::Bitbucket(p) => p.get_repo_metadata(owner, repo).await,
        }
    }

    async fn get_org_info(&self, org: &str) -> Result<OrgInfo, SniffError> {
        match self {
            Self::GitHub(p) => p.get_org_info(org).await,
            Self::GitLab(p) => p.get_org_info(org).await,
            Self::Gitea(p) => p.get_org_info(org).await,
            Self::Bitbucket(p) => p.get_org_info(org).await,
        }
    }

    async fn list_documents(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<DocumentRef>, SniffError> {
        match self {
            Self::GitHub(p) => p.list_documents(owner, repo).await,
            Self::GitLab(p) => p.list_documents(owner, repo).await,
            Self::Gitea(p) => p.list_documents(owner, repo).await,
            Self::Bitbucket(p) => p.list_documents(owner, repo).await,
        }
    }

    async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> Result<String, SniffError> {
        match self {
            Self::GitHub(p) => p.get_file_content(owner, repo, path).await,
            Self::GitLab(p) => p.get_file_content(owner, repo, path).await,
            Self::Gitea(p) => p.get_file_content(owner, repo, path).await,
            Self::Bitbucket(p) => p.get_file_content(owner, repo, path).await,
        }
    }

    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
        state: PullRequestState,
    ) -> Result<Vec<PullRequestInfo>, SniffError> {
        match self {
            Self::GitHub(p) => p.list_pull_requests(owner, repo, state).await,
            Self::GitLab(p) => p.list_pull_requests(owner, repo, state).await,
            Self::Gitea(p) => p.list_pull_requests(owner, repo, state).await,
            Self::Bitbucket(p) => p.list_pull_requests(owner, repo, state).await,
        }
    }

    async fn list_issues(&self, owner: &str, repo: &str) -> Result<Vec<IssueInfo>, SniffError> {
        match self {
            Self::GitHub(p) => p.list_issues(owner, repo).await,
            Self::GitLab(p) => p.list_issues(owner, repo).await,
            Self::Gitea(p) => p.list_issues(owner, repo).await,
            Self::Bitbucket(p) => p.list_issues(owner, repo).await,
        }
    }

    async fn get_tags_and_releases(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<TagsAndReleases, SniffError> {
        match self {
            Self::GitHub(p) => p.get_tags_and_releases(owner, repo).await,
            Self::GitLab(p) => p.get_tags_and_releases(owner, repo).await,
            Self::Gitea(p) => p.get_tags_and_releases(owner, repo).await,
            Self::Bitbucket(p) => p.get_tags_and_releases(owner, repo).await,
        }
    }

    async fn detect_cicd(&self, owner: &str, repo: &str) -> Result<Option<CiCdInfo>, SniffError> {
        match self {
            Self::GitHub(p) => p.detect_cicd(owner, repo).await,
            Self::GitLab(p) => p.detect_cicd(owner, repo).await,
            Self::Gitea(p) => p.detect_cicd(owner, repo).await,
            Self::Bitbucket(p) => p.detect_cicd(owner, repo).await,
        }
    }

    async fn list_workflow_runs(
        &self,
        owner: &str,
        repo: &str,
        limit: usize,
    ) -> Result<Vec<CiCdInfo>, SniffError> {
        match self {
            Self::GitHub(p) => p.list_workflow_runs(owner, repo, limit).await,
            Self::GitLab(p) => p.list_workflow_runs(owner, repo, limit).await,
            Self::Gitea(p) => p.list_workflow_runs(owner, repo, limit).await,
            Self::Bitbucket(p) => p.list_workflow_runs(owner, repo, limit).await,
        }
    }

    async fn list_org_repos(&self, org: &str) -> Result<Vec<OrgRepoRef>, SniffError> {
        match self {
            Self::GitHub(p) => p.list_org_repos(org).await,
            Self::GitLab(p) => p.list_org_repos(org).await,
            Self::Gitea(p) => p.list_org_repos(org).await,
            Self::Bitbucket(p) => p.list_org_repos(org).await,
        }
    }

    fn build_key_urls(&self, owner: &str, repo: &str) -> KeyUrls {
        match self {
            Self::GitHub(p) => p.build_key_urls(owner, repo),
            Self::GitLab(p) => p.build_key_urls(owner, repo),
            Self::Gitea(p) => p.build_key_urls(owner, repo),
            Self::Bitbucket(p) => p.build_key_urls(owner, repo),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_git_remote_from_url_github() {
        let remote = GitRemote::from_url("https://github.com/rust-lang/cargo").unwrap();
        assert_eq!(remote.provider(), GitProvider::GitHub);
    }

    #[test]
    fn test_git_remote_from_url_gitlab() {
        let remote = GitRemote::from_url("https://gitlab.com/inkscape/inkscape").unwrap();
        assert_eq!(remote.provider(), GitProvider::GitLab);
    }

    #[test]
    fn test_git_remote_from_url_gitlab_self_hosted() {
        let remote = GitRemote::from_url("https://gitlab.example.com/team/project").unwrap();
        assert_eq!(remote.provider(), GitProvider::GitLab);
    }

    #[test]
    fn test_git_remote_from_url_gitea() {
        let remote = GitRemote::from_url("https://codeberg.org/forgejo/forgejo").unwrap();
        assert_eq!(remote.provider(), GitProvider::Gitea);
    }

    #[test]
    fn test_git_remote_from_url_bitbucket() {
        let remote =
            GitRemote::from_url("https://bitbucket.org/atlassian/python-bitbucket").unwrap();
        assert_eq!(remote.provider(), GitProvider::Bitbucket);
    }

    #[test]
    fn test_git_remote_from_url_ssh() {
        let remote = GitRemote::from_url("git@github.com:rust-lang/cargo.git").unwrap();
        assert_eq!(remote.provider(), GitProvider::GitHub);
    }

    #[test]
    fn test_git_remote_parse_url() {
        let parsed = GitRemote::parse_url("https://github.com/rust-lang/cargo").unwrap();
        assert_eq!(parsed.owner, "rust-lang");
        assert_eq!(parsed.repo, "cargo");
        assert_eq!(parsed.provider, GitProvider::GitHub);
    }

    #[test]
    fn test_git_remote_from_url_invalid() {
        let result = GitRemote::from_url("not-a-url");
        assert!(result.is_err());
    }
}
