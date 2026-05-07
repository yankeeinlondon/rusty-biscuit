//! Thinking / reasoning block rendering.
//!
//! Reasoning events are handled inline in [`LiveSemanticSink::on_semantic_event`]
//! by calling [`claudine::stream::thinking::render_thinking_block`] and
//! forwarding the rendered lines through [`Section::Thinking`].

// All thinking-related logic is currently inline in the SemanticEventSink impl.
// This module is reserved for future extraction of reasoning-specific helpers.
