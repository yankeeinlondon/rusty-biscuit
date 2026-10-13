//! Bitbucket remote repository provider.
//!
//! This module implements [`RemoteRepoProvider`] for Bitbucket Cloud using the
//! generated schematic-schema client.

use async_trait::async_trait;
use schematic_schema::bitbucket::*;
use schematic_schema::schematic_runtime::CallOptions;

use super::{
    count_api_request,
    envelope::{Replies, Unauthorized},
    provider::RemoteRepoProvider,
    snapshot::{RemoteRepoSnapshot, RemoteTree, RemoteTreeFile, documents_from_tree},
    types::{
        CiCdInfo, DocumentRef, GitProvider, IssueInfo, KeyUrls, OrgInfo, OrgRepoRef,
        PullRequestInfo, PullRequestState, ReleaseInfo, RepoMetadata, TagInfo, TagsAndReleases,
    },
};
use crate::error::SniffError;

const REPLIES: Replies = Replies::new(
    "Bitbucket",
    Unauthorized::Missing("BITBUCKET_USERNAME + BITBUCKET_APP_PASSWORD"),
);

/// Bitbucket remote repository provider.
///
/// Uses the schematic-generated Bitbucket API client for all HTTP operations.
/// Each request sends Basic auth from `BITBUCKET_USERNAME` and
/// `BITBUCKET_APP_PASSWORD` when both are set, and is sent anonymously
/// otherwise; public Bitbucket Cloud repositories can be read anonymously.
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
        Self::build(Bitbucket::new())
    }

    /// Create with a custom base URL (for Bitbucket Server/Data Center).
    ///
    /// ## Arguments
    ///
    /// * `base_url` - Base URL for the Bitbucket Server API (e.g., `https://bitbucket.example.com/rest/api/2.0`)
    ///
    /// ## Errors
    ///
    /// Returns `SniffError::RemoteInit` if the client cannot be initialized,
    /// including when `base_url` is not an HTTP(S) URL with a host.
    pub fn with_base_url(base_url: &str) -> Result<Self, SniffError> {
        Self::build(Bitbucket::new().base_url(base_url))
    }

    fn build(builder: Bitbucket) -> Result<Self, SniffError> {
        let client = builder.build().map_err(|error| REPLIES.init(&error))?;
        Ok(Self { client })
    }

    /// Builds the report's tree from the root listing plus any doc directories.
    ///
    /// ## Notes
    ///
    /// Bitbucket has no recursive-tree endpoint, so the "tree" is assembled from
    /// per-directory listings: the root, plus `docs/` and `doc/` when the root says
    /// they exist. Those two are independent once the branch is known, so they run
    /// concurrently (R11.3) rather than as a serial chain.
    ///
    /// Depth stops at one level below the root, which is the pre-existing contract —
    /// nested documents under `docs/guide/` were not detected before this change and
    /// are not detected now.
    async fn fetch_tree(
        &self,
        workspace: &str,
        repo_slug: &str,
        commit: &str,
    ) -> Result<RemoteTree, SniffError> {
        let (root_entries, root_truncated) =
            collect_directory_entries(&self.client, workspace, repo_slug, commit, "").await?;

        let mut files = files_of(&root_entries);

        let listing = |dir: &'static str| {
            let present = has_directory(&root_entries, dir);
            async move {
                if !present {
                    return None;
                }
                collect_directory_entries(&self.client, workspace, repo_slug, commit, dir)
                    .await
                    .ok()
            }
        };
        let (docs, doc) = tokio::join!(listing("docs"), listing("doc"));

        let mut truncated = root_truncated;
        for (entries, was_truncated) in [docs, doc].into_iter().flatten() {
            files.extend(files_of(&entries));
            truncated |= was_truncated;
        }

        Ok(RemoteTree::observed(files, truncated))
    }
}

impl Default for BitbucketRemote {
    /// ## Panics
    ///
    /// Panics if the HTTP client cannot be initialized; use
    /// [`BitbucketRemote::new`] to handle that as an error.
    fn default() -> Self {
        Self::new().expect("the Bitbucket client builds from its default definition")
    }
}

/// Maximum pages followed for one directory listing.
///
/// Bounds the correctness-preserving pagination below. At Bitbucket's 100-item
/// maximum page size this covers 1,000 entries in a single directory, far past any
/// real repository root or `docs/` folder — the bound exists so a pathological
/// directory cannot turn one listing into an unbounded request loop.
const MAX_LISTING_PAGES: i64 = 10;

/// Collects every entry of one directory, following pagination.
///
/// ## Returns
///
/// The directory's entries and whether the listing was cut short at
/// [`MAX_LISTING_PAGES`] with more pages still outstanding.
///
/// ## Notes
///
/// Bitbucket has no recursive-tree flag, so callers walk directory by directory
/// and each call is its own request against the tree. Pagination was previously
/// not followed at all ("For MVP, just return the first page"), which silently
/// truncated any directory past one page — the same defect as an ignored GitHub
/// `truncated` flag, just spelled differently (R11.5).
async fn collect_directory_entries(
    client: &Bitbucket,
    workspace: &str,
    repo_slug: &str,
    commit: &str,
    path: &str,
) -> Result<(Vec<SourceEntry>, bool), SniffError> {
    let mut entries = Vec::new();

    for page in 1..=MAX_LISTING_PAGES {
        let mut request = GetSourceRequest::new(workspace, repo_slug, commit, path);
        request.pagelen = Some(100);
        if page > 1 {
            request.page = Some(page);
            // Pages past the first are continuations, counted separately so a
            // report distinguishes them from the duplicate root listings this
            // phase removed.
            count_api_request("tree_continuation");
        } else {
            count_api_request("tree");
        }

        let body = REPLIES.value(client.send(request, &CallOptions::default()).await)?;
        let response = decode_directory_listing(&body)?;
        entries.extend(response.values);

        if response.next.is_none() {
            return Ok((entries, false));
        }
    }

    Ok((entries, true))
}

/// Decodes a `GetSource` body for a directory path as a paginated JSON listing.
///
/// ## Errors
///
/// A body that is not a listing is a `RemoteApi` error, never an empty
/// directory: reading it as empty would report "no docs, no CI" with full
/// confidence.
fn decode_directory_listing(body: &[u8]) -> Result<PaginatedResponse<SourceEntry>, SniffError> {
    serde_json::from_slice(body).map_err(|error| SniffError::RemoteApi {
        provider: "Bitbucket".to_string(),
        status: 0,
        message: format!("JSON parse error: {error}"),
    })
}

/// Decodes a `GetSource` body for a file path as text.
///
/// ## Notes
///
/// Invalid UTF-8 is replaced rather than rejected, as the runtime's text codec
/// does for endpoints that declare a text response.
fn decode_file_text(body: &[u8]) -> String {
    String::from_utf8_lossy(body).into_owned()
}

/// Keeps file entries and drops directories, normalizing to [`RemoteTreeFile`].
fn files_of(entries: &[SourceEntry]) -> Vec<RemoteTreeFile> {
    entries
        .iter()
        .filter(|entry| entry.is_file())
        .filter_map(|entry| {
            Some(RemoteTreeFile {
                path: entry.path.clone()?,
                size: entry.size,
            })
        })
        .collect()
}

/// Whether `entries` contains a directory named `name`.
fn has_directory(entries: &[SourceEntry], name: &str) -> bool {
    entries
        .iter()
        .any(|e| e.is_directory() && e.path.as_deref() == Some(name))
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
        count_api_request("metadata");
        let info = REPLIES.value(self.client.send(request, &CallOptions::default()).await)?;

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
            full_name: info
                .full_name
                .unwrap_or_else(|| format!("{}/{}", workspace, repo_slug)),
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

    async fn snapshot(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<RemoteRepoSnapshot, SniffError> {
        let metadata = self.get_repo_metadata(workspace, repo_slug).await?;
        // A tree failure degrades the projections that read it; it must not sink a
        // report whose required metadata already succeeded (R11.6).
        let tree = self
            .fetch_tree(workspace, repo_slug, &metadata.default_branch)
            .await
            .unwrap_or_else(|_| RemoteTree::unavailable());

        Ok(RemoteRepoSnapshot {
            owner: workspace.to_string(),
            repo: repo_slug.to_string(),
            metadata,
            tree,
        })
    }

    async fn list_documents(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Vec<DocumentRef>, SniffError> {
        let snapshot = self.snapshot(workspace, repo_slug).await?;
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
        workspace: &str,
        repo_slug: &str,
        path: &str,
    ) -> Result<String, SniffError> {
        // First get the repo to find the default branch
        let repo_request = GetRepositoryRequest::new(workspace, repo_slug);
        count_api_request("metadata");
        let info = REPLIES.value(
            self.client
                .send(repo_request, &CallOptions::default())
                .await,
        )?;

        let commit = info.default_branch().unwrap_or("main").to_string();

        let request = GetSourceRequest::new(workspace, repo_slug, &commit, path);
        count_api_request("contents");
        let body = REPLIES.value(self.client.send(request, &CallOptions::default()).await)?;
        Ok(decode_file_text(&body))
    }

    async fn list_pull_requests(
        &self,
        workspace: &str,
        repo_slug: &str,
        state: PullRequestState,
    ) -> Result<Vec<PullRequestInfo>, SniffError> {
        let mut request = ListPullRequestsRequest::new(workspace, repo_slug);

        // Map PullRequestState to Bitbucket API state parameter
        match state {
            PullRequestState::Open => {
                request = request.with_state("OPEN".to_string());
            }
            PullRequestState::Merged => {
                request = request.with_state("MERGED".to_string());
            }
            PullRequestState::Closed | PullRequestState::Draft | PullRequestState::All => {
                // Bitbucket doesn't support querying multiple states or "ALL" in a single
                // request. Leave state unset to get all PRs, then post-filter.
            }
        }

        count_api_request("pulls");
        let response = REPLIES.value(self.client.send(request, &CallOptions::default()).await)?;

        // Extract .values from paginated response (first page only for MVP)
        let mut prs: Vec<PullRequestInfo> = response
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
                let merged_at = if is_merged { updated_at.clone() } else { None };
                let source_repo = pr
                    .source
                    .as_ref()
                    .and_then(|source| source.repository.as_ref())
                    .and_then(|repository| repository.full_name.clone());
                let source_repo_is_target = source_repo
                    .as_ref()
                    .zip(
                        pr.destination
                            .as_ref()
                            .and_then(|destination| destination.repository.as_ref())
                            .and_then(|repository| repository.full_name.as_ref()),
                    )
                    .map(|(source, target)| source.eq_ignore_ascii_case(target));
                let source_head_sha = pr
                    .source
                    .as_ref()
                    .and_then(|source| source.commit_hash())
                    .filter(|sha| !sha.is_empty())
                    .map(str::to_string);

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
                    // Bitbucket Cloud's PR API does not expose labels,
                    // priority, or kind — those fields exist on Issues only.
                    // The list endpoint also omits the PR description; a
                    // per-PR `GET /pullrequests/{id}` follow-up would be
                    // required to populate `body`.
                    labels: Vec::new(),
                    body: None,
                    created_at: pr.created_on.unwrap_or_default(),
                    updated_at,
                    merged_at,
                    html_url,
                    source_repo,
                    source_repo_is_target,
                    source_head_sha,
                }
            })
            .collect();

        // Post-filter for states Bitbucket API doesn't support directly
        match state {
            PullRequestState::Closed => {
                prs.retain(|pr| pr.state == "declined" || pr.state == "superseded");
            }
            PullRequestState::Merged => {
                prs.retain(|pr| pr.state == "merged");
            }
            PullRequestState::Draft => {
                // Bitbucket doesn't have draft PRs
                prs.clear();
            }
            _ => {}
        }

        Ok(prs)
    }

    async fn list_issues(
        &self,
        workspace: &str,
        repo_slug: &str,
    ) -> Result<Vec<IssueInfo>, SniffError> {
        let request = ListIssuesRequest::new(workspace, repo_slug);

        // Bitbucket issues may be disabled - handle 404 gracefully
        count_api_request("issues");
        let reply = self
            .client
            .send(request, &CallOptions::default())
            .await
            .map_err(|error| REPLIES.transport(&error))?;
        if reply.meta.status.as_u16() == 404 {
            // Issue tracker is disabled on this repo
            return Ok(Vec::new());
        }
        let response = REPLIES.value(Ok(reply))?;

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
                let closed_at = if !is_open { updated_at.clone() } else { None };

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
        count_api_request("tags");
        let tags_response = REPLIES.value(
            self.client
                .send(tags_request, &CallOptions::default())
                .await,
        )?;

        // Convert tags (first page only for MVP)
        let tag_infos: Vec<TagInfo> = tags_response
            .values
            .into_iter()
            .map(|tag| {
                // Extract values before moving tag fields
                let is_annotated = tag.is_annotated();
                let name = tag.name.clone().unwrap_or_default();
                let commit_sha = tag
                    .target
                    .as_ref()
                    .and_then(|t| t.hash.clone())
                    .unwrap_or_default();
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
        count_api_request("releases");
        // Any failure, transport or status, reads as "no downloads".
        let downloads_response = self
            .client
            .send(downloads_request, &CallOptions::default())
            .await
            .ok()
            .and_then(|reply| reply.into_result().ok())
            .unwrap_or_else(|| PaginatedResponse {
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
        let snapshot = self.snapshot(workspace, repo_slug).await?;
        self.detect_cicd_with(&snapshot).await
    }

    async fn detect_cicd_with(
        &self,
        snapshot: &RemoteRepoSnapshot,
    ) -> Result<Option<CiCdInfo>, SniffError> {
        if !snapshot.tree.contains("bitbucket-pipelines.yml") {
            return Ok(None);
        }

        Ok(Some(CiCdInfo {
            provider: "Bitbucket Pipelines".to_string(),
            config_path: Some("bitbucket-pipelines.yml".to_string()),
            name: "Bitbucket Pipelines".to_string(),
            status: "detected".to_string(),
            conclusion: None,
            html_url: Some(format!(
                "https://bitbucket.org/{}/{}/addon/pipelines/home",
                snapshot.owner, snapshot.repo
            )),
            started_at: None,
            head_branch: None,
            event: None,
        }))
    }

    async fn list_org_repos(&self, _workspace: &str) -> Result<Vec<OrgRepoRef>, SniffError> {
        // The generated client has ListWorkspaceReposRequest, but this provider does not call
        // it yet, so workspace listings are always empty.
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
    fn a_directory_listing_body_decodes_to_its_entries() {
        let body =
            br#"{"values": [{"path": "README.md", "type": "commit_file", "size": 3}], "page": 1}"#;

        let listing = decode_directory_listing(body).unwrap();

        assert_eq!(listing.values.len(), 1);
        assert_eq!(listing.values[0].path.as_deref(), Some("README.md"));
        assert!(listing.next.is_none());
    }

    #[test]
    fn a_malformed_directory_listing_body_is_a_json_error() {
        let result = decode_directory_listing(b"# README\n");

        match result {
            Err(SniffError::RemoteApi {
                provider,
                status,
                message,
            }) => {
                assert_eq!(provider, "Bitbucket");
                assert_eq!(status, 0);
                assert!(message.starts_with("JSON parse error"), "{message}");
            }
            other => panic!("expected a RemoteApi JSON error, got {other:?}"),
        }
    }

    #[test]
    fn file_text_replaces_invalid_utf8() {
        assert_eq!(decode_file_text(b"caf\xC3\xA9 \xFF"), "caf\u{E9} \u{FFFD}");
    }

    #[test]
    fn test_build_key_urls() {
        let provider = BitbucketRemote::default();
        let urls = provider.build_key_urls("atlassian", "python-bitbucket");

        assert_eq!(
            urls.repo,
            "https://bitbucket.org/atlassian/python-bitbucket"
        );
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
    fn test_provider_returns_bitbucket() {
        let provider = BitbucketRemote::default();
        assert_eq!(provider.provider(), GitProvider::Bitbucket);
    }
}
