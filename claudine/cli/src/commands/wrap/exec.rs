use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::components::status::{Status, StatusState};
use biscuit_terminal::terminal::Terminal;
use claudine::stream::logs::{EarlyTermination, StderrBridgeHandle, StderrIngestOutcome};
use claudine::stream::parser::{SemanticStreamParser, StreamParseError};
use claudine::stream::progress::{self, LiveMetrics};
use claudine::stream::prompt_timing as prompt_timing_mod;
use claudine::stream::prompt_timing::{HeaderKind, PromptTimingContext};
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;
use std::sync::mpsc::{Receiver, TryRecvError};
use tracing::{Span, info_span};

use super::stream_io::StreamOutput;

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

#[allow(dead_code)]
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
    #[allow(dead_code)]
    pub(crate) telemetry: ProcessTelemetry,
}

/// Renders streamed assistant text as Markdown, flushing at block boundaries.
///
/// Accumulates incoming text and detects Markdown block boundaries (blank lines,
/// code fence closings) to render complete blocks through darkmatter for rich
/// terminal output (syntax highlighting, tables, bold/italic, etc.).
struct StreamTextRenderer {
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

/// Spawn the provider child process and return its exit code.
///
/// ## Environment
///
/// The `env` parameter must be the **complete** environment for the child
/// process. The child is launched with `env_clear()` followed by `envs(env)`,
/// so any variable not present in `env` will be absent from the child. This
/// is the only gate for environment sanitization — if a variable is missing
/// from `env`, it will not reach the child.
///
/// ## Signal Handling
///
/// If Claudine receives a second SIGINT (Ctrl-C) while waiting for the child,
/// it sends SIGTERM to the child. A third SIGINT sends SIGKILL.
///
/// ## Timeout
///
/// When `timeout` is `Some(seconds)`, the child is sent SIGTERM after the
/// specified duration, followed by SIGKILL after a 5-second grace period.
///
/// ## Stdout Filtering
///
/// When `stdout_noise_prefixes` is non-empty, stdout is piped through a
/// filter that suppresses lines starting with any of the given prefixes.
/// This is used in non-interactive mode to strip provider debug noise
/// (e.g. Gemini CLI's hook execution logs) from the response.
pub(crate) fn run_child(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    io: ChildIoOptions<'_>,
    child_spawned: &mut bool,
) -> Result<ProcessResult<i32>> {
    // Debug assertion: critical variables must be present.
    debug_assert!(
        env.contains_key(&OsString::from("PATH")),
        "child env is missing PATH — env::build_child_env likely has a bug"
    );
    debug_assert!(
        env.contains_key(&OsString::from("HOME")),
        "child env is missing HOME — env::build_child_env likely has a bug"
    );

    let filter_stdout = !io.stdout_noise_prefixes.is_empty();
    let filter_stderr = !io.stderr_noise_prefixes.is_empty();

    let needs_stdin_pipe = io.stdin_seed.is_some();

    // Whether we isolate the child into its own process group. Needed only
    // when we pipe streams (so we can clean up orphaned descendants that
    // keep the pipe fds open — see `kill_process_group`). For pure TTY
    // inheritance (interactive TUIs like Claude/Codex), isolating into a
    // background pgroup causes the child to receive SIGTTIN on stdin read
    // and hang indefinitely.
    let isolate_process_group = filter_stdout || filter_stderr || needs_stdin_pipe;

    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(if needs_stdin_pipe {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stdout(if filter_stdout {
            Stdio::piped()
        } else {
            Stdio::inherit()
        })
        .stderr(if filter_stderr {
            Stdio::piped()
        } else {
            Stdio::inherit()
        });

    #[cfg(unix)]
    if isolate_process_group {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let spawned_at = Instant::now();
    let mut child = command.spawn()?;
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(child.id()));

    // Shared first-response trackers. Each channel stamps the first
    // non-filtered line it sees so we can compute best-effort latency
    // even on the legacy passthrough path.
    let first_stdout_at: Option<Arc<std::sync::Mutex<Option<Instant>>>> = if filter_stdout {
        Some(Arc::new(std::sync::Mutex::new(None)))
    } else {
        None
    };
    let first_stderr_at: Option<Arc<std::sync::Mutex<Option<Instant>>>> = if filter_stderr {
        Some(Arc::new(std::sync::Mutex::new(None)))
    } else {
        None
    };

    // Spawn stdout/stderr reader threads BEFORE writing stdin to avoid a
    // pipe deadlock: if the prompt exceeds the OS pipe buffer (~64 KB on
    // macOS) and the child writes to stdout/stderr during startup, both
    // processes block on pipe I/O with no reader on the other end.
    let stdout_handle = if filter_stdout {
        let pipe = child.stdout.take().expect(
            "child stdout must be piped: Stdio::piped() was set on the child Command above",
        );
        let prefixes: Vec<String> = io
            .stdout_noise_prefixes
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plain = crate::log::is_plain();
        let first_at = first_stdout_at.clone().expect("set when filter_stdout");
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut out = std::io::stdout().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
                }
                {
                    let mut g = first_at.lock().unwrap();
                    if g.is_none() {
                        *g = Some(Instant::now());
                    }
                }
                let stripped = if plain {
                    biscuit_terminal::prelude::strip_escape_codes(&line)
                } else {
                    line
                };
                let _ = writeln!(out, "{stripped}");
            }
        }))
    } else {
        None
    };

    let stderr_handle = if filter_stderr {
        let pipe = child.stderr.take().expect(
            "child stderr must be piped: Stdio::piped() was set on the child Command above",
        );
        let prefixes: Vec<String> = io
            .stderr_noise_prefixes
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plain = crate::log::is_plain();
        let first_at = first_stderr_at.clone().expect("set when filter_stderr");
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut err = std::io::stderr().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
                }
                {
                    let mut g = first_at.lock().unwrap();
                    if g.is_none() {
                        *g = Some(Instant::now());
                    }
                }
                let stripped = if plain {
                    biscuit_terminal::prelude::strip_escape_codes(&line)
                } else {
                    line
                };
                let _ = writeln!(err, "{stripped}");
            }
        }))
    } else {
        None
    };

    // Write stdin seed AFTER reader threads are spawned (see deadlock note above).
    if let Some(seed) = io.stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        stdin_pipe.write_all(seed.as_bytes())?;
        // Drop closes the pipe so the child sees EOF.
    }

    let (exit_code, termination) = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child, isolate_process_group)?
    };

    if isolate_process_group {
        kill_process_group(&mut child);
    }

    let thread_join_timeout = Duration::from_secs(5);
    if let Some(handle) = stdout_handle {
        join_with_timeout(handle, thread_join_timeout);
    }
    if let Some(handle) = stderr_handle {
        join_with_timeout(handle, thread_join_timeout);
    }

    let total_elapsed = spawned_at.elapsed();
    let first_response = resolve_first_response(
        None,
        first_stdout_at.as_ref().and_then(|a| *a.lock().unwrap()),
        first_stderr_at.as_ref().and_then(|a| *a.lock().unwrap()),
        spawned_at,
    );

    Ok(ProcessResult {
        data: exit_code,
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed,
            first_response_latency: first_response,
        },
    })
}

/// After the main child exits, kill any orphaned descendant processes so
/// inherited pipe fds are closed and reader threads unblock. Without this,
/// a subagent spawned by the child (e.g. OpenCode Task tool) that inherits
/// stdout/stderr can keep the pipe open indefinitely, causing the reader
/// threads to hang on `BufReader::lines()`.
#[cfg(unix)]
fn kill_process_group(child: &mut Child) {
    let pid = child.id() as i32;
    // Send SIGTERM to the process group first (graceful), then SIGKILL.
    unsafe {
        // kill(-pgid, ...) sends to the entire process group.
        // With process_group(0), the pgid == child pid.
        if libc::kill(-pid, libc::SIGTERM) == 0 {
            // Give descendants a brief grace period to exit.
            std::thread::sleep(Duration::from_millis(200));
            // Ensure everything is dead.
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

#[cfg(not(unix))]
fn kill_process_group(_child: &mut Child) {}

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

/// Wait for the child, forwarding SIGINT/SIGTERM on repeated Ctrl-C.
///
/// When `child_in_own_pgroup` is true, the child was spawned with
/// `process_group(0)` and the installed SIGINT handler manually forwards
/// signals to `-child_pid` so descendants also receive them. When it is
/// false, the child shares the parent's process group (required for
/// interactive TUIs that read the controlling TTY); in that case the
/// terminal already delivers SIGINT to the child naturally, so we only
/// track the interrupt count locally.
///
/// Returns `(exit_code, termination_kind)`.
#[cfg(unix)]
fn wait_with_signal_handling(
    child: &mut Child,
    child_in_own_pgroup: bool,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};

    let interrupt_count = Arc::new(AtomicU8::new(0));
    let child_exited = Arc::new(AtomicBool::new(false));
    let child_pid = child.id();

    let counter = Arc::clone(&interrupt_count);
    let exited = Arc::clone(&child_exited);
    let _guard = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            // Never signal a PID we no longer own — the kernel may have
            // recycled `child_pid` onto an unrelated process by now.
            if exited.load(Ordering::SeqCst) {
                return;
            }
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if !child_in_own_pgroup {
                // Child shares our process group; the terminal already
                // delivered SIGINT to it. Just track the count so the
                // termination kind is reported correctly.
                return;
            }
            match count {
                1 => {
                    libc::kill(-(child_pid as i32), libc::SIGINT);
                }
                2 => {
                    libc::kill(-(child_pid as i32), libc::SIGTERM);
                }
                _ => {
                    libc::kill(-(child_pid as i32), libc::SIGKILL);
                }
            }
        })
    }?;

    let status = child.wait()?;
    // Set the exit flag BEFORE the `_guard` drops so a SIGINT that arrives
    // in the narrow window between `wait` returning and the guard being
    // dropped still sees an exited child and refuses to signal the PID.
    child_exited.store(true, Ordering::SeqCst);
    let code = exit_code_from_status(status);
    let was_interrupted = interrupt_count.load(Ordering::SeqCst) > 0;
    let termination = if was_interrupted {
        claudine::harness::ProcessTermination::Interrupted
    } else {
        claudine::harness::ProcessTermination::Completed
    };
    Ok((code, termination))
}

#[cfg(not(unix))]
fn wait_with_signal_handling(
    child: &mut Child,
    _child_in_own_pgroup: bool,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    let status = child.wait()?;
    Ok((
        exit_code_from_status(status),
        claudine::harness::ProcessTermination::Completed,
    ))
}

/// Polling wait loop used when an [`EarlyTermination`] receiver is attached
/// to the structured stream executor.
///
/// Behaves like [`wait_with_signal_handling`] while also polling the
/// stderr-bridge channel. When a signal arrives, the child's process group
/// is sent `SIGTERM` and escalated to `SIGKILL` after a 5-second grace
/// period. User Ctrl-C still reports `Interrupted`; wrapper-driven early
/// termination (rate-limit recovery) preserves a
/// normal `Completed` termination so downstream failure handling can inspect
/// synthesized summary fields instead of treating the run like a user cancel.
///
/// Isolated to the bridge path so non-OpenCode runs keep the existing
/// `child.wait()`-based helper.
#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn wait_with_signal_and_early_termination(
    child: &mut Child,
    child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    live_metrics: Option<LiveMetrics>,
    stop_threshold: Duration,
    wall_clock_timeout: Option<Duration>,
    step_timeout: Option<Duration>,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    use std::sync::atomic::{AtomicBool, AtomicU8};

    let interrupt_count = Arc::new(AtomicU8::new(0));
    let child_exited = Arc::new(AtomicBool::new(false));
    let child_pid = child.id();

    let counter = Arc::clone(&interrupt_count);
    let exited = Arc::clone(&child_exited);
    let _guard = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            // Don't signal a PID we no longer own (it may have been
            // recycled onto an unrelated process).
            if exited.load(Ordering::SeqCst) {
                return;
            }
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if !child_in_own_pgroup {
                return;
            }
            match count {
                1 => {
                    libc::kill(-(child_pid as i32), libc::SIGINT);
                }
                2 => {
                    libc::kill(-(child_pid as i32), libc::SIGTERM);
                }
                _ => {
                    libc::kill(-(child_pid as i32), libc::SIGKILL);
                }
            }
        })
    }?;

    let mut early_termination: Option<EarlyTermination> = None;
    let mut wall_clock_tripped = false;
    let mut grace_deadline: Option<Instant> = None;
    let poll_interval = Duration::from_millis(75);
    let grace_period = Duration::from_secs(5);
    let loop_start = Instant::now();

    loop {
        if let Some(status) = child.try_wait()? {
            // Mark the PID as reaped before the 5-second grace window's
            // signal guard can drop so we never signal a recycled PID.
            child_exited.store(true, Ordering::SeqCst);
            let code = exit_code_from_status(status);
            let was_interrupted = interrupt_count.load(Ordering::SeqCst) > 0;
            let termination = if was_interrupted {
                claudine::harness::ProcessTermination::Interrupted
            } else if wall_clock_tripped {
                claudine::harness::ProcessTermination::TimedOut
            } else if early_termination.is_some() {
                early_termination_process_outcome(early_termination.as_ref())
            } else {
                claudine::harness::ProcessTermination::Completed
            };
            return Ok((code, termination, early_termination));
        }

        // Wall-clock timeout check: short-circuits directly to TimedOut
        // without routing through the EarlyTermination surface (matches
        // the legacy wait_with_timeout behavior for the streaming path).
        if !wall_clock_tripped
            && early_termination.is_none()
            && let Some(budget) = wall_clock_timeout
            && loop_start.elapsed() >= budget
        {
            tracing::warn!(
                child_pid,
                timeout_secs = budget.as_secs(),
                "wall-clock timeout exceeded; sending SIGTERM to child process group",
            );
            let kill_pid = if child_in_own_pgroup {
                -(child_pid as i32)
            } else {
                child_pid as i32
            };
            unsafe {
                libc::kill(kill_pid, libc::SIGTERM);
            }
            wall_clock_tripped = true;
            grace_deadline = Some(Instant::now() + grace_period);
        }

        if early_termination.is_none() && !wall_clock_tripped {
            match early_rx.try_recv() {
                Ok(signal) => {
                    tracing::info!(
                        child_pid,
                        "early-termination signal received; sending SIGTERM to child process group",
                    );
                    let kill_pid = if child_in_own_pgroup {
                        -(child_pid as i32)
                    } else {
                        child_pid as i32
                    };
                    unsafe {
                        libc::kill(kill_pid, libc::SIGTERM);
                    }
                    early_termination = Some(signal);
                    grace_deadline = Some(Instant::now() + grace_period);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {
                    // Channel closed with no signal; continue normal polling.
                }
            }
        }

        if early_termination.is_none()
            && !wall_clock_tripped
            && let Some(metrics) = live_metrics.as_ref()
            && let Some(signal) =
                detect_opencode_hang_termination(metrics, Instant::now(), stop_threshold)
        {
            let kill_pid = if child_in_own_pgroup {
                -(child_pid as i32)
            } else {
                child_pid as i32
            };
            unsafe {
                libc::kill(kill_pid, libc::SIGTERM);
            }
            early_termination = Some(signal);
            grace_deadline = Some(Instant::now() + grace_period);
        }

        // Step-silence timeout check: user-configured hard kill that
        // maps to ProcessTermination::TimedOut. Fires after every other
        // early-termination branch so wall-clock and rate-limit recoveries
        // keep precedence when they happen in the same poll tick.
        if early_termination.is_none()
            && !wall_clock_tripped
            && let Some(silence_budget) = step_timeout
            && let Some(metrics) = live_metrics.as_ref()
            && let Some(signal) = detect_step_timeout(metrics, Instant::now(), silence_budget)
        {
            tracing::warn!(
                child_pid,
                step_timeout_secs = silence_budget.as_secs(),
                "step_timeout exceeded; sending SIGTERM to child process group",
            );
            let kill_pid = if child_in_own_pgroup {
                -(child_pid as i32)
            } else {
                child_pid as i32
            };
            unsafe {
                libc::kill(kill_pid, libc::SIGTERM);
            }
            early_termination = Some(signal);
            grace_deadline = Some(Instant::now() + grace_period);
        }

        if let Some(deadline) = grace_deadline
            && Instant::now() >= deadline
        {
            tracing::warn!(
                child_pid,
                "child did not exit after early-termination SIGTERM; escalating to SIGKILL",
            );
            let kill_pid = if child_in_own_pgroup {
                -(child_pid as i32)
            } else {
                child_pid as i32
            };
            unsafe {
                libc::kill(kill_pid, libc::SIGKILL);
            }
            grace_deadline = None;
        }

        std::thread::sleep(poll_interval);
    }
}

#[cfg(not(unix))]
#[allow(clippy::too_many_arguments)]
fn wait_with_signal_and_early_termination(
    child: &mut Child,
    _child_in_own_pgroup: bool,
    early_rx: Receiver<EarlyTermination>,
    live_metrics: Option<LiveMetrics>,
    stop_threshold: Duration,
    wall_clock_timeout: Option<Duration>,
    step_timeout: Option<Duration>,
) -> Result<(
    i32,
    claudine::harness::ProcessTermination,
    Option<EarlyTermination>,
)> {
    let mut early_termination: Option<EarlyTermination> = None;
    let mut wall_clock_tripped = false;
    let mut grace_deadline: Option<Instant> = None;
    let poll_interval = Duration::from_millis(75);
    let grace_period = Duration::from_secs(5);
    let loop_start = Instant::now();

    loop {
        if let Some(status) = child.try_wait()? {
            let code = exit_code_from_status(status);
            let termination = if wall_clock_tripped {
                claudine::harness::ProcessTermination::TimedOut
            } else if early_termination.is_some() {
                early_termination_process_outcome(early_termination.as_ref())
            } else {
                claudine::harness::ProcessTermination::Completed
            };
            return Ok((code, termination, early_termination));
        }

        if !wall_clock_tripped
            && early_termination.is_none()
            && let Some(budget) = wall_clock_timeout
            && loop_start.elapsed() >= budget
        {
            let _ = child.kill();
            wall_clock_tripped = true;
            grace_deadline = Some(Instant::now() + grace_period);
        }

        if early_termination.is_none() && !wall_clock_tripped {
            match early_rx.try_recv() {
                Ok(signal) => {
                    let _ = child.kill();
                    early_termination = Some(signal);
                    grace_deadline = Some(Instant::now() + grace_period);
                }
                Err(TryRecvError::Empty) => {}
                Err(TryRecvError::Disconnected) => {}
            }
        }

        if early_termination.is_none()
            && !wall_clock_tripped
            && let Some(metrics) = live_metrics.as_ref()
            && let Some(signal) =
                detect_opencode_hang_termination(metrics, Instant::now(), stop_threshold)
        {
            let _ = child.kill();
            early_termination = Some(signal);
            grace_deadline = Some(Instant::now() + grace_period);
        }

        if early_termination.is_none()
            && !wall_clock_tripped
            && let Some(silence_budget) = step_timeout
            && let Some(metrics) = live_metrics.as_ref()
            && let Some(signal) = detect_step_timeout(metrics, Instant::now(), silence_budget)
        {
            let _ = child.kill();
            early_termination = Some(signal);
            grace_deadline = Some(Instant::now() + grace_period);
        }

        if let Some(deadline) = grace_deadline
            && Instant::now() >= deadline
        {
            let _ = child.kill();
            grace_deadline = None;
        }

        std::thread::sleep(poll_interval);
    }
}

/// Overwrite the synthesized summary fields when the stderr bridge or the
/// OpenCode wait loop signals an early-exit condition.
///
/// Preserves any parser-provided `rate_limit` fields field-by-field:
/// `is_throttled` is forced to `Some(true)`, `message` is only set when
/// absent, and `reset_at` keeps the first non-`None` value.
fn apply_early_termination_to_summary(
    summary: &mut StreamExecutionSummary,
    termination: &EarlyTermination,
) {
    match termination {
        EarlyTermination::RateLimit { message, reset_at } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("usage_limit_reached".into());
            summary.error_message = Some(message.clone());

            let mut rate_limit = summary.rate_limit.clone().unwrap_or_default();
            rate_limit.is_throttled = Some(true);
            if rate_limit.message.is_none() {
                rate_limit.message = Some(message.clone());
            }
            if rate_limit.reset_at.is_none()
                && let Some(reset) = reset_at
            {
                rate_limit.reset_at = Some(*reset);
            }
            summary.rate_limit = Some(rate_limit);
        }
        EarlyTermination::CompletedButHung { .. } => {
            summary.exit_code = 0;
            summary.is_error = false;
            summary.error_kind = None;
            summary.error_message = None;
        }
        EarlyTermination::StepTimeout { message } => {
            summary.exit_code = 1;
            summary.is_error = true;
            summary.error_kind = Some("step_timeout".into());
            summary.error_message = Some(message.clone());
        }
    }
}

/// Wait for the child with a timeout, sending SIGTERM then SIGKILL.
///
/// Returns `(exit_code, termination_kind)`.
#[cfg(unix)]
fn wait_with_timeout(
    child: &mut Child,
    seconds: u64,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let grace_period = Duration::from_secs(5);

    loop {
        match child.try_wait()? {
            Some(status) => {
                return Ok((
                    exit_code_from_status(status),
                    claudine::harness::ProcessTermination::Completed,
                ));
            }
            None => {
                if Instant::now() >= deadline {
                    tracing::warn!(
                        timeout_secs = seconds,
                        child_pid = child.id(),
                        "child process timed out; sending SIGTERM"
                    );
                    // Send SIGTERM
                    unsafe {
                        libc::kill(child.id() as i32, libc::SIGTERM);
                    }

                    // Wait for grace period
                    let kill_deadline = Instant::now() + grace_period;
                    loop {
                        match child.try_wait()? {
                            Some(status) => {
                                return Ok((
                                    exit_code_from_status(status),
                                    claudine::harness::ProcessTermination::TimedOut,
                                ));
                            }
                            None => {
                                if Instant::now() >= kill_deadline {
                                    tracing::warn!(
                                        timeout_secs = seconds,
                                        child_pid = child.id(),
                                        "child process did not exit after SIGTERM; sending SIGKILL"
                                    );
                                    // Send SIGKILL
                                    unsafe {
                                        libc::kill(child.id() as i32, libc::SIGKILL);
                                    }
                                    let status = child.wait()?;
                                    return Ok((
                                        exit_code_from_status(status),
                                        claudine::harness::ProcessTermination::TimedOut,
                                    ));
                                }
                                std::thread::sleep(Duration::from_millis(100));
                            }
                        }
                    }
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

#[cfg(not(unix))]
fn wait_with_timeout(
    child: &mut Child,
    seconds: u64,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(seconds);

    loop {
        match child.try_wait()? {
            Some(status) => {
                return Ok((
                    exit_code_from_status(status),
                    claudine::harness::ProcessTermination::Completed,
                ));
            }
            None => {
                if Instant::now() >= deadline {
                    tracing::warn!(
                        timeout_secs = seconds,
                        child_pid = child.id(),
                        "child process timed out; killing process"
                    );
                    child.kill()?;
                    let status = child.wait()?;
                    return Ok((
                        exit_code_from_status(status),
                        claudine::harness::ProcessTermination::TimedOut,
                    ));
                }
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

/// Captured output from a child process.
pub(crate) struct CapturedChildOutput {
    pub(crate) exit_code: i32,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

/// Spawn a provider child process and capture its output.
///
/// Behaves like `run_child()` but pipes stdout and stderr into strings
/// instead of forwarding to the terminal. Noise filtering still applies
/// to the captured output. No output is printed live.
pub(crate) fn run_child_capture(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    io: ChildIoOptions<'_>,
    child_spawned: &mut bool,
) -> Result<ProcessResult<CapturedChildOutput>> {
    debug_assert!(
        env.contains_key(&OsString::from("PATH")),
        "child env is missing PATH"
    );
    debug_assert!(
        env.contains_key(&OsString::from("HOME")),
        "child env is missing HOME"
    );

    let needs_stdin_pipe = io.stdin_seed.is_some();

    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(if needs_stdin_pipe {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let spawned_at = Instant::now();
    let mut child = command.spawn()?;
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(child.id()));

    // Shared first-response trackers (always piped in capture mode).
    let first_stdout_at = Arc::new(std::sync::Mutex::new(None));
    let first_stderr_at = Arc::new(std::sync::Mutex::new(None));

    // Spawn reader threads BEFORE writing stdin (see run_child deadlock note).

    // Capture stdout into a string, applying noise filtering
    let stdout_pipe = child
        .stdout
        .take()
        .expect("child stdout must be piped: Stdio::piped() was set on the child Command above");
    let stdout_noise: Vec<String> = io
        .stdout_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let first_stdout_at_clone = Arc::clone(&first_stdout_at);
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if stdout_noise.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }
            {
                let mut g = first_stdout_at_clone.lock().unwrap();
                if g.is_none() {
                    *g = Some(Instant::now());
                }
            }
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    // Capture stderr into a string, applying noise filtering
    let stderr_pipe = child
        .stderr
        .take()
        .expect("child stderr must be piped: Stdio::piped() was set on the child Command above");
    let stderr_noise: Vec<String> = io
        .stderr_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let first_stderr_at_clone = Arc::clone(&first_stderr_at);
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if stderr_noise.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }
            {
                let mut g = first_stderr_at_clone.lock().unwrap();
                if g.is_none() {
                    *g = Some(Instant::now());
                }
            }
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    // Write stdin seed AFTER reader threads are spawned (see run_child deadlock note).
    if let Some(seed) = io.stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        stdin_pipe.write_all(seed.as_bytes())?;
    }

    let (exit_code, termination) = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child, true)?
    };

    kill_process_group(&mut child);

    let thread_join_timeout = Duration::from_secs(5);
    let stdout = join_with_timeout_or(stdout_handle, thread_join_timeout, String::new());
    let stderr = join_with_timeout_or(stderr_handle, thread_join_timeout, String::new());

    let total_elapsed = spawned_at.elapsed();
    let first_response = resolve_first_response(
        None,
        *first_stdout_at.lock().unwrap(),
        *first_stderr_at.lock().unwrap(),
        spawned_at,
    );

    Ok(ProcessResult {
        data: CapturedChildOutput {
            exit_code,
            stdout,
            stderr,
        },
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed,
            first_response_latency: first_response,
        },
    })
}

/// Spawn a dedicated ticker that runs [`StreamTextRenderer::flush_if_idle`]
/// every 30 seconds.
///
/// Independent from the prompt-scoped timing monitor so buffered markdown
/// reaches stdout even on runs that have no prompt context (wrapper
/// passthrough) and regardless of whether any periodic header is being
/// emitted. The 30-second cadence and 30-second silence window are the
/// tuning preserved from the previous heartbeat thread.
fn spawn_flush_if_idle_ticker(
    stream_output: Arc<StreamOutput>,
    text_renderer: Arc<std::sync::Mutex<StreamTextRenderer>>,
) -> (Arc<AtomicBool>, thread::JoinHandle<()>) {
    const SILENCE_WINDOW: Duration = Duration::from_secs(30);
    const CADENCE: Duration = Duration::from_secs(30);

    let done = Arc::new(AtomicBool::new(false));
    let done_flag = Arc::clone(&done);
    let handle = thread::spawn(move || {
        let mut next_tick = Instant::now() + CADENCE;
        while !done_flag.load(Ordering::Relaxed) {
            let now = Instant::now();
            if now >= next_tick {
                if let Ok(mut r) = text_renderer.lock() {
                    let mut writer = stream_output.stdout_writer();
                    r.flush_if_idle(&mut writer, SILENCE_WINDOW);
                }
                next_tick += CADENCE;
                continue;
            }
            let sleep_for = next_tick
                .saturating_duration_since(now)
                .min(Duration::from_secs(1));
            thread::sleep(sleep_for);
        }
    });

    (done, handle)
}

/// Spawn the prompt-scoped timing monitor.
///
/// Emits the periodic timing header anchored on the prompt's start time
/// (`t=0`, `t=10m`, `t=20m`, …) plus two fire-once `Status::Warning`
/// messages when the user-configured `timeout_warn` / `step_timeout_warn`
/// thresholds cross. Only spawned for structured runs that carry a
/// prompt context (every `claudine compose` / `inline-compose` /
/// `sequence` run); wrapper passthrough runs skip the monitor entirely.
///
/// When both warnings cross on the same poll cycle, the step-scoped
/// warning is emitted first per the feature spec.
fn spawn_prompt_timing_monitor(
    started_at: Instant,
    started_at_wall: chrono::DateTime<chrono::Local>,
    prompt_timing: PromptTimingContext,
    hard_timeout: Option<Duration>,
    hard_step_timeout: Option<Duration>,
    live_metrics: LiveMetrics,
    stream_output: Arc<StreamOutput>,
) -> (Arc<AtomicBool>, thread::JoinHandle<()>) {
    let done = Arc::new(AtomicBool::new(false));
    let done_flag = Arc::clone(&done);
    let prompt_path_display = prompt_timing.absolute_path.display().to_string();

    let handle = thread::spawn(move || {
        let term = crate::log::terminal();

        emit_timing_header(
            HeaderKind::Zero,
            Duration::ZERO,
            &prompt_timing,
            &prompt_path_display,
            &term,
            &stream_output,
        );

        let mut next_header_tick = started_at + prompt_timing_mod::HEADER_CADENCE;
        let mut timeout_warn_fired = false;
        let poll_interval = Duration::from_secs(1);

        while !done_flag.load(Ordering::Relaxed) {
            let now = Instant::now();
            let elapsed = now.saturating_duration_since(started_at);

            if now >= next_header_tick {
                emit_timing_header(
                    HeaderKind::Tick,
                    elapsed,
                    &prompt_timing,
                    &prompt_path_display,
                    &term,
                    &stream_output,
                );
                next_header_tick += prompt_timing_mod::HEADER_CADENCE;
            }

            // Step-scoped warning must be emitted BEFORE the prompt-scoped
            // warning when both cross on the same poll cycle (feature spec
            // §2, "Ordering when both cross in the same cycle").
            if let Some(threshold) = prompt_timing.step_timeout_warn {
                maybe_emit_step_timeout_warn(
                    &prompt_timing,
                    &prompt_path_display,
                    &live_metrics,
                    threshold,
                    hard_step_timeout,
                    started_at_wall,
                    started_at,
                    now,
                    &term,
                    &stream_output,
                );
            }

            if let Some(threshold) = prompt_timing.timeout_warn
                && !timeout_warn_fired
                && elapsed >= threshold
            {
                emit_timeout_warn(
                    &prompt_timing,
                    &prompt_path_display,
                    elapsed,
                    threshold,
                    hard_timeout,
                    started_at_wall,
                    started_at,
                    &term,
                    &stream_output,
                );
                timeout_warn_fired = true;
            }

            let sleep_for = next_header_tick
                .saturating_duration_since(now)
                .min(poll_interval);
            thread::sleep(sleep_for);
        }
    });

    (done, handle)
}

fn emit_timing_header(
    kind: HeaderKind,
    elapsed: Duration,
    prompt_timing: &PromptTimingContext,
    prompt_path_display: &str,
    term: &Terminal,
    stream_output: &StreamOutput,
) {
    let body = prompt_timing_mod::render_header_prose(kind, elapsed, prompt_timing);
    let rendered = Prose::new(body).render(term);
    stream_output.emit_stderr_line(&rendered);
    tracing::info!(
        prompt_path = %prompt_path_display,
        elapsed_secs = elapsed.as_secs(),
        tick_kind = match kind {
            HeaderKind::Zero => "zero",
            HeaderKind::Tick => "tick",
        },
        "prompt timing header emitted",
    );
}

#[allow(clippy::too_many_arguments)]
fn emit_timeout_warn(
    prompt_timing: &PromptTimingContext,
    prompt_path_display: &str,
    elapsed: Duration,
    threshold: Duration,
    hard_timeout: Option<Duration>,
    started_at_wall: chrono::DateTime<chrono::Local>,
    started_at: Instant,
    term: &Terminal,
    stream_output: &StreamOutput,
) {
    let hard_remaining = hard_timeout.and_then(|hard| {
        let deadline_instant = started_at + hard;
        let remaining = deadline_instant.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            None
        } else {
            let deadline_wall = started_at_wall + chrono::Duration::from_std(hard).ok()?;
            Some((remaining, deadline_wall))
        }
    });
    let body = prompt_timing_mod::render_timeout_warn_prose(elapsed, hard_remaining, prompt_timing);
    let rendered = Status::from_prose(body)
        .state(StatusState::Warning)
        .render(term);
    stream_output.emit_stderr_line(&rendered);
    tracing::info!(
        prompt_path = %prompt_path_display,
        elapsed_secs = elapsed.as_secs(),
        threshold_secs = threshold.as_secs(),
        remaining_secs = hard_remaining.map(|(r, _)| r.as_secs()).unwrap_or(0),
        hard_timeout_set = hard_timeout.is_some(),
        "timeout_warn emitted",
    );
}

#[allow(clippy::too_many_arguments)]
fn maybe_emit_step_timeout_warn(
    prompt_timing: &PromptTimingContext,
    prompt_path_display: &str,
    live_metrics: &LiveMetrics,
    threshold: Duration,
    hard_step_timeout: Option<Duration>,
    started_at_wall: chrono::DateTime<chrono::Local>,
    started_at: Instant,
    now: Instant,
    term: &Terminal,
    stream_output: &StreamOutput,
) {
    let (silence, last_event_at) = {
        let mut state = match live_metrics.lock() {
            Ok(state) => state,
            Err(_) => return,
        };
        if !progress::should_warn_stall(&state, now, threshold) {
            return;
        }
        state.last_stall_warning_at = Some(now);
        let Some(last_event) = state.last_event_at else {
            return;
        };
        (now.saturating_duration_since(last_event), last_event)
    };

    let hard_remaining = hard_step_timeout.and_then(|hard| {
        let deadline_instant = last_event_at + hard;
        let remaining = deadline_instant.saturating_duration_since(now);
        if remaining.is_zero() {
            None
        } else {
            let last_event_wall = instant_to_local(started_at_wall, started_at, last_event_at);
            let deadline_wall = last_event_wall + chrono::Duration::from_std(hard).ok()?;
            Some((remaining, deadline_wall))
        }
    });

    let body =
        prompt_timing_mod::render_step_timeout_warn_prose(silence, hard_remaining, prompt_timing);
    let rendered = Status::from_prose(body)
        .state(StatusState::Warning)
        .render(term);
    stream_output.emit_stderr_line(&rendered);
    tracing::info!(
        prompt_path = %prompt_path_display,
        silence_secs = silence.as_secs(),
        threshold_secs = threshold.as_secs(),
        remaining_secs = hard_remaining.map(|(r, _)| r.as_secs()).unwrap_or(0),
        hard_step_timeout_set = hard_step_timeout.is_some(),
        "step_timeout_warn emitted",
    );
}

/// Translate a monotonic [`Instant`] into a wall-clock `DateTime<Local>`
/// using the prompt-scoped anchor (`started_at_wall` / `started_at`).
fn instant_to_local(
    anchor_wall: chrono::DateTime<chrono::Local>,
    anchor_instant: Instant,
    sample: Instant,
) -> chrono::DateTime<chrono::Local> {
    if sample >= anchor_instant {
        let delta = sample.duration_since(anchor_instant);
        match chrono::Duration::from_std(delta) {
            Ok(d) => anchor_wall + d,
            Err(_) => anchor_wall,
        }
    } else {
        let delta = anchor_instant.duration_since(sample);
        match chrono::Duration::from_std(delta) {
            Ok(d) => anchor_wall - d,
            Err(_) => anchor_wall,
        }
    }
}

/// Resolve the OpenCode silent-stall recovery threshold.
///
/// This is separate from the user-facing stall warning: the warning fires
/// first, then OpenCode gets additional time to recover before Claudine
/// kills the hung process group. Invalid or non-positive values fall back to
/// 5 minutes.
/// Format a duration in seconds for internal early-termination messages.
///
/// Used by the step-silence and OpenCode-hang detectors to compose their
/// `EarlyTermination::*` messages (these feed the summary, not the
/// user-visible timing surface). Kept as a small local helper so the
/// internal format stays stable.
fn format_internal_duration(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3_600 {
        format!("{}m", secs / 60)
    } else {
        format!("{}h{}m", secs / 3_600, (secs % 3_600) / 60)
    }
}

/// Detect a step-silence timeout for the harness `step_timeout` field.
///
/// Returns `Some(EarlyTermination::StepTimeout)` when the time since the last
/// stream event exceeds `step_timeout`. Returns `None` when `last_event_at`
/// is not yet populated (first-event grace so provider startup does not
/// trip a kill) or when silence is still under budget.
///
/// Unlike [`detect_opencode_hang_termination`], this helper does not gate on
/// `in_flight` state or `provider_status`: any silence past the budget is a
/// hard kill. The caller is responsible for SIGTERM escalation.
fn detect_step_timeout(
    metrics: &LiveMetrics,
    now: Instant,
    step_timeout: Duration,
) -> Option<EarlyTermination> {
    let state = metrics.lock().ok()?;
    let last_event_at = state.last_event_at?;
    let silence = now.saturating_duration_since(last_event_at);
    if silence >= step_timeout {
        let silence_text = format_internal_duration(silence.as_secs());
        Some(EarlyTermination::StepTimeout {
            message: format!(
                "no stream activity for {silence_text}; terminating due to step_timeout"
            ),
        })
    } else {
        None
    }
}

fn detect_opencode_hang_termination(
    metrics: &LiveMetrics,
    now: Instant,
    stop_threshold: Duration,
) -> Option<EarlyTermination> {
    let state = metrics.lock().ok()?;
    let last_event_at = state.last_event_at?;
    let silence = now.saturating_duration_since(last_event_at);

    if !state.in_flight.is_empty() || !state.in_flight_subagents.is_empty() {
        return None;
    }

    let silence_text = format_internal_duration(silence.as_secs());
    if silence >= stop_threshold && state.provider_status.as_deref() == Some("stop") {
        return Some(EarlyTermination::CompletedButHung {
            message: format!(
                "OpenCode reported stop but stayed alive for {silence_text}; terminating hung process"
            ),
        });
    }

    None
}

fn early_termination_process_outcome(
    early_termination: Option<&EarlyTermination>,
) -> claudine::harness::ProcessTermination {
    match early_termination {
        Some(EarlyTermination::StepTimeout { .. }) => {
            claudine::harness::ProcessTermination::TimedOut
        }
        Some(EarlyTermination::CompletedButHung { .. }) => {
            claudine::harness::ProcessTermination::Completed
        }
        Some(EarlyTermination::RateLimit { .. }) => {
            claudine::harness::ProcessTermination::Completed
        }
        None => claudine::harness::ProcessTermination::Completed,
    }
}

/// Signal a timing ticker thread to stop and join it.
///
/// Shared by the flush-if-idle ticker and the prompt-timing monitor —
/// both return the same `(done_flag, handle)` pair and need identical
/// teardown. `None` is accepted so callers can pass through optional
/// handles without an extra match.
fn stop_timing_ticker(ticker: Option<(Arc<AtomicBool>, thread::JoinHandle<()>)>) {
    if let Some((done, handle)) = ticker {
        done.store(true, Ordering::Relaxed);
        let _ = handle.join();
    }
}

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
/// [`SemanticEvent::OutputText`]: claudine::stream::semantic::SemanticEvent::OutputText
/// [`SemanticEvent::Reasoning`]: claudine::stream::semantic::SemanticEvent::Reasoning
/// [`LiveSemanticSink`]: super::live_semantic_sink::LiveSemanticSink
pub(crate) type SemanticParserBuilder = Box<
    dyn FnOnce(OutputTextCallback, ReasoningCallback) -> Box<dyn SemanticStreamParser>
        + Send
        + 'static,
>;

/// Spawn a provider child process with structured semantic stream parsing.
///
/// This is the Phase 3.4 replacement for [`run_child_stream`]. The
/// difference is the stdout loop: instead of switching on a returned
/// [`SemanticEvent`]s, the parser drives a [`SemanticEventSink`] that the
/// caller has already wired up for status rendering, dispatch, metrics,
/// and JSONL logging. This function's only rendering responsibility is
/// wiring the terminal-local `StreamTextRenderer` instance to the sink
/// through the builder callback so it can run inside the parser thread.
/// Reasoning rendering is owned entirely by `LiveSemanticSink`.
///
/// [`SemanticEventSink`]: claudine::stream::semantic::SemanticEventSink
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_child_stream_semantic(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    step_timeout: Option<u64>,
    stderr_noise_prefixes: &[&str],
    suppress_stderr_on_success: bool,
    show_timing_output: bool,
    stdin_seed: Option<&str>,
    build_parser: SemanticParserBuilder,
    child_spawned: &mut bool,
    live_metrics: LiveMetrics,
    stream_output: Arc<StreamOutput>,
    stderr_bridge: Option<StderrBridgeHandle>,
    prompt_timing: Option<PromptTimingContext>,
) -> Result<ProcessResult<StreamExecutionSummary>> {
    debug_assert!(env.contains_key(&OsString::from("PATH")));
    debug_assert!(env.contains_key(&OsString::from("HOME")));

    let needs_stdin_pipe = stdin_seed.is_some();
    let started_at = Instant::now();
    let started_at_wall = chrono::Local::now();

    let mut command = Command::new(binary);
    command
        .args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(if needs_stdin_pipe {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }

    let mut child = command.spawn()?;
    *child_spawned = true;
    Span::current().record("child_pid", tracing::field::display(child.id()));

    // Terminal-local renderer for OutputText (stdout markdown). Wrapped
    // in Arc<Mutex<_>> so the builder closures can retain independent
    // handles without untangling lifetimes across the FnMut boundary of
    // the sink's callback storage. Shared with the flush-if-idle ticker
    // so buffered markdown can be surfaced even when the provider stalls
    // without closing stdout.
    //
    // Note: reasoning rendering is owned entirely by LiveSemanticSink,
    // which emits BlockQuote-formatted thinking text through the
    // section-aware stderr emitter. The reasoning_cb passed to the
    // parser builder is a no-op.
    let text_renderer: Arc<std::sync::Mutex<StreamTextRenderer>> =
        Arc::new(std::sync::Mutex::new(StreamTextRenderer::new()));

    let stdout_output = stream_output.clone();
    let wait_loop_metrics = live_metrics.clone();

    // Dedicated 30-second ticker that flushes any buffered markdown the
    // provider has not terminated with a paragraph boundary. Independent
    // from the prompt-timing monitor per feature spec.
    let flush_ticker = Some(spawn_flush_if_idle_ticker(
        stream_output.clone(),
        text_renderer.clone(),
    ));

    // Prompt-scoped periodic header + warnings. Only started when the
    // caller provided a prompt context (every composition run) and when
    // the CLI is rendering timing output at all. Wrapper passthrough
    // runs pass `prompt_timing = None` and skip the monitor entirely.
    let timing_monitor = if show_timing_output {
        prompt_timing.map(|ctx| {
            spawn_prompt_timing_monitor(
                started_at,
                started_at_wall,
                ctx,
                timeout.map(Duration::from_secs),
                step_timeout.map(Duration::from_secs),
                live_metrics.clone(),
                stream_output.clone(),
            )
        })
    } else {
        None
    };

    // First-response trackers for structured stream mode:
    // semantic stdout (preferred), raw stdout (fallback), stderr (final fallback).
    let first_semantic_at = Arc::new(std::sync::Mutex::new(None));
    let first_raw_stdout_at = Arc::new(std::sync::Mutex::new(None));
    let first_stderr_at = Arc::new(std::sync::Mutex::new(None));

    // Spawn reader threads BEFORE writing stdin (see run_child deadlock note).
    let stdout_pipe = child
        .stdout
        .take()
        .expect("child stdout must be piped: Stdio::piped() was set on the child Command above");
    let stream_span = Span::current();
    let stdout_renderer = text_renderer.clone();
    let first_semantic_at_clone = Arc::clone(&first_semantic_at);
    let first_raw_stdout_at_clone = Arc::clone(&first_raw_stdout_at);
    let stdout_handle = thread::spawn(move || {
        let _stream_guard = stream_span.enter();
        let _parse_span = info_span!("stream_parse").entered();
        let reader = BufReader::new(stdout_pipe);
        let mut out = stdout_output.stdout_writer();

        let text_renderer = stdout_renderer;

        let output_cb: OutputTextCallback = {
            let text = text_renderer.clone();
            let mut writer = stdout_output.stdout_writer();
            let first_at = first_semantic_at_clone;
            Box::new(move |chunk: &str| {
                if !chunk.is_empty() {
                    let mut g = first_at.lock().unwrap();
                    if g.is_none() {
                        *g = Some(Instant::now());
                    }
                }
                if let Ok(mut r) = text.lock() {
                    r.push(&mut writer, chunk);
                }
            })
        };
        let reasoning_cb: ReasoningCallback = Box::new(|_chunk: &str| {});

        let mut parser: Box<dyn SemanticStreamParser> = build_parser(output_cb, reasoning_cb);
        let mut fallback_mode = false;

        for line in reader.lines() {
            let Ok(line) = line else { break };

            {
                let mut g = first_raw_stdout_at_clone.lock().unwrap();
                if g.is_none() {
                    *g = Some(Instant::now());
                }
            }

            if fallback_mode {
                let _ = writeln!(out, "{}", crate::log::maybe_strip(&line));
                continue;
            }

            match parser.feed_line(&line) {
                Ok(()) => {}
                Err(StreamParseError::MalformedLine { .. }) => {
                    // Semantic parsers emit Warning events instead of
                    // returning MalformedLine, but guard the variant here
                    // in case a legacy adapter still surfaces it.
                    tracing::debug!("skipping malformed stream line: {line}");
                }
                Err(StreamParseError::Fatal(_)) => {
                    if let Ok(mut r) = text_renderer.lock() {
                        r.flush_remaining(&mut out);
                    }
                    fallback_mode = true;
                    let _ = writeln!(out, "{}", crate::log::maybe_strip(&line));
                }
            }
        }

        if let Ok(mut r) = text_renderer.lock() {
            r.flush_remaining(&mut out);
        }
        parser
    });

    let pipe = child
        .stderr
        .take()
        .expect("child stderr must be piped: Stdio::piped() was set on the child Command above");
    let prefixes: Vec<String> = stderr_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let plain = crate::log::is_plain();
    let stderr_term = crate::log::terminal();
    let stderr_span = Span::current();
    let (mut bridge_for_thread, finalize_for_main, early_terminate_rx) = match stderr_bridge {
        Some(StderrBridgeHandle {
            bridge,
            finalize,
            early_terminate,
        }) => (Some(bridge), Some(finalize), early_terminate),
        None => (None, None, None),
    };
    let has_bridge = bridge_for_thread.is_some();
    let capture_always = has_bridge;
    let first_stderr_at_clone = Arc::clone(&first_stderr_at);
    let stderr_handle = thread::spawn(move || {
        let _stderr_guard = stderr_span.enter();
        let reader = BufReader::new(pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }

            {
                let mut g = first_stderr_at_clone.lock().unwrap();
                if g.is_none() {
                    *g = Some(Instant::now());
                }
            }

            // When a bridge is installed, offer the raw line first so it can
            // classify structured log records (and their multi-line tails) in
            // real time. Consumed lines are suppressed from raw passthrough
            // and never land in the captured buffer — the semantic event the
            // bridge emitted already carries everything operators need.
            if let Some(bridge) = bridge_for_thread.as_mut()
                && matches!(bridge.ingest(&line), StderrIngestOutcome::Consumed)
            {
                continue;
            }

            let formatted = crate::output::try_format_api_error(&line, &stderr_term);
            let output_line = formatted.as_deref().unwrap_or(&line);
            let output_line = if plain {
                biscuit_terminal::prelude::strip_escape_codes(output_line)
            } else {
                output_line.to_string()
            };

            if suppress_stderr_on_success || capture_always {
                if !captured.is_empty() {
                    captured.push('\n');
                }
                captured.push_str(&output_line);
            }
            if !suppress_stderr_on_success {
                let mut err = std::io::stderr().lock();
                let _ = writeln!(err, "{output_line}");
            }
        }
        captured
    });

    if let Some(seed) = stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        stdin_pipe.write_all(seed.as_bytes())?;
    }

    // OpenCode hang recovery: `opencode_stop_threshold` is the post-"stop"
    // grace window (120s). It does not drive a user-visible timing line.
    let opencode_stop_threshold = Duration::from_secs(120);
    let wall_clock_timeout = timeout.map(Duration::from_secs);
    let step_timeout_duration = step_timeout.map(Duration::from_secs);
    let needs_advanced_wait = wall_clock_timeout.is_some()
        || step_timeout_duration.is_some()
        || early_terminate_rx.is_some();
    let (exit_code, termination, early_termination) = if needs_advanced_wait {
        // Synthesize a disconnected receiver when no stderr bridge is
        // installed so the wait loop can still enforce wall-clock and
        // step timeouts for non-OpenCode providers.
        let rx = early_terminate_rx.unwrap_or_else(|| {
            let (_tx, rx) = std::sync::mpsc::channel();
            rx
        });
        wait_with_signal_and_early_termination(
            &mut child,
            true,
            rx,
            Some(wait_loop_metrics),
            opencode_stop_threshold,
            wall_clock_timeout,
            step_timeout_duration,
        )?
    } else {
        let (code, term) = wait_with_signal_handling(&mut child, true)?;
        (code, term, None)
    };

    if let Some(
        EarlyTermination::CompletedButHung { message } | EarlyTermination::StepTimeout { message },
    ) = early_termination.as_ref()
    {
        let rendered = Status::new(message)
            .state(StatusState::Warning)
            .render(&crate::log::terminal());
        stream_output.emit_stderr_line(&rendered);
    }

    kill_process_group(&mut child);
    stop_timing_ticker(flush_ticker);
    stop_timing_ticker(timing_monitor);

    let thread_join_timeout = Duration::from_secs(5);
    let parser: Box<dyn SemanticStreamParser> = join_with_timeout_or(
        stdout_handle,
        thread_join_timeout,
        Box::new(ErrorParser { exit_code }),
    );

    let captured = join_with_timeout_or(stderr_handle, thread_join_timeout, String::new());
    if suppress_stderr_on_success && exit_code != 0 && !captured.is_empty() {
        eprintln!("{captured}");
    }

    let mut summary = parser.finish(exit_code);
    if summary.duration_ms.is_none() {
        summary.duration_ms = Some(started_at.elapsed().as_millis() as u64);
    }

    // Apply early-termination overrides before the stderr finalizer so the
    // finalizer's badge recomputation observes the synthesized error fields
    // (for example, a `usage_limit_reached` error_kind that maps to a Quota
    // badge). The bridge signaled early termination from the stderr thread;
    // the main wait loop then killed the child's process group, so the raw
    // exit code reflects SIGTERM rather than a meaningful provider status.
    if let Some(termination) = early_termination.as_ref() {
        apply_early_termination_to_summary(&mut summary, termination);
    }

    // Merge stderr-derived diagnostics after both reader threads have
    // joined. The bridge accumulated counters into its own shared state
    // during streaming; the finalizer reads that state and enriches the
    // summary. `stderr_text` is attached here so every structured
    // bridge-enabled session carries the captured stderr regardless of
    // `suppress_stderr_on_success`. Badge recomputation happens inside
    // the finalizer so stderr-derived diagnostics show up in the final
    // `summary.badges` vector.
    if let Some(finalize) = finalize_for_main {
        if !captured.is_empty() && summary.stderr_text.is_none() {
            summary.stderr_text = Some(captured.clone());
        }
        finalize(&mut summary);
    }

    let first_response = resolve_first_response(
        *first_semantic_at.lock().unwrap(),
        *first_raw_stdout_at.lock().unwrap(),
        *first_stderr_at.lock().unwrap(),
        started_at,
    );

    Ok(ProcessResult {
        data: summary,
        termination,
        telemetry: ProcessTelemetry {
            total_elapsed: started_at.elapsed(),
            first_response_latency: first_response,
        },
    })
}

/// Resolve first-response latency from three observed timestamps.
///
/// Preferred precedence: semantic stdout > raw stdout > stderr.
fn resolve_first_response(
    semantic: Option<Instant>,
    raw_stdout: Option<Instant>,
    stderr: Option<Instant>,
    spawned_at: Instant,
) -> Option<Duration> {
    semantic
        .or(raw_stdout)
        .or(stderr)
        .map(|t| t.saturating_duration_since(spawned_at))
}

fn exit_code_from_status(status: ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }

    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return 128 + signal;
        }
    }

    1
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
            biscuit_terminal::prelude::strip_escape_codes(&String::from_utf8(out).expect("utf8"));

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
    fn apply_early_termination_rate_limit_sets_usage_limit_summary_fields() {
        use chrono::TimeZone;
        let reset_at = chrono::Utc
            .with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
            .unwrap();
        let mut summary = StreamExecutionSummary {
            exit_code: 143,
            is_error: false,
            ..Default::default()
        };
        let termination = EarlyTermination::RateLimit {
            message: "Usage limit reached; resets at 2026-04-16 04:18:56 UTC".into(),
            reset_at: Some(reset_at),
        };

        apply_early_termination_to_summary(&mut summary, &termination);

        assert_eq!(summary.exit_code, 1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("usage_limit_reached"));
        assert!(
            summary
                .error_message
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains("usage limit"),
        );
        let rl = summary.rate_limit.as_ref().expect("rate_limit populated");
        assert_eq!(rl.is_throttled, Some(true));
        assert_eq!(rl.reset_at, Some(reset_at));
        assert!(
            rl.message
                .as_deref()
                .unwrap_or("")
                .to_lowercase()
                .contains("usage limit")
        );
    }

    #[test]
    fn apply_early_termination_preserves_existing_rate_limit_fields() {
        use chrono::TimeZone;
        let existing_reset = chrono::Utc.with_ymd_and_hms(2026, 4, 16, 2, 0, 0).unwrap();
        let mut summary = StreamExecutionSummary {
            rate_limit: Some(claudine::stream::summary::RateLimitInfo {
                is_throttled: Some(false),
                retry_after_ms: Some(5000),
                message: Some("pre-existing".into()),
                reset_at: Some(existing_reset),
            }),
            ..Default::default()
        };
        let incoming_reset = chrono::Utc
            .with_ymd_and_hms(2026, 4, 16, 4, 18, 56)
            .unwrap();
        let termination = EarlyTermination::RateLimit {
            message: "Usage limit reached".into(),
            reset_at: Some(incoming_reset),
        };

        apply_early_termination_to_summary(&mut summary, &termination);

        let rl = summary.rate_limit.as_ref().unwrap();
        // is_throttled is forced to true even when existing said false.
        assert_eq!(rl.is_throttled, Some(true));
        // Existing message is preserved.
        assert_eq!(rl.message.as_deref(), Some("pre-existing"));
        // Existing reset_at is preserved (do not clobber parser-provided state).
        assert_eq!(rl.reset_at, Some(existing_reset));
        // retry_after_ms is untouched.
        assert_eq!(rl.retry_after_ms, Some(5000));
    }

    #[test]
    fn apply_early_termination_completed_but_hung_restores_success() {
        let mut summary = StreamExecutionSummary {
            exit_code: 143,
            is_error: true,
            error_kind: Some("agent_native".into()),
            error_message: Some("killed".into()),
            ..Default::default()
        };

        apply_early_termination_to_summary(
            &mut summary,
            &EarlyTermination::CompletedButHung {
                message: "OpenCode reported stop but stayed alive".into(),
            },
        );

        assert_eq!(summary.exit_code, 0);
        assert!(!summary.is_error);
        assert!(summary.error_kind.is_none());
        assert!(summary.error_message.is_none());
    }

    #[test]
    fn detect_opencode_hang_termination_recovers_after_stop_reason() {
        let metrics = claudine::stream::progress::new_live_metrics();
        let now = Instant::now();
        {
            let mut state = metrics.lock().unwrap();
            state.last_event_at = Some(now - Duration::from_secs(180));
            state.provider_status = Some("stop".into());
        }

        let detected = detect_opencode_hang_termination(&metrics, now, Duration::from_secs(120));

        assert!(matches!(
            detected,
            Some(EarlyTermination::CompletedButHung { .. })
        ));
    }

    #[test]
    fn detect_step_timeout_fires_after_silence_exceeds_budget() {
        let metrics = claudine::stream::progress::new_live_metrics();
        let now = Instant::now();
        {
            let mut state = metrics.lock().unwrap();
            state.last_event_at = Some(now - Duration::from_secs(6));
        }

        let detected = detect_step_timeout(&metrics, now, Duration::from_secs(5));

        assert!(matches!(
            detected,
            Some(EarlyTermination::StepTimeout { .. })
        ));
    }

    #[test]
    fn detect_step_timeout_returns_none_when_recent() {
        let metrics = claudine::stream::progress::new_live_metrics();
        let now = Instant::now();
        {
            let mut state = metrics.lock().unwrap();
            state.last_event_at = Some(now - Duration::from_secs(1));
        }

        let detected = detect_step_timeout(&metrics, now, Duration::from_secs(5));

        assert!(detected.is_none());
    }

    #[test]
    fn detect_step_timeout_returns_none_when_last_event_at_is_none() {
        // First-event grace: a fresh session with no observed SemanticEvent
        // must never trip the deadline, even if the budget is tiny.
        let metrics = claudine::stream::progress::new_live_metrics();
        let now = Instant::now();

        let detected = detect_step_timeout(&metrics, now, Duration::from_secs(1));

        assert!(detected.is_none());
    }

    #[test]
    fn early_termination_process_outcome_maps_step_timeout_to_timed_out() {
        let termination = EarlyTermination::StepTimeout {
            message: "no stream activity for 6s; terminating due to step_timeout".into(),
        };

        let outcome = early_termination_process_outcome(Some(&termination));

        assert_eq!(outcome, claudine::harness::ProcessTermination::TimedOut);
    }

    #[test]
    fn apply_early_termination_step_timeout_sets_step_timeout_error() {
        let mut summary = StreamExecutionSummary::default();

        apply_early_termination_to_summary(
            &mut summary,
            &EarlyTermination::StepTimeout {
                message: "no stream activity for 6s; terminating due to step_timeout".into(),
            },
        );

        assert_eq!(summary.exit_code, 1);
        assert!(summary.is_error);
        assert_eq!(summary.error_kind.as_deref(), Some("step_timeout"));
        assert!(
            summary
                .error_message
                .as_deref()
                .unwrap_or("")
                .contains("no stream activity"),
        );
    }

    #[test]
    fn first_response_preference_semantic_over_raw_over_stderr() {
        let spawned_at = Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        let semantic = Some(Instant::now());
        std::thread::sleep(Duration::from_millis(5));
        let raw = Some(Instant::now());
        std::thread::sleep(Duration::from_millis(5));
        let stderr = Some(Instant::now());

        let resolved = resolve_first_response(semantic, raw, stderr, spawned_at);
        assert!(resolved.is_some());
        // Semantic should win even though it is the earliest
        assert!(resolved.unwrap() >= Duration::from_millis(5));
        assert!(resolved.unwrap() < Duration::from_millis(15));
    }

    #[test]
    fn first_response_falls_back_to_raw_when_no_semantic() {
        let spawned_at = Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        let raw = Some(Instant::now());

        let resolved = resolve_first_response(None, raw, None, spawned_at);
        assert!(resolved.is_some());
        assert!(resolved.unwrap() >= Duration::from_millis(5));
    }

    #[test]
    fn first_response_falls_back_to_stderr_when_nothing_else() {
        let spawned_at = Instant::now();
        std::thread::sleep(Duration::from_millis(5));
        let stderr = Some(Instant::now());

        let resolved = resolve_first_response(None, None, stderr, spawned_at);
        assert!(resolved.is_some());
        assert!(resolved.unwrap() >= Duration::from_millis(5));
    }

    #[test]
    fn first_response_none_when_no_output_at_all() {
        let spawned_at = Instant::now();
        let resolved = resolve_first_response(None, None, None, spawned_at);
        assert!(resolved.is_none());
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
