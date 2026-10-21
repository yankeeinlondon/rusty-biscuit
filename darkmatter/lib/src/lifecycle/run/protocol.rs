//! What a [`Run`](super::Run) asks for and what a host answers with.

use std::fmt;

use super::super::LifecycleSignal;
use super::super::context::LifecycleErrorInfo;
use super::super::executor::{LifecycleEventOutcome, StackControl};
use super::super::runtime::{EvaluationMode, LifecycleTransitionAbort};

/// A run's identity, unique within the process.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RunId(pub(super) u64);

/// The identity a reply must present: the run, document, attempt, and
/// iteration a request belongs to, and its generation.
///
/// Only the run issues tickets. Generations rise with every request, so a
/// ticket names exactly one request in the run's life.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ticket {
    pub(super) run: RunId,
    pub(super) document: u32,
    pub(super) attempt: u32,
    pub(super) iteration: u32,
    pub(super) generation: u64,
}

#[allow(missing_docs)]
impl Ticket {
    pub fn run(&self) -> RunId {
        self.run
    }

    /// One-based; a proxy hand-off adopts the next document.
    pub fn document(&self) -> u32 {
        self.document
    }

    /// One-based within the iteration; a retry or resume starts the next.
    pub fn attempt(&self) -> u32 {
        self.attempt
    }

    /// One-based; the iteration gate starts the next.
    pub fn iteration(&self) -> u32 {
        self.iteration
    }

    pub fn generation(&self) -> u64 {
        self.generation
    }
}

/// Why this attempt's preparation runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryReason {
    /// The document's first attempt in its iteration.
    Initial,
    /// An admitted `retry`.
    Retry,
    /// An admitted `resume` of the session.
    Resume,
}

/// The recovery a directive asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryKind {
    Retry,
    Resume,
    Proxy,
}

impl RecoveryKind {
    pub(super) fn of(control: &StackControl) -> Option<Self> {
        match control {
            StackControl::Retry { .. } => Some(Self::Retry),
            StackControl::Resume { .. } => Some(Self::Resume),
            StackControl::Proxy { .. } => Some(Self::Proxy),
            _ => None,
        }
    }

    #[allow(missing_docs)]
    pub fn name(self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::Resume => "resume",
            Self::Proxy => "proxy",
        }
    }
}

/// Whether the operation left a session a `resume` can continue.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionAvailability {
    Available,
    Unavailable,
}

/// The approval state a successful preparation established. A shell the run
/// permits must be checked against the host's approval authority at this
/// epoch; the run's permission is not itself an approval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ApprovalEpoch(pub u64);

/// The error a run has selected, which later events read as `err`.
#[derive(Clone, Debug, PartialEq)]
pub enum RunError {
    /// An event, a preparation, an operation, or a recovery reported it.
    Reported(LifecycleErrorInfo),
    /// A recovery directive in an event that cannot recover.
    SetupRecoveryUnsupported {
        event: LifecycleSignal,
        action: RecoveryKind,
    },
    /// The transition decision refused a recovery outright; the run ended
    /// without further events.
    Aborted(LifecycleTransitionAbort),
}

impl RunError {
    /// The host-classified error, when the failure is one.
    pub fn reported(&self) -> Option<&LifecycleErrorInfo> {
        match self {
            Self::Reported(error) => Some(error),
            _ => None,
        }
    }

    /// A short label: the diagnostic code, or the verb of an uncoded failure.
    pub fn label(&self) -> &str {
        match self {
            Self::Reported(error) => error.snapshot.as_ref().map_or(error.variant.as_str(), |snapshot| &snapshot.code),
            Self::SetupRecoveryUnsupported { .. } => "setup-recovery-unsupported",
            Self::Aborted(LifecycleTransitionAbort::EvaluationAfterFinalize) => "evaluation-after-finalize",
            Self::Aborted(LifecycleTransitionAbort::ResumeWithoutSession) => "resume-without-session",
            Self::Aborted(LifecycleTransitionAbort::ProxyBudgetExhausted) => "proxy-budget-exhausted",
            Self::Aborted(LifecycleTransitionAbort::DeferredExecutionUnsupported) => "defer-unsupported",
        }
    }
}

/// Evaluate one event's stack and reply with its outcome.
#[derive(Clone, Debug, PartialEq)]
pub struct EventRequest {
    pub signal: LifecycleSignal,
    /// How the event was entered; the run ignores recovery directives outside
    /// [`EvaluationMode::NormalRecovery`].
    pub mode: EvaluationMode,
    /// The error for the event's `err` global: the selected error, except
    /// that the `finalize` after a preflight `blocked`'s refused recovery
    /// reads the error that blocked the run. Whether the event may read it is
    /// the host's binding policy.
    pub error: Option<RunError>,
}

/// Decide a recovery directive's admission.
#[derive(Clone, Debug, PartialEq)]
pub struct RecoveryRequest {
    pub kind: RecoveryKind,
    /// The evaluated directive, with its arguments.
    pub control: StackControl,
    /// The event whose stack asked for it.
    pub from: LifecycleSignal,
    /// Whether the operation started in this attempt.
    pub launched: bool,
    pub session: SessionAvailability,
}

/// How a run ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Finish {
    Succeeded,
    Failed,
    /// `initialize` opted the document out.
    Skipped,
    Cancelled,
}

/// The one unit of work a run waits on.
#[derive(Clone, Debug, PartialEq)]
pub enum RequestKind {
    EvaluateEvent(EventRequest),
    /// Compose and preflight the document for this entry.
    Prepare {
        entry: EntryReason,
    },
    /// Perform the host operation (launch the agent, run the children).
    PerformOperation {
        entry: EntryReason,
    },
    /// The operation reported that it started; its completion is owed.
    AwaitCompletion,
    Recover(RecoveryRequest),
    /// The run is over; it accepts no reply.
    Done(Finish),
}

impl RequestKind {
    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::EvaluateEvent(_) => "evaluate-event",
            Self::Prepare { .. } => "prepare",
            Self::PerformOperation { .. } => "perform-operation",
            Self::AwaitCompletion => "await-completion",
            Self::Recover(_) => "recover",
            Self::Done(_) => "done",
        }
    }
}

/// An owned request: it holds no borrow of its run.
#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub(super) ticket: Ticket,
    pub(super) kind: RequestKind,
}

#[allow(missing_docs)]
impl Request {
    pub fn ticket(&self) -> Ticket {
        self.ticket
    }

    pub fn kind(&self) -> &RequestKind {
        &self.kind
    }

    /// The event this request evaluates, if it evaluates one.
    pub fn event(&self) -> Option<&EventRequest> {
        match &self.kind {
            RequestKind::EvaluateEvent(event) => Some(event),
            _ => None,
        }
    }
}

/// Where a host-reported failure happened; it selects `blocked` or
/// `failure` together with whether the operation launched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FailureStage {
    /// Composition, preflight, or a host preflight check.
    Preflight,
    /// Internal setup: materialization, or parsing a hand-off target.
    Setup,
    /// A proxy hand-off the host would not commit.
    HandoffRefused,
    /// A recovery the host would not admit.
    RecoveryRefused,
    /// Launch construction, the pre-spawn part of an attempt.
    LaunchConstruction,
    /// A resume whose launch identity no longer matches its session.
    ResumeIncompatible,
}

/// A failure the host reports, with where it happened and the entry of the
/// attempt it fails (which must be the pending one).
#[derive(Clone, Debug, PartialEq)]
pub struct HostFailure {
    pub stage: FailureStage,
    pub entry: EntryReason,
    pub error: LifecycleErrorInfo,
}

/// The answer to [`RequestKind::Prepare`].
#[derive(Clone, Debug, PartialEq)]
pub enum Preparation {
    /// Preflight passed; shells unlock at `start` under `epoch`.
    Ready {
        epoch: ApprovalEpoch,
    },
    /// Stage [`FailureStage::Preflight`] or [`FailureStage::Setup`].
    Failed(HostFailure),
}

/// What the operation produced. A host names a verdict, never an event.
#[derive(Clone, Debug, PartialEq)]
pub enum CompletionVerdict {
    Succeeded,
    Failed(LifecycleErrorInfo),
}

/// The answer to [`RequestKind::PerformOperation`] or
/// [`RequestKind::AwaitCompletion`].
#[derive(Clone, Debug, PartialEq)]
pub enum OperationReport {
    /// Not valid as the answer to [`RequestKind::AwaitCompletion`].
    Started {
        session: SessionAvailability,
    },
    Completed { verdict: CompletionVerdict, session: SessionAvailability },
    /// Stage [`FailureStage::Setup`], [`FailureStage::LaunchConstruction`], or
    /// [`FailureStage::ResumeIncompatible`].
    NotStarted(HostFailure),
}

/// The answer to [`RequestKind::Recover`].
#[derive(Clone, Debug, PartialEq)]
pub enum RecoveryReply {
    /// For a retry or resume: the host's budget ceiling for this control, the
    /// attempt number at which no further attempt is admitted. The run
    /// decides admission against it.
    Budget {
        ceiling: u32,
    },
    /// For a proxy: the host committed the hand-off; the adopted document's
    /// authored events.
    Adopted(AuthoredEvents),
    /// The host refused the recovery; the run continues with `blocked` or
    /// `failure` as cleanup.
    Refused(HostFailure),
    /// The run ends here with the failure selected and no further event: a
    /// committed hand-off whose target never bootstrapped (stage
    /// [`FailureStage::Setup`]), or a retry or resume the host cannot honor at
    /// all (stage [`FailureStage::RecoveryRefused`]).
    Aborted(HostFailure),
}

/// A host's answer to the pending request.
#[derive(Clone, Debug, PartialEq)]
pub enum Reply {
    /// The outcome of the stack the host ran.
    Event(LifecycleEventOutcome),
    /// The `loop` gate's outcome and the host's verdict on another iteration.
    Gate {
        outcome: LifecycleEventOutcome,
        iterate: bool,
    },
    Prepared(Preparation),
    Operation(OperationReport),
    Recovery(RecoveryReply),
}

impl Reply {
    pub(super) fn name(&self) -> &'static str {
        match self {
            Self::Event(_) | Self::Gate { .. } => "event",
            Self::Prepared(_) => "prepared",
            Self::Operation(_) => "operation",
            Self::Recovery(_) => "recovery",
        }
    }
}

/// Why a reply was rejected. A rejected reply changes nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplyError {
    /// The ticket belongs to another run.
    ForeignRun,
    /// The ticket's work was invalidated by [`Run::cancel`](super::Run::cancel).
    CancelledWork {
        generation: u64,
    },
    /// The request was already answered.
    Duplicate {
        generation: u64,
    },
    /// The ticket is not the pending request's.
    Stale {
        generation: u64,
        current: u64,
    },
    /// The run is over.
    Finished,
    /// The reply does not answer this kind of request.
    WrongKind {
        expected: &'static str,
        received: &'static str,
    },
    /// A failure names an entry other than the pending attempt's.
    EntryMismatch {
        expected: EntryReason,
        received: EntryReason,
    },
    /// A failure stage this request cannot report.
    StageNotApplicable {
        stage: FailureStage,
        request: &'static str,
    },
    /// `skip` outside `initialize`, or an iteration verdict outside `loop`.
    DirectiveNotPermitted {
        directive: &'static str,
        event: LifecycleSignal,
    },
    /// A budget for a proxy, or an adoption for a retry or resume.
    WrongRecovery {
        requested: RecoveryKind,
    },
}

/// Why [`Run::cancel`](super::Run::cancel) did nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CancelError {
    Finished,
    AlreadyCancelled,
}

/// Why [`Run::shell_permission`](super::Run::shell_permission) refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellRefusal {
    /// The ticket is not the event the run is evaluating now.
    NotCurrentEvent,
    InitializeForbidsShell,
    /// Shells unlock at `start`, after a successful preparation.
    Locked,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShellLock {
    Locked,
    Unlocked,
}

/// The events a document authors a non-empty stack for. A run requests
/// `cancelled` during cancellation only when the document authors it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AuthoredEvents(u8);

impl AuthoredEvents {
    fn bit(signal: LifecycleSignal) -> u8 {
        let index = LifecycleSignal::ALL.iter().position(|candidate| *candidate == signal).expect("ALL lists every signal");
        1 << index
    }

    pub fn of(signals: impl IntoIterator<Item = LifecycleSignal>) -> Self {
        Self(signals.into_iter().fold(0, |bits, signal| bits | Self::bit(signal)))
    }

    /// The events `config` has a non-empty stack for.
    pub fn from_config(config: &super::super::LifecycleConfig) -> Self {
        Self::of(LifecycleSignal::ALL.into_iter().filter(|signal| !config.stack(*signal).is_empty()))
    }

    pub fn contains(self, signal: LifecycleSignal) -> bool {
        self.0 & Self::bit(signal) != 0
    }
}

/// One line of a run's record of what it did.
#[derive(Clone, Debug, PartialEq)]
pub enum TranscriptEntry {
    Event { signal: LifecycleSignal, mode: EvaluationMode },
    Prepare(EntryReason),
    Operation,
    OperationStarted,
    Recover(RecoveryKind),
    Adopted { document: u32 },
    Iteration(u32),
    Shell(ShellLock),
    /// The selected error changed.
    Error(RunError),
    Cancel,
    Finish(Finish),
}

impl fmt::Display for TranscriptEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Event { signal, .. } => write!(f, "event {}", signal.property_name()),
            Self::Prepare(entry) => write!(f, "prepare {}", format!("{entry:?}").to_lowercase()),
            Self::Operation => f.write_str("operation"),
            Self::OperationStarted => f.write_str("operation started"),
            Self::Recover(kind) => write!(f, "recover {}", kind.name()),
            Self::Adopted { document } => write!(f, "adopt document {document}"),
            Self::Iteration(iteration) => write!(f, "iteration {iteration}"),
            Self::Shell(ShellLock::Locked) => f.write_str("shell locked"),
            Self::Shell(ShellLock::Unlocked) => f.write_str("shell unlocked"),
            Self::Error(error) => write!(f, "error {}", error.label()),
            Self::Cancel => f.write_str("cancel"),
            Self::Finish(finish) => write!(f, "finish {}", format!("{finish:?}").to_lowercase()),
        }
    }
}
