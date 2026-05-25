use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::messages::{Message, SystemPrompt};
use super::tools::{Tool, ToolChoice};

// =============================================================================
// Extended Thinking Types
// =============================================================================

/// Configuration for extended thinking.
///
/// Extended thinking allows the model to reason internally before responding,
/// improving accuracy on complex tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ThinkingConfig {
    /// Whether extended thinking is enabled.
    #[serde(rename = "type")]
    pub thinking_type: ThinkingType,

    /// Token budget for thinking (minimum 1024).
    ///
    /// Only used when type is "enabled".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget_tokens: Option<u32>,
}

/// Extended thinking enablement state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingType {
    /// Extended thinking is enabled.
    Enabled,
    /// Extended thinking is disabled.
    Disabled,
}

impl ThinkingConfig {
    /// Enables extended thinking with the specified budget.
    ///
    /// Budget must be at least 1024 tokens.
    pub fn enabled(budget_tokens: u32) -> Self {
        Self {
            thinking_type: ThinkingType::Enabled,
            budget_tokens: Some(budget_tokens.max(1024)),
        }
    }

    /// Disables extended thinking.
    pub fn disabled() -> Self {
        Self {
            thinking_type: ThinkingType::Disabled,
            budget_tokens: None,
        }
    }
}

// =============================================================================
// Request Types
// =============================================================================

/// Request metadata for tracking purposes.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Metadata {
    /// External user identifier for abuse detection.
    ///
    /// Should be a UUID or hash, not raw identifiable information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Service tier selection for the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    /// Automatically select the appropriate tier.
    Auto,
    /// Use only standard capacity (no priority queue).
    StandardOnly,
}

/// Request body for the Create Message endpoint (POST /v1/messages).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CreateMessageBody {
    /// Model identifier (e.g., "claude-sonnet-4-5-20250514").
    pub model: String,

    /// Conversation messages.
    ///
    /// Messages alternate between user and assistant roles.
    pub messages: Vec<Message>,

    /// Maximum tokens to generate.
    ///
    /// Must be less than the model's maximum output limit.
    pub max_tokens: u32,

    /// System prompt providing context and instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,

    /// Tools available for the model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// How to select tools for use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Sampling temperature (0.0-1.0).
    ///
    /// Lower values are more deterministic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,

    /// Top-P (nucleus) sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,

    /// Top-K sampling parameter.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,

    /// Custom stop sequences.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequences: Option<Vec<String>>,

    /// Whether to stream the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,

    /// Request metadata for tracking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Metadata>,

    /// Service tier selection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<ServiceTier>,

    /// Extended thinking configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

impl Default for CreateMessageBody {
    fn default() -> Self {
        Self {
            model: String::new(),
            messages: Vec::new(),
            max_tokens: 1024,
            system: None,
            tools: None,
            tool_choice: None,
            temperature: None,
            top_p: None,
            top_k: None,
            stop_sequences: None,
            stream: None,
            metadata: None,
            service_tier: None,
            thinking: None,
        }
    }
}

impl CreateMessageBody {
    /// Creates a new message request.
    pub fn new(model: impl Into<String>, messages: Vec<Message>, max_tokens: u32) -> Self {
        Self {
            model: model.into(),
            messages,
            max_tokens,
            ..Default::default()
        }
    }

    /// Sets the system prompt.
    pub fn with_system(mut self, system: impl Into<String>) -> Self {
        self.system = Some(SystemPrompt::text(system));
        self
    }

    /// Adds tools for the model to use.
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// Sets the tool choice strategy.
    pub fn with_tool_choice(mut self, choice: ToolChoice) -> Self {
        self.tool_choice = Some(choice);
        self
    }

    /// Sets the sampling temperature.
    pub fn with_temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// Enables extended thinking with the specified budget.
    pub fn with_thinking(mut self, budget_tokens: u32) -> Self {
        self.thinking = Some(ThinkingConfig::enabled(budget_tokens));
        self
    }
}

/// Request body for the Count Tokens endpoint (POST /v1/messages/count_tokens).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CountTokensBody {
    /// Model identifier.
    pub model: String,

    /// Conversation messages.
    pub messages: Vec<Message>,

    /// System prompt.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<SystemPrompt>,

    /// Tools to include in token count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,

    /// Tool choice strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_choice: Option<ToolChoice>,

    /// Extended thinking configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<ThinkingConfig>,
}

impl CountTokensBody {
    /// Creates a new count tokens request.
    pub fn new(model: impl Into<String>, messages: Vec<Message>) -> Self {
        Self {
            model: model.into(),
            messages,
            ..Default::default()
        }
    }
}

// =============================================================================
// Response Types (API-level)
// =============================================================================

/// Response from the Count Tokens endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CountTokensResponse {
    /// Number of input tokens.
    pub input_tokens: u32,

    /// Tokens that would be written to cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,

    /// Tokens that would be read from cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

// =============================================================================
// Models API Types
// =============================================================================

/// Information about an available model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ModelInfo {
    /// Unique model identifier.
    pub id: String,

    /// RFC 3339 datetime of when the model was released.
    pub created_at: String,

    /// Human-readable display name.
    pub display_name: String,

    /// Object type (always "model").
    #[serde(rename = "type")]
    pub model_type: String,
}

/// Response from the List Models endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ListModelsResponse {
    /// List of available models.
    pub data: Vec<ModelInfo>,

    /// First ID in the list (for backward pagination).
    pub first_id: String,

    /// Last ID in the list (for forward pagination).
    pub last_id: String,

    /// Whether more results are available.
    pub has_more: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_message_body_default() {
        let body = CreateMessageBody::new(
            "claude-sonnet-4-5-20250514",
            vec![Message::user("Hello")],
            1024,
        );

        assert_eq!(body.model, "claude-sonnet-4-5-20250514");
        assert_eq!(body.max_tokens, 1024);
        assert!(body.tools.is_none());
    }

    #[test]
    fn create_message_body_with_tools() {
        let tool = Tool::new(
            "search",
            "Search the web",
            serde_json::json!({
                "type": "object",
                "properties": {"query": {"type": "string"}},
                "required": ["query"]
            }),
        );

        let body = CreateMessageBody::new(
            "claude-sonnet-4-5-20250514",
            vec![Message::user("Search for rust tutorials")],
            1024,
        )
        .with_tools(vec![tool])
        .with_tool_choice(ToolChoice::Auto);

        assert!(body.tools.is_some());
        assert_eq!(body.tools.as_ref().unwrap().len(), 1);
    }

    #[test]
    fn thinking_config_creation() {
        let enabled = ThinkingConfig::enabled(2048);
        assert!(matches!(enabled.thinking_type, ThinkingType::Enabled));
        assert_eq!(enabled.budget_tokens, Some(2048));

        let disabled = ThinkingConfig::disabled();
        assert!(matches!(disabled.thinking_type, ThinkingType::Disabled));
    }

    #[test]
    fn model_info_deserialization() {
        let json = r#"{
            "id": "claude-sonnet-4-5-20250514",
            "created_at": "2025-05-14T00:00:00Z",
            "display_name": "Claude Sonnet 4.5",
            "type": "model"
        }"#;

        let model: ModelInfo = serde_json::from_str(json).unwrap();
        assert_eq!(model.id, "claude-sonnet-4-5-20250514");
        assert_eq!(model.display_name, "Claude Sonnet 4.5");
    }

    #[test]
    fn list_models_response_deserialization() {
        let json = r#"{
            "data": [
                {
                    "id": "claude-opus-4-5-20251101",
                    "created_at": "2025-11-01T00:00:00Z",
                    "display_name": "Claude Opus 4.5",
                    "type": "model"
                }
            ],
            "first_id": "claude-opus-4-5-20251101",
            "last_id": "claude-opus-4-5-20251101",
            "has_more": false
        }"#;

        let response: ListModelsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.len(), 1);
        assert!(!response.has_more);
    }

    #[test]
    fn count_tokens_body_creation() {
        let body = CountTokensBody::new(
            "claude-sonnet-4-5-20250514",
            vec![Message::user("How many tokens is this?")],
        );

        assert_eq!(body.model, "claude-sonnet-4-5-20250514");
        assert_eq!(body.messages.len(), 1);
    }
}
