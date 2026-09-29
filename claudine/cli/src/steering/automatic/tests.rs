//! Automatic help against a real controller with fixture eligibility and a
//! recording executor: the opportunity cap, unavailable and unconfirmed
//! notices, bounded non-blocking sends, and hard-stop priority.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use claudine::provider::Provider;
use claudine::runaway::RepetitionSignal;
use claudine::steering::audit::{DeliveryReport, SteeringAuditLog};
use claudine::steering::contract::{SendOutcome, SteeringOrigin, SteeringRequest, SteeringResult};
use claudine::steering::controller::{
    ControllerConfig, DeliveryDeadlines, DeliveryFuture, EligibilityFn, ExecutionFacts, SteeringController,
    SteeringExecutor,
};
use claudine::steering::eligibility::{AutomaticEligibility, Eligibility, ManualEligibility, Route};
use claudine::steering::identity::{ExecutionId, ProcessStartIdentity};
use claudine::steering::vocabulary::{
    AdapterRef, CaseSupport, ExecutionState, LaunchMode, OperationIntent, SteeringAvailability, SteeringMechanism,
};

use super::*;

pub(crate) const WARNING: RepetitionSignal = RepetitionSignal::Warning { cycle_len: 1, repeats: 15, stop_limit: 30 };

fn rpc_steer() -> &'static SteeringMechanism {
    claudine::steering::facts(Provider::Pi).mechanisms.iter().find(|m| m.id == "rpc-steer").expect("researched mechanism")
}

/// Eligibility offering Pi's researched `rpc-steer` for automatic help, so
/// delivery is exercised without an activation grant.
pub(crate) fn automatically_routable() -> EligibilityFn {
    Arc::new(|_| {
        let route = Route { mechanism: rpc_steer(), adapter: AdapterRef { id: "fixture", revision: 1 }, support: CaseSupport::NonInterrupting };
        Eligibility {
            manual: ManualEligibility { availability: SteeringAvailability::NonInterrupting, route: Some(route), blockers: Vec::new() },
            automatic: AutomaticEligibility::Eligible(route),
        }
    })
}

/// What the fixture executor answers.
#[derive(Clone, Copy)]
pub(crate) enum Answer {
    With(SendOutcome),
    /// Never answers; the controller's deadline decides.
    Never,
}

pub(crate) struct Recording {
    pub(crate) answer: Answer,
    pub(crate) delivered: Mutex<Vec<SteeringRequest>>,
    /// Submissions whose future was dropped before it finished.
    pub(crate) abandoned: AtomicUsize,
}

impl Recording {
    pub(crate) fn new(answer: Answer) -> Arc<Self> {
        Arc::new(Self { answer, delivered: Mutex::new(Vec::new()), abandoned: AtomicUsize::new(0) })
    }

    pub(crate) fn count(&self) -> usize {
        self.delivered.lock().unwrap().len()
    }
}

struct Executor(Arc<Recording>);

/// Counts a submission whose future is dropped unfinished.
struct AbandonGuard(Arc<Recording>, bool);

impl Drop for AbandonGuard {
    fn drop(&mut self) {
        if !self.1 {
            self.0.abandoned.fetch_add(1, Ordering::SeqCst);
        }
    }
}

impl SteeringExecutor for Executor {
    fn deliver(&self, request: SteeringRequest, route: Route, _deadlines: DeliveryDeadlines) -> DeliveryFuture {
        let recording = Arc::clone(&self.0);
        Box::pin(async move {
            recording.delivered.lock().unwrap().push(request.clone());
            let mut guard = AbandonGuard(Arc::clone(&recording), false);
            let outcome = match recording.answer {
                Answer::With(outcome) => outcome,
                Answer::Never => std::future::pending().await,
            };
            guard.1 = true;
            DeliveryReport::new(SteeringResult::submitted(&request, Some(route.mechanism.id), outcome))
        })
    }
}

pub(crate) fn controller(audit: &tempfile::TempDir, eligibility: EligibilityFn, recording: &Arc<Recording>) -> SteeringController {
    let config = ControllerConfig {
        execution: ExecutionId::random(),
        wrapper: ProcessStartIdentity::new(std::process::id(), "fixture-start").unwrap(),
        facts: ExecutionFacts {
            provider: Provider::Pi,
            profile_id: Some("retained-rpc".into()),
            os: claudine::steering::host_os(),
            launch_mode: LaunchMode::NonInteractive,
            provider_version: Some("0.87.1".into()),
            cwd: Some("/work/project".into()),
            name: None,
        },
        initial_state: ExecutionState::Working,
        eligibility,
        audit: SteeringAuditLog::at(audit.path()),
    };
    SteeringController::spawn(config, Arc::new(Executor(Arc::clone(recording))))
}

pub(crate) type Collected = Arc<Mutex<Vec<(NoticeLevel, String)>>>;

pub(crate) fn collecting() -> (NoticeFn, Collected) {
    let collected: Collected = Arc::default();
    let sink = Arc::clone(&collected);
    (Arc::new(move |level, text: &str| sink.lock().unwrap().push((level, text.to_string()))), collected)
}

/// Waits (bounded) until `ready` holds.
pub(crate) async fn eventually(what: &str, ready: impl Fn() -> bool) {
    for _ in 0..500 {
        if ready() {
            return;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    panic!("timed out waiting for {what}");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn three_warnings_send_three_helper_messages_then_nothing() {
    let tmp = tempfile::tempdir().unwrap();
    let recording = Recording::new(Answer::With(SendOutcome::Queued));
    let (notice, notices) = collecting();
    let mut help = AutomaticHelp::new(Some(controller(&tmp, automatically_routable(), &recording)), notice);

    for sent in 1..=3 {
        help.on_signal(&WARNING);
        eventually("the warning's reply", || notices.lock().unwrap().len() == sent).await;
    }
    help.on_signal(&WARNING);
    tokio::time::sleep(Duration::from_millis(100)).await;

    let delivered = recording.delivered.lock().unwrap().clone();
    assert_eq!(delivered.len(), 3, "the fourth warning has no opportunity left");
    for request in &delivered {
        assert_eq!(request.origin, SteeringOrigin::Automatic);
        assert_eq!(request.operation, OperationIntent::SteerActiveTurn);
        assert!(request.consent.is_none(), "automatic help never interrupts");
        assert_eq!(request.message.as_str(), helper_message(1, 15));
    }
    assert_ne!(delivered[0].id, delivered[1].id);
    let notices = notices.lock().unwrap().clone();
    assert_eq!(notices.len(), 3);
    for (index, (level, text)) in notices.iter().enumerate() {
        assert_eq!(*level, NoticeLevel::Sent);
        assert!(text.contains(&format!("queued; {} of 3", index + 1)), "{text}");
        assert!(text.contains("repetition limit of 30 still applies"), "{text}");
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn an_unavailable_route_prints_one_notice_and_spends_every_opportunity() {
    let tmp = tempfile::tempdir().unwrap();
    let recording = Recording::new(Answer::With(SendOutcome::Accepted));
    // The build's own eligibility: Pi's managed profile is blocked.
    let owner = controller(&tmp, Arc::new(claudine::steering::eligibility::evaluate), &recording);
    let reason = owner.automatic_route().expect_err("the shipped policy offers no automatic route");
    let (notice, notices) = collecting();
    let mut help = AutomaticHelp::new(Some(owner), notice);

    for _ in 0..5 {
        help.on_signal(&WARNING);
    }
    assert_eq!(help.budget.used(), 3, "unavailable opportunities count");
    assert_eq!(recording.count(), 0, "nothing reaches the provider");
    let notices = notices.lock().unwrap().clone();
    assert_eq!(notices.len(), 1, "deduplicated: {notices:?}");
    assert_eq!(notices[0].0, NoticeLevel::Warning);
    assert!(notices[0].1.contains("cannot send automatic steering to this session"));
    assert!(notices[0].1.contains(&reason), "{} / {reason}", notices[0].1);
    assert!(notices[0].1.contains("keep enforcing the repetition limit of 30"));
}

#[tokio::test]
async fn without_an_owner_every_warning_is_unavailable() {
    let (notice, notices) = collecting();
    let mut help = AutomaticHelp::new(None, notice);
    help.on_signal(&WARNING);
    help.on_signal(&WARNING);
    let notices = notices.lock().unwrap().clone();
    assert_eq!(notices.len(), 1);
    assert!(notices[0].1.contains(NO_OWNER_REASON));
    assert_eq!(help.budget.used(), 2);
}

#[tokio::test(start_paused = true)]
async fn a_send_never_blocks_and_an_unanswered_one_is_reported_unconfirmed() {
    let tmp = tempfile::tempdir().unwrap();
    let recording = Recording::new(Answer::Never);
    let (notice, notices) = collecting();
    let mut help = AutomaticHelp::new(Some(controller(&tmp, automatically_routable(), &recording)), notice);

    let started = std::time::Instant::now();
    help.on_signal(&WARNING);
    assert!(started.elapsed() < Duration::from_millis(200), "the stream reader is never held");

    let dispatched = tokio::time::Instant::now();
    eventually("the deadline's reply", || !notices.lock().unwrap().is_empty()).await;
    assert!(dispatched.elapsed() >= claudine::steering::controller::AUTOMATIC_ACCEPTANCE_DEADLINE);
    let notices = notices.lock().unwrap().clone();
    assert_eq!(notices[0].0, NoticeLevel::Warning);
    assert!(notices[0].1.contains("was not confirmed (unknown"), "{}", notices[0].1);
    assert_eq!(recording.count(), 1, "never retried");
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_warning_while_one_is_in_flight_is_busy_and_still_spends_its_opportunity() {
    let tmp = tempfile::tempdir().unwrap();
    let recording = Recording::new(Answer::Never);
    let (notice, notices) = collecting();
    let mut help = AutomaticHelp::new(Some(controller(&tmp, automatically_routable(), &recording)), notice);

    help.on_signal(&WARNING);
    eventually("the first submission", || recording.count() == 1).await;
    help.on_signal(&WARNING);
    help.on_signal(&WARNING);
    eventually("the busy replies", || !notices.lock().unwrap().is_empty()).await;
    tokio::time::sleep(Duration::from_millis(100)).await;
    help.on_signal(&WARNING);

    assert_eq!(help.budget.used(), 3);
    assert_eq!(recording.count(), 1, "one automatic request at a time");
    let notices = notices.lock().unwrap().clone();
    assert_eq!(notices.len(), 1, "identical busy notices are deduplicated: {notices:?}");
    assert!(notices[0].1.contains("not sent (busy)"), "{}", notices[0].1);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn a_hard_stop_abandons_the_send_in_flight_and_refuses_later_warnings() {
    let tmp = tempfile::tempdir().unwrap();
    let recording = Recording::new(Answer::Never);
    let owner = controller(&tmp, automatically_routable(), &recording);
    let (notice, notices) = collecting();
    let mut help = AutomaticHelp::new(Some(owner.clone()), notice);

    help.on_signal(&WARNING);
    eventually("the submission", || recording.count() == 1).await;
    help.hard_stop();
    eventually("the abandoned submission", || recording.abandoned.load(Ordering::SeqCst) == 1).await;

    help.on_signal(&WARNING);
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert_eq!(recording.count(), 1, "nothing is sent after the stop");
    assert_eq!(help.budget.used(), 1, "a warning after the stop is not an opportunity");
    assert!(notices.lock().unwrap().is_empty(), "no notice follows the stop");
    let late = owner.submit(claudine::steering::controller::Submission {
        request: recording.delivered.lock().unwrap()[0].clone(),
        expected: None,
        opportunity: None,
    });
    assert_eq!(late.await.result.outcome, SendOutcome::Unavailable, "the controller is stopped");
}

#[tokio::test]
async fn recovery_spends_nothing() {
    let (notice, notices) = collecting();
    let mut help = AutomaticHelp::new(None, notice);
    help.on_signal(&RepetitionSignal::Recovered { cycle_len: 1 });
    assert_eq!(help.budget.used(), 0);
    assert!(notices.lock().unwrap().is_empty());
}

#[test]
fn the_helper_message_carries_counts_and_never_output() {
    assert!(helper_message(1, 15).ends_with("(Observed: the same line 15 times in a row.)"));
    assert!(helper_message(6, 16).ends_with("(Observed: the same 6-line block 16 times in a row.)"));
    assert!(helper_message(1, 15).starts_with(HELPER_MESSAGE));
}
