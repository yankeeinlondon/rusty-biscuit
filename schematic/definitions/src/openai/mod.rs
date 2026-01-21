//! OpenAI API definition.
//!
//! This module provides a complete definition of the OpenAI Models API,
//! demonstrating how to define REST APIs using the schematic types.

mod types;

pub use types::{DeleteModelResponse, ListModelsResponse, Model};

use schematic_define::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

/// Creates the OpenAI API definition.
///
/// This defines the OpenAI Models API with endpoints for listing,
/// retrieving, and deleting models.
///
/// ## Endpoints
///
/// - `ListModels` - GET /models
/// - `RetrieveModel` - GET /models/{model}
/// - `DeleteModel` - DELETE /models/{model}
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::openai::define_openai_api;
///
/// let api = define_openai_api();
/// assert_eq!(api.name, "OpenAI");
/// assert_eq!(api.endpoints.len(), 3);
/// ```
pub fn define_openai_api() -> RestApi {
    RestApi {
        name: "OpenAI".to_string(),
        description: "OpenAI REST API for model management".to_string(),
        base_url: "https://api.openai.com/v1".to_string(),
        docs_url: Some("https://platform.openai.com/docs/api-reference".to_string()),
        auth: AuthStrategy::BearerToken { header: None },
        env_auth: vec!["OPENAI_API_KEY".to_string()],
        env_username: None,
        headers: vec![],
        endpoints: vec![
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/models".to_string(),
                description: "Lists the currently available models".to_string(),
                request: None,
                response: ApiResponse::json_type("ListModelsResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "RetrieveModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{model}".to_string(),
                description: "Retrieves a model instance".to_string(),
                request: None,
                response: ApiResponse::json_type("Model"),
                headers: vec![],
            },
            Endpoint {
                id: "DeleteModel".to_string(),
                method: RestMethod::Delete,
                path: "/models/{model}".to_string(),
                description: "Delete a fine-tuned model".to_string(),
                request: None,
                response: ApiResponse::json_type("DeleteModelResponse"),
                headers: vec![],
            },
        ],
        module_path: None,
        request_suffix: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_has_correct_metadata() {
        let api = define_openai_api();

        assert_eq!(api.name, "OpenAI");
        assert_eq!(api.base_url, "https://api.openai.com/v1");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn api_uses_bearer_auth() {
        let api = define_openai_api();

        match &api.auth {
            AuthStrategy::BearerToken { header } => {
                assert!(header.is_none());
            }
            _ => panic!("Expected BearerToken auth strategy"),
        }
        assert_eq!(api.env_auth, vec!["OPENAI_API_KEY"]);
    }

    #[test]
    fn api_has_three_endpoints() {
        let api = define_openai_api();
        assert_eq!(api.endpoints.len(), 3);
    }

    #[test]
    fn list_models_endpoint() {
        let api = define_openai_api();
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListModels").unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/models");
        assert!(endpoint.request.is_none());
    }

    #[test]
    fn retrieve_model_uses_path_parameter() {
        let api = define_openai_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "RetrieveModel")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/models/{model}");
        assert!(endpoint.path.contains("{model}"));
    }

    #[test]
    fn delete_model_uses_path_parameter() {
        let api = define_openai_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "DeleteModel")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Delete);
        assert_eq!(endpoint.path, "/models/{model}");
    }
}
