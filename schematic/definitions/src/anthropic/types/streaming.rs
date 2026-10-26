//! Server-sent events returned by Create Message when `stream` is true.
//!
//! Modeled from <https://platform.claude.com/docs/en/build-with-claude/streaming>
//! (evidence collected 2026-10-22). Each SSE event's `data` carries a `type`
//! equal to the SSE event name, so [`MessageStreamEvent`] is internally
//! tagged on `type`. Anthropic may add event types; an unknown `type` fails
//! to decode, which the runtime reports as a malformed stream item while the
//! stream continues.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::messages::{ContentBlock, MessageRole, StopReason, Usage};

/// One event of a streamed Create Message reply.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MessageStreamEvent {
    /// Opens the stream with a message whose `content` is empty.
    MessageStart {
        /// The message being streamed.
        message: StreamMessage,
    },
    /// Opens the content block at `index` of the final message.
    ContentBlockStart {
        /// Position of the block in the final message's `content`.
        index: u32,
        /// The block's initial, usually empty, state.
        content_block: ContentBlock,
    },
    /// An incremental update to the content block at `index`.
    ContentBlockDelta {
        /// Position of the block in the final message's `content`.
        index: u32,
        /// The update to apply.
        delta: ContentBlockDelta,
    },
    /// Closes the content block at `index`.
    ContentBlockStop {
        /// Position of the block in the final message's `content`.
        index: u32,
    },
    /// Top-level changes to the final message.
    MessageDelta {
        /// Changed top-level fields.
        delta: MessageDelta,
        /// Cumulative token counts; omitted by some streams.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        usage: Option<MessageDeltaUsage>,
    },
    /// The final event; the stream's declared terminator.
    MessageStop,
    /// Keep-alive; may appear any number of times.
    Ping,
    /// An error delivered inside the stream, such as `overloaded_error`.
    Error {
        /// The error's type and message.
        error: StreamErrorDetail,
    },
}

impl schematic_define::Registration for MessageStreamEvent {
    const RUST_IDENTIFIER: Option<&'static str> = Some("MessageStreamEvent");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// The update carried by a `content_block_delta` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlockDelta {
    /// Text to append to a `text` block.
    TextDelta {
        /// The text fragment.
        text: String,
    },
    /// A fragment of a `tool_use` block's `input`; fragments concatenate to
    /// JSON only once the block stops.
    InputJsonDelta {
        /// The partial JSON fragment.
        partial_json: String,
    },
    /// Text to append to a `thinking` block.
    ThinkingDelta {
        /// The thinking fragment.
        thinking: String,
    },
    /// The `thinking` block's signature, sent just before the block stops.
    SignatureDelta {
        /// The signature.
        signature: String,
    },
}

impl schematic_define::Registration for ContentBlockDelta {
    const RUST_IDENTIFIER: Option<&'static str> = Some("ContentBlockDelta");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}

/// The message carried by `message_start`.
///
/// Unlike [`super::MessageResponse`], `stop_reason` is null until a later
/// `message_delta`, and `usage` may be omitted.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct StreamMessage {
    /// Unique message identifier.
    pub id: String,
    /// Object type (always "message").
    #[serde(rename = "type")]
    pub message_type: String,
    /// Message role (always "assistant").
    pub role: MessageRole,
    /// Model used for generation.
    pub model: String,
    /// Content blocks; empty at `message_start`.
    #[serde(default)]
    pub content: Vec<ContentBlock>,
    /// Why generation stopped; null at `message_start`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// The stop sequence that was matched, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
    /// Initial token usage.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
}

/// Top-level message fields changed by `message_delta`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MessageDelta {
    /// Why generation stopped.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_reason: Option<StopReason>,
    /// The stop sequence that was matched, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stop_sequence: Option<String>,
}

/// Cumulative token counts reported by `message_delta`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MessageDeltaUsage {
    /// Output tokens so far.
    pub output_tokens: u32,
    /// Input tokens.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u32>,
    /// Tokens written to cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_creation_input_tokens: Option<u32>,
    /// Tokens read from cache.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cache_read_input_tokens: Option<u32>,
}

/// The body of an in-stream `error` event.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StreamErrorDetail {
    /// Error type, such as `overloaded_error`.
    #[serde(rename = "type")]
    pub error_type: String,
    /// Human-readable message.
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(data: &str) -> MessageStreamEvent {
        serde_json::from_str(data).expect("documented event decodes")
    }

    #[test]
    fn message_start_decodes_with_null_stop_reason() {
        let event = decode(r#"{"type": "message_start", "message": {"id": "msg_1nZdL29xx5MUA1yADyHTEsnR8uuvGzszyY", "type": "message", "role": "assistant", "content": [], "model": "claude-opus-5-5", "stop_reason": null, "stop_sequence": null, "usage": {"input_tokens": 25, "output_tokens": 1}}}"#);
        let MessageStreamEvent::MessageStart { message } = event else { panic!("expected message_start, got {event:?}") };
        assert_eq!(message.id, "msg_1nZdL29xx5MUA1yADyHTEsnR8uuvGzszyY");
        assert!(message.content.is_empty());
        assert_eq!(message.stop_reason, None);
        assert_eq!(message.usage.map(|usage| usage.input_tokens), Some(25));
    }

    #[test]
    fn message_start_decodes_without_usage() {
        let event = decode(r#"{"type": "message_start", "message": {"id": "msg_01", "type": "message", "role": "assistant", "content": [], "model": "claude-opus-5-5", "stop_reason": null, "stop_sequence": null}}"#);
        assert!(matches!(event, MessageStreamEvent::MessageStart { message } if message.usage.is_none()));
    }

    #[test]
    fn content_block_start_decodes_text_tool_use_and_thinking() {
        let text = decode(r#"{"type": "content_block_start", "index": 0, "content_block": {"type": "text", "text": ""}}"#);
        assert!(matches!(text, MessageStreamEvent::ContentBlockStart { index: 0, content_block: ContentBlock::Text { .. } }));
        let tool = decode(r#"{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"toolu_01T1x1fJ34qAmk2tNTrN7Up6","name":"get_weather","input":{}}}"#);
        assert!(matches!(tool, MessageStreamEvent::ContentBlockStart { index: 1, content_block: ContentBlock::ToolUse { ref name, .. } } if name == "get_weather"));
        let thinking = decode(r#"{"type": "content_block_start", "index": 0, "content_block": {"type": "thinking", "thinking": "", "signature": ""}}"#);
        assert!(matches!(thinking, MessageStreamEvent::ContentBlockStart { content_block: ContentBlock::Thinking { .. }, .. }));
    }

    #[test]
    fn content_block_delta_decodes_every_delta_kind() {
        let cases = [
            (r#"{"type": "content_block_delta","index": 0,"delta": {"type": "text_delta", "text": "ello frien"}}"#, ContentBlockDelta::TextDelta { text: "ello frien".into() }),
            (r#"{"type": "content_block_delta","index": 0,"delta": {"type": "input_json_delta","partial_json": "{\"location\": \"San Fra"}}"#, ContentBlockDelta::InputJsonDelta { partial_json: r#"{"location": "San Fra"#.into() }),
            (r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "thinking_delta", "thinking": "1071 = 2 × 462 + 147"}}"#, ContentBlockDelta::ThinkingDelta { thinking: "1071 = 2 × 462 + 147".into() }),
            (r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "signature_delta", "signature": "EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxBdjrkzLoky3dl1pkiMOYds..."}}"#, ContentBlockDelta::SignatureDelta { signature: "EqQBCgIYAhIM1gbcDa9GJwZA2b3hGgxBdjrkzLoky3dl1pkiMOYds...".into() }),
        ];
        for (data, expected) in cases {
            assert_eq!(decode(data), MessageStreamEvent::ContentBlockDelta { index: 0, delta: expected });
        }
    }

    #[test]
    fn content_block_stop_decodes() {
        assert_eq!(decode(r#"{"type": "content_block_stop", "index": 2}"#), MessageStreamEvent::ContentBlockStop { index: 2 });
    }

    #[test]
    fn message_delta_decodes_with_and_without_usage() {
        let with_usage = decode(r#"{"type":"message_delta","delta":{"stop_reason":"end_turn","stop_sequence":null},"usage":{"input_tokens":10682,"cache_creation_input_tokens":0,"cache_read_input_tokens":0,"output_tokens":510,"server_tool_use":{"web_search_requests":1}}}"#);
        let MessageStreamEvent::MessageDelta { delta, usage } = with_usage else { panic!("expected message_delta") };
        assert_eq!(delta.stop_reason, Some(StopReason::EndTurn));
        assert_eq!(usage.map(|usage| (usage.output_tokens, usage.input_tokens)), Some((510, Some(10682))));

        let without_usage = decode(r#"{"type": "message_delta", "delta": {"stop_reason": "tool_use", "stop_sequence": null}}"#);
        assert!(matches!(without_usage, MessageStreamEvent::MessageDelta { usage: None, delta } if delta.stop_reason == Some(StopReason::ToolUse)));
    }

    #[test]
    fn message_stop_and_ping_decode_as_unit_variants() {
        assert_eq!(decode(r#"{"type": "message_stop"}"#), MessageStreamEvent::MessageStop);
        assert_eq!(decode(r#"{"type": "ping"}"#), MessageStreamEvent::Ping);
    }

    #[test]
    fn error_event_decodes_as_typed_variant() {
        let event = decode(r#"{"type": "error", "error": {"type": "overloaded_error", "message": "Overloaded"}}"#);
        assert_eq!(event, MessageStreamEvent::Error { error: StreamErrorDetail { error_type: "overloaded_error".into(), message: "Overloaded".into() } });
    }

    #[test]
    fn unknown_event_type_fails_to_decode() {
        // The runtime surfaces this failure as a malformed stream item and
        // keeps reading, so a future event type does not end the stream.
        assert!(serde_json::from_str::<MessageStreamEvent>(r#"{"type": "message_annotation", "index": 0}"#).is_err());
        assert!(serde_json::from_str::<MessageStreamEvent>(r#"{"type": "content_block_delta", "index": 0, "delta": {"type": "future_delta", "value": 1}}"#).is_err());
    }
}
