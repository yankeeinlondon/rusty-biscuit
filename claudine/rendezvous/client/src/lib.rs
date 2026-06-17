//! Library surface for the rendezvous test client.
//!
//! The test client's primary role is to exercise the daemon's gRPC IPC
//! interface from integration tests and from CI smoke checks. Lifting the
//! `connect_uds` helper into a library makes both the binary and future
//! tests share a single, vetted code path for opening a `Channel` over a
//! Unix Domain Socket.

use std::io;
use std::path::{Path, PathBuf};

use hyper_util::rt::TokioIo;
use rendezvous_core::RendezvousClient;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

/// Errors returned while connecting the gRPC client to a daemon UDS.
#[derive(Debug, thiserror::Error)]
pub enum ConnectError {
    /// The placeholder URI used by tonic to drive the connector could not
    /// be parsed. This is unexpected and indicates a programmer error.
    #[error("invalid placeholder URI: {0}")]
    Uri(#[from] tonic::transport::Error),
}

/// Build a connected [`RendezvousClient`] backed by a `UnixStream`
/// pointed at `socket_path`.
///
/// The endpoint URI is a placeholder; tonic only uses the scheme to drive
/// HTTP/2 framing — the supplied connector reroutes the actual byte stream
/// over the Unix socket.
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

async fn connect_unix(path: &Path) -> Result<TokioIo<UnixStream>, io::Error> {
    let stream = UnixStream::connect(path).await?;
    Ok(TokioIo::new(stream))
}
