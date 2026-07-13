//! In-memory session-log manager.
//!
//! Phase 2 holds one [`LoroDoc`] per active chunk, keyed by the
//! deterministic chunk ID. Every append:
//!
//! 1. Resolves (or rotates to) the active chunk for the session,
//! 2. Stages the entry in a cloned Loro doc,
//! 3. Exports a fresh snapshot from the staged doc,
//! 4. Persists the snapshot to redb (the source of truth),
//! 5. Swaps the staged doc into the live in-memory state,
//! 6. Queues the entry for the DuckDB projection batcher.
//!
//! On startup, the manager rehydrates the in-memory state from redb so
//! the daemon can resume sequence counters and active chunk pointers
//! after a crash.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use loro::{ExportMode, LoroDoc, VersionVector};
use parking_lot::Mutex;
use rendezvous_core::{
    ChunkConfig, ChunkId, ChunkMetadata, EnvelopeSealer, Entry, NodeIdentity, PayloadKind,
    SignedEnvelope, ENVELOPE_HASH_LENGTH, PUBLIC_KEY_LENGTH, SIGNATURE_LENGTH,
};
use serde_json::Value as JsonValue;

use crate::batcher::{BatcherError, BatcherHandle};
use crate::projection::{Projection, ProjectionError, ProjectionRow};
use crate::storage::{Storage, StorageError};

/// Loro container name used to hold the per-chunk metadata map.
const METADATA_CONTAINER: &str = "metadata";

/// Loro container name used to hold the per-chunk append-only entries
/// list. Each list element is a JSON-encoded [`Entry`].
const ENTRIES_CONTAINER: &str = "entries";

/// Errors that the session-log manager can surface.
#[derive(Debug, thiserror::Error)]
pub enum SessionLogError {
    /// The underlying redb storage layer failed.
    #[error(transparent)]
    Storage(#[from] StorageError),

    /// The Loro CRDT engine failed.
    #[error("loro error: {0}")]
    Loro(String),

    /// JSON serialization of an entry or metadata blob failed.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    /// Submitting a row to the projection batcher failed.
    #[error(transparent)]
    Batcher(#[from] BatcherError),

    /// The DuckDB projection layer failed.
    #[error(transparent)]
    Projection(#[from] ProjectionError),

    /// A persisted entry could not be decoded back into the strongly
    /// typed [`Entry`] schema.
    #[error("decoded entry has unexpected shape: {reason}")]
    DecodeEntry { reason: String },

    /// A remote CRDT payload failed session-log schema validation.
    #[error("remote document schema validation failed: {reason}")]
    SchemaValidation { reason: String },

    /// A remote update imported without error but left operations
    /// pending (their causal dependencies are missing locally). Every
    /// delta in this protocol is exported against our advertised
    /// version, so pending ops mean the sender's history no longer
    /// reaches back to our version — typically because it re-based
    /// (compacted) its document past us. The update must not be
    /// treated as applied; recovery is a snapshot-replace.
    #[error("remote update for {chunk_id} left ops pending ({detail}); snapshot-replace required")]
    PendingRemoteOps { chunk_id: String, detail: String },
}

impl From<loro::LoroError> for SessionLogError {
    fn from(error: loro::LoroError) -> Self {
        Self::Loro(error.to_string())
    }
}

impl From<loro::LoroEncodeError> for SessionLogError {
    fn from(error: loro::LoroEncodeError) -> Self {
        Self::Loro(error.to_string())
    }
}

/// Bytes returned from [`SessionLogManager::export_updates_since`].
#[derive(Clone, Debug)]
pub struct ExportedUpdate {
    /// How the receiver must apply `bytes`: an incremental update
    /// bundle (`Delta`), a full snapshot for a peer with no prior
    /// state (`Snapshot`), or a full snapshot the peer must adopt
    /// wholesale (`SnapshotReplace` — sent when the peer's version
    /// predates this document's shallow root, so the connecting delta
    /// history no longer exists).
    pub kind: PayloadKind,
    /// Encoded Loro payload. Empty when there are no updates beyond
    /// the supplied state vector.
    pub bytes: Vec<u8>,
}

/// Outcome of an [`SessionLogManager::append_entry`] call.
#[derive(Clone, Debug)]
pub struct AppendOutcome {
    /// Chunk the entry was appended to (post-rotation).
    pub chunk: ChunkId,
    /// Sequence number assigned to the entry.
    pub sequence: u64,
    /// Wall-clock at the moment the entry was appended.
    pub created_at_unix_ms: i64,
    /// `true` if appending caused the active chunk to rotate to a new
    /// one.
    pub rotated: bool,
}

/// Opaque staged result of importing a remote update without persisting
/// the resulting snapshot. The sync path uses this to persist the
/// accepted envelope before committing the snapshot. Drop without
/// calling [`SessionLogManager::commit_staged_update`] to discard.
pub(crate) struct StagedRemoteUpdate {
    state: ChunkState,
}

/// In-memory state for one chunk.
struct ChunkState {
    doc: LoroDoc,
    metadata: ChunkMetadata,
    entry_count: u64,
    byte_estimate: u64,
}

impl ChunkState {
    fn new(metadata: ChunkMetadata) -> Result<Self, SessionLogError> {
        let doc = LoroDoc::new();
        // Initialise the metadata map so freshly created chunks are
        // immediately introspectable.
        let map = doc.get_map(METADATA_CONTAINER);
        map.insert("owner_node_id", metadata.owner_node_id.as_str())?;
        map.insert("session_id", metadata.session_id.as_str())?;
        map.insert("chunk_index", metadata.chunk_index as i64)?;
        map.insert("created_at_unix_ms", metadata.created_at_unix_ms)?;
        if let Some(prev) = metadata.previous_chunk_id.as_deref() {
            map.insert("previous_chunk_id", prev)?;
        }
        let _list = doc.get_list(ENTRIES_CONTAINER);
        doc.commit();
        Ok(Self {
            doc,
            metadata,
            entry_count: 0,
            byte_estimate: 0,
        })
    }

    fn from_snapshot(snapshot: &[u8], metadata: ChunkMetadata) -> Result<Self, SessionLogError> {
        let doc = LoroDoc::from_snapshot(snapshot)?;
        let (entry_count, byte_estimate) = doc_entry_stats(&doc);
        Ok(Self {
            doc,
            metadata,
            entry_count,
            byte_estimate,
        })
    }

    fn append(&mut self, entry: &Entry) -> Result<(), SessionLogError> {
        let serialised = serde_json::to_string(entry)?;
        let list = self.doc.get_list(ENTRIES_CONTAINER);
        list.push(serialised.as_str())?;
        self.doc.commit();
        self.entry_count += 1;
        self.byte_estimate += entry.size_estimate();
        Ok(())
    }

    fn snapshot_bytes(&self) -> Result<Vec<u8>, SessionLogError> {
        Ok(self.doc.export(ExportMode::Snapshot)?)
    }

    fn recompute_stats(&mut self) {
        let (count, bytes) = doc_entry_stats(&self.doc);
        self.entry_count = count;
        self.byte_estimate = bytes;
    }

    fn collect_entries(&self) -> Result<Vec<Entry>, SessionLogError> {
        let list = self.doc.get_list(ENTRIES_CONTAINER);
        let mut decode_error: Option<SessionLogError> = None;
        let mut out = Vec::with_capacity(list.len());
        list.for_each(|value| {
            if decode_error.is_some() {
                return;
            }
            match value_to_string(&value) {
                Some(json) => match serde_json::from_str::<Entry>(&json) {
                    Ok(entry) => out.push(entry),
                    Err(err) => decode_error = Some(SessionLogError::Json(err)),
                },
                None => {
                    decode_error = Some(SessionLogError::DecodeEntry {
                        reason: "entries list contained a non-string value".into(),
                    });
                }
            }
        });
        if let Some(err) = decode_error {
            Err(err)
        } else {
            Ok(out)
        }
    }
}

/// Per-session state tracked by the manager: which chunk is currently
/// being appended to, and the next sequence number to hand out.
#[derive(Clone, Debug)]
struct SessionCursor {
    active_chunk_index: u64,
    next_sequence: u64,
}

struct ManagerInner {
    /// Chunk path → in-memory state.
    chunks: HashMap<String, ChunkState>,
    /// Session key (`session/{node}/{session}`) → cursor.
    sessions: HashMap<String, SessionCursor>,
    /// Session key → per-session append serialization lock. Held for the
    /// whole stage→persist→merge window of an append so two appends to
    /// the same session cannot reserve the same sequence or stage from
    /// the same base chunk. Different sessions get different locks, so
    /// they still run concurrently across the redb fsync.
    session_locks: HashMap<String, Arc<Mutex<()>>>,
}

/// Owner of the session-log in-memory state plus the redb storage handle
/// and the batcher submission point.
#[derive(Clone)]
pub struct SessionLogManager {
    inner: Arc<Mutex<ManagerInner>>,
    storage: Storage,
    batcher: BatcherHandle,
    projection: Projection,
    config: ChunkConfig,
    clock: Arc<dyn Clock + Send + Sync>,
    identity: Arc<NodeIdentity>,
    sealer: Arc<Mutex<EnvelopeSealer>>,
}

impl std::fmt::Debug for SessionLogManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SessionLogManager")
            .field("config", &self.config)
            .field("storage_path", &self.storage.path())
            .finish_non_exhaustive()
    }
}

/// Trait used so tests can pin the wall-clock to deterministic values.
pub trait Clock: Send + Sync {
    fn now_unix_ms(&self) -> i64;
}

/// Default clock that reads `SystemTime::now()`.
#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_ms(&self) -> i64 {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();
        i64::try_from(now).unwrap_or(i64::MAX)
    }
}

impl SessionLogManager {
    /// Build a new manager wired to the given storage, batcher,
    /// chunk-rotation configuration, and signing identity.
    pub fn new(
        storage: Storage,
        batcher: BatcherHandle,
        projection: Projection,
        config: ChunkConfig,
        identity: Arc<NodeIdentity>,
    ) -> Result<Self, SessionLogError> {
        Self::with_clock(storage, batcher, projection, config, identity, Arc::new(SystemClock))
    }

    /// Build a manager with an injected clock. Tests use this to feed
    /// deterministic `created_at_unix_ms` values into entries.
    pub fn with_clock(
        storage: Storage,
        batcher: BatcherHandle,
        projection: Projection,
        config: ChunkConfig,
        identity: Arc<NodeIdentity>,
        clock: Arc<dyn Clock + Send + Sync>,
    ) -> Result<Self, SessionLogError> {
        let node_id = identity.node_id();
        let start_counter = storage.load_outbound_counter(&node_id)?;
        let sealer = Arc::new(Mutex::new(EnvelopeSealer::with_start((*identity).clone(), start_counter)));
        let manager = Self {
            inner: Arc::new(Mutex::new(ManagerInner {
                chunks: HashMap::new(),
                sessions: HashMap::new(),
                session_locks: HashMap::new(),
            })),
            storage,
            batcher,
            projection,
            config,
            clock,
            identity,
            sealer,
        };
        manager.rehydrate_from_storage()?;
        manager.rebuild_projection_from_storage()?;
        Ok(manager)
    }

    #[must_use]
    pub fn identity(&self) -> &NodeIdentity {
        &self.identity
    }

    /// Stable hex-encoded node identifier of this manager's identity.
    #[must_use]
    pub fn node_id(&self) -> String {
        self.identity.node_id()
    }

    /// Export the current Loro snapshot for `chunk` and seal it in a
    /// [`SignedEnvelope`] authenticated by this manager's identity.
    ///
    /// Used by the (forthcoming) network layer to push deltas to
    /// paired peers. The returned envelope can be transmitted as-is;
    /// receivers verify it with [`SignedEnvelope::verify`] or feed it
    /// through an [`rendezvous_core::EnvelopeInbox`] for replay
    /// protection.
    pub fn sign_chunk_snapshot(
        &self,
        chunk: &ChunkId,
    ) -> Result<Option<SignedEnvelope>, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();
        let snapshot_bytes = if let Some(state) = inner.chunks.get(&key) {
            state.snapshot_bytes()?
        } else {
            drop(inner);
            let Some(bytes) = self.storage.load_snapshot(chunk)? else {
                return Ok(None);
            };
            bytes
        };
        // Capture the counter that follows the sealed envelope inside the
        // same lock scope so a concurrent seal cannot advance the counter
        // past the value we persist.
        let (envelope, counter_to_persist) = {
            let mut sealer = self.sealer.lock();
            let envelope = sealer.seal(
                chunk.as_path(),
                PayloadKind::Snapshot,
                snapshot_bytes,
            );
            let counter = sealer.next_counter();
            (envelope, counter)
        };
        self.storage.save_outbound_counter(&self.node_id(), counter_to_persist)?;
        Ok(Some(envelope))
    }

    #[must_use]
    pub fn config(&self) -> ChunkConfig {
        self.config
    }

    /// Shared handle on the outbound envelope sealer. The sync service
    /// uses this instead of creating its own so both code paths issue
    /// monotonically increasing message IDs from the same counter.
    #[must_use]
    pub fn sealer(&self) -> Arc<Mutex<EnvelopeSealer>> {
        Arc::clone(&self.sealer)
    }

    /// Per-session append serialization lock, created on first use.
    fn session_lock(&self, session_key: &str) -> Arc<Mutex<()>> {
        let mut inner = self.inner.lock();
        Arc::clone(
            inner
                .session_locks
                .entry(session_key.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(()))),
        )
    }

    /// Append a new entry to the session, rotating chunks if required.
    pub fn append_entry(
        &self,
        owner_node_id: &str,
        session_id: &str,
        source: impl Into<String>,
        level: impl Into<String>,
        message: impl Into<String>,
        metadata: Option<JsonValue>,
    ) -> Result<AppendOutcome, SessionLogError> {
        let session_key = format!("session/{owner_node_id}/{session_id}");
        let now = self.clock.now_unix_ms();

        // Serialize same-session appends across the entire
        // stage→persist→merge window. The global `inner` lock alone is
        // not enough: it is deliberately dropped across the redb fsync so
        // unrelated sessions stay concurrent, which leaves a window where
        // two appends to THIS session could clone the same cursor, reserve
        // the same sequence, and stage from the same base chunk — then
        // race their independent whole-snapshot redb writes and durably
        // drop one entry. Holding the per-session lock for the whole call
        // closes that window while preserving cross-session concurrency.
        let append_guard = self.session_lock(&session_key);
        let _append_guard = append_guard.lock();

        let mut inner = self.inner.lock();

        let cursor = inner
            .sessions
            .entry(session_key.clone())
            .or_insert(SessionCursor {
                active_chunk_index: 0,
                next_sequence: 0,
            })
            .clone();

        let active_id = ChunkId::new(owner_node_id, session_id, cursor.active_chunk_index);
        let active_path = active_id.as_path();

        let needs_rotation = inner
            .chunks
            .get(&active_path)
            .map(|state| {
                self.config
                    .should_rotate(state.entry_count, state.byte_estimate)
            })
            .unwrap_or(false);

        let sequence = cursor.next_sequence;

        let entry = Entry {
            sequence,
            created_at_unix_ms: now,
            source: source.into(),
            level: level.into(),
            message: message.into(),
            metadata,
        };

        // Stage mutation in a separate doc so a persistence failure does
        // not leave unacknowledged data in the live in-memory state.
        let (chunk_id, rotated, staged_state) = if needs_rotation {
            let previous_metadata = inner
                .chunks
                .get(&active_path)
                .expect("active chunk exists")
                .metadata
                .clone();
            let next_metadata = ChunkMetadata::rotated_from(&previous_metadata, now);
            let next_id = next_metadata.chunk_id();
            let mut staged = ChunkState::new(next_metadata)?;
            staged.append(&entry)?;
            (next_id, true, staged)
        } else if let Some(existing) = inner.chunks.get(&active_path) {
            let pre_snapshot = existing.snapshot_bytes()?;
            let mut staged = ChunkState::from_snapshot(&pre_snapshot, existing.metadata.clone())?;
            staged.append(&entry)?;
            (active_id, false, staged)
        } else {
            let metadata = ChunkMetadata::initial(owner_node_id, session_id, now);
            let mut staged = ChunkState::new(metadata)?;
            staged.append(&entry)?;
            (active_id, false, staged)
        };

        let chunk_path = chunk_id.as_path();
        let snapshot_bytes = staged_state.snapshot_bytes()?;

        // Drop the lock before the synchronous redb fsync so unrelated
        // sessions can keep making progress.
        drop(inner);

        // Persist to redb BEFORE mutating the live in-memory state so a
        // storage failure cannot leave unacknowledged entries resident.
        self.storage.save_snapshot(&chunk_id, &snapshot_bytes)?;

        // Re-acquire briefly to merge the staged snapshot into the live
        // state. Importing is idempotent, so concurrent appends that
        // raced ahead are preserved rather than overwritten.
        let mut inner = self.inner.lock();
        match inner.chunks.get_mut(&chunk_path) {
            Some(existing) => {
                existing.doc.import(&snapshot_bytes)?;
                existing.recompute_stats();
            }
            None => {
                inner.chunks.insert(chunk_path.clone(), staged_state);
            }
        }

        if rotated {
            let session_cursor = inner
                .sessions
                .get_mut(&session_key)
                .expect("session cursor exists");
            if session_cursor.active_chunk_index < chunk_id.chunk_index {
                session_cursor.active_chunk_index = chunk_id.chunk_index;
            }
        }

        let session_cursor = inner
            .sessions
            .get_mut(&session_key)
            .expect("session cursor exists");
        if sequence + 1 > session_cursor.next_sequence {
            session_cursor.next_sequence = sequence + 1;
        }

        drop(inner);

        // Fan out to the projection batcher. redb is the source of truth
        // so a closed/failed batcher must not cause the caller to retry
        // and create a duplicate entry.
        let metadata_json = entry
            .metadata
            .as_ref()
            .map(serde_json::to_string)
            .transpose()?
            .unwrap_or_default();
        if let Err(err) = self.batcher.submit(ProjectionRow {
            chunk: chunk_id.clone(),
            sequence,
            created_at_unix_ms: entry.created_at_unix_ms,
            source: entry.source.clone(),
            level: entry.level.clone(),
            message: entry.message.clone(),
            metadata_json,
        }) {
            tracing::warn!(
                target: "rendezvous_daemon::session_log",
                %err,
                "projection enqueue failed after redb durability; entry is still authoritative in redb",
            );
        }

        Ok(AppendOutcome {
            chunk: chunk_id,
            sequence,
            created_at_unix_ms: entry.created_at_unix_ms,
            rotated,
        })
    }

    /// List the in-memory entries for a chunk, in insertion order.
    pub fn list_chunk_entries(&self, chunk: &ChunkId) -> Result<Vec<Entry>, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();
        if let Some(state) = inner.chunks.get(&key) {
            return state.collect_entries();
        }
        drop(inner);
        // Fallback: maybe the chunk lives only on disk (e.g. caller is
        // asking about a rotated chunk we haven't hydrated). Load it
        // lazily, but do not cache the result — that path is only used
        // when consumers want a one-shot read.
        let Some(snapshot) = self.storage.load_snapshot(chunk)? else {
            return Ok(Vec::new());
        };
        // Read the real metadata from the snapshot instead of fabricating
        // a created_at value that would fail the crate's own validator.
        let doc = LoroDoc::from_snapshot(&snapshot)?;
        let metadata = validate_and_extract_metadata(
            &doc,
            &chunk.owner_node_id,
            &chunk.session_id,
            chunk.chunk_index,
        )?;
        let state = ChunkState::from_snapshot(&snapshot, metadata)?;
        state.collect_entries()
    }

    /// List all chunk IDs known for the session, in ascending order.
    pub fn list_session_chunks(
        &self,
        owner_node_id: &str,
        session_id: &str,
    ) -> Result<Vec<ChunkId>, SessionLogError> {
        Ok(self.storage.list_chunks(owner_node_id, session_id)?)
    }

    /// List every chunk that this manager knows about (in-memory plus
    /// any chunk persisted to redb), used as the advertise set during
    /// sync. Returns paths in stable lexicographic order.
    pub fn list_all_chunks(&self) -> Result<Vec<ChunkId>, SessionLogError> {
        let mut seen: HashMap<String, ChunkId> = HashMap::new();
        {
            let inner = self.inner.lock();
            for (path, state) in &inner.chunks {
                seen.insert(path.clone(), state.metadata.chunk_id());
            }
        }
        self.storage.iter_snapshots(|chunk, _bytes| {
            seen.entry(chunk.as_path()).or_insert(chunk);
            Ok(())
        })?;
        let mut out: Vec<(String, ChunkId)> = seen.into_iter().collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(out.into_iter().map(|(_, c)| c).collect())
    }

    /// Encode the Loro version vector for `chunk`, or `None` if the
    /// daemon has no copy of that chunk locally.
    pub fn chunk_state_vector(&self, chunk: &ChunkId) -> Result<Option<Vec<u8>>, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();
        if let Some(state) = inner.chunks.get(&key) {
            return Ok(Some(state.doc.oplog_vv().encode()));
        }
        drop(inner);
        let Some(snapshot) = self.storage.load_snapshot(chunk)? else {
            return Ok(None);
        };
        let doc = LoroDoc::from_snapshot(&snapshot)?;
        Ok(Some(doc.oplog_vv().encode()))
    }

    /// Export the Loro updates the local chunk has that are missing
    /// from `remote_state_vector`. Returns `None` if the chunk does not
    /// exist locally. When `remote_state_vector` is `None`, exports the
    /// full snapshot (used when the peer has no copy of the chunk).
    pub fn export_updates_since(
        &self,
        chunk: &ChunkId,
        remote_state_vector: Option<&[u8]>,
    ) -> Result<Option<ExportedUpdate>, SessionLogError> {
        let key = chunk.as_path();
        let snapshot_bytes;
        let doc = {
            let inner = self.inner.lock();
            if let Some(state) = inner.chunks.get(&key) {
                let bytes = state.snapshot_bytes()?;
                drop(inner);
                snapshot_bytes = bytes;
                LoroDoc::from_snapshot(&snapshot_bytes)?
            } else {
                drop(inner);
                let Some(bytes) = self.storage.load_snapshot(chunk)? else {
                    return Ok(None);
                };
                snapshot_bytes = bytes;
                LoroDoc::from_snapshot(&snapshot_bytes)?
            }
        };
        match remote_state_vector {
            Some(raw) => {
                let vv = VersionVector::decode(raw)?;
                // Shallow-root gate: a re-based (history-compacted) doc
                // no longer holds ops before its shallow root. Exporting
                // updates for an older peer version *succeeds*, but the
                // ops import as permanently-pending on the receiver
                // (verified by the register-compaction spike), so the
                // only safe answer for such a peer is a snapshot it
                // adopts wholesale.
                let shallow_since = doc.shallow_since_vv();
                let peer_covers_shallow_root = shallow_since
                    .iter()
                    .all(|(peer, counter)| vv.get(peer).is_some_and(|c| *c >= *counter));
                if !peer_covers_shallow_root {
                    return Ok(Some(ExportedUpdate {
                        kind: PayloadKind::SnapshotReplace,
                        bytes: doc.export(ExportMode::Snapshot)?,
                    }));
                }
                let updates = doc.export(ExportMode::updates(&vv))?;
                if updates.is_empty() {
                    Ok(Some(ExportedUpdate {
                        kind: PayloadKind::Delta,
                        bytes: Vec::new(),
                    }))
                } else {
                    Ok(Some(ExportedUpdate {
                        kind: PayloadKind::Delta,
                        bytes: updates,
                    }))
                }
            }
            None => Ok(Some(ExportedUpdate {
                kind: PayloadKind::Snapshot,
                bytes: doc.export(ExportMode::Snapshot)?,
            })),
        }
    }

    /// Stage a remote update by importing the payload into a temporary
    /// Loro doc, validating the document schema, but WITHOUT persisting
    /// the resulting snapshot. The caller must follow this with
    /// [`Self::commit_staged_update`] to persist and apply, or drop the
    /// result to discard.
    pub(crate) fn stage_remote_update(
        &self,
        chunk: &ChunkId,
        update_bytes: &[u8],
    ) -> Result<StagedRemoteUpdate, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();

        let staged_state = if let Some(state) = inner.chunks.get(&key) {
            let pre_snapshot = state.snapshot_bytes()?;
            let staged_doc = LoroDoc::from_snapshot(&pre_snapshot)?;
            let original_entry_json = collect_entry_json_strings(&staged_doc)?;
            let status = staged_doc.import(update_bytes)?;
            reject_pending_ops(chunk, &status)?;

            validate_metadata_unchanged(&staged_doc, &state.metadata)?;
            validate_remote_entries(&staged_doc, state.entry_count)?;
            validate_append_only_prefix(&staged_doc, &original_entry_json)?;

            let (count, bytes) = doc_entry_stats(&staged_doc);
            ChunkState {
                doc: staged_doc,
                metadata: state.metadata.clone(),
                entry_count: count,
                byte_estimate: bytes,
            }
        } else {
            let staged_doc = LoroDoc::new();
            let status = staged_doc.import(update_bytes)?;
            reject_pending_ops(chunk, &status)?;

            validate_remote_entries(&staged_doc, 0)?;
            let metadata = validate_and_extract_metadata(
                &staged_doc,
                &chunk.owner_node_id,
                &chunk.session_id,
                chunk.chunk_index,
            )?;

            let (count, bytes) = doc_entry_stats(&staged_doc);
            ChunkState {
                doc: staged_doc,
                metadata,
                entry_count: count,
                byte_estimate: bytes,
            }
        };

        drop(inner);
        Ok(StagedRemoteUpdate { state: staged_state })
    }

    /// Commit a previously staged remote update by persisting the
    /// snapshot to redb and merging the staged state into the live
    /// in-memory map. Returns whether the local chunk advanced.
    pub(crate) fn commit_staged_update(
        &self,
        chunk: &ChunkId,
        staged: StagedRemoteUpdate,
    ) -> Result<bool, SessionLogError> {
        let key = chunk.as_path();
        let snapshot_bytes = staged.state.snapshot_bytes()?;

        self.storage.save_snapshot(chunk, &snapshot_bytes)?;

        // Merge rather than wholesale replace so concurrent inbound
        // sync sessions on the same chunk cannot drop each other's
        // entries.
        let mut inner = self.inner.lock();
        let advanced = match inner.chunks.get_mut(&key) {
            Some(existing) => {
                let before_vv = existing.doc.oplog_vv();
                existing.doc.import(&snapshot_bytes)?;
                existing.recompute_stats();
                existing.doc.oplog_vv() != before_vv
            }
            None => {
                inner.chunks.insert(key.clone(), staged.state);
                true
            }
        };

        refresh_session_cursor(&mut inner, chunk);

        Ok(advanced)
    }

    /// Stage a remote snapshot-replace: the payload is a self-contained
    /// snapshot (typically shallow, i.e. history-compacted) that will
    /// REPLACE the local replica wholesale instead of merging into it.
    /// Sent by an owner whose document was re-based past our version.
    /// Validation still enforces chunk identity, entry schema, and the
    /// append-only entry prefix against current local state — a replace
    /// may extend the entry list but never rewrite or shrink it.
    pub(crate) fn stage_remote_replace(
        &self,
        chunk: &ChunkId,
        snapshot_bytes: &[u8],
    ) -> Result<StagedRemoteUpdate, SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();

        let staged_doc = LoroDoc::new();
        let status = staged_doc.import(snapshot_bytes)?;
        reject_pending_ops(chunk, &status)?;

        let metadata = if let Some(state) = inner.chunks.get(&key) {
            validate_metadata_unchanged(&staged_doc, &state.metadata)?;
            validate_remote_entries(&staged_doc, state.entry_count)?;
            let original_entry_json = collect_entry_json_strings(&state.doc)?;
            validate_append_only_prefix(&staged_doc, &original_entry_json)?;
            state.metadata.clone()
        } else {
            validate_remote_entries(&staged_doc, 0)?;
            validate_and_extract_metadata(
                &staged_doc,
                &chunk.owner_node_id,
                &chunk.session_id,
                chunk.chunk_index,
            )?
        };

        let (count, bytes) = doc_entry_stats(&staged_doc);
        drop(inner);
        Ok(StagedRemoteUpdate {
            state: ChunkState {
                doc: staged_doc,
                metadata,
                entry_count: count,
                byte_estimate: bytes,
            },
        })
    }

    /// Commit a staged snapshot-replace: persist the snapshot and swap
    /// the in-memory state wholesale. Unlike
    /// [`Self::commit_staged_update`] this deliberately does NOT merge —
    /// the incoming document's history root is newer than the local
    /// replica's, and importing across that gap does not converge
    /// (verified by the register-compaction spike). Nothing local can be
    /// lost: replicas of a foreign-owned chunk are read-only copies of
    /// the owner's document.
    pub(crate) fn commit_staged_replace(
        &self,
        chunk: &ChunkId,
        staged: StagedRemoteUpdate,
    ) -> Result<bool, SessionLogError> {
        let key = chunk.as_path();
        let snapshot_bytes = staged.state.snapshot_bytes()?;

        self.storage.save_snapshot(chunk, &snapshot_bytes)?;

        let mut inner = self.inner.lock();
        let advanced = match inner.chunks.get(&key) {
            Some(existing) => existing.doc.oplog_vv() != staged.state.doc.oplog_vv(),
            None => true,
        };
        inner.chunks.insert(key, staged.state);

        refresh_session_cursor(&mut inner, chunk);

        Ok(advanced)
    }

    /// Apply a delta or snapshot received from a peer. The bytes are
    /// imported into the in-memory chunk state (creating it if
    /// necessary) and the resulting snapshot is persisted to redb so
    /// the on-disk view stays authoritative.
    ///
    /// Returns `true` when the local chunk advanced as a result of the
    /// import (the version vector grew or a new chunk was created).
    pub fn apply_remote_update(
        &self,
        chunk: &ChunkId,
        update_bytes: &[u8],
    ) -> Result<bool, SessionLogError> {
        if update_bytes.is_empty() {
            return Ok(false);
        }
        let staged = self.stage_remote_update(chunk, update_bytes)?;
        self.commit_staged_update(chunk, staged)
    }

    /// Apply a snapshot-replace received from a peer: stage, validate,
    /// and swap the local replica wholesale. See
    /// [`Self::stage_remote_replace`] / [`Self::commit_staged_replace`].
    pub fn apply_remote_replace(
        &self,
        chunk: &ChunkId,
        snapshot_bytes: &[u8],
    ) -> Result<bool, SessionLogError> {
        if snapshot_bytes.is_empty() {
            return Ok(false);
        }
        let staged = self.stage_remote_replace(chunk, snapshot_bytes)?;
        self.commit_staged_replace(chunk, staged)
    }

    /// After a remote update has been applied to a chunk, submit every
    /// entry in that chunk to the projection batcher. Callers should
    /// invoke this only from the live sync path — startup replay relies
    /// on [`Self::rebuild_projection_from_storage`] instead.
    pub fn submit_chunk_to_projection(
        &self,
        chunk: &ChunkId,
    ) -> Result<(), SessionLogError> {
        let key = chunk.as_path();
        let inner = self.inner.lock();
        let Some(state) = inner.chunks.get(&key) else {
            return Ok(());
        };
        let list = state.doc.get_list(ENTRIES_CONTAINER);
        list.for_each(|value| {
            if let Some(json) = value_to_string(&value)
                && let Ok(entry) = serde_json::from_str::<Entry>(&json)
            {
                let metadata_json = entry
                    .metadata
                    .as_ref()
                    .map(|v| serde_json::to_string(v).unwrap_or_default())
                    .unwrap_or_default();
                let _ = self.batcher.submit(ProjectionRow {
                    chunk: chunk.clone(),
                    sequence: entry.sequence,
                    created_at_unix_ms: entry.created_at_unix_ms,
                    source: entry.source.clone(),
                    level: entry.level.clone(),
                    message: entry.message.clone(),
                    metadata_json,
                });
            }
        });
        Ok(())
    }

    /// Rebuild the DuckDB projection from every snapshot persisted in
    /// redb. Called once during startup so a restarted daemon with an
    /// empty DuckDB file can serve correct analytical queries.
    ///
    /// The truncate+repopulate is performed in a single DuckDB
    /// transaction so a failure mid-rebuild cannot leave the projection
    /// silently empty.
    fn rebuild_projection_from_storage(&self) -> Result<(), SessionLogError> {
        let mut snapshots: Vec<(ChunkId, Vec<u8>)> = Vec::new();
        self.storage.iter_snapshots(|chunk, bytes| {
            snapshots.push((chunk, bytes));
            Ok(())
        })?;
        let mut rows: Vec<ProjectionRow> = Vec::new();
        for (chunk, bytes) in snapshots {
            let doc = LoroDoc::from_snapshot(&bytes)?;
            let list = doc.get_list(ENTRIES_CONTAINER);
            list.for_each(|value| {
                if let Some(json) = value_to_string(&value)
                    && let Ok(entry) = serde_json::from_str::<Entry>(&json)
                {
                    let metadata_json = entry
                        .metadata
                        .as_ref()
                        .map(|v| serde_json::to_string(v).unwrap_or_default())
                        .unwrap_or_default();
                    rows.push(ProjectionRow {
                        chunk: chunk.clone(),
                        sequence: entry.sequence,
                        created_at_unix_ms: entry.created_at_unix_ms,
                        source: entry.source.clone(),
                        level: entry.level.clone(),
                        message: entry.message.clone(),
                        metadata_json,
                    });
                }
            });
        }
        self.projection.replace_all_rows(&rows)?;
        Ok(())
    }

    /// Rehydrate the in-memory state from every snapshot currently
    /// persisted in redb, then replay any accepted envelopes whose
    /// payloads may not yet be reflected in the snapshot (crash
    /// recovery). Subsequent appends pick up where the previous process
    /// left off.
    fn rehydrate_from_storage(&self) -> Result<(), SessionLogError> {
        let mut to_load: Vec<(ChunkId, Vec<u8>)> = Vec::new();
        self.storage.iter_snapshots(|chunk, bytes| {
            to_load.push((chunk, bytes));
            Ok(())
        })?;

        let mut inner = self.inner.lock();
        for (chunk_id, snapshot) in &to_load {
            let doc = LoroDoc::from_snapshot(snapshot)?;
            let metadata = validate_and_extract_metadata(
                &doc,
                &chunk_id.owner_node_id,
                &chunk_id.session_id,
                chunk_id.chunk_index,
            )?;
            let state = ChunkState::from_snapshot(snapshot, metadata)?;
            let session_key = chunk_id.session_key();
            let cursor = inner
                .sessions
                .entry(session_key)
                .or_insert(SessionCursor {
                    active_chunk_index: chunk_id.chunk_index,
                    next_sequence: 0,
                });
            if chunk_id.chunk_index >= cursor.active_chunk_index {
                cursor.active_chunk_index = chunk_id.chunk_index;
            }
            // The next sequence number is one greater than the highest
            // sequence persisted across all chunks of this session.
            let max_seq = state
                .collect_entries()?
                .into_iter()
                .map(|e| e.sequence)
                .max()
                .map(|m| m + 1)
                .unwrap_or(0);
            if max_seq > cursor.next_sequence {
                cursor.next_sequence = max_seq;
            }
            inner.chunks.insert(chunk_id.as_path(), state);
        }
        drop(inner);

        self.replay_accepted_envelopes_on_startup()?;

        Ok(())
    }

    /// Re-apply all persisted accepted-envelope payloads into the
    /// in-memory chunks. Loro imports are idempotent (a delta already
    /// present in the version vector is silently ignored), so this is
    /// safe for envelopes that were already applied. Envelopes whose
    /// `save_snapshot` was lost due to a crash will be recovered here.
    fn replay_accepted_envelopes_on_startup(&self) -> Result<(), SessionLogError> {
        let mut to_replay: Vec<crate::storage::AcceptedEnvelope> = Vec::new();
        self.storage.iter_accepted_envelopes(|envelope| {
            to_replay.push(envelope);
            Ok(())
        })?;

        for accepted in &to_replay {
            if accepted.payload_bytes.is_empty() {
                continue;
            }
            let chunk_id = match accepted.document_id.parse::<rendezvous_core::DocumentId>() {
                Ok(rendezvous_core::DocumentId::SessionChunk(chunk)) => chunk,
                // Register envelopes are persisted for replay
                // protection only — registers rehydrate from their own
                // redb table, and a crash-lost register commit heals on
                // the next sync (the peer re-sends against our
                // advertised version). Nothing to replay here.
                Ok(_) => continue,
                Err(err) => {
                    tracing::warn!(
                        target: "rendezvous_daemon::session_log",
                        document_id = %accepted.document_id,
                        %err,
                        "skipping accepted envelope with unrecognized document_id during replay",
                    );
                    continue;
                }
            };

            let Some(signed) = reconstruct_signed_envelope(accepted) else {
                tracing::warn!(
                    target: "rendezvous_daemon::session_log",
                    chunk_id = %chunk_id.as_path(),
                    "skipping accepted envelope with corrupt hex fields during replay",
                );
                continue;
            };

            if let Err(err) = signed.verify() {
                tracing::warn!(
                    target: "rendezvous_daemon::session_log",
                    chunk_id = %chunk_id.as_path(),
                    %err,
                    "skipping accepted envelope that failed signature verification during replay",
                );
                continue;
            }

            let apply_result = match PayloadKind::from_byte(accepted.payload_kind as i32) {
                Some(PayloadKind::SnapshotReplace) => {
                    self.apply_remote_replace(&chunk_id, &accepted.payload_bytes)
                }
                _ => self.apply_remote_update(&chunk_id, &accepted.payload_bytes),
            };
            match apply_result {
                Ok(_) => {}
                Err(SessionLogError::Loro(reason)) => {
                    tracing::warn!(
                        target: "rendezvous_daemon::session_log",
                        chunk_id = %chunk_id.as_path(),
                        %reason,
                        "skipping malformed accepted envelope during replay",
                    );
                }
                Err(SessionLogError::PendingRemoteOps { detail, .. }) => {
                    // Replay iterates envelopes in storage order, which
                    // can deliver an old delta after a replace already
                    // moved the doc's history root. The snapshot in redb
                    // is authoritative; the stale envelope is skipped.
                    tracing::warn!(
                        target: "rendezvous_daemon::session_log",
                        chunk_id = %chunk_id.as_path(),
                        %detail,
                        "skipping accepted envelope with disconnected history during replay",
                    );
                }
                Err(other) => return Err(other),
            }
        }

        Ok(())
    }
}

fn read_map_string(doc: &LoroDoc, key: &str) -> Result<String, SessionLogError> {
    use loro::ValueOrContainer;
    let map = doc.get_map(METADATA_CONTAINER);
    let Some(voc) = map.get(key) else {
        return Err(SessionLogError::SchemaValidation {
            reason: format!("metadata missing required key: {key}"),
        });
    };
    match voc {
        ValueOrContainer::Value(loro::LoroValue::String(s)) => Ok((*s).to_string()),
        _ => Err(SessionLogError::SchemaValidation {
            reason: format!("metadata key {key} is not a string"),
        }),
    }
}

fn read_map_i64(doc: &LoroDoc, key: &str) -> Result<i64, SessionLogError> {
    use loro::ValueOrContainer;
    let map = doc.get_map(METADATA_CONTAINER);
    let Some(voc) = map.get(key) else {
        return Err(SessionLogError::SchemaValidation {
            reason: format!("metadata missing required key: {key}"),
        });
    };
    match voc {
        ValueOrContainer::Value(loro::LoroValue::I64(n)) => Ok(n),
        ValueOrContainer::Value(loro::LoroValue::Double(n)) if n.fract() == 0.0 => {
            Ok(n as i64)
        }
        _ => Err(SessionLogError::SchemaValidation {
            reason: format!("metadata key {key} is not an integer"),
        }),
    }
}

fn validate_and_extract_metadata(
    doc: &LoroDoc,
    expected_owner: &str,
    expected_session_id: &str,
    expected_chunk_index: u64,
) -> Result<ChunkMetadata, SessionLogError> {
    let owner = read_map_string(doc, "owner_node_id")?;
    if owner != expected_owner {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata owner_node_id {owner:?} does not match expected {expected_owner:?}"
            ),
        });
    }

    let session_id = read_map_string(doc, "session_id")?;
    if session_id != expected_session_id {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata session_id {session_id:?} does not match expected {expected_session_id:?}"
            ),
        });
    }

    let chunk_index = read_map_i64(doc, "chunk_index")?;
    if chunk_index < 0 || chunk_index as u64 != expected_chunk_index {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata chunk_index {chunk_index} does not match expected {expected_chunk_index}"
            ),
        });
    }

    let created_at = read_map_i64(doc, "created_at_unix_ms")?;
    if created_at <= 0 {
        return Err(SessionLogError::SchemaValidation {
            reason: format!("metadata created_at_unix_ms is invalid: {created_at}"),
        });
    }

    let previous_chunk_id = if expected_chunk_index == 0 {
        let map = doc.get_map(METADATA_CONTAINER);
        if map.get("previous_chunk_id").is_some() {
            return Err(SessionLogError::SchemaValidation {
                reason: "metadata previous_chunk_id must be absent for chunk 0".into(),
            });
        }
        None
    } else {
        let prev = read_map_string(doc, "previous_chunk_id")?;
        let expected_prev = ChunkId::new(
            expected_owner.to_string(),
            expected_session_id.to_string(),
            expected_chunk_index - 1,
        )
        .as_path();
        if prev != expected_prev {
            return Err(SessionLogError::SchemaValidation {
                reason: format!(
                    "metadata previous_chunk_id {prev:?} does not match expected {expected_prev:?}"
                ),
            });
        }
        Some(prev)
    };

    Ok(ChunkMetadata {
        owner_node_id: owner,
        session_id,
        chunk_index: chunk_index as u64,
        created_at_unix_ms: created_at,
        previous_chunk_id,
    })
}

fn validate_metadata_unchanged(
    doc: &LoroDoc,
    expected: &ChunkMetadata,
) -> Result<(), SessionLogError> {
    let owner = read_map_string(doc, "owner_node_id")?;
    if owner != expected.owner_node_id {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata owner_node_id {owner:?} mutated from {:?}",
                expected.owner_node_id
            ),
        });
    }

    let session_id = read_map_string(doc, "session_id")?;
    if session_id != expected.session_id {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata session_id {session_id:?} mutated from {:?}",
                expected.session_id
            ),
        });
    }

    let chunk_index = read_map_i64(doc, "chunk_index")?;
    if chunk_index < 0 || chunk_index as u64 != expected.chunk_index {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata chunk_index {chunk_index} mutated from {}",
                expected.chunk_index
            ),
        });
    }

    let created_at = read_map_i64(doc, "created_at_unix_ms")?;
    if created_at != expected.created_at_unix_ms {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "metadata created_at_unix_ms {created_at} mutated from {}",
                expected.created_at_unix_ms
            ),
        });
    }

    match &expected.previous_chunk_id {
        Some(expected_prev) => {
            let prev = read_map_string(doc, "previous_chunk_id")?;
            if prev != *expected_prev {
                return Err(SessionLogError::SchemaValidation {
                    reason: format!(
                        "metadata previous_chunk_id {prev:?} mutated from {expected_prev:?}"
                    ),
                });
            }
        }
        None => {
            let map = doc.get_map(METADATA_CONTAINER);
            if map.get("previous_chunk_id").is_some() {
                return Err(SessionLogError::SchemaValidation {
                    reason: "metadata previous_chunk_id mutated from None".into(),
                });
            }
        }
    }

    Ok(())
}

fn validate_remote_entries(
    doc: &LoroDoc,
    existing_entry_count: u64,
) -> Result<(), SessionLogError> {
    let list = doc.get_list(ENTRIES_CONTAINER);
    let len = list.len() as u64;

    if len < existing_entry_count {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "entry count decreased from {existing_entry_count} to {len}"
            ),
        });
    }

    let mut validation_error: Option<SessionLogError> = None;
    let mut last_seq: Option<u64> = None;

    list.for_each(|value| {
        if validation_error.is_some() {
            return;
        }
        match value_to_string(&value) {
            Some(json) => match serde_json::from_str::<Entry>(&json) {
                Ok(entry) => {
                    if let Some(last) = last_seq
                        && entry.sequence <= last
                    {
                        validation_error = Some(SessionLogError::SchemaValidation {
                            reason: format!(
                                "non-monotonic sequence: {} follows {}",
                                entry.sequence, last
                            ),
                        });
                    }
                    last_seq = Some(entry.sequence);
                }
                Err(err) => {
                    validation_error = Some(SessionLogError::SchemaValidation {
                        reason: format!("entry JSON decode failed: {err}"),
                    });
                }
            },
            None => {
                validation_error = Some(SessionLogError::SchemaValidation {
                    reason: "entries list contained a non-string value".into(),
                });
            }
        }
    });

    if let Some(err) = validation_error {
        Err(err)
    } else {
        Ok(())
    }
}

fn collect_entry_json_strings(doc: &LoroDoc) -> Result<Vec<String>, SessionLogError> {
    let list = doc.get_list(ENTRIES_CONTAINER);
    let mut out = Vec::with_capacity(list.len());
    let mut err: Option<SessionLogError> = None;
    list.for_each(|value| {
        if err.is_some() {
            return;
        }
        match value_to_string(&value) {
            Some(json) => out.push(json),
            None => {
                err = Some(SessionLogError::DecodeEntry {
                    reason: "entries list contained a non-string value".into(),
                });
            }
        }
    });
    if let Some(e) = err {
        Err(e)
    } else {
        Ok(out)
    }
}

fn validate_append_only_prefix(
    doc: &LoroDoc,
    original_json: &[String],
) -> Result<(), SessionLogError> {
    let staged_json = collect_entry_json_strings(doc)?;
    if staged_json.len() < original_json.len() {
        return Err(SessionLogError::SchemaValidation {
            reason: format!(
                "entry count decreased from {} to {}",
                original_json.len(),
                staged_json.len()
            ),
        });
    }
    for (i, original) in original_json.iter().enumerate() {
        if staged_json[i] != *original {
            return Err(SessionLogError::SchemaValidation {
                reason: format!("entry at index {i} mutated in append-only prefix"),
            });
        }
    }
    Ok(())
}

/// Fail an import whose [`loro::ImportStatus`] parked operations as
/// pending. Loro reports this as a *successful* import, so without the
/// guard a replica silently stops converging (the ops' causal
/// dependencies are missing locally and can never arrive if the sender
/// re-based past us).
fn reject_pending_ops(
    chunk: &ChunkId,
    status: &loro::ImportStatus,
) -> Result<(), SessionLogError> {
    match status.pending.as_ref() {
        Some(pending) => Err(SessionLogError::PendingRemoteOps {
            chunk_id: chunk.as_path(),
            detail: format!("{pending:?}"),
        }),
        None => Ok(()),
    }
}

/// Advance the per-session cursor (active chunk index + next sequence)
/// after `chunk`'s in-memory state changed via a remote update or
/// replace. Expects the chunk to be present in `inner.chunks`.
fn refresh_session_cursor(inner: &mut ManagerInner, chunk: &ChunkId) {
    let key = chunk.as_path();
    let session_key = chunk.session_key();
    let state = inner.chunks.get(&key).expect("just inserted or merged");
    let mut highest_seq = None;
    state
        .doc
        .get_list(ENTRIES_CONTAINER)
        .for_each(|value| {
            if let Some(json) = value_to_string(&value)
                && let Ok(entry) = serde_json::from_str::<Entry>(&json)
            {
                highest_seq = Some(highest_seq.map_or(entry.sequence, |h: u64| h.max(entry.sequence)));
            }
        });
    let cursor = inner.sessions.entry(session_key).or_insert(SessionCursor {
        active_chunk_index: chunk.chunk_index,
        next_sequence: 0,
    });
    if chunk.chunk_index > cursor.active_chunk_index {
        cursor.active_chunk_index = chunk.chunk_index;
    }
    if let Some(seq) = highest_seq
        && seq + 1 > cursor.next_sequence
    {
        cursor.next_sequence = seq + 1;
    }
}

fn hex_decode(hex: &str) -> Option<Vec<u8>> {
    if !hex.len().is_multiple_of(2) {
        return None;
    }
    let mut bytes = Vec::with_capacity(hex.len() / 2);
    for i in (0..hex.len()).step_by(2) {
        bytes.push(u8::from_str_radix(&hex[i..i + 2], 16).ok()?);
    }
    Some(bytes)
}

fn reconstruct_signed_envelope(accepted: &crate::storage::AcceptedEnvelope) -> Option<SignedEnvelope> {
    let sender_bytes = hex_decode(&accepted.sender_hex)?;
    let sender: [u8; PUBLIC_KEY_LENGTH] = sender_bytes.try_into().ok()?;
    let message_id_bytes = hex_decode(&accepted.message_id_hex)?;
    let message_id: [u8; ENVELOPE_HASH_LENGTH] = message_id_bytes.try_into().ok()?;
    let content_hash_bytes = hex_decode(&accepted.content_hash_hex)?;
    let content_hash: [u8; ENVELOPE_HASH_LENGTH] = content_hash_bytes.try_into().ok()?;
    let signature_bytes = hex_decode(&accepted.signature_hex)?;
    let signature: [u8; SIGNATURE_LENGTH] = signature_bytes.try_into().ok()?;
    let payload_kind = PayloadKind::from_byte(accepted.payload_kind as i32)?;
    Some(SignedEnvelope {
        sender,
        message_id,
        document_id: accepted.document_id.clone(),
        payload_kind,
        content_hash,
        signature,
        payload: accepted.payload_bytes.clone(),
    })
}

fn value_to_string(value: &loro::ValueOrContainer) -> Option<String> {
    use loro::LoroValue;
    use loro::ValueOrContainer;
    match value {
        ValueOrContainer::Value(LoroValue::String(s)) => Some((**s).to_string()),
        _ => None,
    }
}

fn doc_entry_stats(doc: &loro::LoroDoc) -> (u64, u64) {
    let list = doc.get_list(ENTRIES_CONTAINER);
    let mut count = 0u64;
    let mut bytes = 0u64;
    list.for_each(|value| {
        if let Some(json) = value_to_string(&value)
            && let Ok(entry) = serde_json::from_str::<Entry>(&json)
        {
            count += 1;
            bytes += entry.size_estimate();
        }
    });
    (count, bytes)
}


#[cfg(test)]
mod tests;
