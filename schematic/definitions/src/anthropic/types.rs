//! Anthropic API types.
//!
//! This module contains request and response types for the Anthropic Messages API,
//! including support for tool use, extended thinking, and the agent loop pattern.
//!
//! ## Agent Loop Pattern
//!
//! The Messages API supports iterative tool use through the agent loop:
//!
//! 1. Send messages with tools defined
//! 2. If response has `stop_reason: "tool_use"`, execute requested tools
//! 3. Append tool results to messages and continue
//! 4. Repeat until `stop_reason: "end_turn"`

use serde::{Deserialize, Serialize};

// =============================================================================
// Content Block Types
// =============================================================================

/// The role of a message in the conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// A message from the user.
    User,
    /// A message from the assistant.
    Assistant,
}

/// Cache control settings for prompt caching.
///
/// Prompt caching reduces costs by up to 90% for repeated context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CacheControl {
    /// Cache type (currently only "ephemeral" is supported).
    #[serde(rename = "type")]
    pub cache_type: String,

    /// Time-to-live for the cache entry.
    ///
    /// Valid values: "5m" (default), "1h"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<String>,
}

impl CacheControl {
    /// Creates a new ephemeral cache control with default TTL.
    pub fn ephemeral() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: None,
        }
    }

    /// Creates a new ephemeral cache control with 1-hour TTL.
    pub fn ephemeral_1h() -> Self {
        Self {
            cache_type: "ephemeral".to_string(),
            ttl: Some("1h".to_string()),
        }
    }
}

/// Source for image content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ImageSource {
    /// Base64-encoded image data.
    Base64 {
        /// The base64-encoded image data.
        data: String,
        /// MIME type of the image.
        media_type: String,
    },
    /// URL reference to an image.
    Url {
        /// The URL of the image.
        url: String,
    },
}

/// Source for document content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DocumentSource {
    /// Base64-encoded document data.
    Base64 {
        /// The base64-encoded document data.
        data: String,
        /// MIME type of the document.
        media_type: String,
    },
    /// URL reference to a document.
    Url {
        /// The URL of the document.
        url: String,
    },
    /// Plain text content.
    Text {
        /// The text content.
        data: String,
        /// MIME type (typically "text/plain").
        media_type: String,
    },
}

/// Content block for messages.
///
/// Messages contain an array of content blocks that can include text,
/// images, documents, tool use requests, and tool results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content.
    Text {
        /// The text content.
        text: String,
        /// Optional cache control for prompt caching.
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Image content.
    Image {
        /// The image source (base64 or URL).
        source: ImageSource,
        /// Optional cache control for prompt caching.
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Document content (PDF, text files).
    Document {
        /// The document source.
        source: DocumentSource,
        /// Optional title for the document.
        #[serde(skip_serializing_if = "Option::is_none")]
        title: Option<String>,
        /// Optional context about the document.
        #[serde(skip_serializing_if = "Option::is_none")]
        context: Option<String>,
        /// Optional cache control for prompt caching.
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },

    /// Tool use request from the assistant.
    ///
    /// When the model wants to use a tool, it returns this content block.
    /// Execute the tool and return a `ToolResult` block in the next message.
    ToolUse {
        /// Unique identifier for this tool use.
        id: String,
        /// Name of the tool to execute.
        name: String,
        /// Input parameters for the tool.
        input: serde_json::Value,
    },

    /// Result from a tool execution.
    ///
    /// Include this in a user message after executing a tool requested
    /// by the assistant.
    ToolResult {
        /// The tool_use_id from the corresponding ToolUse block.
        tool_use_id: String,
        /// The result content (string or nested content blocks).
        content: ToolResultContent,
        /// Whether the tool execution resulted in an error.
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },

    /// Extended thinking block (response only).
    ///
    /// Contains the model's internal reasoning when extended thinking is enabled.
    Thinking {
        /// The thinking content.
        thinking: String,
        /// Cryptographic signature for verification.
        #[serde(skip_serializing_if = "Option::is_none")]
        signature: Option<String>,
    },
}

impl ContentBlock {
    /// Creates a text content block.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: None,
        }
    }

    /// Creates a text content block with cache control.
    pub fn text_cached(text: impl Into<String>, cache: CacheControl) -> Self {
        Self::Text {
            text: text.into(),
            cache_control: Some(cache),
        }
    }

    /// Creates a tool result content block.
    pub fn tool_result(tool_use_id: impl Into<String>, content: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: ToolResultContent::Text(content.into()),
            is_error: None,
        }
    }

    /// Creates a tool result content block indicating an error.
    pub fn tool_error(tool_use_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self::ToolResult {
            tool_use_id: tool_use_id.into(),
            content: ToolResultContent::Text(error.into()),
            is_error: Some(true),
        }
    }
}

/// Content for a tool result.
///
/// Can be a simple string or an array of content blocks for rich results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Simple text result.
    Text(String),
    /// Rich content blocks.
    Blocks(Vec<ContentBlock>),
}

// =============================================================================
// Tool Definition Types
// =============================================================================

/// A tool definition for the model to use.
///
/// Tools are defined using JSON Schema for the input parameters.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Tool {
    /// Unique name for the tool.
    ///
    /// Must match the pattern `^[a-zA-Z0-9_-]{1,64}$`.
    pub name: String,

    /// Human-readable description of what the tool does.
    ///
    /// Include when and how to use the tool. Be specific about trigger conditions.
    pub description: String,

    /// JSON Schema defining the input parameters.
    pub input_schema: serde_json::Value,

    /// Optional cache control for the tool definition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl Tool {
    /// Creates a new tool definition.
    pub fn new(
        name: impl Into<String>,
        description: impl Into<String>,
        input_schema: serde_json::Value,
    ) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema,
            cache_control: None,
        }
    }

    /// Creates a tool with an object schema.
    ///
    /// Helper for the common case of tools with object-typed inputs.
    pub fn with_object_schema(
        name: impl Into<String>,
        description: impl Into<String>,
        properties: serde_json::Value,
        required: Vec<String>,
    ) -> Self {
        Self::new(
            name,
            description,
            serde_json::json!({
                "type": "object",
                "properties": properties,
                "required": required
            }),
        )
    }
}

/// How the model should choose which tools to use.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolChoice {
    /// Let the model decide whether to use tools.
    Auto,
    /// Model must use at least one tool.
    Any,
    /// Model must use the specified tool.
    Tool {
        /// Name of the required tool.
        name: String,
    },
    /// Model should not use any tools.
    None,
}

// =============================================================================
// Message Types
// =============================================================================

/// A message in the conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    /// The role of the message author.
    pub role: MessageRole,

    /// Content blocks making up the message.
    ///
    /// For simple text messages, use a single TextBlock.
    /// For multimodal or tool interactions, use multiple blocks.
    pub content: Vec<ContentBlock>,
}

impl Message {
    /// Creates a user message with text content.
    pub fn user(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::User,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Creates a user message with multiple content blocks.
    pub fn user_blocks(content: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::User,
            content,
        }
    }

    /// Creates an assistant message with text content.
    pub fn assistant(text: impl Into<String>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content: vec![ContentBlock::text(text)],
        }
    }

    /// Creates an assistant message with multiple content blocks.
    pub fn assistant_blocks(content: Vec<ContentBlock>) -> Self {
        Self {
            role: MessageRole::Assistant,
            content,
        }
    }
}

/// System prompt content block.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SystemContent {
    /// Text system prompt.
    Text {
        /// The system prompt text.
        text: String,
        /// Optional cache control.
        #[serde(skip_serializing_if = "Option::is_none")]
        cache_control: Option<CacheControl>,
    },
}

/// System prompt for the conversation.
///
/// Can be a simple string or an array of content blocks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SystemPrompt {
    /// Simple text system prompt.
    Text(String),
    /// Structured system prompt with cache control.
    Blocks(Vec<SystemContent>),
}

impl SystemPrompt {
    /// Creates a simple text system prompt.
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text(text.into())
    }

    /// Creates a system prompt with cache control.
    pub fn cached(text: impl Into<String>, cache: CacheControl) -> Self {
        Self::Blocks(vec![SystemContent::Text {
            text: text.into(),
            cache_control: Some(cache),
        }])
    }
}

// =============================================================================
// Extended Thinking Types
// =============================================================================

/// Configuration for extended thinking.
///
/// Extended thinking allows the model to reason internally before responding,
/// improving accuracy on complex tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Metadata {
    /// External user identifier for abuse detection.
    ///
    /// Should be a UUID or hash, not raw identifiable information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
}

/// Service tier selection for the request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ServiceTier {
    /// Automatically select the appropriate tier.
    Auto,
    /// Use only standard capacity (no priority queue).
    StandardOnly,
}

/// Request body for the Create Message endpoint (POST /v1/messages).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
// Response Types
// =============================================================================

/// Why the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    /// Natural end of the response.
    EndTurn,
    /// Reached the max_tokens limit.
    MaxTokens,
    /// Hit a custom stop sequence.
    StopSequence,
    /// Model wants to use a tool (continue the agent loop).
    ToolUse,
    /// Model paused for user input.
    PauseTurn,
    /// Model refused to generate content.
    Refusal,
}

impl StopReason {
    /// Returns true if the agent loop should continue.
    ///
    /// The loop should continue when the model requests tool use.
    pub fn should_continue(&self) -> bool {
        matches!(self, Self::ToolUse)
    }

    /// Returns true if this is a terminal state.
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::EndTurn | Self::MaxTokens | Self::StopSequence | Self::Refusal
        )
    }
}

/// Token usage statistics.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Usage {
    /// Number of input tokens.
    pub input_tokens: u32,

    /// Number of output tokens.
    pub output_tokens: u32,

    /// Tokens written to cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,

    /// Tokens read from cache.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,

    /// Service tier used for this request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_tier: Option<String>,
}

/// Response from the Create Message endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MessageResponse {
    /// Unique message identifier.
    pub id: String,

    /// Object type (always "message").
    #[serde(rename = "type")]
    pub response_type: String,

    /// Message role (always "assistant").
    pub role: MessageRole,

    /// Model used for generation.
    pub model: String,

    /// Content blocks in the response.
    ///
    /// May contain text, tool_use, and thinking blocks.
    pub content: Vec<ContentBlock>,

    /// Why generation stopped.
    pub stop_reason: StopReason,

    /// The stop sequence that was matched, if any.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,

    /// Token usage statistics.
    pub usage: Usage,
}

impl MessageResponse {
    /// Returns true if the model is requesting tool use.
    pub fn needs_tool_execution(&self) -> bool {
        self.stop_reason == StopReason::ToolUse
    }

    /// Extracts tool use blocks from the response.
    pub fn tool_use_blocks(&self) -> Vec<&ContentBlock> {
        self.content
            .iter()
            .filter(|b| matches!(b, ContentBlock::ToolUse { .. }))
            .collect()
    }

    /// Extracts text content from the response.
    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                ContentBlock::Text { text, .. } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Response from the Count Tokens endpoint.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
    fn message_role_serialization() {
        assert_eq!(
            serde_json::to_string(&MessageRole::User).unwrap(),
            "\"user\""
        );
        assert_eq!(
            serde_json::to_string(&MessageRole::Assistant).unwrap(),
            "\"assistant\""
        );
    }

    #[test]
    fn content_block_text_serialization() {
        let block = ContentBlock::text("Hello");
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"text\""));
        assert!(json.contains("\"text\":\"Hello\""));
    }

    #[test]
    fn content_block_tool_use_serialization() {
        let block = ContentBlock::ToolUse {
            id: "toolu_123".to_string(),
            name: "calculator".to_string(),
            input: serde_json::json!({"a": 1, "b": 2}),
        };
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_use\""));
        assert!(json.contains("\"name\":\"calculator\""));
    }

    #[test]
    fn content_block_tool_result_serialization() {
        let block = ContentBlock::tool_result("toolu_123", "42");
        let json = serde_json::to_string(&block).unwrap();
        assert!(json.contains("\"type\":\"tool_result\""));
        assert!(json.contains("\"tool_use_id\":\"toolu_123\""));
    }

    #[test]
    fn message_user_creation() {
        let msg = Message::user("Hello, Claude!");
        assert_eq!(msg.role, MessageRole::User);
        assert_eq!(msg.content.len(), 1);
    }

    #[test]
    fn tool_definition() {
        let tool = Tool::with_object_schema(
            "calculator",
            "Perform math operations",
            serde_json::json!({
                "a": {"type": "number"},
                "b": {"type": "number"},
                "op": {"type": "string", "enum": ["add", "subtract"]}
            }),
            vec!["a".to_string(), "b".to_string(), "op".to_string()],
        );

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("\"name\":\"calculator\""));
        assert!(json.contains("\"type\":\"object\""));
    }

    #[test]
    fn tool_choice_serialization() {
        assert_eq!(
            serde_json::to_string(&ToolChoice::Auto).unwrap(),
            "{\"type\":\"auto\"}"
        );
        assert_eq!(
            serde_json::to_string(&ToolChoice::Any).unwrap(),
            "{\"type\":\"any\"}"
        );

        let specific = ToolChoice::Tool {
            name: "calculator".to_string(),
        };
        let json = serde_json::to_string(&specific).unwrap();
        assert!(json.contains("\"type\":\"tool\""));
        assert!(json.contains("\"name\":\"calculator\""));
    }

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
    fn stop_reason_logic() {
        assert!(StopReason::ToolUse.should_continue());
        assert!(!StopReason::EndTurn.should_continue());

        assert!(StopReason::EndTurn.is_terminal());
        assert!(StopReason::MaxTokens.is_terminal());
        assert!(!StopReason::ToolUse.is_terminal());
    }

    #[test]
    fn message_response_deserialization() {
        let json = r#"{
            "id": "msg_123",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250514",
            "content": [{"type": "text", "text": "Hello!"}],
            "stop_reason": "end_turn",
            "usage": {"input_tokens": 10, "output_tokens": 5}
        }"#;

        let response: MessageResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.id, "msg_123");
        assert_eq!(response.stop_reason, StopReason::EndTurn);
        assert!(!response.needs_tool_execution());
    }

    #[test]
    fn message_response_with_tool_use() {
        let json = r#"{
            "id": "msg_456",
            "type": "message",
            "role": "assistant",
            "model": "claude-sonnet-4-5-20250514",
            "content": [
                {"type": "text", "text": "Let me calculate that."},
                {"type": "tool_use", "id": "toolu_789", "name": "calculator", "input": {"a": 5, "b": 3, "op": "add"}}
            ],
            "stop_reason": "tool_use",
            "usage": {"input_tokens": 15, "output_tokens": 20}
        }"#;

        let response: MessageResponse = serde_json::from_str(json).unwrap();
        assert!(response.needs_tool_execution());
        assert_eq!(response.tool_use_blocks().len(), 1);
        assert_eq!(response.text_content(), "Let me calculate that.");
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
    fn cache_control_creation() {
        let ephemeral = CacheControl::ephemeral();
        assert_eq!(ephemeral.cache_type, "ephemeral");
        assert!(ephemeral.ttl.is_none());

        let one_hour = CacheControl::ephemeral_1h();
        assert_eq!(one_hour.ttl, Some("1h".to_string()));
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
    fn system_prompt_variants() {
        let simple = SystemPrompt::text("You are a helpful assistant.");
        let json = serde_json::to_string(&simple).unwrap();
        assert!(json.contains("You are a helpful assistant."));

        let cached = SystemPrompt::cached("You are a helpful assistant.", CacheControl::ephemeral());
        let json = serde_json::to_string(&cached).unwrap();
        assert!(json.contains("cache_control"));
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
