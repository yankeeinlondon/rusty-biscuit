//! Listening on a Unix-domain stream socket.
//!
//! Owns the socket's directory, the bind, and the unlink — and nothing else.
//! The service it serves arrives fully built.
//!
//! ## Notes
//!
//! The security boundary here is the *directory*, not the socket file. A socket
//! is created by `bind(2)` with `0777 & ~umask`, and there is no way to ask the
//! kernel for a narrower mode at creation time, so the mode is corrected
//! immediately afterwards. That correction is not a race: the parent directory
//! has already been proven owner-only, so no other user can name the socket
//! during the window, whatever the umask happened to be.

use std::io;
use std::os::unix::fs::{FileTypeExt, MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

use rendezvous_core::local_endpoint::LocalEndpoint;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;

use crate::private_dir::{ensure_private_dir, expected_uid};
use crate::server::{
    EndpointCleanup, PreparedDaemon, ServerError, ServerHandle, serve_local_incoming,
};

/// Mode the bound socket is forced to, regardless of the inherited umask.
const SOCKET_MODE: u32 = 0o600;

/// Bind `endpoint` and serve `prepared` on it.
pub(super) fn serve(
    endpoint: LocalEndpoint,
    prepared: PreparedDaemon,
) -> Result<ServerHandle, ServerError> {
    let path = endpoint
        .as_unix_path()
        .expect("ensure_native accepted this endpoint, so it is a Unix socket")
        .to_path_buf();

    // An endpoint with no parent is a bare relative name; `ensure_private_dir`
    // reports that as `NotAbsolute` rather than this module inventing a second
    // way to say it.
    ensure_private_dir(path.parent().unwrap_or_else(|| Path::new("")))?;
    clear_endpoint(&endpoint, &path)?;

    let listener = UnixListener::bind(&path).map_err(|source| bind_error(&endpoint, source))?;
    let bound = harden(&endpoint, &path).inspect_err(|_| {
        // Leaving a socket nobody is listening on would make the next boot
        // reclaim it as stale — correct, but only by accident. Undo our own
        // bind instead.
        drop(std::fs::remove_file(&path));
    })?;

    let incoming = UnixListenerStream::new(listener);
    let cleanup = Box::new(SocketCleanup {
        endpoint: endpoint.clone(),
        path,
        bound,
    });

    Ok(serve_local_incoming(endpoint, prepared, incoming, cleanup))
}

/// Narrow the freshly bound socket to owner-only and record which socket it is.
fn harden(endpoint: &LocalEndpoint, path: &Path) -> Result<SocketIdentity, ServerError> {
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(SOCKET_MODE))
        .map_err(|source| listener_error(endpoint, source))?;

    SocketIdentity::read(path)
        .map_err(|source| listener_error(endpoint, source))?
        .ok_or_else(|| ServerError::Listener {
            endpoint: endpoint.to_string(),
            source: io::Error::new(
                io::ErrorKind::NotFound,
                "the socket vanished between bind and inspection",
            ),
        })
}

/// Make the endpoint available to bind, or explain why it cannot be.
///
/// Only one occupant is reclaimable: a socket this user owns that nobody is
/// listening on, which is what an unclean shutdown leaves behind. Everything
/// else — another user's socket, a live daemon, a file, a directory, a symlink —
/// is somebody's data or somebody's service, and removing it is not this
/// process's call.
fn clear_endpoint(endpoint: &LocalEndpoint, path: &Path) -> Result<(), ServerError> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(source) if source.kind() == io::ErrorKind::PermissionDenied => {
            return Err(access_denied(endpoint, source));
        }
        Err(source) => return Err(listener_error(endpoint, source)),
    };

    let file_type = meta.file_type();
    if !file_type.is_socket() {
        return Err(ServerError::EndpointOccupied {
            endpoint: endpoint.to_string(),
            found: describe(&file_type).to_string(),
        });
    }

    let expected = expected_uid()?;
    if meta.uid() != expected {
        return Err(ServerError::EndpointOccupied {
            endpoint: endpoint.to_string(),
            found: format!("a socket owned by uid {}", meta.uid()),
        });
    }

    match probe(path) {
        Liveness::Live => Err(ServerError::EndpointInUse {
            endpoint: endpoint.to_string(),
        }),
        Liveness::Stale => remove_stale(endpoint, path),
        Liveness::Denied(source) => Err(access_denied(endpoint, source)),
    }
}

/// Whether anything is listening on the socket at `path`.
///
/// A connect attempt is the only honest test: a socket file outlives the
/// process that bound it, so its presence says nothing, and a pidfile would
/// just move the same staleness problem one file over.
fn probe(path: &Path) -> Liveness {
    match std::os::unix::net::UnixStream::connect(path) {
        Ok(_) => Liveness::Live,
        Err(error) => match error.kind() {
            // Nobody is bound to it any more.
            io::ErrorKind::ConnectionRefused | io::ErrorKind::NotFound => Liveness::Stale,
            io::ErrorKind::PermissionDenied => Liveness::Denied(error),
            // A full backlog (EAGAIN) or a timeout means a daemon is there and
            // busy. Anything else is unexplained. Both resolve to "do not
            // delete": refusing to boot is recoverable, unlinking a live
            // daemon's endpoint is not.
            _ => Liveness::Live,
        },
    }
}

enum Liveness {
    Live,
    Stale,
    Denied(io::Error),
}

/// Clear a socket left behind by an unclean shutdown, so a restarted daemon can
/// take its own endpoint back.
fn remove_stale(endpoint: &LocalEndpoint, path: &Path) -> Result<(), ServerError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(source) if source.kind() == io::ErrorKind::PermissionDenied => {
            Err(access_denied(endpoint, source))
        }
        Err(source) => Err(listener_error(endpoint, source)),
    }
}

fn describe(file_type: &std::fs::FileType) -> &'static str {
    if file_type.is_symlink() {
        "a symlink"
    } else if file_type.is_dir() {
        "a directory"
    } else if file_type.is_file() {
        "a regular file"
    } else if file_type.is_fifo() {
        "a FIFO"
    } else {
        "an entry that is not a socket"
    }
}

fn bind_error(endpoint: &LocalEndpoint, source: io::Error) -> ServerError {
    match source.kind() {
        io::ErrorKind::AddrInUse => ServerError::EndpointInUse {
            endpoint: endpoint.to_string(),
        },
        io::ErrorKind::PermissionDenied => access_denied(endpoint, source),
        _ => listener_error(endpoint, source),
    }
}

fn access_denied(endpoint: &LocalEndpoint, source: io::Error) -> ServerError {
    ServerError::AccessDenied {
        endpoint: endpoint.to_string(),
        source,
    }
}

fn listener_error(endpoint: &LocalEndpoint, source: io::Error) -> ServerError {
    ServerError::Listener {
        endpoint: endpoint.to_string(),
        source,
    }
}

/// Which socket, specifically, is at a path.
///
/// A path is not an identity: between this daemon's bind and its shutdown, the
/// entry can be unlinked and re-created by a restarted daemon that now owns the
/// name. `(dev, ino)` distinguishes that replacement from our own socket;
/// `uid` is carried so a replacement by another user is equally visible.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SocketIdentity {
    dev: u64,
    ino: u64,
    uid: u32,
}

impl SocketIdentity {
    /// The socket at `path`, or `None` if there is no socket there.
    fn read(path: &Path) -> io::Result<Option<Self>> {
        match std::fs::symlink_metadata(path) {
            Ok(meta) if meta.file_type().is_socket() => Ok(Some(Self {
                dev: meta.dev(),
                ino: meta.ino(),
                uid: meta.uid(),
            })),
            Ok(_) => Ok(None),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }
}

/// Unlinks the socket once the listener is closed.
struct SocketCleanup {
    endpoint: LocalEndpoint,
    path: PathBuf,
    bound: SocketIdentity,
}

impl EndpointCleanup for SocketCleanup {
    fn release(&mut self) -> Result<(), ServerError> {
        let current = SocketIdentity::read(&self.path).map_err(|source| ServerError::Cleanup {
            endpoint: self.endpoint.to_string(),
            source,
        })?;

        if current != Some(self.bound) {
            if current.is_some() {
                tracing::warn!(
                    target: "rendezvous_daemon::local_transport",
                    endpoint = %self.endpoint,
                    "the endpoint was replaced while this daemon ran; leaving it to its owner"
                );
            }
            // Gone, or no longer ours. Either way the postcondition — this
            // daemon's socket is not at that path — already holds, and
            // unlinking would take down whoever replaced us.
            return Ok(());
        }

        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(ServerError::Cleanup {
                endpoint: self.endpoint.to_string(),
                source,
            }),
        }
    }
}

#[cfg(test)]
mod tests;
