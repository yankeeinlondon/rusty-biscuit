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
    /// `true` when the bytes carry a full snapshot (the peer had no
    /// prior state), `false` for an incremental update bundle.
    pub is_snapshot: bool,
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
                let updates = doc.export(ExportMode::updates(&vv))?;
                if updates.is_empty() {
                    Ok(Some(ExportedUpdate {
                        is_snapshot: false,
                        bytes: Vec::new(),
                    }))
                } else {
                    Ok(Some(ExportedUpdate {
                        is_snapshot: false,
                        bytes: updates,
                    }))
                }
            }
            None => Ok(Some(ExportedUpdate {
                is_snapshot: true,
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
            staged_doc.import(update_bytes)?;

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
            staged_doc.import(update_bytes)?;

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
            let Ok(chunk_id) = accepted.document_id.parse::<ChunkId>() else {
                tracing::warn!(
                    target: "rendezvous_daemon::session_log",
                    document_id = %accepted.document_id,
                    "skipping accepted envelope with malformed document_id during replay",
                );
                continue;
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

            match self.apply_remote_update(&chunk_id, &accepted.payload_bytes) {
                Ok(_) => {}
                Err(SessionLogError::Loro(reason)) => {
                    tracing::warn!(
                        target: "rendezvous_daemon::session_log",
                        chunk_id = %chunk_id.as_path(),
                        %reason,
                        "skipping malformed accepted envelope during replay",
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
mod tests {
    use super::*;
    use crate::batcher::{BatcherConfig, BatcherWorker, spawn};
    use crate::projection::Projection;
    use crate::storage::AcceptedEnvelope;
    use std::sync::atomic::{AtomicI64, Ordering};
    use tempfile::TempDir;

    fn make_remote_snapshot(
        owner_node_id: &str,
        session_id: &str,
        chunk_index: u64,
        entries: &[Entry],
    ) -> Vec<u8> {
        let doc = loro::LoroDoc::new();
        let map = doc.get_map(METADATA_CONTAINER);
        map.insert("owner_node_id", owner_node_id).unwrap();
        map.insert("session_id", session_id).unwrap();
        map.insert("chunk_index", chunk_index as i64).unwrap();
        map.insert("created_at_unix_ms", 5_000_i64).unwrap();
        if chunk_index > 0 {
            let prev = ChunkId::new(
                owner_node_id.to_string(),
                session_id.to_string(),
                chunk_index - 1,
            );
            map.insert("previous_chunk_id", prev.as_path().as_str()).unwrap();
        }
        let list = doc.get_list(ENTRIES_CONTAINER);
        for entry in entries {
            list.push(serde_json::to_string(entry).unwrap().as_str()).unwrap();
        }
        doc.commit();
        doc.export(ExportMode::Snapshot).unwrap()
    }

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
        manager: SessionLogManager,
        storage: Storage,
        worker: BatcherWorker,
        _tmp: TempDir,
    }

    fn build_harness(config: ChunkConfig) -> Harness {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("open storage");
        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager = SessionLogManager::with_clock(
            storage.clone(),
            worker.handle(),
            projection,
            config,
            Arc::new(NodeIdentity::from_seed([7u8; 32])),
            Arc::new(FixedClock::new(1_000)),
        )
        .expect("manager");
        Harness {
            manager,
            storage,
            worker,
            _tmp: tmp,
        }
    }

    #[test]
    fn append_persists_and_increments_sequence() {
        let harness = build_harness(ChunkConfig::default());
        let first = harness
            .manager
            .append_entry("node-a", "session-1", "test", "info", "hello", None)
            .expect("append");
        assert_eq!(first.sequence, 0);
        assert_eq!(first.chunk.chunk_index, 0);
        assert!(!first.rotated);

        let second = harness
            .manager
            .append_entry("node-a", "session-1", "test", "info", "world", None)
            .expect("append");
        assert_eq!(second.sequence, 1);
        assert_eq!(second.chunk.chunk_index, 0);

        let entries = harness
            .manager
            .list_chunk_entries(&first.chunk)
            .expect("list");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "hello");
        assert_eq!(entries[1].message, "world");

        assert_eq!(harness.storage.snapshot_count().unwrap(), 1);
        harness.worker.shutdown();
    }

    #[test]
    fn concurrent_appends_to_one_session_keep_unique_durable_sequences() {
        use std::sync::Barrier;

        let harness = build_harness(ChunkConfig::default());

        // Force genuine contention on the stage→persist→merge window: a
        // barrier releases both appends at once so, without per-session
        // serialization, they would clone the same cursor, reserve the
        // same sequence, and race their whole-snapshot redb writes.
        let append_count = 8usize;
        let barrier = Arc::new(Barrier::new(append_count));
        let handles: Vec<_> = (0..append_count)
            .map(|i| {
                let manager = harness.manager.clone();
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    manager
                        .append_entry(
                            "node-a",
                            "session-1",
                            "test",
                            "info",
                            format!("message-{i}"),
                            None,
                        )
                        .expect("concurrent append")
                })
            })
            .collect();

        let mut sequences: Vec<u64> = handles
            .into_iter()
            .map(|h| h.join().expect("append thread").sequence)
            .collect();
        sequences.sort_unstable();
        let expected: Vec<u64> = (0..append_count as u64).collect();
        assert_eq!(
            sequences, expected,
            "appends must hand out unique, gap-free sequences 0..N"
        );

        let chunk = ChunkId::new("node-a", "session-1", 0);

        let in_memory = harness
            .manager
            .list_chunk_entries(&chunk)
            .expect("list in-memory");
        let mut in_memory_messages: Vec<String> =
            in_memory.iter().map(|e| e.message.clone()).collect();
        in_memory_messages.sort();
        let expected_messages: Vec<String> =
            (0..append_count).map(|i| format!("message-{i}")).collect();
        assert_eq!(
            in_memory_messages, expected_messages,
            "every concurrent append must be present in memory"
        );

        harness.worker.shutdown();

        // Reload a fresh manager from the same redb storage: the durable
        // snapshot must already contain every entry. This is the part the
        // sequential tests miss — a lost redb write would only surface
        // after reload.
        let projection2 = Projection::in_memory().expect("projection2");
        let worker2 = spawn(projection2.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager2 = SessionLogManager::with_clock(
            harness.storage.clone(),
            worker2.handle(),
            projection2,
            ChunkConfig::default(),
            Arc::new(NodeIdentity::from_seed([7u8; 32])),
            Arc::new(FixedClock::new(10_000)),
        )
        .expect("reloaded manager");

        let durable = manager2
            .list_chunk_entries(&chunk)
            .expect("list after reload");
        let mut durable_messages: Vec<String> =
            durable.iter().map(|e| e.message.clone()).collect();
        durable_messages.sort();
        assert_eq!(
            durable_messages, expected_messages,
            "every concurrent append must survive in durable redb storage"
        );

        let mut durable_sequences: Vec<u64> = durable.iter().map(|e| e.sequence).collect();
        durable_sequences.sort_unstable();
        assert_eq!(
            durable_sequences, expected,
            "durable entries must keep unique, gap-free sequences after reload"
        );

        worker2.shutdown();
    }

    #[test]
    fn rotation_happens_at_configured_entry_cap() {
        let harness = build_harness(ChunkConfig::new(2, 1024));
        let chunk_zero = harness
            .manager
            .append_entry("node-a", "session-1", "t", "info", "a", None)
            .expect("a")
            .chunk;
        let chunk_zero_again = harness
            .manager
            .append_entry("node-a", "session-1", "t", "info", "b", None)
            .expect("b")
            .chunk;
        assert_eq!(chunk_zero, chunk_zero_again);

        let rotated = harness
            .manager
            .append_entry("node-a", "session-1", "t", "info", "c", None)
            .expect("c");
        assert!(rotated.rotated);
        assert_eq!(rotated.chunk.chunk_index, 1);
        assert_eq!(rotated.sequence, 2);

        let chunks = harness
            .manager
            .list_session_chunks("node-a", "session-1")
            .expect("chunks");
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[1].chunk_index, 1);
        harness.worker.shutdown();
    }

    #[test]
    fn rehydrate_picks_up_existing_snapshots() {
        let tmp = TempDir::new().expect("tempdir");
        let storage_path = tmp.path().join("session.redb");
        let storage = Storage::open(&storage_path).expect("storage");
        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager = SessionLogManager::with_clock(
            storage.clone(),
            worker.handle(),
            projection,
            ChunkConfig::default(),
            Arc::new(NodeIdentity::from_seed([3u8; 32])),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager");
        for _ in 0..3 {
            manager
                .append_entry("node-a", "session-1", "t", "info", "x", None)
                .expect("append");
        }
        worker.shutdown();
        drop(manager);

        let projection2 = Projection::in_memory().expect("projection");
        let worker2 = spawn(projection2.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager2 = SessionLogManager::with_clock(
            storage,
            worker2.handle(),
            projection2,
            ChunkConfig::default(),
            Arc::new(NodeIdentity::from_seed([3u8; 32])),
            Arc::new(FixedClock::new(100)),
        )
        .expect("manager2");
        let resumed = manager2
            .append_entry("node-a", "session-1", "t", "info", "fourth", None)
            .expect("append");
        assert_eq!(resumed.sequence, 3);
        worker2.shutdown();
        drop(tmp);
    }

    #[test]
    fn sign_chunk_snapshot_round_trips_through_verify() {
        let harness = build_harness(ChunkConfig::default());
        let outcome = harness
            .manager
            .append_entry("node-a", "session-1", "src", "info", "hello", None)
            .expect("append");
        let envelope = harness
            .manager
            .sign_chunk_snapshot(&outcome.chunk)
            .expect("sign")
            .expect("snapshot exists");
        assert_eq!(envelope.sender, harness.manager.identity().public_key_bytes());
        let payload = envelope.verify().expect("verify");
        assert!(!payload.is_empty());
        harness.worker.shutdown();
    }

    #[test]
    fn sign_chunk_snapshot_for_unknown_chunk_returns_none() {
        let harness = build_harness(ChunkConfig::default());
        let missing = ChunkId::new("node-z", "session-missing", 0);
        let result = harness
            .manager
            .sign_chunk_snapshot(&missing)
            .expect("sign call");
        assert!(result.is_none());
        harness.worker.shutdown();
    }

    #[test]
    fn signed_envelopes_from_different_identities_have_different_senders() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
        let projection_a = Projection::in_memory().expect("projection_a");
        let worker_a = spawn(projection_a.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager_a = SessionLogManager::with_clock(
            storage.clone(),
            worker_a.handle(),
            projection_a,
            ChunkConfig::default(),
            Arc::new(NodeIdentity::from_seed([11u8; 32])),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager_a");
        let outcome = manager_a
            .append_entry("node-a", "session-1", "src", "info", "msg", None)
            .expect("append");

        let projection_b = Projection::in_memory().expect("projection_b");
        let worker_b = spawn(projection_b.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager_b = SessionLogManager::with_clock(
            storage,
            worker_b.handle(),
            projection_b,
            ChunkConfig::default(),
            Arc::new(NodeIdentity::from_seed([22u8; 32])),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager_b");

        let envelope_a = manager_a
            .sign_chunk_snapshot(&outcome.chunk)
            .expect("sign a")
            .expect("snapshot a");
        let envelope_b = manager_b
            .sign_chunk_snapshot(&outcome.chunk)
            .expect("sign b")
            .expect("snapshot b");
        assert_ne!(envelope_a.sender, envelope_b.sender);
        envelope_a.verify().expect("verify a");
        envelope_b.verify().expect("verify b");

        worker_a.shutdown();
        worker_b.shutdown();
        drop(tmp);
    }

    #[test]
    fn rebuild_projection_from_storage_populates_duckdb() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
        let projection1 = Projection::in_memory().expect("projection1");
        let worker1 = spawn(projection1.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager1 = SessionLogManager::with_clock(
            storage.clone(),
            worker1.handle(),
            projection1,
            ChunkConfig::default(),
            Arc::new(NodeIdentity::from_seed([5u8; 32])),
            Arc::new(FixedClock::new(500)),
        )
        .expect("manager1");
        manager1
            .append_entry("node-a", "session-1", "src", "info", "alpha", None)
            .expect("append 1");
        manager1
            .append_entry("node-a", "session-1", "src", "info", "beta", None)
            .expect("append 2");
        worker1.shutdown();
        drop(manager1);

        let projection2 = Projection::in_memory().expect("projection2");
        let worker2 = spawn(projection2.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let _manager2 = SessionLogManager::with_clock(
            storage,
            worker2.handle(),
            projection2.clone(),
            ChunkConfig::default(),
            Arc::new(NodeIdentity::from_seed([5u8; 32])),
            Arc::new(FixedClock::new(900)),
        )
        .expect("manager2");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if projection2.row_count().unwrap() >= 2 {
                break;
            }
            if std::time::Instant::now() >= deadline {
                panic!("timed out waiting for projection rebuild");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let rows = projection2
            .entries_for_session("node-a", "session-1")
            .expect("query");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].message, "alpha");
        assert_eq!(rows[1].message, "beta");
        worker2.shutdown();
    }

    #[test]
    fn sealer_counter_persists_across_restart() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([77u8; 32]));
        let node_id = identity.node_id();

        let projection1 = Projection::in_memory().expect("projection1");
        let worker1 = spawn(projection1.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager1 = SessionLogManager::with_clock(
            storage.clone(),
            worker1.handle(),
            projection1,
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager1");

        let outcome = manager1
            .append_entry("n", "s", "t", "info", "m", None)
            .expect("append");
        let env1 = manager1
            .sign_chunk_snapshot(&outcome.chunk)
            .expect("sign")
            .expect("envelope");
        let counter1 = u64::from_be_bytes(env1.message_id[24..].try_into().unwrap());
        worker1.shutdown();
        drop(manager1);

        assert_eq!(storage.load_outbound_counter(&node_id).expect("load"), counter1 + 1);

        let projection2 = Projection::in_memory().expect("projection2");
        let worker2 = spawn(projection2.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager2 = SessionLogManager::with_clock(
            storage.clone(),
            worker2.handle(),
            projection2,
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(100)),
        )
        .expect("manager2");

        let env2 = manager2
            .sign_chunk_snapshot(&outcome.chunk)
            .expect("sign")
            .expect("envelope");
        let counter2 = u64::from_be_bytes(env2.message_id[24..].try_into().unwrap());
        assert!(counter2 > counter1, "counter after restart ({counter2}) must exceed pre-restart ({counter1})");
        worker2.shutdown();
    }

    #[test]
    fn accepted_envelope_only_replay_recovers_missing_snapshot() {
        let tmp = TempDir::new().expect("tempdir");
        let storage_path = tmp.path().join("session.redb");
        let storage = Storage::open(&storage_path).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([44u8; 32]));

        let projection1 = Projection::in_memory().expect("projection1");
        let worker1 = spawn(projection1.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager1 = SessionLogManager::with_clock(
            storage.clone(),
            worker1.handle(),
            projection1,
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager1");

        let sender_identity = NodeIdentity::from_seed([55u8; 32]);
        let sender_hex = sender_identity.node_id();
        let sender_chunk = ChunkId::new(&sender_hex, "remote-session", 0);

        let mut sender_sealer =
            EnvelopeSealer::with_start(sender_identity, 0);
        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "from-envelope".into(),
            metadata: None,
        };
        let snapshot = make_remote_snapshot(&sender_hex, "remote-session", 0, &[entry]);
        let envelope = sender_sealer.seal(
            sender_chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot.clone(),
        );
        let msg_id_hex = {
            let mut out = String::with_capacity(envelope.message_id.len() * 2);
            for byte in &envelope.message_id {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };
        let content_hash_hex = {
            let mut out = String::with_capacity(envelope.content_hash.len() * 2);
            for byte in &envelope.content_hash {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };
        let signature_hex = {
            let mut out = String::with_capacity(envelope.signature.len() * 2);
            for byte in &envelope.signature {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };

        manager1
            .apply_remote_update(&sender_chunk, &snapshot)
            .expect("apply");

        let accepted = AcceptedEnvelope {
            sender_hex: sender_hex.clone(),
            message_id_hex: msg_id_hex.clone(),
            document_id: sender_chunk.as_path(),
            payload_kind: PayloadKind::Snapshot.to_byte(),
            content_hash_hex,
            signature_hex,
            payload_bytes: snapshot.clone(),
            accepted_at_unix_ms: 5_001,
        };
        storage
            .save_accepted_envelope(&sender_hex, &msg_id_hex, &accepted)
            .expect("save accepted envelope");

        assert!(
            storage.load_snapshot(&sender_chunk).expect("load").is_some(),
            "snapshot must exist before removal",
        );
        storage.remove_snapshot(&sender_chunk).expect("remove snapshot");
        assert!(
            storage.load_snapshot(&sender_chunk).expect("load").is_none(),
            "snapshot must be gone after removal",
        );

        worker1.shutdown();
        drop(manager1);

        let projection2 = Projection::in_memory().expect("projection2");
        let worker2 = spawn(projection2.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager2 = SessionLogManager::with_clock(
            storage.clone(),
            worker2.handle(),
            projection2.clone(),
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(10_000)),
        )
        .expect("manager2");

        let entries = manager2
            .list_chunk_entries(&sender_chunk)
            .expect("list after replay");
        assert_eq!(entries.len(), 1, "replay should recover one entry; got {entries:?}");
        assert_eq!(entries[0].message, "from-envelope");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if projection2.row_count().unwrap() >= 1 {
                break;
            }
            if std::time::Instant::now() >= deadline {
                panic!("timed out waiting for projection row from envelope replay");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        let rows = projection2
            .entries_for_session(&sender_hex, "remote-session")
            .expect("query");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].message, "from-envelope");

        worker2.shutdown();
        drop(tmp);
    }

    #[test]
    fn envelope_before_snapshot_crash_window_recovers_on_restart() {
        let tmp = TempDir::new().expect("tempdir");
        let storage_path = tmp.path().join("session.redb");
        let storage = Storage::open(&storage_path).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([88u8; 32]));

        let sender_identity = NodeIdentity::from_seed([99u8; 32]);
        let sender_hex = sender_identity.node_id();
        let sender_chunk = ChunkId::new(&sender_hex, "crash-session", 0);

        let mut sender_sealer = EnvelopeSealer::with_start(sender_identity, 0);
        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 7_000,
            source: "remote".into(),
            level: "info".into(),
            message: "crash-window".into(),
            metadata: None,
        };
        let snapshot = make_remote_snapshot(&sender_hex, "crash-session", 0, &[entry]);
        let envelope = sender_sealer.seal(
            sender_chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot.clone(),
        );
        let msg_id_hex = {
            let mut out = String::with_capacity(envelope.message_id.len() * 2);
            for byte in &envelope.message_id {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };
        let content_hash_hex = {
            let mut out = String::with_capacity(envelope.content_hash.len() * 2);
            for byte in &envelope.content_hash {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };
        let signature_hex = {
            let mut out = String::with_capacity(envelope.signature.len() * 2);
            for byte in &envelope.signature {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };

        let accepted = AcceptedEnvelope {
            sender_hex: sender_hex.clone(),
            message_id_hex: msg_id_hex.clone(),
            document_id: sender_chunk.as_path(),
            payload_kind: PayloadKind::Snapshot.to_byte(),
            content_hash_hex,
            signature_hex,
            payload_bytes: snapshot.clone(),
            accepted_at_unix_ms: 7_001,
        };

        storage
            .save_accepted_envelope(&sender_hex, &msg_id_hex, &accepted)
            .expect("save envelope");

        assert!(
            storage.load_snapshot(&sender_chunk).expect("load").is_none(),
            "snapshot must not exist — we simulate the crash window where \
             the envelope was persisted but the snapshot write was lost",
        );

        let projection2 = Projection::in_memory().expect("projection2");
        let worker2 = spawn(projection2.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager2 = SessionLogManager::with_clock(
            storage.clone(),
            worker2.handle(),
            projection2.clone(),
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(20_000)),
        )
        .expect("manager2");

        let entries = manager2
            .list_chunk_entries(&sender_chunk)
            .expect("entries after replay");
        assert_eq!(
            entries.len(),
            1,
            "replay should recover one entry from the crash window; got {entries:?}",
        );
        assert_eq!(entries[0].message, "crash-window");

        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        loop {
            if projection2.row_count().unwrap() >= 1 {
                break;
            }
            if std::time::Instant::now() >= deadline {
                panic!("timed out waiting for projection row from crash-window replay");
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        let has_dup = storage.has_accepted_envelope(&sender_hex, &msg_id_hex).expect("has");
        assert!(has_dup, "accepted envelope must be durable after restart");

        worker2.shutdown();
        drop(tmp);
    }

    #[test]
    fn malformed_loro_payload_rejected_without_envelope_row() {
        let tmp = TempDir::new().expect("tempdir");
        let storage_path = tmp.path().join("session.redb");
        let storage = Storage::open(&storage_path).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([66u8; 32]));

        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager = SessionLogManager::with_clock(
            storage.clone(),
            worker.handle(),
            projection,
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager");

        let garbage_chunk = ChunkId::new("remote-node", "bad-session", 0);
        let garbage_payload = b"not valid loro data".to_vec();
        let result = manager.apply_remote_update(&garbage_chunk, &garbage_payload);
        assert!(result.is_err(), "malformed Loro payload must be rejected");

        assert_eq!(
            storage.accepted_envelope_count().expect("count"),
            0,
            "no accepted envelope should exist for rejected payload",
        );

        worker.shutdown();
        drop(manager);

        let projection2 = Projection::in_memory().expect("projection2");
        let worker2 = spawn(projection2.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager2 = SessionLogManager::with_clock(
            storage,
            worker2.handle(),
            projection2,
            ChunkConfig::default(),
            identity,
            Arc::new(FixedClock::new(100)),
        )
        .expect("restart must succeed when no malformed envelope was persisted");

        let entries = manager2.list_chunk_entries(&garbage_chunk).expect("list");
        assert!(entries.is_empty(), "no entries from rejected payload");

        worker2.shutdown();
        drop(tmp);
    }

    #[test]
    fn malformed_loro_payload_in_accepted_envelope_skipped_on_restart() {
        let tmp = TempDir::new().expect("tempdir");
        let storage_path = tmp.path().join("session.redb");
        let storage = Storage::open(&storage_path).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([77u8; 32]));

        let sender_identity = NodeIdentity::from_seed([88u8; 32]);
        let sender_hex = sender_identity.node_id();
        let sender_chunk = ChunkId::new(&sender_hex, "poison-session", 0);

        let mut sender_sealer = EnvelopeSealer::with_start(sender_identity, 0);
        let garbage_payload = b"this is not a valid loro snapshot".to_vec();
        let envelope = sender_sealer.seal(
            sender_chunk.as_path(),
            PayloadKind::Snapshot,
            garbage_payload.clone(),
        );

        let accepted = AcceptedEnvelope {
            sender_hex: sender_hex.clone(),
            message_id_hex: envelope.message_id_hex(),
            document_id: sender_chunk.as_path(),
            payload_kind: PayloadKind::Snapshot.to_byte(),
            content_hash_hex: {
                let mut out = String::with_capacity(envelope.content_hash.len() * 2);
                for byte in &envelope.content_hash {
                    use std::fmt::Write;
                    let _ = write!(out, "{byte:02x}");
                }
                out
            },
            signature_hex: {
                let mut out = String::with_capacity(envelope.signature.len() * 2);
                for byte in &envelope.signature {
                    use std::fmt::Write;
                    let _ = write!(out, "{byte:02x}");
                }
                out
            },
            payload_bytes: garbage_payload,
            accepted_at_unix_ms: 1_000,
        };
        storage
            .save_accepted_envelope(&sender_hex, &envelope.message_id_hex(), &accepted)
            .expect("save");

        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection, BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager = SessionLogManager::with_clock(
            storage,
            worker.handle(),
            Projection::in_memory().expect("projection2"),
            ChunkConfig::default(),
            identity,
            Arc::new(FixedClock::new(1)),
        )
        .expect("restarting with a malformed accepted envelope must succeed (tolerant replay)");

        let entries = manager.list_chunk_entries(&sender_chunk).expect("list");
        assert!(
            entries.is_empty(),
            "malformed payload should be skipped; got {entries:?}"
        );

        worker.shutdown();
        drop(tmp);
    }

    #[test]
    fn failed_persist_does_not_leave_entry_in_memory() {
        let harness = build_harness(ChunkConfig::default());

        let first = harness
            .manager
            .append_entry("node-a", "session-1", "test", "info", "hello", None)
            .expect("first append");
        assert_eq!(first.sequence, 0);

        harness.storage.inject_save_failure();

        let result = harness
            .manager
            .append_entry("node-a", "session-1", "test", "info", "should-fail", None);
        assert!(result.is_err(), "append must fail when persistence fails");

        let entries = harness
            .manager
            .list_chunk_entries(&first.chunk)
            .expect("list");
        assert_eq!(entries.len(), 1, "only the first entry should be visible");
        assert_eq!(entries[0].message, "hello");

        let chunks = harness
            .manager
            .list_session_chunks("node-a", "session-1")
            .expect("chunks");
        assert_eq!(chunks.len(), 1);

        let exported = harness
            .manager
            .export_updates_since(&first.chunk, None)
            .expect("export")
            .expect("snapshot exists");
        let doc = LoroDoc::from_snapshot(&exported.bytes).expect("parse exported snapshot");
        let list = doc.get_list(ENTRIES_CONTAINER);
        assert_eq!(list.len(), 1, "exported snapshot should have one entry");

        let third = harness
            .manager
            .append_entry("node-a", "session-1", "test", "info", "after-failure", None)
            .expect("append after failure");
        assert_eq!(third.sequence, 1, "sequence should resume from 1");

        let entries = harness
            .manager
            .list_chunk_entries(&first.chunk)
            .expect("list after recovery");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "hello");
        assert_eq!(entries[1].message, "after-failure");

        harness.worker.shutdown();
    }

    #[test]
    fn failed_persist_on_remote_update_does_not_leave_data_in_memory() {
        let harness = build_harness(ChunkConfig::default());

        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "from-remote".into(),
            metadata: None,
        };
        let remote_snapshot = make_remote_snapshot("remote-node", "remote-session", 0, &[entry]);

        let remote_chunk = ChunkId::new("remote-node", "remote-session", 0);

        let advanced = harness
            .manager
            .apply_remote_update(&remote_chunk, &remote_snapshot)
            .expect("first remote update");
        assert!(advanced);

        let entries = harness
            .manager
            .list_chunk_entries(&remote_chunk)
            .expect("list after first update");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "from-remote");

        let remote_doc2 = LoroDoc::from_snapshot(&remote_snapshot).unwrap();
        let remote_list2 = remote_doc2.get_list(ENTRIES_CONTAINER);
        let entry2 = Entry {
            sequence: 1,
            created_at_unix_ms: 6_000,
            source: "remote".into(),
            level: "info".into(),
            message: "should-not-persist".into(),
            metadata: None,
        };
        remote_list2
            .push(serde_json::to_string(&entry2).unwrap().as_str())
            .unwrap();
        remote_doc2.commit();
        let remote_update = remote_doc2.export(ExportMode::Snapshot).unwrap();

        harness.storage.inject_save_failure();
        let result = harness
            .manager
            .apply_remote_update(&remote_chunk, &remote_update);
        assert!(
            result.is_err(),
            "remote update must fail when persistence fails"
        );

        let entries = harness
            .manager
            .list_chunk_entries(&remote_chunk)
            .expect("list after failed update");
        assert_eq!(entries.len(), 1, "only the first entry should be visible");
        assert_eq!(entries[0].message, "from-remote");

        let exported = harness
            .manager
            .export_updates_since(&remote_chunk, None)
            .expect("export")
            .expect("snapshot exists");
        let doc = LoroDoc::from_snapshot(&exported.bytes).expect("parse");
        let list = doc.get_list(ENTRIES_CONTAINER);
        assert_eq!(list.len(), 1);

        let advanced = harness
            .manager
            .apply_remote_update(&remote_chunk, &remote_update)
            .expect("retry");
        assert!(advanced);

        let entries = harness
            .manager
            .list_chunk_entries(&remote_chunk)
            .expect("list after retry");
        assert_eq!(entries.len(), 2);

        harness.worker.shutdown();
    }

    #[test]
    fn accepted_envelope_failure_prevents_snapshot_persistence() {
        let tmp = TempDir::new().expect("tempdir");
        let storage_path = tmp.path().join("session.redb");
        let storage = Storage::open(&storage_path).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([11u8; 32]));

        let sender_identity = NodeIdentity::from_seed([22u8; 32]);
        let sender_hex = sender_identity.node_id();
        let sender_chunk = ChunkId::new(&sender_hex, "fail-envelope-session", 0);

        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 3_000,
            source: "remote".into(),
            level: "info".into(),
            message: "envelope-fail-entry".into(),
            metadata: None,
        };
        let payload = make_remote_snapshot(&sender_hex, "fail-envelope-session", 0, &[entry]);

        storage.inject_accepted_envelope_failure();
        let accepted = AcceptedEnvelope {
            sender_hex: sender_hex.clone(),
            message_id_hex: "deadbeef".into(),
            document_id: sender_chunk.as_path(),
            payload_kind: PayloadKind::Snapshot.to_byte(),
            content_hash_hex: "cafe".into(),
            signature_hex: String::new(),
            payload_bytes: payload.clone(),
            accepted_at_unix_ms: 3_001,
        };
        let result = storage.save_accepted_envelope(&sender_hex, "deadbeef", &accepted);
        assert!(result.is_err(), "accepted-envelope save must fail when injected");

        assert!(
            storage.load_snapshot(&sender_chunk).expect("load").is_none(),
            "no snapshot should exist when the envelope write was rejected",
        );

        assert_eq!(
            storage.accepted_envelope_count().expect("count"),
            0,
            "no accepted envelope should exist",
        );

        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager = SessionLogManager::with_clock(
            storage.clone(),
            worker.handle(),
            projection,
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager");

        let entries = manager.list_chunk_entries(&sender_chunk).expect("list");
        assert!(
            entries.is_empty(),
            "no entries should be visible without the accepted envelope; got {entries:?}",
        );

        worker.shutdown();
        drop(tmp);
    }

    #[test]
    fn envelope_before_snapshot_ordering_prevents_duplicate_on_restart() {
        let tmp = TempDir::new().expect("tempdir");
        let storage_path = tmp.path().join("session.redb");
        let storage = Storage::open(&storage_path).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([33u8; 32]));

        let sender_identity = NodeIdentity::from_seed([44u8; 32]);
        let sender_hex = sender_identity.node_id();
        let sender_chunk = ChunkId::new(&sender_hex, "ordering-session", 0);

        let mut sender_sealer = EnvelopeSealer::with_start(sender_identity, 0);
        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 9_000,
            source: "remote".into(),
            level: "info".into(),
            message: "ordering-entry".into(),
            metadata: None,
        };
        let snapshot = make_remote_snapshot(&sender_hex, "ordering-session", 0, &[entry]);
        let envelope = sender_sealer.seal(
            sender_chunk.as_path(),
            PayloadKind::Snapshot,
            snapshot.clone(),
        );
        let msg_id_hex = envelope.message_id_hex();
        let content_hash_hex = {
            let mut out = String::with_capacity(envelope.content_hash.len() * 2);
            for byte in &envelope.content_hash {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };
        let signature_hex = {
            let mut out = String::with_capacity(envelope.signature.len() * 2);
            for byte in &envelope.signature {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };

        let accepted = AcceptedEnvelope {
            sender_hex: sender_hex.clone(),
            message_id_hex: msg_id_hex.clone(),
            document_id: sender_chunk.as_path(),
            payload_kind: PayloadKind::Snapshot.to_byte(),
            content_hash_hex,
            signature_hex,
            payload_bytes: snapshot.clone(),
            accepted_at_unix_ms: 9_001,
        };

        storage
            .save_accepted_envelope(&sender_hex, &msg_id_hex, &accepted)
            .expect("save accepted envelope first");

        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager = SessionLogManager::with_clock(
            storage.clone(),
            worker.handle(),
            projection,
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(1)),
        )
        .expect("manager");

        manager
            .apply_remote_update(&sender_chunk, &snapshot)
            .expect("apply after envelope is persisted");

        let entries = manager.list_chunk_entries(&sender_chunk).expect("list");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "ordering-entry");

        worker.shutdown();
        drop(manager);

        let projection2 = Projection::in_memory().expect("projection2");
        let worker2 = spawn(projection2.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager2 = SessionLogManager::with_clock(
            storage.clone(),
            worker2.handle(),
            projection2.clone(),
            ChunkConfig::default(),
            Arc::clone(&identity),
            Arc::new(FixedClock::new(50_000)),
        )
        .expect("manager2");

        let entries2 = manager2.list_chunk_entries(&sender_chunk).expect("list after restart");
        assert_eq!(entries2.len(), 1, "entry should survive restart");
        assert_eq!(entries2[0].message, "ordering-entry");

        assert!(
            storage.has_accepted_envelope(&sender_hex, &msg_id_hex).expect("has"),
            "accepted envelope must still be present after restart",
        );

        let projection3 = Projection::in_memory().expect("projection3");
        let worker3 = spawn(projection3.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let mut inbox = rendezvous_core::EnvelopeInbox::new();
        let verified = inbox.accept(&envelope).expect("verify envelope again").to_vec();
        let accepted2 = AcceptedEnvelope {
            sender_hex: sender_hex.clone(),
            message_id_hex: msg_id_hex.clone(),
            document_id: sender_chunk.as_path(),
            payload_kind: PayloadKind::Snapshot.to_byte(),
            content_hash_hex: accepted.content_hash_hex.clone(),
            signature_hex: accepted.signature_hex.clone(),
            payload_bytes: verified,
            accepted_at_unix_ms: 99_000,
        };
        let dup_result = storage.save_accepted_envelope(&sender_hex, &msg_id_hex, &accepted2);
        assert!(
            dup_result.is_ok(),
            "save_accepted_envelope must succeed (idempotent)",
        );
        assert_eq!(
            storage.accepted_envelope_count().expect("count"),
            1,
            "duplicate accepted envelope must not create a second row",
        );

        worker2.shutdown();
        worker3.shutdown();
        drop(tmp);
    }

    #[test]
    fn append_succeeds_after_batcher_shutdown_without_retry_ambiguity() {
        let harness = build_harness(ChunkConfig::default());

        let first = harness
            .manager
            .append_entry("node-a", "session-1", "test", "info", "before-shutdown", None)
            .expect("first append");
        assert_eq!(first.sequence, 0);

        harness.worker.shutdown();

        let second = harness
            .manager
            .append_entry("node-a", "session-1", "test", "info", "after-shutdown", None)
            .expect("append must succeed even with closed batcher");
        assert_eq!(second.sequence, 1);

        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            1,
            "snapshot must be durable in redb",
        );

        let entries = harness
            .manager
            .list_chunk_entries(&first.chunk)
            .expect("list");
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].message, "before-shutdown");
        assert_eq!(entries[1].message, "after-shutdown");
    }

    #[test]
    fn existing_entry_message_mutation_rejected_as_append_only_violation() {
        let harness = build_harness(ChunkConfig::default());
        let remote_node = "remote-node";
        let chunk = ChunkId::new(remote_node, "append-only-mutate", 0);

        let entry1 = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "original".into(),
            metadata: None,
        };
        let snapshot1 = make_remote_snapshot(remote_node, "append-only-mutate", 0, &[entry1]);
        harness
            .manager
            .apply_remote_update(&chunk, &snapshot1)
            .expect("first apply");

        let entries_before = harness.manager.list_chunk_entries(&chunk).expect("list");
        assert_eq!(entries_before.len(), 1);
        assert_eq!(entries_before[0].message, "original");

        let doc = LoroDoc::from_snapshot(&snapshot1).unwrap();
        let list = doc.get_list(ENTRIES_CONTAINER);
        list.delete(0, 1).unwrap();
        let mutated_entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "mutated".into(),
            metadata: None,
        };
        list.insert(
            0,
            serde_json::to_string(&mutated_entry).unwrap().as_str(),
        )
        .unwrap();
        doc.commit();
        let snapshot2 = doc.export(ExportMode::Snapshot).unwrap();

        let result = harness.manager.apply_remote_update(&chunk, &snapshot2);
        assert!(
            result.is_err(),
            "mutation of existing entry must be rejected"
        );

        let entries_after = harness.manager.list_chunk_entries(&chunk).expect("list");
        assert_eq!(entries_after.len(), 1);
        assert_eq!(entries_after[0].message, "original");

        harness.worker.shutdown();
    }

    #[test]
    fn existing_entry_deletion_with_replacement_rejected_as_append_only_violation() {
        let harness = build_harness(ChunkConfig::default());
        let remote_node = "remote-node";
        let chunk = ChunkId::new(remote_node, "append-only-replace", 0);

        let entry_a = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "alpha".into(),
            metadata: None,
        };
        let entry_b = Entry {
            sequence: 1,
            created_at_unix_ms: 6_000,
            source: "remote".into(),
            level: "info".into(),
            message: "beta".into(),
            metadata: None,
        };
        let snapshot1 =
            make_remote_snapshot(remote_node, "append-only-replace", 0, &[entry_a, entry_b]);
        harness
            .manager
            .apply_remote_update(&chunk, &snapshot1)
            .expect("first apply");

        let snapshots_before = harness.storage.snapshot_count().expect("count");

        let doc = LoroDoc::from_snapshot(&snapshot1).unwrap();
        let list = doc.get_list(ENTRIES_CONTAINER);
        list.delete(0, 1).unwrap();
        let replacement = Entry {
            sequence: 0,
            created_at_unix_ms: 9_000,
            source: "attacker".into(),
            level: "warn".into(),
            message: "replaced".into(),
            metadata: None,
        };
        list.insert(
            0,
            serde_json::to_string(&replacement).unwrap().as_str(),
        )
        .unwrap();
        doc.commit();
        let snapshot2 = doc.export(ExportMode::Snapshot).unwrap();

        let result = harness.manager.apply_remote_update(&chunk, &snapshot2);
        assert!(
            result.is_err(),
            "deletion+replacement of existing entry must be rejected"
        );

        let entries_after = harness.manager.list_chunk_entries(&chunk).expect("list");
        assert_eq!(entries_after.len(), 2);
        assert_eq!(entries_after[0].message, "alpha");
        assert_eq!(entries_after[1].message, "beta");
        assert_eq!(
            harness.storage.snapshot_count().expect("count"),
            snapshots_before,
        );

        harness.worker.shutdown();
    }

    #[test]
    fn existing_entry_reordering_rejected_as_append_only_violation() {
        let harness = build_harness(ChunkConfig::default());
        let remote_node = "remote-node";
        let chunk = ChunkId::new(remote_node, "append-only-reorder", 0);

        let entry_a = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "alpha".into(),
            metadata: None,
        };
        let entry_b = Entry {
            sequence: 1,
            created_at_unix_ms: 6_000,
            source: "remote".into(),
            level: "info".into(),
            message: "beta".into(),
            metadata: None,
        };
        let snapshot1 =
            make_remote_snapshot(remote_node, "append-only-reorder", 0, &[entry_a, entry_b]);
        harness
            .manager
            .apply_remote_update(&chunk, &snapshot1)
            .expect("first apply");

        let doc = LoroDoc::from_snapshot(&snapshot1).unwrap();
        let list = doc.get_list(ENTRIES_CONTAINER);
        list.delete(0, 2).unwrap();
        let swapped_b = Entry {
            sequence: 0,
            created_at_unix_ms: 6_000,
            source: "remote".into(),
            level: "info".into(),
            message: "beta".into(),
            metadata: None,
        };
        let swapped_a = Entry {
            sequence: 1,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "alpha".into(),
            metadata: None,
        };
        list.insert(
            0,
            serde_json::to_string(&swapped_b).unwrap().as_str(),
        )
        .unwrap();
        list.insert(
            1,
            serde_json::to_string(&swapped_a).unwrap().as_str(),
        )
        .unwrap();
        doc.commit();
        let snapshot2 = doc.export(ExportMode::Snapshot).unwrap();

        let result = harness.manager.apply_remote_update(&chunk, &snapshot2);
        assert!(
            result.is_err(),
            "reordering of existing entries must be rejected"
        );

        let entries_after = harness.manager.list_chunk_entries(&chunk).expect("list");
        assert_eq!(entries_after[0].message, "alpha");
        assert_eq!(entries_after[1].message, "beta");

        harness.worker.shutdown();
    }

    #[test]
    fn first_snapshot_with_previous_chunk_id_on_chunk_0_rejected() {
        let harness = build_harness(ChunkConfig::default());
        let remote_node = "remote-node";
        let chunk = ChunkId::new(remote_node, "prev-id-session", 0);

        let doc = loro::LoroDoc::new();
        let map = doc.get_map(METADATA_CONTAINER);
        map.insert("owner_node_id", remote_node).unwrap();
        map.insert("session_id", "prev-id-session").unwrap();
        map.insert("chunk_index", 0i64).unwrap();
        map.insert("created_at_unix_ms", 5_000i64).unwrap();
        map.insert("previous_chunk_id", "bogus-prev-chunk").unwrap();
        let list = doc.get_list(ENTRIES_CONTAINER);
        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "bad-prev".into(),
            metadata: None,
        };
        list.push(serde_json::to_string(&entry).unwrap().as_str())
            .unwrap();
        doc.commit();
        let snapshot = doc.export(ExportMode::Snapshot).unwrap();

        let result = harness.manager.apply_remote_update(&chunk, &snapshot);
        assert!(
            result.is_err(),
            "first snapshot with previous_chunk_id for chunk 0 must be rejected"
        );

        let entries = harness.manager.list_chunk_entries(&chunk).expect("list");
        assert!(
            entries.is_empty(),
            "no entries should be visible; got {entries:?}"
        );

        harness.worker.shutdown();
    }

    #[test]
    fn first_snapshot_with_integer_previous_chunk_id_on_chunk_0_rejected() {
        let harness = build_harness(ChunkConfig::default());
        let remote_node = "remote-node";
        let chunk = ChunkId::new(remote_node, "prev-int-session", 0);

        let doc = loro::LoroDoc::new();
        let map = doc.get_map(METADATA_CONTAINER);
        map.insert("owner_node_id", remote_node).unwrap();
        map.insert("session_id", "prev-int-session").unwrap();
        map.insert("chunk_index", 0i64).unwrap();
        map.insert("created_at_unix_ms", 5_000i64).unwrap();
        map.insert("previous_chunk_id", 999i64).unwrap();
        let list = doc.get_list(ENTRIES_CONTAINER);
        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "int-prev".into(),
            metadata: None,
        };
        list.push(serde_json::to_string(&entry).unwrap().as_str())
            .unwrap();
        doc.commit();
        let snapshot = doc.export(ExportMode::Snapshot).unwrap();

        let result = harness.manager.apply_remote_update(&chunk, &snapshot);
        assert!(
            result.is_err(),
            "first snapshot with integer previous_chunk_id for chunk 0 must be rejected"
        );

        harness.worker.shutdown();
    }

    #[test]
    fn existing_chunk_0_with_integer_previous_chunk_id_rejected() {
        let harness = build_harness(ChunkConfig::default());
        let remote_node = "remote-node";
        let chunk = ChunkId::new(remote_node, "exist-prev-int", 0);

        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "initial".into(),
            metadata: None,
        };
        let snapshot1 = make_remote_snapshot(remote_node, "exist-prev-int", 0, &[entry]);
        harness
            .manager
            .apply_remote_update(&chunk, &snapshot1)
            .expect("first apply");

        let doc = LoroDoc::from_snapshot(&snapshot1).unwrap();
        let map = doc.get_map(METADATA_CONTAINER);
        map.insert("previous_chunk_id", 42i64).unwrap();
        doc.commit();
        let snapshot2 = doc.export(ExportMode::Snapshot).unwrap();

        let result = harness.manager.apply_remote_update(&chunk, &snapshot2);
        assert!(
            result.is_err(),
            "existing chunk 0 with injected integer previous_chunk_id must be rejected"
        );

        let entries = harness.manager.list_chunk_entries(&chunk).expect("list");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].message, "initial");

        harness.worker.shutdown();
    }

    fn build_accepted_envelope(
        sender_identity: &NodeIdentity,
        chunk: &ChunkId,
        payload: Vec<u8>,
    ) -> (AcceptedEnvelope, rendezvous_core::SignedEnvelope) {
        let mut sealer = EnvelopeSealer::new(sender_identity.clone());
        let signed = sealer.seal(chunk.as_path(), PayloadKind::Snapshot, payload.clone());
        let accepted = AcceptedEnvelope {
            sender_hex: signed.sender_node_id(),
            message_id_hex: signed.message_id_hex(),
            document_id: chunk.as_path(),
            payload_kind: PayloadKind::Snapshot.to_byte(),
            content_hash_hex: {
                let mut out = String::with_capacity(signed.content_hash.len() * 2);
                for byte in &signed.content_hash {
                    use std::fmt::Write;
                    let _ = write!(out, "{byte:02x}");
                }
                out
            },
            signature_hex: {
                let mut out = String::with_capacity(signed.signature.len() * 2);
                for byte in &signed.signature {
                    use std::fmt::Write;
                    let _ = write!(out, "{byte:02x}");
                }
                out
            },
            payload_bytes: payload,
            accepted_at_unix_ms: 5_001,
        };
        (accepted, signed)
    }

    fn start_fresh_manager(
        storage: Storage,
        identity: &Arc<NodeIdentity>,
        clock_start: i64,
    ) -> (SessionLogManager, BatcherWorker) {
        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(projection.clone(), BatcherConfig {
            flush_interval: std::time::Duration::from_millis(20),
            flush_size: 16,
        });
        let manager = SessionLogManager::with_clock(
            storage,
            worker.handle(),
            projection,
            ChunkConfig::default(),
            Arc::clone(identity),
            Arc::new(FixedClock::new(clock_start)),
        )
        .expect("manager");
        (manager, worker)
    }

    #[test]
    fn replay_skips_accepted_envelope_with_tampered_signature() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([55u8; 32]));

        let sender_identity = NodeIdentity::from_seed([66u8; 32]);
        let sender_hex = sender_identity.node_id();
        let chunk = ChunkId::new(&sender_hex, "replay-sig-session", 0);

        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "replay-sig".into(),
            metadata: None,
        };
        let payload = make_remote_snapshot(&sender_hex, "replay-sig-session", 0, &[entry]);

        let (mut accepted, _signed) =
            build_accepted_envelope(&sender_identity, &chunk, payload.clone());
        accepted.signature_hex = {
            let bytes: Vec<u8> = (0..64).map(|_| 0xFFu8).collect();
            let mut out = String::with_capacity(128);
            for byte in &bytes {
                use std::fmt::Write;
                let _ = write!(out, "{byte:02x}");
            }
            out
        };

        storage
            .save_accepted_envelope(&sender_hex, &accepted.message_id_hex, &accepted)
            .expect("save");

        let (manager, worker) = start_fresh_manager(storage, &identity, 10_000);

        let entries = manager.list_chunk_entries(&chunk).expect("list");
        assert!(
            entries.is_empty(),
            "tampered signature must be skipped during replay; got {entries:?}"
        );

        worker.shutdown();
    }

    #[test]
    fn replay_skips_accepted_envelope_with_tampered_content_hash() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([56u8; 32]));

        let sender_identity = NodeIdentity::from_seed([67u8; 32]);
        let sender_hex = sender_identity.node_id();
        let chunk = ChunkId::new(&sender_hex, "replay-hash-session", 0);

        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "replay-hash".into(),
            metadata: None,
        };
        let payload = make_remote_snapshot(&sender_hex, "replay-hash-session", 0, &[entry]);

        let (mut accepted, _signed) =
            build_accepted_envelope(&sender_identity, &chunk, payload.clone());
        accepted.content_hash_hex = "ff".repeat(32);

        storage
            .save_accepted_envelope(&sender_hex, &accepted.message_id_hex, &accepted)
            .expect("save");

        let (manager, worker) = start_fresh_manager(storage, &identity, 10_000);

        let entries = manager.list_chunk_entries(&chunk).expect("list");
        assert!(
            entries.is_empty(),
            "tampered content hash must be skipped during replay; got {entries:?}"
        );

        worker.shutdown();
    }

    #[test]
    fn replay_skips_accepted_envelope_with_malformed_sender_hex() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([57u8; 32]));

        let sender_identity = NodeIdentity::from_seed([68u8; 32]);
        let sender_hex = sender_identity.node_id();
        let chunk = ChunkId::new(&sender_hex, "replay-malformed-session", 0);

        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "replay-malformed".into(),
            metadata: None,
        };
        let payload = make_remote_snapshot(&sender_hex, "replay-malformed-session", 0, &[entry]);

        let (mut accepted, _signed) =
            build_accepted_envelope(&sender_identity, &chunk, payload.clone());
        accepted.sender_hex = "ZZZZ_not_valid_hex".to_string();

        storage
            .save_accepted_envelope(&accepted.sender_hex, &accepted.message_id_hex, &accepted)
            .expect("save");

        let (manager, worker) = start_fresh_manager(storage, &identity, 10_000);

        let entries = manager.list_chunk_entries(&chunk).expect("list");
        assert!(
            entries.is_empty(),
            "malformed sender hex must be skipped during replay; got {entries:?}"
        );

        worker.shutdown();
    }

    #[test]
    fn replay_skips_accepted_envelope_with_wrong_payload_kind() {
        let tmp = TempDir::new().expect("tempdir");
        let storage = Storage::open(tmp.path().join("session.redb")).expect("storage");
        let identity = Arc::new(NodeIdentity::from_seed([58u8; 32]));

        let sender_identity = NodeIdentity::from_seed([69u8; 32]);
        let sender_hex = sender_identity.node_id();
        let chunk = ChunkId::new(&sender_hex, "replay-kind-session", 0);

        let entry = Entry {
            sequence: 0,
            created_at_unix_ms: 5_000,
            source: "remote".into(),
            level: "info".into(),
            message: "replay-kind".into(),
            metadata: None,
        };
        let payload = make_remote_snapshot(&sender_hex, "replay-kind-session", 0, &[entry]);

        let (mut accepted, _signed) =
            build_accepted_envelope(&sender_identity, &chunk, payload.clone());
        accepted.payload_kind = PayloadKind::Delta.to_byte();

        storage
            .save_accepted_envelope(&sender_hex, &accepted.message_id_hex, &accepted)
            .expect("save");

        let (manager, worker) = start_fresh_manager(storage, &identity, 10_000);

        let entries = manager.list_chunk_entries(&chunk).expect("list");
        assert!(
            entries.is_empty(),
            "wrong payload kind must be skipped during replay; got {entries:?}"
        );

        worker.shutdown();
    }
}
