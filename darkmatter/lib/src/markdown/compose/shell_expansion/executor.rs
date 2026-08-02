//! Command execution for shell directives.
//!
//! This module provides safe command execution with timeout protection,
//! working directory resolution, and stdout/stderr capture.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Command, ExitStatus};
use std::sync::Arc;
use std::sync::mpsc::RecvTimeoutError;
use std::thread::JoinHandle;
use std::time::Duration;

use shared_child::SharedChild;
use tracing::{debug, instrument, warn};

use super::types::{
    ChainOperator, CommandAction, RedirectionConfig, ShellCommandOrigin, ShellDirective,
    ShellExpansionError, ShellExpansionOptions, ShellPipeline, ShellTimeoutBehavior, StderrTarget,
    StdoutTarget,
};
use crate::markdown::compose::ComposeSource;

/// Detailed output from a successful shell command execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CommandExecution {
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Present when the command timed out but timeout fallback converted it to
    /// an empty-string result.
    pub timeout_fallback: Option<std::time::Duration>,
}

impl CommandExecution {
    pub(crate) fn from_streams(stdout: String, stderr: String) -> Self {
        Self {
            stdout,
            stderr,
            timeout_fallback: None,
        }
    }

    fn timeout_fallback(timeout: std::time::Duration) -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            timeout_fallback: Some(timeout),
        }
    }

    /// Returns stdout and stderr combined using the body-shell contract.
    pub fn combined_output(&self) -> String {
        let mut output = self.stdout.clone();
        if !self.stderr.is_empty() {
            if !output.is_empty() {
                output.push('\n');
            }
            output.push_str(&self.stderr);
        }
        output
    }
}

/// Resolves the working directory for command execution.
///
/// Resolution order:
/// 1. `options.shell.working_directory` if set
/// 2. Source file's parent directory if `ComposeSource::File`
/// 3. `options.shell.policy_root` if set
/// 4. `std::env::current_dir()`
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::shell_expansion::executor::resolve_working_directory;
/// use darkmatter::markdown::compose::shell_expansion::types::ShellExpansionOptions;
/// use darkmatter::markdown::compose::ComposeSource;
/// use std::path::PathBuf;
///
/// let options = ShellExpansionOptions::default();
/// let source = ComposeSource::Unknown;
/// let working_dir = resolve_working_directory(&options, &source);
/// // Returns current directory
/// ```
#[instrument(skip_all)]
pub fn resolve_working_directory(
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> PathBuf {
    if let Some(ref wd) = shell_opts.working_directory {
        return wd.clone();
    }
    if let ComposeSource::File(path) = source
        && let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        return parent.to_path_buf();
    }
    if let Some(ref root) = shell_opts.policy_root {
        return root.clone();
    }
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}

/// Executes a shell directive command with timeout and output capture.
///
/// ## Returns
///
/// Combined stdout and stderr output if the command succeeds (exit code 0).
///
/// ## Errors
///
/// - `CommandNotFound` if the executable doesn't exist in PATH
/// - `ExecutionFailed` if the command exits with non-zero status
/// - `Timeout` if the command exceeds the configured timeout
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::errors::SourceContext;
/// use darkmatter::markdown::compose::shell_expansion::executor::execute_command;
/// use darkmatter::markdown::compose::shell_expansion::types::{ErrorHandling, ShellCommandOrigin, ShellDirective, ShellExpansionOptions};
/// use darkmatter::markdown::compose::ComposeSource;
/// use std::path::PathBuf;
///
/// let ctx = SourceContext::new(PathBuf::from("/t"), PathBuf::from("t"), "");
/// let directive = ShellDirective {
///     raw_command: "echo hello".to_string(),
///     executable: "echo".to_string(),
///     args: vec!["hello".to_string()],
///     span: 0..10,
///     indent: String::new(),
///     origin: ShellCommandOrigin::Body { line: 1 },
///     error_handling: ErrorHandling::default(),
///     timeout_override: None,
///     no_cache: false,
///     pipeline: None,
///     ctx,
/// };
/// let options = ShellExpansionOptions::default();
/// let source = ComposeSource::Unknown;
/// let output = execute_command(&directive, &options, &source).unwrap();
/// assert!(output.contains("hello"));
/// ```
#[instrument(skip_all, fields(
    command = %directive.raw_command,
    executable = %directive.executable,
    line = directive.origin.line_number(),
))]
pub fn execute_command(
    directive: &ShellDirective,
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> Result<String, ShellExpansionError> {
    Ok(execute_command_detailed(directive, shell_opts, source)?.combined_output())
}

/// Executes a shell directive command with timeout and output capture.
///
/// Returns stdout/stderr separately so callers can implement different output
/// contracts for body and frontmatter shell expansion.
pub(crate) fn execute_command_detailed(
    directive: &ShellDirective,
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> Result<CommandExecution, ShellExpansionError> {
    // If there's a pipeline with redirections, use the action-based path
    if let Some(ref pipeline) = directive.pipeline
        && let Some(action) = pipeline.actions.first()
        && action.command.redirection != RedirectionConfig::default()
    {
        let working_dir = resolve_working_directory(shell_opts, source);
        let timeout = directive.timeout_override.unwrap_or(shell_opts.timeout);
        return execute_single_action(
            &action.command,
            &working_dir,
            timeout,
            shell_opts,
            &directive.raw_command,
            &directive.origin,
            &directive.ctx,
        );
    }

    // Standard single-command path (no redirections)
    let resolved_path =
        which::which(&directive.executable).map_err(|_| ShellExpansionError::CommandNotFound {
            ctx: Box::new(directive.ctx.clone()),
            command: directive.executable.clone(),
            origin: directive.origin.clone(),
        })?;

    let working_dir = resolve_working_directory(shell_opts, source);
    debug!(working_dir = %working_dir.display(), "shell: executing command");

    // 3. Build command
    let mut cmd = Command::new(&resolved_path);
    cmd.args(&directive.args)
        .current_dir(&working_dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    if shell_opts.strip_ansi {
        cmd.env("NO_COLOR", "1");
    }

    // 4. Spawn
    let child = SharedChild::spawn(&mut cmd)
        .map(Arc::new)
        .map_err(|e| ShellExpansionError::ExecutionFailed {
            ctx: Box::new(directive.ctx.clone()),
            command: directive.raw_command.clone(),
            code: -1,
            stdout: String::new(),
            stderr: e.to_string(),
            origin: directive.origin.clone(),
        })?;

    // 5. Drain stdout and stderr concurrently via threads
    let stdout_handle = child.take_stdout();
    let stderr_handle = child.take_stderr();

    let stdout_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stdout) = stdout_handle {
            let _ = stdout.read_to_end(&mut buf);
        }
        buf
    });

    let stderr_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        if let Some(mut stderr) = stderr_handle {
            let _ = stderr.read_to_end(&mut buf);
        }
        buf
    });

    // 6. Wait with timeout
    let timeout = directive.timeout_override.unwrap_or(shell_opts.timeout);

    let status = match wait_with_timeout(&child, timeout) {
        Ok(WaitOutcome::Exited(status)) => status,
        Ok(WaitOutcome::TimedOut) => {
            warn!(?timeout, "shell: command timed out");
            match shell_opts.timeout_behavior {
                ShellTimeoutBehavior::Error => {
                    return Err(ShellExpansionError::Timeout {
                        ctx: Box::new(directive.ctx.clone()),
                        command: directive.raw_command.clone(),
                        timeout,
                        origin: directive.origin.clone(),
                    });
                }
                ShellTimeoutBehavior::EmptyString => {
                    return Ok(CommandExecution::timeout_fallback(timeout));
                }
            }
        }
        Err(e) => {
            return Err(ShellExpansionError::ExecutionFailed {
                ctx: Box::new(directive.ctx.clone()),
                command: directive.raw_command.clone(),
                code: -1,
                stdout: String::new(),
                stderr: e.to_string(),
                origin: directive.origin.clone(),
            });
        }
    };

    let stdout_bytes = join_output_thread_raw(stdout_thread, "stdout", &directive.ctx)?;
    let stderr_bytes = join_output_thread_raw(stderr_thread, "stderr", &directive.ctx)?;
    let mut stdout = String::from_utf8_lossy(&stdout_bytes).to_string();
    let mut stderr = String::from_utf8_lossy(&stderr_bytes).to_string();

    if shell_opts.strip_ansi {
        stdout = biscuit_terminal::prelude::strip_escape_codes(stdout);
        stderr = biscuit_terminal::prelude::strip_escape_codes(stderr);
    }

    if status.success() {
        let output = CommandExecution::from_streams(stdout, stderr);
        debug!(
            exit_code = 0,
            output_len = output.combined_output().len(),
            "shell: command succeeded"
        );
        Ok(output)
    } else {
        Err(ShellExpansionError::ExecutionFailed {
            ctx: Box::new(directive.ctx.clone()),
            command: directive.raw_command.clone(),
            code: status.code().unwrap_or(-1),
            stdout,
            stderr,
            origin: directive.origin.clone(),
        })
    }
}

/// Executes a shell directive, dispatching to pipeline execution if the
/// directive contains a chain, otherwise to single-command execution.
pub(crate) fn execute_directive_impl(
    directive: &ShellDirective,
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> Result<CommandExecution, ShellExpansionError> {
    if let Some(ref pipeline) = directive.pipeline
        && pipeline.actions.len() > 1
    {
        return execute_pipeline_detailed(directive, pipeline, shell_opts, source);
    }
    execute_command_detailed(directive, shell_opts, source)
}

/// Executes a pipeline of chained commands with per-command redirections.
fn execute_pipeline_detailed(
    directive: &ShellDirective,
    pipeline: &ShellPipeline,
    shell_opts: &ShellExpansionOptions,
    source: &ComposeSource,
) -> Result<CommandExecution, ShellExpansionError> {
    let working_dir = resolve_working_directory(shell_opts, source);
    let timeout = directive.timeout_override.unwrap_or(shell_opts.timeout);

    let mut combined_stdout = String::new();
    let mut combined_stderr = String::new();
    let mut last_success = true;
    let mut timeout_fallback = None;

    for action in &pipeline.actions {
        // Check chain condition
        match action.operator {
            ChainOperator::None => {}
            ChainOperator::And => {
                if !last_success {
                    continue;
                }
            }
            ChainOperator::Or => {
                if last_success {
                    continue;
                }
            }
        }

        let result = execute_single_action(
            &action.command,
            &working_dir,
            timeout,
            shell_opts,
            &directive.raw_command,
            &directive.origin,
            &directive.ctx,
        );

        match result {
            Ok(exec) => {
                last_success = true;
                timeout_fallback = timeout_fallback.or(exec.timeout_fallback);
                if !exec.stdout.is_empty() {
                    if !combined_stdout.is_empty() {
                        combined_stdout.push('\n');
                    }
                    combined_stdout.push_str(&exec.stdout);
                }
                if !exec.stderr.is_empty() {
                    if !combined_stderr.is_empty() {
                        combined_stderr.push('\n');
                    }
                    combined_stderr.push_str(&exec.stderr);
                }
            }
            Err(ShellExpansionError::ExecutionFailed {
                code,
                stdout,
                stderr,
                ..
            }) => {
                last_success = false;
                if !stdout.is_empty() {
                    if !combined_stdout.is_empty() {
                        combined_stdout.push('\n');
                    }
                    combined_stdout.push_str(&stdout);
                }
                if !stderr.is_empty() {
                    if !combined_stderr.is_empty() {
                        combined_stderr.push('\n');
                    }
                    combined_stderr.push_str(&stderr);
                }
                // If this is the last action (or no subsequent Or handler), propagate failure
                let is_last = std::ptr::eq(action, &pipeline.actions[pipeline.actions.len() - 1]);
                // Check if next action handles failure with ||
                let next_handles_failure = pipeline
                    .actions
                    .iter()
                    .position(|a| std::ptr::eq(a, action))
                    .map(|idx| {
                        idx + 1 < pipeline.actions.len()
                            && pipeline.actions[idx + 1].operator == ChainOperator::Or
                    })
                    .unwrap_or(false);

                if is_last || !next_handles_failure {
                    // Check if any remaining actions could handle the failure
                    let pos = pipeline
                        .actions
                        .iter()
                        .position(|a| std::ptr::eq(a, action))
                        .unwrap();
                    let any_or_handler = pipeline.actions[pos + 1..]
                        .iter()
                        .any(|a| a.operator == ChainOperator::Or);

                    if !any_or_handler {
                        return Err(ShellExpansionError::ExecutionFailed {
                            ctx: Box::new(directive.ctx.clone()),
                            command: directive.raw_command.clone(),
                            code,
                            stdout: combined_stdout,
                            stderr: combined_stderr,
                            origin: directive.origin.clone(),
                        });
                    }
                }
            }
            Err(ShellExpansionError::Timeout { .. }) => {
                return Err(ShellExpansionError::Timeout {
                    ctx: Box::new(directive.ctx.clone()),
                    command: directive.raw_command.clone(),
                    timeout,
                    origin: directive.origin.clone(),
                });
            }
            Err(e) => return Err(e),
        }
    }

    let mut execution = CommandExecution::from_streams(combined_stdout, combined_stderr);
    execution.timeout_fallback = timeout_fallback;
    Ok(execution)
}

/// Executes a single command action with its redirection config.
///
/// For `2>&1` and `>&2` redirections, both child streams are wired to a single
/// OS pipe (via `std::io::pipe`) before spawning so that emission order is
/// preserved by the kernel rather than reconstructed after exit.
fn execute_single_action(
    action: &CommandAction,
    working_dir: &std::path::Path,
    timeout: std::time::Duration,
    shell_opts: &ShellExpansionOptions,
    raw_command: &str,
    origin: &super::types::ShellCommandOrigin,
    ctx: &biscuit_terminal::errors::SourceContext,
) -> Result<CommandExecution, ShellExpansionError> {
    let resolved_path =
        which::which(&action.executable).map_err(|_| ShellExpansionError::CommandNotFound {
            ctx: Box::new(ctx.clone()),
            command: action.executable.clone(),
            origin: origin.clone(),
        })?;

    let mut cmd = Command::new(&resolved_path);
    cmd.args(&action.args)
        .current_dir(working_dir)
        .stdin(std::process::Stdio::null());

    if shell_opts.strip_ansi {
        cmd.env("NO_COLOR", "1");
    }

    let capture = configure_streams(&mut cmd, &action.redirection).map_err(|e| {
        ShellExpansionError::ExecutionFailed {
            ctx: Box::new(ctx.clone()),
            command: raw_command.to_string(),
            code: -1,
            stdout: String::new(),
            stderr: format!("failed to create stream pipe: {e}"),
            origin: origin.clone(),
        }
    })?;

    let child = SharedChild::spawn(&mut cmd)
        .map(Arc::new)
        .map_err(|e| ShellExpansionError::ExecutionFailed {
            ctx: Box::new(ctx.clone()),
            command: raw_command.to_string(),
            code: -1,
            stdout: String::new(),
            stderr: e.to_string(),
            origin: origin.clone(),
        })?;

    // `Command::spawn` takes `&mut self` and keeps any parent-owned `Stdio`s
    // (like our merged pipe writers) alive inside `cmd` until it is dropped.
    // If we do not drop `cmd` here, the parent retains its own copies of the
    // pipe writers and the merged reader will never see EOF.
    drop(cmd);

    let CaptureHandles {
        merged_reader,
        merge_target,
    } = capture;

    let read_strategy = match merged_reader {
        Some(reader) => {
            let merged_thread = std::thread::spawn(move || {
                let mut buf = Vec::new();
                let mut reader = reader;
                let _ = reader.read_to_end(&mut buf);
                buf
            });
            ReadStrategy::Merged {
                thread: merged_thread,
                target: merge_target,
            }
        }
        None => {
            let stdout_handle = child.take_stdout();
            let stderr_handle = child.take_stderr();

            let stdout_thread = std::thread::spawn(move || {
                let mut buf = Vec::new();
                if let Some(mut stdout) = stdout_handle {
                    let _ = stdout.read_to_end(&mut buf);
                }
                buf
            });

            let stderr_thread = std::thread::spawn(move || {
                let mut buf = Vec::new();
                if let Some(mut stderr) = stderr_handle {
                    let _ = stderr.read_to_end(&mut buf);
                }
                buf
            });

            ReadStrategy::Separate {
                stdout: stdout_thread,
                stderr: stderr_thread,
            }
        }
    };

    let status = match wait_with_timeout(&child, timeout) {
        Ok(WaitOutcome::Exited(status)) => status,
        Ok(WaitOutcome::TimedOut) => {
            warn!(?timeout, "shell: command timed out");
            match shell_opts.timeout_behavior {
                ShellTimeoutBehavior::Error => {
                    return Err(ShellExpansionError::Timeout {
                        ctx: Box::new(ctx.clone()),
                        command: raw_command.to_string(),
                        timeout,
                        origin: origin.clone(),
                    });
                }
                ShellTimeoutBehavior::EmptyString => {
                    return Ok(CommandExecution::timeout_fallback(timeout));
                }
            }
        }
        Err(e) => {
            return Err(ShellExpansionError::ExecutionFailed {
                ctx: Box::new(ctx.clone()),
                command: raw_command.to_string(),
                code: -1,
                stdout: String::new(),
                stderr: e.to_string(),
                origin: origin.clone(),
            });
        }
    };

    let (mut final_stdout, mut final_stderr) = match read_strategy {
        ReadStrategy::Merged { thread, target } => {
            let bytes = join_output_thread_raw(thread, "merged", ctx)?;
            let merged = String::from_utf8_lossy(&bytes).to_string();
            match target {
                MergeTarget::Stdout => (merged, String::new()),
                MergeTarget::Stderr => (String::new(), merged),
            }
        }
        ReadStrategy::Separate { stdout, stderr } => {
            let stdout_bytes = join_output_thread_raw(stdout, "stdout", ctx)?;
            let stderr_bytes = join_output_thread_raw(stderr, "stderr", ctx)?;
            (
                String::from_utf8_lossy(&stdout_bytes).to_string(),
                String::from_utf8_lossy(&stderr_bytes).to_string(),
            )
        }
    };

    if shell_opts.strip_ansi {
        final_stdout = biscuit_terminal::prelude::strip_escape_codes(final_stdout);
        final_stderr = biscuit_terminal::prelude::strip_escape_codes(final_stderr);
    }

    if status.success() {
        Ok(CommandExecution::from_streams(final_stdout, final_stderr))
    } else {
        Err(ShellExpansionError::ExecutionFailed {
            ctx: Box::new(ctx.clone()),
            command: raw_command.to_string(),
            code: status.code().unwrap_or(-1),
            stdout: final_stdout,
            stderr: final_stderr,
            origin: origin.clone(),
        })
    }
}

/// Identifies which output field (stdout or stderr) the merged stream's bytes
/// should populate after the child exits.
#[derive(Debug, Clone, Copy)]
enum MergeTarget {
    Stdout,
    Stderr,
}

/// How captured bytes will be read after the child is spawned.
enum ReadStrategy {
    Merged {
        thread: JoinHandle<Vec<u8>>,
        target: MergeTarget,
    },
    Separate {
        stdout: JoinHandle<Vec<u8>>,
        stderr: JoinHandle<Vec<u8>>,
    },
}

/// Capture configuration returned by [`configure_streams`].
struct CaptureHandles {
    /// When the redirection merges streams at the OS level, the read end of
    /// the shared pipe lives here. The writer copies are owned by the
    /// `Command` and dropped after spawn.
    merged_reader: Option<std::io::PipeReader>,
    /// Where the merged bytes should land in the final result.
    /// Ignored when `merged_reader` is `None`.
    merge_target: MergeTarget,
}

/// Configures the child's stdout and stderr based on the redirection.
///
/// For `2>&1` and `>&2` we create a single pipe and wire both child streams
/// to it so the OS preserves emission order. For all other redirections we
/// fall back to separate `Stdio::piped()` / `Stdio::null()` channels.
fn configure_streams(
    cmd: &mut Command,
    redir: &RedirectionConfig,
) -> std::io::Result<CaptureHandles> {
    let merge = match (redir.stdout, redir.stderr) {
        (StdoutTarget::ToStderr, _) => Some(MergeTarget::Stderr),
        (_, StderrTarget::ToStdout) => Some(MergeTarget::Stdout),
        _ => None,
    };

    if let Some(target) = merge {
        let (reader, writer) = std::io::pipe()?;
        let writer_clone = writer.try_clone()?;
        cmd.stdout(writer);
        cmd.stderr(writer_clone);
        return Ok(CaptureHandles {
            merged_reader: Some(reader),
            merge_target: target,
        });
    }

    match redir.stdout {
        StdoutTarget::Capture => {
            cmd.stdout(std::process::Stdio::piped());
        }
        StdoutTarget::Null => {
            cmd.stdout(std::process::Stdio::null());
        }
        StdoutTarget::ToStderr => unreachable!("handled by merge branch"),
    }

    match redir.stderr {
        StderrTarget::Capture => {
            cmd.stderr(std::process::Stdio::piped());
        }
        StderrTarget::Null => {
            cmd.stderr(std::process::Stdio::null());
        }
        StderrTarget::ToStdout => unreachable!("handled by merge branch"),
    }

    Ok(CaptureHandles {
        merged_reader: None,
        merge_target: MergeTarget::Stdout,
    })
}

/// Why a [`wait_with_timeout`] call stopped waiting.
pub(super) enum WaitOutcome {
    Exited(ExitStatus),
    TimedOut,
}

/// Test-only record of the blocking wait spans [`recv_wait`] requested.
///
/// Exists because elapsed wall-clock time cannot distinguish a single blocking
/// wait from the 10ms poll loop it replaced (Finding 17): a 10ms tick is far
/// inside any bound that stays non-flaky on a loaded host. The span *shape* can
/// — one span sized to the caller's whole budget is only producible by a
/// blocking wait.
///
/// Thread-local: `wait_with_timeout` blocks on its caller's thread, so spans
/// never mix between concurrent tests in one process.
#[cfg(test)]
mod wait_probe {
    use std::cell::RefCell;
    use std::time::Duration;

    thread_local! {
        static SPANS: RefCell<Vec<Duration>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record(span: Duration) {
        SPANS.with(|spans| spans.borrow_mut().push(span));
    }

    /// Drains this thread's spans, returning those recorded since the last call.
    pub(super) fn take() -> Vec<Duration> {
        SPANS.with(|spans| std::mem::take(&mut *spans.borrow_mut()))
    }
}

/// Performs the one blocking wait span, reporting its length to [`wait_probe`].
///
/// Wrapping `recv_timeout` rather than calling it inline is deliberate: it gives
/// the tests a seam that observes *how long the implementation asked to block
/// for*, which is the signal that separates one full-budget blocking wait from a
/// poll loop. A loop calling this with a short span records many short spans; a
/// `try_wait` + `sleep` loop bypasses it and records none. Both fail the
/// exactly-one-full-budget-span assertion.
fn recv_wait(
    rx: &std::sync::mpsc::Receiver<std::io::Result<ExitStatus>>,
    span: Duration,
) -> Result<std::io::Result<ExitStatus>, RecvTimeoutError> {
    #[cfg(test)]
    wait_probe::record(span);
    rx.recv_timeout(span)
}

/// Blocks until the child exits or `timeout` elapses, then reports which.
///
/// A helper thread performs the OS-blocking wait and hands the status back over
/// a channel, so the caller neither polls nor sleeps: a child that exits early
/// is observed immediately rather than up to one poll interval later, and an
/// idle wait costs no syscalls. `SharedChild` is what makes this safe — it
/// permits the timeout path to kill and reap through the same handle the waiter
/// thread is blocked on.
///
/// ## Notes
///
/// Callers must already be draining stdout/stderr concurrently. Blocking here
/// while a child fills an undrained pipe would deadlock: the child blocks on
/// write, we block on exit, and only the timeout breaks the tie.
pub(super) fn wait_with_timeout(
    child: &Arc<SharedChild>,
    timeout: Duration,
) -> Result<WaitOutcome, std::io::Error> {
    let waiter = Arc::clone(child);
    let (tx, rx) = std::sync::mpsc::channel();
    let wait_thread = std::thread::spawn(move || {
        let _ = tx.send(waiter.wait());
    });

    let outcome = match recv_wait(&rx, timeout) {
        Ok(status) => status.map(WaitOutcome::Exited),
        Err(RecvTimeoutError::Timeout) => {
            // Kill first so the blocked waiter thread can observe the exit and
            // reap the child; joining below guarantees it did.
            let _ = child.kill();
            Ok(WaitOutcome::TimedOut)
        }
        Err(RecvTimeoutError::Disconnected) => Err(std::io::Error::other(
            "process wait thread terminated without reporting a status",
        )),
    };

    let _ = wait_thread.join();
    outcome
}

fn join_output_thread_raw(
    handle: JoinHandle<Vec<u8>>,
    stream_name: &str,
    ctx: &biscuit_terminal::errors::SourceContext,
) -> Result<Vec<u8>, ShellExpansionError> {
    handle
        .join()
        .map_err(|_| ShellExpansionError::ExecutionFailed {
            ctx: Box::new(ctx.clone()),
            command: String::new(),
            code: -1,
            stdout: String::new(),
            stderr: format!("{stream_name} capture thread panicked"),
            origin: ShellCommandOrigin::Body { line: 0 },
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::shell_expansion::types::{
        ErrorHandling, PipelineAction, ShellCommandOrigin,
    };
    use tempfile::TempDir;

    fn test_ctx() -> biscuit_terminal::errors::SourceContext {
        biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("/test"),
            std::path::PathBuf::from("test"),
            String::new(),
        )
    }

    /// Helper to build a ShellDirective with default error handling.
    fn directive(raw: &str, exe: &str, args: &[&str], line: usize) -> ShellDirective {
        ShellDirective {
            raw_command: raw.to_string(),
            executable: exe.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            span: 0..raw.len(),
            indent: String::new(),
            origin: ShellCommandOrigin::Body { line },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            no_cache: false,
            ctx: test_ctx(),
        }
    }

    // ---------------------------------------------------------------------
    // Unix-utility fixture tests
    //
    // These predate the F17 helper-child redesign and reach for `echo`, `true`,
    // `false`, `cat`, `sleep`, and `env` by name. None of the six exists as a
    // spawnable executable on Windows (`echo` is a cmd.exe builtin, not a
    // binary), so `Command::new` there fails with `CommandNotFound` and the
    // assertion under test is never reached. They are gated rather than left to
    // fail: the executor behavior each one covers is target-neutral, and the
    // Unix utility is only a convenient fixture — not the subject.
    //
    // Porting them needs helper-child modes that echo argv and dump the
    // environment (see `ChildMode`); that migration is scoped to Phase 11. The
    // gate exists so this binary is Windows-runnable as a whole *now*, which the
    // F17 Windows behavioral run requires.
    // ---------------------------------------------------------------------

    #[cfg(unix)]
    #[test]
    fn echo_hello_returns_output() {
        let directive = directive("echo hello", "echo", &["hello"], 1);
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let output = execute_command(&directive, &options, &source).unwrap();
        assert_eq!(output.trim(), "hello");
    }

    #[cfg(unix)]
    #[test]
    fn empty_output_returns_empty_string() {
        let d = directive("true", "true", &[], 1);
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output, "");
    }

    #[cfg(unix)]
    #[test]
    fn non_zero_exit_produces_execution_failed() {
        let d = directive("false", "false", &[], 1);
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let result = execute_command(&d, &options, &source);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShellExpansionError::ExecutionFailed { code, .. } => {
                assert_ne!(code, 0);
            }
            err => panic!("Expected ExecutionFailed, got: {:?}", err),
        }
    }

    #[test]
    fn command_not_found_for_nonexistent_executable() {
        let d = directive("nonexistent_command_xyz", "nonexistent_command_xyz", &[], 1);
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let result = execute_command(&d, &options, &source);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShellExpansionError::CommandNotFound {
                command, origin, ..
            } => {
                assert_eq!(command, "nonexistent_command_xyz");
                assert_eq!(origin, ShellCommandOrigin::Body { line: 1 });
            }
            err => panic!("Expected CommandNotFound, got: {:?}", err),
        }
    }

    #[cfg(unix)]
    #[test]
    fn timeout_kills_long_running_command() {
        let d = directive("sleep 10", "sleep", &["10"], 1);
        let options = ShellExpansionOptions {
            timeout: Duration::from_millis(100),
            ..Default::default()
        };
        let source = ComposeSource::Unknown;

        let result = execute_command(&d, &options, &source);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShellExpansionError::Timeout { timeout, .. } => {
                assert_eq!(timeout, Duration::from_millis(100));
            }
            err => panic!("Expected Timeout, got: {:?}", err),
        }
    }

    #[test]
    fn working_directory_resolution_priority() {
        // Test 1: working_directory takes highest priority
        let temp_dir = TempDir::new().unwrap();
        let options = ShellExpansionOptions {
            working_directory: Some(temp_dir.path().to_path_buf()),
            policy_root: Some(PathBuf::from("/some/other/path")),
            ..Default::default()
        };
        let source = ComposeSource::File(PathBuf::from("/yet/another/path/file.md"));
        let wd = resolve_working_directory(&options, &source);
        assert_eq!(wd, temp_dir.path());

        // Test 2: File source parent when working_directory is None
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::File(PathBuf::from("/path/to/file.md"));
        let wd = resolve_working_directory(&options, &source);
        assert_eq!(wd, PathBuf::from("/path/to"));

        // Test 3: policy_root when no file source
        let options = ShellExpansionOptions {
            policy_root: Some(PathBuf::from("/policy/root")),
            ..Default::default()
        };
        let source = ComposeSource::Unknown;
        let wd = resolve_working_directory(&options, &source);
        assert_eq!(wd, PathBuf::from("/policy/root"));

        // Test 4: current_dir as fallback
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;
        let wd = resolve_working_directory(&options, &source);
        assert!(!wd.as_os_str().is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn stdin_is_null_command_does_not_hang() {
        let d = directive("cat", "cat", &[], 1);
        let options = ShellExpansionOptions {
            timeout: Duration::from_secs(1),
            ..Default::default()
        };
        let source = ComposeSource::Unknown;

        let result = execute_command(&d, &options, &source);
        match result {
            Ok(output) => assert_eq!(output, ""),
            Err(ShellExpansionError::Timeout { .. }) => {
                panic!("Command should not timeout with null stdin");
            }
            Err(_) => {}
        }
    }

    #[test]
    fn stderr_is_captured_and_combined() {
        let d = child_directive(ChildMode::Streams { code: 0 });
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let output = execute_command_detailed(&d, &options, &source).unwrap();
        assert_eq!(framed_payload(&output.stdout), "out");
        assert_eq!(framed_payload(&output.stderr), "err");
        // Only stdout carries libtest's preamble, so the tail of the combined
        // output still pins both the order and the newline separator.
        let frame = PAYLOAD_FRAME as char;
        let combined = output.combined_output();
        assert!(
            combined.ends_with(&format!("{frame}out\n{frame}err")),
            "combined output must be stdout then stderr, newline separated: {combined:?}"
        );
    }

    #[test]
    fn join_output_thread_reports_panic() {
        let handle = std::thread::spawn(|| -> Vec<u8> {
            panic!("boom");
        });

        let err = join_output_thread_raw(handle, "stdout", &test_ctx()).unwrap_err();
        match err {
            ShellExpansionError::ExecutionFailed { code, stderr, .. } => {
                assert_eq!(code, -1);
                assert!(stderr.contains("stdout capture thread panicked"));
            }
            other => panic!("Expected ExecutionFailed, got {other:?}"),
        }
    }

    #[test]
    fn bare_filename_source_falls_through_to_cwd() {
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::File(PathBuf::from("file.md"));
        let wd = resolve_working_directory(&options, &source);
        assert!(
            !wd.as_os_str().is_empty(),
            "Working directory must not be empty for bare filenames"
        );
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_with_bare_filename_source_succeeds() {
        let d = directive("echo works", "echo", &["works"], 5);
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::File(PathBuf::from("test.md"));

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output.trim(), "works");
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_with_quoted_args() {
        let d = ShellDirective {
            raw_command: r#"echo "hello world""#.to_string(),
            executable: "echo".to_string(),
            args: vec!["hello world".to_string()],
            span: 0..18,
            indent: String::new(),
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            no_cache: false,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output.trim(), "hello world");
    }

    #[test]
    fn execution_failed_includes_output_streams() {
        let d = child_directive(ChildMode::Streams { code: 42 });
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        match execute_command(&d, &options, &source) {
            Err(ShellExpansionError::ExecutionFailed {
                code,
                stdout,
                stderr,
                ..
            }) => {
                assert_eq!(code, 42);
                assert_eq!(framed_payload(&stdout), "out");
                assert_eq!(framed_payload(&stderr), "err");
            }
            other => panic!("Expected ExecutionFailed with code 42, got: {:?}", other),
        }
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_strips_ansi_by_default() {
        let d = ShellDirective {
            raw_command: "echo ...".to_string(),
            executable: "echo".to_string(),
            args: vec!["\x1b[31mhello\x1b[0m".to_string()],
            span: 0..0,
            indent: String::new(),
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            no_cache: false,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions::default(); // strip_ansi: true by default
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output.trim(), "hello");
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_keeps_ansi_when_opt_out() {
        let d = ShellDirective {
            raw_command: "echo ...".to_string(),
            executable: "echo".to_string(),
            args: vec!["\x1b[31mhello\x1b[0m".to_string()],
            span: 0..0,
            indent: String::new(),
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            no_cache: false,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions {
            strip_ansi: false,
            ..Default::default()
        };
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output.trim(), "\x1b[31mhello\x1b[0m");
    }

    #[cfg(unix)]
    #[test]
    fn execute_command_sets_no_color_env() {
        let d = ShellDirective {
            raw_command: "env".to_string(),
            executable: "env".to_string(),
            args: vec![],
            span: 0..0,
            indent: String::new(),
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            no_cache: false,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions::default(); // strip_ansi: true by default
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert!(output.contains("NO_COLOR=1"));
    }

    #[cfg(unix)]
    #[test]
    fn per_command_timeout_override_beats_global() {
        let d = ShellDirective {
            raw_command: "sleep 10".to_string(),
            executable: "sleep".to_string(),
            args: vec!["10".to_string()],
            span: 0..0,
            indent: String::new(),
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: Some(Duration::from_millis(100)),
            pipeline: None,
            no_cache: false,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions {
            timeout: Duration::from_secs(60), // Global timeout is 60s
            ..Default::default()
        };
        let source = ComposeSource::Unknown;

        let result = execute_command(&d, &options, &source);
        assert!(result.is_err());
        match result.unwrap_err() {
            ShellExpansionError::Timeout { timeout, .. } => {
                assert_eq!(timeout, Duration::from_millis(100));
            }
            err => panic!("Expected Timeout, got: {:?}", err),
        }
    }

    // ---------------------------------------------------------------------
    // Finding 17 helper child process
    //
    // The F17 tests need a child that saturates its pipes, one that exits
    // instantly, and one that outlives a timeout. Reaching for `python`,
    // `true`, and `sleep` made them Unix-only and let them pass silently when
    // the interpreter was absent. Instead the test binary re-executes *itself*
    // as the helper, so the suite depends on nothing beyond the toolchain that
    // built it and behaves identically on every target.
    // ---------------------------------------------------------------------

    /// argv token that turns a re-executed copy of this test binary into a
    /// helper child process.
    ///
    /// The mode travels in argv rather than the environment because the executor
    /// under test gives a `ShellDirective` no way to set child env — the parent
    /// would have to mutate its own process env, which is global and racy. libtest
    /// treats an unrecognized positional as a name filter, and under `--exact`
    /// this token matches no test, so it is inert to the harness.
    const CHILD_MODE_ARG: &str = "dm-child-mode=";

    /// stdout payload byte for [`ChildMode::Saturate`].
    ///
    /// The helper *is* the test binary, so libtest prints its own `running 1
    /// test` preamble into the pipe before our payload and we cannot suppress
    /// it. Both payload bytes are chosen to be absent from that preamble and
    /// from this module's test names, which makes the first payload byte an
    /// unambiguous frame start (see [`payload_after_preamble`]) and keeps the
    /// merged-stream counts exact.
    const SATURATE_STDOUT_BYTE: u8 = b'#';

    /// stderr payload byte for [`ChildMode::Saturate`]; see [`SATURATE_STDOUT_BYTE`].
    const SATURATE_STDERR_BYTE: u8 = b'@';

    /// Precedes each [`ChildMode::Streams`] payload, marking where libtest's
    /// preamble ends and the helper's own bytes begin. Same reasoning as
    /// [`SATURATE_STDOUT_BYTE`], but these payloads are short and arbitrary, so
    /// they need an explicit frame rather than a distinctive alphabet.
    const PAYLOAD_FRAME: u8 = b'#';

    /// Bytes per stream for the saturation tests. Comfortably above the 64 KiB
    /// pipe buffer both Unix and Windows default to, so an undrained pipe is
    /// guaranteed to wedge the child rather than merely be a tight fit.
    const SATURATION_BYTES: usize = 256 * 1024;

    /// What a re-executed copy of this test binary should do instead of testing.
    enum ChildMode {
        /// Exit 0 immediately, writing nothing. Replaces `true`.
        Noop,
        /// Interleave `SATURATION_BYTES` onto stdout and stderr in 8 KiB chunks,
        /// so neither pipe can be fully drained before the other is written to.
        Saturate,
        /// Append a byte to the given file every 20ms, forever. Replaces `sleep`,
        /// and the file doubles as a platform-neutral liveness probe.
        Heartbeat(std::path::PathBuf),
        /// Write `#out` to stdout and `#err` to stderr, then exit with `code`.
        /// Replaces a `python -c` one-liner that printed to both streams.
        Streams { code: i32 },
    }

    impl ChildMode {
        /// Renders the mode into the argv token, and parses it back in the child.
        fn spec(&self) -> String {
            match self {
                Self::Noop => "noop".to_string(),
                Self::Saturate => "saturate".to_string(),
                Self::Heartbeat(path) => format!("heartbeat:{}", path.display()),
                Self::Streams { code } => format!("streams:{code}"),
            }
        }

        fn parse(spec: &str) -> Self {
            // `split_once` stops at the first colon, so a Windows `C:\...`
            // heartbeat path survives intact in the argument.
            match spec.split_once(':') {
                None if spec == "noop" => Self::Noop,
                None if spec == "saturate" => Self::Saturate,
                Some(("heartbeat", path)) => Self::Heartbeat(std::path::PathBuf::from(path)),
                Some(("streams", code)) => Self::Streams {
                    code: code.parse().expect("streams:<exit-code>"),
                },
                _ => panic!("unknown child mode `{spec}`"),
            }
        }
    }

    /// Helper-child entrypoint. A no-op during an ordinary test run; when this
    /// binary is re-executed by [`child_directive`] it becomes the helper
    /// process and never returns.
    #[test]
    fn child_process_entrypoint() {
        let Some(spec) = std::env::args()
            .find_map(|arg| arg.strip_prefix(CHILD_MODE_ARG).map(str::to_owned))
        else {
            return;
        };

        let exit_code = match ChildMode::parse(&spec) {
            ChildMode::Noop => 0,
            ChildMode::Saturate => {
                saturate_both_streams();
                0
            }
            ChildMode::Heartbeat(path) => heartbeat_forever(&path),
            ChildMode::Streams { code } => {
                write_framed_streams();
                code
            }
        };

        // Exit rather than return: libtest would otherwise print its result
        // footer into the very pipes the parent is asserting on.
        std::process::exit(exit_code);
    }

    fn write_framed_streams() {
        use std::io::Write;

        let frame = PAYLOAD_FRAME as char;
        let mut stdout = std::io::stdout();
        let mut stderr = std::io::stderr();
        write!(stdout, "{frame}out").expect("child: write stdout");
        stdout.flush().expect("child: flush stdout");
        write!(stderr, "{frame}err").expect("child: write stderr");
        stderr.flush().expect("child: flush stderr");
    }

    fn saturate_both_streams() {
        use std::io::Write;

        const CHUNK: usize = 8 * 1024;
        let mut stdout = std::io::stdout();
        let mut stderr = std::io::stderr();
        let mut written = 0;
        while written < SATURATION_BYTES {
            let n = CHUNK.min(SATURATION_BYTES - written);
            stdout
                .write_all(&vec![SATURATE_STDOUT_BYTE; n])
                .expect("child: write stdout");
            stdout.flush().expect("child: flush stdout");
            stderr
                .write_all(&vec![SATURATE_STDERR_BYTE; n])
                .expect("child: write stderr");
            stderr.flush().expect("child: flush stderr");
            written += n;
        }
    }

    fn heartbeat_forever(path: &std::path::Path) -> ! {
        use std::io::Write;

        loop {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(path)
            {
                let _ = file.write_all(b".");
                let _ = file.flush();
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Builds a directive that re-executes this test binary in `mode`.
    ///
    /// `--nocapture` is required: without it libtest buffers the helper's
    /// `println!`-family writes internally and they never reach the OS pipe the
    /// executor is draining.
    ///
    /// The `--exact` filter is derived from `module_path!()` rather than
    /// hardcoded, so moving or renaming this module cannot silently leave the
    /// filter matching zero tests.
    fn child_directive(mode: ChildMode) -> ShellDirective {
        let exe = std::env::current_exe().expect("locate current test executable");
        let module = module_path!()
            .split_once("::")
            .map(|(_crate_name, rest)| rest)
            .expect("test module path has a crate segment");
        let spec = mode.spec();

        ShellDirective {
            raw_command: format!("<test-helper> {spec}"),
            executable: exe.to_string_lossy().into_owned(),
            args: vec![
                "--exact".to_string(),
                format!("{module}::child_process_entrypoint"),
                "--nocapture".to_string(),
                format!("{CHILD_MODE_ARG}{spec}"),
            ],
            span: 0..0,
            indent: String::new(),
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: Some(Duration::from_secs(60)),
            pipeline: None,
            no_cache: false,
            ctx: test_ctx(),
        }
    }

    /// Returns the helper's payload, dropping libtest's preamble ahead of it.
    ///
    /// Panics rather than returning empty when no payload byte is present: that
    /// means the helper never ran (most likely the `--exact` filter matched no
    /// test), and a silent empty string would turn that into a confusing
    /// length-mismatch instead of naming the real fault.
    fn payload_after_preamble(stream: &str, payload_byte: u8) -> &str {
        let start = stream.find(payload_byte as char).unwrap_or_else(|| {
            panic!(
                "helper child produced no `{}` payload; captured {} bytes: {:?}",
                payload_byte as char,
                stream.len(),
                stream.chars().take(200).collect::<String>(),
            )
        });
        &stream[start..]
    }

    fn count_byte(stream: &str, byte: u8) -> usize {
        stream.bytes().filter(|b| *b == byte).count()
    }

    /// Returns a [`ChildMode::Streams`] payload with its frame and any preceding
    /// libtest preamble removed.
    fn framed_payload(stream: &str) -> &str {
        &payload_after_preamble(stream, PAYLOAD_FRAME)[1..]
    }

    /// F17 — the blocking wait must keep draining both pipes concurrently.
    /// A wait that blocked on child exit without draining would wedge: the child
    /// blocks writing into a full pipe, we block waiting for it to exit, and
    /// only the timeout breaks the tie. Covers `execute_command_detailed`.
    #[test]
    fn saturated_dual_stream_capture_does_not_deadlock() {
        let d = child_directive(ChildMode::Saturate);
        let options = ShellExpansionOptions {
            strip_ansi: false,
            ..Default::default()
        };

        let output = execute_command_detailed(&d, &options, &ComposeSource::Unknown).unwrap();
        let stdout = payload_after_preamble(&output.stdout, SATURATE_STDOUT_BYTE);
        let stderr = payload_after_preamble(&output.stderr, SATURATE_STDERR_BYTE);
        assert_eq!(stdout.len(), SATURATION_BYTES);
        assert_eq!(stderr.len(), SATURATION_BYTES);
        assert!(stdout.bytes().all(|b| b == SATURATE_STDOUT_BYTE));
        assert!(stderr.bytes().all(|b| b == SATURATE_STDERR_BYTE));
    }

    /// F17 — the same saturation guarantee for the redirection/`execute_single_action`
    /// executor, whose `ReadStrategy::Separate` branch owns its own drain threads.
    #[test]
    fn saturated_dual_stream_capture_does_not_deadlock_in_pipeline_executor() {
        let mut d = child_directive(ChildMode::Saturate);
        let noop = child_directive(ChildMode::Noop);
        // Two actions force `execute_pipeline_detailed` -> `execute_single_action`
        // rather than the standard single-command path.
        d.pipeline = Some(ShellPipeline {
            actions: vec![
                PipelineAction {
                    operator: ChainOperator::None,
                    command: CommandAction {
                        executable: d.executable.clone(),
                        args: d.args.clone(),
                        redirection: RedirectionConfig::default(),
                    },
                },
                PipelineAction {
                    operator: ChainOperator::And,
                    command: CommandAction {
                        executable: noop.executable.clone(),
                        args: noop.args.clone(),
                        redirection: RedirectionConfig::default(),
                    },
                },
            ],
        });
        let options = ShellExpansionOptions {
            strip_ansi: false,
            ..Default::default()
        };

        let output = execute_directive_impl(&d, &options, &ComposeSource::Unknown).unwrap();
        // Counted, not framed: the pipeline concatenates both actions' captures,
        // so each stream carries two libtest preambles rather than one leading
        // one. The payload bytes cannot occur in either.
        assert_eq!(
            count_byte(&output.stdout, SATURATE_STDOUT_BYTE),
            SATURATION_BYTES
        );
        assert_eq!(
            count_byte(&output.stderr, SATURATE_STDERR_BYTE),
            SATURATION_BYTES
        );
    }

    /// F17 — the merged (`2>&1`) capture path shares one OS pipe for both
    /// streams, so saturation must not wedge its single merged reader either.
    #[test]
    fn saturated_merged_stream_capture_does_not_deadlock() {
        let mut d = child_directive(ChildMode::Saturate);
        d.pipeline = Some(ShellPipeline {
            actions: vec![PipelineAction {
                operator: ChainOperator::None,
                command: CommandAction {
                    executable: d.executable.clone(),
                    args: d.args.clone(),
                    redirection: RedirectionConfig {
                        stdout: StdoutTarget::Capture,
                        stderr: StderrTarget::ToStdout,
                    },
                },
            }],
        });
        let options = ShellExpansionOptions {
            strip_ansi: false,
            ..Default::default()
        };

        let output = execute_command_detailed(&d, &options, &ComposeSource::Unknown).unwrap();
        // Both streams land in stdout; stderr stays empty for a `2>&1` merge.
        // Counted per payload byte rather than framed: a merged pipe interleaves
        // the two streams at chunk granularity, so only the totals are defined.
        assert_eq!(
            count_byte(&output.stdout, SATURATE_STDOUT_BYTE),
            SATURATION_BYTES
        );
        assert_eq!(
            count_byte(&output.stdout, SATURATE_STDERR_BYTE),
            SATURATION_BYTES
        );
        assert_eq!(output.stderr, "");
    }

    /// The wait budget the no-poll tests hand to the executor. Arbitrary, but far
    /// from any real duration so an assertion failure names the shape mismatch
    /// rather than looking like a timing coincidence.
    const NO_POLL_BUDGET: Duration = Duration::from_secs(7);

    /// F17 — the wait must observe an early exit immediately rather than at the
    /// next poll tick, so it must hand its whole budget to one blocking span
    /// instead of slicing it into ticks.
    ///
    /// This asserts the *shape* of the wait, not its duration. Elapsed time
    /// cannot separate the two implementations: the retired loop's 10ms tick
    /// fits inside any bound that stays non-flaky on a loaded host. The retired
    /// `try_wait` + `sleep(10ms)` loop records no spans at all, and a
    /// short-span polling variant records many; only a single full-budget span
    /// passes. Covers the `execute_command_detailed` wait.
    #[test]
    fn wait_hands_the_full_budget_to_one_blocking_span() {
        let mut d = child_directive(ChildMode::Noop);
        d.timeout_override = Some(NO_POLL_BUDGET);
        let options = ShellExpansionOptions::default();

        wait_probe::take();
        execute_command(&d, &options, &ComposeSource::Unknown).unwrap();

        assert_eq!(wait_probe::take(), vec![NO_POLL_BUDGET]);
    }

    /// F17 — the same no-poll guarantee for the redirection executor, whose wait
    /// is a separate call site from the standard one.
    #[test]
    fn pipeline_wait_hands_the_full_budget_to_one_blocking_span() {
        let mut d = child_directive(ChildMode::Noop);
        d.timeout_override = Some(NO_POLL_BUDGET);
        // A non-default redirection routes through `execute_single_action`.
        d.pipeline = Some(ShellPipeline {
            actions: vec![PipelineAction {
                operator: ChainOperator::None,
                command: CommandAction {
                    executable: d.executable.clone(),
                    args: d.args.clone(),
                    redirection: RedirectionConfig {
                        stdout: StdoutTarget::Capture,
                        stderr: StderrTarget::ToStdout,
                    },
                },
            }],
        });
        let options = ShellExpansionOptions::default();

        wait_probe::take();
        execute_command_detailed(&d, &options, &ComposeSource::Unknown).unwrap();

        assert_eq!(wait_probe::take(), vec![NO_POLL_BUDGET]);
    }

    /// F17 — a timed-out child is killed, not merely abandoned. The wait
    /// primitive owns the kill+reap, so prove the process actually stopped
    /// running rather than being left behind past the timeout.
    ///
    /// The liveness proof is the helper's own heartbeat file rather than a
    /// process-table query: `pgrep` does not exist on Windows, whereas a file
    /// that stops growing is the same evidence everywhere.
    ///
    /// The *reap* half is proven by this test terminating at all.
    /// `wait_with_timeout` joins the thread parked in `SharedChild::wait`, which
    /// cannot return until the child is reaped — so a kill that failed to reap
    /// hangs this test rather than passing it.
    #[test]
    fn timed_out_child_process_is_killed_and_reaped() {
        let tmp = TempDir::new().unwrap();
        let heartbeat = tmp.path().join("heartbeat");
        let mut d = child_directive(ChildMode::Heartbeat(heartbeat.clone()));
        d.timeout_override = Some(Duration::from_millis(500));
        let options = ShellExpansionOptions {
            timeout_behavior: ShellTimeoutBehavior::EmptyString,
            ..Default::default()
        };

        let result = execute_command_detailed(&d, &options, &ComposeSource::Unknown).unwrap();
        assert_eq!(result.timeout_fallback, Some(Duration::from_millis(500)));

        // Without this, a helper that never started would satisfy the
        // stopped-growing check below and the test would pass vacuously.
        let beats_at_timeout = std::fs::metadata(&heartbeat).map(|m| m.len()).unwrap_or(0);
        assert!(
            beats_at_timeout > 0,
            "helper child never wrote a heartbeat, so it cannot show it was killed"
        );

        // Long enough for many more 20ms beats had the kill not landed.
        std::thread::sleep(Duration::from_millis(400));
        let beats_after = std::fs::metadata(&heartbeat).unwrap().len();
        assert_eq!(
            beats_after, beats_at_timeout,
            "timed-out child kept beating past the timeout; it was abandoned, not killed"
        );
    }

    /// F17 — timeout must still select the `Timeout` error (not `ExecutionFailed`)
    /// for the redirection executor, whose wait path is separate from the
    /// standard one.
    #[test]
    fn pipeline_executor_timeout_selects_timeout_error() {
        let tmp = TempDir::new().unwrap();
        let mut d = child_directive(ChildMode::Heartbeat(tmp.path().join("heartbeat")));
        d.timeout_override = Some(Duration::from_millis(100));
        d.pipeline = Some(ShellPipeline {
            actions: vec![PipelineAction {
                operator: ChainOperator::None,
                command: CommandAction {
                    executable: d.executable.clone(),
                    args: d.args.clone(),
                    redirection: RedirectionConfig {
                        stdout: StdoutTarget::Capture,
                        stderr: StderrTarget::ToStdout,
                    },
                },
            }],
        });
        let options = ShellExpansionOptions::default();

        let err = execute_command_detailed(&d, &options, &ComposeSource::Unknown).unwrap_err();
        match err {
            ShellExpansionError::Timeout { timeout, .. } => {
                assert_eq!(timeout, Duration::from_millis(100));
            }
            other => panic!("Expected Timeout, got: {other:?}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn timeout_with_empty_string_behavior_returns_empty() {
        let d = ShellDirective {
            raw_command: "sleep 10".to_string(),
            executable: "sleep".to_string(),
            args: vec!["10".to_string()],
            span: 0..0,
            indent: String::new(),
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: Some(Duration::from_millis(100)),
            pipeline: None,
            no_cache: false,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions {
            timeout: Duration::from_secs(60),
            timeout_behavior: ShellTimeoutBehavior::EmptyString,
            ..Default::default()
        };
        let source = ComposeSource::Unknown;

        let result = execute_command_detailed(&d, &options, &source);
        assert!(result.is_ok());
        let result = result.unwrap();
        assert_eq!(result.stdout, "");
        assert_eq!(result.stderr, "");
        assert_eq!(result.timeout_fallback, Some(Duration::from_millis(100)));
    }
}
