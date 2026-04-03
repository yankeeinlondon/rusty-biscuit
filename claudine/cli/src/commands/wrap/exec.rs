use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, IsTerminal, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::Instant;

use biscuit_terminal::terminal::Terminal;
use claudine::stream::parser::{StreamChunk, StreamParseError, StreamParser};
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;

pub(crate) struct ChildIoOptions<'a> {
    pub(crate) stdout_noise_prefixes: &'a [&'a str],
    pub(crate) stderr_noise_prefixes: &'a [&'a str],
    pub(crate) stdin_seed: Option<&'a str>,
    /// After writing `stdin_seed`, keep the pipe open and relay bytes from
    /// `/dev/tty` so the child's TUI receives keyboard and mouse events.
    /// Without this, piped stdin causes orphaned mouse tracking escape
    /// sequences to echo as text in the terminal.
    pub(crate) relay_tty_after_seed: bool,
}

/// Result of a child process execution, enriched with termination info.
pub(crate) struct ProcessResult<T> {
    pub(crate) data: T,
    pub(crate) termination: claudine::harness::ProcessTermination,
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
            self.process_line(out, &line);
        }
    }

    /// Process a single complete line, accumulating into the block buffer
    /// and flushing when a block boundary is detected.
    fn process_line<W: Write>(&mut self, out: &mut W, line: &str) {
        let trimmed = line.trim();

        // Track code fence open/close (``` or ~~~)
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            self.block_buffer.push_str(line);
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
            self.block_buffer.push_str(line);
            return;
        }

        // Blank line outside a code fence = block boundary
        if trimmed.is_empty() {
            // Include the blank line so darkmatter sees proper paragraph spacing
            self.block_buffer.push_str(line);
            self.flush_block(out);
            return;
        }

        // Regular content — accumulate
        self.block_buffer.push_str(line);
    }

    /// Render the accumulated block through darkmatter and write to output.
    fn flush_block<W: Write>(&mut self, out: &mut W) {
        if self.block_buffer.is_empty() {
            return;
        }
        let block = std::mem::take(&mut self.block_buffer);
        self.render_markdown(out, &block);
    }

    /// Flush any remaining buffered content (incomplete line + block buffer).
    fn flush_remaining<W: Write>(&mut self, out: &mut W) {
        if !self.line_buffer.is_empty() {
            let leftover = std::mem::take(&mut self.line_buffer);
            self.block_buffer.push_str(&leftover);
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
            let _ = out.write_all(rendered.as_bytes());
        } else {
            let _ = out.write_all(text.as_bytes());
        }
        let _ = out.flush();
    }
}

/// Renders streamed thinking/reasoning text dimmed to stderr.
///
/// Accumulates thinking text and emits it via Prose dim+italic styling so it
/// is visually distinct from assistant text. A leading "Thinking..." label is
/// printed when the first thinking chunk arrives.
///
/// Writes to stderr using short-lived locks to avoid blocking the separate
/// stderr processing thread.
struct StreamThinkingRenderer {
    buffer: String,
    active: bool,
}

impl StreamThinkingRenderer {
    fn new() -> Self {
        Self {
            buffer: String::new(),
            active: false,
        }
    }

    fn push(&mut self, text: &str) {
        if !self.active {
            // Emit a dim+italic "Thinking..." header on first thinking chunk
            let header = Self::render_dim_italic("\u{27e1} Thinking...");
            eprintln!("{header}");
            self.active = true;
        }
        self.buffer.push_str(text);

        // Emit complete lines immediately (dimmed)
        while let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].to_string();
            self.buffer.drain(..=newline_pos);
            let rendered = Self::render_dim(&line);
            eprintln!("{rendered}");
        }
    }

    /// Flush remaining thinking text and reset state when switching to
    /// assistant text or finishing the stream.
    fn flush_if_active(&mut self) {
        if !self.active {
            return;
        }
        if !self.buffer.is_empty() {
            let remaining = std::mem::take(&mut self.buffer);
            let rendered = Self::render_dim(&remaining);
            eprintln!("{rendered}");
        }
        // Blank line to separate thinking from assistant text
        eprintln!();
        self.active = false;
    }

    fn render_dim(text: &str) -> String {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::Renderable;
        let safe = text.replace('<', "\\<");
        Prose::new(format!("<dim>{safe}</dim>")).render(&crate::log::terminal())
    }

    fn render_dim_italic(text: &str) -> String {
        use biscuit_terminal::components::prose::Prose;
        use biscuit_terminal::components::renderable::Renderable;
        let safe = text.replace('<', "\\<");
        Prose::new(format!("<dim><i>{safe}</i></dim>")).render(&crate::log::terminal())
    }
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

    let mut child = command.spawn()?;

    // Write stdin seed then either close the pipe (non-interactive) or
    // relay /dev/tty input through it (interactive TUI).
    let stdin_relay_handle = if let Some(seed) = io.stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        stdin_pipe.write_all(seed.as_bytes())?;
        if io.relay_tty_after_seed {
            // Keep the pipe open and forward terminal input so the child's
            // TUI receives keyboard/mouse events after the initial prompt.
            Some(thread::spawn(move || {
                use std::io::Read;
                let Ok(mut tty) = std::fs::File::open("/dev/tty") else {
                    return;
                };
                let mut buf = [0u8; 4096];
                loop {
                    match tty.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => {
                            if stdin_pipe.write_all(&buf[..n]).is_err() {
                                break;
                            }
                        }
                    }
                }
            }))
        } else {
            // Drop closes the pipe — child sees EOF.
            None
        }
    } else {
        None
    };

    // Spawn filter threads that read child output line-by-line and
    // suppress lines matching any noise prefix.
    let stdout_handle = if filter_stdout {
        let pipe = child.stdout.take().expect("stdout was set to piped");
        let prefixes: Vec<String> = io
            .stdout_noise_prefixes
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plain = crate::log::is_plain();
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut out = std::io::stdout().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
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
        let pipe = child.stderr.take().expect("stderr was set to piped");
        let prefixes: Vec<String> = io
            .stderr_noise_prefixes
            .iter()
            .map(|s| s.to_string())
            .collect();
        let plain = crate::log::is_plain();
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut err = std::io::stderr().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
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

    let (exit_code, termination) = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child)?
    };

    if let Some(handle) = stdout_handle {
        let _ = handle.join();
    }
    if let Some(handle) = stderr_handle {
        let _ = handle.join();
    }
    // The relay thread will exit when the child's stdin pipe breaks (child
    // exited) or /dev/tty EOF.  Don't block on join — the thread may be
    // stuck in a blocking read on /dev/tty.
    drop(stdin_relay_handle);

    Ok(ProcessResult {
        data: exit_code,
        termination,
    })
}

/// Wait for the child, forwarding SIGINT/SIGTERM on repeated Ctrl-C.
///
/// Returns `(exit_code, termination_kind)`.
#[cfg(unix)]
fn wait_with_signal_handling(
    child: &mut Child,
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU8, Ordering};

    let interrupt_count = Arc::new(AtomicU8::new(0));
    let child_pid = child.id();

    // Install a SIGINT handler that escalates on repeated presses.
    let counter = Arc::clone(&interrupt_count);
    let _guard = unsafe {
        signal_hook::low_level::register(signal_hook::consts::SIGINT, move || {
            let count = counter.fetch_add(1, Ordering::SeqCst) + 1;
            match count {
                1 => {
                    // First Ctrl-C: forward SIGINT to the child process.
                    // Registering this handler replaced the default behavior
                    // (which would propagate to the process group), so we
                    // must explicitly forward the signal.
                    libc::kill(child_pid as i32, libc::SIGINT);
                }
                2 => {
                    // Second Ctrl-C: escalate to SIGTERM
                    libc::kill(child_pid as i32, libc::SIGTERM);
                }
                _ => {
                    // Third+ Ctrl-C: force kill
                    libc::kill(child_pid as i32, libc::SIGKILL);
                }
            }
        })
    }?;

    let status = child.wait()?;
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
) -> Result<(i32, claudine::harness::ProcessTermination)> {
    let status = child.wait()?;
    Ok((
        exit_code_from_status(status),
        claudine::harness::ProcessTermination::Completed,
    ))
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

    let mut child = command.spawn()?;

    // Write stdin seed and close the pipe so the child sees EOF.
    if let Some(seed) = io.stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        stdin_pipe.write_all(seed.as_bytes())?;
    }

    // Capture stdout into a string, applying noise filtering
    let stdout_pipe = child.stdout.take().expect("stdout was set to piped");
    let stdout_noise: Vec<String> = io
        .stdout_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if stdout_noise.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    // Capture stderr into a string, applying noise filtering
    let stderr_pipe = child.stderr.take().expect("stderr was set to piped");
    let stderr_noise: Vec<String> = io
        .stderr_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if stderr_noise.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    let (exit_code, termination) = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child)?
    };

    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();

    Ok(ProcessResult {
        data: CapturedChildOutput {
            exit_code,
            stdout,
            stderr,
        },
        termination,
    })
}

/// Spawn a provider child process with structured stream parsing.
///
/// Stdout is piped through the provider's stream parser. Parsed
/// assistant text is written to the real stdout. Metadata accumulates
/// in the parser state. Stderr is forwarded normally (with noise filtering).
///
/// Returns the stream execution summary (which includes exit code).
#[allow(clippy::too_many_arguments)]
pub(crate) fn run_child_stream(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    stderr_noise_prefixes: &[&str],
    suppress_stderr_on_success: bool,
    stdin_seed: Option<&str>,
    parser: Box<dyn StreamParser>,
) -> Result<ProcessResult<StreamExecutionSummary>> {
    debug_assert!(env.contains_key(&OsString::from("PATH")));
    debug_assert!(env.contains_key(&OsString::from("HOME")));

    let needs_stdin_pipe = stdin_seed.is_some();
    let started_at = Instant::now();

    // Always pipe stderr in structured stream mode so we can intercept and
    // format raw API error JSON into human-readable messages.
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
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn()?;

    // Write stdin seed and close the pipe so the child sees EOF.
    if let Some(seed) = stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        stdin_pipe.write_all(seed.as_bytes())?;
    }

    // Pipe stdout through the stream parser
    let stdout_pipe = child.stdout.take().expect("stdout was set to piped");
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_pipe);
        let mut out = std::io::stdout().lock();
        let mut parser = parser;
        let mut fallback_mode = false;
        let mut renderer = StreamTextRenderer::new();
        let mut thinking_renderer = StreamThinkingRenderer::new();

        for line in reader.lines() {
            let Ok(line) = line else { break };

            if fallback_mode {
                // Fatal parse error: forward remaining raw stdout
                let _ = writeln!(out, "{}", crate::log::maybe_strip(&line));
                continue;
            }

            match parser.feed_line(&line) {
                Ok(Some(StreamChunk::Text(text))) => {
                    thinking_renderer.flush_if_active();
                    renderer.push(&mut out, &text);
                }
                Ok(Some(StreamChunk::Thinking(text))) => {
                    thinking_renderer.push(&text);
                }
                Ok(None) => {
                    // Metadata-only line
                }
                Err(StreamParseError::MalformedLine { .. }) => {
                    // Silently skip — providers (especially Gemini) mix
                    // non-JSON noise (stack traces, hook logs) into stdout.
                    tracing::debug!("skipping malformed stream line: {line}");
                }
                Err(StreamParseError::Fatal(_)) => {
                    // Fall back to raw forwarding
                    thinking_renderer.flush_if_active();
                    renderer.flush_remaining(&mut out);
                    fallback_mode = true;
                    let _ = writeln!(out, "{}", crate::log::maybe_strip(&line));
                }
            }
        }

        thinking_renderer.flush_if_active();
        renderer.flush_remaining(&mut out);
        parser
    });

    // Stderr processing thread: filters noise prefixes and formats raw API errors.
    let pipe = child.stderr.take().expect("stderr was set to piped");
    let prefixes: Vec<String> = stderr_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let plain = crate::log::is_plain();
    let stderr_term = crate::log::terminal();
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }
            // Format raw API error JSON into human-readable messages
            let formatted = crate::output::try_format_api_error(&line, &stderr_term);
            let output_line = formatted.as_deref().unwrap_or(&line);
            let output_line = if plain {
                biscuit_terminal::prelude::strip_escape_codes(output_line)
            } else {
                output_line.to_string()
            };

            if suppress_stderr_on_success {
                if !captured.is_empty() {
                    captured.push('\n');
                }
                captured.push_str(&output_line);
            } else {
                let mut err = std::io::stderr().lock();
                let _ = writeln!(err, "{output_line}");
            }
        }
        captured
    });

    let (exit_code, termination) = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child)?
    };

    let parser = stdout_handle.join().unwrap_or_else(|_| {
        // If the thread panicked, create a minimal error summary
        Box::new(ErrorParser { exit_code })
    });

    let captured = stderr_handle.join().unwrap_or_default();
    if suppress_stderr_on_success && exit_code != 0 && !captured.is_empty() {
        eprintln!("{captured}");
    }

    let mut summary = parser.finish(exit_code);
    if summary.duration_ms.is_none() {
        summary.duration_ms = Some(started_at.elapsed().as_millis() as u64);
    }

    Ok(ProcessResult {
        data: summary,
        termination,
    })
}

/// Spawn a provider child process with structured stream parsing, capturing output.
///
/// Like `run_child_stream` but captures assistant text instead of printing.
/// Used by compose flows.
#[allow(dead_code, clippy::too_many_arguments)]
pub(crate) fn run_child_stream_capture(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    stderr_noise_prefixes: &[&str],
    stdin_seed: Option<&str>,
    parser: Box<dyn StreamParser>,
) -> Result<ProcessResult<StreamExecutionSummary>> {
    debug_assert!(env.contains_key(&OsString::from("PATH")));
    debug_assert!(env.contains_key(&OsString::from("HOME")));

    let needs_stdin_pipe = stdin_seed.is_some();
    let started_at = Instant::now();

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

    let mut child = command.spawn()?;

    if let Some(seed) = stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        stdin_pipe.write_all(seed.as_bytes())?;
    }

    let stdout_pipe = child.stdout.take().expect("stdout was set to piped");
    let stdout_handle = thread::spawn(move || {
        let reader = BufReader::new(stdout_pipe);
        let mut parser = parser;

        for line in reader.lines() {
            let Ok(line) = line else { break };
            // Feed all lines; text accumulates in parser.assistant_text
            let _ = parser.feed_line(&line);
        }

        parser
    });

    let stderr_pipe = child.stderr.take().expect("stderr was set to piped");
    let stderr_noise: Vec<String> = stderr_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(stderr_pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if stderr_noise
                .iter()
                .any(|prefix| line.starts_with(prefix.as_str()))
            {
                continue;
            }
            if !captured.is_empty() {
                captured.push('\n');
            }
            captured.push_str(&line);
        }
        captured
    });

    let (exit_code, termination) = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child)?
    };

    let parser = stdout_handle
        .join()
        .unwrap_or_else(|_| Box::new(ErrorParser { exit_code }));
    let stderr_text = stderr_handle.join().unwrap_or_default();

    let mut summary = parser.finish(exit_code);
    if summary.duration_ms.is_none() {
        summary.duration_ms = Some(started_at.elapsed().as_millis() as u64);
    }
    if !stderr_text.trim().is_empty() {
        if summary.error_message.is_none() && exit_code != 0 {
            summary.error_message = Some(stderr_text.lines().next().unwrap_or("").to_string());
            summary.is_error = true;
        }
        summary.stderr_text = Some(stderr_text);
    }

    Ok(ProcessResult {
        data: summary,
        termination,
    })
}

/// Minimal fallback parser used when the real parser thread panics.
struct ErrorParser {
    exit_code: i32,
}

impl StreamParser for ErrorParser {
    fn feed_line(
        &mut self,
        _line: &str,
    ) -> std::result::Result<Option<StreamChunk>, StreamParseError> {
        Ok(None)
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

    #[cfg(unix)]
    fn current_env() -> HashMap<OsString, OsString> {
        std::env::vars_os().collect()
    }

    #[cfg(unix)]
    #[test]
    fn structured_capture_drains_stderr_and_preserves_diagnostics() {
        let env = current_env();
        let cwd = std::env::current_dir().unwrap();
        let parser = claudine::stream::create_parser(
            claudine::events::Provider::Claude,
            claudine::stream::parser::NullSink,
            claudine::stream::ParserConfig::default(),
        );
        let script = r#"
i=0
while [ "$i" -lt 20000 ]; do
  echo "provider stderr line $i" >&2
  i=$((i + 1))
done
printf '%s\n' '{"type":"init","session_id":"sess-1","model":"claude-sonnet"}'
printf '%s\n' '{"type":"assistant","content":[{"type":"text","text":"hello"}]}'
printf '%s\n' '{"type":"result","duration_ms":25}'
"#;
        let args = vec!["-c".to_string(), script.to_string()];

        let result = run_child_stream_capture(
            Path::new("/bin/sh"),
            &args,
            &env,
            &cwd,
            Some(5),
            &[],
            None,
            parser,
        )
        .unwrap();
        let summary = result.data;

        assert_eq!(summary.exit_code, 0);
        assert_eq!(summary.assistant_text, "hello");
        assert!(summary.stderr_text.is_some());
        assert!(
            summary
                .stderr_text
                .as_deref()
                .unwrap()
                .contains("provider stderr line")
        );
    }

    #[cfg(unix)]
    #[test]
    fn structured_stream_falls_back_to_wall_clock_duration_when_missing() {
        let env = current_env();
        let cwd = std::env::current_dir().unwrap();
        let parser = claudine::stream::create_parser(
            claudine::events::Provider::OpenCode,
            claudine::stream::parser::NullSink,
            claudine::stream::ParserConfig {
                model: Some("minimax/MiniMax-M2.5-highspeed".into()),
            },
        );
        let script = r#"
printf '%s\n' '{"type":"session_start","model":"minimax/MiniMax-M2.5-highspeed"}'
printf '%s\n' '{"type":"step_start","sessionID":"ses_1"}'
printf '%s\n' '{"type":"text","text":"hello"}'
printf '%s\n' '{"type":"step_finish","part":{"reason":"stop","cost":0.02,"tokens":{"input":150,"output":101,"total":251,"cache":{"read":42}}}}'
"#;
        let args = vec!["-c".to_string(), script.to_string()];

        let result = run_child_stream(
            Path::new("/bin/sh"),
            &args,
            &env,
            &cwd,
            Some(5),
            &[],
            false,
            None,
            parser,
        )
        .unwrap();
        let summary = result.data;

        assert_eq!(summary.exit_code, 0);
        assert_eq!(summary.assistant_text, "hello");
        assert!(summary.duration_ms.is_some());
        assert!(summary.duration_ms.unwrap() < 5_000);
    }

    #[test]
    fn stream_text_renderer_flushes_on_blank_line() {
        let mut renderer = StreamTextRenderer {
            block_buffer: String::new(),
            line_buffer: String::new(),
            in_code_fence: false,
            term: None,
            terminal_options: None,
        };
        let mut out = Vec::new();

        renderer.push(&mut out, "First paragraph.\n\nSecond");
        let flushed = String::from_utf8(out).unwrap();

        assert!(flushed.contains("First paragraph."));
        // "Second" has no newline yet — sits in line_buffer
        assert_eq!(renderer.line_buffer, "Second");
        assert!(renderer.block_buffer.is_empty());
    }

    #[test]
    fn stream_text_renderer_buffers_code_fence() {
        let mut renderer = StreamTextRenderer {
            block_buffer: String::new(),
            line_buffer: String::new(),
            in_code_fence: false,
            term: None,
            terminal_options: None,
        };
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
    fn stream_text_renderer_flush_remaining_drains_everything() {
        let mut renderer = StreamTextRenderer {
            block_buffer: String::new(),
            line_buffer: String::new(),
            in_code_fence: false,
            term: None,
            terminal_options: None,
        };
        let mut out = Vec::new();

        renderer.push(&mut out, "trailing text without newline");
        assert!(out.is_empty());

        renderer.flush_remaining(&mut out);
        let flushed = String::from_utf8(out).unwrap();
        assert_eq!(flushed, "trailing text without newline");
    }
}
