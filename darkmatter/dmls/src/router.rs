//! The LSP dispatch loop.
//!
//! The router *shape* — a plain `Connection` receiver loop with explicit
//! request/notification dispatch, `shutdown`/`exit` handling, and a panic
//! boundary at the dispatch call — is adapted from the Apache-2.0 `iwes`
//! language server (IWE project, Dmytro Halichenko,
//! <https://github.com/iwe-org/iwe>), per the AD-1 decision record. The
//! giant match stays deliberately small; the Phase 4 provider registry
//! replaces it.
//!
//! Nothing is emitted before `initialize` completes ([`run_server`] finishes
//! the handshake before constructing the router), so clients that cannot
//! decode positions until the encoding is negotiated (Helix) are safe by
//! construction.

use std::collections::{HashMap, HashSet};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_channel::Sender;
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    Cancel, DidChangeConfiguration, DidChangeTextDocument, DidChangeWatchedFiles,
    DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Initialized,
    Notification as _, Progress,
};
use lsp_types::request::{
    CodeActionRequest, Completion, DocumentHighlightRequest, DocumentLinkRequest,
    DocumentSymbolRequest, FoldingRangeRequest, Formatting, GotoDefinition, HoverRequest,
    PrepareRenameRequest, References, RegisterCapability, Rename, Request as _,
    SemanticTokensFullRequest, SemanticTokensRangeRequest, SemanticTokensRefresh,
    WillRenameFiles, WorkDoneProgressCreate, WorkspaceSymbolRequest,
};
use lsp_types::{
    CancelParams, CodeActionParams, CompletionParams, CompletionResponse,
    DidChangeConfigurationParams, DidChangeTextDocumentParams, DidChangeWatchedFilesParams,
    DidCloseTextDocumentParams, DidOpenTextDocumentParams, DidSaveTextDocumentParams,
    DocumentFormattingParams, DocumentHighlightParams, DocumentLinkParams, DocumentSymbolParams,
    DocumentSymbolResponse, FileChangeType, FoldingRangeParams, GotoDefinitionParams,
    GotoDefinitionResponse, HoverParams, InitializeParams, InitializeResult, NumberOrString,
    ProgressParams, ProgressParamsValue, ReferenceParams, RegistrationParams, RenameFilesParams,
    RenameParams, SemanticTokensParams, SemanticTokensRangeParams, SemanticTokensRangeResult,
    SemanticTokensResult, ServerInfo, TextDocumentPositionParams, Uri, WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressCreateParams, WorkDoneProgressEnd,
    WorkDoneProgressReport, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use thiserror::Error;

use crate::capabilities::{ClientProfile, negotiate_position_encoding, server_capabilities};
use crate::config::ConfigState;
use crate::diagnostics::DiagnosticsScheduler;
use crate::graph::{WorkspaceGraph, WorkspaceIndex};
use crate::overlay::OverlayState;
use crate::providers::{DocumentContext, ProviderRegistry};
use crate::workspace::snapshot::SharedSnapshot;
use crate::workspace::startup::{ProgressReporter, SilentProgress, collect_indices};
use crate::workspace::watch::{WatchMode, coalesce_changes, watch_registration};
use crate::workspace::{DocumentStore, uri_to_file_path, workspace_roots};

/// Fatal server errors (protocol violations, not per-request failures).
#[derive(Debug, Error)]
pub enum ServerError {
    /// The LSP handshake or framing failed.
    #[error("LSP protocol error: {0}")]
    Protocol(#[from] lsp_server::ProtocolError),
    /// An LSP payload could not be (de)serialized.
    #[error("failed to (de)serialize an LSP payload: {0}")]
    Json(#[from] serde_json::Error),
    /// The client sent `exit` without a prior `shutdown` request.
    #[error("client sent `exit` before `shutdown`")]
    ExitWithoutShutdown,
    /// The client disconnected without sending `exit`.
    #[error("client disconnected without `exit`")]
    ChannelClosed,
}

/// Server startup options carried from the CLI.
#[derive(Debug, Default)]
pub struct RunOptions {
    /// Explicit `--config` path (wins over `.dmls.toml` discovery).
    pub config_path: Option<PathBuf>,
}

/// Performs the `initialize` handshake and runs the router until `exit`.
///
/// ## Errors
///
/// Returns a [`ServerError`] for protocol-fatal conditions; per-request
/// failures are answered as LSP error responses instead.
pub fn run_server(connection: Connection, options: RunOptions) -> Result<(), ServerError> {
    let (initialize_id, initialize_params) = connection.initialize_start()?;
    let params: InitializeParams = serde_json::from_value(initialize_params)?;

    let encoding = negotiate_position_encoding(&params.capabilities);
    let profile = ClientProfile::from_initialize(&params, encoding);
    tracing::info!(
        client = profile.client_name.as_deref().unwrap_or("unknown"),
        version = profile.client_version.as_deref().unwrap_or("unknown"),
        ?encoding,
        "initializing"
    );

    let result = InitializeResult {
        capabilities: server_capabilities(&profile),
        server_info: Some(ServerInfo {
            name: "dmls".to_string(),
            version: Some(env!("CARGO_PKG_VERSION").to_string()),
        }),
    };
    connection.initialize_finish(initialize_id, serde_json::to_value(result)?)?;

    let roots = workspace_roots(&params);
    let config = ConfigState::load(roots.clone(), options.config_path);

    let wiki_roots = crate::workspace::resolve_wiki_roots(
        &roots,
        config.effective().wiki.wiki_root.as_deref(),
    );
    let index = Arc::new(Mutex::new(WorkspaceIndex::new()));
    let snapshot = {
        let mut guard = index.lock().expect("fresh index lock");
        guard.set_wiki_roots(wiki_roots);
        SharedSnapshot::new(guard.snapshot())
    };

    let watch_mode = WatchMode::for_profile(&profile);
    register_watchers(&connection.sender, watch_mode, config.effective());
    spawn_startup_index(
        connection.sender.clone(),
        roots.clone(),
        config.effective().workspace.clone(),
        Arc::clone(&index),
        snapshot.clone(),
        profile.supports_work_done_progress,
    );

    let diagnostics = DiagnosticsScheduler::new(connection.sender.clone(), config.effective());
    let state = ServerState {
        documents: DocumentStore::new(encoding),
        config,
        profile,
        roots,
        index,
        snapshot,
        registry: ProviderRegistry::with_substrate(),
        diagnostics,
        overlay: OverlayState::default(),
        watch_mode,
    };
    Router::new(connection, state).run()
}

/// Registers dynamic file watchers for a [`WatchMode::ClientWatched`] client.
///
/// A [`WatchMode::ServerRescan`] client (no reliable watcher — e.g. Neovim on
/// Linux) is served by [`ServerState::rescan_workspace`] on save instead, so it
/// needs no registration.
fn register_watchers(sender: &Sender<Message>, watch_mode: WatchMode, config: &crate::DmlsConfig) {
    if watch_mode != WatchMode::ClientWatched {
        return;
    }
    let Some(registration) = watch_registration(&config.workspace) else {
        return;
    };
    let params = RegistrationParams {
        registrations: vec![registration],
    };
    let Ok(value) = serde_json::to_value(params) else {
        return;
    };
    let request = Request::new(
        RequestId::from("dmls/register-watchers".to_string()),
        RegisterCapability::METHOD.to_string(),
        value,
    );
    // Fire-and-forget: the client's response is ignored by the loop.
    let _ = sender.send(Message::Request(request));
}

/// Spawns the background startup indexer (worker pool inside), publishing a
/// snapshot when done and reporting `window/workDoneProgress` on the way.
fn spawn_startup_index(
    sender: Sender<Message>,
    roots: Vec<PathBuf>,
    workspace_config: crate::config::WorkspaceConfig,
    index: Arc<Mutex<WorkspaceIndex>>,
    snapshot: SharedSnapshot,
    progress_enabled: bool,
) {
    std::thread::spawn(move || {
        let reporter = LspProgress::new(sender, progress_enabled);
        let discovered = collect_indices(&roots, &workspace_config, &reporter);
        let mut guard = index.lock().expect("index lock poisoned");
        // Open buffers already in the index win over disk versions.
        guard.adopt_missing(discovered);
        snapshot.store(guard.snapshot());
        tracing::info!(documents = guard.len(), "startup index complete");
    });
}

/// Mutable server state shared by all handlers.
pub struct ServerState {
    /// Open documents (client buffer authoritative).
    pub documents: DocumentStore,
    /// Layered configuration.
    pub config: ConfigState,
    /// Per-client capability gates.
    pub profile: ClientProfile,
    /// Workspace roots (for watch rescans and path resolution).
    pub roots: Vec<PathBuf>,
    /// The workspace index (parse products + live snapshot).
    pub index: Arc<Mutex<WorkspaceIndex>>,
    /// The snapshot readers load.
    pub snapshot: SharedSnapshot,
    /// The ordered provider chain answering feature requests.
    pub registry: ProviderRegistry,
    /// Push-diagnostics scheduler/publisher.
    pub diagnostics: DiagnosticsScheduler,
    /// Frontmatter overlay caches (last-good AST + effective schema).
    pub overlay: OverlayState,
    /// How this client learns about on-disk changes. `ServerRescan` drives a
    /// save-triggered rescan for unopened files; `ClientWatched` relies on
    /// `didChangeWatchedFiles`.
    pub watch_mode: WatchMode,
}

impl ServerState {
    /// Re-indexes an open buffer into the workspace index and publishes the
    /// resulting snapshot. A URI without a filesystem path is ignored.
    fn index_buffer(&self, uri: &lsp_types::Uri, text: &str) {
        let Some(path) = uri_to_file_path(uri) else {
            return;
        };
        let mut index = self.index.lock().expect("index lock poisoned");
        index.set_document(&path, text);
        self.snapshot.store(index.snapshot());
    }

    /// Builds a [`DocumentContext`] for an open document and runs `f` against
    /// it. Returns `None` when the document is not open or lacks a filesystem
    /// path.
    fn with_document<R>(&self, uri: &Uri, f: impl FnOnce(&DocumentContext) -> R) -> Option<R> {
        let open = self.documents.get(uri)?;
        let path = uri_to_file_path(uri)?;
        let snapshot = self.snapshot.load();
        let doc_id = snapshot.document_id(&path);
        let overlay = self.overlay.for_document(
            uri,
            open.text(),
            &path,
            self.config.effective(),
            &self.roots,
        );
        let ctx = DocumentContext {
            uri,
            path: &path,
            text: open.text(),
            source_map: open.source_map().as_ref(),
            graph: snapshot.as_ref(),
            doc_id,
            config: self.config.effective(),
            profile: &self.profile,
            overlay: overlay.as_ref(),
        };
        Some(f(&ctx))
    }

    /// Recomputes and publishes diagnostics for one open document (empty when
    /// it is unknown, which clears the client's markers).
    fn refresh_diagnostics(&self, uri: &Uri) {
        let version = self.documents.get(uri).map(|doc| doc.version());
        let diagnostics = self
            .with_document(uri, |ctx| self.registry.diagnostics(ctx))
            .unwrap_or_default();
        self.diagnostics.publish(uri.clone(), version, diagnostics);
    }

    /// Builds the environment the rename providers need beyond the request
    /// payload (open buffers, snapshot, config/profile, roots, encoding).
    fn rename_env<'a>(
        &'a self,
        graph: &'a WorkspaceGraph,
    ) -> crate::providers::rename::RenameEnv<'a> {
        crate::providers::rename::RenameEnv {
            documents: &self.documents,
            graph,
            config: self.config.effective(),
            profile: &self.profile,
            roots: &self.roots,
            encoding: self.profile.position_encoding,
        }
    }

    /// Refreshes diagnostics for every open document — a change to one file can
    /// flip another's link validity, so the whole open set is re-published.
    fn refresh_all_diagnostics(&self) {
        for uri in self.documents.open_uris() {
            self.refresh_diagnostics(&uri);
        }
        let mut trigger_transitions = HashMap::new();
        for transition in self.overlay.take_trigger_diagnostic_transitions() {
            trigger_transitions.insert(transition.path, transition.error);
        }
        for (path, error) in trigger_transitions {
            let Some(uri) = crate::workspace::file_path_to_uri(&path) else {
                continue;
            };
            // Open-buffer diagnostics are versioned and derive from the
            // authoritative in-memory text. Recompute them after every scan
            // has settled because a later document can move or clear trigger
            // failure ownership after the envelope's first refresh.
            if self.documents.is_open(&uri) {
                self.refresh_diagnostics(&uri);
                continue;
            }
            let diagnostics = error
                .as_deref()
                .map(crate::diagnostics::frontmatter::trigger_load_diagnostic)
                .into_iter()
                .collect();
            self.diagnostics.publish(uri, None, diagnostics);
        }
    }

    /// The filesystem paths of all currently open buffers (buffer authoritative
    /// over disk during a rescan).
    fn open_document_paths(&self) -> HashSet<PathBuf> {
        self.documents
            .open_uris()
            .iter()
            .filter_map(uri_to_file_path)
            .collect()
    }

    /// Rescans the workspace from disk and reconciles the index for
    /// [`WatchMode::ServerRescan`] clients (no reliable file watcher).
    ///
    /// This is the save-driven fallback that keeps unopened files current for
    /// clients like Neovim on Linux: it re-runs discovery, reads and re-hashes
    /// the discovered files on the worker pool, and adopts created/changed and
    /// drops deleted files (open buffers stay authoritative). It reads disk but
    /// never composes, executes, or fetches — the same passive contract as
    /// startup indexing.
    ///
    /// ## Returns
    ///
    /// `true` when the snapshot changed, so the caller refreshes diagnostics.
    fn rescan_workspace(&self) -> bool {
        let discovered = collect_indices(
            &self.roots,
            &self.config.effective().workspace,
            &SilentProgress,
        );
        let open_paths = self.open_document_paths();
        let mut index = self.index.lock().expect("index lock poisoned");
        let changed = index.reconcile_disk(discovered, &open_paths);
        if changed {
            self.snapshot.store(index.snapshot());
        }
        changed
    }

    /// Reloads configuration after a `workspace/didChangeConfiguration` and
    /// re-derives every part of the analysis universe that depends on it, so a
    /// settings change takes effect without a server restart.
    ///
    /// Beyond storing the new settings, this recomputes the wiki resolution
    /// roots (`wiki.wiki_root`) and rebuilds the graph against them, re-discovers
    /// and reconciles the workspace against the new discovery globs
    /// (`workspace.include`/`exclude`), and re-publishes diagnostics for every
    /// open document. Schema-extension activation (`schema.extensions`) flows
    /// through the overlay's content-and-config-keyed schema cache, so it
    /// re-assembles on the diagnostics refresh by construction. Nothing runs
    /// when the effective config is unchanged, and the expensive re-discovery
    /// runs only when the `workspace` section actually changed.
    ///
    /// ## Returns
    ///
    /// `true` when a token-affecting change occurred and the client honors
    /// `workspace/semanticTokens/refresh`, so the caller (which owns the
    /// connection) sends the refresh. The new config already applies to every
    /// later request regardless of this signal.
    fn reload_config(&mut self, settings: serde_json::Value) -> bool {
        let before = self.config.effective().clone();
        self.config.apply_client_settings(settings);
        let after = self.config.effective();
        if *after == before {
            return false;
        }
        let refresh_semantic_tokens =
            should_request_semantic_tokens_refresh(&self.profile, &before, after);

        let wiki_roots = (after.wiki.wiki_root != before.wiki.wiki_root).then(|| {
            crate::workspace::resolve_wiki_roots(&self.roots, after.wiki.wiki_root.as_deref())
        });
        let discovered = (after.workspace != before.workspace)
            .then(|| collect_indices(&self.roots, &after.workspace, &SilentProgress));
        let open_paths = self.open_document_paths();

        {
            let mut index = self.index.lock().expect("index lock poisoned");
            let mut dirty = false;
            if let Some(wiki_roots) = wiki_roots {
                index.set_wiki_roots(wiki_roots);
                dirty = true;
            }
            if let Some(discovered) = discovered {
                dirty |= index.reconcile_disk(discovered, &open_paths);
            }
            if dirty {
                self.snapshot.store(index.snapshot());
            }
        }
        self.refresh_all_diagnostics();
        refresh_semantic_tokens
    }

    /// Applies a coalesced on-disk change: read + re-index, or drop.
    fn apply_disk_change(&self, path: &Path, kind: FileChangeType) {
        let mut index = self.index.lock().expect("index lock poisoned");
        match kind {
            FileChangeType::DELETED => {
                index.remove_document(path);
            }
            _ => match std::fs::read_to_string(path) {
                Ok(source) => {
                    index.set_document(path, &source);
                }
                Err(error) => {
                    tracing::debug!("watched change unreadable {}: {error}", path.display());
                    return;
                }
            },
        }
        self.snapshot.store(index.snapshot());
    }
}

/// A [`ProgressReporter`] that drives `window/workDoneProgress` over the LSP
/// connection, throttled to 5% steps so a large workspace does not flood the
/// client with notifications.
struct LspProgress {
    sender: Sender<Message>,
    enabled: bool,
    last_step: AtomicUsize,
}

impl LspProgress {
    const TOKEN: &'static str = "dmls/startup-index";

    fn new(sender: Sender<Message>, enabled: bool) -> Self {
        Self {
            sender,
            enabled,
            last_step: AtomicUsize::new(usize::MAX),
        }
    }

    fn send_progress(&self, value: WorkDoneProgress) {
        let params = ProgressParams {
            token: NumberOrString::String(Self::TOKEN.to_string()),
            value: ProgressParamsValue::WorkDone(value),
        };
        if let Ok(value) = serde_json::to_value(params) {
            let notification = Notification::new(Progress::METHOD.to_string(), value);
            let _ = self.sender.send(Message::Notification(notification));
        }
    }
}

impl ProgressReporter for LspProgress {
    fn begin(&self, title: &str) {
        if !self.enabled {
            return;
        }
        // Server-initiated progress requires a create round-trip; the client's
        // response is ignored by the loop.
        let create = WorkDoneProgressCreateParams {
            token: NumberOrString::String(Self::TOKEN.to_string()),
        };
        if let Ok(value) = serde_json::to_value(create) {
            let request = Request::new(
                RequestId::from("dmls/progress-create".to_string()),
                WorkDoneProgressCreate::METHOD.to_string(),
                value,
            );
            let _ = self.sender.send(Message::Request(request));
        }
        self.send_progress(WorkDoneProgress::Begin(WorkDoneProgressBegin {
            title: title.to_string(),
            cancellable: Some(false),
            message: None,
            percentage: Some(0),
        }));
    }

    fn report(&self, done: usize, total: usize) {
        if !self.enabled || total == 0 {
            return;
        }
        let step = done * 20 / total;
        let previous = self.last_step.swap(step, Ordering::SeqCst);
        if step == previous && done != total {
            return;
        }
        self.send_progress(WorkDoneProgress::Report(WorkDoneProgressReport {
            cancellable: Some(false),
            message: Some(format!("{done}/{total} files")),
            percentage: Some((done * 100 / total) as u32),
        }));
    }

    fn end(&self) {
        if !self.enabled {
            return;
        }
        self.send_progress(WorkDoneProgress::End(WorkDoneProgressEnd { message: None }));
    }
}

/// Bounded `$/cancelRequest` bookkeeping.
///
/// Dispatch is currently synchronous, so a cancellation can only beat its
/// request when the client misbehaves — but the ledger becomes load-bearing
/// once indexing moves to the worker pool (Phase 3), and it lets queued
/// requests be answered `RequestCanceled` without running.
#[derive(Debug, Default)]
struct CancelLedger {
    ids: Vec<RequestId>,
}

impl CancelLedger {
    const CAPACITY: usize = 64;

    fn record(&mut self, id: RequestId) {
        if self.ids.len() == Self::CAPACITY {
            self.ids.remove(0);
        }
        self.ids.push(id);
    }

    fn take(&mut self, id: &RequestId) -> bool {
        match self.ids.iter().position(|candidate| candidate == id) {
            Some(index) => {
                self.ids.remove(index);
                true
            }
            None => false,
        }
    }
}

/// Tracks outstanding server-initiated `workspace/semanticTokens/refresh`
/// requests.
///
/// Each send gets a unique id (`dmls/semantic-tokens-refresh/<n>`) so two
/// config changes in flight before the first response do not collide on one
/// id and make the client's responses ambiguous. The loop retires responses by
/// id and logs a client rejection.
#[derive(Debug, Default)]
struct RefreshLedger {
    next: u64,
    outstanding: HashSet<RequestId>,
}

impl RefreshLedger {
    const PREFIX: &'static str = "dmls/semantic-tokens-refresh";

    /// Allocates a unique refresh request id and records it as outstanding.
    fn issue(&mut self) -> RequestId {
        let id = RequestId::from(format!("{}/{}", Self::PREFIX, self.next));
        self.next += 1;
        self.outstanding.insert(id.clone());
        id
    }

    /// Consumes a client response addressed to a refresh request.
    ///
    /// A matching outstanding id is retired; a carried error is logged at
    /// `warn` (a client refusing the refresh must surface, but must not fail
    /// the loop), a success is retired quietly at `debug`.
    ///
    /// ## Returns
    ///
    /// `true` when the response matched a tracked refresh id (now retired);
    /// `false` when the id is not a refresh request the caller should keep its
    /// existing handling for.
    fn consume(&mut self, response: &Response) -> bool {
        if !self.outstanding.remove(&response.id) {
            return false;
        }
        match &response.error {
            Some(error) => tracing::warn!(
                id = %response.id,
                code = error.code,
                "client rejected semantic-tokens refresh: {}",
                error.message
            ),
            None => tracing::debug!(id = %response.id, "semantic-tokens refresh acknowledged"),
        }
        true
    }
}

/// The dispatch loop.
pub struct Router {
    connection: Connection,
    state: ServerState,
    cancelled: CancelLedger,
    refresh: RefreshLedger,
}

impl Router {
    /// Builds a router over an initialized connection.
    pub fn new(connection: Connection, state: ServerState) -> Self {
        Self {
            connection,
            state,
            cancelled: CancelLedger::default(),
            refresh: RefreshLedger::default(),
        }
    }

    /// Runs until `shutdown`/`exit` (Ok) or a protocol-fatal error.
    pub fn run(mut self) -> Result<(), ServerError> {
        loop {
            let message = self
                .connection
                .receiver
                .recv()
                .map_err(|_| ServerError::ChannelClosed)?;
            match message {
                Message::Request(request) => {
                    if self.connection.handle_shutdown(&request)? {
                        return Ok(());
                    }
                    self.handle_request(request);
                }
                Message::Notification(notification) => {
                    if notification.method == "exit" {
                        return Err(ServerError::ExitWithoutShutdown);
                    }
                    self.handle_notification(notification);
                }
                Message::Response(response) => {
                    // Retire an outstanding refresh (logging a client rejection);
                    // an untracked id keeps the original ignore behavior.
                    if !self.refresh.consume(&response) {
                        tracing::debug!(id = %response.id, "ignoring client response");
                    }
                }
            }
        }
    }

    /// Panic boundary: a panicking handler answers `InternalError` instead
    /// of taking down the request loop.
    fn handle_request(&mut self, request: Request) {
        let id = request.id.clone();
        if self.cancelled.take(&id) {
            self.respond(Response::new_err(
                id,
                ErrorCode::RequestCanceled as i32,
                "request was cancelled".to_string(),
            ));
            return;
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| self.dispatch_request(request)));
        match outcome {
            Ok(response) => self.respond(response),
            Err(_) => {
                tracing::error!(id = %id, "request handler panicked");
                self.respond(Response::new_err(
                    id,
                    ErrorCode::InternalError as i32,
                    "request handler panicked".to_string(),
                ));
            }
        }
    }

    fn dispatch_request(&mut self, request: Request) -> Response {
        let id = request.id.clone();
        match request.method.as_str() {
            DocumentSymbolRequest::METHOD => self.document_symbol(id, request.params),
            WorkspaceSymbolRequest::METHOD => self.workspace_symbol(id, request.params),
            GotoDefinition::METHOD => self.definition(id, request.params),
            DocumentLinkRequest::METHOD => self.document_link(id, request.params),
            References::METHOD => self.references(id, request.params),
            DocumentHighlightRequest::METHOD => self.document_highlight(id, request.params),
            FoldingRangeRequest::METHOD => self.folding_range(id, request.params),
            HoverRequest::METHOD => self.hover(id, request.params),
            SemanticTokensFullRequest::METHOD => self.semantic_tokens_full(id, request.params),
            SemanticTokensRangeRequest::METHOD => self.semantic_tokens_range(id, request.params),
            Completion::METHOD => self.completion(id, request.params),
            CodeActionRequest::METHOD => self.code_action(id, request.params),
            Formatting::METHOD => self.formatting(id, request.params),
            PrepareRenameRequest::METHOD => self.prepare_rename(id, request.params),
            Rename::METHOD => self.rename(id, request.params),
            WillRenameFiles::METHOD => self.will_rename_files(id, request.params),
            other => {
                tracing::debug!(method = other, "unhandled request");
                Response::new_err(
                    id,
                    ErrorCode::MethodNotFound as i32,
                    format!("dmls does not support `{other}` yet"),
                )
            }
        }
    }

    fn document_symbol(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<DocumentSymbolParams>(params) else {
            return invalid_params(id);
        };
        let symbols = self
            .state
            .with_document(&params.text_document.uri, |ctx| {
                self.state.registry.document_symbols(ctx)
            })
            .unwrap_or_default();
        Response::new_ok(id, DocumentSymbolResponse::Nested(symbols))
    }

    fn workspace_symbol(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<WorkspaceSymbolParams>(params) else {
            return invalid_params(id);
        };
        let snapshot = self.state.snapshot.load();
        let symbols = self
            .state
            .registry
            .workspace_symbols(&params.query, snapshot.as_ref());
        Response::new_ok(id, WorkspaceSymbolResponse::Nested(symbols))
    }

    fn definition(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<GotoDefinitionParams>(params) else {
            return invalid_params(id);
        };
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let locations = self
            .state
            .with_document(&uri, |ctx| match ctx.source_map.lsp_to_byte(position) {
                Some(offset) => self.state.registry.definition(ctx, offset),
                None => Vec::new(),
            })
            .unwrap_or_default();
        Response::new_ok(id, GotoDefinitionResponse::Array(locations))
    }

    fn document_link(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<DocumentLinkParams>(params) else {
            return invalid_params(id);
        };
        let links = self
            .state
            .with_document(&params.text_document.uri, |ctx| {
                self.state.registry.document_links(ctx)
            })
            .unwrap_or_default();
        Response::new_ok(id, links)
    }

    fn references(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<ReferenceParams>(params) else {
            return invalid_params(id);
        };
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        let include = params.context.include_declaration;
        let locations = self
            .state
            .with_document(&uri, |ctx| match ctx.source_map.lsp_to_byte(position) {
                Some(offset) => self.state.registry.references(ctx, offset, include),
                None => Vec::new(),
            })
            .unwrap_or_default();
        Response::new_ok(id, locations)
    }

    fn document_highlight(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<DocumentHighlightParams>(params) else {
            return invalid_params(id);
        };
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let highlights = self
            .state
            .with_document(&uri, |ctx| match ctx.source_map.lsp_to_byte(position) {
                Some(offset) => self.state.registry.document_highlights(ctx, offset),
                None => Vec::new(),
            })
            .unwrap_or_default();
        Response::new_ok(id, highlights)
    }

    fn folding_range(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<FoldingRangeParams>(params) else {
            return invalid_params(id);
        };
        let folds = self
            .state
            .with_document(&params.text_document.uri, |ctx| {
                self.state.registry.folding_ranges(ctx)
            })
            .unwrap_or_default();
        Response::new_ok(id, folds)
    }

    fn hover(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<HoverParams>(params) else {
            return invalid_params(id);
        };
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        let hover = self
            .state
            .with_document(&uri, |ctx| {
                ctx.source_map
                    .lsp_to_byte(position)
                    .and_then(|offset| self.state.registry.hover(ctx, offset))
            })
            .flatten();
        Response::new_ok(id, hover)
    }

    fn semantic_tokens_full(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<SemanticTokensParams>(params) else {
            return invalid_params(id);
        };
        // A missing document answers `null` (no tokens), the same empty-result
        // convention neighboring providers use for an unknown document.
        let result = self
            .state
            .with_document(&params.text_document.uri, |ctx| {
                SemanticTokensResult::Tokens(crate::providers::semantic_tokens::semantic_tokens_full(
                    ctx,
                ))
            });
        Response::new_ok(id, result)
    }

    fn semantic_tokens_range(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<SemanticTokensRangeParams>(params) else {
            return invalid_params(id);
        };
        let range = params.range;
        let result = self
            .state
            .with_document(&params.text_document.uri, |ctx| {
                SemanticTokensRangeResult::Tokens(
                    crate::providers::semantic_tokens::semantic_tokens_range(ctx, range),
                )
            });
        Response::new_ok(id, result)
    }

    fn completion(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<CompletionParams>(params) else {
            return invalid_params(id);
        };
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        let items = self
            .state
            .with_document(&uri, |ctx| match ctx.source_map.lsp_to_byte(position) {
                Some(offset) => self.state.registry.completion(ctx, offset),
                None => Vec::new(),
            })
            .unwrap_or_default();
        Response::new_ok(id, CompletionResponse::Array(items))
    }

    fn code_action(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<CodeActionParams>(params) else {
            return invalid_params(id);
        };
        let uri = params.text_document.uri;
        let diagnostics = params.context.diagnostics;
        let actions = self
            .state
            .with_document(&uri, |ctx| {
                crate::providers::code_actions::code_actions(ctx, &diagnostics)
            })
            .unwrap_or_default();
        Response::new_ok(id, actions)
    }

    fn formatting(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<DocumentFormattingParams>(params) else {
            return invalid_params(id);
        };
        let edits = self
            .state
            .with_document(&params.text_document.uri, |ctx| {
                crate::providers::formatting::formatting(ctx)
            })
            .unwrap_or_default();
        Response::new_ok(id, edits)
    }

    fn prepare_rename(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<TextDocumentPositionParams>(params) else {
            return invalid_params(id);
        };
        let snapshot = self.state.snapshot.load();
        let env = self.state.rename_env(snapshot.as_ref());
        let response = crate::providers::rename::prepare_rename(
            &env,
            &params.text_document.uri,
            params.position,
        );
        Response::new_ok(id, response)
    }

    fn rename(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<RenameParams>(params) else {
            return invalid_params(id);
        };
        let position = params.text_document_position.position;
        let uri = params.text_document_position.text_document.uri;
        let snapshot = self.state.snapshot.load();
        let env = self.state.rename_env(snapshot.as_ref());
        match crate::providers::rename::rename(&env, &uri, position, &params.new_name) {
            Ok(edit) => Response::new_ok(id, edit),
            // Refuse unsafe/ambiguous renames with an error the client surfaces,
            // rather than applying a partial edit (spec criterion 9).
            Err(error) => Response::new_err(
                id,
                ErrorCode::InvalidRequest as i32,
                error.message().to_string(),
            ),
        }
    }

    fn will_rename_files(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<RenameFilesParams>(params) else {
            return invalid_params(id);
        };
        let snapshot = self.state.snapshot.load();
        let env = self.state.rename_env(snapshot.as_ref());
        let edit = crate::providers::rename::will_rename_files(&env, &params.files);
        Response::new_ok(id, edit)
    }

    fn handle_notification(&mut self, notification: Notification) {
        let method = notification.method.clone();
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            self.dispatch_notification(notification)
        }));
        if outcome.is_err() {
            tracing::error!(method = %method, "notification handler panicked");
        }
    }

    fn dispatch_notification(&mut self, notification: Notification) {
        match notification.method.as_str() {
            DidOpenTextDocument::METHOD => {
                let Some(params) =
                    parse_params::<DidOpenTextDocumentParams>(notification.params)
                else {
                    return;
                };
                let document = params.text_document;
                tracing::debug!(uri = document.uri.as_str(), version = document.version, "didOpen");
                let uri = document.uri;
                let text = document.text;
                self.state
                    .documents
                    .open(uri.clone(), document.version, text.clone());
                self.state.index_buffer(&uri, &text);
                self.state.refresh_all_diagnostics();
            }
            DidChangeTextDocument::METHOD => {
                let Some(params) =
                    parse_params::<DidChangeTextDocumentParams>(notification.params)
                else {
                    return;
                };
                self.apply_change(params);
            }
            DidCloseTextDocument::METHOD => {
                let Some(params) =
                    parse_params::<DidCloseTextDocumentParams>(notification.params)
                else {
                    return;
                };
                let uri = params.text_document.uri;
                tracing::debug!(uri = uri.as_str(), "didClose");
                if let Err(error) = self.state.documents.close(&uri) {
                    tracing::warn!("{error}");
                }
                self.state.overlay.forget(&uri);
                // Clear the closed document's markers, then refresh the rest
                // (its links may have propped up another document's validity).
                self.state.diagnostics.publish(uri, None, Vec::new());
                self.state.refresh_all_diagnostics();
            }
            DidSaveTextDocument::METHOD => {
                let Some(params) =
                    parse_params::<DidSaveTextDocumentParams>(notification.params)
                else {
                    return;
                };
                // Full sync keeps the saved buffer itself authoritative, so the
                // save carries no new state for that document. It is, however,
                // the trigger for the server-side rescan fallback: a
                // `ServerRescan` client (no reliable watcher) learns about
                // *other* files that were created/changed/deleted on disk only
                // here — a `ClientWatched` client gets that from
                // `didChangeWatchedFiles`, so it skips the scan.
                tracing::debug!(uri = params.text_document.uri.as_str(), "didSave");
                if self.state.watch_mode == WatchMode::ServerRescan {
                    self.state.rescan_workspace();
                    // Schema-root YAML is intentionally outside Markdown
                    // workspace discovery, but a save is also the watcher-less
                    // trigger-registry rescan boundary.
                    self.state.refresh_all_diagnostics();
                }
            }
            DidChangeConfiguration::METHOD => {
                let Some(params) =
                    parse_params::<DidChangeConfigurationParams>(notification.params)
                else {
                    return;
                };
                tracing::debug!("didChangeConfiguration");
                if self.state.reload_config(params.settings) {
                    self.send_semantic_tokens_refresh();
                }
            }
            DidChangeWatchedFiles::METHOD => {
                let Some(params) =
                    parse_params::<DidChangeWatchedFilesParams>(notification.params)
                else {
                    return;
                };
                self.apply_watched_changes(params);
            }
            Cancel::METHOD => {
                let Some(params) = parse_params::<CancelParams>(notification.params) else {
                    return;
                };
                let id = match params.id {
                    NumberOrString::Number(number) => RequestId::from(number),
                    NumberOrString::String(string) => RequestId::from(string),
                };
                tracing::debug!(id = %id, "cancelRequest");
                self.cancelled.record(id);
            }
            Initialized::METHOD => {}
            other => {
                tracing::debug!(method = other, "ignoring notification");
            }
        }
    }

    fn apply_change(&mut self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let version = params.text_document.version;
        tracing::debug!(uri = uri.as_str(), version, "didChange");
        // Full-document sync: only whole-text replacement events are valid.
        let mut applied_text: Option<String> = None;
        for change in params.content_changes {
            if change.range.is_some() {
                tracing::warn!(
                    uri = uri.as_str(),
                    "ignoring ranged change: dmls advertises full document sync"
                );
                continue;
            }
            match self.state.documents.change(&uri, version, change.text) {
                Ok(()) => {
                    applied_text = self.state.documents.get(&uri).map(|d| d.text().to_string());
                }
                Err(error) => tracing::warn!("{error}"),
            }
        }
        if let Some(text) = applied_text {
            self.state.index_buffer(&uri, &text);
            self.state.refresh_all_diagnostics();
        }
    }

    /// Applies coalesced `didChangeWatchedFiles` events to the workspace index.
    fn apply_watched_changes(&mut self, params: DidChangeWatchedFilesParams) {
        let events = params.changes.into_iter().filter_map(|event| {
            uri_to_file_path(&event.uri).map(|path| (path, event.typ))
        });
        for change in coalesce_changes(events) {
            // An open buffer is authoritative over disk; skip its watch events.
            if let Some(uri) = crate::workspace::file_path_to_uri(&change.path)
                && self.state.documents.is_open(&uri)
            {
                continue;
            }
            tracing::debug!(path = %change.path.display(), "watched change");
            self.state.apply_disk_change(&change.path, change.kind);
        }
        // A disk change can flip an open document's link validity.
        self.state.refresh_all_diagnostics();
    }

    fn respond(&self, response: Response) {
        if let Err(error) = self.connection.sender.send(Message::Response(response)) {
            tracing::warn!("failed to send response: {error}");
        }
    }

    /// Requests `workspace/semanticTokens/refresh` with a freshly allocated,
    /// unique id, tracked as outstanding until the client responds.
    fn send_semantic_tokens_refresh(&mut self) {
        let id = self.refresh.issue();
        send_refresh_request(&self.connection.sender, id);
    }
}

/// Builds an `InvalidParams` error response for a malformed request payload.
fn invalid_params(id: RequestId) -> Response {
    Response::new_err(
        id,
        ErrorCode::InvalidParams as i32,
        "malformed request params".to_string(),
    )
}

/// Whether a config change affects the semantic-token stream.
///
/// Only two knobs change the emitted tokens: the `semantic_tokens.enable`
/// master switch and `wiki.enable` (which gates the F4 wiki family). Any other
/// config change leaves the token stream identical, so no refresh is warranted.
fn semantic_tokens_refresh_needed(before: &crate::DmlsConfig, after: &crate::DmlsConfig) -> bool {
    before.semantic_tokens != after.semantic_tokens || before.wiki.enable != after.wiki.enable
}

/// Whether the server should send `workspace/semanticTokens/refresh` after a
/// config change: the change must be token-affecting **and** the client must
/// honor the refresh request. An incapable client still gets the new config on
/// its next full/range request — it just is not proactively nudged.
fn should_request_semantic_tokens_refresh(
    profile: &ClientProfile,
    before: &crate::DmlsConfig,
    after: &crate::DmlsConfig,
) -> bool {
    profile.supports_semantic_tokens_refresh && semantic_tokens_refresh_needed(before, after)
}

/// Sends a `workspace/semanticTokens/refresh` request under a pre-allocated id.
///
/// The client's response is retired later by the loop via [`RefreshLedger`]; a
/// send failure (the client is gone) is logged rather than propagated, so a
/// token-config change never fails the `didChangeConfiguration` notification
/// that triggered it.
fn send_refresh_request(sender: &Sender<Message>, id: RequestId) {
    let request = Request::new(
        id,
        SemanticTokensRefresh::METHOD.to_string(),
        serde_json::Value::Null,
    );
    if let Err(error) = sender.send(Message::Request(request)) {
        tracing::warn!("failed to request semantic-tokens refresh: {error}");
    }
}

/// Deserializes notification params, logging (not crashing) on mismatch.
fn parse_params<P: serde::de::DeserializeOwned>(params: serde_json::Value) -> Option<P> {
    match serde_json::from_value(params) {
        Ok(parsed) => Some(parsed),
        Err(error) => {
            tracing::warn!("malformed notification params: {error}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cancel_ledger_take_removes() {
        let mut ledger = CancelLedger::default();
        ledger.record(RequestId::from(7));
        assert!(ledger.take(&RequestId::from(7)));
        assert!(!ledger.take(&RequestId::from(7)));
    }

    #[test]
    fn test_cancel_ledger_bounded() {
        let mut ledger = CancelLedger::default();
        for id in 0..(CancelLedger::CAPACITY as i32 + 10) {
            ledger.record(RequestId::from(id));
        }
        assert_eq!(ledger.ids.len(), CancelLedger::CAPACITY);
        // The oldest entries were evicted; the newest survive.
        assert!(!ledger.take(&RequestId::from(0)));
        assert!(ledger.take(&RequestId::from(CancelLedger::CAPACITY as i32 + 9)));
    }

    #[test]
    fn test_cancel_ledger_string_and_number_ids_distinct() {
        let mut ledger = CancelLedger::default();
        ledger.record(RequestId::from("42".to_string()));
        assert!(!ledger.take(&RequestId::from(42)));
        assert!(ledger.take(&RequestId::from("42".to_string())));
    }

    /// A client profile whose only interesting bit is refresh support.
    fn profile_with_refresh(refresh: bool) -> ClientProfile {
        let caps = if refresh {
            serde_json::json!({ "workspace": { "semanticTokens": { "refreshSupport": true } } })
        } else {
            serde_json::json!({})
        };
        let params: InitializeParams =
            serde_json::from_value(serde_json::json!({ "capabilities": caps })).unwrap();
        ClientProfile::from_initialize(&params, crate::source_map::PositionEncoding::Utf16)
    }

    #[test]
    fn test_semantic_tokens_refresh_needed_detects_token_knobs() {
        let base = crate::DmlsConfig::default();

        let mut enable_off = base.clone();
        enable_off.semantic_tokens.enable = false;
        assert!(semantic_tokens_refresh_needed(&base, &enable_off));

        let mut wiki_off = base.clone();
        wiki_off.wiki.enable = false;
        assert!(semantic_tokens_refresh_needed(&base, &wiki_off));

        // A non-token change is not token-affecting.
        let mut debounce = base.clone();
        debounce.diagnostics.debounce_ms = 999;
        assert!(!semantic_tokens_refresh_needed(&base, &debounce));

        assert!(!semantic_tokens_refresh_needed(&base, &base));
    }

    #[test]
    fn test_should_request_refresh_gated_on_capability() {
        let before = crate::DmlsConfig::default();
        let mut after = before.clone();
        after.semantic_tokens.enable = false;

        // Capable client + token-affecting change → send refresh.
        let capable = profile_with_refresh(true);
        assert!(should_request_semantic_tokens_refresh(
            &capable, &before, &after
        ));

        // Incapable client: config still applies to later requests, but no
        // proactive refresh is sent.
        let incapable = profile_with_refresh(false);
        assert!(!should_request_semantic_tokens_refresh(
            &incapable, &before, &after
        ));

        // Capable client + no token-affecting change → nothing to refresh.
        assert!(!should_request_semantic_tokens_refresh(
            &capable, &before, &before
        ));
    }

    #[test]
    fn test_send_refresh_request_isolates_failure() {
        let (sender, receiver) = crossbeam_channel::unbounded::<Message>();
        // The client is gone: the send fails and is swallowed rather than
        // panicking or failing the notification.
        drop(receiver);
        send_refresh_request(
            &sender,
            RequestId::from("dmls/semantic-tokens-refresh/0".to_string()),
        );
    }

    #[test]
    fn test_refresh_ledger_issues_distinct_tracked_ids() {
        let mut ledger = RefreshLedger::default();
        let first = ledger.issue();
        let second = ledger.issue();
        // Two sends before any response must not collide on one id, or the
        // client's responses become ambiguous.
        assert_ne!(first, second);
        assert!(ledger.outstanding.contains(&first));
        assert!(ledger.outstanding.contains(&second));
        assert_eq!(ledger.outstanding.len(), 2);
    }

    #[test]
    fn test_refresh_ledger_consumes_error_response() {
        let mut ledger = RefreshLedger::default();
        let id = ledger.issue();
        let response = Response::new_err(
            id.clone(),
            ErrorCode::InternalError as i32,
            "client refused refresh".to_string(),
        );
        // A rejection is consumed (and logged) without failing the loop, and
        // the id is retired from tracking.
        assert!(ledger.consume(&response));
        assert!(!ledger.outstanding.contains(&id));
    }

    #[test]
    fn test_refresh_ledger_consumes_success_response() {
        let mut ledger = RefreshLedger::default();
        let id = ledger.issue();
        let response = Response::new_ok(id.clone(), serde_json::Value::Null);
        assert!(ledger.consume(&response));
        assert!(!ledger.outstanding.contains(&id));
    }

    #[test]
    fn test_refresh_ledger_ignores_unrelated_response() {
        let mut ledger = RefreshLedger::default();
        let tracked = ledger.issue();
        let response = Response::new_ok(RequestId::from(999), serde_json::Value::Null);
        // An unrelated id is not a refresh response: the caller keeps its own
        // handling and the outstanding set is undisturbed.
        assert!(!ledger.consume(&response));
        assert!(ledger.outstanding.contains(&tracked));
        assert_eq!(ledger.outstanding.len(), 1);
    }
}
