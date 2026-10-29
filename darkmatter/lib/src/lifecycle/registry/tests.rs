use std::sync::OnceLock;

use super::*;

fn unavailable(code: &str) -> Availability {
    Availability::Unavailable(UnavailabilityReason::new(code))
}

/// A renaming host shaped like an agent runner, for the lifecycle unit tests
/// whose documents use its spellings: `composed` is `start`, `failed` is
/// `blocked`, `success`/`failure` are the outcome pair, `loop` is the gate.
pub(crate) fn agent_host() -> &'static EventRegistry {
    static REGISTRY: OnceLock<EventRegistry> = OnceLock::new();
    REGISTRY.get_or_init(build_agent_host)
}

fn build_agent_host() -> EventRegistry {
    let mut registry = EventRegistry::new("agent");
    registry.rename(CoreEvent::Composed, "start").unwrap();
    registry.rename(CoreEvent::Failed, "blocked").unwrap();
    for (name, position, role) in [
        ("success", Position::AfterComposition, Role::TerminalOutcome),
        ("failure", Position::AfterComposition, Role::TerminalOutcome),
        ("loop", Position::AfterFinalize, Role::IterationGate),
    ] {
        let view = generic_view(&format!("agent.{name}"), Availability::Available);
        registry.register(EventDescriptor::new(name, position, role, view)).unwrap();
    }
    registry
        .register_outcome_pair(
            OutcomePair::new("success", "failure").downgrade("success", "failure").downgrade("blocked", "failure"),
        )
        .unwrap();
    registry.declare_host_global("loop").unwrap();
    registry.declare_host_global("group").unwrap();
    registry
}

fn note(name: &str, position: Position) -> EventDescriptor {
    EventDescriptor::new(name, position, Role::Notification, generic_view("test.note", Availability::Available))
}

fn outcome(name: &str) -> EventDescriptor {
    EventDescriptor::new(
        name,
        Position::AfterComposition,
        Role::TerminalOutcome,
        generic_view("test.outcome", Availability::Available),
    )
}

fn paired() -> EventRegistry {
    let mut registry = EventRegistry::new("test");
    registry.register(outcome("shipped")).unwrap();
    registry.register(outcome("dropped")).unwrap();
    registry.register_outcome_pair(OutcomePair::new("shipped", "dropped")).unwrap();
    registry
}

#[test]
fn every_registration_error_is_typed() {
    let mut registry = EventRegistry::new("test");
    registry.register(note("announce", Position::BeforeComposition)).unwrap();
    assert_eq!(
        registry.register(note("announce", Position::AfterFinalize)),
        Err(RegistrationError::DuplicateName { name: "announce".to_string() })
    );
    assert_eq!(
        registry.register(note("finalize", Position::AfterFinalize)),
        Err(RegistrationError::DarkmatterEventName { name: "finalize".to_string(), event: CoreEvent::Finalize })
    );
    assert_eq!(
        registry.register(note("env", Position::AfterFinalize)),
        Err(RegistrationError::ReservedRoot { name: "env".to_string() })
    );
    assert_eq!(registry.rename(CoreEvent::Finalize, "end"), Err(RegistrationError::FixedEvent { event: CoreEvent::Finalize }));
    registry.rename(CoreEvent::Composed, "ready").unwrap();
    assert_eq!(
        registry.rename(CoreEvent::Composed, "go"),
        Err(RegistrationError::SecondName {
            event: CoreEvent::Composed,
            existing: "ready".to_string(),
            attempted: "go".to_string()
        })
    );
    let gate = |name: &str, position| {
        EventDescriptor::new(name, position, Role::IterationGate, generic_view("test.gate", Availability::Available))
    };
    assert_eq!(
        registry.register(gate("again", Position::BeforeComposition)),
        Err(RegistrationError::RoleOutOfPosition {
            name: "again".to_string(),
            role: Role::IterationGate,
            position: Position::BeforeComposition
        })
    );
    registry.register(gate("again", Position::AfterFinalize)).unwrap();
    assert_eq!(
        registry.register(gate("more", Position::AfterFinalize)),
        Err(RegistrationError::SecondIterationGate { name: "more".to_string(), existing: "again".to_string() })
    );
    registry.register(outcome("shipped")).unwrap();
    assert_eq!(registry.validate(), Err(RegistrationError::UnpairedOutcome { name: "shipped".to_string() }));
    assert_eq!(
        registry.register_outcome_pair(OutcomePair::new("shipped", "shipped")),
        Err(RegistrationError::DuplicateAlternative { name: "shipped".to_string() })
    );
    assert_eq!(
        registry.register_outcome_pair(OutcomePair::new("shipped", "announce")),
        Err(RegistrationError::MissingAlternative { name: "announce".to_string() })
    );
    registry.register(outcome("dropped")).unwrap();
    assert_eq!(
        registry.register_outcome_pair(OutcomePair::new("shipped", "dropped").downgrade("ready", "dropped")),
        Err(RegistrationError::DowngradeSourceNotOutcome { from: "ready".to_string() })
    );
    assert_eq!(
        registry.register_outcome_pair(OutcomePair::new("shipped", "dropped").downgrade("shipped", "finalize")),
        Err(RegistrationError::DowngradeTargetNotAlternative { from: "shipped".to_string(), to: "finalize".to_string() })
    );
    registry.register_outcome_pair(OutcomePair::new("shipped", "dropped")).unwrap();
    assert_eq!(registry.validate(), Ok(()));
    assert_eq!(registry.register_outcome_pair(OutcomePair::new("shipped", "dropped")), Err(RegistrationError::SecondOutcomePair));
    assert_eq!(registry.declare_host_global("err"), Err(RegistrationError::DuplicateGlobal { name: "err".to_string() }));
    assert_eq!(registry.declare_host_global("ctx"), Err(RegistrationError::ReservedRoot { name: "ctx".to_string() }));

    let held = registry.clone();
    assert_eq!(registry.declare_host_global("late"), Err(RegistrationError::RunStarted));
    assert_eq!(
        registry.declare_core_globals(CoreEvent::Initialize, generic_view("test.init", Availability::Available)),
        Err(RegistrationError::RunStarted)
    );
    drop(held);
    registry.declare_host_global("late").unwrap();
}

#[test]
fn an_unknown_position_or_role_spelling_is_a_typed_error() {
    for position in [Position::BeforeComposition, Position::AfterComposition, Position::AfterFinalize] {
        assert_eq!(position.as_str().parse::<Position>(), Ok(position));
    }
    for role in [Role::Notification, Role::TerminalOutcome, Role::IterationGate] {
        assert_eq!(role.as_str().parse::<Role>(), Ok(role));
    }
    assert_eq!(
        "after-start".parse::<Position>(),
        Err(DescriptorParseError::UnknownPosition { value: "after-start".to_string() })
    );
    assert_eq!("terminal".parse::<Role>(), Err(DescriptorParseError::UnknownRole { value: "terminal".to_string() }));
}

#[test]
fn only_the_engine_fires_an_event_and_only_at_its_registered_point() {
    let mut registry = paired();
    let announce = registry.register(note("announce", Position::AfterComposition)).unwrap();
    let gate = registry
        .register(EventDescriptor::new(
            "again",
            Position::AfterFinalize,
            Role::IterationGate,
            generic_view("test.gate", Availability::Available),
        ))
        .unwrap();
    let shipped = registry.id("shipped").unwrap();
    let failed = EventId::core(CoreEvent::Failed);

    assert_eq!(registry.check_emission(shipped, Emitter::Engine, RunPoint::Terminal), Ok(()));
    assert_eq!(registry.check_emission(failed, Emitter::Engine, RunPoint::Terminal), Ok(()));
    assert_eq!(registry.check_emission(announce, Emitter::Engine, RunPoint::AfterComposition), Ok(()));
    assert_eq!(registry.check_emission(gate, Emitter::Engine, RunPoint::IterationGate), Ok(()));
    assert_eq!(
        registry.check_emission(shipped, Emitter::Handler(announce), RunPoint::Terminal),
        Err(EmissionError::HandlerEmission { handler: "announce".to_string(), event: "shipped".to_string() })
    );
    assert_eq!(
        registry.check_emission(announce, Emitter::Engine, RunPoint::Terminal),
        Err(EmissionError::RoleNotRegistered { event: "announce".to_string(), role: Role::TerminalOutcome })
    );
    assert_eq!(
        registry.check_emission(announce, Emitter::Engine, RunPoint::IterationGate),
        Err(EmissionError::RoleNotRegistered { event: "announce".to_string(), role: Role::IterationGate })
    );
    assert_eq!(
        registry.check_emission(shipped, Emitter::Engine, RunPoint::AfterFinalize),
        Err(EmissionError::OutOfPosition {
            event: "shipped".to_string(),
            attempted: RunPoint::AfterFinalize,
            registered: RunPoint::Terminal
        })
    );
    assert_eq!(
        registry.check_emission(EventId::core(CoreEvent::Initialize), Emitter::Engine, RunPoint::Composed),
        Err(EmissionError::OutOfPosition {
            event: "initialize".to_string(),
            attempted: RunPoint::Composed,
            registered: RunPoint::Initialize
        })
    );
    let foreign = gate;
    assert_eq!(EventRegistry::md().check_emission(foreign, Emitter::Engine, RunPoint::IterationGate), Err(EmissionError::ForeignEvent));
}

#[test]
fn md_registers_only_the_core_events_under_their_own_names() {
    let md = EventRegistry::md();
    assert!(!md.has_outcome_pair());
    assert_eq!(md.validate(), Ok(()));
    assert_eq!(md.authored_event_keys(), ["initialize", "composed", "failed", "finalize", "cancelled"]);
    for signal in [LifecycleSignal::Success, LifecycleSignal::Failure, LifecycleSignal::Loop] {
        assert_eq!(md.id_of(signal), None);
    }
    for key in ["start", "blocked", "success", "failure", "loop"] {
        assert_eq!(md.resolve_key(key), KeyResolution::Data);
    }
    assert_eq!(md.host_globals(), ["err", "timing"]);
    assert_eq!(md.err_availability(EventId::core(CoreEvent::Failed)), Some(&Availability::Available));
    assert_eq!(md.err_availability(EventId::core(CoreEvent::Initialize)), Some(&unavailable("lifecycle.no-error-before-composition")));
}

#[test]
fn a_renaming_host_maps_every_event_to_its_engine_slot_in_a_fixed_key_order() {
    let registry = agent_host();
    assert_eq!(
        registry.authored_event_keys(),
        ["initialize", "start", "success", "blocked", "failure", "finalize", "loop", "cancelled"]
    );
    for signal in LifecycleSignal::ALL {
        let id = registry.id_of(signal).expect("every slot has an event");
        assert_eq!(registry.signal(id), Some(signal));
        assert_eq!(registry.name_of(signal), Some(signal.property_name()));
    }
    assert_eq!(
        registry.resolve_key("failed"),
        KeyResolution::Renamed { authored: "failed".to_string(), replacement: "blocked".to_string() }
    );
    assert_eq!(registry.host_globals(), ["err", "timing", "loop", "group"]);

    // A notification has no slot and follows the slotted keys.
    let mut registry = build_agent_host();
    let audit = registry.register(note("audit", Position::AfterFinalize)).unwrap();
    assert_eq!(registry.signal(audit), None);
    assert_eq!(registry.authored_event_keys().last(), Some(&"audit"));
    assert_eq!(registry.notifications(Position::AfterFinalize).collect::<Vec<_>>(), [audit]);
}

#[test]
fn every_generic_late_global_is_declared_by_the_global_catalog() {
    let catalog = crate::markdown::schemas::catalog::GlobalCatalog::darkmatter();
    for name in GENERIC_GLOBALS {
        assert!(catalog.global(name).is_some(), "`{name}` is a generic late global the catalog does not declare");
    }
}
