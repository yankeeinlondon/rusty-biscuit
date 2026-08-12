//! The named-pipe half of the connector.

use std::ffi::{OsStr, OsString};

use hyper_util::rt::TokioIo;
use rendezvous_core::RendezvousClient;
use rendezvous_core::local_endpoint::LocalEndpoint;
use tokio::net::windows::named_pipe::ClientOptions;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

use super::{
    BusyRetry, ConnectError, ErrorSlot, PLACEHOLDER_URI, finish, open_with_busy_retry,
};

/// Connect to the named pipe called `name`.
///
/// `name` stays an `OsString` all the way to `ClientOptions::open`, which
/// accepts one directly: routing it through `PathBuf`/`to_string_lossy` would
/// corrupt a name Win32 accepts but UTF-8 cannot spell.
pub(super) async fn connect(
    endpoint: &LocalEndpoint,
    name: &OsStr,
    retry: BusyRetry,
) -> Result<RendezvousClient<Channel>, ConnectError> {
    let slot = ErrorSlot::default();
    let name = name.to_os_string();

    let result = Endpoint::try_from(PLACEHOLDER_URI)?
        .connect_with_connector(service_fn({
            let slot = slot.clone();
            let endpoint = endpoint.clone();
            move |_: Uri| {
                let (name, slot, endpoint) = (name.clone(), slot.clone(), endpoint.clone());
                async move {
                    let opened = open_with_busy_retry(
                        &endpoint,
                        retry,
                        || open(&name),
                        tokio::time::sleep,
                    )
                    .await;
                    slot.deflect(opened).map(TokioIo::new)
                }
            }
        }))
        .await;

    finish(&slot, result)
}

/// One open attempt, reporting `ERROR_PIPE_BUSY` verbatim so the retry loop
/// can recognize it.
///
/// `ClientOptions::open` is synchronous but returns immediately, so wrapping it
/// in an async fn costs nothing and lets the retry loop treat both transports'
/// openers alike.
async fn open(name: &OsString) -> std::io::Result<tokio::net::windows::named_pipe::NamedPipeClient> {
    ClientOptions::new().open(name)
}
