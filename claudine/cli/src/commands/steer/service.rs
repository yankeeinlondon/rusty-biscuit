//! Where `claudine steer` discovers sessions and delivers a request.
//!
//! The command talks to one [`SteeringService`]; production uses
//! [`LocalService`] (managed registrations from the local Rendezvous daemon,
//! routed to their owner), and tests substitute fakes so selection, consent,
//! and receipts are exercised without a provider.

use std::sync::Arc;

use claudine::secrets::RedactedText;
use claudine::steering::contract::{SendOutcome, SteeringRequest, SteeringResult};
use claudine::steering::discovery::{DiscoveryFailed, DiscoveryReport, ManagedSource, SessionListing};
use claudine::steering::identity::SteeringTargetId;

use crate::steering::requester;

/// What one delivery attempt established.
#[derive(Debug, Clone)]
pub(crate) struct Delivery {
    pub(crate) result: SteeringResult,
    /// Why the request was not confirmed, already redacted against the
    /// message.
    pub(crate) detail: Option<RedactedText>,
}

/// Discovery and delivery for the steer command.
pub(crate) trait SteeringService {
    /// One bounded discovery pass over every source this build has.
    async fn discover(&self) -> Result<DiscoveryReport, DiscoveryFailed>;

    /// Delivers `request` to the session `row` describes, bound to the
    /// identity `row` was listed with.
    async fn deliver(&self, row: &SessionListing, request: &SteeringRequest) -> Delivery;
}

/// Managed sessions through the local Rendezvous daemon. This build has no
/// native discoverer, so natively launched sessions are not listed and a
/// native target cannot be delivered to.
pub(crate) struct LocalService;

/// Why a native target cannot be delivered to by this build.
pub(crate) const NATIVE_DELIVERY_UNIMPLEMENTED: &str =
    "this build has no delivery adapter for natively launched sessions; nothing was sent";

/// Why a managed row without its owner's binding cannot be routed.
pub(crate) const MISSING_BINDING: &str =
    "the listing did not carry the owner's binding for this session; list again and reselect";

impl SteeringService for LocalService {
    async fn discover(&self) -> Result<DiscoveryReport, DiscoveryFailed> {
        let managed: Arc<dyn ManagedSource> = Arc::new(requester::DaemonManagedSource);
        claudine::steering::discovery::discover(Some(managed), &[]).await
    }

    async fn deliver(&self, row: &SessionListing, request: &SteeringRequest) -> Delivery {
        let unsent = |detail: &str| Delivery {
            result: SteeringResult::submitted(request, None, SendOutcome::Unavailable),
            detail: Some(claudine::secrets::Redactor::for_message(request.message.as_str()).redact(detail)),
        };
        match (&row.id, &row.binding) {
            (SteeringTargetId::Managed { .. }, Some(binding)) => {
                let routed = requester::route_to_owner(request, binding, row.provider).await;
                let detail = routed.detail(request);
                Delivery { result: routed.result, detail }
            }
            (SteeringTargetId::Managed { .. }, None) => unsent(MISSING_BINDING),
            (SteeringTargetId::Native { .. }, _) => unsent(NATIVE_DELIVERY_UNIMPLEMENTED),
        }
    }
}
