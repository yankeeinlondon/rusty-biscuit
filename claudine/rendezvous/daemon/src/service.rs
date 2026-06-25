//! The `Rendezvous` gRPC service implementation.
//!
//! Phase 1 covered `Ping` and `Status`. Phase 2 layers on the
//! session-log RPCs: `AppendEntry`, `ListChunkEntries`,
//! `ListSessionChunks`, and `QueryProjection`. Each RPC is a thin
//! dispatch around the [`SessionLogManager`] plus the DuckDB
//! [`Projection`] handle.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Instant;

use rendezvous_core::{
    AppendEntryRequest, AppendEntryResponse, ApprovePeerRequest, ApprovePeerResponse, ChunkId,
    ConnectToPeerRequest, ConnectToPeerResponse, CreateInvitationRequest,
    CreateInvitationResponse, DAEMON_VERSION, Invitation, ListChunkEntriesRequest,
    ListChunkEntriesResponse, ListPairingsRequest, ListPairingsResponse, ListPeersRequest,
    ListPeersResponse, ListSessionChunksRequest, ListSessionChunksResponse, NodeIdentity,
    PairingInfo, PeerSource, PingRequest, PingResponse, ProjectionRow as ProtoProjectionRow,
    QueryProjectionRequest, QueryProjectionResponse, Rendezvous, RevokePeerRequest,
    RevokePeerResponse, SessionEntry, StatusRequest, StatusResponse,
    SyncChunkOutcome as ProtoSyncChunkOutcome, SyncWithPeerRequest, SyncWithPeerResponse,
};
use tonic::{Request, Response, Status};

use crate::peers::PeerRegistry;
use crate::projection::Projection;
use crate::session_log::SessionLogManager;
use crate::storage::Storage;
use crate::sync::{SyncError, SyncService};

/// In-process gRPC service exposed by the daemon over its UDS transport.
pub struct RendezvousService {
    started_at: Instant,
    started_at_unix_ms: i64,
    session_log: SessionLogManager,
    projection: Projection,
    identity: Arc<NodeIdentity>,
    storage: Storage,
    sync_service: SyncService,
    peers: Option<PeerRegistry>,
    quic_local_addr: Option<SocketAddr>,
}

impl std::fmt::Debug for RendezvousService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RendezvousService")
            .field("started_at_unix_ms", &self.started_at_unix_ms)
            .field("projection", &self.projection)
            .field("quic_local_addr", &self.quic_local_addr)
            .finish_non_exhaustive()
    }
}

impl RendezvousService {
    /// Construct a service wired to the Phase-2 session-log persistence
    /// path. The service does not own the batcher worker — that lives
    /// inside the [`crate::server::ServerHandle`] so it can be shut down
    /// in lockstep with the gRPC server.
    #[must_use]
    pub fn new(
        session_log: SessionLogManager,
        projection: Projection,
        identity: Arc<NodeIdentity>,
        storage: Storage,
        sync_service: SyncService,
    ) -> Self {
        let started_at = Instant::now();
        let started_at_unix_ms = unix_now_ms();
        Self {
            started_at,
            started_at_unix_ms,
            session_log,
            projection,
            identity,
            storage,
            sync_service,
            peers: None,
            quic_local_addr: None,
        }
    }

    /// Attach the Phase-4 peer registry plus the QUIC endpoint's
    /// bound address. Without this attachment the
    /// `CreateInvitation` / `ConnectToPeer` / `ListPeers` RPCs return
    /// `Unavailable` so the daemon can still be exercised without
    /// networking turned on.
    #[must_use]
    pub fn with_peers(mut self, peers: PeerRegistry, quic_local_addr: SocketAddr) -> Self {
        self.peers = Some(peers);
        self.quic_local_addr = Some(quic_local_addr);
        self
    }
}

#[tonic::async_trait]
impl Rendezvous for RendezvousService {
    async fn ping(
        &self,
        request: Request<PingRequest>,
    ) -> Result<Response<PingResponse>, Status> {
        let nonce = request.into_inner().nonce;
        tracing::debug!(nonce = %nonce, "ping received");
        Ok(Response::new(PingResponse {
            nonce,
            daemon_version: DAEMON_VERSION.to_string(),
            timestamp_unix_ms: unix_now_ms(),
        }))
    }

    async fn status(
        &self,
        _request: Request<StatusRequest>,
    ) -> Result<Response<StatusResponse>, Status> {
        let uptime_seconds = i64::try_from(self.started_at.elapsed().as_secs()).unwrap_or(i64::MAX);
        Ok(Response::new(StatusResponse {
            daemon_version: DAEMON_VERSION.to_string(),
            uptime_seconds,
            started_at_unix_ms: self.started_at_unix_ms,
        }))
    }

    async fn append_entry(
        &self,
        request: Request<AppendEntryRequest>,
    ) -> Result<Response<AppendEntryResponse>, Status> {
        let body = request.into_inner();
        let metadata = parse_optional_json(&body.metadata_json)
            .map_err(|err| Status::invalid_argument(format!("metadata_json: {err}")))?;
        let owner_node_id = self.identity.node_id();
        let outcome = tokio::task::spawn_blocking({
            let session_log = self.session_log.clone();
            let session_id = body.session_id;
            let source = body.source;
            let level = body.level;
            let message = body.message;
            move || {
                session_log.append_entry(
                    &owner_node_id,
                    &session_id,
                    source,
                    level,
                    message,
                    metadata,
                )
            }
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(internal)?;
        Ok(Response::new(AppendEntryResponse {
            chunk_id: outcome.chunk.as_path(),
            chunk_index: outcome.chunk.chunk_index,
            sequence: outcome.sequence,
            created_at_unix_ms: outcome.created_at_unix_ms,
            rotated_chunk: outcome.rotated,
        }))
    }

    async fn list_chunk_entries(
        &self,
        request: Request<ListChunkEntriesRequest>,
    ) -> Result<Response<ListChunkEntriesResponse>, Status> {
        let body = request.into_inner();
        let chunk: ChunkId = body
            .chunk_id
            .parse()
            .map_err(|err: rendezvous_core::ChunkIdParseError| {
                Status::invalid_argument(err.to_string())
            })?;
        let chunk_for_closure = chunk.clone();
        let entries = tokio::task::spawn_blocking({
            let session_log = self.session_log.clone();
            move || session_log.list_chunk_entries(&chunk_for_closure)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(internal)?;
        let proto_entries = entries
            .into_iter()
            .map(|e| {
                let metadata_json = e
                    .metadata
                    .as_ref()
                    .map(serde_json::to_string)
                    .transpose()
                    .unwrap_or(None)
                    .unwrap_or_default();
                SessionEntry {
                    sequence: e.sequence,
                    created_at_unix_ms: e.created_at_unix_ms,
                    source: e.source,
                    level: e.level,
                    message: e.message,
                    metadata_json,
                }
            })
            .collect();
        Ok(Response::new(ListChunkEntriesResponse {
            chunk_id: chunk.as_path(),
            entries: proto_entries,
        }))
    }

    async fn list_session_chunks(
        &self,
        request: Request<ListSessionChunksRequest>,
    ) -> Result<Response<ListSessionChunksResponse>, Status> {
        let body = request.into_inner();
        let chunks = tokio::task::spawn_blocking({
            let session_log = self.session_log.clone();
            let owner = body.owner_node_id;
            let session = body.session_id;
            move || session_log.list_session_chunks(&owner, &session)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(internal)?;
        Ok(Response::new(ListSessionChunksResponse {
            chunk_ids: chunks.into_iter().map(|c| c.as_path()).collect(),
        }))
    }

    async fn create_invitation(
        &self,
        request: Request<CreateInvitationRequest>,
    ) -> Result<Response<CreateInvitationResponse>, Status> {
        let local_addr = self
            .quic_local_addr
            .ok_or_else(|| Status::unavailable("QUIC endpoint not enabled"))?;
        let body = request.into_inner();
        let socket_addr = if body.advertise_addr.is_empty() {
            advertised_socket_addr(local_addr)
        } else {
            body.advertise_addr
                .parse::<SocketAddr>()
                .map_err(|err| {
                    Status::invalid_argument(format!("advertise_addr is not a valid SocketAddr: {err}"))
                })?
        };
        let invitation = Invitation::new(self.identity.as_ref(), socket_addr);
        Ok(Response::new(CreateInvitationResponse {
            invitation: invitation.encode(),
            node_id: invitation.node_id(),
            socket_addr: invitation.socket_addr.to_string(),
        }))
    }

    async fn connect_to_peer(
        &self,
        request: Request<ConnectToPeerRequest>,
    ) -> Result<Response<ConnectToPeerResponse>, Status> {
        let peers = self
            .peers
            .as_ref()
            .ok_or_else(|| Status::unavailable("peer registry not enabled"))?;
        let body = request.into_inner();
        let invitation: Invitation = body
            .invitation
            .parse()
            .map_err(|err: rendezvous_core::InvitationError| {
                Status::invalid_argument(err.to_string())
            })?;
        let node_id = invitation.node_id();
        match peers
            .connect(node_id.clone(), invitation.socket_addr, PeerSource::Manual)
            .await
        {
            Ok(peer) => {
                Ok(Response::new(ConnectToPeerResponse { peer: Some(peer) }))
            }
            Err(error) => Err(Status::unavailable(format!(
                "failed to connect to peer {node_id}: {error}"
            ))),
        }
    }

    async fn list_peers(
        &self,
        _request: Request<ListPeersRequest>,
    ) -> Result<Response<ListPeersResponse>, Status> {
        let peers = match &self.peers {
            Some(registry) => registry.list(),
            None => Vec::new(),
        };
        Ok(Response::new(ListPeersResponse { peers }))
    }

    async fn approve_peer(
        &self,
        request: Request<ApprovePeerRequest>,
    ) -> Result<Response<ApprovePeerResponse>, Status> {
        let body = request.into_inner();
        let node_id = normalize_node_id(&body.node_id)?;
        let now = unix_now_ms();
        let node_id_for_closure = node_id.clone();
        tokio::task::spawn_blocking({
            let storage = self.storage.clone();
            let note = body.note.clone();
            move || storage.upsert_pairing(&node_id_for_closure, now, &note)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(internal)?;
        Ok(Response::new(ApprovePeerResponse {
            pairing: Some(PairingInfo {
                node_id,
                paired_at_unix_ms: now,
                note: body.note,
            }),
        }))
    }

    async fn revoke_peer(
        &self,
        request: Request<RevokePeerRequest>,
    ) -> Result<Response<RevokePeerResponse>, Status> {
        let body = request.into_inner();
        let node_id = normalize_node_id(&body.node_id)?;
        let removed = tokio::task::spawn_blocking({
            let storage = self.storage.clone();
            move || storage.remove_pairing(&node_id)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(internal)?;
        Ok(Response::new(RevokePeerResponse { removed }))
    }

    async fn list_pairings(
        &self,
        _request: Request<ListPairingsRequest>,
    ) -> Result<Response<ListPairingsResponse>, Status> {
        let listed = tokio::task::spawn_blocking({
            let storage = self.storage.clone();
            move || storage.list_pairings()
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(internal)?;
        let pairings = listed
            .into_iter()
            .map(|(node_id, value)| PairingInfo {
                node_id,
                paired_at_unix_ms: value.paired_at_unix_ms,
                note: value.note,
            })
            .collect();
        Ok(Response::new(ListPairingsResponse { pairings }))
    }

    async fn sync_with_peer(
        &self,
        request: Request<SyncWithPeerRequest>,
    ) -> Result<Response<SyncWithPeerResponse>, Status> {
        let peers = self
            .peers
            .as_ref()
            .ok_or_else(|| Status::unavailable("peer registry not enabled"))?;
        let body = request.into_inner();
        let node_id = normalize_node_id(&body.node_id)?;
        let connection = peers
            .connection_for(&node_id)
            .ok_or_else(|| Status::failed_precondition(format!(
                "no active QUIC connection to peer {node_id}; call ConnectToPeer first",
            )))?;
        let outcome = self
            .sync_service
            .sync_initiator(&connection, &node_id)
            .await
            .map_err(map_sync_error)?;
        let chunks = outcome
            .chunks
            .iter()
            .map(|c| ProtoSyncChunkOutcome {
                chunk_id: c.chunk_id.clone(),
                received_bytes: c.received_bytes,
                sent_bytes: c.sent_bytes,
                advanced: c.advanced,
            })
            .collect();
        Ok(Response::new(SyncWithPeerResponse {
            node_id,
            received_bytes: outcome.received_bytes,
            sent_bytes: outcome.sent_bytes,
            chunks,
        }))
    }

    async fn query_projection(
        &self,
        request: Request<QueryProjectionRequest>,
    ) -> Result<Response<QueryProjectionResponse>, Status> {
        let body = request.into_inner();
        let owner_node_id = body.owner_node_id;
        let session_id = body.session_id;
        let rows = tokio::task::spawn_blocking({
            let projection = self.projection.clone();
            move || projection.entries_for_session(&owner_node_id, &session_id)
        })
        .await
        .map_err(|e| Status::internal(e.to_string()))?
        .map_err(internal)?;
        let proto_rows = rows
            .into_iter()
            .map(|row| ProtoProjectionRow {
                chunk_id: row.chunk.as_path(),
                chunk_index: row.chunk.chunk_index,
                sequence: row.sequence,
                created_at_unix_ms: row.created_at_unix_ms,
                source: row.source,
                level: row.level,
                message: row.message,
            })
            .collect();
        Ok(Response::new(QueryProjectionResponse { rows: proto_rows }))
    }
}

fn parse_optional_json(raw: &str) -> Result<Option<serde_json::Value>, serde_json::Error> {
    if raw.is_empty() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(raw)?))
}

fn internal<E: std::fmt::Display>(error: E) -> Status {
    Status::internal(error.to_string())
}

fn normalize_node_id(raw: &str) -> Result<String, Status> {
    let trimmed = raw.trim().to_ascii_lowercase();
    if trimmed.len() != 64 || !trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Status::invalid_argument(
            "node_id must be 64 lowercase hex characters",
        ));
    }
    Ok(trimmed)
}

fn map_sync_error(error: SyncError) -> Status {
    match &error {
        SyncError::NotPaired(_) => Status::failed_precondition(error.to_string()),
        SyncError::Timeout(_) => Status::deadline_exceeded(error.to_string()),
        SyncError::MalformedPayload { .. } => Status::data_loss(error.to_string()),
        _ => Status::internal(error.to_string()),
    }
}

/// Substitute the wildcard binding address (`0.0.0.0` / `[::]`) with a
/// concrete loopback address so the invitation can be passed to a
/// daemon on the same host without further editing.
fn advertised_socket_addr(local: SocketAddr) -> SocketAddr {
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    let ip = match local.ip() {
        IpAddr::V4(ip) if ip.is_unspecified() => IpAddr::V4(Ipv4Addr::LOCALHOST),
        IpAddr::V6(ip) if ip.is_unspecified() => IpAddr::V6(Ipv6Addr::LOCALHOST),
        other => other,
    };
    SocketAddr::new(ip, local.port())
}

fn unix_now_ms() -> i64 {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(now).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batcher::{BatcherConfig, BatcherWorker, spawn};
    use crate::projection::Projection;
    use crate::session_log::SessionLogManager;
    use crate::storage::Storage;
    use rendezvous_core::{ChunkConfig, NodeIdentity};
    use std::sync::Arc;
    use std::time::Duration;
    use tempfile::TempDir;

    struct Harness {
        service: RendezvousService,
        _worker: BatcherWorker,
        _tmp: TempDir,
    }

    fn harness() -> Harness {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection.clone(), BatcherConfig {
            flush_interval: Duration::from_millis(20),
            flush_size: 16,
        });
        let identity = Arc::new(NodeIdentity::from_seed([5u8; 32]));
        let session_log = SessionLogManager::new(
            storage.clone(),
            worker.handle(),
            projection.clone(),
            ChunkConfig::default(),
            Arc::clone(&identity),
        )
        .expect("mgr");
        let sync_service = SyncService::new(
            session_log.clone(),
            storage.clone(),
            Arc::clone(&identity),
        );
        let service = RendezvousService::new(
            session_log,
            projection,
            identity,
            storage,
            sync_service,
        );
        Harness {
            service,
            _worker: worker,
            _tmp: tmp,
        }
    }

    #[tokio::test]
    async fn ping_echoes_nonce_and_reports_version() {
        let h = harness();
        let response = h
            .service
            .ping(Request::new(PingRequest {
                nonce: "abc-123".into(),
            }))
            .await
            .expect("ping ok");
        let body = response.into_inner();
        assert_eq!(body.nonce, "abc-123");
        assert_eq!(body.daemon_version, DAEMON_VERSION);
        assert!(body.timestamp_unix_ms > 0);
    }

    #[tokio::test]
    async fn append_then_list_chunk_entries() {
        let h = harness();
        let node_id = h.service.identity.node_id();
        h.service
            .append_entry(Request::new(AppendEntryRequest {
                owner_node_id: String::new(),
                session_id: "s-1".into(),
                source: "test".into(),
                level: "info".into(),
                message: "hello".into(),
                metadata_json: String::new(),
            }))
            .await
            .expect("append");
        let chunk_id = format!("session/{node_id}/s-1/part/0");
        let listed = h
            .service
            .list_chunk_entries(Request::new(ListChunkEntriesRequest {
                chunk_id: chunk_id.clone(),
            }))
            .await
            .expect("list")
            .into_inner();
        assert_eq!(listed.entries.len(), 1);
        assert_eq!(listed.entries[0].message, "hello");
    }
}
