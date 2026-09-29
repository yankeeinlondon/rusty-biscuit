//! Deterministic steering eligibility for one concrete session.
//!
//! Eligibility is derived, never stored: a route exists only when the
//! researched case supports it, the mechanism's operation fits the session
//! state, its acknowledgment arrives before completion, access is available,
//! a reviewed grant matches the exact provider version and case, and the
//! grant's adapter revision is implemented. Every failed gate becomes a
//! [`Blocker`] with an actionable reason.
//!
//! Automatic help additionally requires a non-interrupting route that can
//! reach a turn which never ends ([`SteeringMechanism::rescues_active_loop`]);
//! a next-turn follow-up, an idle start, or an interruption never qualifies.

use std::fmt;

use super::vocabulary::{
    AccessStatus, ActivationGrant, AdapterRef, CaseSupport, ExecutionState, HostOs, LaunchMode,
    LaunchOrigin, ProviderSteering, ReceiptStrength, ReceiptTiming, SteeringAvailability,
    SteeringMechanism,
};
use crate::provider_id::Provider;

/// The observed facts about one session that eligibility depends on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SessionFacts<'a> {
    pub provider: Provider,
    /// Researched launch-profile ID this session was launched under.
    pub profile_id: &'a str,
    pub os: HostOs,
    pub launch_mode: LaunchMode,
    pub origin: LaunchOrigin,
    pub state: ExecutionState,
    /// Exact provider version, when it could be established.
    pub provider_version: Option<&'a str>,
}

/// One specific reason a mechanism or session cannot be steered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Blocker {
    StateUnknown,
    VersionUnknown,
    NoResearchCase,
    CaseNotSupported { support: CaseSupport, reason: &'static str },
    /// The mechanism's operation cannot serve the session's state.
    OperationMismatch { mechanism: &'static str },
    /// Acknowledgment arrives too late (or never) to return after acceptance.
    NoEarlyAcceptance { mechanism: &'static str, timing: ReceiptTiming },
    AccessNotAvailable { mechanism: &'static str, status: AccessStatus, prerequisite: &'static str },
    /// No reviewed grant covers this exact version and case. `setup` is the
    /// researched prerequisite when access requires separate setup.
    NotActivated { mechanism: &'static str, setup: Option<&'static str> },
    AdapterNotImplemented { mechanism: &'static str, adapter: AdapterRef },
}

impl fmt::Display for Blocker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StateUnknown => f.write_str("session state could not be established"),
            Self::VersionUnknown => f.write_str("provider version could not be established"),
            Self::NoResearchCase => f.write_str("no research covers this launch profile, OS, mode, origin, and state"),
            Self::CaseNotSupported { support, reason } => write!(f, "research marks this case {support}: {reason}"),
            Self::OperationMismatch { mechanism } => {
                write!(f, "`{mechanism}` cannot deliver to a session in this state")
            }
            Self::NoEarlyAcceptance { mechanism, timing } => write!(
                f,
                "`{mechanism}` acknowledgment timing is {timing}, so acceptance cannot be confirmed before the turn ends"
            ),
            Self::AccessNotAvailable { mechanism, status, prerequisite } => {
                write!(f, "`{mechanism}` access is {status}: {prerequisite}")
            }
            Self::NotActivated { mechanism, setup } => {
                write!(
                    f,
                    "`{mechanism}` has no reviewed live verification for this provider version and launch profile"
                )?;
                match setup {
                    Some(setup) => write!(f, "; it also requires setup: {setup}"),
                    None => Ok(()),
                }
            }
            Self::AdapterNotImplemented { mechanism, adapter } => write!(
                f,
                "`{mechanism}` needs adapter `{}` revision {}, which this build does not implement",
                adapter.id, adapter.revision
            ),
        }
    }
}

/// A usable route: a mechanism reachable through an implemented adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route {
    pub mechanism: &'static SteeringMechanism,
    pub adapter: AdapterRef,
    pub support: CaseSupport,
}

impl Route {
    /// The strongest receipt this route can ever report.
    pub fn max_receipt(&self) -> ReceiptStrength {
        self.mechanism.receipts.max_strength()
    }
}

/// Manual (`claudine steer`) eligibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualEligibility {
    pub availability: SteeringAvailability,
    /// Preferred route: non-interrupting before interruption-required.
    pub route: Option<Route>,
    /// Reasons each unusable mechanism (or the session) was blocked.
    pub blockers: Vec<Blocker>,
}

/// Why automatic help cannot reach this session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomaticBlocker {
    /// Automatic help only targets a working session.
    NotWorking(ExecutionState),
    /// No non-interrupting route exists; the manual blockers explain why.
    NoNonInterruptingRoute,
    /// Non-interrupting routes exist, but none reaches a never-ending turn.
    CannotRescueActiveTurn { mechanisms: Vec<&'static str> },
}

impl fmt::Display for AutomaticBlocker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotWorking(state) => write!(f, "automatic help needs a working session; state is {state}"),
            Self::NoNonInterruptingRoute => f.write_str("no verified non-interrupting delivery is available"),
            Self::CannotRescueActiveTurn { mechanisms } => write!(
                f,
                "{} deliver only after the current turn, which cannot help a turn that never ends",
                mechanisms.iter().map(|m| format!("`{m}`")).collect::<Vec<_>>().join(", ")
            ),
        }
    }
}

/// Automatic (repetition-warning) eligibility.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AutomaticEligibility {
    Eligible(Route),
    Unavailable(AutomaticBlocker),
}

/// Manual and automatic eligibility for one session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eligibility {
    pub manual: ManualEligibility,
    pub automatic: AutomaticEligibility,
}

/// Evaluates `session` against the generated facts, reviewed grants, and
/// implemented adapters of this build.
pub fn evaluate(session: &SessionFacts<'_>) -> Eligibility {
    evaluate_with(
        session,
        super::facts(session.provider),
        super::activation_grants(),
        &|adapter| super::adapters::is_usable(adapter),
    )
}

/// [`evaluate`] over explicit inputs.
pub fn evaluate_with(
    session: &SessionFacts<'_>,
    facts: &'static ProviderSteering,
    grants: &[ActivationGrant],
    adapter_usable: &dyn Fn(AdapterRef) -> bool,
) -> Eligibility {
    let (routes, blockers) = routes(session, facts, grants, adapter_usable);
    let route = routes
        .iter()
        .find(|route| route.support == CaseSupport::NonInterrupting)
        .or_else(|| routes.first())
        .copied();
    let availability = match route.map(|route| route.support) {
        Some(CaseSupport::NonInterrupting) => SteeringAvailability::NonInterrupting,
        Some(CaseSupport::InterruptionRequired) => SteeringAvailability::InterruptionRequired,
        _ => SteeringAvailability::Unavailable,
    };
    let automatic = automatic(session.state, &routes);
    Eligibility { manual: ManualEligibility { availability, route, blockers }, automatic }
}

fn automatic(state: ExecutionState, routes: &[Route]) -> AutomaticEligibility {
    if state != ExecutionState::Working {
        return AutomaticEligibility::Unavailable(AutomaticBlocker::NotWorking(state));
    }
    let non_interrupting: Vec<&Route> =
        routes.iter().filter(|route| route.support == CaseSupport::NonInterrupting).collect();
    if non_interrupting.is_empty() {
        return AutomaticEligibility::Unavailable(AutomaticBlocker::NoNonInterruptingRoute);
    }
    match non_interrupting.iter().find(|route| route.mechanism.rescues_active_loop()) {
        Some(route) => AutomaticEligibility::Eligible(**route),
        None => AutomaticEligibility::Unavailable(AutomaticBlocker::CannotRescueActiveTurn {
            mechanisms: non_interrupting.iter().map(|route| route.mechanism.id).collect(),
        }),
    }
}

fn routes(
    session: &SessionFacts<'_>,
    facts: &'static ProviderSteering,
    grants: &[ActivationGrant],
    adapter_usable: &dyn Fn(AdapterRef) -> bool,
) -> (Vec<Route>, Vec<Blocker>) {
    if session.state == ExecutionState::Unknown {
        return (Vec::new(), vec![Blocker::StateUnknown]);
    }
    let Some(version) = session.provider_version else {
        return (Vec::new(), vec![Blocker::VersionUnknown]);
    };
    let Some(case) = facts.cases.iter().find(|case| {
        case.profile_id == session.profile_id
            && case.os == session.os
            && case.launch_mode == session.launch_mode
            && case.origin == session.origin
            && case.state == session.state
    }) else {
        return (Vec::new(), vec![Blocker::NoResearchCase]);
    };
    if !matches!(case.support, CaseSupport::NonInterrupting | CaseSupport::InterruptionRequired) {
        return (Vec::new(), vec![Blocker::CaseNotSupported { support: case.support, reason: case.reason }]);
    }

    let mut routes = Vec::new();
    let mut blockers = Vec::new();
    for mechanism in case
        .mechanism_ids
        .iter()
        .filter_map(|id| facts.mechanisms.iter().find(|mechanism| mechanism.id == *id))
    {
        match route(session, version, facts, grants, adapter_usable, mechanism) {
            Ok(route) => routes.push(route),
            Err(blocker) => blockers.push(blocker),
        }
    }
    if routes.is_empty() && blockers.is_empty() {
        blockers.push(Blocker::CaseNotSupported { support: case.support, reason: case.reason });
    }
    (routes, blockers)
}

fn route(
    session: &SessionFacts<'_>,
    version: &str,
    facts: &'static ProviderSteering,
    grants: &[ActivationGrant],
    adapter_usable: &dyn Fn(AdapterRef) -> bool,
    mechanism: &'static SteeringMechanism,
) -> Result<Route, Blocker> {
    let id = mechanism.id;
    let support = mechanism
        .operation_intent
        .supports(session.state, mechanism.conversation_effect)
        .ok_or(Blocker::OperationMismatch { mechanism: id })?;
    if !mechanism.receipt_timing.supports_prompt_return() {
        return Err(Blocker::NoEarlyAcceptance { mechanism: id, timing: mechanism.receipt_timing });
    }
    let access = facts
        .access
        .iter()
        .find(|access| access.mechanism_id == id && access.profile_id == session.profile_id && access.os == session.os);
    // Setup-required access describes how the launch profile must be started.
    // The session matched that profile; a reviewed exact grant is what attests
    // the profile satisfies the prerequisite, so without one it stays a reason.
    let setup = match access {
        Some(access) if access.status == AccessStatus::Available => None,
        Some(access) if access.status == AccessStatus::SetupRequired => Some(access.prerequisite),
        Some(access) => {
            return Err(Blocker::AccessNotAvailable { mechanism: id, status: access.status, prerequisite: access.prerequisite });
        }
        None => {
            return Err(Blocker::AccessNotAvailable {
                mechanism: id,
                status: AccessStatus::Unknown,
                prerequisite: "no researched access finding",
            });
        }
    };
    let grant = grants
        .iter()
        .find(|grant| {
            grant.provider == session.provider.as_slug()
                && grant.mechanism_id == id
                && grant.operation == mechanism.operation_intent
                && grant.profile_id == session.profile_id
                && grant.os == session.os
                && grant.provider_version == version
                && grant.launch_mode == session.launch_mode
                && grant.origin == session.origin
                && grant.state == session.state
        })
        .ok_or(Blocker::NotActivated { mechanism: id, setup })?;
    if !adapter_usable(grant.adapter) {
        return Err(Blocker::AdapterNotImplemented { mechanism: id, adapter: grant.adapter });
    }
    Ok(Route { mechanism, adapter: grant.adapter, support })
}

#[cfg(test)]
mod tests;
