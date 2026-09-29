//! The execution-owned steering controller.
//!
//! One controller exists per managed agent execution, and it is the only path
//! to that execution's steering I/O. Every request — a manual one routed by
//! the local Rendezvous daemon, or an automatic one raised inside the owner —
//! enters one bounded queue, and a single worker submits them one at a time,
//! so provider mutations never interleave. The provider's event reader keeps
//! draining output independently: a [`SteeringExecutor`] submits and awaits
//! acceptance, it never becomes the stream reader.
//!
//! Immediately before submitting, the worker revalidates the request against
//! the owner's current binding ([`ManagedTarget::revalidate`]) and the
//! session's current eligibility. A changed target or a changed operation is
//! reported, never retargeted, and a non-interrupting request never turns
//! into an interruption.
//!
//! Bounds are named constants with tests, not configuration:
//! [`MAX_PENDING_PER_EXECUTION`] requests queued or in flight, at most one
//! automatic, [`MANUAL_ACCEPTANCE_DEADLINE`] and
//! [`AUTOMATIC_ACCEPTANCE_DEADLINE`] from dispatch to acceptance (queue wait
//! included), plus [`CONSENTED_CANCELLATION_DEADLINE`] before the manual
//! deadline for a consented interruption. A request that expires unsent is
//! discarded as [`SendOutcome::Busy`]; one that may have been submitted is
//! [`SendOutcome::Unknown`] and is never retried. A request ID is accepted
//! once per execution, so neither a reconnection nor a repeated correlation
//! ID can replay a message.
//!
//! The controller does not depend on the daemon. Automatic help calls
//! [`SteeringController::submit`] directly, so it keeps working when
//! Rendezvous is absent.
//!
//! Topic: `claudine/docs/topics/steering-routing.md`.

use std::collections::HashSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;

use super::audit::{AuditContext, AuditFailure, DeliveryReport, PendingAudit, SteeringAuditLog};
use super::contract::{SendOutcome, SteeringOrigin, SteeringRequest, SteeringResult};
use super::eligibility::{AutomaticEligibility, AvailabilitySummary, Eligibility, Route, SessionFacts};
use super::identity::{
    ConversationGeneration, ExecutionId, ManagedTarget, OpportunityId, ProcessStartIdentity, SteeringTargetId,
};
use super::vocabulary::{ExecutionState, HostOs, LaunchMode, LaunchOrigin, OperationIntent};
use crate::provider_id::Provider;
use crate::secrets::RedactedText;

#[cfg(test)]
mod tests;

/// Requests one execution holds queued or in flight. More are refused as busy;
/// an accepted request is never evicted to make room.
pub const MAX_PENDING_PER_EXECUTION: usize = 16;
/// Dispatch-to-acceptance deadline for a manual request.
pub const MANUAL_ACCEPTANCE_DEADLINE: Duration = Duration::from_secs(10);
/// Dispatch-to-acceptance deadline for an automatic request.
pub const AUTOMATIC_ACCEPTANCE_DEADLINE: Duration = Duration::from_secs(2);
/// Time a consented interruption gets to establish cancellation; the manual
/// acceptance deadline follows it.
pub const CONSENTED_CANCELLATION_DEADLINE: Duration = Duration::from_secs(10);
/// How long the worker keeps waiting, after the caller's deadline, for a
/// submission's own result before it abandons it. The late result is only
/// logged; this bounds how long one wedged submission can hold the queue.
pub const LATE_RESULT_WINDOW: Duration = Duration::from_secs(30);

/// Why a session with no mapped launch profile cannot be steered.
pub const UNMAPPED_PROFILE_REASON: &str =
    "this launch is not mapped to a researched steering launch profile, so no steering route can be verified";

/// Deadlines one delivery must meet, measured from dispatch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeliveryDeadlines {
    /// When cancellation must be established; only for a consented
    /// interruption.
    pub cancellation: Option<Instant>,
    /// When the provider must have accepted the message.
    pub acceptance: Instant,
}

impl DeliveryDeadlines {
    /// The deadlines for `request` dispatched at `dispatched`.
    pub fn for_request(request: &SteeringRequest, dispatched: Instant) -> Self {
        match request.origin {
            SteeringOrigin::Automatic => {
                Self { cancellation: None, acceptance: dispatched + AUTOMATIC_ACCEPTANCE_DEADLINE }
            }
            SteeringOrigin::Manual if request.may_interrupt() => {
                let cancellation = dispatched + CONSENTED_CANCELLATION_DEADLINE;
                Self { cancellation: Some(cancellation), acceptance: cancellation + MANUAL_ACCEPTANCE_DEADLINE }
            }
            SteeringOrigin::Manual => Self { cancellation: None, acceptance: dispatched + MANUAL_ACCEPTANCE_DEADLINE },
        }
    }
}

/// A submission in progress.
pub type DeliveryFuture = Pin<Box<dyn Future<Output = DeliveryReport> + Send>>;

/// The provider-specific half of steering: submits one request over one
/// route and reports what the provider established.
///
/// The controller calls it for one request at a time. It should resolve by
/// `deadlines.acceptance`; a report after that reaches only the audit log as a
/// late result, and one still pending [`LATE_RESULT_WINDOW`] later is
/// abandoned.
pub trait SteeringExecutor: Send + Sync {
    fn deliver(&self, request: SteeringRequest, route: Route, deadlines: DeliveryDeadlines) -> DeliveryFuture;
}

/// Facts about a managed execution that do not change while it runs. The
/// provider version may be unknown at spawn and recorded once the managed
/// launch reads it ([`SteeringController::set_provider_version`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionFacts {
    pub provider: Provider,
    /// Researched launch-profile ID this launch runs under, when mapped.
    pub profile_id: Option<String>,
    pub os: HostOs,
    pub launch_mode: LaunchMode,
    /// Exact provider version, when established.
    pub provider_version: Option<String>,
    /// Working directory in portable form.
    pub cwd: Option<String>,
    /// Human-readable session name, when the provider exposes one.
    pub name: Option<String>,
}

/// The owner's current view of its execution, published to listeners.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControllerSnapshot {
    pub target: ManagedTarget,
    pub state: ExecutionState,
    /// The provider child's own process identity, once spawned.
    pub provider_process: Option<ProcessStartIdentity>,
    pub facts: ExecutionFacts,
    /// Manual availability as this owner's build evaluates it.
    pub availability: AvailabilitySummary,
}

/// Session eligibility for the controller; [`super::eligibility::evaluate`]
/// in production.
pub type EligibilityFn = Arc<dyn Fn(&SessionFacts<'_>) -> Eligibility + Send + Sync>;

/// Everything needed to start a controller.
pub struct ControllerConfig {
    pub execution: ExecutionId,
    pub wrapper: ProcessStartIdentity,
    pub facts: ExecutionFacts,
    pub initial_state: ExecutionState,
    pub eligibility: EligibilityFn,
    pub audit: SteeringAuditLog,
}

impl ControllerConfig {
    /// A fresh execution ID, a working initial state, the build's
    /// eligibility, and the per-user audit log.
    pub fn new(wrapper: ProcessStartIdentity, facts: ExecutionFacts) -> Self {
        Self {
            execution: ExecutionId::random(),
            wrapper,
            facts,
            initial_state: ExecutionState::Working,
            eligibility: Arc::new(super::eligibility::evaluate),
            audit: SteeringAuditLog::default_location(),
        }
    }
}

/// One request handed to the controller.
#[derive(Debug, Clone)]
pub struct Submission {
    pub request: SteeringRequest,
    /// The binding the requester selected. `None` means "whatever the owner
    /// currently runs", which only in-owner automatic help may use.
    pub expected: Option<ManagedTarget>,
    /// The automatic warning opportunity this request serves.
    pub opportunity: Option<OpportunityId>,
}

/// What the controller established for one request.
#[derive(Debug, Clone)]
pub struct ControllerReply {
    pub result: SteeringResult,
    /// Why the request was not confirmed, redacted against its message.
    pub detail: Option<RedactedText>,
    pub audit_failures: Vec<AuditFailure>,
}

#[derive(Debug, Default)]
struct Admission {
    pending: usize,
    automatic_pending: bool,
    seen: HashSet<super::identity::RequestId>,
    closed: bool,
}

struct Shared {
    snapshot: watch::Sender<ControllerSnapshot>,
    eligibility: EligibilityFn,
    audit: SteeringAuditLog,
    admission: Mutex<Admission>,
    shutdown: watch::Sender<bool>,
}

struct Job {
    submission: Submission,
    deadlines: DeliveryDeadlines,
    audit: PendingAudit,
    reply: oneshot::Sender<ControllerReply>,
}

/// A cloneable handle to one execution's controller. The worker stops when
/// [`SteeringController::shutdown`] is called or every handle is dropped.
#[derive(Clone)]
pub struct SteeringController {
    shared: Arc<Shared>,
    queue: mpsc::UnboundedSender<Job>,
}

impl std::fmt::Debug for SteeringController {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SteeringController").field("target", &self.snapshot().target).finish_non_exhaustive()
    }
}

impl SteeringController {
    /// Starts the controller's worker on the current Tokio runtime.
    pub fn spawn(config: ControllerConfig, executor: Arc<dyn SteeringExecutor>) -> Self {
        let target = ManagedTarget {
            execution: config.execution,
            wrapper: config.wrapper,
            generation: ConversationGeneration(0),
            conversation: None,
        };
        let mut snapshot = ControllerSnapshot {
            target,
            state: config.initial_state,
            provider_process: None,
            facts: config.facts,
            availability: AvailabilitySummary::unavailable(UNMAPPED_PROFILE_REASON),
        };
        snapshot.availability = availability(&config.eligibility, &snapshot);
        let (shutdown, _) = watch::channel(false);
        let shared = Arc::new(Shared {
            snapshot: watch::channel(snapshot).0,
            eligibility: config.eligibility,
            audit: config.audit,
            admission: Mutex::new(Admission::default()),
            shutdown,
        });
        let (queue, jobs) = mpsc::unbounded_channel();
        tokio::spawn(run_worker(Arc::clone(&shared), executor, jobs));
        Self { shared, queue }
    }

    /// The current binding, state, and availability.
    pub fn snapshot(&self) -> ControllerSnapshot {
        self.shared.snapshot.borrow().clone()
    }

    /// Observes every snapshot change, for publishing the registration.
    pub fn subscribe(&self) -> watch::Receiver<ControllerSnapshot> {
        self.shared.snapshot.subscribe()
    }

    /// Records the provider's execution state.
    pub fn set_state(&self, state: ExecutionState) {
        self.update(|snapshot| snapshot.state = state);
    }

    /// Records the provider child's process identity.
    pub fn set_provider_process(&self, process: ProcessStartIdentity) {
        self.update(|snapshot| snapshot.provider_process = Some(process));
    }

    /// Records the exact provider version a managed launch established from
    /// the provider itself, before its task was submitted. Eligibility never
    /// guesses one, so until this is called no version-bound grant applies.
    pub fn set_provider_version(&self, version: String) {
        self.update(|snapshot| snapshot.facts.provider_version = Some(version));
    }

    /// Records the provider conversation. Any change — including the first
    /// report — starts a new generation, so a target listed under the old
    /// binding is rejected as stale rather than delivered to this one.
    pub fn set_conversation(&self, conversation: Option<String>) -> ConversationGeneration {
        self.update(|snapshot| {
            if snapshot.target.conversation != conversation {
                snapshot.target.generation.0 += 1;
                snapshot.target.conversation = conversation;
            }
        });
        self.snapshot().target.generation
    }

    fn update(&self, change: impl FnOnce(&mut ControllerSnapshot)) {
        let eligibility = &self.shared.eligibility;
        self.shared.snapshot.send_if_modified(|snapshot| {
            let before = snapshot.clone();
            change(snapshot);
            snapshot.availability = availability(eligibility, snapshot);
            *snapshot != before
        });
    }

    /// The route automatic help would use now, or why there is none. Automatic
    /// help names this route's operation in its request; the worker checks
    /// the route again before submitting.
    pub fn automatic_route(&self) -> Result<Route, String> {
        let snapshot = self.snapshot();
        let facts = session_facts(&snapshot).ok_or_else(|| UNMAPPED_PROFILE_REASON.to_string())?;
        match (self.shared.eligibility)(&facts).automatic {
            AutomaticEligibility::Eligible(route) => Ok(route),
            AutomaticEligibility::Unavailable(blocker) => Err(blocker.to_string()),
        }
    }

    /// Stops accepting requests. Queued requests are answered as unavailable
    /// without submission; one being submitted is answered as unknown.
    pub fn shutdown(&self) {
        self.shared.admission.lock().unwrap_or_else(|poison| poison.into_inner()).closed = true;
        self.shared.shutdown.send_replace(true);
    }

    /// Queues `submission` and waits for what the provider established.
    pub async fn submit(&self, submission: Submission) -> ControllerReply {
        let deadlines = DeliveryDeadlines::for_request(&submission.request, Instant::now());
        let context = audit_context(&self.snapshot(), &submission, None);
        let audit = PendingAudit::begin(&self.shared.audit, &submission.request, &context);
        if let Err((outcome, detail)) = self.admit(&submission.request) {
            return finish(&self.shared.audit, audit, &submission.request, None, outcome, Some(detail));
        }
        let (reply, receiver) = oneshot::channel();
        let request = submission.request.clone();
        let job = Job { submission, deadlines, audit, reply };
        if let Err(mpsc::error::SendError(job)) = self.queue.send(job) {
            release(&self.shared, &request);
            return finish(
                &self.shared.audit,
                job.audit,
                &request,
                None,
                SendOutcome::Unavailable,
                Some("the execution has ended".into()),
            );
        }
        receiver.await.unwrap_or_else(|_| ControllerReply {
            result: SteeringResult::submitted(&request, None, SendOutcome::Unknown),
            detail: None,
            audit_failures: Vec::new(),
        })
    }

    /// Admits a request into the bounded queue, or says why not. Refusals
    /// here happen before anything could reach the provider.
    fn admit(&self, request: &SteeringRequest) -> Result<(), (SendOutcome, String)> {
        let own = SteeringTargetId::Managed { execution: self.snapshot().target.execution };
        let mut admission = self.shared.admission.lock().unwrap_or_else(|poison| poison.into_inner());
        if admission.closed {
            return Err((SendOutcome::Unavailable, "the execution has ended".into()));
        }
        if request.target != own {
            return Err((SendOutcome::Refused, "the request names a different target than this execution".into()));
        }
        if !admission.seen.insert(request.id) {
            return Err((
                SendOutcome::Refused,
                "this request ID was already used for this execution; requests are never replayed".into(),
            ));
        }
        if request.operation == OperationIntent::InterruptThenSubmit && !request.may_interrupt() {
            return Err((
                SendOutcome::Refused,
                "interruption requires explicit manual consent bound to this target and operation".into(),
            ));
        }
        let automatic = request.origin == SteeringOrigin::Automatic;
        if automatic && admission.automatic_pending {
            return Err((SendOutcome::Busy, "an automatic request is already pending for this execution".into()));
        }
        if admission.pending >= MAX_PENDING_PER_EXECUTION {
            return Err((SendOutcome::Busy, "this execution's steering queue is full".into()));
        }
        admission.pending += 1;
        admission.automatic_pending |= automatic;
        Ok(())
    }
}

fn release(shared: &Shared, request: &SteeringRequest) {
    let mut admission = shared.admission.lock().unwrap_or_else(|poison| poison.into_inner());
    admission.pending = admission.pending.saturating_sub(1);
    if request.origin == SteeringOrigin::Automatic {
        admission.automatic_pending = false;
    }
}

fn availability(eligibility: &EligibilityFn, snapshot: &ControllerSnapshot) -> AvailabilitySummary {
    match session_facts(snapshot) {
        Some(facts) => eligibility(&facts).manual.summary(),
        None => AvailabilitySummary::unavailable(UNMAPPED_PROFILE_REASON),
    }
}

fn session_facts(snapshot: &ControllerSnapshot) -> Option<SessionFacts<'_>> {
    Some(SessionFacts {
        provider: snapshot.facts.provider,
        profile_id: snapshot.facts.profile_id.as_deref()?,
        os: snapshot.facts.os,
        launch_mode: snapshot.facts.launch_mode,
        origin: LaunchOrigin::Claudine,
        state: snapshot.state,
        provider_version: snapshot.facts.provider_version.as_deref(),
    })
}

fn audit_context(snapshot: &ControllerSnapshot, submission: &Submission, route: Option<&Route>) -> AuditContext {
    AuditContext {
        provider: Some(snapshot.facts.provider),
        conversation: snapshot.target.conversation.clone(),
        generation: Some(snapshot.target.generation),
        profile: snapshot.facts.profile_id.clone(),
        mechanism: route.map(|route| route.mechanism.id),
        opportunity: submission.opportunity,
    }
}

/// Picks the route the current eligibility offers for this request's
/// operation, or explains why none fits.
fn select_route(eligibility: &EligibilityFn, snapshot: &ControllerSnapshot, request: &SteeringRequest) -> Result<Route, String> {
    let facts = session_facts(snapshot).ok_or_else(|| UNMAPPED_PROFILE_REASON.to_string())?;
    let eligibility = eligibility(&facts);
    let route = match request.origin {
        SteeringOrigin::Manual => {
            let summary = eligibility.manual.summary();
            eligibility.manual.route.ok_or_else(|| summary.reason.unwrap_or_default())?
        }
        SteeringOrigin::Automatic => match eligibility.automatic {
            AutomaticEligibility::Eligible(route) => route,
            AutomaticEligibility::Unavailable(blocker) => return Err(blocker.to_string()),
        },
    };
    if route.mechanism.operation_intent != request.operation {
        return Err(format!(
            "the requested `{}` operation is no longer available; this session now offers `{}`",
            request.operation, route.mechanism.operation_intent
        ));
    }
    Ok(route)
}

async fn run_worker(shared: Arc<Shared>, executor: Arc<dyn SteeringExecutor>, mut jobs: mpsc::UnboundedReceiver<Job>) {
    let mut shutdown = shared.shutdown.subscribe();
    loop {
        let job = tokio::select! {
            biased;
            _ = shutdown.wait_for(|stopped| *stopped) => break,
            job = jobs.recv() => match job {
                Some(job) => job,
                None => break,
            },
        };
        let request = job.submission.request.clone();
        process(&shared, &executor, job, &mut shutdown).await;
        release(&shared, &request);
    }
    jobs.close();
    while let Ok(job) = jobs.try_recv() {
        let request = job.submission.request.clone();
        let reply = finish(
            &shared.audit,
            job.audit,
            &request,
            None,
            SendOutcome::Unavailable,
            Some("the execution ended before the request was submitted".into()),
        );
        let _ = job.reply.send(reply);
        release(&shared, &request);
    }
}

async fn process(shared: &Shared, executor: &Arc<dyn SteeringExecutor>, job: Job, shutdown: &mut watch::Receiver<bool>) {
    let Job { submission, deadlines, audit, reply } = job;
    let request = &submission.request;
    let refuse = |audit, outcome, detail: String| finish(&shared.audit, audit, request, None, outcome, Some(detail));

    if Instant::now() >= deadlines.acceptance {
        let _ = reply.send(refuse(audit, SendOutcome::Busy, "the request expired before submission; nothing was sent".into()));
        return;
    }
    let snapshot = shared.snapshot.borrow().clone();
    if let Some(expected) = &submission.expected
        && let Err(stale) = expected.revalidate(&snapshot.target)
    {
        let _ = reply.send(refuse(audit, SendOutcome::Unavailable, format!("the selected target changed: {stale}")));
        return;
    }
    let route = match select_route(&shared.eligibility, &snapshot, request) {
        Ok(route) => route,
        Err(reason) => {
            let _ = reply.send(refuse(audit, SendOutcome::Unavailable, reason));
            return;
        }
    };

    let mut delivery = tokio::spawn(executor.deliver(request.clone(), route, deadlines));
    let late = tokio::select! {
        biased;
        _ = shutdown.wait_for(|stopped| *stopped) => {
            delivery.abort();
            let detail = "the execution stopped while the request was being submitted".to_string();
            let _ = reply.send(finish(&shared.audit, audit, request, Some(&route), SendOutcome::Unknown, Some(detail)));
            return;
        }
        outcome = tokio::time::timeout_at(deadlines.acceptance, &mut delivery) => match outcome {
            Ok(Ok(report)) => {
                let audited = audit.finish(&shared.audit, &capped(report, &route));
                let _ = reply.send(ControllerReply { result: audited.result, detail: audited.error, audit_failures: audited.audit_failures });
                return;
            }
            Ok(Err(_)) => {
                let detail = "the submission failed after it may have reached the provider".to_string();
                let _ = reply.send(finish(&shared.audit, audit, request, Some(&route), SendOutcome::Unknown, Some(detail)));
                return;
            }
            Err(_) => {
                let detail = "no acceptance arrived before the deadline; the message may have been submitted and is never retried".to_string();
                let audited = finish_audited(&shared.audit, audit, request, Some(&route), SendOutcome::Unknown, Some(detail));
                let late = audited.late.clone();
                let _ = reply.send(ControllerReply { result: audited.result, detail: audited.error, audit_failures: audited.audit_failures });
                late
            }
        },
    };

    // The caller has its answer. Keep mutations serialized by waiting for
    // this submission to finish, and log what it eventually established.
    tokio::select! {
        biased;
        _ = shutdown.wait_for(|stopped| *stopped) => delivery.abort(),
        outcome = tokio::time::timeout(LATE_RESULT_WINDOW, &mut delivery) => match outcome {
            Ok(Ok(report)) => {
                let _ = late.record(&shared.audit, &capped(report, &route));
            }
            Ok(Err(_)) => {}
            Err(_) => {
                delivery.abort();
                tracing::warn!(request = %request.id, "steering submission abandoned after its late-result window");
            }
        },
    }
}

/// Lowers a report to what the route's mechanism can establish.
fn capped(mut report: DeliveryReport, route: &Route) -> DeliveryReport {
    let ceiling = route.max_receipt();
    report.result.outcome = report.result.outcome.capped_at(ceiling);
    report.result.receipt = report.result.outcome.receipt();
    if let Some(interruption) = report.result.interruption.as_mut() {
        interruption.replacement = interruption.replacement.map(|outcome| outcome.capped_at(ceiling));
    }
    report
}

fn finish_audited(
    log: &SteeringAuditLog,
    audit: PendingAudit,
    request: &SteeringRequest,
    route: Option<&Route>,
    outcome: SendOutcome,
    detail: Option<String>,
) -> super::audit::AuditedSend {
    let mut report = DeliveryReport::new(SteeringResult::submitted(request, route.map(|route| route.mechanism.id), outcome));
    report.error = detail;
    audit.finish(log, &report)
}

fn finish(
    log: &SteeringAuditLog,
    audit: PendingAudit,
    request: &SteeringRequest,
    route: Option<&Route>,
    outcome: SendOutcome,
    detail: Option<String>,
) -> ControllerReply {
    let audited = finish_audited(log, audit, request, route, outcome, detail);
    ControllerReply { result: audited.result, detail: audited.error, audit_failures: audited.audit_failures }
}
