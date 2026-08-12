//! Thinking / reasoning block rendering.
//!
//! `Reasoning` deltas are buffered in [`LiveSemanticSink::thinking_stream`]
//! (a [`claudine::render::ThinkingStream`]) and flushed as coalesced blocks
//! through [`Section::Thinking`]. The append itself happens in the step-5
//! Reasoning arm of [`LiveSemanticSink::on_semantic_event`]; the boundary
//! drain lives here.

use super::LiveSemanticSink;
use super::Section;
use claudine::render::StreamRenderable;

impl LiveSemanticSink {
    /// Drain any buffered reasoning into the [`Section::Thinking`] section.
    ///
    /// Called at the top of [`Self::on_semantic_event`] for every
    /// non-`Reasoning` event so a coalesced thought renders as one block
    /// *before* the next event (tool call, output text, turn end) renders —
    /// otherwise a tool line could interleave mid-thought. Also called from
    /// `Drop` so a lone trailing thought still renders.
    ///
    /// Each returned frame is a fully-rendered `▌ ` BlockQuote; splitting it
    /// per line preserves the section dedup and one-blank-between-sections
    /// spacing contract exactly as the previous inline render did.
    pub(crate) fn flush_pending_thinking(&mut self) {
        for frame in self.thinking_stream.close() {
            for line in frame.lines() {
                self.emit_section_line(Section::Thinking, line);
            }
        }
    }
}
