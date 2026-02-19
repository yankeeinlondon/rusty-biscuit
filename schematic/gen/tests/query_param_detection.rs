//! Tests for detecting hardcoded query parameters in endpoint paths.
//!
//! These tests verify that the code generator can detect and handle
//! hardcoded query parameters in paths like `/users?per_page=100`.
//! This is important for the pagination migration (Phase 2) where
//! hardcoded params will be extracted into `EndpointParams`.

use schematic_define::{ApiResponse, Endpoint, RestMethod};

/// Helper to check if a path contains hardcoded query parameters.
fn path_has_hardcoded_query_params(path: &str) -> bool {
    path.contains('?')
}

/// Helper to extract the base path (before ?) from a path.
fn extract_base_path(path: &str) -> &str {
    path.split('?').next().unwrap_or(path)
}

/// Helper to extract query string (after ?) from a path.
fn extract_query_string(path: &str) -> Option<&str> {
    path.split('?').nth(1)
}

// =============================================================================
// Detection tests
// =============================================================================

#[test]
fn detect_single_hardcoded_query_param() {
    let path = "/repos/{owner}/{repo}/tags?per_page=100";
    assert!(path_has_hardcoded_query_params(path));
    assert_eq!(extract_base_path(path), "/repos/{owner}/{repo}/tags");
    assert_eq!(extract_query_string(path), Some("per_page=100"));
}

#[test]
fn detect_multiple_hardcoded_query_params() {
    let path = "/repos/{owner}/{repo}/pulls?state=all&sort=updated&direction=desc&per_page=100";
    assert!(path_has_hardcoded_query_params(path));
    assert_eq!(extract_base_path(path), "/repos/{owner}/{repo}/pulls");
    assert_eq!(
        extract_query_string(path),
        Some("state=all&sort=updated&direction=desc&per_page=100")
    );
}

#[test]
fn detect_recursive_flag_query_param() {
    let path = "/repos/{owner}/{repo}/git/trees/{tree_sha}?recursive=1";
    assert!(path_has_hardcoded_query_params(path));
    assert_eq!(
        extract_base_path(path),
        "/repos/{owner}/{repo}/git/trees/{tree_sha}"
    );
    assert_eq!(extract_query_string(path), Some("recursive=1"));
}

#[test]
fn no_query_params_in_simple_path() {
    let path = "/repos/{owner}/{repo}";
    assert!(!path_has_hardcoded_query_params(path));
    assert_eq!(extract_base_path(path), path);
    assert_eq!(extract_query_string(path), None);
}

#[test]
fn no_query_params_in_nested_path() {
    let path = "/repositories/{workspace}/{repo_slug}/pullrequests/{id}/comments";
    assert!(!path_has_hardcoded_query_params(path));
    assert_eq!(extract_base_path(path), path);
    assert_eq!(extract_query_string(path), None);
}

// =============================================================================
// Real endpoint examples from GitHub/GitLab/Gitea definitions
// =============================================================================

#[test]
fn github_style_pagination_path() {
    // Current GitHub pattern with hardcoded per_page
    let endpoint = Endpoint {
        id: "ListTags".to_string(),
        method: RestMethod::Get,
        path: "/repos/{owner}/{repo}/tags?per_page=100".to_string(),
        description: "List repository tags".to_string(),
        request: None,
        response: ApiResponse::json_vec_type("RepoTag"),
        headers: vec![],
        params: None,
    };

    assert!(path_has_hardcoded_query_params(&endpoint.path));
    assert_eq!(
        extract_base_path(&endpoint.path),
        "/repos/{owner}/{repo}/tags"
    );
}

#[test]
fn gitlab_style_pagination_with_recursive() {
    // GitLab pattern with multiple hardcoded params
    let endpoint = Endpoint {
        id: "ListRepositoryTree".to_string(),
        method: RestMethod::Get,
        path: "/projects/{id}/repository/tree?per_page=100&recursive=true".to_string(),
        description: "List repository tree recursively".to_string(),
        request: None,
        response: ApiResponse::json_vec_type("TreeNode"),
        headers: vec![],
        params: None,
    };

    assert!(path_has_hardcoded_query_params(&endpoint.path));
    assert_eq!(
        extract_base_path(&endpoint.path),
        "/projects/{id}/repository/tree"
    );
    assert!(
        extract_query_string(&endpoint.path)
            .unwrap()
            .contains("per_page=100")
    );
    assert!(
        extract_query_string(&endpoint.path)
            .unwrap()
            .contains("recursive=true")
    );
}

#[test]
fn gitea_style_pagination_path() {
    // Gitea uses 'limit' instead of 'per_page'
    let endpoint = Endpoint {
        id: "ListTags".to_string(),
        method: RestMethod::Get,
        path: "/repos/{owner}/{repo}/tags?limit=50".to_string(),
        description: "List repository tags".to_string(),
        request: None,
        response: ApiResponse::json_vec_type("Tag"),
        headers: vec![],
        params: None,
    };

    assert!(path_has_hardcoded_query_params(&endpoint.path));
    assert_eq!(
        extract_base_path(&endpoint.path),
        "/repos/{owner}/{repo}/tags"
    );
    assert_eq!(extract_query_string(&endpoint.path), Some("limit=50"));
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn path_with_query_param_and_path_params() {
    // Path params {owner}, {repo}, {pull_number} AND query params
    let path = "/repos/{owner}/{repo}/pulls/{pull_number}/files?per_page=100";
    assert!(path_has_hardcoded_query_params(path));
    assert_eq!(
        extract_base_path(path),
        "/repos/{owner}/{repo}/pulls/{pull_number}/files"
    );
}

#[test]
fn path_with_multiple_ampersands() {
    let path = "/search?q=rust&sort=stars&order=desc&per_page=100";
    assert!(path_has_hardcoded_query_params(path));
    assert_eq!(extract_base_path(path), "/search");
    assert_eq!(
        extract_query_string(path),
        Some("q=rust&sort=stars&order=desc&per_page=100")
    );
}

#[test]
fn empty_path() {
    let path = "";
    assert!(!path_has_hardcoded_query_params(path));
    assert_eq!(extract_base_path(path), "");
    assert_eq!(extract_query_string(path), None);
}

#[test]
fn path_with_only_question_mark() {
    // Edge case: path ends with ? but no actual params
    let path = "/test?";
    assert!(path_has_hardcoded_query_params(path));
    assert_eq!(extract_base_path(path), "/test");
    assert_eq!(extract_query_string(path), Some(""));
}

// =============================================================================
// Query param parsing tests
// =============================================================================

/// Helper to parse query params into key-value pairs.
fn parse_query_params(query: &str) -> Vec<(&str, &str)> {
    query
        .split('&')
        .filter(|s| !s.is_empty())
        .filter_map(|param| {
            let mut parts = param.splitn(2, '=');
            let key = parts.next()?;
            let value = parts.next().unwrap_or("");
            Some((key, value))
        })
        .collect()
}

#[test]
fn parse_single_query_param() {
    let query = "per_page=100";
    let params = parse_query_params(query);
    assert_eq!(params, vec![("per_page", "100")]);
}

#[test]
fn parse_multiple_query_params() {
    let query = "state=all&sort=updated&direction=desc&per_page=100";
    let params = parse_query_params(query);
    assert_eq!(
        params,
        vec![
            ("state", "all"),
            ("sort", "updated"),
            ("direction", "desc"),
            ("per_page", "100"),
        ]
    );
}

#[test]
fn parse_boolean_style_query_params() {
    let query = "recursive=true&per_page=100";
    let params = parse_query_params(query);
    assert_eq!(params, vec![("recursive", "true"), ("per_page", "100")]);
}

#[test]
fn parse_query_param_without_value() {
    // Some APIs use params like ?recursive (no =value)
    let query = "recursive";
    let params = parse_query_params(query);
    assert_eq!(params, vec![("recursive", "")]);
}

#[test]
fn parse_empty_query_string() {
    let query = "";
    let params = parse_query_params(query);
    assert!(params.is_empty());
}

// =============================================================================
// Migration validation tests
// =============================================================================

/// Validates that pagination-related params can be identified.
fn is_pagination_param(name: &str) -> bool {
    matches!(
        name,
        "page" | "per_page" | "pagelen" | "limit" | "offset" | "cursor" | "after" | "before"
    )
}

/// Validates that filter/sort params can be identified.
fn is_filter_param(name: &str) -> bool {
    matches!(
        name,
        "state"
            | "sort"
            | "direction"
            | "order"
            | "type"
            | "draft"
            | "pre_release"
            | "pre-release"
            | "recursive"
    )
}

#[test]
fn classify_github_pullrequests_params() {
    // /repos/{owner}/{repo}/pulls?state=all&sort=updated&direction=desc&per_page=100
    let query = "state=all&sort=updated&direction=desc&per_page=100";
    let params = parse_query_params(query);

    let pagination: Vec<_> = params
        .iter()
        .filter(|(k, _)| is_pagination_param(k))
        .collect();
    let filters: Vec<_> = params.iter().filter(|(k, _)| is_filter_param(k)).collect();

    assert_eq!(pagination.len(), 1, "per_page is pagination");
    assert_eq!(
        filters.len(),
        3,
        "state, sort, direction are filters: {:?}",
        filters
    );
}

#[test]
fn classify_gitlab_tree_params() {
    // /projects/{id}/repository/tree?per_page=100&recursive=true
    let query = "per_page=100&recursive=true";
    let params = parse_query_params(query);

    let pagination: Vec<_> = params
        .iter()
        .filter(|(k, _)| is_pagination_param(k))
        .collect();
    let filters: Vec<_> = params.iter().filter(|(k, _)| is_filter_param(k)).collect();

    assert_eq!(pagination.len(), 1, "per_page is pagination");
    assert_eq!(filters.len(), 1, "recursive is a filter");
}

#[test]
fn classify_gitea_releases_params() {
    // /repos/{owner}/{repo}/releases?draft=true&pre-release=true&limit=50
    let query = "draft=true&pre-release=true&limit=50";
    let params = parse_query_params(query);

    let pagination: Vec<_> = params
        .iter()
        .filter(|(k, _)| is_pagination_param(k))
        .collect();
    let filters: Vec<_> = params.iter().filter(|(k, _)| is_filter_param(k)).collect();

    assert_eq!(pagination.len(), 1, "limit is pagination");
    assert_eq!(
        filters.len(),
        2,
        "draft, pre-release are filters: {:?}",
        filters
    );
}
