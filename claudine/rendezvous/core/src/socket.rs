//! Helpers for resolving the local IPC endpoint used between the rendezvous
//! daemon and its clients.
//!
//! Resolution order (first match wins):
//!
//! 1. The `RENDEZVOUS_SOCKET` environment variable, if set and non-empty.
//!    On Unix this is the filesystem path of the Unix Domain Socket; on
//!    Windows it is the named-pipe path (e.g. `\\.\pipe\rendezvous-daemon`).
//! 2. The platform default:
//!    - Unix: `$XDG_RUNTIME_DIR/rendezvous/daemon.sock`, or
//!      `$TMPDIR/rendezvous-<user>/daemon.sock` (falling back to `/tmp/...`).
//!    - Windows: `\\.\pipe\rendezvous-daemon`.
//!
//! Tests and the test client can override the endpoint with the env var to
//! avoid clashing with a running daemon.

use std::env;
use std::path::{Path, PathBuf};

use crate::local_endpoint::LocalEndpoint;

/// Environment variable that overrides the IPC endpoint (UDS path on Unix,
/// named-pipe name on Windows).
pub const SOCKET_ENV_VAR: &str = "RENDEZVOUS_SOCKET";

/// Tag a legacy path-shaped endpoint with the transport it always implied.
///
/// The path-shaped API decided the transport by compile target and never said
/// so; this reproduces that decision exactly, so a caller still on
/// [`default_socket_path`] keeps the endpoint it has today while the typed
/// connector lands. It is a migration seam, not a general conversion — a
/// `PathBuf` is not evidence of a transport, which is the whole reason
/// [`LocalEndpoint`] exists. Phase 6 removes this module and its callers
/// together.
#[must_use]
pub fn legacy_local_endpoint(path: PathBuf) -> LocalEndpoint {
    #[cfg(unix)]
    {
        LocalEndpoint::UnixSocket(path)
    }
    #[cfg(windows)]
    {
        LocalEndpoint::WindowsNamedPipe(path.into_os_string())
    }
}

/// Default socket file name within the resolved parent directory (Unix only).
#[cfg(unix)]
pub const SOCKET_FILE_NAME: &str = "daemon.sock";

/// Default Windows named-pipe name (without the `\\.\pipe\` prefix).
#[cfg(windows)]
pub const DEFAULT_PIPE_NAME: &str = "rendezvous-daemon";

/// Errors that can occur while resolving or preparing the socket path.
#[derive(Debug, thiserror::Error)]
pub enum SocketPathError {
    /// The parent directory for the socket could not be created.
    #[error("failed to create socket parent directory {path}: {source}")]
    CreateParent {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

/// Resolve the IPC endpoint using the standard precedence rules.
///
/// This is a pure function: it inspects environment variables but does not
/// touch the file system. On Unix use [`ensure_parent_dir`] before binding
/// the returned path with a `UnixListener`. On Windows the returned path is
/// a pipe name and needs no filesystem preparation.
#[must_use]
pub fn default_socket_path() -> PathBuf {
    if let Some(explicit) = env::var_os(SOCKET_ENV_VAR)
        && !explicit.is_empty()
    {
        return PathBuf::from(explicit);
    }

    #[cfg(unix)]
    {
        default_socket_path_unix()
    }
    #[cfg(windows)]
    {
        default_socket_path_windows()
    }
    #[cfg(not(any(unix, windows)))]
    {
        PathBuf::from(String::new())
    }
}

/// Unix-only default UDS path resolution.
#[cfg(unix)]
fn default_socket_path_unix() -> PathBuf {
    if let Some(runtime) = env::var_os("XDG_RUNTIME_DIR")
        && !runtime.is_empty()
    {
        return PathBuf::from(runtime)
            .join("rendezvous")
            .join(SOCKET_FILE_NAME);
    }

    let tmp = env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let user_segment = env::var("USER")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default".to_string());
    tmp.join(format!("rendezvous-{user_segment}"))
        .join(SOCKET_FILE_NAME)
}

/// Windows-only default named-pipe path resolution.
#[cfg(windows)]
fn default_socket_path_windows() -> PathBuf {
    PathBuf::from(format!(r"\\.\pipe\{DEFAULT_PIPE_NAME}"))
}

/// Ensure the parent directory of `socket_path` exists (creating it
/// recursively if necessary). Returns the directory that was created or
/// confirmed to exist. Only meaningful on Unix where the endpoint is a
/// filesystem path.
pub fn ensure_parent_dir(socket_path: &Path) -> Result<PathBuf, SocketPathError> {
    let Some(parent) = socket_path.parent() else {
        return Ok(PathBuf::from("."));
    };

    if parent.as_os_str().is_empty() {
        return Ok(PathBuf::from("."));
    }

    std::fs::create_dir_all(parent).map_err(|source| SocketPathError::CreateParent {
        path: parent.to_path_buf(),
        source,
    })?;
    Ok(parent.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Mutex guard to keep env-var-mutating tests serialised within this
    /// module. `cargo test`/`cargo nextest` run integration tests in
    /// parallel processes, but unit tests inside the same binary share a
    /// process, so we must keep mutations from racing.
    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        use std::sync::{Mutex, OnceLock};
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Captures env vars touched by [`default_socket_path`] and restores
    /// them on drop so each test starts from a known clean baseline.
    struct EnvSnapshot {
        keys: Vec<(&'static str, Option<std::ffi::OsString>)>,
        _guard: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvSnapshot {
        fn new() -> Self {
            let guard = env_guard();
            let mut keys = vec![(SOCKET_ENV_VAR, env::var_os(SOCKET_ENV_VAR))];
            #[cfg(unix)]
            {
                for k in ["XDG_RUNTIME_DIR", "TMPDIR", "USER"] {
                    keys.push((k, env::var_os(k)));
                }
            }
            // SAFETY: tests in this module are serialised by `env_guard`.
            unsafe {
                env::remove_var(SOCKET_ENV_VAR);
                #[cfg(unix)]
                {
                    env::remove_var("XDG_RUNTIME_DIR");
                    env::remove_var("TMPDIR");
                    env::remove_var("USER");
                }
            }
            Self {
                keys,
                _guard: guard,
            }
        }
    }

    impl Drop for EnvSnapshot {
        fn drop(&mut self) {
            for (key, value) in &self.keys {
                // SAFETY: tests in this module are serialised by `EnvSnapshot`.
                unsafe {
                    match value {
                        Some(v) => env::set_var(key, v),
                        None => env::remove_var(key),
                    }
                }
            }
        }
    }

    /// `RENDEZVOUS_SOCKET` overrides the endpoint verbatim on every platform.
    /// This is the cross-platform contract: setting it to a sentinel value
    /// surfaces as-is on both Unix and Windows.
    #[test]
    fn explicit_env_var_wins() {
        let _snapshot = EnvSnapshot::new();
        // SAFETY: serialised by `EnvSnapshot`.
        unsafe {
            env::set_var(SOCKET_ENV_VAR, "/tmp/custom-rs.sock");
        }
        assert_eq!(default_socket_path(), PathBuf::from("/tmp/custom-rs.sock"));
    }

    #[cfg(unix)]
    #[test]
    fn xdg_runtime_dir_takes_precedence_over_tmpdir() {
        let _snapshot = EnvSnapshot::new();
        // SAFETY: serialised by `EnvSnapshot`.
        unsafe {
            env::set_var("XDG_RUNTIME_DIR", "/run/user/1000");
            env::set_var("TMPDIR", "/var/tmp/ignored");
        }
        assert_eq!(
            default_socket_path(),
            PathBuf::from("/run/user/1000/rendezvous/daemon.sock"),
        );
    }

    #[cfg(unix)]
    #[test]
    fn falls_back_to_tmpdir_with_user_segment() {
        let _snapshot = EnvSnapshot::new();
        // SAFETY: serialised by `EnvSnapshot`.
        unsafe {
            env::set_var("TMPDIR", "/var/folders/abc");
            env::set_var("USER", "alice");
        }
        assert_eq!(
            default_socket_path(),
            PathBuf::from("/var/folders/abc/rendezvous-alice/daemon.sock"),
        );
    }

    /// Windows default endpoint is a named pipe under `\\.\pipe\`. This test
    /// only runs on a Windows host but the assertion documents the contract.
    #[cfg(windows)]
    #[test]
    fn windows_default_is_named_pipe() {
        let _snapshot = EnvSnapshot::new();
        assert_eq!(
            default_socket_path(),
            PathBuf::from(r"\\.\pipe\rendezvous-daemon"),
        );
    }

    /// On Windows the `RENDEZVOUS_SOCKET` override is the pipe name verbatim.
    #[cfg(windows)]
    #[test]
    fn windows_env_var_overrides_pipe_name() {
        let _snapshot = EnvSnapshot::new();
        // SAFETY: serialised by `EnvSnapshot`.
        unsafe {
            env::set_var(SOCKET_ENV_VAR, r"\\.\pipe\custom-rs");
        }
        assert_eq!(default_socket_path(), PathBuf::from(r"\\.\pipe\custom-rs"));
    }

    #[test]
    fn ensure_parent_dir_creates_nested_directories() {
        let tmp = std::env::temp_dir().join(format!(
            "rendezvous-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or_default(),
        ));
        let socket = tmp.join("nested").join("daemon.sock");

        let created = ensure_parent_dir(&socket).expect("create parent");
        assert!(created.is_dir());
        assert_eq!(created, tmp.join("nested"));

        std::fs::remove_dir_all(&tmp).ok();
    }
}
