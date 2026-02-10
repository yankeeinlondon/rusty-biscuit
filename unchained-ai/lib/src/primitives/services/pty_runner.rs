use std::io::{Read, Write};
use std::time::Duration;

use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use strip_ansi_escapes::strip;

use super::error::AgentStatusError;

/// Default timeout for PTY operations.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(5);

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
/// Returns `AgentStatusError::TimeoutError` if the command exceeds the timeout.
pub async fn run_pty_command(
    program: &str,
    args: &[&str],
    timeout_duration: Option<Duration>,
) -> Result<String, AgentStatusError> {
    let timeout_dur = timeout_duration.unwrap_or(DEFAULT_TIMEOUT);
    let program = program.to_string();
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();

    tokio::task::spawn_blocking(move || {
        run_pty_blocking(&program, &args, timeout_dur)
    })
    .await
    .map_err(|e| AgentStatusError::PtySpawnError(
        format!("Task join error: {}", e)
    ))?
}

fn run_pty_blocking(
    program: &str,
    args: &[String],
    timeout_dur: Duration,
) -> Result<String, AgentStatusError> {
    use std::sync::mpsc;
    use std::thread;

    let pty_system = native_pty_system();

    let pair = pty_system.openpty(PtySize {
        rows: 24,
        cols: 80,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| AgentStatusError::PtySpawnError(
        format!("Failed to open PTY: {}", e)
    ))?;

    let mut cmd = CommandBuilder::new(program);
    for arg in args {
        cmd.arg(arg);
    }

    let mut child = pair.slave.spawn_command(cmd)
        .map_err(|e| AgentStatusError::PtySpawnError(
            format!("Failed to spawn '{}': {}", program, e)
        ))?;

    // Get reader before dropping slave
    let mut reader = pair.master.try_clone_reader()
        .map_err(|e| AgentStatusError::PtyReadError(
            format!("Failed to clone reader: {}", e)
        ))?;

    // CRITICAL: Drop slave so PTY gets EOF after child exits
    drop(pair.slave);

    // Read output in a separate thread to avoid blocking
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let mut raw_output = Vec::new();
        let result = reader.read_to_end(&mut raw_output);
        tx.send((raw_output, result)).ok();
    });

    // Wait for reader with timeout. The reader finishes when the child exits
    // (since slave is dropped, master reader gets EOF). If the child hangs,
    // recv_timeout fires and we kill the child to unblock the reader thread.
    let (raw_output, read_result) = match rx.recv_timeout(timeout_dur) {
        Ok(result) => result,
        Err(_) => {
            let _ = child.kill();
            return Err(AgentStatusError::TimeoutError(
                format!("Command timed out after {}s", timeout_dur.as_secs())
            ));
        }
    };

    // Reap the child process
    let _ = child.wait();

    read_result.map_err(|e| AgentStatusError::PtyReadError(
        format!("Failed to read PTY output: {}", e)
    ))?;

    // Strip ANSI escape sequences
    let stripped = strip(&raw_output);

    String::from_utf8(stripped)
        .map_err(|e| AgentStatusError::ParseError(
            format!("Output is not valid UTF-8: {}", e)
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_echo_command() {
        let result = run_pty_command("echo", &["hello world"], None).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.trim().contains("hello world"));
    }

    #[tokio::test]
    async fn test_ansi_stripping() {
        // echo with ANSI should have codes stripped
        let result = run_pty_command("printf", &[r"\033[31mred\033[0m"], None).await;
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.trim().contains("red"));
        assert!(!output.contains("\x1b["));
    }

    #[tokio::test]
    async fn test_nonexistent_command() {
        let result = run_pty_command("nonexistent_command_xyz", &[], None).await;
        assert!(result.is_err());
    }
}
