//! Registration, the reply protocol, cancellation, shell permission, and the
//! spec section 8 behavior, driven through the Claudine and `md` profiles.

use darkmatter::markdown::compose::expression::Availability;
use serde_json::json;

use super::engine::{
    ActionTypes, ApprovalEpoch, CancelError, CoreEvent, Directive, DocumentInput, Entry, EvaluationMode, EventDescriptor,
    EventId, EventOutcome, FailureStage, Finish, HostError, HostFailure, KeyResolution, LifecycleError, OperationReport,
    Outcome, OutcomePair, Position, Preparation, RecoveryKind, RecoveryReply, RegistrationError, Reply, ReplyError,
    RequestKind, Role, Run, SessionAvailability, ShellLock, ShellRefusal, StartError, TranscriptEntry, err_view,
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

fn raised_as(code: &str, message: &str) -> LifecycleError {
    LifecycleError::Host(HostError::new(code, message))
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

fn mode_of(run: &Run, name: &str) -> Vec<EvaluationMode> {
    run.transcript()
        .iter()
        .filter_map(|entry| match entry {
            TranscriptEntry::Event { name: fired, mode } if fired == name => Some(*mode),
            _ => None,
        })
        .collect()
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
fn every_request_is_bound_to_run_document_attempt_iteration_and_a_rising_generation() {
    let mut run = failed_run();
    let mut tickets = vec![run.pending().ticket()];
    let mut step = |run: &mut Run, reply: Reply| {
        answer(run, reply);
        tickets.push(run.pending().ticket());
    };
    step(&mut run, Reply::Event(ok().directive(Directive::Retry { max_attempts: 1 })));
    step(&mut run, Reply::Recovery(RecoveryReply::Admitted));
    assert_eq!(run.pending().kind(), &RequestKind::Prepare { entry: Entry::Retry });
    step(&mut run, Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(2) }));
    step(&mut run, Reply::Event(ok()));
    step(&mut run, Reply::Operation(OperationReport::Completed { outcome: Outcome::Succeeded, session: SessionAvailability::Available }));
    step(&mut run, Reply::Event(ok()));
    step(&mut run, Reply::Event(ok()));
    assert_eq!(run.pending().event_name(), Some("loop"));
    step(&mut run, Reply::Event(ok().directive(Directive::Iterate)));
    step(&mut run, Reply::Event(ok()));
    step(&mut run, Reply::Operation(OperationReport::Completed { outcome: Outcome::Failed(host("agent.failed")), session: SessionAvailability::Available }));
    step(&mut run, Reply::Event(ok().directive(Directive::Proxy { target: "fallback.md".to_string() })));
    step(&mut run, Reply::Recovery(RecoveryReply::Adopted(DocumentInput::new("fallback.md"))));

    assert!(tickets.iter().all(|ticket| ticket.run() == run.id()));
    assert!(tickets.windows(2).all(|pair| pair[0].generation() < pair[1].generation()));
    let identity = |index: usize| (tickets[index].document(), tickets[index].attempt(), tickets[index].iteration());
    assert_eq!(identity(0), (1, 1, 1), "failure, first attempt");
    assert_eq!(identity(2), (1, 2, 1), "retry preparation");
    assert_eq!(identity(8), (1, 1, 2), "start of the second iteration");
    assert_eq!(identity(tickets.len() - 1), (2, 1, 1), "initialize of the adopted document");
    assert_eq!(run.pending().event_name(), Some("initialize"));
}

#[test]
fn stale_duplicate_foreign_wrong_kind_and_late_replies_change_nothing() {
    let mut run = claudine(DocumentInput::new("review.md"));
    let other = claudine(DocumentInput::new("review.md"));
    let initialize = run.pending().ticket();
    let rejected = |run: &mut Run, ticket, reply, error| {
        let before = run.clone();
        assert_eq!(run.reply(ticket, reply), Err(error));
        assert_eq!(*run, before);
    };

    rejected(&mut run, other.pending().ticket(), Reply::Event(ok().set("x", json!(1))), ReplyError::ForeignRun);
    rejected(
        &mut run,
        initialize,
        Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(1) }),
        ReplyError::WrongKind { expected: "evaluate-event", received: "prepared" },
    );
    answer(&mut run, Reply::Event(ok().set("x", json!(1))));
    rejected(&mut run, initialize, Reply::Event(ok().set("x", json!(2))), ReplyError::Duplicate { generation: initialize.generation() });
    ready(&mut run);
    let start = run.pending().ticket();
    rejected(
        &mut run,
        initialize,
        Reply::Event(ok()),
        ReplyError::Stale { generation: initialize.generation(), current: start.generation() },
    );
    event(&mut run, "start", ok());
    let operation = run.pending().ticket();
    rejected(&mut run, operation, Reply::Event(ok()), ReplyError::WrongKind { expected: "perform-operation", received: "event" });
    rejected(&mut run, operation, Reply::Recovery(RecoveryReply::Admitted), ReplyError::WrongKind { expected: "perform-operation", received: "recovery" });
    answer(&mut run, Reply::Operation(OperationReport::Started { session: SessionAvailability::Available }));
    let completion = run.pending().ticket();
    rejected(
        &mut run,
        completion,
        Reply::Operation(OperationReport::Started { session: SessionAvailability::Available }),
        ReplyError::WrongKind { expected: "await-completion", received: "operation" },
    );
    complete(&mut run, Outcome::Succeeded);
    event(&mut run, "success", ok());
    event(&mut run, "finalize", ok());
    event(&mut run, "loop", ok());
    let done = run.pending().ticket();
    assert_eq!(run.finish(), Some(Finish::Succeeded));
    rejected(&mut run, done, Reply::Event(ok()), ReplyError::Finished);
    assert_eq!(run.state().get("x"), Some(&json!(1)));
}

#[test]
fn misplaced_directives_and_inconsistent_facts_change_nothing() {
    let rejected = |run: &mut Run, reply, error| {
        let before = run.clone();
        assert_eq!(run.answer(reply), Err(error));
        assert_eq!(*run, before);
    };
    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok());
    rejected(
        &mut run,
        Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, Entry::Retry, "x"))),
        ReplyError::EntryMismatch { expected: Entry::Initial, received: Entry::Retry },
    );
    rejected(
        &mut run,
        Reply::Prepared(Preparation::Failed(failure(FailureStage::LaunchConstruction, Entry::Initial, "x"))),
        ReplyError::StageNotApplicable { stage: FailureStage::LaunchConstruction, request: "prepare" },
    );
    ready(&mut run);
    for (directive, name) in [(Directive::Skip, "skip"), (Directive::Iterate, "iterate")] {
        rejected(
            &mut run,
            Reply::Event(ok().directive(directive)),
            ReplyError::DirectiveNotPermitted { directive: name, event: "start".to_string() },
        );
    }
    event(&mut run, "start", ok());
    rejected(
        &mut run,
        Reply::Operation(OperationReport::NotStarted(failure(FailureStage::Preflight, Entry::Initial, "x"))),
        ReplyError::StageNotApplicable { stage: FailureStage::Preflight, request: "perform-operation" },
    );
    complete(&mut run, Outcome::Failed(host("agent.failed")));
    event(&mut run, "failure", ok().directive(Directive::Proxy { target: "other.md".to_string() }));
    rejected(&mut run, Reply::Recovery(RecoveryReply::Admitted), ReplyError::WrongRecovery { requested: RecoveryKind::Proxy });
    // Only a committed handoff's setup failure aborts a proxy.
    rejected(
        &mut run,
        Reply::Recovery(RecoveryReply::Aborted(failure(FailureStage::RecoveryRefused, Entry::Initial, "x"))),
        ReplyError::StageNotApplicable { stage: FailureStage::RecoveryRefused, request: "recover" },
    );
    rejected(
        &mut run,
        Reply::Recovery(RecoveryReply::Adopted(DocumentInput::new("other.md").with("composed", json!([])))),
        ReplyError::Document(StartError::RenamedEvent { authored: "composed".to_string(), replacement: "start".to_string() }),
    );
}

#[test]
fn cancel_invalidates_outstanding_work_then_runs_cancelled_and_finalize() {
    let mut run = claudine(DocumentInput::new("review.md").with("cancelled", json!([{ "info": "stopping" }])));
    event(&mut run, "initialize", ok());
    ready(&mut run);
    event(&mut run, "start", ok());
    answer(&mut run, Reply::Operation(OperationReport::Started { session: SessionAvailability::Available }));
    let work = run.pending().ticket();

    run.cancel().unwrap();
    let cancelled = run.pending();
    assert_eq!(cancelled.event().map(|event| (event.name.as_str(), event.mode)), Some(("cancelled", EvaluationMode::CancellationCleanup)));
    assert_eq!(cancelled.event().and_then(|event| event.stack.clone()), Some(json!([{ "info": "stopping" }])));

    let before = run.clone();
    let late = Reply::Operation(OperationReport::Completed { outcome: Outcome::Succeeded, session: SessionAvailability::Available });
    assert_eq!(run.reply(work, late), Err(ReplyError::CancelledWork { generation: work.generation() }));
    assert_eq!(run, before);

    event(&mut run, "cancelled", ok().directive(Directive::Retry { max_attempts: 1 }));
    assert_eq!(mode_of(&run, "finalize"), [EvaluationMode::CancellationCleanup]);
    event(&mut run, "finalize", ok());
    assert_eq!(
        run.transcript_lines()[4..],
        ["operation", "operation started", "cancel", "error composition.cancelled", "event cancelled", "event finalize", "finish cancelled"]
    );
    assert_eq!(run.selected_error(), Some(&raised_as("composition.cancelled", "the run was cancelled")));
    assert_eq!(run.cancel(), Err(CancelError::Finished));
}

#[test]
fn cancel_without_a_cancelled_stack_runs_only_finalize_even_during_initialize() {
    let mut run = claudine(DocumentInput::new("review.md"));
    let initialize = run.pending().ticket();
    run.cancel().unwrap();
    let before = run.clone();
    assert_eq!(run.cancel(), Err(CancelError::AlreadyCancelled));
    assert_eq!(run, before);

    let finalize = run.pending().ticket();
    assert_eq!(run.shell_permission(&finalize), Err(ShellRefusal::Locked));
    assert_eq!(
        run.reply(initialize, Reply::Event(ok().directive(Directive::Skip))),
        Err(ReplyError::CancelledWork { generation: initialize.generation() })
    );
    event(&mut run, "finalize", ok().directive(Directive::Retry { max_attempts: 1 }));
    assert_eq!(
        run.transcript_lines(),
        ["event initialize", "cancel", "error composition.cancelled", "event finalize", "finish cancelled"]
    );
}

#[test]
fn late_replies_after_cancellation_cannot_emit_an_outcome_reopen_shells_or_finalize_again() {
    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok());
    let prepare = run.pending().ticket();
    run.cancel().unwrap();
    assert!(run.reply(prepare, Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(9) })).is_err());
    assert_eq!(run.shell_lock(), ShellLock::Locked);
    let finalize = run.pending().ticket();
    event(&mut run, "finalize", ok());
    assert_eq!(run.reply(finalize, Reply::Event(ok())), Err(ReplyError::Duplicate { generation: finalize.generation() }));
    assert_eq!(run.reply(prepare, Reply::Event(ok())), Err(ReplyError::CancelledWork { generation: prepare.generation() }));

    // Cancelled while `success` was being evaluated: its late reply cannot
    // request recovery, and `finalize` runs exactly once.
    let mut run = started();
    complete(&mut run, Outcome::Succeeded);
    let success = run.pending().ticket();
    run.cancel().unwrap();
    assert!(run.reply(success, Reply::Event(ok().directive(Directive::Retry { max_attempts: 1 }))).is_err());
    event(&mut run, "finalize", ok());
    assert_eq!(events(&run), ["initialize", "start", "success", "finalize"]);
    assert_eq!(run.finish(), Some(Finish::Cancelled));
}

#[test]
fn a_cancel_during_cleanup_lets_that_cleanup_finish_without_a_second_pass() {
    let mut run = failed_run();
    event(&mut run, "failure", ok());
    let finalize = run.pending().ticket();
    run.cancel().unwrap();
    assert_eq!(run.pending().ticket(), finalize);
    event(&mut run, "finalize", ok().directive(Directive::Retry { max_attempts: 1 }));
    assert_eq!(events(&run), ["initialize", "start", "failure", "finalize"]);
    assert_eq!(run.finish(), Some(Finish::Cancelled));
}

#[test]
fn the_run_alone_answers_shell_permission_from_initialize_to_proxy_adoption() {
    let mut registry = claudine_profile();
    // A host declaring `shell` for `initialize` cannot relax the invariant.
    registry.declare_core_actions(CoreEvent::Initialize, ActionTypes::all()).unwrap();
    let mut run = Run::start(&registry, DocumentInput::new("review.md")).unwrap();
    let initialize = run.pending().ticket();
    assert_eq!(run.shell_permission(&initialize), Err(ShellRefusal::InitializeForbidsShell));
    event(&mut run, "initialize", ok());
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::NotCurrentEvent));
    assert_eq!(run.shell_permission(&initialize), Err(ShellRefusal::NotCurrentEvent));
    answer(&mut run, Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(4) }));
    assert_eq!(run.shell_permission(&run.pending().ticket()), Ok(ApprovalEpoch(4)));

    event(&mut run, "start", ok());
    complete(&mut run, Outcome::Failed(host("agent.failed")));
    assert_eq!(run.shell_permission(&run.pending().ticket()), Ok(ApprovalEpoch(4)));
    event(&mut run, "failure", ok().directive(Directive::Proxy { target: "fallback.md".to_string() }));
    answer(&mut run, Reply::Recovery(RecoveryReply::Adopted(DocumentInput::new("fallback.md"))));
    assert_eq!(run.shell_lock(), ShellLock::Locked);
    event(&mut run, "initialize", ok());
    answer(&mut run, Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, Entry::Initial, "audit.denied"))));
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::Locked), "early blocked");
    event(&mut run, "blocked", ok());
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::Locked), "early finalize");
    event(&mut run, "finalize", ok());

    let shell: Vec<_> = run.transcript_lines().into_iter().filter(|line| line.starts_with("shell") || line.starts_with("adopt")).collect();
    assert_eq!(shell, ["shell unlocked", "shell locked", "adopt document 2"]);

    // An event that does not admit `shell` refuses it even when unlocked.
    let mut registry = md_profile();
    registry.register(note("announce", Position::AfterComposition).with_actions(ActionTypes::of([]))).unwrap();
    let mut run = Run::start(&registry, DocumentInput::new("report.md")).unwrap();
    event(&mut run, "initialize", ok());
    ready(&mut run);
    event(&mut run, "composed", ok());
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::NotAllowedInEvent { event: "announce".to_string() }));
}

#[test]
fn the_failure_stage_and_launch_state_select_blocked_or_failure() {
    // Composition or preflight failure: blocked, with normal recovery rules.
    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok());
    answer(&mut run, Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, Entry::Initial, "schema.invalid"))));
    event(&mut run, "blocked", ok());
    event(&mut run, "finalize", ok());
    assert_eq!(events(&run), ["initialize", "blocked", "finalize"]);
    assert_eq!(mode_of(&run, "blocked"), [EvaluationMode::NormalRecovery]);

    let not_started = |stage| {
        let mut run = started();
        answer(&mut run, Reply::Operation(OperationReport::NotStarted(failure(stage, Entry::Initial, "setup.failed"))));
        run
    };
    // Internal setup failure after `start`, before launch: blocked.
    let mut run = not_started(FailureStage::Setup);
    event(&mut run, "blocked", ok());
    event(&mut run, "finalize", ok());
    assert_eq!(events(&run), ["initialize", "start", "blocked", "finalize"]);
    assert_eq!(mode_of(&run, "blocked"), [EvaluationMode::RefusalCleanup]);

    // Launch construction after `start`: failure.
    let mut run = not_started(FailureStage::LaunchConstruction);
    event(&mut run, "failure", ok().directive(Directive::Retry { max_attempts: 1 }));
    event(&mut run, "finalize", ok());
    assert_eq!(events(&run), ["initialize", "start", "failure", "finalize"]);

    // A proxy refused before launch, even after `start` fired: blocked.
    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok());
    ready(&mut run);
    event(&mut run, "start", ok().directive(Directive::Proxy { target: "loop.md".to_string() }));
    answer(&mut run, Reply::Recovery(RecoveryReply::Refused(failure(FailureStage::HandoffRefused, Entry::Initial, "proxy.cycle"))));
    event(&mut run, "blocked", ok());
    event(&mut run, "finalize", ok());
    assert_eq!(events(&run), ["initialize", "start", "blocked", "finalize"]);
    assert_eq!(run.selected_error(), Some(&raised("proxy.cycle")));

    // A resume refused as incompatible after re-entering at `start`: failure.
    let mut run = failed_run();
    event(&mut run, "failure", ok().directive(Directive::Resume { max_attempts: 1 }));
    answer(&mut run, Reply::Recovery(RecoveryReply::Admitted));
    assert_eq!(run.pending().kind(), &RequestKind::Prepare { entry: Entry::Resume });
    ready(&mut run);
    event(&mut run, "start", ok());
    answer(&mut run, Reply::Operation(OperationReport::NotStarted(failure(FailureStage::ResumeIncompatible, Entry::Resume, "resume.incompatible"))));
    event(&mut run, "failure", ok());
    event(&mut run, "finalize", ok());
    assert_eq!(events(&run), ["initialize", "start", "failure", "start", "failure", "finalize"]);
    assert_eq!(mode_of(&run, "failure"), [EvaluationMode::NormalRecovery, EvaluationMode::RefusalCleanup]);

    // A handoff refused after launch: `success` already holds the slot, so
    // no second terminal event fires; only the owed `finalize`.
    let mut run = started();
    complete(&mut run, Outcome::Succeeded);
    event(&mut run, "success", ok().directive(Directive::Proxy { target: "next.md".to_string() }));
    answer(&mut run, Reply::Recovery(RecoveryReply::Refused(failure(FailureStage::HandoffRefused, Entry::Initial, "proxy.unowned"))));
    event(&mut run, "finalize", ok());
    assert_eq!(events(&run), ["initialize", "start", "success", "finalize"]);
    assert_eq!((run.terminal(), run.finish()), (Some("success"), Some(Finish::Failed)));
}

#[test]
fn an_explicit_error_downgrades_success_or_blocked_to_failure_whose_recovery_stays_eligible() {
    let mut run = started();
    complete(&mut run, Outcome::Succeeded);
    event(&mut run, "success", ok().directive(Directive::Error(raised("review.rejected"))));
    assert_eq!(run.terminal(), Some("failure"));
    event(&mut run, "failure", ok().directive(Directive::Retry { max_attempts: 1 }));
    assert!(matches!(run.pending().kind(), RequestKind::Recover(recovery) if recovery.kind == RecoveryKind::Retry && recovery.from_name == "failure"));

    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok());
    answer(&mut run, Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, Entry::Initial, "schema.invalid"))));
    event(&mut run, "blocked", ok().directive(Directive::Error(raised("blocked.explicit"))));
    assert_eq!((run.terminal(), run.selected_error()), (Some("failure"), Some(&raised("blocked.explicit"))));
    event(&mut run, "failure", ok().directive(Directive::Retry { max_attempts: 1 }));
    assert!(matches!(run.pending().kind(), RequestKind::Recover(_)));

    // `failure` has no downgrade: its explicit error ends in cleanup only.
    let mut run = failed_run();
    event(&mut run, "failure", ok().directive(Directive::Error(raised("failure.explicit"))));
    event(&mut run, "finalize", ok().directive(Directive::Retry { max_attempts: 1 }));
    assert_eq!(events(&run), ["initialize", "start", "failure", "finalize"]);
    assert_eq!(run.finish(), Some(Finish::Failed));
}

#[test]
fn skip_ends_the_run_from_initialize_without_finalize() {
    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok().directive(Directive::Skip));
    assert_eq!(run.transcript_lines(), ["event initialize", "finish skipped"]);
}

#[test]
fn retry_or_resume_from_a_terminal_event_skips_that_attempts_finalize() {
    for (directive, entry) in [(Directive::Retry { max_attempts: 1 }, Entry::Retry), (Directive::Resume { max_attempts: 1 }, Entry::Resume)] {
        let mut run = failed_run();
        event(&mut run, "failure", ok().directive(directive));
        answer(&mut run, Reply::Recovery(RecoveryReply::Admitted));
        assert_eq!(run.pending().kind(), &RequestKind::Prepare { entry });
        ready(&mut run);
        event(&mut run, "start", ok());
        complete(&mut run, Outcome::Succeeded);
        event(&mut run, "success", ok());
        event(&mut run, "finalize", ok());
        event(&mut run, "loop", ok());
        assert_eq!(events(&run), ["initialize", "start", "failure", "start", "success", "finalize", "loop"]);
        assert_eq!(run.finish(), Some(Finish::Succeeded));
    }

    // The host owns the budget; an exhausted one continues to `finalize`.
    let mut run = failed_run();
    event(&mut run, "failure", ok().directive(Directive::Retry { max_attempts: 1 }));
    answer(&mut run, Reply::Recovery(RecoveryReply::Exhausted));
    assert_eq!(mode_of(&run, "finalize"), [EvaluationMode::NormalRecovery]);
}

#[test]
fn proxy_from_a_terminal_event_never_runs_the_source_finalize() {
    let mut run = failed_run();
    event(&mut run, "failure", ok().directive(Directive::Proxy { target: "fallback.md".to_string() }));
    answer(&mut run, Reply::Recovery(RecoveryReply::Adopted(DocumentInput::new("fallback.md").with("goal", json!("retry")))));
    assert_eq!(run.state().get("goal"), Some(&json!("retry")));
    event(&mut run, "initialize", ok());
    ready(&mut run);
    event(&mut run, "start", ok());
    complete(&mut run, Outcome::Succeeded);
    event(&mut run, "success", ok());
    event(&mut run, "finalize", ok());
    event(&mut run, "loop", ok());
    assert_eq!(events(&run), ["initialize", "start", "failure", "initialize", "start", "success", "finalize", "loop"]);
}

#[test]
fn a_recovery_chosen_in_finalize_is_never_followed_by_a_second_finalize_for_that_attempt() {
    let mut run = failed_run();
    event(&mut run, "failure", ok());
    event(&mut run, "finalize", ok().directive(Directive::Retry { max_attempts: 1 }));
    answer(&mut run, Reply::Recovery(RecoveryReply::Admitted));
    ready(&mut run);
    event(&mut run, "start", ok());
    complete(&mut run, Outcome::Failed(host("agent.failed")));
    event(&mut run, "failure", ok());
    event(&mut run, "finalize", ok());
    assert_eq!(events(&run), ["initialize", "start", "failure", "finalize", "start", "failure", "finalize"]);

    // An evaluation error inside `finalize` ends the run without finalizing again.
    let mut run = failed_run();
    event(&mut run, "failure", ok());
    event(&mut run, "finalize", ok().evaluation_error(raised("finalize.eval")));
    assert_eq!(events(&run), ["initialize", "start", "failure", "finalize"]);
    assert_eq!((run.selected_error(), run.finish()), (Some(&raised("finalize.eval")), Some(Finish::Failed)));
}

#[test]
fn blocked_reached_from_preflight_cannot_recover() {
    for (directive, action) in [
        (Directive::Retry { max_attempts: 1 }, RecoveryKind::Retry),
        (Directive::Resume { max_attempts: 1 }, RecoveryKind::Resume),
        (Directive::Proxy { target: "other.md".to_string() }, RecoveryKind::Proxy),
    ] {
        let mut run = claudine(DocumentInput::new("review.md"));
        event(&mut run, "initialize", ok());
        answer(&mut run, Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, Entry::Initial, "audit.denied"))));
        event(&mut run, "blocked", ok().directive(directive));
        assert_eq!(
            run.selected_error(),
            Some(&LifecycleError::SetupRecoveryUnsupported { event: "blocked".to_string(), action })
        );
        event(&mut run, "finalize", ok());
        assert!(!run.transcript_lines().iter().any(|line| line.starts_with("recover")));
        assert_eq!(run.finish(), Some(Finish::Failed));
    }
}

#[test]
fn an_evaluation_failure_in_start_is_caught_by_failure_and_finalize_as_cleanup_only() {
    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok());
    ready(&mut run);
    event(&mut run, "start", ok().evaluation_error(raised("start.eval")));
    event(&mut run, "failure", ok().directive(Directive::Retry { max_attempts: 1 }));
    event(&mut run, "finalize", ok().evaluation_error(raised("finalize.eval")));
    assert_eq!(events(&run), ["initialize", "start", "failure", "finalize"]);
    assert_eq!(mode_of(&run, "failure"), [EvaluationMode::FatalEvaluationCleanup]);
    // The later evaluation error replaces the earlier one.
    assert_eq!(run.selected_error(), Some(&raised("finalize.eval")));
    assert_eq!((run.terminal(), run.finish()), (Some("failure"), Some(Finish::Failed)));

    // An evaluation failure in `blocked` re-designates the slot to `failure`.
    let mut run = claudine(DocumentInput::new("review.md"));
    event(&mut run, "initialize", ok());
    answer(&mut run, Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, Entry::Initial, "audit.denied"))));
    event(&mut run, "blocked", ok().evaluation_error(raised("blocked.eval")));
    assert_eq!(run.terminal(), Some("failure"));
    event(&mut run, "failure", ok());
    event(&mut run, "finalize", ok());
    assert_eq!(run.selected_error(), Some(&raised("blocked.eval")));
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

#[test]
fn the_iteration_gate_reenters_at_start_after_a_successful_unbroken_pass_only() {
    let mut run = started();
    complete(&mut run, Outcome::Succeeded);
    event(&mut run, "success", ok());
    event(&mut run, "finalize", ok());
    event(&mut run, "loop", ok().directive(Directive::Iterate));
    let ticket = run.pending().ticket();
    assert_eq!((run.pending().event_name(), ticket.iteration(), ticket.attempt()), (Some("start"), 2, 1));
    assert_eq!(run.shell_permission(&ticket), Ok(ApprovalEpoch(1)));
    event(&mut run, "start", ok());
    complete(&mut run, Outcome::Succeeded);
    event(&mut run, "success", ok().directive(Directive::Break));
    event(&mut run, "finalize", ok());
    assert_eq!(
        run.transcript_lines()[5..],
        [
            "event success",
            "event finalize",
            "event loop",
            "iteration 2",
            "event start",
            "operation",
            "event success",
            "event finalize",
            "finish succeeded",
        ]
    );

    // A failed pass ends the loop before its gate.
    let mut run = failed_run();
    event(&mut run, "failure", ok());
    event(&mut run, "finalize", ok());
    assert_eq!(run.finish(), Some(Finish::Failed));
    assert!(!events(&run).contains(&"loop".to_string()));
}
