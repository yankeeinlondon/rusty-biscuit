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
//! - **Branches**: A branch and its head commit
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
//! | Git Trees | `GetGitTree` |
//! | Contents | `GetRepositoryContentRaw` |
//! | Pull Requests | `ListPullRequests`, `ListPullRequestFiles` |
//! | Issues | `ListIssues`, `GetIssue`, `ListIssueComments`, `ListIssueTimeline` |
//! | Branches | `GetBranch` |
//! | Tags/Releases | `ListTags`, `ListReleases`, `GetTagReference`, `GetAnnotatedTag` |
//! | Organizations | `ListOrgRepos` |

mod types;

pub use types::*;

use crate::registry::assemble_module_group;
use schematic_define::{
    AuthStrategy, DefinitionError, Endpoint, EndpointParams, ParamDef,
    RestApi, QueryParamType,
    pagination::{Collection, MoreSignal, PageSize, Strategy, ValueSource},
};

/// Gitea's page-number pagination: a 1-based `page`, a `limit` size the
/// server caps at 50, a `Link` header whose `next` relation marks a further
/// page, and the item count in `X-Total-Count`.
fn gitea_page_number() -> Strategy {
    Strategy::page_number("page", MoreSignal::link_rel("next"))
        .page_size(PageSize::new("limit").server_default(30).max(50))
        .total(ValueSource::header("X-Total-Count"))
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
/// | GetGitTree | GET | /repos/{owner}/{repo}/git/trees/{sha} | Get tree (optionally recursive) |
/// | GetRepositoryContentRaw | GET | /repos/{owner}/{repo}/raw/{filepath} | Get raw file content |
/// | ListPullRequests | GET | /repos/{owner}/{repo}/pulls | List pull requests |
/// | ListPullRequestFiles | GET | /repos/{owner}/{repo}/pulls/{index}/files | List PR files |
/// | ListIssues | GET | /repos/{owner}/{repo}/issues | List issues (excludes PRs with type=issues) |
/// | GetIssue | GET | /repos/{owner}/{repo}/issues/{index} | Get single issue |
/// | ListIssueComments | GET | /repos/{owner}/{repo}/issues/{index}/comments | List comments |
/// | ListIssueTimeline | GET | /repos/{owner}/{repo}/issues/{index}/timeline | List timeline |
/// | ListTags | GET | /repos/{owner}/{repo}/tags | List repository tags |
/// | ListReleases | GET | /repos/{owner}/{repo}/releases | List releases |
/// | GetBranch | GET | /repos/{owner}/{repo}/branches/{branch} | Get a branch and its head commit |
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
    module_group().expect("valid gitea definition").remove(0)
}

/// Assemble the API and its schemas from one ordered registration context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::gitea", |ctx| {
        ctx.register::<RepositoryInfo>()?;
        ctx.register::<GitTreeResponse>()?;
        ctx.register::<PullRequestSummary>()?;
        ctx.register::<PullRequestFile>()?;
        ctx.register::<IssueSummary>()?;
        ctx.register::<IssueComment>()?;
        ctx.register::<TimelineEvent>()?;
        ctx.register::<Label>()?;
        ctx.register::<RepoTag>()?;
        ctx.register::<Release>()?;
        ctx.register::<Branch>()?;
        ctx.register::<GitRef>()?;
        ctx.register::<AnnotatedTagObject>()?;
        let endpoints = vec![

            Endpoint::get("GetRepository", "/repos/{owner}/{repo}")
                .describes("Get repository metadata including default branch")
                .returns_json::<RepositoryInfo>(ctx)
                .build()?,

            Endpoint::get("GetGitTree", "/repos/{owner}/{repo}/git/trees/{sha}")
                .describes("Get a Git tree; set recursive to include nested entries")
                .returns_json::<GitTreeResponse>(ctx)
                .query(ParamDef::new("recursive", QueryParamType::Boolean))
                .pagination(Collection::at("tree").item_type("GitTreeEntry").unpaginated())
                .build()?,

            Endpoint::get("GetRepositoryContentRaw", "/repos/{owner}/{repo}/raw/{+filepath}")
                .describes("Get raw file content from repository")
                .returns_text()
                .build()?,

            Endpoint::get("ListPullRequests", "/repos/{owner}/{repo}/pulls")
                .describes("List pull requests with metadata")
                .returns_json::<Vec<PullRequestSummary>>(ctx)
                .params(EndpointParams::default()
                        .with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)"))
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
                        ),)
                .build()?,
            Endpoint::get("ListPullRequestFiles", "/repos/{owner}/{repo}/pulls/{index}/files")
                .describes("List files changed in a pull request")
                .returns_json::<Vec<PullRequestFile>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)")))
                .build()?,

            Endpoint::get("ListIssues", "/repos/{owner}/{repo}/issues")
                .describes("List issues (use type=issues to exclude PRs)")
                .returns_json::<Vec<IssueSummary>>(ctx)
                .params(EndpointParams::default()
                        .with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)"))
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
                        ),)
                .build()?,
            Endpoint::get("GetIssue", "/repos/{owner}/{repo}/issues/{index}")
                .describes("Get a single issue by index")
                .returns_json::<IssueSummary>(ctx)
                .build()?,
            Endpoint::get("ListIssueComments", "/repos/{owner}/{repo}/issues/{index}/comments")
                .describes("List comments on an issue")
                .returns_json::<Vec<IssueComment>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)")))
                .build()?,
            Endpoint::get("ListIssueTimeline", "/repos/{owner}/{repo}/issues/{index}/timeline")
                .describes("List timeline events for an issue")
                .returns_json::<Vec<TimelineEvent>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)")))
                .build()?,

            Endpoint::get("ListTags", "/repos/{owner}/{repo}/tags")
                .describes("List repository tags")
                .returns_json::<Vec<RepoTag>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)")))
                .build()?,
            Endpoint::get("ListReleases", "/repos/{owner}/{repo}/releases")
                .describes("List releases (linked to tags via tag_name)")
                .returns_json::<Vec<Release>>(ctx)
                .params(EndpointParams::default()
                        .with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)"))
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
                        ),)
                .build()?,

            Endpoint::get("GetBranch", "/repos/{owner}/{repo}/branches/{branch}")
                .describes("Get a branch (commit.id is the branch head commit)")
                .returns_json::<Branch>(ctx)
                .build()?,

            Endpoint::get("GetTagReference", "/repos/{owner}/{repo}/git/refs/{git_ref}")
                .describes("Get tag reference (returns array; check object.type: 'commit' vs 'tag')")
                .returns_json::<Vec<GitRef>>(ctx)
                .pagination(Collection::root().unpaginated())
                .build()?,
            Endpoint::get("GetAnnotatedTag", "/repos/{owner}/{repo}/git/tags/{sha}")
                .describes("Get annotated tag object details (message, tagger)")
                .returns_json::<AnnotatedTagObject>(ctx)
                .build()?,

            Endpoint::get("ListOrgRepos", "/orgs/{org}/repos")
                .describes("List repositories for an organization")
                .returns_json::<Vec<RepositoryInfo>>(ctx)
                .params(EndpointParams::default().with_query_param("page", QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
                        .with_query_param("limit", QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)")))
                .build()?,
        ];
        let api = RestApi::builder(ctx, "Gitea", "https://gitea.example.com/api/v1")
            .describes("Gitea REST API v1.25+ for repository, PR, issue, and release workflows")
            .authentication(AuthStrategy::ApiKey {
            header: "Authorization".to_string(),
            value_prefix: Some("token ".to_string()),
        }, vec!["GITEA_TOKEN".to_string()])
            .docs_url("https://docs.gitea.com/api/1.25/")
            .module_path("gitea".to_string())
            .default_pagination(gitea_page_number())
            .header("Accept".to_string(), "application/json".to_string())
            .endpoints(endpoints)
            .build()?;
        Ok(vec![api])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{ApiResponse, RestMethod, TypeRef};
    use schematic_define::pagination::{resolve, Resolution};

    // =============================================
    // api.schemas tests
    // =============================================

    #[test]
    fn schemas_contain_all_response_types() {
        let api = define_gitea_api();

        assert!(api.component("RepositoryInfo").is_some());
        assert!(api.component("GitTreeResponse").is_some());
        assert!(api.component("PullRequestSummary").is_some());
        assert!(api.component("Vec<PullRequestSummary>").is_none());
        assert!(api.component("IssueSummary").is_some());
        assert!(api.component("Vec<IssueSummary>").is_none());
        assert!(api.component("Label").is_some());
        assert!(api.component("RepoTag").is_some());
        assert!(api.component("Release").is_some());
        assert_eq!(crate::registry::registered_names(&api).len(), 13);
    }

    #[test]
    fn references_resolve_in_api_schemas() {
        let api = define_gitea_api();

        api.validate_references().expect("every reference resolves in api.schemas");
    }

    #[test]
    fn openapi_export_includes_api_schemas() {
        let api = define_gitea_api();
        let openapi_schemas = schematic_define::openapi::export(&api, &schematic_define::openapi::ExportOptions::new())
            .expect("export succeeds").components.expect("components").schemas;

        assert!(
            openapi_schemas.len() >= 13,
            "expected at least 13 schemas, got {}",
            openapi_schemas.len()
        );
        assert!(openapi_schemas.contains_key("RepositoryInfo"));
        assert!(openapi_schemas.contains_key("PullRequestSummary"));
        assert!(!openapi_schemas.contains_key("Vec<PullRequestSummary>"));
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
    fn git_tree_endpoint_has_optional_recursive_query() {
        let api = define_gitea_api();
        let tree = api.endpoints.iter().find(|e| e.id == "GetGitTree").unwrap();
        assert_eq!(tree.path, "/repos/{owner}/{repo}/git/trees/{sha}");
        assert!(api.endpoints.iter().all(|e| e.id != "GetGitTreeRecursive"));
        let recursive = tree.params.as_ref().unwrap().query.iter().find(|p| p.name == "recursive").unwrap();
        assert!(!recursive.required);
        assert_eq!(recursive.param_type, QueryParamType::Boolean);
        let resolved = resolve(&api, tree);
        assert!(matches!(resolved, Resolution::Unpaginated { .. }));
        assert_eq!(resolved.list().unwrap().item_type.as_deref(), Some("GitTreeEntry"));
        assert!(schematic_define::pagination::caller_knobs(&resolved).is_empty());
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
        assert!(params.query.iter().any(|p| p.name == "page"));
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
        assert!(params.query.iter().any(|p| p.name == "page"));
    }

    #[test]
    fn issue_endpoints_use_pagination_and_type_filter() {
        let api = define_gitea_api();

        let list = api.endpoints.iter().find(|e| e.id == "ListIssues").unwrap();
        // Clean path without hardcoded query params
        assert_eq!(list.path, "/repos/{owner}/{repo}/issues");
        // Pagination and filters in params field
        let params = list.params.as_ref().expect("ListIssues should have params");
        assert!(params.query.iter().any(|p| p.name == "page"));
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
        assert!(params.query.iter().any(|p| p.name == "page"));

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
        assert!(params.query.iter().any(|p| p.name == "page"));
    }

    #[test]
    fn tag_and_release_endpoints_use_pagination() {
        let api = define_gitea_api();

        let tags = api.endpoints.iter().find(|e| e.id == "ListTags").unwrap();
        // Clean path without hardcoded query params
        assert_eq!(tags.path, "/repos/{owner}/{repo}/tags");
        let params = tags.params.as_ref().expect("ListTags should have params");
        assert!(params.query.iter().any(|p| p.name == "page"));

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
        assert!(params.query.iter().any(|p| p.name == "page"));
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
                    matches!(schema, TypeRef::Array(_)),
                    "GetTagReference should return Vec<GitRef>, got {:?}",
                    schema
                );
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[test]
    fn get_branch_endpoint() {
        let api = define_gitea_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetBranch")
            .expect("GetBranch endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/repos/{owner}/{repo}/branches/{branch}");
        assert!(endpoint.request.is_none());
        match &endpoint.response {
            ApiResponse::Json(schema) => assert!(*schema == TypeRef::Named("Branch".into())),
            _ => panic!("Expected JSON response"),
        }
        assert!(matches!(api.auth, AuthStrategy::ApiKey { .. }));
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
                        matches!(schema, TypeRef::Array(_)),
                        "Endpoint {} should return Vec type, got {:?}",
                        id,
                        schema
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
            "GetIssue",
            "GetBranch",
            "GetAnnotatedTag",
        ];

        for id in single_endpoints {
            let endpoint = api.endpoints.iter().find(|e| e.id == id).unwrap();
            // GetRepositoryContentRaw returns Text, which is fine; only
            // assert single-item shape for JSON responses.
            if let ApiResponse::Json(schema) = &endpoint.response {
                assert!(
                    !matches!(schema, TypeRef::Array(_)),
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
                params.query.iter().any(|p| p.name == "page"),
                "Endpoint {} should use pagination",
                id
            );

            // Verify it's Gitea-style pagination (page + limit)
            assert!(params.query.iter().any(|p| p.name == "limit"));
            let resolution = resolve(&api, endpoint);
            assert!(matches!(resolution, Resolution::Paginated { .. }));
            let knobs = schematic_define::pagination::caller_knobs(&resolution);
            assert!(knobs.iter().any(|k| matches!(k, schematic_define::pagination::Knob::StartPage { .. })));
            assert!(knobs.iter().any(|k| matches!(k, schematic_define::pagination::Knob::PageSize { param, .. } if param == "limit")));
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
                    matches!(schema, TypeRef::Array(_)),
                    "ListOrgRepos should return Vec type, got {:?}",
                    schema
                );
            }
            _ => panic!("Expected JSON response"),
        }

        let params = endpoint.params.as_ref().expect("should have params");
        assert!(params.query.iter().any(|p| p.name == "page"));
    }

    #[test]
    fn no_hardcoded_limit_in_paths() {
        let api = define_gitea_api();

        for endpoint in &api.endpoints {

            assert!(
                !endpoint.path.contains("limit="),
                "Endpoint {} has hardcoded limit in path: {}",
                endpoint.id,
                endpoint.path
            );
        }
    }
    #[test]
    fn typed_module_group_preserves_endpoint_order_and_completeness() {
        let group = module_group().unwrap();
        assert_eq!(group.len(), 1);
        let api = &group[0];
        assert_eq!(api.endpoints.iter().map(|e| e.id.as_str()).collect::<Vec<_>>(), vec!["GetRepository", "GetGitTree", "GetRepositoryContentRaw", "ListPullRequests", "ListPullRequestFiles", "ListIssues", "GetIssue", "ListIssueComments", "ListIssueTimeline", "ListTags", "ListReleases", "GetBranch", "GetTagReference", "GetAnnotatedTag", "ListOrgRepos"]);
        assert_eq!(crate::registry::registered_names(&group[0]), vec!["RepositoryInfo", "GitTreeResponse", "PullRequestSummary", "Label", "PullRequestFile", "IssueSummary", "IssueComment", "TimelineEvent", "RepoTag", "Release", "Branch", "GitRef", "AnnotatedTagObject"]);
        api.validate_references().unwrap();
        assert!(api.endpoints.iter().all(|e| !e.path.contains('?')));
    }

}
