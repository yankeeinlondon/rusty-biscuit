//! Command execution for shell directives.
//!
//! This module provides safe command execution with timeout protection,
//! working directory resolution, and stdout/stderr capture.

use std::io::Read;
use std::path::PathBuf;
use std::process::Command;
use std::thread::JoinHandle;
use std::time::Instant;

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
///     origin: ShellCommandOrigin::Body { line: 1 },
///     error_handling: ErrorHandling::default(),
///     timeout_override: None,
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
    let mut child = cmd
        .spawn()
        .map_err(|e| ShellExpansionError::ExecutionFailed {
            ctx: Box::new(directive.ctx.clone()),
            command: directive.raw_command.clone(),
            code: -1,
            stdout: String::new(),
            stderr: e.to_string(),
            origin: directive.origin.clone(),
        })?;

    // 5. Drain stdout and stderr concurrently via threads
    let stdout_handle = child.stdout.take();
    let stderr_handle = child.stderr.take();

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

    // 6. Poll with timeout
    let timeout = directive.timeout_override.unwrap_or(shell_opts.timeout);
    let start = Instant::now();
    let sleep_duration = std::time::Duration::from_millis(10);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process completed
                let stdout_bytes =
                    join_output_thread_raw(stdout_thread, "stdout", &directive.ctx)?;
                let stderr_bytes =
                    join_output_thread_raw(stderr_thread, "stderr", &directive.ctx)?;
                let stdout_str = String::from_utf8_lossy(&stdout_bytes).to_string();
                let stderr_str = String::from_utf8_lossy(&stderr_bytes).to_string();

                if status.success() {
                    let (mut stdout, mut stderr) = (stdout_str, stderr_str);

                    if shell_opts.strip_ansi {
                        stdout = biscuit_terminal::prelude::strip_escape_codes(stdout);
                        stderr = biscuit_terminal::prelude::strip_escape_codes(stderr);
                    }

                    let output = CommandExecution::from_streams(stdout, stderr);

                    debug!(
                        exit_code = 0,
                        output_len = output.combined_output().len(),
                        "shell: command succeeded"
                    );
                    return Ok(output);
                } else {
                    let mut stdout = stdout_str;
                    let mut stderr = stderr_str;

                    if shell_opts.strip_ansi {
                        stdout = biscuit_terminal::prelude::strip_escape_codes(stdout);
                        stderr = biscuit_terminal::prelude::strip_escape_codes(stderr);
                    }

                    return Err(ShellExpansionError::ExecutionFailed {
                        ctx: Box::new(directive.ctx.clone()),
                        command: directive.raw_command.clone(),
                        code: status.code().unwrap_or(-1),
                        stdout,
                        stderr,
                        origin: directive.origin.clone(),
                    });
                }
            }
            Ok(None) => {
                // Still running
                if start.elapsed() >= timeout {
                    // Kill the child
                    let _ = child.kill();
                    let _ = child.wait();
                    warn!(elapsed = ?start.elapsed(), "shell: command timed out");
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
                std::thread::sleep(sleep_duration);
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
        }
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

    let mut child = cmd
        .spawn()
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
            let stdout_handle = child.stdout.take();
            let stderr_handle = child.stderr.take();

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

    let start = Instant::now();
    let sleep_duration = std::time::Duration::from_millis(10);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
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
                    return Ok(CommandExecution::from_streams(final_stdout, final_stderr));
                } else {
                    return Err(ShellExpansionError::ExecutionFailed {
                        ctx: Box::new(ctx.clone()),
                        command: raw_command.to_string(),
                        code: status.code().unwrap_or(-1),
                        stdout: final_stdout,
                        stderr: final_stderr,
                        origin: origin.clone(),
                    });
                }
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
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
                std::thread::sleep(sleep_duration);
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
        }
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
    use crate::markdown::compose::shell_expansion::types::{ErrorHandling, ShellCommandOrigin};
    use std::ffi::OsStr;
    use std::time::Duration;
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
            origin: ShellCommandOrigin::Body { line },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            ctx: test_ctx(),
        }
    }

    #[test]
    fn echo_hello_returns_output() {
        let directive = directive("echo hello", "echo", &["hello"], 1);
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let output = execute_command(&directive, &options, &source).unwrap();
        assert_eq!(output.trim(), "hello");
    }

    #[test]
    fn empty_output_returns_empty_string() {
        let d = directive("true", "true", &[], 1);
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output, "");
    }

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
        let Some(python) = find_python() else {
            return;
        };

        let d = ShellDirective {
            raw_command: format!("{} -c ...", python.display()),
            executable: python.to_string_lossy().to_string(),
            args: vec![
                "-c".to_string(),
                "import sys; sys.stderr.write('oops'); sys.stdout.write('ok')".to_string(),
            ],
            span: 0..10,
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let output = execute_command_detailed(&d, &options, &source).unwrap();
        assert_eq!(output.stdout, "ok");
        assert_eq!(output.stderr, "oops");
        assert_eq!(output.combined_output(), "ok\noops");
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

    #[test]
    fn execute_command_with_bare_filename_source_succeeds() {
        let d = directive("echo works", "echo", &["works"], 5);
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::File(PathBuf::from("test.md"));

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output.trim(), "works");
    }

    #[test]
    fn execute_command_with_quoted_args() {
        let d = ShellDirective {
            raw_command: r#"echo "hello world""#.to_string(),
            executable: "echo".to_string(),
            args: vec!["hello world".to_string()],
            span: 0..18,
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions::default();
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output.trim(), "hello world");
    }

    #[test]
    fn execution_failed_includes_output_streams() {
        let Some(python) = find_python() else {
            return;
        };

        let d = ShellDirective {
            raw_command: format!("{} -c ...", python.display()),
            executable: python.to_string_lossy().to_string(),
            args: vec![
                "-c".to_string(),
                "import sys; sys.stdout.write('out'); sys.stderr.write('err'); sys.exit(42)"
                    .to_string(),
            ],
            span: 0..10,
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            ctx: test_ctx(),
        };
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
                assert_eq!(stdout, "out");
                assert_eq!(stderr, "err");
            }
            other => panic!("Expected ExecutionFailed with code 42, got: {:?}", other),
        }
    }

    fn find_python() -> Option<PathBuf> {
        ["python3", "python"]
            .into_iter()
            .find_map(|candidate| which::which(candidate).ok())
            .filter(|path| !path.as_os_str().is_empty())
    }

    #[test]
    fn find_python_returns_python_binary_when_available() {
        if let Some(path) = find_python() {
            assert_ne!(path.as_os_str(), OsStr::new(""));
        }
    }

    #[test]
    fn execute_command_strips_ansi_by_default() {
        let d = ShellDirective {
            raw_command: "echo ...".to_string(),
            executable: "echo".to_string(),
            args: vec!["\x1b[31mhello\x1b[0m".to_string()],
            span: 0..0,
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions::default(); // strip_ansi: true by default
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert_eq!(output.trim(), "hello");
    }

    #[test]
    fn execute_command_keeps_ansi_when_opt_out() {
        let d = ShellDirective {
            raw_command: "echo ...".to_string(),
            executable: "echo".to_string(),
            args: vec!["\x1b[31mhello\x1b[0m".to_string()],
            span: 0..0,
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
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

    #[test]
    fn execute_command_sets_no_color_env() {
        let d = ShellDirective {
            raw_command: "env".to_string(),
            executable: "env".to_string(),
            args: vec![],
            span: 0..0,
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: None,
            pipeline: None,
            ctx: test_ctx(),
        };
        let options = ShellExpansionOptions::default(); // strip_ansi: true by default
        let source = ComposeSource::Unknown;

        let output = execute_command(&d, &options, &source).unwrap();
        assert!(output.contains("NO_COLOR=1"));
    }

    #[test]
    fn per_command_timeout_override_beats_global() {
        let d = ShellDirective {
            raw_command: "sleep 10".to_string(),
            executable: "sleep".to_string(),
            args: vec!["10".to_string()],
            span: 0..0,
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: Some(Duration::from_millis(100)),
            pipeline: None,
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

    #[test]
    fn timeout_with_empty_string_behavior_returns_empty() {
        let d = ShellDirective {
            raw_command: "sleep 10".to_string(),
            executable: "sleep".to_string(),
            args: vec!["10".to_string()],
            span: 0..0,
            origin: ShellCommandOrigin::Body { line: 1 },
            error_handling: ErrorHandling::default(),
            timeout_override: Some(Duration::from_millis(100)),
            pipeline: None,
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
