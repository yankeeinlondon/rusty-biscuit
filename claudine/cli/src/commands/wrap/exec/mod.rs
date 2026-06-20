use std::io::{IsTerminal, Write};
use std::path::Path;
use std::process::Child;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use biscuit_terminal::terminal::Terminal;
use claudine::stream::parser::{SemanticStreamParser, StreamParseError};
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;

pub(crate) mod exit;
pub(crate) mod spawn;
pub(crate) mod stream_capture;
pub(crate) mod subagent_watchdog;
pub(crate) mod termination;
pub(crate) mod timeouts;
pub(crate) mod watchdog;
pub(crate) mod wiring;

pub(crate) use exit::{cleanup_mcp_injection, exit_code_from_status, resolve_first_response};
pub(crate) use spawn::{run_child, run_child_capture, run_child_stream_semantic};

/// Switch the wrapper process cwd to the child's cwd and sync `PWD`.
///
/// Rust's `set_current_dir` calls `chdir(2)` but does NOT touch the
/// `PWD` environment variable — the shell convention is that `PWD`
/// tracks "where the user thinks they are", which can differ from
/// `getcwd(3)`. Several downstream tools (notably OpenCode's
/// `run.ts:276` resolving `process.env.PWD ?? process.cwd()`) trust
/// `PWD` over the real cwd. If we don't sync them, the spawned
/// child inherits the user's pre-chdir `PWD` (e.g. a package
/// subdirectory the user ran `just commit` from) and resolves paths
/// against the wrong root.
///
/// # Safety
///
/// Single-threaded wrapper startup; no other thread reads or writes
/// `PWD` concurrently with this call.
pub(crate) fn switch_process_cwd(child_cwd: &Path) -> Result<()> {
    let current = std::env::current_dir()?;
    if current != child_cwd {
        std::env::set_current_dir(child_cwd)?;
    }
    unsafe {
        std::env::set_var("PWD", child_cwd.as_os_str());
    }
    Ok(())
}

pub(crate) struct ChildIoOptions<'a> {
    pub(crate) stdout_noise_prefixes: &'a [&'a str],
    pub(crate) stderr_noise_prefixes: &'a [&'a str],
    pub(crate) stdin_seed: Option<&'a str>,
}

/// Execution telemetry collected for a single child process.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ProcessTelemetry {
    pub total_elapsed: Duration,
    pub first_response_latency: Option<Duration>,
}

impl ProcessTelemetry {
    /// Convert telemetry into the shared [`AgentExecutionPerf`] model.
    pub(crate) fn into_agent_perf(
        self,
        api_duration_ms: Option<u64>,
    ) -> crate::perf::AgentExecutionPerf {
        crate::perf::AgentExecutionPerf {
            launches: 1,
            total_elapsed: self.total_elapsed,
            first_response_latency: self.first_response_latency,
            provider_api_duration: api_duration_ms.map(Duration::from_millis),
        }
    }
}

/// Result of a child process execution, enriched with termination info.
pub(crate) struct ProcessResult<T> {
    pub(crate) data: T,
    pub(crate) termination: claudine::harness::ProcessTermination,
    pub(crate) telemetry: ProcessTelemetry,
    /// Immediate child PID returned by `std::process::Command::spawn()`,
    /// captured immediately after a successful spawn. `None` only appears
    /// in fabricated results (e.g. parse-failure fallbacks) — a real
    /// spawn either produces `Some(child.id())` here or returns `Err`
    /// before `ProcessResult` is constructed.
    ///
    /// Per-attempt by construction: every call to a spawn function
    /// returns a fresh `ProcessResult`, so harness retries and
    /// composition iterations never inherit a stale PID from a prior
    /// attempt.
    //
    // Read in Phase 3 by the dispatch / stream-summary / reporting
    // surfaces; flagged here so the Phase 2 capture-only change does
    // not trip `-D warnings`.
    #[allow(dead_code)]
    pub(crate) agent_pid: Option<u32>,
    /// Structured runaway-guard context (Phase 6), populated when the run
    /// ended on a content-guard trip (exit-expression / repetition / volume).
    /// `None` for ordinary completions, timeouts, and rate-limit aborts.
    /// Carried so the attempt outcome can thread `error_kind` + guard detail
    /// into the failure-handler payload (C3a).
    pub(crate) guard_context: Option<claudine::harness::GuardContext>,
}

/// Renders streamed assistant text as Markdown, flushing at block boundaries.
///
/// Accumulates incoming text and detects Markdown block boundaries (blank lines,
/// code fence closings) to render complete blocks through darkmatter for rich
/// terminal output (syntax highlighting, tables, bold/italic, etc.).
pub(crate) struct StreamTextRenderer {
    /// Accumulated text for the current Markdown block.
    block_buffer: String,
    /// Trailing text without a newline (incomplete line).
    line_buffer: String,
    /// Whether we are inside a fenced code block (``` or ~~~).
    in_code_fence: bool,
    /// True when the partial line in `line_buffer` has already been written
    /// to stdout raw. When the newline eventually arrives we only emit `\n`
    /// instead of re-rendering through darkmatter, avoiding duplicate
    /// output. Safe to enable now that all stderr status lines route through
    /// `StreamOutput`, which guarantees a newline-boundary before writing.
    partial_line_committed: bool,
    /// Timestamp of the last write into `block_buffer`. Used by
    /// [`flush_if_idle`] so the heartbeat thread can surface buffered
    /// assistant text when the provider stalls without emitting a paragraph
    /// boundary. Reset to `None` whenever the block flushes.
    last_block_growth_at: Option<Instant>,
    /// Terminal reference for rendering.
    term: Option<Terminal>,
    /// Cached darkmatter options (created once to avoid repeated theme detection).
    terminal_options: Option<darkmatter::markdown::output::terminal::TerminalOptions>,
}

impl StreamTextRenderer {
    fn new() -> Self {
        let term = std::io::stdout().is_terminal().then(crate::log::terminal);
        let terminal_options = term.as_ref().map(|_| {
            use darkmatter::markdown::output::terminal::{TerminalImageMode, TerminalOptions};
            let mut opts = TerminalOptions::default();
            opts.image_mode = TerminalImageMode::Never;
            opts
        });
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

    fn push<W: Write>(&mut self, out: &mut W, text: &str) {
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
                let _ = out.write_all(b"\n");
                let _ = out.flush();
                self.partial_line_committed = false;
                continue;
            }

            self.process_line(out, &line);
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
            let _ = out.write_all(partial.as_bytes());
            let _ = out.flush();
            self.partial_line_committed = true;
        }
    }

    /// Process a single complete line, accumulating into the block buffer
    /// and flushing when a block boundary is detected.
    fn process_line<W: Write>(&mut self, out: &mut W, line: &str) {
        let trimmed = line.trim();

        // Track code fence open/close (``` or ~~~)
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            self.append_block(line);
            if self.in_code_fence {
                // Closing fence — render the complete fenced block
                self.in_code_fence = false;
                self.flush_block(out);
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
            self.flush_block(out);
            return;
        }

        // Ordered/unordered list items are complete enough to stream line-by-line.
        // Waiting for a blank line or EOF can hide useful progress for minutes if
        // the provider stalls after emitting the last list item.
        if is_stream_safe_list_item(trimmed) {
            self.flush_block(out);
            self.append_block(line);
            self.flush_block(out);
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
            self.flush_block(out);
        }
    }

    /// Append `content` to the block buffer and stamp the growth clock so
    /// [`flush_if_idle`] can tell how long the buffer has been sitting idle.
    fn append_block(&mut self, content: &str) {
        self.block_buffer.push_str(content);
        self.last_block_growth_at = Some(Instant::now());
    }

    /// Render the accumulated block through darkmatter and write to output.
    fn flush_block<W: Write>(&mut self, out: &mut W) {
        if self.block_buffer.is_empty() {
            return;
        }
        let block = std::mem::take(&mut self.block_buffer);
        self.last_block_growth_at = None;
        self.render_markdown(out, &block);
    }

    /// Flush buffered markdown if it has been sitting idle for at least
    /// `idle_threshold`. Returns `true` when something was flushed.
    ///
    /// Called by the heartbeat thread before it emits its own status line so
    /// dangling paragraphs cannot remain invisible while the provider stalls
    /// without closing stdout. An empty buffer is a no-op; a fresh write
    /// inside the threshold is a no-op.
    fn flush_if_idle<W: Write>(&mut self, out: &mut W, idle_threshold: Duration) -> bool {
        if self.block_buffer.is_empty() {
            return false;
        }
        let Some(stamped_at) = self.last_block_growth_at else {
            return false;
        };
        if stamped_at.elapsed() < idle_threshold {
            return false;
        }
        self.flush_block(out);
        true
    }

    /// Flush any remaining buffered content (incomplete line + block buffer).
    fn flush_remaining<W: Write>(&mut self, out: &mut W) {
        if !self.line_buffer.is_empty() {
            let leftover = std::mem::take(&mut self.line_buffer);
            if self.partial_line_committed {
                // Already streamed raw — do not re-render through darkmatter.
                self.partial_line_committed = false;
            } else {
                self.append_block(&leftover);
            }
        }
        self.flush_block(out);
    }

    fn render_markdown<W: Write>(&self, out: &mut W, text: &str) {
        if let Some(term) = &self.term {
            let rendered = crate::output::render_assistant_markdown_with_options(
                text,
                term,
                self.terminal_options.as_ref(),
            );
            let normalized = normalize_stream_rendered_newlines(text, &rendered);
            let _ = out.write_all(normalized.as_bytes());
        } else {
            let _ = out.write_all(text.as_bytes());
        }
        let _ = out.flush();
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
fn line_ends_sentence(trimmed: &str) -> bool {
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

/// Callback type used by [`run_child_stream_semantic`] for assistant text.
pub(crate) type OutputTextCallback = Box<dyn FnMut(&str) + Send + 'static>;

/// Callback type used by [`run_child_stream_semantic`] for reasoning text.
pub(crate) type ReasoningCallback = Box<dyn FnMut(&str) + Send + 'static>;

/// Factory signature used by [`run_child_stream_semantic`] to construct the
/// parser inside the stdout reader thread.
///
/// The caller receives two callbacks: one for stdout markdown
/// ([`SemanticEvent::OutputText`]) and one for reasoning text
/// ([`SemanticEvent::Reasoning`]). The reasoning callback is currently a
/// no-op in the structured-stream path because `LiveSemanticSink` renders
/// reasoning directly through its section-aware stderr emitter. The second
/// parameter is retained for signature compatibility.
///
/// The trailing `Option<u32>` is the immediate child PID captured after a
/// successful spawn (invoked from the reader thread, so the PID is already
/// known). Builders stamp it onto their sink so live records carry
/// `EventMeta.agent_pid`.
///
/// [`SemanticEvent::OutputText`]: claudine::stream::semantic::SemanticEvent::OutputText
/// [`SemanticEvent::Reasoning`]: claudine::stream::semantic::SemanticEvent::Reasoning
/// [`LiveSemanticSink`]: super::live_semantic_sink::LiveSemanticSink
pub(crate) type SemanticParserBuilder = Box<
    dyn FnOnce(OutputTextCallback, ReasoningCallback, Option<u32>) -> Box<dyn SemanticStreamParser>
        + Send
        + 'static,
>;

/// Minimal fallback parser used when the real parser thread panics.
struct ErrorParser {
    exit_code: i32,
}

impl SemanticStreamParser for ErrorParser {
    fn feed_line(&mut self, _line: &str) -> std::result::Result<(), StreamParseError> {
        Ok(())
    }

    fn finish(self: Box<Self>, _exit_code: i32) -> StreamExecutionSummary {
        StreamExecutionSummary {
            is_error: true,
            error_kind: Some("parse_failure".into()),
            error_message: Some("Stream parser thread panicked".into()),
            exit_code: self.exit_code,
            ..Default::default()
        }
    }
}

/// Join a thread with a timeout. Returns `true` if the thread joined
/// successfully within the deadline, `false` if it timed out.
///
/// On timeout the thread is **leaked** (detached) rather than panicked,
/// because the reader threads only terminate when their pipe closes and
/// there is no safe way to interrupt a blocking `BufReader::lines()` call
/// from outside.
fn join_with_timeout(handle: thread::JoinHandle<()>, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        // `is_finished()` is available on Rust 1.69+ and does not block.
        if handle.is_finished() {
            let _ = handle.join();
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    tracing::warn!(
        "reader thread did not exit within {:?}; detaching (pipe may still be held open by a descendant process)",
        timeout
    );
    std::mem::forget(handle);
    false
}

/// Join a thread that returns a value, with a timeout. Returns the value
/// on success or a fallback on timeout.
fn join_with_timeout_or<T>(handle: thread::JoinHandle<T>, timeout: Duration, fallback: T) -> T {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if handle.is_finished() {
            return handle.join().unwrap_or(fallback);
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    tracing::warn!(
        "reader thread did not exit within {:?}; using fallback result",
        timeout
    );
    std::mem::forget(handle);
    fallback
}

/// After the main child exits, kill any orphaned descendant processes so
/// inherited pipe fds are closed and reader threads unblock. Without this,
/// a subagent spawned by the child (e.g. OpenCode Task tool) that inherits
/// stdout/stderr can keep the pipe open indefinitely, causing the reader
/// threads to hang on `BufReader::lines()`.
#[cfg(unix)]
fn kill_process_group(child: &mut Child) {
    let pid = child.id() as i32;
    // Derive the grace period from the same `TimeoutConfig` knob that
    // governs SIGTERM->SIGKILL escalation in the streaming wait loop,
    // so the two termination paths stay consistent.
    let kill_grace = timeouts::TimeoutConfig::resolve(None, None).kill_grace;
    // Send SIGTERM to the process group first (graceful), then SIGKILL.
    unsafe {
        // kill(-pgid, ...) sends to the entire process group.
        // With process_group(0), the pgid == child pid.
        if libc::kill(-pid, libc::SIGTERM) == 0 {
            // Give descendants the configured grace period to exit.
            std::thread::sleep(kill_grace);
            // Ensure everything is dead.
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill_process_group(_child: &mut Child) {}

/// Cooperative cancellation with an interruptible sleep for the wrap
/// ticker threads.
///
/// A bare `AtomicBool` polled between fixed `thread::sleep` calls leaves
/// teardown waiting out the in-flight sleep — up to ~1 s per ticker, paid
/// on every non-interactive run at process exit. Backing the flag with a
/// condvar lets [`TickerCancel::cancel`] wake a sleeping ticker
/// immediately, so [`stop_timing_ticker`]'s `join()` returns at once.
#[derive(Clone)]
pub(crate) struct TickerCancel {
    inner: Arc<(std::sync::Mutex<bool>, std::sync::Condvar)>,
}

impl TickerCancel {
    pub(crate) fn new() -> Self {
        Self {
            inner: Arc::new((std::sync::Mutex::new(false), std::sync::Condvar::new())),
        }
    }

    /// Request cancellation and wake any thread parked in [`Self::sleep`].
    pub(crate) fn cancel(&self) {
        let (lock, cvar) = &*self.inner;
        *lock.lock().unwrap() = true;
        cvar.notify_all();
    }

    /// `true` once [`Self::cancel`] has been called.
    pub(crate) fn is_cancelled(&self) -> bool {
        *self.inner.0.lock().unwrap()
    }

    /// Sleep up to `dur`, returning the instant `cancel` is called.
    ///
    /// ## Returns
    ///
    /// `true` if cancellation was observed (so the caller should stop), or
    /// `false` if the full `dur` elapsed without cancellation.
    pub(crate) fn sleep(&self, dur: Duration) -> bool {
        let (lock, cvar) = &*self.inner;
        let guard = lock.lock().unwrap();
        if *guard {
            return true;
        }
        let (guard, _timed_out) = cvar.wait_timeout(guard, dur).unwrap();
        *guard
    }
}

/// Signal a timing ticker thread to stop and join it.
///
/// Shared by the flush-if-idle ticker and the prompt-timing monitor —
/// both return the same `(cancel, handle)` pair and need identical
/// teardown. `None` is accepted so callers can pass through optional
/// handles without an extra match.
fn stop_timing_ticker(ticker: Option<(TickerCancel, thread::JoinHandle<()>)>) {
    if let Some((cancel, handle)) = ticker {
        cancel.cancel();
        let _ = handle.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use darkmatter::markdown::output::terminal::{TerminalImageMode, TerminalOptions};

    fn test_renderer() -> StreamTextRenderer {
        StreamTextRenderer {
            block_buffer: String::new(),
            line_buffer: String::new(),
            in_code_fence: false,
            partial_line_committed: false,
            last_block_growth_at: None,
            term: None,
            terminal_options: None,
        }
    }

    fn markdown_renderer() -> StreamTextRenderer {
        let term = Terminal::new_optimistic(80);
        let mut opts = TerminalOptions::default();
        opts.image_mode = TerminalImageMode::Never;
        StreamTextRenderer {
            block_buffer: String::new(),
            line_buffer: String::new(),
            in_code_fence: false,
            partial_line_committed: false,
            last_block_growth_at: None,
            term: Some(term),
            terminal_options: Some(opts),
        }
    }

    #[test]
    fn stream_text_renderer_flushes_on_blank_line() {
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        renderer.push(&mut out, "First paragraph.\n\nSecond");
        let flushed = String::from_utf8(out).unwrap();

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
        let mut out = Vec::new();

        // Opening fence + code — should NOT flush yet
        renderer.push(&mut out, "```rust\nfn main() {}\n");
        assert!(out.is_empty());
        assert!(renderer.in_code_fence);

        // Closing fence — should flush the whole block
        renderer.push(&mut out, "```\n");
        let flushed = String::from_utf8(out).unwrap();
        assert!(flushed.contains("fn main()"));
        assert!(!renderer.in_code_fence);
    }

    #[test]
    fn stream_text_renderer_streams_partial_line_immediately() {
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        renderer.push(&mut out, "trailing text without newline");
        let flushed = String::from_utf8(out).unwrap();
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
        let mut out = Vec::new();

        renderer.push(&mut out, "Group A: progress.rs");
        renderer.push(&mut out, "\n");
        let flushed = String::from_utf8(out).unwrap();
        assert_eq!(flushed, "Group A: progress.rs\n");
        assert!(!renderer.partial_line_committed);
    }

    #[test]
    fn stream_text_renderer_flush_remaining_does_not_duplicate_streamed_text() {
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        renderer.push(&mut out, "already streamed");
        renderer.flush_remaining(&mut out);
        let flushed = String::from_utf8(out).unwrap();
        assert_eq!(flushed, "already streamed");
    }

    #[test]
    fn stream_text_renderer_flushes_list_items_immediately() {
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        renderer.push(&mut out, "1. first item\n2. second item\n");
        let flushed = String::from_utf8(out).unwrap();

        assert_eq!(flushed, "1. first item\n2. second item\n");
        assert!(renderer.block_buffer.is_empty());
        assert!(renderer.line_buffer.is_empty());
    }

    #[test]
    fn markdown_streamed_list_items_do_not_gain_blank_lines() {
        let mut renderer = markdown_renderer();
        let mut out = Vec::new();

        renderer.push(
            &mut out,
            "- Hash: `f525870d`\n- Package: `claudine`\n- Operation: `feat`\n",
        );
        let flushed =
            biscuit_terminal::prelude::strip_escape_codes(String::from_utf8(out).expect("utf8"));

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
        let mut out = Vec::new();

        // Paragraph without trailing blank line — sits in block_buffer.
        renderer.push(&mut out, "dangling paragraph line\n");
        assert!(
            !renderer.block_buffer.is_empty(),
            "content should be buffered until a boundary or idle flush"
        );
        assert!(
            out.is_empty(),
            "block buffered text should not be emitted yet"
        );

        // Threshold not reached — flush_if_idle is a no-op.
        assert!(!renderer.flush_if_idle(&mut out, Duration::from_secs(60)));
        assert!(out.is_empty());
        assert!(!renderer.block_buffer.is_empty());

        // After the idle window has elapsed, the buffered block flushes.
        std::thread::sleep(Duration::from_millis(20));
        assert!(renderer.flush_if_idle(&mut out, Duration::from_millis(5)));

        let flushed = String::from_utf8(out).unwrap();
        assert!(
            flushed.contains("dangling paragraph line"),
            "expected flushed output to contain the buffered paragraph; got: {flushed:?}"
        );
        assert!(renderer.block_buffer.is_empty());
    }

    #[test]
    fn flush_if_idle_does_not_emit_when_block_empty() {
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        // No buffered content — must not flush regardless of threshold.
        assert!(!renderer.flush_if_idle(&mut out, Duration::from_millis(0)));
        assert!(out.is_empty());
    }

    #[test]
    fn flush_if_idle_resets_growth_clock() {
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        // Accumulate content, wait past threshold, flush.
        renderer.push(&mut out, "first block\n");
        std::thread::sleep(Duration::from_millis(20));
        assert!(renderer.flush_if_idle(&mut out, Duration::from_millis(5)));
        assert!(renderer.block_buffer.is_empty());

        // New content arrives — growth clock restarts. An immediate idle
        // check with a large threshold must not flush the fresh content.
        renderer.push(&mut out, "second block\n");
        assert!(
            !renderer.flush_if_idle(&mut out, Duration::from_secs(30)),
            "growth clock should restart when new content lands"
        );
        assert!(!renderer.block_buffer.is_empty());

        // After the new block has been idle long enough, it flushes.
        std::thread::sleep(Duration::from_millis(20));
        assert!(renderer.flush_if_idle(&mut out, Duration::from_millis(5)));
        assert!(renderer.block_buffer.is_empty());

        let flushed = String::from_utf8(out).unwrap();
        assert!(flushed.contains("first block"));
        assert!(flushed.contains("second block"));
    }

    #[test]
    fn flushes_long_prose_on_sentence_terminator() {
        // After the block buffer accumulates substantial prose (past the
        // sentence-flush threshold) and the latest line ends with sentence-
        // terminating punctuation, flush early so the user sees progress
        // without waiting for a blank line.
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        let long_sentence = "This is a long sentence the agent is writing as part of an \
            extended paragraph that has not yet reached a blank line boundary and would \
            otherwise sit invisible in the buffer waiting for darkmatter to render it.\n";
        assert!(long_sentence.len() > SENTENCE_FLUSH_MIN_BYTES);

        renderer.push(&mut out, long_sentence);
        let flushed = String::from_utf8(out).unwrap();
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
        let mut out = Vec::new();

        renderer.push(&mut out, "OK.\n");
        assert!(
            out.is_empty(),
            "short line should not trigger sentence flush"
        );
        assert!(!renderer.block_buffer.is_empty());
    }

    #[test]
    fn does_not_sentence_flush_inside_code_fence() {
        // Content inside a fenced block must never trigger sentence-level
        // flush — the renderer waits for the closing fence.
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        renderer.push(&mut out, "```\n");
        let long_inside = "This is a really long line inside a code fence that ends with a \
            period and is more than the sentence-flush threshold characters long because \
            we want to verify that fence content is never flushed by the heuristic.\n";
        assert!(long_inside.len() > SENTENCE_FLUSH_MIN_BYTES);
        renderer.push(&mut out, long_inside);

        assert!(
            out.is_empty(),
            "fenced content must not sentence-flush; got: {:?}",
            String::from_utf8(out.clone()).unwrap()
        );
        assert!(renderer.in_code_fence);
    }

    #[test]
    fn does_not_sentence_flush_when_line_lacks_terminator() {
        // A long line that does not end in . ! or ? must not flush — only
        // sentence-terminated lines qualify.
        let mut renderer = test_renderer();
        let mut out = Vec::new();

        let long_no_terminator = "This is a long line that the agent is writing without \
            ever reaching a terminating period and so it should remain buffered until \
            either a blank line arrives or the idle threshold expires from above and so \
            we keep going for a while longer to comfortably exceed the byte threshold\n";
        assert!(long_no_terminator.len() > SENTENCE_FLUSH_MIN_BYTES);

        renderer.push(&mut out, long_no_terminator);
        assert!(
            out.is_empty(),
            "non-terminated line must not sentence-flush"
        );
        assert!(!renderer.block_buffer.is_empty());
    }

    #[test]
    fn flush_if_idle_ticker_contract_surfaces_dangling_paragraph() {
        // Contract exercised by `spawn_flush_if_idle_ticker`: when the
        // 30-second ticker fires idle, buffered assistant text must reach
        // stdout so the next stderr status line never appears above stale
        // paragraphs. Simulated directly rather than through the real
        // thread for deterministic timing.
        let renderer: Arc<std::sync::Mutex<StreamTextRenderer>> =
            Arc::new(std::sync::Mutex::new(test_renderer()));
        let mut stdout_bytes: Vec<u8> = Vec::new();

        // Provider emits a final paragraph without a trailing blank line.
        {
            let mut r = renderer.lock().unwrap();
            r.push(&mut stdout_bytes, "final summary line\n");
        }
        assert!(
            stdout_bytes.is_empty(),
            "buffered text must not escape before the idle window elapses"
        );

        std::thread::sleep(Duration::from_millis(15));
        let flushed = {
            let mut r = renderer.lock().unwrap();
            r.flush_if_idle(&mut stdout_bytes, Duration::from_millis(5))
        };
        assert!(flushed, "idle flush should have fired");

        let stdout_text = String::from_utf8(stdout_bytes).unwrap();
        assert!(stdout_text.contains("final summary line"));
    }

    #[test]
    fn process_telemetry_into_agent_perf_populates_all_fields() {
        let telemetry = ProcessTelemetry {
            total_elapsed: Duration::from_secs(3),
            first_response_latency: Some(Duration::from_millis(500)),
        };
        let perf = telemetry.into_agent_perf(Some(1200));
        assert_eq!(perf.launches, 1);
        assert_eq!(perf.total_elapsed, Duration::from_secs(3));
        assert_eq!(
            perf.first_response_latency,
            Some(Duration::from_millis(500))
        );
        assert_eq!(
            perf.provider_api_duration,
            Some(Duration::from_millis(1200))
        );
    }

    #[test]
    fn process_telemetry_into_agent_perf_omits_api_when_none() {
        let telemetry = ProcessTelemetry {
            total_elapsed: Duration::from_secs(1),
            first_response_latency: None,
        };
        let perf = telemetry.into_agent_perf(None);
        assert_eq!(perf.launches, 1);
        assert_eq!(perf.provider_api_duration, None);
        assert_eq!(perf.first_response_latency, None);
    }
}
