//! Bitbucket remote repository provider.
//!
//! This module implements [`RemoteRepoProvider`] for Bitbucket Cloud using the
//! generated schematic-schema client.

use async_trait::async_trait;
use schematic_schema::bitbucket::*;
use schematic_schema::shared::SchematicError;

use super::{
    provider::RemoteRepoProvider,
    types::{
        CiCdInfo, DocumentCategory, DocumentRef, GitProvider, IssueInfo, KeyUrls, OrgInfo,
        OrgRepoRef, PullRequestInfo, ReleaseInfo, RepoMetadata, TagInfo, TagsAndReleases,
    },
};
use crate::error::SniffError;

/// Bitbucket remote repository provider.
///
/// Uses the schematic-generated Bitbucket API client for all HTTP operations.
/// Authentication uses Basic auth via `BITBUCKET_USERNAME` and `BITBUCKET_APP_PASSWORD`
/// environment variables.
///
/// ## Bitbucket-Specific Terminology
///
/// - Bitbucket uses "workspace" instead of "org/owner"
/// - Uses "pullrequests" (all states) instead of "pull requests"
/// - No first-class releases - uses "downloads" for artifacts
/// - Issue tracker is optional and may be disabled on repos
///
/// ## Examples
///
/// ```ignore
/// use sniff::remote::{BitbucketRemote, RemoteRepoProvider};
///
/// let provider = BitbucketRemote::new()?;
/// let metadata = provider.get_repo_metadata("atlassian", "python-bitbucket").await?;
/// println!("Stars: {:?}", metadata.stars);
/// ```
pub struct BitbucketRemote {
    client: Bitbucket,
}

impl BitbucketRemote {
    /// Create a new Bitbucket provider using environment variables.
    ///
    /// Uses `BITBUCKET_USERNAME` and `BITBUCKET_APP_PASSWORD` for authentication.
    /// App passwords can be created at:
    /// <https://bitbucket.org/account/settings/app-passwords/>
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    pub fn new() -> Result<Self, SniffError> {
        Ok(Self {
            client: Bitbucket::new(),
        })
    }

    /// Create with a custom base URL (for Bitbucket Server/Data Center).
    ///
    /// ## Arguments
    ///
    /// * `base_url` - Base URL for the Bitbucket Server API (e.g., `https://bitbucket.example.com/rest/api/2.0`)
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized.
    pub fn with_base_url(base_url: &str) -> Result<Self, SniffError> {
        Ok(Self {
            client: Bitbucket::with_base_url(base_url),
        })
    }
}

impl Default for BitbucketRemote {
    fn default() -> Self {
        Self {
            client: Bitbucket::new(),
        }
    }
}

/// Map a [`SchematicError`] to a [`SniffError`].
///
/// Handles special cases:
/// - Missing credentials -> `MissingCredentials`
/// - 401 -> `MissingCredentials`
/// - 403/429 -> `RateLimited`
/// - 404 -> `RemoteApi` with "Not found"
/// - Other HTTP errors -> `RemoteApi`
fn map_schematic_error(err: SchematicError) -> SniffError {
    match err {
        SchematicError::MissingCredential {
            env_vars,
        } => SniffError::MissingCredentials {
            provider: "Bitbucket".to_string(),
            env_var: env_vars.join(" or "),
        },
        SchematicError::ApiError {
            status: 401,
            body,
        } => SniffError::MissingCredentials {
            provider: "Bitbucket".to_string(),
            env_var: format!(
                "BITBUCKET_USERNAME + BITBUCKET_APP_PASSWORD (API returned 401: {})",
                body
            ),
        },
        SchematicError::ApiError {
            status: 403,
            body,
        } => {
            if body.to_lowercase().contains("rate limit") {
                SniffError::RateLimited {
                    provider: "Bitbucket".to_string(),
                    retry_after: None,
                }
            } else {
                SniffError::RemoteApi {
                    provider: "Bitbucket".to_string(),
                    status: 403,
                    message: body,
                }
            }
        }
        SchematicError::ApiError {
            status: 429,
            ..
        } => SniffError::RateLimited {
            provider: "Bitbucket".to_string(),
            retry_after: None,
        },
        SchematicError::ApiError {
            status: 404,
            ..
        } => SniffError::RemoteApi {
            provider: "Bitbucket".to_string(),
            status: 404,
            message: "Not found".to_string(),
        },
        SchematicError::ApiError {
            status,
            body,
        } => SniffError::RemoteApi {
            provider: "Bitbucket".to_string(),
            status,
            message: body,
        },
        SchematicError::Http(e) => SniffError::RemoteApi {
            provider: "Bitbucket".to_string(),
            status: 0,
            message: e.to_string(),
        },
        SchematicError::Json(e) => SniffError::RemoteApi {
            provider: "Bitbucket".to_string(),
            status: 0,
            message: format!("JSON parse error: {}", e),
        },
        other => SniffError::RemoteApi {
            provider: "Bitbucket".to_string(),
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

/// Recursively collect all entries from a directory tree.
///
/// Bitbucket's API requires recursive traversal since there's no
/// recursive=true flag like GitHub. For MVP, we only traverse top-level
/// and common doc directories.
async fn collect_directory_entries(
    client: &Bitbucket,
    workspace: &str,
    repo_slug: &str,
    commit: &str,
    path: &str,
) -> Result<Vec<SourceEntry>, SniffError> {
    let request = ListDirectoryContentsRequest::new(workspace, repo_slug, commit, path);
    let response: PaginatedResponse<SourceEntry> =
        client.request(request).await.map_err(map_schematic_error)?;

    // For MVP, just return the first page of results
    // Full pagination can be added later by following response.next
    Ok(response.values)
}

#[async_trait]
impl RemoteRepoProvider for BitbucketRemote {
    fn provider(&self) -> GitProvider {
        GitProvider::Bitbucket
    }

    async fn get_repo_metadata(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<RepoMetadata, SniffError> {
        let request = GetRepositoryRequest::new(workspace, repo_slug);
        let info: Repository = self.client.request(request).await.map_err(map_schematic_error)?;

        // Extract values before moving info fields
        let html_url = info
            .links
            .as_ref()
            .and_then(|l| l.get("html"))
            .map(|link| link.href.clone())
            .unwrap_or_else(|| format!("https://bitbucket.org/{}/{}", workspace, repo_slug));

        let default_branch = info.default_branch().unwrap_or("main").to_string();

        Ok(RepoMetadata {
            name: info.name.unwrap_or_default(),
            full_name: info.full_name.unwrap_or_else(|| format!("{}/{}", workspace, repo_slug)),
            description: info.description,
            private: info.is_private,
            default_branch,
            language: info.language,
            stars: None,       // Bitbucket doesn't have stars
            forks: None,       // Fork count not directly available
            open_issues: None, // Would require a separate API call
            archived: false,   // Bitbucket doesn't have an archived concept
            created_at: info.created_on,
            updated_at: info.updated_on,
            pushed_at: None,    // Not available in Bitbucket API
            license: None,      // Would need to parse LICENSE file
            topics: Vec::new(), // Bitbucket doesn't have topics
            has_issues: info.has_issues,
            has_wiki: info.has_wiki,
            homepage: None, // Not available in Bitbucket API
            html_url,
        })
    }

    async fn get_org_info(&self, workspace: &str) -> Result<OrgInfo, SniffError> {
        // Bitbucket doesn't have a direct workspace info endpoint in our schema.
        // We can extract workspace info from the repository response.
        // For now, return a placeholder based on the workspace slug.
        Ok(OrgInfo {
            name: workspace.to_string(),
            display_name: None,
            description: None,
            avatar_url: None,
            html_url: Some(format!("https://bitbucket.org/{}", workspace)),
        })
    }

    async fn list_documents(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Vec<DocumentRef>, SniffError> {
        // First get the repo to find the default branch
        let repo_request = GetRepositoryRequest::new(workspace, repo_slug);
        let info: Repository =
            self.client.request(repo_request).await.map_err(map_schematic_error)?;

        let commit = info.default_branch().unwrap_or("main").to_string();

        // Get root directory contents
        let root_entries = collect_directory_entries(
            &self.client,
            workspace,
            repo_slug,
            &commit,
            "", // Empty path for root
        )
        .await?;

        let mut documents = Vec::new();

        // Process root entries
        for entry in &root_entries {
            if entry.is_file()
                && let Some(path) = &entry.path
                && let Some(category) = categorize_document(path)
            {
                documents.push(DocumentRef {
                    path: path.clone(),
                    category,
                    size: entry.size,
                });
            }
        }

        // Check for docs/ directory
        let has_docs =
            root_entries.iter().any(|e| e.is_directory() && e.path.as_deref() == Some("docs"));

        if has_docs
            && let Ok(docs_entries) =
                collect_directory_entries(&self.client, workspace, repo_slug, &commit, "docs").await
        {
            for entry in docs_entries {
                if entry.is_file()
                    && let Some(path) = &entry.path
                    && let Some(category) = categorize_document(path)
                {
                    documents.push(DocumentRef {
                        path: path.clone(),
                        category,
                        size: entry.size,
                    });
                }
            }
        }

        // Check for doc/ directory
        let has_doc =
            root_entries.iter().any(|e| e.is_directory() && e.path.as_deref() == Some("doc"));

        if has_doc
            && let Ok(doc_entries) =
                collect_directory_entries(&self.client, workspace, repo_slug, &commit, "doc").await
        {
            for entry in doc_entries {
                if entry.is_file()
                    && let Some(path) = &entry.path
                    && let Some(category) = categorize_document(path)
                {
                    documents.push(DocumentRef {
                        path: path.clone(),
                        category,
                        size: entry.size,
                    });
                }
            }
        }

        Ok(documents)
    }

    async fn get_file_content(
        &self,
        workspace: &str,
        repo_slug: &str,
        path: &str,
    ) -> Result<String, SniffError> {
        // First get the repo to find the default branch
        let repo_request = GetRepositoryRequest::new(workspace, repo_slug);
        let info: Repository =
            self.client.request(repo_request).await.map_err(map_schematic_error)?;

        let commit = info.default_branch().unwrap_or("main").to_string();

        let request = GetFileContentRawRequest::new(workspace, repo_slug, &commit, path);
        self.client.get_file_content_raw(request).await.map_err(map_schematic_error)
    }

    async fn list_pull_requests(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Vec<PullRequestInfo>, SniffError> {
        let request = ListPullRequestsRequest::new(workspace, repo_slug);
        let response: PaginatedResponse<PullRequest> =
            self.client.request(request).await.map_err(map_schematic_error)?;

        // Extract .values from paginated response (first page only for MVP)
        Ok(response
            .values
            .into_iter()
            .map(|pr| {
                // Extract HTML URL from links
                let html_url = pr
                    .links
                    .as_ref()
                    .and_then(|l| l.get("html"))
                    .map(|link| link.href.clone())
                    .unwrap_or_default();

                // Normalize Bitbucket state to lowercase
                let state = pr
                    .state
                    .as_deref()
                    .map(|s| s.to_lowercase())
                    .unwrap_or_else(|| "unknown".to_string());

                // Extract updated_at/merged_at before moving
                let is_merged = pr.is_merged();
                let updated_at = pr.updated_on.clone();
                let merged_at = if is_merged {
                    updated_at.clone()
                } else {
                    None
                };

                PullRequestInfo {
                    number: pr.id.unwrap_or(0),
                    title: pr.title.unwrap_or_default(),
                    state,
                    author: pr
                        .author
                        .and_then(|a| a.name().map(String::from))
                        .unwrap_or_else(|| "unknown".to_string()),
                    draft: false, // Bitbucket doesn't have draft PRs
                    source_branch: pr
                        .source
                        .as_ref()
                        .and_then(|s| s.branch_name().map(String::from)),
                    target_branch: pr
                        .destination
                        .as_ref()
                        .and_then(|d| d.branch_name().map(String::from)),
                    created_at: pr.created_on.unwrap_or_default(),
                    updated_at,
                    merged_at,
                    html_url,
                }
            })
            .collect())
    }

    async fn list_issues(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Vec<IssueInfo>, SniffError> {
        let request = ListIssuesRequest::new(workspace, repo_slug);

        // Bitbucket issues may be disabled - handle 404 gracefully
        let response: PaginatedResponse<Issue> = match self.client.request(request).await {
            Ok(resp) => resp,
            Err(SchematicError::ApiError {
                status: 404,
                ..
            }) => {
                // Issue tracker is disabled on this repo
                return Ok(Vec::new());
            }
            Err(e) => return Err(map_schematic_error(e)),
        };

        // Extract .values from paginated response (first page only for MVP)
        Ok(response
            .values
            .into_iter()
            .map(|issue| {
                // Extract HTML URL from links
                let html_url = issue
                    .links
                    .as_ref()
                    .and_then(|l| l.get("html"))
                    .map(|link| link.href.clone())
                    .unwrap_or_default();

                // Extract is_open and updated_on before moving
                let is_open = issue.is_open();
                let updated_at = issue.updated_on.clone();
                let closed_at = if !is_open {
                    updated_at.clone()
                } else {
                    None
                };

                // Normalize state: Bitbucket uses "new", "open", "resolved", etc.
                let state = issue
                    .state
                    .as_deref()
                    .map(|s| {
                        if s == "new" || s == "open" {
                            "open".to_string()
                        } else {
                            "closed".to_string()
                        }
                    })
                    .unwrap_or_else(|| "unknown".to_string());

                IssueInfo {
                    number: issue.id.unwrap_or(0),
                    title: issue.title.unwrap_or_default(),
                    state,
                    author: issue
                        .reporter
                        .and_then(|r| r.name().map(String::from))
                        .unwrap_or_else(|| "unknown".to_string()),
                    comment_count: issue.comment_count.map(|c| c as u64),
                    labels: Vec::new(), // Bitbucket uses priority/kind instead of labels
                    created_at: issue.created_on.unwrap_or_default(),
                    updated_at,
                    closed_at,
                    html_url,
                }
            })
            .collect())
    }

    async fn get_tags_and_releases(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<TagsAndReleases, SniffError> {
        // Fetch tags
        let tags_request = ListTagsRequest::new(workspace, repo_slug);
        let tags_response: PaginatedResponse<Tag> =
            self.client.request(tags_request).await.map_err(map_schematic_error)?;

        // Convert tags (first page only for MVP)
        let tag_infos: Vec<TagInfo> = tags_response
            .values
            .into_iter()
            .map(|tag| {
                // Extract values before moving tag fields
                let is_annotated = tag.is_annotated();
                let name = tag.name.clone().unwrap_or_default();
                let commit_sha =
                    tag.target.as_ref().and_then(|t| t.hash.clone()).unwrap_or_default();
                let tagger = tag.tagger.as_ref().and_then(|t| t.raw.clone());

                TagInfo {
                    name,
                    commit_sha,
                    annotated: is_annotated,
                    message: tag.message,
                    tagger,
                    tagged_at: tag.date,
                }
            })
            .collect();

        // Fetch downloads (Bitbucket's equivalent of releases)
        let downloads_request = ListDownloadsRequest::new(workspace, repo_slug);
        let downloads_response: PaginatedResponse<Download> =
            self.client.request(downloads_request).await.unwrap_or_else(|_| PaginatedResponse {
                values: Vec::new(),
                next: None,
                previous: None,
                size: None,
                pagelen: None,
                page: None,
            });

        // Convert downloads to releases (first page only for MVP)
        let release_infos: Vec<ReleaseInfo> = downloads_response
            .values
            .into_iter()
            .map(|download| {
                // Extract values before moving
                let html_url = download.download_url().map(String::from);
                let name = download.name.clone();
                let tag_name = name.clone().unwrap_or_default();

                ReleaseInfo {
                    name,
                    tag_name,
                    draft: false,
                    prerelease: false,
                    published_at: download.created_on,
                    html_url,
                }
            })
            .collect();

        Ok(TagsAndReleases {
            tags: tag_infos,
            releases: release_infos,
        })
    }

    async fn detect_cicd(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Option<CiCdInfo>, SniffError> {
        // First get the repo to find the default branch
        let repo_request = GetRepositoryRequest::new(workspace, repo_slug);
        let info: Repository =
            self.client.request(repo_request).await.map_err(map_schematic_error)?;

        let commit = info.default_branch().unwrap_or("main").to_string();

        // Get root directory contents
        let root_entries =
            collect_directory_entries(&self.client, workspace, repo_slug, &commit, "").await?;

        // Look for bitbucket-pipelines.yml
        let has_pipelines = root_entries.iter().any(|entry| {
            entry.is_file() && entry.path.as_deref() == Some("bitbucket-pipelines.yml")
        });

        if has_pipelines {
            Ok(Some(CiCdInfo {
                provider: "Bitbucket Pipelines".to_string(),
                config_path: Some("bitbucket-pipelines.yml".to_string()),
                name: "Bitbucket Pipelines".to_string(),
                status: "detected".to_string(),
                conclusion: None,
                html_url: Some(format!(
                    "https://bitbucket.org/{}/{}/addon/pipelines/home",
                    workspace, repo_slug
                )),
                started_at: None,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_org_repos(&self, _workspace: &str) -> Result<Vec<OrgRepoRef>, SniffError> {
        // Bitbucket ListWorkspaceRepos endpoint is not implemented in schematic-definitions
        // Return empty for now
        Ok(Vec::new())
    }

    fn build_key_urls(&self, workspace: &str, repo_slug: &str) -> KeyUrls {
        let base = format!("https://bitbucket.org/{}/{}", workspace, repo_slug);

        KeyUrls {
            repo: base.clone(),
            homepage: None, // Would come from repo metadata
            docs: None,
            issues: Some(format!("{}/issues", base)),
            pull_requests: Some(format!("{}/pull-requests", base)),
            wiki: Some(format!("{}/wiki", base)),
            ci_cd: Some(format!("{}/addon/pipelines/home", base)),
            insights: Some(format!("{}/commits", base)),
            releases: Some(format!("{}/downloads", base)),
            settings: Some(format!("{}/admin", base)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_readme() {
        assert_eq!(categorize_document("README.md"), Some(DocumentCategory::Readme));
        assert_eq!(categorize_document("readme.txt"), Some(DocumentCategory::Readme));
        assert_eq!(categorize_document("sub/README.md"), Some(DocumentCategory::Readme));
    }

    #[test]
    fn test_categorize_docs_folder() {
        assert_eq!(categorize_document("docs/guide.md"), Some(DocumentCategory::DocsFolder));
        assert_eq!(categorize_document("doc/api.md"), Some(DocumentCategory::DocsFolder));
    }

    #[test]
    fn test_categorize_source_doc() {
        assert_eq!(categorize_document("src/README.md"), Some(DocumentCategory::SourceDoc));
        // Non-doc files in src/ should be None
        assert_eq!(categorize_document("src/main.rs"), None);
    }

    #[test]
    fn test_categorize_other() {
        assert_eq!(categorize_document("CHANGELOG.md"), Some(DocumentCategory::Other));
        assert_eq!(categorize_document("LICENSE"), Some(DocumentCategory::Other));
        assert_eq!(categorize_document("CONTRIBUTING.md"), Some(DocumentCategory::Other));
    }

    #[test]
    fn test_categorize_non_doc() {
        assert_eq!(categorize_document("main.rs"), None);
        assert_eq!(categorize_document("lib/utils.js"), None);
    }

    #[test]
    fn test_build_key_urls() {
        let provider = BitbucketRemote::default();
        let urls = provider.build_key_urls("atlassian", "python-bitbucket");

        assert_eq!(urls.repo, "https://bitbucket.org/atlassian/python-bitbucket");
        assert_eq!(
            urls.issues,
            Some("https://bitbucket.org/atlassian/python-bitbucket/issues".to_string())
        );
        assert_eq!(
            urls.pull_requests,
            Some("https://bitbucket.org/atlassian/python-bitbucket/pull-requests".to_string())
        );
        assert_eq!(
            urls.ci_cd,
            Some(
                "https://bitbucket.org/atlassian/python-bitbucket/addon/pipelines/home".to_string()
            )
        );
        assert_eq!(
            urls.releases,
            Some("https://bitbucket.org/atlassian/python-bitbucket/downloads".to_string())
        );
    }

    #[test]
    fn test_map_schematic_error_missing_credentials() {
        let err = SchematicError::MissingCredential {
            env_vars: vec![
                "BITBUCKET_USERNAME".to_string(),
                "BITBUCKET_APP_PASSWORD".to_string(),
            ],
        };
        match map_schematic_error(err) {
            SniffError::MissingCredentials {
                provider,
                env_var,
            } => {
                assert_eq!(provider, "Bitbucket");
                assert!(env_var.contains("BITBUCKET_USERNAME"));
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
            SniffError::RateLimited {
                provider,
                ..
            } => {
                assert_eq!(provider, "Bitbucket");
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
                assert_eq!(provider, "Bitbucket");
                assert_eq!(status, 404);
                assert_eq!(message, "Not found");
            }
            _ => panic!("Expected RemoteApi error"),
        }
    }

    #[test]
    fn test_provider_returns_bitbucket() {
        let provider = BitbucketRemote::default();
        assert_eq!(provider.provider(), GitProvider::Bitbucket);
    }
}
