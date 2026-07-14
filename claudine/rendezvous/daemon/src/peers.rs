//! In-memory peer registry plus the orchestration glue that ties the
//! QUIC endpoint and mDNS discovery together.
//!
//! The registry indexes peers by hex-encoded `node_id`. When mDNS
//! resolves a service or a manual invitation is decoded, the resulting
//! [`PeerRecord`] lands in the registry; a follow-up
//! [`PeerRegistry::connect`] (or an inbound handshake) flips its
//! connection state.

use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use rendezvous_core::{PeerConnectionState, PeerInfo, PeerSource};
use tokio::sync::{mpsc, Semaphore};
use tokio::task::{JoinHandle, JoinSet};

use crate::discovery::DiscoveredPeer;
use crate::quic::{InboundConnection, QuicError, QuicEndpoint};
use crate::sync::{SyncError, SyncService};

/// Cadence of the periodic re-sync worker. Chosen so at least two
/// sync rounds fall inside the dashboard's 60s staleness window, keeping
/// `last_synced_unix_ms` fresh for a reachable-and-healthy peer.
///
/// The round itself runs every peer CONCURRENTLY under
/// [`RESYNC_MAX_CONCURRENCY`], and each peer's sync is bounded by
/// [`RESYNC_PER_PEER_TIMEOUT`] (strictly shorter than this interval), so
/// one wedged peer can never push a concurrently-healthy peer past the
/// 60s threshold. See [`PeerRegistry::resync_connected_peers`].
const RESYNC_INTERVAL: Duration = Duration::from_secs(20);

/// Per-peer bound on a single re-sync round. Kept strictly shorter than
/// [`RESYNC_INTERVAL`] so a peer that accepts QUIC but then stops
/// responding is abandoned well before the next tick — it can never
/// consume a whole interval and starve the healthy peers sharing the
/// round. Deliberately tighter than [`crate::sync::SYNC_OVERALL_TIMEOUT`]
/// (the engine's own 30s ceiling), which therefore never dominates here.
const RESYNC_PER_PEER_TIMEOUT: Duration = Duration::from_secs(8);

/// Upper bound on concurrent per-peer sync sessions inside one round.
/// Bounds fan-out (file descriptors, CPU, redb contention) on a large
/// mesh while still letting a healthy peer make progress independently
/// of a wedged one.
const RESYNC_MAX_CONCURRENCY: usize = 8;

/// Maximum extra delay added to each tick before a round runs. Spreads
/// rounds across daemons so a freshly-booted mesh does not thunder-herd
/// on a shared 20s boundary. Small relative to [`RESYNC_INTERVAL`] so the
/// two-rounds-per-60s freshness property still holds.
const RESYNC_TICK_JITTER: Duration = Duration::from_secs(2);

/// Base delay before the first automatic re-dial of a
/// disconnected-but-known peer. Subsequent failed attempts back off
/// exponentially up to [`RECONNECT_BACKOFF_MAX`].
const RECONNECT_BACKOFF_BASE: Duration = Duration::from_secs(5);

/// Ceiling on the exponential re-dial backoff.
const RECONNECT_BACKOFF_MAX: Duration = Duration::from_secs(300);

/// A single peer's re-sync work, driven by [`run_bounded_round`]. Boxed
/// so the production (real QUIC) and test (in-memory) job futures share
/// one homogeneous type.
type BoxResyncFuture = Pin<Box<dyn Future<Output = ResyncOutcome> + Send>>;

/// Outcome of one peer's re-sync attempt, as interpreted by
/// [`PeerRegistry::apply_resync_outcome`].
#[derive(Debug)]
enum ResyncOutcome {
    /// The round converged; stamp `last_synced_unix_ms`.
    Ok,
    /// The per-peer timeout fired. The QUIC connection is (probably)
    /// still open — the peer is wedged, not gone — so it stays
    /// `Connected` and is retried next tick.
    TimedOut,
    /// Sync failed but the connection is still alive (e.g. `NotPaired`,
    /// a protocol hiccup). Record the error; keep the peer `Connected`.
    SoftError(String),
    /// The connection is closed/dead. Transition the peer OUT of
    /// `Connected` so `ListPeers`/staleness reflect reality, and arm the
    /// re-dial path.
    DeadConnection(String),
}

/// Snapshot of what the daemon knows about a single peer.
#[derive(Clone, Debug)]
pub struct PeerRecord {
    pub node_id: String,
    pub socket_addr: SocketAddr,
    pub source: PeerSource,
    pub state: PeerConnectionState,
    pub last_seen_unix_ms: i64,
    /// Timestamp of the most recent successful direct-sync round with
    /// this peer (0 = never synced). Stamped by
    /// [`PeerRegistry::record_sync_success`]; the honest freshness
    /// clock the dashboard's staleness threshold reads.
    pub last_synced_unix_ms: i64,
    pub last_error: Option<String>,
    /// Active QUIC connection (when one is open). Held so the daemon
    /// can drive outbound sync sessions to this peer without
    /// re-dialling. Not exposed in [`PeerRecord::to_proto`].
    pub connection: Option<quinn::Connection>,
    /// Earliest wall-clock time (Unix ms) at which the periodic worker
    /// may attempt an automatic re-dial of this peer. `0` = eligible
    /// immediately. Advanced by the exponential backoff in
    /// [`reconnect_backoff`] after each failed attempt. Only consulted
    /// while `state == Disconnected`.
    pub reconnect_after_unix_ms: i64,
    /// Count of consecutive failed automatic re-dials; drives the
    /// exponential backoff. Reset to `0` on any successful sync.
    pub reconnect_attempts: u32,
}

impl PeerRecord {
    fn to_proto(&self) -> PeerInfo {
        PeerInfo {
            node_id: self.node_id.clone(),
            socket_addr: self.socket_addr.to_string(),
            source: self.source as i32,
            state: self.state as i32,
            last_seen_unix_ms: self.last_seen_unix_ms,
            last_error: self.last_error.clone().unwrap_or_default(),
            last_synced_unix_ms: self.last_synced_unix_ms,
        }
    }

    fn new_discovered(
        node_id: String,
        socket_addr: SocketAddr,
        source: PeerSource,
        now_unix_ms: i64,
    ) -> Self {
        Self {
            node_id,
            socket_addr,
            source,
            state: PeerConnectionState::Discovered,
            last_seen_unix_ms: now_unix_ms,
            last_synced_unix_ms: 0,
            last_error: None,
            connection: None,
            reconnect_after_unix_ms: 0,
            reconnect_attempts: 0,
        }
    }
}

/// Keyed peer index.
type PeerMap = HashMap<String, PeerRecord>;

/// Thread-safe handle on the peer registry plus its bound QUIC
/// endpoint.
#[derive(Clone)]
pub struct PeerRegistry {
    inner: Arc<Inner>,
}

struct Inner {
    peers: RwLock<PeerMap>,
    endpoint: Arc<RwLock<Option<QuicEndpoint>>>,
    sync_service: RwLock<Option<SyncService>>,
}

impl std::fmt::Debug for PeerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PeerRegistry")
            .field("peer_count", &self.inner.peers.read().len())
            .finish_non_exhaustive()
    }
}

/// Background workers spawned by [`PeerRegistry::spawn_with`]. Holding
/// the handle keeps the workers alive; dropping it lets them shut
/// down naturally as their inputs close.
pub struct PeerRegistryWorkers {
    discovery_task: Option<JoinHandle<()>>,
    inbound_task: Option<JoinHandle<()>>,
    resync_task: Option<JoinHandle<()>>,
}

impl PeerRegistryWorkers {
    /// Abort the background workers. Safe to call multiple times.
    pub fn shutdown(&mut self) {
        if let Some(task) = self.discovery_task.take() {
            task.abort();
        }
        if let Some(task) = self.inbound_task.take() {
            task.abort();
        }
        if let Some(task) = self.resync_task.take() {
            task.abort();
        }
    }
}

impl Drop for PeerRegistryWorkers {
    fn drop(&mut self) {
        self.shutdown();
    }
}

impl PeerRegistry {
    /// Build a registry from the local QUIC endpoint plus the
    /// discovery / inbound channels. Background tasks are spawned that
    /// translate channel traffic into [`PeerRecord`] updates.
    pub fn spawn_with(
        endpoint: QuicEndpoint,
        discovery_rx: Option<mpsc::UnboundedReceiver<DiscoveredPeer>>,
        inbound_rx: Option<mpsc::UnboundedReceiver<InboundConnection>>,
    ) -> (Self, PeerRegistryWorkers) {
        let inner = Arc::new(Inner {
            peers: RwLock::new(HashMap::new()),
            endpoint: Arc::new(RwLock::new(Some(endpoint))),
            sync_service: RwLock::new(None),
        });
        let registry = Self {
            inner: Arc::clone(&inner),
        };

        let discovery_task = discovery_rx.map(|mut rx| {
            let reg = registry.clone();
            tokio::spawn(async move {
                while let Some(peer) = rx.recv().await {
                    reg.record_discovery(peer);
                }
            })
        });

        let inbound_task = inbound_rx.map(|mut rx| {
            let reg = registry.clone();
            tokio::spawn(async move {
                while let Some(conn) = rx.recv().await {
                    reg.record_inbound(conn);
                }
            })
        });

        // Keep `last_synced` advancing on a healthy mesh. The first
        // tick fires immediately from `interval`; consume it so the
        // worker's real cadence starts one interval out (the
        // connect-time sync already stamps freshness). Aborting this
        // task (see `PeerRegistryWorkers::shutdown`) drops the round's
        // `JoinSet`, which cancels any in-flight per-peer sync tasks.
        let resync_task = {
            let reg = registry.clone();
            Some(tokio::spawn(async move {
                let mut ticker = tokio::time::interval(RESYNC_INTERVAL);
                ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
                ticker.tick().await;
                let mut jitter = Jitter::new();
                loop {
                    ticker.tick().await;
                    // Per-tick jitter so a booted mesh does not sync on a
                    // shared 20s boundary.
                    tokio::time::sleep(jitter.duration_up_to(RESYNC_TICK_JITTER)).await;
                    reg.resync_connected_peers().await;
                }
            }))
        };

        (
            registry,
            PeerRegistryWorkers {
                discovery_task,
                inbound_task,
                resync_task,
            },
        )
    }

    /// Snapshot of all known peers, sorted by `node_id` for stable
    /// CLI/RPC output.
    pub fn list(&self) -> Vec<PeerInfo> {
        let map = self.inner.peers.read();
        let mut out: Vec<PeerInfo> = map.values().map(PeerRecord::to_proto).collect();
        out.sort_by(|a, b| a.node_id.cmp(&b.node_id));
        out
    }

    /// Install (or replace) the [`SyncService`] used to drive inbound
    /// sync sessions and to expose connections for outbound sync.
    pub fn set_sync_service(&self, service: SyncService) {
        *self.inner.sync_service.write() = Some(service);
    }

    /// Borrow the active QUIC connection for `node_id`, if one is
    /// open. Returned by clone so the caller can drive it without
    /// holding the peer-map lock.
    #[must_use]
    pub fn connection_for(&self, node_id: &str) -> Option<quinn::Connection> {
        self.inner
            .peers
            .read()
            .get(node_id)
            .and_then(|rec| rec.connection.clone())
    }

    /// Connect to a peer described by a decoded invitation. The peer
    /// record is created (or updated) with `state =
    /// CONNECTED`/`FAILED` based on the outcome of the QUIC handshake.
    pub async fn connect(
        &self,
        node_id: String,
        socket_addr: SocketAddr,
        source: PeerSource,
    ) -> Result<PeerInfo, QuicError> {
        self.upsert_manual(node_id.clone(), socket_addr, source);
        let endpoint = self
            .inner
            .endpoint
            .read()
            .as_ref()
            .map(QuicEndpoint::endpoint);
        let Some(endpoint) = endpoint else {
            // Endpoint already torn down — leave the record in the
            // FAILED state and let the caller see it.
            self.mark_failed(&node_id, "endpoint shut down");
            return Ok(self.snapshot(&node_id));
        };

        let mut updated = self.snapshot(&node_id);
        updated.state = PeerConnectionState::Connecting as i32;
        self.update_state(&node_id, PeerConnectionState::Connecting, None);

        let connect_future = endpoint.connect(socket_addr, "rendezvous");
        let result = match connect_future {
            Ok(connecting) => {
                tokio::time::timeout(Duration::from_secs(5), connecting)
                    .await
                    .map_err(|_| QuicError::Connection(quinn::ConnectionError::TimedOut))
                    .and_then(|res| res.map_err(QuicError::Connection))
            }
            Err(err) => Err(QuicError::Connect(err)),
        };

        match result {
            Ok(conn) => {
                self.attach_connection(&node_id, conn.clone());
                self.update_state(&node_id, PeerConnectionState::Connected, None);
                // Fire-and-forget sync attempt: if the peer is paired
                // on both sides, the protocol converges in one round.
                // If pairing is missing, the engine returns
                // `NotPaired` and we simply leave the connection idle.
                let sync_service = self.inner.sync_service.read().clone();
                if let Some(service) = sync_service {
                    let conn_for_sync = conn.clone();
                    let registry = self.clone();
                    let node_for_sync = node_id.clone();
                    tokio::spawn(async move {
                        match service
                            .sync_initiator(&conn_for_sync, &node_for_sync)
                            .await
                        {
                            Ok(outcome) => {
                                registry.record_sync_success(&node_for_sync);
                                tracing::info!(
                                    target: "rendezvous_daemon::peers",
                                    peer = %node_for_sync,
                                    received_bytes = outcome.received_bytes,
                                    sent_bytes = outcome.sent_bytes,
                                    "initial sync completed",
                                );
                            }
                            Err(error) => {
                                tracing::debug!(
                                    target: "rendezvous_daemon::peers",
                                    peer = %node_for_sync,
                                    %error,
                                    "initial sync skipped or failed",
                                );
                                registry.record_sync_error(&node_for_sync, &error.to_string());
                            }
                        }
                    });
                }
            }
            Err(error) => {
                let msg = error.to_string();
                self.update_state(&node_id, PeerConnectionState::Failed, Some(msg));
                return Err(error);
            }
        }
        Ok(self.snapshot(&node_id))
    }

    fn attach_connection(&self, node_id: &str, conn: quinn::Connection) {
        let mut map = self.inner.peers.write();
        if let Some(rec) = map.get_mut(node_id) {
            rec.connection = Some(conn);
        }
    }

    fn record_sync_error(&self, node_id: &str, msg: &str) {
        let mut map = self.inner.peers.write();
        if let Some(rec) = map.get_mut(node_id) {
            rec.last_error = Some(msg.to_string());
        }
    }

    /// Stamp a successful direct-sync round against `node_id`. This is
    /// the only writer of `last_synced_unix_ms` — the freshness clock
    /// the dashboard reads, deliberately independent of the
    /// mDNS-driven `last_seen_unix_ms`. Called by every sync path
    /// (connect-time auto-sync, the periodic worker, and the explicit
    /// `SyncWithPeer` RPC) so the clock advances no matter what drove
    /// the round.
    pub fn record_sync_success(&self, node_id: &str) {
        let mut map = self.inner.peers.write();
        if let Some(rec) = map.get_mut(node_id) {
            rec.last_synced_unix_ms = unix_now_ms();
            rec.last_error = None;
            // A round converged: the connection is healthy again, so
            // clear any re-dial backoff accrued while it was down.
            rec.reconnect_attempts = 0;
            rec.reconnect_after_unix_ms = 0;
        }
    }

    /// Re-run the direct-sync initiator against every currently
    /// connected peer, stamping [`record_sync_success`] on each round
    /// that converges. Driven on an interval by the periodic re-sync
    /// worker (see [`PeerRegistry::spawn_with`]) so `last_synced`
    /// keeps advancing on a healthy mesh — the initial sync alone is
    /// one-shot-on-connect and would let the freshness clock age out
    /// past the dashboard's 60s threshold.
    ///
    /// Every connected peer runs CONCURRENTLY (capped by
    /// [`RESYNC_MAX_CONCURRENCY`]) with a per-peer bound of
    /// [`RESYNC_PER_PEER_TIMEOUT`], so a peer that accepts QUIC then
    /// wedges cannot delay a healthy peer in the same round.
    ///
    /// After the sync phase, disconnected-but-known peers are re-dialled
    /// (see the reconnection-ownership note below).
    ///
    /// ## Notes
    ///
    /// Idle cost budget: every round advertises ALL known chunks and
    /// registers to every connected peer, even when nothing changed —
    /// there is deliberately no change-detection short-circuit yet. Cost
    /// per daemon is therefore ~O(peers × documents) every
    /// [`RESYNC_INTERVAL`], approaching O(hosts² × documents) across a
    /// full mesh. Acceptable at the current small-mesh scale; revisit
    /// with a version-vector diff before scaling out.
    ///
    /// [`record_sync_success`]: PeerRegistry::record_sync_success
    async fn resync_connected_peers(&self) {
        let Some(service) = self.inner.sync_service.read().clone() else {
            return;
        };
        let now = unix_now_ms();

        // --- Sync phase: every connected peer, concurrently. ---------
        // Snapshot targets before awaiting so the peer-map lock is never
        // held across a sync round.
        let sync_targets: Vec<(String, quinn::Connection)> = {
            let map = self.inner.peers.read();
            map.values()
                .filter(|rec| rec.state == PeerConnectionState::Connected)
                .filter_map(|rec| {
                    rec.connection.clone().map(|conn| (rec.node_id.clone(), conn))
                })
                .collect()
        };
        if !sync_targets.is_empty() {
            let jobs: Vec<(String, BoxResyncFuture)> = sync_targets
                .into_iter()
                .map(|(node_id, connection)| {
                    let service = service.clone();
                    let job_node_id = node_id.clone();
                    let fut: BoxResyncFuture = Box::pin(async move {
                        match service.sync_initiator(&connection, &job_node_id).await {
                            Ok(_) => ResyncOutcome::Ok,
                            Err(error) => {
                                let msg = error.to_string();
                                if sync_error_is_fatal_to_connection(&error, &connection) {
                                    ResyncOutcome::DeadConnection(msg)
                                } else {
                                    ResyncOutcome::SoftError(msg)
                                }
                            }
                        }
                    });
                    (node_id, fut)
                })
                .collect();
            let results =
                run_bounded_round(jobs, RESYNC_PER_PEER_TIMEOUT, RESYNC_MAX_CONCURRENCY).await;
            for (node_id, outcome, _elapsed) in results {
                self.apply_resync_outcome(&node_id, outcome);
            }
        }

        // --- Reconnection phase --------------------------------------
        // Reconnection ownership: the periodic re-sync worker is the
        // SINGLE owner of automatic re-dial. A sync that finds a dead
        // connection parks the peer in `Disconnected` (dropping the
        // stale `quinn::Connection`); this phase then re-dials it on a
        // later tick, backing off exponentially. mDNS rediscovery only
        // refreshes address/`last_seen` — it never opens a connection —
        // and the explicit `ConnectToPeer` RPC remains an operator
        // action, so without this path a returning peer would never
        // recover on its own.
        let reconnect_targets = self.collect_reconnect_targets(now);
        if !reconnect_targets.is_empty() {
            let sem = Arc::new(Semaphore::new(RESYNC_MAX_CONCURRENCY.max(1)));
            let mut set = JoinSet::new();
            for (node_id, addr, source) in reconnect_targets {
                let registry = self.clone();
                let sem = Arc::clone(&sem);
                set.spawn(async move {
                    let _permit = sem.acquire_owned().await.expect("semaphore not closed");
                    registry.attempt_reconnect(node_id, addr, source).await;
                });
            }
            while set.join_next().await.is_some() {}
        }
    }

    /// Fold one peer's [`ResyncOutcome`] back into its record.
    fn apply_resync_outcome(&self, node_id: &str, outcome: ResyncOutcome) {
        match outcome {
            ResyncOutcome::Ok => self.record_sync_success(node_id),
            ResyncOutcome::TimedOut => self.record_sync_error(
                node_id,
                &format!("re-sync timed out after {RESYNC_PER_PEER_TIMEOUT:?}"),
            ),
            ResyncOutcome::SoftError(msg) => self.record_sync_error(node_id, &msg),
            ResyncOutcome::DeadConnection(msg) => {
                tracing::debug!(
                    target: "rendezvous_daemon::peers",
                    peer = %node_id,
                    error = %msg,
                    "peer connection dead; transitioning to Disconnected",
                );
                self.mark_disconnected(node_id, msg);
            }
        }
    }

    /// Move a peer OUT of `Connected` after its connection was found
    /// dead, dropping the stale handle and arming the re-dial path. The
    /// backoff counter is preserved so repeated failures keep growing
    /// the delay.
    fn mark_disconnected(&self, node_id: &str, error: String) {
        let mut map = self.inner.peers.write();
        if let Some(rec) = map.get_mut(node_id) {
            rec.state = PeerConnectionState::Disconnected;
            rec.connection = None;
            rec.last_error = Some(error);
            rec.last_seen_unix_ms = unix_now_ms();
            // Eligible for re-dial from now, subject to any existing
            // backoff window from prior failures.
            rec.reconnect_after_unix_ms = rec.reconnect_after_unix_ms.max(unix_now_ms());
        }
    }

    /// Disconnected peers whose backoff window has elapsed and that have
    /// no live connection — the automatic re-dial candidates for this
    /// tick.
    fn collect_reconnect_targets(&self, now: i64) -> Vec<(String, SocketAddr, PeerSource)> {
        let map = self.inner.peers.read();
        map.values()
            .filter(|rec| rec.state == PeerConnectionState::Disconnected)
            .filter(|rec| rec.connection.is_none())
            .filter(|rec| rec.reconnect_after_unix_ms <= now)
            .map(|rec| (rec.node_id.clone(), rec.socket_addr, rec.source))
            .collect()
    }

    /// Attempt a single automatic re-dial. On success [`connect`] has
    /// already reset the state and fired an initial sync; on failure the
    /// peer returns to `Disconnected` with a longer backoff.
    ///
    /// [`connect`]: PeerRegistry::connect
    async fn attempt_reconnect(&self, node_id: String, addr: SocketAddr, source: PeerSource) {
        match self.connect(node_id.clone(), addr, source).await {
            Ok(info) if info.state == PeerConnectionState::Connected as i32 => {
                self.reset_reconnect_backoff(&node_id);
            }
            _ => self.note_reconnect_failure(&node_id),
        }
    }

    /// Grow the re-dial backoff after a failed automatic reconnect and
    /// park the peer back in `Disconnected` ([`connect`] leaves it
    /// `Failed`).
    ///
    /// [`connect`]: PeerRegistry::connect
    fn note_reconnect_failure(&self, node_id: &str) {
        let mut map = self.inner.peers.write();
        if let Some(rec) = map.get_mut(node_id) {
            rec.reconnect_attempts = rec.reconnect_attempts.saturating_add(1);
            rec.state = PeerConnectionState::Disconnected;
            let backoff = reconnect_backoff(rec.reconnect_attempts);
            rec.reconnect_after_unix_ms =
                unix_now_ms().saturating_add(backoff.as_millis() as i64);
        }
    }

    fn reset_reconnect_backoff(&self, node_id: &str) {
        let mut map = self.inner.peers.write();
        if let Some(rec) = map.get_mut(node_id) {
            rec.reconnect_attempts = 0;
            rec.reconnect_after_unix_ms = 0;
        }
    }

    fn record_discovery(&self, peer: DiscoveredPeer) {
        let mut map = self.inner.peers.write();
        let now = unix_now_ms();
        let entry = map
            .entry(peer.node_id.clone())
            .or_insert_with(|| PeerRecord::new_discovered(
                peer.node_id.clone(),
                peer.socket_addr,
                PeerSource::Mdns,
                now,
            ));
        entry.socket_addr = peer.socket_addr;
        entry.last_seen_unix_ms = now;
        if entry.source == PeerSource::Unspecified {
            entry.source = PeerSource::Mdns;
        }
    }

    fn record_inbound(&self, conn: InboundConnection) {
        let temp_key = format!("inbound:{}", conn.remote_addr);
        let now = unix_now_ms();
        {
            let mut map = self.inner.peers.write();
            let entry = map.entry(temp_key.clone()).or_insert_with(|| PeerRecord {
                node_id: temp_key.clone(),
                socket_addr: conn.remote_addr,
                source: PeerSource::Inbound,
                state: PeerConnectionState::Connected,
                last_seen_unix_ms: now,
                last_synced_unix_ms: 0,
                last_error: None,
                connection: None,
                reconnect_after_unix_ms: 0,
                reconnect_attempts: 0,
            });
            entry.socket_addr = conn.remote_addr;
            entry.state = PeerConnectionState::Connected;
            entry.last_seen_unix_ms = now;
            entry.last_error = None;
            entry.connection = Some(conn.connection.clone());
        }

        // Spawn a sync-responder loop that drains any bidirectional
        // streams the peer opens for sync sessions. Each successful
        // handshake tells us the peer's real node_id, so we re-key the
        // placeholder record under that identity.
        let sync_service = self.inner.sync_service.read().clone();
        if let Some(service) = sync_service {
            let connection = conn.connection.clone();
            let registry = self.clone();
            let temp_key = temp_key.clone();
            tokio::spawn(async move {
                loop {
                    match connection.accept_bi().await {
                        Ok((send, recv)) => {
                            let service = service.clone();
                            let registry = registry.clone();
                            let temp_key = temp_key.clone();
                            tokio::spawn(async move {
                                if let Err(error) = service
                                    .sync_responder_with_callback(
                                        send,
                                        recv,
                                        move |real_node_id| {
                                            registry.rekey_inbound(&temp_key, &real_node_id);
                                        },
                                    )
                                    .await
                                {
                                    tracing::debug!(
                                        target: "rendezvous_daemon::peers",
                                        %error,
                                        "inbound sync session ended with error",
                                    );
                                }
                            });
                        }
                        Err(quinn::ConnectionError::ApplicationClosed(_))
                        | Err(quinn::ConnectionError::LocallyClosed)
                        | Err(quinn::ConnectionError::Reset) => {
                            return;
                        }
                        Err(error) => {
                            tracing::debug!(
                                target: "rendezvous_daemon::peers",
                                %error,
                                "QUIC accept_bi terminated",
                            );
                            return;
                        }
                    }
                }
            });
        } else {
            drop(conn.connection);
        }
    }

    fn upsert_manual(&self, node_id: String, socket_addr: SocketAddr, source: PeerSource) {
        let mut map = self.inner.peers.write();
        let now = unix_now_ms();
        map.entry(node_id.clone())
            .and_modify(|rec| {
                rec.socket_addr = socket_addr;
                rec.source = source;
                rec.last_seen_unix_ms = now;
            })
            .or_insert(PeerRecord {
                node_id,
                socket_addr,
                source,
                state: PeerConnectionState::Discovered,
                last_seen_unix_ms: now,
                last_synced_unix_ms: 0,
                last_error: None,
                connection: None,
                reconnect_after_unix_ms: 0,
                reconnect_attempts: 0,
            });
    }

    fn update_state(
        &self,
        node_id: &str,
        state: PeerConnectionState,
        error: Option<String>,
    ) {
        let mut map = self.inner.peers.write();
        if let Some(rec) = map.get_mut(node_id) {
            rec.state = state;
            rec.last_seen_unix_ms = unix_now_ms();
            rec.last_error = error;
        }
    }

    fn mark_failed(&self, node_id: &str, error: &str) {
        self.update_state(node_id, PeerConnectionState::Failed, Some(error.to_string()));
    }

    fn snapshot(&self, node_id: &str) -> PeerInfo {
        let map = self.inner.peers.read();
        map.get(node_id)
            .map(PeerRecord::to_proto)
            .unwrap_or_else(|| PeerInfo {
                node_id: node_id.to_string(),
                socket_addr: String::new(),
                source: PeerSource::Unspecified as i32,
                state: PeerConnectionState::Unspecified as i32,
                last_seen_unix_ms: unix_now_ms(),
                last_error: String::from("peer not found in registry"),
                last_synced_unix_ms: 0,
            })
    }

    /// Promote an inbound placeholder record (keyed by socket address)
    /// to the peer's real `node_id` once the sync handshake has
    /// authenticated it. If a record for the real `node_id` already
    /// exists, its connection is replaced by the authenticated inbound
    /// one and the placeholder is removed.
    pub fn rekey_inbound(&self, temp_key: &str, real_node_id: &str) {
        if temp_key == real_node_id {
            return;
        }
        let mut map = self.inner.peers.write();
        let Some(mut record) = map.remove(temp_key) else {
            return;
        };
        record.node_id = real_node_id.to_string();
        record.source = PeerSource::Inbound;
        let now = unix_now_ms();
        match map.entry(real_node_id.to_string()) {
            Entry::Occupied(mut entry) => {
                let existing = entry.get_mut();
                existing.connection = record.connection;
                existing.state = PeerConnectionState::Connected;
                existing.last_seen_unix_ms = now;
                existing.last_error = None;
            }
            Entry::Vacant(entry) => {
                record.last_seen_unix_ms = now;
                record.last_error = None;
                entry.insert(record);
            }
        }
    }

    /// Take the QUIC endpoint out so callers can shut it down during
    /// graceful teardown.
    pub fn take_endpoint(&self) -> Option<QuicEndpoint> {
        self.inner.endpoint.write().take()
    }
}

fn unix_now_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(now).unwrap_or(i64::MAX)
}

/// Drive a set of per-peer re-sync jobs concurrently, bounded by
/// `max_concurrency` and a per-job `per_peer_timeout`. Returns each
/// peer's outcome plus the (virtual-clock-aware) time from round start
/// to that job settling — the timing evidence that a wedged peer does
/// not delay a healthy one.
///
/// The jobs run in a local [`JoinSet`]; dropping this future (e.g. when
/// the worker task is aborted at shutdown) drops the set and cancels
/// every in-flight job.
async fn run_bounded_round(
    jobs: Vec<(String, BoxResyncFuture)>,
    per_peer_timeout: Duration,
    max_concurrency: usize,
) -> Vec<(String, ResyncOutcome, Duration)> {
    let start = tokio::time::Instant::now();
    let sem = Arc::new(Semaphore::new(max_concurrency.max(1)));
    let mut set = JoinSet::new();
    for (node_id, fut) in jobs {
        let sem = Arc::clone(&sem);
        set.spawn(async move {
            let _permit = sem.acquire_owned().await.expect("semaphore not closed");
            let outcome = match tokio::time::timeout(per_peer_timeout, fut).await {
                Ok(outcome) => outcome,
                Err(_) => ResyncOutcome::TimedOut,
            };
            (node_id, outcome, start.elapsed())
        });
    }
    let mut results = Vec::new();
    while let Some(joined) = set.join_next().await {
        match joined {
            Ok(triple) => results.push(triple),
            Err(join_err) if join_err.is_cancelled() => {}
            Err(join_err) => {
                tracing::debug!(
                    target: "rendezvous_daemon::peers",
                    error = %join_err,
                    "re-sync job panicked",
                );
            }
        }
    }
    results
}

/// Whether a sync failure means the underlying QUIC connection is gone
/// (so the peer must leave `Connected`) rather than a transient/protocol
/// issue on a still-live connection. A closed connection is authoritative
/// regardless of which error surfaced; otherwise only the transport-level
/// error variants are treated as fatal.
fn sync_error_is_fatal_to_connection(error: &SyncError, connection: &quinn::Connection) -> bool {
    if connection.close_reason().is_some() {
        return true;
    }
    matches!(
        error,
        SyncError::QuicConnection(_)
            | SyncError::QuicWrite(_)
            | SyncError::QuicRead(_)
            | SyncError::QuicReadExact(_)
    )
}

/// Exponential re-dial backoff: [`RECONNECT_BACKOFF_BASE`] doubled per
/// consecutive failure, capped at [`RECONNECT_BACKOFF_MAX`]. `attempts`
/// is the count of failures so far (0 → base delay).
fn reconnect_backoff(attempts: u32) -> Duration {
    let base = RECONNECT_BACKOFF_BASE.as_secs().max(1);
    let shift = attempts.saturating_sub(1).min(16);
    let secs = base
        .saturating_mul(1u64 << shift)
        .min(RECONNECT_BACKOFF_MAX.as_secs());
    Duration::from_secs(secs)
}

/// Tiny process-local xorshift PRNG used only to spread re-sync ticks
/// across daemons. Seeded from [`std::collections::hash_map::RandomState`]
/// (randomized per process) so two daemons booted together do not lock
/// step. Not cryptographic and deliberately not shared.
struct Jitter {
    state: u64,
}

impl Jitter {
    fn new() -> Self {
        use std::hash::{BuildHasher, Hasher};
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u8(0xA5);
        let seed = hasher.finish();
        Self {
            // Guard against the degenerate all-zero state.
            state: if seed == 0 { 0x9E37_79B9_7F4A_7C15 } else { seed },
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    /// A uniformly-distributed delay in `[0, max)`.
    fn duration_up_to(&mut self, max: Duration) -> Duration {
        let max_ms = u64::try_from(max.as_millis()).unwrap_or(u64::MAX).max(1);
        Duration::from_millis(self.next_u64() % max_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Registry with no QUIC endpoint or sync service — enough to
    /// exercise the state machine (`apply_resync_outcome`,
    /// reconnection bookkeeping) with hand-built records.
    fn test_registry() -> PeerRegistry {
        PeerRegistry {
            inner: Arc::new(Inner {
                peers: RwLock::new(HashMap::new()),
                endpoint: Arc::new(RwLock::new(None)),
                sync_service: RwLock::new(None),
            }),
        }
    }

    /// A `Connected` record with no live connection handle (the handle is
    /// irrelevant to the state-machine transitions under test).
    fn connected_record(node_id: &str) -> PeerRecord {
        PeerRecord {
            node_id: node_id.to_string(),
            socket_addr: "127.0.0.1:9000".parse().expect("addr"),
            source: PeerSource::Manual,
            state: PeerConnectionState::Connected,
            last_seen_unix_ms: unix_now_ms(),
            last_synced_unix_ms: 0,
            last_error: None,
            connection: None,
            reconnect_after_unix_ms: 0,
            reconnect_attempts: 0,
        }
    }

    fn insert(reg: &PeerRegistry, rec: PeerRecord) {
        reg.inner.peers.write().insert(rec.node_id.clone(), rec);
    }

    fn state_of(reg: &PeerRegistry, node_id: &str) -> PeerConnectionState {
        reg.inner.peers.read()[node_id].state
    }

    // A successful round advances the freshness clock; a failed round
    // (soft error OR timeout) leaves it untouched and keeps the peer
    // Connected.
    #[test]
    fn ok_advances_last_synced_while_failures_do_not() {
        let reg = test_registry();
        insert(&reg, connected_record("ok"));
        insert(&reg, connected_record("soft"));
        insert(&reg, connected_record("slow"));

        reg.apply_resync_outcome("ok", ResyncOutcome::Ok);
        reg.apply_resync_outcome("soft", ResyncOutcome::SoftError("boom".into()));
        reg.apply_resync_outcome("slow", ResyncOutcome::TimedOut);

        let map = reg.inner.peers.read();
        assert!(map["ok"].last_synced_unix_ms > 0, "success stamps last_synced");
        assert_eq!(map["soft"].last_synced_unix_ms, 0, "soft error must not stamp");
        assert_eq!(map["slow"].last_synced_unix_ms, 0, "timeout must not stamp");
        assert_eq!(map["soft"].last_error.as_deref(), Some("boom"));
        // A wedged/soft-failing peer stays Connected (the connection is
        // presumed alive); only a dead connection leaves Connected.
        assert_eq!(map["ok"].state, PeerConnectionState::Connected);
        assert_eq!(map["soft"].state, PeerConnectionState::Connected);
        assert_eq!(map["slow"].state, PeerConnectionState::Connected);
    }

    // A dead connection leaves Connected, drops the handle, and arms the
    // exponential-backoff re-dial path; a later success clears it.
    #[test]
    fn dead_connection_disconnects_and_backs_off_redial() {
        let reg = test_registry();
        let mut rec = connected_record("gone");
        rec.last_synced_unix_ms = unix_now_ms();
        insert(&reg, rec);

        reg.apply_resync_outcome("gone", ResyncOutcome::DeadConnection("reset".into()));
        {
            let map = reg.inner.peers.read();
            let r = &map["gone"];
            assert_eq!(r.state, PeerConnectionState::Disconnected);
            assert!(r.connection.is_none(), "dead handle must be dropped");
            assert_eq!(r.last_error.as_deref(), Some("reset"));
        }

        // Reconnect path is exercised: the peer is a re-dial candidate.
        let now = unix_now_ms();
        let targets = reg.collect_reconnect_targets(now + 1);
        assert_eq!(targets.len(), 1, "disconnected peer is a re-dial target");
        assert_eq!(targets[0].0, "gone");

        // A failed re-dial parks it back in Disconnected with a backoff
        // window that hides it from the next tick's candidate set.
        reg.note_reconnect_failure("gone");
        let after_one = reg.inner.peers.read()["gone"].reconnect_after_unix_ms;
        assert!(after_one > now, "backoff pushes the next attempt out");
        assert_eq!(state_of(&reg, "gone"), PeerConnectionState::Disconnected);
        assert!(
            reg.collect_reconnect_targets(now).is_empty(),
            "peer is not a candidate during its backoff window",
        );

        // Backoff grows with successive failures.
        reg.note_reconnect_failure("gone");
        let after_two = reg.inner.peers.read()["gone"].reconnect_after_unix_ms;
        assert!(after_two >= after_one, "backoff must not shrink");

        // A successful sync clears the backoff bookkeeping.
        reg.record_sync_success("gone");
        let map = reg.inner.peers.read();
        assert_eq!(map["gone"].reconnect_attempts, 0);
        assert_eq!(map["gone"].reconnect_after_unix_ms, 0);
    }

    #[test]
    fn reconnect_backoff_grows_and_caps() {
        assert_eq!(reconnect_backoff(0), RECONNECT_BACKOFF_BASE);
        assert_eq!(reconnect_backoff(1), RECONNECT_BACKOFF_BASE);
        assert!(reconnect_backoff(2) > reconnect_backoff(1));
        assert!(reconnect_backoff(3) > reconnect_backoff(2));
        // Deep backoff saturates at the ceiling, never beyond it.
        assert_eq!(reconnect_backoff(1_000), RECONNECT_BACKOFF_MAX);
    }

    // Core regression: one peer wedged for the full per-peer timeout must
    // not delay a concurrently-healthy peer. The wedged job's timeout
    // (200ms here) runs alongside the healthy job's 10ms sync — the
    // healthy peer settles first, an order of magnitude inside the
    // timeout, even though it is listed AFTER the wedged one. (Short real
    // timers rather than paused time, which needs tokio's `test-util`
    // feature; the margin between 10ms and 200ms keeps it deterministic.)
    #[tokio::test]
    async fn wedged_peer_does_not_starve_healthy_peer() {
        let per_peer_timeout = Duration::from_millis(200);
        let jobs: Vec<(String, BoxResyncFuture)> = vec![
            (
                "wedged".to_string(),
                Box::pin(async {
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                    ResyncOutcome::Ok
                }),
            ),
            (
                "healthy".to_string(),
                Box::pin(async {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                    ResyncOutcome::Ok
                }),
            ),
        ];

        let results = run_bounded_round(jobs, per_peer_timeout, RESYNC_MAX_CONCURRENCY).await;

        let healthy = results.iter().find(|r| r.0 == "healthy").expect("healthy");
        let wedged = results.iter().find(|r| r.0 == "wedged").expect("wedged");

        assert!(matches!(healthy.1, ResyncOutcome::Ok));
        assert!(matches!(wedged.1, ResyncOutcome::TimedOut));
        assert!(
            healthy.2 < per_peer_timeout,
            "healthy settled at {:?}, not behind the wedged peer's {per_peer_timeout:?} timeout",
            healthy.2,
        );
        assert!(
            wedged.2 >= per_peer_timeout,
            "wedged peer is bounded by the per-peer timeout",
        );
    }

    // Dropping a round (as the worker's `resync_task.abort()` does at
    // shutdown) must cancel every in-flight per-peer job.
    #[tokio::test]
    async fn aborting_a_round_cancels_in_flight_jobs() {
        let cancelled = Arc::new(AtomicBool::new(false));
        let completed = Arc::new(AtomicBool::new(false));

        struct CancelGuard(Arc<AtomicBool>);
        impl Drop for CancelGuard {
            fn drop(&mut self) {
                self.0.store(true, Ordering::SeqCst);
            }
        }

        let jobs: Vec<(String, BoxResyncFuture)> = {
            let cancelled = Arc::clone(&cancelled);
            let completed = Arc::clone(&completed);
            vec![(
                "wedged".to_string(),
                Box::pin(async move {
                    let _guard = CancelGuard(cancelled);
                    tokio::time::sleep(Duration::from_secs(3600)).await;
                    completed.store(true, Ordering::SeqCst);
                    ResyncOutcome::Ok
                }),
            )]
        };

        // A generous per-peer timeout so the abort — not the timeout — is
        // what ends the job.
        let handle = tokio::spawn(run_bounded_round(
            jobs,
            Duration::from_secs(3600),
            RESYNC_MAX_CONCURRENCY,
        ));
        // Let the job start and park on its sleep.
        tokio::time::sleep(Duration::from_millis(20)).await;
        handle.abort();
        let _ = handle.await;

        // Drive the runtime so the dropped JoinSet reaps its children.
        for _ in 0..100 {
            if cancelled.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(2)).await;
        }

        assert!(
            cancelled.load(Ordering::SeqCst),
            "in-flight job must be dropped when the round is aborted",
        );
        assert!(
            !completed.load(Ordering::SeqCst),
            "aborted job must not run to completion",
        );
    }
}
