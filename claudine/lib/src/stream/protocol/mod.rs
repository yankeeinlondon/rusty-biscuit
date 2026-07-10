//! Strongly-typed protocol models for agentic CLI stream events.
//!
//! Each submodule defines serde-derived structs and a tagged enum for a single
//! provider's streaming event format. Parsers in the parent module deserialize
//! event lines into these typed values instead of walking `serde_json::Value`
//! trees. Unknown event types fall through to a `Value`-based fallback in the
//! parser so the surrounding pipeline continues to tolerate format evolution.

pub mod antigravity;
pub mod claude;
pub mod codex;
pub mod gemini;
pub mod kimi;
pub mod opencode;
pub mod pi;
pub mod qwen;

/// Errors produced while attempting to deserialize a provider event line.
///
/// Currently used only as documentation for the contract between protocol
/// modules and stream parsers. Parsers handle deserialization failures inline
/// by falling back to `serde_json::Value` parsing and tracing the unknown
/// event type — this enum exists so future callers can surface typed errors
/// without changing the protocol modules.
#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("unknown event type: {0}")]
    UnknownEventType(String),
    #[error("deserialization failed for {event_type}: {reason}")]
    DeserializationFailed { event_type: String, reason: String },
}
