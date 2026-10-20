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

use crate::registry::assemble_module_group;
use schematic_define::{DefinitionContext, DefinitionError};
#[cfg(test)]
use schematic_define::{ApiResponse, RestMethod};
use schematic_define::{
    ApiKeyEnv, AuthStrategy, Endpoint, EnvList, EnvMapping,
    RestApi,
    pagination::{Collection},
};

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
    module_group().expect("valid anthropic definitions").remove(0)
}

fn build_anthropic_api(ctx: &mut DefinitionContext) -> Result<RestApi, DefinitionError> {
    let endpoints = vec![

            Endpoint::post("CreateMessage", "/messages")
                .describes("Create a message with optional tool use for agent interactions")
                .request_json::<CreateMessageBody>(ctx)
                .returns_json::<MessageResponse>(ctx)
                .build()?,

            Endpoint::post("CountTokens", "/messages/count_tokens")
                .describes("Count tokens in a message before sending")
                .request_json::<CountTokensBody>(ctx)
                .returns_json::<CountTokensResponse>(ctx)
                .build()?,

            Endpoint::get("ListModels", "/models")
                .describes("List available Claude models")
                .returns_json::<ListModelsResponse>(ctx)
                .pagination(Collection::at("data").item_type("ModelInfo").unpaginated())
                .build()?,
            Endpoint::get("RetrieveModel", "/models/{model_id}")
                .describes("Get information about a specific model")
                .returns_json::<ModelInfo>(ctx)
                .build()?,
        ];
    RestApi::builder(ctx, "Anthropic", "https://api.anthropic.com/v1")
        .describes("Anthropic Messages API for Claude AI interactions and agent tool use")
        .authentication(AuthStrategy::ApiKey {
            header: "X-Api-Key".to_string(),
            value_prefix: None,
        }, vec!["ANTHROPIC_API_KEY".to_string()])
        .docs_url("https://docs.anthropic.com/en/api/messages")
        .env_mapping(EnvMapping {
            api_key: Some(ApiKeyEnv {
                names: EnvList::new(vec!["ANTHROPIC_API_KEY".to_string()]),
                header: "X-Api-Key".to_string(),
            }),
            ..Default::default()
        })
        .header("anthropic-version", "2023-06-01")
        .endpoints(endpoints)
        .build()
}

/// Assemble sibling APIs and their ordered nominal schemas in one context.
pub fn module_group() -> Result<Vec<RestApi>, DefinitionError> {
    assemble_module_group("::schematic_definitions::anthropic", |ctx| {
        ctx.register::<CreateMessageBody>()?;
        ctx.register::<CountTokensBody>()?;
        ctx.register::<MessageResponse>()?;
        ctx.register::<CountTokensResponse>()?;
        ctx.register::<ListModelsResponse>()?;
        ctx.register::<ModelInfo>()?;
        ctx.register::<Message>()?;
        ctx.register::<ContentBlock>()?;
        ctx.register::<Tool>()?;
        Ok(vec![
            build_anthropic_api(ctx)?,
        ])
    })
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
            AuthStrategy::ApiKey { header, .. } => {
                assert_eq!(header, "X-Api-Key");
            }
            _ => panic!("Expected ApiKey auth strategy"),
        }
        assert_eq!(api.env_auth, vec!["ANTHROPIC_API_KEY"]);
    }

    #[test]
    fn api_has_version_header() {
        let api = define_anthropic_api();

        let version_header = api.headers.iter().find(|(k, _)| k == "anthropic-version");
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
        let endpoint = api.endpoints.iter().find(|e| e.id == "ListModels").unwrap();

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

    #[test]
    fn module_group_preserves_endpoint_and_registry_inventory() {
        let group = module_group().unwrap();
        assert_eq!(
            group.iter().flat_map(|api| api.endpoints.iter().map(|endpoint| endpoint.id.as_str())).collect::<Vec<_>>(),
            vec!["CreateMessage", "CountTokens", "ListModels", "RetrieveModel"]
        );
        assert_eq!(crate::registry::registered_names(&group[0]), vec!["CreateMessageBody", "ContentBlock", "Message", "Tool", "CountTokensBody", "MessageResponse", "CountTokensResponse", "ListModelsResponse", "ModelInfo"]);
        for api in &group {
            api.validate_references().unwrap();
            assert_eq!(api.schemas, group[0].schemas);
        }
    }

}
