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
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crossbeam_channel::{Receiver, Sender};
use lsp_server::{Connection, ErrorCode, Message, Notification, Request, RequestId, Response};
use lsp_types::notification::{
    Cancel, DidChangeConfiguration, DidChangeTextDocument, DidChangeWatchedFiles,
    DidCloseTextDocument, DidOpenTextDocument, DidSaveTextDocument, Initialized,
    Notification as _, Progress, ShowMessage,
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
    GotoDefinitionResponse, HoverParams, InitializeParams, InitializeResult, MessageType,
    NumberOrString,
    ProgressParams, ProgressParamsValue, ReferenceParams, RegistrationParams, RenameFilesParams,
    RenameParams, SemanticTokensParams, SemanticTokensRangeParams, SemanticTokensRangeResult,
    SemanticTokensResult, ServerInfo, ShowMessageParams, TextDocumentPositionParams, Uri,
    WorkDoneProgress,
    WorkDoneProgressBegin, WorkDoneProgressCreateParams, WorkDoneProgressEnd,
    WorkDoneProgressReport, WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use darkmatter::markdown::compose::RequestSnapshot;
use thiserror::Error;

use crate::capabilities::{ClientProfile, negotiate_position_encoding, server_capabilities};
use crate::clock::{Clock, SystemClock};
use crate::config::{ConfigSeverity, ConfigState};
use crate::context::{DocumentResolution, RepositoryContexts};
use crate::diagnostics::DiagnosticsScheduler;
use crate::full_index::{Build, FullIndexState, Trigger, Waiter};
use crate::graph::{FullBuildStep, WorkspaceGraph, WorkspaceIndex};
use crate::overlay::{OverlayState, RegistrySummary};
use crate::providers::{DocumentContext, ProviderRegistry};
use crate::request_timing::{
    CompletionEvent, MessageKind, Outcome, RequestTimings, Stage, Wait, document_uri,
};
use crate::workspace::snapshot::SharedSnapshot;
use crate::graph::Listing;
use crate::workspace::discover::{DiscoveryReport, ListedKind, listed_kind};
use crate::workspace::startup::{
    FullIndexRound, ProgressReporter, list_files, parse_full_index_round,
};
use crate::workspace::watch::{
    WatchMode, changed_manifests, coalesce_changes, is_config_file, is_context_input,
    is_ignore_file, scan_manifests, schema_watch_registration, watch_registration,
};
use crate::workspace::{DocumentStore, uri_to_file_path, workspace_identifier, workspace_roots};

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
#[derive(Debug)]
pub struct RunOptions {
    /// Explicit `--config` path (wins over `.dmls.toml` discovery).
    pub config_path: Option<PathBuf>,
    /// `HOME`, the environment, and extra `@` roots every file-resolution
    /// context is built from, fixed for the server's lifetime. The binary
    /// takes it from the process once; tests pass a fixture snapshot.
    pub snapshot: RequestSnapshot,
    /// The monotonic clock request timing reads (the slow-request log).
    pub clock: Arc<dyn Clock>,
    /// Test seams over startup's background work.
    pub startup: StartupHooks,
}

impl RunOptions {
    /// Options with no `--config` override, timed by the system clock.
    pub fn new(snapshot: RequestSnapshot) -> Self {
        Self {
            config_path: None,
            snapshot,
            clock: Arc::new(SystemClock),
            startup: StartupHooks::default(),
        }
    }

    /// Replaces the clock request timing reads.
    pub fn with_clock(mut self, clock: Arc<dyn Clock>) -> Self {
        self.clock = clock;
        self
    }

    /// Replaces the startup test seams.
    pub fn with_startup_hooks(mut self, hooks: StartupHooks) -> Self {
        self.startup = hooks;
        self
    }
}

/// Test seams over the server's background startup work. The binary uses
/// the default: no gate, no observer.
#[derive(Debug, Default)]
pub struct StartupHooks {
    /// When set, the workspace file-list walk starts only after this
    /// receives a message or disconnects, so a test can observe the server
    /// before the list is ready.
    pub file_list_gate: Option<Receiver<()>>,
    /// When set, every full-index build round waits, after reading its files
    /// and before handing them to the dispatch thread, until this receives a
    /// message or disconnects, so a test can change the workspace while the
    /// round's reading goes stale.
    pub full_index_gate: Option<Receiver<()>>,
    /// Receives a [`StartupEvent`] as each background step completes.
    pub observer: Option<Sender<StartupEvent>>,
}

/// A background step that completed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupEvent {
    /// The workspace folders' trigger registries are prepared and the
    /// registry line is logged. Precedes the file-list walk.
    RegistryReady,
    /// The workspace file list is installed and open documents' diagnostics
    /// were refreshed against it.
    FileListInstalled,
    /// A full-index build round finished reading its files (sent from the
    /// worker, before the [`StartupHooks::full_index_gate`] wait).
    FullIndexRoundRead,
}

/// A result background work hands to the dispatch thread, which owns every
/// index mutation.
enum Internal {
    /// Startup prepared the workspace folders' trigger registries.
    RegistryReady {
        summary: RegistrySummary,
        elapsed_ms: u128,
    },
    /// A full-index build round finished reading.
    FullIndexRound {
        build: u64,
        round: FullIndexRound,
    },
    /// The startup file-list walk finished. The dispatch thread ends the
    /// progress once the list is installed, so a client that sees the end
    /// sees diagnostics computed against the list.
    FileList {
        report: DiscoveryReport,
        elapsed_ms: u128,
        progress: LspProgress,
    },
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
    show_config_diagnostics(&connection.sender, &config);

    let wiki_roots = crate::workspace::resolve_wiki_roots(
        &roots,
        config.effective().wiki.wiki_root.as_deref(),
    );
    let contexts = Arc::new(RepositoryContexts::new(options.snapshot));
    let index = Arc::new(Mutex::new(WorkspaceIndex::new(
        Arc::clone(&contexts) as Arc<dyn crate::graph::DocumentContexts>
    )));
    let snapshot = {
        let mut guard = index.lock().expect("fresh index lock");
        guard.set_wiki_roots(wiki_roots);
        SharedSnapshot::new(guard.snapshot())
    };

    let watch_mode = WatchMode::for_profile(&profile);
    // The rescan fallback learns of a manifest change by comparing scans, so
    // it needs a baseline from startup.
    let manifests = match watch_mode {
        WatchMode::ServerRescan => scan_manifests(&roots),
        WatchMode::ClientWatched => HashMap::new(),
    };
    let overlay = Arc::new(OverlayState::default());
    let (internal_tx, internal_rx) = crossbeam_channel::unbounded();
    spawn_startup(StartupWork {
        internal: internal_tx.clone(),
        sender: connection.sender.clone(),
        roots: roots.clone(),
        workspace_config: config.effective().workspace.clone(),
        progress_enabled: profile.supports_work_done_progress,
        gate: options.startup.file_list_gate,
        overlay: Arc::clone(&overlay),
        contexts: Arc::clone(&contexts),
        clock: Arc::clone(&options.clock),
    });

    let diagnostics = DiagnosticsScheduler::new(connection.sender.clone(), config.effective());
    let workspace = workspace_identifier(&params, &roots);
    let state = ServerState {
        documents: DocumentStore::new(encoding),
        config,
        profile,
        roots,
        index,
        snapshot,
        registry: ProviderRegistry::with_substrate(),
        diagnostics,
        overlay,
        watch_mode,
        contexts,
        manifests,
        schema_watch_fallback: false,
        timings: RequestTimings::start(Arc::clone(&options.clock)),
        clock: options.clock,
        workspace,
    };
    let mut router = Router::new(connection, state).with_internal(
        (internal_tx, internal_rx),
        options.startup.observer,
        options.startup.full_index_gate,
    );
    router.register_workspace_watchers(false);
    router.run()
}

/// Sends a `client/registerCapability` for one watcher registration under
/// `request_id`, replacing an earlier registration with the same id first
/// (`client/unregisterCapability`) when `replace` is set. Fire-and-forget:
/// the router reads the client's answer only to detect a rejection.
fn send_watch_registration(
    sender: &Sender<Message>,
    registration: lsp_types::Registration,
    request_id: &str,
    replace: bool,
) {
    if replace {
        let params = lsp_types::UnregistrationParams {
            unregisterations: vec![lsp_types::Unregistration {
                id: registration.id.clone(),
                method: registration.method.clone(),
            }],
        };
        if let Ok(value) = serde_json::to_value(params) {
            let request = Request::new(
                RequestId::from(format!("{request_id}/unregister")),
                lsp_types::request::UnregisterCapability::METHOD.to_string(),
                value,
            );
            let _ = sender.send(Message::Request(request));
        }
    }
    let params = RegistrationParams {
        registrations: vec![registration],
    };
    let Ok(value) = serde_json::to_value(params) else {
        return;
    };
    let request = Request::new(
        RequestId::from(request_id.to_string()),
        RegisterCapability::METHOD.to_string(),
        value,
    );
    let _ = sender.send(Message::Request(request));
}

/// The watcher registrations DMLS has sent, so a client rejection can switch
/// the affected inputs to the save-driven fallback.
#[derive(Debug, Default)]
struct WatchLedger {
    /// Outstanding workspace-watcher registration request ids.
    workspace: HashSet<RequestId>,
    /// Outstanding schema-location registration request ids.
    schema: HashSet<RequestId>,
    /// The folders the last schema-location registration watches.
    schema_folders: std::collections::BTreeSet<PathBuf>,
    /// Whether a schema-location registration was ever sent.
    schema_registered: bool,
    next: u64,
}

impl WatchLedger {
    fn issue(&mut self, prefix: &str) -> String {
        self.next += 1;
        format!("dmls/{prefix}/{}", self.next)
    }
}

/// What the background startup thread needs.
struct StartupWork {
    internal: Sender<Internal>,
    sender: Sender<Message>,
    roots: Vec<PathBuf>,
    workspace_config: crate::config::WorkspaceConfig,
    progress_enabled: bool,
    gate: Option<Receiver<()>>,
    overlay: Arc<OverlayState>,
    contexts: Arc<RepositoryContexts>,
    clock: Arc<dyn Clock>,
}

/// Spawns startup's background work, in order: the workspace folders'
/// trigger registries, then the file-list walk, which reads no file contents
/// and reports `window/workDoneProgress`. Each result goes to the dispatch
/// thread, which logs and installs it.
fn spawn_startup(work: StartupWork) {
    std::thread::spawn(move || {
        let started = work.clock.now();
        work.overlay.prepare_workspace_registries(&work.roots, &work.contexts);
        let summary = work.overlay.trigger_registry_summary();
        let elapsed_ms = work.clock.now().saturating_duration_since(started).as_millis();
        let _ = work.internal.send(Internal::RegistryReady { summary, elapsed_ms });

        if let Some(gate) = work.gate {
            let _ = gate.recv();
        }
        let reporter = LspProgress::new(work.sender, work.progress_enabled, LspProgress::STARTUP_TOKEN);
        reporter.begin("Listing workspace files");
        let started = work.clock.now();
        let report = list_files(&work.roots, &work.workspace_config);
        let elapsed_ms = work.clock.now().saturating_duration_since(started).as_millis();
        let _ = work.internal.send(Internal::FileList { report, elapsed_ms, progress: reporter });
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
    pub overlay: Arc<OverlayState>,
    /// How this client learns about on-disk changes. `ServerRescan` drives a
    /// save-triggered rescan for unopened files; `ClientWatched` relies on
    /// `didChangeWatchedFiles`.
    pub watch_mode: WatchMode,
    /// One file-resolution context per repository, shared with the graph.
    pub contexts: Arc<RepositoryContexts>,
    /// Package-manifest hashes from the last scan, for [`WatchMode::ServerRescan`].
    manifests: HashMap<PathBuf, u64>,
    /// Schema inputs outside the workspace globs have no watcher (the client
    /// cannot root one at a folder, or rejected it), so every save marks the
    /// trigger registries stale.
    schema_watch_fallback: bool,
    /// The clock every message's [`RequestTimings`] reads.
    clock: Arc<dyn Clock>,
    /// The message being dispatched; the router replaces it at each dequeue.
    timings: RequestTimings,
    /// The workspace identifier every workspace log line carries.
    workspace: String,
}

impl ServerState {
    /// Re-indexes an open buffer into the workspace index and publishes the
    /// resulting snapshot. A URI without a filesystem path is ignored.
    fn index_buffer(&self, uri: &lsp_types::Uri, text: &str) {
        let Some(path) = uri_to_file_path(uri) else {
            return;
        };
        let mut index = self.index.lock().expect("index lock poisoned");
        index.open_buffer(&path, text);
        self.snapshot.store(index.snapshot());
    }

    /// Restores the disk view of a closed buffer in the workspace index.
    fn close_buffer(&self, uri: &Uri) {
        let Some(path) = uri_to_file_path(uri) else {
            return;
        };
        let mut index = self.index.lock().expect("index lock poisoned");
        index.close_buffer(&path);
        self.snapshot.store(index.snapshot());
    }

    /// Installs a file-list walk's result in the workspace index.
    ///
    /// ## Returns
    ///
    /// The Markdown paths that joined or left the list.
    fn install_listing(&self, report: DiscoveryReport) -> Vec<PathBuf> {
        let complete = report.is_complete();
        let markdown = report.files.into_iter().collect();
        let yaml = report.yaml.into_iter().collect();
        let mut index = self.index.lock().expect("index lock poisoned");
        let changed = index.install_listing(markdown, yaml, complete);
        self.snapshot.store(index.snapshot());
        changed
    }

    /// Lists the workspace again on the dispatch thread (a rescan, or a
    /// discovery configuration or ignore-file change) and installs the list.
    ///
    /// ## Returns
    ///
    /// The Markdown paths that joined or left the list.
    fn relist(&self) -> Vec<PathBuf> {
        let started = std::time::Instant::now();
        let report = list_files(&self.roots, &self.config.effective().workspace);
        tracing::info!(
            markdown = report.files.len(),
            yaml = report.yaml.len(),
            duplicates = report.duplicates.len(),
            elapsed_ms = started.elapsed().as_millis(),
            workspace = %self.workspace,
            "workspace rescan complete"
        );
        self.install_listing(report)
    }

    /// Parses what the open document at `uri` needs from the documents it
    /// references (see [`WorkspaceIndex::prepare_targets`]), publishing a new
    /// snapshot when anything was read.
    fn prepare_document(&self, uri: &Uri) {
        let Some(path) = uri_to_file_path(uri) else {
            return;
        };
        let mut index = self.index.lock().expect("index lock poisoned");
        if index.prepare_targets(&path) {
            self.snapshot.store(index.snapshot());
        }
    }

    /// Parses one referenced document whose headings a request needs (a
    /// hover preview, a definition, a heading completion), unless it is
    /// already parsed and current.
    fn prepare_target(&self, target: Option<PathBuf>) {
        let Some(target) = target else {
            return;
        };
        let mut index = self.index.lock().expect("index lock poisoned");
        if !index.ensure_parsed(&[target]).is_empty() {
            self.snapshot.store(index.snapshot());
        }
    }

    /// The document a request at `position` in `uri` points into: the
    /// target of the link, wiki link, or transclusion under the cursor, or
    /// the document an anchor completion there names.
    fn target_at(&self, uri: &Uri, position: lsp_types::Position) -> Option<PathBuf> {
        let path = uri_to_file_path(uri)?;
        let offset = self.documents.get(uri)?.source_map().lsp_to_byte(position)?;
        let snapshot = self.snapshot.load();
        if let Some(target) = crate::providers::reference_target_at(&snapshot, &path, offset) {
            return Some(target);
        }
        self.with_document(uri, |ctx| crate::providers::completion_heading_target(ctx, offset))
            .flatten()
    }

    /// Whether the full index is built (see [`crate::full_index`]).
    fn index_is_full(&self) -> bool {
        self.index.lock().expect("index lock poisoned").is_full()
    }

    /// Builds a [`DocumentContext`] for an open document and runs `f` against
    /// it. Returns `None` when the document is not open, or is neither a
    /// `file:` nor an `untitled:` document.
    fn with_document<R>(&self, uri: &Uri, f: impl FnOnce(&DocumentContext) -> R) -> Option<R> {
        let open = self.documents.get(uri)?;
        let (path, resolution) = self.document_resolution(uri)?;
        let snapshot = self.snapshot.load();
        let doc_id = snapshot.document_id(&path);
        let overlay = self.timings.time(Stage::OverlayBuild, || {
            self.overlay.for_document_timed(
                uri,
                open.text(),
                &path,
                self.config.effective(),
                &self.roots,
                &resolution,
                &self.timings,
            )
        });
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
            resolution: &resolution,
        };
        Some(self.timings.time(Stage::ProviderChain, || f(&ctx)))
    }

    /// The path a document is analyzed at and its file-resolution context.
    ///
    /// A `file:` document uses its repository's context. An `untitled:`
    /// buffer borrows the context of the single repository the workspace
    /// folders lie in, and is analyzed as if it sat at that repository's
    /// root; with no single repository it gets the context failure.
    fn document_resolution(&self, uri: &Uri) -> Option<(PathBuf, DocumentResolution)> {
        if let Some(path) = uri_to_file_path(uri) {
            let resolution = self.contexts.for_document(&path);
            return Some((path, resolution));
        }
        let name = untitled_name(uri)?;
        let resolution = self.contexts.for_untitled(&self.roots);
        let path = match resolution.context() {
            Ok(context) => context.request_cwd().join(&name),
            Err(_) => PathBuf::from(&name),
        };
        Some((path, resolution))
    }

    /// Drops the cached contexts `changed` can affect and, when any dropped,
    /// rebuilds the graph so every path reference re-resolves.
    ///
    /// ## Returns
    ///
    /// `true` when a context was dropped.
    fn invalidate_contexts<'p>(&self, changed: impl IntoIterator<Item = &'p Path>) -> bool {
        let mut dropped = false;
        for path in changed {
            dropped |= self.contexts.invalidate(path);
        }
        if dropped {
            self.relink();
        }
        dropped
    }

    /// Rebuilds the graph against the current contexts.
    fn relink(&self) {
        let mut index = self.index.lock().expect("index lock poisoned");
        index.relink();
        self.snapshot.store(index.snapshot());
    }

    /// Recomputes and publishes diagnostics for one open document (empty when
    /// it is unknown, which clears the client's markers).
    ///
    /// The document's referenced targets are parsed first, as far as its
    /// diagnostics need them. Until the file list is ready, findings that
    /// depend on which paths exist are withheld (see [`listing_gates`]); the
    /// list's arrival refreshes every open document.
    fn refresh_diagnostics(&self, uri: &Uri) {
        let version = self.documents.get(uri).map(|doc| doc.version());
        if self.documents.is_open(uri) {
            self.prepare_document(uri);
        }
        let listing = self.snapshot.load().listing();
        let diagnostics = self
            .with_document(uri, |ctx| self.registry.diagnostics(ctx))
            .unwrap_or_default()
            .into_iter()
            .filter(|diagnostic| !listing_gates(listing, diagnostic))
            .collect();
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

    /// Rescans the workspace from disk and reconciles the index for
    /// [`WatchMode::ServerRescan`] clients (no reliable file watcher).
    ///
    /// This is the save-driven fallback that keeps unopened files current for
    /// clients like Neovim on Linux: it lists the workspace again (adopting
    /// created and dropping deleted files), then re-reads and re-hashes the
    /// closed documents already parsed, re-parsing only those whose bytes
    /// changed. Unparsed files stay unparsed. Open buffers stay
    /// authoritative. It reads disk but never composes, executes, or fetches.
    ///
    /// The scan also re-hashes every package manifest, and a document or
    /// manifest it finds changed drops the cached contexts above it, as a
    /// watched event would. Every trigger registry is marked stale, since the
    /// scan cannot tell which schema-root files changed.
    ///
    /// ## Returns
    ///
    /// The changed document and manifest paths.
    fn rescan_workspace(&mut self) -> Vec<PathBuf> {
        let mut changed = self.relist();
        {
            let mut index = self.index.lock().expect("index lock poisoned");
            changed.extend(index.recheck_parsed());
            self.snapshot.store(index.snapshot());
        }
        let manifests = scan_manifests(&self.roots);
        changed.extend(changed_manifests(&self.manifests, &manifests));
        self.manifests = manifests;
        self.invalidate_contexts(changed.iter().map(PathBuf::as_path));
        // The rescan cannot say which schema-root files changed.
        self.overlay.invalidate_all_trigger_registries();
        changed
    }

    /// Reloads configuration after a `workspace/didChangeConfiguration` and
    /// re-derives every part of the analysis universe that depends on it, so a
    /// settings change takes effect without a server restart.
    ///
    /// Beyond storing the new settings, this recomputes the wiki resolution
    /// roots (`wiki.wiki_root`) and rebuilds the graph against them, lists
    /// the workspace again against the new discovery globs
    /// (`workspace.include`/`exclude`, without reading files), and
    /// re-publishes diagnostics for every
    /// open document. Schema-extension activation (`schema.extensions`) flows
    /// through the overlay's content-and-config-keyed schema cache, so it
    /// re-assembles on the diagnostics refresh by construction. Every cached
    /// file-resolution context is dropped and every trigger registry marked
    /// stale, even when the effective config is unchanged, and both are
    /// rebuilt on the next request. Nothing else runs when the
    /// effective config is unchanged, and the re-listing runs only when the
    /// `workspace` section actually changed.
    ///
    /// ## Returns
    ///
    /// `true` when a token-affecting change occurred and the client honors
    /// `workspace/semanticTokens/refresh`, so the caller (which owns the
    /// connection) sends the refresh. The new config already applies to every
    /// later request regardless of this signal.
    fn reload_config(&mut self, settings: serde_json::Value) -> bool {
        self.update_config(|config| config.apply_client_settings(settings))
    }

    /// [`reload_config`](Self::reload_config) for a change to the config
    /// file on disk (a watched `.dmls.toml`).
    fn reload_config_file(&mut self) -> bool {
        self.update_config(ConfigState::reload)
    }

    /// Applies `update` to the configuration and re-derives what depends on
    /// it; see [`reload_config`](Self::reload_config).
    fn update_config(&mut self, update: impl FnOnce(&mut ConfigState)) -> bool {
        let contexts_dropped = self.contexts.clear();
        self.overlay.invalidate_all_trigger_registries();
        let before = self.config.effective().clone();
        update(&mut self.config);
        let after = self.config.effective();
        if *after == before {
            if contexts_dropped {
                self.relink();
                self.refresh_all_diagnostics();
            }
            return false;
        }
        let refresh_semantic_tokens =
            should_request_semantic_tokens_refresh(&self.profile, &before, after);

        let wiki_roots = (after.wiki.wiki_root != before.wiki.wiki_root).then(|| {
            crate::workspace::resolve_wiki_roots(&self.roots, after.wiki.wiki_root.as_deref())
        });
        let workspace_changed = after.workspace != before.workspace;

        // Each of these rebuilds the graph, which also re-resolves every
        // reference against the fresh contexts.
        let mut rebuilt = false;
        if workspace_changed {
            self.relist();
            rebuilt = true;
        }
        {
            let mut index = self.index.lock().expect("index lock poisoned");
            if let Some(wiki_roots) = wiki_roots {
                index.set_wiki_roots(wiki_roots);
                rebuilt = true;
            }
            if contexts_dropped && !rebuilt {
                index.relink();
                rebuilt = true;
            }
            if rebuilt {
                self.snapshot.store(index.snapshot());
            }
        }
        self.refresh_all_diagnostics();
        refresh_semantic_tokens
    }

    /// Applies a coalesced on-disk change to the file list and index.
    ///
    /// A Markdown file the walk would list joins the list; a parsed closed
    /// copy is marked stale rather than re-read (the full index, once built,
    /// re-reads it at once). A deletion drops it. YAML files only update the
    /// list. Paths the walk would not list are ignored.
    fn apply_disk_change(&self, path: &Path, kind: FileChangeType) {
        let mut index = self.index.lock().expect("index lock poisoned");
        let changed = if kind == FileChangeType::DELETED {
            index.yaml_changed(path, false);
            index.disk_deleted(path)
        } else {
            match listed_kind(&self.roots, &self.config.effective().workspace, path) {
                Some(ListedKind::Markdown) => index.disk_changed(path),
                Some(ListedKind::Yaml) => {
                    index.yaml_changed(path, true);
                    false
                }
                None => false,
            }
        };
        if changed {
            self.snapshot.store(index.snapshot());
        }
    }
}

/// Logs the startup registry line: `schema registry ready`, or
/// `schema registry initialized with failures` when a prepared registry
/// failed.
fn log_registry_ready(summary: &RegistrySummary, elapsed_ms: u128, workspace: &str) {
    macro_rules! registry_line {
        ($message:literal) => {
            tracing::info!(
                roots = summary.roots,
                tree_schemas = summary.tree_schemas,
                scoped_schemas = summary.scoped_schemas,
                trigger_schemas = summary.trigger_schemas,
                unclassified = summary.unclassified,
                failed_triggers = summary.failed_triggers,
                failed_scopes = summary.failed_scopes,
                elapsed_ms,
                workspace = %workspace,
                $message
            )
        };
    }
    if summary.failed_scopes > 0 {
        registry_line!("schema registry initialized with failures");
    } else {
        registry_line!("schema registry ready");
    }
}

/// Whether a diagnostic is withheld under the file list's `listing` state.
///
/// Until the list is ready, every finding that depends on which paths exist
/// waits: broken and missing-anchor links, broken transclusions, and wiki
/// links that do not resolve to one file. An incomplete list keeps
/// withholding the wiki findings, because wiki resolution is decided by the
/// list alone; the others are decided by disk.
fn listing_gates(listing: Listing, diagnostic: &lsp_types::Diagnostic) -> bool {
    use crate::diagnostics::codes::code;
    let Some(NumberOrString::String(code)) = &diagnostic.code else {
        return false;
    };
    let wiki = [code::WIKI_UNRESOLVED_TARGET, code::WIKI_AMBIGUOUS_TARGET, code::WIKI_HEADING_MISSING];
    match listing {
        Listing::Pending => [
            code::BROKEN_PATH,
            code::MISSING_ANCHOR,
            code::TRANSCLUSION_BROKEN_PATH,
            code::TRANSCLUSION_CYCLE,
        ]
        .iter()
        .chain(&wiki)
        .any(|gated| gated == code),
        Listing::Ready { complete: false } => wiki.contains(&code.as_str()),
        Listing::Ready { complete: true } => false,
    }
}

/// A [`ProgressReporter`] that drives `window/workDoneProgress` over the LSP
/// connection, throttled to 5% steps so a large workspace does not flood the
/// client with notifications.
struct LspProgress {
    sender: Sender<Message>,
    enabled: bool,
    token: &'static str,
    last_step: AtomicUsize,
}

impl LspProgress {
    /// The startup file-list walk.
    const STARTUP_TOKEN: &'static str = "dmls/startup-index";
    /// A full-index build, across all of its rounds.
    const FULL_INDEX_TOKEN: &'static str = "dmls/full-index";

    fn new(sender: Sender<Message>, enabled: bool, token: &'static str) -> Self {
        Self {
            sender,
            enabled,
            token,
            last_step: AtomicUsize::new(usize::MAX),
        }
    }

    fn send_progress(&self, value: WorkDoneProgress) {
        let params = ProgressParams {
            token: NumberOrString::String(self.token.to_string()),
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
            token: NumberOrString::String(self.token.to_string()),
        };
        if let Ok(value) = serde_json::to_value(create) {
            let request = Request::new(
                RequestId::from(format!("{}/create", self.token)),
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

/// Bounded `$/cancelRequest` bookkeeping for requests not yet dequeued.
///
/// Dispatch is synchronous, so a cancellation can beat its request only when
/// the client sends it first; the ledger lets such a request be answered
/// `RequestCanceled` without running. A request waiting for the full index
/// is cancelled directly instead (see [`crate::full_index`]).
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

    fn contains(&self, id: &RequestId) -> bool {
        self.ids.contains(id)
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
    /// Results from background work, handled between client messages.
    internal: Receiver<Internal>,
    /// Where full-index build rounds send their results.
    internal_tx: Option<Sender<Internal>>,
    /// The [`StartupHooks::observer`].
    observer: Option<Sender<StartupEvent>>,
    /// The [`StartupHooks::full_index_gate`].
    full_index_gate: Option<Receiver<()>>,
    watches: WatchLedger,
    full_index: FullIndexState,
    /// The id the next full-index build gets.
    next_build: u64,
}

impl Router {
    /// Builds a router over an initialized connection.
    pub fn new(connection: Connection, state: ServerState) -> Self {
        Self {
            connection,
            state,
            cancelled: CancelLedger::default(),
            refresh: RefreshLedger::default(),
            internal: crossbeam_channel::never(),
            internal_tx: None,
            observer: None,
            full_index_gate: None,
            watches: WatchLedger::default(),
            full_index: FullIndexState::NotStarted,
            next_build: 0,
        }
    }

    fn with_internal(
        mut self,
        (internal_tx, internal): (Sender<Internal>, Receiver<Internal>),
        observer: Option<Sender<StartupEvent>>,
        full_index_gate: Option<Receiver<()>>,
    ) -> Self {
        self.internal = internal;
        self.internal_tx = Some(internal_tx);
        self.observer = observer;
        self.full_index_gate = full_index_gate;
        self
    }

    /// Runs until `shutdown`/`exit` (Ok) or a protocol-fatal error.
    pub fn run(mut self) -> Result<(), ServerError> {
        loop {
            let message = crossbeam_channel::select! {
                recv(self.connection.receiver) -> message => {
                    message.map_err(|_| ServerError::ChannelClosed)?
                }
                recv(self.internal) -> event => {
                    match event {
                        Ok(event) => self.handle_internal(event),
                        // The background worker is gone; stop selecting on it.
                        Err(_) => self.internal = crossbeam_channel::never(),
                    }
                    continue;
                }
            };
            match message {
                Message::Request(request) => {
                    if request.method == lsp_types::request::Shutdown::METHOD {
                        self.abandon_full_index();
                    }
                    if self.connection.handle_shutdown(&request)? {
                        return Ok(());
                    }
                    self.state.timings = RequestTimings::start(Arc::clone(&self.state.clock));
                    self.handle_request(request);
                }
                Message::Notification(notification) => {
                    if notification.method == "exit" {
                        return Err(ServerError::ExitWithoutShutdown);
                    }
                    self.state.timings = RequestTimings::start(Arc::clone(&self.state.clock));
                    self.handle_notification(notification);
                }
                Message::Response(response) => {
                    // Retire an outstanding refresh (logging a client rejection);
                    // an untracked id keeps the original ignore behavior.
                    if !self.refresh.consume(&response) && !self.consume_watch_response(&response) {
                        tracing::debug!(id = %response.id, "ignoring client response");
                    }
                }
            }
            self.sync_schema_watches();
        }
    }

    /// Registers the workspace watchers for a [`WatchMode::ClientWatched`]
    /// client (`replace` after a discovery-configuration change).
    ///
    /// A [`WatchMode::ServerRescan`] client (no reliable watcher — e.g.
    /// Neovim on Linux) is served by [`ServerState::rescan_workspace`] on save
    /// instead, so it needs no registration.
    fn register_workspace_watchers(&mut self, replace: bool) {
        if self.state.watch_mode != WatchMode::ClientWatched {
            return;
        }
        let Some(registration) = watch_registration(&self.state.config.effective().workspace) else {
            return;
        };
        let id = self.watches.issue("register-watchers");
        self.watches.workspace.insert(RequestId::from(id.clone()));
        send_watch_registration(&self.connection.sender, registration, &id, replace);
    }

    /// Keeps the schema-location watches equal to the folders the
    /// prepared trigger registries read, re-registering when that set
    /// changes. A client without `relativePatternSupport` gets none and is
    /// served by the save-driven fallback.
    fn sync_schema_watches(&mut self) {
        if self.state.watch_mode != WatchMode::ClientWatched || self.state.schema_watch_fallback {
            return;
        }
        if !self.state.profile.supports_relative_watch_patterns {
            tracing::debug!("client cannot root watchers at a folder; schema inputs use the save fallback");
            self.state.schema_watch_fallback = true;
            return;
        }
        let folders = self.state.overlay.trigger_watch_folders();
        if folders == self.watches.schema_folders {
            return;
        }
        let Some(registration) =
            schema_watch_registration(folders.iter().map(PathBuf::as_path))
        else {
            return;
        };
        let replace = self.watches.schema_registered;
        let id = self.watches.issue("register-schema-watchers");
        self.watches.schema.insert(RequestId::from(id.clone()));
        self.watches.schema_registered = true;
        self.watches.schema_folders = folders;
        send_watch_registration(&self.connection.sender, registration, &id, replace);
    }

    /// Retires a watcher-registration response. A rejected workspace
    /// registration switches the server to [`WatchMode::ServerRescan`]; a
    /// rejected schema-location registration switches schema inputs to the
    /// save-driven fallback.
    ///
    /// ## Returns
    ///
    /// Whether the response answered a watcher registration.
    fn consume_watch_response(&mut self, response: &Response) -> bool {
        let rejected = response.error.as_ref().map(|error| error.message.clone());
        if self.watches.workspace.remove(&response.id) {
            if let Some(message) = rejected {
                tracing::warn!(%message, "client rejected file watchers; rescanning on save instead");
                self.state.watch_mode = WatchMode::ServerRescan;
                self.state.manifests = scan_manifests(&self.state.roots);
            }
            return true;
        }
        if self.watches.schema.remove(&response.id) {
            if let Some(message) = rejected {
                tracing::warn!(%message, "client rejected schema-location watchers; rescanning schemas on save instead");
                self.state.schema_watch_fallback = true;
            }
            return true;
        }
        // An unregistration's answer carries no state.
        response.id.to_string().contains("/unregister")
    }

    /// Installs a background result. Runs on the dispatch thread, between
    /// client messages, like any notification.
    fn handle_internal(&mut self, event: Internal) {
        self.state.timings = RequestTimings::start(Arc::clone(&self.state.clock));
        if catch_unwind(AssertUnwindSafe(|| self.install_internal(event))).is_err() {
            tracing::error!("background result handler panicked");
        }
        self.sync_schema_watches();
    }

    fn install_internal(&mut self, event: Internal) {
        match event {
            Internal::RegistryReady { summary, elapsed_ms } => {
                log_registry_ready(&summary, elapsed_ms, &self.state.workspace);
                // Preparation may have found a failing trigger file, whose
                // diagnostic this publishes.
                self.state.refresh_all_diagnostics();
                self.notify_observer(StartupEvent::RegistryReady);
            }
            Internal::FileList { report, elapsed_ms, progress } => {
                tracing::info!(
                    markdown = report.files.len(),
                    yaml = report.yaml.len(),
                    duplicates = report.duplicates.len(),
                    elapsed_ms,
                    workspace = %self.state.workspace,
                    "workspace files listed"
                );
                self.state.install_listing(report);
                self.state.refresh_all_diagnostics();
                progress.end();
                self.notify_observer(StartupEvent::FileListInstalled);
                if matches!(&self.full_index, FullIndexState::Building(build) if build.awaiting_list) {
                    self.start_full_index_round();
                }
            }
            Internal::FullIndexRound { build, round } => self.finish_full_index_round(build, round),
        }
    }

    fn notify_observer(&self, event: StartupEvent) {
        if let Some(observer) = &self.observer {
            let _ = observer.send(event);
        }
    }

    /// Holds a request that needs the full index until the build finishes,
    /// starting a build when none is running.
    fn wait_for_full_index(&mut self, request: Request, trigger: Trigger) {
        let timings = std::mem::replace(
            &mut self.state.timings,
            RequestTimings::start(Arc::clone(&self.state.clock)),
        );
        timings.record_wait(Wait::FullIndex);
        let waiter = Waiter { request, timings };
        if let FullIndexState::Building(build) = &mut self.full_index {
            if build.awaiting_list {
                waiter.timings.record_wait(Wait::FileList);
            }
            build.waiters.push(waiter);
            return;
        }
        self.next_build += 1;
        let progress = Arc::new(LspProgress::new(
            self.connection.sender.clone(),
            self.state.profile.supports_work_done_progress,
            LspProgress::FULL_INDEX_TOKEN,
        ));
        progress.begin("Building the full index");
        let awaiting_list = self.state.snapshot.load().listing() == Listing::Pending;
        if awaiting_list {
            waiter.timings.record_wait(Wait::FileList);
        }
        tracing::debug!(trigger = %trigger.as_str(), awaiting_list, "full index build started");
        self.full_index = FullIndexState::Building(Build {
            id: self.next_build,
            trigger,
            started: self.state.clock.now(),
            cancel: Arc::new(AtomicBool::new(false)),
            progress,
            waiters: vec![waiter],
            awaiting_list,
        });
        if !awaiting_list {
            self.start_full_index_round();
        }
    }

    /// Starts the building build's next round on a background thread, which
    /// reads the unparsed listed files with the index lock released.
    fn start_full_index_round(&mut self) {
        let FullIndexState::Building(build) = &mut self.full_index else {
            return;
        };
        build.awaiting_list = false;
        let files = self.state.index.lock().expect("index lock poisoned").begin_full_build();
        let Some(internal) = self.internal_tx.clone() else {
            return;
        };
        let id = build.id;
        let cancel = Arc::clone(&build.cancel);
        let progress = Arc::clone(&build.progress);
        let observer = self.observer.clone();
        let gate = self.full_index_gate.clone();
        std::thread::spawn(move || {
            let round = parse_full_index_round(&files, progress.as_ref(), &cancel);
            if let Some(observer) = observer {
                let _ = observer.send(StartupEvent::FullIndexRoundRead);
            }
            if let Some(gate) = gate {
                let _ = gate.recv();
            }
            let _ = internal.send(Internal::FullIndexRound { build: id, round });
        });
    }

    /// Reconciles a round's results and moves the build on: another round,
    /// ready (answering every waiter), or failed (answering every waiter
    /// with the error). A round from a cancelled build is discarded.
    fn finish_full_index_round(&mut self, id: u64, round: FullIndexRound) {
        let current = matches!(&self.full_index, FullIndexState::Building(build) if build.id == id);
        if !current || round.cancelled {
            return;
        }
        let step = {
            let mut index = self.state.index.lock().expect("index lock poisoned");
            let step = index.finish_full_build(round.parsed, round.failed);
            self.state.snapshot.store(index.snapshot());
            step
        };
        let failure = match step {
            FullBuildStep::More(_) => {
                self.start_full_index_round();
                return;
            }
            FullBuildStep::Ready => None,
            FullBuildStep::Failed(failed) => Some(failed),
        };
        let FullIndexState::Building(mut build) = std::mem::take(&mut self.full_index) else {
            return;
        };
        build.progress.end();
        let waiters = std::mem::take(&mut build.waiters);
        match failure {
            None => {
                self.log_full_index_built(&build);
                self.full_index = FullIndexState::Ready;
                // Every document's headings are known now, so anchor checks
                // that waited on an unparsed target can run. Published before
                // the answers, so a client that treats an answer as a barrier
                // never receives an older publication after it.
                self.state.refresh_all_diagnostics();
                for waiter in waiters {
                    self.state.timings = waiter.timings;
                    self.answer_request(waiter.request);
                }
            }
            Some(failed) => {
                let (first, error) = &failed[0];
                tracing::warn!(
                    unreadable = failed.len(),
                    first = %first.display(),
                    %error,
                    trigger = %build.trigger.as_str(),
                    workspace = %self.state.workspace,
                    "full index build failed"
                );
                let message = format!(
                    "dmls could not build the workspace index: {} could not be read ({error}){}",
                    first.display(),
                    match failed.len() {
                        1 => String::new(),
                        more => format!(", and {} more file(s)", more - 1),
                    }
                );
                self.full_index = FullIndexState::Failed;
                for waiter in waiters {
                    self.answer_waiter(waiter, ErrorCode::RequestFailed, &message, Outcome::Error);
                }
            }
        }
    }

    /// Logs `full index built` with the index's document, heading, and link
    /// counts.
    fn log_full_index_built(&self, build: &Build) {
        let (markdown, headings, links) = {
            let index = self.state.index.lock().expect("index lock poisoned");
            let snapshot = index.snapshot();
            let mut headings = 0;
            let mut links = 0;
            for doc in 0..snapshot.document_count() {
                let doc = crate::graph::DocumentId(doc as u32);
                headings += snapshot.headings(doc).count();
                links += snapshot.links(doc).count() + snapshot.wiki_links(doc).count();
            }
            (index.listed().len(), headings, links)
        };
        let elapsed_ms = self.state.clock.now().saturating_duration_since(build.started).as_millis();
        tracing::info!(
            markdown,
            headings,
            links,
            trigger = %build.trigger.as_str(),
            elapsed_ms,
            workspace = %self.state.workspace,
            "full index built"
        );
    }

    /// Answers a waiting request with an error and emits its completion
    /// event.
    fn answer_waiter(&mut self, waiter: Waiter, code: ErrorCode, message: &str, outcome: Outcome) {
        let id = waiter.request.id.clone();
        self.respond(Response::new_err(id.clone(), code as i32, message.to_string()));
        CompletionEvent {
            kind: MessageKind::Request { id: &id },
            method: &waiter.request.method,
            uri: document_uri(&waiter.request.params).as_deref(),
            outcome,
        }
        .emit(&waiter.timings);
    }

    /// Cancels a request waiting for the full index. The build continues for
    /// any other waiter; a build nobody waits for any more is cancelled and
    /// never becomes ready.
    ///
    /// ## Returns
    ///
    /// Whether `id` was waiting.
    fn cancel_full_index_waiter(&mut self, id: &RequestId) -> bool {
        let FullIndexState::Building(build) = &mut self.full_index else {
            return false;
        };
        let Some(position) = build.waiters.iter().position(|waiter| waiter.request.id == *id) else {
            return false;
        };
        let waiter = build.waiters.remove(position);
        if build.waiters.is_empty() {
            build.progress.end();
            tracing::debug!(trigger = %build.trigger.as_str(), "full index build cancelled");
            // Dropping the build stops its workers.
            self.full_index = FullIndexState::NotStarted;
            self.state.index.lock().expect("index lock poisoned").abandon_full_build();
        }
        self.answer_waiter(waiter, ErrorCode::RequestCanceled, "request was cancelled", Outcome::Cancelled);
        true
    }

    /// Stops a running build at shutdown, answering its waiters.
    fn abandon_full_index(&mut self) {
        let FullIndexState::Building(mut build) = std::mem::take(&mut self.full_index) else {
            return;
        };
        build.progress.end();
        self.state.index.lock().expect("index lock poisoned").abandon_full_build();
        for waiter in std::mem::take(&mut build.waiters) {
            self.answer_waiter(waiter, ErrorCode::ServerCancelled, "the server is shutting down", Outcome::Cancelled);
        }
    }

    /// Panic boundary: a panicking handler answers `InternalError` instead
    /// of taking down the request loop.
    ///
    /// Every path ends in exactly one completion event, emitted after the
    /// response is sent so the figure covers the send. A request that needs
    /// the full index while it is not built waits for the build instead (see
    /// [`crate::full_index`]); its event is emitted when it is answered.
    fn handle_request(&mut self, request: Request) {
        if let Some(trigger) = Trigger::for_method(&request.method)
            && !self.cancelled.contains(&request.id)
            && !self.state.index_is_full()
        {
            self.wait_for_full_index(request, trigger);
            return;
        }
        self.answer_request(request);
    }

    /// Dispatches a request and answers it, under the panic boundary; see
    /// [`handle_request`](Self::handle_request).
    fn answer_request(&mut self, request: Request) {
        let id = request.id.clone();
        let method = request.method.clone();
        let uri = document_uri(&request.params);
        let outcome = if self.cancelled.take(&id) {
            self.respond(Response::new_err(
                id.clone(),
                ErrorCode::RequestCanceled as i32,
                "request was cancelled".to_string(),
            ));
            Outcome::Cancelled
        } else {
            match catch_unwind(AssertUnwindSafe(|| self.dispatch_request(request))) {
                Ok(response) => {
                    let outcome = response_outcome(&response);
                    self.respond(response);
                    outcome
                }
                Err(_) => {
                    tracing::error!(id = %id, "request handler panicked");
                    self.respond(Response::new_err(
                        id.clone(),
                        ErrorCode::InternalError as i32,
                        "request handler panicked".to_string(),
                    ));
                    Outcome::Panic
                }
            }
        };
        CompletionEvent {
            kind: MessageKind::Request { id: &id },
            method: &method,
            uri: uri.as_deref(),
            outcome,
        }
        .emit(&self.state.timings);
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
            #[cfg(test)]
            logging_tests::PANIC_METHOD => panic!("test handler panic"),
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
        let symbols = self.state.timings.time(Stage::ProviderChain, || {
            self.state.registry.workspace_symbols(&params.query, snapshot.as_ref())
        });
        Response::new_ok(id, WorkspaceSymbolResponse::Nested(symbols))
    }

    fn definition(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<GotoDefinitionParams>(params) else {
            return invalid_params(id);
        };
        let position = params.text_document_position_params.position;
        let uri = params.text_document_position_params.text_document.uri;
        self.state.prepare_target(self.state.target_at(&uri, position));
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
        self.state.prepare_target(self.state.target_at(&uri, position));
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
        self.state.prepare_target(self.state.target_at(&uri, position));
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

    /// Answers with the provider's edits after showing its warnings (a
    /// declined or impossible format) as `Warning` messages, so the client
    /// sees why nothing changed before the empty edit list arrives.
    fn formatting(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<DocumentFormattingParams>(params) else {
            return invalid_params(id);
        };
        let outcome = self
            .state
            .with_document(&params.text_document.uri, |ctx| {
                crate::providers::formatting::formatting(ctx)
            })
            .unwrap_or_default();
        for warning in outcome.warnings {
            show_message(&self.connection.sender, MessageType::WARNING, warning);
        }
        Response::new_ok(id, outcome.edits)
    }

    fn prepare_rename(&self, id: RequestId, params: serde_json::Value) -> Response {
        let Ok(params) = serde_json::from_value::<TextDocumentPositionParams>(params) else {
            return invalid_params(id);
        };
        let snapshot = self.state.snapshot.load();
        let env = self.state.rename_env(snapshot.as_ref());
        let response = self.state.timings.time(Stage::ProviderChain, || {
            crate::providers::rename::prepare_rename(
                &env,
                &params.text_document.uri,
                params.position,
            )
        });
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
        let renamed = self.state.timings.time(Stage::ProviderChain, || {
            crate::providers::rename::rename(&env, &uri, position, &params.new_name)
        });
        match renamed {
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
        let edit = self.state.timings.time(Stage::ProviderChain, || {
            crate::providers::rename::will_rename_files(&env, &params.files)
        });
        Response::new_ok(id, edit)
    }

    fn handle_notification(&mut self, notification: Notification) {
        let method = notification.method.clone();
        let uri = document_uri(&notification.params);
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            self.dispatch_notification(notification)
        }));
        let outcome = match outcome {
            Ok(()) => Outcome::Ok,
            Err(_) => {
                tracing::error!(method = %method, "notification handler panicked");
                Outcome::Panic
            }
        };
        CompletionEvent {
            kind: MessageKind::Notification,
            method: &method,
            uri: uri.as_deref(),
            outcome,
        }
        .emit(&self.state.timings);
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
                self.state.close_buffer(&uri);
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
                // Trigger registries read schema files from disk, so saving one
                // (open or not) is an input change; unsaved edits are not.
                if let Some(path) = uri_to_file_path(&params.text_document.uri)
                    && matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml" | "yml"))
                {
                    self.state.overlay.invalidate_trigger_inputs(&[path]);
                    if self.state.watch_mode == WatchMode::ClientWatched {
                        self.state.refresh_all_diagnostics();
                    }
                }
                if self.state.watch_mode == WatchMode::ClientWatched && self.state.schema_watch_fallback {
                    // No watcher covers schema inputs outside the workspace.
                    self.state.overlay.invalidate_all_trigger_registries();
                    self.state.refresh_all_diagnostics();
                }
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
                let workspace = self.state.config.effective().workspace.clone();
                let refresh = self.state.reload_config(params.settings);
                self.after_config_change(&workspace, refresh);
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
                if !self.cancel_full_index_waiter(&id) {
                    self.cancelled.record(id);
                }
            }
            Initialized::METHOD => {}
            other => {
                tracing::debug!(method = other, "ignoring notification");
            }
        }
    }

    /// Publishes config diagnostics, re-registers the workspace watchers
    /// when the discovery globs changed from `workspace`, and requests a
    /// semantic-token refresh when `refresh` says so.
    fn after_config_change(&mut self, workspace: &crate::config::WorkspaceConfig, refresh: bool) {
        show_config_diagnostics(&self.connection.sender, &self.state.config);
        if self.state.config.effective().workspace != *workspace {
            self.register_workspace_watchers(true);
        }
        if refresh {
            self.send_semantic_tokens_refresh();
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
    ///
    /// Every event drops the cached contexts above its path and invalidates
    /// the trigger registries it affects (every registry, for a context
    /// input, which can move a schema root). A context input (a package
    /// manifest or `.git` file) does nothing else. A config file or ignore
    /// file change re-lists the workspace (the config file also reloads the
    /// configuration). Any other change updates the file list unless the
    /// document is open; see [`ServerState::apply_disk_change`].
    fn apply_watched_changes(&mut self, params: DidChangeWatchedFilesParams) {
        let events = params.changes.into_iter().filter_map(|event| {
            uri_to_file_path(&event.uri).map(|path| (path, event.typ))
        });
        let changes = coalesce_changes(events);
        self.state
            .invalidate_contexts(changes.iter().map(|change| change.path.as_path()));
        // A manifest can add, remove, or move a schema root of any document.
        if changes.iter().any(|change| is_context_input(&change.path)) {
            self.state.overlay.invalidate_all_trigger_registries();
        } else {
            let paths: Vec<PathBuf> = changes.iter().map(|change| change.path.clone()).collect();
            self.state.overlay.invalidate_trigger_inputs(&paths);
        }
        // A discovery input rebuilds the list (and, for the config file,
        // everything the configuration shapes).
        if changes.iter().any(|change| is_config_file(&change.path)) {
            let workspace = self.state.config.effective().workspace.clone();
            let refresh = self.state.reload_config_file();
            self.after_config_change(&workspace, refresh);
        } else if changes.iter().any(|change| is_ignore_file(&change.path)) {
            self.state.relist();
        }
        for change in changes {
            if is_context_input(&change.path) || is_config_file(&change.path) || is_ignore_file(&change.path) {
                continue;
            }
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

/// The buffer name of an `untitled:` URI (`untitled:Untitled-1` gives
/// `Untitled-1`); `None` for any other scheme.
fn untitled_name(uri: &Uri) -> Option<String> {
    let rest = uri.as_str().strip_prefix("untitled:")?;
    let name = rest.rsplit(['/', '\\']).find(|segment| !segment.is_empty()).unwrap_or("untitled");
    Some(name.to_string())
}

/// How a dispatched request's response reads in the completion event.
fn response_outcome(response: &Response) -> Outcome {
    match &response.error {
        None => Outcome::Ok,
        Some(error) if error.code == ErrorCode::RequestCanceled as i32 => Outcome::Cancelled,
        Some(_) => Outcome::Error,
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

/// Sends one `window/showMessage` notification; a send failure (the client
/// is gone) is logged.
fn show_message(sender: &Sender<Message>, typ: MessageType, message: String) {
    let Ok(params) = serde_json::to_value(ShowMessageParams { typ, message }) else {
        return;
    };
    let notification = Notification::new(ShowMessage::METHOD.to_string(), params);
    if let Err(error) = sender.send(Message::Notification(notification)) {
        tracing::warn!("failed to show a message: {error}");
    }
}

/// Shows every finding of the configuration just loaded or changed, one
/// message each. The findings are recomputed on every load, so a migration
/// warning appears once per load or change and never per request.
fn show_config_diagnostics(sender: &Sender<Message>, config: &ConfigState) {
    for diagnostic in config.diagnostics() {
        let typ = match diagnostic.severity {
            ConfigSeverity::Error => MessageType::ERROR,
            ConfigSeverity::Warning => MessageType::WARNING,
        };
        show_message(sender, typ, format!("dmls: {diagnostic}"));
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
mod logging_tests;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_untitled_name_reads_only_untitled_uris() {
        let name = |text: &str| untitled_name(&text.parse::<Uri>().unwrap());
        assert_eq!(name("untitled:Untitled-1").as_deref(), Some("Untitled-1"));
        assert_eq!(name("untitled:/scratch/notes.md").as_deref(), Some("notes.md"));
        assert_eq!(name("untitled:").as_deref(), Some("untitled"));
        assert_eq!(name("file:///w/a.md"), None);
    }

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
