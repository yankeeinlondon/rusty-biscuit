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
//! **Important**: Gitea uses the `Authorization: token <pat>` header format.
//! When setting `GITEA_TOKEN`, include the `token ` prefix:
//!
//! ```bash
//! export GITEA_TOKEN="token your_personal_access_token"
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

mod types;

pub use types::*;

use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

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
/// Set the `GITEA_TOKEN` environment variable with the **full auth value**:
///
/// ```bash
/// export GITEA_TOKEN="token your_personal_access_token"
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
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::gitea::define_gitea_api;
///
/// let api = define_gitea_api();
/// assert_eq!(api.name, "Gitea");
/// assert_eq!(api.endpoints.len(), 14);
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
        },
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
            },
            // =================================================================
            // Repository Contents (raw file access)
            // =================================================================
            Endpoint {
                id: "GetRepositoryContentRaw".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/raw/{filepath}".to_string(),
                description: "Get raw file content from repository".to_string(),
                request: None,
                response: ApiResponse::Text,
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Pull Requests
            // =================================================================
            Endpoint {
                id: "ListPullRequests".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/pulls?state=all&sort=recentupdate&limit=50"
                    .to_string(),
                description: "List pull requests with metadata (all states, most recent first)"
                    .to_string(),
                request: None,
                response: ApiResponse::json_vec_type("PullRequestSummary"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListPullRequestFiles".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/pulls/{index}/files?limit=50".to_string(),
                description: "List files changed in a pull request".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("PullRequestFile"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Issues
            // =================================================================
            Endpoint {
                id: "ListIssues".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues?state=all&type=issues&limit=50".to_string(),
                description: "List issues (excludes PRs with type=issues filter)".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("IssueSummary"),
                headers: vec![],
                params: None,
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
            },
            Endpoint {
                id: "ListIssueComments".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues/{index}/comments?limit=50".to_string(),
                description: "List comments on an issue".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("IssueComment"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListIssueTimeline".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/issues/{index}/timeline?limit=50".to_string(),
                description: "List timeline events for an issue".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("TimelineEvent"),
                headers: vec![],
                params: None,
            },
            // =================================================================
            // Tags and Releases
            // =================================================================
            Endpoint {
                id: "ListTags".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/tags?limit=50".to_string(),
                description: "List repository tags".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("RepoTag"),
                headers: vec![],
                params: None,
            },
            Endpoint {
                id: "ListReleases".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/releases?draft=true&pre-release=true&limit=50"
                    .to_string(),
                description: "List releases (linked to tags via tag_name, includes drafts/prereleases)".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("Release"),
                headers: vec![],
                params: None,
            },
            // Note: Gitea's /git/refs/{ref} returns an ARRAY, unlike GitHub's single object
            // Path uses {git_ref} instead of {ref} to avoid Rust keyword collision
            Endpoint {
                id: "GetTagReference".to_string(),
                method: RestMethod::Get,
                path: "/repos/{owner}/{repo}/git/refs/{git_ref}".to_string(),
                description: "Get tag reference (returns array; check object.type: 'commit' vs 'tag')".to_string(),
                request: None,
                response: ApiResponse::json_vec_type("GitRef"),
                headers: vec![],
                params: None,
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
            },
        ],
        module_path: Some("gitea".to_string()),
        request_suffix: None,
        env_mapping: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            AuthStrategy::ApiKey { header } => {
                assert_eq!(header, "Authorization");
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
    fn api_has_fourteen_endpoints() {
        let api = define_gitea_api();
        assert_eq!(api.endpoints.len(), 14);
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
    fn pull_request_endpoints_use_limit_param() {
        let api = define_gitea_api();

        let list_prs = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequests")
            .unwrap();
        assert!(list_prs.path.contains("state=all"));
        assert!(list_prs.path.contains("limit=50"));
        // Gitea uses limit, not per_page
        assert!(!list_prs.path.contains("per_page"));

        let list_files = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListPullRequestFiles")
            .unwrap();
        assert!(list_files.path.contains("{index}"));
        assert!(list_files.path.contains("limit=50"));
    }

    #[test]
    fn issue_endpoints_use_type_filter() {
        let api = define_gitea_api();

        let list = api.endpoints.iter().find(|e| e.id == "ListIssues").unwrap();
        assert!(list.path.contains("state=all"));
        // Gitea-specific: type=issues filters out PRs
        assert!(list.path.contains("type=issues"));
        assert!(list.path.contains("limit=50"));

        let get = api.endpoints.iter().find(|e| e.id == "GetIssue").unwrap();
        assert!(get.path.contains("{index}"));

        let comments = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueComments")
            .unwrap();
        assert!(comments.path.contains("{index}/comments"));

        let timeline = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListIssueTimeline")
            .unwrap();
        assert!(timeline.path.contains("timeline"));
    }

    #[test]
    fn tag_and_release_endpoints() {
        let api = define_gitea_api();

        let tags = api.endpoints.iter().find(|e| e.id == "ListTags").unwrap();
        assert!(tags.path.contains("tags?limit=50"));

        let releases = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListReleases")
            .unwrap();
        assert!(releases.path.contains("releases"));
        // Gitea-specific: include drafts and prereleases
        assert!(releases.path.contains("draft=true"));
        assert!(releases.path.contains("pre-release=true"));

        let ref_endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetTagReference")
            .unwrap();
        assert!(ref_endpoint.path.contains("git/refs/{git_ref}"));

        let annotated = api
            .endpoints
            .iter()
            .find(|e| e.id == "GetAnnotatedTag")
            .unwrap();
        assert!(annotated.path.contains("git/tags/{sha}"));
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
            match &endpoint.response {
                ApiResponse::Json(schema) => {
                    assert!(
                        !schema.type_name.starts_with("Vec<"),
                        "Endpoint {} should return single item, not Vec",
                        id
                    );
                }
                _ => {} // GetRepositoryContentRaw returns Text, which is fine
            }
        }
    }

    #[test]
    fn module_path_is_set() {
        let api = define_gitea_api();
        assert_eq!(api.module_path, Some("gitea".to_string()));
    }
}
