//! Shared in-memory DMLS session fixture for the Level-1 LSP integration
//! binaries: `lsp_session`, `no_side_effects`, and `suggest_constraint_phase1`.
//!
//! `spec.md` requires DMLS sessions to own bounded cleanup on success, failure,
//! *and* cancellation. Each of those three targets previously carried its own
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
//! Each consuming binary uses a different subset of this API, so the module is
//! blanket `dead_code`-exempt rather than carrying per-item exemptions that
//! would drift as tests move between targets.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;

use lsp_server::{Connection, Message, Notification, Request, RequestId, Response};
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
}

impl<'workspace> LspFixture<'workspace> {
    pub fn start(workspace: &'workspace LspWorkspace) -> Self {
        Self::start_with_worker_epilogue_delay(workspace, Duration::ZERO)
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
        let (server_side, client_side) = Connection::memory();
        let (outcome_tx, outcome_rx) = mpsc::channel();
        let observations = workspace.observations();
        let worker_observations = Arc::clone(&observations);
        let workspace_root: PathBuf = workspace.path().to_path_buf();
        let server_thread = std::thread::spawn(move || {
            let result = dmls::run_server(server_side, dmls::RunOptions::default())
                .map_err(|error| error.to_string());
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

    /// Drains messages until the startup-index `workDoneProgress` `end` arrives,
    /// guaranteeing the background disk walk has finished. Only usable when the
    /// client advertised `window.workDoneProgress`.
    pub fn wait_for_startup_complete(&mut self) {
        let is_end = |notification: &Notification| {
            notification.method == "$/progress"
                && notification.params["token"] == json!("dmls/startup-index")
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

    pub fn initialize(&mut self, params: Value) -> Value {
        let response = self.request("initialize", params);
        assert!(
            response.error.is_none(),
            "initialize failed: {:?}",
            response.error
        );
        self.notify("initialized", json!({}));
        response.result.expect("initialize result")
    }

    pub fn flush_server(&mut self) {
        let response = self.request("workspace/symbol", json!({ "query": "" }));
        assert!(
            response.error.is_none(),
            "server flush request failed: {:?}",
            response.error
        );
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
