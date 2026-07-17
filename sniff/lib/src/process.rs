//! Bounded subprocess execution.
//!
//! Every child process sniff spawns goes through [`run_with_timeout`]. It is the
//! single place that owns the deadline, the pipe draining, and the reaping, so a
//! wedged or verbose child can never wedge a detection.
//!
//! See `sniff/features/2026-07-16-performance/phases/06-remote-network-and-subprocess/spec.md`
//! for the contract this module implements.

use std::ffi::OsStr;
use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use tracing::warn;

use crate::performance::{self, counters};

/// Named per-probe deadlines.
///
/// These are policy. Changing one changes what sniff promises a caller about how
/// long a detection can block — treat an edit here as a policy change, never as
/// an incidental refactor.
pub(crate) mod timeouts {
    use std::time::Duration;

    /// Service-manager listing and enrichment commands.
    pub(crate) const SERVICE_COMMAND: Duration = Duration::from_secs(3);

    /// Windows `powershell` locale fallback.
    #[cfg_attr(not(target_os = "windows"), allow(dead_code))]
    pub(crate) const WINDOWS_LOCALE: Duration = Duration::from_secs(3);

    /// macOS `diskutil info`.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    pub(crate) const DISKUTIL: Duration = Duration::from_secs(5);

    /// Install-plan host verification probes.
    pub(crate) const HOST_CAPABILITY: Duration = Duration::from_secs(2);

    /// Program `--version` probes.
    pub(crate) const PROGRAM_SCHEMA: Duration = Duration::from_secs(3);

    /// NTP status queries.
    pub(crate) const NTP: Duration = Duration::from_secs(3);
}

/// How often the deadline is checked while the child runs.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Captured result of a completed child process.
#[derive(Debug)]
pub(crate) struct CapturedOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

impl CapturedOutput {
    /// Stdout as UTF-8, replacing invalid sequences.
    pub(crate) fn stdout_lossy(&self) -> std::borrow::Cow<'_, str> {
        String::from_utf8_lossy(&self.stdout)
    }
}

/// Why a bounded child failed to produce output.
#[derive(Debug)]
pub(crate) enum ProcessError {
    /// The executable could not be spawned (missing, not executable, ...).
    Spawn(std::io::Error),
    /// The child exceeded its deadline and was killed.
    Timeout,
}

impl std::fmt::Display for ProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(e) => write!(f, "could not spawn: {e}"),
            Self::Timeout => write!(f, "timed out"),
        }
    }
}

impl std::error::Error for ProcessError {}

impl From<ProcessError> for std::io::Error {
    fn from(err: ProcessError) -> Self {
        match err {
            ProcessError::Spawn(e) => e,
            ProcessError::Timeout => {
                std::io::Error::new(std::io::ErrorKind::TimedOut, "command timed out")
            }
        }
    }
}

/// Runs `program` with `args` under an explicit deadline, capturing both pipes.
///
/// The executable is invoked directly — never through a shell — so no argument is
/// subject to shell interpretation. Stdin is null, so a child that reads input
/// sees EOF rather than blocking on a terminal sniff may not own.
///
/// ## Returns
///
/// The child's [`CapturedOutput`] regardless of exit status; a non-zero exit is a
/// result, not an error, because several callers parse output from a failing
/// command. Use [`CapturedOutput::status`] to discriminate.
///
/// ## Errors
///
/// [`ProcessError::Spawn`] if the child could not start, [`ProcessError::Timeout`]
/// if it exceeded `timeout` (in which case it has been killed and reaped).
///
/// ## Notes
///
/// Both pipes are drained on dedicated threads while the parent polls the
/// deadline. This is the whole point of the helper: a `try_wait()` loop over an
/// undrained `Stdio::piped()` deadlocks any child that writes more than one pipe
/// buffer, because the child blocks in `write()` and so never exits for the loop
/// to observe.
pub(crate) fn run_with_timeout<S, A>(
    program: S,
    args: &[A],
    timeout: Duration,
) -> Result<CapturedOutput, ProcessError>
where
    S: AsRef<OsStr>,
    A: AsRef<OsStr>,
{
    let mut child = Command::new(program.as_ref())
        .args(args.iter().map(AsRef::as_ref))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(ProcessError::Spawn)?;
    performance::increment_counter(counters::PROC_SPAWNS, 1);

    // Drain both pipes concurrently with the wait. `take()` moves the handles out
    // so dropping `child` later cannot close them from under the readers.
    let stdout_handle = child.stdout.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        })
    });
    let stderr_handle = child.stderr.take().map(|mut pipe| {
        std::thread::spawn(move || {
            let mut buf = Vec::new();
            let _ = pipe.read_to_end(&mut buf);
            buf
        })
    });

    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                if start.elapsed() >= timeout {
                    performance::increment_counter(counters::PROC_TIMEOUTS, 1);
                    warn!(
                        program = %program.as_ref().to_string_lossy(),
                        timeout_ms = timeout.as_millis(),
                        "subprocess exceeded its deadline; killing"
                    );
                    let _ = child.kill();
                    // Reap, so the killed child cannot linger as a zombie.
                    let _ = child.wait();
                    return Err(ProcessError::Timeout);
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(ProcessError::Spawn(e));
            }
        }
    };

    // The child has exited, so its pipe write ends are closed and both readers see
    // EOF; joining cannot hang. A panicked reader degrades to empty output rather
    // than propagating.
    let stdout = stdout_handle
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();
    let stderr = stderr_handle
        .map(|h| h.join().unwrap_or_default())
        .unwrap_or_default();

    Ok(CapturedOutput {
        status,
        stdout,
        stderr,
    })
}

/// Runs a command under a deadline and returns stdout only on a successful exit.
///
/// The common shape for probes that treat any failure — spawn, non-zero exit, or
/// timeout — as "unavailable".
pub(crate) fn run_for_stdout<S, A>(program: S, args: &[A], timeout: Duration) -> Option<String>
where
    S: AsRef<OsStr>,
    A: AsRef<OsStr>,
{
    let out = run_with_timeout(program, args, timeout).ok()?;
    if !out.status.success() {
        return None;
    }
    String::from_utf8(out.stdout).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A child that outstrips its pipe buffer must still be captured whole.
    ///
    /// This is the regression the module exists for: the pre-Phase-6 `try_wait()`
    /// loops left stdout undrained, so this child would block in `write()` and be
    /// killed at the deadline with its output lost.
    #[cfg(unix)]
    #[test]
    fn output_larger_than_a_pipe_buffer_does_not_deadlock() {
        // 1 MiB — far beyond any platform's pipe buffer (64 KiB on Linux).
        let out = run_with_timeout(
            "sh",
            &["-c", "yes abcdefghijklmnopqrstuvwxyz | head -c 1048576"],
            Duration::from_secs(30),
        )
        .expect("child should complete, not time out");

        assert!(out.status.success());
        assert_eq!(out.stdout.len(), 1_048_576);
    }

    /// Both pipes drain concurrently, so a child filling stderr as well as stdout
    /// cannot wedge on either.
    #[cfg(unix)]
    #[test]
    fn both_pipes_drain_concurrently() {
        let out = run_with_timeout(
            "sh",
            &["-c", "yes out | head -c 200000 & yes err | head -c 200000 >&2; wait"],
            Duration::from_secs(30),
        )
        .expect("child should complete");

        assert_eq!(out.stdout.len(), 200_000);
        assert_eq!(out.stderr.len(), 200_000);
    }

    /// Tests inject a short deadline rather than sleeping for a production one.
    #[cfg(unix)]
    #[test]
    fn a_hung_child_is_killed_at_its_deadline() {
        let start = Instant::now();
        let result = run_with_timeout("sleep", &["30"], Duration::from_millis(200));

        assert!(matches!(result, Err(ProcessError::Timeout)));
        // Generous bound: asserts the deadline was honored, not the scheduler's precision.
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "killed at deadline, not after the child's own 30s"
        );
    }

    /// A timed-out child is waited on, so it cannot survive as a zombie.
    #[cfg(unix)]
    #[test]
    fn a_timed_out_child_is_reaped() {
        let result = run_with_timeout("sleep", &["30"], Duration::from_millis(100));
        assert!(matches!(result, Err(ProcessError::Timeout)));

        // If the child were unreaped it would remain our zombie; `wait`ing for any
        // child would then succeed. ECHILD ("no child processes") is the pass.
        // SAFETY: `waitpid` with WNOHANG only reports already-exited children.
        let rc = unsafe { libc::waitpid(-1, std::ptr::null_mut(), libc::WNOHANG) };
        assert_eq!(rc, -1, "no unreaped child should remain");
        assert_eq!(
            std::io::Error::last_os_error().raw_os_error(),
            Some(libc::ECHILD)
        );
    }

    /// A non-zero exit is a result, not an error — callers parse failing output.
    #[cfg(unix)]
    #[test]
    fn a_failing_exit_still_returns_captured_output() {
        let out = run_with_timeout("sh", &["-c", "echo oops >&2; exit 3"], Duration::from_secs(5))
            .expect("a non-zero exit is not a helper error");

        assert!(!out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "oops");
        assert_eq!(run_for_stdout("sh", &["-c", "exit 3"], Duration::from_secs(5)), None);
    }

    #[test]
    fn a_missing_executable_is_a_spawn_error() {
        let result = run_with_timeout(
            "sniff-nonexistent-program-xyz",
            &["--version"],
            Duration::from_secs(5),
        );
        assert!(matches!(result, Err(ProcessError::Spawn(_))));
    }

    /// Arguments reach the child verbatim — the helper never invokes a shell, so
    /// shell metacharacters are inert data.
    #[cfg(unix)]
    #[test]
    fn arguments_are_not_shell_interpreted() {
        let out = run_with_timeout("echo", &["$HOME; rm -rf /", "&&", "|"], Duration::from_secs(5))
            .expect("echo should run");

        assert_eq!(out.stdout_lossy().trim(), "$HOME; rm -rf / && |");
    }
}
