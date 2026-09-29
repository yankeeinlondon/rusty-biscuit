//! Conversions between Claudine's steering types and the Rendezvous wire.
//!
//! Rendezvous carries Claudine values as their snake_case wire strings and
//! never interprets them; this module is the one place that turns them back
//! into typed values. Every load-bearing field is parsed strictly: an
//! unknown, missing, or malformed value is a [`WireError`], never a default.

use chrono::{DateTime, Utc};
use claudine::provider::Provider;
use claudine::secrets::RedactedText;
use claudine::steering::contract::{
    CancellationOutcome, InterruptionConsent, InterruptionOutcome, MessageError, SendOutcome, SteeringMessage,
    SteeringOrigin, SteeringRequest, SteeringResult,
};
use claudine::steering::controller::{ControllerReply, ControllerSnapshot, Submission};
use claudine::steering::discovery::{ObservationSource, SessionListing};
use claudine::steering::eligibility::AvailabilitySummary;
use claudine::steering::identity::{ConversationGeneration, IdentityError, ManagedTarget, ProcessStartIdentity, RequestId};
use claudine::steering::vocabulary::{OperationIntent, SteeringAvailability};
use rendezvous_core::{ManagedTargetInfo, SteeringDelivery, SteeringReply, TargetBinding};
use serde::Serialize;
use serde::de::DeserializeOwned;

/// A wire value that could not be read.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub(crate) enum WireError {
    #[error("`{field}` is missing")]
    Missing { field: &'static str },
    #[error("`{field}` has the unknown value `{value}`")]
    UnknownValue { field: &'static str, value: String },
    #[error("`{field}` is not a valid identity")]
    Identity {
        field: &'static str,
        #[source]
        source: IdentityError,
    },
    #[error("`message` is not a deliverable steering message")]
    Message(#[source] MessageError),
    #[error("`{field}` {problem}")]
    Inconsistent { field: &'static str, problem: &'static str },
}

fn identity(field: &'static str) -> impl FnOnce(IdentityError) -> WireError {
    move |source| WireError::Identity { field, source }
}

/// The wire string of a snake_case serde enum.
pub(crate) fn wire<T: Serialize>(value: T) -> String {
    match serde_json::to_value(value) {
        Ok(serde_json::Value::String(text)) => text,
        other => unreachable!("steering wire enums serialize as strings, got {other:?}"),
    }
}

fn parse<T: DeserializeOwned>(field: &'static str, text: &str) -> Result<T, WireError> {
    // A value outside the vocabulary is the whole finding; serde's message
    // would only restate it.
    serde_json::from_value(serde_json::Value::String(text.to_string()))
        .map_err(|_| WireError::UnknownValue { field, value: text.to_string() })
}

fn provider(field: &'static str, slug: &str) -> Result<Provider, WireError> {
    claudine::steering::provider_by_slug(slug).ok_or_else(|| WireError::UnknownValue { field, value: slug.to_string() })
}

pub(crate) fn binding_to_wire(target: &ManagedTarget) -> TargetBinding {
    TargetBinding {
        execution_id: target.execution.to_string(),
        wrapper_pid: target.wrapper.pid,
        wrapper_start: target.wrapper.start().to_string(),
        generation: target.generation.0,
        conversation: target.conversation.clone(),
    }
}

pub(crate) fn binding_from_wire(binding: &TargetBinding) -> Result<ManagedTarget, WireError> {
    Ok(ManagedTarget {
        execution: binding.execution_id.parse().map_err(identity("binding.execution_id"))?,
        wrapper: ProcessStartIdentity::new(binding.wrapper_pid, binding.wrapper_start.clone())
            .map_err(identity("binding.wrapper_start"))?,
        generation: ConversationGeneration(binding.generation),
        conversation: binding.conversation.clone(),
    })
}

/// The owner's registration for `snapshot`, stamped now.
pub(crate) fn snapshot_to_info(snapshot: &ControllerSnapshot) -> ManagedTargetInfo {
    let facts = &snapshot.facts;
    ManagedTargetInfo {
        binding: Some(binding_to_wire(&snapshot.target)),
        provider: facts.provider.as_slug().to_string(),
        cwd: facts.cwd.clone(),
        name: facts.name.clone(),
        state: wire(snapshot.state),
        launch_profile: facts.profile_id.clone(),
        provider_version: facts.provider_version.clone(),
        availability: wire(snapshot.availability.availability),
        reason: snapshot.availability.reason.clone(),
        setup_requirements: snapshot.availability.setup_requirements.clone(),
        provider_pid: snapshot.provider_process.as_ref().map(|process| process.pid),
        provider_start: snapshot.provider_process.as_ref().map(|process| process.start().to_string()),
        observed_at_unix_ms: Utc::now().timestamp_millis(),
    }
}

/// A listing row for one registration, keeping the owner's availability.
pub(crate) fn info_to_listing(info: ManagedTargetInfo) -> Result<SessionListing, WireError> {
    let binding = info.binding.as_ref().ok_or(WireError::Missing { field: "binding" })?;
    let target = binding_from_wire(binding)?;
    let availability: SteeringAvailability = parse("availability", &info.availability)?;
    let provider_process = match (info.provider_pid, info.provider_start.as_deref()) {
        (Some(pid), Some(start)) => Some(ProcessStartIdentity::new(pid, start).map_err(identity("provider_start"))?),
        (None, None) => None,
        _ => {
            return Err(WireError::Inconsistent {
                field: "provider_pid",
                problem: "must be sent together with `provider_start`",
            });
        }
    };
    Ok(SessionListing {
        id: target.id(),
        provider: provider("provider", &info.provider)?,
        name: info.name,
        cwd: info.cwd,
        state: parse("state", &info.state)?,
        origins: vec![ObservationSource::Managed],
        launch_profile: info.launch_profile,
        provider_version: info.provider_version,
        availability: AvailabilitySummary {
            availability,
            reason: info.reason,
            setup_requirements: info.setup_requirements,
        },
        observed_at: DateTime::<Utc>::from_timestamp_millis(info.observed_at_unix_ms)
            .ok_or(WireError::Inconsistent { field: "observed_at_unix_ms", problem: "is out of range" })?,
        session_key: provider_process.zip(target.conversation),
    })
}

/// The delivery a requester routes for `request` selected under `expected`.
pub(crate) fn request_to_delivery(request: &SteeringRequest, expected: &ManagedTarget) -> SteeringDelivery {
    SteeringDelivery {
        request_id: request.id.to_string(),
        expected: Some(binding_to_wire(expected)),
        origin: wire(request.origin),
        operation: wire(request.operation),
        message: request.message.as_str().to_string(),
        consented_operation: request.consent.as_ref().map(|consent| wire(consent.operation)),
    }
}

/// The owner's submission for a routed delivery. Routed requests are always
/// manual: automatic help is raised inside the owner, never routed to it.
pub(crate) fn delivery_to_submission(delivery: SteeringDelivery) -> Result<Submission, WireError> {
    let id: RequestId = delivery.request_id.parse().map_err(identity("request_id"))?;
    let expected = binding_from_wire(delivery.expected.as_ref().ok_or(WireError::Missing { field: "expected" })?)?;
    let origin: SteeringOrigin = parse("origin", &delivery.origin)?;
    if origin != SteeringOrigin::Manual {
        return Err(WireError::Inconsistent { field: "origin", problem: "must be manual; automatic help is never routed" });
    }
    let operation: OperationIntent = parse("operation", &delivery.operation)?;
    let target = expected.id();
    let consent = delivery
        .consented_operation
        .as_deref()
        .map(|consented| {
            parse::<OperationIntent>("consented_operation", consented)
                .map(|operation| InterruptionConsent { target: target.clone(), operation })
        })
        .transpose()?;
    let message = SteeringMessage::new(delivery.message).map_err(WireError::Message)?;
    Ok(Submission {
        request: SteeringRequest { id, target, origin, operation, message, consent },
        expected: Some(expected),
        opportunity: None,
    })
}

/// The owner's reply for one delivery.
pub(crate) fn reply_to_wire(request_id: &str, reply: &ControllerReply) -> SteeringReply {
    let interruption = reply.result.interruption;
    SteeringReply {
        request_id: request_id.to_string(),
        outcome: wire(reply.result.outcome),
        mechanism: reply.result.mechanism.map(str::to_string),
        cancellation: interruption.map(|phases| wire(phases.cancellation)),
        replacement: interruption.and_then(|phases| phases.replacement).map(wire),
        detail: reply.detail.as_ref().map(|detail| detail.as_str().to_string()),
    }
}

/// A refusal for a delivery the owner could not read. Nothing was submitted.
/// The reply's `detail` is a String wire field, so the typed cause is
/// rendered here, at that boundary.
pub(crate) fn refusal(request_id: &str, unreadable: &WireError) -> SteeringReply {
    SteeringReply {
        request_id: request_id.to_string(),
        outcome: wire(SendOutcome::Refused),
        detail: Some(super::render_chain("the owner could not read the request", unreadable)),
        ..SteeringReply::default()
    }
}

/// The typed result for `request` from its owner's `reply`.
pub(crate) fn reply_from_wire(
    request: &SteeringRequest,
    provider: Provider,
    reply: &SteeringReply,
) -> Result<SteeringResult, WireError> {
    if reply.request_id != request.id.to_string() {
        return Err(WireError::Inconsistent { field: "request_id", problem: "belongs to a different request" });
    }
    let outcome: SendOutcome = parse("outcome", &reply.outcome)?;
    let mechanism = match reply.mechanism.as_deref() {
        Some(id) => Some(
            claudine::steering::facts(provider)
                .mechanisms
                .iter()
                .find(|mechanism| mechanism.id == id)
                .map(|mechanism| mechanism.id)
                .ok_or_else(|| WireError::UnknownValue { field: "mechanism", value: id.to_string() })?,
        ),
        None => None,
    };
    let interruption = match reply.cancellation.as_deref() {
        Some(cancellation) => Some(InterruptionOutcome {
            cancellation: parse::<CancellationOutcome>("cancellation", cancellation)?,
            replacement: reply.replacement.as_deref().map(|text| parse("replacement", text)).transpose()?,
        }),
        None if reply.replacement.is_some() => {
            return Err(WireError::Inconsistent { field: "replacement", problem: "needs a `cancellation` outcome" });
        }
        None => None,
    };
    Ok(SteeringResult {
        request_id: request.id,
        target: request.target.clone(),
        mechanism,
        outcome,
        receipt: outcome.receipt(),
        interruption,
    })
}

/// Redacts owner-provided detail text against the request's own message.
pub(crate) fn redact_detail(request: &SteeringRequest, detail: &str) -> RedactedText {
    claudine::secrets::Redactor::for_message(request.message.as_str()).redact(detail)
}
