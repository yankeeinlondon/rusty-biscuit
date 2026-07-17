//! The daemon's configuration, its shared boot pipeline, and the handle that
//! owns a running instance.
//!
//! Everything here is transport-neutral. [`prepare_daemon`] builds the whole
//! stack — redb storage, DuckDB projection, batcher, node identity, session
//! log, registers, capability refresher, QUIC, discovery, peer workers, and the
//! gRPC service — exactly once, from a [`DaemonConfig`], with no idea what will
//! carry its bytes. [`local_transport`](crate::local_transport) then binds one
//! endpoint to it.
//!
//! That split is the point: a second transport adds a listener, an acceptor,
//! and a cleanup rule, and inherits the rest. It cannot grow a parallel copy of
//! the storage and network stack, because there is only one place to construct
//! them.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use rendezvous_core::local_endpoint::{LocalEndpoint, LocalEndpointError};
use rendezvous_core::{ChunkConfig, NodeIdentity, NodeIdentityError, RendezvousServer};
use tokio::sync::oneshot;
use tokio::task::JoinHandle;
use tonic::transport::Server;

use crate::batcher::{BatcherConfig, BatcherWorker, spawn as spawn_batcher};
use crate::discovery::{self, DiscoveryError, DiscoveryHandle};
use crate::peers::{PeerRegistry, PeerRegistryWorkers};
use crate::private_dir::{PrivateDirError, ensure_private_dir};
use crate::projection::{Projection, ProjectionError};
use crate::quic::{QuicEndpoint, QuicError};
use crate::refresher::spawn_register_refresher;
use crate::register::{RegisterError, RegisterStore};
use crate::service::RendezvousService;
use crate::session_log::{SessionLogError, SessionLogManager};
use crate::storage::{Storage, StorageError};
use crate::sync::SyncService;

/// Configuration for the persistence stack the daemon spawns alongside
/// the gRPC server.
#[derive(Clone, Debug)]
pub struct DaemonConfig {
    /// Root directory holding every durable file below. The daemon requires it
    /// to be private to the current OS user, and creates it if absent.
    pub data_dir: PathBuf,
    /// Path of the redb file used as the source of truth for Loro
    /// snapshots and the session-chunks catalog.
    pub storage_path: PathBuf,
    /// Path of the DuckDB file used as the analytical projection. Use
    /// [`DaemonConfig::with_in_memory_projection`] to keep the
    /// projection ephemeral (useful for tests).
    pub projection_path: Option<PathBuf>,
    /// Path of the secret-seed file backing the daemon's
    /// [`NodeIdentity`]. The file is created automatically on first
    /// boot with owner-only permissions on Unix.
    pub identity_path: PathBuf,
    /// Chunk rotation thresholds.
    pub chunk_config: ChunkConfig,
    /// Micro-batching configuration for the projection pipeline.
    pub batcher_config: BatcherConfig,
    /// Phase-4 networking configuration. `None` disables QUIC, mDNS,
    /// and the peer registry entirely, so the daemon still works for
    /// local-only Phase 1–3 testing.
    pub networking: Option<NetworkConfig>,
    /// Directories scanned (bounded depth) for git checkouts to fill
    /// the `repos/{node_id}` register. Empty (the default) leaves the
    /// register unwritten — the feature is opt-in until the host's
    /// coding roots are configured.
    pub repo_scan_roots: Vec<PathBuf>,
}

/// Configuration for the Phase-4 networking stack (QUIC + mDNS).
#[derive(Clone, Debug)]
pub struct NetworkConfig {
    /// UDP socket address the QUIC endpoint binds to. Use a `0` port
    /// to let the OS pick.
    pub quic_bind: SocketAddr,
    /// When `true`, advertise this daemon over mDNS and browse for
    /// peers. Disable in tests where multicast is unavailable.
    pub mdns_enabled: bool,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            quic_bind: SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 0),
            mdns_enabled: true,
        }
    }
}

impl DaemonConfig {
    /// Build a Phase-2 daemon config rooted under `data_dir`. The redb
    /// file lives at `<data_dir>/session.redb`, the DuckDB file at
    /// `<data_dir>/projection.duckdb`, and the node-identity seed at
    /// `<data_dir>/node.key`.
    ///
    /// ## Notes
    ///
    /// `data_dir` is authorized, not trusted: the daemon validates that it is
    /// private to the current user before opening the identity seed or the
    /// databases, whether it came from
    /// [`default_data_dir`](crate::private_dir::default_data_dir) or from an
    /// operator's override.
    #[must_use]
    pub fn with_data_dir(data_dir: impl AsRef<Path>) -> Self {
        let data_dir = data_dir.as_ref().to_path_buf();
        Self {
            storage_path: data_dir.join("session.redb"),
            projection_path: Some(data_dir.join("projection.duckdb")),
            identity_path: data_dir.join("node.key"),
            data_dir,
            chunk_config: ChunkConfig::default(),
            batcher_config: BatcherConfig::default(),
            networking: Some(NetworkConfig::default()),
            repo_scan_roots: Vec::new(),
        }
    }

    /// Configure the directories scanned for git checkouts (the
    /// `repos/{node_id}` register).
    #[must_use]
    pub fn with_repo_scan_roots(mut self, roots: Vec<PathBuf>) -> Self {
        self.repo_scan_roots = roots;
        self
    }

    /// Mark the projection database as in-memory only. The redb path is
    /// left untouched so durability is preserved.
    #[must_use]
    pub fn with_in_memory_projection(mut self) -> Self {
        self.projection_path = None;
        self
    }

    /// Override the Phase-4 networking configuration.
    #[must_use]
    pub fn with_networking(mut self, networking: NetworkConfig) -> Self {
        self.networking = Some(networking);
        self
    }

    /// Disable Phase-4 networking (QUIC + mDNS). Useful for tests that
    /// only need the gRPC/persistence stack.
    #[must_use]
    pub fn without_networking(mut self) -> Self {
        self.networking = None;
        self
    }

    /// Override the chunk-rotation configuration. Useful for tests that
    /// need to trigger rotation deterministically with a small entry
    /// cap.
    #[must_use]
    pub fn with_chunk_config(mut self, chunk_config: ChunkConfig) -> Self {
        self.chunk_config = chunk_config;
        self
    }
}

/// Errors that the server boot path can surface to the daemon binary.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ServerError {
    /// The endpoint is malformed, or names a transport this target cannot
    /// speak.
    #[error(transparent)]
    Endpoint(#[from] LocalEndpointError),

    /// Another daemon already holds the endpoint. Rendezvous does not evict a
    /// live daemon: the two would fight over the same databases.
    #[error("the rendezvous endpoint at {endpoint} is already in use by a running daemon")]
    EndpointInUse {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
    },

    /// Something already sits at the endpoint that the daemon will not remove:
    /// the wrong kind of entry, or one belonging to another user. Only a socket
    /// this user owns and nobody is listening on is reclaimable.
    #[error(
        "the rendezvous endpoint at {endpoint} is occupied by {found}, which the daemon will not remove"
    )]
    EndpointOccupied {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        /// What was found there, e.g. `a directory`.
        found: String,
    },

    /// The OS refused this process access to the endpoint.
    #[error("access denied preparing the rendezvous endpoint at {endpoint}")]
    AccessDenied {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        #[source]
        source: std::io::Error,
    },

    /// A directory or endpoint the daemon must own privately belongs to another
    /// user, is the wrong type, is reached through a symlink, or grants access
    /// beyond its owner.
    #[error(transparent)]
    Ownership(#[from] PrivateDirError),

    /// The listener could not be created at the endpoint, for a reason with no
    /// more specific variant.
    #[error("failed to listen on the rendezvous endpoint at {endpoint}")]
    Listener {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        #[source]
        source: std::io::Error,
    },

    /// The endpoint could not be released after the server stopped. The daemon
    /// has already shut down; what remains is a stale endpoint an operator may
    /// have to remove.
    #[error("failed to release the rendezvous endpoint at {endpoint}")]
    Cleanup {
        /// Lossy rendering of the endpoint, for the message only.
        endpoint: String,
        #[source]
        source: std::io::Error,
    },

    /// The redb storage layer could not be opened.
    #[error(transparent)]
    Storage(#[from] StorageError),

    /// The DuckDB projection could not be opened.
    #[error(transparent)]
    Projection(#[from] ProjectionError),

    /// The register store could not be rehydrated from storage.
    #[error(transparent)]
    Register(#[from] RegisterError),

    /// The session-log manager could not be initialised.
    #[error(transparent)]
    SessionLog(#[from] SessionLogError),

    /// The daemon's node-identity seed could not be loaded or generated.
    #[error(transparent)]
    Identity(#[from] NodeIdentityError),

    /// The Phase-4 QUIC endpoint could not be brought up.
    #[error(transparent)]
    Quic(#[from] QuicError),

    /// The Phase-4 mDNS discovery layer could not be brought up.
    #[error(transparent)]
    Discovery(#[from] DiscoveryError),

    /// The tonic transport returned an error while serving.
    #[error("tonic transport error: {0}")]
    Transport(#[from] tonic::transport::Error),

    /// The background server task could not be joined.
    #[error("server task join error: {0}")]
    Join(#[from] tokio::task::JoinError),
}

/// Releases a bound endpoint once the server has stopped.
///
/// What that means differs per transport — a Unix socket is a filesystem entry
/// somebody has to unlink, a named pipe is closed with its handles — so the
/// shared shutdown path holds the rule as a token rather than knowing it.
pub(crate) trait EndpointCleanup: Send {
    /// Release the endpoint. Called at most once, from shutdown or drop.
    fn release(&mut self) -> Result<(), ServerError>;
}

/// Owns a running daemon: its endpoint, its workers, and a oneshot shutdown
/// trigger. Drop the handle to stop without waiting, or call
/// [`ServerHandle::shutdown`] to await a clean stop.
#[must_use = "the daemon stops only when this handle is shut down or dropped"]
pub struct ServerHandle {
    endpoint: LocalEndpoint,
    cleanup: Option<Box<dyn EndpointCleanup>>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    join_handle: Option<JoinHandle<Result<(), tonic::transport::Error>>>,
    batcher: Option<BatcherWorker>,
    capability_task: Option<JoinHandle<()>>,
    capability_shutdown_tx: Option<oneshot::Sender<()>>,
    identity: Arc<NodeIdentity>,
    peers: Option<PeerRegistry>,
    peer_workers: Option<PeerRegistryWorkers>,
    discovery: Option<DiscoveryHandle>,
    quic_local_addr: Option<SocketAddr>,
}

impl ServerHandle {
    /// The endpoint this daemon is serving on.
    #[must_use]
    pub fn local_endpoint(&self) -> &LocalEndpoint {
        &self.endpoint
    }

    #[must_use]
    pub fn node_id(&self) -> String {
        self.identity.node_id()
    }

    /// Shared handle on the daemon's signing identity. Useful for
    /// integration tests that need to verify envelopes produced by the
    /// daemon.
    #[must_use]
    pub fn identity(&self) -> Arc<NodeIdentity> {
        Arc::clone(&self.identity)
    }

    /// Local QUIC bind address, when Phase-4 networking is enabled.
    #[must_use]
    pub fn quic_local_addr(&self) -> Option<SocketAddr> {
        self.quic_local_addr
    }

    /// Shared handle on the peer registry, when Phase-4 networking is
    /// enabled.
    #[must_use]
    pub fn peers(&self) -> Option<PeerRegistry> {
        self.peers.clone()
    }

    /// Signal the server to stop, await its completion, release the endpoint,
    /// and shut down the projection batcher. Subsequent calls are no-ops.
    ///
    /// ## Errors
    ///
    /// Returns [`ServerError::Join`] when a worker task panicked,
    /// [`ServerError::Transport`] when tonic reported a serving failure, and
    /// [`ServerError::Cleanup`] when the endpoint could not be released.
    pub async fn shutdown(mut self) -> Result<(), ServerError> {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(join) = self.join_handle.take() {
            join.await??;
        }
        if let Some(mut workers) = self.peer_workers.take() {
            workers.shutdown();
        }
        if let Some(mut discovery) = self.discovery.take() {
            discovery.shutdown().await;
        }
        if let Some(registry) = self.peers.as_ref()
            && let Some(mut endpoint) = registry.take_endpoint()
        {
            endpoint.shutdown().await;
        }
        if let Some(worker) = self.batcher.take() {
            tokio::task::spawn_blocking(move || worker.shutdown())
                .await
                .map_err(ServerError::Join)?;
        }
        // Signal the capability refresher and WAIT for it: an
        // in-flight detection pass holds the storage handle, and a
        // restarted daemon reopening the same redb file would hit
        // `DatabaseAlreadyOpen` if we only fired-and-forgot.
        if let Some(tx) = self.capability_shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.capability_task.take() {
            let _ = task.await;
        }
        if let Some(mut cleanup) = self.cleanup.take() {
            cleanup.release()?;
        }
        Ok(())
    }
}

impl Drop for ServerHandle {
    fn drop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(mut workers) = self.peer_workers.take() {
            workers.shutdown();
        }
        if let Some(worker) = self.batcher.take() {
            worker.shutdown();
        }
        if let Some(tx) = self.capability_shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.capability_task.take() {
            // Drop cannot await; the refresher exits on its own after
            // the signal (waiting out any in-flight detection pass).
            task.abort();
        }
        if let Some(mut cleanup) = self.cleanup.take()
            && let Err(error) = cleanup.release()
        {
            tracing::warn!(
                target: "rendezvous_daemon::server",
                endpoint = %self.endpoint,
                %error,
                "failed to release the local endpoint"
            );
        }
    }
}

/// Everything a daemon needs except a way to be reached.
///
/// Produced once by [`prepare_daemon`] and consumed once by a transport's
/// serve path.
pub(crate) struct PreparedDaemon {
    pub(crate) service: RendezvousService,
    identity: Arc<NodeIdentity>,
    batcher: BatcherWorker,
    capability_task: JoinHandle<()>,
    capability_shutdown_tx: oneshot::Sender<()>,
    peers: Option<PeerRegistry>,
    peer_workers: Option<PeerRegistryWorkers>,
    discovery: Option<DiscoveryHandle>,
    quic_local_addr: Option<SocketAddr>,
}

/// Build the daemon's storage, identity, register, network, and worker stack.
///
/// ## Errors
///
/// Returns [`ServerError::Ownership`] when the data root is not private to this
/// user, and the corresponding subsystem variant when storage, the projection,
/// the identity seed, the registers, QUIC, or discovery cannot be brought up.
pub(crate) fn prepare_daemon(config: DaemonConfig) -> Result<PreparedDaemon, ServerError> {
    // Before the identity seed or any database is opened: a data root another
    // user can write is a data root another user can plant a signing key in.
    ensure_private_dir(&config.data_dir)?;

    let storage = Storage::open(&config.storage_path)?;
    let projection = match &config.projection_path {
        Some(path) => Projection::open(path)?,
        None => Projection::in_memory()?,
    };
    let batcher = spawn_batcher(projection.clone(), config.batcher_config.clone());
    let identity = Arc::new(NodeIdentity::load_or_generate(&config.identity_path)?);
    tracing::info!(
        target: "rendezvous_daemon::server",
        node_id = %identity.node_id(),
        identity_path = %config.identity_path.display(),
        "loaded node identity"
    );
    let session_log = SessionLogManager::new(
        storage.clone(),
        batcher.handle(),
        projection.clone(),
        config.chunk_config,
        Arc::clone(&identity),
    )?;
    let registers = RegisterStore::new(storage.clone(), Arc::clone(&identity))?;
    let sync_service = SyncService::new(
        session_log.clone(),
        registers.clone(),
        storage.clone(),
        Arc::clone(&identity),
    );
    // Fill (and hourly refresh) this host's capability register so the
    // mesh learns what this node can run.
    let (capability_shutdown_tx, capability_shutdown_rx) = oneshot::channel();
    let capability_task = spawn_register_refresher(
        registers.clone(),
        config.repo_scan_roots.clone(),
        capability_shutdown_rx,
    );

    let mut service = RendezvousService::new(
        session_log,
        projection.clone(),
        Arc::clone(&identity),
        storage.clone(),
        sync_service.clone(),
        registers,
    );
    let (peers, peer_workers, discovery, quic_local_addr) = match config.networking.clone() {
        Some(net_config) => {
            let mut quic_endpoint = QuicEndpoint::bind(net_config.quic_bind)?;
            let quic_local_addr = quic_endpoint.local_addr();
            let inbound_rx = quic_endpoint.take_accept_rx();

            let (discovery_handle, discovery_rx) = if net_config.mdns_enabled {
                let instance_name = format!("rs-{}", &identity.node_id()[..16]);
                let mut handle = discovery::start(
                    &instance_name,
                    &identity.node_id(),
                    quic_local_addr.port(),
                    Vec::new(),
                )?;
                let rx = handle.take_rx();
                (Some(handle), rx)
            } else {
                (None, None)
            };

            let (registry, workers) =
                PeerRegistry::spawn_with(quic_endpoint, discovery_rx, inbound_rx);
            registry.set_sync_service(sync_service.clone());
            service = service.with_peers(registry.clone(), quic_local_addr);
            tracing::info!(
                target: "rendezvous_daemon::server",
                quic_addr = %quic_local_addr,
                mdns = net_config.mdns_enabled,
                "Phase-4 networking ready",
            );
            (Some(registry), Some(workers), discovery_handle, Some(quic_local_addr))
        }
        None => (None, None, None, None),
    };

    crate::local_transport::record_shared_boot();

    Ok(PreparedDaemon {
        service,
        identity,
        batcher,
        capability_task,
        capability_shutdown_tx,
        peers,
        peer_workers,
        discovery,
        quic_local_addr,
    })
}

/// Serve `prepared` over `incoming` until shutdown, and hand back the handle
/// that owns the running daemon.
///
/// Every transport ends here: `incoming` is the only thing that differs between
/// a Unix socket and a named pipe by the time tonic sees it.
pub(crate) fn serve_local_incoming<S, IO, IoError>(
    endpoint: LocalEndpoint,
    prepared: PreparedDaemon,
    incoming: S,
    cleanup: Box<dyn EndpointCleanup>,
) -> ServerHandle
where
    S: tokio_stream::Stream<Item = Result<IO, IoError>> + Send + 'static,
    IO: tonic::transport::server::Connected + tokio::io::AsyncRead + tokio::io::AsyncWrite,
    IO: Send + Unpin + 'static,
    IoError: Into<Box<dyn std::error::Error + Send + Sync>> + Send + 'static,
{
    let PreparedDaemon {
        service,
        identity,
        batcher,
        capability_task,
        capability_shutdown_tx,
        peers,
        peer_workers,
        discovery,
        quic_local_addr,
    } = prepared;

    let server = RendezvousServer::new(service);
    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let join_handle = tokio::spawn(async move {
        Server::builder()
            .add_service(server)
            .serve_with_incoming_shutdown(incoming, async move {
                let _ = shutdown_rx.await;
            })
            .await
    });

    ServerHandle {
        endpoint,
        cleanup: Some(cleanup),
        shutdown_tx: Some(shutdown_tx),
        join_handle: Some(join_handle),
        batcher: Some(batcher),
        capability_task: Some(capability_task),
        capability_shutdown_tx: Some(capability_shutdown_tx),
        identity,
        peers,
        peer_workers,
        discovery,
        quic_local_addr,
    }
}

#[cfg(test)]
mod tests;
