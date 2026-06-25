//! Cross-platform library surface for connecting a gRPC client to the
//! rendezvous daemon.
//!
//! The client connects to the daemon over the platform's preferred local IPC
//! transport: a Unix Domain Socket on Unix, or a Windows named pipe on Windows.
//! [`connect`] dispatches to the right primitive based on the compile target,
//! and the test client / claudine lifecycle requeue code share a single vetted
//! connector through it.
//!
//! ## Platform primitives
//!
//! - [`connect_uds`] (Unix only) — opens a `UnixStream` and wraps it in a
//!   tonic `Channel`.
//! - [`connect_named_pipe`] (Windows only) — opens a `NamedPipeClient` via
//!   `tokio::net::windows::named_pipe::ClientOptions` and wraps it in a tonic
//!   `Channel`.

use std::io;
use std::path::PathBuf;

use hyper_util::rt::TokioIo;
use rendezvous_core::RendezvousClient;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

/// Errors returned while connecting the gRPC client to a daemon.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// The placeholder URI used by tonic to drive the connector could not
    /// be parsed, or the underlying byte-stream connection failed. Tonic
    /// wraps both in `tonic::transport::Error`.
    #[error("invalid placeholder URI or connection failure: {0}")]
    Uri(#[from] tonic::transport::Error),
}

/// Cross-platform connector: opens a gRPC [`Channel`] to the daemon at
/// `endpoint` using the platform's local IPC transport.
///
/// On Unix `endpoint` is the filesystem path of the Unix Domain Socket; on
/// Windows it is the named-pipe path (`\\.\pipe\<name>`). Callers normally
/// pass [`rendezvous_core::socket::default_socket_path`], which resolves the
/// right shape per platform.
///
/// ## Errors
///
/// Returns [`ConnectError::Uri`] if tonic could not establish the channel
/// (no daemon, missing pipe, transport-layer failure).
pub async fn connect(
    endpoint: impl Into<PathBuf>,
) -> Result<RendezvousClient<Channel>, ConnectError> {
    let path = endpoint.into();
    #[cfg(unix)]
    {
        connect_uds(path).await
    }
    #[cfg(windows)]
    {
        // Named-pipe paths live in the Win32 device namespace and use `\`
        // separators; convert via `to_string_lossy` so `ClientOptions::open`
        // receives the raw pipe name.
        let pipe_name = path.to_string_lossy().into_owned();
        connect_named_pipe(pipe_name).await
    }
}

/// Build a connected [`RendezvousClient`] backed by a `UnixStream`
/// pointed at `socket_path`.
///
/// The endpoint URI is a placeholder; tonic only uses the scheme to drive
/// HTTP/2 framing — the supplied connector reroutes the actual byte stream
/// over the Unix socket.
#[cfg(unix)]
pub async fn connect_uds(
    socket_path: impl Into<PathBuf>,
) -> Result<RendezvousClient<Channel>, ConnectError> {
    let path = socket_path.into();
    let channel = Endpoint::try_from("http://[::]:0")?
        .connect_with_connector(service_fn(move |_: Uri| {
            let path = path.clone();
            async move { connect_unix(&path).await }
        }))
        .await?;
    Ok(RendezvousClient::new(channel))
}

#[cfg(unix)]
async fn connect_unix(
    path: &std::path::Path,
) -> Result<TokioIo<tokio::net::UnixStream>, io::Error> {
    let stream = tokio::net::UnixStream::connect(path).await?;
    Ok(TokioIo::new(stream))
}

/// Build a connected [`RendezvousClient`] backed by a Windows named pipe.
///
/// `pipe_name` must be a fully-qualified pipe path such as
/// `\\.\pipe\rendezvous-daemon`. The connector opens the pipe client-side via
/// `tokio::net::windows::named_pipe::ClientOptions` and routes the byte stream
/// through tonic's HTTP/2 framer the same way [`connect_uds`] does on Unix.
#[cfg(windows)]
pub async fn connect_named_pipe(
    pipe_name: impl Into<String>,
) -> Result<RendezvousClient<Channel>, ConnectError> {
    use tokio::net::windows::named_pipe::ClientOptions;

    let pipe_name = pipe_name.into();
    let channel = Endpoint::try_from("http://[::]:0")?
        .connect_with_connector(service_fn(move |_: Uri| {
            let pipe_name = pipe_name.clone();
            async move {
                let stream = ClientOptions::new().open(&pipe_name)?;
                Ok::<_, io::Error>(TokioIo::new(stream))
            }
        }))
        .await?;
    Ok(RendezvousClient::new(channel))
}
