//! Binding one [`LocalEndpoint`] to one prepared daemon.
//!
//! [`spawn_local_server`] is the daemon's production entry point on every
//! target. It resolves nothing and decides nothing about storage or
//! networking — [`prepare_daemon`](crate::server::prepare_daemon) already did
//! all of that — and instead does the one thing that cannot be shared: it hands
//! the endpoint to the platform module that knows how to listen on it.
//!
//! The platform modules own listener construction, connection acceptance,
//! access control on the endpoint itself, and endpoint cleanup. Nothing else.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::sync::atomic::{AtomicU64, Ordering};

use rendezvous_core::local_endpoint::LocalEndpoint;

use crate::server::{DaemonConfig, ServerError, ServerHandle, prepare_daemon};

/// Completed runs of the shared boot pipeline in this process.
static SHARED_BOOTS: AtomicU64 = AtomicU64::new(0);

/// Note that the shared boot pipeline finished a run.
pub(crate) fn record_shared_boot() {
    SHARED_BOOTS.fetch_add(1, Ordering::Relaxed);
}

/// How many times the shared boot pipeline has run in this process.
///
/// Instrumentation for the tests that pin the portable-startup contract: one
/// daemon must advance this by exactly one, on every transport. A transport
/// that grew its own copy of the storage or network stack would either not
/// advance it or advance it twice.
#[must_use]
pub fn shared_boot_count() -> u64 {
    SHARED_BOOTS.load(Ordering::Relaxed)
}

/// Boot a daemon and serve it at `endpoint`.
///
/// ## Errors
///
/// Returns [`ServerError::Endpoint`] when `endpoint` names a transport this
/// target cannot speak, [`ServerError::Ownership`] when the data root or the
/// endpoint's directory is not private to this user,
/// [`ServerError::EndpointInUse`] when a live daemon already holds the
/// endpoint, [`ServerError::EndpointOccupied`] when something the daemon may
/// not remove sits at the endpoint, [`ServerError::AccessDenied`] or
/// [`ServerError::Listener`] when the listener cannot be created, and the
/// subsystem variants when the shared stack cannot be brought up.
pub fn spawn_local_server(
    endpoint: LocalEndpoint,
    config: DaemonConfig,
) -> Result<ServerHandle, ServerError> {
    // Ahead of the boot: refusing an unusable endpoint after opening the
    // databases would leave a redb lock behind for the next attempt to trip on.
    endpoint.ensure_native()?;

    let prepared = prepare_daemon(config)?;

    #[cfg(unix)]
    {
        unix::serve(endpoint, prepared)
    }
    #[cfg(windows)]
    {
        windows::serve(endpoint, prepared)
    }
}
