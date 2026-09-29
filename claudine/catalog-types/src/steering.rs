//! Shared steering and execution-interface vocabulary.
//!
//! These types describe researched provider facts (mechanisms, capability
//! cases, verification) and the reviewed activation grants that the generator
//! emits into `claudine/lib/src/steering/generated.rs`. The pure rule methods
//! on them ([`OperationIntent::supports`], [`SteeringMechanism::rescues_active_loop`],
//! [`ReceiptTiming::supports_prompt_return`], [`OperationIntent::required_assertions`])
//! are the single implementation shared by the generator's activation checker
//! and the library's runtime eligibility.
//!
//! Transport, request encoding, operation intent, receipt strength, execution
//! state, and delivery state are deliberately separate types: one never
//! implies another.
//!
//! Design authority: `2026-09-08-steering` (Resolved Engineering Contract).

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumIter, EnumString, IntoStaticStr, VariantNames};

macro_rules! wire_enum {
    ($(#[$meta:meta])* pub enum $name:ident { $($(#[$vmeta:meta])* $variant:ident),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(
            Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash,
            Serialize, Deserialize, AsRefStr, Display, EnumIter, EnumString,
            IntoStaticStr, VariantNames,
        )]
        #[serde(rename_all = "snake_case")]
        #[strum(serialize_all = "snake_case")]
        pub enum $name { $($(#[$vmeta])* $variant),+ }
    };
}

wire_enum! {
    /// Operating system a research case or verification applies to. Native
    /// Windows is distinct from Linux/WSL.
    pub enum HostOs { Macos, Linux, Windows }
}

wire_enum! {
    /// How the provider session was launched.
    pub enum LaunchMode { Interactive, NonInteractive }
}

wire_enum! {
    /// Who launched the provider session.
    pub enum LaunchOrigin { Native, Claudine }
}

wire_enum! {
    /// Observed activity of a session. `Unknown` is explicit: idle is never
    /// inferred from absence of evidence.
    pub enum ExecutionState {
        /// A turn is in progress.
        Working,
        /// The session is open and waiting for input.
        Idle,
        /// State could not be established.
        Unknown,
    }
}

wire_enum! {
    /// Carrier transport of a steering mechanism (not its wire encoding).
    pub enum SteeringTransport {
        UnixSocket, NamedPipe, Http, Websocket, Stdio, Hook, Extension, Other, Unknown,
    }
}

wire_enum! {
    /// What a steering operation asks the provider to do.
    pub enum OperationIntent {
        /// Inject input into the current active turn.
        SteerActiveTurn,
        /// Start a new turn in an idle session.
        StartIdleTurn,
        /// Cancel current work, then submit replacement input.
        InterruptThenSubmit,
        /// Queue input consumed after current work.
        QueueFollowUp,
        Unknown,
    }
}

wire_enum! {
    /// Effect of an operation on the running conversation.
    pub enum ConversationEffect {
        PreserveRunningTurn, CancelTurnSameConversation, ResumeSameConversation,
        NewConversation, Unknown,
    }
}

wire_enum! {
    /// When delivered input reaches the model.
    pub enum DeliveryBoundary {
        DuringGeneration, NextToolBoundary, EndOfToolBatch, NextTurn, IdleTurnStart, Unknown,
    }
}

wire_enum! {
    /// Provider-side state of one submitted message.
    pub enum DeliveryState { Accepted, Queued, Held, Delivered, Refused, Expired, Unknown }
}

wire_enum! {
    /// What a sender can honestly report about a submission, weakest first.
    /// Ordering is meaningful: a stronger receipt implies the weaker ones.
    pub enum ReceiptStrength {
        /// No provider confirmation was established.
        Unknown,
        /// The provider accepted the request.
        Accepted,
        /// The provider confirmed it will deliver the message later.
        Queued,
        /// The provider confirmed delivery into the conversation.
        Delivered,
    }
}

wire_enum! {
    /// When a mechanism's acknowledgment becomes available.
    pub enum ReceiptTiming { Early, Terminal, MultiPhase, None, Unknown }
}

wire_enum! {
    /// One researched receipt guarantee, normalized from its per-fact
    /// research members (`confirmed`, `rejected`/`not_*`, `unknown`).
    pub enum GuaranteeLevel { Confirmed, Negative, Unknown }
}

wire_enum! {
    /// Researched support verdict for one capability case.
    pub enum CaseSupport { NonInterrupting, InterruptionRequired, Unsupported, Unknown }
}

wire_enum! {
    /// Availability of steering for one concrete session.
    pub enum SteeringAvailability { NonInterrupting, InterruptionRequired, Unavailable }
}

wire_enum! {
    /// Researched access to a mechanism for one profile and OS.
    pub enum AccessStatus { Available, SetupRequired, Blocked, Unknown }
}

wire_enum! {
    /// Outcome of one live verification record.
    pub enum VerificationOutcome { Passed, Failed, Inconclusive }
}

wire_enum! {
    /// Typed claim a live verification record establishes.
    pub enum AssertionKind {
        /// Input reached the intended session and conversation.
        TargetIdentity,
        /// A correlated provider acceptance was observed before completion.
        AcceptanceSignal,
        /// The message was observed in the conversation's model input.
        ConversationDelivery,
        /// The delivery boundary matched the researched claim.
        DeliveryBoundary,
        /// Running work continued without cancellation.
        RunningWorkPreserved,
        /// Cancellation completed before replacement input was submitted.
        CancellationEstablished,
        /// Extensions, skills, templates, and context stayed enabled.
        ResourcePreservation,
        /// Duplicate-submission behavior was recorded.
        DuplicateBehavior,
        /// Owned process cleanup was recorded.
        ProcessCleanup,
        /// The record documents a loss or failure boundary. Such a record is
        /// regression evidence and never counts toward activation.
        ExpectedLoss,
    }
}

wire_enum! {
    /// Kind of execution interface in the non-interactive topic.
    pub enum ExecutionInterfaceKind {
        OneShotCli, RetainedSubprocess, LocalServer, NetworkServer, Sdk, AttachResume, Other,
    }
}

impl OperationIntent {
    /// The case support this operation provides in `state`, given its
    /// researched conversation effect. `None` means the operation cannot
    /// serve that state (for example, an idle start for a working session).
    pub fn supports(self, state: ExecutionState, effect: ConversationEffect) -> Option<CaseSupport> {
        use ConversationEffect as E;
        use OperationIntent as O;
        match (state, self, effect) {
            (ExecutionState::Working, O::SteerActiveTurn | O::QueueFollowUp, E::PreserveRunningTurn) => {
                Some(CaseSupport::NonInterrupting)
            }
            (ExecutionState::Idle, O::StartIdleTurn, E::PreserveRunningTurn | E::ResumeSameConversation) => {
                Some(CaseSupport::NonInterrupting)
            }
            (ExecutionState::Working, O::InterruptThenSubmit, E::CancelTurnSameConversation) => {
                Some(CaseSupport::InterruptionRequired)
            }
            _ => None,
        }
    }

    /// Assertions a verification record set must establish before this
    /// operation can be activated. `Unknown` can never be activated.
    pub fn required_assertions(self) -> Option<&'static [AssertionKind]> {
        use AssertionKind as A;
        match self {
            Self::SteerActiveTurn | Self::QueueFollowUp => Some(&[
                A::TargetIdentity,
                A::AcceptanceSignal,
                A::ConversationDelivery,
                A::RunningWorkPreserved,
            ]),
            Self::StartIdleTurn => {
                Some(&[A::TargetIdentity, A::AcceptanceSignal, A::ConversationDelivery])
            }
            Self::InterruptThenSubmit => Some(&[
                A::TargetIdentity,
                A::AcceptanceSignal,
                A::CancellationEstablished,
                A::ConversationDelivery,
            ]),
            Self::Unknown => None,
        }
    }
}

impl ReceiptTiming {
    /// Whether acknowledgment arrives early enough for `claudine steer` to
    /// return after acceptance. A terminal-only or absent acknowledgment
    /// cannot, and unknown timing is never assumed to.
    pub fn supports_prompt_return(self) -> bool {
        matches!(self, Self::Early | Self::MultiPhase)
    }
}

/// Researched receipt guarantees for one mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ReceiptGuarantees {
    pub request_acceptance: GuaranteeLevel,
    pub persistence: GuaranteeLevel,
    pub scheduling: GuaranteeLevel,
    pub conversation_delivery: GuaranteeLevel,
}

impl ReceiptGuarantees {
    /// The strongest receipt the protocol can ever establish. Persistence
    /// alone is not scheduling, so it never upgrades a receipt to queued.
    pub fn max_strength(&self) -> ReceiptStrength {
        if self.conversation_delivery == GuaranteeLevel::Confirmed {
            ReceiptStrength::Delivered
        } else if self.scheduling == GuaranteeLevel::Confirmed {
            ReceiptStrength::Queued
        } else if self.request_acceptance == GuaranteeLevel::Confirmed {
            ReceiptStrength::Accepted
        } else {
            ReceiptStrength::Unknown
        }
    }
}

/// One researched steering mechanism.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SteeringMechanism {
    pub id: &'static str,
    pub transport: SteeringTransport,
    /// Researched request encoding (prose; framing varies per protocol).
    pub request_format: &'static str,
    pub operation_intent: OperationIntent,
    pub conversation_effect: ConversationEffect,
    pub delivery_boundary: DeliveryBoundary,
    pub receipt_timing: ReceiptTiming,
    pub receipts: ReceiptGuarantees,
    /// Delivery states an external sender can observe.
    pub delivery_states: &'static [DeliveryState],
}

impl SteeringMechanism {
    /// Whether this mechanism can deliver automatic help into a turn that
    /// may never end. Requires current-work input that preserves the turn
    /// at a boundary reachable inside that turn; a next-turn follow-up, an
    /// idle start, an interruption, or an unknown boundary never qualifies.
    pub fn rescues_active_loop(&self) -> bool {
        matches!(
            self.operation_intent,
            OperationIntent::SteerActiveTurn | OperationIntent::QueueFollowUp
        ) && self.conversation_effect == ConversationEffect::PreserveRunningTurn
            && matches!(
                self.delivery_boundary,
                DeliveryBoundary::DuringGeneration
                    | DeliveryBoundary::NextToolBoundary
                    | DeliveryBoundary::EndOfToolBatch
            )
    }
}

/// One researched capability case: an exact profile/OS/mode/origin/state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SteeringCase {
    pub profile_id: &'static str,
    pub os: HostOs,
    pub launch_mode: LaunchMode,
    pub origin: LaunchOrigin,
    pub state: ExecutionState,
    pub support: CaseSupport,
    pub mechanism_ids: &'static [&'static str],
    pub reason: &'static str,
}

/// Researched access to one mechanism for a profile and OS.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SteeringAccess {
    pub mechanism_id: &'static str,
    pub profile_id: &'static str,
    pub os: HostOs,
    pub status: AccessStatus,
    pub prerequisite: &'static str,
}

/// One live verification record with its typed assertions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct SteeringVerification {
    pub id: &'static str,
    pub mechanism_id: &'static str,
    pub profile_id: &'static str,
    pub os: HostOs,
    pub provider_version: &'static str,
    pub launch_mode: LaunchMode,
    pub origin: LaunchOrigin,
    pub state: ExecutionState,
    pub outcome: VerificationOutcome,
    pub assertions: &'static [AssertionKind],
}

/// One researched execution interface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExecutionInterface {
    pub id: &'static str,
    pub kind: ExecutionInterfaceKind,
}

/// The researched execution-interface selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ExecutionSelection {
    pub preferred: &'static str,
    pub fallback: Option<&'static str>,
}

/// Every researched steering fact for one provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ProviderSteering {
    pub mechanisms: &'static [SteeringMechanism],
    pub cases: &'static [SteeringCase],
    pub access: &'static [SteeringAccess],
    pub verification: &'static [SteeringVerification],
    pub execution_interfaces: &'static [ExecutionInterface],
    pub execution_selection: Option<ExecutionSelection>,
}

/// A reviewed adapter implementation identifier and revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct AdapterRef {
    pub id: &'static str,
    pub revision: u32,
}

/// A reviewed activation grant: exactly one provider/version/OS/profile/
/// origin/state/operation for one mechanism through one adapter revision.
/// Grants are hand-owned policy, never inferred from research.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct ActivationGrant {
    pub provider: &'static str,
    pub mechanism_id: &'static str,
    pub operation: OperationIntent,
    pub adapter: AdapterRef,
    pub profile_id: &'static str,
    pub os: HostOs,
    pub provider_version: &'static str,
    pub launch_mode: LaunchMode,
    pub origin: LaunchOrigin,
    pub state: ExecutionState,
    pub verification_ids: &'static [&'static str],
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mechanism(intent: OperationIntent, boundary: DeliveryBoundary) -> SteeringMechanism {
        SteeringMechanism {
            id: "m",
            transport: SteeringTransport::Stdio,
            request_format: "",
            operation_intent: intent,
            conversation_effect: ConversationEffect::PreserveRunningTurn,
            delivery_boundary: boundary,
            receipt_timing: ReceiptTiming::Early,
            receipts: ReceiptGuarantees {
                request_acceptance: GuaranteeLevel::Confirmed,
                persistence: GuaranteeLevel::Unknown,
                scheduling: GuaranteeLevel::Unknown,
                conversation_delivery: GuaranteeLevel::Unknown,
            },
            delivery_states: &[],
        }
    }

    #[test]
    fn next_turn_follow_up_is_not_active_loop_rescue() {
        assert!(!mechanism(OperationIntent::QueueFollowUp, DeliveryBoundary::NextTurn).rescues_active_loop());
        assert!(mechanism(OperationIntent::QueueFollowUp, DeliveryBoundary::NextToolBoundary).rescues_active_loop());
        assert!(mechanism(OperationIntent::SteerActiveTurn, DeliveryBoundary::EndOfToolBatch).rescues_active_loop());
        assert!(!mechanism(OperationIntent::SteerActiveTurn, DeliveryBoundary::Unknown).rescues_active_loop());
        assert!(!mechanism(OperationIntent::StartIdleTurn, DeliveryBoundary::IdleTurnStart).rescues_active_loop());
    }

    #[test]
    fn interruption_never_rescues_even_at_a_tool_boundary() {
        let mut interrupt = mechanism(OperationIntent::InterruptThenSubmit, DeliveryBoundary::NextToolBoundary);
        interrupt.conversation_effect = ConversationEffect::CancelTurnSameConversation;
        assert!(!interrupt.rescues_active_loop());
    }

    #[test]
    fn terminal_or_unknown_acknowledgment_cannot_return_after_acceptance() {
        assert!(ReceiptTiming::Early.supports_prompt_return());
        assert!(ReceiptTiming::MultiPhase.supports_prompt_return());
        for timing in [ReceiptTiming::Terminal, ReceiptTiming::None, ReceiptTiming::Unknown] {
            assert!(!timing.supports_prompt_return(), "{timing}");
        }
    }

    #[test]
    fn operation_support_matches_state_and_effect() {
        use ConversationEffect as E;
        use ExecutionState as S;
        use OperationIntent as O;
        assert_eq!(O::SteerActiveTurn.supports(S::Working, E::PreserveRunningTurn), Some(CaseSupport::NonInterrupting));
        assert_eq!(O::StartIdleTurn.supports(S::Idle, E::ResumeSameConversation), Some(CaseSupport::NonInterrupting));
        assert_eq!(O::StartIdleTurn.supports(S::Working, E::PreserveRunningTurn), None);
        assert_eq!(
            O::InterruptThenSubmit.supports(S::Working, E::CancelTurnSameConversation),
            Some(CaseSupport::InterruptionRequired)
        );
        assert_eq!(O::InterruptThenSubmit.supports(S::Idle, E::CancelTurnSameConversation), None);
        assert_eq!(O::SteerActiveTurn.supports(S::Unknown, E::PreserveRunningTurn), None);
        assert_eq!(O::Unknown.supports(S::Working, E::Unknown), None);
        assert_eq!(O::Unknown.required_assertions(), None);
    }

    #[test]
    fn receipt_strength_never_upgrades_from_persistence_alone() {
        let mut receipts = ReceiptGuarantees {
            request_acceptance: GuaranteeLevel::Unknown,
            persistence: GuaranteeLevel::Confirmed,
            scheduling: GuaranteeLevel::Unknown,
            conversation_delivery: GuaranteeLevel::Unknown,
        };
        assert_eq!(receipts.max_strength(), ReceiptStrength::Unknown);
        receipts.request_acceptance = GuaranteeLevel::Confirmed;
        assert_eq!(receipts.max_strength(), ReceiptStrength::Accepted);
        receipts.scheduling = GuaranteeLevel::Confirmed;
        assert_eq!(receipts.max_strength(), ReceiptStrength::Queued);
        receipts.conversation_delivery = GuaranteeLevel::Confirmed;
        assert_eq!(receipts.max_strength(), ReceiptStrength::Delivered);
        assert!(ReceiptStrength::Accepted < ReceiptStrength::Delivered);
    }

    #[test]
    fn wire_forms_are_snake_case() {
        assert_eq!(OperationIntent::VARIANTS[0], "steer_active_turn");
        assert_eq!("non_interactive".parse::<LaunchMode>().unwrap(), LaunchMode::NonInteractive);
        assert_eq!(serde_json::to_string(&AssertionKind::ExpectedLoss).unwrap(), "\"expected_loss\"");
        assert!("interactive_ish".parse::<LaunchMode>().is_err());
    }
}
