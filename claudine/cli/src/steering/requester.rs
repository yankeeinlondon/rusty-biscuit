//! The requester's side of managed steering: listing managed targets and
//! routing one request to its owner through the local Rendezvous daemon.
//!
//! A requester reports only what the owner established. Anything the daemon
//! or transport decides alone is at most unavailable, refused, busy, or
//! unknown: a missing daemon is "no local route" (unavailable), a request
//! refused before forwarding sent nothing, and any failure after the request
//! may have reached the owner is unknown and is never retried. The owner
//! writes the audit records; the requester writes none.

#![cfg_attr(not(test), expect(dead_code, reason = "`claudine steer` is the production caller"))]

use std::time::Duration;

use claudine::provider::Provider;
use claudine::secrets::RedactedText;
use claudine::steering::contract::{SendOutcome, SteeringRequest, SteeringResult};
use claudine::steering::controller::DeliveryDeadlines;
use claudine::steering::discovery::{DiscoveryFuture, ManagedListing, ManagedSource};
use claudine::steering::identity::ManagedTarget;
use rendezvous_core::{
    ListManagedTargetsRequest, MAX_ROUTE_DEADLINE_MS, RendezvousClient, RouteOutcome, RouteSteeringRequest,
};
use tonic::transport::Channel;

use super::{DaemonAccessError, render_chain, wire};

/// Bound on connecting to the local daemon.
pub(crate) const CONNECT_TIMEOUT: Duration = Duration::from_secs(1);
/// Transport slack added to the owner's own acceptance deadline, so the
/// owner's verdict arrives before the daemon gives up waiting for it.
pub(crate) const ROUTE_GRACE: Duration = Duration::from_secs(1);

async fn connect_daemon() -> Result<RendezvousClient<Channel>, DaemonAccessError> {
    let endpoint = rendezvous_core::default_local_endpoint()?;
    Ok(tokio::time::timeout(CONNECT_TIMEOUT, rendezvous_client::connect(&endpoint))
        .await
        .map_err(|_| DaemonAccessError::TimedOut)??)
}

/// One managed registration that could not be read.
#[derive(Debug, thiserror::Error)]
#[error("a managed registration could not be read")]
pub(crate) struct UnreadableRegistration(#[source] wire::WireError);

/// Managed registrations held by the local daemon.
pub(crate) struct DaemonManagedSource;

impl ManagedSource for DaemonManagedSource {
    fn list(&self) -> DiscoveryFuture<ManagedListing> {
        Box::pin(async {
            let mut client = connect_daemon().await?;
            let targets = client
                .list_managed_targets(ListManagedTargetsRequest {})
                .await
                .map_err(DaemonAccessError::Rejected)?
                .into_inner()
                .targets;
            let mut listing = ManagedListing::default();
            for info in targets {
                match wire::info_to_listing(info) {
                    Ok(row) => listing.sessions.push(row),
                    Err(unreadable) => listing.problems.push(Box::new(UnreadableRegistration(unreadable))),
                }
            }
            Ok(listing)
        })
    }
}

/// Why routing could not report an owner-established result.
#[derive(Debug, thiserror::Error)]
pub(crate) enum RouteFailure {
    #[error("no local steering route")]
    NoRoute(#[source] DaemonAccessError),
    #[error("the request was rejected before it was forwarded")]
    Rejected(#[source] tonic::Status),
    #[error("routing failed after the request was sent")]
    AfterSend(#[source] tonic::Status),
    #[error("the owner's reply could not be read")]
    UnreadableReply(#[source] wire::WireError),
    #[error("the daemon reported an owner reply without one")]
    MissingReply,
    /// The daemon's own content-free explanation (a String wire field).
    #[error("{0}")]
    Daemon(String),
    #[error("the selected target changed: {0}")]
    Stale(String),
    #[error("the daemon could not establish whether the owner accepted the request")]
    Undetermined,
}

/// What routing established for one request.
#[derive(Debug)]
pub(crate) struct RoutedSend {
    pub(crate) result: SteeringResult,
    /// The owner's own (already redacted) explanation.
    owner_detail: Option<RedactedText>,
    /// Why nothing owner-established came back, when that is the case.
    pub(crate) failure: Option<RouteFailure>,
}

impl RoutedSend {
    /// The explanation to show for `request`, redacted against its message.
    pub(crate) fn detail(&self, request: &SteeringRequest) -> Option<RedactedText> {
        match (&self.owner_detail, &self.failure) {
            (Some(detail), _) => Some(detail.clone()),
            (None, Some(failure)) => {
                let text = match std::error::Error::source(failure) {
                    Some(cause) => render_chain(&failure.to_string(), cause),
                    None => failure.to_string(),
                };
                Some(wire::redact_detail(request, &text))
            }
            (None, None) => None,
        }
    }
}

/// How long the daemon may wait for `request`'s owner.
fn route_deadline(request: &SteeringRequest) -> u32 {
    let now = tokio::time::Instant::now();
    let budget = DeliveryDeadlines::for_request(request, now).acceptance.duration_since(now) + ROUTE_GRACE;
    u32::try_from(budget.as_millis()).unwrap_or(u32::MAX).min(MAX_ROUTE_DEADLINE_MS)
}

/// Routes `request`, selected under `expected`, to its owner.
pub(crate) async fn route_to_owner(request: &SteeringRequest, expected: &ManagedTarget, provider: Provider) -> RoutedSend {
    let unconfirmed = |outcome: SendOutcome, failure: RouteFailure| RoutedSend {
        result: SteeringResult::submitted(request, None, outcome),
        owner_detail: None,
        failure: Some(failure),
    };
    let mut client = match connect_daemon().await {
        Ok(client) => client,
        Err(unreachable) => return unconfirmed(SendOutcome::Unavailable, RouteFailure::NoRoute(unreachable)),
    };
    let routed = client
        .route_steering(RouteSteeringRequest {
            delivery: Some(wire::request_to_delivery(request, expected)),
            deadline_ms: route_deadline(request),
        })
        .await;
    let response = match routed {
        Ok(response) => response.into_inner(),
        // Rejected on validation, before anything was forwarded.
        Err(status) if status.code() == tonic::Code::InvalidArgument => {
            return unconfirmed(SendOutcome::Refused, RouteFailure::Rejected(status));
        }
        Err(status) => return unconfirmed(SendOutcome::Unknown, RouteFailure::AfterSend(status)),
    };
    match RouteOutcome::try_from(response.outcome) {
        Ok(RouteOutcome::OwnerReplied) => {
            let Some(reply) = response.reply else {
                return unconfirmed(SendOutcome::Unknown, RouteFailure::MissingReply);
            };
            match wire::reply_from_wire(request, provider, &reply) {
                Ok(result) => RoutedSend {
                    result,
                    owner_detail: reply.detail.as_deref().map(|detail| wire::redact_detail(request, detail)),
                    failure: None,
                },
                Err(unreadable) => unconfirmed(SendOutcome::Unknown, RouteFailure::UnreadableReply(unreadable)),
            }
        }
        Ok(RouteOutcome::NoOwner) => unconfirmed(SendOutcome::Unavailable, RouteFailure::Daemon(response.detail)),
        Ok(RouteOutcome::StaleTarget) => unconfirmed(SendOutcome::Unavailable, RouteFailure::Stale(response.detail)),
        Ok(RouteOutcome::DuplicateRequest) => unconfirmed(SendOutcome::Refused, RouteFailure::Daemon(response.detail)),
        Ok(RouteOutcome::Busy) => unconfirmed(SendOutcome::Busy, RouteFailure::Daemon(response.detail)),
        Ok(RouteOutcome::Unknown) => unconfirmed(SendOutcome::Unknown, RouteFailure::Daemon(response.detail)),
        Ok(RouteOutcome::Unspecified) | Err(_) => unconfirmed(SendOutcome::Unknown, RouteFailure::Undetermined),
    }
}
