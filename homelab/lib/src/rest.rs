//! Construction and outcome mapping shared by the REST-backed device clients
//! (Eversolo and Samsung TV REST).

use std::time::Duration;

use schematic_schema::schematic_runtime::{
    Api, ApiError, BuildError, CallOptions, Client, ClientBuilder, EndpointSpec, ErrorKind,
    NoDeclaredError, Outcome, Response, TransportError,
};

/// Bounds establishing a connection to a LAN device that may be asleep.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);

/// Bounds each request, so an unresponsive device fails instead of hanging.
pub(crate) const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);

/// Applies the base URL and the device timeouts to a generated client builder.
pub(crate) fn device_builder<A: Api>(
    builder: ClientBuilder<A>,
    base_url: String,
) -> ClientBuilder<A> {
    builder
        .base_url(base_url)
        .connect_timeout(CONNECT_TIMEOUT)
        .timeout(REQUEST_TIMEOUT)
}

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
}

/// Sends `request` with default call options and maps the outcome.
pub(crate) async fn send<A, R>(client: &Client<A>, request: R) -> Result<R::Success, RestError>
where
    A: Api,
    R: EndpointSpec<Api = A, Failure = NoDeclaredError>,
{
    // Boxed: the runtime's send future is several KB, and eversolo-integration
    // joins up to eight of them inside larger futures, which overflowed a
    // 2 MiB stack in debug builds when held inline.
    into_result(Box::pin(client.send(request, &CallOptions::default())).await)
}

/// Turns a send result into the success value or a [`RestError`].
fn into_result<T>(
    sent: Result<Response<T, NoDeclaredError>, TransportError>,
) -> Result<T, RestError> {
    let (meta, outcome) = sent?.into_parts();
    match outcome {
        Outcome::Success(value) => Ok(value),
        Outcome::Failure(error) => {
            let status = meta.status.as_u16();
            Err(match error.kind {
                ErrorKind::Undeclared => RestError::Status {
                    status,
                    body: String::from_utf8_lossy(&error.body).into_owned(),
                },
                _ => RestError::Response { status, error },
            })
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

#[cfg(test)]
mod tests {
    use schematic_schema::eversolo::Eversolo;
    use schematic_schema::schematic_runtime::SharedTransport;

    use super::*;

    #[test]
    fn device_builder_sets_the_request_timeout() {
        let client = device_builder(Eversolo::builder(), "http://127.0.0.1:9529".to_string())
            .build()
            .expect("client builds");

        assert!(format!("{client:?}").contains("timeout: 10s"));
    }

    #[test]
    fn device_builder_sets_a_connect_timeout() {
        // A client attached to a shared transport refuses connection
        // options and names the one it found, which exposes that the
        // connect timeout was set.
        let transport = SharedTransport::builder().build().expect("transport");

        let refused = device_builder(Eversolo::builder(), "http://127.0.0.1:9529".to_string())
            .transport(&transport)
            .build()
            .expect_err("connection options conflict with a shared transport");

        assert_eq!(
            refused,
            BuildError::TransportOptionConflict {
                option: "connect_timeout"
            }
        );
        assert_eq!(CONNECT_TIMEOUT, Duration::from_secs(5));
    }
}
