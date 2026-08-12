//! Streaming assistant-text renderer.
//!
//! `AssistantStream` is the canonical [`StreamRenderable`] consumer: every
//! non-interactive TTY session streams assistant [`OutputText`] deltas through
//! it. It accumulates incoming text, detects Markdown block boundaries (blank
//! lines, code-fence closings, stream-safe list items, sentence terminators)
//! and renders complete blocks through the [`FinalMessage`] component for rich
//! terminal output (syntax highlighting, tables, bold/italic, etc.).
//!
//! [`OutputText`]: crate::stream::semantic::SemanticEvent::OutputText

use std::time::{Duration, Instant};

use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::output::terminal::TerminalOptions;

use crate::render::{FinalMessage, StreamRenderable};

/// Renders streamed assistant text as Markdown, flushing at block boundaries.
///
/// Accumulates incoming text and detects Markdown block boundaries (blank
/// lines, code fence closings) to render complete blocks through darkmatter for
/// rich terminal output (syntax highlighting, tables, bold/italic, etc.).
///
/// Sink concerns (which writer, TTY detection, color depth) stay with the
/// caller: the CLI resolves the [`Terminal`] and cached [`TerminalOptions`] and
/// passes them in, then writes the frames each phase returns.
pub struct AssistantStream {
    /// Accumulated text for the current Markdown block.
    block_buffer: String,
    /// Trailing text without a newline (incomplete line).
    line_buffer: String,
    /// Whether we are inside a fenced code block (``` or ~~~).
    in_code_fence: bool,
    /// True when the partial line in `line_buffer` has already been emitted
    /// raw. When the newline eventually arrives we only emit `\n` instead of
    /// re-rendering through darkmatter, avoiding duplicate output. Safe to
    /// enable because all stderr status lines route through the sink's
    /// `StreamOutput`, which guarantees a newline-boundary before writing.
    partial_line_committed: bool,
    /// Timestamp of the last write into `block_buffer`. Used by
    /// [`StreamRenderable::flush_idle`] so the heartbeat thread can surface
    /// buffered assistant text when the provider stalls without emitting a
    /// paragraph boundary. Reset to `None` whenever the block flushes.
    last_block_growth_at: Option<Instant>,
    /// Terminal reference for rendering.
    term: Option<Terminal>,
    /// Cached darkmatter options (created once to avoid repeated theme detection).
    terminal_options: Option<TerminalOptions>,
}

impl AssistantStream {
    /// Construct a streaming renderer over an already-resolved sink.
    ///
    /// `term` is `Some` when the sink is a TTY that should receive rendered
    /// Markdown; `None` streams raw text. `terminal_options` should be a
    /// cached instance (e.g. `image_mode = Never`) so hot-path renders skip
    /// repeated theme detection. The CLI owns the `stdout().is_terminal()`
    /// decision and passes the results here — this component never probes the
    /// terminal itself.
    pub fn new(term: Option<Terminal>, terminal_options: Option<TerminalOptions>) -> Self {
        Self {
            block_buffer: String::new(),
            line_buffer: String::new(),
            in_code_fence: false,
            partial_line_committed: false,
            last_block_growth_at: None,
            term,
            terminal_options,
        }
    }

    fn push_frames(&mut self, frames: &mut Vec<String>, text: &str) {
        if text.is_empty() {
            return;
        }

        self.line_buffer.push_str(text);

        // Extract and process each complete line (ending with \n).
        while let Some(newline_pos) = self.line_buffer.find('\n') {
            let line = self.line_buffer[..=newline_pos].to_string();
            self.line_buffer.drain(..=newline_pos);

            if self.partial_line_committed {
                // Partial line was already streamed raw; emit only the newline
                // and skip markdown rendering to avoid duplicate output.
                frames.push("\n".to_string());
                self.partial_line_committed = false;
                continue;
            }

            self.process_line(frames, &line);
        }

        // Stream the remaining partial line immediately so the user sees
        // progress even when the provider stalls before sending a newline.
        // Safe across the stdout/stderr boundary because status emissions go
        // through `StreamOutput`, which inserts a newline before writing
        // stderr when stdout is mid-line. Skip when we're inside a fenced
        // block or actively accumulating a markdown block — those paths need
        // the full block before rendering.
        if !self.line_buffer.is_empty() && !self.in_code_fence && self.block_buffer.is_empty() {
            let partial = std::mem::take(&mut self.line_buffer);
            frames.push(partial);
            self.partial_line_committed = true;
        }
    }

    /// Process a single complete line, accumulating into the block buffer
    /// and flushing when a block boundary is detected.
    fn process_line(&mut self, frames: &mut Vec<String>, line: &str) {
        let trimmed = line.trim();

        // Track code fence open/close (``` or ~~~)
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            self.append_block(line);
            if self.in_code_fence {
                // Closing fence — render the complete fenced block
                self.in_code_fence = false;
                self.flush_block(frames);
            } else {
                self.in_code_fence = true;
            }
            return;
        }

        // Inside a code fence — just accumulate, don't look for boundaries
        if self.in_code_fence {
            self.append_block(line);
            return;
        }

        // Blank line outside a code fence = block boundary
        if trimmed.is_empty() {
            // Include the blank line so darkmatter sees proper paragraph spacing
            self.append_block(line);
            self.flush_block(frames);
            return;
        }

        // Ordered/unordered list items are complete enough to stream line-by-line.
        // Waiting for a blank line or EOF can hide useful progress for minutes if
        // the provider stalls after emitting the last list item.
        if is_stream_safe_list_item(trimmed) {
            self.flush_block(frames);
            self.append_block(line);
            self.flush_block(frames);
            return;
        }

        // Regular content — accumulate.
        self.append_block(line);

        // Sentence-level early flush: once the block has accumulated past
        // the size threshold and the latest line ends with sentence-
        // terminating punctuation, flush so the user sees prose as it is
        // written instead of waiting for a blank-line boundary. Fence and
        // list cases above already returned, and short buffers fall below
        // the threshold, so this never fires mid-code and never chops
        // short responses.
        if self.block_buffer.len() >= SENTENCE_FLUSH_MIN_BYTES && line_ends_sentence(trimmed) {
            self.flush_block(frames);
        }
    }

    /// Append `content` to the block buffer and stamp the growth clock so
    /// [`StreamRenderable::flush_idle`] can tell how long the buffer has been
    /// sitting idle.
    fn append_block(&mut self, content: &str) {
        self.block_buffer.push_str(content);
        self.last_block_growth_at = Some(Instant::now());
    }

    /// Render the accumulated block through darkmatter and push its frame.
    fn flush_block(&mut self, frames: &mut Vec<String>) {
        if self.block_buffer.is_empty() {
            return;
        }
        let block = std::mem::take(&mut self.block_buffer);
        self.last_block_growth_at = None;
        self.render_markdown(frames, &block);
    }

    fn render_markdown(&self, frames: &mut Vec<String>, text: &str) {
        if let Some(term) = &self.term {
            let component = FinalMessage::new(text);
            let component = match self.terminal_options.as_ref() {
                Some(opts) => component.with_options(opts.clone()),
                None => component,
            };
            let rendered = component.render(term);
            let normalized = normalize_stream_rendered_newlines(text, &rendered);
            frames.push(normalized);
        } else {
            frames.push(text.to_string());
        }
    }
}

impl StreamRenderable for AssistantStream {
    fn append(&mut self, chunk: &str) -> Vec<String> {
        let mut frames = Vec::new();
        self.push_frames(&mut frames, chunk);
        frames
    }

    /// Flush buffered markdown if it has been sitting idle for at least
    /// `idle_threshold`.
    ///
    /// Called by the heartbeat thread before it emits its own status line so
    /// dangling paragraphs cannot remain invisible while the provider stalls
    /// without closing stdout. An empty buffer is a no-op; a fresh write
    /// inside the threshold is a no-op (returns no frames).
    fn flush_idle(&mut self, idle_threshold: Duration) -> Vec<String> {
        let mut frames = Vec::new();
        if self.block_buffer.is_empty() {
            return frames;
        }
        let Some(stamped_at) = self.last_block_growth_at else {
            return frames;
        };
        if stamped_at.elapsed() < idle_threshold {
            return frames;
        }
        self.flush_block(&mut frames);
        frames
    }

    /// Drain any remaining buffered content (incomplete line + block buffer).
    fn close(&mut self) -> Vec<String> {
        let mut frames = Vec::new();
        if !self.line_buffer.is_empty() {
            let leftover = std::mem::take(&mut self.line_buffer);
            if self.partial_line_committed {
                // Already streamed raw — do not re-render through darkmatter.
                self.partial_line_committed = false;
            } else {
                self.append_block(&leftover);
            }
        }
        self.flush_block(&mut frames);
        frames
    }
}

/// Minimum buffered byte count before a sentence-terminator at the end of a
/// line is allowed to trigger an early flush. Short single-line responses
/// (e.g. `"OK."`) stay buffered so they don't render as their own pseudo-
/// paragraph; only multi-line or otherwise substantial prose qualifies.
const SENTENCE_FLUSH_MIN_BYTES: usize = 200;

/// Returns `true` when the trimmed line ends with a sentence-terminating
/// character (`.`, `!`, `?`), optionally followed by a trailing closing
/// quote / bracket / parenthesis. Trailing whitespace is already stripped by
/// the caller.
///
/// Shared with [`ThinkingStream`](crate::render::ThinkingStream) so both
/// stream components apply the same sentence-boundary heuristic for their
/// progress flush.
pub(crate) fn line_ends_sentence(trimmed: &str) -> bool {
    let bytes = trimmed.as_bytes();
    let mut idx = bytes.len();
    while idx > 0 {
        let ch = bytes[idx - 1];
        if matches!(ch, b'"' | b'\'' | b')' | b']' | b'}') {
            idx -= 1;
            continue;
        }
        return matches!(ch, b'.' | b'!' | b'?');
    }
    false
}

fn is_stream_safe_list_item(line: &str) -> bool {
    line.starts_with("- ") || line.starts_with("* ") || line.starts_with("+ ") || {
        let digits = line.bytes().take_while(|b| b.is_ascii_digit()).count();
        digits > 0 && line[digits..].starts_with(". ")
    }
}

/// Darkmatter renders each streamed fragment as a standalone Markdown
/// document. For short fragments such as a single heading or list item,
/// that can add synthetic trailing blank lines that were not present in
/// the provider stream, which then shows up as loose-list spacing in the
/// terminal. Preserve the provider-authored trailing newline count instead.
fn normalize_stream_rendered_newlines(source: &str, rendered: &str) -> String {
    let desired_trailing_newlines = source.bytes().rev().take_while(|b| *b == b'\n').count();
    let mut kept_lines: Vec<&str> = rendered.split_inclusive('\n').collect();
    while let Some(last) = kept_lines.last() {
        let stripped = biscuit_terminal::prelude::strip_escape_codes(*last);
        let visual = stripped.trim_end_matches('\n').trim();
        if visual.is_empty() {
            kept_lines.pop();
        } else {
            break;
        }
    }

    let joined = kept_lines.concat();
    let trimmed = joined.trim_end_matches('\n');
    if desired_trailing_newlines == 0 && trimmed.len() == joined.len() {
        return joined;
    }

    let mut normalized = trimmed.to_string();
    for _ in 0..desired_trailing_newlines {
        normalized.push('\n');
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::output::terminal::TerminalImageMode;

    /// Concatenate the frames a phase returns into the exact byte stream the
    /// sink would write — the frames-API analogue of the old captured writer.
    fn frames(v: Vec<String>) -> String {
        v.concat()
    }

    fn test_renderer() -> AssistantStream {
        AssistantStream::new(None, None)
    }

    fn markdown_renderer() -> AssistantStream {
        let term = Terminal::new_optimistic(80);
        let mut opts = TerminalOptions::default();
        opts.image_mode = TerminalImageMode::Never;
        AssistantStream::new(Some(term), Some(opts))
    }

    #[test]
    fn stream_text_renderer_flushes_on_blank_line() {
        let mut renderer = test_renderer();

        let flushed = frames(renderer.append("First paragraph.\n\nSecond"));

        assert!(flushed.contains("First paragraph."));
        // Trailing partial "Second" now streams raw immediately because
        // StreamOutput coordination guarantees stderr lines won't interleave.
        assert!(flushed.contains("Second"));
        assert!(renderer.line_buffer.is_empty());
        assert!(renderer.partial_line_committed);
        assert!(renderer.block_buffer.is_empty());
    }

    #[test]
    fn stream_text_renderer_buffers_code_fence() {
        let mut renderer = test_renderer();

        // Opening fence + code — should NOT flush yet
        let opening = frames(renderer.append("```rust\nfn main() {}\n"));
        assert!(opening.is_empty());
        assert!(renderer.in_code_fence);

        // Closing fence — should flush the whole block
        let flushed = frames(renderer.append("```\n"));
        assert!(flushed.contains("fn main()"));
        assert!(!renderer.in_code_fence);
    }

    #[test]
    fn stream_text_renderer_streams_partial_line_immediately() {
        let mut renderer = test_renderer();

        let flushed = frames(renderer.append("trailing text without newline"));
        assert_eq!(flushed, "trailing text without newline");
        assert!(renderer.line_buffer.is_empty());
        assert!(renderer.partial_line_committed);
    }

    #[test]
    fn stream_text_renderer_newline_after_partial_emits_only_newline() {
        // When the partial line was already streamed raw, the arriving
        // newline must not cause the line to be re-rendered — otherwise the
        // user sees the same content twice (once raw, once markdown).
        let mut renderer = test_renderer();

        let mut flushed = frames(renderer.append("Group A: progress.rs"));
        flushed.push_str(&frames(renderer.append("\n")));
        assert_eq!(flushed, "Group A: progress.rs\n");
        assert!(!renderer.partial_line_committed);
    }

    #[test]
    fn stream_text_renderer_flush_remaining_does_not_duplicate_streamed_text() {
        let mut renderer = test_renderer();

        let mut flushed = frames(renderer.append("already streamed"));
        flushed.push_str(&frames(renderer.close()));
        assert_eq!(flushed, "already streamed");
    }

    #[test]
    fn stream_text_renderer_flushes_list_items_immediately() {
        let mut renderer = test_renderer();

        let flushed = frames(renderer.append("1. first item\n2. second item\n"));

        assert_eq!(flushed, "1. first item\n2. second item\n");
        assert!(renderer.block_buffer.is_empty());
        assert!(renderer.line_buffer.is_empty());
    }

    #[test]
    fn markdown_streamed_list_items_do_not_gain_blank_lines() {
        let mut renderer = markdown_renderer();

        let flushed = biscuit_terminal::prelude::strip_escape_codes(frames(renderer.append(
            "- Hash: `f525870d`\n- Package: `claudine`\n- Operation: `feat`\n",
        )));

        assert!(
            !flushed.contains("\n\n"),
            "streamed list items should stay contiguous; got: {flushed:?}"
        );
    }

    #[test]
    fn normalize_stream_rendered_newlines_matches_source_trailing_newlines() {
        assert_eq!(
            normalize_stream_rendered_newlines("- item\n", "- item\n\n"),
            "- item\n"
        );
        assert_eq!(
            normalize_stream_rendered_newlines("paragraph\n\n", "paragraph\n\n\n"),
            "paragraph\n\n"
        );
        assert_eq!(normalize_stream_rendered_newlines("done", "done\n"), "done");
    }

    #[test]
    fn flush_if_idle_emits_block_after_threshold() {
        let mut renderer = test_renderer();

        // Paragraph without trailing blank line — sits in block_buffer.
        let buffered = frames(renderer.append("dangling paragraph line\n"));
        assert!(
            !renderer.block_buffer.is_empty(),
            "content should be buffered until a boundary or idle flush"
        );
        assert!(
            buffered.is_empty(),
            "block buffered text should not be emitted yet"
        );

        // Threshold not reached — flush_idle is a no-op.
        assert!(renderer.flush_idle(Duration::from_secs(60)).is_empty());
        assert!(!renderer.block_buffer.is_empty());

        // After the idle window has elapsed, the buffered block flushes.
        std::thread::sleep(Duration::from_millis(20));
        let flushed = frames(renderer.flush_idle(Duration::from_millis(5)));
        assert!(
            flushed.contains("dangling paragraph line"),
            "expected flushed output to contain the buffered paragraph; got: {flushed:?}"
        );
        assert!(renderer.block_buffer.is_empty());
    }

    #[test]
    fn flush_if_idle_does_not_emit_when_block_empty() {
        let mut renderer = test_renderer();

        // No buffered content — must not flush regardless of threshold.
        assert!(renderer.flush_idle(Duration::from_millis(0)).is_empty());
    }

    #[test]
    fn flush_if_idle_resets_growth_clock() {
        let mut renderer = test_renderer();

        // Accumulate content, wait past threshold, flush.
        renderer.append("first block\n");
        std::thread::sleep(Duration::from_millis(20));
        let first = frames(renderer.flush_idle(Duration::from_millis(5)));
        assert!(!first.is_empty());
        assert!(renderer.block_buffer.is_empty());

        // New content arrives — growth clock restarts. An immediate idle
        // check with a large threshold must not flush the fresh content.
        renderer.append("second block\n");
        assert!(
            renderer.flush_idle(Duration::from_secs(30)).is_empty(),
            "growth clock should restart when new content lands"
        );
        assert!(!renderer.block_buffer.is_empty());

        // After the new block has been idle long enough, it flushes.
        std::thread::sleep(Duration::from_millis(20));
        let second = frames(renderer.flush_idle(Duration::from_millis(5)));
        assert!(!second.is_empty());
        assert!(renderer.block_buffer.is_empty());

        assert!(first.contains("first block"));
        assert!(second.contains("second block"));
    }

    #[test]
    fn flushes_long_prose_on_sentence_terminator() {
        // After the block buffer accumulates substantial prose (past the
        // sentence-flush threshold) and the latest line ends with sentence-
        // terminating punctuation, flush early so the user sees progress
        // without waiting for a blank line.
        let mut renderer = test_renderer();

        let long_sentence = "This is a long sentence the agent is writing as part of an \
            extended paragraph that has not yet reached a blank line boundary and would \
            otherwise sit invisible in the buffer waiting for darkmatter to render it.\n";
        assert!(long_sentence.len() > SENTENCE_FLUSH_MIN_BYTES);

        let flushed = frames(renderer.append(long_sentence));
        assert!(
            flushed.contains("extended paragraph"),
            "long sentence-terminated line should flush early; got: {flushed:?}"
        );
        assert!(renderer.block_buffer.is_empty());
    }

    #[test]
    fn does_not_flush_short_line_on_sentence_terminator() {
        // A short response like "OK." must remain buffered — only buffers
        // past the size threshold are eligible for sentence-level flush.
        let mut renderer = test_renderer();

        let flushed = frames(renderer.append("OK.\n"));
        assert!(flushed.is_empty(), "short line should not trigger sentence flush");
        assert!(!renderer.block_buffer.is_empty());
    }

    #[test]
    fn does_not_sentence_flush_inside_code_fence() {
        // Content inside a fenced block must never trigger sentence-level
        // flush — the renderer waits for the closing fence.
        let mut renderer = test_renderer();

        renderer.append("```\n");
        let long_inside = "This is a really long line inside a code fence that ends with a \
            period and is more than the sentence-flush threshold characters long because \
            we want to verify that fence content is never flushed by the heuristic.\n";
        assert!(long_inside.len() > SENTENCE_FLUSH_MIN_BYTES);
        let flushed = frames(renderer.append(long_inside));

        assert!(
            flushed.is_empty(),
            "fenced content must not sentence-flush; got: {flushed:?}"
        );
        assert!(renderer.in_code_fence);
    }

    #[test]
    fn does_not_sentence_flush_when_line_lacks_terminator() {
        // A long line that does not end in . ! or ? must not flush — only
        // sentence-terminated lines qualify.
        let mut renderer = test_renderer();

        let long_no_terminator = "This is a long line that the agent is writing without \
            ever reaching a terminating period and so it should remain buffered until \
            either a blank line arrives or the idle threshold expires from above and so \
            we keep going for a while longer to comfortably exceed the byte threshold\n";
        assert!(long_no_terminator.len() > SENTENCE_FLUSH_MIN_BYTES);

        let flushed = frames(renderer.append(long_no_terminator));
        assert!(flushed.is_empty(), "non-terminated line must not sentence-flush");
        assert!(!renderer.block_buffer.is_empty());
    }

    #[test]
    fn flush_if_idle_ticker_contract_surfaces_dangling_paragraph() {
        // Contract exercised by `spawn_flush_if_idle_ticker`: when the
        // 30-second ticker fires idle, buffered assistant text must reach
        // stdout so the next stderr status line never appears above stale
        // paragraphs. Simulated directly rather than through the real
        // thread for deterministic timing.
        let renderer: std::sync::Arc<std::sync::Mutex<AssistantStream>> =
            std::sync::Arc::new(std::sync::Mutex::new(test_renderer()));
        let mut stdout_text = String::new();

        // Provider emits a final paragraph without a trailing blank line.
        {
            let mut r = renderer.lock().unwrap();
            stdout_text.push_str(&frames(r.append("final summary line\n")));
        }
        assert!(
            stdout_text.is_empty(),
            "buffered text must not escape before the idle window elapses"
        );

        std::thread::sleep(Duration::from_millis(15));
        let flushed = {
            let mut r = renderer.lock().unwrap();
            frames(r.flush_idle(Duration::from_millis(5)))
        };
        assert!(!flushed.is_empty(), "idle flush should have fired");
        assert!(flushed.contains("final summary line"));
    }
}
