//! The portable connector: one `connect` for every target, and one error
//! vocabulary callers can act on.
//!
//! A caller hands over a [`LocalEndpoint`] and gets back a gRPC client. The
//! endpoint carries its own transport, so nothing here asks the caller which
//! platform it is on, and no production call site needs a `cfg` branch.
//!
//! ## Notes
//!
//! Tonic collapses every connector failure into an opaque
//! `tonic::transport::Error`, which would erase the distinction between "no
//! daemon is listening", "you may not open this endpoint", and "the pipe is
//! saturated" — the three cases a caller actually branches on. The connector
//! therefore classifies the OS error inside the connector closure and carries
//! the verdict out through [`ErrorSlot`], rather than reading tonic's message
//! back out as text.

#[cfg(unix)]
mod unix;
#[cfg(windows)]
mod windows;

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use rendezvous_core::RendezvousClient;
use rendezvous_core::local_endpoint::{LocalEndpoint, LocalEndpointError};
use tonic::transport::Channel;

/// The URI tonic uses to drive HTTP/2 framing.
///
/// It is plumbing, not endpoint identity: the supplied connector produces the
/// byte stream, so this authority is never resolved and never dialed.
const PLACEHOLDER_URI: &str = "http://[::]:0";

/// Win32 `ERROR_PIPE_BUSY` — the pipe exists, but every instance is currently
/// serving another client.
///
/// Only the Windows connector can encounter this, but the busy-retry machinery
/// below stays compiled on every target: it is ordinary generic code, and
/// keeping it live means its tests — which need no named pipe — run on macOS
/// and Linux too, rather than only on the one host that can least afford to be
/// the first to find a bug in it.
#[cfg_attr(not(windows), allow(dead_code))]
const ERROR_PIPE_BUSY: i32 = 231;

/// The busy-retry budget applied to a named-pipe open.
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BusyRetry {
    /// Total time an open may spend waiting out a busy endpoint.
    budget: Duration,
    /// Pause between attempts. Fixed rather than exponential: a busy pipe
    /// clears as soon as the daemon loops back to accept, so backing off
    /// exponentially would add latency without easing contention.
    backoff: Duration,
}

impl Default for BusyRetry {
    fn default() -> Self {
        Self {
            budget: Duration::from_secs(5),
            backoff: Duration::from_millis(50),
        }
    }
}

/// Failures connecting a gRPC client to the rendezvous daemon.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConnectError {
    /// The endpoint names a transport this target cannot speak, or is
    /// otherwise unusable as one.
    #[error(transparent)]
    IncompatibleEndpoint(#[from] LocalEndpointError),

    /// Nothing is listening at the endpoint.
    ///
    /// Covers both "the endpoint does not exist" and "it exists but refused
    /// the connection" — a stale Unix socket file left by a dead daemon
    /// produces the latter. They are one case to a caller: start the daemon.
    #[error("no rendezvous daemon is listening at {endpoint}")]
    NotFound {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        /// The OS error that reported it.
        #[source]
        source: io::Error,
    },

    /// The endpoint exists but this process may not open it. On Unix that is
    /// the socket's or a parent directory's mode; on Windows, its DACL.
    #[error("permission denied opening the rendezvous endpoint at {endpoint}")]
    PermissionDenied {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        /// The OS error that reported it.
        #[source]
        source: io::Error,
    },

    /// Every instance of the endpoint stayed busy for the whole retry budget.
    /// The daemon is alive; it is just saturated.
    #[error("the rendezvous endpoint at {endpoint} stayed busy for {waited:.1?}")]
    BusyTimeout {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        /// How long the connector waited before giving up.
        waited: Duration,
        /// The last busy report from the OS.
        #[source]
        source: io::Error,
    },

    /// The transport could not be opened, for a reason with no more specific
    /// variant. The OS error is preserved as the source.
    #[error("failed to open the rendezvous endpoint at {endpoint}")]
    Io {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        /// The OS error that reported it.
        #[source]
        source: io::Error,
    },

    /// The byte stream opened, but tonic could not build an HTTP/2 channel on
    /// it.
    #[error("gRPC transport setup failed")]
    Transport(#[from] tonic::transport::Error),
}

/// Open a gRPC channel to the daemon listening at `endpoint`.
///
/// ## Examples
///
/// ```no_run
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let endpoint = rendezvous_core::default_local_endpoint()?;
/// let mut client = rendezvous_client::connect(&endpoint).await?;
/// # Ok(())
/// # }
/// ```
///
/// ## Errors
///
/// Returns [`ConnectError::IncompatibleEndpoint`] when `endpoint` names the
/// other target's transport, [`ConnectError::NotFound`] when no daemon is
/// listening, [`ConnectError::PermissionDenied`] when the endpoint may not be
/// opened, [`ConnectError::BusyTimeout`] when a Windows pipe stays saturated
/// for the whole retry budget, and [`ConnectError::Io`] or
/// [`ConnectError::Transport`] for other transport failures.
pub async fn connect(endpoint: &LocalEndpoint) -> Result<RendezvousClient<Channel>, ConnectError> {
    endpoint.ensure_native()?;

    #[cfg(unix)]
    {
        let path = endpoint
            .as_unix_path()
            .expect("ensure_native accepted this endpoint, so it is a Unix socket");
        unix::connect(endpoint, path).await
    }
    #[cfg(windows)]
    {
        let name = endpoint
            .as_windows_pipe_name()
            .expect("ensure_native accepted this endpoint, so it is a named pipe");
        windows::connect(endpoint, name, BusyRetry::default()).await
    }
}

/// Carries a classified failure out of tonic's connector closure.
///
/// The closure must hand tonic an `io::Error`, and tonic then discards it into
/// its own opaque type. Stashing the verdict here on the way past is what lets
/// [`finish`] report the real cause — with the original OS error still in the
/// source chain — instead of a generic transport failure.
#[derive(Clone, Default)]
struct ErrorSlot(Arc<Mutex<Option<ConnectError>>>);

impl ErrorSlot {
    /// Pass `result` through, keeping any error for [`finish`] to recover.
    ///
    /// Tonic only needs *an* error to fail the connect, so it gets a
    /// same-message stand-in while the classified original stays here.
    fn deflect<T>(&self, result: Result<T, ConnectError>) -> io::Result<T> {
        result.map_err(|error| {
            let stand_in = io::Error::other(error.to_string());
            *self.lock() = Some(error);
            stand_in
        })
    }

    fn take(&self) -> Option<ConnectError> {
        self.lock().take()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<ConnectError>> {
        self.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

/// Turn tonic's result into a client, restoring the classified cause when the
/// connector — rather than tonic's own HTTP/2 setup — is what failed.
fn finish(
    slot: &ErrorSlot,
    result: Result<Channel, tonic::transport::Error>,
) -> Result<RendezvousClient<Channel>, ConnectError> {
    match result {
        Ok(channel) => Ok(RendezvousClient::new(channel)),
        Err(transport) => Err(slot.take().unwrap_or(ConnectError::Transport(transport))),
    }
}

/// Whether `error` means the endpoint exists but every instance is busy.
#[cfg_attr(not(windows), allow(dead_code))]
fn is_busy(error: &io::Error) -> bool {
    error.raw_os_error() == Some(ERROR_PIPE_BUSY)
}

/// Map an OS error from opening the local transport onto the vocabulary
/// callers branch on, keeping the original as the source.
fn classify(endpoint: &LocalEndpoint, source: io::Error) -> ConnectError {
    let endpoint = endpoint.to_string();
    match source.kind() {
        io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => {
            ConnectError::NotFound { endpoint, source }
        }
        io::ErrorKind::PermissionDenied => ConnectError::PermissionDenied { endpoint, source },
        _ => ConnectError::Io { endpoint, source },
    }
}

/// Open the transport, waiting out a busy endpoint within one bounded budget.
///
/// ## Notes
///
/// The budget is spent in whole backoff units rather than measured against a
/// clock. Every wait is one `sleep(backoff)`, so counting them bounds the wall
/// clock just as tightly as reading it would — and it keeps the loop
/// deterministic under an injected sleep, which is what makes deadline
/// exhaustion testable without a fake clock or a real named pipe.
#[cfg_attr(not(windows), allow(dead_code))]
async fn open_with_busy_retry<T, Open, Opening, Sleep, Sleeping>(
    endpoint: &LocalEndpoint,
    retry: BusyRetry,
    mut open: Open,
    mut sleep: Sleep,
) -> Result<T, ConnectError>
where
    Open: FnMut() -> Opening,
    Opening: Future<Output = io::Result<T>>,
    Sleep: FnMut(Duration) -> Sleeping,
    Sleeping: Future<Output = ()>,
{
    let mut waited = Duration::ZERO;
    loop {
        let error = match open().await {
            Ok(opened) => return Ok(opened),
            Err(error) => error,
        };
        if !is_busy(&error) {
            return Err(classify(endpoint, error));
        }
        if waited + retry.backoff > retry.budget {
            return Err(ConnectError::BusyTimeout {
                endpoint: endpoint.to_string(),
                waited,
                source: error,
            });
        }
        sleep(retry.backoff).await;
        waited += retry.backoff;
    }
}

#[cfg(test)]
mod tests;
