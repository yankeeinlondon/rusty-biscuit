//! Listening on a Windows named pipe.
//!
//! Owns the pipe instances, the acceptor, and the DACL that keeps other users
//! out — and nothing else. The service it serves arrives fully built.
//!
//! ## Notes
//!
//! There is no cleanup step. A named pipe is not a filesystem entry: the name
//! exists exactly as long as some instance handle is open, so it disappears when
//! the acceptor and the connected instances are dropped. Nothing on this target
//! corresponds to the Unix unlink, and nothing here may be tempted to delete a
//! path.
//!
//! ## Threat boundary
//!
//! The DACL keeps *other non-administrative users* out. It does not attempt to
//! exclude Administrators or SYSTEM: either can take ownership of any object,
//! rewrite the DACL, and debug the process, so an ACE denying them would be
//! security theater rather than a boundary.

use std::ffi::{OsStr, OsString};
use std::io;
use std::pin::Pin;
use std::task::{Context, Poll};

use rendezvous_core::local_endpoint::LocalEndpoint;
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::windows::named_pipe::{NamedPipeServer, PipeMode, ServerOptions};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_stream::wrappers::ReceiverStream;
use tonic::transport::server::Connected;
use windows::Win32::Security::SECURITY_ATTRIBUTES;

use crate::private_dir::{SecurityDescriptor, current_user_descriptor};
use crate::server::{
    EndpointCleanup, PreparedDaemon, ServerError, ServerHandle, serve_local_incoming,
};

/// How many connected instances may wait for tonic to take them.
///
/// The acceptor creates the next instance eagerly, so a client never finds the
/// name unserved; this only bounds how far ahead of tonic it may run.
const ACCEPT_BACKLOG: usize = 16;

/// Bind `endpoint` and serve `prepared` on it.
pub(super) fn serve(
    endpoint: LocalEndpoint,
    prepared: PreparedDaemon,
) -> Result<ServerHandle, ServerError> {
    let name = endpoint
        .as_windows_pipe_name()
        .expect("ensure_native accepted this endpoint, so it is a named pipe")
        .to_os_string();

    let descriptor = current_user_descriptor()?;
    // `first_pipe_instance` is the exclusion: if another daemon already holds
    // the name, this fails rather than quietly adding an instance to somebody
    // else's pipe and racing them for connections.
    let first = create_instance(&name, &descriptor, true)
        .map_err(|source| create_error(&endpoint, source))?;

    let (tx, rx) = mpsc::channel(ACCEPT_BACKLOG);
    let task = tokio::spawn(accept_loop(name, descriptor, first, tx));

    let cleanup = Box::new(PipeCleanup { task });
    Ok(serve_local_incoming(
        endpoint,
        prepared,
        ReceiverStream::new(rx),
        cleanup,
    ))
}

/// A connected pipe instance, in the shape tonic's server requires.
///
/// tonic implements [`Connected`] for `TcpStream` and `UnixStream` but not for
/// `NamedPipeServer`, so the Windows transport supplies it rather than the
/// shared `serve_local_incoming` loosening its bound for one platform.
///
/// `ConnectInfo` is `()` deliberately: a named pipe has no peer address, and
/// the local threat boundary is the DACL applied at instance creation, which is
/// enforced by the kernel before a connection ever arrives. Nothing downstream
/// inspects per-connection identity, so there is nothing honest to report here.
struct PipeConnection(NamedPipeServer);

impl Connected for PipeConnection {
    type ConnectInfo = ();

    fn connect_info(&self) -> Self::ConnectInfo {}
}

impl AsyncRead for PipeConnection {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_read(cx, buf)
    }
}

impl AsyncWrite for PipeConnection {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        Pin::new(&mut self.0).poll_write(cx, buf)
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_flush(cx)
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        Pin::new(&mut self.0).poll_shutdown(cx)
    }
}

/// Accept connections forever, keeping an unconnected instance available at all
/// times.
async fn accept_loop(
    name: OsString,
    descriptor: SecurityDescriptor,
    mut server: NamedPipeServer,
    tx: mpsc::Sender<io::Result<PipeConnection>>,
) {
    loop {
        if let Err(error) = server.connect().await {
            let _ = tx.send(Err(error)).await;
            return;
        }

        // The successor is created *before* the connected instance is handed
        // off. A pipe name with no unconnected instance rejects clients with
        // ERROR_FILE_NOT_FOUND rather than queueing them, so creating the
        // successor afterwards would turn every accept into a window where a
        // client racing in is told the daemon does not exist.
        let next = match create_instance(&name, &descriptor, false) {
            Ok(next) => next,
            Err(error) => {
                let _ = tx.send(Err(error)).await;
                return;
            }
        };

        let connected = std::mem::replace(&mut server, next);
        if tx.send(Ok(PipeConnection(connected))).await.is_err() {
            // tonic stopped serving. Returning drops `server`, closing the last
            // unconnected instance and releasing the name.
            return;
        }
    }
}

/// Create one instance of the pipe called `name`.
///
/// `first` must be true for exactly the first instance, and false for every
/// successor: `first_pipe_instance` is a claim on the *name*, and a successor
/// asserting it would collide with its own predecessor.
fn create_instance(
    name: &OsStr,
    descriptor: &SecurityDescriptor,
    first: bool,
) -> io::Result<NamedPipeServer> {
    let mut attributes = SECURITY_ATTRIBUTES {
        nLength: u32::try_from(size_of::<SECURITY_ATTRIBUTES>()).unwrap_or(u32::MAX),
        lpSecurityDescriptor: descriptor.as_ptr(),
        bInheritHandle: false.into(),
    };

    // SAFETY: `attributes` outlives the call, and its descriptor pointer is
    // owned by `descriptor`, which the caller holds for at least as long.
    unsafe {
        ServerOptions::new()
            .first_pipe_instance(first)
            // A pipe reachable over SMB is a pipe a remote host can reach. The
            // local control plane is local.
            .reject_remote_clients(true)
            // gRPC is a byte stream. Message mode would frame it for us,
            // wrongly.
            .pipe_mode(PipeMode::Byte)
            .access_inbound(true)
            .access_outbound(true)
            .create_with_security_attributes_raw(name, std::ptr::from_mut(&mut attributes).cast())
    }
}

fn create_error(endpoint: &LocalEndpoint, source: io::Error) -> ServerError {
    match source.kind() {
        // `first_pipe_instance` refused: the name belongs to a daemon that is
        // already running.
        io::ErrorKind::AddrInUse | io::ErrorKind::AlreadyExists => ServerError::EndpointInUse {
            endpoint: endpoint.to_string(),
        },
        io::ErrorKind::PermissionDenied => ServerError::AccessDenied {
            endpoint: endpoint.to_string(),
            source,
        },
        _ => ServerError::Listener {
            endpoint: endpoint.to_string(),
            source,
        },
    }
}

/// Closes the daemon's pipe instances.
struct PipeCleanup {
    task: JoinHandle<()>,
}

impl EndpointCleanup for PipeCleanup {
    fn release(&mut self) -> Result<(), ServerError> {
        // The acceptor is parked in `connect()` on an instance nothing will
        // ever connect to, so it will not notice the receiver closing on its
        // own. Aborting drops that instance; the name goes with the last
        // handle. There is no filesystem entry to remove and therefore no
        // cleanup that can fail.
        self.task.abort();
        Ok(())
    }
}

#[cfg(test)]
mod tests;
