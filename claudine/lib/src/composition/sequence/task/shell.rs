//! Running a task's approved shell commands and capturing their stdout.
//!
//! A task's commands arrive already resolved by preflight — approved bytes are
//! executed bytes — so nothing here interpolates. What it adds over the
//! lifecycle [`ShellRunner`](super::super::super::lifecycle::executor::ShellRunner)
//! is the two things a task needs and a lifecycle action does not: the command's
//! stdout (which becomes the task's `outputs` entry) and a per-command deadline.

use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// The default per-command budget when a task authors no `timeout:`.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the wait loop re-checks a running child.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// What one command produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandOutput {
    /// Everything the command wrote to stdout, undecorated.
    pub stdout: String,
    /// The process exit code. `-1` when the platform reported none.
    pub exit_code: i32,
    /// `true` when the command was killed for exceeding its budget. `exit_code`
    /// is then the kill's code and carries no information about the work.
    pub timed_out: bool,
    /// `true` when the command was killed because the user interrupted the run.
    ///
    /// This is how Ctrl+C reaches a *running* child rather than only the gap
    /// between commands: every sibling in a parallel group watches the same flag,
    /// so one press fans out to all of them.
    pub interrupted: bool,
}

/// Runs one approved command under a deadline, capturing stdout.
///
/// Injectable so task tests assert dispatch, byte parity, and ordering without
/// spawning processes.
pub trait TaskShellRunner: Sync {
    /// Run `command`, killing it after `timeout` or as soon as `interrupt` is
    /// set.
    ///
    /// ## Errors
    ///
    /// Returns the underlying [`std::io::Error`] only when the process could
    /// not be spawned or waited on. A command that ran and failed reports its
    /// code through [`ShellCommandOutput::exit_code`].
    fn run(
        &self,
        command: &str,
        timeout: Duration,
        interrupt: Option<&AtomicBool>,
    ) -> Result<ShellCommandOutput, std::io::Error>;
}

/// Production [`TaskShellRunner`]: the platform's system shell, stdout piped,
/// stderr inherited so a command's diagnostics stay visible to the operator.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemTaskShell;

impl TaskShellRunner for SystemTaskShell {
    fn run(
        &self,
        command: &str,
        timeout: Duration,
        interrupt: Option<&AtomicBool>,
    ) -> Result<ShellCommandOutput, std::io::Error> {
        let mut child = system_shell_command(command)
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .stdin(Stdio::null())
            .spawn()?;

        // stdout is drained on its own thread: a command that fills the pipe
        // buffer would otherwise block forever and the deadline below would
        // report a timeout for what is really a reader that never ran.
        let mut pipe = child.stdout.take().expect("stdout was piped");
        let reader = std::thread::spawn(move || {
            let mut buffer = Vec::new();
            let _ = pipe.read_to_end(&mut buffer);
            buffer
        });

        let start = Instant::now();
        let mut timed_out = false;
        let mut interrupted = false;
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            // `kill` rather than a signal: it is the one termination primitive
            // with identical semantics on macOS, Linux, and Windows.
            if interrupt.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                interrupted = true;
                let _ = child.kill();
                break child.wait()?;
            }
            if start.elapsed() >= timeout {
                timed_out = true;
                let _ = child.kill();
                break child.wait()?;
            }
            std::thread::sleep(POLL_INTERVAL);
        };

        let bytes = reader.join().unwrap_or_default();
        Ok(ShellCommandOutput {
            stdout: String::from_utf8_lossy(&bytes).into_owned(),
            exit_code: status.code().unwrap_or(-1),
            timed_out,
            interrupted,
        })
    }
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(windows)]
fn system_shell_command(command: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(not(windows))]
fn system_shell_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}
