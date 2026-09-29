//! Provider-neutral steering request and result contracts.
//!
//! A result reports only what the provider established: the receipt
//! strength follows from the outcome, an uncertain submission is
//! [`SendOutcome::Unknown`] rather than refused, and interruption keeps its
//! cancellation and replacement outcomes separate so a stopped turn whose
//! replacement failed is reported as [`SendOutcome::PartialInterruption`].

use std::fmt;

use serde::{Deserialize, Serialize};

use super::identity::{RequestId, SteeringTargetId};
use super::vocabulary::{OperationIntent, ReceiptStrength};

/// Largest steering message accepted before dispatch, in UTF-8 bytes.
pub const MAX_MESSAGE_BYTES: usize = 64 * 1024;

/// Who asked for the steering message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SteeringOrigin {
    /// `claudine steer`.
    Manual,
    /// An automatic repetition warning.
    Automatic,
}

/// Why a message was rejected before dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MessageError {
    #[error("steering message is empty or whitespace-only")]
    Empty,
    #[error("steering message contains a NUL character")]
    ContainsNul,
    #[error("steering message is {bytes} bytes; the limit is {limit} bytes")]
    TooLong { bytes: usize, limit: usize },
}

/// A validated steering message. Its text is delivered byte-for-byte; it is
/// never trimmed, truncated, or split. `Debug` omits the text so it cannot
/// leak into traces unredacted.
#[derive(Clone, PartialEq, Eq)]
pub struct SteeringMessage(String);

impl SteeringMessage {
    /// Validates `text` against the general limits.
    pub fn new(text: impl Into<String>) -> Result<Self, MessageError> {
        let text = text.into();
        if text.trim().is_empty() {
            return Err(MessageError::Empty);
        }
        if text.contains('\0') {
            return Err(MessageError::ContainsNul);
        }
        check_length(text.len(), MAX_MESSAGE_BYTES)?;
        Ok(Self(text))
    }

    /// Additionally enforces a smaller verified provider limit.
    pub fn check_provider_limit(&self, limit: usize) -> Result<(), MessageError> {
        check_length(self.0.len(), limit.min(MAX_MESSAGE_BYTES))
    }

    /// The original text, for in-memory delivery only.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn check_length(bytes: usize, limit: usize) -> Result<(), MessageError> {
    if bytes > limit {
        return Err(MessageError::TooLong { bytes, limit });
    }
    Ok(())
}

impl fmt::Debug for SteeringMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "SteeringMessage({} bytes)", self.0.len())
    }
}

/// Interactive consent to interrupt, bound to one target and operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InterruptionConsent {
    pub target: SteeringTargetId,
    pub operation: OperationIntent,
}

/// One steering request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SteeringRequest {
    pub id: RequestId,
    pub target: SteeringTargetId,
    pub origin: SteeringOrigin,
    pub operation: OperationIntent,
    pub message: SteeringMessage,
    pub consent: Option<InterruptionConsent>,
}

impl SteeringRequest {
    /// Whether this request may interrupt running work. Only an explicit
    /// manual consent for this exact target and operation authorizes it;
    /// automatic requests never interrupt.
    pub fn may_interrupt(&self) -> bool {
        self.origin == SteeringOrigin::Manual
            && self.operation == OperationIntent::InterruptThenSubmit
            && self
                .consent
                .as_ref()
                .is_some_and(|consent| consent.target == self.target && consent.operation == self.operation)
    }
}

/// Provider-established result of one submission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SendOutcome {
    Accepted,
    Queued,
    Delivered,
    Refused,
    /// Held by provider inbound policy; undelivered until policy changes.
    Held,
    Unavailable,
    /// The owner could not take the request: its bounded queue was full, or
    /// the request expired before submission. Nothing was submitted.
    Busy,
    /// Cancellation succeeded but the replacement was not confirmed.
    PartialInterruption,
    /// Submission may have happened; nothing was confirmed. Never retried.
    Unknown,
}

impl SendOutcome {
    /// Receipt strength this outcome establishes.
    pub fn receipt(self) -> ReceiptStrength {
        match self {
            Self::Accepted => ReceiptStrength::Accepted,
            Self::Queued => ReceiptStrength::Queued,
            Self::Delivered => ReceiptStrength::Delivered,
            Self::Refused
            | Self::Held
            | Self::Unavailable
            | Self::Busy
            | Self::PartialInterruption
            | Self::Unknown => ReceiptStrength::Unknown,
        }
    }

    /// Whether the provider confirmed acceptance, queueing, or delivery.
    pub fn is_confirmed(self) -> bool {
        self.receipt() > ReceiptStrength::Unknown
    }

    /// This outcome, lowered so its receipt does not exceed `ceiling` (the
    /// strongest receipt the delivering mechanism can establish). A report
    /// stronger than the mechanism can prove is never passed on.
    pub fn capped_at(self, ceiling: ReceiptStrength) -> Self {
        if self.receipt() <= ceiling {
            return self;
        }
        match ceiling {
            ReceiptStrength::Delivered => Self::Delivered,
            ReceiptStrength::Queued => Self::Queued,
            ReceiptStrength::Accepted => Self::Accepted,
            ReceiptStrength::Unknown => Self::Unknown,
        }
    }
}

/// Whether consented cancellation took effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CancellationOutcome {
    Established,
    Refused,
    /// No completion evidence arrived within the deadline.
    Unknown,
}

/// Separate outcomes of an interrupt-then-submit operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct InterruptionOutcome {
    pub cancellation: CancellationOutcome,
    /// `None` when replacement was not submitted because cancellation was
    /// not established.
    pub replacement: Option<SendOutcome>,
}

impl InterruptionOutcome {
    /// The overall outcome. A replacement is never submitted unless
    /// cancellation was established; cancellation without a confirmed
    /// replacement is a partial outcome, not a failure to interrupt.
    pub fn overall(self) -> SendOutcome {
        match (self.cancellation, self.replacement) {
            (CancellationOutcome::Established, Some(outcome)) if outcome.is_confirmed() => outcome,
            (CancellationOutcome::Established, _) => SendOutcome::PartialInterruption,
            (CancellationOutcome::Refused, _) => SendOutcome::Refused,
            (CancellationOutcome::Unknown, _) => SendOutcome::Unknown,
        }
    }
}

/// The typed result of one steering request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SteeringResult {
    pub request_id: RequestId,
    pub target: SteeringTargetId,
    /// Researched mechanism used, when one was selected.
    pub mechanism: Option<&'static str>,
    pub outcome: SendOutcome,
    pub receipt: ReceiptStrength,
    pub interruption: Option<InterruptionOutcome>,
}

impl SteeringResult {
    /// A non-interrupting submission result.
    pub fn submitted(request: &SteeringRequest, mechanism: Option<&'static str>, outcome: SendOutcome) -> Self {
        Self {
            request_id: request.id,
            target: request.target.clone(),
            mechanism,
            outcome,
            receipt: outcome.receipt(),
            interruption: None,
        }
    }

    /// An interrupt-then-submit result; its outcome derives from the phases.
    pub fn interrupted(request: &SteeringRequest, mechanism: &'static str, phases: InterruptionOutcome) -> Self {
        let outcome = phases.overall();
        Self {
            request_id: request.id,
            target: request.target.clone(),
            mechanism: Some(mechanism),
            outcome,
            receipt: outcome.receipt(),
            interruption: Some(phases),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::steering::identity::ExecutionId;

    fn request(origin: SteeringOrigin, operation: OperationIntent) -> SteeringRequest {
        SteeringRequest {
            id: RequestId::from_u128(1),
            target: SteeringTargetId::Managed { execution: ExecutionId::from_u128(2) },
            origin,
            operation,
            message: SteeringMessage::new("check the failing test").unwrap(),
            consent: None,
        }
    }

    #[test]
    fn message_validation_preserves_text_and_rejects_bad_input() {
        let original = "  keep leading space\n\tand tabs ✓ ";
        assert_eq!(SteeringMessage::new(original).unwrap().as_str(), original);
        assert_eq!(SteeringMessage::new(" \n\t").unwrap_err(), MessageError::Empty);
        assert_eq!(SteeringMessage::new("").unwrap_err(), MessageError::Empty);
        assert_eq!(SteeringMessage::new("a\0b").unwrap_err(), MessageError::ContainsNul);
        assert!(SteeringMessage::new("x".repeat(MAX_MESSAGE_BYTES)).is_ok());
        assert_eq!(
            SteeringMessage::new("x".repeat(MAX_MESSAGE_BYTES + 1)).unwrap_err(),
            MessageError::TooLong { bytes: MAX_MESSAGE_BYTES + 1, limit: MAX_MESSAGE_BYTES }
        );
        // Multibyte characters count as UTF-8 bytes, not characters.
        let multibyte = "é".repeat(MAX_MESSAGE_BYTES / 2 + 1);
        assert!(matches!(SteeringMessage::new(multibyte), Err(MessageError::TooLong { .. })));
    }

    #[test]
    fn provider_limit_is_enforced_without_truncation() {
        let message = SteeringMessage::new("abcdef").unwrap();
        assert_eq!(message.check_provider_limit(6), Ok(()));
        assert_eq!(message.check_provider_limit(5), Err(MessageError::TooLong { bytes: 6, limit: 5 }));
        assert_eq!(message.as_str(), "abcdef");
    }

    #[test]
    fn debug_never_prints_message_text() {
        let message = SteeringMessage::new("token=sk-secret").unwrap();
        assert_eq!(format!("{message:?}"), "SteeringMessage(15 bytes)");
        assert!(!format!("{:?}", request(SteeringOrigin::Manual, OperationIntent::SteerActiveTurn)).contains("failing"));
    }

    #[test]
    fn only_matching_manual_consent_permits_interruption() {
        let mut manual = request(SteeringOrigin::Manual, OperationIntent::InterruptThenSubmit);
        assert!(!manual.may_interrupt(), "no consent");
        manual.consent = Some(InterruptionConsent { target: manual.target.clone(), operation: manual.operation });
        assert!(manual.may_interrupt());

        let mut other_target = manual.clone();
        other_target.consent = Some(InterruptionConsent {
            target: SteeringTargetId::Managed { execution: ExecutionId::from_u128(3) },
            operation: OperationIntent::InterruptThenSubmit,
        });
        assert!(!other_target.may_interrupt(), "consent is bound to its target");

        let mut automatic = manual.clone();
        automatic.origin = SteeringOrigin::Automatic;
        assert!(!automatic.may_interrupt(), "automatic help never interrupts");

        let mut steer = manual.clone();
        steer.operation = OperationIntent::SteerActiveTurn;
        assert!(!steer.may_interrupt(), "consent is bound to its operation");
    }

    #[test]
    fn only_provider_confirmation_is_success() {
        for outcome in [SendOutcome::Accepted, SendOutcome::Queued, SendOutcome::Delivered] {
            assert!(outcome.is_confirmed(), "{outcome:?}");
        }
        for outcome in [
            SendOutcome::Refused,
            SendOutcome::Held,
            SendOutcome::Unavailable,
            SendOutcome::Busy,
            SendOutcome::PartialInterruption,
            SendOutcome::Unknown,
        ] {
            assert!(!outcome.is_confirmed(), "{outcome:?}");
            assert_eq!(outcome.receipt(), ReceiptStrength::Unknown);
        }
    }

    #[test]
    fn interruption_outcomes_stay_separate_and_honest() {
        let partial = InterruptionOutcome { cancellation: CancellationOutcome::Established, replacement: Some(SendOutcome::Unknown) };
        assert_eq!(partial.overall(), SendOutcome::PartialInterruption);
        let not_sent = InterruptionOutcome { cancellation: CancellationOutcome::Established, replacement: None };
        assert_eq!(not_sent.overall(), SendOutcome::PartialInterruption);
        let complete = InterruptionOutcome { cancellation: CancellationOutcome::Established, replacement: Some(SendOutcome::Queued) };
        assert_eq!(complete.overall(), SendOutcome::Queued);
        let refused = InterruptionOutcome { cancellation: CancellationOutcome::Refused, replacement: None };
        assert_eq!(refused.overall(), SendOutcome::Refused);
        let unknown = InterruptionOutcome { cancellation: CancellationOutcome::Unknown, replacement: None };
        assert_eq!(unknown.overall(), SendOutcome::Unknown);

        let request = request(SteeringOrigin::Manual, OperationIntent::InterruptThenSubmit);
        let result = SteeringResult::interrupted(&request, "abort-submit", partial);
        assert_eq!(result.outcome, SendOutcome::PartialInterruption);
        assert_eq!(result.receipt, ReceiptStrength::Unknown);
        assert_eq!(result.interruption, Some(partial));
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["outcome"], "partial_interruption");
        assert_eq!(json["interruption"]["cancellation"], "established");
        assert_eq!(json["interruption"]["replacement"], "unknown");
        assert_eq!(json["target"], "managed:00000000-0000-0000-0000-000000000002");
    }

    #[test]
    fn submitted_result_receipt_follows_outcome() {
        let request = request(SteeringOrigin::Automatic, OperationIntent::SteerActiveTurn);
        let result = SteeringResult::submitted(&request, Some("rpc-steer"), SendOutcome::Accepted);
        assert_eq!(result.receipt, ReceiptStrength::Accepted);
        assert_eq!(result.interruption, None);
        let unknown = SteeringResult::submitted(&request, None, SendOutcome::Unknown);
        assert_eq!(unknown.receipt, ReceiptStrength::Unknown);
    }
}
