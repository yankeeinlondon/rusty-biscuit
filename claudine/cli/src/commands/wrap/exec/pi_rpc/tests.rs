//! Deterministic tests of the Pi RPC owner over an in-memory wire.
//!
//! [`Wire`] records what the session writes; a scripted fake Pi answers each
//! command by feeding records back through [`StdioControl::observe`], exactly
//! as the stdout forwarder does. Executor tests go through a real
//! [`SteeringController`], so bounds, auditing, and route selection run as in
//! production. The real-provider counterpart is
//! `tests/real/real_pi_managed_rpc.rs`.

use std::io::Write;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use claudine::provider::Provider;
use claudine::steering::audit::SteeringAuditLog;
use claudine::steering::contract::{
    CancellationOutcome, InterruptionConsent, SendOutcome, SteeringMessage, SteeringOrigin, SteeringRequest,
};
use claudine::steering::controller::{ControllerConfig, EligibilityFn, ExecutionFacts, SteeringController, Submission};
use claudine::steering::eligibility::{AutomaticEligibility, Eligibility, ManualEligibility, Route};
use claudine::steering::identity::{ExecutionId, ProcessStartIdentity, RequestId};
use claudine::steering::vocabulary::{
    CaseSupport, ExecutionState, LaunchMode, OperationIntent, SteeringAvailability,
};
use claudine::stream::logs::EarlyTermination;
use serde_json::{Value, json};

use super::super::control::{ControlChannels, Readiness, StdioControl};
use super::super::termination::CompletionTermination;
use super::{PiRpcSession, commands};

const SESSION: &str = "01a0eae3-8823-705a-8978-46829cdf84e7";
const MESSAGE: &str = "Recheck the failing test ✓ before editing";

/// What the session wrote, one JSON command per line.
#[derive(Clone, Default)]
struct Wire(Arc<Mutex<Vec<u8>>>);

impl Write for Wire {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl Wire {
    fn commands(&self) -> Vec<Value> {
        let bytes = self.0.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect()
    }

    fn kinds(&self) -> Vec<String> {
        self.commands().iter().map(|command| command["type"].as_str().unwrap().to_string()).collect()
    }
}

struct Opened {
    session: PiRpcSession,
    wire: Wire,
    early: Receiver<EarlyTermination>,
    completion: Receiver<CompletionTermination>,
}

fn open() -> Opened {
    let session = PiRpcSession::new(None);
    let wire = Wire::default();
    let (early_tx, early) = mpsc::channel();
    let (completion_tx, completion) = mpsc::channel();
    session
        .open_writer(Box::new(wire.clone()), ControlChannels { early: early_tx, completion: completion_tx })
        .unwrap();
    Opened { session, wire, early, completion }
}

fn response(command: &Value, success: bool, data: Option<Value>) -> Value {
    let mut record = json!({"type": "response", "command": command["type"], "id": command["id"], "success": success});
    if let Some(data) = data {
        record["data"] = data;
    }
    if !success {
        record["error"] = json!("fixture refusal");
    }
    record
}

fn state(session: &str, streaming: bool, pending: u64) -> Value {
    json!({"sessionId": session, "isStreaming": streaming, "isCompacting": false, "pendingMessageCount": pending})
}

fn ready(opened: &Opened) {
    let probe = &opened.wire.commands()[0];
    assert_eq!(probe, &commands::get_state(commands::READY_ID), "readiness is the first command");
    opened.session.observe(&response(probe, true, Some(state(SESSION, false, 0))).to_string());
    assert_eq!(opened.session.readiness(), Readiness::Ready);
}

/// Pi as a script: `answer` returns the records Pi emits for one command.
struct FakePi {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl FakePi {
    fn start(opened: &Opened, answer: impl Fn(&Value) -> Vec<Value> + Send + 'static) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (session, wire, stopped) = (opened.session.clone(), opened.wire.clone(), Arc::clone(&stop));
        // Taken now, not in the thread: a command written before the thread
        // runs must still be answered.
        let mut seen = wire.commands().len();
        let handle = thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                let commands = wire.commands();
                for command in &commands[seen..] {
                    for record in answer(command) {
                        session.observe(&record.to_string());
                    }
                }
                seen = commands.len();
                thread::sleep(Duration::from_millis(2));
            }
        });
        Self { stop, handle: Some(handle) }
    }
}

impl Drop for FakePi {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// A Pi that reports `streaming` for its state and accepts every message.
fn accepting(streaming: Arc<AtomicBool>) -> impl Fn(&Value) -> Vec<Value> {
    move |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state(SESSION, streaming.load(Ordering::SeqCst), 0)))],
        "abort" => {
            streaming.store(false, Ordering::SeqCst);
            vec![response(command, true, None)]
        }
        "steer" | "prompt" => vec![response(command, true, None)],
        _ => Vec::new(),
    }
}

fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        thread::sleep(Duration::from_millis(5));
    }
}

/// Stdin is closed: the session released its writer.
fn closed(opened: &Opened) -> bool {
    opened.session.inner.writer.lock().unwrap().is_none()
}

// ---------------------------------------------------------------------------
// Readiness and the task
// ---------------------------------------------------------------------------

#[test]
fn readiness_needs_a_state_answer_naming_a_session() {
    let opened = open();
    assert_eq!(opened.session.readiness(), Readiness::Pending);
    ready(&opened);
    assert_eq!(opened.session.inner.state.lock().unwrap().conversation.as_deref(), Some(SESSION));
    assert!(!opened.session.abandon_readiness("late"), "a ready child stays ready");

    for (label, record) in [
        ("refused", json!({"type":"response","command":"get_state","id":commands::READY_ID,"success":false,"error":"boom"})),
        ("no session", json!({"type":"response","command":"get_state","id":commands::READY_ID,"success":true,"data":{"isStreaming":false}})),
        ("unreadable success", json!({"type":"response","command":"get_state","id":commands::READY_ID,"success":"yes"})),
    ] {
        let opened = open();
        opened.session.observe(&record.to_string());
        assert!(matches!(opened.session.readiness(), Readiness::Failed(_)), "{label}");
    }

    let opened = open();
    assert!(opened.session.abandon_readiness("timed out"));
    assert_eq!(opened.session.readiness(), Readiness::Failed("timed out".into()));
    // A late answer cannot revive an abandoned child.
    opened.session.observe(&response(&commands::get_state(commands::READY_ID), true, Some(state(SESSION, false, 0))).to_string());
    assert_eq!(opened.session.readiness(), Readiness::Failed("timed out".into()));
}

#[test]
fn the_task_is_one_prompt_with_the_original_text() {
    let opened = open();
    ready(&opened);
    let task = "- a Markdown bullet first\n\nthen @file-looking text and ✓ unicode";
    opened.session.submit(task).unwrap();
    assert_eq!(opened.wire.commands()[1], commands::prompt(commands::TASK_ID, task));
}

// ---------------------------------------------------------------------------
// Unattended requests
// ---------------------------------------------------------------------------

#[test]
fn dialogs_are_cancelled_notices_ignored_and_unknown_methods_end_the_run() {
    let opened = open();
    ready(&opened);
    for method in ["confirm", "select", "input", "editor"] {
        let id = format!("ui-{method}");
        opened.session.observe(&json!({"type":"extension_ui_request","id":id,"method":method,"title":"t"}).to_string());
        assert_eq!(opened.wire.commands().last().unwrap(), &commands::cancel_dialog(&id), "{method}");
    }
    let written = opened.wire.commands().len();
    for method in ["notify", "setStatus", "setWidget", "setTitle", "set_editor_text"] {
        opened.session.observe(&json!({"type":"extension_ui_request","id":"n","method":method}).to_string());
    }
    assert_eq!(opened.wire.commands().len(), written, "notices are never answered");
    assert!(opened.early.try_recv().is_err(), "nothing so far ends the run");

    opened.session.observe(&json!({"type":"extension_ui_request","id":"x","method":"custom"}).to_string());
    let EarlyTermination::InputRequired { message } = opened.early.try_recv().unwrap() else { panic!("input_required") };
    assert!(message.contains("`custom`"), "{message}");
    assert_eq!(opened.wire.commands().len(), written, "no answer is invented for an unknown method");

    opened.session.observe(r#"{"type":"extension_ui_request","method":"confirm"}"#);
    assert!(matches!(opened.early.try_recv(), Ok(EarlyTermination::InputRequired { .. })), "an unanswerable dialog");
}

/// A clean exit after an unanswerable request would report success; the
/// session keeps stdin open so the wait loop acts on `input_required`.
#[test]
fn an_input_required_run_is_never_ended_gracefully() {
    let opened = open();
    ready(&opened);
    let _pi = FakePi::start(&opened, |command| vec![response(command, true, Some(state(SESSION, false, 0)))]);
    opened.session.observe(&json!({"type":"extension_ui_request","id":"x","method":"custom"}).to_string());
    assert!(matches!(opened.early.try_recv(), Ok(EarlyTermination::InputRequired { .. })));
    opened.session.observe(&json!({"type":"response","command":"prompt","id":commands::TASK_ID,"success":true}).to_string());
    opened.session.observe(r#"{"type":"agent_settled"}"#);
    thread::sleep(Duration::from_millis(100));
    assert!(!closed(&opened), "stdin stays open while the run is failed as input_required");
}

// ---------------------------------------------------------------------------
// Settlement
// ---------------------------------------------------------------------------

#[test]
fn a_settled_quiescent_run_closes_stdin_once() {
    let opened = open();
    ready(&opened);
    let _pi = FakePi::start(&opened, |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state(SESSION, false, 0)))],
        _ => Vec::new(),
    });
    opened.session.observe(r#"{"type":"agent_start"}"#);
    assert!(!closed(&opened), "a running task keeps stdin open");
    opened.session.observe(r#"{"type":"agent_end","willRetry":false}"#);
    assert!(!closed(&opened), "agent_end is not settlement");
    opened.session.observe(r#"{"type":"agent_settled"}"#);
    wait_until("stdin to close", || closed(&opened));
    assert_eq!(opened.wire.kinds(), ["get_state", "get_state"], "one quiescence check, then close");
    assert!(opened.completion.try_recv().is_err(), "Pi gets its exit grace first");
}

#[test]
fn work_after_settling_keeps_the_run_open() {
    for (label, busy_state) in [("streaming", state(SESSION, true, 0)), ("compacting", json!({"sessionId":SESSION,"isStreaming":false,"isCompacting":true,"pendingMessageCount":0}))] {
        let opened = open();
        ready(&opened);
        let answered = Arc::new(AtomicU64::new(0));
        let count = Arc::clone(&answered);
        let _pi = FakePi::start(&opened, move |command| {
            count.fetch_add(1, Ordering::SeqCst);
            vec![response(command, true, Some(busy_state.clone()))]
        });
        opened.session.observe(r#"{"type":"agent_settled"}"#);
        wait_until("the check", || answered.load(Ordering::SeqCst) == 1);
        thread::sleep(Duration::from_millis(50));
        assert!(!closed(&opened), "{label}: Pi is still working");
        assert!(!opened.session.inner.state.lock().unwrap().closing, "{label}: steering is accepted again");
    }
}

#[test]
fn a_new_turn_during_the_check_keeps_the_run_open() {
    let opened = open();
    ready(&opened);
    let session = opened.session.clone();
    let _pi = FakePi::start(&opened, move |command| {
        // A queued continuation starts before Pi answers the check.
        session.observe(r#"{"type":"agent_start"}"#);
        vec![response(command, true, Some(state(SESSION, false, 0)))]
    });
    opened.session.observe(r#"{"type":"agent_settled"}"#);
    thread::sleep(Duration::from_millis(100));
    assert!(!closed(&opened));
}

#[test]
fn queued_input_at_settlement_is_reported_undelivered_and_the_run_ends() {
    let opened = open();
    ready(&opened);
    let _pi = FakePi::start(&opened, |command| vec![response(command, true, Some(state(SESSION, false, 1)))]);
    opened.session.observe(r#"{"type":"agent_settled"}"#);
    wait_until("stdin to close", || closed(&opened));
    assert!(!opened.wire.kinds().iter().any(|kind| kind == "prompt" || kind == "steer" || kind == "follow_up"), "nothing is resent");
}

#[test]
fn an_extension_handled_task_ends_without_a_turn_and_a_refused_one_at_once() {
    let opened = open();
    ready(&opened);
    let _pi = FakePi::start(&opened, |command| vec![response(command, true, Some(state(SESSION, false, 0)))]);
    opened.session.observe(&json!({"type":"response","command":"prompt","id":commands::TASK_ID,"success":true}).to_string());
    wait_until("stdin to close", || closed(&opened));

    let refused = open();
    ready(&refused);
    refused.session.observe(&json!({"type":"response","command":"prompt","id":commands::TASK_ID,"success":false,"error":"x"}).to_string());
    wait_until("stdin to close", || closed(&refused));
    assert_eq!(refused.wire.kinds(), ["get_state"], "no check is needed when nothing will run");
}

#[test]
fn a_task_that_starts_a_turn_is_not_closed_by_its_acceptance() {
    let opened = open();
    ready(&opened);
    let _pi = FakePi::start(&opened, |command| vec![response(command, true, Some(state(SESSION, true, 0)))]);
    opened.session.observe(&json!({"type":"response","command":"prompt","id":commands::TASK_ID,"success":true}).to_string());
    opened.session.observe(r#"{"type":"agent_start"}"#);
    thread::sleep(Duration::from_millis(100));
    assert!(!closed(&opened));
}

#[test]
fn finishing_releases_everything_waiting_on_the_child() {
    let opened = open();
    let waiter = opened.session.inner.request(commands::get_state).unwrap();
    opened.session.finish();
    assert!(matches!(opened.session.readiness(), Readiness::Failed(_)), "an exited child never becomes ready");
    assert!(matches!(waiter.blocking_recv(), Ok(Err(_))), "a waiting command learns the child exited");
    assert!(closed(&opened));
}

// ---------------------------------------------------------------------------
// Steering through the owner's controller
// ---------------------------------------------------------------------------

fn mechanism(id: &str) -> &'static claudine::steering::vocabulary::SteeringMechanism {
    claudine::steering::facts(Provider::Pi).mechanisms.iter().find(|m| m.id == id).unwrap()
}

/// Eligibility offering Pi's researched mechanism for each state, so the
/// adapter is exercised without an activation grant.
fn routable(interrupt: bool) -> EligibilityFn {
    Arc::new(move |session| {
        let (id, support) = match (session.state, interrupt) {
            (ExecutionState::Working, false) => ("rpc-steer", CaseSupport::NonInterrupting),
            (ExecutionState::Working, true) => ("rpc-abort-submit", CaseSupport::InterruptionRequired),
            _ => ("rpc-idle-prompt", CaseSupport::NonInterrupting),
        };
        let route = Route { mechanism: mechanism(id), adapter: claudine::steering::adapters::PI_RPC, support };
        let availability = match support {
            CaseSupport::NonInterrupting => SteeringAvailability::NonInterrupting,
            _ => SteeringAvailability::InterruptionRequired,
        };
        Eligibility {
            manual: ManualEligibility { availability, route: Some(route), blockers: Vec::new() },
            automatic: AutomaticEligibility::Eligible(route),
        }
    })
}

struct Owned {
    opened: Opened,
    controller: SteeringController,
    _audit: tempfile::TempDir,
}

fn owned(interrupt: bool) -> Owned {
    let opened = open();
    let audit = tempfile::tempdir().unwrap();
    let config = ControllerConfig {
        execution: ExecutionId::random(),
        wrapper: ProcessStartIdentity::new(std::process::id(), "fixture-start").unwrap(),
        facts: ExecutionFacts {
            provider: Provider::Pi,
            profile_id: Some(super::PROFILE_ID.into()),
            os: claudine::steering::host_os(),
            launch_mode: LaunchMode::NonInteractive,
            provider_version: Some("0.84.4".into()),
            cwd: None,
            name: None,
        },
        initial_state: ExecutionState::Working,
        eligibility: routable(interrupt),
        audit: SteeringAuditLog::at(audit.path()),
    };
    let controller = SteeringController::spawn(config, opened.session.steering_executor().unwrap());
    opened.session.bind_controller(controller.clone());
    ready(&opened);
    Owned { opened, controller, _audit: audit }
}

fn submission(owned: &Owned, operation: OperationIntent) -> Submission {
    let target = owned.controller.snapshot().target;
    let mut request = SteeringRequest {
        id: RequestId::random(),
        target: target.id(),
        origin: SteeringOrigin::Manual,
        operation,
        message: SteeringMessage::new(MESSAGE).unwrap(),
        consent: None,
    };
    if operation == OperationIntent::InterruptThenSubmit {
        request.consent = Some(InterruptionConsent { target: request.target.clone(), operation });
    }
    Submission { request, expected: Some(target), opportunity: None }
}

#[tokio::test(flavor = "multi_thread")]
async fn readiness_binds_the_target_and_turns_bump_state() {
    let owned = owned(false);
    let snapshot = owned.controller.snapshot();
    assert_eq!(snapshot.target.conversation.as_deref(), Some(SESSION));
    assert_eq!(snapshot.facts.profile_id.as_deref(), Some("retained-rpc"));
    owned.opened.session.observe(r#"{"type":"agent_settled"}"#);
    assert_eq!(owned.controller.snapshot().state, ExecutionState::Idle);
    owned.opened.session.observe(r#"{"type":"agent_start"}"#);
    assert_eq!(owned.controller.snapshot().state, ExecutionState::Working);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_working_session_is_steered_after_a_fresh_state_check() {
    let owned = owned(false);
    let _pi = FakePi::start(&owned.opened, accepting(Arc::new(AtomicBool::new(true))));
    let reply = owned.controller.submit(submission(&owned, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Queued, "{:?}", reply.detail);
    assert_eq!(reply.result.mechanism, Some("rpc-steer"));
    let commands = owned.opened.wire.commands();
    let kinds: Vec<_> = commands.iter().map(|c| c["type"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["get_state", "get_state", "steer"], "readiness, the fresh check, then the steer");
    assert_eq!(commands[2]["message"], MESSAGE, "the original bytes reach Pi");
}

#[tokio::test(flavor = "multi_thread")]
async fn nothing_is_sent_when_the_fresh_state_disagrees() {
    // No longer working.
    let owned1 = owned(false);
    let _pi = FakePi::start(&owned1.opened, accepting(Arc::new(AtomicBool::new(false))));
    let reply = owned1.controller.submit(submission(&owned1, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert!(reply.detail.unwrap().as_str().contains("no longer working"));
    assert!(!owned1.opened.wire.kinds().contains(&"steer".to_string()));
    assert_eq!(owned1.controller.snapshot().state, ExecutionState::Idle, "the owner learns the state");

    // Switched to another session: the target moves to a new generation.
    let owned2 = owned(false);
    let before = owned2.controller.snapshot().target.generation;
    let _pi2 = FakePi::start(&owned2.opened, |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state("another-session", true, 0)))],
        _ => vec![response(command, true, None)],
    });
    let reply = owned2.controller.submit(submission(&owned2, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert!(reply.detail.unwrap().as_str().contains("different session"));
    assert!(!owned2.opened.wire.kinds().contains(&"steer".to_string()));
    let after = owned2.controller.snapshot().target;
    assert_eq!(after.conversation.as_deref(), Some("another-session"));
    assert!(after.generation > before, "a listing taken before the switch is now stale");
}

#[tokio::test(flavor = "multi_thread")]
async fn refusals_and_silence_are_reported_as_established() {
    let owned1 = owned(false);
    let _pi = FakePi::start(&owned1.opened, |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state(SESSION, true, 0)))],
        _ => vec![response(command, false, None)],
    });
    let reply = owned1.controller.submit(submission(&owned1, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Refused);

    // The child exits while the steer waits: still unknown, not refused.
    let owned3 = owned(false);
    let session = owned3.opened.session.clone();
    let _pi3 = FakePi::start(&owned3.opened, move |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state(SESSION, true, 0)))],
        _ => {
            session.finish();
            Vec::new()
        }
    });
    let reply = owned3.controller.submit(submission(&owned3, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unknown, "{:?}", reply.detail);
}

/// Automatic, for its two-second deadline.
#[tokio::test(flavor = "multi_thread")]
async fn an_unanswered_steer_is_unknown_and_never_resent() {
    let owned = owned(false);
    let _pi = FakePi::start(&owned.opened, |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state(SESSION, true, 0)))],
        _ => Vec::new(),
    });
    let mut silent = submission(&owned, OperationIntent::SteerActiveTurn);
    silent.request.origin = SteeringOrigin::Automatic;
    silent.expected = None;
    let reply = owned.controller.submit(silent).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unknown);
    thread::sleep(Duration::from_millis(100));
    let steers = owned.opened.wire.kinds().iter().filter(|kind| *kind == "steer").count();
    assert_eq!(steers, 1, "an unanswered steer is never resent");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_settled_run_refuses_new_steering() {
    let owned = owned(false);
    owned.opened.session.inner.state.lock().unwrap().closing = true;
    let reply = owned.controller.submit(submission(&owned, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert_eq!(owned.opened.wire.kinds(), ["get_state"], "only the readiness check was ever written");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_idle_session_takes_a_prompt() {
    let owned = owned(false);
    owned.controller.set_state(ExecutionState::Idle);
    let _pi = FakePi::start(&owned.opened, accepting(Arc::new(AtomicBool::new(false))));
    let reply = owned.controller.submit(submission(&owned, OperationIntent::StartIdleTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted, "{:?}", reply.detail);
    assert_eq!(owned.opened.wire.commands().last().unwrap()["type"], "prompt");
}

#[tokio::test(flavor = "multi_thread")]
async fn consented_interruption_aborts_waits_revalidates_then_submits() {
    let owned = owned(true);
    let _pi = FakePi::start(&owned.opened, accepting(Arc::new(AtomicBool::new(true))));
    let reply = owned.controller.submit(submission(&owned, OperationIntent::InterruptThenSubmit)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted, "{:?}", reply.detail);
    let phases = reply.result.interruption.unwrap();
    assert_eq!(phases.cancellation, CancellationOutcome::Established);
    assert_eq!(phases.replacement, Some(SendOutcome::Accepted));
    let kinds = owned.opened.wire.kinds();
    assert_eq!(kinds, ["get_state", "get_state", "abort", "get_state", "prompt"]);
    assert!(!kinds.iter().any(|kind| kind == "clear_queue"), "pending queues are never cleared");
}

#[tokio::test(flavor = "multi_thread")]
async fn interruption_phases_stay_separate_when_one_fails() {
    // Replacement refused after a confirmed cancellation.
    let owned1 = owned(true);
    let streaming = Arc::new(AtomicBool::new(true));
    let _pi = FakePi::start(&owned1.opened, move |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state(SESSION, streaming.load(Ordering::SeqCst), 0)))],
        "abort" => {
            streaming.store(false, Ordering::SeqCst);
            vec![response(command, true, None)]
        }
        _ => vec![response(command, false, None)],
    });
    let reply = owned1.controller.submit(submission(&owned1, OperationIntent::InterruptThenSubmit)).await;
    assert_eq!(reply.result.outcome, SendOutcome::PartialInterruption);
    assert_eq!(reply.result.interruption.unwrap().replacement, Some(SendOutcome::Refused));

    // Abort refused: no replacement is sent.
    let owned2 = owned(true);
    let _pi2 = FakePi::start(&owned2.opened, |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state(SESSION, true, 0)))],
        _ => vec![response(command, false, None)],
    });
    let reply = owned2.controller.submit(submission(&owned2, OperationIntent::InterruptThenSubmit)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Refused);
    assert!(!owned2.opened.wire.kinds().contains(&"prompt".to_string()));

    // Session switched during the abort: cancelled, replacement withheld.
    let owned3 = owned(true);
    let aborted = Arc::new(AtomicBool::new(false));
    let _pi3 = FakePi::start(&owned3.opened, move |command| match command["type"].as_str().unwrap() {
        "get_state" if aborted.load(Ordering::SeqCst) => vec![response(command, true, Some(state("switched", false, 0)))],
        "get_state" => vec![response(command, true, Some(state(SESSION, true, 0)))],
        "abort" => {
            aborted.store(true, Ordering::SeqCst);
            vec![response(command, true, None)]
        }
        _ => vec![response(command, true, None)],
    });
    let reply = owned3.controller.submit(submission(&owned3, OperationIntent::InterruptThenSubmit)).await;
    assert_eq!(reply.result.outcome, SendOutcome::PartialInterruption);
    assert_eq!(reply.result.interruption.unwrap().cancellation, CancellationOutcome::Established);
    assert_eq!(reply.result.interruption.unwrap().replacement, None);
    assert!(!owned3.opened.wire.kinds().contains(&"prompt".to_string()), "never sent into the new session");
}

/// The adapter against the installed Pi, over a real `pi --mode rpc` child.
///
/// `real_` tier: opt-in with `CLAUDINE_CONTRACT_REAL=1` and `pi` on `PATH`
/// (`just test-real real_pi_protocol` in `claudine/`). Pi runs the
/// deterministic `tests/fixtures/steering/pi-probe.ts` model in a disposable
/// agent directory. The shipped policy blocks this profile, so these tests
/// drive the adapter through a controller with routable eligibility; they are
/// evidence about the adapter's protocol handling, not an activation.
#[cfg(feature = "real-tests")]
mod real_pi_protocol {
    use std::io::{BufRead, BufReader};
    use std::path::{Path, PathBuf};
    use std::process::{Child, Command, Stdio};

    use super::*;

    struct RealPi {
        owned: Owned,
        child: Child,
        probe: PathBuf,
        _root: tempfile::TempDir,
    }

    impl Drop for RealPi {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    fn real_pi() -> Option<PathBuf> {
        if std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() != Ok("1") {
            eprintln!("skipping real_pi_protocol (set CLAUDINE_CONTRACT_REAL=1 to run)");
            return None;
        }
        which::which("pi").ok().or_else(|| {
            eprintln!("skipping real_pi_protocol (binary `pi` not on PATH)");
            None
        })
    }

    /// A real Pi owned exactly as the wrapper owns one: the session holds
    /// stdin, a forwarder hands it every stdout line, and a controller with
    /// routable eligibility drives the adapter.
    fn start(pi: &Path, interrupt: bool) -> RealPi {
        let root = tempfile::tempdir().unwrap();
        let (agent, probe, sessions) = (root.path().join("agent"), root.path().join("probe"), root.path().join("sessions"));
        for dir in [&agent, &probe, &sessions] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let extension = biscuit_test_harness::manifest_dir!().join("tests/fixtures/steering/pi-probe.ts");
        let settings = json!({
            "defaultProvider": "claudine-probe", "defaultModel": "fixture", "defaultThinkingLevel": "off",
            "extensions": [extension], "sessionDir": sessions,
        });
        std::fs::write(agent.join("settings.json"), settings.to_string()).unwrap();
        let mut child = Command::new(pi)
            .args(["--mode", "rpc", "--no-approve"])
            .current_dir(root.path())
            .env("HOME", root.path())
            .env("PI_CODING_AGENT_DIR", &agent)
            .env("PI_OFFLINE", "1")
            .env("PI_TELEMETRY", "0")
            .env("CLAUDINE_PI_PROBE_DIR", &probe)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("launch the installed Pi");

        let session = PiRpcSession::new(None);
        let (controller, audit) = controller_for(&session, interrupt);
        let stdout = child.stdout.take().unwrap();
        let observer = session.clone();
        thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                observer.observe(&line);
            }
        });
        let (early_tx, early) = mpsc::channel();
        let (completion_tx, completion) = mpsc::channel();
        let channels = ControlChannels { early: early_tx, completion: completion_tx };
        StdioControl::open(&session, child.stdin.take().unwrap(), child.id(), channels).unwrap();
        wait_until("Pi to be ready", || session.readiness() == Readiness::Ready);
        let owned = Owned { opened: Opened { session, wire: Wire::default(), early, completion }, controller, _audit: audit };
        RealPi { owned, child, probe, _root: root }
    }

    /// A controller over `session`'s adapter, bound to the session, with an
    /// idle initial state (no task has been submitted yet).
    fn controller_for(session: &PiRpcSession, interrupt: bool) -> (SteeringController, tempfile::TempDir) {
        let audit = tempfile::tempdir().unwrap();
        let config = ControllerConfig {
            execution: ExecutionId::random(),
            wrapper: ProcessStartIdentity::new(std::process::id(), "fixture-start").unwrap(),
            facts: ExecutionFacts {
                provider: Provider::Pi,
                profile_id: Some(super::super::PROFILE_ID.into()),
                os: claudine::steering::host_os(),
                launch_mode: LaunchMode::NonInteractive,
                provider_version: None,
                cwd: None,
                name: None,
            },
            initial_state: ExecutionState::Idle,
            eligibility: routable(interrupt),
            audit: SteeringAuditLog::at(audit.path()),
        };
        let controller = SteeringController::spawn(config, session.steering_executor().unwrap());
        session.bind_controller(controller.clone());
        (controller, audit)
    }

    fn marker(pi: &RealPi, name: &str) -> PathBuf {
        pi.probe.join(name)
    }

    fn wait_for_marker(pi: &RealPi, name: &str) {
        wait_until(name, || marker(pi, name).is_file());
    }

    fn release(pi: &RealPi, slot: &str) {
        std::fs::write(marker(pi, &format!("release-{slot}")), "release").unwrap();
    }

    fn wait_for_exit(pi: &mut RealPi) -> std::process::ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            if let Some(status) = pi.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "Pi did not exit after the run settled");
            thread::sleep(Duration::from_millis(20));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn real_pi_protocol_steers_a_working_turn_then_settles_and_closes() {
        let Some(binary) = real_pi() else { return };
        let mut pi = start(&binary, false);
        StdioControl::submit(&pi.owned.opened.session, "PROBE_TOOL_BATCH").unwrap();
        wait_for_marker(&pi, "tool-a-started");
        assert_eq!(pi.owned.controller.snapshot().state, ExecutionState::Working);

        let mut steer = submission(&pi.owned, OperationIntent::SteerActiveTurn);
        steer.request.message = SteeringMessage::new("STEERING_NONCE").unwrap();
        let reply = pi.owned.controller.submit(steer).await;
        assert_eq!(reply.result.outcome, SendOutcome::Queued, "{:?}", reply.detail);
        assert!(!marker(&pi, "tool-a-finished").is_file(), "acceptance precedes tool completion");
        release(&pi, "a");
        wait_for_marker(&pi, "tool-b-started");
        assert!(!marker(&pi, "steering-observed").is_file(), "steering waits for the whole batch");
        release(&pi, "b");
        wait_for_marker(&pi, "steering-observed");

        let status = wait_for_exit(&mut pi);
        assert!(status.success(), "the settled one-shot run closed stdin and Pi exited cleanly: {status:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn real_pi_protocol_starts_an_idle_turn_with_a_prompt() {
        let Some(binary) = real_pi() else { return };
        let mut pi = start(&binary, false);
        let mut idle = submission(&pi.owned, OperationIntent::StartIdleTurn);
        idle.request.message = SteeringMessage::new("TEMPLATE_NONCE").unwrap();
        let reply = pi.owned.controller.submit(idle).await;
        assert_eq!(reply.result.outcome, SendOutcome::Accepted, "{:?}", reply.detail);
        wait_for_marker(&pi, "template-observed");
        // The idle prompt's turn settles like any other and ends the run.
        assert!(wait_for_exit(&mut pi).success());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn real_pi_protocol_interrupts_with_consent_and_keeps_the_phases() {
        let Some(binary) = real_pi() else { return };
        let mut pi = start(&binary, true);
        StdioControl::submit(&pi.owned.opened.session, "PROBE_TOOL_BATCH").unwrap();
        wait_for_marker(&pi, "tool-a-started");
        let mut interrupt = submission(&pi.owned, OperationIntent::InterruptThenSubmit);
        interrupt.request.message = SteeringMessage::new("TEMPLATE_NONCE").unwrap();
        let reply = pi.owned.controller.submit(interrupt).await;
        assert_eq!(reply.result.outcome, SendOutcome::Accepted, "{:?}", reply.detail);
        let phases = reply.result.interruption.unwrap();
        assert_eq!(phases.cancellation, CancellationOutcome::Established);
        assert_eq!(phases.replacement, Some(SendOutcome::Accepted));
        wait_for_marker(&pi, "template-observed");
        assert!(!marker(&pi, "tool-a-finished").is_file(), "the aborted tool never finished normally");
        assert!(wait_for_exit(&mut pi).success());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn a_delivery_and_the_settlement_decision_never_interleave() {
    let owned = owned(false);
    // Pi settles while the steer is between its state check and its answer.
    let session = owned.opened.session.clone();
    let _pi = FakePi::start(&owned.opened, move |command| match command["type"].as_str().unwrap() {
        "get_state" => vec![response(command, true, Some(state(SESSION, command["id"] == "claudine-1", 0)))],
        "steer" => {
            session.observe(r#"{"type":"agent_settled"}"#);
            thread::sleep(Duration::from_millis(50));
            vec![response(command, true, None)]
        }
        _ => Vec::new(),
    });
    let reply = owned.controller.submit(submission(&owned, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Queued);
    wait_until("stdin to close", || closed(&owned.opened));
    assert_eq!(
        owned.opened.wire.kinds(),
        ["get_state", "get_state", "steer", "get_state"],
        "the settlement check waits for the delivery to finish"
    );
}
