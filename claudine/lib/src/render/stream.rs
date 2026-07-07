//! Incremental (streaming) render contract for wrapped-session output.
//!
//! `StreamRenderable` is the claudine-local, additive counterpart to
//! `biscuit_terminal`'s `TerminalRenderable` for components whose content
//! arrives as deltas rather than as one finished block (`ThinkingToken`,
//! `AssistantStream`). It is deliberately *not* upstreamed into the shared
//! renderable crate in v1 — promote it only if darkmatter/biscuit-terminal
//! grow the same need. Design authority:
//! `features/2026-07-02-provider-metadata/design/render-components.md`
//! (Ruling 3, "streaming contract").
//!
//! ## The writer stays with the caller
//!
//! Each phase returns the exact byte-frames the sink should write, in order —
//! the component never holds a `W: Write`. This keeps sink concerns (which
//! writer, TTY detection, color depth, the stdout/stderr newline boundary)
//! CLI-side per Ruling 4, while the block-boundary state machine lives in the
//! lib component. A frame is already fully rendered: markdown output that has
//! been newline-normalized, or a raw partial line / newline-only follow-up.

use std::time::Duration;

/// Three-phase span contract for incrementally-rendered stream components.
///
/// The concatenation of every frame returned across a component's lifetime is
/// the exact byte stream the sink writes — no frame is re-ordered, elided, or
/// transformed by the caller.
pub trait StreamRenderable {
    /// Frames to emit before the first content. Usually none.
    fn open(&mut self) -> Vec<String> {
        Vec::new()
    }

    /// Accumulate a delta and return any frames that completed as a result.
    fn append(&mut self, chunk: &str) -> Vec<String>;

    /// Heartbeat hook: flush buffered content that has been idle at least
    /// `idle_threshold`. Default no-op for components without an idle buffer.
    fn flush_idle(&mut self, idle_threshold: Duration) -> Vec<String> {
        let _ = idle_threshold;
        Vec::new()
    }

    /// Drain the tail at end-of-stream.
    fn close(&mut self) -> Vec<String>;
}
