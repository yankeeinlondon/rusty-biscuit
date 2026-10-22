//! The request/reply driver for one lifecycle run.
//!
//! A [`Run`] is synchronous and inert between calls. [`Run::pending`] returns
//! the one unit of work it waits on as an owned [`Request`]: evaluate an
//! event, prepare the document, perform the host operation, or decide a
//! recovery. The host performs it and answers with typed facts through
//! [`Run::reply`]. Nothing else advances the run, and no callback reaches its
//! state.
//!
//! ```text
//! let mut run = Run::start(settings, authored);
//! loop {
//!     let request = run.pending();
//!     let reply = match request.kind() {
//!         RequestKind::EvaluateEvent(event) => Reply::Event(host.run_stack(event)),
//!         RequestKind::Prepare { entry } => Reply::Prepared(host.prepare(*entry)),
//!         RequestKind::PerformOperation { .. } | RequestKind::AwaitCompletion => Reply::Operation(host.perform()),
//!         RequestKind::Recover(recovery) => Reply::Recovery(host.recover(recovery)),
//!         RequestKind::Done(finish) => break *finish,
//!     };
//!     run.reply(request.ticket(), reply)?;
//! }
//! ```
//!
//! ## The protocol contract
//!
//! - A [`Ticket`] binds each request to its run, document, attempt,
//!   iteration, and a generation that rises with every request. A reply must
//!   present the pending request's ticket.
//! - A foreign, stale, duplicate, wrong-kind, or inconsistent reply returns a
//!   [`ReplyError`] and leaves the run exactly as it was.
//! - [`Run::cancel`] invalidates every generation issued so far before
//!   cleanup starts, so a late reply from cancelled work cannot emit an
//!   outcome, reopen shells, or finalize again. A cancel during cleanup lets
//!   that cleanup finish without a second pass. Stopping the cancelled work
//!   itself stays the host's job.
//! - Shell permission has one owner, [`Run::shell_permission`].
//!
//! The run keeps its accounting in the same [`RunAccounting`] the direct-call
//! [`LifecycleRunGuard`](super::run_guard::LifecycleRunGuard) keeps, routes
//! through [`decide_lifecycle_transition`] and [`LifecycleCatchProtocol`],
//! asks the [policy kernel](super::policy) which recoveries an event admits
//! and how a refused one is routed, and decides an admitted one over the
//! host's budget. Each run owns all of its state, so concurrent subjects each
//! drive their own run.

use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use super::LifecycleSignal;
use super::context::LifecycleErrorInfo;
use super::control::ControlDispatch;
use super::envelope::DiagnosticCatalog;
use super::executor::{LifecycleEventOutcome, StackControl};
use super::policy::{RecoveryVerdict, Refusal, RefusalRoute, SubjectKind, failure_event, recovery_verdict, refusal_route};
use super::run_guard::RunAccounting;
use super::runtime::{
    EvaluationMode, LifecycleCatchExecution, LifecycleCatchProtocol, LifecycleCatchState,
    LifecycleTransitionAbort, LifecycleTransitionDecision, LifecycleTransitionError, LifecycleTransitionInput,
    decide_lifecycle_transition, explicit_error,
};

mod protocol;

pub use protocol::{
    ApprovalEpoch, AuthoredEvents, CancelError, CompletionVerdict, EntryReason, EventRequest, FailureStage, Finish,
    HostFailure, OperationReport, Preparation, RecoveryKind, RecoveryReply, RecoveryRequest, Reply, ReplyError, Request,
    RequestKind, RunError, RunId, SessionAvailability, ShellLock, ShellRefusal, Ticket, TranscriptEntry,
};

static NEXT_RUN: AtomicU64 = AtomicU64::new(1);

/// What a run needs from its host before the first request.
#[derive(Clone)]
pub struct RunSettings {
    catalog: Arc<dyn DiagnosticCatalog + Send>,
    cancellation: Option<LifecycleErrorInfo>,
    subject: SubjectKind,
}

impl RunSettings {
    /// `catalog` classifies the error an explicit `error(...)` selects.
    pub fn new(catalog: Arc<dyn DiagnosticCatalog + Send>) -> Self {
        Self { catalog, cancellation: None, subject: SubjectKind::Document }
    }

    /// The kind of subject the run drives, a document unless set; it decides
    /// which recoveries an event admits and how a refused one is routed.
    #[must_use]
    pub fn with_subject(mut self, subject: SubjectKind) -> Self {
        self.subject = subject;
        self
    }

    /// The error a cancelled run selects, so `cancelled` and `finalize` read it
    /// as `err`. Without one, cancellation keeps the selected error as it was.
    #[must_use]
    pub fn with_cancellation_error(mut self, error: LifecycleErrorInfo) -> Self {
        self.cancellation = Some(error);
        self
    }
}

impl fmt::Debug for RunSettings {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RunSettings")
            .field("cancellation", &self.cancellation)
            .field("subject", &self.subject)
            .finish_non_exhaustive()
    }
}

/// A run's observable state, for comparing a run before and after a call.
#[derive(Clone, Debug, PartialEq)]
pub struct RunSnapshot {
    #[allow(missing_docs)]
    pub pending: Request,
    #[allow(missing_docs)]
    pub accounting: RunAccounting,
    #[allow(missing_docs)]
    pub error: Option<RunError>,
    #[allow(missing_docs)]
    pub transcript: Vec<TranscriptEntry>,
    /// Everything else a later reply depends on, in a fixed order: the
    /// answered generation, the cancellation floor, entry, epoch, session,
    /// and the broken and cancelled flags.
    pub progress: String,
}

/// One lifecycle run; see the [module documentation](self).
#[derive(Clone, Debug)]
pub struct Run {
    id: RunId,
    settings: RunSettings,
    authored: AuthoredEvents,
    accounting: RunAccounting,
    pending: Request,
    generation: u64,
    answered: Option<u64>,
    cancel_floor: Option<u64>,
    document: u32,
    attempt: u32,
    iteration: u32,
    entry: EntryReason,
    epoch: Option<ApprovalEpoch>,
    session: SessionAvailability,
    error: Option<RunError>,
    catch: Option<LifecycleCatchProtocol>,
    broken: bool,
    cancelled: bool,
    shell: ShellLock,
    transcript: Vec<TranscriptEntry>,
}

impl Run {
    /// A run of a document authoring `authored`, waiting on `initialize`.
    pub fn start(settings: RunSettings, authored: AuthoredEvents) -> Self {
        let id = RunId(NEXT_RUN.fetch_add(1, Ordering::Relaxed));
        let mut run = Self {
            id,
            settings,
            authored,
            accounting: RunAccounting::default(),
            // Replaced by the first request below.
            pending: Request {
                ticket: Ticket { run: id, document: 1, attempt: 1, iteration: 1, generation: 0 },
                kind: RequestKind::Done(Finish::Succeeded),
            },
            generation: 0,
            answered: None,
            cancel_floor: None,
            document: 1,
            attempt: 1,
            iteration: 1,
            entry: EntryReason::Initial,
            epoch: None,
            session: SessionAvailability::Unavailable,
            error: None,
            catch: None,
            broken: false,
            cancelled: false,
            shell: ShellLock::Locked,
            transcript: Vec::new(),
        };
        run.fire(LifecycleSignal::Initialize, EvaluationMode::NormalRecovery);
        run
    }

    #[allow(missing_docs)]
    pub fn id(&self) -> RunId {
        self.id
    }

    /// The one request the run accepts a reply for.
    pub fn pending(&self) -> Request {
        self.pending.clone()
    }

    /// How the run ended, once it has.
    pub fn finish(&self) -> Option<Finish> {
        match self.pending.kind {
            RequestKind::Done(finish) => Some(finish),
            _ => None,
        }
    }

    /// The error later events read as `err`.
    pub fn selected_error(&self) -> Option<&RunError> {
        self.error.as_ref()
    }

    /// The event holding the terminal slot, after any redesignation.
    pub fn terminal(&self) -> Option<LifecycleSignal> {
        self.accounting.terminal_signal()
    }

    #[allow(missing_docs)]
    pub fn accounting(&self) -> &RunAccounting {
        &self.accounting
    }

    #[allow(missing_docs)]
    pub fn shell_lock(&self) -> ShellLock {
        self.shell
    }

    /// The ordered record of events, preparations, operations, recoveries,
    /// shell-lock changes, selected-error changes, and the finish.
    pub fn transcript(&self) -> &[TranscriptEntry] {
        &self.transcript
    }

    #[allow(missing_docs)]
    pub fn transcript_lines(&self) -> Vec<String> {
        self.transcript.iter().map(ToString::to_string).collect()
    }

    #[allow(missing_docs)]
    pub fn snapshot(&self) -> RunSnapshot {
        RunSnapshot {
            pending: self.pending.clone(),
            accounting: self.accounting,
            error: self.error.clone(),
            transcript: self.transcript.clone(),
            progress: format!(
                "{:?} {:?} {:?} {:?} {:?} {} {}",
                self.answered,
                self.cancel_floor,
                self.entry,
                self.epoch,
                self.session,
                self.broken,
                self.cancelled
            ),
        }
    }

    /// Replies to whatever is pending.
    ///
    /// ## Errors
    ///
    /// As [`Self::reply`].
    pub fn answer(&mut self, reply: Reply) -> Result<(), ReplyError> {
        self.reply(self.pending.ticket, reply)
    }

    /// The only answer to "may this shell run now?".
    ///
    /// A host presents the ticket of the event it is evaluating. The epoch it
    /// gets back is the one its approval authority checks the command against.
    ///
    /// ## Errors
    ///
    /// [`ShellRefusal::NotCurrentEvent`] for any ticket but the pending
    /// event's, [`ShellRefusal::InitializeForbidsShell`] in `initialize`, and
    /// [`ShellRefusal::Locked`] before `start` of the current attempt.
    pub fn shell_permission(&self, ticket: &Ticket) -> Result<ApprovalEpoch, ShellRefusal> {
        let RequestKind::EvaluateEvent(request) = &self.pending.kind else {
            return Err(ShellRefusal::NotCurrentEvent);
        };
        if *ticket != self.pending.ticket {
            return Err(ShellRefusal::NotCurrentEvent);
        }
        if request.signal == LifecycleSignal::Initialize {
            return Err(ShellRefusal::InitializeForbidsShell);
        }
        match (self.accounting.start_emitted(), self.epoch) {
            (true, Some(epoch)) => Ok(epoch),
            _ => Err(ShellRefusal::Locked),
        }
    }

    /// Invalidates outstanding work, then runs cleanup in
    /// [`EvaluationMode::CancellationCleanup`]: `cancelled` when the document
    /// authors it and the terminal slot is free, then `finalize` unless it
    /// already ran.
    ///
    /// ## Errors
    ///
    /// [`CancelError`] when the run is over or already cancelled; nothing
    /// changes.
    pub fn cancel(&mut self) -> Result<(), CancelError> {
        if self.finish().is_some() {
            return Err(CancelError::Finished);
        }
        if self.cancelled {
            return Err(CancelError::AlreadyCancelled);
        }
        self.cancelled = true;
        self.transcript.push(TranscriptEntry::Cancel);
        let in_cleanup = self.catch.is_some()
            || self.pending.event().is_some_and(|event| {
                event.mode != EvaluationMode::NormalRecovery
                    || matches!(event.signal, LifecycleSignal::Finalize | LifecycleSignal::Cancelled)
            });
        if in_cleanup {
            return Ok(());
        }
        self.cancel_floor = Some(self.generation);
        if let Some(error) = self.settings.cancellation.clone() {
            self.set_error(RunError::Reported(error));
        }
        if self.accounting.finalize_emitted() {
            self.conclude();
        } else if self.authored.contains(LifecycleSignal::Cancelled) && !self.accounting.terminal_emitted() {
            self.fire(LifecycleSignal::Cancelled, EvaluationMode::CancellationCleanup);
        } else {
            self.fire(LifecycleSignal::Finalize, EvaluationMode::CancellationCleanup);
        }
        Ok(())
    }

    /// Applies `reply` to the pending request.
    ///
    /// ## Errors
    ///
    /// A [`ReplyError`] when `ticket` is not the pending request's or `reply`
    /// does not answer it; the run is left exactly as it was.
    pub fn reply(&mut self, ticket: Ticket, reply: Reply) -> Result<(), ReplyError> {
        self.check_ticket(ticket)?;
        self.check_reply(&reply)?;
        self.answered = Some(ticket.generation);
        match (self.pending.kind.clone(), reply) {
            (RequestKind::EvaluateEvent(request), Reply::Event(outcome)) => self.on_event(&request, outcome, false),
            (RequestKind::EvaluateEvent(request), Reply::Gate { outcome, iterate }) => {
                self.on_event(&request, outcome, iterate);
            }
            (RequestKind::Prepare { .. }, Reply::Prepared(Preparation::Ready { epoch })) => {
                self.epoch = Some(epoch);
                self.fire(LifecycleSignal::Start, EvaluationMode::NormalRecovery);
            }
            (RequestKind::Prepare { .. }, Reply::Prepared(Preparation::Failed(failure))) => {
                self.set_error(RunError::Reported(failure.error));
                self.fire(LifecycleSignal::Blocked, EvaluationMode::NormalRecovery);
            }
            (_, Reply::Operation(OperationReport::Started { session })) => {
                self.accounting.mark_provider_launched();
                self.session = session;
                self.transcript.push(TranscriptEntry::OperationStarted);
                self.issue(RequestKind::AwaitCompletion);
            }
            (_, Reply::Operation(OperationReport::Completed { verdict, session })) => {
                self.accounting.mark_provider_launched();
                self.session = session;
                match verdict {
                    CompletionVerdict::Succeeded => self.fire(LifecycleSignal::Success, EvaluationMode::NormalRecovery),
                    CompletionVerdict::Failed(error) => {
                        self.set_error(RunError::Reported(error));
                        self.fire(LifecycleSignal::Failure, EvaluationMode::NormalRecovery);
                    }
                }
            }
            (_, Reply::Operation(OperationReport::NotStarted(failure)))
            | (_, Reply::Recovery(RecoveryReply::Refused(failure))) => self.refuse(failure),
            (RequestKind::Recover(recovery), Reply::Recovery(RecoveryReply::Budget { ceiling })) => {
                self.admit(&recovery, ceiling);
            }
            (_, Reply::Recovery(RecoveryReply::Adopted(authored))) => self.adopt(authored),
            (_, Reply::Recovery(RecoveryReply::Aborted(failure))) => {
                self.set_error(RunError::Reported(failure.error));
                self.conclude();
            }
            (kind, reply) => unreachable!("check_reply admitted {} for {}", reply.name(), kind.name()),
        }
        Ok(())
    }

    fn check_ticket(&self, ticket: Ticket) -> Result<(), ReplyError> {
        if ticket.run != self.id {
            return Err(ReplyError::ForeignRun);
        }
        if self.cancel_floor.is_some_and(|floor| ticket.generation <= floor) {
            return Err(ReplyError::CancelledWork { generation: ticket.generation });
        }
        if self.answered == Some(ticket.generation) {
            return Err(ReplyError::Duplicate { generation: ticket.generation });
        }
        if ticket != self.pending.ticket {
            return Err(ReplyError::Stale { generation: ticket.generation, current: self.pending.ticket.generation });
        }
        if self.finish().is_some() {
            return Err(ReplyError::Finished);
        }
        Ok(())
    }

    fn check_reply(&self, reply: &Reply) -> Result<(), ReplyError> {
        let failure_fits = |failure: &HostFailure, allowed: &[FailureStage], request: &'static str| {
            if failure.entry != self.entry {
                return Err(ReplyError::EntryMismatch { expected: self.entry, received: failure.entry });
            }
            if !allowed.contains(&failure.stage) {
                return Err(ReplyError::StageNotApplicable { stage: failure.stage, request });
            }
            Ok(())
        };
        let event_fits = |request: &EventRequest, outcome: &LifecycleEventOutcome, gate: bool| {
            let misplaced = if gate && request.signal != LifecycleSignal::Loop {
                Some("iterate")
            } else if matches!(outcome.control, Some(StackControl::Skip)) && request.signal != LifecycleSignal::Initialize {
                Some("skip")
            } else {
                None
            };
            match misplaced {
                Some(directive) => Err(ReplyError::DirectiveNotPermitted { directive, event: request.signal }),
                None => Ok(()),
            }
        };
        match (&self.pending.kind, reply) {
            (RequestKind::EvaluateEvent(request), Reply::Event(outcome)) => event_fits(request, outcome, false),
            (RequestKind::EvaluateEvent(request), Reply::Gate { outcome, .. }) => event_fits(request, outcome, true),
            (RequestKind::Prepare { .. }, Reply::Prepared(Preparation::Failed(failure))) => {
                failure_fits(failure, &[FailureStage::Preflight, FailureStage::Setup], "prepare")
            }
            (RequestKind::Prepare { .. }, Reply::Prepared(Preparation::Ready { .. })) => Ok(()),
            (RequestKind::PerformOperation { .. }, Reply::Operation(OperationReport::NotStarted(failure))) => failure_fits(
                failure,
                &[FailureStage::Setup, FailureStage::LaunchConstruction, FailureStage::ResumeIncompatible],
                "perform-operation",
            ),
            (RequestKind::PerformOperation { .. }, Reply::Operation(_))
            | (RequestKind::AwaitCompletion, Reply::Operation(OperationReport::Completed { .. })) => Ok(()),
            (RequestKind::Recover(recovery), Reply::Recovery(answer)) => match answer {
                RecoveryReply::Budget { .. } if recovery.kind == RecoveryKind::Proxy => {
                    Err(ReplyError::WrongRecovery { requested: recovery.kind })
                }
                RecoveryReply::Adopted(_) if recovery.kind != RecoveryKind::Proxy => {
                    Err(ReplyError::WrongRecovery { requested: recovery.kind })
                }
                RecoveryReply::Refused(failure) => failure_fits(
                    failure,
                    &[FailureStage::HandoffRefused, FailureStage::RecoveryRefused, FailureStage::Setup],
                    "recover",
                ),
                RecoveryReply::Aborted(failure) => {
                    let stage =
                        if recovery.kind == RecoveryKind::Proxy { FailureStage::Setup } else { FailureStage::RecoveryRefused };
                    failure_fits(failure, &[stage], "recover")
                }
                RecoveryReply::Budget { .. } | RecoveryReply::Adopted(_) => Ok(()),
            },
            (kind, reply) => Err(ReplyError::WrongKind { expected: kind.name(), received: reply.name() }),
        }
    }

    fn issue(&mut self, kind: RequestKind) {
        self.generation += 1;
        let entry = match &kind {
            RequestKind::EvaluateEvent(event) => Some(TranscriptEntry::Event { signal: event.signal, mode: event.mode }),
            RequestKind::Prepare { entry } => Some(TranscriptEntry::Prepare(*entry)),
            RequestKind::PerformOperation { .. } => Some(TranscriptEntry::Operation),
            RequestKind::AwaitCompletion => None,
            RequestKind::Recover(recovery) => Some(TranscriptEntry::Recover(recovery.kind)),
            RequestKind::Done(finish) => Some(TranscriptEntry::Finish(*finish)),
        };
        self.transcript.extend(entry);
        let ticket = Ticket {
            run: self.id,
            document: self.document,
            attempt: self.attempt,
            iteration: self.iteration,
            generation: self.generation,
        };
        self.pending = Request { ticket, kind };
    }

    /// Records `signal` in the accounting and requests its evaluation.
    ///
    /// `finalize` with no terminal event closes the slot first: a run that ends
    /// before any terminal event (a `break` or a cancellation before the
    /// operation) still finalizes once.
    fn fire(&mut self, signal: LifecycleSignal, mode: EvaluationMode) {
        self.fire_reading(signal, mode, self.error.clone());
    }

    /// [`Self::fire`] with `error` as the event's `err`.
    fn fire_reading(&mut self, signal: LifecycleSignal, mode: EvaluationMode, error: Option<RunError>) {
        if signal == LifecycleSignal::Finalize && !self.accounting.terminal_emitted() {
            self.accounting.defuse();
        }
        let recorded = self.accounting.record_event_emission(signal);
        debug_assert!(recorded, "the run fires {signal:?} only while its accounting admits it");
        self.evaluate_reading(signal, mode, error);
    }

    fn evaluate(&mut self, signal: LifecycleSignal, mode: EvaluationMode) {
        self.evaluate_reading(signal, mode, self.error.clone());
    }

    /// Requests `signal` with `error` as its `err`, which differs from the
    /// selected error only for the `finalize` after a refused recovery.
    fn evaluate_reading(&mut self, signal: LifecycleSignal, mode: EvaluationMode, error: Option<RunError>) {
        let lock = if self.accounting.start_emitted() { ShellLock::Unlocked } else { ShellLock::Locked };
        self.sync_shell(lock);
        self.issue(RequestKind::EvaluateEvent(EventRequest { signal, mode, error }));
    }

    fn sync_shell(&mut self, lock: ShellLock) {
        if self.shell != lock {
            self.shell = lock;
            self.transcript.push(TranscriptEntry::Shell(lock));
        }
    }

    fn set_error(&mut self, error: RunError) {
        if self.error.as_ref() != Some(&error) {
            self.transcript.push(TranscriptEntry::Error(error.clone()));
        }
        self.error = Some(error);
    }

    fn transition_input<'o>(
        &self,
        event: LifecycleSignal,
        outcome: &'o LifecycleEventOutcome,
        control_budget: u32,
    ) -> LifecycleTransitionInput<'o> {
        LifecycleTransitionInput {
            event,
            terminal_slot: self.accounting.terminal_signal(),
            provider_launched: self.accounting.provider_launched(),
            has_prior_error: self.error.is_some(),
            outcome,
            has_session: self.session == SessionAvailability::Available,
            attempt: self.attempt,
            control_budget,
            // Every document the run has adopted is in the chain, the origin
            // (document 1) included.
            proxy_hops_used: self.document as usize,
            // Path identity is the host's; it refuses a repeated target.
            proxy_target_seen: false,
            finalize_emitted: self.accounting.finalize_emitted(),
        }
    }

    /// The finish the transition decision gives a run ending now.
    fn verdict(&self) -> Finish {
        if self.cancelled {
            return Finish::Cancelled;
        }
        let outcome = LifecycleEventOutcome::default();
        let mut input = self.transition_input(LifecycleSignal::Finalize, &outcome, 0);
        input.finalize_emitted = true;
        match decide_lifecycle_transition(&input) {
            LifecycleTransitionDecision::TerminalSuccess => Finish::Succeeded,
            _ => Finish::Failed,
        }
    }

    fn conclude(&mut self) {
        let finish = self.verdict();
        self.issue(RequestKind::Done(finish));
    }

    fn finish_or_finalize(&mut self, mode: EvaluationMode) {
        if self.accounting.finalize_emitted() {
            self.conclude();
        } else {
            self.fire(LifecycleSignal::Finalize, mode);
        }
    }

    fn on_event(&mut self, request: &EventRequest, outcome: LifecycleEventOutcome, iterate: bool) {
        let signal = request.signal;
        if let Some(mut catch) = self.catch.take() {
            let recorded = catch.record(signal, outcome);
            debug_assert!(recorded, "the catch protocol requested {signal:?}");
            return self.drive_catch(catch);
        }
        if self.cancelled {
            return self.finish_or_finalize(EvaluationMode::CancellationCleanup);
        }
        let decision = decide_lifecycle_transition(&self.transition_input(signal, &outcome, 0));
        let caught = match &decision {
            LifecycleTransitionDecision::CatchFailure { error } => *error != LifecycleTransitionError::Prior,
            LifecycleTransitionDecision::Finalize { error } => *error == Some(LifecycleTransitionError::Evaluation),
            LifecycleTransitionDecision::Abort(abort) => *abort == LifecycleTransitionAbort::EvaluationAfterFinalize,
            _ => false,
        };
        if caught {
            return self.start_catch(signal, outcome);
        }
        if request.mode != EvaluationMode::NormalRecovery {
            return self.finish_or_finalize(request.mode);
        }
        if let Some(control) = &outcome.control {
            match recovery_verdict(self.settings.subject, signal, control) {
                Some(RecoveryVerdict::Admit) => return self.request_recovery(signal, control.clone(), decision),
                Some(RecoveryVerdict::Refuse(refusal)) => return self.refuse_recovery(refusal),
                Some(RecoveryVerdict::Ignore) | None => {}
            }
        }
        match &outcome.control {
            Some(StackControl::Skip) => return self.issue(RequestKind::Done(Finish::Skipped)),
            Some(StackControl::Break { .. }) => {
                self.broken = true;
                if matches!(signal, LifecycleSignal::Initialize | LifecycleSignal::Start) {
                    return self.finish_or_finalize(EvaluationMode::NormalRecovery);
                }
            }
            Some(StackControl::Error { reason }) if signal == LifecycleSignal::Success => {
                return self.downgrade(reason.as_deref());
            }
            Some(StackControl::Error { reason }) => {
                if let LifecycleTransitionDecision::TerminalFailure { error: LifecycleTransitionError::ExplicitControl } =
                    decision
                {
                    let error = explicit_error(&*self.settings.catalog, signal, reason.as_deref());
                    self.set_error(RunError::Reported(error));
                    return self.finish_or_finalize(EvaluationMode::RefusalCleanup);
                }
            }
            _ => {}
        }
        self.advance(signal, decision, iterate);
    }

    /// The next step after `signal` completed without a catch or recovery.
    fn advance(&mut self, signal: LifecycleSignal, decision: LifecycleTransitionDecision, iterate: bool) {
        match decision {
            LifecycleTransitionDecision::Continue => match signal {
                LifecycleSignal::Initialize => self.issue(RequestKind::Prepare { entry: self.entry }),
                LifecycleSignal::Start => self.issue(RequestKind::PerformOperation { entry: self.entry }),
                _ if iterate => self.iterate(),
                _ => self.conclude(),
            },
            LifecycleTransitionDecision::Finalize { .. } => {
                self.fire(LifecycleSignal::Finalize, EvaluationMode::NormalRecovery);
            }
            // A pass that ended in success may iterate; a broken one may not.
            LifecycleTransitionDecision::TerminalSuccess if signal == LifecycleSignal::Finalize && !self.broken => {
                self.fire(LifecycleSignal::Loop, EvaluationMode::NormalRecovery);
            }
            _ => self.conclude(),
        }
    }

    /// An explicit `error(...)` in `success` downgrades the run: the slot is
    /// re-designated and `failure` runs with recovery still eligible.
    fn downgrade(&mut self, reason: Option<&str>) {
        let error = explicit_error(&*self.settings.catalog, LifecycleSignal::Success, reason);
        self.set_error(RunError::Reported(error));
        self.accounting.redesignate_terminal_to_failure();
        self.evaluate(LifecycleSignal::Failure, EvaluationMode::NormalRecovery);
    }

    fn start_catch(&mut self, origin: LifecycleSignal, outcome: LifecycleEventOutcome) {
        let state = LifecycleCatchState {
            terminal_slot: self.accounting.terminal_signal(),
            finalize_emitted: self.accounting.finalize_emitted(),
        };
        let prior = self.error.as_ref().and_then(RunError::reported).cloned();
        let catch = LifecycleCatchProtocol::with_catalog(&*self.settings.catalog, origin, state, prior, outcome);
        self.drive_catch(catch);
    }

    fn drive_catch(&mut self, mut catch: LifecycleCatchProtocol) {
        while let Some(step) = catch.next_step().cloned() {
            if let Some(error) = step.error {
                self.set_error(RunError::Reported(error));
            }
            let runs = match step.execution {
                LifecycleCatchExecution::Record => self.accounting.record_event_emission(step.signal),
                LifecycleCatchExecution::RedesignateBlockedAsFailure => {
                    self.accounting.redesignate_terminal_to_failure();
                    true
                }
            };
            if runs {
                self.catch = Some(catch);
                return self.evaluate(step.signal, step.mode);
            }
            // The accounting refused the event, so it ran nothing.
            catch.record(step.signal, LifecycleEventOutcome::default());
        }
        let result = catch.finish().expect("no catch step is pending");
        if let Some(error) = result.evaluation_error {
            self.set_error(RunError::Reported(error));
        }
        self.conclude();
    }

    /// Asks the host to decide an admitted recovery; a retry or resume is
    /// decided against the budget it answers with, and only a hand-off past
    /// the proxy budget is refused before asking.
    fn request_recovery(&mut self, from: LifecycleSignal, control: StackControl, decision: LifecycleTransitionDecision) {
        let kind = RecoveryKind::of(&control).expect("only recovery controls are admitted");
        if kind == RecoveryKind::Proxy
            && let LifecycleTransitionDecision::Abort(abort) = decision
        {
            return self.refuse_recovery(Refusal::Abort(abort));
        }
        let recovery = RecoveryRequest {
            kind,
            control,
            from,
            launched: self.accounting.provider_launched(),
            session: self.session,
        };
        self.issue(RequestKind::Recover(recovery));
    }

    fn admit(&mut self, recovery: &RecoveryRequest, ceiling: u32) {
        let outcome = LifecycleEventOutcome { control: Some(recovery.control.clone()), ..LifecycleEventOutcome::default() };
        let decision = decide_lifecycle_transition(&self.transition_input(recovery.from, &outcome, ceiling));
        match decision {
            LifecycleTransitionDecision::Reenter(ControlDispatch::Retry { .. }) => self.reenter(EntryReason::Retry),
            LifecycleTransitionDecision::Reenter(ControlDispatch::Resume { .. }) => self.reenter(EntryReason::Resume),
            LifecycleTransitionDecision::Abort(abort) => self.refuse_recovery(Refusal::Abort(abort)),
            exhausted => self.advance(recovery.from, exhausted, false),
        }
    }

    /// Selects `refusal` as the run's error and runs the cleanup its route
    /// asks for, in [`EvaluationMode::RefusalCleanup`].
    fn refuse_recovery(&mut self, refusal: Refusal) {
        let prior = self.error.clone();
        self.set_error(match refusal {
            Refusal::SetupRecoveryUnsupported { event, action } => RunError::SetupRecoveryUnsupported { event, action },
            Refusal::Abort(abort) => RunError::Aborted(abort),
        });
        let owed = self.accounting.terminal_emitted() && !self.accounting.finalize_emitted();
        match refusal_route(self.settings.subject, &refusal) {
            RefusalRoute::FinalizeIfOwed if owed => {
                self.fire_reading(LifecycleSignal::Finalize, EvaluationMode::RefusalCleanup, prior);
            }
            RefusalRoute::FinalizeIfOwedReadingRefusal if owed => {
                self.fire(LifecycleSignal::Finalize, EvaluationMode::RefusalCleanup);
            }
            RefusalRoute::FinalizeIfOwed | RefusalRoute::FinalizeIfOwedReadingRefusal => self.conclude(),
            RefusalRoute::FailureThenFinalize => {
                if !self.accounting.terminal_emitted() {
                    self.fire(LifecycleSignal::Failure, EvaluationMode::RefusalCleanup);
                } else if self.accounting.redesignate_terminal_to_failure() {
                    self.evaluate(LifecycleSignal::Failure, EvaluationMode::RefusalCleanup);
                } else {
                    self.finish_or_finalize(EvaluationMode::RefusalCleanup);
                }
            }
        }
    }

    /// Selects the event for a host failure from its stage and launch state,
    /// never firing a second terminal event.
    fn refuse(&mut self, failure: HostFailure) {
        let signal = failure_event(failure.stage, self.accounting.provider_launched());
        self.set_error(RunError::Reported(failure.error));
        if self.accounting.terminal_emitted() {
            self.finish_or_finalize(EvaluationMode::RefusalCleanup);
        } else {
            self.fire(signal, EvaluationMode::RefusalCleanup);
        }
    }

    fn reset_attempt(&mut self) {
        self.error = None;
        self.broken = false;
    }

    fn reenter(&mut self, entry: EntryReason) {
        self.attempt += 1;
        self.entry = entry;
        if entry == EntryReason::Retry {
            self.session = SessionAvailability::Unavailable;
        }
        self.accounting.reset_for_next_iteration();
        self.reset_attempt();
        self.sync_shell(ShellLock::Locked);
        self.issue(RequestKind::Prepare { entry });
    }

    fn adopt(&mut self, authored: AuthoredEvents) {
        self.authored = authored;
        self.document += 1;
        self.iteration = 1;
        self.attempt = 1;
        self.entry = EntryReason::Initial;
        self.epoch = None;
        self.session = SessionAvailability::Unavailable;
        self.accounting.reset_for_proxy();
        self.reset_attempt();
        self.sync_shell(ShellLock::Locked);
        self.transcript.push(TranscriptEntry::Adopted { document: self.document });
        self.fire(LifecycleSignal::Initialize, EvaluationMode::NormalRecovery);
    }

    fn iterate(&mut self) {
        self.iteration += 1;
        self.attempt = 1;
        self.entry = EntryReason::Initial;
        self.accounting.reset_for_next_iteration();
        self.reset_attempt();
        self.transcript.push(TranscriptEntry::Iteration(self.iteration));
        self.fire(LifecycleSignal::Start, EvaluationMode::NormalRecovery);
    }
}
