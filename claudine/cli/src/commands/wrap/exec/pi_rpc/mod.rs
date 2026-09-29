//! Pi's managed RPC execution: the owner of one `pi --mode rpc` child.
//!
//! [`PiRpcSession`] is the [`StdioControl`] for a managed Pi launch. It owns
//! the child's stdin for the whole run and keeps a single serialized writer:
//!
//! - **Readiness.** `get_state` (id [`commands::READY_ID`]) must succeed and
//!   name a session before the task is sent. Its session id binds the
//!   execution's steering target.
//! - **Task.** The task is one `prompt` (id [`commands::TASK_ID`]). Its
//!   response proves only that Pi accepted, queued, or handled it.
//! - **Unattended requests.** Pi's RPC mode reports `hasUI: true` to
//!   extensions, so their dialogs wait for an answer. No person can give one,
//!   and Claudine never invents approval, so a dialog is answered with Pi's
//!   documented cancellation. Notices need no answer. A request whose method
//!   is not documented has no known safe answer and ends the run as
//!   `input_required`.
//! - **Settlement.** `agent_end` can be followed by retries, compaction, or
//!   queued continuation; `agent_settled` cannot. After it (and after a task
//!   an extension handled without a model turn), a `get_state` taken under the
//!   mutation lock decides whether the run is quiescent. Only then is stdin
//!   closed, which ends Pi. An accepted steering message still queued at that
//!   point is reported as undelivered; it is never resent.
//!
//! Every provider mutation — steering delivery ([`executor`]) and the
//! settlement decision — holds one async mutation lock, so a steer, its fresh
//! state check, and the decision to close never interleave. Pi exposes no
//! expected-session guard, and an extension can switch sessions on its own, so
//! the lock cannot make targeting safe; the activation policy blocks this
//! profile for steering (`docs/providers/steering-activation.yaml`).
//!
//! Topic: `claudine/docs/topics/pi-rpc.md`.

use std::collections::HashMap;
use std::io::Write;
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
use claudine::stream::protocol::pi::rpc::{
    Record, RecordError, Response, SessionState, UiMethodKind, UiRequest, read_record, session_state, ui_method_kind,
};
use serde_json::Value;
use tokio::sync::oneshot;

use super::control::{ControlChannels, FallbackLaunch, Readiness, StdioControl};
use super::termination::CompletionTermination;
use crate::steering::render_chain;

pub(crate) mod commands;
mod executor;

#[cfg(test)]
mod tests;

/// Research launch-profile id of a managed Pi RPC launch.
pub(crate) const PROFILE_ID: &str = "retained-rpc";
/// How long Pi gets to answer its readiness check. Startup loads every
/// extension, skill, and template, so this is generous; nothing has been
/// submitted while it runs.
pub(crate) const READINESS_DEADLINE: Duration = Duration::from_secs(30);
/// How long the settlement check waits for its `get_state`.
const SETTLEMENT_CHECK_DEADLINE: Duration = Duration::from_secs(5);
/// How long Pi gets to exit once its stdin is closed before the run is
/// completed by terminating it.
const EXIT_GRACE: Duration = Duration::from_secs(5);

/// An answer to one correlated command, or why none can come.
type Answer = Result<Response, RpcError>;

/// Why a command to Pi produced no usable answer.
#[derive(Debug, Clone, thiserror::Error)]
pub(crate) enum RpcError {
    /// Nothing reached Pi.
    #[error("nothing could be written to Pi")]
    NotWritten(#[source] Arc<std::io::Error>),
    /// Written, but no answer came in time; Pi may have acted on it.
    #[error("Pi did not answer before the deadline")]
    NoAnswer,
    /// Written, but Pi exited first; Pi may have acted on it.
    #[error("Pi exited before answering")]
    Exited,
    /// Pi answered, but the answer could not be read.
    #[error("Pi's answer could not be read")]
    Unreadable(#[source] RecordError),
    /// Pi answered with a refusal. The text is Pi's own; it has no typed
    /// source.
    #[error("Pi refused it ({0})")]
    Refused(String),
}

impl RpcError {
    fn not_written(error: std::io::Error) -> Self {
        Self::NotWritten(Arc::new(error))
    }

    /// Whether the command may have reached Pi.
    fn may_have_been_sent(&self) -> bool {
        matches!(self, Self::NoAnswer | Self::Exited | Self::Unreadable(_))
    }
}

/// Why the settlement check runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Trigger {
    /// Pi accepted the task; an extension may have handled it without a turn.
    TaskAccepted,
    /// Pi refused the task, so no turn will run.
    TaskRefused,
    /// Pi reported `agent_settled`.
    Settled,
}

#[derive(Debug)]
struct State {
    readiness: Readiness,
    /// The Pi session id the execution is bound to.
    conversation: Option<String>,
    /// `agent_start` events seen; a check that saw this move is stale.
    runs_started: u64,
    /// Settled and deciding whether to close: new steering is refused.
    closing: bool,
    /// Stdin is closed.
    closed: bool,
    /// The child has exited.
    finished: bool,
    /// Pi is waiting for input no one can give; the run is being failed, so
    /// it is never ended gracefully (a clean exit would hide the failure).
    input_required: bool,
}

struct Inner {
    writer: Mutex<Option<Box<dyn Write + Send>>>,
    /// Held across every provider mutation.
    mutation: tokio::sync::Mutex<()>,
    state: Mutex<State>,
    pending: Mutex<HashMap<String, oneshot::Sender<Answer>>>,
    next_id: AtomicU64,
    channels: Mutex<Option<ControlChannels>>,
    triggers: Mutex<Option<mpsc::Sender<Trigger>>>,
    controller: OnceLock<SteeringController>,
    fallback: Option<FallbackLaunch>,
}

/// The control session for one managed Pi RPC child.
#[derive(Clone)]
pub(crate) struct PiRpcSession {
    inner: Arc<Inner>,
}

impl PiRpcSession {
    /// A session whose child, if it is not ready before the task is sent,
    /// is replaced by `fallback`.
    pub(crate) fn new(fallback: Option<FallbackLaunch>) -> Self {
        Self {
            inner: Arc::new(Inner {
                writer: Mutex::new(None),
                mutation: tokio::sync::Mutex::new(()),
                state: Mutex::new(State {
                    readiness: Readiness::Pending,
                    conversation: None,
                    runs_started: 0,
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
                fallback,
            }),
        }
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
            for trigger in checks {
                inner.settlement_check(trigger);
            }
        });
        self.inner.send(&commands::get_state(commands::READY_ID))
    }
}

impl StdioControl for PiRpcSession {
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
        self.inner.send(&commands::prompt(commands::TASK_ID, task))
    }

    fn finish(&self) {
        self.inner.finish();
    }

    fn fallback(&self) -> Option<FallbackLaunch> {
        self.inner.fallback.clone()
    }

    fn steering_profile(&self) -> Option<&'static str> {
        Some(PROFILE_ID)
    }

    fn steering_executor(&self) -> Option<Arc<dyn SteeringExecutor>> {
        Some(Arc::new(executor::PiRpcExecutor::new(Arc::clone(&self.inner))))
    }

    fn bind_controller(&self, controller: SteeringController) {
        let _ = self.inner.controller.set(controller);
    }
}

impl Inner {
    /// Writes one command line. Fails once stdin is closed.
    fn send(&self, command: &Value) -> std::io::Result<()> {
        let mut writer = lock(&self.writer);
        let Some(writer) = writer.as_mut() else {
            return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "the provider's input is closed"));
        };
        let mut line = serde_json::to_vec(command)?;
        line.push(b'\n');
        writer.write_all(&line)?;
        writer.flush()
    }

    /// Sends `command` under a fresh correlation id and returns the channel
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

    fn observe(&self, line: &str) {
        match read_record(line) {
            Ok(Record::Response(response)) => self.on_response(response),
            Ok(Record::UiRequest(request)) => self.on_ui_request(&request),
            Ok(Record::AgentStart) => {
                lock(&self.state).runs_started += 1;
                if let Some(controller) = self.controller() {
                    controller.set_state(ExecutionState::Working);
                }
            }
            Ok(Record::AgentSettled) => {
                if let Some(controller) = self.controller() {
                    controller.set_state(ExecutionState::Idle);
                }
                self.trigger(Trigger::Settled);
            }
            Ok(Record::Other) => {}
            Err(error) => self.on_unreadable(line, &error),
        }
    }

    fn on_response(&self, response: Response) {
        match response.id.as_str() {
            commands::READY_ID => {
                let readiness = match read_state(response.outcome) {
                    Ok(state) => {
                        self.bind_conversation(state.session_id);
                        Readiness::Ready
                    }
                    Err(failure) => Readiness::Failed(render_chain("Pi's readiness check failed", &failure)),
                };
                let mut state = lock(&self.state);
                if state.readiness == Readiness::Pending {
                    state.readiness = readiness;
                }
            }
            commands::TASK_ID => {
                self.trigger(if response.outcome.is_ok() { Trigger::TaskAccepted } else { Trigger::TaskRefused });
            }
            id => {
                if let Some(waiter) = lock(&self.pending).remove(id) {
                    let _ = waiter.send(Ok(response));
                }
            }
        }
    }

    fn on_ui_request(&self, request: &UiRequest) {
        match ui_method_kind(&request.method) {
            UiMethodKind::Dialog => {
                if let Err(error) = self.send(&commands::cancel_dialog(&request.id)) {
                    tracing::debug!(target: "claudine::wrap", error = %error, "could not cancel a Pi extension dialog");
                }
            }
            UiMethodKind::Notice => {}
            UiMethodKind::Unsupported => self.input_required(format!(
                "a Pi extension is waiting for `{}` input, which a managed run has no documented way to answer",
                request.method
            )),
        }
    }

    /// A record the owner depends on could not be read. It is routed by
    /// whatever can still be told about it, and never read as success.
    fn on_unreadable(&self, line: &str, error: &RecordError) {
        let raw: Option<Value> = serde_json::from_str(line.trim()).ok();
        let kind = raw.as_ref().and_then(|raw| raw.get("type")).and_then(Value::as_str);
        let id = raw.as_ref().and_then(|raw| raw.get("id")).and_then(Value::as_str);
        let unreadable = RpcError::Unreadable(error.clone());
        match (kind, id) {
            (Some("extension_ui_request"), _) => {
                self.input_required(render_chain("a Pi extension is waiting on a UI request Claudine could not read", &unreadable));
            }
            (Some("response"), Some(commands::READY_ID)) => {
                let mut state = lock(&self.state);
                if state.readiness == Readiness::Pending {
                    state.readiness = Readiness::Failed(render_chain("Pi's readiness check failed", &unreadable));
                }
            }
            (Some("response"), Some(id)) => {
                if let Some(waiter) = lock(&self.pending).remove(id) {
                    let _ = waiter.send(Err(unreadable));
                }
            }
            _ => tracing::debug!(target: "claudine::wrap", error = error as &dyn std::error::Error, "unreadable Pi RPC record"),
        }
    }

    fn input_required(&self, message: String) {
        lock(&self.state).input_required = true;
        if let Some(channels) = lock(&self.channels).as_ref() {
            let _ = channels.early.send(EarlyTermination::InputRequired { message });
        }
    }

    fn bind_conversation(&self, conversation: String) {
        lock(&self.state).conversation = Some(conversation.clone());
        if let Some(controller) = self.controller() {
            controller.set_conversation(Some(conversation));
        }
    }

    fn trigger(&self, trigger: Trigger) {
        if let Some(triggers) = lock(&self.triggers).as_ref() {
            let _ = triggers.send(trigger);
        }
    }

    /// Decides, under the mutation lock, whether the one-shot run is over.
    fn settlement_check(&self, trigger: Trigger) {
        let _mutation = self.mutation.blocking_lock();
        let runs_before = {
            let mut state = lock(&self.state);
            if state.closed || state.finished || state.input_required {
                return;
            }
            if trigger == Trigger::Settled {
                state.closing = true;
            }
            state.runs_started
        };
        if trigger == Trigger::TaskRefused {
            self.close(None);
            return;
        }
        match self.blocking_state(SETTLEMENT_CHECK_DEADLINE) {
            Ok(fresh) => {
                let restarted = lock(&self.state).runs_started != runs_before;
                if restarted || fresh.is_streaming || fresh.is_compacting {
                    lock(&self.state).closing = false;
                } else if fresh.pending_messages > 0 {
                    self.close(Some(fresh.pending_messages));
                } else {
                    self.close(None);
                }
            }
            // A task that just started may still be setting up; a later
            // settlement decides. After a settlement nothing else will ask.
            Err(failure) if trigger == Trigger::Settled => {
                tracing::warn!(
                    target: "claudine::wrap",
                    error = &failure as &dyn std::error::Error,
                    "Pi's state after settling could not be confirmed; ending the run"
                );
                self.close(None);
            }
            Err(failure) => {
                tracing::debug!(
                    target: "claudine::wrap",
                    error = &failure as &dyn std::error::Error,
                    "Pi's state after accepting the task could not be confirmed"
                );
            }
        }
    }

    /// A `get_state` answered on this (non-async) thread.
    fn blocking_state(&self, deadline: Duration) -> Result<SessionState, RpcError> {
        let mut answer = self.request(commands::get_state).map_err(RpcError::not_written)?;
        let until = Instant::now() + deadline;
        loop {
            match answer.try_recv() {
                Ok(answer) => return answer.and_then(|response| read_state(response.outcome)),
                Err(oneshot::error::TryRecvError::Closed) => return Err(RpcError::Exited),
                Err(oneshot::error::TryRecvError::Empty) if Instant::now() >= until => {
                    return Err(RpcError::NoAnswer);
                }
                Err(oneshot::error::TryRecvError::Empty) => thread::sleep(Duration::from_millis(10)),
            }
        }
    }

    /// Closes stdin, which ends Pi, and completes the run if Pi lingers.
    fn close(&self, undelivered: Option<u64>) {
        {
            let mut state = lock(&self.state);
            if state.input_required {
                return;
            }
            state.closing = true;
            state.closed = true;
        }
        if let Some(count) = undelivered {
            let noun = if count == 1 { "message was" } else { "messages were" };
            crate::log::warn(&format!(
                "{count} queued Pi {noun} still undelivered when the run settled; nothing is resent"
            ));
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

/// Reads the outcome of a `get_state` response.
fn read_state(outcome: Result<Value, String>) -> Result<SessionState, RpcError> {
    let data = outcome.map_err(RpcError::Refused)?;
    session_state(&data).map_err(RpcError::Unreadable)
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poison| poison.into_inner())
}
