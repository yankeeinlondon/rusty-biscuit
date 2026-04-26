//! GitHub remote repository provider.
//!
//! This module implements [`RemoteRepoProvider`] for GitHub using the
//! generated schematic-schema client.

use async_trait::async_trait;
use schematic_schema::github::*;
use schematic_schema::shared::SchematicError;

use super::{
    provider::RemoteRepoProvider,
    types::{
        CiCdInfo, DocumentCategory, DocumentRef, GitProvider, IssueInfo, KeyUrls, LicenseRef,
        OrgInfo, OrgRepoRef, PullRequestInfo, ReleaseInfo, RepoMetadata, TagInfo, TagsAndReleases,
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

/// Map a [`SchematicError`] to a [`SniffError`].
///
/// Handles special cases:
/// - 401 -> `MissingCredentials`
/// - 403 with rate limit info -> `RateLimited`
/// - 404 -> `RemoteApi` with "Not found"
/// - Other HTTP errors -> `RemoteApi`
fn map_schematic_error(err: SchematicError) -> SniffError {
    match err {
        SchematicError::MissingCredential { env_vars } => SniffError::MissingCredentials {
            provider: "GitHub".to_string(),
            env_var: env_vars.join(" or "),
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

/// Categorize a file path as a document type.
fn categorize_document(path: &str) -> Option<DocumentCategory> {
    let lower = path.to_lowercase();
    let filename = path.rsplit('/').next().unwrap_or(path).to_lowercase();

    // Files in src/ directories (source documentation) - check first
    if lower.starts_with("src/") {
        // Only markdown/text files in src count as source docs
        if is_documentation_file(&filename) {
            return Some(DocumentCategory::SourceDoc);
        }
        return None;
    }

    // Files in docs/ or doc/ directories
    if lower.starts_with("docs/") || lower.starts_with("doc/") {
        return Some(DocumentCategory::DocsFolder);
    }

    // README files at root or in non-src directories
    if filename.starts_with("readme") {
        return Some(DocumentCategory::Readme);
    }

    // Other markdown/text files at root or elsewhere
    if is_documentation_file(&filename) {
        return Some(DocumentCategory::Other);
    }

    None
}

/// Check if a filename is a documentation file.
fn is_documentation_file(filename: &str) -> bool {
    let lower = filename.to_lowercase();
    lower.ends_with(".md")
        || lower.ends_with(".markdown")
        || lower.ends_with(".txt")
        || lower.ends_with(".rst")
        || lower == "license"
        || lower == "licence"
        || lower == "changelog"
        || lower == "changes"
        || lower == "history"
        || lower == "contributing"
        || lower == "authors"
        || lower == "contributors"
        || lower == "code_of_conduct"
}

#[async_trait]
impl RemoteRepoProvider for GitHubRemote {
    fn provider(&self) -> GitProvider {
        GitProvider::GitHub
    }

    async fn get_repo_metadata(&self, owner: &str, repo: &str) -> Result<RepoMetadata, SniffError> {
        let request = GetRepositoryRequest::new(owner, repo);
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

    async fn list_documents(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<DocumentRef>, SniffError> {
        // First get the repo to find the default branch
        let repo_request = GetRepositoryRequest::new(owner, repo);
        let info: RepositoryInfo = self
            .client
            .request(repo_request)
            .await
            .map_err(map_schematic_error)?;

        // Use the default branch as the tree SHA
        let tree_request = GetGitTreeRecursiveRequest::new(owner, repo, &info.default_branch);
        let tree: GitTreeResponse = self
            .client
            .request(tree_request)
            .await
            .map_err(map_schematic_error)?;

        let documents: Vec<DocumentRef> = tree
            .tree
            .into_iter()
            .filter(|entry| entry.entry_type == "blob")
            .filter_map(|entry| {
                categorize_document(&entry.path).map(|category| DocumentRef {
                    path: entry.path,
                    category,
                    size: entry.size,
                })
            })
            .collect();

        Ok(documents)
    }

    async fn get_file_content(
        &self,
        owner: &str,
        repo: &str,
        path: &str,
    ) -> Result<String, SniffError> {
        let request = GetRepositoryContentRawRequest::new(owner, repo, path);
        self.client
            .get_repository_content_raw(request)
            .await
            .map_err(map_schematic_error)
    }

    async fn list_pull_requests(
        &self,
        owner: &str,
        repo: &str,
    ) -> Result<Vec<PullRequestInfo>, SniffError> {
        let request = ListPullRequestsRequest::new(owner, repo);
        let prs: Vec<PullRequestSummary> = self
            .client
            .request(request)
            .await
            .map_err(map_schematic_error)?;

        Ok(prs
            .into_iter()
            .map(|pr| PullRequestInfo {
                number: pr.number,
                title: pr.title,
                state: pr.state,
                author: pr.user.login,
                draft: pr.draft.unwrap_or(false),
                source_branch: Some(pr.head.ref_name),
                target_branch: Some(pr.base.ref_name),
                labels: Vec::new(),
                body: pr.body,
                created_at: pr.created_at,
                updated_at: Some(pr.updated_at),
                merged_at: pr.merged_at,
                html_url: pr.html_url,
            })
            .collect())
    }

    async fn list_issues(&self, owner: &str, repo: &str) -> Result<Vec<IssueInfo>, SniffError> {
        let request = ListIssuesRequest::new(owner, repo);
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
        let tags: Vec<RepoTag> = self
            .client
            .request(tags_request)
            .await
            .map_err(map_schematic_error)?;

        // Fetch releases
        let releases_request = ListReleasesRequest::new(owner, repo);
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
        // First get the repo to find the default branch
        let repo_request = GetRepositoryRequest::new(owner, repo);
        let info: RepositoryInfo = self
            .client
            .request(repo_request)
            .await
            .map_err(map_schematic_error)?;

        // Get the tree to look for CI/CD config files
        let tree_request = GetGitTreeRecursiveRequest::new(owner, repo, &info.default_branch);
        let tree: GitTreeResponse = self
            .client
            .request(tree_request)
            .await
            .map_err(map_schematic_error)?;

        // Look for GitHub Actions workflows
        let has_workflows = tree.tree.iter().any(|entry| {
            entry.path.starts_with(".github/workflows/") && entry.entry_type == "blob"
        });

        if has_workflows {
            Ok(Some(CiCdInfo {
                provider: "GitHub Actions".to_string(),
                config_path: Some(".github/workflows".to_string()),
                name: "GitHub Actions".to_string(),
                status: "detected".to_string(),
                conclusion: None,
                html_url: Some(format!("https://github.com/{}/{}/actions", owner, repo)),
                started_at: None,
            }))
        } else {
            Ok(None)
        }
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
    fn test_categorize_readme() {
        assert_eq!(
            categorize_document("README.md"),
            Some(DocumentCategory::Readme)
        );
        assert_eq!(
            categorize_document("readme.txt"),
            Some(DocumentCategory::Readme)
        );
        assert_eq!(
            categorize_document("sub/README.md"),
            Some(DocumentCategory::Readme)
        );
    }

    #[test]
    fn test_categorize_docs_folder() {
        assert_eq!(
            categorize_document("docs/guide.md"),
            Some(DocumentCategory::DocsFolder)
        );
        assert_eq!(
            categorize_document("doc/api.md"),
            Some(DocumentCategory::DocsFolder)
        );
    }

    #[test]
    fn test_categorize_source_doc() {
        assert_eq!(
            categorize_document("src/README.md"),
            Some(DocumentCategory::SourceDoc)
        );
        // Non-doc files in src/ should be None
        assert_eq!(categorize_document("src/main.rs"), None);
    }

    #[test]
    fn test_categorize_other() {
        assert_eq!(
            categorize_document("CHANGELOG.md"),
            Some(DocumentCategory::Other)
        );
        assert_eq!(
            categorize_document("LICENSE"),
            Some(DocumentCategory::Other)
        );
        assert_eq!(
            categorize_document("CONTRIBUTING.md"),
            Some(DocumentCategory::Other)
        );
    }

    #[test]
    fn test_categorize_non_doc() {
        assert_eq!(categorize_document("main.rs"), None);
        assert_eq!(categorize_document("lib/utils.js"), None);
    }

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
}
