//! Shared test utilities for schematic-gen tests.
//!
//! This module provides common helper functions for creating test fixtures
//! across the codebase, reducing duplication and ensuring consistency.

use proc_macro2::TokenStream;
use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod, Schema};

/// Creates a minimal RestApi for testing.
///
/// ## Arguments
///
/// * `name` - API name (used for struct/enum names)
/// * `base_url` - Base URL for the API
/// * `auth` - Authentication strategy
/// * `env_auth` - Environment variable names for auth credentials
pub fn make_api(name: &str, base_url: &str, auth: AuthStrategy, env_auth: Vec<String>) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{} API", name),
        base_url: base_url.to_string(),
        docs_url: None,
        auth,
        env_auth,
        env_username: None,
        headers: vec![],
        endpoints: vec![],
    }
}

/// Creates a RestApi with a single endpoint for testing.
pub fn make_api_with_endpoint(
    name: &str,
    auth: AuthStrategy,
    env_auth: Vec<String>,
    endpoint: Endpoint,
) -> RestApi {
    RestApi {
        name: name.to_string(),
        description: format!("{} API", name),
        base_url: "https://api.example.com".to_string(),
        docs_url: None,
        auth,
        env_auth,
        env_username: None,
        headers: vec![],
        endpoints: vec![endpoint],
    }
}

/// Creates an Endpoint for testing.
///
/// ## Arguments
///
/// * `id` - Endpoint identifier (becomes struct/enum variant name)
/// * `method` - HTTP method
/// * `path` - URL path (may contain `{param}` placeholders)
/// * `request` - Optional request body schema
pub fn make_endpoint(
    id: &str,
    method: RestMethod,
    path: &str,
    request: Option<Schema>,
) -> Endpoint {
    Endpoint {
        id: id.to_string(),
        method,
        path: path.to_string(),
        description: format!("Test endpoint for {}", id),
        request,
        response: ApiResponse::json_type("TestResponse"),
        headers: vec![],
    }
}

/// Creates a simple RestApi with a single GET endpoint.
pub fn make_simple_api() -> RestApi {
    RestApi {
        name: "TestApi".to_string(),
        description: "Test API".to_string(),
        base_url: "https://api.test.com/v1".to_string(),
        docs_url: None,
        auth: AuthStrategy::None,
        env_auth: vec![],
        env_username: None,
        headers: vec![],
        endpoints: vec![Endpoint {
            id: "ListItems".to_string(),
            method: RestMethod::Get,
            path: "/items".to_string(),
            description: "List all items".to_string(),
            request: None,
            response: ApiResponse::json_type("ListItemsResponse"),
            headers: vec![],
        }],
    }
}

/// Creates a complex RestApi mimicking OpenAI's structure.
///
/// Includes:
/// - Bearer token authentication
/// - Multiple endpoints (GET, GET with param, POST with body)
/// - Documentation URL
pub fn make_complex_api() -> RestApi {
    RestApi {
        name: "OpenAI".to_string(),
        description: "OpenAI REST API".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        docs_url: Some("https://platform.openai.com/docs".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        env_auth: vec!["OPENAI_API_KEY".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/models".to_string(),
                description: "Lists available models".to_string(),
                request: None,
                response: ApiResponse::json_type("ListModelsResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "RetrieveModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{model}".to_string(),
                description: "Retrieves a model".to_string(),
                request: None,
                response: ApiResponse::json_type("Model"),
                headers: vec![],
            },
            Endpoint {
                id: "CreateCompletion".to_string(),
                method: RestMethod::Post,
                path: "/completions".to_string(),
                description: "Creates a completion".to_string(),
                request: Some(Schema::new("CreateCompletionRequest")),
                response: ApiResponse::json_type("Completion"),
                headers: vec![],
            },
        ],
    }
}

/// Validates that generated code is syntactically correct.
///
/// ## Errors
///
/// Returns an error string if the generated code fails to parse.
pub fn validate_generated_code(tokens: &TokenStream) -> Result<(), String> {
    syn::parse2::<syn::File>(tokens.clone()).map_err(|e| e.to_string())?;
    Ok(())
}

/// Formats generated code using prettyplease.
///
/// ## Errors
///
/// Returns an error string if the code fails to parse.
pub fn format_generated_code(tokens: &TokenStream) -> Result<String, String> {
    let file = syn::parse2::<syn::File>(tokens.clone()).map_err(|e| e.to_string())?;
    Ok(prettyplease::unparse(&file))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn make_api_creates_valid_api() {
        let api = make_api("Test", "https://api.test.com", AuthStrategy::None, vec![]);
        assert_eq!(api.name, "Test");
        assert_eq!(api.base_url, "https://api.test.com");
        assert!(api.endpoints.is_empty());
    }

    #[test]
    fn make_endpoint_creates_valid_endpoint() {
        let endpoint = make_endpoint("GetUser", RestMethod::Get, "/users/{id}", None);
        assert_eq!(endpoint.id, "GetUser");
        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/users/{id}");
    }

    #[test]
    fn make_simple_api_has_one_endpoint() {
        let api = make_simple_api();
        assert_eq!(api.endpoints.len(), 1);
        assert_eq!(api.auth, AuthStrategy::None);
    }

    #[test]
    fn make_complex_api_has_multiple_endpoints() {
        let api = make_complex_api();
        assert_eq!(api.endpoints.len(), 3);
        assert!(matches!(api.auth, AuthStrategy::BearerToken { .. }));
    }
}
