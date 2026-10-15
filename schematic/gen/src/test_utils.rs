//! Shared test utilities for schematic-gen tests.
//!
//! This module provides common helper functions for creating test fixtures
//! across the codebase, reducing duplication and ensuring consistency.

use proc_macro2::TokenStream;
use schematic_define::{
    ApiRequest, ApiResponse, AuthStrategy, CrossOrigin, Endpoint, RestApi, RestMethod,
};

/// Creates a minimal RestApi for testing.
///
/// ## Arguments
///
/// * `name` - API name (used for struct/enum names)
/// * `base_url` - Base URL for the API
/// * `auth` - Authentication strategy
/// * `env_auth` - Environment variable names for auth credentials
pub fn make_api(name: &str, base_url: &str, auth: AuthStrategy, env_auth: Vec<String>) -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = name.to_string(); fixture.description = format!("{} API", name); fixture.base_url = base_url.to_string(); fixture.docs_url = None; fixture.auth = auth; fixture.auth_policy = None; fixture.env_auth = env_auth; fixture.env_username = None; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; fixture }
}

/// Creates a RestApi with a single endpoint for testing.
pub fn make_api_with_endpoint(
    name: &str,
    auth: AuthStrategy,
    env_auth: Vec<String>,
    endpoint: Endpoint,
) -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = name.to_string(); fixture.description = format!("{} API", name); fixture.base_url = "https://api.example.com".to_string(); fixture.docs_url = None; fixture.auth = auth; fixture.auth_policy = None; fixture.env_auth = env_auth; fixture.env_username = None; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![endpoint]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; fixture }
}

/// Creates an Endpoint for testing.
///
/// ## Arguments
///
/// * `id` - Endpoint identifier (becomes the request struct name)
/// * `method` - HTTP method
/// * `path` - URL path (may contain `{param}` placeholders)
/// * `request` - Optional request body schema
pub fn make_endpoint(
    id: &str,
    method: RestMethod,
    path: &str,
    request: Option<ApiRequest>,
) -> Endpoint {
    { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = id.to_string(); fixture.method = method; fixture.path = path.to_string(); fixture.description = format!("Test endpoint for {}", id); fixture.request = request; fixture.response = ApiResponse::json_type("TestResponse"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }
}

/// Creates a simple RestApi with a single GET endpoint.
pub fn make_simple_api() -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "TestApi".to_string(); fixture.description = "Test API".to_string(); fixture.base_url = "https://api.test.com/v1".to_string(); fixture.docs_url = None; fixture.auth = AuthStrategy::None; fixture.auth_policy = None; fixture.env_auth = vec![]; fixture.env_username = None; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![{ let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "ListItems".to_string(); fixture.method = RestMethod::Get; fixture.path = "/items".to_string(); fixture.description = "List all items".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("ListItemsResponse"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture }]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; fixture }
}

/// Creates a complex RestApi mimicking OpenAI's structure.
///
/// Includes:
/// - Bearer token authentication
/// - Multiple endpoints (GET, GET with param, POST with body)
/// - Documentation URL
pub fn make_complex_api() -> RestApi {
    { let mut fixture = RestApi::builder(&schematic_define::DefinitionContext::types_in("::fixture"), "Fixture", "https://example.test").describes("Fixture").no_auth().build().unwrap(); fixture.name = "OpenAI".to_string(); fixture.description = "OpenAI REST API".to_string(); fixture.base_url = "https://api.openai.com/v1".to_string(); fixture.docs_url = Some("https://platform.openai.com/docs".to_string()); fixture.auth = AuthStrategy::BearerToken { header: None }; fixture.auth_policy = None; fixture.env_auth = vec!["OPENAI_API_KEY".to_string()]; fixture.env_username = None; fixture.env_mapping = None; fixture.headers = vec![]; fixture.endpoints = vec![
            { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "ListModels".to_string(); fixture.method = RestMethod::Get; fixture.path = "/models".to_string(); fixture.description = "Lists available models".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("ListModelsResponse"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
            { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "RetrieveModel".to_string(); fixture.method = RestMethod::Get; fixture.path = "/models/{model}".to_string(); fixture.description = "Retrieves a model".to_string(); fixture.request = None; fixture.response = ApiResponse::json_type("Model"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
            { let mut fixture = Endpoint::get("Fixture", "/").describes("Fixture").returns_empty().build().unwrap(); fixture.id = "CreateCompletion".to_string(); fixture.method = RestMethod::Post; fixture.path = "/completions".to_string(); fixture.description = "Creates a completion".to_string(); fixture.request = Some(ApiRequest::json_type("CreateCompletionBody")); fixture.response = ApiResponse::json_type("Completion"); fixture.headers = vec![]; fixture.params = None; fixture.cross_origin = None; fixture.pagination = None; fixture },
        ]; fixture.module_path = None; fixture.request_suffix = None; fixture.version = None; fixture.cross_origin = CrossOrigin::default(); fixture.request_id_header = None; fixture.secret_names = vec![]; fixture.default_pagination = None; fixture }
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
