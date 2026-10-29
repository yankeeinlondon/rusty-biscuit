//! Claudine's registration with Darkmatter's event registry.
//!
//! `composed` is `start` and `failed` is `blocked`; the agent loop ends in the
//! `success`/`failure` outcome pair, where an explicit error in `success` or
//! `blocked` downgrades to `failure`; and `loop` is the post-`finalize`
//! iteration gate. `err` is unavailable in `initialize`, `start`, `success`,
//! and `loop`, and `timing` is available everywhere; the binding catalog
//! ([`super::bindings`]) reads both from here. Every event admits every action
//! type except `initialize`, which is shell-free. `loop` and `group` are
//! Claudine's late globals beside Darkmatter's `err` and `timing`, named from
//! Claudine's host catalog ([`crate::composition::schema::globals`]).

use std::sync::{Arc, OnceLock};

use darkmatter::lifecycle::registry::{
    ActionType, ActionTypes, CoreEvent, EventDescriptor, EventRegistry, OutcomePair, Position, Role, generic_view,
};
use darkmatter::markdown::compose::expression::{Availability, BindingView, UnavailabilityReason};

use super::LifecycleSignal;
use super::bindings::{EVENT_HAS_NO_ERROR, LifecycleScope};
use crate::composition::reserved::LOOP_ROOT;
use crate::composition::schema::globals::host_global;

/// Claudine's event registry, built once per process.
///
/// ## Panics
///
/// Never in practice: registration is fixed code, and a test builds it.
pub fn claudine_registry() -> &'static Arc<EventRegistry> {
    static REGISTRY: OnceLock<Arc<EventRegistry>> = OnceLock::new();
    REGISTRY.get_or_init(|| Arc::new(build()))
}

fn err(signal: LifecycleSignal) -> Availability {
    if signal.can_carry_error() {
        Availability::Available
    } else {
        Availability::Unavailable(UnavailabilityReason::new(EVENT_HAS_NO_ERROR))
    }
}

fn view(signal: LifecycleSignal) -> Arc<BindingView> {
    generic_view(LifecycleScope::Event(signal).id(), err(signal))
}

fn build() -> EventRegistry {
    let mut registry = EventRegistry::new("claudine");
    registry.rename(CoreEvent::Composed, "start").expect("`start` is free");
    registry.rename(CoreEvent::Failed, "blocked").expect("`blocked` is free");
    for event in CoreEvent::ALL {
        registry
            .declare_core_globals(event, view(event.signal()))
            .expect("the registry is not shared yet");
    }
    registry
        .declare_core_actions(
            CoreEvent::Initialize,
            ActionTypes::of([ActionType::Expression, ActionType::Flow, ActionType::Communicate]),
        )
        .expect("the registry is not shared yet");
    for (signal, position, role) in [
        (LifecycleSignal::Success, Position::AfterComposition, Role::TerminalOutcome),
        (LifecycleSignal::Failure, Position::AfterComposition, Role::TerminalOutcome),
        (LifecycleSignal::Loop, Position::AfterFinalize, Role::IterationGate),
    ] {
        registry
            .register(EventDescriptor::new(signal.property_name(), position, role, view(signal)))
            .expect("Claudine's events register");
    }
    registry
        .register_outcome_pair(
            OutcomePair::new("success", "failure").downgrade("success", "failure").downgrade("blocked", "failure"),
        )
        .expect("Claudine's outcome pair registers");
    for global in [host_global(LOOP_ROOT).name(), host_global("group").name()] {
        registry.declare_host_global(global).expect("Claudine's globals are unreserved and distinct");
    }
    registry
        .register_global_catalog(crate::composition::schema::globals::claudine_globals())
        .expect("the registry is not shared yet");
    registry.validate().expect("every Claudine terminal outcome is paired");
    registry
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::composition::lifecycle::bindings::{GroupMembership, binding_view};

    /// The keys Claudine deferred and parsed as lifecycle stacks before the
    /// registry derived them.
    const PREVIOUS_EVENT_KEYS: [&str; 8] =
        ["initialize", "start", "success", "blocked", "failure", "finalize", "loop", "cancelled"];
    /// The late roots a lifecycle placeholder could read before the registry
    /// declared them.
    const PREVIOUS_HOST_GLOBALS: [&str; 4] = ["loop", "err", "timing", "group"];

    #[test]
    fn claudine_registry_builds_and_derives_the_previous_lifecycle_keys_and_host_globals() {
        let registry = claudine_registry();
        assert_eq!(registry.authored_event_keys(), PREVIOUS_EVENT_KEYS);
        assert_eq!(
            registry.host_globals().iter().map(AsRef::as_ref).collect::<BTreeSet<&str>>(),
            BTreeSet::from(PREVIOUS_HOST_GLOBALS)
        );
        assert_eq!(registry.host_globals().len(), PREVIOUS_HOST_GLOBALS.len());
    }

    #[test]
    fn claudine_registry_maps_every_slot_to_its_claudine_name() {
        let registry = claudine_registry();
        for signal in LifecycleSignal::ALL {
            let id = registry.id_of(signal).expect("Claudine has an event for every slot");
            assert_eq!(registry.name(id), signal.property_name());
            assert_eq!(registry.signal(id), Some(signal));
        }
    }

    #[test]
    fn claudine_registry_err_availability_agrees_with_the_binding_catalog() {
        let registry = claudine_registry();
        for signal in LifecycleSignal::ALL {
            let id = registry.id_of(signal).expect("Claudine has an event for every slot");
            let registered = registry.err_availability(id).expect("every event declares `err`");
            assert_eq!(matches!(registered, Availability::Available), signal.can_carry_error(), "{signal:?}");
            for group in [GroupMembership::Member, GroupMembership::Outside, GroupMembership::Unknown] {
                let catalog = binding_view(LifecycleScope::Event(signal), group);
                assert_eq!(catalog.global("err").map(|global| global.availability()), Some(registered), "{signal:?}");
            }
        }
    }

    #[test]
    fn initialize_alone_excludes_shell_actions() {
        let registry = claudine_registry();
        for signal in LifecycleSignal::ALL {
            let id = registry.id_of(signal).expect("Claudine has an event for every slot");
            let actions = registry.actions(id);
            assert_eq!(actions.contains(ActionType::Shell), signal != LifecycleSignal::Initialize, "{signal:?}");
            for admitted in [ActionType::Expression, ActionType::Flow, ActionType::Communicate] {
                assert!(actions.contains(admitted), "{signal:?} {admitted:?}");
            }
        }
    }
}
