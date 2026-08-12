//! The Unix-domain-socket half of the connector.

use std::path::Path;

use hyper_util::rt::TokioIo;
use rendezvous_core::RendezvousClient;
use rendezvous_core::local_endpoint::LocalEndpoint;
use tokio::net::UnixStream;
use tonic::transport::{Channel, Endpoint, Uri};
use tower::service_fn;

use super::{ConnectError, ErrorSlot, PLACEHOLDER_URI, classify, finish};

/// Connect to the Unix socket at `path`.
///
/// A Unix socket has no busy state — the kernel either completes the connect
/// against the listener's backlog or fails outright — so there is nothing to
/// retry here, unlike the Windows pipe path.
pub(super) async fn connect(
    endpoint: &LocalEndpoint,
    path: &Path,
) -> Result<RendezvousClient<Channel>, ConnectError> {
    let slot = ErrorSlot::default();
    let path = path.to_path_buf();

    let result = Endpoint::try_from(PLACEHOLDER_URI)?
        .connect_with_connector(service_fn({
            let slot = slot.clone();
            let endpoint = endpoint.clone();
            move |_: Uri| {
                let (path, slot, endpoint) = (path.clone(), slot.clone(), endpoint.clone());
                async move {
                    let opened = UnixStream::connect(&path)
                        .await
                        .map_err(|source| classify(&endpoint, source));
                    slot.deflect(opened).map(TokioIo::new)
                }
            }
        }))
        .await;

    finish(&slot, result)
}
