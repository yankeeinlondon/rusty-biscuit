//! Shared in-memory DMLS session fixture for the `l1` integration binary's LSP
//! modules: `lsp_session`, `no_side_effects`, and `suggest_constraint_phase1`.
//!
//! `spec.md` requires DMLS sessions to own bounded cleanup on success, failure,
//! *and* cancellation. Each of those three modules previously carried its own
//! near-duplicate copy of this client driver and only one of them retained the
//! server `JoinHandle` at all, so an assertion failure detached a live server
//! thread. This module is the single implementation of that cleanup contract.
//!
//! Fixture shape originally ported from `iwes/tests/fixture.rs` (Apache-2.0,
//! IWE project).
//!
//! ## Ownership and ordering
//!
//! [`LspFixture`] borrows the [`LspWorkspace`] it serves for its entire life.
//! Because `LspFixture` has a `Drop` impl, dropck makes that shared borrow
//! *strict*: releasing the workspace while a fixture is alive is a compile
//! error, and a fixture declared after its workspace is always dropped first.
//! Fixture teardown therefore provably completes before the workspace directory
//! is deleted. `lsp_session`'s
//! `server_worker_finishes_before_workspace_release_during_unwind` observes
//! that same ordering at runtime, on the failure path.
//!
//! ## Notes
//!
//! The consuming modules use only a subset of this API between them, so the
//! module is blanket `dead_code`-exempt rather than carrying per-item
//! exemptions that would drift as tests change.
#![allow(dead_code)]

pub mod oracle;

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
use darkmatter::markdown::compose::RequestSnapshot;
use serde_json::{Value, json};

/// Deadline every request/notification helper gives one protocol message.
pub const MESSAGE_BOUND: Duration = Duration::from_secs(10);

/// Completion bound for the server worker during teardown.
///
/// `JoinHandle` has no timed join, so the bound cannot live on `join()`. It
/// lives on the outcome channel instead: the worker records its completion and
/// sends its outcome as the very last things it does, so *observing the
/// outcome* proves the worker body has finished and makes the following
/// `join()` a formality that cannot block meaningfully.
///
/// Ten seconds is the same budget [`MESSAGE_BOUND`] gives a single in-flight
/// protocol message: the slowest legitimate reason a worker has not returned is
/// that it is finishing one already-accepted request or its startup disk walk,
/// and neither is granted more than that anywhere else in the suite. It also
/// has to stay well under the runner's `terminate-after` ceiling
/// (`.config/nextest.toml`: `slow-timeout = 5s × 6`), because a teardown that
/// outlives the ceiling is killed before it can print the diagnostic that makes
/// the timeout actionable.
pub const SERVER_EXIT_BOUND: Duration = Duration::from_secs(10);

/// An ordered teardown observation, recorded so a failure-path test can assert
/// the ordering rather than infer it from drop-order rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeardownEvent {
    /// The server worker body returned. `workspace_present` is the existence of
    /// the workspace root as observed *from the worker thread* at that instant.
    WorkerFinished { workspace_present: bool },
    /// [`SERVER_EXIT_BOUND`] elapsed with the worker still running, so it was
    /// deliberately detached instead of joined.
    WorkerAbandoned,
    /// The workspace directory has been deleted.
    WorkspaceReleased,
}

/// Shared teardown log. Cloned into the server worker and retained by the
/// workspace, so a test can hold an `Arc` and read the log after both are gone.
#[derive(Debug, Default)]
pub struct TeardownObservations {
    events: Mutex<Vec<TeardownEvent>>,
}

impl TeardownObservations {
    fn record(&self, event: TeardownEvent) {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .push(event);
    }

    pub fn events(&self) -> Vec<TeardownEvent> {
        self.events
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }
}

/// Temporary workspace root for one DMLS session test.
///
/// Deliberately not a bare `tempfile::TempDir`: [`LspFixture`] borrows this
/// value, which is what turns "the server worker is reaped before the workspace
/// is deleted" into a rule the borrow checker enforces.
pub struct LspWorkspace {
    dir: Option<tempfile::TempDir>,
    observations: Arc<TeardownObservations>,
}

impl LspWorkspace {
    pub fn new() -> Self {
        Self {
            dir: Some(tempfile::tempdir().expect("create workspace temp dir")),
            observations: Arc::new(TeardownObservations::default()),
        }
    }

    pub fn path(&self) -> &Path {
        self.dir.as_ref().expect("workspace released").path()
    }

    /// Handle on the teardown log that outlives this workspace.
    pub fn observations(&self) -> Arc<TeardownObservations> {
        Arc::clone(&self.observations)
    }
}

impl Default for LspWorkspace {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for LspWorkspace {
    fn drop(&mut self) {
        // Delete before recording, so `WorkspaceReleased` means the directory is
        // actually gone rather than merely scheduled for deletion.
        drop(self.dir.take());
        self.observations.record(TeardownEvent::WorkspaceReleased);
    }
}

/// Client-side driver over an in-memory `Connection::memory()` pair.
///
/// Borrows its [`LspWorkspace`] so the server worker is provably reaped before
/// the workspace directory is deleted.
pub struct LspFixture<'workspace> {
    workspace: &'workspace LspWorkspace,
    client: Option<Connection>,
    server_outcome: mpsc::Receiver<Result<(), String>>,
    server_thread: Option<std::thread::JoinHandle<()>>,
    observations: Arc<TeardownObservations>,
    /// Whether the `shutdown` request completed before teardown. A cancelled
    /// session sends `exit` without it, which DMLS legitimately reports as an
    /// error outcome; only a session that *did* handshake owes a clean exit.
    clean_shutdown: bool,
    next_id: i32,
    /// Server → client notifications buffered while awaiting a response (the
    /// server pushes `publishDiagnostics` out-of-band).
    notifications: Vec<Notification>,
    /// Server → client requests buffered while awaiting a response (the server
    /// pushes `workspace/semanticTokens/refresh`, progress-create, and watcher
    /// registrations fire-and-forget).
    server_requests: Vec<Request>,
    /// Startup events the server reports once each background result is
    /// installed ([`dmls::StartupHooks::observer`]).
    startup_events: crossbeam_channel::Receiver<dmls::StartupEvent>,
    /// Holds the file-list walk until dropped or sent to, for a fixture
    /// started with [`LspFixture::start_with_file_list_gate`].
    file_list_gate: Option<crossbeam_channel::Sender<()>>,
    /// Holds each index build round's results until sent to (one round
    /// per message) or dropped, for a fixture started with
    /// [`LspFixture::start_with_index_round_gate`].
    index_round_gate: Option<crossbeam_channel::Sender<()>>,
    /// Holds each trigger-registry job until sent to (one job per message)
    /// or dropped, for a fixture started with
    /// [`LspFixture::start_with_registry_gate`].
    registry_gate: Option<crossbeam_channel::Sender<()>>,
    /// Holds each workspace-pass probe until sent to (one probe per message)
    /// or dropped, for a fixture started with
    /// [`LspFixture::start_with_registry_probe_gate`].
    registry_probe_gate: Option<crossbeam_channel::Sender<()>>,
    /// Holds each startup-phase probe until sent to (one probe per message)
    /// or dropped, for a fixture started with
    /// [`LspFixture::start_with_startup_probe_gate`].
    startup_probe_gate: Option<crossbeam_channel::Sender<()>>,
    /// The last [`dmls::StartupEvent::RegistryJobsOutstanding`] count seen.
    registry_jobs: usize,
}

impl<'workspace> LspFixture<'workspace> {
    pub fn start(workspace: &'workspace LspWorkspace) -> Self {
        Self::start_with_worker_epilogue_delay(workspace, Duration::ZERO)
    }

    /// The snapshot every session gets unless a test supplies its own: the
    /// workspace as request directory, a fixture `HOME` inside it, and an
    /// empty environment, so no test reads the developer's home or variables.
    pub fn fixture_snapshot(workspace: &LspWorkspace) -> RequestSnapshot {
        RequestSnapshot::new(workspace.path())
            .with_home(Some(workspace.path().join("home")))
    }

    /// Like [`LspFixture::start`], with the server's request snapshot (its
    /// `HOME`, environment, and extra `@` roots) supplied by the test.
    pub fn start_with_snapshot(
        workspace: &'workspace LspWorkspace,
        snapshot: RequestSnapshot,
    ) -> Self {
        Self::spawn(workspace, Duration::ZERO, snapshot)
    }

    /// Like [`LspFixture::start`], but the worker waits `epilogue_delay` after
    /// `run_server` returns and before it records completion and publishes its
    /// outcome.
    ///
    /// This exists so a failure-path test can distinguish "teardown waited for
    /// the worker" from "teardown happened to outrun a worker that was about to
    /// finish anyway". With a zero delay both look identical; with a non-zero
    /// one, only a teardown that genuinely blocks on the outcome can still
    /// observe the workspace intact. Every ordinary test uses `Duration::ZERO`
    /// and pays nothing.
    pub fn start_with_worker_epilogue_delay(
        workspace: &'workspace LspWorkspace,
        epilogue_delay: Duration,
    ) -> Self {
        Self::spawn(workspace, epilogue_delay, Self::fixture_snapshot(workspace))
    }

    /// Like [`LspFixture::start`], but the server's workspace file-list walk
    /// waits until [`LspFixture::release_file_list`], so a test can observe
    /// the server before the list is ready. Initialize it with
    /// [`LspFixture::initialize_unlisted`].
    pub fn start_with_file_list_gate(workspace: &'workspace LspWorkspace) -> Self {
        let (gate_tx, gate_rx) = crossbeam_channel::bounded(1);
        let mut fixture = Self::spawn_with_gate(
            workspace,
            Duration::ZERO,
            Self::fixture_snapshot(workspace),
            Some(gate_rx),
        );
        fixture.file_list_gate = Some(gate_tx);
        fixture
    }

    /// Like [`LspFixture::start`], but each index build round, once its
    /// store pass has read its files, waits for [`LspFixture::release_index_round`]
    /// (or [`LspFixture::open_index_round_gate`]) before the server reconciles
    /// it, so a test can change the workspace while the round's reading is
    /// stale.
    pub fn start_with_index_round_gate(workspace: &'workspace LspWorkspace) -> Self {
        Self::start_with_index_round_gate_and_snapshot(workspace, Self::fixture_snapshot(workspace))
    }

    /// [`LspFixture::start_with_index_round_gate`] with the server's request
    /// snapshot supplied by the test.
    pub fn start_with_index_round_gate_and_snapshot(
        workspace: &'workspace LspWorkspace,
        snapshot: RequestSnapshot,
    ) -> Self {
        let (gate_tx, gate_rx) = crossbeam_channel::unbounded();
        let mut fixture = Self::spawn_with_hooks(workspace, Duration::ZERO, snapshot, None, Some(gate_rx));
        fixture.index_round_gate = Some(gate_tx);
        fixture
    }

    /// Like [`LspFixture::start`], but each trigger-registry job the server
    /// queues after startup waits for [`LspFixture::release_registry_job`]
    /// (or [`LspFixture::open_registry_gate`]) before it prepares anything,
    /// so a test can talk to the server while a preparation is held.
    pub fn start_with_registry_gate(workspace: &'workspace LspWorkspace) -> Self {
        let (gate_tx, gate_rx) = crossbeam_channel::unbounded();
        let hooks = dmls::StartupHooks { registry_gate: Some(gate_rx), ..dmls::StartupHooks::default() };
        let mut fixture = Self::spawn_with_all_hooks(workspace, Duration::ZERO, Self::fixture_snapshot(workspace), hooks);
        fixture.registry_gate = Some(gate_tx);
        fixture
    }

    /// [`LspFixture::start_with_registry_gate`] on a server timed by `clock`,
    /// so a test can advance time while a request is parked on a held job.
    pub fn start_with_registry_gate_and_clock(
        workspace: &'workspace LspWorkspace,
        clock: Arc<dyn dmls::clock::Clock>,
    ) -> Self {
        let (gate_tx, gate_rx) = crossbeam_channel::unbounded();
        let hooks = dmls::StartupHooks { registry_gate: Some(gate_rx), ..dmls::StartupHooks::default() };
        let mut fixture =
            Self::spawn_with_clock(workspace, Duration::ZERO, Self::fixture_snapshot(workspace), hooks, Some(clock));
        fixture.registry_gate = Some(gate_tx);
        fixture
    }

    /// Lets one held trigger-registry job through.
    pub fn release_registry_job(&self) {
        if let Some(gate) = &self.registry_gate {
            gate.send(()).expect("registry gate open");
        }
    }

    /// Lets every current and later trigger-registry job through.
    pub fn open_registry_gate(&mut self) {
        self.registry_gate = None;
    }

    /// Like [`LspFixture::start`], but each probe of a workspace pass on the
    /// registry worker waits for [`LspFixture::release_registry_probe`] (or
    /// [`LspFixture::open_registry_probe_gate`]) before it is prepared, so a
    /// test can hold a pass between two probes. Document jobs are not held.
    pub fn start_with_registry_probe_gate(workspace: &'workspace LspWorkspace) -> Self {
        let (gate_tx, gate_rx) = crossbeam_channel::unbounded();
        let hooks = dmls::StartupHooks { registry_probe_gate: Some(gate_rx), ..dmls::StartupHooks::default() };
        let mut fixture = Self::spawn_with_all_hooks(workspace, Duration::ZERO, Self::fixture_snapshot(workspace), hooks);
        fixture.registry_probe_gate = Some(gate_tx);
        fixture
    }

    /// Lets one held workspace probe through.
    pub fn release_registry_probe(&self) {
        if let Some(gate) = &self.registry_probe_gate {
            gate.send(()).expect("registry probe gate open");
        }
    }

    /// Lets every current and later workspace probe through.
    pub fn open_registry_probe_gate(&mut self) {
        self.registry_probe_gate = None;
    }

    /// Like [`LspFixture::start`], but each probe preparation a startup
    /// phase runs on the registry worker, once it has read its inputs, waits
    /// for [`LspFixture::release_startup_probe`] (or
    /// [`LspFixture::open_startup_probe_gate`]) before installing its result,
    /// so a test can hold startup's preparation in flight while discovery,
    /// requests, and changes proceed. Initialize it with
    /// [`LspFixture::initialize_unlisted`].
    pub fn start_with_startup_probe_gate(workspace: &'workspace LspWorkspace) -> Self {
        let (gate_tx, gate_rx) = crossbeam_channel::unbounded();
        let hooks = dmls::StartupHooks { startup_probe_gate: Some(gate_rx), ..dmls::StartupHooks::default() };
        let mut fixture = Self::spawn_with_all_hooks(workspace, Duration::ZERO, Self::fixture_snapshot(workspace), hooks);
        fixture.startup_probe_gate = Some(gate_tx);
        fixture
    }

    /// Lets one held startup probe through.
    pub fn release_startup_probe(&self) {
        if let Some(gate) = &self.startup_probe_gate {
            gate.send(()).expect("startup probe gate open");
        }
    }

    /// Lets every current and later startup probe through.
    pub fn open_startup_probe_gate(&mut self) {
        self.startup_probe_gate = None;
    }

    /// The kinds of the trigger-registry preparations that finish, in the
    /// order the server handled them, until no job is queued or running.
    ///
    /// The server reports a job's decremented count before its
    /// [`dmls::StartupEvent::RegistryPrepared`], so each decrement owes one
    /// completion still to read.
    pub fn registry_jobs_until_idle(&mut self) -> Vec<dmls::RegistryJobKind> {
        let mut jobs = Vec::new();
        let mut owed = 0usize;
        let mut waiting = false;
        loop {
            let event = if waiting {
                match self.startup_events.recv_timeout(MESSAGE_BOUND) {
                    Ok(event) => event,
                    Err(error) => panic!(
                        "{} registry jobs still outstanding after {MESSAGE_BOUND:?}: {error}",
                        self.registry_jobs
                    ),
                }
            } else {
                match self.startup_events.try_recv() {
                    Ok(event) => event,
                    Err(_) if self.registry_jobs == 0 && owed == 0 => return jobs,
                    Err(_) => {
                        waiting = true;
                        continue;
                    }
                }
            };
            match event {
                dmls::StartupEvent::RegistryJobsOutstanding(outstanding) => {
                    owed += self.registry_jobs.saturating_sub(outstanding);
                    self.registry_jobs = outstanding;
                }
                dmls::StartupEvent::RegistryPrepared { job, .. } => {
                    owed = owed.saturating_sub(1);
                    jobs.push(job);
                }
                _ => {}
            }
            if waiting && self.registry_jobs == 0 && owed == 0 {
                return jobs;
            }
        }
    }

    /// Waits for a trigger-registry preparation of kind `job` to finish and
    /// its result to be handled, returning the thread it ran on.
    pub fn wait_for_registry_prepared(&mut self, job: dmls::RegistryJobKind) -> std::thread::ThreadId {
        loop {
            let (seen, thread) = self.next_registry_prepared();
            if seen == job {
                return thread;
            }
        }
    }

    /// Waits for the next trigger-registry preparation to finish and its
    /// result to be handled, returning its kind and the thread it ran on.
    pub fn next_registry_prepared(&mut self) -> (dmls::RegistryJobKind, std::thread::ThreadId) {
        loop {
            match self.next_event() {
                Ok(dmls::StartupEvent::RegistryPrepared { job, thread }) => return (job, thread),
                Ok(_) => {}
                Err(error) => panic!("no registry preparation within {MESSAGE_BOUND:?}: {error}"),
            }
        }
    }

    /// The thread running the server's dispatch loop.
    pub fn server_thread_id(&self) -> std::thread::ThreadId {
        self.server_thread.as_ref().expect("server running").thread().id()
    }

    /// Lets one held index build round through.
    pub fn release_index_round(&self) {
        if let Some(gate) = &self.index_round_gate {
            gate.send(()).expect("index round gate open");
        }
    }

    /// Lets every current and later index build round through.
    pub fn open_index_round_gate(&mut self) {
        self.index_round_gate = None;
    }

    /// Waits for the server to report `event`, skipping other startup events.
    pub fn wait_for_event(&mut self, event: dmls::StartupEvent) {
        loop {
            match self.next_event() {
                Ok(seen) if seen == event => return,
                Ok(_) => {}
                Err(error) => panic!("no {event:?} within {MESSAGE_BOUND:?}: {error}"),
            }
        }
    }

    /// The startup events the server reports, in order, up to and including
    /// the first that `last` accepts.
    pub fn startup_events_until(&mut self, last: impl Fn(&dmls::StartupEvent) -> bool) -> Vec<dmls::StartupEvent> {
        let mut events = Vec::new();
        loop {
            match self.next_event() {
                Ok(event) => {
                    events.push(event);
                    if last(&event) {
                        return events;
                    }
                }
                Err(error) => panic!("no expected startup event within {MESSAGE_BOUND:?}: {error}; saw {events:?}"),
            }
        }
    }

    /// The next startup event, noting every registry job count it carries.
    fn next_event(&mut self) -> Result<dmls::StartupEvent, crossbeam_channel::RecvTimeoutError> {
        let event = self.startup_events.recv_timeout(MESSAGE_BOUND)?;
        if let dmls::StartupEvent::RegistryJobsOutstanding(outstanding) = event {
            self.registry_jobs = outstanding;
        }
        Ok(event)
    }

    /// Waits until the server has no trigger-registry job queued or
    /// running, so every diagnostic and answer those jobs held has been
    /// sent (not necessarily received: [`LspFixture::flush_server`] also
    /// buffers it).
    /// The server reports each count change before it answers the request
    /// whose handling caused it, so after a response this sees every job
    /// queued until then.
    pub fn wait_for_registry_jobs(&mut self) {
        self.drain_registry_job_counts();
        while self.registry_jobs > 0 {
            if let Err(error) = self.next_event() {
                panic!("{} registry jobs still outstanding after {MESSAGE_BOUND:?}: {error}", self.registry_jobs);
            }
        }
    }

    /// Lets a gated file-list walk run.
    pub fn release_file_list(&mut self) {
        if let Some(gate) = self.file_list_gate.take() {
            let _ = gate.send(());
        }
    }

    /// Waits until the server has installed the workspace file list and
    /// refreshed open documents' diagnostics against it.
    pub fn wait_for_file_list(&mut self) {
        self.wait_for_event(dmls::StartupEvent::FileListInstalled);
    }

    fn spawn(
        workspace: &'workspace LspWorkspace,
        epilogue_delay: Duration,
        snapshot: RequestSnapshot,
    ) -> Self {
        Self::spawn_with_hooks(workspace, epilogue_delay, snapshot, None, None)
    }

    fn spawn_with_gate(
        workspace: &'workspace LspWorkspace,
        epilogue_delay: Duration,
        snapshot: RequestSnapshot,
        gate: Option<crossbeam_channel::Receiver<()>>,
    ) -> Self {
        Self::spawn_with_hooks(workspace, epilogue_delay, snapshot, gate, None)
    }

    fn spawn_with_hooks(
        workspace: &'workspace LspWorkspace,
        epilogue_delay: Duration,
        snapshot: RequestSnapshot,
        file_list_gate: Option<crossbeam_channel::Receiver<()>>,
        index_round_gate: Option<crossbeam_channel::Receiver<()>>,
    ) -> Self {
        let hooks = dmls::StartupHooks { file_list_gate, index_round_gate, ..dmls::StartupHooks::default() };
        Self::spawn_with_all_hooks(workspace, epilogue_delay, snapshot, hooks)
    }

    /// Starts the server with `hooks`' gates; the fixture supplies the
    /// observer.
    fn spawn_with_all_hooks(
        workspace: &'workspace LspWorkspace,
        epilogue_delay: Duration,
        snapshot: RequestSnapshot,
        hooks: dmls::StartupHooks,
    ) -> Self {
        Self::spawn_with_clock(workspace, epilogue_delay, snapshot, hooks, None)
    }

    /// [`LspFixture::spawn_with_all_hooks`], timed by `clock` when given.
    fn spawn_with_clock(
        workspace: &'workspace LspWorkspace,
        epilogue_delay: Duration,
        snapshot: RequestSnapshot,
        mut hooks: dmls::StartupHooks,
        clock: Option<Arc<dyn dmls::clock::Clock>>,
    ) -> Self {
        let (events_tx, events_rx) = crossbeam_channel::unbounded();
        hooks.observer = Some(events_tx);
        let (server_side, client_side) = Connection::memory();
        let (outcome_tx, outcome_rx) = mpsc::channel();
        let observations = workspace.observations();
        let worker_observations = Arc::clone(&observations);
        let workspace_root: PathBuf = workspace.path().to_path_buf();
        let server_thread = std::thread::spawn(move || {
            let mut options = dmls::RunOptions::new(snapshot).with_startup_hooks(hooks);
            if let Some(clock) = clock {
                options = options.with_clock(clock);
            }
            let result = dmls::run_server(server_side, options).map_err(|error| error.to_string());
            if !epilogue_delay.is_zero() {
                std::thread::sleep(epilogue_delay);
            }
            // Recording and sending are the worker's last two acts, in this
            // order, so a teardown that observes the outcome has necessarily
            // observed everything the worker body did.
            worker_observations.record(TeardownEvent::WorkerFinished {
                workspace_present: workspace_root.exists(),
            });
            let _ = outcome_tx.send(result);
        });
        Self {
            workspace,
            client: Some(client_side),
            server_outcome: outcome_rx,
            server_thread: Some(server_thread),
            observations,
            clean_shutdown: false,
            next_id: 0,
            notifications: Vec::new(),
            server_requests: Vec::new(),
            startup_events: events_rx,
            file_list_gate: None,
            index_round_gate: None,
            registry_gate: None,
            registry_probe_gate: None,
            startup_probe_gate: None,
            registry_jobs: 0,
        }
    }

    pub fn workspace_path(&self) -> &Path {
        self.workspace.path()
    }

    /// Server-initiated requests seen so far and not yet taken by
    /// [`LspFixture::take_server_request`].
    pub fn buffered_server_requests(&self) -> &[Request] {
        &self.server_requests
    }

    pub fn request(&mut self, method: &str, params: Value) -> Response {
        self.next_id += 1;
        let id = RequestId::from(self.next_id);
        self.send_request_with_id(id.clone(), method, params);
        self.expect_response(id)
    }

    pub fn send_request_with_id(&self, id: RequestId, method: &str, params: Value) {
        self.client
            .as_ref()
            .expect("client connection closed")
            .sender
            .send(Message::Request(Request::new(
                id,
                method.to_string(),
                params,
            )))
            .expect("send request");
    }

    pub fn notify(&self, method: &str, params: Value) {
        self.client
            .as_ref()
            .expect("client connection closed")
            .sender
            .send(Message::Notification(Notification::new(
                method.to_string(),
                params,
            )))
            .expect("send notification");
    }

    pub fn expect_response(&mut self, id: RequestId) -> Response {
        loop {
            let message = self.recv("response");
            match message {
                Message::Response(response) if response.id == id => return response,
                Message::Notification(notification) => self.notifications.push(notification),
                // Server-initiated requests (progress create, watcher
                // registration, semantic-tokens refresh) are fire-and-forget;
                // buffer them so tests can observe what the server pushed.
                Message::Request(request) => self.server_requests.push(request),
                other => panic!("unexpected message while waiting for response: {other:?}"),
            }
        }
    }

    /// Waits for the latest `publishDiagnostics` for `uri`, returning its
    /// `diagnostics` array. Drains buffered notifications first.
    pub fn wait_for_diagnostics(&mut self, uri: &str) -> Vec<Value> {
        self.wait_for_diagnostics_params(uri)["diagnostics"]
            .as_array()
            .cloned()
            .unwrap_or_default()
    }

    pub fn wait_for_diagnostics_params(&mut self, uri: &str) -> Value {
        loop {
            if let Some(params) = self.take_buffered_diagnostics(uri) {
                return params;
            }
            let message = self.recv("diagnostics");
            match message {
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(request) => self.server_requests.push(request),
                other => panic!("unexpected message while waiting for diagnostics: {other:?}"),
            }
        }
    }

    pub fn take_buffered_diagnostics(&mut self, uri: &str) -> Option<Value> {
        let position = self.notifications.iter().rposition(|notification| {
            notification.method == "textDocument/publishDiagnostics"
                && notification.params["uri"] == json!(uri)
        })?;
        let notification = self.notifications.remove(position);
        Some(notification.params)
    }

    /// Drains messages until the startup file-list walk's `workDoneProgress`
    /// `end` (token `dmls/startup-index`) arrives, guaranteeing the list is
    /// installed. Only usable when the client advertised
    /// `window.workDoneProgress`.
    pub fn wait_for_startup_complete(&mut self) {
        self.wait_for_progress_end("dmls/startup-index");
    }

    /// Drains messages until a `workDoneProgress` `end` for `token` arrives.
    pub fn wait_for_progress_end(&mut self, token: &str) {
        let is_end = |notification: &Notification| {
            notification.method == "$/progress"
                && notification.params["token"] == json!(token)
                && notification.params["value"]["kind"] == json!("end")
        };
        loop {
            if let Some(position) = self.notifications.iter().position(is_end) {
                self.notifications.remove(position);
                return;
            }
            let message = self.recv("startup progress");
            match message {
                Message::Notification(notification) => self.notifications.push(notification),
                Message::Request(request) => self.server_requests.push(request),
                other => panic!("unexpected message while waiting for startup: {other:?}"),
            }
        }
    }

    /// Initializes the session and waits for the workspace file list and
    /// for startup's trigger registries, so every later diagnostic is
    /// computed against both. The list installs first; the startup registry
    /// line follows startup's last preparation phase.
    pub fn initialize(&mut self, params: Value) -> Value {
        let result = self.initialize_unlisted(params);
        self.wait_for_startup_events();
        result
    }

    /// Waits for both [`dmls::StartupEvent::FileListInstalled`] and the
    /// startup [`dmls::StartupEvent::RegistryPrepared`], in either order.
    pub fn wait_for_startup_events(&mut self) {
        let (mut listed, mut prepared) = (false, false);
        while !(listed && prepared) {
            match self.next_event() {
                Ok(dmls::StartupEvent::FileListInstalled) => listed = true,
                Ok(dmls::StartupEvent::RegistryPrepared { job: dmls::RegistryJobKind::Startup, .. }) => {
                    prepared = true;
                }
                Ok(_) => {}
                Err(error) => panic!(
                    "startup incomplete within {MESSAGE_BOUND:?} (listed {listed}, prepared {prepared}): {error}"
                ),
            }
        }
    }

    /// Initializes the session without waiting for the file list or for the
    /// startup trigger registries, whose line follows the walk (so a gated
    /// walk would hold it). [`LspFixture::initialize`] waits for both.
    pub fn initialize_unlisted(&mut self, params: Value) -> Value {
        let response = self.request("initialize", params);
        assert!(
            response.error.is_none(),
            "initialize failed: {:?}",
            response.error
        );
        self.notify("initialized", json!({}));
        response.result.expect("initialize result")
    }

    /// Waits until the server has handled every message sent so far, and
    /// not for the trigger-registry jobs they queued (a held job would never
    /// finish).
    pub fn round_trip(&mut self) {
        let response = self.request("workspace/symbol", json!({ "query": "" }));
        assert!(response.error.is_none(), "round-trip request failed: {:?}", response.error);
    }

    /// Waits until the server has handled every message sent so far and
    /// finished the trigger-registry jobs they queued, with every message
    /// the server sent meanwhile buffered.
    pub fn flush_server(&mut self) {
        self.flush_server_with("workspace/symbol", json!({ "query": "" }));
    }

    /// [`LspFixture::flush_server`] with `method` as the round trip, for a
    /// test that must not pay for `workspace/symbol`'s index build.
    pub fn flush_server_with(&mut self, method: &str, params: Value) {
        loop {
            let response = self.request(method, params.clone());
            assert!(
                response.error.is_none(),
                "server flush request failed: {:?}",
                response.error
            );
            // A count that moved may have moved after this response, with
            // what the finished job sent still unread: settle, then make
            // another round trip to buffer it.
            if !self.drain_registry_job_counts() && self.registry_jobs == 0 {
                return;
            }
            self.wait_for_registry_jobs();
        }
    }

    /// Takes the startup events already sent, noting registry job counts.
    ///
    /// ## Returns
    ///
    /// Whether any count arrived.
    fn drain_registry_job_counts(&mut self) -> bool {
        let mut moved = false;
        while let Ok(event) = self.startup_events.try_recv() {
            if let dmls::StartupEvent::RegistryJobsOutstanding(outstanding) = event {
                self.registry_jobs = outstanding;
                moved = true;
            }
        }
        moved
    }

    /// Pumps messages until a buffered server-initiated request with `method`
    /// is seen, removing and returning `true`; returns `false` if none arrives
    /// within a short window (the in-memory pair delivers immediately, so a
    /// miss reliably means the server never sent it).
    pub fn wait_for_server_request(&mut self, method: &str) -> bool {
        self.take_server_request(method).is_some()
    }

    /// Like [`LspFixture::wait_for_server_request`], but returns the matched
    /// `Request` so a caller can inspect its `id` (e.g. proving two refreshes
    /// carry distinct request ids); `None` on the same short-window miss.
    pub fn take_server_request(&mut self, method: &str) -> Option<Request> {
        loop {
            if let Some(position) = self
                .server_requests
                .iter()
                .position(|request| request.method == method)
            {
                return Some(self.server_requests.remove(position));
            }
            match self
                .client
                .as_ref()
                .expect("client connection closed")
                .receiver
                .recv_timeout(Duration::from_secs(1))
            {
                Ok(Message::Request(request)) => self.server_requests.push(request),
                Ok(Message::Notification(notification)) => self.notifications.push(notification),
                Ok(Message::Response(_)) => {}
                Err(_) => return None,
            }
        }
    }

    /// Sends a client → server response, e.g. answering a server-initiated
    /// `workspace/semanticTokens/refresh` so it routes through the loop's
    /// `Message::Response` arm and into the `RefreshLedger`.
    pub fn respond(&self, response: Response) {
        self.client
            .as_ref()
            .expect("client connection closed")
            .sender
            .send(Message::Response(response))
            .expect("send response");
    }

    fn recv(&self, awaiting: &str) -> Message {
        self.client
            .as_ref()
            .expect("client connection closed")
            .receiver
            .recv_timeout(MESSAGE_BOUND)
            .unwrap_or_else(|_| panic!("{awaiting} before timeout"))
    }

    /// Clean shutdown: the protocol handshake, then the shared bounded teardown.
    pub fn shutdown(mut self) {
        let response = self.request("shutdown", Value::Null);
        self.exit_after_shutdown(response);
    }

    /// The teardown of [`LspFixture::shutdown`] for a test that sent the
    /// `shutdown` request itself and holds its `response`.
    pub fn exit_after_shutdown(mut self, response: Response) {
        assert!(
            response.error.is_none(),
            "shutdown failed: {:?}",
            response.error
        );
        self.clean_shutdown = true;
        let problems = self.release_server();
        assert!(
            problems.is_empty(),
            "DMLS session teardown failed: {}",
            problems.join("; ")
        );
    }

    /// Bounded teardown shared by [`LspFixture::shutdown`] and `Drop`.
    ///
    /// Returns the problems observed instead of panicking on them, because the
    /// two callers must react differently: `shutdown` asserts (a clean exit is
    /// its contract) while `Drop` only reports, since panicking a second time
    /// during an unwind aborts the whole test process and destroys the
    /// assertion diagnostic the test exists to produce.
    fn release_server(&mut self) -> Vec<String> {
        let Some(server_thread) = self.server_thread.take() else {
            return Vec::new();
        };
        let mut problems = Vec::new();

        // Ask the server to exit, then close the client end, so a server still
        // blocked in `recv` observes the disconnect even if the notification
        // itself was never dequeued.
        if let Some(client) = self.client.as_ref() {
            let _ = client.sender.send(Message::Notification(Notification::new(
                "exit".to_string(),
                Value::Null,
            )));
        }
        self.client.take();

        match self.server_outcome.recv_timeout(SERVER_EXIT_BOUND) {
            Ok(outcome) => {
                match outcome {
                    Err(error) if self.clean_shutdown => {
                        problems.push(format!("server exited with error: {error}"));
                    }
                    // An aborted session never sent `shutdown`, so the server is
                    // right to complain about the bare `exit`; that is the
                    // cancellation path working, not a teardown failure.
                    Err(_) | Ok(()) => {}
                }
                // The outcome arrived, so the worker body is done and this join
                // waits only for the thread's own epilogue.
                if server_thread.join().is_err() {
                    problems.push("server thread panicked".to_string());
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                // The sender was dropped without sending: the worker unwound.
                // It is finished or finishing, so joining is still bounded.
                if server_thread.join().is_err() {
                    problems.push("server thread panicked".to_string());
                } else {
                    problems.push("server thread ended without reporting an outcome".to_string());
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                // Joining here would be unbounded — there is no timed join — so
                // the worker is deliberately detached and reported instead of
                // hanging the runner forever.
                self.observations.record(TeardownEvent::WorkerAbandoned);
                drop(server_thread);
                problems.push(format!(
                    "server worker did not finish within {SERVER_EXIT_BOUND:?}; detached rather than joined"
                ));
            }
        }
        problems
    }
}

impl Drop for LspFixture<'_> {
    fn drop(&mut self) {
        let problems = self.release_server();
        if problems.is_empty() {
            return;
        }
        let message = format!("DMLS session teardown failed: {}", problems.join("; "));
        if std::thread::panicking() {
            // Panicking while a panic is already unwinding aborts the process
            // and takes the real assertion diagnostic with it, so report and
            // let the original failure stand.
            eprintln!("{message}");
        } else {
            panic!("{message}");
        }
    }
}
