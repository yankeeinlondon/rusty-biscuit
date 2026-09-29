use schematic_define::{ApiResponse, Endpoint};

/// Returns all branch endpoints.
pub fn all() -> Vec<Endpoint> {
    vec![Endpoint {
        id: "GetBranch".to_string(),
        method: schematic_define::RestMethod::Get,
        // GitLab rejects a raw `/` in `{branch}`; the generated client
        // percent-encodes it as `%2F`, which GitLab requires.
        path: "/projects/{id}/repository/branches/{branch}".to_string(),
        description: "Get a single branch (commit.id is the branch head commit)".to_string(),
        request: None,
        response: ApiResponse::json_type("Branch"),
        headers: vec![],
        params: None,
        oauth_scopes: None,
    }]
}
