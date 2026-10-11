//! Deterministic tests of the Codex app-server owner over an in-memory wire.
//!
//! [`Wire`] records what the session writes; a scripted fake Codex answers
//! each request by feeding messages back through [`StdioControl::observe`],
//! exactly as the stdout forwarder does. Executor tests go through a real
//! [`SteeringController`], so bounds, auditing, and route selection run as in
//! production. The real-provider counterparts are `real_codex_protocol`
//! below and `tests/real/real_codex_app_server.rs`.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use claudine::provider::Provider;
use claudine::steering::audit::SteeringAuditLog;
use claudine::steering::contract::{
    CancellationOutcome, InterruptionConsent, SendOutcome, SteeringMessage, SteeringOrigin, SteeringRequest,
};
use claudine::steering::controller::{ControllerConfig, DeliveryDeadlines, EligibilityFn, ExecutionFacts, SteeringController, Submission};
use claudine::steering::eligibility::{AutomaticEligibility, Eligibility, ManualEligibility, Route};
use claudine::steering::identity::{ExecutionId, ProcessStartIdentity, RequestId};
use claudine::steering::vocabulary::{CaseSupport, ExecutionState, LaunchMode, OperationIntent, SteeringAvailability};
use claudine::stream::logs::EarlyTermination;
use serde_json::{Value, json};

use super::super::control::{ControlChannels, Readiness, StdioControl};
use super::super::termination::CompletionTermination;
use super::launch::{ManagedLaunch, ThreadRequest, Unmapped, plan};
use super::{CodexAppServerSession, commands};

const THREAD: &str = "01a0eb8a-ecce-7432-81d1-6fb51dba267b";
const TURN: &str = "01a0eb8a-ecfd-74f0-90e9-1710f12cbae5";
const USER_AGENT: &str = "claudine/0.157.1 (Mac OS 27.2.0; arm64) Terminal/1 (claudine; 0.1.0)";
const MESSAGE: &str = "Recheck the failing test ✓ before editing";

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| value.to_string()).collect()
}

// ---------------------------------------------------------------------------
// Mapping an exec argv
// ---------------------------------------------------------------------------

/// A directory inside a Git repository, so exec's trust check passes.
fn repository() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join(".git")).unwrap();
    dir
}

#[test]
fn an_exec_argv_maps_to_an_equivalent_app_server_launch() {
    let repo = repository();
    let args = strings(&[
        "exec", "--json", "-m", "gpt-5.5", "--sandbox", "workspace-write", "-c", "developer_instructions=\"be brief\"",
        "--enable", "unified_exec", "--disable=web_search", "--strict-config", "--ephemeral", "--color", "never",
        "--output-last-message", "/tmp/last.txt",
    ]);
    let launch = plan(&args, repo.path()).unwrap();
    assert_eq!(
        launch.args,
        strings(&[
            "app-server", "--listen", "stdio://", "-c", "developer_instructions=\"be brief\"", "--enable", "unified_exec",
            "--disable", "web_search", "--strict-config",
        ])
    );
    assert_eq!(launch.exec_args, args, "the exec argv is kept as the fallback");
    assert_eq!(launch.last_message, Some(PathBuf::from("/tmp/last.txt")));
    let ThreadRequest::Start { params } = &launch.thread else { panic!("a new thread") };
    assert_eq!(params["model"], "gpt-5.5");
    assert_eq!(params["sandbox"], "workspace-write");
    assert_eq!(params["ephemeral"], true);
    assert_eq!(params["approvalPolicy"], "never", "exec never asks for approval");
    assert_eq!(params["cwd"], repo.path().to_string_lossy().as_ref());
}

#[test]
fn resume_and_bypass_map_and_the_git_check_follows_exec() {
    let outside = tempfile::tempdir().unwrap();
    let resume = plan(&strings(&["exec", "resume", THREAD, "--json", "--skip-git-repo-check"]), outside.path()).unwrap();
    let ThreadRequest::Resume { thread_id, params } = &resume.thread else { panic!("a resumed thread") };
    assert_eq!(thread_id, THREAD);
    assert!(params.get("sandbox").is_none(), "no sandbox option means Codex's configured default, as in exec");

    let bypass = plan(&strings(&["e", "--dangerously-bypass-approvals-and-sandbox"]), outside.path()).unwrap();
    let ThreadRequest::Start { params } = &bypass.thread else { panic!("a new thread") };
    assert_eq!(params["sandbox"], "danger-full-access");

    // exec refuses to run outside a repository without either flag.
    assert_eq!(plan(&strings(&["exec", "--json"]), outside.path()), Err(Unmapped::Untrusted));
    let nested = repository();
    std::fs::create_dir_all(nested.path().join("a/b")).unwrap();
    assert!(plan(&strings(&["exec", "--json"]), &nested.path().join("a/b")).is_ok(), "an ancestor's .git counts");
}

#[test]
fn anything_without_an_exact_equivalent_stays_on_exec() {
    let repo = repository();
    for (args, expected) in [
        (&["exec", "-i", "shot.png"][..], Unmapped::Option("-i".into())),
        (&["exec", "--output-schema", "schema.json"][..], Unmapped::Option("--output-schema".into())),
        (&["exec", "--profile", "work"][..], Unmapped::Option("--profile".into())),
        (&["exec", "--oss"][..], Unmapped::Option("--oss".into())),
        (&["exec", "--add-dir", "/tmp"][..], Unmapped::Option("--add-dir".into())),
        (&["exec", "-C", "/tmp"][..], Unmapped::Option("-C".into())),
        (&["exec", "--approve-for-me"][..], Unmapped::Option("--approve-for-me".into())),
        (&["exec", "summarize the repo"][..], Unmapped::Subcommand("summarize the repo".into())),
        (&["exec", "--json", "--", "text"][..], Unmapped::Option("--".into())),
        (&["exec", "review"][..], Unmapped::Subcommand("review".into())),
        (&["exec", "resume", "--last"][..], Unmapped::Option("resume --last".into())),
        (&["exec", "resume"][..], Unmapped::MissingValue("resume".into())),
        (&["exec", "--model"][..], Unmapped::MissingValue("--model".into())),
        (&["exec", "--sandbox", "yolo"][..], Unmapped::Sandbox("yolo".into())),
        (&["exec", "--sandbox"][..], Unmapped::MissingValue("--sandbox".into())),
        (&["--json"][..], Unmapped::NotExec),
        (&[][..], Unmapped::NotExec),
    ] {
        assert_eq!(plan(&strings(args), repo.path()), Err(expected), "{args:?}");
    }
}

// ---------------------------------------------------------------------------
// The session over an in-memory wire
// ---------------------------------------------------------------------------

/// What the session wrote, one JSON message per line.
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
    fn messages(&self) -> Vec<Value> {
        let bytes = self.0.lock().unwrap().clone();
        String::from_utf8(bytes).unwrap().lines().map(|line| serde_json::from_str(line).unwrap()).collect()
    }

    /// Method names of requests and notifications; answers to server
    /// requests appear as `answer`.
    fn methods(&self) -> Vec<String> {
        self.messages()
            .iter()
            .map(|message| message["method"].as_str().map_or_else(|| "answer".to_string(), str::to_string))
            .collect()
    }
}

struct Opened {
    session: CodexAppServerSession,
    wire: Wire,
    early: Receiver<EarlyTermination>,
    completion: Receiver<CompletionTermination>,
    _last: tempfile::TempDir,
}

fn managed_launch(last_message: Option<PathBuf>) -> ManagedLaunch {
    ManagedLaunch {
        args: strings(&["app-server", "--listen", "stdio://"]),
        exec_args: strings(&["exec", "--json"]),
        thread: ThreadRequest::Start { params: json!({"approvalPolicy": "never"}) },
        last_message,
    }
}

fn open() -> Opened {
    let last = tempfile::tempdir().unwrap();
    let session = CodexAppServerSession::new(managed_launch(Some(last.path().join("last.txt"))));
    let wire = Wire::default();
    let (early_tx, early) = mpsc::channel();
    let (completion_tx, completion) = mpsc::channel();
    session.open_writer(Box::new(wire.clone()), ControlChannels { early: early_tx, completion: completion_tx }).unwrap();
    Opened { session, wire, early, completion, _last: last }
}

fn result(id: &Value, result: Value) -> String {
    json!({"id": id, "result": result}).to_string()
}

fn refusal(id: &Value, message: &str) -> String {
    json!({"id": id, "error": {"code": -32600, "message": message}}).to_string()
}

fn turn(id: &str, status: &str) -> Value {
    json!({"id": id, "items": [], "status": status, "error": null})
}

fn notification(method: &str, params: Value) -> String {
    json!({"method": method, "params": params}).to_string()
}

fn turn_started(id: &str) -> String {
    notification("turn/started", json!({"threadId": THREAD, "turn": turn(id, "inProgress")}))
}

fn turn_completed(id: &str, status: &str) -> String {
    notification("turn/completed", json!({"threadId": THREAD, "turn": turn(id, status)}))
}

fn thread_status(kind: &str) -> Value {
    json!({"thread": {"id": THREAD, "status": {"type": kind}}})
}

/// Answers `initialize` and the thread request.
fn ready(opened: &Opened) {
    let messages = opened.wire.messages();
    assert_eq!(messages[0], commands::initialize(), "initialize is the first message");
    opened.session.observe(&result(&messages[0]["id"], json!({"userAgent": USER_AGENT})));
    let messages = opened.wire.messages();
    assert_eq!(messages[1], commands::initialized());
    assert_eq!(messages[2]["method"], "thread/start");
    assert_eq!(messages[2]["params"], json!({"approvalPolicy": "never"}));
    opened.session.observe(&result(&messages[2]["id"], json!({"thread": {"id": THREAD, "status": {"type": "idle"}}})));
    assert_eq!(opened.session.readiness(), Readiness::Ready);
}

/// Codex as a script: `answer` returns the lines Codex emits for one message.
struct FakeCodex {
    stop: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

impl FakeCodex {
    fn start(opened: &Opened, answer: impl Fn(&Value) -> Vec<String> + Send + 'static) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let (session, wire, stopped) = (opened.session.clone(), opened.wire.clone(), Arc::clone(&stop));
        // Taken now, not in the thread: a message written before the thread
        // runs must still be answered.
        let mut seen = wire.messages().len();
        let handle = thread::spawn(move || {
            while !stopped.load(Ordering::SeqCst) {
                let messages = wire.messages();
                for message in &messages[seen..] {
                    for line in answer(message) {
                        session.observe(&line);
                    }
                }
                seen = messages.len();
                thread::sleep(Duration::from_millis(2));
            }
        });
        Self { stop, handle: Some(handle) }
    }
}

impl Drop for FakeCodex {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
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

fn method(message: &Value) -> &str {
    message["method"].as_str().unwrap_or("")
}

#[test]
fn readiness_initializes_then_starts_the_thread() {
    let opened = open();
    assert_eq!(opened.session.readiness(), Readiness::Pending);
    ready(&opened);
    assert_eq!(opened.session.inner.thread_id().as_deref(), Some(THREAD));
    assert!(!opened.session.abandon_readiness("late"), "a ready child stays ready");
    assert_eq!(
        opened.session.launch_args().unwrap(),
        strings(&["app-server", "--listen", "stdio://"]),
        "the child runs the app-server argv"
    );
    assert_eq!(opened.session.fallback().unwrap().args, strings(&["exec", "--json"]));
}

#[test]
fn a_failed_or_unreadable_handshake_is_never_ready() {
    let initialize_id = json!(commands::INITIALIZE_ID);
    let thread_id = json!(commands::THREAD_ID);
    for (label, lines) in [
        ("initialize refused", vec![refusal(&initialize_id, "unsupported client")]),
        ("no user agent", vec![result(&initialize_id, json!({}))]),
        ("initialize unreadable", vec![json!({"id": commands::INITIALIZE_ID}).to_string()]),
        ("thread refused", vec![result(&initialize_id, json!({"userAgent": USER_AGENT})), refusal(&thread_id, "no model")]),
        (
            "thread without id",
            vec![result(&initialize_id, json!({"userAgent": USER_AGENT})), result(&thread_id, json!({"thread": {}}))],
        ),
    ] {
        let opened = open();
        for line in lines {
            opened.session.observe(&line);
        }
        assert!(matches!(opened.session.readiness(), Readiness::Failed(_)), "{label}");
        assert!(!opened.wire.methods().contains(&"turn/start".to_string()), "{label}: no task");
    }

    let opened = open();
    assert!(opened.session.abandon_readiness("timed out"));
    // A late answer cannot revive an abandoned child.
    opened.session.observe(&result(&initialize_id, json!({"userAgent": USER_AGENT})));
    opened.session.observe(&result(&thread_id, json!({"thread": {"id": THREAD}})));
    assert_eq!(opened.session.readiness(), Readiness::Failed("timed out".into()));
}

#[test]
fn the_task_is_one_turn_start_with_the_original_text() {
    let opened = open();
    ready(&opened);
    let task = "- a Markdown bullet first\n\nthen @file-looking text and ✓ unicode";
    opened.session.submit(task).unwrap();
    let start = opened.wire.messages().last().unwrap().clone();
    assert_eq!(start, commands::turn_start(commands::TASK_REQUEST_ID, THREAD, task, commands::TASK_REQUEST_ID));
    assert_eq!(start["params"]["input"][0]["text"], task);
}

#[test]
fn server_requests_are_answered_as_exec_answers_them() {
    let opened = open();
    ready(&opened);
    for (id, method) in [
        (json!(0), "item/commandExecution/requestApproval"),
        (json!(1), "item/fileChange/requestApproval"),
        (json!("r"), "item/tool/requestUserInput"),
        (json!(3), "item/permissions/requestApproval"),
        (json!(4), "item/tool/call"),
        (json!(5), "currentTime/read"),
        (json!(6), "a/future/request"),
    ] {
        opened.session.observe(&json!({"id": id, "method": method, "params": {"threadId": THREAD}}).to_string());
        let answer = opened.wire.messages().last().unwrap().clone();
        assert_eq!(answer, commands::not_supported(&id, method), "{method}");
        assert!(answer.get("result").is_none(), "{method}: never approved");
    }
    opened.session.observe(&json!({"id": 7, "method": "mcpServer/elicitation/request", "params": {}}).to_string());
    assert_eq!(opened.wire.messages().last().unwrap()["result"]["action"], "cancel");
    assert!(opened.early.try_recv().is_err(), "nothing so far ends the run");

    // A request that cannot be read cannot be answered.
    let written = opened.wire.messages().len();
    opened.session.observe(r#"{"id":null,"method":"item/tool/requestUserInput","params":{}}"#);
    let Ok(EarlyTermination::InputRequired { message }) = opened.early.try_recv() else { panic!("input_required") };
    assert!(message.contains("could not read"), "{message}");
    assert_eq!(opened.wire.messages().len(), written, "no answer is invented");
}

#[test]
fn an_input_required_run_is_never_ended_gracefully() {
    let opened = open();
    ready(&opened);
    let _codex = FakeCodex::start(&opened, |message| match method(message) {
        "thread/read" => vec![result(&message["id"], thread_status("idle"))],
        _ => Vec::new(),
    });
    opened.session.observe(r#"{"id":null,"method":"item/tool/requestUserInput","params":{}}"#);
    opened.session.observe(&turn_started(TURN));
    opened.session.observe(&turn_completed(TURN, "completed"));
    thread::sleep(Duration::from_millis(100));
    assert!(!closed(&opened), "stdin stays open while the run is failed as input_required");
}

// ---------------------------------------------------------------------------
// Settlement
// ---------------------------------------------------------------------------

#[test]
fn a_completed_idle_run_writes_the_final_message_and_closes_stdin_once() {
    let opened = open();
    ready(&opened);
    let _codex = FakeCodex::start(&opened, |message| match method(message) {
        "thread/read" => vec![result(&message["id"], thread_status("idle"))],
        _ => Vec::new(),
    });
    opened.session.observe(&turn_started(TURN));
    let agent = |text: &str| notification("item/completed", json!({"threadId": THREAD, "turnId": TURN, "item": {"type": "agentMessage", "id": "m", "text": text}}));
    opened.session.observe(&agent("Interim note."));
    opened.session.observe(&agent("Final answer."));
    assert!(!closed(&opened), "a running turn keeps stdin open");
    opened.session.observe(&turn_completed(TURN, "completed"));
    wait_until("stdin to close", || closed(&opened));
    let reads = opened.wire.methods().iter().filter(|method| *method == "thread/read").count();
    assert_eq!(reads, 1);
    let last = opened.session.inner.launch.last_message.clone().unwrap();
    assert_eq!(std::fs::read_to_string(last).unwrap(), "Final answer.", "exec's final-message file");
    // Codex exited normally: the lingering-child completion is never needed.
    drop(opened.completion);
}

#[test]
fn work_after_the_turn_keeps_the_run_open() {
    // Codex still reports the thread active (a steering turn started).
    let opened = open();
    ready(&opened);
    let _codex = FakeCodex::start(&opened, |message| match method(message) {
        "thread/read" => vec![result(&message["id"], thread_status("active"))],
        _ => Vec::new(),
    });
    opened.session.observe(&turn_started(TURN));
    opened.session.observe(&turn_completed(TURN, "completed"));
    wait_until("the settlement check", || opened.wire.methods().contains(&"thread/read".to_string()));
    thread::sleep(Duration::from_millis(50));
    assert!(!closed(&opened));
    assert!(!opened.session.inner.state.lock().unwrap().closing, "steering is accepted again");

    // A turn started while the check was being answered.
    let opened = open();
    ready(&opened);
    let session = opened.session.clone();
    let _codex = FakeCodex::start(&opened, move |message| match method(message) {
        "thread/read" => {
            session.observe(&turn_started("second"));
            session.observe(&turn_completed("second", "completed"));
            vec![result(&message["id"], thread_status("idle"))]
        }
        _ => Vec::new(),
    });
    opened.session.observe(&turn_started(TURN));
    opened.session.observe(&turn_completed(TURN, "completed"));
    // The first check sees the new turn and stays open; the second turn's
    // own completion then closes the run.
    wait_until("stdin to close", || closed(&opened));
    let reads = opened.wire.methods().iter().filter(|method| *method == "thread/read").count();
    assert_eq!(reads, 2, "the stale check did not close the run");
}

#[test]
fn a_refused_task_ends_the_run_and_an_unconfirmed_state_ends_it_with_a_warning() {
    let opened = open();
    ready(&opened);
    let _codex = FakeCodex::start(&opened, |message| match method(message) {
        "thread/read" => vec![result(&message["id"], thread_status("idle"))],
        _ => Vec::new(),
    });
    opened.session.submit("task").unwrap();
    opened.session.observe(&refusal(&json!(commands::TASK_REQUEST_ID), "input too large"));
    wait_until("stdin to close", || closed(&opened));

    let opened = open();
    ready(&opened);
    let _codex = FakeCodex::start(&opened, |message| match method(message) {
        "thread/read" => vec![refusal(&message["id"], "thread not loaded")],
        _ => Vec::new(),
    });
    opened.session.observe(&turn_started(TURN));
    opened.session.observe(&turn_completed(TURN, "completed"));
    wait_until("stdin to close", || closed(&opened));
}

#[test]
fn finishing_releases_everything_waiting_on_the_child() {
    let opened = open();
    let waiter = opened.session.inner.request(|id| commands::thread_read(id, THREAD)).unwrap();
    opened.session.finish();
    assert!(matches!(opened.session.readiness(), Readiness::Failed(_)), "an exited child never becomes ready");
    assert!(matches!(waiter.blocking_recv(), Ok(Err(_))), "a waiting request learns the child exited");
    assert!(closed(&opened));
}

#[test]
fn only_responses_to_the_sessions_own_requests_are_control_replies() {
    let session = CodexAppServerSession::new(managed_launch(None));
    for reply in [
        r#"{"id":"claudine-3","result":{"turnId":"t"}}"#,
        r#"{"error":{"code":-32600,"message":"no active turn to steer"},"id":"claudine-4"}"#,
        r#"  {"id":"claudine-thread","result":{"thread":{"id":"t"}}}  "#,
    ] {
        assert!(session.is_control_reply(reply), "{reply}");
    }
    for activity in [
        r#"{"method":"turn/started","params":{"threadId":"t","turn":{"id":"u"}}}"#,
        r#"{"method":"item/completed","params":{"item":{"type":"agentMessage","id":"m","text":"the \"id\" was ok"}}}"#,
        r#"{"id":0,"method":"item/commandExecution/requestApproval","params":{}}"#,
        "not json but mentions \"id\"",
        "",
    ] {
        assert!(!session.is_control_reply(activity), "{activity}");
    }
}

// ---------------------------------------------------------------------------
// Steering through the owner's controller
// ---------------------------------------------------------------------------

fn mechanism(id: &str) -> &'static claudine::steering::vocabulary::SteeringMechanism {
    claudine::steering::facts(Provider::Codex).mechanisms.iter().find(|m| m.id == id).unwrap()
}

/// Eligibility offering Codex's researched mechanism for each state, so the
/// adapter is exercised independently of the shipped activation policy.
fn routable(interrupt: bool) -> EligibilityFn {
    Arc::new(move |session| {
        let (id, support) = match (session.state, interrupt) {
            (ExecutionState::Working, false) => ("app-server-steer", CaseSupport::NonInterrupting),
            (ExecutionState::Working, true) => ("app-server-interrupt-then-start", CaseSupport::InterruptionRequired),
            _ => ("app-server-turn-start", CaseSupport::NonInterrupting),
        };
        let route = Route { mechanism: mechanism(id), adapter: claudine::steering::adapters::CODEX_APP_SERVER, support };
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

fn controller_for(session: &CodexAppServerSession, interrupt: bool, state: ExecutionState) -> (SteeringController, tempfile::TempDir) {
    let audit = tempfile::tempdir().unwrap();
    let config = ControllerConfig {
        execution: ExecutionId::random(),
        wrapper: ProcessStartIdentity::new(std::process::id(), "fixture-start").unwrap(),
        facts: ExecutionFacts {
            provider: Provider::Codex,
            profile_id: Some(super::PROFILE_ID.into()),
            os: claudine::steering::host_os(),
            launch_mode: LaunchMode::NonInteractive,
            provider_version: None,
            cwd: None,
            name: None,
        },
        initial_state: state,
        eligibility: routable(interrupt),
        audit: SteeringAuditLog::at(audit.path()),
    };
    let controller = SteeringController::spawn(config, session.steering_executor().unwrap());
    session.bind_controller(controller.clone());
    (controller, audit)
}

/// A ready session with a running turn, owned by a controller.
fn owned(interrupt: bool) -> Owned {
    let opened = open();
    let (controller, audit) = controller_for(&opened.session, interrupt, ExecutionState::Working);
    ready(&opened);
    opened.session.observe(&turn_started(TURN));
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

fn sent(owned: &Owned, name: &str) -> Vec<Value> {
    owned.opened.wire.messages().into_iter().filter(|message| method(message) == name).collect()
}

#[tokio::test(flavor = "multi_thread")]
async fn readiness_binds_the_thread_version_process_state() {
    let owned = owned(false);
    let snapshot = owned.controller.snapshot();
    assert_eq!(snapshot.target.conversation.as_deref(), Some(THREAD));
    assert_eq!(snapshot.facts.profile_id.as_deref(), Some("managed-app-server"));
    assert_eq!(snapshot.facts.provider_version.as_deref(), Some("0.157.1"), "read from initialize");
    assert_eq!(snapshot.state, ExecutionState::Working);
    owned.opened.session.observe(&turn_completed(TURN, "completed"));
    assert_eq!(owned.controller.snapshot().state, ExecutionState::Idle);
}

#[tokio::test(flavor = "multi_thread")]
async fn a_working_turn_is_steered_with_its_exact_turn_as_the_guard() {
    let owned = owned(false);
    let _codex = FakeCodex::start(&owned.opened, |message| match method(message) {
        "turn/steer" => vec![result(&message["id"], json!({"turnId": message["params"]["expectedTurnId"]}))],
        _ => Vec::new(),
    });
    let submission = submission(&owned, OperationIntent::SteerActiveTurn);
    let request_id = submission.request.id.to_string();
    let reply = owned.controller.submit(submission).await;
    assert_eq!(reply.result.outcome, SendOutcome::Queued, "{:?}", reply.detail);
    assert_eq!(reply.result.mechanism, Some("app-server-steer"));
    let steers = sent(&owned, "turn/steer");
    assert_eq!(steers.len(), 1);
    assert_eq!(steers[0]["params"]["threadId"], THREAD);
    assert_eq!(steers[0]["params"]["expectedTurnId"], TURN, "Codex's atomic target guard");
    assert_eq!(steers[0]["params"]["input"][0]["text"], MESSAGE, "the original bytes reach Codex");
    assert_eq!(steers[0]["params"]["clientUserMessageId"], request_id.as_str(), "correlated with the request");
    assert!(sent(&owned, "thread/read").is_empty(), "the guard is the check; no separate state read");
}

#[tokio::test(flavor = "multi_thread")]
async fn refusals_mismatches_and_silence_are_reported_as_established() {
    // Codex refuses: the turn ended or changed after it was observed.
    let owned1 = owned(false);
    let _codex = FakeCodex::start(&owned1.opened, |message| {
        vec![refusal(&message["id"], "expected active turn id `x` but found `y`")]
    });
    let reply = owned1.controller.submit(submission(&owned1, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Refused);
    assert!(reply.detail.unwrap().as_str().contains("was not accepted"));

    // An answer naming another turn is not proof for this one.
    let owned2 = owned(false);
    let _codex2 = FakeCodex::start(&owned2.opened, |message| vec![result(&message["id"], json!({"turnId": "another"}))]);
    let reply = owned2.controller.submit(submission(&owned2, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unknown);

    // The child exits while the steer waits.
    let owned3 = owned(false);
    let session = owned3.opened.session.clone();
    let _codex3 = FakeCodex::start(&owned3.opened, move |_| {
        session.finish();
        Vec::new()
    });
    let reply = owned3.controller.submit(submission(&owned3, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unknown, "{:?}", reply.detail);

    // No running turn: nothing is sent.
    // The completed turn starts a settlement check that holds the mutation
    // lock until Codex answers its `thread/read`; unanswered, it would wait
    // out SETTLEMENT_CHECK_DEADLINE (five seconds). Codex reports the thread
    // still active, so the check reopens the run (`closing = false`).
    let owned4 = owned(false);
    let _codex4 = FakeCodex::start(&owned4.opened, |message| match method(message) {
        "thread/read" => vec![result(&message["id"], thread_status("active"))],
        _ => Vec::new(),
    });
    owned4.opened.session.observe(&turn_completed(TURN, "completed"));
    wait_until("the settlement check to reopen the run", || {
        sent(&owned4, "thread/read").len() == 1 && !owned4.opened.session.inner.state.lock().unwrap().closing
    });
    owned4.controller.set_state(ExecutionState::Working);
    let reply = owned4.controller.submit(submission(&owned4, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert!(sent(&owned4, "turn/steer").is_empty());
}

/// Automatic, for its two-second deadline.
#[tokio::test(flavor = "multi_thread")]
async fn an_unanswered_steer_is_unknown_and_never_resent() {
    let owned = owned(false);
    let _codex = FakeCodex::start(&owned.opened, |_| Vec::new());
    let mut silent = submission(&owned, OperationIntent::SteerActiveTurn);
    silent.request.origin = SteeringOrigin::Automatic;
    silent.expected = None;
    let reply = owned.controller.submit(silent).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unknown);
    thread::sleep(Duration::from_millis(100));
    assert_eq!(sent(&owned, "turn/steer").len(), 1, "an unanswered steer is never resent");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_settling_run_refuses_new_steering() {
    let owned = owned(false);
    owned.opened.session.inner.state.lock().unwrap().closing = true;
    let reply = owned.controller.submit(submission(&owned, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert_eq!(owned.opened.wire.methods(), ["initialize", "initialized", "thread/start"], "nothing else was written");
}

#[tokio::test(flavor = "multi_thread")]
async fn an_idle_thread_takes_a_turn_after_a_fresh_check_and_stays_open_for_it() {
    let opened = open();
    let (controller, _audit) = controller_for(&opened.session, false, ExecutionState::Idle);
    ready(&opened);
    let owned = Owned { opened, controller, _audit };
    let session = owned.opened.session.clone();
    let _codex = FakeCodex::start(&owned.opened, move |message| match method(message) {
        "thread/read" => vec![result(&message["id"], thread_status("idle"))],
        "turn/start" => {
            let answer = result(&message["id"], json!({"turn": turn("steered-turn", "inProgress")}));
            session.observe(&answer);
            vec![turn_started("steered-turn")]
        }
        _ => Vec::new(),
    });
    let reply = owned.controller.submit(submission(&owned, OperationIntent::StartIdleTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted, "{:?}", reply.detail);
    assert_eq!(owned.opened.wire.methods(), ["initialize", "initialized", "thread/start", "thread/read", "turn/start"]);
    assert_eq!(owned.controller.snapshot().state, ExecutionState::Working);
    assert!(!closed(&owned.opened), "the new turn keeps the run open");

    // Codex says the thread is busy: nothing is sent.
    let opened = open();
    let (controller, _audit) = controller_for(&opened.session, false, ExecutionState::Idle);
    ready(&opened);
    let busy = Owned { opened, controller, _audit };
    let _codex = FakeCodex::start(&busy.opened, |message| match method(message) {
        "thread/read" => vec![result(&message["id"], thread_status("active"))],
        _ => Vec::new(),
    });
    let reply = busy.controller.submit(submission(&busy, OperationIntent::StartIdleTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Unavailable);
    assert!(sent(&busy, "turn/start").is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn consented_interruption_interrupts_waits_then_starts_the_replacement() {
    let owned = owned(true);
    let session = owned.opened.session.clone();
    let _codex = FakeCodex::start(&owned.opened, move |message| match method(message) {
        "turn/interrupt" => {
            session.observe(&result(&message["id"], json!({})));
            vec![turn_completed(TURN, "interrupted")]
        }
        "turn/start" => vec![result(&message["id"], json!({"turn": turn("replacement", "inProgress")}))],
        _ => Vec::new(),
    });
    let reply = owned.controller.submit(submission(&owned, OperationIntent::InterruptThenSubmit)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Accepted, "{:?}", reply.detail);
    let phases = reply.result.interruption.unwrap();
    assert_eq!(phases.cancellation, CancellationOutcome::Established);
    assert_eq!(phases.replacement, Some(SendOutcome::Accepted));
    let interrupt = &sent(&owned, "turn/interrupt")[0];
    assert_eq!(interrupt["params"], json!({"threadId": THREAD, "turnId": TURN}));
    assert_eq!(sent(&owned, "turn/start")[0]["params"]["input"][0]["text"], MESSAGE);
    assert!(!closed(&owned.opened), "the replacement turn keeps the run open");
}

#[tokio::test(flavor = "multi_thread")]
async fn interruption_phases_stay_separate_when_one_fails() {
    // Replacement refused after a confirmed cancellation.
    let owned1 = owned(true);
    let session = owned1.opened.session.clone();
    let _codex = FakeCodex::start(&owned1.opened, move |message| match method(message) {
        "turn/interrupt" => {
            session.observe(&result(&message["id"], json!({})));
            vec![turn_completed(TURN, "interrupted")]
        }
        _ => vec![refusal(&message["id"], "busy")],
    });
    let reply = owned1.controller.submit(submission(&owned1, OperationIntent::InterruptThenSubmit)).await;
    assert_eq!(reply.result.outcome, SendOutcome::PartialInterruption);
    assert_eq!(reply.result.interruption.unwrap().replacement, Some(SendOutcome::Refused));

    // Interrupt refused: no replacement is sent.
    let owned2 = owned(true);
    let _codex2 = FakeCodex::start(&owned2.opened, |message| vec![refusal(&message["id"], "no active turn")]);
    let reply = owned2.controller.submit(submission(&owned2, OperationIntent::InterruptThenSubmit)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Refused);
    assert!(sent(&owned2, "turn/start").is_empty());

    // The turn finished on its own before the interrupt took effect: it was
    // not cancelled, and nothing replaces it.
    let owned3 = owned(true);
    let session = owned3.opened.session.clone();
    let _codex3 = FakeCodex::start(&owned3.opened, move |message| match method(message) {
        "turn/interrupt" => {
            session.observe(&result(&message["id"], json!({})));
            vec![turn_completed(TURN, "completed")]
        }
        _ => Vec::new(),
    });
    let reply = owned3.controller.submit(submission(&owned3, OperationIntent::InterruptThenSubmit)).await;
    assert_eq!(reply.result.interruption.unwrap().cancellation, CancellationOutcome::Refused);
    assert!(sent(&owned3, "turn/start").is_empty());

    // Interrupt acknowledged but no completion arrives in time: unknown. The
    // executor is driven directly so the cancellation deadline can be short
    // (the controller's is `CONSENTED_CANCELLATION_DEADLINE`, ten seconds).
    // One second is ~500x FakeCodex's 2 ms poll, so a loaded runner still gets
    // the acknowledgement in before the deadline; the error text proves the
    // deadline was hit while waiting for completion, not while awaiting the
    // interrupt answer (which would also report `Unknown`).
    let owned4 = owned(true);
    let _codex4 = FakeCodex::start(&owned4.opened, |message| match method(message) {
        "turn/interrupt" => vec![result(&message["id"], json!({}))],
        _ => Vec::new(),
    });
    let cancellation = tokio::time::Instant::now() + Duration::from_secs(1);
    let deadlines = DeliveryDeadlines { cancellation: Some(cancellation), acceptance: cancellation + Duration::from_secs(1) };
    let request = submission(&owned4, OperationIntent::InterruptThenSubmit).request;
    let report = super::executor::deliver(&owned4.opened.session.inner, request, "fixture", deadlines).await;
    assert_eq!(report.result.interruption.unwrap().cancellation, CancellationOutcome::Unknown);
    assert!(
        report.error.as_deref().is_some_and(|error| error.contains("cancellation was not confirmed before the deadline")),
        "{:?}",
        report.error
    );
    assert!(sent(&owned4, "turn/start").is_empty(), "no replacement without confirmed cancellation");
}

#[tokio::test(flavor = "multi_thread")]
async fn a_delivery_and_the_settlement_decision_never_interleave() {
    let owned = owned(false);
    // The turn completes while the steer waits for its answer.
    let session = owned.opened.session.clone();
    let _codex = FakeCodex::start(&owned.opened, move |message| match method(message) {
        "turn/steer" => {
            session.observe(&turn_completed(TURN, "completed"));
            thread::sleep(Duration::from_millis(50));
            vec![result(&message["id"], json!({"turnId": TURN}))]
        }
        "thread/read" => vec![result(&message["id"], thread_status("idle"))],
        _ => Vec::new(),
    });
    let reply = owned.controller.submit(submission(&owned, OperationIntent::SteerActiveTurn)).await;
    assert_eq!(reply.result.outcome, SendOutcome::Queued);
    wait_until("stdin to close", || closed(&owned.opened));
    assert_eq!(
        owned.opened.wire.methods(),
        ["initialize", "initialized", "thread/start", "turn/steer", "thread/read"],
        "the settlement check waits for the delivery to finish"
    );
}

/// The session against the installed Codex, over a real `codex app-server`
/// child and a scripted model (`tests/common/codex_model.rs`).
///
/// `real_` tier: opt-in with `CLAUDINE_CONTRACT_REAL=1` and `codex` on
/// `PATH` (`just test-real real_codex_protocol` in `claudine/`). Codex runs
/// with a disposable `CODEX_HOME`, so no user configuration, credential,
/// history, or session is involved. These drive the adapter through a
/// controller with routable eligibility, independent of the shipped policy.
#[cfg(feature = "real-tests")]
mod real_codex_protocol {
    use std::io::{BufRead, BufReader};
    use std::process::{Child, Command, Stdio};

    use super::super::codex_model::{Reply, ScriptedModel};
    use super::*;

    struct RealCodex {
        owned: Owned,
        child: Child,
        model: ScriptedModel,
        root: tempfile::TempDir,
    }

    impl Drop for RealCodex {
        fn drop(&mut self) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }

    fn real_codex() -> Option<PathBuf> {
        if std::env::var("CLAUDINE_CONTRACT_REAL").as_deref() != Ok("1") {
            eprintln!("skipping real_codex_protocol (set CLAUDINE_CONTRACT_REAL=1 to run)");
            return None;
        }
        which::which("codex").ok().or_else(|| {
            eprintln!("skipping real_codex_protocol (binary `codex` not on PATH)");
            None
        })
    }

    /// A real Codex owned exactly as the wrapper owns one: the session maps
    /// an exec argv, holds stdin, and sees every stdout line; a controller
    /// with routable eligibility drives the adapter.
    fn start(codex: &Path, model: ScriptedModel, interrupt: bool) -> RealCodex {
        start_in(codex, model, interrupt, tempfile::tempdir().unwrap(), &[])
    }

    /// [`start`] in an existing disposable root, with `leading` exec
    /// arguments after `exec` (for example `resume <thread>`).
    fn start_in(codex: &Path, model: ScriptedModel, interrupt: bool, root: tempfile::TempDir, leading: &[&str]) -> RealCodex {
        let (home, work) = (root.path().join("codex-home"), root.path().join("work"));
        for dir in [&home, &work] {
            std::fs::create_dir_all(dir).unwrap();
        }
        let mut exec_args = strings(&["exec"]);
        exec_args.extend(strings(leading));
        exec_args.extend(strings(&["--json", "--skip-git-repo-check", "--sandbox", "danger-full-access"]));
        exec_args.extend(model.codex_args());
        exec_args.extend(strings(&["--output-last-message"]));
        exec_args.push(root.path().join("last.txt").to_string_lossy().into_owned());
        let session = CodexAppServerSession::for_exec_args(&exec_args, &work).expect("a mappable exec argv");
        let mut child = Command::new(codex)
            .args(session.launch_args().unwrap())
            .current_dir(&work)
            .env("CODEX_HOME", &home)
            .env_remove("CODEX_SQLITE_HOME")
            .env_remove("OPENAI_API_KEY")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("launch the installed Codex");
        let (controller, audit) = controller_for(&session, interrupt, ExecutionState::Idle);
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
        wait_until("Codex to be ready", || session.readiness() == Readiness::Ready);
        let owned = Owned {
            opened: Opened { session, wire: Wire::default(), early, completion, _last: tempfile::tempdir().unwrap() },
            controller,
            _audit: audit,
        };
        RealCodex { owned, child, model, root }
    }

    fn wait_for_exit(codex: &mut RealCodex) -> std::process::ExitStatus {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Some(status) = codex.child.try_wait().unwrap() {
                return status;
            }
            assert!(Instant::now() < deadline, "Codex did not exit after the run settled");
            thread::sleep(Duration::from_millis(20));
        }
    }

    fn wait_for_state(codex: &RealCodex, state: ExecutionState) {
        wait_until("the execution state", || codex.owned.controller.snapshot().state == state);
    }

    /// Replies to the task with a long tool, then answers once a steer
    /// reached the model.
    fn steered_model(nonce: &'static str, tool: &'static str) -> ScriptedModel {
        ScriptedModel::start(Box::new(move |requests: &[String]| {
            let latest = requests.last().unwrap();
            if latest.contains(nonce) {
                Reply::message(format!("ACK {nonce}"))
            } else if requests.len() == 1 {
                Reply::shell(tool)
            } else {
                Reply::message("finished without steering")
            }
        }))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn real_codex_protocol_steers_a_working_turn_at_the_tool_boundary() {
        let Some(binary) = real_codex() else { return };
        let mut codex = start(&binary, steered_model("STEERING_NONCE", "sleep 3"), false);
        assert_eq!(codex.owned.controller.snapshot().facts.provider_version.as_deref().map(|v| !v.is_empty()), Some(true));
        StdioControl::submit(&codex.owned.opened.session, "run the long tool").unwrap();
        wait_for_state(&codex, ExecutionState::Working);
        wait_until("the tool to start", || codex.model.requests().len() == 1);
        thread::sleep(Duration::from_millis(300));

        let mut steer = submission(&codex.owned, OperationIntent::SteerActiveTurn);
        steer.request.message = SteeringMessage::new("STEERING_NONCE").unwrap();
        let reply = codex.owned.controller.submit(steer).await;
        assert_eq!(reply.result.outcome, SendOutcome::Queued, "{:?}", reply.detail);
        assert!(!codex.model.saw("STEERING_NONCE"), "accepted while the tool still runs, before the model saw it");

        let status = wait_for_exit(&mut codex);
        assert!(status.success(), "the settled one-shot run closed stdin and Codex exited cleanly: {status:?}");
        let requests = codex.model.requests();
        assert_eq!(requests.len(), 2, "one turn: the tool round, then the answer");
        assert!(
            requests[1].contains("STEERING_NONCE") && requests[1].contains("function_call_output"),
            "the steer reached the model together with the uncancelled tool's result"
        );
        let last = std::fs::read_to_string(codex.root.path().join("last.txt")).unwrap();
        assert_eq!(last, "ACK STEERING_NONCE", "the final message answers the steer");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn real_codex_protocol_refuses_a_stale_turn_and_sends_nothing() {
        let Some(binary) = real_codex() else { return };
        let codex = start(&binary, steered_model("STALE_NONCE", "sleep 3"), false);
        StdioControl::submit(&codex.owned.opened.session, "run the long tool").unwrap();
        wait_for_state(&codex, ExecutionState::Working);
        wait_until("the tool to start", || codex.model.requests().len() == 1);
        thread::sleep(Duration::from_millis(300));
        // Pretend the owner still believes an earlier turn is running.
        codex.owned.opened.session.inner.state.lock().unwrap().active_turn = Some("an-earlier-turn".into());
        let mut steer = submission(&codex.owned, OperationIntent::SteerActiveTurn);
        steer.request.message = SteeringMessage::new("STALE_NONCE").unwrap();
        let reply = codex.owned.controller.submit(steer).await;
        assert_eq!(reply.result.outcome, SendOutcome::Refused, "{:?}", reply.detail);
        thread::sleep(Duration::from_secs(4));
        assert!(!codex.model.saw("STALE_NONCE"), "a refused steer never reaches the model");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn real_codex_protocol_interrupts_with_consent_and_keeps_the_phases() {
        let Some(binary) = real_codex() else { return };
        let marker = tempfile::tempdir().unwrap();
        let survivor = marker.path().join("survivor");
        let tool: &'static str = Box::leak(format!("sleep 20; touch '{}'", survivor.display()).into_boxed_str());
        let mut codex = start(&binary, steered_model("REPLACEMENT_NONCE", tool), true);
        StdioControl::submit(&codex.owned.opened.session, "run the long tool").unwrap();
        wait_for_state(&codex, ExecutionState::Working);
        wait_until("the tool to start", || codex.model.requests().len() == 1);
        thread::sleep(Duration::from_millis(300));

        let mut interrupt = submission(&codex.owned, OperationIntent::InterruptThenSubmit);
        interrupt.request.message = SteeringMessage::new("REPLACEMENT_NONCE").unwrap();
        let reply = codex.owned.controller.submit(interrupt).await;
        assert_eq!(reply.result.outcome, SendOutcome::Accepted, "{:?}", reply.detail);
        let phases = reply.result.interruption.unwrap();
        assert_eq!(phases.cancellation, CancellationOutcome::Established);
        assert_eq!(phases.replacement, Some(SendOutcome::Accepted));

        assert!(wait_for_exit(&mut codex).success());
        assert!(codex.model.saw("REPLACEMENT_NONCE"), "the replacement reached the model");
        thread::sleep(Duration::from_secs(21));
        assert!(!survivor.exists(), "the interrupted tool was stopped, not left running");
    }

    /// Regression evidence for "never resend": Codex does not deduplicate
    /// `clientUserMessageId`, so the same steer written twice is delivered
    /// twice. The adapter never does this; the test writes it directly.
    #[tokio::test(flavor = "multi_thread")]
    async fn real_codex_protocol_delivers_a_repeated_client_message_id_twice() {
        let Some(binary) = real_codex() else { return };
        let codex = start(&binary, steered_model("DUPLICATE_NONCE", "sleep 3"), false);
        StdioControl::submit(&codex.owned.opened.session, "run the long tool").unwrap();
        wait_until("the tool to start", || codex.model.requests().len() == 1);
        thread::sleep(Duration::from_millis(300));
        let (thread_id, turn_id) = {
            let state = codex.owned.opened.session.inner.state.lock().unwrap();
            (state.thread_id.clone().unwrap(), state.active_turn.clone().unwrap())
        };
        for _ in 0..2 {
            let answer = codex
                .owned
                .opened
                .session
                .inner
                .request(|id| commands::turn_steer(id, &thread_id, &turn_id, "DUPLICATE_NONCE", "same-client-id"))
                .unwrap();
            let result = tokio::time::timeout(Duration::from_secs(10), answer).await.unwrap().unwrap().unwrap();
            assert_eq!(result["turnId"], turn_id.as_str(), "both are accepted");
        }
        wait_until("the model's next request", || codex.model.requests().len() == 2);
        let delivered = codex.model.requests()[1].matches("DUPLICATE_NONCE").count();
        assert_eq!(delivered, 2, "the repeated id was not suppressed");
    }

    /// Regression evidence for settlement: closing stdin while a turn runs
    /// makes the app-server exit 0 at once and abandon the turn, so a clean
    /// exit alone never means the work finished.
    #[cfg(unix)]
    #[tokio::test(flavor = "multi_thread")]
    async fn real_codex_protocol_closing_stdin_mid_turn_abandons_it() {
        let Some(binary) = real_codex() else { return };
        let marker = tempfile::tempdir().unwrap();
        let survivor = marker.path().join("survivor");
        let tool: &'static str = Box::leak(format!("sleep 5; touch '{}'", survivor.display()).into_boxed_str());
        let mut codex = start(&binary, steered_model("UNUSED_NONCE", tool), false);
        StdioControl::submit(&codex.owned.opened.session, "run the long tool").unwrap();
        wait_until("the tool to start", || codex.model.requests().len() == 1);
        thread::sleep(Duration::from_millis(300));
        let closed_at = Instant::now();
        codex.owned.opened.session.inner.writer.lock().unwrap().take();
        let status = wait_for_exit(&mut codex);
        assert!(status.success(), "Codex reports success: {status:?}");
        assert!(closed_at.elapsed() < Duration::from_secs(3), "without finishing the running tool");
        thread::sleep(Duration::from_secs(6));
        assert!(!survivor.exists(), "the abandoned tool did not run to completion");
        assert_eq!(codex.model.requests().len(), 1, "the turn never continued");
    }

    /// A steer sent while the model is generating is answered at once and
    /// reaches the model when that response ends: the turn then continues with
    /// the steer instead of completing.
    #[tokio::test(flavor = "multi_thread")]
    async fn real_codex_protocol_steers_during_generation_at_the_end_of_the_response() {
        let Some(binary) = real_codex() else { return };
        let model = ScriptedModel::start(Box::new(|requests: &[String]| {
            if requests.last().unwrap().contains("GENERATION_NONCE") {
                Reply::message("ACK GENERATION_NONCE")
            } else {
                Reply::Slow { delay: Duration::from_secs(3), reply: Box::new(Reply::message("a slow final answer")) }
            }
        }));
        let mut codex = start(&binary, model, false);
        StdioControl::submit(&codex.owned.opened.session, "answer slowly").unwrap();
        wait_until("generation to start", || codex.model.requests().len() == 1);
        thread::sleep(Duration::from_millis(500));

        let mut steer = submission(&codex.owned, OperationIntent::SteerActiveTurn);
        steer.request.message = SteeringMessage::new("GENERATION_NONCE").unwrap();
        steer.request.origin = SteeringOrigin::Automatic;
        steer.expected = None;
        let reply = codex.owned.controller.submit(steer).await;
        assert_eq!(reply.result.outcome, SendOutcome::Queued, "answered within the automatic deadline: {:?}", reply.detail);
        assert!(!codex.model.saw("GENERATION_NONCE"), "accepted while the response was still generating");
        assert!(wait_for_exit(&mut codex).success());
        let last = std::fs::read_to_string(codex.root.path().join("last.txt")).unwrap();
        assert_eq!(last, "ACK GENERATION_NONCE", "the same turn continued with the steer");
    }

    /// `exec resume <id>` maps to `thread/resume`: a later run continues the
    /// same thread, and the model sees the earlier turn.
    #[tokio::test(flavor = "multi_thread")]
    async fn real_codex_protocol_resumes_a_thread() {
        let Some(binary) = real_codex() else { return };
        let model = |answer: &'static str| ScriptedModel::scripted(vec![Reply::message(answer)], Reply::message(answer));
        let mut first = start(&binary, model("FIRST_ANSWER"), false);
        StdioControl::submit(&first.owned.opened.session, "FIRST_TASK").unwrap();
        assert!(wait_for_exit(&mut first).success());
        let thread = first.owned.opened.session.inner.thread_id().expect("the first run's thread");
        let root = std::mem::replace(&mut first.root, tempfile::tempdir().unwrap());
        drop(first);

        let mut resumed = start_in(&binary, model("SECOND_ANSWER"), false, root, &["resume", &thread]);
        assert_eq!(resumed.owned.opened.session.inner.thread_id().as_deref(), Some(thread.as_str()), "the same thread");
        assert_eq!(resumed.owned.controller.snapshot().target.conversation.as_deref(), Some(thread.as_str()));
        StdioControl::submit(&resumed.owned.opened.session, "SECOND_TASK").unwrap();
        assert!(wait_for_exit(&mut resumed).success());
        let request = resumed.model.requests().pop().unwrap();
        assert!(request.contains("FIRST_TASK") && request.contains("FIRST_ANSWER"), "the resumed thread carries its history");
        assert!(request.contains("SECOND_TASK"));
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn real_codex_protocol_starts_an_idle_turn() {
        let Some(binary) = real_codex() else { return };
        let mut codex = start(&binary, ScriptedModel::scripted(vec![Reply::message("idle reply")], Reply::message("again")), false);
        let thread = codex.owned.opened.session.inner.thread_id().expect("a started thread");
        assert_eq!(codex.owned.controller.snapshot().target.conversation.as_deref(), Some(thread.as_str()));
        let mut idle = submission(&codex.owned, OperationIntent::StartIdleTurn);
        idle.request.message = SteeringMessage::new("IDLE_NONCE").unwrap();
        let reply = codex.owned.controller.submit(idle).await;
        assert_eq!(reply.result.outcome, SendOutcome::Accepted, "{:?}", reply.detail);
        let turns = {
            let state = codex.owned.opened.session.inner.state.lock().unwrap();
            state.completed_turns.len() + usize::from(state.active_turn.is_some())
        };
        assert_eq!(turns, 1, "exactly one turn, in the bound thread");
        // The idle turn settles like any other and ends the run.
        assert!(wait_for_exit(&mut codex).success());
        assert!(codex.model.saw("IDLE_NONCE"));
        let last = std::fs::read_to_string(codex.root.path().join("last.txt")).unwrap();
        assert_eq!(last, "idle reply");
    }
}
