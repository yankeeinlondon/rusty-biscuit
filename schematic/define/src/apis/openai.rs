//! OpenAI API definition.
//!
//! This module provides a complete definition of the OpenAI Models API,
//! demonstrating how to define REST APIs using the schematic types.

use serde::{Deserialize, Serialize};

use crate::{ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

/// An OpenAI model object.
///
/// Describes a model available through the OpenAI API.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Model {
    /// The model identifier (e.g., "gpt-4").
    pub id: String,
    /// The object type, always "model".
    pub object: String,
    /// Unix timestamp of when the model was created.
    pub created: i64,
    /// The organization that owns the model.
    pub owned_by: String,
}

/// Response from the List Models endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListModelsResponse {
    /// The object type, always "list".
    pub object: String,
    /// List of model objects.
    pub data: Vec<Model>,
}

/// Response from the Delete Model endpoint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteModelResponse {
    /// The model identifier that was deleted.
    pub id: String,
    /// The object type, always "model".
    pub object: String,
    /// Whether the deletion was successful.
    pub deleted: bool,
}

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
/// ## Example
///
/// ```rust
/// use schematic_define::apis::define_openai_api;
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
        endpoints: vec![
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/models".to_string(),
                description: "Lists the currently available models".to_string(),
                request: None,
                response: ApiResponse::json_type("ListModelsResponse"),
            },
            Endpoint {
                id: "RetrieveModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{model}".to_string(),
                description: "Retrieves a model instance".to_string(),
                request: None,
                response: ApiResponse::json_type("Model"),
            },
            Endpoint {
                id: "DeleteModel".to_string(),
                method: RestMethod::Delete,
                path: "/models/{model}".to_string(),
                description: "Delete a fine-tuned model".to_string(),
                request: None,
                response: ApiResponse::json_type("DeleteModelResponse"),
            },
        ],
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

    #[test]
    fn model_schema_serialization() {
        let model = Model {
            id: "gpt-4".to_string(),
            object: "model".to_string(),
            created: 1687882411,
            owned_by: "openai".to_string(),
        };

        let json = serde_json::to_string(&model).unwrap();
        let parsed: Model = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.id, model.id);
        assert_eq!(parsed.owned_by, model.owned_by);
    }

    #[test]
    fn list_models_response_serialization() {
        let response = ListModelsResponse {
            object: "list".to_string(),
            data: vec![Model {
                id: "gpt-4".to_string(),
                object: "model".to_string(),
                created: 1687882411,
                owned_by: "openai".to_string(),
            }],
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: ListModelsResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.object, "list");
        assert_eq!(parsed.data.len(), 1);
    }

    #[test]
    fn delete_model_response_serialization() {
        let response = DeleteModelResponse {
            id: "ft:gpt-4:my-org".to_string(),
            object: "model".to_string(),
            deleted: true,
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: DeleteModelResponse = serde_json::from_str(&json).unwrap();

        assert!(parsed.deleted);
        assert_eq!(parsed.id, "ft:gpt-4:my-org");
    }
}
