//! Phase-5 direct-sync engine.
//!
//! Two paired daemons exchange Loro state vectors over a QUIC
//! bidirectional stream and push the deltas the other side is missing.
//! The protocol is symmetric — both peers send their advertisements
//! before reading any responses — so a single round-trip is enough to
//! converge.
//!
//! Frame layout is defined in [`remote_signal_core::sync`] (a
//! length-prefixed `SyncFrame` protobuf per message). Pairing is
//! enforced at the start of every session: peers whose `node_id` is
//! not present in the local pairings table are rejected before any
//! session-log data is touched.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use parking_lot::Mutex;
use prost::Message;
use quinn::{Connection, RecvStream, SendStream};
use remote_signal_core::sync::{SyncWireError, encode_frame};
use remote_signal_core::{
    ChunkId, ChunkIdParseError, EnvelopeError, EnvelopeInbox, EnvelopeSealer, NodeIdentity,
    PayloadKind, SignedEnvelopeWire, SyncAdvertiseEnd, SyncChunkAdvertise,
    SyncDelta, SyncEnd, SyncFrame, SyncHello, identity::PUBLIC_KEY_LENGTH, sync_frame,
};

use crate::session_log::{SessionLogError, SessionLogManager};
use crate::storage::{AcceptedEnvelope, Storage};

/// Wire-level protocol version advertised in [`SyncHello`].
pub const SYNC_PROTOCOL_VERSION: u32 = 1;

/// Maximum time the sync engine waits for a full session to finish.
/// Phase 5 always runs in-process or LAN-local so a generous default
/// is fine; later phases will refine this.
pub const SYNC_OVERALL_TIMEOUT: Duration = Duration::from_secs(30);

/// Per-chunk outcome of a sync run, mirrored onto the gRPC response.
#[derive(Clone, Debug, Default)]
pub struct SyncChunkOutcome {
    pub chunk_id: String,
    pub received_bytes: u64,
    pub sent_bytes: u64,
    pub advanced: bool,
}

/// Aggregated outcome of a sync run.
#[derive(Clone, Debug, Default)]
pub struct SyncOutcome {
    pub received_bytes: u64,
    pub sent_bytes: u64,
    pub chunks: Vec<SyncChunkOutcome>,
}

/// Errors surfaced by the sync engine.
#[derive(Debug, thiserror::Error)]
pub enum SyncError {
    /// The remote `node_id` is not in our pairings table.
    #[error("peer {0} is not paired with this daemon")]
    NotPaired(String),

    /// The remote claimed an unexpected version in its `SyncHello`.
    #[error("unsupported sync protocol version: {0}")]
    UnsupportedVersion(u32),

    /// The remote `node_id` did not match the value advertised on
    /// connection (e.g. the QUIC peer claimed a different identity).
    #[error("sync hello node_id mismatch: expected {expected}, got {actual}")]
    HelloNodeIdMismatch { expected: String, actual: String },

    /// The envelope's sender is not the owner of the document it
    /// targets. Remote peers may only write documents in their own
    /// namespace.
    #[error("ownership violation: sender {sender} attempted to write document owned by {owner}")]
    OwnershipViolation { sender: String, owner: String },

    /// The remote sent an envelope whose signature could not be
    /// verified.
    #[error("envelope verification failed: {0}")]
    Envelope(#[from] EnvelopeError),

    /// A chunk-id string sent by the remote was malformed.
    #[error("malformed chunk id: {0}")]
    ChunkIdParse(#[from] ChunkIdParseError),

    /// Local session-log persistence failed.
    #[error(transparent)]
    SessionLog(#[from] SessionLogError),

    /// Pairing-store access failed.
    #[error(transparent)]
    Storage(#[from] crate::storage::StorageError),

    /// Underlying QUIC stream I/O failed.
    #[error("QUIC stream error: {0}")]
    QuicWrite(#[from] quinn::WriteError),

    /// Underlying QUIC read failed.
    #[error("QUIC read error: {0}")]
    QuicRead(#[from] quinn::ReadError),

    /// Underlying QUIC stream gave up mid-frame.
    #[error("QUIC read exact error: {0}")]
    QuicReadExact(#[from] quinn::ReadExactError),

    /// Establishing a fresh bidi stream failed.
    #[error("QUIC connection error: {0}")]
    QuicConnection(#[from] quinn::ConnectionError),

    /// The wire-format decode/encode layer failed.
    #[error("sync wire error: {0}")]
    Wire(#[from] SyncWireError),

    /// The peer sent the wrong frame at the wrong time, or terminated
    /// the stream prematurely.
    #[error("sync protocol error: {0}")]
    Protocol(String),

    /// The session ran past [`SYNC_OVERALL_TIMEOUT`].
    #[error("sync session timed out after {0:?}")]
    Timeout(Duration),

    /// The envelope payload could not be imported into the local CRDT
    /// state (structurally invalid Loro bytes).
    #[error("malformed CRDT payload for chunk {chunk_id}: {reason}")]
    MalformedPayload { chunk_id: String, reason: String },
}

impl SyncError {
    fn protocol<S: Into<String>>(reason: S) -> Self {
        Self::Protocol(reason.into())
    }
}

/// Service responsible for driving a sync session against one paired
/// peer. Holds shared handles on the session log, the pairings store,
/// and the local identity so both inbound and outbound runs use the
/// exact same code path. The sealer is shared from
/// [`SessionLogManager`](crate::session_log::SessionLogManager) so
/// all outbound message IDs are monotonic across the daemon lifetime.
#[derive(Clone)]
pub struct SyncService {
    session_log: SessionLogManager,
    storage: Storage,
    identity: Arc<NodeIdentity>,
    sealer: Arc<Mutex<EnvelopeSealer>>,
}

impl SyncService {
    /// Build a new sync service from the running daemon's handles.
    /// The sealer is obtained from the session-log manager so both code
    /// paths share the same monotonic counter.
    #[must_use]
    pub fn new(
        session_log: SessionLogManager,
        storage: Storage,
        identity: Arc<NodeIdentity>,
    ) -> Self {
        let sealer = session_log.sealer();
        Self {
            session_log,
            storage,
            identity,
            sealer,
        }
    }

    #[must_use]
    pub fn node_id(&self) -> String {
        self.identity.node_id()
    }

    /// Drive a sync session as the initiator. Opens a fresh bidi
    /// stream on `connection`, exchanges hello frames with the peer
    /// (whose expected `node_id` was learned out-of-band), then runs
    /// the advertise + delta phases in both directions.
    pub async fn sync_initiator(
        &self,
        connection: &Connection,
        expected_peer_node_id: &str,
    ) -> Result<SyncOutcome, SyncError> {
        let (send, recv) = connection.open_bi().await?;
        self.run_session(send, recv, Some(expected_peer_node_id.to_string())).await
    }

    /// Drive a sync session as the responder. Accepts a bidi stream
    /// from `connection` (the caller has already received the bi
    /// pair) and runs the same protocol the initiator does. The
    /// responder does not know the peer's `node_id` up front — it is
    /// learned from the incoming `SyncHello` and verified against the
    /// local pairings table.
    pub async fn sync_responder(
        &self,
        send: SendStream,
        recv: RecvStream,
    ) -> Result<SyncOutcome, SyncError> {
        self.run_session(send, recv, None).await
    }

    fn assert_paired(&self, peer_node_id: &str) -> Result<(), SyncError> {
        if self.storage.get_pairing(peer_node_id)?.is_none() {
            return Err(SyncError::NotPaired(peer_node_id.to_string()));
        }
        Ok(())
    }

    /// Validate, apply, and persist a single incoming delta envelope.
    ///
    /// The CRDT payload is staged in memory and validated against the
    /// session-log schema, then the accepted envelope is persisted to
    /// redb, and finally the staged snapshot is committed. This ordering
    /// ensures that if envelope persistence fails, no snapshot exists on
    /// disk, and if snapshot persistence fails after the envelope was
    /// saved, startup replay recovers from the persisted envelope.
    ///
    /// Returns `true` when the local chunk state advanced.
    pub(crate) fn receive_delta(
        &self,
        peer_node_id_hex: &str,
        chunk: &ChunkId,
        delta_chunk_id: &str,
        envelope: &remote_signal_core::SignedEnvelope,
        is_snapshot: bool,
        inbox: &mut EnvelopeInbox,
    ) -> Result<bool, SyncError> {
        if hex_encode(&envelope.sender) != peer_node_id_hex {
            return Err(SyncError::protocol(
                "delta envelope sender does not match hello node_id",
            ));
        }
        if envelope.document_id != delta_chunk_id {
            return Err(SyncError::protocol(format!(
                "envelope document_id {:?} does not match delta chunk_id {:?}",
                envelope.document_id, delta_chunk_id,
            )));
        }
        if chunk.owner_node_id != peer_node_id_hex {
            return Err(SyncError::OwnershipViolation {
                sender: peer_node_id_hex.to_string(),
                owner: chunk.owner_node_id.clone(),
            });
        }
        let expected_kind = if is_snapshot {
            PayloadKind::Snapshot
        } else {
            PayloadKind::Delta
        };
        if envelope.payload_kind != expected_kind {
            return Err(SyncError::protocol(format!(
                "envelope payload_kind {:?} does not match delta is_snapshot {:?}",
                envelope.payload_kind, is_snapshot,
            )));
        }
        let msg_id_hex = envelope.message_id_hex();
        let sender_hex = envelope.sender_node_id();
        if self.storage.has_accepted_envelope(&sender_hex, &msg_id_hex)? {
            return Err(SyncError::Envelope(
                EnvelopeError::DuplicateMessageId(msg_id_hex),
            ));
        }
        let payload = inbox.accept(envelope)?.to_vec();

        // Stage the import and validate schema WITHOUT persisting.
        let staged = match self.session_log.stage_remote_update(chunk, &payload) {
            Ok(s) => s,
            Err(crate::session_log::SessionLogError::Loro(reason)) => {
                return Err(SyncError::MalformedPayload {
                    chunk_id: chunk.as_path(),
                    reason,
                });
            }
            Err(crate::session_log::SessionLogError::SchemaValidation { reason }) => {
                return Err(SyncError::MalformedPayload {
                    chunk_id: chunk.as_path(),
                    reason,
                });
            }
            Err(other) => return Err(SyncError::SessionLog(other)),
        };

        // Persist the accepted envelope BEFORE the snapshot so a failure
        // here leaves no durable snapshot on disk.
        let now_unix_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        let accepted = AcceptedEnvelope {
            sender_hex: sender_hex.clone(),
            message_id_hex: msg_id_hex.clone(),
            document_id: envelope.document_id.clone(),
            payload_kind: envelope.payload_kind.to_byte(),
            content_hash_hex: hex_encode(&envelope.content_hash),
            signature_hex: hex_encode(&envelope.signature),
            payload_bytes: payload,
            accepted_at_unix_ms: i64::try_from(now_unix_ms).unwrap_or(i64::MAX),
        };
        self.storage.save_accepted_envelope(
            &sender_hex,
            &msg_id_hex,
            &accepted,
        )?;

        // Now commit the staged snapshot. If this fails after the envelope
        // was persisted, startup replay will recover.
        let advanced = self.session_log.commit_staged_update(chunk, staged)?;

        if advanced {
            self.session_log.submit_chunk_to_projection(chunk)?;
        }

        Ok(advanced)
    }

    async fn run_session(
        &self,
        mut send: SendStream,
        mut recv: RecvStream,
        expected_peer_node_id: Option<String>,
    ) -> Result<SyncOutcome, SyncError> {
        let work = async {
            let mut outcome = SyncOutcome::default();

            // ---- Hello exchange ----------------------------------
            let hello_frame = SyncFrame {
                kind: Some(sync_frame::Kind::Hello(SyncHello {
                    node_id: self.identity.public_key_bytes().to_vec(),
                    version: SYNC_PROTOCOL_VERSION,
                })),
            };
            let sent = write_frame(&mut send, &hello_frame).await?;
            outcome.sent_bytes += sent;

            let (peer_hello, received) = read_hello(&mut recv).await?;
            outcome.received_bytes += received;
            if peer_hello.version != SYNC_PROTOCOL_VERSION {
                return Err(SyncError::UnsupportedVersion(peer_hello.version));
            }
            if peer_hello.node_id.len() != PUBLIC_KEY_LENGTH {
                return Err(SyncError::protocol(format!(
                    "hello node_id length {} != {PUBLIC_KEY_LENGTH}",
                    peer_hello.node_id.len(),
                )));
            }
            let peer_node_id_hex = hex_encode(&peer_hello.node_id);
            if let Some(expected) = expected_peer_node_id.as_deref()
                && expected != peer_node_id_hex
            {
                return Err(SyncError::HelloNodeIdMismatch {
                    expected: expected.to_string(),
                    actual: peer_node_id_hex,
                });
            }
            if expected_peer_node_id.is_some() {
                // Initiator path: the peer's identity has been confirmed
                // by the hello matching the expected node_id (from the
                // invitation). Auto-pair if not already paired so that
                // invitation-based connections do not require a separate
                // explicit approval step on the initiator side.
                if self.storage.get_pairing(&peer_node_id_hex)?.is_none() {
                    let now_unix_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    self.storage.upsert_pairing(
                        &peer_node_id_hex,
                        i64::try_from(now_unix_ms).unwrap_or(i64::MAX),
                        "auto-paired after identity confirmation",
                    )?;
                }
            } else {
                // Responder path: require pre-existing pairing.
                self.assert_paired(&peer_node_id_hex)?;
            }

            // ---- Advertise phase (we send our own-namespace state) --
            let local_node_id = self.identity.node_id();
            let local_chunks = self.session_log.list_all_chunks()?;
            let mut chunk_outcomes: HashMap<String, SyncChunkOutcome> = HashMap::new();
            for chunk in &local_chunks {
                if chunk.owner_node_id != local_node_id {
                    continue;
                }
                let vv = self.session_log.chunk_state_vector(chunk)?.unwrap_or_default();
                let frame = SyncFrame {
                    kind: Some(sync_frame::Kind::Advertise(SyncChunkAdvertise {
                        chunk_id: chunk.as_path(),
                        version_vector: vv,
                    })),
                };
                let sent = write_frame(&mut send, &frame).await?;
                outcome.sent_bytes += sent;
                chunk_outcomes.entry(chunk.as_path()).or_insert(SyncChunkOutcome {
                    chunk_id: chunk.as_path(),
                    ..Default::default()
                });
            }
            let sent = write_frame(
                &mut send,
                &SyncFrame {
                    kind: Some(sync_frame::Kind::AdvertiseEnd(SyncAdvertiseEnd {})),
                },
            )
            .await?;
            outcome.sent_bytes += sent;

            // ---- Read remote advertisements (only their namespace) --
            let mut remote_state: HashMap<ChunkId, Vec<u8>> = HashMap::new();
            loop {
                let (frame, received) = read_frame(&mut recv).await?;
                outcome.received_bytes += received;
                match frame.kind {
                    Some(sync_frame::Kind::Advertise(ad)) => {
                        let chunk: ChunkId = ad.chunk_id.parse()?;
                        if chunk.owner_node_id == peer_node_id_hex {
                            remote_state.insert(chunk, ad.version_vector);
                        }
                    }
                    Some(sync_frame::Kind::AdvertiseEnd(_)) => break,
                    Some(sync_frame::Kind::Error(err)) => {
                        return Err(SyncError::protocol(format!(
                            "peer error {} during advertise: {}",
                            err.code, err.message
                        )));
                    }
                    other => {
                        return Err(SyncError::protocol(format!(
                            "unexpected frame during advertise phase: {other:?}",
                        )));
                    }
                }
            }

            // ---- Push deltas the remote is missing ---------------
            // Only push deltas for chunks WE own. The remote will push
            // their own namespace deltas to us in the reading phase.
            for chunk in local_chunks.iter().filter(|c| c.owner_node_id == local_node_id) {
                let remote_vv = remote_state.get(chunk).and_then(|v| {
                    if v.is_empty() { None } else { Some(v.as_slice()) }
                });
                let Some(exported) = self.session_log.export_updates_since(chunk, remote_vv)?
                else {
                    continue;
                };
                if exported.bytes.is_empty() {
                    continue;
                }
                let document_id = chunk.as_path();
                let payload_kind = if exported.is_snapshot {
                    PayloadKind::Snapshot
                } else {
                    PayloadKind::Delta
                };
                let envelope = self.sealer.lock().seal(
                    document_id.clone(),
                    payload_kind,
                    exported.bytes,
                );
                self.storage.save_outbound_counter(
                    &self.identity.node_id(),
                    self.sealer.lock().next_counter(),
                )?;
                let frame = SyncFrame {
                    kind: Some(sync_frame::Kind::Delta(SyncDelta {
                        chunk_id: document_id.clone(),
                        envelope: Some(SignedEnvelopeWire::from_envelope(&envelope)),
                        is_snapshot: exported.is_snapshot,
                    })),
                };
                let sent = write_frame(&mut send, &frame).await?;
                outcome.sent_bytes += sent;
                if let Some(o) = chunk_outcomes.get_mut(&chunk.as_path()) {
                    o.sent_bytes += sent;
                } else {
                    chunk_outcomes.insert(chunk.as_path(), SyncChunkOutcome {
                        chunk_id: chunk.as_path(),
                        sent_bytes: sent,
                        ..Default::default()
                    });
                }
            }
            let sent = write_frame(
                &mut send,
                &SyncFrame {
                    kind: Some(sync_frame::Kind::End(SyncEnd {})),
                },
            )
            .await?;
            outcome.sent_bytes += sent;
            send.finish().ok();

            // ---- Read remote deltas and apply them ---------------
            let mut inbox = EnvelopeInbox::new();
            loop {
                let (frame, received) = read_frame(&mut recv).await?;
                outcome.received_bytes += received;
                match frame.kind {
                    Some(sync_frame::Kind::Delta(delta)) => {
                        let chunk: ChunkId = delta.chunk_id.parse()?;
                        let wire = delta
                            .envelope
                            .ok_or_else(|| SyncError::protocol("delta missing envelope"))?;
                        let envelope = wire.into_envelope()?;
                        let advanced = self.receive_delta(
                            &peer_node_id_hex,
                            &chunk,
                            &delta.chunk_id,
                            &envelope,
                            delta.is_snapshot,
                            &mut inbox,
                        )?;
                        let entry = chunk_outcomes.entry(chunk.as_path()).or_insert(
                            SyncChunkOutcome {
                                chunk_id: chunk.as_path(),
                                ..Default::default()
                            },
                        );
                        entry.received_bytes += received;
                        entry.advanced = entry.advanced || advanced;
                    }
                    Some(sync_frame::Kind::End(_)) => break,
                    Some(sync_frame::Kind::Error(err)) => {
                        return Err(SyncError::protocol(format!(
                            "peer error {} during delta phase: {}",
                            err.code, err.message
                        )));
                    }
                    other => {
                        return Err(SyncError::protocol(format!(
                            "unexpected frame during delta phase: {other:?}",
                        )));
                    }
                }
            }

            // ---- Assemble result --------------------------------
            let mut chunks: Vec<SyncChunkOutcome> = chunk_outcomes.into_values().collect();
            chunks.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
            outcome.chunks = chunks;
            Ok(outcome)
        };

        match tokio::time::timeout(SYNC_OVERALL_TIMEOUT, work).await {
            Ok(res) => res,
            Err(_) => Err(SyncError::Timeout(SYNC_OVERALL_TIMEOUT)),
        }
    }
}

async fn write_frame(send: &mut SendStream, frame: &SyncFrame) -> Result<u64, SyncError> {
    let bytes = encode_frame(frame);
    let len = bytes.len() as u64;
    send.write_all(&bytes).await?;
    Ok(len)
}

async fn read_hello(recv: &mut RecvStream) -> Result<(SyncHello, u64), SyncError> {
    let (frame, received) = read_frame(recv).await?;
    match frame.kind {
        Some(sync_frame::Kind::Hello(hello)) => Ok((hello, received)),
        Some(sync_frame::Kind::Error(err)) => Err(SyncError::protocol(format!(
            "peer error during hello: {} ({})",
            err.message, err.code
        ))),
        other => Err(SyncError::protocol(format!(
            "expected SyncHello as first frame, got {other:?}",
        ))),
    }
}

async fn read_frame(recv: &mut RecvStream) -> Result<(SyncFrame, u64), SyncError> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf).await?;
    let len = u32::from_be_bytes(len_buf);
    if len > remote_signal_core::sync::MAX_FRAME_LEN {
        return Err(SyncError::Wire(SyncWireError::FrameTooLarge(len)));
    }
    let mut buf = vec![0u8; len as usize];
    recv.read_exact(&mut buf).await?;
    let frame = SyncFrame::decode(buf.as_slice()).map_err(SyncWireError::from)?;
    Ok((frame, 4 + len as u64))
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::batcher::{BatcherConfig, BatcherWorker, spawn};
    use crate::projection::Projection;
    use crate::session_log::Clock;
    use loro::{ExportMode, LoroDoc};
    use remote_signal_core::{
        ChunkConfig, ChunkId, EnvelopeSealer, Entry, NodeIdentity, PayloadKind,
    };

    const ENTRIES_CONTAINER: &str = "entries";
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;
    use tempfile::TempDir;

    struct FixedClock {
        next: AtomicI64,
    }

    impl FixedClock {
        fn new(start: i64) -> Self {
            Self {
                next: AtomicI64::new(start),
            }
        }
    }

    impl Clock for FixedClock {
        fn now_unix_ms(&self) -> i64 {
            self.next.fetch_add(1, Ordering::SeqCst)
        }
    }

    struct Harness {
        service: SyncService,
        storage: Storage,
        peer_identity: NodeIdentity,
        peer_sealer: EnvelopeSealer,
        worker: BatcherWorker,
        _tmp: TempDir,
    }

    fn build_harness() -> Harness {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("open storage");
        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });

        let identity = Arc::new(NodeIdentity::from_seed([10u8; 32]));

        let manager = crate::session_log::SessionLogManager::with_clock(
            storage.clone(),
            worker.handle(),
            projection,
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(1_000)),
        )
        .expect("manager");

        let peer_identity = NodeIdentity::from_seed([20u8; 32]);
        let peer_hex = peer_identity.node_id();
        storage
            .upsert_pairing(&peer_hex, 1_000, "test peer")
            .expect("pair");

        let service = SyncService::new(manager, storage.clone(), identity);
        let peer_sealer = EnvelopeSealer::new(peer_identity.clone());

        Harness {
            service,
            storage,
            peer_identity,
            peer_sealer,
            worker,
            _tmp: tmp,
        }
    }

    fn make_valid_loro_snapshot(message: &str) -> Vec<u8> {
        let doc = LoroDoc::new();
        let list = doc.get_list(ENTRIES_CONTAINER);
        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: message.into(),
            metadata: None,
        };
        list.push(serde_json::to_string(&entry).unwrap().as_str()).unwrap();
        doc.commit();
        doc.export(ExportMode::Snapshot).unwrap()
    }

    fn baseline_snapshot_count(harness: &Harness) -> u64 {
        harness.storage.snapshot_count().expect("snapshot count")
    }

    fn baseline_envelope_count(harness: &Harness) -> u64 {
        harness.storage.accepted_envelope_count().expect("envelope count")
    }

    #[test]
    fn valid_delta_persists_accepted_envelope_and_snapshot() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "test-session", 0);
        let snapshot = make_valid_loro_snapshot("hello");
        let envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot.clone(),
        );
        let mut inbox = EnvelopeInbox::new();

        let advanced = harness
            .service
            .receive_delta(
                &peer_hex,
                &chunk,
                &chunk.as_path(),
                &envelope,
                true,
                &mut inbox,
            )
            .expect("receive_delta");

        assert!(advanced);
        assert!(
            harness.storage.snapshot_count().expect("count") >= 1,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            1,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn malformed_crdt_payload_leaves_no_accepted_envelope_or_snapshot() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "malformed-session", 0);
        let garbage = b"not valid loro data at all".to_vec();
        let envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            garbage,
        );
        let mut inbox = EnvelopeInbox::new();

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err(), "malformed payload must be rejected");
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
            "no snapshot should be persisted for malformed payload",
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
            "no accepted envelope should be persisted for malformed payload",
        );
        harness.worker.shutdown();
    }

    #[test]
    fn invalid_signature_rejected_without_storage_mutation() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "sig-session", 0);
        let snapshot = make_valid_loro_snapshot("sig-test");
        let mut envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );
        envelope.signature[0] ^= 0xFF;

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn payload_hash_mismatch_rejected_without_storage_mutation() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "hash-session", 0);
        let snapshot = make_valid_loro_snapshot("hash-test");
        let mut envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );
        envelope.payload = b"tampered payload".to_vec();

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn mismatched_sender_rejected_without_storage_mutation() {
        let harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "sender-session", 0);
        let snapshot = make_valid_loro_snapshot("sender-test");

        let impostor = NodeIdentity::from_seed([99u8; 32]);
        let mut impostor_sealer = EnvelopeSealer::new(impostor);
        let envelope = impostor_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn foreign_namespace_rejected_without_storage_mutation() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let local_hex = harness.service.node_id();
        let foreign_chunk = ChunkId::new(&local_hex, "stolen-session", 0);
        let snapshot = make_valid_loro_snapshot("namespace-test");
        let envelope = harness.peer_sealer.seal(
            foreign_chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &foreign_chunk,
            &foreign_chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn duplicate_message_id_rejected_without_storage_mutation() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "dup-session", 0);
        let snapshot = make_valid_loro_snapshot("dup-test");
        let envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot.clone(),
        );

        let mut inbox = EnvelopeInbox::new();

        let first = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );
        assert!(first.is_ok());

        let snapshots_after_first = harness.storage.snapshot_count().expect("count");
        let envelopes_after_first = harness.storage.accepted_envelope_count().expect("count");

        let second = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );
        assert!(second.is_err(), "duplicate must be rejected");

        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_after_first,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_after_first,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn payload_kind_mismatch_rejected_without_storage_mutation() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "kind-session", 0);
        let snapshot = make_valid_loro_snapshot("kind-test");
        let envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Delta,
            snapshot,
        );

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn document_id_mismatch_rejected_without_storage_mutation() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "docid-session", 0);
        let other_chunk = ChunkId::new(&peer_hex, "other-session", 0);
        let snapshot = make_valid_loro_snapshot("docid-test");
        let envelope = harness.peer_sealer.seal(
            other_chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn envelope_persistence_failure_leaves_no_snapshot_nor_envelope() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "envelope-fail-session", 0);
        let snapshot = make_valid_loro_snapshot("envelope-fail-entry");
        let envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);

        harness.storage.inject_accepted_envelope_failure();
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(
            result.is_err(),
            "receive_delta must fail when envelope persistence fails",
        );
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
            "no snapshot should be persisted when envelope save fails",
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
            "no accepted envelope should exist when envelope save fails",
        );
        harness.worker.shutdown();
    }

    fn make_schema_invalid_snapshot_non_string_entry() -> Vec<u8> {
        let doc = LoroDoc::new();
        let list = doc.get_list(ENTRIES_CONTAINER);
        list.insert(0, 42).unwrap();
        doc.commit();
        doc.export(ExportMode::Snapshot).unwrap()
    }

    fn make_schema_invalid_snapshot_bad_json() -> Vec<u8> {
        let doc = LoroDoc::new();
        let list = doc.get_list(ENTRIES_CONTAINER);
        list.push("not valid entry json").unwrap();
        doc.commit();
        doc.export(ExportMode::Snapshot).unwrap()
    }

    fn make_schema_invalid_snapshot_non_monotonic() -> Vec<u8> {
        let doc = LoroDoc::new();
        let list = doc.get_list(ENTRIES_CONTAINER);
        let entry1 = Entry {
            sequence: 5,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "first".into(),
            metadata: None,
        };
        let entry2 = Entry {
            sequence: 3,
            created_at_unix_ms: 6_000,
            source: "remote".into(),
            level: "info".into(),
            message: "second".into(),
            metadata: None,
        };
        list.push(serde_json::to_string(&entry1).unwrap().as_str()).unwrap();
        list.push(serde_json::to_string(&entry2).unwrap().as_str()).unwrap();
        doc.commit();
        doc.export(ExportMode::Snapshot).unwrap()
    }

    #[test]
    fn schema_invalid_non_string_entry_rejected_without_persistence() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "schema-non-string", 0);
        let snapshot = make_schema_invalid_snapshot_non_string_entry();
        let envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn schema_invalid_bad_entry_json_rejected_without_persistence() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "schema-bad-json", 0);
        let snapshot = make_schema_invalid_snapshot_bad_json();
        let envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }

    #[test]
    fn schema_invalid_non_monotonic_sequence_rejected_without_persistence() {
        let mut harness = build_harness();
        let peer_hex = harness.peer_identity.node_id();
        let chunk = ChunkId::new(&peer_hex, "schema-non-monotonic", 0);
        let snapshot = make_schema_invalid_snapshot_non_monotonic();
        let envelope = harness.peer_sealer.seal(
            chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot,
        );

        let snapshots_before = baseline_snapshot_count(&harness);
        let envelopes_before = baseline_envelope_count(&harness);
        let mut inbox = EnvelopeInbox::new();

        let result = harness.service.receive_delta(
            &peer_hex,
            &chunk,
            &chunk.as_path(),
            &envelope,
            true,
            &mut inbox,
        );

        assert!(result.is_err());
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );
        assert_eq!(
            harness.storage.accepted_envelope_count().expect("count"),
            envelopes_before,
        );
        harness.worker.shutdown();
    }
}
