//! The prototype's registry-phase tests: registration, renames, the core-only
//! `md` profile, per-event `err` views, and registered action types. The run
//! protocol itself is tested against production in `lifecycle_run_protocol`.

use darkmatter::markdown::compose::expression::Availability;
use serde_json::json;

use super::engine::{
    ActionTypes, ApprovalEpoch, CoreEvent, Directive, DocumentInput, Entry, EventDescriptor,
    EventId, EventOutcome, FailureStage, Finish, HostError, HostFailure, KeyResolution, LifecycleError, OperationReport,
    Outcome, OutcomePair, Position, Preparation, RecoveryReply, RegistrationError, Reply, ReplyError,
    Role, Run, SessionAvailability, ShellRefusal, StartError, err_view,
};
use super::profiles::{claudine_profile, md_profile};

fn ok() -> EventOutcome {
    EventOutcome::default()
}

fn host(code: &str) -> HostError {
    HostError::new(code, code)
}

fn raised(code: &str) -> LifecycleError {
    LifecycleError::Host(host(code))
}

fn failure(stage: FailureStage, entry: Entry, code: &str) -> HostFailure {
    HostFailure { stage, entry, error: host(code) }
}

fn answer(run: &mut Run, reply: Reply) {
    run.answer(reply).unwrap_or_else(|error| panic!("{error:?} after {:?}", run.transcript_lines()));
}

fn event(run: &mut Run, name: &str, outcome: EventOutcome) {
    assert_eq!(run.pending().event_name(), Some(name), "after {:?}", run.transcript_lines());
    answer(run, Reply::Event(outcome));
}

fn ready(run: &mut Run) {
    answer(run, Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(1) }));
}

fn complete(run: &mut Run, outcome: Outcome) {
    answer(run, Reply::Operation(OperationReport::Completed { outcome, session: SessionAvailability::Available }));
}

fn claudine(document: DocumentInput) -> Run {
    Run::start(&claudine_profile(), document).expect("Claudine documents start")
}

/// A Claudine run past `start`, waiting for its operation.
fn started() -> Run {
    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok());
    ready(&mut run);
    event(&mut run, "start", ok());
    run
}

/// A Claudine run whose operation failed, waiting on `failure`.
fn failed_run() -> Run {
    let mut run = started();
    complete(&mut run, Outcome::Failed(host("agent.failed")));
    run
}

fn events(run: &Run) -> Vec<String> {
    run.transcript_lines().into_iter().filter_map(|line| line.strip_prefix("event ").map(str::to_string)).collect()
}

fn note(name: &str, position: Position) -> EventDescriptor {
    EventDescriptor::new(name, position, Role::Notification, err_view("test.note", Availability::Available))
}

fn outcome_event(name: &str) -> EventDescriptor {
    EventDescriptor::new(name, Position::AfterComposition, Role::TerminalOutcome, err_view("test.outcome", Availability::Available))
}

#[test]
fn registration_rejects_duplicates_darkmatter_names_and_reserved_roots() {
    let mut registry = md_profile();
    registry.register(note("announce", Position::BeforeComposition)).unwrap();
    assert_eq!(
        registry.register(note("announce", Position::AfterComposition)),
        Err(RegistrationError::DuplicateName { name: "announce".to_string() })
    );
    for event in CoreEvent::ALL {
        let name = event.darkmatter_name();
        assert_eq!(
            registry.register(note(name, Position::BeforeComposition)),
            Err(RegistrationError::DarkmatterEventName { name: name.to_string(), event })
        );
    }
    for root in ["fm", "ctx", "env", "current"] {
        let reserved = RegistrationError::ReservedRoot { name: root.to_string() };
        assert_eq!(registry.register(note(root, Position::BeforeComposition)), Err(reserved.clone()));
        assert_eq!(registry.rename(CoreEvent::Composed, root), Err(reserved));
    }
    // A host spelling is taken once registered; the Darkmatter spelling of a
    // renamed event stays reserved.
    registry.rename(CoreEvent::Composed, "begin").unwrap();
    assert_eq!(registry.register(note("begin", Position::BeforeComposition)), Err(RegistrationError::DuplicateName { name: "begin".to_string() }));
    assert_eq!(
        registry.register(note("composed", Position::BeforeComposition)),
        Err(RegistrationError::DarkmatterEventName { name: "composed".to_string(), event: CoreEvent::Composed })
    );
    assert_eq!(registry.rename(CoreEvent::Failed, "announce"), Err(RegistrationError::DuplicateName { name: "announce".to_string() }));
}

#[test]
fn guaranteed_events_keep_their_names_and_an_event_takes_one_name() {
    let mut registry = md_profile();
    for event in [CoreEvent::Initialize, CoreEvent::Cancelled, CoreEvent::Finalize] {
        assert_eq!(registry.rename(event, "renamed"), Err(RegistrationError::FixedEvent { event }));
    }
    registry.rename(CoreEvent::Failed, "blocked").unwrap();
    assert_eq!(
        registry.rename(CoreEvent::Failed, "refused"),
        Err(RegistrationError::SecondName { event: CoreEvent::Failed, existing: "blocked".to_string(), attempted: "refused".to_string() })
    );
    assert_eq!(registry.resolve_key("blocked"), KeyResolution::Event(EventId::core(CoreEvent::Failed)));
    assert_eq!(
        registry.resolve_key("failed"),
        KeyResolution::Renamed { authored: "failed".to_string(), replacement: "blocked".to_string() }
    );
    assert_eq!(registry.resolve_key("refused"), KeyResolution::Data);
}

#[test]
fn a_role_registers_only_in_its_position() {
    let mut registry = md_profile();
    let misplaced = EventDescriptor::new("done", Position::BeforeComposition, Role::TerminalOutcome, err_view("t.done", Availability::Available));
    assert_eq!(
        registry.register(misplaced),
        Err(RegistrationError::RoleOutOfPosition {
            name: "done".to_string(),
            role: Role::TerminalOutcome,
            position: Position::BeforeComposition
        })
    );
    let gate = |name: &str, position| EventDescriptor::new(name, position, Role::IterationGate, err_view("t.gate", Availability::Available));
    assert!(matches!(registry.register(gate("again", Position::AfterComposition)), Err(RegistrationError::RoleOutOfPosition { .. })));
    registry.register(gate("again", Position::AfterFinalize)).unwrap();
    assert_eq!(
        registry.register(gate("more", Position::AfterFinalize)),
        Err(RegistrationError::SecondIterationGate { name: "more".to_string(), existing: "again".to_string() })
    );
    registry.register(note("audit", Position::AfterFinalize)).unwrap();
}

#[test]
fn an_outcome_pair_needs_two_distinct_registered_alternatives() {
    let mut registry = md_profile();
    registry.register(outcome_event("shipped")).unwrap();
    registry.register(outcome_event("dropped")).unwrap();
    registry.register(note("announce", Position::AfterComposition)).unwrap();

    assert_eq!(
        registry.register_outcome_pair(OutcomePair::new("shipped", "shipped")),
        Err(RegistrationError::DuplicateAlternative { name: "shipped".to_string() })
    );
    for missing in ["absent", "announce", "failed"] {
        assert_eq!(
            registry.register_outcome_pair(OutcomePair::new("shipped", missing)),
            Err(RegistrationError::MissingAlternative { name: missing.to_string() })
        );
    }
    // A terminal-outcome event outside a pair has no way to be selected.
    assert_eq!(
        Run::start(&registry, DocumentInput::new("a.md")).unwrap_err(),
        StartError::Registration(RegistrationError::UnpairedOutcome { name: "shipped".to_string() })
    );
    registry.register_outcome_pair(OutcomePair::new("shipped", "dropped")).unwrap();
    assert_eq!(registry.register_outcome_pair(OutcomePair::new("shipped", "dropped")), Err(RegistrationError::SecondOutcomePair));
    assert!(Run::start(&registry, DocumentInput::new("a.md")).is_ok());
}

#[test]
fn a_downgrade_targets_only_a_registered_alternative() {
    let mut registry = md_profile();
    registry.register(outcome_event("shipped")).unwrap();
    registry.register(outcome_event("dropped")).unwrap();
    registry.register(note("announce", Position::AfterComposition)).unwrap();
    for to in ["announce", "failed", "finalize"] {
        assert_eq!(
            registry.register_outcome_pair(OutcomePair::new("shipped", "dropped").downgrade("shipped", to)),
            Err(RegistrationError::DowngradeTargetNotAlternative { from: "shipped".to_string(), to: to.to_string() })
        );
    }
    for from in ["announce", "dropped", "initialize"] {
        assert_eq!(
            registry.register_outcome_pair(OutcomePair::new("shipped", "dropped").downgrade(from, "dropped")),
            Err(RegistrationError::DowngradeSourceNotOutcome { from: from.to_string() })
        );
    }
    registry
        .register_outcome_pair(OutcomePair::new("shipped", "dropped").downgrade("shipped", "dropped").downgrade("failed", "dropped"))
        .unwrap();
}

#[test]
fn registration_is_fixed_while_a_run_holds_the_registry() {
    let mut registry = claudine_profile();
    let run = Run::start(&registry, DocumentInput::new("review.md")).unwrap();
    assert_eq!(registry.register(note("late", Position::BeforeComposition)), Err(RegistrationError::RunStarted));
    assert_eq!(registry.rename(CoreEvent::Composed, "begin"), Err(RegistrationError::RunStarted));
    assert_eq!(registry.declare_core_actions(CoreEvent::Finalize, ActionTypes::of([])), Err(RegistrationError::RunStarted));
    assert_eq!(run.registry().id("late"), None);
    drop(run);
    registry.register(note("late", Position::BeforeComposition)).unwrap();
}

#[test]
fn the_core_only_md_profile_has_no_outcome_pair_so_only_failed_holds_the_terminal_slot() {
    assert!(!md_profile().has_outcome_pair());
    let md = || Run::start(&md_profile(), DocumentInput::new("report.md")).unwrap();

    let mut denied = md();
    event(&mut denied, "initialize", ok());
    answer(&mut denied, Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, Entry::Initial, "md.shell-denied"))));
    event(&mut denied, "failed", ok());
    event(&mut denied, "finalize", ok());
    assert_eq!((denied.terminal(), denied.finish()), (Some("failed"), Some(Finish::Failed)));

    // A failed write after composition selects no event: `finalize`, non-zero.
    let mut unwritten = md();
    event(&mut unwritten, "initialize", ok());
    ready(&mut unwritten);
    event(&mut unwritten, "composed", ok());
    complete(&mut unwritten, Outcome::Failed(host("md.write-failed")));
    event(&mut unwritten, "finalize", ok());
    assert_eq!(events(&unwritten), ["initialize", "composed", "finalize"]);
    assert_eq!((unwritten.terminal(), unwritten.finish()), (None, Some(Finish::Failed)));

    // An error in `composed` does not route to `failed`; one in `initialize` does.
    let mut composed_error = md();
    event(&mut composed_error, "initialize", ok());
    ready(&mut composed_error);
    event(&mut composed_error, "composed", ok().evaluation_error(raised("md.composed")));
    event(&mut composed_error, "finalize", ok());
    assert_eq!(events(&composed_error), ["initialize", "composed", "finalize"]);
    assert_eq!(composed_error.finish(), Some(Finish::Failed));

    let mut initialize_error = md();
    event(&mut initialize_error, "initialize", ok().directive(Directive::Error(raised("md.missing-changelog"))));
    event(&mut initialize_error, "failed", ok().evaluation_error(raised("md.failed-stack")));
    event(&mut initialize_error, "finalize", ok());
    assert_eq!(events(&initialize_error), ["initialize", "failed", "finalize"]);
    // An error in `failed` replaces the selected error without re-running it.
    assert_eq!(initialize_error.selected_error(), Some(&raised("md.failed-stack")));
}

#[test]
fn darkmatter_spellings_fail_under_a_renaming_host_and_host_only_keys_are_data_under_md() {
    for (authored, replacement) in [("composed", "start"), ("failed", "blocked")] {
        assert_eq!(
            Run::start(&claudine_profile(), DocumentInput::new("review.md").with(authored, json!([]))).unwrap_err(),
            StartError::RenamedEvent { authored: authored.to_string(), replacement: replacement.to_string() }
        );
    }
    let run = Run::start(
        &md_profile(),
        DocumentInput::new("report.md").with("start", json!([{ "say": "hi" }])).with("success", json!(true)),
    )
    .unwrap();
    assert_eq!(run.state().get("start"), Some(&json!([{ "say": "hi" }])));
    assert_eq!(run.state().get("success"), Some(&json!(true)));
    assert_eq!(run.pending().event().and_then(|event| event.stack.clone()), None);
}

#[test]
fn an_event_whose_view_marks_err_unavailable_yields_the_views_reason() {
    let run = claudine(DocumentInput::new("review.md").with("err", json!("a frontmatter value named err")));
    let request = run.pending();
    let Err(LifecycleError::Unavailable(read)) = request.event().unwrap().read_err() else {
        panic!("initialize must refuse err");
    };
    assert_eq!(
        (read.root.as_str(), read.reason.code(), read.scope.as_str()),
        ("err", "lifecycle.no-error-before-composition", "claudine.initialize")
    );

    let run = failed_run();
    assert_eq!(run.pending().event().unwrap().read_err(), Ok(Some(&raised("agent.failed"))));
    let mut run = started();
    complete(&mut run, Outcome::Succeeded);
    let Err(LifecycleError::Unavailable(read)) = run.pending().event().unwrap().read_err() else {
        panic!("success must refuse err");
    };
    assert_eq!(read.reason.code(), "claudine.success-has-no-error");
}

/// The prototype parses an adopted document through the registry; the
/// production run receives the host's parse as a fact.
#[test]
fn an_adopted_document_is_split_by_the_registry() {
    let mut run = failed_run();
    event(&mut run, "failure", ok().directive(Directive::Proxy { target: "other.md".to_string() }));
    let before = run.clone();
    assert_eq!(
        run.answer(Reply::Recovery(RecoveryReply::Adopted(DocumentInput::new("other.md").with("composed", json!([]))))),
        Err(ReplyError::Document(StartError::RenamedEvent { authored: "composed".to_string(), replacement: "start".to_string() }))
    );
    assert_eq!(run, before);
}

/// Registered action types refine shell permission; a host cannot relax the
/// `initialize` invariant.
#[test]
fn registered_action_types_refine_shell_permission() {
    let mut registry = claudine_profile();
    registry.declare_core_actions(CoreEvent::Initialize, ActionTypes::all()).unwrap();
    let run = Run::start(&registry, DocumentInput::new("review.md")).unwrap();
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::InitializeForbidsShell));

    let mut registry = md_profile();
    registry.register(note("announce", Position::AfterComposition).with_actions(ActionTypes::of([]))).unwrap();
    let mut run = Run::start(&registry, DocumentInput::new("report.md")).unwrap();
    event(&mut run, "initialize", ok());
    ready(&mut run);
    event(&mut run, "composed", ok());
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::NotAllowedInEvent { event: "announce".to_string() }));
}
