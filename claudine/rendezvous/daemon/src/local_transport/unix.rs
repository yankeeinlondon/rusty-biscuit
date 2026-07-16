//! Listening on a Unix-domain stream socket.
//!
//! Owns the socket's directory, the bind, and the unlink — and nothing else.
//! The service it serves arrives fully built.

use std::path::{Path, PathBuf};

use rendezvous_core::local_endpoint::LocalEndpoint;
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;

use crate::server::{
    EndpointCleanup, PreparedDaemon, ServerError, ServerHandle, serve_local_incoming,
};

/// Bind `endpoint` and serve `prepared` on it.
pub(super) fn serve(
    endpoint: LocalEndpoint,
    prepared: PreparedDaemon,
) -> Result<ServerHandle, ServerError> {
    let path = endpoint
        .as_unix_path()
        .expect("ensure_native accepted this endpoint, so it is a Unix socket")
        .to_path_buf();

    if let Some(parent) = path.parent() {
        ensure_parent_dir(&endpoint, parent)?;
    }
    remove_stale(&endpoint, &path)?;

    let listener = UnixListener::bind(&path).map_err(|source| bind_error(&endpoint, source))?;
    let incoming = UnixListenerStream::new(listener);
    let cleanup = Box::new(SocketCleanup {
        endpoint: endpoint.clone(),
        path,
    });

    Ok(serve_local_incoming(endpoint, prepared, incoming, cleanup))
}

fn ensure_parent_dir(endpoint: &LocalEndpoint, parent: &Path) -> Result<(), ServerError> {
    if parent.as_os_str().is_empty() {
        return Ok(());
    }
    std::fs::create_dir_all(parent).map_err(|source| match source.kind() {
        std::io::ErrorKind::PermissionDenied => ServerError::AccessDenied {
            endpoint: endpoint.to_string(),
            source,
        },
        _ => ServerError::Listener {
            endpoint: endpoint.to_string(),
            source,
        },
    })
}

/// Clear a socket left behind by an unclean shutdown, so a restarted daemon can
/// take its own endpoint back.
fn remove_stale(endpoint: &LocalEndpoint, path: &Path) -> Result<(), ServerError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(source) if source.kind() == std::io::ErrorKind::PermissionDenied => {
            Err(ServerError::AccessDenied {
                endpoint: endpoint.to_string(),
                source,
            })
        }
        Err(source) => Err(ServerError::Listener {
            endpoint: endpoint.to_string(),
            source,
        }),
    }
}

fn bind_error(endpoint: &LocalEndpoint, source: std::io::Error) -> ServerError {
    match source.kind() {
        std::io::ErrorKind::AddrInUse => ServerError::EndpointInUse {
            endpoint: endpoint.to_string(),
        },
        std::io::ErrorKind::PermissionDenied => ServerError::AccessDenied {
            endpoint: endpoint.to_string(),
            source,
        },
        _ => ServerError::Listener {
            endpoint: endpoint.to_string(),
            source,
        },
    }
}

/// Unlinks the socket once the listener is closed.
struct SocketCleanup {
    endpoint: LocalEndpoint,
    path: PathBuf,
}

impl EndpointCleanup for SocketCleanup {
    fn release(&mut self) -> Result<(), ServerError> {
        match std::fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            // Someone else already cleared it. The endpoint is gone either way,
            // which is the whole postcondition.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(ServerError::Cleanup {
                endpoint: self.endpoint.to_string(),
                source,
            }),
        }
    }
}
