//! Streaming reasoning (thinking) renderer.
//!
//! `ThinkingStream` is the [`StreamRenderable`] consumer for
//! `SemanticEvent::Reasoning` deltas. It exists to eliminate provider variance
//! in *render quality*: Claude emits one `Reasoning` event per `thinking_delta`
//! (token-level fragments), Kimi accumulates a whole thought into one event,
//! and Codex/OpenCode emit block-level. Rendering each delta as its own
//! bordered [`render_thinking_block`] made a single Claude thought appear as a
//! run of short bordered fragments while Kimi's identical thought rendered as
//! one clean block. Buffering deltas here and flushing coalesced blocks makes
//! every provider's thinking render identically *by construction* — the
//! component treats one-delta Kimi and many-delta Claude the same way, with no
//! `match provider`.
//!
//! ## Flush rule
//!
//! On `append`, text accumulates into a buffer and everything up to the last
//! newline flushes as one coalesced block; the trailing partial line is held so
//! an in-progress sentence keeps growing across deltas. A held partial that
//! passes [`SENTENCE_FLUSH_MIN_BYTES`] and ends a sentence flushes early so a
//! long single-paragraph thought still shows progress. `flush_idle` drains a
//! buffer sitting idle past the heartbeat threshold, and `close` drains the
//! remainder at end-of-stream.
//!
//! The *coalescing win* comes from the sink flushing this stream (via `close`)
//! before any non-`Reasoning` event renders: Claude's residual token fragments
//! — short, newline-free, below the sentence threshold — collapse into one
//! block at that boundary instead of one bordered fragment per delta.
//!
//! Design authority:
//! `features/2026-07-02-provider-metadata/design/render-components.md`
//! (Rulings 1 + 3).

use std::time::{Duration, Instant};

use biscuit_terminal::terminal::Terminal;

use crate::render::StreamRenderable;
use crate::render::assistant_stream::line_ends_sentence;
use crate::stream::thinking::render_thinking_block;

/// Minimum buffered byte count before a sentence terminator at the tail of the
/// held partial is allowed to trigger an early progress flush. Short thoughts
/// stay buffered so they coalesce into a single block at the next boundary
/// rather than rendering as their own pseudo-fragment. Mirrors
/// `AssistantStream`'s threshold.
const SENTENCE_FLUSH_MIN_BYTES: usize = 200;

/// Buffers streamed reasoning deltas and flushes coalesced thinking blocks.
///
/// Each flushed chunk renders through [`render_thinking_block`] — the same
/// dim-italic Prose in a `▌ ` [`BlockQuote`](biscuit_terminal::components::block_quote::BlockQuote)
/// used for a single block — so downstream per-line section dedup and spacing
/// keep working unchanged. Sink concerns (which writer, TTY detection, color
/// depth) stay with the caller: the CLI resolves the [`Terminal`] and passes it
/// in, then writes the frames each phase returns.
pub struct ThinkingStream {
    /// Reasoning text accumulated but not yet flushed.
    buffer: String,
    /// Timestamp of the last append into `buffer`. Drives
    /// [`StreamRenderable::flush_idle`]; reset to `None` whenever the buffer
    /// fully drains.
    last_growth_at: Option<Instant>,
    /// Terminal used to render each coalesced block.
    term: Terminal,
}

impl ThinkingStream {
    /// Construct a streaming reasoning renderer over an already-resolved
    /// [`Terminal`]. The CLI owns the `stderr().is_terminal()` decision and
    /// passes the resolved terminal here — this component never probes the
    /// terminal itself.
    pub fn new(term: Terminal) -> Self {
        Self {
            buffer: String::new(),
            last_growth_at: None,
            term,
        }
    }

    /// Render `text` as a thinking block and push its frame, skipping empty
    /// renders (whitespace-only input renders to nothing).
    fn render_frame(&self, frames: &mut Vec<String>, text: &str) {
        let block = render_thinking_block(text, &self.term);
        if !block.is_empty() {
            frames.push(block);
        }
    }

    /// Drain the entire buffer as one coalesced block.
    fn flush_all(&mut self, frames: &mut Vec<String>) {
        if self.buffer.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.buffer);
        self.last_growth_at = None;
        self.render_frame(frames, &text);
    }
}

impl StreamRenderable for ThinkingStream {
    fn append(&mut self, chunk: &str) -> Vec<String> {
        let mut frames = Vec::new();
        if chunk.is_empty() {
            return frames;
        }
        self.buffer.push_str(chunk);
        self.last_growth_at = Some(Instant::now());

        // Flush every complete line (through the last newline) as one block,
        // holding the trailing partial so an in-progress sentence keeps
        // accumulating across deltas.
        if let Some(last_newline) = self.buffer.rfind('\n') {
            let complete: String = self.buffer.drain(..=last_newline).collect();
            self.render_frame(&mut frames, &complete);
            if self.buffer.is_empty() {
                self.last_growth_at = None;
            }
        }

        // Progress flush for a long single-paragraph thought that never sends
        // a newline: once the held partial passes the threshold and ends a
        // sentence, flush it so the user sees the thought grow instead of
        // waiting for the next event boundary.
        if self.buffer.len() >= SENTENCE_FLUSH_MIN_BYTES
            && line_ends_sentence(self.buffer.trim_end())
        {
            self.flush_all(&mut frames);
        }

        frames
    }

    /// Flush the buffered thought if it has been sitting idle for at least
    /// `idle_threshold`. Empty buffer or a fresh append inside the threshold is
    /// a no-op — this is the heartbeat's stall-visibility hook.
    fn flush_idle(&mut self, idle_threshold: Duration) -> Vec<String> {
        let mut frames = Vec::new();
        if self.buffer.is_empty() {
            return frames;
        }
        let Some(stamped_at) = self.last_growth_at else {
            return frames;
        };
        if stamped_at.elapsed() < idle_threshold {
            return frames;
        }
        self.flush_all(&mut frames);
        frames
    }

    /// Drain the remaining buffered thought at end-of-stream (or at the sink's
    /// boundary flush before a non-`Reasoning` event renders).
    fn close(&mut self) -> Vec<String> {
        let mut frames = Vec::new();
        self.flush_all(&mut frames);
        frames
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Concatenate the frames a phase returns into the exact byte stream the
    /// sink would write.
    fn frames(v: Vec<String>) -> String {
        v.concat()
    }

    fn renderer() -> ThinkingStream {
        ThinkingStream::new(Terminal::new_optimistic(80))
    }

    fn thinking_block_count(rendered: &str) -> usize {
        rendered
            .lines()
            .filter(|l| l.contains('\u{258c}'))
            .count()
    }

    #[test]
    fn coalesces_token_fragments_into_one_block_at_close() {
        // Claude-shaped: one sentence arrives as several newline-free deltas.
        // Each delta on its own must NOT render; the whole thought coalesces
        // into a single bordered block at the boundary flush.
        let mut r = renderer();
        assert!(frames(r.append("Let me ")).is_empty());
        assert!(frames(r.append("think about ")).is_empty());
        assert!(frames(r.append("this problem.")).is_empty());

        let flushed = frames(r.close());
        assert!(
            flushed.contains("Let me think about this problem."),
            "coalesced block must carry the joined text: {flushed:?}"
        );
        assert_eq!(
            thinking_block_count(&flushed),
            1,
            "the joined thought must render as ONE ▌ block, not per-fragment: {flushed:?}"
        );
    }

    #[test]
    fn flushes_completed_lines_and_holds_trailing_partial() {
        let mut r = renderer();
        let flushed = frames(r.append("first paragraph\nsecond partial"));
        assert!(
            flushed.contains("first paragraph"),
            "completed line should flush: {flushed:?}"
        );
        assert!(
            !flushed.contains("second partial"),
            "trailing partial must stay buffered: {flushed:?}"
        );
        // The held partial drains on close.
        let rest = frames(r.close());
        assert!(rest.contains("second partial"), "close must drain tail: {rest:?}");
    }

    #[test]
    fn kimi_shaped_single_event_renders_one_block() {
        // Kimi accumulates a whole thought into one Reasoning event. A single
        // append with no trailing newline holds until the boundary, then
        // renders as one block — identical to the Claude coalesced case.
        let mut r = renderer();
        assert!(frames(r.append("A fully accumulated thought")).is_empty());
        let flushed = frames(r.close());
        assert_eq!(thinking_block_count(&flushed), 1);
        assert!(flushed.contains("A fully accumulated thought"));
    }

    #[test]
    fn long_single_paragraph_flushes_on_sentence_terminator() {
        let mut r = renderer();
        let long = "This is a long uninterrupted thought the agent is forming as one \
            paragraph that never emits a newline and would otherwise stay invisible in \
            the buffer until the next event boundary flushes it out to the reader.";
        assert!(long.len() > SENTENCE_FLUSH_MIN_BYTES);
        let flushed = frames(r.append(long));
        assert!(
            flushed.contains("uninterrupted thought"),
            "long sentence-terminated thought should flush early: {flushed:?}"
        );
        assert!(r.buffer.is_empty(), "buffer should drain after progress flush");
    }

    #[test]
    fn short_thought_does_not_early_flush() {
        let mut r = renderer();
        let flushed = frames(r.append("Hmm."));
        assert!(flushed.is_empty(), "short thought must stay buffered: {flushed:?}");
        assert!(!r.buffer.is_empty());
    }

    #[test]
    fn flush_idle_emits_after_threshold() {
        let mut r = renderer();
        assert!(frames(r.append("dangling thought")).is_empty());
        // Threshold not reached — no-op.
        assert!(r.flush_idle(Duration::from_secs(60)).is_empty());
        std::thread::sleep(Duration::from_millis(20));
        let flushed = frames(r.flush_idle(Duration::from_millis(5)));
        assert!(flushed.contains("dangling thought"), "idle flush should fire: {flushed:?}");
        assert!(r.buffer.is_empty());
    }

    #[test]
    fn flush_idle_noop_when_empty() {
        let mut r = renderer();
        assert!(r.flush_idle(Duration::from_millis(0)).is_empty());
    }

    #[test]
    fn close_on_empty_is_noop() {
        let mut r = renderer();
        assert!(frames(r.close()).is_empty());
    }
}
