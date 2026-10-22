//! Host registrations on the production event registry for the prototype
//! run. The run knows none of these names; each event's behavior comes from
//! its position and role. Claudine's production registration is Claudine's
//! own; this copy keeps the prototype's views, so its tests read the same
//! reasons.

use darkmatter::markdown::compose::expression::{Availability, UnavailabilityReason};

use super::engine::{CoreEvent, EventDescriptor, OutcomePair, Position, Registry, Role, err_view};

fn unavailable(reason: &str) -> Availability {
    Availability::Unavailable(UnavailabilityReason::new(reason))
}

/// Darkmatter's five events and nothing else: no outcome pair, so the
/// terminal slot can only ever hold `failed`.
pub fn md_profile() -> Registry {
    Registry::md()
}

/// Claudine as a host: `composed` is `start`, `failed` is `blocked`, the
/// agent loop ends in `success` or `failure` (both may downgrade to
/// `failure` on an explicit error), and `loop` gates the next iteration.
pub fn claudine_profile() -> Registry {
    let mut registry = Registry::new("claudine");
    registry.rename(CoreEvent::Composed, "start").expect("start is free");
    registry.rename(CoreEvent::Failed, "blocked").expect("blocked is free");
    for descriptor in [
        EventDescriptor::new(
            "success",
            Position::AfterComposition,
            Role::TerminalOutcome,
            err_view("claudine.success", unavailable("claudine.success-has-no-error")),
        ),
        EventDescriptor::new("failure", Position::AfterComposition, Role::TerminalOutcome, err_view("claudine.failure", Availability::Available)),
        EventDescriptor::new(
            "loop",
            Position::AfterFinalize,
            Role::IterationGate,
            err_view("claudine.loop", unavailable("claudine.loop-has-no-error")),
        ),
    ] {
        registry.register(descriptor).expect("Claudine's events register");
    }
    registry
        .register_outcome_pair(OutcomePair::new("success", "failure").downgrade("success", "failure").downgrade("blocked", "failure"))
        .expect("Claudine's outcome pair registers");
    registry
}
