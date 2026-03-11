//! OpenAI API definition.
//!
//! This module provides a complete definition of the OpenAI Models API,
//! demonstrating how to define REST APIs using the schematic types.

mod types;

pub use types::{DeleteModelResponse, ListModelsResponse, Model};

use crate::registry::SchemaRegistry;
use schematic_define::{
    ApiResponse, AuthStrategy, Endpoint, EnvList, EnvMapping, RestApi, RestMethod,
};

/// Creates a schema registry containing all OpenAI response types.
///
/// This registry can be used to generate OpenAPI schemas for the OpenAI API.
/// All response types used by the API endpoints are registered.
///
/// ## Examples
///
/// ```
/// use schematic_definitions::openai::{openapi_registry, define_openai_api};
///
/// let registry = openapi_registry();
/// let api = define_openai_api();
///
/// // Registry contains all response types
/// assert!(registry.get("Model").is_some());
/// assert!(registry.get("ListModelsResponse").is_some());
/// assert!(registry.get("DeleteModelResponse").is_some());
///
/// // Registry is complete for the API
/// assert!(registry.validate_completeness(&api).is_ok());
/// ```
#[must_use]
pub fn openapi_registry() -> SchemaRegistry {
    SchemaRegistry::new()
        .register::<Model>("Model")
        .register::<ListModelsResponse>("ListModelsResponse")
        .register::<DeleteModelResponse>("DeleteModelResponse")
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
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "RetrieveModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{model}".to_string(),
                description: "Retrieves a model instance".to_string(),
                request: None,
                response: ApiResponse::json_type("Model"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
            Endpoint {
                id: "DeleteModel".to_string(),
                method: RestMethod::Delete,
                path: "/models/{model}".to_string(),
                description: "Delete a fine-tuned model".to_string(),
                request: None,
                response: ApiResponse::json_type("DeleteModelResponse"),
                headers: vec![],
                params: None,
                oauth_scopes: None,
            },
        ],
        module_path: None,
        request_suffix: None,
        env_mapping: Some(EnvMapping {
            bearer_token: Some(EnvList::new(vec!["OPENAI_API_KEY".to_string()])),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // =============================================
    // openapi_registry() tests
    // =============================================

    #[test]
    fn openapi_registry_contains_all_response_types() {
        let registry = openapi_registry();

        assert!(registry.get("Model").is_some());
        assert!(registry.get("ListModelsResponse").is_some());
        assert!(registry.get("DeleteModelResponse").is_some());
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn openapi_registry_validates_against_api() {
        let registry = openapi_registry();
        let api = define_openai_api();

        let result = registry.validate_completeness(&api);
        assert!(result.is_ok(), "Registry should be complete: {:?}", result);
    }

    #[test]
    fn openapi_registry_converts_to_openapi_schemas() {
        let registry = openapi_registry();
        let openapi_schemas = registry.to_openapi_schemas();

        assert_eq!(openapi_schemas.len(), 3);
        assert!(openapi_schemas.contains_key("Model"));
        assert!(openapi_schemas.contains_key("ListModelsResponse"));
        assert!(openapi_schemas.contains_key("DeleteModelResponse"));
    }

    // =============================================
    // API definition tests
    // =============================================

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

    // =============================================
    // OpenAPI export integration tests
    // =============================================

    #[test]
    fn export_openai_api_to_openapi() {
        use schematic_define::openapi::{ExportOptions, export};

        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("1.0.0");

        let doc = export(&api, &registry, &options).unwrap();

        // Verify OpenAPI version
        assert_eq!(doc.openapi, "3.0.3");

        // Verify info
        assert_eq!(doc.info.title, "OpenAI");
        assert_eq!(doc.info.version, "1.0.0");

        // Verify servers
        assert_eq!(doc.servers.len(), 1);
        assert_eq!(doc.servers[0].url, "https://api.openai.com/v1");

        // Verify paths include all endpoints
        assert!(doc.paths.paths.contains_key("/models"));
        assert!(doc.paths.paths.contains_key("/models/{model}"));

        // Verify security scheme
        let components = doc.components.as_ref().unwrap();
        assert!(components.security_schemes.contains_key("bearerAuth"));

        // Verify schemas
        assert!(components.schemas.contains_key("Model"));
        assert!(components.schemas.contains_key("ListModelsResponse"));
        assert!(components.schemas.contains_key("DeleteModelResponse"));
    }

    #[test]
    fn export_openai_api_to_json() {
        use schematic_define::openapi::{ExportFormat, ExportOptions, export, serialize};

        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("1.0.0");

        let doc = export(&api, &registry, &options).unwrap();
        let json = serialize(&doc, ExportFormat::Json).unwrap();

        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed["openapi"], "3.0.3");
        assert_eq!(parsed["info"]["title"], "OpenAI");
    }

    #[test]
    fn export_openai_api_to_yaml() {
        use schematic_define::openapi::{ExportFormat, ExportOptions, export, serialize};

        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().with_version("1.0.0");

        let doc = export(&api, &registry, &options).unwrap();
        let yaml = serialize(&doc, ExportFormat::Yaml).unwrap();

        // Verify it contains expected content
        assert!(yaml.contains("openapi:"));
        assert!(yaml.contains("OpenAI"));
    }

    #[test]
    fn export_preserves_vendor_extensions() {
        use schematic_define::openapi::{ExportOptions, export};

        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();

        // Document-level extension should be present
        assert!(doc.extensions.contains_key("x-schematic"));
    }

    #[test]
    fn export_can_skip_vendor_extensions() {
        use schematic_define::openapi::{ExportOptions, export};

        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new().skip_extensions();

        let doc = export(&api, &registry, &options).unwrap();

        // Should NOT have vendor extensions
        assert!(!doc.extensions.contains_key("x-schematic"));
    }

    #[test]
    fn export_includes_external_docs() {
        use schematic_define::openapi::{ExportOptions, export};

        let api = define_openai_api();
        let registry = openapi_registry();
        let options = ExportOptions::new();

        let doc = export(&api, &registry, &options).unwrap();

        // Should include external docs from docs_url
        assert!(doc.external_docs.is_some());
        assert_eq!(
            doc.external_docs.as_ref().unwrap().url,
            "https://platform.openai.com/docs/api-reference"
        );
    }
}
