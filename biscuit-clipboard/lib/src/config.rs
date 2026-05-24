use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::ClipboardError;

pub const DEFAULT_PORT: u16 = 17530;
pub const CLIP_PORT_ENV: &str = "CLIP_PORT";
pub const CLIP_RUNTIME_DIR_ENV: &str = "CLIP_RUNTIME_DIR";
pub const PID_FILENAME: &str = "clipper.pid";
pub const PORT_FILENAME: &str = "clipper.port";

/// Watcher restart/backoff policy used by the supervisor.
///
/// The defaults match the spec: at most 3 retries, exponential backoff
/// starting at 1 second and capped at 30 seconds.
///
/// ## Environment Variables
///
/// - `CLIP_WATCHER_MAX_RETRIES` — max consecutive restart attempts
///   before the supervisor reports `Stopped`.
/// - `CLIP_WATCHER_BASE_DELAY_MS` — base delay (milliseconds) for the
///   exponential backoff.
/// - `CLIP_WATCHER_MAX_DELAY_MS` — ceiling (milliseconds) for the
///   exponential backoff.
#[derive(Clone, Debug)]
pub struct WatcherRestartPolicy {
    pub max_retries: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
}

impl WatcherRestartPolicy {
    pub fn from_env() -> Self {
        let mut policy = Self::default();
        if let Some(v) = std::env::var("CLIP_WATCHER_MAX_RETRIES")
            .ok()
            .and_then(|s| s.parse().ok())
        {
            policy.max_retries = v;
        }
        if let Some(v) = std::env::var("CLIP_WATCHER_BASE_DELAY_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            policy.base_delay = Duration::from_millis(v);
        }
        if let Some(v) = std::env::var("CLIP_WATCHER_MAX_DELAY_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
        {
            policy.max_delay = Duration::from_millis(v);
        }
        policy
    }
}

impl Default for WatcherRestartPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
        }
    }
}

pub fn configured_port() -> u16 {
    std::env::var(CLIP_PORT_ENV)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(DEFAULT_PORT)
}

/// Resolve the runtime directory for `clipper`'s pid/port files,
/// joining `biscuit-clipboard` onto the supplied base.
///
/// ## Notes
///
/// Tests use this with a `tempfile::TempDir` to avoid touching the
/// user's real runtime directory.
pub fn runtime_dir_in(base: &Path) -> PathBuf {
    base.join("biscuit-clipboard")
}

/// Resolve the runtime directory for `clipper`'s pid/port files.
///
/// ## Environment Variables
///
/// - `CLIP_RUNTIME_DIR` — when set, overrides the platform default
///   (used for tests and unusual deployments).
///
/// ### Priority Order
///
/// 1. `CLIP_RUNTIME_DIR`
/// 2. [`dirs::runtime_dir`]
/// 3. [`dirs::cache_dir`]
///
/// ## Errors
///
/// Returns [`ClipboardError::Backend`] if no base directory can be
/// determined.
pub fn runtime_dir() -> Result<PathBuf, ClipboardError> {
    if let Some(override_dir) = std::env::var_os(CLIP_RUNTIME_DIR_ENV)
        && !override_dir.is_empty()
    {
        return Ok(PathBuf::from(override_dir));
    }

    let base = dirs::runtime_dir()
        .or_else(dirs::cache_dir)
        .ok_or_else(|| {
            ClipboardError::Backend("Could not determine runtime directory".to_string())
        })?;
    Ok(runtime_dir_in(&base))
}

pub fn read_port_file() -> Option<u16> {
    let path = runtime_dir().ok()?.join(PORT_FILENAME);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

pub fn read_pid_file() -> Option<u32> {
    let path = runtime_dir().ok()?.join(PID_FILENAME);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Read the port file from a specific runtime directory.
///
/// ## Notes
///
/// The bare [`read_port_file`] resolves the runtime directory via
/// [`runtime_dir`]; tests typically want to point at a `TempDir`
/// without going through the env-var override.
pub fn read_port_file_in(runtime_dir: &Path) -> Option<u16> {
    let path = runtime_dir.join(PORT_FILENAME);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Read the pid file from a specific runtime directory.
///
/// ## Notes
///
/// The bare [`read_pid_file`] resolves the runtime directory via
/// [`runtime_dir`]; tests typically want to point at a `TempDir`
/// without going through the env-var override.
pub fn read_pid_file_in(runtime_dir: &Path) -> Option<u32> {
    let path = runtime_dir.join(PID_FILENAME);
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

/// Check whether a given OS process id is currently alive.
///
/// Uses [`sysinfo`] for cross-platform portability — including
/// Windows, where the historical `libc::kill(pid, 0)` shim does not
/// exist.
///
/// ## Returns
///
/// `true` if the process is currently visible to the OS, `false`
/// otherwise (including for already-reaped or never-existed pids).
pub fn is_pid_alive(pid: u32) -> bool {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate};

    let mut sys = sysinfo::System::new();
    let target = sysinfo::Pid::from_u32(pid);
    sys.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[target]),
        true,
        ProcessRefreshKind::nothing(),
    );
    sys.process(target).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    fn test_configured_port_default() {
        unsafe { std::env::remove_var(CLIP_PORT_ENV) };
        assert_eq!(configured_port(), DEFAULT_PORT);
    }

    #[test]
    fn test_runtime_dir_in_appends_biscuit_clipboard() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = runtime_dir_in(tmp.path());
        assert_eq!(dir, tmp.path().join("biscuit-clipboard"));
    }

    #[test]
    #[serial]
    fn test_runtime_dir_env_override() {
        let tmp = tempfile::tempdir().unwrap();
        // SAFETY: env mutation is serialized via serial_test
        unsafe { std::env::set_var(CLIP_RUNTIME_DIR_ENV, tmp.path()) };
        let dir = runtime_dir().unwrap();
        unsafe { std::env::remove_var(CLIP_RUNTIME_DIR_ENV) };
        assert_eq!(dir, tmp.path());
    }

    #[test]
    #[serial]
    fn test_read_port_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var(CLIP_RUNTIME_DIR_ENV, tmp.path()) };
        let result = read_port_file();
        unsafe { std::env::remove_var(CLIP_RUNTIME_DIR_ENV) };
        assert!(result.is_none());
    }

    #[test]
    #[serial]
    fn test_read_pid_file_missing() {
        let tmp = tempfile::tempdir().unwrap();
        unsafe { std::env::set_var(CLIP_RUNTIME_DIR_ENV, tmp.path()) };
        let result = read_pid_file();
        unsafe { std::env::remove_var(CLIP_RUNTIME_DIR_ENV) };
        assert!(result.is_none());
    }

    #[test]
    fn test_read_port_file_in_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(read_port_file_in(tmp.path()).is_none());
    }

    #[test]
    fn test_read_pid_file_in_missing() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(read_pid_file_in(tmp.path()).is_none());
    }

    #[test]
    fn test_read_port_file_in_present() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(PORT_FILENAME), "12345\n").unwrap();
        assert_eq!(read_port_file_in(tmp.path()), Some(12345));
    }

    #[test]
    fn test_read_pid_file_in_present() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join(PID_FILENAME), "42\n").unwrap();
        assert_eq!(read_pid_file_in(tmp.path()), Some(42));
    }

    #[test]
    fn test_is_pid_alive_current() {
        assert!(is_pid_alive(std::process::id()));
    }

    #[test]
    fn test_is_pid_alive_nonexistent() {
        // Using a deliberately huge pid that should not be in use on any
        // mainstream OS. This works on macOS, Linux, and Windows.
        assert!(!is_pid_alive(u32::MAX - 1));
    }

    #[cfg(unix)]
    #[test]
    fn test_is_pid_alive_after_child_exits() {
        use std::process::Command;
        let mut child = Command::new("true").spawn().expect("spawn true");
        let pid = child.id();
        child.wait().expect("wait child");
        // Child has been reaped; the pid is no longer alive.
        assert!(!is_pid_alive(pid));
    }
}
