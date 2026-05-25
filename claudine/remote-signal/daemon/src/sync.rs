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
use std::time::Duration;

use prost::Message;
use quinn::{Connection, RecvStream, SendStream};
use remote_signal_core::sync::{SyncWireError, encode_frame};
use remote_signal_core::{
    ChunkId, ChunkIdParseError, EnvelopeError, EnvelopeInbox, NodeIdentity, SignedEnvelope,
    SignedEnvelopeWire, SyncAdvertiseEnd, SyncChunkAdvertise, SyncDelta, SyncEnd, SyncFrame,
    SyncHello, identity::PUBLIC_KEY_LENGTH, sync_frame,
};

use crate::session_log::{SessionLogError, SessionLogManager};
use crate::storage::Storage;

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
}

impl SyncError {
    fn protocol<S: Into<String>>(reason: S) -> Self {
        Self::Protocol(reason.into())
    }
}

/// Service responsible for driving a sync session against one paired
/// peer. Holds shared handles on the session log, the pairings store,
/// and the local identity so both inbound and outbound runs use the
/// exact same code path.
#[derive(Clone)]
pub struct SyncService {
    session_log: SessionLogManager,
    storage: Storage,
    identity: Arc<NodeIdentity>,
}

impl SyncService {
    /// Build a new sync service from the running daemon's handles.
    #[must_use]
    pub fn new(
        session_log: SessionLogManager,
        storage: Storage,
        identity: Arc<NodeIdentity>,
    ) -> Self {
        Self {
            session_log,
            storage,
            identity,
        }
    }

    /// Local node id (hex-encoded public key).
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
        // Pairing must be in place locally before we open the stream.
        self.assert_paired(expected_peer_node_id)?;
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
            self.assert_paired(&peer_node_id_hex)?;

            // ---- Advertise phase (we send our local state) -------
            let local_chunks = self.session_log.list_all_chunks()?;
            let mut chunk_outcomes: HashMap<String, SyncChunkOutcome> = HashMap::new();
            for chunk in &local_chunks {
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

            // ---- Read remote advertisements ----------------------
            let mut remote_state: HashMap<ChunkId, Vec<u8>> = HashMap::new();
            loop {
                let (frame, received) = read_frame(&mut recv).await?;
                outcome.received_bytes += received;
                match frame.kind {
                    Some(sync_frame::Kind::Advertise(ad)) => {
                        let chunk: ChunkId = ad.chunk_id.parse()?;
                        remote_state.insert(chunk, ad.version_vector);
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
            // For every chunk the remote advertised, send updates
            // since their VV. For chunks they did NOT advertise but
            // we do have, send the full snapshot.
            let mut local_paths: HashMap<String, ChunkId> =
                local_chunks.iter().map(|c| (c.as_path(), c.clone())).collect();
            // First handle every chunk the remote knows about.
            for (chunk, vv_bytes) in &remote_state {
                let remote_vv = if vv_bytes.is_empty() { None } else { Some(vv_bytes.as_slice()) };
                let Some(exported) = self.session_log.export_updates_since(chunk, remote_vv)?
                else {
                    // Remote knows a chunk we don't — skip; the
                    // remote will push a snapshot to us.
                    continue;
                };
                local_paths.remove(&chunk.as_path());
                if exported.bytes.is_empty() {
                    continue;
                }
                let envelope = SignedEnvelope::seal(&self.identity, exported.bytes);
                let frame = SyncFrame {
                    kind: Some(sync_frame::Kind::Delta(SyncDelta {
                        chunk_id: chunk.as_path(),
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
            // Then push snapshots for any chunk the remote does not
            // know about at all.
            for (path, chunk) in &local_paths {
                let Some(exported) = self.session_log.export_updates_since(chunk, None)? else {
                    continue;
                };
                if exported.bytes.is_empty() {
                    continue;
                }
                let envelope = SignedEnvelope::seal(&self.identity, exported.bytes);
                let frame = SyncFrame {
                    kind: Some(sync_frame::Kind::Delta(SyncDelta {
                        chunk_id: path.clone(),
                        envelope: Some(SignedEnvelopeWire::from_envelope(&envelope)),
                        is_snapshot: true,
                    })),
                };
                let sent = write_frame(&mut send, &frame).await?;
                outcome.sent_bytes += sent;
                if let Some(o) = chunk_outcomes.get_mut(path) {
                    o.sent_bytes += sent;
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
                        if hex_encode(&envelope.sender) != peer_node_id_hex {
                            return Err(SyncError::protocol(
                                "delta envelope sender does not match hello node_id",
                            ));
                        }
                        let payload = inbox.accept(&envelope)?.to_vec();
                        let advanced = self.session_log.apply_remote_update(&chunk, &payload)?;
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
