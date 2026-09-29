//! Codex's managed app-server execution: the owner of one
//! `codex app-server --listen stdio://` child that stands in for `codex exec`.
//!
//! [`CodexAppServerSession`] is the [`StdioControl`] for that launch. It owns
//! the child's stdin for the whole run and keeps a single serialized writer:
//!
//! - **Readiness.** `initialize` must answer with a user agent (which names
//!   the exact Codex version), then `initialized` is sent and the thread is
//!   started or resumed with exec's settings ([`launch`]). The thread id binds
//!   the execution's steering target.
//! - **Task.** The task is one `turn/start` ([`commands::TASK_REQUEST_ID`]).
//!   Its answer names the turn; a refusal fails the run.
//! - **Unattended requests.** Answered exactly as `codex exec` answers them:
//!   an MCP elicitation is cancelled and every other server request gets a
//!   JSON-RPC error. Nothing is ever approved. A request that cannot even be
//!   read has no safe answer and ends the run as `input_required`.
//! - **Settlement.** Closing stdin while a turn runs makes the app-server exit
//!   0 and abandon the turn, so stdin closes only after a `turn/completed`,
//!   when a `thread/read` taken under the mutation lock shows the thread idle
//!   and no turn started meanwhile (a steering message can extend a turn or
//!   start another). exec's `--output-last-message` file is written from the
//!   last agent message just before closing.
//!
//! Every provider mutation — steering delivery ([`executor`]) and the
//! settlement decision — holds one async mutation lock. Unlike Pi, Codex
//! gives steering an atomic target guard (`expectedTurnId`), and nothing but
//! this owner can move the thread, so targeting is safe without a block.
//!
//! Topic: `claudine/docs/topics/codex-app-server.md`.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;
use std::process::ChildStdin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

use claudine::steering::controller::{SteeringController, SteeringExecutor};
use claudine::steering::identity::ProcessStartIdentity;
use claudine::steering::vocabulary::ExecutionState;
use claudine::stream::logs::EarlyTermination;
use claudine::stream::protocol::codex::app_server::{
    self, Message, MessageError, Response, RpcFailure, ServerRequest, ThreadActivity, TurnStatus,
};
use serde_json::Value;
use tokio::sync::oneshot;

use super::control::{ControlChannels, FallbackLaunch, Readiness, StdioControl};
use super::termination::CompletionTermination;
use crate::steering::render_chain;

pub(crate) mod commands;
mod executor;
pub(crate) mod launch;

#[cfg(test)]
mod tests;
// Shared with the `real` test binary, which uses the rest of it.
#[cfg(all(test, feature = "real-tests"))]
#[path = "../../../../../tests/common/codex_model.rs"]
#[allow(dead_code)]
mod codex_model;

pub(crate) use launch::{ManagedLaunch, ThreadRequest, Unmapped};

/// Research launch-profile id of a managed Codex app-server launch.
pub(crate) const PROFILE_ID: &str = "managed-app-server";
/// How long Codex gets to initialize and start its thread. Thread start
/// loads configuration, skills, and MCP servers; nothing has been submitted
/// while it runs.
pub(crate) const READINESS_DEADLINE: Duration = Duration::from_secs(60);
/// How long the settlement check waits for its `thread/read`.
const SETTLEMENT_CHECK_DEADLINE: Duration = Duration::from_secs(5);
/// How long Codex gets to exit once its stdin is closed before the run is
/// completed by terminating it.
const EXIT_GRACE: Duration = Duration::from_secs(5);

/// What a fallback `codex exec` run gives up.
const EXEC_FALLBACK_LOSSES: &str =
    "Running `codex exec` instead: this run cannot be steered or queried.";

/// An answer to one correlated request, or why none can come.
type Answer = Result<Value, RpcError>;

/// Why a request to Codex produced no usable answer.
#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum RpcError {
    /// Nothing reached Codex.
    #[error("nothing could be written to Codex")]
    NotWritten(#[source] Arc<std::io::Error>),
    /// Written, but no answer came in time; Codex may have acted on it.
    #[error("Codex did not answer before the deadline")]
    NoAnswer,
    /// Written, but Codex exited first; Codex may have acted on it.
    #[error("Codex exited before answering")]
    Exited,
    /// Codex answered, but the answer could not be read.
    #[error("Codex's answer could not be read")]
    Unreadable(#[source] MessageError),
    /// Codex answered with a JSON-RPC error. The text is Codex's own.
    #[error("Codex refused it ({0})")]
    Refused(RpcFailure),
}

impl RpcError {
    fn not_written(error: std::io::Error) -> Self {
        Self::NotWritten(Arc::new(error))
    }

    /// Whether the request may have reached Codex.
    fn may_have_been_sent(&self) -> bool {
        matches!(self, Self::NoAnswer | Self::Exited | Self::Unreadable(_))
    }
}

#[derive(Debug)]
struct State {
    readiness: Readiness,
    thread_id: Option<String>,
    /// The thread's running turn, from the turn's own notifications and
    /// `turn/start` answers.
    active_turn: Option<String>,
    /// Turns that have completed, with how they ended.
    completed_turns: HashMap<String, TurnStatus>,
    /// `turn/started` notifications seen; a check that saw this move is stale.
    turns_started: u64,
    last_agent_message: Option<String>,
    /// Settled and deciding whether to close: new steering is refused.
    closing: bool,
    /// Stdin is closed.
    closed: bool,
    /// The child has exited.
    finished: bool,
    /// Codex is waiting on a request no one can answer; the run is being
    /// failed, so it is never ended gracefully.
    input_required: bool,
}

pub(super) struct Inner {
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    /// Held across every provider mutation.
    mutation: tokio::sync::Mutex<()>,
    state: Mutex<State>,
    pending: Mutex<HashMap<String, oneshot::Sender<Answer>>>,
    next_id: AtomicU64,
    channels: Mutex<Option<ControlChannels>>,
    triggers: Mutex<Option<mpsc::Sender<()>>>,
    controller: OnceLock<SteeringController>,
    launch: ManagedLaunch,
}

/// The control session for one managed Codex app-server child.
#[derive(Clone)]
pub(crate) struct CodexAppServerSession {
    inner: Arc<Inner>,
}

impl CodexAppServerSession {
    pub(crate) fn new(launch: ManagedLaunch) -> Self {
        Self {
            inner: Arc::new(Inner {
                writer: Mutex::new(None),
                mutation: tokio::sync::Mutex::new(()),
                state: Mutex::new(State {
                    readiness: Readiness::Pending,
                    thread_id: None,
                    active_turn: None,
                    completed_turns: HashMap::new(),
                    turns_started: 0,
                    last_agent_message: None,
                    closing: false,
                    closed: false,
                    finished: false,
                    input_required: false,
                }),
                pending: Mutex::new(HashMap::new()),
                next_id: AtomicU64::new(1),
                channels: Mutex::new(None),
                triggers: Mutex::new(None),
                controller: OnceLock::new(),
                launch,
            }),
        }
    }

    /// The session for `args` run in `cwd`, when a managed launch can carry
    /// them.
    pub(crate) fn for_exec_args(args: &[String], cwd: &Path) -> Result<Self, Unmapped> {
        launch::plan(args, cwd).map(Self::new)
    }

    /// Opens the session over any writer; [`StdioControl::open`] passes the
    /// child's stdin.
    fn open_writer(&self, writer: Box<dyn Write + Send>, channels: ControlChannels) -> std::io::Result<()> {
        *lock(&self.inner.writer) = Some(writer);
        *lock(&self.inner.channels) = Some(channels);
        let (triggers, checks) = mpsc::channel();
        *lock(&self.inner.triggers) = Some(triggers);
        let inner = Arc::clone(&self.inner);
        thread::spawn(move || {
            for () in checks {
                inner.settlement_check();
            }
        });
        self.inner.send(&commands::initialize())
    }
}

impl StdioControl for CodexAppServerSession {
    fn launch_args(&self) -> Option<Vec<String>> {
        Some(self.inner.launch.args.clone())
    }

    fn open(&self, stdin: ChildStdin, child_pid: u32, channels: ControlChannels) -> std::io::Result<()> {
        if let Some(controller) = self.inner.controller.get()
            && let Some(start) = crate::cli_utils::process_start(child_pid)
            && let Ok(process) = ProcessStartIdentity::new(child_pid, start.to_string())
        {
            controller.set_provider_process(process);
        }
        self.open_writer(Box::new(stdin), channels)
    }

    fn observe(&self, line: &str) {
        self.inner.observe(line);
    }

    /// Responses answer this session's own requests; Codex's activity
    /// arrives as notifications and server requests.
    fn is_control_reply(&self, line: &str) -> bool {
        line.contains("\"id\"") && matches!(app_server::read_message(line), Ok(Message::Response(_)))
    }

    fn readiness(&self) -> Readiness {
        lock(&self.inner.state).readiness.clone()
    }

    fn abandon_readiness(&self, reason: &str) -> bool {
        let mut state = lock(&self.inner.state);
        match state.readiness {
            Readiness::Pending => {
                state.readiness = Readiness::Failed(reason.to_string());
                true
            }
            Readiness::Ready => false,
            Readiness::Failed(_) => true,
        }
    }

    fn readiness_deadline(&self) -> Duration {
        READINESS_DEADLINE
    }

    fn submit(&self, task: &str) -> std::io::Result<()> {
        let thread_id = lock(&self.inner.state).thread_id.clone().unwrap_or_default();
        self.inner.send(&commands::turn_start(
            commands::TASK_REQUEST_ID,
            &thread_id,
            task,
            commands::TASK_REQUEST_ID,
        ))
    }

    fn finish(&self) {
        self.inner.finish();
    }

    fn fallback(&self) -> Option<FallbackLaunch> {
        Some(FallbackLaunch { args: self.inner.launch.exec_args.clone(), warning: EXEC_FALLBACK_LOSSES.to_string() })
    }

    fn steering_profile(&self) -> Option<&'static str> {
        Some(PROFILE_ID)
    }

    fn steering_executor(&self) -> Option<Arc<dyn SteeringExecutor>> {
        Some(Arc::new(executor::CodexAppServerExecutor::new(Arc::clone(&self.inner))))
    }

    fn bind_controller(&self, controller: SteeringController) {
        let _ = self.inner.controller.set(controller);
    }
}

impl Inner {
    /// Writes one message line. Fails once stdin is closed.
    fn send(&self, message: &Value) -> std::io::Result<()> {
        let mut writer = lock(&self.writer);
        let Some(writer) = writer.as_mut() else {
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "the provider's input is closed"));
        };
        let mut line = serde_json::to_vec(message)?;
        line.push(b'\n');
        writer.write_all(&line)?;
        writer.flush()
    }

    /// Sends a request under a fresh correlation id and returns the channel
    /// its answer arrives on. `build` receives the id.
    fn request(&self, build: impl FnOnce(&str) -> Value) -> std::io::Result<oneshot::Receiver<Answer>> {
        let id = format!("claudine-{}", self.next_id.fetch_add(1, Ordering::Relaxed));
        let (sender, receiver) = oneshot::channel();
        lock(&self.pending).insert(id.clone(), sender);
        if let Err(error) = self.send(&build(&id)) {
            lock(&self.pending).remove(&id);
            return Err(error);
        }
        Ok(receiver)
    }

    fn controller(&self) -> Option<&SteeringController> {
        self.controller.get()
    }

    fn thread_id(&self) -> Option<String> {
        lock(&self.state).thread_id.clone()
    }

    fn observe(&self, line: &str) {
        match app_server::read_message(line) {
            Ok(Message::Response(response)) => self.on_response(response),
            Ok(Message::Request(request)) => self.on_server_request(&request),
            Ok(Message::Notification { method, params }) => self.on_notification(&method, &params),
            Ok(Message::Other) => {}
            Err(error) => self.on_unreadable(line, &error),
        }
    }

    fn on_response(&self, response: Response) {
        let Some(id) = response.id else {
            tracing::debug!(target: "claudine::wrap", "Codex answered a request this session did not send");
            return;
        };
        match id.as_str() {
            commands::INITIALIZE_ID => match response.outcome.map_err(RpcError::Refused).and_then(|result| {
                app_server::initialize_info(&result).map_err(RpcError::Unreadable)
            }) {
                Ok(info) => {
                    if let (Some(controller), Some(version)) = (self.controller(), info.provider_version) {
                        controller.set_provider_version(version);
                    }
                    let thread = match &self.launch.thread {
                        ThreadRequest::Start { params } => commands::thread_start(params),
                        ThreadRequest::Resume { thread_id, params } => commands::thread_resume(thread_id, params),
                    };
                    let sent = self.send(&commands::initialized()).and_then(|()| self.send(&thread));
                    if let Err(error) = sent {
                        self.fail_readiness(render_chain("the thread could not be requested", &RpcError::not_written(error)));
                    }
                }
                Err(failure) => self.fail_readiness(render_chain("Codex did not initialize", &failure)),
            },
            commands::THREAD_ID => match response.outcome.map_err(RpcError::Refused).and_then(|result| {
                app_server::started_thread(&result).map_err(RpcError::Unreadable)
            }) {
                Ok(thread_id) => {
                    lock(&self.state).thread_id = Some(thread_id.clone());
                    if let Some(controller) = self.controller() {
                        controller.set_conversation(Some(thread_id));
                    }
                    let mut state = lock(&self.state);
                    if state.readiness == Readiness::Pending {
                        state.readiness = Readiness::Ready;
                    }
                }
                Err(failure) => self.fail_readiness(render_chain("Codex did not start the thread", &failure)),
            },
            commands::TASK_REQUEST_ID => match response.outcome {
                Ok(result) => match app_server::started_turn(&result) {
                    Ok(turn) => self.note_turn_started(&turn.id, false),
                    Err(error) => {
                        tracing::debug!(target: "claudine::wrap", error = &error as &dyn std::error::Error, "Codex's answer to the task could not be read");
                    }
                },
                // The parser reports the refusal; no turn will run.
                Err(_) => self.trigger(),
            },
            id => {
                if let Some(waiter) = lock(&self.pending).remove(id) {
                    let _ = waiter.send(response.outcome.map_err(RpcError::Refused));
                }
            }
        }
    }

    fn fail_readiness(&self, reason: String) {
        let mut state = lock(&self.state);
        if state.readiness == Readiness::Pending {
            state.readiness = Readiness::Failed(reason);
        }
    }

    /// Answers a server request as `codex exec` does: cancel an MCP
    /// elicitation, refuse everything else. Nothing is ever approved.
    fn on_server_request(&self, request: &ServerRequest) {
        let answer = if request.method == "mcpServer/elicitation/request" {
            commands::cancel_elicitation(&request.id)
        } else {
            commands::not_supported(&request.id, &request.method)
        };
        if let Err(error) = self.send(&answer) {
            tracing::debug!(target: "claudine::wrap", error = %error, method = %request.method, "could not answer a Codex request");
        }
    }

    fn on_notification(&self, method: &str, params: &Value) {
        match method {
            "turn/started" => match app_server::turn_event(params) {
                Ok(event) if self.is_own_thread(&event.thread_id) => self.note_turn_started(&event.turn.id, true),
                Ok(_) => {}
                Err(error) => {
                    tracing::debug!(target: "claudine::wrap", error = &error as &dyn std::error::Error, "unreadable Codex turn/started");
                }
            },
            "turn/completed" => match app_server::turn_event(params) {
                Ok(event) if self.is_own_thread(&event.thread_id) => {
                    {
                        let mut state = lock(&self.state);
                        if state.active_turn.as_deref() == Some(event.turn.id.as_str()) {
                            state.active_turn = None;
                        }
                        state.completed_turns.insert(event.turn.id, event.turn.status);
                    }
                    if let Some(controller) = self.controller() {
                        controller.set_state(ExecutionState::Idle);
                    }
                    self.trigger();
                }
                Ok(_) => {}
                // Without a readable completion the run can only settle when
                // Codex exits; the watchdogs bound that.
                Err(error) => {
                    tracing::warn!(target: "claudine::wrap", error = &error as &dyn std::error::Error, "unreadable Codex turn/completed");
                }
            },
            "item/completed" => {
                if params.pointer("/item/type").and_then(Value::as_str) == Some("agentMessage")
                    && let Some(text) = params.pointer("/item/text").and_then(Value::as_str)
                {
                    lock(&self.state).last_agent_message = Some(text.to_string());
                }
            }
            _ => {}
        }
    }

    fn is_own_thread(&self, thread_id: &str) -> bool {
        lock(&self.state).thread_id.as_deref() == Some(thread_id)
    }

    /// Records a running turn. `notified` is the turn's own `turn/started`,
    /// which counts toward the settlement check; a `turn/start` answer names
    /// the same turn earlier.
    fn note_turn_started(&self, turn_id: &str, notified: bool) {
        {
            let mut state = lock(&self.state);
            if state.completed_turns.contains_key(turn_id) {
                return;
            }
            state.active_turn = Some(turn_id.to_string());
            if notified {
                state.turns_started += 1;
            }
        }
        if let Some(controller) = self.controller() {
            controller.set_state(ExecutionState::Working);
        }
    }

    /// A message the owner depends on could not be read. It is routed by
    /// whatever can still be told about it, and never read as success.
    fn on_unreadable(&self, line: &str, error: &MessageError) {
        let raw: Option<Value> = serde_json::from_str(line.trim()).ok();
        let method = raw.as_ref().and_then(|raw| raw.get("method"));
        let id = raw.as_ref().and_then(|raw| raw.get("id")).and_then(Value::as_str);
        let unreadable = RpcError::Unreadable(error.clone());
        match (method, id) {
            (Some(_), _) => {
                self.input_required(render_chain("Codex is waiting on a request Claudine could not read", &unreadable));
            }
            (None, Some(commands::INITIALIZE_ID | commands::THREAD_ID)) => {
                self.fail_readiness(render_chain("Codex's readiness answer could not be read", &unreadable));
            }
            (None, Some(id)) => {
                if let Some(waiter) = lock(&self.pending).remove(id) {
                    let _ = waiter.send(Err(unreadable));
                }
            }
            _ => tracing::debug!(target: "claudine::wrap", error = error as &dyn std::error::Error, "unreadable Codex message"),
        }
    }

    fn input_required(&self, message: String) {
        lock(&self.state).input_required = true;
        if let Some(channels) = lock(&self.channels).as_ref() {
            let _ = channels.early.send(EarlyTermination::InputRequired { message });
        }
    }

    fn trigger(&self) {
        if let Some(triggers) = lock(&self.triggers).as_ref() {
            let _ = triggers.send(());
        }
    }

    /// Decides, under the mutation lock, whether the one-shot run is over.
    fn settlement_check(&self) {
        let _mutation = self.mutation.blocking_lock();
        let (turns_before, thread_id) = {
            let mut state = lock(&self.state);
            if state.closed || state.finished || state.input_required || state.active_turn.is_some() {
                return;
            }
            state.closing = true;
            (state.turns_started, state.thread_id.clone())
        };
        let Some(thread_id) = thread_id else {
            self.close();
            return;
        };
        match self.blocking_activity(&thread_id, SETTLEMENT_CHECK_DEADLINE) {
            Ok(ThreadActivity::Active) => lock(&self.state).closing = false,
            Ok(_) if lock(&self.state).turns_started != turns_before => lock(&self.state).closing = false,
            Ok(_) => self.close(),
            Err(failure) => {
                tracing::warn!(
                    target: "claudine::wrap",
                    error = &failure as &dyn std::error::Error,
                    "Codex's state after the turn could not be confirmed; ending the run"
                );
                self.close();
            }
        }
    }

    /// A `thread/read` answered on this (non-async) thread.
    fn blocking_activity(&self, thread_id: &str, deadline: Duration) -> Result<ThreadActivity, RpcError> {
        let mut answer = self.request(|id| commands::thread_read(id, thread_id)).map_err(RpcError::not_written)?;
        let until = Instant::now() + deadline;
        loop {
            match answer.try_recv() {
                Ok(answer) => {
                    return answer.and_then(|result| {
                        app_server::thread_activity(&result, thread_id).map_err(RpcError::Unreadable)
                    });
                }
                Err(oneshot::error::TryRecvError::Closed) => return Err(RpcError::Exited),
                Err(oneshot::error::TryRecvError::Empty) if Instant::now() >= until => return Err(RpcError::NoAnswer),
                Err(oneshot::error::TryRecvError::Empty) => thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    /// Writes exec's last-message file, closes stdin (which ends Codex), and
    /// completes the run if Codex lingers.
    fn close(&self) {
        let last_message = {
            let mut state = lock(&self.state);
            if state.input_required {
                return;
            }
            state.closing = true;
            state.closed = true;
            state.last_agent_message.clone()
        };
        if let (Some(path), Some(text)) = (&self.launch.last_message, last_message)
            && let Err(error) = std::fs::write(path, text)
        {
            tracing::warn!(target: "claudine::wrap", error = %error, "could not write Codex's final message");
        }
        lock(&self.writer).take();
        // Once the child exits the wait loop drops the receiver, so a send
        // after a normal exit goes nowhere.
        if let Some(completion) = lock(&self.channels).as_ref().map(|channels| channels.completion.clone()) {
            thread::spawn(move || {
                thread::sleep(EXIT_GRACE);
                let _ = completion.send(CompletionTermination);
            });
        }
    }

    fn finish(&self) {
        {
            let mut state = lock(&self.state);
            state.finished = true;
            state.closed = true;
            if state.readiness == Readiness::Pending {
                state.readiness = Readiness::Failed("the provider exited before it was ready".into());
            }
        }
        lock(&self.writer).take();
        lock(&self.triggers).take();
        for (_, waiter) in lock(&self.pending).drain() {
            let _ = waiter.send(Err(RpcError::Exited));
        }
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}
