//! GitHub REST API definition.
//!
//! This module provides a focused definition of the GitHub REST API v2022-11-28,
//! optimized for common developer workflows: README discovery, pull requests,
//! issues, and tag/release management.
//!
//! ## API Overview
//!
//! The GitHub REST API provides access to:
//!
//! - **Repository metadata**: Basic repo info and default branch
//! - **Git Trees**: Recursive file discovery (handles large repos)
//! - **Repository Contents**: Raw file content with proper Accept headers
//! - **Pull Requests**: List PRs with associated metadata and files
//! - **Issues**: List issues, comments, and timeline events
//! - **Tags & Releases**: Distinguish lightweight vs annotated tags, link releases
//!
//! ## Authentication
//!
//! Uses Bearer token authentication via `GITHUB_TOKEN` or `GH_TOKEN` environment
//! variables. Required headers are automatically included:
//!
//! - `Accept: application/vnd.github+json`
//! - `X-GitHub-Api-Version: 2022-11-28`
//! - `User-Agent: schematic-github-client`
//!
//! ## Endpoint Coverage (V1)
//!
//! | Category | Endpoints |
//! |----------|-----------|
//! | Repository | `GetRepository` |
//! | Git Trees | `GetGitTree`, `GetGitTreeRecursive` |
//! | Contents | `GetRepositoryContentRaw` |
//! | Pull Requests | `ListPullRequests`, `ListPullRequestFiles` |
//! | Issues | `ListIssues`, `GetIssue`, `ListIssueComments`, `ListIssueTimeline` |
//! | Tags/Releases | `ListTags`, `ListReleases`, `GetTagReference`, `GetAnnotatedTag` |
//! | Actions | `ListWorkflowRuns` |
//! | Organizations | `ListOrgRepos` |

mod types;

pub use types::*;

use crate::registry::SchemaRegistry;
use schematic_define::{
    ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi, RestMethod,
    params::{EndpointParams, PaginationStyle, QueryParamType},
};

/// Creates a schema registry containing all GitHub response types.
///
/// This registry can be used to generate OpenAPI schemas for the GitHub API.
/// All response types used by the API endpoints are registered.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::github::{openapi_registry, define_github_api};
///
/// let registry = openapi_registry();
/// let api = define_github_api();
///
/// // Registry contains all response types
/// assert!(registry.get("RepositoryInfo").is_some());
/// assert!(registry.get("PullRequestSummary").is_some());
/// assert!(registry.get("IssueSummary").is_some());
///
/// // Registry is complete for the API
/// assert!(registry.validate_completeness(&api).is_ok());
/// ```
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        .register::<RepositoryInfo>("RepositoryInfo")
        .register::<GitTreeResponse>("GitTreeResponse")
        .register::<PullRequestSummary>("PullRequestSummary")
        .register::<Vec<PullRequestSummary>>("Vec<PullRequestSummary>")
        .register::<PullRequestFile>("PullRequestFile")
        .register::<Vec<PullRequestFile>>("Vec<PullRequestFile>")
        .register::<IssueSummary>("IssueSummary")
        .register::<Vec<IssueSummary>>("Vec<IssueSummary>")
        .register::<IssueComment>("IssueComment")
        .register::<Vec<IssueComment>>("Vec<IssueComment>")
        .register::<TimelineEvent>("TimelineEvent")
        .register::<Vec<TimelineEvent>>("Vec<TimelineEvent>")
        .register::<RepoTag>("RepoTag")
        .register::<Vec<RepoTag>>("Vec<RepoTag>")
        .register::<Release>("Release")
        .register::<Vec<Release>>("Vec<Release>")
        .register::<GitRef>("GitRef")
        .register::<AnnotatedTagObject>("AnnotatedTagObject")
        .register::<WorkflowRunsResponse>("WorkflowRunsResponse")
        .register::<Vec<RepositoryInfo>>("Vec<RepositoryInfo>")
}

/// Creates the GitHub REST API definition.
///
/// This defines a focused slice of the GitHub REST API covering repository
/// metadata, file discovery, pull requests, issues, and tags/releases.
///
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | GetRepository | GET | /repos/{owner}/{repo} | Get repository metadata |
/// | GetGitTree | GET | /repos/{owner}/{repo}/git/trees/{tree_sha} | Get tree (non-recursive) |
/// | GetGitTreeRecursive | GET | /repos/{owner}/{repo}/git/trees/{tree_sha} | Get tree recursively |
/// | GetRepositoryContentRaw | GET | /repos/{owner}/{repo}/contents/{path} | Get raw file content |
/// | ListPullRequests | GET | /repos/{owner}/{repo}/pulls | List pull requests |
/// | ListPullRequestFiles | GET | /repos/{owner}/{repo}/pulls/{pull_number}/files | List PR files |
/// | ListIssues | GET | /repos/{owner}/{repo}/issues | List issues |
/// | GetIssue | GET | /repos/{owner}/{repo}/issues/{issue_number} | Get single issue |
/// | ListIssueComments | GET | /repos/{owner}/{repo}/issues/{issue_number}/comments | List comments |
/// | ListIssueTimeline | GET | /repos/{owner}/{repo}/issues/{issue_number}/timeline | List timeline |
/// | ListTags | GET | /repos/{owner}/{repo}/tags | List repository tags |
/// | ListReleases | GET | /repos/{owner}/{repo}/releases | List releases |
/// | GetTagReference | GET | /repos/{owner}/{repo}/git/ref/tags/{tag} | Get tag reference |
/// | GetAnnotatedTag | GET | /repos/{owner}/{repo}/git/tags/{tag_sha} | Get annotated tag object |
/// | ListWorkflowRuns | GET | /repos/{owner}/{repo}/actions/runs | List workflow runs |
/// | ListOrgRepos | GET | /orgs/{org}/repos | List organization repositories |
///
/// All `List*` endpoints support pagination via `page` and `per_page` query parameters.
/// The `GetGitTreeRecursive` endpoint has a required `recursive` boolean parameter.
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::github::define_github_api;
///
/// let api = define_github_api();
/// assert_eq!(api.name, "GitHub");
/// assert_eq!(api.endpoints.len(), 16);
/// ```
pub fn define_github_api() -> RestApi {
    RestApi {
        name: "GitHub".to_string(),
        description: "GitHub REST API v2022-11-28 for repository, PR, issue, and release workflows"
            .to_string(),
        base_url: "https://api.github.com".to_string(),
        docs_url: Some("https://docs.github.com/en/rest".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        auth_policy: None,
        env_auth: vec!["GITHUB_TOKEN".to_string(), "GH_TOKEN".to_string()],
        env_username: None,
        headers: vec![
            (
                "Accept".to_string(),
                "application/vnd.github+json".to_string(),
            ),
            ("X-GitHub-Api-Version".to_string(), "2022-11-28".to_string()),
            (
                "User-Agent".to_string(),
                "schematic-github-client".to_string(),
            ),
        ],
        endpoints: vec![
            // =================================================================
            // Repository Metadata
            // =================================================================
            Endpoint {
                id: "GetRepository".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}".to_string(),
                description: "Get repository metadata including default branch".to_string(),
                request: None,
                response: ApiResponse::json_type("RepositoryInfo"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            // =================================================================
            // Git Trees (for file discovery)
            // =================================================================
            Endpoint {
                id: "GetGitTree".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/git/trees/{tree_sha}".to_string(),
                description: "Get a Git tree (non-recursive, single level)".to_string(),
                request: None,
                response: ApiResponse::json_type("GitTreeResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetGitTreeRecursive".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/git/trees/{tree_sha}".to_string(),
                description: "Get a Git tree recursively (may truncate at 100k entries or 7MB)"
                    .to_string(),
                request: None,
                response: ApiResponse::json_type("GitTreeResponse"),
                headers: vec![],
                params: Some(EndpointParams::default().with_query_param(
                    "recursive",
                    QueryParamType::Boolean,
                    true,
                    Some("Fetch tree recursively (always true for this endpoint)"),
                )),
                oauth_scopes: None,
            },
            // =================================================================
            // Repository Contents (raw file access)
            // =================================================================
            Endpoint {
                id: "GetRepositoryContentRaw".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/contents/{+path}".to_string(),
                description: "Get raw file content from repository".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![(
                    "Accept".to_string(),
                    "application/vnd.github.raw+json".to_string(),
                )],
                params: None,
                oauth_scopes: None,
            },
            // =================================================================
            // Pull Requests
            // =================================================================
            Endpoint {
                id: "ListPullRequests".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/pulls".to_string(),
                description: "List pull requests with metadata".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("PullRequestSummary"),
                headers: vec![],
                params: Some(
                    EndpointParams::default()
                        .with_pagination(PaginationStyle::github())
                        .with_query_param(
                            "state",
                            QueryParamType::Enum(vec![
                                "open".to_string(),
                                "closed".to_string(),
                                "all".to_string(),
                            ]),
                            false,
                            Some("Filter by PR state"),
                        )
                        .with_query_param(
                            "sort",
                            QueryParamType::Enum(vec![
                                "created".to_string(),
                                "updated".to_string(),
                                "popularity".to_string(),
                                "long-running".to_string(),
                            ]),
                            false,
                            Some("Sort field"),
                        )
                        .with_query_param(
                            "direction",
                            QueryParamType::Enum(vec!["asc".to_string(), "desc".to_string()]),
                            false,
                            Some("Sort direction"),
                        ),
                ),
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListPullRequestFiles".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/pulls/{pull_number}/files".to_string(),
                description: "List files changed in a pull request".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("PullRequestFile"),
                headers: vec![],
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::github())),
                oauth_scopes: None,
            },
            // =================================================================
            // Issues
            // =================================================================
            Endpoint {
                id: "ListIssues".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues".to_string(),
                description: "List issues (includes PRs; filter by pull_request field)".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("IssueSummary"),
                headers: vec![],
                params: Some(
                    EndpointParams::default()
                        .with_pagination(PaginationStyle::github())
                        .with_query_param(
                            "state",
                            QueryParamType::Enum(vec![
                                "open".to_string(),
                                "closed".to_string(),
                                "all".to_string(),
                            ]),
                            false,
                            Some("Filter by issue state"),
                        )
                        .with_query_param(
                            "sort",
                            QueryParamType::Enum(vec![
                                "created".to_string(),
                                "updated".to_string(),
                                "comments".to_string(),
                            ]),
                            false,
                            Some("Sort field"),
                        )
                        .with_query_param(
                            "direction",
                            QueryParamType::Enum(vec!["asc".to_string(), "desc".to_string()]),
                            false,
                            Some("Sort direction"),
                        ),
                ),
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetIssue".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues/{issue_number}".to_string(),
                description: "Get a single issue by number".to_string(),
                request: None,
                response: ApiResponse::json_type("IssueSummary"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListIssueComments".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues/{issue_number}/comments".to_string(),
                description: "List comments on an issue".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("IssueComment"),
                headers: vec![],
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::github())),
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListIssueTimeline".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues/{issue_number}/timeline".to_string(),
                description: "List timeline events for an issue".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("TimelineEvent"),
                headers: vec![],
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::github())),
                oauth_scopes: None,
            },
            // =================================================================
            // Tags and Releases
            // =================================================================
            Endpoint {
                id: "ListTags".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/tags".to_string(),
                description: "List repository tags".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("RepoTag"),
                headers: vec![],
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::github())),
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListReleases".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/releases".to_string(),
                description: "List releases (linked to tags via tag_name)".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("Release"),
                headers: vec![],
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::github())),
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetTagReference".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/git/ref/tags/{tag}".to_string(),
                description: "Get tag reference (check object.type: 'commit' vs 'tag')".to_string(),
                request: None,
                response: ApiResponse::json_type("GitRef"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetAnnotatedTag".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/git/tags/{tag_sha}".to_string(),
                description: "Get annotated tag object details (message, tagger, verification)"
                    .to_string(),
                request: None,
                response: ApiResponse::json_type("AnnotatedTagObject"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            // =================================================================
            // Workflow Runs (Actions / CI/CD)
            // =================================================================
            Endpoint {
                id: "ListWorkflowRuns".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/actions/runs".to_string(),
                description: "List workflow runs for a repository".to_string(),
                request: None,
                response: ApiResponse::json_type("WorkflowRunsResponse"),
                headers: vec![],
                params: Some(
                    EndpointParams::default()
                        .with_pagination(PaginationStyle::github())
                        .with_query_param(
                            "status",
                            QueryParamType::Enum(vec![
                                "completed".to_string(),
                                "action_required".to_string(),
                                "cancelled".to_string(),
                                "failure".to_string(),
                                "neutral".to_string(),
                                "skipped".to_string(),
                                "stale".to_string(),
                                "success".to_string(),
                                "timed_out".to_string(),
                                "in_progress".to_string(),
                                "queued".to_string(),
                                "requested".to_string(),
                                "waiting".to_string(),
                                "pending".to_string(),
                            ]),
                            false,
                            Some("Filter by workflow run status"),
                        )
                        .with_query_param(
                            "branch",
                            QueryParamType::String,
                            false,
                            Some("Filter by branch name"),
                        )
                        .with_query_param(
                            "event",
                            QueryParamType::String,
                            false,
                            Some("Filter by event type (e.g., push, pull_request)"),
                        ),
                ),
                oauth_scopes: None,
            },
            // =================================================================
            // Organization Repositories
            // =================================================================
            Endpoint {
                id: "ListOrgRepos".to_string(),
                method: RestMethod::Get,
                path: "/orgs/{org}/repos".to_string(),
                description: "List repositories for an organization".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("RepositoryInfo"),
                headers: vec![],
                params: Some(
                    EndpointParams::default()
                        .with_pagination(PaginationStyle::github())
                        .with_query_param(
                            "repo_type",
                            QueryParamType::Enum(vec![
                                "all".to_string(),
                                "public".to_string(),
                                "private".to_string(),
                                "forks".to_string(),
                                "sources".to_string(),
                                "member".to_string(),
                            ]),
                            false,
                            Some("Filter by repository type (API param: type)"),
                        )
                        .with_query_param(
                            "sort",
                            QueryParamType::Enum(vec![
                                "created".to_string(),
                                "updated".to_string(),
                                "pushed".to_string(),
                                "full_name".to_string(),
                            ]),
                            false,
                            Some("Sort field"),
                        )
                        .with_query_param(
                            "direction",
                            QueryParamType::Enum(vec!["asc".to_string(), "desc".to_string()]),
                            false,
                            Some("Sort direction"),
                        ),
                ),
                oauth_scopes: None,
            },
        ],
        module_path: Some("github".to_string()),
        request_suffix: None,
        version: None,
        env_mapping: Some(EnvMapping {
            bearer_token: Some(EnvList::new(vec![
                "GITHUB_TOKEN".to_string(),
                "GH_TOKEN".to_string(),
            ])),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================
    // openapi_registry() tests
    // =============================================

    #[test]
    fn openapi_registry_contains_all_response_types() {
        let registry = openapi_registry();

        assert!(registry.get("RepositoryInfo").is_some());
        assert!(registry.get("GitTreeResponse").is_some());
        assert!(registry.get("PullRequestSummary").is_some());
        assert!(registry.get("Vec<PullRequestSummary>").is_some());
        assert!(registry.get("IssueSummary").is_some());
        assert!(registry.get("Vec<IssueSummary>").is_some());
        assert!(registry.get("RepoTag").is_some());
        assert!(registry.get("Release").is_some());
        assert!(registry.get("WorkflowRunsResponse").is_some());
        assert_eq!(registry.len(), 20);
    }

    #[test]
    fn openapi_registry_validates_against_api() {
        let registry = openapi_registry();
        let api = define_github_api();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }

    #[test]
    fn openapi_registry_converts_to_openapi_schemas() {
        let registry = openapi_registry();
        let openapi_schemas = registry.to_openapi_schemas();

        assert!(
            openapi_schemas.len() >= 20,
            "expected at least 20 schemas, got {}",
            openapi_schemas.len()
        );
        assert!(openapi_schemas.contains_key("RepositoryInfo"));
        assert!(openapi_schemas.contains_key("PullRequestSummary"));
        assert!(openapi_schemas.contains_key("Vec<PullRequestSummary>"));
        assert!(openapi_schemas.contains_key("IssueSummary"));
        assert!(openapi_schemas.contains_key("WorkflowRunsResponse"));
    }

    // =============================================
    // API definition tests
    // =============================================

    #[test]
    fn api_has_correct_metadata() {
        let api = define_github_api();

        assert_eq!(api.name, "GitHub");
        assert_eq!(api.base_url, "https://api.github.com");
        assert!(api.docs_url.is_some());
        assert!(api.docs_url.as_ref().unwrap().contains("docs.github.com"));
    }

    #[test]
    fn api_uses_bearer_token_auth() {
        let api = define_github_api();

        match &api.auth {
            AuthStrategy::BearerToken { header } => {
                assert!(header.is_none()); // Uses default Authorization header
            }
            _ => panic!("Expected BearerToken auth strategy"),
        }

        assert!(api.env_auth.contains(&"GITHUB_TOKEN".to_string()));
        assert!(api.env_auth.contains(&"GH_TOKEN".to_string()));
    }

    #[test]
    fn api_has_required_headers() {
        let api = define_github_api();

        let accept = api.headers.iter().find(|(k, _)| k == "Accept");
        assert!(accept.is_some());
        assert_eq!(accept.unwrap().1, "application/vnd.github+json");

        let version = api
            .headers
            .iter()
            .find(|(k, _)| k == "X-GitHub-Api-Version");
        assert!(version.is_some());
        assert_eq!(version.unwrap().1, "2022-11-28");

        let user_agent = api.headers.iter().find(|(k, _)| k == "User-Agent");
        assert!(user_agent.is_some());
    }

    #[test]
    fn api_has_sixteen_endpoints() {
        let api = define_github_api();
        assert_eq!(api.endpoints.len(), 16);
    }

    #[test]
    fn get_repository_endpoint() {
        let api = define_github_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetRepository")
            .expect("GetRepository endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/repos/{owner}/{repo}");
        assert!(endpoint.request.is_none());
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn git_tree_endpoints() {
        let api = define_github_api();

        let tree = api.endpoints.iter().find(|e| e.id == "GetGitTree").unwrap();
        assert_eq!(tree.path, "/repos/{owner}/{repo}/git/trees/{tree_sha}");
        assert!(!tree.path.contains("recursive"));
        assert!(tree.params.is_none());

        let recursive = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetGitTreeRecursive")
            .unwrap();
        assert_eq!(recursive.path, "/repos/{owner}/{repo}/git/trees/{tree_sha}");
        // recursive param is now explicit
        let params = recursive.params.as_ref().expect("should have params");
        assert!(params.query.iter().any(|p| p.name == "recursive"));
    }

    #[test]
    fn raw_content_endpoint_has_accept_override() {
        let api = define_github_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetRepositoryContentRaw")
            .expect("GetRepositoryContentRaw endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert!(matches!(endpoint.response, ApiResponse::Text));

        let accept_header = endpoint.headers.iter().find(|(k, _)| k == "Accept");
        assert!(accept_header.is_some());
        assert_eq!(accept_header.unwrap().1, "application/vnd.github.raw+json");
    }

    #[test]
    fn pull_request_endpoints() {
        let api = define_github_api();

        let list_prs = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequests")
            .unwrap();
        assert_eq!(list_prs.path, "/repos/{owner}/{repo}/pulls");
        let params = list_prs.params.as_ref().expect("should have params");
        assert!(params.has_pagination());
        assert!(params.query.iter().any(|p| p.name == "state"));
        assert!(params.query.iter().any(|p| p.name == "sort"));
        assert!(params.query.iter().any(|p| p.name == "direction"));

        let list_files = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequestFiles")
            .unwrap();
        assert!(list_files.path.contains("{pull_number}"));
        let file_params = list_files.params.as_ref().expect("should have params");
        assert!(file_params.has_pagination());
    }

    #[test]
    fn issue_endpoints() {
        let api = define_github_api();

        let list = api.endpoints.iter().find(|e| e.id == "ListIssues").unwrap();
        assert_eq!(list.path, "/repos/{owner}/{repo}/issues");
        let params = list.params.as_ref().expect("should have params");
        assert!(params.has_pagination());
        assert!(params.query.iter().any(|p| p.name == "state"));

        let get = api.endpoints.iter().find(|e| e.id == "GetIssue").unwrap();
        assert!(get.path.contains("{issue_number}"));

        let comments = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueComments")
            .unwrap();
        assert!(comments.path.contains("{issue_number}/comments"));
        let comment_params = comments.params.as_ref().expect("should have params");
        assert!(comment_params.has_pagination());

        let timeline = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueTimeline")
            .unwrap();
        assert!(timeline.path.contains("timeline"));
        let timeline_params = timeline.params.as_ref().expect("should have params");
        assert!(timeline_params.has_pagination());
    }

    #[test]
    fn tag_and_release_endpoints() {
        let api = define_github_api();

        let tags = api.endpoints.iter().find(|e| e.id == "ListTags").unwrap();
        assert_eq!(tags.path, "/repos/{owner}/{repo}/tags");
        let tag_params = tags.params.as_ref().expect("should have params");
        assert!(tag_params.has_pagination());

        let releases = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListReleases")
            .unwrap();
        assert_eq!(releases.path, "/repos/{owner}/{repo}/releases");
        let release_params = releases.params.as_ref().expect("should have params");
        assert!(release_params.has_pagination());

        let ref_endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetTagReference")
            .unwrap();
        assert!(ref_endpoint.path.contains("git/ref/tags/{tag}"));

        let annotated = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetAnnotatedTag")
            .unwrap();
        assert!(annotated.path.contains("git/tags/{tag_sha}"));
    }

    #[test]
    fn env_mapping_configured() {
        let api = define_github_api();

        let mapping = api.env_mapping.as_ref().expect("env_mapping should be set");
        let bearer = mapping
            .bearer_token
            .as_ref()
            .expect("bearer_token mapping should be set");

        assert!(bearer.names().contains(&"GITHUB_TOKEN".to_string()));
        assert!(bearer.names().contains(&"GH_TOKEN".to_string()));
    }

    #[test]
    fn all_endpoints_have_descriptions() {
        let api = define_github_api();

        for endpoint in &api.endpoints {
            assert!(
                !endpoint.description.is_empty(),
                "Endpoint {} missing description",
                endpoint.id
            );
        }
    }

    #[test]
    fn list_endpoints_return_vec_types() {
        let api = define_github_api();

        let list_endpoints = [
            "ListPullRequests",
            "ListPullRequestFiles",
            "ListIssues",
            "ListIssueComments",
            "ListIssueTimeline",
            "ListTags",
            "ListReleases",
            "ListOrgRepos",
        ];

        for id in list_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            match &endpoint.response {
                ApiResponse::Json(schema) => {
                    assert!(
                        schema.type_name.starts_with("Vec<"),
                        "Endpoint {} should return Vec type, got {}",
                        id,
                        schema.type_name
                    );
                }
                _ => panic!("Endpoint {} should return JSON", id),
            }
        }
    }

    #[test]
    fn single_item_endpoints_return_non_vec_types() {
        let api = define_github_api();

        let single_endpoints = [
            "GetRepository",
            "GetGitTree",
            "GetGitTreeRecursive",
            "GetIssue",
            "GetTagReference",
            "GetAnnotatedTag",
            "ListWorkflowRuns", // Returns WorkflowRunsResponse wrapper, not Vec
        ];

        for id in single_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            // GetRepositoryContentRaw returns Text, which is fine; only
            // assert single-item shape for JSON responses.
            if let ApiResponse::Json(schema) = &endpoint.response {
                assert!(
                    !schema.type_name.starts_with("Vec<"),
                    "Endpoint {} should return single item, not Vec",
                    id
                );
            }
        }
    }

    #[test]
    fn list_endpoints_have_pagination() {
        let api = define_github_api();

        let list_endpoints = [
            "ListPullRequests",
            "ListPullRequestFiles",
            "ListIssues",
            "ListIssueComments",
            "ListIssueTimeline",
            "ListTags",
            "ListReleases",
            "ListWorkflowRuns",
            "ListOrgRepos",
        ];

        for id in list_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            let params = endpoint
                .params
                .as_ref()
                .unwrap_or_else(|| panic!("Endpoint {} should have params", id));
            assert!(
                params.has_pagination(),
                "Endpoint {} should have pagination",
                id
            );
            // Verify GitHub-style pagination (page + per_page)
            assert!(
                params.query.iter().any(|p| p.name == "page"),
                "Endpoint {} should have page param",
                id
            );
            assert!(
                params.query.iter().any(|p| p.name == "per_page"),
                "Endpoint {} should have per_page param",
                id
            );
        }
    }

    #[test]
    fn workflow_runs_endpoint() {
        let api = define_github_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListWorkflowRuns")
            .expect("ListWorkflowRuns endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/repos/{owner}/{repo}/actions/runs");

        // Returns WorkflowRunsResponse (not Vec) because GitHub wraps in total_count + array
        match &endpoint.response {
            ApiResponse::Json(schema) => {
                assert_eq!(schema.type_name, "WorkflowRunsResponse");
            }
            _ => panic!("Expected JSON response"),
        }

        let params = endpoint.params.as_ref().expect("should have params");
        assert!(params.has_pagination());
        assert!(params.query.iter().any(|p| p.name == "status"));
        assert!(params.query.iter().any(|p| p.name == "branch"));
        assert!(params.query.iter().any(|p| p.name == "event"));
    }

    #[test]
    fn list_org_repos_endpoint() {
        let api = define_github_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListOrgRepos")
            .expect("ListOrgRepos endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/orgs/{org}/repos");

        match &endpoint.response {
            ApiResponse::Json(schema) => {
                assert!(
                    schema.type_name.starts_with("Vec<"),
                    "ListOrgRepos should return Vec type, got {}",
                    schema.type_name
                );
            }
            _ => panic!("Expected JSON response"),
        }

        let params = endpoint.params.as_ref().expect("should have params");
        assert!(params.has_pagination());
        assert!(params.query.iter().any(|p| p.name == "repo_type"));
        assert!(params.query.iter().any(|p| p.name == "sort"));
        assert!(params.query.iter().any(|p| p.name == "direction"));
    }

    #[test]
    fn no_hardcoded_query_params_in_paths() {
        let api = define_github_api();

        for endpoint in &api.endpoints {
            assert!(
                !endpoint.path.contains("per_page="),
                "Endpoint {} should not have hardcoded per_page in path",
                endpoint.id
            );
            assert!(
                !endpoint.path.contains("state="),
                "Endpoint {} should not have hardcoded state in path",
                endpoint.id
            );
            assert!(
                !endpoint.path.contains("sort="),
                "Endpoint {} should not have hardcoded sort in path",
                endpoint.id
            );
            assert!(
                !endpoint.path.contains("direction="),
                "Endpoint {} should not have hardcoded direction in path",
                endpoint.id
            );
        }
    }
}
