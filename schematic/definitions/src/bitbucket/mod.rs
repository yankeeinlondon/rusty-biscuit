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
//! Bitbucket uses cursor-based pagination with a `next` URL field. Most list
//! endpoints return `PaginatedResponse<T>` with a `values` array and optional
//! `next` URL for the next page.
//!
//! ## Endpoint Coverage (14 endpoints)
//!
//! | Category | Endpoints |
//! |----------|-----------|
//! | Repository | `GetRepository` |
//! | Contents | `ListDirectoryContents`, `GetFileContentRaw` |
//! | Pull Requests | `ListPullRequests`, `GetPullRequest`, `ListPullRequestComments` |
//! | Issues | `ListIssues`, `GetIssue`, `ListIssueComments`, `ListIssueChanges` |
//! | Tags | `ListTags`, `GetTag` |
//! | Downloads | `ListDownloads`, `GetDownload` |

mod types;

pub use types::*;

use schematic_define::{
    ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi, RestMethod,
};

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
/// | ListDirectoryContents | GET | /repositories/{workspace}/{repo_slug}/src/{commit}/{path} | List directory contents |
/// | GetFileContentRaw | GET | /repositories/{workspace}/{repo_slug}/src/{commit}/{path} | Get raw file content |
/// | ListPullRequests | GET | /repositories/{workspace}/{repo_slug}/pullrequests | List pull requests |
/// | GetPullRequest | GET | /repositories/{workspace}/{repo_slug}/pullrequests/{id} | Get single PR |
/// | ListPullRequestComments | GET | /repositories/{workspace}/{repo_slug}/pullrequests/{id}/comments | List PR comments |
/// | ListIssues | GET | /repositories/{workspace}/{repo_slug}/issues | List issues |
/// | GetIssue | GET | /repositories/{workspace}/{repo_slug}/issues/{id} | Get single issue |
/// | ListIssueComments | GET | /repositories/{workspace}/{repo_slug}/issues/{id}/comments | List issue comments |
/// | ListIssueChanges | GET | /repositories/{workspace}/{repo_slug}/issues/{id}/changes | List issue changes |
/// | ListTags | GET | /repositories/{workspace}/{repo_slug}/refs/tags | List repository tags |
/// | GetTag | GET | /repositories/{workspace}/{repo_slug}/refs/tags/{name} | Get single tag |
/// | ListDownloads | GET | /repositories/{workspace}/{repo_slug}/downloads | List downloads |
/// | GetDownload | GET | /repositories/{workspace}/{repo_slug}/downloads/{filename} | Download file |
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::bitbucket::define_bitbucket_api;
///
/// let api = define_bitbucket_api();
/// assert_eq!(api.name, "Bitbucket");
/// assert_eq!(api.endpoints.len(), 14);
/// ```
pub fn define_bitbucket_api() -> RestApi {
    RestApi {
        name: "Bitbucket".to_string(),
        description:
            "Bitbucket Cloud REST API v2.0 for repository, PR, issue, and tag workflows"
                .to_string(),
        base_url: "https://api.bitbucket.org/2.0".to_string(),
        docs_url: Some("https://developer.atlassian.com/cloud/bitbucket/rest/".to_string()),
        auth: AuthStrategy::Basic,
        env_auth: vec!["BITBUCKET_APP_PASSWORD".to_string()],
        env_username: Some("BITBUCKET_USERNAME".to_string()),
        headers: vec![],
        endpoints: vec![
            // =================================================================
            // Repository Metadata
            // =================================================================
            Endpoint {
                id: "GetRepository".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}".to_string(),
                description: "Get repository metadata including default branch and workspace info"
                    .to_string(),
                request: None,
                response: ApiResponse::json_type("Repository"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Directory and File Contents
            // =================================================================
            Endpoint {
                id: "ListDirectoryContents".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/src/{commit}/{path}?pagelen=50"
                    .to_string(),
                description: "List directory contents at a specific commit".to_string(),
                request: None,
                response: ApiResponse::json_type("PaginatedResponse<SourceEntry>"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetFileContentRaw".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/src/{commit}/{path}".to_string(),
                description: "Get raw file content at a specific commit".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![("Accept".to_string(), "text/plain".to_string())],
                params: None,
            },
            // =================================================================
            // Pull Requests
            // =================================================================
            Endpoint {
                id: "ListPullRequests".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/pullrequests?state=all&pagelen=50"
                    .to_string(),
                description: "List pull requests (all states)".to_string(),
                request: None,
                response: ApiResponse::json_type("PaginatedResponse<PullRequest>"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetPullRequest".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/pullrequests/{id}".to_string(),
                description: "Get a single pull request by ID".to_string(),
                request: None,
                response: ApiResponse::json_type("PullRequest"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListPullRequestComments".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/pullrequests/{id}/comments?pagelen=50"
                    .to_string(),
                description: "List comments on a pull request".to_string(),
                request: None,
                response: ApiResponse::json_type("PaginatedResponse<PullRequestComment>"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Issues
            // =================================================================
            Endpoint {
                id: "ListIssues".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/issues?pagelen=50".to_string(),
                description: "List repository issues".to_string(),
                request: None,
                response: ApiResponse::json_type("PaginatedResponse<Issue>"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetIssue".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/issues/{id}".to_string(),
                description: "Get a single issue by ID".to_string(),
                request: None,
                response: ApiResponse::json_type("Issue"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListIssueComments".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/issues/{id}/comments?pagelen=50"
                    .to_string(),
                description: "List comments on an issue".to_string(),
                request: None,
                response: ApiResponse::json_type("PaginatedResponse<IssueComment>"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListIssueChanges".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/issues/{id}/changes?pagelen=50"
                    .to_string(),
                description: "List change history for an issue".to_string(),
                request: None,
                response: ApiResponse::json_type("PaginatedResponse<IssueChange>"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Tags
            // =================================================================
            Endpoint {
                id: "ListTags".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/refs/tags?pagelen=50".to_string(),
                description: "List repository tags".to_string(),
                request: None,
                response: ApiResponse::json_type("PaginatedResponse<Tag>"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetTag".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/refs/tags/{name}".to_string(),
                description: "Get a single tag by name".to_string(),
                request: None,
                response: ApiResponse::json_type("Tag"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Downloads
            // =================================================================
            Endpoint {
                id: "ListDownloads".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/downloads?pagelen=50".to_string(),
                description: "List repository downloads (release artifacts)".to_string(),
                request: None,
                response: ApiResponse::json_type("PaginatedResponse<Download>"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "GetDownload".to_string(),
                method: RestMethod::Get,
                path: "/repositories/{workspace}/{repo_slug}/downloads/{filename}".to_string(),
                description: "Download a file from repository downloads".to_string(),
                request: None,
                response: ApiResponse::Binary,
                headers: vec![],
                params: None,
            },
        ],
        module_path: Some("bitbucket".to_string()),
        request_suffix: None,
        env_mapping: Some(EnvMapping {
            basic_user: Some(EnvList::new(vec!["BITBUCKET_USERNAME".to_string()])),
            basic_pass: Some(EnvList::new(vec!["BITBUCKET_APP_PASSWORD".to_string()])),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_has_correct_metadata() {
        let api = define_bitbucket_api();

        assert_eq!(api.name, "Bitbucket");
        assert_eq!(api.base_url, "https://api.bitbucket.org/2.0");
        assert!(api.docs_url.is_some());
        assert!(api
            .docs_url
            .as_ref()
            .unwrap()
            .contains("developer.atlassian.com"));
    }

    #[test]
    fn api_uses_basic_auth() {
        let api = define_bitbucket_api();

        assert!(matches!(api.auth, AuthStrategy::Basic));
        assert!(api.env_auth.contains(&"BITBUCKET_APP_PASSWORD".to_string()));
        assert_eq!(
            api.env_username,
            Some("BITBUCKET_USERNAME".to_string())
        );
    }

    #[test]
    fn api_has_fourteen_endpoints() {
        let api = define_bitbucket_api();
        assert_eq!(api.endpoints.len(), 14);
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
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn directory_contents_endpoint() {
        let api = define_bitbucket_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListDirectoryContents")
            .expect("ListDirectoryContents endpoint missing");

        assert!(endpoint.path.contains("{commit}"));
        assert!(endpoint.path.contains("{path}"));
        assert!(endpoint.path.contains("pagelen=50"));

        match &endpoint.response {
            ApiResponse::Json(schema) => {
                assert!(
                    schema.type_name.contains("PaginatedResponse<SourceEntry>"),
                    "Expected PaginatedResponse<SourceEntry>, got {}",
                    schema.type_name
                );
            }
            _ => panic!("Expected JSON response"),
        }
    }

    #[test]
    fn raw_content_endpoint_has_accept_override() {
        let api = define_bitbucket_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetFileContentRaw")
            .expect("GetFileContentRaw endpoint missing");

        assert_eq!(endpoint.method, RestMethod::Get);
        assert!(matches!(endpoint.response, ApiResponse::Text));

        let accept_header = endpoint.headers.iter().find(|(k, _)| k == "Accept");
        assert!(accept_header.is_some());
        assert_eq!(accept_header.unwrap().1, "text/plain");
    }

    #[test]
    fn pull_request_endpoints() {
        let api = define_bitbucket_api();

        let list_prs = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequests")
            .expect("ListPullRequests endpoint missing");
        assert!(list_prs.path.contains("state=all"));
        assert!(list_prs.path.contains("pagelen=50"));

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
        assert!(list_issues.path.contains("pagelen=50"));

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
        assert!(list_tags.path.contains("pagelen=50"));

        let get_tag = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetTag")
            .expect("GetTag endpoint missing");
        assert!(get_tag.path.contains("{name}"));
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

        assert!(matches!(endpoint.response, ApiResponse::Binary));
    }

    #[test]
    fn env_mapping_configured() {
        let api = define_bitbucket_api();

        let mapping = api
            .env_mapping
            .as_ref()
            .expect("env_mapping should be set");

        let basic_user = mapping
            .basic_user
            .as_ref()
            .expect("basic_user mapping should be set");
        assert!(basic_user.names().contains(&"BITBUCKET_USERNAME".to_string()));

        let basic_pass = mapping
            .basic_pass
            .as_ref()
            .expect("basic_pass mapping should be set");
        assert!(basic_pass.names().contains(&"BITBUCKET_APP_PASSWORD".to_string()));
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
            "ListDirectoryContents",
            "ListPullRequests",
            "ListPullRequestComments",
            "ListIssues",
            "ListIssueComments",
            "ListIssueChanges",
            "ListTags",
            "ListDownloads",
        ];

        for id in list_endpoints {
            let endpoint = api
                .endpoints
                .iter()
                .find(|e| e.id == id)
                .unwrap_or_else(|| panic!("Endpoint {} not found", id));

            match &endpoint.response {
                ApiResponse::Json(schema) => {
                    assert!(
                        schema.type_name.starts_with("PaginatedResponse<"),
                        "Endpoint {} should return PaginatedResponse type, got {}",
                        id,
                        schema.type_name
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
            ("GetTag", "Tag"),
        ];

        for (id, expected_type) in single_endpoints {
            let endpoint = api
                .endpoints
                .iter()
                .find(|e| e.id == id)
                .unwrap_or_else(|| panic!("Endpoint {} not found", id));

            match &endpoint.response {
                ApiResponse::Json(schema) => {
                    assert!(
                        !schema.type_name.starts_with("PaginatedResponse<"),
                        "Endpoint {} should return single item, not PaginatedResponse",
                        id
                    );
                    assert_eq!(
                        schema.type_name, expected_type,
                        "Endpoint {} should return {}, got {}",
                        id, expected_type, schema.type_name
                    );
                }
                _ => {} // GetFileContentRaw returns Text, GetDownload returns Binary
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
}
