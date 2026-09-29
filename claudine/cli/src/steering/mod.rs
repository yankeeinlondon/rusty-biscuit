//! Managed steering glue between the wrapper, `claudine::steering`, and the
//! local Rendezvous daemon.
//!
//! - [`owner`] — the wrapper's per-execution controller and its control link.
//! - [`automatic`] — automatic repetition help, sent to that controller.
//! - [`requester`] — listing managed targets and routing a request to one.
//! - `wire` — the typed ↔ protobuf conversions both sides share.
//!
//! Topic: `claudine/docs/topics/steering-routing.md`.

pub(crate) mod automatic;
pub(crate) mod owner;
pub(crate) mod requester;
mod wire;

use std::error::Error;

/// Why the local Rendezvous daemon could not be used.
#[derive(Debug, thiserror::Error)]
pub(crate) enum DaemonAccessError {
    #[error("the local Rendezvous endpoint could not be resolved")]
    Endpoint(#[from] rendezvous_core::LocalEndpointError),
    #[error("connecting to the local Rendezvous daemon timed out")]
    TimedOut,
    #[error("the local Rendezvous daemon is not reachable")]
    Unreachable(#[from] rendezvous_client::ConnectError),
    #[error("the local Rendezvous daemon rejected the call")]
    Rejected(#[from] tonic::Status),
    #[error("the local Rendezvous daemon did not accept the steering registration")]
    NotAccepted,
}

/// `context`, then `cause` and its source chain, as one line. This is where
/// a typed failure becomes the text a steering result or reply carries.
pub(crate) fn render_chain(context: &str, cause: &(dyn Error + 'static)) -> String {
    let mut text = format!("{context}: {cause}");
    let mut next = cause.source();
    while let Some(inner) = next {
        text.push_str(": ");
        text.push_str(&inner.to_string());
        next = inner.source();
    }
    text
}

#[cfg(test)]
pub(crate) mod tests;
