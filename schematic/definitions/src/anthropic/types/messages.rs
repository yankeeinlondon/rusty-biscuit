use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::tools::CacheControl;

/// The role of a message in the conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// A message from the user.
    User,
    /// A message from the assistant.
    Assistant,
}

/// Source for image content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum ToolResultContent {
    /// Simple text result.
    Text(String),
    /// Rich content blocks.
    Blocks(Vec<ContentBlock>),
}

/// A message in the conversation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

/// Why the model stopped generating.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
}
