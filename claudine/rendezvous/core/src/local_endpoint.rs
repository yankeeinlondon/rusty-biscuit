//! The local control-plane endpoint shared by the rendezvous daemon and its
//! clients.
//!
//! The local plane is platform-native: a Unix-domain **stream** socket on
//! macOS, Linux, and WSL, and a Windows named pipe on native Windows. Those
//! two are not interchangeable and are not both filesystem paths, so
//! [`LocalEndpoint`] keeps them as separate variants rather than as one string
//! a caller has to guess the transport from.
//!
//! Resolution order for [`default_local_endpoint`] (first match wins):
//!
//! 1. `RENDEZVOUS_ENDPOINT`, parsed and validated for the current target.
//! 2. The per-user default, qualified by the stable OS user identity from
//!    [`sniff::os::current_user_id`]:
//!    - Linux/WSL with a usable `$XDG_RUNTIME_DIR`:
//!      `$XDG_RUNTIME_DIR/claudine/rendezvous/daemon.sock`;
//!    - other Unix (and Unix without a usable runtime directory):
//!      `<tempdir>/claudine-rendezvous-uid-<uid>/daemon.sock`;
//!    - Windows: `\\.\pipe\claudine-rendezvous-sid-<sid>`.
//!
//! ## Notes
//!
//! Resolution never mutates the filesystem. It inspects type, symlink status,
//! ownership, and mode to decide whether the runtime directory is usable, but
//! creating, chmod-ing, and removing directories and endpoints belongs to the
//! daemon's transport layer, which is the only component that binds them.
//!
//! Failure is typed. There is no username, `default`, or process-random
//! fallback: a process that cannot learn who it is must not go on to create
//! per-user private state.

use std::env;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use sniff::os::StableUserId;

#[cfg(not(any(unix, windows)))]
compile_error!(
    "the rendezvous local control plane requires Unix domain sockets or Windows named pipes"
);

/// Environment variable that overrides the local endpoint.
///
/// The value is an OS-native string: a Unix socket path on Unix, a
/// `\\.\pipe\...` name on Windows. It is never reinterpreted for the other
/// target.
pub const ENDPOINT_ENV_VAR: &str = "RENDEZVOUS_ENDPOINT";

/// Socket file name inside the per-user Rendezvous runtime directory (Unix).
#[cfg(unix)]
pub const ENDPOINT_FILE_NAME: &str = "daemon.sock";

/// Shared stem of every default endpoint name, on both transports.
pub const ENDPOINT_STEM: &str = "claudine-rendezvous";

/// The only prefix a Windows named pipe on the local machine may carry.
#[cfg(windows)]
pub const PIPE_NAME_PREFIX: &str = r"\\.\pipe\";

/// Prefixes that identify a Windows named-pipe name on any target.
///
/// Matched on every target so a Unix host can recognize — and reject — a pipe
/// name instead of quietly treating it as a relative file path.
const PIPE_PREFIXES: [&[u8]; 2] = [br"\\.\pipe\", br"\\?\pipe\"];

/// Failures resolving or validating a local endpoint.
#[derive(Debug, thiserror::Error)]
pub enum LocalEndpointError {
    /// `RENDEZVOUS_ENDPOINT` was set to an empty value. This is a
    /// configuration mistake rather than a request for the default, so it is
    /// reported instead of silently ignored.
    #[error("{ENDPOINT_ENV_VAR} is set but empty; unset it to use the per-user default")]
    EmptyOverride,

    /// The endpoint has the right transport but is not a usable name for it.
    #[error("{ENDPOINT_ENV_VAR} value '{value}' is not a usable {expected}: {reason}")]
    MalformedOverride {
        /// Lossy rendering of the rejected value, for the message only.
        value: String,
        /// What the current target needs, e.g. `Unix socket path`.
        expected: &'static str,
        /// Why the value cannot serve as that.
        reason: &'static str,
    },

    /// The endpoint names a transport this target cannot speak.
    #[error("local endpoint '{endpoint}' is {actual}, but this target requires {expected}")]
    IncompatibleTransport {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        /// What the endpoint actually is.
        actual: &'static str,
        /// What the current target requires.
        expected: &'static str,
    },

    /// The stable OS user could not be discovered, so no per-user endpoint can
    /// be derived.
    #[error("cannot derive a per-user local endpoint without a stable OS user identity")]
    UserIdentity(#[from] sniff::SniffError),

    /// The host reported an identity of the wrong kind for this target's
    /// transport. Unreachable in practice; kept typed so a future identity
    /// source cannot silently produce a mis-qualified endpoint.
    #[error("the host reported {actual}, which cannot qualify {expected}")]
    MismatchedIdentity {
        /// The identity kind actually returned.
        actual: &'static str,
        /// The identity kind this target's endpoint needs.
        expected: &'static str,
    },
}

/// A local control-plane endpoint, tagged with its transport.
///
/// ## Notes
///
/// There is deliberately no common `path()` accessor. A Windows pipe name is
/// not a filesystem path — it is never opened, stat-ed, or removed with
/// filesystem calls — so an accessor that returned one for both variants would
/// invite exactly the confusion the type exists to prevent.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum LocalEndpoint {
    /// A Unix-domain stream socket at a filesystem path.
    UnixSocket(PathBuf),
    /// A Windows named pipe, e.g. `\\.\pipe\claudine-rendezvous-sid-S-1-5-...`.
    ///
    /// Held as an `OsString` and passed to Win32 as one: routing it through
    /// `PathBuf`/`to_string_lossy` would corrupt a non-UTF-16-representable
    /// name and imply filesystem semantics it does not have.
    WindowsNamedPipe(OsString),
}

impl LocalEndpoint {
    /// The socket path, or `None` if this endpoint is a named pipe.
    #[must_use]
    pub fn as_unix_path(&self) -> Option<&Path> {
        match self {
            LocalEndpoint::UnixSocket(path) => Some(path),
            LocalEndpoint::WindowsNamedPipe(_) => None,
        }
    }

    /// The pipe name, or `None` if this endpoint is a Unix socket.
    #[must_use]
    pub fn as_windows_pipe_name(&self) -> Option<&OsStr> {
        match self {
            LocalEndpoint::WindowsNamedPipe(name) => Some(name),
            LocalEndpoint::UnixSocket(_) => None,
        }
    }

    /// Whether the current compilation target can bind or connect this
    /// endpoint.
    #[must_use]
    pub fn is_native(&self) -> bool {
        matches!(
            (cfg!(unix), self),
            (true, LocalEndpoint::UnixSocket(_)) | (false, LocalEndpoint::WindowsNamedPipe(_))
        )
    }

    /// Check that this endpoint's transport exists on the current target.
    ///
    /// ## Errors
    ///
    /// Returns [`LocalEndpointError::IncompatibleTransport`] for a named pipe
    /// on Unix or a Unix socket on Windows.
    pub fn ensure_native(&self) -> Result<(), LocalEndpointError> {
        if self.is_native() {
            return Ok(());
        }
        Err(LocalEndpointError::IncompatibleTransport {
            endpoint: self.to_string(),
            actual: self.transport_label(),
            expected: native_transport_label(),
        })
    }

    /// Parse an OS-native override value for the current target.
    ///
    /// ## Errors
    ///
    /// Returns [`LocalEndpointError::EmptyOverride`] for an empty value,
    /// [`LocalEndpointError::IncompatibleTransport`] when the value names the
    /// other target's transport, and
    /// [`LocalEndpointError::MalformedOverride`] when it names this target's
    /// transport but cannot be used as one.
    pub fn from_override(value: &OsStr) -> Result<Self, LocalEndpointError> {
        if value.is_empty() {
            return Err(LocalEndpointError::EmptyOverride);
        }

        #[cfg(unix)]
        {
            parse_unix_override(value)
        }
        #[cfg(windows)]
        {
            parse_windows_override(value)
        }
    }

    fn transport_label(&self) -> &'static str {
        match self {
            LocalEndpoint::UnixSocket(_) => "a Unix domain socket",
            LocalEndpoint::WindowsNamedPipe(_) => "a Windows named pipe",
        }
    }
}

/// Human-facing rendering only.
///
/// The output is lossy for a non-Unicode endpoint. Dispatch on the variant, or
/// use [`LocalEndpoint::as_unix_path`] / [`LocalEndpoint::as_windows_pipe_name`];
/// never round-trip an endpoint through this projection.
impl std::fmt::Display for LocalEndpoint {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalEndpoint::UnixSocket(path) => write!(f, "{}", path.display()),
            LocalEndpoint::WindowsNamedPipe(name) => write!(f, "{}", name.to_string_lossy()),
        }
    }
}

fn native_transport_label() -> &'static str {
    if cfg!(unix) {
        "a Unix domain socket"
    } else {
        "a Windows named pipe"
    }
}

/// Whether `value` carries a Win32 named-pipe prefix.
///
/// Win32 matches the `pipe` keyword case-insensitively; the separators around
/// it are unaffected by ASCII case folding, so folding the whole prefix is
/// equivalent and does not loosen the match.
fn looks_like_pipe_name(value: &OsStr) -> bool {
    let bytes = value.as_encoded_bytes();
    PIPE_PREFIXES
        .iter()
        .any(|prefix| bytes.len() >= prefix.len() && bytes[..prefix.len()].eq_ignore_ascii_case(prefix))
}

#[cfg(unix)]
fn parse_unix_override(value: &OsStr) -> Result<LocalEndpoint, LocalEndpointError> {
    if looks_like_pipe_name(value) {
        return Err(LocalEndpointError::IncompatibleTransport {
            endpoint: value.to_string_lossy().into_owned(),
            actual: "a Windows named pipe",
            expected: "a Unix domain socket",
        });
    }

    let path = Path::new(value);
    if !path.is_absolute() {
        return Err(LocalEndpointError::MalformedOverride {
            value: value.to_string_lossy().into_owned(),
            expected: "Unix socket path",
            reason: "the path must be absolute so the daemon and its clients \
                     resolve it identically regardless of working directory",
        });
    }
    if path.file_name().is_none() {
        return Err(LocalEndpointError::MalformedOverride {
            value: value.to_string_lossy().into_owned(),
            expected: "Unix socket path",
            reason: "the path must name a socket file, not a directory",
        });
    }

    Ok(LocalEndpoint::UnixSocket(path.to_path_buf()))
}

#[cfg(windows)]
fn parse_windows_override(value: &OsStr) -> Result<LocalEndpoint, LocalEndpointError> {
    if !looks_like_pipe_name(value) {
        return Err(LocalEndpointError::IncompatibleTransport {
            endpoint: value.to_string_lossy().into_owned(),
            actual: "a filesystem path",
            expected: "a Windows named pipe",
        });
    }

    let bytes = value.as_encoded_bytes();
    let name = &bytes[PIPE_NAME_PREFIX.len()..];
    if name.is_empty() {
        return Err(LocalEndpointError::MalformedOverride {
            value: value.to_string_lossy().into_owned(),
            expected: "Windows named pipe",
            reason: r"the prefix '\\.\pipe\' must be followed by a pipe name",
        });
    }
    if name.contains(&b'\\') {
        return Err(LocalEndpointError::MalformedOverride {
            value: value.to_string_lossy().into_owned(),
            expected: "Windows named pipe",
            reason: r"the name after '\\.\pipe\' may not contain a backslash",
        });
    }
    // Win32 caps the whole pipe name at 256 characters.
    if value.as_encoded_bytes().len() > 256 {
        return Err(LocalEndpointError::MalformedOverride {
            value: value.to_string_lossy().into_owned(),
            expected: "Windows named pipe",
            reason: "the name exceeds the 256-character Win32 limit",
        });
    }

    Ok(LocalEndpoint::WindowsNamedPipe(value.to_os_string()))
}

/// Resolve the local endpoint for the current user and target.
///
/// ## Examples
///
/// ```no_run
/// let endpoint = rendezvous_core::local_endpoint::default_local_endpoint()?;
/// endpoint.ensure_native()?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
///
/// ## Errors
///
/// Returns a [`LocalEndpointError`] when `RENDEZVOUS_ENDPOINT` is set to an
/// empty, malformed, or target-incompatible value, or when the stable OS user
/// cannot be discovered. An unsafe override is rejected rather than replaced
/// with a default.
///
/// ## Notes
///
/// This does not create, chmod, or remove anything. A `$XDG_RUNTIME_DIR` that
/// fails inspection is treated as *unavailable* — resolution falls back to the
/// UID-qualified temp directory rather than failing, because an unusable
/// runtime directory is an environment quirk, not a caller error.
pub fn default_local_endpoint() -> Result<LocalEndpoint, LocalEndpointError> {
    if let Some(explicit) = env::var_os(ENDPOINT_ENV_VAR) {
        return LocalEndpoint::from_override(&explicit);
    }

    let user = sniff::os::current_user_id()?;

    #[cfg(unix)]
    {
        default_unix_endpoint(&user)
    }
    #[cfg(windows)]
    {
        default_windows_endpoint(&user)
    }
}

#[cfg(unix)]
fn default_unix_endpoint(user: &StableUserId) -> Result<LocalEndpoint, LocalEndpointError> {
    let StableUserId::UnixUid(uid) = user else {
        return Err(LocalEndpointError::MismatchedIdentity {
            actual: "a Windows SID",
            expected: "a Unix domain socket",
        });
    };

    if let Some(runtime) = env::var_os("XDG_RUNTIME_DIR").filter(|value| !value.is_empty()) {
        let runtime = PathBuf::from(runtime);
        if runtime_dir_is_usable(&runtime, *uid) {
            return Ok(LocalEndpoint::UnixSocket(
                runtime
                    .join("claudine")
                    .join("rendezvous")
                    .join(ENDPOINT_FILE_NAME),
            ));
        }
    }

    Ok(LocalEndpoint::UnixSocket(
        env::temp_dir()
            .join(format!("{ENDPOINT_STEM}-{}", user.endpoint_component()))
            .join(ENDPOINT_FILE_NAME),
    ))
}

/// Whether `$XDG_RUNTIME_DIR` is a private directory belonging to `uid`.
///
/// Inspection only — a directory that passes here is still re-checked, and the
/// Rendezvous subdirectory beneath it still created, by the daemon before it
/// binds. Anything unusable returns `false` so resolution falls back; nothing
/// here reports an error.
#[cfg(unix)]
fn runtime_dir_is_usable(runtime: &Path, uid: u32) -> bool {
    use std::os::unix::fs::MetadataExt;

    if !runtime.is_absolute() {
        return false;
    }
    // Non-following: a symlinked runtime directory could be re-pointed after
    // the check, and its target's ownership says nothing about the link's.
    let Ok(meta) = std::fs::symlink_metadata(runtime) else {
        return false;
    };
    meta.is_dir() && meta.uid() == uid && meta.mode() & 0o022 == 0
}

#[cfg(windows)]
fn default_windows_endpoint(user: &StableUserId) -> Result<LocalEndpoint, LocalEndpointError> {
    let StableUserId::WindowsSid(_) = user else {
        return Err(LocalEndpointError::MismatchedIdentity {
            actual: "a Unix UID",
            expected: "a Windows named pipe",
        });
    };

    Ok(LocalEndpoint::WindowsNamedPipe(OsString::from(format!(
        "{PIPE_NAME_PREFIX}{ENDPOINT_STEM}-{}",
        user.endpoint_component()
    ))))
}

#[cfg(test)]
mod tests;
