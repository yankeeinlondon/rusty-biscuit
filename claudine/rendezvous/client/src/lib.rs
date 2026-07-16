//! Connecting a gRPC client to the rendezvous daemon's local control plane.
//!
//! [`connect`] takes a [`LocalEndpoint`], which already knows whether it is a
//! Unix-domain socket or a Windows named pipe. Callers pass one and get a
//! client back; selecting the transport, opening it, waiting out a busy pipe,
//! and classifying the failure all happen behind that one entry point, so a
//! production call site never needs a `cfg(unix)`/`cfg(windows)` branch.
//!
//! ## Errors
//!
//! [`ConnectError`] keeps the cases a caller acts on apart: nothing listening,
//! not permitted, saturated, an endpoint this target cannot speak, and other
//! transport failures. Each preserves the originating OS error as its source.

mod connector;

pub use connector::{ConnectError, connect};

#[cfg(unix)]
use std::path::PathBuf;

#[cfg(unix)]
use rendezvous_core::RendezvousClient;
#[cfg(unix)]
use rendezvous_core::local_endpoint::LocalEndpoint;
#[cfg(unix)]
use tonic::transport::Channel;

/// Connect to a Unix socket at `path`.
///
/// A Unix-only convenience for test fixtures that already hold a socket path
/// and have no endpoint to hand. Production callers use [`connect`]; Phase 6
/// migrates the daemon and client integration suites onto [`LocalEndpoint`]
/// and removes this.
///
/// ## Errors
///
/// As [`connect`].
#[cfg(unix)]
pub async fn connect_uds(
    path: impl Into<PathBuf>,
) -> Result<RendezvousClient<Channel>, ConnectError> {
    connect(&LocalEndpoint::UnixSocket(path.into())).await
}
