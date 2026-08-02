//! Shared command construction and process setup for the three spawn modes.
//!
//! The inherited-output, captured-output, and semantic-stream executors each
//! own their pipe/thread/parser wiring, but share the stable command build,
//! process-group isolation, stdin-seed write, and wall-clock ticker contracts
//! collected here.

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::Write;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use color_eyre::eyre::Result;

use super::super::TickerCancel;
use super::super::termination::WatchdogTermination;
use super::super::watchdog::spawn_wall_clock_timeout_ticker;

/// Windows process-creation flag that puts the child in a new process group
/// (the Win32 `CREATE_NEW_PROCESS_GROUP`). A new group is the prerequisite for
/// targeting the child tree with `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT,
/// group)` from the unified wait loop. Defined locally to avoid pulling the
/// `windows` crate into the `Command` build site.
#[cfg(windows)]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// Debug-assert that the child environment carries the critical variables.
///
/// The child is launched with `env_clear()` followed by `envs(env)`, so any
/// variable missing from `env` is absent from the child. Windows environment
/// names are case-insensitive and commonly expose `Path` and `USERPROFILE`;
/// the invariant follows those platform semantics rather than requiring Unix
/// spellings in the map.
pub(super) fn debug_assert_child_env(env: &HashMap<OsString, OsString>) {
    let contains = |name: &str| {
        #[cfg(windows)]
        {
            env.keys()
                .any(|key| key.to_string_lossy().eq_ignore_ascii_case(name))
        }
        #[cfg(not(windows))]
        {
            env.contains_key(&OsString::from(name))
        }
    };
    debug_assert!(
        contains("PATH"),
        "child env is missing PATH — env::build_child_env likely has a bug"
    );
    debug_assert!(
        contains("HOME") || cfg!(windows) && contains("USERPROFILE"),
        "child env is missing HOME — env::build_child_env likely has a bug"
    );
}

/// Build the base child command shared by every spawn mode.
///
/// The environment is applied with `env_clear()` then `envs(env)`, so `env`
/// must be the **complete** child environment — the only gate for environment
/// sanitization. Stdio and process-group isolation are configured by the
/// caller after this returns.
pub(super) fn base_command(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
) -> Command {
    let mut command = Command::new(binary);
    command.args(args).env_clear().envs(env).current_dir(cwd);
    command
}

/// Isolate the child into its own process group so the unified wait loop can
/// target the whole tree.
///
/// On Unix this sets `process_group(0)` (the pgid becomes the child pid); on
/// Windows it sets `CREATE_NEW_PROCESS_GROUP` so
/// `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, group)` and the Job Object can
/// reach descendants. The inherited-output path calls this only when it pipes
/// a stream (a background pgroup would give an interactive TUI SIGTTIN on
/// stdin read); the capture and semantic paths always isolate.
pub(super) fn isolate_into_process_group(command: &mut Command) {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(CREATE_NEW_PROCESS_GROUP);
    }
}

/// Write the stdin seed to the child, treating a `BrokenPipe` as benign.
///
/// A `BrokenPipe` means the child closed stdin (or exited) before we finished
/// writing the seed — a legitimate child behavior (agent stubs in tests,
/// providers that ignore their seed and exit immediately). Any other I/O error
/// propagates. Callers must spawn their reader threads **before** calling this
/// to avoid a pipe deadlock on prompts larger than the OS pipe buffer.
///
/// Dropping the taken stdin handle closes the pipe so the child sees EOF.
pub(super) fn write_stdin_seed(child: &mut Child, seed: &str) -> Result<()> {
    if let Some(mut stdin_pipe) = child.stdin.take() {
        match stdin_pipe.write_all(seed.as_bytes()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {}
            Err(e) => return Err(e.into()),
        }
    }
    Ok(())
}

/// Ticker handle (cancel flag + join handle) for teardown via
/// `stop_timing_ticker`; `None` when no `timeout` is configured.
type WallClockTicker = Option<(TickerCancel, JoinHandle<()>)>;

/// Watchdog receiver the wait loop polls for wall-clock breaches; `None` when
/// no `timeout` is configured.
type WallClockWatchdogRx = Option<Receiver<WatchdogTermination>>;

/// Spawn the wall-clock timeout ticker feeding the unified wait loop, if a
/// `timeout` is configured.
///
/// Returns the ticker handle (for teardown via `stop_timing_ticker`) and the
/// watchdog receiver the wait loop polls. Both are `None` when `timeout` is
/// `None`. Routing the timeout through the signal-aware loop is what keeps
/// Ctrl+C effective even when a `timeout` is set.
pub(super) fn wall_clock_timeout_ticker(
    timeout: Option<u64>,
    spawned_at: Instant,
) -> (WallClockTicker, WallClockWatchdogRx) {
    match timeout {
        Some(seconds) => {
            let (tx, rx) = std::sync::mpsc::channel();
            let ticker =
                spawn_wall_clock_timeout_ticker(Duration::from_secs(seconds), spawned_at, tx);
            (Some(ticker), Some(rx))
        }
        None => (None, None),
    }
}
