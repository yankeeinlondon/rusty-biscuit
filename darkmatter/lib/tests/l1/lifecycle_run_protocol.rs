//! The request/reply protocol of `darkmatter::lifecycle::run::Run`: ticket
//! binding, rejected replies, cancellation, the shell owner, the failure-stage
//! table, downgrade, skip, recovery exceptions, and isolation between runs.
//! Ported from the Stage 1 prototype's protocol tests; the events are the
//! fixed `LifecycleSignal` set.

use std::error::Error as StdError;
use std::sync::Arc;

use darkmatter::lifecycle::actions::{LifecyclePath, RetryBackoff};
use darkmatter::lifecycle::envelope::{DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION, DiagnosticCatalog, DiagnosticSnapshot};
use darkmatter::lifecycle::executor::{ActionLocation, LifecycleEventOutcome, StackControl};
use darkmatter::lifecycle::run::{
    ApprovalEpoch, AuthoredEvents, CancelError, CompletionVerdict, EntryReason, FailureStage, Finish, HostFailure,
    OperationReport, Preparation, RecoveryKind, RecoveryReply, Reply, ReplyError, RequestKind, Run, RunError,
    RunSettings, SessionAvailability, ShellLock, ShellRefusal, TranscriptEntry,
};
use darkmatter::lifecycle::runtime::{EvaluationMode, LifecycleTransitionAbort};
use darkmatter::lifecycle::{LifecycleErrorInfo, LifecycleSignal};
use indexmap::IndexMap;
use serde_json::json;

use LifecycleSignal::{Blocked, Cancelled, Failure, Finalize, Initialize, Loop, Start, Success};

struct NoCodes;

impl DiagnosticCatalog for NoCodes {
    fn select(&self, _: &(dyn StdError + 'static)) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_code(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
    fn snapshot_for_error_kind(&self, _: &str, _: String) -> Option<DiagnosticSnapshot> {
        None
    }
}

fn coded(code: &str, message: &str) -> LifecycleErrorInfo {
    let snapshot = DiagnosticSnapshot {
        schema_version: DIAGNOSTIC_SNAPSHOT_SCHEMA_VERSION,
        category: code.split('.').next().unwrap_or(code).to_string(),
        code: code.to_string(),
        disposition: "correctable".to_string(),
        origin: "author".to_string(),
        severity: "error".to_string(),
        detail: json!({}),
        message: message.to_string(),
        cause: None,
        frontmatter_excerpt: None,
    };
    LifecycleErrorInfo::from_snapshot("Host", code.to_string(), snapshot)
}

fn host(code: &str) -> LifecycleErrorInfo {
    coded(code, code)
}

fn raised(code: &str) -> RunError {
    RunError::Reported(host(code))
}

/// What an explicit `error("…")` selects.
fn explicit(reason: &str) -> RunError {
    RunError::Reported(LifecycleErrorInfo::from_action_failure_with(&NoCodes, "error", reason))
}

fn failure(stage: FailureStage, entry: EntryReason, code: &str) -> HostFailure {
    HostFailure { stage, entry, error: host(code) }
}

fn ok() -> LifecycleEventOutcome {
    LifecycleEventOutcome::default()
}

fn directed(control: StackControl) -> LifecycleEventOutcome {
    LifecycleEventOutcome { control: Some(control), ..ok() }
}

fn evaluation_error(code: &str) -> LifecycleEventOutcome {
    LifecycleEventOutcome { evaluation_error: Some(host(code)), ..ok() }
}

fn retry() -> StackControl {
    StackControl::Retry { max_attempts: 1, backoff: RetryBackoff::Fixed, delay: "0s".to_string(), overlay: IndexMap::new() }
}

fn resume() -> StackControl {
    StackControl::Resume { message: "continue".to_string(), max_attempts: 1 }
}

fn proxy(from: LifecycleSignal, target: &str) -> StackControl {
    StackControl::Proxy {
        target: target.to_string(),
        overlay: IndexMap::new(),
        location: ActionLocation::new(from, LifecyclePath::root(from.property_name())),
    }
}

fn error(reason: &str) -> StackControl {
    StackControl::Error { reason: Some(reason.to_string()) }
}

fn settings() -> RunSettings {
    RunSettings::new(Arc::new(NoCodes)).with_cancellation_error(coded("composition.cancelled", "the run was cancelled"))
}

fn run_of(authored: AuthoredEvents) -> Run {
    Run::start(settings(), authored)
}

fn fresh() -> Run {
    run_of(AuthoredEvents::default())
}

fn answer(run: &mut Run, reply: Reply) {
    run.answer(reply).unwrap_or_else(|error| panic!("{error:?} after {:?}", run.transcript_lines()));
}

fn event(run: &mut Run, signal: LifecycleSignal, outcome: LifecycleEventOutcome) {
    assert_eq!(run.pending().event().map(|event| event.signal), Some(signal), "after {:?}", run.transcript_lines());
    answer(run, Reply::Event(outcome));
}

fn gate(run: &mut Run, iterate: bool) {
    assert_eq!(run.pending().event().map(|event| event.signal), Some(Loop), "after {:?}", run.transcript_lines());
    answer(run, Reply::Gate { outcome: ok(), iterate });
}

fn ready(run: &mut Run) {
    answer(run, Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(1) }));
}

fn complete(run: &mut Run, verdict: CompletionVerdict) {
    answer(run, Reply::Operation(OperationReport::Completed { verdict, session: SessionAvailability::Available }));
}

fn budget(run: &mut Run, ceiling: u32) {
    answer(run, Reply::Recovery(RecoveryReply::Budget { ceiling }));
}

/// A run past `start`, waiting for its operation.
fn started() -> Run {
    let mut run = fresh();
    event(&mut run, Initialize, ok());
    ready(&mut run);
    event(&mut run, Start, ok());
    run
}

/// A run whose operation failed, waiting on `failure`.
fn failed_run() -> Run {
    let mut run = started();
    complete(&mut run, CompletionVerdict::Failed(host("agent.failed")));
    run
}

/// A run whose preparation failed, waiting on `blocked`.
fn preflight_blocked(code: &str) -> Run {
    let mut run = fresh();
    event(&mut run, Initialize, ok());
    answer(&mut run, Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, EntryReason::Initial, code))));
    run
}

fn events(run: &Run) -> Vec<LifecycleSignal> {
    run.transcript()
        .iter()
        .filter_map(|entry| match entry {
            TranscriptEntry::Event { signal, .. } => Some(*signal),
            _ => None,
        })
        .collect()
}

fn mode_of(run: &Run, wanted: LifecycleSignal) -> Vec<EvaluationMode> {
    run.transcript()
        .iter()
        .filter_map(|entry| match entry {
            TranscriptEntry::Event { signal, mode } if *signal == wanted => Some(*mode),
            _ => None,
        })
        .collect()
}

/// `reply` is refused with `expected`, and the run is unchanged.
fn rejected(run: &mut Run, ticket: darkmatter::lifecycle::run::Ticket, reply: Reply, expected: ReplyError) {
    let before = run.snapshot();
    assert_eq!(run.reply(ticket, reply), Err(expected));
    assert_eq!(run.snapshot(), before);
}

#[test]
fn every_request_is_bound_to_run_document_attempt_iteration_and_a_rising_generation() {
    let mut run = failed_run();
    let mut tickets = vec![run.pending().ticket()];
    let mut step = |run: &mut Run, reply: Reply| {
        answer(run, reply);
        tickets.push(run.pending().ticket());
    };
    step(&mut run, Reply::Event(directed(retry())));
    step(&mut run, Reply::Recovery(RecoveryReply::Budget { ceiling: 2 }));
    assert_eq!(run.pending().kind(), &RequestKind::Prepare { entry: EntryReason::Retry });
    step(&mut run, Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(2) }));
    step(&mut run, Reply::Event(ok()));
    step(&mut run, Reply::Operation(OperationReport::Completed { verdict: CompletionVerdict::Succeeded, session: SessionAvailability::Available }));
    step(&mut run, Reply::Event(ok()));
    step(&mut run, Reply::Event(ok()));
    assert_eq!(run.pending().event().map(|event| event.signal), Some(Loop));
    step(&mut run, Reply::Gate { outcome: ok(), iterate: true });
    step(&mut run, Reply::Event(ok()));
    step(
        &mut run,
        Reply::Operation(OperationReport::Completed {
            verdict: CompletionVerdict::Failed(host("agent.failed")),
            session: SessionAvailability::Available,
        }),
    );
    step(&mut run, Reply::Event(directed(proxy(Failure, "fallback.md"))));
    step(&mut run, Reply::Recovery(RecoveryReply::Adopted(AuthoredEvents::default())));

    assert!(tickets.iter().all(|ticket| ticket.run() == run.id()));
    assert!(tickets.windows(2).all(|pair| pair[0].generation() < pair[1].generation()));
    let identity = |index: usize| (tickets[index].document(), tickets[index].attempt(), tickets[index].iteration());
    assert_eq!(identity(0), (1, 1, 1), "failure, first attempt");
    assert_eq!(identity(2), (1, 2, 1), "retry preparation");
    assert_eq!(identity(8), (1, 1, 2), "start of the second iteration");
    assert_eq!(identity(tickets.len() - 1), (2, 1, 1), "initialize of the adopted document");
    assert_eq!(run.pending().event().map(|event| event.signal), Some(Initialize));
}

#[test]
fn stale_duplicate_foreign_wrong_kind_and_late_replies_change_nothing() {
    let mut run = fresh();
    let other = fresh();
    let initialize = run.pending().ticket();
    let failing = || LifecycleEventOutcome { action_error: Some(host("x")), ..ok() };

    rejected(&mut run, other.pending().ticket(), Reply::Event(failing()), ReplyError::ForeignRun);
    rejected(
        &mut run,
        initialize,
        Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(1) }),
        ReplyError::WrongKind { expected: "evaluate-event", received: "prepared" },
    );
    answer(&mut run, Reply::Event(ok()));
    rejected(&mut run, initialize, Reply::Event(failing()), ReplyError::Duplicate { generation: initialize.generation() });
    ready(&mut run);
    let start = run.pending().ticket();
    rejected(
        &mut run,
        initialize,
        Reply::Event(ok()),
        ReplyError::Stale { generation: initialize.generation(), current: start.generation() },
    );
    event(&mut run, Start, ok());
    let operation = run.pending().ticket();
    rejected(&mut run, operation, Reply::Event(ok()), ReplyError::WrongKind { expected: "perform-operation", received: "event" });
    rejected(
        &mut run,
        operation,
        Reply::Recovery(RecoveryReply::Budget { ceiling: 2 }),
        ReplyError::WrongKind { expected: "perform-operation", received: "recovery" },
    );
    answer(&mut run, Reply::Operation(OperationReport::Started { session: SessionAvailability::Available }));
    let completion = run.pending().ticket();
    rejected(
        &mut run,
        completion,
        Reply::Operation(OperationReport::Started { session: SessionAvailability::Available }),
        ReplyError::WrongKind { expected: "await-completion", received: "operation" },
    );
    complete(&mut run, CompletionVerdict::Succeeded);
    event(&mut run, Success, ok());
    event(&mut run, Finalize, ok());
    event(&mut run, Loop, ok());
    let done = run.pending().ticket();
    assert_eq!(run.finish(), Some(Finish::Succeeded));
    rejected(&mut run, done, Reply::Event(ok()), ReplyError::Finished);
    assert_eq!(run.selected_error(), None);
}

#[test]
fn misplaced_directives_and_inconsistent_facts_change_nothing() {
    let rejected = |run: &mut Run, reply, expected| {
        let ticket = run.pending().ticket();
        rejected(run, ticket, reply, expected);
    };
    let mut run = fresh();
    event(&mut run, Initialize, ok());
    rejected(
        &mut run,
        Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, EntryReason::Retry, "x"))),
        ReplyError::EntryMismatch { expected: EntryReason::Initial, received: EntryReason::Retry },
    );
    rejected(
        &mut run,
        Reply::Prepared(Preparation::Failed(failure(FailureStage::LaunchConstruction, EntryReason::Initial, "x"))),
        ReplyError::StageNotApplicable { stage: FailureStage::LaunchConstruction, request: "prepare" },
    );
    ready(&mut run);
    rejected(&mut run, Reply::Event(directed(StackControl::Skip)), ReplyError::DirectiveNotPermitted { directive: "skip", event: Start });
    rejected(&mut run, Reply::Gate { outcome: ok(), iterate: true }, ReplyError::DirectiveNotPermitted { directive: "iterate", event: Start });
    event(&mut run, Start, ok());
    rejected(
        &mut run,
        Reply::Operation(OperationReport::NotStarted(failure(FailureStage::Preflight, EntryReason::Initial, "x"))),
        ReplyError::StageNotApplicable { stage: FailureStage::Preflight, request: "perform-operation" },
    );
    complete(&mut run, CompletionVerdict::Failed(host("agent.failed")));
    event(&mut run, Failure, directed(proxy(Failure, "other.md")));
    rejected(&mut run, Reply::Recovery(RecoveryReply::Budget { ceiling: 2 }), ReplyError::WrongRecovery { requested: RecoveryKind::Proxy });
    // Only a committed hand-off's setup failure aborts a proxy.
    rejected(
        &mut run,
        Reply::Recovery(RecoveryReply::Aborted(failure(FailureStage::RecoveryRefused, EntryReason::Initial, "x"))),
        ReplyError::StageNotApplicable { stage: FailureStage::RecoveryRefused, request: "recover" },
    );
}

#[test]
fn cancel_invalidates_outstanding_work_then_runs_cancelled_and_finalize() {
    let mut run = run_of(AuthoredEvents::of([Cancelled]));
    event(&mut run, Initialize, ok());
    ready(&mut run);
    event(&mut run, Start, ok());
    answer(&mut run, Reply::Operation(OperationReport::Started { session: SessionAvailability::Available }));
    let work = run.pending().ticket();

    run.cancel().unwrap();
    let cancelled = run.pending();
    assert_eq!(cancelled.event().map(|event| (event.signal, event.mode)), Some((Cancelled, EvaluationMode::CancellationCleanup)));

    let late = Reply::Operation(OperationReport::Completed { verdict: CompletionVerdict::Succeeded, session: SessionAvailability::Available });
    rejected(&mut run, work, late, ReplyError::CancelledWork { generation: work.generation() });

    event(&mut run, Cancelled, directed(retry()));
    assert_eq!(mode_of(&run, Finalize), [EvaluationMode::CancellationCleanup]);
    event(&mut run, Finalize, ok());
    assert_eq!(
        run.transcript_lines()[4..],
        ["operation", "operation started", "cancel", "error composition.cancelled", "event cancelled", "event finalize", "finish cancelled"]
    );
    assert_eq!(run.selected_error(), Some(&RunError::Reported(coded("composition.cancelled", "the run was cancelled"))));
    assert_eq!(run.cancel(), Err(CancelError::Finished));
}

#[test]
fn cancel_without_a_cancelled_stack_runs_only_finalize_even_during_initialize() {
    let mut run = fresh();
    let initialize = run.pending().ticket();
    run.cancel().unwrap();
    let before = run.snapshot();
    assert_eq!(run.cancel(), Err(CancelError::AlreadyCancelled));
    assert_eq!(run.snapshot(), before);

    let finalize = run.pending().ticket();
    assert_eq!(run.shell_permission(&finalize), Err(ShellRefusal::Locked));
    assert_eq!(
        run.reply(initialize, Reply::Event(directed(StackControl::Skip))),
        Err(ReplyError::CancelledWork { generation: initialize.generation() })
    );
    event(&mut run, Finalize, directed(retry()));
    assert_eq!(run.transcript_lines(), ["event initialize", "cancel", "error composition.cancelled", "event finalize", "finish cancelled"]);
}

#[test]
fn late_replies_after_cancellation_cannot_emit_an_outcome_reopen_shells_or_finalize_again() {
    let mut run = fresh();
    event(&mut run, Initialize, ok());
    let prepare = run.pending().ticket();
    run.cancel().unwrap();
    assert!(run.reply(prepare, Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(9) })).is_err());
    assert_eq!(run.shell_lock(), ShellLock::Locked);
    let finalize = run.pending().ticket();
    event(&mut run, Finalize, ok());
    assert_eq!(run.reply(finalize, Reply::Event(ok())), Err(ReplyError::Duplicate { generation: finalize.generation() }));
    assert_eq!(run.reply(prepare, Reply::Event(ok())), Err(ReplyError::CancelledWork { generation: prepare.generation() }));

    // Cancelled while `success` was being evaluated: its late reply cannot
    // request recovery, and `finalize` runs exactly once.
    let mut run = started();
    complete(&mut run, CompletionVerdict::Succeeded);
    let success = run.pending().ticket();
    run.cancel().unwrap();
    assert!(run.reply(success, Reply::Event(directed(retry()))).is_err());
    event(&mut run, Finalize, ok());
    assert_eq!(events(&run), [Initialize, Start, Success, Finalize]);
    assert_eq!(run.finish(), Some(Finish::Cancelled));
}

#[test]
fn a_cancel_during_cleanup_lets_that_cleanup_finish_without_a_second_pass() {
    let mut run = failed_run();
    event(&mut run, Failure, ok());
    let finalize = run.pending().ticket();
    run.cancel().unwrap();
    assert_eq!(run.pending().ticket(), finalize);
    event(&mut run, Finalize, directed(retry()));
    assert_eq!(events(&run), [Initialize, Start, Failure, Finalize]);
    assert_eq!(run.finish(), Some(Finish::Cancelled));
}

#[test]
fn the_run_alone_answers_shell_permission_from_initialize_to_proxy_adoption() {
    let mut run = fresh();
    let initialize = run.pending().ticket();
    assert_eq!(run.shell_permission(&initialize), Err(ShellRefusal::InitializeForbidsShell));
    event(&mut run, Initialize, ok());
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::NotCurrentEvent));
    assert_eq!(run.shell_permission(&initialize), Err(ShellRefusal::NotCurrentEvent));
    answer(&mut run, Reply::Prepared(Preparation::Ready { epoch: ApprovalEpoch(4) }));
    assert_eq!(run.shell_permission(&run.pending().ticket()), Ok(ApprovalEpoch(4)));

    event(&mut run, Start, ok());
    complete(&mut run, CompletionVerdict::Failed(host("agent.failed")));
    assert_eq!(run.shell_permission(&run.pending().ticket()), Ok(ApprovalEpoch(4)));
    event(&mut run, Failure, directed(proxy(Failure, "fallback.md")));
    answer(&mut run, Reply::Recovery(RecoveryReply::Adopted(AuthoredEvents::default())));
    assert_eq!(run.shell_lock(), ShellLock::Locked);
    event(&mut run, Initialize, ok());
    answer(&mut run, Reply::Prepared(Preparation::Failed(failure(FailureStage::Preflight, EntryReason::Initial, "audit.denied"))));
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::Locked), "early blocked");
    event(&mut run, Blocked, ok());
    assert_eq!(run.shell_permission(&run.pending().ticket()), Err(ShellRefusal::Locked), "early finalize");
    event(&mut run, Finalize, ok());

    let shell: Vec<_> = run.transcript_lines().into_iter().filter(|line| line.starts_with("shell") || line.starts_with("adopt")).collect();
    assert_eq!(shell, ["shell unlocked", "shell locked", "adopt document 2"]);
}

#[test]
fn the_failure_stage_and_launch_state_select_blocked_or_failure() {
    // Composition or preflight failure: blocked, with normal recovery rules.
    let mut run = preflight_blocked("schema.invalid");
    event(&mut run, Blocked, ok());
    event(&mut run, Finalize, ok());
    assert_eq!(events(&run), [Initialize, Blocked, Finalize]);
    assert_eq!(mode_of(&run, Blocked), [EvaluationMode::NormalRecovery]);

    let not_started = |stage| {
        let mut run = started();
        answer(&mut run, Reply::Operation(OperationReport::NotStarted(failure(stage, EntryReason::Initial, "setup.failed"))));
        run
    };
    // Internal setup failure after `start`, before launch: blocked.
    let mut run = not_started(FailureStage::Setup);
    event(&mut run, Blocked, ok());
    event(&mut run, Finalize, ok());
    assert_eq!(events(&run), [Initialize, Start, Blocked, Finalize]);
    assert_eq!(mode_of(&run, Blocked), [EvaluationMode::RefusalCleanup]);

    // Launch construction after `start`: failure.
    let mut run = not_started(FailureStage::LaunchConstruction);
    event(&mut run, Failure, directed(retry()));
    event(&mut run, Finalize, ok());
    assert_eq!(events(&run), [Initialize, Start, Failure, Finalize]);

    // A proxy refused before launch, even after `start` fired: blocked.
    let mut run = fresh();
    event(&mut run, Initialize, ok());
    ready(&mut run);
    event(&mut run, Start, directed(proxy(Start, "loop.md")));
    answer(&mut run, Reply::Recovery(RecoveryReply::Refused(failure(FailureStage::HandoffRefused, EntryReason::Initial, "proxy.cycle"))));
    event(&mut run, Blocked, ok());
    event(&mut run, Finalize, ok());
    assert_eq!(events(&run), [Initialize, Start, Blocked, Finalize]);
    assert_eq!(run.selected_error(), Some(&raised("proxy.cycle")));

    // A resume refused as incompatible after re-entering at `start`: failure.
    let mut run = failed_run();
    event(&mut run, Failure, directed(resume()));
    budget(&mut run, 2);
    assert_eq!(run.pending().kind(), &RequestKind::Prepare { entry: EntryReason::Resume });
    ready(&mut run);
    event(&mut run, Start, ok());
    answer(
        &mut run,
        Reply::Operation(OperationReport::NotStarted(failure(FailureStage::ResumeIncompatible, EntryReason::Resume, "resume.incompatible"))),
    );
    event(&mut run, Failure, ok());
    event(&mut run, Finalize, ok());
    assert_eq!(events(&run), [Initialize, Start, Failure, Start, Failure, Finalize]);
    assert_eq!(mode_of(&run, Failure), [EvaluationMode::NormalRecovery, EvaluationMode::RefusalCleanup]);

    // A hand-off refused after launch: `success` already holds the slot, so
    // no second terminal event fires; only the owed `finalize`.
    let mut run = started();
    complete(&mut run, CompletionVerdict::Succeeded);
    event(&mut run, Success, directed(proxy(Success, "next.md")));
    answer(&mut run, Reply::Recovery(RecoveryReply::Refused(failure(FailureStage::HandoffRefused, EntryReason::Initial, "proxy.unowned"))));
    event(&mut run, Finalize, ok());
    assert_eq!(events(&run), [Initialize, Start, Success, Finalize]);
    assert_eq!((run.terminal(), run.finish()), (Some(Success), Some(Finish::Failed)));
}

#[test]
fn an_explicit_error_downgrades_success_to_failure_whose_recovery_stays_eligible() {
    let mut run = started();
    complete(&mut run, CompletionVerdict::Succeeded);
    event(&mut run, Success, directed(error("review.rejected")));
    assert_eq!(run.terminal(), Some(Failure));
    event(&mut run, Failure, directed(retry()));
    assert!(matches!(run.pending().kind(), RequestKind::Recover(recovery) if recovery.kind == RecoveryKind::Retry && recovery.from == Failure));

    // `failure` has no downgrade: its explicit error ends in cleanup only.
    let mut run = failed_run();
    event(&mut run, Failure, directed(error("failure.explicit")));
    event(&mut run, Finalize, directed(retry()));
    assert_eq!(events(&run), [Initialize, Start, Failure, Finalize]);
    assert_eq!(run.finish(), Some(Finish::Failed));
}

/// Production routes an explicit `error(...)` in a preflight `blocked` through
/// the setup catch protocol: the slot is re-designated to `failure`, which
/// runs as refusal cleanup, so its recovery directive is not honored.
#[test]
fn an_explicit_error_in_preflight_blocked_is_caught_by_failure_as_cleanup() {
    let mut run = preflight_blocked("schema.invalid");
    event(&mut run, Blocked, directed(error("blocked.explicit")));
    assert_eq!((run.terminal(), run.selected_error()), (Some(Failure), Some(&explicit("blocked.explicit"))));
    event(&mut run, Failure, directed(retry()));
    assert_eq!(mode_of(&run, Failure), [EvaluationMode::RefusalCleanup]);
    event(&mut run, Finalize, ok());
    assert!(!run.transcript_lines().iter().any(|line| line.starts_with("recover")));
    assert_eq!(run.finish(), Some(Finish::Failed));
}

#[test]
fn skip_ends_the_run_from_initialize_without_finalize() {
    let mut run = fresh();
    event(&mut run, Initialize, directed(StackControl::Skip));
    assert_eq!(run.transcript_lines(), ["event initialize", "finish skipped"]);
}

#[test]
fn retry_or_resume_from_a_terminal_event_skips_that_attempts_finalize() {
    for (control, entry) in [(retry(), EntryReason::Retry), (resume(), EntryReason::Resume)] {
        let mut run = failed_run();
        event(&mut run, Failure, directed(control));
        budget(&mut run, 2);
        assert_eq!(run.pending().kind(), &RequestKind::Prepare { entry });
        ready(&mut run);
        event(&mut run, Start, ok());
        complete(&mut run, CompletionVerdict::Succeeded);
        event(&mut run, Success, ok());
        event(&mut run, Finalize, ok());
        event(&mut run, Loop, ok());
        assert_eq!(events(&run), [Initialize, Start, Failure, Start, Success, Finalize, Loop]);
        assert_eq!(run.finish(), Some(Finish::Succeeded));
    }

    // The host owns the budget; an exhausted one continues to `finalize`.
    let mut run = failed_run();
    event(&mut run, Failure, directed(retry()));
    budget(&mut run, 1);
    assert_eq!(mode_of(&run, Finalize), [EvaluationMode::NormalRecovery]);
}

#[test]
fn a_resume_without_a_session_ends_the_run_without_finalize() {
    let mut run = fresh();
    event(&mut run, Initialize, ok());
    ready(&mut run);
    event(&mut run, Start, directed(resume()));
    budget(&mut run, 2);
    assert_eq!(run.selected_error(), Some(&RunError::Aborted(LifecycleTransitionAbort::ResumeWithoutSession)));
    assert_eq!(events(&run), [Initialize, Start]);
    assert_eq!(run.finish(), Some(Finish::Failed));
}

#[test]
fn proxy_from_a_terminal_event_never_runs_the_source_finalize() {
    let mut run = failed_run();
    event(&mut run, Failure, directed(proxy(Failure, "fallback.md")));
    answer(&mut run, Reply::Recovery(RecoveryReply::Adopted(AuthoredEvents::default())));
    event(&mut run, Initialize, ok());
    ready(&mut run);
    event(&mut run, Start, ok());
    complete(&mut run, CompletionVerdict::Succeeded);
    event(&mut run, Success, ok());
    event(&mut run, Finalize, ok());
    event(&mut run, Loop, ok());
    assert_eq!(events(&run), [Initialize, Start, Failure, Initialize, Start, Success, Finalize, Loop]);
}

#[test]
fn a_recovery_chosen_in_finalize_is_never_followed_by_a_second_finalize_for_that_attempt() {
    let mut run = failed_run();
    event(&mut run, Failure, ok());
    event(&mut run, Finalize, directed(retry()));
    budget(&mut run, 2);
    ready(&mut run);
    event(&mut run, Start, ok());
    complete(&mut run, CompletionVerdict::Failed(host("agent.failed")));
    event(&mut run, Failure, ok());
    event(&mut run, Finalize, ok());
    assert_eq!(events(&run), [Initialize, Start, Failure, Finalize, Start, Failure, Finalize]);

    // An evaluation error inside `finalize` ends the run without finalizing again.
    let mut run = failed_run();
    event(&mut run, Failure, ok());
    event(&mut run, Finalize, evaluation_error("finalize.eval"));
    assert_eq!(events(&run), [Initialize, Start, Failure, Finalize]);
    assert_eq!((run.selected_error(), run.finish()), (Some(&raised("finalize.eval")), Some(Finish::Failed)));
}

/// Traces `retry-in-preflight-blocked` and `resume-in-preflight-blocked`: a
/// resume is refused for want of a session, the others as setup recovery.
#[test]
fn blocked_reached_from_preflight_cannot_recover() {
    for (control, refused) in [
        (retry(), RunError::SetupRecoveryUnsupported { event: Blocked, action: RecoveryKind::Retry }),
        (resume(), RunError::Aborted(LifecycleTransitionAbort::ResumeWithoutSession)),
        (proxy(Blocked, "other.md"), RunError::SetupRecoveryUnsupported { event: Blocked, action: RecoveryKind::Proxy }),
    ] {
        let mut run = preflight_blocked("audit.denied");
        event(&mut run, Blocked, directed(control));
        assert_eq!(run.selected_error(), Some(&refused));
        event(&mut run, Finalize, ok());
        assert!(!run.transcript_lines().iter().any(|line| line.starts_with("recover")));
        assert_eq!(run.finish(), Some(Finish::Failed));
    }
}

/// Production's setup-phase recovery rules (traces `retry-in-initialize`,
/// `resume-in-initialize`, `proxy-in-initialize`,
/// `retry-in-preflight-blocked`, `retry-in-start`).
#[test]
fn setup_phase_recovery_follows_production() {
    // A retry in `initialize` ends the run: no catch event, no `finalize`.
    let mut run = fresh();
    event(&mut run, Initialize, directed(retry()));
    let unsupported = RunError::SetupRecoveryUnsupported { event: Initialize, action: RecoveryKind::Retry };
    assert_eq!((run.selected_error(), run.finish()), (Some(&unsupported), Some(Finish::Failed)));
    assert_eq!(events(&run), [Initialize]);

    // A resume in `initialize` is refused for want of a session (trace
    // `resume-in-initialize`).
    let mut run = fresh();
    event(&mut run, Initialize, directed(resume()));
    assert_eq!(run.selected_error(), Some(&RunError::Aborted(LifecycleTransitionAbort::ResumeWithoutSession)));
    assert_eq!((events(&run), run.finish()), (vec![Initialize], Some(Finish::Failed)));

    // A hand-off in `initialize` adopts the target.
    let mut run = fresh();
    event(&mut run, Initialize, directed(proxy(Initialize, "target.md")));
    answer(&mut run, Reply::Recovery(RecoveryReply::Adopted(AuthoredEvents::default())));
    assert_eq!(run.pending().ticket().document(), 2);
    event(&mut run, Initialize, ok());

    // A preflight `blocked`'s `finalize` reads the error that blocked the run.
    let mut run = preflight_blocked("audit.denied");
    event(&mut run, Blocked, directed(retry()));
    assert_eq!(run.pending().event().and_then(|event| event.error.clone()), Some(raised("audit.denied")));

    // A retry in `start` before launch re-enters preparation; once spent, the
    // run continues to its operation.
    let mut run = fresh();
    event(&mut run, Initialize, ok());
    ready(&mut run);
    event(&mut run, Start, directed(retry()));
    budget(&mut run, 2);
    assert_eq!(run.pending().kind(), &RequestKind::Prepare { entry: EntryReason::Retry });
    ready(&mut run);
    event(&mut run, Start, directed(retry()));
    budget(&mut run, 2);
    assert!(matches!(run.pending().kind(), RequestKind::PerformOperation { .. }));
    assert_eq!(events(&run), [Initialize, Start, Start]);
}

#[test]
fn an_evaluation_failure_in_start_is_caught_by_failure_and_finalize_as_cleanup_only() {
    let mut run = fresh();
    event(&mut run, Initialize, ok());
    ready(&mut run);
    event(&mut run, Start, evaluation_error("start.eval"));
    event(&mut run, Failure, directed(retry()));
    event(&mut run, Finalize, evaluation_error("finalize.eval"));
    assert_eq!(events(&run), [Initialize, Start, Failure, Finalize]);
    assert_eq!(mode_of(&run, Failure), [EvaluationMode::FatalEvaluationCleanup]);
    // The later evaluation error replaces the earlier one.
    assert_eq!(run.selected_error(), Some(&raised("finalize.eval")));
    assert_eq!((run.terminal(), run.finish()), (Some(Failure), Some(Finish::Failed)));

    // An evaluation failure in `blocked` re-designates the slot to `failure`.
    let mut run = preflight_blocked("audit.denied");
    event(&mut run, Blocked, evaluation_error("blocked.eval"));
    assert_eq!(run.terminal(), Some(Failure));
    event(&mut run, Failure, ok());
    event(&mut run, Finalize, ok());
    assert_eq!(run.selected_error(), Some(&raised("blocked.eval")));
}

#[test]
fn the_iteration_gate_reenters_at_start_after_a_successful_unbroken_pass_only() {
    let mut run = started();
    complete(&mut run, CompletionVerdict::Succeeded);
    event(&mut run, Success, ok());
    event(&mut run, Finalize, ok());
    gate(&mut run, true);
    let ticket = run.pending().ticket();
    assert_eq!((run.pending().event().map(|event| event.signal), ticket.iteration(), ticket.attempt()), (Some(Start), 2, 1));
    assert_eq!(run.shell_permission(&ticket), Ok(ApprovalEpoch(1)));
    event(&mut run, Start, ok());
    complete(&mut run, CompletionVerdict::Succeeded);
    event(&mut run, Success, directed(StackControl::Break { reason: None, code: 0 }));
    event(&mut run, Finalize, ok());
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
    event(&mut run, Failure, ok());
    event(&mut run, Finalize, ok());
    assert_eq!(run.finish(), Some(Finish::Failed));
    assert!(!events(&run).contains(&Loop));
}

/// Two legs of one group, driven interleaved: each run's tickets, slot,
/// shell lock, and cancellation are its own.
#[test]
fn concurrent_runs_share_no_state() {
    let mut left = fresh();
    let mut right = run_of(AuthoredEvents::of([Cancelled]));
    event(&mut left, Initialize, ok());
    event(&mut right, Initialize, ok());
    ready(&mut left);
    assert_eq!(right.shell_lock(), ShellLock::Locked);
    assert_eq!(left.shell_lock(), ShellLock::Unlocked);

    rejected(&mut right, left.pending().ticket(), Reply::Event(ok()), ReplyError::ForeignRun);
    ready(&mut right);
    event(&mut left, Start, ok());
    event(&mut right, Start, ok());
    let right_work = right.pending().ticket();
    answer(&mut right, Reply::Operation(OperationReport::Started { session: SessionAvailability::Unavailable }));

    let left_before = left.snapshot();
    right.cancel().unwrap();
    assert_eq!(left.snapshot(), left_before);
    rejected(&mut left, right_work, Reply::Event(ok()), ReplyError::ForeignRun);

    complete(&mut left, CompletionVerdict::Failed(host("leg.failed")));
    event(&mut right, Cancelled, ok());
    event(&mut left, Failure, ok());
    event(&mut right, Finalize, ok());
    event(&mut left, Finalize, ok());

    assert_eq!((left.terminal(), left.finish()), (Some(Failure), Some(Finish::Failed)));
    assert_eq!((right.terminal(), right.finish()), (Some(Cancelled), Some(Finish::Cancelled)));
    assert_eq!(left.selected_error(), Some(&raised("leg.failed")));
    assert_ne!(left.id(), right.id());
}
