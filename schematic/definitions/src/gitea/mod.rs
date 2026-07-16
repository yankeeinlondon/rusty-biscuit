//! Gitea REST API definition.
//!
//! This module provides a focused definition of the Gitea REST API v1.25+,
//! optimized for common developer workflows: README discovery, pull requests,
//! issues, and tag/release management.
//!
//! ## API Overview
//!
//! The Gitea REST API provides access to:
//!
//! - **Repository metadata**: Basic repo info and default branch
//! - **Git Trees**: Recursive file discovery (handles large repos)
//! - **Repository Contents**: Raw file content via `/raw/{filepath}`
//! - **Pull Requests**: List PRs with associated metadata and files
//! - **Issues**: List issues, comments, and timeline events
//! - **Tags & Releases**: Distinguish lightweight vs annotated tags, link releases
//!
//! ## Authentication
//!
//! Uses API key authentication via `GITEA_TOKEN` environment variable.
//!
//! Gitea uses the `Authorization: token <pat>` header format. The definition
//! carries the `token ` scheme prefix, so set `GITEA_TOKEN` to a bare PAT:
//!
//! ```bash
//! export GITEA_TOKEN="your_personal_access_token"
//! ```
//!
//! Required headers are automatically included:
//! - `Accept: application/json`
//!
//! ## Base URL
//!
//! The base URL defaults to `https://gitea.example.com/api/v1` as a placeholder.
//! For self-hosted instances, configure the base URL for your Gitea server.
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
//! | Organizations | `ListOrgRepos` |

mod types;

pub use types::*;

use crate::registry::SchemaRegistry;
use schematic_define::{
    ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod,
    params::{EndpointParams, PaginationStyle, QueryParamType},
};

/// Creates a schema registry containing all Gitea response types.
///
/// This registry can be used to generate OpenAPI schemas for the Gitea API.
/// All response types used by the API endpoints are registered.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::gitea::{openapi_registry, define_gitea_api};
///
/// let registry = openapi_registry();
/// let api = define_gitea_api();
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
        .register::<Label>("Label")
        .register::<RepoTag>("RepoTag")
        .register::<Vec<RepoTag>>("Vec<RepoTag>")
        .register::<Release>("Release")
        .register::<Vec<Release>>("Vec<Release>")
        .register::<GitRef>("GitRef")
        .register::<Vec<GitRef>>("Vec<GitRef>")
        .register::<AnnotatedTagObject>("AnnotatedTagObject")
        .register::<Vec<RepositoryInfo>>("Vec<RepositoryInfo>")
}

/// Creates the Gitea REST API definition.
///
/// This defines a focused slice of the Gitea REST API covering repository
/// metadata, file discovery, pull requests, issues, and tags/releases.
///
/// ## Base URL
///
/// The default base URL is a placeholder (`https://gitea.example.com/api/v1`).
/// For production use, configure this to your Gitea instance URL.
///
/// ## Authentication
///
/// Gitea uses the `Authorization: token <pat>` header format (not Bearer).
/// The definition supplies the `token ` prefix, so set `GITEA_TOKEN` to a
/// bare personal access token:
///
/// ```bash
/// export GITEA_TOKEN="your_personal_access_token"
/// ```
///
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | GetRepository | GET | /repos/{owner}/{repo} | Get repository metadata |
/// | GetGitTree | GET | /repos/{owner}/{repo}/git/trees/{sha} | Get tree (non-recursive) |
/// | GetGitTreeRecursive | GET | /repos/{owner}/{repo}/git/trees/{sha}?recursive=true | Get tree recursively |
/// | GetRepositoryContentRaw | GET | /repos/{owner}/{repo}/raw/{filepath} | Get raw file content |
/// | ListPullRequests | GET | /repos/{owner}/{repo}/pulls | List pull requests |
/// | ListPullRequestFiles | GET | /repos/{owner}/{repo}/pulls/{index}/files | List PR files |
/// | ListIssues | GET | /repos/{owner}/{repo}/issues | List issues (excludes PRs with type=issues) |
/// | GetIssue | GET | /repos/{owner}/{repo}/issues/{index} | Get single issue |
/// | ListIssueComments | GET | /repos/{owner}/{repo}/issues/{index}/comments | List comments |
/// | ListIssueTimeline | GET | /repos/{owner}/{repo}/issues/{index}/timeline | List timeline |
/// | ListTags | GET | /repos/{owner}/{repo}/tags | List repository tags |
/// | ListReleases | GET | /repos/{owner}/{repo}/releases | List releases |
/// | GetTagReference | GET | /repos/{owner}/{repo}/git/refs/{git_ref} | Get tag reference (returns array) |
/// | GetAnnotatedTag | GET | /repos/{owner}/{repo}/git/tags/{sha} | Get annotated tag object |
/// | ListOrgRepos | GET | /orgs/{org}/repos | List organization repositories |
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::gitea::define_gitea_api;
///
/// let api = define_gitea_api();
/// assert_eq!(api.name, "Gitea");
/// assert_eq!(api.endpoints.len(), 15);
/// ```
pub fn define_gitea_api() -> RestApi {
    RestApi {
        name: "Gitea".to_string(),
        description: "Gitea REST API v1.25+ for repository, PR, issue, and release workflows"
            .to_string(),
        base_url: "https://gitea.example.com/api/v1".to_string(),
        docs_url: Some("https://docs.gitea.com/api/1.25/".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "Authorization".to_string(),
            value_prefix: Some("token ".to_string()),
        },
        auth_policy: None,
        env_auth: vec!["GITEA_TOKEN".to_string()],
        env_username: None,
        headers: vec![("Accept".to_string(), "application/json".to_string())],
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
                path: "/repos/{owner}/{repo}/git/trees/{sha}".to_string(),
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
                path: "/repos/{owner}/{repo}/git/trees/{sha}?recursive=true".to_string(),
                description: "Get a Git tree recursively (may be paginated for large repos)"
                    .to_string(),
                request: None,
                response: ApiResponse::json_type("GitTreeResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            // =================================================================
            // Repository Contents (raw file access)
            // =================================================================
            Endpoint {
                id: "GetRepositoryContentRaw".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/raw/{+filepath}".to_string(),
                description: "Get raw file content from repository".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
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
                        .with_pagination(PaginationStyle::gitea())
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
                                "oldest".to_string(),
                                "recentupdate".to_string(),
                                "leastupdate".to_string(),
                                "mostcomment".to_string(),
                                "leastcomment".to_string(),
                                "priority".to_string(),
                            ]),
                            false,
                            Some("Sort order for results"),
                        ),
                ),
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListPullRequestFiles".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/pulls/{index}/files".to_string(),
                description: "List files changed in a pull request".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("PullRequestFile"),
                headers: vec![],
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitea())),
                oauth_scopes: None,
            },
            // =================================================================
            // Issues
            // =================================================================
            Endpoint {
                id: "ListIssues".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues".to_string(),
                description: "List issues (use type=issues to exclude PRs)".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("IssueSummary"),
                headers: vec![],
                params: Some(
                    EndpointParams::default()
                        .with_pagination(PaginationStyle::gitea())
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
                            "issue_type",
                            QueryParamType::Enum(vec![
                                "issues".to_string(),
                                "pulls".to_string(),
                                "all".to_string(),
                            ]),
                            false,
                            Some("Filter by type (issues, pulls, or all)"),
                        ),
                ),
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetIssue".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues/{index}".to_string(),
                description: "Get a single issue by index".to_string(),
                request: None,
                response: ApiResponse::json_type("IssueSummary"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListIssueComments".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues/{index}/comments".to_string(),
                description: "List comments on an issue".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("IssueComment"),
                headers: vec![],
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitea())),
                oauth_scopes: None,
            },
            Endpoint {
                id: "ListIssueTimeline".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues/{index}/timeline".to_string(),
                description: "List timeline events for an issue".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("TimelineEvent"),
                headers: vec![],
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitea())),
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
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitea())),
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
                params: Some(
                    EndpointParams::default()
                        .with_pagination(PaginationStyle::gitea())
                        .with_query_param(
                            "draft",
                            QueryParamType::Boolean,
                            false,
                            Some("Include draft releases"),
                        )
                        .with_query_param(
                            "pre_release",
                            QueryParamType::Boolean,
                            false,
                            Some("Include pre-releases"),
                        ),
                ),
                oauth_scopes: None,
            },
            // Note: Gitea's /git/refs/{ref} returns an ARRAY, unlike GitHub's single object
            // Path uses {git_ref} instead of {ref} to avoid Rust keyword collision
            Endpoint {
                id: "GetTagReference".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/git/refs/{git_ref}".to_string(),
                description:
                    "Get tag reference (returns array; check object.type: 'commit' vs 'tag')"
                        .to_string(),
                request: None,
                response: ApiResponse::json_vec_type("GitRef"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "GetAnnotatedTag".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/git/tags/{sha}".to_string(),
                description: "Get annotated tag object details (message, tagger)".to_string(),
                request: None,
                response: ApiResponse::json_type("AnnotatedTagObject"),
                headers: vec![],
                params: None,
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
                params: Some(EndpointParams::default().with_pagination(PaginationStyle::gitea())),
                oauth_scopes: None,
            },
        ],
        module_path: Some("gitea".to_string()),
        request_suffix: None,
        version: None,
        env_mapping: None,
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
        assert!(registry.get("Label").is_some());
        assert!(registry.get("RepoTag").is_some());
        assert!(registry.get("Release").is_some());
        assert_eq!(registry.len(), 21);
    }

    #[test]
    fn openapi_registry_validates_against_api() {
        let registry = openapi_registry();
        let api = define_gitea_api();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }

    #[test]
    fn openapi_registry_converts_to_openapi_schemas() {
        let registry = openapi_registry();
        let openapi_schemas = registry.to_openapi_schemas();

        assert!(
            openapi_schemas.len() >= 21,
            "expected at least 21 schemas, got {}",
            openapi_schemas.len()
        );
        assert!(openapi_schemas.contains_key("RepositoryInfo"));
        assert!(openapi_schemas.contains_key("PullRequestSummary"));
        assert!(openapi_schemas.contains_key("Vec<PullRequestSummary>"));
        assert!(openapi_schemas.contains_key("IssueSummary"));
        assert!(openapi_schemas.contains_key("Label"));
    }

    // =============================================
    // API definition tests
    // =============================================

    #[test]
    fn api_has_correct_metadata() {
        let api = define_gitea_api();

        assert_eq!(api.name, "Gitea");
        assert_eq!(api.base_url, "https://gitea.example.com/api/v1");
        assert!(api.docs_url.is_some());
        assert!(api.docs_url.as_ref().unwrap().contains("docs.gitea.com"));
    }

    #[test]
    fn api_uses_api_key_auth() {
        let api = define_gitea_api();

        match &api.auth {
            AuthStrategy::ApiKey {
                header,
                value_prefix,
            } => {
                assert_eq!(header, "Authorization");
                // Gitea prepends the `token ` scheme so a bare PAT works.
                assert_eq!(value_prefix.as_deref(), Some("token "));
            }
            _ => panic!("Expected ApiKey auth strategy"),
        }

        assert!(api.env_auth.contains(&"GITEA_TOKEN".to_string()));
    }

    #[test]
    fn api_has_accept_header() {
        let api = define_gitea_api();

        let accept = api.headers.iter().find(|(k, _)| k == "Accept");
        assert!(accept.is_some());
        assert_eq!(accept.unwrap().1, "application/json");
    }

    #[test]
    fn api_has_fifteen_endpoints() {
        let api = define_gitea_api();
        assert_eq!(api.endpoints.len(), 15);
    }

    #[test]
    fn get_repository_endpoint() {
        let api = define_gitea_api();
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
        let api = define_gitea_api();

        let tree = api.endpoints.iter().find(|e| e.id == "GetGitTree").unwrap();
        assert_eq!(tree.path, "/repos/{owner}/{repo}/git/trees/{sha}");
        assert!(!tree.path.contains("recursive"));

        let recursive = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetGitTreeRecursive")
            .unwrap();
        assert!(recursive.path.contains("recursive=true"));
    }

    #[test]
    fn raw_content_endpoint_uses_raw_path() {
        let api = define_gitea_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetRepositoryContentRaw")
            .expect("GetRepositoryContentRaw endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert!(endpoint.path.contains("/raw/"));
        assert!(matches!(endpoint.response, ApiResponse::Text));
    }

    #[test]
    fn pull_request_endpoints_use_pagination() {
        let api = define_gitea_api();

        let list_prs = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequests")
            .unwrap();
        // Clean path without hardcoded query params
        assert_eq!(list_prs.path, "/repos/{owner}/{repo}/pulls");
        // Pagination and query params are in params field
        let params = list_prs
            .params
            .as_ref()
            .expect("ListPullRequests should have params");
        assert!(params.has_pagination());
        assert!(params.query.iter().any(|p| p.name == "page"));
        assert!(params.query.iter().any(|p| p.name == "limit"));
        assert!(params.query.iter().any(|p| p.name == "state"));
        assert!(params.query.iter().any(|p| p.name == "sort"));

        let list_files = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequestFiles")
            .unwrap();
        assert!(list_files.path.contains("{index}"));
        // Clean path without hardcoded limit
        assert!(!list_files.path.contains("limit"));
        let params = list_files
            .params
            .as_ref()
            .expect("ListPullRequestFiles should have params");
        assert!(params.has_pagination());
    }

    #[test]
    fn issue_endpoints_use_pagination_and_type_filter() {
        let api = define_gitea_api();

        let list = api.endpoints.iter().find(|e| e.id == "ListIssues").unwrap();
        // Clean path without hardcoded query params
        assert_eq!(list.path, "/repos/{owner}/{repo}/issues");
        // Pagination and filters in params field
        let params = list.params.as_ref().expect("ListIssues should have params");
        assert!(params.has_pagination());
        assert!(params.query.iter().any(|p| p.name == "state"));
        // Gitea-specific: type param filters out PRs
        assert!(params.query.iter().any(|p| p.name == "issue_type"));

        let get = api.endpoints.iter().find(|e| e.id == "GetIssue").unwrap();
        assert!(get.path.contains("{index}"));
        // GetIssue doesn't need pagination
        assert!(get.params.is_none());

        let comments = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueComments")
            .unwrap();
        assert!(comments.path.contains("{index}/comments"));
        assert!(!comments.path.contains("limit")); // No hardcoded limit in path
        let params = comments
            .params
            .as_ref()
            .expect("ListIssueComments should have params");
        assert!(params.has_pagination());

        let timeline = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueTimeline")
            .unwrap();
        assert!(timeline.path.contains("timeline"));
        assert!(!timeline.path.contains("limit")); // No hardcoded limit in path
        let params = timeline
            .params
            .as_ref()
            .expect("ListIssueTimeline should have params");
        assert!(params.has_pagination());
    }

    #[test]
    fn tag_and_release_endpoints_use_pagination() {
        let api = define_gitea_api();

        let tags = api.endpoints.iter().find(|e| e.id == "ListTags").unwrap();
        // Clean path without hardcoded query params
        assert_eq!(tags.path, "/repos/{owner}/{repo}/tags");
        let params = tags.params.as_ref().expect("ListTags should have params");
        assert!(params.has_pagination());

        let releases = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListReleases")
            .unwrap();
        // Clean path without hardcoded query params
        assert_eq!(releases.path, "/repos/{owner}/{repo}/releases");
        let params = releases
            .params
            .as_ref()
            .expect("ListReleases should have params");
        assert!(params.has_pagination());
        // Gitea-specific: draft and pre-release filters as explicit params
        assert!(params.query.iter().any(|p| p.name == "draft"));
        assert!(params.query.iter().any(|p| p.name == "pre_release"));

        let ref_endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetTagReference")
            .unwrap();
        assert!(ref_endpoint.path.contains("git/refs/{git_ref}"));
        // GetTagReference doesn't need pagination
        assert!(ref_endpoint.params.is_none());

        let annotated = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetAnnotatedTag")
            .unwrap();
        assert!(annotated.path.contains("git/tags/{sha}"));
        // GetAnnotatedTag doesn't need pagination
        assert!(annotated.params.is_none());
    }

    #[test]
    fn get_tag_reference_returns_vec() {
        let api = define_gitea_api();

        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetTagReference")
            .expect("GetTagReference endpoint missing");

        // Gitea returns an array for /git/refs/{ref}, unlike GitHub
        match &endpoint.response {
            ApiResponse::Json(schema) => {
                assert!(
                    schema.type_name.starts_with("Vec<"),
                    "GetTagReference should return Vec<GitRef>, got {}",
                    schema.type_name
                );
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[test]
    fn all_endpoints_have_descriptions() {
        let api = define_gitea_api();

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
        let api = define_gitea_api();

        let list_endpoints = [
            "ListPullRequests",
            "ListPullRequestFiles",
            "ListIssues",
            "ListIssueComments",
            "ListIssueTimeline",
            "ListTags",
            "ListReleases",
            "GetTagReference", // Gitea returns array for this!
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
        let api = define_gitea_api();

        let single_endpoints = [
            "GetRepository",
            "GetGitTree",
            "GetGitTreeRecursive",
            "GetIssue",
            "GetAnnotatedTag",
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
    fn module_path_is_set() {
        let api = define_gitea_api();
        assert_eq!(api.module_path, Some("gitea".to_string()));
    }

    #[test]
    fn all_list_endpoints_use_gitea_pagination() {
        use schematic_define::params::PaginationStyle;

        let api = define_gitea_api();

        // All endpoints that return Vec types should use Gitea pagination
        // (except GetTagReference which returns array but doesn't paginate)
        let paginated_endpoints = [
            "ListPullRequests",
            "ListPullRequestFiles",
            "ListIssues",
            "ListIssueComments",
            "ListIssueTimeline",
            "ListTags",
            "ListReleases",
            "ListOrgRepos",
        ];

        for id in paginated_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            let params = endpoint
                .params
                .as_ref()
                .unwrap_or_else(|| panic!("Endpoint {} should have params", id));

            assert!(
                params.has_pagination(),
                "Endpoint {} should use pagination",
                id
            );

            // Verify it's Gitea-style pagination (page + limit)
            match &params.pagination {
                Some(PaginationStyle::PageNumber {
                    page_param,
                    per_page_param,
                    default_per_page,
                    max_per_page,
                }) => {
                    assert_eq!(page_param, "page", "Endpoint {} page param", id);
                    assert_eq!(
                        per_page_param, "limit",
                        "Endpoint {} should use 'limit' not 'per_page'",
                        id
                    );
                    assert_eq!(*default_per_page, 50, "Endpoint {} default_per_page", id);
                    assert_eq!(*max_per_page, 100, "Endpoint {} max_per_page", id);
                }
                _ => panic!("Endpoint {} should use PageNumber pagination style", id),
            }
        }
    }

    #[test]
    fn list_org_repos_endpoint() {
        let api = define_gitea_api();
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
    }

    #[test]
    fn no_hardcoded_limit_in_paths() {
        let api = define_gitea_api();

        for endpoint in &api.endpoints {
            // GetGitTreeRecursive is allowed to have ?recursive=true (behavior flag, not pagination)
            if endpoint.id == "GetGitTreeRecursive" {
                continue;
            }

            assert!(
                !endpoint.path.contains("limit="),
                "Endpoint {} has hardcoded limit in path: {}",
                endpoint.id,
                endpoint.path
            );
        }
    }
}
