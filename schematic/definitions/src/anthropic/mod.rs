//! Anthropic API definition.
//!
//! This module provides a complete definition of the Anthropic Messages API,
//! which powers Claude and enables building AI agents through tool use.
//!
//! ## API Overview
//!
//! The Anthropic API is designed around a single powerful endpoint (Messages)
//! that supports:
//!
//! - **Conversational AI**: Multi-turn conversations with Claude
//! - **Tool Use**: Define tools, receive tool requests, return results
//! - **Extended Thinking**: Enable internal reasoning for complex tasks
//! - **Prompt Caching**: Reduce costs by 90% for repeated context
//! - **Multimodal Input**: Text, images, and documents
//!
//! ## Agent Loop Pattern
//!
//! The core pattern for building agents:
//!
//! ```text
//! loop {
//!     response = client.create_message(request)
//!     if response.stop_reason == "end_turn" {
//!         break  // Task complete
//!     }
//!     if response.stop_reason == "tool_use" {
//!         // Execute tools, append results to messages, continue
//!     }
//! }
//! ```
//!
//! ## Authentication
//!
//! Uses API key authentication via the `X-Api-Key` header.
//! Set via environment variable: `ANTHROPIC_API_KEY`
//!
//! Required header: `anthropic-version: 2023-06-01`

mod types;

pub use types::*;

use schematic_define::{ApiRequest, ApiResponse, AuthStrategy, Endpoint, RestApi, RestMethod};

/// Creates the Anthropic API definition.
///
/// This defines the Anthropic Messages API with endpoints for message creation,
/// token counting, and model discovery.
///
/// ## Endpoints
///
/// | ID | Method | Path | Description |
/// |----|--------|------|-------------|
/// | CreateMessage | POST | /messages | Create a message (agent loop core) |
/// | CountTokens | POST | /messages/count_tokens | Count tokens before sending |
/// | ListModels | GET | /models | List available models |
/// | RetrieveModel | GET | /models/{model_id} | Get specific model info |
///
/// ## Examples
///
/// ```rust
/// use schematic_definitions::anthropic::define_anthropic_api;
///
/// let api = define_anthropic_api();
/// assert_eq!(api.name, "Anthropic");
/// assert_eq!(api.endpoints.len(), 4);
/// ```
pub fn define_anthropic_api() -> RestApi {
    RestApi {
        name: "Anthropic".to_string(),
        description: "Anthropic Messages API for Claude AI interactions and agent tool use"
            .to_string(),
        base_url: "https://api.anthropic.com/v1".to_string(),
        docs_url: Some("https://docs.anthropic.com/en/api/messages".to_string()),
        auth: AuthStrategy::ApiKey {
            header: "X-Api-Key".to_string(),
        },
        env_auth: vec!["ANTHROPIC_API_KEY".to_string()],
        env_username: None,
        headers: vec![("anthropic-version".to_string(), "2023-06-01".to_string())],
        endpoints: vec![
            // Core Messages endpoint - the heart of the agent API
            Endpoint {
                id: "CreateMessage".to_string(),
                method: RestMethod::Post,
                path: "/messages".to_string(),
                description: "Create a message with optional tool use for agent interactions"
                    .to_string(),
                request: Some(ApiRequest::json_type("CreateMessageBody")),
                response: ApiResponse::json_type("MessageResponse"),
                headers: vec![],
            },
            // Token counting for cost estimation
            Endpoint {
                id: "CountTokens".to_string(),
                method: RestMethod::Post,
                path: "/messages/count_tokens".to_string(),
                description: "Count tokens in a message before sending".to_string(),
                request: Some(ApiRequest::json_type("CountTokensBody")),
                response: ApiResponse::json_type("CountTokensResponse"),
                headers: vec![],
            },
            // Model discovery
            Endpoint {
                id: "ListModels".to_string(),
                method: RestMethod::Get,
                path: "/models".to_string(),
                description: "List available Claude models".to_string(),
                request: None,
                response: ApiResponse::json_type("ListModelsResponse"),
                headers: vec![],
            },
            Endpoint {
                id: "RetrieveModel".to_string(),
                method: RestMethod::Get,
                path: "/models/{model_id}".to_string(),
                description: "Get information about a specific model".to_string(),
                request: None,
                response: ApiResponse::json_type("ModelInfo"),
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
        let api = define_anthropic_api();

        assert_eq!(api.name, "Anthropic");
        assert_eq!(api.base_url, "https://api.anthropic.com/v1");
        assert!(api.docs_url.is_some());
    }

    #[test]
    fn api_uses_api_key_auth() {
        let api = define_anthropic_api();

        match &api.auth {
            AuthStrategy::ApiKey { header } => {
                assert_eq!(header, "X-Api-Key");
            }
            _ => panic!("Expected ApiKey auth strategy"),
        }
        assert_eq!(api.env_auth, vec!["ANTHROPIC_API_KEY"]);
    }

    #[test]
    fn api_has_version_header() {
        let api = define_anthropic_api();

        let version_header = api
            .headers
            .iter()
            .find(|(k, _)| k == "anthropic-version");
        assert!(version_header.is_some());
        assert_eq!(version_header.unwrap().1, "2023-06-01");
    }

    #[test]
    fn api_has_four_endpoints() {
        let api = define_anthropic_api();
        assert_eq!(api.endpoints.len(), 4);
    }

    #[test]
    fn create_message_endpoint() {
        let api = define_anthropic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "CreateMessage")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/messages");
        assert!(endpoint.request.is_some());
        assert!(matches!(endpoint.response, ApiResponse::Json { .. }));
    }

    #[test]
    fn count_tokens_endpoint() {
        let api = define_anthropic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "CountTokens")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Post);
        assert_eq!(endpoint.path, "/messages/count_tokens");
        assert!(endpoint.request.is_some());
    }

    #[test]
    fn list_models_endpoint() {
        let api = define_anthropic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "ListModels")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/models");
        assert!(endpoint.request.is_none());
    }

    #[test]
    fn retrieve_model_endpoint() {
        let api = define_anthropic_api();
        let endpoint = api
            .endpoints
            .iter()
            .find(|e| e.id == "RetrieveModel")
            .unwrap();

        assert_eq!(endpoint.method, RestMethod::Get);
        assert_eq!(endpoint.path, "/models/{model_id}");
        assert!(endpoint.path.contains("{model_id}"));
    }
}
