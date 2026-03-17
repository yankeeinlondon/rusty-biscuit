use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::thread;
use std::time::Instant;

use claudine::stream::parser::{StreamParseError, StreamParser};
use claudine::stream::summary::StreamExecutionSummary;
use color_eyre::eyre::Result;

pub(crate) struct ChildIoOptions<'a> {
    pub(crate) stdout_noise_prefixes: &'a [&'a str],
    pub(crate) stderr_noise_prefixes: &'a [&'a str],
    pub(crate) stdin_seed: Option<&'a str>,
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
) -> Result<i32> {
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

    // Write stdin seed and close the pipe so the child sees EOF.
    if let Some(seed) = io.stdin_seed
        && let Some(mut stdin_pipe) = child.stdin.take()
    {
        stdin_pipe.write_all(seed.as_bytes())?;
        // Drop closes the pipe
    }

    // Spawn filter threads that read child output line-by-line and
    // suppress lines matching any noise prefix.
    let stdout_handle = if filter_stdout {
        let pipe = child.stdout.take().expect("stdout was set to piped");
        let prefixes: Vec<String> = io
            .stdout_noise_prefixes
            .iter()
            .map(|s| s.to_string())
            .collect();
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut out = std::io::stdout().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
                }
                let _ = writeln!(out, "{line}");
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
        Some(thread::spawn(move || {
            let reader = BufReader::new(pipe);
            let mut err = std::io::stderr().lock();
            for line in reader.lines() {
                let Ok(line) = line else { break };
                if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                    continue;
                }
                let _ = writeln!(err, "{line}");
            }
        }))
    } else {
        None
    };

    let exit_code = if let Some(seconds) = timeout {
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

    Ok(exit_code)
}

/// Wait for the child, forwarding SIGINT/SIGTERM on repeated Ctrl-C.
#[cfg(unix)]
fn wait_with_signal_handling(child: &mut Child) -> Result<i32> {
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
    Ok(exit_code_from_status(status))
}

#[cfg(not(unix))]
fn wait_with_signal_handling(child: &mut Child) -> Result<i32> {
    let status = child.wait()?;
    Ok(exit_code_from_status(status))
}

/// Wait for the child with a timeout, sending SIGTERM then SIGKILL.
#[cfg(unix)]
fn wait_with_timeout(child: &mut Child, seconds: u64) -> Result<i32> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(seconds);
    let grace_period = Duration::from_secs(5);

    loop {
        match child.try_wait()? {
            Some(status) => return Ok(exit_code_from_status(status)),
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
                            Some(status) => return Ok(exit_code_from_status(status)),
                            None => {
                                if Instant::now() >= kill_deadline {
                                    // Send SIGKILL
                                    unsafe {
                                        libc::kill(child.id() as i32, libc::SIGKILL);
                                    }
                                    let status = child.wait()?;
                                    return Ok(exit_code_from_status(status));
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
fn wait_with_timeout(child: &mut Child, seconds: u64) -> Result<i32> {
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(seconds);

    loop {
        match child.try_wait()? {
            Some(status) => return Ok(exit_code_from_status(status)),
            None => {
                if Instant::now() >= deadline {
                    child.kill()?;
                    let status = child.wait()?;
                    return Ok(exit_code_from_status(status));
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
) -> Result<CapturedChildOutput> {
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

    let exit_code = if let Some(seconds) = timeout {
        wait_with_timeout(&mut child, seconds)?
    } else {
        wait_with_signal_handling(&mut child)?
    };

    let stdout = stdout_handle.join().unwrap_or_default();
    let stderr = stderr_handle.join().unwrap_or_default();

    Ok(CapturedChildOutput {
        exit_code,
        stdout,
        stderr,
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
) -> Result<StreamExecutionSummary> {
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

        for line in reader.lines() {
            let Ok(line) = line else { break };

            if fallback_mode {
                // Fatal parse error: forward remaining raw stdout
                let _ = writeln!(out, "{line}");
                continue;
            }

            match parser.feed_line(&line) {
                Ok(Some(text)) => {
                    let _ = out.write_all(text.as_bytes());
                    let _ = out.flush();
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
                    fallback_mode = true;
                    let _ = writeln!(out, "{line}");
                }
            }
        }

        parser
    });

    // Stderr processing thread: filters noise prefixes and formats raw API errors.
    let pipe = child.stderr.take().expect("stderr was set to piped");
    let prefixes: Vec<String> = stderr_noise_prefixes
        .iter()
        .map(|s| s.to_string())
        .collect();
    let stderr_handle = thread::spawn(move || {
        let reader = BufReader::new(pipe);
        let mut captured = String::new();
        for line in reader.lines() {
            let Ok(line) = line else { break };
            if prefixes.iter().any(|p| line.starts_with(p.as_str())) {
                continue;
            }
            // Format raw API error JSON into human-readable messages
            let formatted = crate::output::try_format_api_error(&line);
            let output_line = formatted.as_deref().unwrap_or(&line);

            if suppress_stderr_on_success {
                if !captured.is_empty() {
                    captured.push('\n');
                }
                captured.push_str(output_line);
            } else {
                let mut err = std::io::stderr().lock();
                let _ = writeln!(err, "{output_line}");
            }
        }
        captured
    });

    let exit_code = if let Some(seconds) = timeout {
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

    Ok(summary)
}

/// Spawn a provider child process with structured stream parsing, capturing output.
///
/// Like `run_child_stream` but captures assistant text instead of printing.
/// Used by compose flows.
#[allow(dead_code)]
pub(crate) fn run_child_stream_capture(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    timeout: Option<u64>,
    stderr_noise_prefixes: &[&str],
    stdin_seed: Option<&str>,
    parser: Box<dyn StreamParser>,
) -> Result<StreamExecutionSummary> {
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

    let exit_code = if let Some(seconds) = timeout {
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

    Ok(summary)
}

/// Minimal fallback parser used when the real parser thread panics.
struct ErrorParser {
    exit_code: i32,
}

impl StreamParser for ErrorParser {
    fn feed_line(&mut self, _line: &str) -> std::result::Result<Option<String>, StreamParseError> {
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

        let summary = run_child_stream_capture(
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

        let summary = run_child_stream(
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

        assert_eq!(summary.exit_code, 0);
        assert_eq!(summary.assistant_text, "hello");
        assert!(summary.duration_ms.is_some());
        assert!(summary.duration_ms.unwrap() < 5_000);
    }
}
