//! Shared process spawning utilities for notification helpers.
//!
//! Helpers all spawn one-shot child processes via `tokio::process::Command`.
//! This module centralizes timeout handling, stderr capture, and exit-code
//! classification so each helper's `send()` stays focused on argv
//! construction and output parsing.

use std::ffi::OsString;
use std::path::Path;
use std::time::Duration;

use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio::time::timeout;

use super::HelperError;

/// Result of running a helper subprocess.
///
/// Captured stdout/stderr are returned as `String` because every helper we
/// integrate emits UTF-8 (the Linux/macOS/Windows CLIs ship locale-friendly
/// output). Helpers that may emit binary data should adapt this struct
/// directly rather than parsing through it.
#[derive(Debug, Clone)]
pub(crate) struct HelperOutput {
    /// Process exit status code. `None` only when the platform fails to
    /// produce one (e.g. killed by signal); we map this to `-1`.
    pub status: i32,
    /// Captured stdout (UTF-8, lossy on invalid bytes).
    pub stdout: String,
    /// Captured stderr (UTF-8, lossy on invalid bytes).
    pub stderr: String,
}

/// Spawn `program` with `args`, optionally piping `stdin_payload` over
/// stdin, and wait for completion subject to `timeout_ms`.
///
/// ## Errors
///
/// - [`HelperError::NotPresent`] when the binary cannot be launched
///   (`ErrorKind::NotFound`).
/// - [`HelperError::Timeout`] when the call exceeds `timeout_ms` (the child
///   is killed before returning).
/// - [`HelperError::Io`] for any other spawn/wait failure.
pub(crate) async fn spawn_helper(
    program: &Path,
    args: &[OsString],
    stdin_payload: Option<&str>,
    timeout_ms: Option<u64>,
) -> Result<HelperOutput, HelperError> {
    let mut command = Command::new(program);
    command.args(args);
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());
    if stdin_payload.is_some() {
        command.stdin(std::process::Stdio::piped());
    } else {
        command.stdin(std::process::Stdio::null());
    }

    let mut child = command.spawn().map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            HelperError::NotPresent
        } else {
            HelperError::Io(error)
        }
    })?;

    if let Some(payload) = stdin_payload
        && let Some(mut stdin) = child.stdin.take()
    {
        stdin.write_all(payload.as_bytes()).await?;
        stdin.flush().await?;
        drop(stdin);
    }

    let wait_future = child.wait_with_output();
    let output = match timeout_ms {
        Some(ms) => match timeout(Duration::from_millis(ms), wait_future).await {
            Ok(result) => result?,
            Err(_) => {
                return Err(HelperError::Timeout { timeout_ms: ms });
            }
        },
        None => wait_future.await?,
    };

    let status = output.status.code().unwrap_or(-1);
    Ok(HelperOutput {
        status,
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    })
}

/// Convenience wrapper for the common helper convention: spawn, expect exit
/// code zero, return parsed stdout. Non-zero status maps to
/// [`HelperError::Exited`] using the captured stderr.
pub(crate) async fn run_expect_success(
    program: &Path,
    args: &[OsString],
    stdin_payload: Option<&str>,
    timeout_ms: Option<u64>,
) -> Result<HelperOutput, HelperError> {
    let output = spawn_helper(program, args, stdin_payload, timeout_ms).await?;
    if output.status != 0 {
        return Err(HelperError::Exited {
            status: output.status,
            stderr: first_line(&output.stderr).to_string(),
        });
    }
    Ok(output)
}

/// Truncate a multi-line error blob to its first non-empty line.
///
/// Helper stderr is often a multi-line trace; the first line is the most
/// useful summary for `helper_fallbacks` metadata.
pub(crate) fn first_line(stderr: &str) -> &str {
    stderr
        .lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn echo_path() -> PathBuf {
        PathBuf::from("/bin/echo")
    }

    #[tokio::test]
    async fn spawn_captures_stdout() {
        let output = spawn_helper(&echo_path(), &[OsString::from("hello")], None, Some(500))
            .await
            .expect("echo should run");
        assert_eq!(output.status, 0);
        assert_eq!(output.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn missing_binary_maps_to_not_present() {
        let result = spawn_helper(
            &PathBuf::from("/definitely/not/a/real/binary"),
            &[],
            None,
            Some(500),
        )
        .await;
        assert!(matches!(result, Err(HelperError::NotPresent)));
    }

    #[tokio::test]
    async fn timeout_kills_long_running_child() {
        let result = spawn_helper(
            &PathBuf::from("/bin/sleep"),
            &[OsString::from("5")],
            None,
            Some(50),
        )
        .await;
        assert!(matches!(result, Err(HelperError::Timeout { .. })));
    }

    #[test]
    fn first_line_skips_blank() {
        assert_eq!(first_line("\n  \nfirst real\nignored"), "first real");
        assert_eq!(first_line(""), "");
    }
}
