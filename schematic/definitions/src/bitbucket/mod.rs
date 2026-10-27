//! Bitbucket Cloud REST API definition.
//!
//! This module provides a focused definition of the Bitbucket Cloud REST API v2.0,
//! optimized for common developer workflows: repository info, pull requests,
//! issues, tags, and downloads.
//!
//! ## API Overview
//!
//! The Bitbucket REST API provides access to:
//!
//! - **Repository metadata**: Basic repo info, default branch, workspace/project
//! - **Directory contents**: List files/folders at any commit
//! - **File content**: Raw file content with proper Accept headers
//! - **Pull Requests**: List PRs with associated metadata and comments
//! - **Issues**: List issues, comments, and change history
//! - **Branches**: Retrieve a branch and its head commit
//! - **Tags**: List and retrieve repository tags (lightweight and annotated)
//! - **Downloads**: List and retrieve release artifacts
//!
//! ## Authentication
//!
//! Uses Basic authentication via `BITBUCKET_USERNAME` and `BITBUCKET_APP_PASSWORD`
//! environment variables. App Passwords can be created at:
//! <https://bitbucket.org/account/settings/app-passwords/>
//!
//! ## Pagination
//!
//! Most list endpoints return `PaginatedResponse<T>` with a `values` array and
//! optional `next` and `previous` URLs. The generated pager follows those
//! links; the `pagelen` page size is a pager control, not a request setter,
//! and `page` stays an ordinary request field:
//!
//! ```text
//! let pull_requests = bitbucket
//!     .list_pull_requests("workspace", "repo")
//!     .page_size(100)
//!     .collect_all()
//!     .await?;
//! ```
//!
//! ## Endpoint Coverage (15 endpoints)
//!
//! | Category | Endpoints |
//! |----------|-----------|
//! | Repository | `GetRepository` |
//! | Contents | `GetSource` |
//! | Pull Requests | `ListPullRequests`, `GetPullRequest`, `ListPullRequestComments` |
//! | Issues | `ListIssues`, `GetIssue`, `ListIssueComments`, `ListIssueChanges` |
//! | Branches | `GetBranch` |
//! | Tags | `ListTags`, `GetTag` |
//! | Downloads | `ListDownloads`, `GetDownload` |
//! | Workspaces | `ListWorkspaceRepos` |

mod types;

pub use types::*;

use crate::registry::assemble_module_group;
use schematic_define::{DefinitionContext, DefinitionError};
#[cfg(test)]
use schematic_define::{ResponseShape, RestMethod};
use schematic_define::{
    Endpoint, EndpointParams, EnvList, EnvMapping,
    RestApi, Success,
    pagination::{Collection, LinkSource, PageSize, Strategy, ValueSource},
};

/// Creates Bitbucket-style pagination parameters.
fn bitbucket_pagination() -> EndpointParams {
    EndpointParams::new()
        .with_query_param("page", schematic_define::QueryParamType::Integer, false, Some("Page number (1-indexed, default: 1)"))
        .with_query_param("pagelen", schematic_define::QueryParamType::Integer, false, Some("Items per page (default: 50, max: 100)"))
}

/// Bitbucket's next-link pagination: the body's `next` and `previous` URLs,
/// a `pagelen` size, and the item count in the body's `size`.
fn bitbucket_next_link() -> Strategy {
    Strategy::next_link(LinkSource::body("next"))
        .prev(LinkSource::body("previous"))
        .page_size(PageSize::new("pagelen").server_default(10).max(100))
        .total(ValueSource::body("size"))
}

/// Creates the Bitbucket Cloud REST API definition.
///
/// This defines a focused slice of the Bitbucket REST API covering repository
/// metadata, file discovery, pull requests, issues, tags, and downloads.
///
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | GetRepository | GET | /repositories/{workspace}/{repo_slug} | Get repository metadata |
/// | GetSource | GET | /repositories/{workspace}/{repo_slug}/src/{commit}/{path} | Directory listing or raw file content |
/// | ListPullRequests | GET | /repositories/{workspace}/{repo_slug}/pullrequests | List pull requests |
/// | GetPullRequest | GET | /repositories/{workspace}/{repo_slug}/pullrequests/{id} | Get single PR |
/// | ListPullRequestComments | GET | /repositories/{workspace}/{repo_slug}/pullrequests/{id}/comments | List PR comments |
/// | ListIssues | GET | /repositories/{workspace}/{repo_slug}/issues | List issues |
/// | GetIssue | GET | /repositories/{workspace}/{repo_slug}/issues/{id} | Get single issue |
/// | ListIssueComments | GET | /repositories/{workspace}/{repo_slug}/issues/{id}/comments | List issue comments |
/// | ListIssueChanges | GET | /repositories/{workspace}/{repo_slug}/issues/{id}/changes | List issue changes |
/// | GetBranch | GET | /repositories/{workspace}/{repo_slug}/refs/branches/{name} | Get single branch |
/// | ListTags | GET | /repositories/{workspace}/{repo_slug}/refs/tags | List repository tags |
/// | GetTag | GET | /repositories/{workspace}/{repo_slug}/refs/tags/{name} | Get single tag |
/// | ListDownloads | GET | /repositories/{workspace}/{repo_slug}/downloads | List downloads |
/// | GetDownload | GET | /repositories/{workspace}/{repo_slug}/downloads/{filename} | Download file |
/// | ListWorkspaceRepos | GET | /repositories/{workspace} | List workspace repositories |
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::bitbucket::define_bitbucket_api;
///
/// let api = define_bitbucket_api();
/// assert_eq!(api.name, "Bitbucket");
/// assert_eq!(api.endpoints.len(), 15);
/// ```
pub fn define_bitbucket_api() -> RestApi {
    module_group().expect("valid bitbucket definitions").remove(0)
}

fn build_bitbucket_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = vec![

            Endpoint::get("GetRepository", "/repositories/{workspace}/{repo_slug}")
                .describes("Get repository metadata including default branch and workspace info")
                .returns_json::<Repository>(ctx)
                .build()?,

            Endpoint::get("GetSource", "/repositories/{workspace}/{repo_slug}/src/{commit}/{path}")
                .describes("Get a directory listing (paginated JSON of SourceEntry) or the \
                              raw content of a file at a specific commit")
                .returns(Success::json(ctx.register::<PaginatedResponse<SourceEntry>>()?).variant("Directory").note(
                    "returned for a directory: its paginated listing. Bitbucket serves a file in the media type \
                     its name implies, so a committed `.json` file also arrives as `application/json` and selects \
                     this shape: a JSON object decodes as an empty listing, other JSON fails to decode. No header \
                     tells the two apart; read a file's bytes with `send_raw`"))
                .returns(Success::binary().fallback().variant("File")
                    .note("returned for a file: its raw content, in the media type Bitbucket derives from the file name"))
                .params(bitbucket_pagination())
                .build()?,

            Endpoint::get("ListPullRequests", "/repositories/{workspace}/{repo_slug}/pullrequests")
                .describes("List pull requests (all states)")
                .returns_json::<PaginatedResponse<PullRequest>>(ctx)
                .params(bitbucket_pagination().with_query_param(
                    "state",
                    schematic_define::QueryParamType::String,
                    false,
                    Some("Filter by state (OPEN, MERGED, DECLINED, SUPERSEDED)"),
                ))
                .pagination(Collection::at("values").item_type("PullRequest"))
                .build()?,
            Endpoint::get("GetPullRequest", "/repositories/{workspace}/{repo_slug}/pullrequests/{id}")
                .describes("Get a single pull request by ID")
                .returns_json::<PullRequest>(ctx)
                .build()?,
            Endpoint::get("ListPullRequestComments", "/repositories/{workspace}/{repo_slug}/pullrequests/{id}/comments")
                .describes("List comments on a pull request")
                .returns_json::<PaginatedResponse<PullRequestComment>>(ctx)
                .params(bitbucket_pagination())
                .pagination(Collection::at("values").item_type("PullRequestComment"))
                .build()?,

            Endpoint::get("ListIssues", "/repositories/{workspace}/{repo_slug}/issues")
                .describes("List repository issues")
                .returns_json::<PaginatedResponse<Issue>>(ctx)
                .params(bitbucket_pagination())
                .pagination(Collection::at("values").item_type("Issue"))
                .build()?,
            Endpoint::get("GetIssue", "/repositories/{workspace}/{repo_slug}/issues/{id}")
                .describes("Get a single issue by ID")
                .returns_json::<Issue>(ctx)
                .build()?,
            Endpoint::get("ListIssueComments", "/repositories/{workspace}/{repo_slug}/issues/{id}/comments")
                .describes("List comments on an issue")
                .returns_json::<PaginatedResponse<IssueComment>>(ctx)
                .params(bitbucket_pagination())
                .pagination(Collection::at("values").item_type("IssueComment"))
                .build()?,
            Endpoint::get("ListIssueChanges", "/repositories/{workspace}/{repo_slug}/issues/{id}/changes")
                .describes("List change history for an issue")
                .returns_json::<PaginatedResponse<IssueChange>>(ctx)
                .params(bitbucket_pagination())
                .pagination(Collection::at("values").item_type("IssueChange"))
                .build()?,

            Endpoint::get("GetBranch", "/repositories/{workspace}/{repo_slug}/refs/branches/{name}")
                .describes("Get a single branch by name (target.hash is the branch head commit)")
                .returns_json::<Branch>(ctx)
                .build()?,

            Endpoint::get("ListTags", "/repositories/{workspace}/{repo_slug}/refs/tags")
                .describes("List repository tags")
                .returns_json::<PaginatedResponse<Tag>>(ctx)
                .params(bitbucket_pagination())
                .pagination(Collection::at("values").item_type("Tag"))
                .build()?,
            Endpoint::get("GetTag", "/repositories/{workspace}/{repo_slug}/refs/tags/{name}")
                .describes("Get a single tag by name")
                .returns_json::<Tag>(ctx)
                .build()?,

            Endpoint::get("ListDownloads", "/repositories/{workspace}/{repo_slug}/downloads")
                .describes("List repository downloads (release artifacts)")
                .returns_json::<PaginatedResponse<Download>>(ctx)
                .params(bitbucket_pagination())
                .pagination(Collection::at("values").item_type("Download"))
                .build()?,
            Endpoint::get("GetDownload", "/repositories/{workspace}/{repo_slug}/downloads/{filename}")
                .describes("Download a file from repository downloads")
                .returns_binary()
                .build()?,

            Endpoint::get("ListWorkspaceRepos", "/repositories/{workspace}")
                .describes("List repositories in a workspace")
                .returns_json::<PaginatedResponse<Repository>>(ctx)
                .params(bitbucket_pagination())
                .pagination(Collection::at("values").item_type("Repository"))
                .build()?,
        ];
    RestApi::builder(ctx, "Bitbucket", "https://api.bitbucket.org/2.0")
        .describes("Bitbucket Cloud REST API v2.0 for repository, PR, issue, and tag workflows")
        .basic_env("BITBUCKET_USERNAME", ["BITBUCKET_APP_PASSWORD"])
        .docs_url("https://developer.atlassian.com/cloud/bitbucket/rest/")
        .module_path("bitbucket")
        .env_mapping(EnvMapping {
            basic_user: Some(EnvList::new(vec!["BITBUCKET_USERNAME".to_string()])),
            basic_pass: Some(EnvList::new(vec!["BITBUCKET_APP_PASSWORD".to_string()])),
            ..Default::default()
        })
        .default_pagination(bitbucket_next_link())
        .endpoints(endpoints)
        .build()
}

/// Assemble sibling APIs and their ordered nominal schemas in one context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::bitbucket", |ctx| {
        ctx.register::<Repository>()?;
        ctx.register::<PaginatedResponse<SourceEntry>>()?;
        ctx.register::<PaginatedResponse<PullRequest>>()?;
        ctx.register::<PullRequest>()?;
        ctx.register::<PaginatedResponse<PullRequestComment>>()?;
        ctx.register::<PaginatedResponse<Issue>>()?;
        ctx.register::<Issue>()?;
        ctx.register::<PaginatedResponse<IssueComment>>()?;
        ctx.register::<PaginatedResponse<IssueChange>>()?;
        ctx.register::<PaginatedResponse<Tag>>()?;
        ctx.register::<Branch>()?;
        ctx.register::<Tag>()?;
        ctx.register::<PaginatedResponse<Download>>()?;
        ctx.register::<PaginatedResponse<Repository>>()?;
        Ok(vec![
            build_bitbucket_api(ctx)?,
        ])
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use schematic_define::{AuthStrategy, TypeRef};

    // =============================================
    // api.schemas tests
    // =============================================

    #[test]
    fn schemas_contain_all_response_types() {
        let api = define_bitbucket_api();

        assert!(api.component("Repository").is_some());
        assert!(api.component("PaginatedResponse<SourceEntry>").is_some());
        assert!(api.component("PaginatedResponse<PullRequest>").is_some());
        assert!(api.component("PullRequest").is_some());
        assert!(api.component("Issue").is_some());
        assert!(api.component("Tag").is_some());
        assert_eq!(crate::registry::registered_names(&api).len(), 14);
    }

    #[test]
    fn references_resolve_in_api_schemas() {
        let api = define_bitbucket_api();

        api.validate_references().expect("every reference resolves in api.schemas");
    }

    #[test]
    fn openapi_export_includes_api_schemas() {
        let api = define_bitbucket_api();
        let openapi_schemas = schematic_define::openapi::export(&api, &schematic_define::openapi::ExportOptions::new())
            .expect("export succeeds").components.expect("components").schemas;

        assert!(
            openapi_schemas.len() >= 13,
            "expected at least 13 schemas, got {}",
            openapi_schemas.len()
        );
        assert!(openapi_schemas.contains_key("Repository"));
        assert!(openapi_schemas.contains_key("PullRequest"));
        assert!(openapi_schemas.contains_key("Issue"));
        assert!(openapi_schemas.contains_key("Tag"));
    }

    // =============================================
    // API definition tests
    // =============================================

    #[test]
    fn api_has_correct_metadata() {
        let api = define_bitbucket_api();

        assert_eq!(api.name, "Bitbucket");
        assert_eq!(api.base_url, "https://api.bitbucket.org/2.0");
        assert!(api.docs_url.is_some());
        assert!(
            api.docs_url
                .as_ref()
                .unwrap()
                .contains("developer.atlassian.com")
        );
    }

    #[test]
    fn api_uses_basic_auth() {
        let api = define_bitbucket_api();

        assert_eq!(api.auth, AuthStrategy::Basic { username_env: EnvList::single("BITBUCKET_USERNAME"), password_env: EnvList::single("BITBUCKET_APP_PASSWORD") });
        assert!(api.env_auth.is_empty());
    }

    #[test]
    fn api_has_fifteen_endpoints() {
        let api = define_bitbucket_api();
        assert_eq!(api.endpoints.len(), 15);
    }

    #[test]
    fn get_repository_endpoint() {
        let api = define_bitbucket_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetRepository")
            .expect("GetRepository endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/repositories/{workspace}/{repo_slug}");
        assert!(endpoint.request.is_none());
        assert!(matches!(endpoint.response[0].shape, ResponseShape::Json { .. }));
    }

    #[test]
    fn source_endpoint_serves_directories_and_files_as_binary() {
        let api = define_bitbucket_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetSource")
            .expect("GetSource endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(
            endpoint.path,
            "/repositories/{workspace}/{repo_slug}/src/{commit}/{path}"
        );
        // R32a: a JSON directory listing, or any other media type as the file.
        let shapes: Vec<_> = endpoint.response.iter()
            .map(|shape| (&shape.shape, shape.variant.as_deref(), shape.fallback, shape.content_type.as_deref()))
            .collect();
        assert_eq!(shapes, [
            (&ResponseShape::Json(TypeRef::Named("PaginatedResponse<SourceEntry>".into())), Some("Directory"), false, None),
            (&ResponseShape::Binary, Some("File"), true, None),
        ]);
        assert!(endpoint.response[0].note.as_deref().is_some_and(|note| note.contains("`.json` file")), "the listing note records the R32a limitation");
        assert!(endpoint.response[1].note.is_some());
        // No `Accept` override: it would break the JSON directory listing.
        assert!(endpoint.headers.is_empty());
        // Directory listings page through `page` / `pagelen`.
        let params = endpoint
            .params
            .as_ref()
            .expect("Should have pagination params");
        assert!(params.query.iter().any(|param| param.name == "pagelen"));

        for removed in ["ListDirectoryContents", "GetFileContentRaw"] {
            assert!(api.endpoints.iter().all(|e| e.id != removed), "{removed}");
        }
    }

    #[test]
    fn pull_request_endpoints() {
        let api = define_bitbucket_api();

        let list_prs = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequests")
            .expect("ListPullRequests endpoint missing");
        // No hardcoded query params - state is now a query param
        assert!(!list_prs.path.contains("?"));
        // Check state param exists
        let params = list_prs.params.as_ref().expect("Should have params");
        assert!(params.query.iter().any(|p| p.name == "state"));
        assert!(params.query.iter().any(|param| param.name == "pagelen"));

        let get_pr = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetPullRequest")
            .expect("GetPullRequest endpoint missing");
        assert!(get_pr.path.contains("{id}"));

        let comments = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequestComments")
            .expect("ListPullRequestComments endpoint missing");
        assert!(comments.path.contains("{id}/comments"));
    }

    #[test]
    fn issue_endpoints() {
        let api = define_bitbucket_api();

        let list_issues = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssues")
            .expect("ListIssues endpoint missing");
        // No hardcoded query params - pagination is in params field
        assert!(!list_issues.path.contains("?"));
        assert!(list_issues.params.as_ref().unwrap().query.iter().any(|param| param.name == "pagelen"));

        let get_issue = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetIssue")
            .expect("GetIssue endpoint missing");
        assert!(get_issue.path.contains("{id}"));

        let comments = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueComments")
            .expect("ListIssueComments endpoint missing");
        assert!(comments.path.contains("{id}/comments"));

        let changes = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueChanges")
            .expect("ListIssueChanges endpoint missing");
        assert!(changes.path.contains("{id}/changes"));
    }

    #[test]
    fn tag_endpoints() {
        let api = define_bitbucket_api();

        let list_tags = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListTags")
            .expect("ListTags endpoint missing");
        assert!(list_tags.path.contains("refs/tags"));
        // No hardcoded query params - pagination is in params field
        assert!(!list_tags.path.contains("?"));
        assert!(list_tags.params.as_ref().unwrap().query.iter().any(|param| param.name == "pagelen"));

        let get_tag = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetTag")
            .expect("GetTag endpoint missing");
        assert!(get_tag.path.contains("{name}"));
    }

    #[test]
    fn get_branch_endpoint() {
        let api = define_bitbucket_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetBranch")
            .expect("GetBranch endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(
            endpoint.path,
            "/repositories/{workspace}/{repo_slug}/refs/branches/{name}"
        );
        assert!(endpoint.request.is_none());
        match &endpoint.response[0].shape {
            ResponseShape::Json(reference) => assert_eq!(reference, &TypeRef::Named("Branch".into())),
            _ => panic!("Expected JSON response"),
        }
        assert!(matches!(api.auth, AuthStrategy::Basic { .. }));
    }

    #[test]
    fn download_endpoints() {
        let api = define_bitbucket_api();

        let list_downloads = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListDownloads")
            .expect("ListDownloads endpoint missing");
        assert!(list_downloads.path.contains("downloads"));

        let get_download = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetDownload")
            .expect("GetDownload endpoint missing");
        assert!(get_download.path.contains("{filename}"));
    }

    #[test]
    fn get_download_returns_binary() {
        let api = define_bitbucket_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetDownload")
            .expect("GetDownload endpoint missing");

        assert!(matches!(endpoint.response[0].shape, ResponseShape::Binary));
    }

    #[test]
    fn env_mapping_configured() {
        let api = define_bitbucket_api();

        let mapping = api.env_mapping.as_ref().expect("env_mapping should be set");

        let basic_user = mapping
            .basic_user
            .as_ref()
            .expect("basic_user mapping should be set");
        assert!(
            basic_user
                .names()
                .contains(&"BITBUCKET_USERNAME".to_string())
        );

        let basic_pass = mapping
            .basic_pass
            .as_ref()
            .expect("basic_pass mapping should be set");
        assert!(
            basic_pass
                .names()
                .contains(&"BITBUCKET_APP_PASSWORD".to_string())
        );
    }

    #[test]
    fn all_endpoints_have_descriptions() {
        let api = define_bitbucket_api();

        for endpoint in &api.endpoints {
            assert!(
                !endpoint.description.is_empty(),
                "Endpoint {} missing description",
                endpoint.id
            );
        }
    }

    #[test]
    fn list_endpoints_return_paginated_response() {
        let api = define_bitbucket_api();

        let list_endpoints = [
            "ListPullRequests",
            "ListPullRequestComments",
            "ListIssues",
            "ListIssueComments",
            "ListIssueChanges",
            "ListTags",
            "ListDownloads",
            "ListWorkspaceRepos",
        ];

        for id in list_endpoints {
            let endpoint = api
                .endpoints
                .iter()
                .find(|e| e.id == id)
                .unwrap_or_else(|| panic!("Endpoint {} not found", id));

            match &endpoint.response[0].shape {
                ResponseShape::Json(reference) => {
                    assert!(
                        matches!(reference, TypeRef::Named(name) if name.starts_with("PaginatedResponse<")),
                        "Endpoint {id} should return PaginatedResponse type, got {reference:?}",
                    );
                }
                _ => panic!("Endpoint {} should return JSON", id),
            }
        }
    }

    #[test]
    fn single_item_endpoints_return_non_paginated_types() {
        let api = define_bitbucket_api();

        let single_endpoints = [
            ("GetRepository", "Repository"),
            ("GetPullRequest", "PullRequest"),
            ("GetIssue", "Issue"),
            ("GetBranch", "Branch"),
            ("GetTag", "Tag"),
        ];

        for (id, expected_type) in single_endpoints {
            let endpoint = api
                .endpoints
                .iter()
                .find(|e| e.id == id)
                .unwrap_or_else(|| panic!("Endpoint {} not found", id));

            // GetSource and GetDownload return Binary;
            // only assert single-item shape for JSON responses.
            if let ResponseShape::Json(reference) = &endpoint.response[0].shape {
                assert!(
                    !matches!(reference, TypeRef::Named(name) if name.starts_with("PaginatedResponse<")),
                    "Endpoint {} should return single item, not PaginatedResponse",
                    id
                );
                assert_eq!(reference, &TypeRef::Named(expected_type.into()), "Endpoint {id} should return {expected_type}");
            }
        }
    }

    #[test]
    fn module_path_is_set() {
        let api = define_bitbucket_api();
        assert_eq!(api.module_path, Some("bitbucket".to_string()));
    }

    #[test]
    fn all_endpoint_ids_are_unique() {
        let api = define_bitbucket_api();
        let mut ids = std::collections::HashSet::new();

        for endpoint in &api.endpoints {
            assert!(
                ids.insert(&endpoint.id),
                "Duplicate endpoint ID: {}",
                endpoint.id
            );
        }
    }

    #[test]
    fn list_endpoints_have_pagination_params() {
        let api = define_bitbucket_api();

        let list_endpoints = [
            "GetSource",
            "ListPullRequests",
            "ListPullRequestComments",
            "ListIssues",
            "ListIssueComments",
            "ListIssueChanges",
            "ListTags",
            "ListDownloads",
            "ListWorkspaceRepos",
        ];

        for id in list_endpoints {
            let endpoint = api
                .endpoints
                .iter()
                .find(|e| e.id == id)
                .unwrap_or_else(|| panic!("Endpoint {} not found", id));

            let params = endpoint
                .params
                .as_ref()
                .unwrap_or_else(|| panic!("Endpoint {} should have params", id));

            assert!(
                params.query.iter().any(|param| param.name == "pagelen"),
                "Endpoint {} should have pagination params",
                id
            );
        }
    }

    #[test]
    fn list_workspace_repos_endpoint() {
        let api = define_bitbucket_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListWorkspaceRepos")
            .expect("ListWorkspaceRepos endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/repositories/{workspace}");

        match &endpoint.response[0].shape {
            ResponseShape::Json(reference) => {
                assert_eq!(reference, &TypeRef::Named("PaginatedResponse<Repository>".into()));
                let rust_type: String = api.rust_type(reference).expect("registered component").split_whitespace().collect();
                assert!(rust_type.contains("bitbucket::PaginatedResponse<") && rust_type.contains("bitbucket::Repository>"), "{rust_type}");
            }
            _ => panic!("Expected JSON response"),
        }

        let params = endpoint.params.as_ref().expect("should have params");
        assert!(params.query.iter().any(|param| param.name == "pagelen"));
    }

    #[test]
    fn paths_have_no_hardcoded_query_params() {
        let api = define_bitbucket_api();

        for endpoint in &api.endpoints {
            assert!(
                !endpoint.path.contains("?"),
                "Endpoint {} should not have hardcoded query params in path: {}",
                endpoint.id,
                endpoint.path
            );
        }
    }

    #[test]
    fn module_group_preserves_endpoint_and_registry_inventory() {
        let group = module_group().unwrap();
        assert_eq!(
            group.iter().flat_map(|api| api.endpoints.iter().map(|endpoint| endpoint.id.as_str())).collect::<Vec<_>>(),
            vec!["GetRepository", "GetSource", "ListPullRequests", "GetPullRequest", "ListPullRequestComments", "ListIssues", "GetIssue", "ListIssueComments", "ListIssueChanges", "GetBranch", "ListTags", "GetTag", "ListDownloads", "GetDownload", "ListWorkspaceRepos"]
        );
        assert_eq!(crate::registry::registered_names(&group[0]), vec!["Repository", "PaginatedResponse<SourceEntry>", "PaginatedResponse<PullRequest>", "PullRequest", "PaginatedResponse<PullRequestComment>", "PaginatedResponse<Issue>", "Issue", "PaginatedResponse<IssueComment>", "PaginatedResponse<IssueChange>", "PaginatedResponse<Tag>", "Tag", "Branch", "PaginatedResponse<Download>", "PaginatedResponse<Repository>"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
    }

}
