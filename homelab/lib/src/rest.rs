//! Construction and outcome mapping shared by the REST-backed device clients
//! (Eversolo and Samsung TV REST).

use std::time::Duration;

use schematic_schema::schematic_runtime::{
    ApiError, BuildError, CallError, ErrorKind, NoDeclaredError, TransportError,
};

/// Bounds establishing a connection to a LAN device that may be asleep.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Bounds each request, so an unresponsive device fails instead of hanging.
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// A failed REST call to a LAN device.
///
/// Display never includes headers or the unredacted request URL. The
/// [`Status`](Self::Status) variant includes the device's response body.
#[derive(Debug, thiserror::Error)]
pub enum RestError {
    /// The HTTP client could not be built, for example because the host does
    /// not form a valid base URL.
    #[error("client could not be built: {0}")]
    Build(#[from] BuildError),

    /// No response arrived: the connection failed, was dropped, or timed out.
    #[error("{0}")]
    Transport(#[from] TransportError),

    /// The device replied with a non-success status.
    #[error("status {status}: {body}")]
    Status {
        /// HTTP status code.
        status: u16,
        /// Response body, decoded as lossy UTF-8.
        body: String,
    },

    /// The device replied, but the body could not be used: a 2xx body that
    /// did not decode, a body cut off mid-read, or a redirect that was not
    /// followed.
    #[error("status {status}: {error}")]
    Response {
        /// HTTP status code.
        status: u16,
        /// The runtime's classification, with the raw body.
        error: ApiError<NoDeclaredError>,
    },

    /// A call failure of a kind the runtime added after this mapping was
    /// written. `CallError` is `#[non_exhaustive]`, so this arm must exist;
    /// today every failure maps to one of the variants above.
    #[error("{0}")]
    Call(CallError<NoDeclaredError>),
}

impl From<CallError<NoDeclaredError>> for RestError {
    fn from(error: CallError<NoDeclaredError>) -> Self {
        match error {
            CallError::Transport(error) => Self::Transport(error),
            CallError::Api { error, meta } => {
                let status = meta.status.as_u16();
                match error.kind {
                    ErrorKind::Undeclared => Self::Status {
                        status,
                        body: String::from_utf8_lossy(&error.body).into_owned(),
                    },
                    _ => Self::Response { status, error },
                }
            }
            error => Self::Call(error),
        }
    }
}

/// Local servers for client tests.
#[cfg(test)]
pub(crate) mod test_support {
    use std::net::SocketAddr;

    /// Starts a listener that accepts each connection and closes it before
    /// any response, so a send fails without a response head.
    pub(crate) async fn closing_listener() -> SocketAddr {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind local listener");
        let address = listener.local_addr().expect("listener address");
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                drop(stream);
            }
        });
        address
    }
}
