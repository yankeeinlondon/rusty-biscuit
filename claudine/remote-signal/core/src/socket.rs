//! Helpers for resolving the Unix Domain Socket path used for IPC between
//! the remote-signal daemon and its local clients.
//!
//! Resolution order (first match wins):
//!
//! 1. The `REMOTE_SIGNAL_SOCKET` environment variable, if set and non-empty.
//! 2. `$XDG_RUNTIME_DIR/remote-signal/daemon.sock`, when `XDG_RUNTIME_DIR`
//!    is set (typical on Linux).
//! 3. `$TMPDIR/remote-signal-<uid-or-user>/daemon.sock`, falling back to
//!    `/tmp/...` when `TMPDIR` is unset (typical on macOS and other Unix).
//!
//! Tests and the test client can override the path with the env var to
//! avoid clashing with a running daemon.

use std::env;
use std::path::{Path, PathBuf};

/// Environment variable that overrides the IPC socket path.
pub const SOCKET_ENV_VAR: &str = "REMOTE_SIGNAL_SOCKET";

/// Default socket file name within the resolved parent directory.
pub const SOCKET_FILE_NAME: &str = "daemon.sock";

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

/// Resolve the socket path using the standard precedence rules.
///
/// This is a pure function: it inspects environment variables but does not
/// touch the file system. Use [`ensure_parent_dir`] before binding the
/// returned path with a `UnixListener`.
#[must_use]
pub fn default_socket_path() -> PathBuf {
    if let Some(explicit) = env::var_os(SOCKET_ENV_VAR)
        && !explicit.is_empty()
    {
        return PathBuf::from(explicit);
    }

    if let Some(runtime) = env::var_os("XDG_RUNTIME_DIR")
        && !runtime.is_empty()
    {
        return PathBuf::from(runtime)
            .join("remote-signal")
            .join(SOCKET_FILE_NAME);
    }

    let tmp = env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/tmp"));
    let user_segment = env::var("USER")
        .ok()
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "default".to_string());
    tmp.join(format!("remote-signal-{user_segment}"))
        .join(SOCKET_FILE_NAME)
}

/// Ensure the parent directory of `socket_path` exists (creating it
/// recursively if necessary). Returns the directory that was created or
/// confirmed to exist.
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
            let keys = [SOCKET_ENV_VAR, "XDG_RUNTIME_DIR", "TMPDIR", "USER"]
                .into_iter()
                .map(|k| (k, env::var_os(k)))
                .collect();
            // SAFETY: tests in this module are serialised by `env_guard`.
            unsafe {
                env::remove_var(SOCKET_ENV_VAR);
                env::remove_var("XDG_RUNTIME_DIR");
                env::remove_var("TMPDIR");
                env::remove_var("USER");
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
                // SAFETY: tests in this module are serialised by `env_guard`.
                unsafe {
                    match value {
                        Some(v) => env::set_var(key, v),
                        None => env::remove_var(key),
                    }
                }
            }
        }
    }

    #[test]
    fn explicit_env_var_wins() {
        let _snapshot = EnvSnapshot::new();
        // SAFETY: serialised by `EnvSnapshot`.
        unsafe {
            env::set_var(SOCKET_ENV_VAR, "/tmp/custom-rs.sock");
        }
        assert_eq!(default_socket_path(), PathBuf::from("/tmp/custom-rs.sock"));
    }

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
            PathBuf::from("/run/user/1000/remote-signal/daemon.sock"),
        );
    }

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
            PathBuf::from("/var/folders/abc/remote-signal-alice/daemon.sock"),
        );
    }

    #[test]
    fn ensure_parent_dir_creates_nested_directories() {
        let tmp = std::env::temp_dir().join(format!(
            "remote-signal-test-{}-{}",
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
