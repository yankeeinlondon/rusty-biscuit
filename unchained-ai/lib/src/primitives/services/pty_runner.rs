use std::io::{Read, Write};
use std::time::Duration;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use strip_ansi_escapes::strip;

use super::error::AgentStatusError;

/// Default timeout for PTY operations.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

/// ConPTY flushes remaining output asynchronously after the child exits, so
/// the reader may reach EOF noticeably later than `child.wait()` returns.
const OUTPUT_DRAIN_TIMEOUT: Duration = Duration::from_secs(5);

/// Cursor-position report answering ConPTY's startup query.
///
/// portable-pty creates every Windows pseudoconsole with
/// `PSEUDOCONSOLE_INHERIT_CURSOR`, which makes ConPTY emit `CSI 6 n` and stall
/// the child's startup until a cursor-position report arrives on the input
/// pipe. Nothing else here writes to the PTY at startup, so we must answer or
/// short-lived commands never run before the command timeout. ConPTY consumes
/// the report during initialization; it is never delivered to the child.
#[cfg(windows)]
const CONPTY_CURSOR_REPORT: &[u8] = b"\x1b[1;1R";

/// A step in an interactive PTY session.
#[derive(Debug, Clone)]
pub enum InteractiveStep {
    /// Write bytes to PTY stdin.
    Write(String),
    /// Sleep before the next step.
    Wait(Duration),
}

/// Runs a command in a PTY and returns the captured output with ANSI codes stripped.
///
/// ## Errors
///
/// Returns `AgentStatusError::PtySpawnError` if the PTY or command fails to spawn.
/// Returns `AgentStatusError::PtyReadError` if reading from the PTY fails.
/// Returns `AgentStatusError::PtyWriteError` if answering ConPTY's startup
/// cursor query fails (Windows only).
/// Returns `AgentStatusError::TimeoutError` if the command exceeds the timeout.
pub async fn run_pty_command(
    program: &str,
    args: &[&str],
    timeout_duration: Option<Duration>,
) -> Result<String, AgentStatusError> {
    let timeout_dur = timeout_duration.unwrap_or(DEFAULT_TIMEOUT);
    let program = program.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    tokio::task::spawn_blocking(move || run_pty_blocking(&program, &args, timeout_dur))
        .await
        .map_err(|e| AgentStatusError::PtySpawnError(format!("Task join error: {}", e)))?
}

fn run_pty_blocking(
    program: &str,
    args: &[String],
    timeout_dur: Duration,
) -> Result<String, AgentStatusError> {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AgentStatusError::PtySpawnError(format!("Failed to open PTY: {}", e)))?;

    let mut cmd = CommandBuilder::new(program);
    for arg in args {
        cmd.arg(arg);
    }

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| {
        AgentStatusError::PtySpawnError(format!("Failed to spawn '{}': {}", program, e))
    })?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| AgentStatusError::PtyReadError(format!("Failed to clone reader: {}", e)))?;

    drop(pair.slave);

    // Held until the session ends: `take_writer` removes the sole input-pipe
    // handle from the master, so dropping it here would signal EOF-on-input
    // to ConPTY while the child is still running.
    #[cfg(windows)]
    let _conpty_input = {
        let mut writer = pair
            .master
            .take_writer()
            .map_err(|e| AgentStatusError::PtyWriteError(format!("Failed to get writer: {}", e)))?;
        writer
            .write_all(CONPTY_CURSOR_REPORT)
            .and_then(|()| writer.flush())
            .map_err(|e| {
                AgentStatusError::PtyWriteError(format!(
                    "Failed to answer ConPTY cursor query: {}",
                    e
                ))
            })?;
        writer
    };

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut raw_output = Vec::new();
        let result = reader.read_to_end(&mut raw_output);
        tx.send((raw_output, result)).ok();
    });

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) => {}
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AgentStatusError::PtyReadError(format!(
                    "Failed to query child status: {}",
                    error
                )));
            }
        }

        if start.elapsed() >= timeout_dur {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AgentStatusError::TimeoutError(format!(
                "Command timed out after {}s",
                timeout_dur.as_secs()
            )));
        }

        thread::sleep(Duration::from_millis(10));
    }

    let _ = child.wait();
    drop(pair.master);

    let (raw_output, read_result) = rx.recv_timeout(OUTPUT_DRAIN_TIMEOUT).map_err(|_| {
        AgentStatusError::TimeoutError("Timed out waiting for PTY output shutdown".to_string())
    })?;
    read_result
        .map_err(|e| AgentStatusError::PtyReadError(format!("Failed to read PTY output: {}", e)))?;

    let stripped = strip(&raw_output);

    String::from_utf8(stripped)
        .map_err(|e| AgentStatusError::ParseError(format!("Output is not valid UTF-8: {}", e)))
}

/// Runs an interactive PTY session: spawns a program, executes a series of
/// write/wait steps, then kills the process and returns captured output.
///
/// Uses a wider terminal (120 cols) to avoid line wrapping in status output.
///
/// ## Errors
///
/// Returns `AgentStatusError::PtySpawnError` if the PTY or command fails to spawn.
/// Returns `AgentStatusError::PtyWriteError` if writing to stdin fails.
/// Returns `AgentStatusError::TimeoutError` if the overall session exceeds the timeout.
pub async fn run_pty_interactive(
    program: &str,
    steps: &[InteractiveStep],
    timeout_duration: Option<Duration>,
) -> Result<String, AgentStatusError> {
    let timeout_dur = timeout_duration.unwrap_or(Duration::from_secs(15));
    let program = program.to_string();
    let steps: Vec<InteractiveStep> = steps.to_vec();

    tokio::task::spawn_blocking(move || run_pty_interactive_blocking(&program, &steps, timeout_dur))
        .await
        .map_err(|e| AgentStatusError::PtySpawnError(format!("Task join error: {}", e)))?
}

fn run_pty_interactive_blocking(
    program: &str,
    steps: &[InteractiveStep],
    timeout_dur: Duration,
) -> Result<String, AgentStatusError> {
    use std::sync::mpsc;
    use std::thread;
    use std::time::Instant;

    let pty_system = native_pty_system();

    let pair = pty_system
        .openpty(PtySize {
            rows: 24,
            cols: 120,
            pixel_width: 0,
            pixel_height: 0,
        })
        .map_err(|e| AgentStatusError::PtySpawnError(format!("Failed to open PTY: {}", e)))?;

    let cmd = CommandBuilder::new(program);

    let mut child = pair.slave.spawn_command(cmd).map_err(|e| {
        AgentStatusError::PtySpawnError(format!("Failed to spawn '{}': {}", program, e))
    })?;

    let mut reader = pair
        .master
        .try_clone_reader()
        .map_err(|e| AgentStatusError::PtyReadError(format!("Failed to clone reader: {}", e)))?;

    let mut writer = pair
        .master
        .take_writer()
        .map_err(|e| AgentStatusError::PtyWriteError(format!("Failed to get writer: {}", e)))?;

    drop(pair.slave);

    #[cfg(windows)]
    writer
        .write_all(CONPTY_CURSOR_REPORT)
        .and_then(|()| writer.flush())
        .map_err(|e| {
            AgentStatusError::PtyWriteError(format!("Failed to answer ConPTY cursor query: {}", e))
        })?;

    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut output = Vec::new();
        let mut buf = [0u8; 4096];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => output.extend_from_slice(&buf[..n]),
                Err(_) => break,
            }
        }
        tx.send(output).ok();
    });

    let start = Instant::now();

    for step in steps {
        if start.elapsed() >= timeout_dur {
            let _ = child.kill();
            let _ = child.wait();
            return Err(AgentStatusError::TimeoutError(format!(
                "Interactive session timed out after {}s",
                timeout_dur.as_secs()
            )));
        }

        match step {
            InteractiveStep::Write(data) => {
                writer
                    .write_all(data.as_bytes())
                    .map_err(|e| AgentStatusError::PtyWriteError(format!("Write failed: {}", e)))?;
                writer
                    .flush()
                    .map_err(|e| AgentStatusError::PtyWriteError(format!("Flush failed: {}", e)))?;
            }
            InteractiveStep::Wait(duration) => {
                let remaining = timeout_dur.saturating_sub(start.elapsed());
                thread::sleep((*duration).min(remaining));
            }
        }
    }

    drop(writer);

    let _ = child.kill();
    let _ = child.wait();
    drop(pair.master);

    let raw_output = rx.recv_timeout(OUTPUT_DRAIN_TIMEOUT).map_err(|_| {
        AgentStatusError::TimeoutError("Timed out waiting for PTY output shutdown".to_string())
    })?;

    let stripped = strip(&raw_output);

    String::from_utf8(stripped)
        .map_err(|e| AgentStatusError::ParseError(format!("Output is not valid UTF-8: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    const ECHO_PROGRAM: &str = "echo";
    #[cfg(windows)]
    const ECHO_PROGRAM: &str = "cmd.exe";

    #[cfg(unix)]
    const ECHO_ARGS: &[&str] = &["hello world"];
    #[cfg(windows)]
    const ECHO_ARGS: &[&str] = &["/D", "/S", "/C", "echo hello world"];

    #[cfg(unix)]
    const ANSI_PROGRAM: &str = "printf";
    #[cfg(windows)]
    const ANSI_PROGRAM: &str = "cmd.exe";

    #[cfg(unix)]
    const ANSI_ARGS: &[&str] = &["\x1b[31mred\x1b[0m"];
    #[cfg(windows)]
    const ANSI_ARGS: &[&str] = &["/D", "/S", "/C", "echo \x1b[31mred\x1b[0m"];

    #[tokio::test]
    async fn test_run_echo_command() {
        let output = run_pty_command(ECHO_PROGRAM, ECHO_ARGS, None)
            .await
            .expect("echo command should succeed");
        assert!(output.trim().contains("hello world"));
    }

    #[tokio::test]
    async fn test_ansi_stripping() {
        let output = run_pty_command(ANSI_PROGRAM, ANSI_ARGS, None)
            .await
            .expect("ANSI-producing command should succeed");
        assert!(output.trim().contains("red"));
        assert!(!output.contains("\x1b["));
    }

    #[tokio::test]
    async fn test_nonexistent_command() {
        let result = run_pty_command("nonexistent_command_xyz", &[], None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[cfg(unix)]
    async fn test_interactive_cat() {
        let steps = vec![
            InteractiveStep::Wait(Duration::from_millis(100)),
            InteractiveStep::Write("hello interactive\n".to_string()),
            InteractiveStep::Wait(Duration::from_millis(200)),
        ];
        let result = run_pty_interactive("cat", &steps, Some(Duration::from_secs(3))).await;
        assert!(result.is_ok(), "Expected Ok, got: {:?}", result);
        let output = result.unwrap();
        assert!(
            output.contains("hello interactive"),
            "Output should contain written text, got: {}",
            output
        );
    }

    #[tokio::test]
    #[cfg(windows)]
    async fn test_interactive_command_shutdown() {
        let steps = vec![
            InteractiveStep::Wait(Duration::from_millis(100)),
            InteractiveStep::Write("exit\r\n".to_string()),
            InteractiveStep::Wait(Duration::from_millis(200)),
        ];
        run_pty_interactive("cmd.exe", &steps, Some(Duration::from_secs(3)))
            .await
            .expect("interactive command should shut down cleanly");
    }

    #[tokio::test]
    async fn test_interactive_nonexistent() {
        let steps = vec![InteractiveStep::Wait(Duration::from_millis(100))];
        let result = run_pty_interactive("nonexistent_command_xyz", &steps, None).await;
        assert!(result.is_err());
    }
}
