//! In-memory peer registry plus the orchestration glue that ties the
//! QUIC endpoint and mDNS discovery together.
//!
//! The registry indexes peers by hex-encoded `node_id`. When mDNS
//! resolves a service or a manual invitation is decoded, the resulting
//! [`PeerRecord`] lands in the registry; a follow-up
//! [`PeerRegistry::connect`] (or an inbound handshake) flips its
//! connection state.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use parking_lot::RwLock;
use remote_signal_core::{PeerConnectionState, PeerInfo, PeerSource};
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use crate::discovery::DiscoveredPeer;
use crate::quic::{InboundConnection, QuicError, QuicEndpoint};
use crate::sync::SyncService;

/// Snapshot of what the daemon knows about a single peer.
#[derive(Clone, Debug)]
pub struct PeerRecord {
    pub node_id: String,
    pub socket_addr: SocketAddr,
    pub source: PeerSource,
    pub state: PeerConnectionState,
    pub last_seen_unix_ms: i64,
    pub last_error: Option<String>,
    /// Active QUIC connection (when one is open). Held so the daemon
    /// can drive outbound sync sessions to this peer without
    /// re-dialling. Not exposed in [`PeerRecord::to_proto`].
    pub connection: Option<quinn::Connection>,
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
            last_error: None,
            connection: None,
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
}

impl PeerRegistryWorkers {
    /// Abort both background workers. Safe to call multiple times.
    pub fn shutdown(&mut self) {
        if let Some(task) = self.discovery_task.take() {
            task.abort();
        }
        if let Some(task) = self.inbound_task.take() {
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

        (
            registry,
            PeerRegistryWorkers {
                discovery_task,
                inbound_task,
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

        let connect_future = endpoint.connect(socket_addr, "remote-signal");
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
                                tracing::info!(
                                    target: "remote_signal_daemon::peers",
                                    peer = %node_for_sync,
                                    received_bytes = outcome.received_bytes,
                                    sent_bytes = outcome.sent_bytes,
                                    "initial sync completed",
                                );
                            }
                            Err(error) => {
                                tracing::debug!(
                                    target: "remote_signal_daemon::peers",
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
        let mut map = self.inner.peers.write();
        let key = format!("inbound:{}", conn.remote_addr);
        let now = unix_now_ms();
        let entry = map.entry(key.clone()).or_insert_with(|| PeerRecord {
            node_id: key.clone(),
            socket_addr: conn.remote_addr,
            source: PeerSource::Inbound,
            state: PeerConnectionState::Connected,
            last_seen_unix_ms: now,
            last_error: None,
            connection: None,
        });
        entry.socket_addr = conn.remote_addr;
        entry.state = PeerConnectionState::Connected;
        entry.last_seen_unix_ms = now;
        entry.last_error = None;
        entry.connection = Some(conn.connection.clone());
        drop(map);

        // Spawn a sync-responder loop that drains any bidirectional
        // streams the peer opens for sync sessions. If sync is not
        // configured, drop the connection.
        let sync_service = self.inner.sync_service.read().clone();
        if let Some(service) = sync_service {
            let connection = conn.connection.clone();
            tokio::spawn(async move {
                loop {
                    match connection.accept_bi().await {
                        Ok((send, recv)) => {
                            let service = service.clone();
                            tokio::spawn(async move {
                                if let Err(error) = service.sync_responder(send, recv).await {
                                    tracing::debug!(
                                        target: "remote_signal_daemon::peers",
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
                                target: "remote_signal_daemon::peers",
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
                last_error: None,
                connection: None,
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
            })
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
